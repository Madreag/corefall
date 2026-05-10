use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use bevy::{
    app::AppExit,
    input::keyboard::KeyCode,
    log::LogPlugin,
    prelude::*,
    window::{PresentMode, WindowCloseRequested, WindowResolution},
};
use clap::Parser;

use cf_actor::IntentSource;
use cf_capture::{
    write_capture_manifest_from_handle, CaptureClock, CaptureConfig, CaptureKeyframeRequested, CaptureMode,
    CaptureStateHandle, CaptureSystems, CfCapturePlugin,
};
use cf_control::{
    engine::{run_m0_inline, M0Engine, M0EngineConfig},
    runtime::{build_engine_config, resolve_run_bundle_root, ConfigInputs},
    server::{ControlCommand, ControlServer, ControlServerConfig},
    EngineHandle, Settings,
};
use cf_render_2d::{ActorRenderState, ActorSpritePlugin, BreachRender, CfRenderPlugin, ExtractionRender};
use cf_replay::diagnostics;
use cf_sim_core::WallClock;
use cf_ui::{HudBreach, HudEnemy, HudMission, HudRifle, HudState, StatusStripPlugin};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, clap::ValueEnum)]
enum Captions {
    On,
    Off,
}

impl Captions {
    fn as_bool(&self) -> bool {
        matches!(self, Captions::On)
    }
}

#[derive(Debug, Parser, Clone)]
#[command(
    name = "cf-app",
    about = "Corefall native app shell. Bevy app + cf-render-2d clear-screen + fixed-tick sim + cf-control loopback API."
)]
struct Cli {
    #[arg(long)]
    scenario: String,
    #[arg(long)]
    seed: Option<u64>,
    #[arg(long)]
    run_seconds: Option<f32>,
    #[arg(long)]
    ticks: Option<u64>,
    #[arg(long, default_value_t = 60)]
    tick_rate_hz: u32,
    #[arg(long)]
    write_run_bundle: bool,
    #[arg(long)]
    run_bundle_dir: Option<PathBuf>,
    #[arg(long)]
    control_api: bool,
    #[arg(long, default_value_t = 17890u16)]
    control_port: u16,
    #[arg(long)]
    control_uds: Option<PathBuf>,
    /// Skip window creation; runs the sim loop only. Useful for CI/scripted smoke.
    #[arg(long)]
    headless_smoke: bool,
    #[arg(long, value_delimiter = ',')]
    debug_capabilities: Vec<String>,
    #[arg(long, default_value_t = 1.0)]
    ui_scale: f32,
    #[arg(long)]
    high_contrast: bool,
    #[arg(long, value_enum, default_value_t = Captions::On)]
    captions: Captions,
    #[arg(long)]
    reduced_motion: bool,
    #[arg(long)]
    reduced_shake: bool,
    #[arg(long)]
    reduced_flash: bool,
    /// **DEBUG-ONLY**: spawn a sub-thread that panics at the configured tick. Used to
    /// capture `system.panic` evidence in a real run bundle (M0-008 / M0.2-F5).
    /// Production runs should never set this.
    #[arg(long)]
    debug_inject_panic_at_tick: Option<u64>,
    /// T-CAPTURE: enable cf-capture frame readback. Defaults to off; pass with no value to
    /// turn on the windowed swapchain capture path at the default 10 Hz cadence.
    #[arg(long)]
    capture_grid: bool,
    /// T-CAPTURE baseline cadence. 10 Hz default = capture every 6 ticks at 60 Hz tick.
    /// Lower values reduce disk + LLM-input pressure; higher values increase motion fidelity.
    #[arg(long, default_value_t = 10.0)]
    capture_frames_hz: f32,
    /// T-CAPTURE: when present, suppress event-triggered keyframes (mission_*, terrain_carved,
    /// projectile_hit, actor_status_changed, weapon_fired, ai.state_changed, system.panic).
    /// Default is keyframes ON.
    #[arg(long)]
    no_capture_events: bool,
    /// T-CAPTURE: switch to offscreen RenderTarget::Image readback (true headless mode without
    /// an OS window). Currently scope-limited; the flag is accepted but the actual offscreen
    /// path is logged-only until the BP2 closure pass lands the wgpu readback wiring.
    /// Use windowed-hidden mode (default) for now.
    #[arg(long)]
    headless_capture: bool,
}

#[derive(Debug, Clone)]
struct CaptureOptions {
    enabled: bool,
    frames_hz: f32,
    event_keyframes: bool,
    headless: bool,
}

impl CaptureOptions {
    fn from_cli(cli: &Cli) -> Self {
        Self {
            enabled: cli.capture_grid,
            frames_hz: cli.capture_frames_hz,
            event_keyframes: !cli.no_capture_events,
            headless: cli.headless_capture,
        }
    }

    fn build_config(&self, output_dir: PathBuf, runtime_tick_rate_hz: u32) -> CaptureConfig {
        CaptureConfig {
            enabled: self.enabled,
            frames_hz: self.frames_hz,
            event_keyframes: self.event_keyframes,
            output_dir,
            thumbnail_w: cf_capture::DEFAULT_THUMBNAIL_W,
            thumbnail_h: cf_capture::DEFAULT_THUMBNAIL_H,
            runtime_tick_rate_hz,
            mode: if self.headless {
                CaptureMode::OffscreenImage
            } else {
                CaptureMode::Windowed
            },
        }
    }
}

fn main() -> Result<()> {
    diagnostics::init("cf::app");
    let cli = Cli::parse();
    let scenario_path = locate_scenario(&cli.scenario)?;
    let config = build_config(&cli, scenario_path)?;
    let capture_opts = CaptureOptions::from_cli(&cli);
    reject_capture_grid_with_headless_smoke(&cli)?;
    tracing::info!(target: "cf::app", scenario = %cli.scenario, headless_smoke = cli.headless_smoke, control_api = cli.control_api, tick_rate_hz = cli.tick_rate_hz, capture_grid = cli.capture_grid, "cf-app M0 starting");

    match (cli.headless_smoke, cli.control_api) {
        (true, true) => run_headless_server(config, cli.control_port, cli.control_uds.clone()),
        (true, false) => run_headless(config),
        (false, _) => run_bevy(
            config,
            cli.control_api,
            cli.control_port,
            cli.control_uds.clone(),
            capture_opts,
        ),
    }
}

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
fn reject_capture_grid_with_headless_smoke(cli: &Cli) -> Result<()> {
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
fn run_headless_server(config: M0EngineConfig, control_port: u16, _uds: Option<PathBuf>) -> Result<()> {
    let engine = Arc::new(M0Engine::new(config.clone()));
    engine.record_run_started();
    engine.record_setting_snapshot();

    let _control_rt = start_control_server(engine.clone(), control_port)?;
    let bundle_written = run_paced_loop(&engine, config.duration_ticks, config.tick_rate_hz);

    // Final bundle drain BEFORE finalize so `system.shutdown {write_run_bundle: true}` is honored.
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
        // The outer `if` already gated on `config.write_run_bundle`, so the
        // remaining branch is unconditionally `!config.write_run_bundle`.
        // Issue #23: removed redundant `else if !config.write_run_bundle`.
        tracing::info!(target: "cf::app", run_id = %engine.run_id(), ticks = engine.current_tick().0, "M0 headless+control-api exited without --write-run-bundle");
    }
    Ok(())
}

/// Pace `engine` against the wall clock at `tick_rate_hz`, exiting when the engine reports
/// shutdown OR the configured `target_ticks` budget is hit (`0` = run until shutdown).
/// Returns `true` if any pending runbundle was written during the loop.
///
/// Extracted for direct unit testing (see `run_paced_loop_holds_wall_clock_cadence`).
fn run_paced_loop(engine: &Arc<M0Engine>, target_ticks: u64, tick_rate_hz: u32) -> bool {
    let tick_dt = std::time::Duration::from_nanos(1_000_000_000 / u64::from(tick_rate_hz.max(1)));
    let started = engine.started_instant();
    // SAFETY (issue #24): `next_tick_at += tick_dt` would theoretically
    // overflow `Instant`'s internal representation after ~584 million years
    // of continuous operation at 60 Hz (u64 nanoseconds since some
    // arbitrary monotonic origin). This is acceptable: no realistic game
    // session will run beyond a few hours; the universe will not last that
    // long. Using `checked_add` here would add a hot-path branch on every
    // tick to handle a case that cannot occur in any deployment scenario.
    let mut next_tick_at = started + tick_dt;
    let mut bundle_written = false;
    // Shutdown polling chunk so the loop can respond within ~5 ms even at low tick rates.
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
        // Wait until next_tick_at, polling shutdown every `poll_chunk`. Crucially, we sleep the
        // FULL remaining tick_dt, not a 2 ms cap that would accelerate the sim.
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

fn flush_pending_runbundle(engine: &Arc<M0Engine>, bundle_written: &mut bool) {
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
fn wait_for_capture_pngs_flushed(
    handle: &cf_capture::CaptureStateHandle,
    captures_dir: &Path,
    timeout: std::time::Duration,
) {
    let started = std::time::Instant::now();
    let poll_interval = std::time::Duration::from_millis(50);
    // Mutex poisoning warning is logged at most once per call. Without this
    // latch the warning would emit on every 50 ms poll iteration up to the
    // 5 s timeout — 100 warnings for a single underlying condition.
    // Addresses PR #26 review Devin Info finding (capture flush log spam).
    let mut poison_warned = false;
    loop {
        // Snapshot the expected PNG paths under the lock, then release it
        // so the capture systems (if any are still flushing into the queue
        // from the very last frame) aren't blocked.
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

fn build_config(cli: &Cli, scenario_path: PathBuf) -> Result<M0EngineConfig> {
    let run_mode = if cli.headless_smoke {
        "headless-smoke".to_string()
    } else if cli.control_api {
        "bevy-control-driven".to_string()
    } else {
        "bevy-interactive".to_string()
    };
    let cli_duration = compute_duration(cli.ticks, cli.run_seconds, cli.tick_rate_hz);
    let inputs = ConfigInputs {
        scenario_id: cli.scenario.clone(),
        scenario_path,
        run_mode,
        run_bundle_root: resolve_run_bundle_root(cli.run_bundle_dir.clone()),
        write_run_bundle: cli.write_run_bundle,
        control_api_enabled: cli.control_api,
        debug_capabilities: cli.debug_capabilities.clone(),
        tick_rate_hz: cli.tick_rate_hz,
        // cf-app paces only when an explicit budget was set on the CLI; an unbounded
        // headless+control-api run sleeps via `run_paced_loop` itself.
        paced: cli_duration > 0,
        settings: Settings {
            ui_scale: cli.ui_scale,
            high_contrast: cli.high_contrast,
            captions: cli.captions.as_bool(),
            reduced_motion: cli.reduced_motion,
            reduced_shake: cli.reduced_shake,
            reduced_flash: cli.reduced_flash,
        },
        seed_override: cli.seed,
        duration_ticks_override: if cli_duration > 0 { Some(cli_duration) } else { None },
        debug_inject_panic_at_tick: cli.debug_inject_panic_at_tick,
    };
    build_engine_config(inputs).context("build_engine_config failed for cf-app")
}

fn run_headless(mut config: M0EngineConfig) -> Result<()> {
    // Headless smoke: pace at the configured Hz when --run-seconds is requested.
    config.paced = config.duration_ticks > 0;
    let outcome = run_m0_inline(config).context("inline M0 run failed")?;
    if let Some(bundle) = &outcome.bundle_dir {
        tracing::info!(target: "cf::app", run_id = %outcome.run_id, ticks = outcome.ticks_run, wall_seconds = outcome.wall_seconds, bundle = %bundle.display(), "M0 run bundle written");
    } else {
        tracing::info!(target: "cf::app", run_id = %outcome.run_id, ticks = outcome.ticks_run, wall_seconds = outcome.wall_seconds, "M0 run finished without --write-run-bundle");
    }
    Ok(())
}

#[derive(Resource)]
struct EngineHolder(Arc<M0Engine>);

#[derive(Resource)]
struct AppRuntime {
    duration_ticks: u64,
    last_announced_tick: u64,
}

#[derive(Resource)]
struct ControlRuntime {
    _runtime: Arc<tokio::runtime::Runtime>,
    bound_addr: SocketAddr,
    server_handle: Mutex<Option<tokio::task::JoinHandle<std::io::Result<()>>>>,
    // Sticky shutdown signal sent into `serve_listener_with_shutdown` so
    // that ControlRuntime::drop can cleanly stop the accept loop + every
    // per-connection observation loop instead of relying on
    // `JoinHandle::abort()` (which leaves in-flight WebSocket connections
    // dangling). Wired in 2026-05-09 as part of the PR #26 review fix
    // (Devin Info: serve_listener creates a dead Notify).
    shutdown_tx: cf_control::server::ShutdownSignal,
}

fn run_bevy(
    config: M0EngineConfig,
    control_api: bool,
    control_port: u16,
    _uds: Option<PathBuf>,
    capture_opts: CaptureOptions,
) -> Result<()> {
    let engine = Arc::new(M0Engine::new(config.clone()));
    engine.record_run_started();
    engine.record_setting_snapshot();

    let control_rt = if control_api {
        Some(start_control_server(engine.clone(), control_port)?)
    } else {
        None
    };

    let captures_dir = engine.run_bundle_dir().join("captures");
    let capture_config = capture_opts.build_config(captures_dir.clone(), config.tick_rate_hz);
    let capture_enabled = capture_config.enabled;
    if capture_enabled && matches!(capture_config.mode, CaptureMode::OffscreenImage) {
        tracing::warn!(
            target: "cf::capture",
            "headless-capture is scope-limited (T-CAPTURE-OFFSCREEN); falling back to baseline log-only output. \
             Use windowed-hidden mode (default) for visual proof until the offscreen RenderTarget readback ships."
        );
    }
    if capture_enabled {
        if let Err(e) = cf_capture::ensure_capture_dir(&captures_dir) {
            tracing::warn!(target: "cf::capture", "failed to create captures dir {}: {e}", captures_dir.display());
        }
    }

    let mut app = App::new();
    let title = format!("Corefall — BP2 Terrain & Replay (v{APP_VERSION})");
    let plugins = DefaultPlugins
        .set(WindowPlugin {
            primary_window: Some(Window {
                title,
                resolution: WindowResolution::new(1280, 720),
                present_mode: PresentMode::AutoVsync,
                resizable: true,
                ..default()
            }),
            ..default()
        })
        .disable::<LogPlugin>();
    // BP2 capture-grid harness: cf-e2e launches cf-app windowed but the OS
    // may steal focus during the run (especially on macOS where the
    // foreground terminal keeps focus). Bevy's default `WinitSettings`
    // throttles unfocused windows to ReactiveLowPower (~60s/frame), which
    // deadlocks the JSON-RPC server because the schedule barely advances.
    // Pin both focused + unfocused to `Continuous` so the engine ticks
    // regardless of focus state.
    use bevy::winit::{UpdateMode, WinitSettings};
    app.insert_resource(WinitSettings {
        focused_mode: UpdateMode::Continuous,
        unfocused_mode: UpdateMode::Continuous,
    });

    app.add_plugins(plugins)
        .add_plugins(CfRenderPlugin::default())
        .add_plugins(ActorSpritePlugin)
        .add_plugins(StatusStripPlugin);
    let capture_handle = CaptureStateHandle::default();
    app.add_plugins(CfCapturePlugin {
        config: capture_config.clone(),
        state_handle: capture_handle.clone(),
    });
    app.insert_resource(CaptureRecorderCursor::default());
    app.insert_resource(Time::<Fixed>::from_hz(f64::from(config.tick_rate_hz)));
    app.insert_resource(EngineHolder(engine.clone()));
    app.insert_resource(AppRuntime {
        duration_ticks: config.duration_ticks,
        last_announced_tick: 0,
    });
    if let Some(rt) = control_rt {
        app.insert_resource(rt);
    }

    app.add_systems(FixedUpdate, drive_engine_tick).add_systems(
        Update,
        (
            esc_or_close_to_exit,
            check_completion,
            log_tick_progress,
            ingest_player_input,
            sync_actor_state_to_render,
            sync_engine_tick_to_capture_clock,
            pump_recorder_events_into_capture_keyframes,
        )
            .chain(),
    );
    // Ensure cf-capture's systems observe the freshest `CaptureClock` tick and
    // any `CaptureKeyframeRequested` messages written this frame, instead of
    // racing the unordered scheduler against the chain above.
    app.configure_sets(
        Update,
        CaptureSystems
            .after(sync_engine_tick_to_capture_clock)
            .after(pump_recorder_events_into_capture_keyframes),
    );

    app.run();

    if capture_enabled {
        // Bevy's `Screenshot::observe(save_to_disk)` is asynchronous: when
        // `app.run()` returns, the observer queue may still hold frames whose
        // PNGs haven't been flushed to disk yet. Issue #17: replace the
        // earlier fixed 500 ms sleep with active filesystem polling — wait
        // until every PNG path the capture log knows about actually exists on
        // disk, OR a generous timeout fires (5 s; well above any plausible
        // disk-flush latency even on slow hardware or high-cadence capture).
        wait_for_capture_pngs_flushed(&capture_handle, &captures_dir, std::time::Duration::from_secs(5));
        match write_capture_manifest_from_handle(&capture_config, &capture_handle) {
            Ok(path) => tracing::info!(
                target: "cf::capture",
                "capture manifest written to {}",
                path.display()
            ),
            Err(e) => tracing::warn!(
                target: "cf::capture",
                "failed to write capture manifest: {e}"
            ),
        }
    }

    // After the Bevy app exits, finalize the run bundle.
    finalize_engine(engine, config.write_run_bundle)?;
    Ok(())
}

fn sync_engine_tick_to_capture_clock(holder: Res<EngineHolder>, mut clock: ResMut<CaptureClock>) {
    clock.current_tick = holder.0.current_tick().0;
}

#[derive(Resource, Default)]
struct CaptureRecorderCursor(usize);

/// (category, event_type) pairs that trigger a capture keyframe. Matching the
/// full `category.event_type` shape (rather than just `event_type`) keeps the
/// keyframe set aligned with the documented contract — `ai.state_changed` and
/// `system.panic` are intentionally narrow, and a future `control.state_changed`
/// or other-category `panic` must not silently inflate the summary grid.
const CAPTURE_KEYFRAME_EVENT_TYPES: &[(&str, &str)] = &[
    ("mission", "objective_started"),
    ("mission", "objective_completed"),
    ("mission", "objective_failed"),
    ("mission", "mission_resolved"),
    ("terrain", "terrain_carved"),
    ("terrain", "tool_refused"),
    ("combat", "projectile_hit"),
    ("actor", "actor_status_changed"),
    ("equipment", "weapon_fired"),
    ("ai", "state_changed"),
    ("system", "panic"),
];

fn pump_recorder_events_into_capture_keyframes(
    holder: Res<EngineHolder>,
    config: Res<CaptureConfig>,
    mut cursor: ResMut<CaptureRecorderCursor>,
    mut writer: MessageWriter<CaptureKeyframeRequested>,
) {
    if !config.enabled || !config.event_keyframes {
        return;
    }
    let recorder = holder.0.recorder();
    let new_events = recorder.events_since(cursor.0);
    cursor.0 += new_events.len();
    for ev in new_events {
        if CAPTURE_KEYFRAME_EVENT_TYPES
            .iter()
            .any(|(cat, ty)| ev.category == *cat && ev.event_type == *ty)
        {
            let label = format!("{}::{}", ev.category, ev.event_type);
            writer.write(CaptureKeyframeRequested {
                tick: ev.tick,
                event_type: format!("{}.{}", ev.category, ev.event_type),
                label,
            });
        }
    }
}

fn drive_engine_tick(holder: Res<EngineHolder>, mut runtime: ResMut<AppRuntime>) {
    if holder.0.shutdown_requested() {
        return;
    }
    if runtime.duration_ticks > 0 && holder.0.current_tick().0 >= runtime.duration_ticks {
        return;
    }
    let _ = holder.0.drive_tick();
    drain_pending_bundle(&holder.0);
    let cur = holder.0.current_tick().0;
    if runtime.duration_ticks > 0 && cur >= runtime.duration_ticks {
        runtime.last_announced_tick = cur;
    }
}

fn check_completion(holder: Res<EngineHolder>, runtime: Res<AppRuntime>, mut events: MessageWriter<AppExit>) {
    if holder.0.shutdown_requested() {
        // Drain any pending runbundle.write before exit so a `system.shutdown` arriving
        // after target_ticks still produces evidence. (Acceptance fix M3.)
        drain_pending_bundle(&holder.0);
        events.write(AppExit::Success);
        return;
    }
    if runtime.duration_ticks > 0 && holder.0.current_tick().0 >= runtime.duration_ticks {
        // Same drain on the natural-exit path: a runbundle.write arriving after the budget
        // hit must still be honored.
        drain_pending_bundle(&holder.0);
        events.write(AppExit::Success);
    }
}

fn log_tick_progress(holder: Res<EngineHolder>, mut runtime: ResMut<AppRuntime>) {
    let cur = holder.0.current_tick().0;
    if cur >= runtime.last_announced_tick + 60 {
        tracing::debug!(target: "cf::app", tick = cur, "sim progressing");
        runtime.last_announced_tick = cur;
    }
}

/// Sample the keyboard each frame and fold it into the engine's pending
/// `ControlIntent` so human input runs through exactly the same path as
/// `cfctl act.player.*` commands. Movement is continuous (held keys); jump /
/// fire / reload / select are edge-triggered.
fn ingest_player_input(
    holder: Res<EngineHolder>,
    keys: Res<ButtonInput<KeyCode>>,
    rt: Option<Res<ControlRuntime>>,
    mut last_move_x: Local<f32>,
    mut last_aim: Local<(f32, f32)>,
    mut last_intent_epoch: Local<u64>,
) {
    let _ = rt; // Reserved; ControlRuntime presence does not gate human input.
    if !holder.0.config().has_actor_world {
        return;
    }
    // WASD letters drive movement; arrow keys drive aim. Decoupling the two
    // axes lets the player strafe (e.g. move left while aiming right), which
    // the previous `aim_x = move_x.signum()` shortcut made impossible. W/S
    // remain on aim_y as alternative bindings to Up/Down for ergonomic reach.
    let move_x = keyboard_axis_pair(&keys, KeyCode::KeyD, KeyCode::KeyA);
    let aim_x = keyboard_axis_pair(&keys, KeyCode::ArrowRight, KeyCode::ArrowLeft);
    let aim_y = keyboard_axis(
        &keys,
        KeyCode::KeyW,
        KeyCode::KeyS,
        KeyCode::ArrowUp,
        KeyCode::ArrowDown,
    );
    // `scenario.reset` (and any future op that zeroes `pending_intent` out
    // from under us) bumps the engine's `intent_epoch`. When that happens we
    // must redispatch any currently-held keys: the engine has forgotten the
    // sticky values, but our edge-detecting locals still hold the pre-reset
    // sample, so without this poke a held movement key would silently drop.
    let engine_epoch = holder.0.intent_epoch();
    let epoch_changed = engine_epoch != *last_intent_epoch;
    if epoch_changed {
        *last_intent_epoch = engine_epoch;
        *last_move_x = 0.0;
        *last_aim = (0.0, 0.0);
    }
    // Only dispatch a move command when the human input actually changes. Idle samples
    // (e.g. no key pressed and last sample was also zero) must NOT clobber sticky
    // `cfctl act.player.move` values, since `ControlIntent.move_x` is continuous and
    // latest-value-wins. Edge-triggered transitions (key press / release / direction
    // change) still fire so releasing a key promptly stops the actor. After an
    // epoch change, force a dispatch when the key is still held so the engine
    // sees the live state.
    let dispatch_move = (move_x - *last_move_x).abs() > f32::EPSILON || (epoch_changed && move_x.abs() > 1e-3);
    // Mirror the move-edge detection for aim: only dispatch when the aim
    // vector actually changes. Aim is a continuous, latest-value-wins field
    // on `ControlIntent`, so re-sending the same vector every frame both
    // wastes a `RwLock` write on the engine and risks clobbering sticky
    // `cfctl act.player.aim` values on idle frames.
    let aim_active = aim_x.abs() > 1e-3 || aim_y.abs() > 1e-3;
    let aim_changed = (aim_x - last_aim.0).abs() > f32::EPSILON || (aim_y - last_aim.1).abs() > f32::EPSILON;
    let dispatch_aim = aim_active && (aim_changed || epoch_changed);
    // Only update the tracker when we actually dispatch. Updating it on every
    // frame would silently desync from the engine state when keys are released
    // (e.g. last_aim resets to (0, 0) without dispatching, then a redundant
    // dispatch fires next time keys are pressed even though the engine still
    // holds the old aim). Same applies to last_move_x.
    if dispatch_move {
        *last_move_x = move_x;
    }
    if dispatch_aim {
        *last_aim = (aim_x, aim_y);
    } else if !aim_active {
        // Aim went inactive (all aim keys released). We deliberately do NOT
        // dispatch a zero-aim command so sticky `cfctl act.player.aim` values
        // are preserved, but we MUST clear the local tracker. Otherwise the
        // next time the player presses the same aim direction, `aim_changed`
        // would compare against the stale non-zero `last_aim` and decide the
        // dispatch is unnecessary, silently dropping the freshly pressed input.
        *last_aim = (0.0, 0.0);
    }
    let block_on = futures_block_on;
    block_on(async {
        if dispatch_move {
            let _ = holder
                .0
                .dispatch(ControlCommand::ActPlayerMove {
                    x: move_x,
                    y: 0.0,
                    source: IntentSource::Human,
                })
                .await;
        }
        if dispatch_aim {
            let _ = holder
                .0
                .dispatch(ControlCommand::ActPlayerAim {
                    x: aim_x,
                    y: aim_y,
                    source: IntentSource::Human,
                })
                .await;
        }
        if keys.just_pressed(KeyCode::Space) {
            let _ = holder
                .0
                .dispatch(ControlCommand::ActPlayerJump {
                    source: IntentSource::Human,
                })
                .await;
        }
        if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::KeyJ) {
            let _ = holder
                .0
                .dispatch(ControlCommand::ActPlayerFire {
                    pressed: true,
                    source: IntentSource::Human,
                })
                .await;
        }
        // Mirror the press with an explicit release so the keyboard bridge
        // honors the `ActPlayerFireParams.pressed` contract: the schema
        // documents `false` as "explicit release for future hold-to-fire
        // weapons." M1's single-press rifle treats fire as an edge that
        // `clear_edges()` resets each tick, so this is a no-op today, but
        // omitting it leaves a latent contract gap that would silently break
        // future hold-to-fire weapons routed through this bridge.
        if keys.just_released(KeyCode::Enter) || keys.just_released(KeyCode::KeyJ) {
            let _ = holder
                .0
                .dispatch(ControlCommand::ActPlayerFire {
                    pressed: false,
                    source: IntentSource::Human,
                })
                .await;
        }
        if keys.just_pressed(KeyCode::KeyR) {
            let _ = holder
                .0
                .dispatch(ControlCommand::ActPlayerReload {
                    source: IntentSource::Human,
                })
                .await;
        }
        if keys.just_pressed(KeyCode::KeyL) {
            let _ = holder
                .0
                .dispatch(ControlCommand::ActPlayerReset {
                    source: IntentSource::Human,
                })
                .await;
        }
        // M1.5: G presses request a dig at the nearest in-range breach strip.
        if keys.just_pressed(KeyCode::KeyG) {
            let _ = holder
                .0
                .dispatch(ControlCommand::ActPlayerDig {
                    target: None,
                    source: IntentSource::Human,
                })
                .await;
        }
        for (slot_key, slot) in [
            (KeyCode::Digit1, 0u32),
            (KeyCode::Digit2, 1u32),
            (KeyCode::Digit3, 2u32),
            (KeyCode::Digit4, 3u32),
        ] {
            if keys.just_pressed(slot_key) {
                let _ = holder
                    .0
                    .dispatch(ControlCommand::ActPlayerSelectItem {
                        slot,
                        source: IntentSource::Human,
                    })
                    .await;
            }
        }
    });
}

fn keyboard_axis(keys: &ButtonInput<KeyCode>, pos_a: KeyCode, neg_a: KeyCode, pos_b: KeyCode, neg_b: KeyCode) -> f32 {
    let pos = keys.pressed(pos_a) || keys.pressed(pos_b);
    let neg = keys.pressed(neg_a) || keys.pressed(neg_b);
    match (pos, neg) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
    }
}

fn keyboard_axis_pair(keys: &ButtonInput<KeyCode>, pos: KeyCode, neg: KeyCode) -> f32 {
    match (keys.pressed(pos), keys.pressed(neg)) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
    }
}

/// Block on a single async dispatch. The control engine is used through async traits
/// even from the synchronous Bevy schedule; the body is small and all work is
/// in-process so blocking is fine.
///
/// Uses a thread-parking waker so that if any future implementation ever returns
/// `Poll::Pending` (for example, a future engine backed by `tokio::sync::RwLock`),
/// the current thread parks until the waker is signalled instead of spinning.
fn futures_block_on<F, T>(future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake};
    use std::thread::{self, Thread};

    struct ThreadWaker(Thread);
    impl Wake for ThreadWaker {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.0.unpark();
        }
    }

    let waker = Arc::new(ThreadWaker(thread::current())).into();
    let mut cx = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(out) => return out,
            Poll::Pending => thread::park(),
        }
    }
}

/// Copy the engine's actor world + rifle state into the Bevy render + HUD
/// resources every frame. The engine is the single source of truth; render +
/// HUD never own authoritative state.
fn sync_actor_state_to_render(
    holder: Res<EngineHolder>,
    mut render_state: ResMut<ActorRenderState>,
    mut hud_state: ResMut<HudState>,
) {
    let snapshot = holder.0.actor_render_snapshot();
    render_state.actors = snapshot.actors.clone();
    render_state.player_actor_id = snapshot.player_actor_id;
    render_state.region_width = holder.0.config().region_width;
    render_state.region_height = holder.0.config().region_height;
    render_state.region_anchor_x = holder.0.config().region_anchor_x;
    render_state.region_anchor_y = holder.0.config().region_anchor_y;
    render_state.floor_y = snapshot.floor_y;

    hud_state.tick = snapshot.tick;
    hud_state.tick_rate_hz = holder.0.config().tick_rate_hz;
    hud_state.player = snapshot
        .player_actor_id
        .and_then(|id| snapshot.actors.iter().find(|a| a.id == id).cloned());
    hud_state.rifle = snapshot.player_rifle.as_ref().map(|r| HudRifle {
        ammo: r.ammo,
        capacity: r.capacity,
        fire_cooldown_ticks: r.fire_cooldown_ticks,
        reload_remaining_ticks: r.reload_remaining_ticks,
        reload_total_ticks: r.reload_total_ticks,
    });

    // M1.5 — propagate mission, enemy, breach, extraction zone to renderer + HUD.
    render_state.breaches = snapshot
        .breaches
        .iter()
        .map(|b| BreachRender {
            id: b.id.clone(),
            bbox_min: b.bbox_min,
            bbox_max: b.bbox_max,
            hp: b.hp,
            max_hp: b.max_hp,
            broken: b.broken,
            refusal_reason: b.refusal_reason.clone(),
        })
        .collect();
    render_state.extraction_zone = snapshot.extraction_zone.as_ref().map(|z| ExtractionRender {
        min: z.min,
        max: z.max,
        completed: z.completed,
    });

    hud_state.mission = snapshot.mission.as_ref().map(|m| HudMission {
        result: m.result.clone(),
        loss_reason: m.loss_reason.clone(),
        elapsed_ticks: m.elapsed_ticks,
        time_limit_ticks: m.time_limit_ticks,
        ticks_remaining: m.ticks_remaining,
        active_objective: m.active_objective.clone(),
        last_event_label: m.last_event_label.clone(),
    });
    hud_state.last_event = snapshot.mission.as_ref().map(|m| m.last_event_label.clone());

    // Pick the first non-controllable actor as the "enemy" for the HUD. The
    // engine is the single source of truth — read state + last_tactic from the
    // matching `EnemyHudView` if one exists (M1.5+), and fall back to neutral
    // labels only when no AI controller is attached (early prototype scenarios).
    hud_state.enemy = snapshot.actors.iter().find(|a| !a.controllable).map(|a| {
        let enemy_view = snapshot.enemies.iter().find(|e| e.actor == a.id);
        HudEnemy {
            state: enemy_view.map(|e| e.state.clone()).unwrap_or_else(|| "—".to_string()),
            last_tactic: enemy_view
                .map(|e| e.last_tactic.clone())
                .unwrap_or_else(|| "—".to_string()),
            hp: a.hp,
            hp_max: a.hp_max,
            status: a.status.clone(),
        }
    });

    // Pick the nearest breach to the player to surface in the HUD.
    if let (Some(player), Some(_)) = (hud_state.player.as_ref(), snapshot.breaches.first()) {
        let px = player.position[0];
        let py = player.position[1];
        // Match `cf_terrain::BreachStrip::distance_to`: distance to the nearest
        // point on the AABB, clamped to zero when the player is inside.
        let aabb_distance = |b: &cf_control::BreachRenderView| -> f32 {
            let dx = (b.bbox_min[0] - px).max(0.0).max(px - b.bbox_max[0]);
            let dy = (b.bbox_min[1] - py).max(0.0).max(py - b.bbox_max[1]);
            ((dx * dx) + (dy * dy)).sqrt()
        };
        let mut best: Option<(&cf_control::BreachRenderView, f32)> = None;
        for b in &snapshot.breaches {
            let d = aabb_distance(b);
            match best {
                None => best = Some((b, d)),
                Some((_, prev)) if d < prev => best = Some((b, d)),
                _ => {}
            }
        }
        if let Some((b, d)) = best {
            // Mirror the engine's dig contract: a strip is in range when the
            // AABB-boundary distance is within the strip's own `dig_range`.
            hud_state.breach = Some(HudBreach {
                id: b.id.clone(),
                material: b.material.clone(),
                hp: b.hp,
                max_hp: b.max_hp,
                broken: b.broken,
                refusal_reason: b.refusal_reason.clone(),
                in_range: d <= b.dig_range,
            });
        } else {
            hud_state.breach = None;
        }
    } else {
        hud_state.breach = None;
    }
}

fn esc_or_close_to_exit(
    keys: Res<ButtonInput<KeyCode>>,
    mut close_events: MessageReader<WindowCloseRequested>,
    mut events: MessageWriter<AppExit>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        tracing::info!(target: "cf::app", "ESC pressed; exiting");
        events.write(AppExit::Success);
    }
    if close_events.read().next().is_some() {
        tracing::info!(target: "cf::app", "window close requested; exiting");
        events.write(AppExit::Success);
    }
}

fn drain_pending_bundle(engine: &Arc<M0Engine>) {
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

fn finalize_engine(engine: Arc<M0Engine>, write_bundle: bool) -> Result<()> {
    // M3: honor any runbundle.write that landed after the loop exited but before finalize.
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

fn start_control_server(engine: Arc<M0Engine>, port: u16) -> Result<ControlRuntime> {
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

impl Drop for ControlRuntime {
    fn drop(&mut self) {
        // Trigger the sticky shutdown signal first so the accept loop +
        // every in-flight connection's observation loop exit cleanly.
        // `JoinHandle::abort()` is the fallback if a task wedges past the
        // shutdown signal.
        cf_control::server::trigger_shutdown(&self.shutdown_tx);
        if let Some(handle) = self.server_handle.lock().ok().and_then(|mut g| g.take()) {
            handle.abort();
        }
        tracing::info!(target: "cf::app", bind = %self.bound_addr, "control server stopped");
    }
}

fn compute_duration(ticks: Option<u64>, run_seconds: Option<f32>, tick_rate_hz: u32) -> u64 {
    if let Some(t) = ticks {
        return t;
    }
    if let Some(sec) = run_seconds {
        return (sec * tick_rate_hz as f32).max(1.0) as u64;
    }
    0
}

fn locate_scenario(scenario_id: &str) -> Result<PathBuf> {
    cf_control::runtime::locate_scenario(scenario_id)
        .with_context(|| format!("scenario lookup failed for {scenario_id}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_control::EngineHandle;
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
        // 60 ticks at 60 Hz must take ~1.0 s (NOT ~120 ms, which is what the pre-fix
        // `.min(2ms)` cap produced). This is the regression test for review finding H1.
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
    fn cli_parses_all_m0_flags_and_tick_rate() {
        let cli = Cli::try_parse_from([
            "cf-app",
            "--scenario",
            "m0_blank",
            "--seed",
            "7",
            "--ticks",
            "60",
            "--tick-rate-hz",
            "120",
            "--write-run-bundle",
            "--run-bundle-dir",
            "/tmp/run",
            "--control-api",
            "--control-port",
            "17890",
            "--headless-smoke",
            "--debug-capabilities",
            "debug",
            "--ui-scale",
            "2.0",
            "--high-contrast",
            "--captions",
            "off",
            "--reduced-motion",
            "--reduced-shake",
            "--reduced-flash",
        ])
        .expect("CLI must accept all M0 flags");
        assert_eq!(cli.scenario, "m0_blank");
        assert_eq!(cli.seed, Some(7));
        assert_eq!(cli.ticks, Some(60));
        assert_eq!(cli.tick_rate_hz, 120);
        assert!(cli.write_run_bundle);
        assert_eq!(cli.run_bundle_dir, Some(PathBuf::from("/tmp/run")));
        assert!(cli.control_api);
        assert_eq!(cli.control_port, 17890);
        assert!(cli.headless_smoke);
        assert_eq!(cli.debug_capabilities, vec!["debug".to_string()]);
        assert!((cli.ui_scale - 2.0).abs() < f32::EPSILON);
        assert!(cli.high_contrast);
        assert!(matches!(cli.captions, Captions::Off));
        assert!(cli.reduced_motion);
        assert!(cli.reduced_shake);
        assert!(cli.reduced_flash);
    }

    #[test]
    fn duration_is_ticks_first_then_seconds() {
        assert_eq!(compute_duration(Some(120), Some(2.0), 60), 120);
        assert_eq!(compute_duration(None, Some(2.0), 60), 120);
        assert_eq!(compute_duration(None, Some(2.0), 120), 240);
        assert_eq!(compute_duration(None, None, 60), 0);
    }

    /// Regression: headless-smoke + capture-grid must reject at startup, not
    /// silently consume --capture-grid and produce zero PNGs. The headless
    /// paths skip Bevy's render world, so there is no swapchain to read back.
    /// Bugbot finding (Medium): "Capture silently dropped in headless mode
    /// without warning."
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
