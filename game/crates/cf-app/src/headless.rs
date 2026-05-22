use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};

use cf_control::{
    engine::{run_m0_inline, M0Engine, M0EngineConfig},
    runtime::{build_engine_config, resolve_run_bundle_root, ConfigInputs},
    server::{ControlServer, ControlServerConfig},
    Settings,
};
use cf_sim_core::WallClock;

use crate::app::resources::ControlRuntime;
use crate::cli::Cli;

/// Reject the `--headless-smoke --capture-grid` combination at parse time.
///
/// The headless paths (`run_headless_server` / `run_headless`) skip the entire
/// Bevy `DefaultPlugins` stack — there is no swapchain, no render world, and
/// no `Screenshot` observer to read back. Silently consuming `--capture-grid`
/// in this combination would produce zero PNGs without a recorded error,
/// violating the AGENTS.md Contract Integrity Gate ("no fake success").
///
/// `cf-e2e --capture-grid` already drops `--headless-smoke` from the spawn
/// args; this guard catches direct `cf-app` invocations from CI scripts or
/// operators.
pub(crate) fn reject_capture_grid_with_headless_smoke(cli: &Cli) -> Result<()> {
    if cli.headless_smoke && cli.capture_grid {
        anyhow::bail!(
            "--capture-grid is incompatible with --headless-smoke: the headless paths skip the \
             Bevy render world, so there is no swapchain to read back. Drop --headless-smoke to \
             use the windowed capture path (windowed-hidden mode is fine), or wait for the \
             offscreen RenderTarget readback to ship per T-CAPTURE done-criteria. \
             cf-e2e --capture-grid already drops --headless-smoke automatically."
        );
    }
    Ok(())
}

/// Headless + control API: start the loopback JSON-RPC server, tick the sim at the configured
/// `--tick-rate-hz` against the wall clock, drain `runbundle.write` requests, and exit when
/// shutdown is requested OR the configured tick budget is hit (a budget of `0` means "run
/// until shutdown").
pub(crate) fn run_headless_server(
    config: M0EngineConfig,
    control_port: u16,
    _uds: Option<PathBuf>,
    control_port_file: Option<PathBuf>,
    unpaced: bool,
) -> Result<()> {
    let engine = Arc::new(M0Engine::new(config.clone()));
    engine.record_run_started();
    engine.record_setting_snapshot();

    let control_rt = start_control_server(engine.clone(), control_port)?;
    write_control_port_file(control_port_file.as_deref(), control_rt.bound_addr)?;
    let _control_rt = control_rt;
    let bundle_written = if unpaced {
        run_unpaced_loop(&engine, config.duration_ticks)
    } else {
        run_paced_loop(&engine, config.duration_ticks, config.tick_rate_hz)
    };

    let mut bundle_written = bundle_written;
    flush_pending_runbundle(&engine, &mut bundle_written);
    engine.record_run_finished(0);
    if config.write_run_bundle {
        let ended = WallClock.now_utc();
        let bundle = engine
            .write_run_bundle(ended, 0)
            .context("final write_run_bundle failed")?;
        tracing::info!(target: "cf::app", run_id = %engine.run_id(), bundle = %bundle.display(), "M0 run bundle written on exit");
    } else {
        tracing::info!(target: "cf::app", run_id = %engine.run_id(), ticks = engine.current_tick().0, "M0 headless+control-api exited without --write-run-bundle");
    }
    Ok(())
}

/// Pace `engine` against the wall clock at `tick_rate_hz`, exiting when the engine reports
/// shutdown OR the configured `target_ticks` budget is hit (`0` = run until shutdown).
/// Returns `true` if any pending runbundle was written during the loop.
///
/// Extracted for direct unit testing (see `run_paced_loop_holds_wall_clock_cadence`).
pub(crate) fn run_paced_loop(engine: &Arc<M0Engine>, target_ticks: u64, tick_rate_hz: u32) -> bool {
    let tick_dt = std::time::Duration::from_nanos(1_000_000_000 / u64::from(tick_rate_hz.max(1)));
    let started = engine.started_instant();
    let mut next_tick_at = started + tick_dt;
    let mut bundle_written = false;
    let poll_chunk = std::time::Duration::from_millis(5);
    loop {
        if engine.shutdown_requested() {
            break;
        }
        if target_ticks > 0 && engine.current_tick().0 >= target_ticks {
            break;
        }
        let _ = engine.drive_tick();
        flush_pending_runbundle(engine, &mut bundle_written);
        loop {
            if engine.shutdown_requested() {
                break;
            }
            let now = std::time::Instant::now();
            if next_tick_at <= now {
                break;
            }
            let remaining = next_tick_at - now;
            std::thread::sleep(remaining.min(poll_chunk));
        }
        next_tick_at += tick_dt;
    }
    bundle_written
}

/// while the control API processes commands on its own tokio runtime. When
/// the engine's clock budget is exhausted, sleep briefly so the control
/// server can dispatch the next `sim.run_for_ticks` / `sim.step` command.
/// Used by `--headless-smoke --control-api --unpaced` and required for
/// cf-e2e scripts whose total sim ticks exceed the wall-clock window cf-e2e's
/// 180s timeout allows (18000-tick m1_5min_endurance would otherwise take
/// 300s of wall-clock pacing).
///
/// Unlike `run_paced_loop`, this **does not** auto-exit when
/// `engine.current_tick() >= config.duration_ticks`. The driving cf-e2e
/// session is authoritative over lifecycle: it explicitly invokes
/// `system.shutdown` (or closes the WS) when the script is done.
/// Auto-exiting on `target_ticks` in unpaced mode would cause cf-app to
/// tear down its control server BEFORE cf-e2e finishes the script,
/// producing the "Connection reset without closing handshake" failure mode
/// observed during M1 R2 development. `_target_ticks` is accepted for
/// signature parity but intentionally unused.
pub(crate) fn run_unpaced_loop(engine: &Arc<M0Engine>, _target_ticks: u64) -> bool {
    let mut bundle_written = false;
    let idle_chunk = std::time::Duration::from_millis(1);
    loop {
        if engine.shutdown_requested() {
            break;
        }
        if engine.drive_tick().is_none() {
            flush_pending_runbundle(engine, &mut bundle_written);
            std::thread::sleep(idle_chunk);
            continue;
        }
        flush_pending_runbundle(engine, &mut bundle_written);
    }
    bundle_written
}

pub(crate) fn flush_pending_runbundle(engine: &Arc<M0Engine>, bundle_written: &mut bool) {
    if !engine.pending_runbundle() {
        return;
    }
    let ended = WallClock.now_utc();
    match engine.write_run_bundle(ended, 0) {
        Ok(_) => {
            tracing::info!(target: "cf::app", "runbundle.write delivered");
            *bundle_written = true;
        }
        Err(err) => tracing::error!(target: "cf::app", error = %err, "runbundle.write failed"),
    }
    engine.clear_pending_runbundle();
}

/// Block until every PNG path the capture handle knows about exists on disk
/// OR `timeout` elapses. Replaces the earlier fixed 500 ms sleep that raced
/// Bevy's asynchronous `Screenshot::observe(save_to_disk)` flush queue
/// (issue #17). Polling cadence is 50 ms — about 10x finer than the 500 ms
/// the sleep used, while bounding wall-clock cost to the actual flush time
/// rather than always waiting the worst case.
pub(crate) fn wait_for_capture_pngs_flushed(
    handle: &cf_capture::CaptureStateHandle,
    captures_dir: &Path,
    timeout: std::time::Duration,
) {
    let started = std::time::Instant::now();
    let poll_interval = std::time::Duration::from_millis(50);
    let mut poison_warned = false;
    loop {
        let expected_paths: Vec<std::path::PathBuf> = {
            match handle.events_log.lock() {
                Ok(events) => events
                    .iter()
                    .map(|entry| captures_dir.join(&entry.png_relpath))
                    .collect(),
                Err(poisoned) => {
                    if !poison_warned {
                        tracing::warn!(
                            target: "cf::capture",
                            "capture events_log mutex poisoned during flush wait; proceeding with manifest write \
                             (warning suppressed for subsequent poll iterations within this call)"
                        );
                        poison_warned = true;
                    }
                    poisoned
                        .into_inner()
                        .iter()
                        .map(|entry| captures_dir.join(&entry.png_relpath))
                        .collect()
                }
            }
        };
        let missing = expected_paths.iter().filter(|p| !p.exists()).count();
        if missing == 0 {
            tracing::debug!(
                target: "cf::capture",
                expected = expected_paths.len(),
                elapsed_ms = started.elapsed().as_millis() as u64,
                "all capture PNGs flushed to disk"
            );
            return;
        }
        if started.elapsed() >= timeout {
            tracing::warn!(
                target: "cf::capture",
                expected = expected_paths.len(),
                missing,
                elapsed_ms = started.elapsed().as_millis() as u64,
                "capture PNG flush wait timed out; returning to caller — \
                 the downstream `write_capture_manifest_from_handle` filter \
                 will defensively skip the {missing} entries whose PNGs \
                 never landed on disk"
            );
            return;
        }
        std::thread::sleep(poll_interval);
    }
}

pub(crate) fn build_config(cli: &Cli, scenario_path: PathBuf) -> Result<M0EngineConfig> {
    let run_mode = if cli.headless_smoke {
        "headless-smoke".to_string()
    } else if cli.control_api {
        "bevy-control-driven".to_string()
    } else {
        "bevy-interactive".to_string()
    };
    let cli_duration = compute_duration(cli.ticks, cli.run_seconds, cli.tick_rate_hz);
    let content_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut settings = Settings::load_from_content_dir(&content_root);
    settings.ui_scale = cli.ui_scale;
    settings.high_contrast = cli.high_contrast;
    settings.captions = cli.captions.as_bool();
    settings.reduced_motion = cli.reduced_motion;
    settings.reduced_shake = cli.reduced_shake;
    settings.reduced_flash = cli.reduced_flash;
    settings.hold_to_confirm = cli.hold_to_confirm;
    settings.hold_threshold_ms = cli.hold_threshold_ms;
    settings.key_remap_enabled = cli.key_remap_enabled;
    settings.tick_rate_hz = cli.tick_rate_hz;
    settings.ai_debug = cli.ai_debug;
    let inputs = ConfigInputs {
        scenario_id: cli.scenario.clone(),
        scenario_path,
        run_mode,
        run_bundle_root: resolve_run_bundle_root(cli.run_bundle_dir.clone()),
        write_run_bundle: cli.write_run_bundle,
        control_api_enabled: cli.control_api,
        debug_capabilities: cli.debug_capabilities.clone(),
        tick_rate_hz: cli.tick_rate_hz,
        capture_grid_enabled: cli.capture_grid,
        paced: cli_duration > 0,
        settings,
        seed_override: cli.seed,
        duration_ticks_override: if cli_duration > 0 { Some(cli_duration) } else { None },
        debug_inject_panic_at_tick: cli.debug_inject_panic_at_tick,
        checksum_cadence_ticks: cli.checksum_cadence_ticks,
        expected_outcome: cli.expected_outcome.map(Into::into),
    };
    let mut config = build_engine_config(inputs).context("build_engine_config failed for cf-app")?;
    config.ledger_chain_enabled = cli.ledger_chain;
    if let Some(cadence) = cli.delta_baseline_cadence_ticks {
        config.delta_baseline_cadence_ticks = cadence;
    }
    Ok(config)
}

pub(crate) fn run_headless(mut config: M0EngineConfig) -> Result<()> {
    config.paced = config.duration_ticks > 0;
    let outcome = run_m0_inline(config).context("inline M0 run failed")?;
    if let Some(bundle) = &outcome.bundle_dir {
        tracing::info!(target: "cf::app", run_id = %outcome.run_id, ticks = outcome.ticks_run, wall_seconds = outcome.wall_seconds, bundle = %bundle.display(), "M0 run bundle written");
    } else {
        tracing::info!(target: "cf::app", run_id = %outcome.run_id, ticks = outcome.ticks_run, wall_seconds = outcome.wall_seconds, "M0 run finished without --write-run-bundle");
    }
    Ok(())
}

pub(crate) fn drain_pending_bundle(engine: &Arc<M0Engine>) {
    if !engine.pending_runbundle() {
        return;
    }
    let ended = WallClock.now_utc();
    match engine.write_run_bundle(ended, 0) {
        Ok(_) => tracing::info!(target: "cf::app", "runbundle.write delivered"),
        Err(err) => tracing::error!(target: "cf::app", error = %err, "runbundle.write failed"),
    }
    engine.clear_pending_runbundle();
}

pub(crate) fn finalize_engine(engine: Arc<M0Engine>, write_bundle: bool) -> Result<()> {
    drain_pending_bundle(&engine);
    engine.record_run_finished(0);
    if write_bundle {
        let ended = WallClock.now_utc();
        let bundle_dir = engine
            .write_run_bundle(ended, 0)
            .context("final write_run_bundle failed")?;
        tracing::info!(target: "cf::app", run_id = %engine.run_id(), bundle = %bundle_dir.display(), "M0 run bundle written on exit");
    } else {
        tracing::info!(target: "cf::app", run_id = %engine.run_id(), ticks = engine.current_tick().0, "M0 Bevy run exited without --write-run-bundle");
    }
    Ok(())
}

pub(crate) fn start_control_server(engine: Arc<M0Engine>, port: u16) -> Result<ControlRuntime> {
    let runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("cf-control")
            .build()
            .context("failed to build tokio runtime for control api")?,
    );
    let bind: SocketAddr = format!("127.0.0.1:{port}")
        .parse()
        .context("control bind not parseable")?;
    let max_hz = engine.config().tick_rate_hz.saturating_mul(2).max(60);
    let server_cfg = ControlServerConfig {
        bind,
        max_observe_hz: max_hz,
        ..Default::default()
    };
    let server = ControlServer::new(server_cfg);
    let (listener, bound) = runtime
        .block_on(server.bind())
        .context("failed to bind control listener")?;
    tracing::info!(target: "cf::app", bind = %bound.bind, "control server bound");
    let (shutdown_tx, shutdown_rx) = cf_control::server::shutdown_signal();
    let handle = runtime
        .spawn(async move { ControlServer::serve_listener_with_shutdown(listener, engine, max_hz, shutdown_rx).await });
    Ok(ControlRuntime {
        _runtime: runtime,
        bound_addr: bound.bind,
        server_handle: Mutex::new(Some(handle)),
        shutdown_tx,
    })
}

pub(crate) fn write_control_port_file(path: Option<&Path>, bound_addr: SocketAddr) -> Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create control port file dir {}", parent.display()))?;
    }
    std::fs::write(path, format!("{}\n", bound_addr.port()))
        .with_context(|| format!("write control port file {}", path.display()))?;
    Ok(())
}

pub(crate) fn compute_duration(ticks: Option<u64>, run_seconds: Option<f32>, tick_rate_hz: u32) -> u64 {
    if let Some(t) = ticks {
        return t;
    }
    if let Some(sec) = run_seconds {
        return (sec * tick_rate_hz as f32).max(1.0) as u64;
    }
    0
}

pub(crate) fn locate_scenario(scenario_id: &str) -> Result<PathBuf> {
    cf_control::runtime::locate_scenario(scenario_id)
        .with_context(|| format!("scenario lookup failed for {scenario_id}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use cf_control::engine::M0EngineConfig;
    use cf_control::EngineHandle;
    use clap::Parser;
    use std::time::Instant;

    static APP_TEST_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn write_test_scenario() -> PathBuf {
        let mut p = std::env::temp_dir();
        let seq = APP_TEST_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        p.push(format!("cf_app_pacing_{}_{}.ron", std::process::id(), seq));
        std::fs::write(
            &p,
            r#"(
  schema_version: 1,
  id: "m0_blank",
  display_name: "M0 Blank Scene",
  description: "Empty scene for pacing test.",
  seed: 42,
  duration_ticks: Some(60),
  region: (anchor: (0.0, 0.0), width: 1280.0, height: 720.0),
  gravity: -980.0,
  teams: [],
  actors: [],
  objectives: [],
  director: None,
  capabilities: (
    debug: false,
    control_api: true,
    save_load: false,
  ),
  save_fields: [],
  expected_tests: ["M0-SMOKE-01"],
  notes: "",
)"#,
        )
        .unwrap();
        p
    }

    #[test]
    fn run_paced_loop_holds_wall_clock_cadence() {
        let scenario_path = write_test_scenario();
        let scenario = cf_control::scenario::Scenario::load_from_file(&scenario_path).unwrap();
        let mut config = M0EngineConfig::for_loaded_scenario(&scenario, scenario_path);
        config.tick_rate_hz = 60;
        config.duration_ticks = 60;
        let engine = Arc::new(M0Engine::new(config.clone()));
        engine.record_run_started();
        let started = Instant::now();
        let _ = run_paced_loop(&engine, config.duration_ticks, config.tick_rate_hz);
        let elapsed = started.elapsed().as_secs_f64();
        engine.record_run_finished(0);
        assert!(
            elapsed >= 0.85,
            "60 ticks @ 60 Hz must take ~1.0 s wall (≥0.85 s), got {elapsed:.3}s. The pre-fix `.min(2ms)` cap accelerated the sim ~5×."
        );
        assert!(
            elapsed <= 1.5,
            "60 ticks @ 60 Hz should finish in well under 1.5 s, got {elapsed:.3}s"
        );
        assert_eq!(engine.current_tick().0, 60);
    }

    #[test]
    fn duration_is_ticks_first_then_seconds() {
        assert_eq!(compute_duration(Some(120), Some(2.0), 60), 120);
        assert_eq!(compute_duration(None, Some(2.0), 60), 120);
        assert_eq!(compute_duration(None, Some(2.0), 120), 240);
        assert_eq!(compute_duration(None, None, 60), 0);
    }

    #[test]
    fn write_control_port_file_records_bound_port() {
        let mut path = std::env::temp_dir();
        path.push(format!("cf_app_control_port_{}_test.txt", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let addr: SocketAddr = "127.0.0.1:43210".parse().unwrap();

        write_control_port_file(Some(&path), addr).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text, "43210\n");
        let _ = std::fs::remove_file(&path);
    }

    /// Regression: headless-smoke + capture-grid must reject at startup, not
    /// silently consume --capture-grid and produce zero PNGs.
    #[test]
    fn rejects_capture_grid_combined_with_headless_smoke() {
        let cli = Cli::try_parse_from(["cf-app", "--scenario", "m0_blank", "--headless-smoke", "--capture-grid"])
            .expect("CLI parse must succeed; the conflict is enforced post-parse");
        let err =
            reject_capture_grid_with_headless_smoke(&cli).expect_err("must reject --headless-smoke + --capture-grid");
        let msg = err.to_string();
        assert!(
            msg.contains("--capture-grid is incompatible with --headless-smoke"),
            "rejection message must explain the conflict, got: {msg}"
        );
    }

    #[test]
    fn allows_capture_grid_without_headless_smoke() {
        let cli = Cli::try_parse_from(["cf-app", "--scenario", "m0_blank", "--capture-grid"])
            .expect("CLI parse must succeed");
        reject_capture_grid_with_headless_smoke(&cli).expect("--capture-grid alone (windowed path) must be allowed");
    }

    #[test]
    fn allows_headless_smoke_without_capture_grid() {
        let cli = Cli::try_parse_from(["cf-app", "--scenario", "m0_blank", "--headless-smoke"])
            .expect("CLI parse must succeed");
        reject_capture_grid_with_headless_smoke(&cli).expect("--headless-smoke alone must be allowed");
    }

    #[test]
    fn final_write_replaces_mid_run_bundle_with_run_finished_evidence() {
        let scenario_path = write_test_scenario();
        let scenario = cf_control::scenario::Scenario::load_from_file(&scenario_path).unwrap();
        let mut config = M0EngineConfig::for_loaded_scenario(&scenario, scenario_path);
        let mut root = std::env::temp_dir();
        let seq = APP_TEST_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        root.push(format!("cf_app_final_bundle_{}_{}", std::process::id(), seq));
        config.run_bundle_root = root.clone();
        config.write_run_bundle = true;
        config.duration_ticks = 6;
        let engine = Arc::new(M0Engine::new(config));
        engine.record_run_started();
        for _ in 0..6 {
            engine.drive_tick();
        }
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result =
            rt.block_on(engine.dispatch(cf_control::server::ControlCommand::RunBundleWrite { id_override: None }));
        assert_eq!(result.status, cf_control::state::ControlEnvelopeStatus::Accepted);
        let mut bundle_written = false;
        flush_pending_runbundle(&engine, &mut bundle_written);
        assert!(bundle_written, "mid-run runbundle.write should write an initial bundle");
        engine.record_run_finished(0);
        let bundle = engine.write_run_bundle(WallClock.now_utc(), 0).unwrap();
        let events = std::fs::read_to_string(bundle.join("events.jsonl")).unwrap();
        assert!(
            events.contains("\"event_type\":\"run_finished\""),
            "final --write-run-bundle evidence must include system.run_finished even if a mid-run runbundle.write already wrote the bundle"
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
