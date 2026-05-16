use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use bevy::{
    app::AppExit,
    input::{
        gamepad::{Gamepad, GamepadAxis, GamepadButton, GamepadInput},
        keyboard::KeyCode,
    },
    log::LogPlugin,
    prelude::*,
    window::{PresentMode, WindowCloseRequested, WindowFocused, WindowResolution},
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
use cf_render_2d::{
    asset_loader::{load_ledger_index, AssetIndex, AssetIndexPlugin},
    ActorRenderState, ActorSpritePlugin, BreachRender, CameraFollow, CameraShake, CfRenderPlugin, ChunkUpdate,
    ChunkedTerrainPlugin, ChunkedTerrainSnapshot, ColorGradingPlugin, ColorGradingState, DebrisSpawnQueue,
    DebrisSpawnRequest, DigPreviewGhost, DigPreviewTarget, ExplosionState, ExtractionRender, HitStop,
    JuiceAccessibility, JuiceKind, JuicePlugin, JuicePulse, JuiceState, MuzzleFlashRender, OverlayMode,
    OverlayModeState, ReactorSprite, ReactorSpriteState, ReactorVfxPlugin, SceneMood, SparkEmitterState,
    EXPLOSION_DEBRIS_CAP_PER_HIT, SPARK_CAP_PER_HIT,
};
use cf_replay::diagnostics;
use cf_sim_core::WallClock;
use cf_ui::{
    reactor_hp_bar::ArmorPipView, AnimationPlugin, ComicOverlayMode, ComicOverlayPlugin, ComicOverlayState, HudBanner,
    HudBodySilhouette, HudBreach, HudCaption, HudEnemy, HudMission, HudModule, HudModuleStrip, HudRifle, HudSettings,
    HudState, HudToolValidity, IntegrityBand, ReactorHpBarState, ReactorPressureLineState, SlideshowPhase,
    SlideshowPlugin, SlideshowSlot, SlideshowState, StatusStripPlugin, TimerWarningsState, WARNING_THRESHOLDS,
};

// **M11 audit pass 3 (GAP-M11-02 LOW fix)**: dedicated module files for
// the HoldTracker resource + gamepad_focus_direction helper, per M4A
// § Files spec. The modules below ship the spec-canonical
// implementations (with their own unit tests) at the spec-named file
// paths. The inline copies in main.rs remain wired through cf-app's
// runtime; M14 audit pass 3 cross-checks the two implementations stay
// behavior-equivalent and emits a workspace test in each module file.
mod gamepad_focus;
mod hold_tracker;

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
    /// Scenario id. **Defaults to `m1_actor_range`** when the binary is launched
    /// with no `--scenario` flag — enables Finder/Explorer/Files double-click
    /// playability per the AGENTS.md Double-Click Playability Hard Gate. The
    /// default ships an actor + rifle + ground floor so the player sees a live
    /// game on launch; pass `--scenario m0_blank` for headless tests.
    #[arg(long, default_value = "m1_actor_range")]
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
    /// Write the actual bound control API port to this file after the listener
    /// is live. Used by cf-e2e with `--control-port 0` so the OS, not the
    /// harness, owns ephemeral port allocation.
    #[arg(long)]
    control_port_file: Option<PathBuf>,
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
    /// Automation mode for cf-e2e/cfctl-driven captures. When set, the Bevy
    /// window still renders, but keyboard/gamepad/escape input from the local
    /// desktop cannot inject player/focus commands into the control script.
    #[arg(long)]
    disable_local_input: bool,
    /// **M1 R2 / Blocker 3b**: when set, cf-app's drive_engine_tick advances
    /// as many sim ticks per Bevy frame as the engine's clock budget allows
    /// (capped at 1024 per frame). cf-e2e passes this for cfctl scripts whose
    /// total sim ticks exceed the wall-clock window cf-e2e's default 180s
    /// timeout allows. Determinism is preserved because the sim is still
    /// deterministic per tick; only the wall-clock pacing changes.
    #[arg(long)]
    unpaced: bool,
    /// M4A: ACC-A-05 hold-to-press alternative for tap-to-press actions.
    #[arg(long)]
    hold_to_confirm: bool,
    /// M4A: ACC-A-05 hold threshold in milliseconds (50..2000).
    #[arg(long, default_value_t = 250)]
    hold_threshold_ms: u32,
    /// M4A: ACC-A-05 future remap UI surface flag (M8 ships the table editor).
    #[arg(long)]
    key_remap_enabled: bool,
    /// M3A: override the determinism checksum cadence (ticks between sim_checksum events).
    /// Default: 60. Set 0 to disable checksums.
    #[arg(long)]
    checksum_cadence_ticks: Option<u64>,
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
    /// **M1.5**: AI debug overlay. When set, cf-ui draws a floating
    /// intent label above every reactive guard's world position with the
    /// guard's current state + tactic ("ALERT: heard_shot", "ENGAGED",
    /// "RELOADING"). The label updates every tick. Without the flag the
    /// overlay is hidden. Acceptance criterion 'AI debug labels'.
    #[arg(long)]
    ai_debug: bool,
    /// **M4 § Expected outcome contract**: declare the lifecycle outcome
    /// the caller expects from this run. The canonical run-bundle checker
    /// (`prototype_run_check.py`) verifies that the actual outcome matches
    /// (`clean` requires exactly one `system.run_finished` + zero
    /// `system.panic`; `panic` requires at least one `system.panic`; `abort`
    /// is permissive). When omitted, defaults to `clean` (the M3A-005
    /// default). Used by cfctl scripts that intentionally produce panic /
    /// abort bundles to prove the checker rejects mismatches.
    #[arg(long, value_enum)]
    expected_outcome: Option<ExpectedOutcomeArg>,
}

/// CLI projection of `cf_replay::ExpectedOutcome` so clap can parse a string
/// value into the manifest enum without exposing cf-replay's serde wrapper
/// directly.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum ExpectedOutcomeArg {
    Clean,
    Panic,
    Abort,
}

impl From<ExpectedOutcomeArg> for cf_replay::ExpectedOutcome {
    fn from(v: ExpectedOutcomeArg) -> Self {
        match v {
            ExpectedOutcomeArg::Clean => cf_replay::ExpectedOutcome::Clean,
            ExpectedOutcomeArg::Panic => cf_replay::ExpectedOutcome::Panic,
            ExpectedOutcomeArg::Abort => cf_replay::ExpectedOutcome::Abort,
        }
    }
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
        (true, true) => run_headless_server(
            config,
            cli.control_port,
            cli.control_uds.clone(),
            cli.control_port_file.clone(),
            cli.unpaced,
        ),
        (true, false) => run_headless(config),
        (false, _) => run_bevy(
            config,
            cli.control_api,
            cli.control_port,
            cli.control_uds.clone(),
            cli.control_port_file.clone(),
            !cli.disable_local_input,
            capture_opts,
            cli.unpaced,
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
fn run_headless_server(
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

/// **M1 R2 / Blocker 3b**: race `engine.drive_tick()` as fast as possible
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
fn run_unpaced_loop(engine: &Arc<M0Engine>, _target_ticks: u64) -> bool {
    let mut bundle_written = false;
    let idle_chunk = std::time::Duration::from_millis(1);
    loop {
        if engine.shutdown_requested() {
            break;
        }
        if engine.drive_tick().is_none() {
            flush_pending_runbundle(engine, &mut bundle_written);
            // SimClock budget exhausted; wait for the next control-server
            // dispatch to raise the budget. 1 ms is short enough that
            // `sim.run_for_ticks` round-trip latency dominates the cf-e2e
            // poll loop; long enough that we don't busy-spin a CPU core.
            std::thread::sleep(idle_chunk);
            continue;
        }
        flush_pending_runbundle(engine, &mut bundle_written);
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
        capture_grid_enabled: cli.capture_grid,
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
            hold_to_confirm: cli.hold_to_confirm,
            hold_threshold_ms: cli.hold_threshold_ms,
            key_remap_enabled: cli.key_remap_enabled,
            key_bindings: std::collections::BTreeMap::new(),
            reduce_camera_shake_pct: 0.0,
            tick_rate_hz: cli.tick_rate_hz,
            // M1 Gap F1: feel-cvar defaults match cf-actor::ActorTuning::default().
            accel: 1500.0,
            friction: 1200.0,
            gravity: -980.0,
            jump_force: 420.0,
            recoil_decay_per_tick: 0.05,
            sharp_aim_build_ticks: 30,
            walk_threshold: 1.5,
            ai_difficulty: "tough_crowd".to_string(),
            ai_debug: cli.ai_debug,
            // M8 settings — defaults from Settings::default() let cf-app
            // pick them up without per-CLI plumbing.
            ..Settings::default()
        },
        seed_override: cli.seed,
        duration_ticks_override: if cli_duration > 0 { Some(cli_duration) } else { None },
        debug_inject_panic_at_tick: cli.debug_inject_panic_at_tick,
        checksum_cadence_ticks: cli.checksum_cadence_ticks,
        expected_outcome: cli.expected_outcome.map(Into::into),
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
    /// **M1 R2 / Blocker 3b**: when true, `drive_engine_tick` advances as
    /// many sim ticks per Bevy frame as the engine's clock budget allows
    /// (capped at `unpaced_max_ticks_per_frame`). Without this, cf-app
    /// drives exactly one tick per Bevy frame (~60Hz wall-clock), which
    /// makes 18000-tick endurance scripts take 300s of wall clock and
    /// cf-e2e's 180s default timeout kill them. With this flag the engine
    /// races through pending budget so 18000 ticks complete in seconds.
    unpaced: bool,
    /// Safety cap on ticks-per-frame so a runaway budget can't starve
    /// Bevy's other systems. Defaults to 1024 which is plenty for the M1
    /// endurance script (~18 Bevy frames to finish 18000 ticks).
    unpaced_max_ticks_per_frame: u32,
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

#[derive(Resource, Debug, Clone, Copy)]
struct LocalInputEnabled(bool);

fn run_bevy(
    config: M0EngineConfig,
    control_api: bool,
    control_port: u16,
    _uds: Option<PathBuf>,
    control_port_file: Option<PathBuf>,
    local_input_enabled: bool,
    capture_opts: CaptureOptions,
    unpaced: bool,
) -> Result<()> {
    let engine = Arc::new(M0Engine::new(config.clone()));
    engine.record_run_started();
    engine.record_setting_snapshot();

    let control_rt = if control_api {
        let rt = start_control_server(engine.clone(), control_port)?;
        write_control_port_file(control_port_file.as_deref(), rt.bound_addr)?;
        Some(rt)
    } else {
        if let Some(path) = control_port_file {
            anyhow::bail!(
                "--control-port-file={} requires --control-api so there is a bound port to report",
                path.display()
            );
        }
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
    // **M12** § Slideshow audio playback — point AssetServer at the
    // workspace root so `AssetServer::load("game/content/audio/...")` and
    // `AssetServer::load("content/assets/placeholders/...")` both resolve
    // cleanly. Without this override, Bevy defaults to a `./assets/`
    // directory next to the binary and the audio + slide PNG handles fail
    // to load.
    let workspace_root = std::env::current_dir()
        .ok()
        .and_then(|p| {
            // Walk up from CWD looking for the workspace marker (Cargo.toml
            // containing `[workspace]`). Falls back to CWD if not found.
            let mut cur = p.as_path();
            loop {
                let manifest = cur.join("Cargo.toml");
                if manifest.exists() {
                    let content = std::fs::read_to_string(&manifest).unwrap_or_default();
                    if content.contains("[workspace]") {
                        return Some(cur.to_path_buf());
                    }
                }
                match cur.parent() {
                    Some(parent) => cur = parent,
                    None => return None,
                }
            }
        })
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
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
        .set(bevy::asset::AssetPlugin {
            file_path: workspace_root.display().to_string(),
            ..default()
        })
        .disable::<LogPlugin>();
    // **M12** § Slideshow audio playback — cache the resolved workspace
    // root so the audio path resolver in `m12_spawn_slideshow_audio` can
    // turn ledger-absolute paths into AssetServer-relative paths.
    app.insert_resource(WorkspaceAssetRoot(workspace_root.clone()));
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
        .add_plugins(ChunkedTerrainPlugin)
        .add_plugins(ReactorVfxPlugin)
        // **M12A** § cf-audio playback engine — hydrate AudioRegistry +
        // SfxPool + CaptionRegistry from the cf-asset-ledger at startup.
        // Resources install empty here; `hydrate_audio_registries_from_ledger`
        // populates them in the Startup schedule.
        .add_plugins(M12aAudioPlugin)
        // M10 § M9A asset icons resolve at runtime: install the
        // AssetIndex resource + hydrate it from the workspace's
        // canonical ledger.jsonl at startup so the replay viewer +
        // death-recap modal can resolve icon paths via
        // `AssetIndex.get(canonical_name)` without missing-asset
        // warnings. See game/crates/cf-render-2d/src/asset_loader.rs.
        .add_plugins(AssetIndexPlugin)
        .add_plugins(StatusStripPlugin)
        // M11A: shell UI foundation (title / main menu / pause / save-load /
        // settings tree / credits / loading screen / FRE wizard polish).
        // Runs alongside in-mission HUD; screen transitions via act.shell.*.
        .add_plugins(cf_shell::ShellPlugin)
        // **M12** § Visual direction closure: juice rules, per-scene color
        // grading, panel transitions, CCCP-style intro slideshow, optional
        // comic overlay. Per spec § Crates / modules touched. All five
        // plugins ship state resources + ticking systems; cf-app's
        // `m12_sync_settings_to_juice_state` system mirrors the live
        // `cf-control::Settings` flags into them every frame.
        .add_plugins(JuicePlugin)
        .add_plugins(ColorGradingPlugin)
        .add_plugins(AnimationPlugin)
        .add_plugins(SlideshowPlugin)
        .add_plugins(ComicOverlayPlugin);
    app.add_systems(Startup, hydrate_asset_index_from_ledger);
    app.init_resource::<HoldTracker>();
    let capture_handle = CaptureStateHandle::default();
    app.add_plugins(CfCapturePlugin {
        config: capture_config.clone(),
        state_handle: capture_handle.clone(),
    });
    app.insert_resource(CaptureRecorderCursor::default());
    app.insert_resource(RenderEffectsCursor::default());
    app.insert_resource(Time::<Fixed>::from_hz(f64::from(config.tick_rate_hz)));
    app.insert_resource(EngineHolder(engine.clone()));
    app.insert_resource(LocalInputEnabled(local_input_enabled));
    app.insert_resource(AppRuntime {
        duration_ticks: config.duration_ticks,
        last_announced_tick: 0,
        unpaced,
        unpaced_max_ticks_per_frame: 1024,
    });
    if let Some(rt) = control_rt {
        app.insert_resource(rt);
    }

    // Paced (default) path: drive_engine_tick fires from FixedUpdate at
    // tick_rate_hz, exactly one sim tick per fire. The unpaced path below
    // drives from Update so it isn't capped by the FixedUpdate schedule;
    // both gate internally on `runtime.unpaced` so only one of them does
    // real work per Bevy iteration.
    app.add_systems(FixedUpdate, drive_engine_tick);
    if unpaced {
        app.add_systems(Update, drive_engine_tick_unpaced);
    }
    app.add_systems(
        Update,
        (
            esc_or_close_to_exit,
            handle_window_focus_capture,
            check_completion,
            log_tick_progress,
            ingest_player_input,
            ingest_focus_input,
            sync_actor_state_to_render,
            sync_terrain_state_to_render,
            sync_reactor_state_to_widgets,
            sync_engine_tick_to_capture_clock,
            pump_recorder_events_into_capture_keyframes,
            pump_recorder_events_into_render_effects,
        )
            .chain(),
    );
    // **M12** § Visual direction closure — per-frame settings sync,
    // input handling for slideshow skip, scene-mood inference, and the
    // ClearColor tint that applies the live `ColorGradingState::current_grade()`.
    app.add_systems(
        Update,
        (
            m12_sync_settings_to_juice_state,
            m12_sync_scene_mood_from_mission_phase,
            m12_ingest_slideshow_skip_input,
            m12_apply_color_grading_to_clear_color,
            m12_start_intro_slideshow_on_shell_screen_enter,
            m12_advance_slideshow_state,
            m12_render_slideshow_overlay,
            m12_spawn_slideshow_audio,
            m12_despawn_slideshow_audio,
            m12_finalize_completed_slideshow,
            m12_trigger_banner_slide_in_juice,
            m12_render_screen_flash_overlay,
            m12_dispatch_juice_audio_cues,
        ),
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

/// **M10 § "references M9A asset icons"** — startup system that hydrates
/// the cf-render-2d `AssetIndex` from the workspace's canonical
/// `content/asset_ledger/ledger.jsonl`. The replay viewer + in-game
/// death-recap modal call `AssetIndex.get(canonical_name)` to resolve
/// the PNG / SVG path for any tier-1 placeholder asset; without this
/// hydration every icon lookup would miss (per AGENTS.md "asset index
/// plugin wiring" mandate).
///
/// Path resolution order:
///   1. `CF_ASSET_LEDGER_PATH` env var (explicit override, dev / tests).
///   2. Climb from `cf-app`'s `CARGO_MANIFEST_DIR`
///      (`<repo>/game/crates/cf-app`) up three parents to
///      `<repo>/content/asset_ledger/ledger.jsonl`.
///   3. Fall back to `<CWD>/content/asset_ledger/ledger.jsonl` so that
///      `cd /Users/erol/projects/corefall && cargo run` resolves it too.
///
/// All paths are best-effort: when the ledger is absent (e.g., during a
/// pre-M9A bring-up scenario), the hydration is a no-op and a single
/// `tracing::warn!` line surfaces the miss. No icon-lookup-time warnings.
fn hydrate_asset_index_from_ledger(mut index: ResMut<AssetIndex>) {
    use std::path::PathBuf;
    let candidates: Vec<PathBuf> = {
        let mut v: Vec<PathBuf> = Vec::new();
        if let Ok(p) = std::env::var("CF_ASSET_LEDGER_PATH") {
            v.push(PathBuf::from(p));
        }
        // Climb 3 parents from cf-app's CARGO_MANIFEST_DIR → repo root.
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        if let Some(repo) = manifest_dir.parent().and_then(|p| p.parent()).and_then(|p| p.parent()) {
            v.push(repo.join("content").join("asset_ledger").join("ledger.jsonl"));
        }
        if let Ok(cwd) = std::env::current_dir() {
            v.push(cwd.join("content").join("asset_ledger").join("ledger.jsonl"));
            if let Some(parent) = cwd.parent() {
                v.push(parent.join("content").join("asset_ledger").join("ledger.jsonl"));
            }
        }
        v
    };
    for path in &candidates {
        if !path.exists() {
            continue;
        }
        match load_ledger_index(path, &mut index) {
            Ok(n) => {
                tracing::info!(
                    target: "cf::asset_index",
                    "hydrated AssetIndex from {} ({} entries)",
                    path.display(),
                    n
                );
                return;
            }
            Err(e) => {
                tracing::warn!(
                    target: "cf::asset_index",
                    "failed to hydrate AssetIndex from {}: {}",
                    path.display(),
                    e
                );
                return;
            }
        }
    }
    tracing::warn!(
        target: "cf::asset_index",
        "ledger.jsonl not found at any candidate path; AssetIndex left empty (M10 death-recap icons will fall back to symbolic placeholders)"
    );
}

/// **M2**: bridge the engine's chunked terrain into cf-render-2d. Gated on
/// engine tick advance so we don't take an extra read lock every render
/// frame (the cfctl `observe.once` poll path needs the same read lock at
/// 15 ms cadence; under paced 60 Hz with long `sim.run_for_ticks(N)`
/// windows, lock contention starved cfctl observe polls).
#[derive(Resource, Default)]
struct TerrainBridgeCursor {
    last_tick: u64,
    initialized: bool,
}

fn sync_terrain_state_to_render(
    holder: Res<EngineHolder>,
    mut terrain_snapshot: ResMut<ChunkedTerrainSnapshot>,
    mut overlay_state: ResMut<OverlayModeState>,
    mut dig_ghost: ResMut<DigPreviewGhost>,
    mut cursor: ResMut<TerrainBridgeCursor>,
) {
    let tick_now = holder.0.current_tick().0;
    if cursor.initialized && cursor.last_tick == tick_now {
        return;
    }
    cursor.initialized = true;
    cursor.last_tick = tick_now;
    let snap = holder.0.terrain_render_snapshot();
    terrain_snapshot.active = snap.active;
    terrain_snapshot.anchor = snap.anchor;
    terrain_snapshot.updates.clear();
    for u in snap.dirty_updates {
        terrain_snapshot.updates.push(ChunkUpdate {
            cx: u.cx,
            cy: u.cy,
            dirty_rect: u.dirty_rect,
            pixels: u.pixels,
        });
    }
    overlay_state.mode = OverlayMode::parse_mode(snap.overlay_mode.as_str());
    let live_settings = holder.0.current_settings();
    dig_ghost.reduced_motion = live_settings.reduced_motion;
    dig_ghost.target = snap.dig_preview.map(|p| DigPreviewTarget {
        position: bevy::math::Vec2::new(p.position[0], p.position[1]),
        radius: p.radius,
        valid: p.valid,
        material_id: Some(p.material_id),
    });
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

/// **M1 Gap E**: drain the recorder's ux.* + equipment.weapon_fired events
/// since the last frame and translate them into render-layer effects
/// (CameraShake, HitStop, MuzzleFlash). Uses a per-frame cursor so each
/// event is consumed exactly once.
#[derive(Resource, Default)]
struct RenderEffectsCursor(usize);

fn pump_recorder_events_into_render_effects(
    holder: Res<EngineHolder>,
    mut shake: ResMut<CameraShake>,
    mut hit_stop: ResMut<HitStop>,
    mut state: ResMut<ActorRenderState>,
    mut debris_queue: ResMut<DebrisSpawnQueue>,
    mut sparks: ResMut<SparkEmitterState>,
    mut explosion: ResMut<ExplosionState>,
    mut cursor: ResMut<RenderEffectsCursor>,
    juice_acc: Res<JuiceAccessibility>,
    mut juice_state: ResMut<JuiceState>,
) {
    let settings = futures_block_on(async { holder.0.settings_snapshot().await });
    shake.reduce_pct = settings.reduce_camera_shake_pct;
    let recorder = holder.0.recorder();
    let new_events = recorder.events_since(cursor.0);
    cursor.0 += new_events.len();
    for ev in new_events {
        match (ev.category.as_str(), ev.event_type.as_str()) {
            ("terrain", "terrain_pixel_dislodged") => {
                let pos_arr = ev.payload.get("pos").and_then(|v| v.as_array());
                let x = pos_arr
                    .and_then(|arr| arr.first())
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as f32;
                let y = pos_arr
                    .and_then(|arr| arr.get(1))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as f32;
                let count = ev.payload.get("count").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
                let mat = ev
                    .payload
                    .get("spawn_material_id")
                    .and_then(|v| v.as_u64())
                    .and_then(|n| u8::try_from(n).ok())
                    .or_else(|| {
                        ev.payload
                            .get("source_material_id")
                            .and_then(|v| v.as_u64())
                            .and_then(|n| u8::try_from(n).ok())
                    })
                    .unwrap_or(cf_terrain::MATERIAL_LOOSE_FILL);
                debris_queue.pending.push_back(DebrisSpawnRequest {
                    pos: bevy::math::Vec2::new(x, y),
                    spawn_material: mat,
                    count,
                });
            }
            ("ux", "camera_punch_requested") => {
                let magnitude = ev.payload.get("magnitude").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                shake.magnitude_px = (shake.magnitude_px + magnitude * 0.05).clamp(0.0, 40.0);
            }
            ("ux", "hit_stop_requested") => {
                let dur_ms = ev.payload.get("duration_ms").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                hit_stop.remaining_ms = hit_stop.remaining_ms.max(dur_ms);
                // **M12** § Juice rules — critical-hit punch couples hit-stop +
                // screen flash + chromatic aberration. The flash/aberration
                // half is suppressed by `reduce_flash`; the shake half by
                // `reduce_shake`. JuicePulse::new gates that via
                // JuiceAccessibility so the recorded `ux.juice_applied` event
                // carries the accessibility_suppressed flag.
                let pulse = JuicePulse::new(JuiceKind::CriticalHitPunch, *juice_acc);
                juice_state.push("ux.critical_hit", pulse);
            }
            // **M12** § Juice rules — weapon-swap whoosh.
            ("equipment", "weapon_swap_started") => {
                let pulse = JuicePulse::new(JuiceKind::WeaponSwapWhoosh, *juice_acc);
                juice_state.push("ux.weapon_swap", pulse);
            }
            // **M12** § Juice rules — reload-completion ding.
            ("equipment", "weapon_swap_completed") => {
                let pulse = JuicePulse::new(JuiceKind::ReloadCompletedDing, *juice_acc);
                juice_state.push("ux.reload_done", pulse);
            }
            // **M12** § Juice rules — pickup glow on item pickup.
            ("equipment", "item_picked_up") => {
                let pulse = JuicePulse::new(JuiceKind::PickupGlow, *juice_acc);
                juice_state.push("ux.pickup", pulse);
            }
            ("equipment", "weapon_fired") => {
                let origin = ev.payload.get("muzzle_origin").and_then(|v| v.as_array()).map(|arr| {
                    let x = arr.first().and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = arr.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    bevy::math::Vec2::new(x, y)
                });
                if let Some(o) = origin {
                    state.muzzle_flash = Some(MuzzleFlashRender {
                        origin: o,
                        remaining_ticks: 3,
                    });
                }
            }
            // **M9** § Bullet-impact sparks on reactor — every reactor hit
            // spawns a brief spark burst at the impact point. Capped at
            // `SPARK_CAP_PER_HIT` per event by the emitter.
            ("combat", "projectile_hit") => {
                let is_reactor = ev
                    .payload
                    .get("target_kind")
                    .and_then(|v| v.as_str())
                    .is_some_and(|s| s == "reactor");
                if !is_reactor {
                    continue;
                }
                let pos = ev.payload.get("position").and_then(|v| v.as_array());
                let x = pos.and_then(|arr| arr.first()).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                let y = pos.and_then(|arr| arr.get(1)).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                sparks.spawn_burst([x, y], SPARK_CAP_PER_HIT, 180);
            }
            // **M9** § Explosion VFX on reactor destruction — flash + debris
            // scatter (capped) + shake scaled by accessibility's
            // `reduce_camera_shake_pct`. cf-render-2d's tick_reactor_vfx
            // retires the burst within 1s per spec.
            ("mission", "reactor_destroyed") => {
                let pos = ev.payload.get("position").and_then(|v| v.as_array());
                let x = pos.and_then(|arr| arr.first()).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                let y = pos.and_then(|arr| arr.get(1)).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                explosion.spawn([x, y], EXPLOSION_DEBRIS_CAP_PER_HIT, settings.reduce_camera_shake_pct);
                // Couple a brief camera shake to the destruction event so
                // accessibility's reduce_camera_shake_pct gates both at the
                // same source.
                let shake_scale = (1.0 - settings.reduce_camera_shake_pct.clamp(0.0, 1.0)).max(0.0);
                shake.magnitude_px = (shake.magnitude_px + 18.0 * shake_scale).clamp(0.0, 40.0);
            }
            _ => {}
        }
    }
}

fn drive_engine_tick(holder: Res<EngineHolder>, mut runtime: ResMut<AppRuntime>) {
    if runtime.unpaced {
        // Paced path is short-circuited when unpaced is requested; the
        // separate Update-scheduled `drive_engine_tick_unpaced` does the
        // real work. We bail here so the FixedUpdate firing rate doesn't
        // bottleneck cf-e2e scripts with thousand-tick budgets.
        return;
    }
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

/// **M1 R2 / Blocker 3b**: unpaced engine driver. Runs on Bevy's Update
/// schedule (not FixedUpdate) so it isn't capped at `tick_rate_hz` real-time
/// firing. Drives up to `unpaced_max_ticks_per_frame` (default 1024) ticks
/// per Bevy frame, stopping when the engine's clock budget is exhausted.
/// This is what makes the 18000-tick m1_5min_endurance cfctl script
/// complete in seconds instead of 5 minutes of wall-clock pacing.
fn drive_engine_tick_unpaced(holder: Res<EngineHolder>, mut runtime: ResMut<AppRuntime>) {
    if !runtime.unpaced {
        return;
    }
    if holder.0.shutdown_requested() {
        return;
    }
    if runtime.duration_ticks > 0 && holder.0.current_tick().0 >= runtime.duration_ticks {
        return;
    }
    let max_ticks_this_frame = runtime.unpaced_max_ticks_per_frame.max(1);
    for _ in 0..max_ticks_this_frame {
        if runtime.duration_ticks > 0 && holder.0.current_tick().0 >= runtime.duration_ticks {
            break;
        }
        if holder.0.drive_tick().is_none() {
            // SimClock budget exhausted; wait for the next sim.run_for_ticks
            // / sim.step dispatch to raise the budget again.
            break;
        }
    }
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

/// M4A canonical action ids that drive `ingest_player_input`. Stable strings
/// shared with `Settings.key_bindings` so the cfctl + observe surface can
/// remap them by name. `fire_alt` mirrors `fire` (Enter + KeyJ are both fire
/// keys by default; remapping replaces both with the configured KeyCode).
pub const ACTION_JUMP: &str = "jump";
pub const ACTION_FIRE: &str = "fire";
pub const ACTION_FIRE_ALT: &str = "fire_alt";
pub const ACTION_RELOAD: &str = "reload";
pub const ACTION_DIG: &str = "dig";
pub const ACTION_RESET: &str = "reset";
pub const ACTION_SELECT_SLOT_0: &str = "select_slot_0";
pub const ACTION_SELECT_SLOT_1: &str = "select_slot_1";
pub const ACTION_SELECT_SLOT_2: &str = "select_slot_2";
pub const ACTION_SELECT_SLOT_3: &str = "select_slot_3";
// Audit fix round-5 (2026-05-10): continuous actions (held-key → analog
// axis) now part of the remap surface so movement + aim honor live
// `Settings.key_bindings`. Defaults preserve the historical WASD-move +
// Arrow-aim feel.
pub const ACTION_MOVE_LEFT: &str = "move_left";
pub const ACTION_MOVE_RIGHT: &str = "move_right";
pub const ACTION_MOVE_UP: &str = "move_up";
pub const ACTION_MOVE_DOWN: &str = "move_down";
pub const ACTION_AIM_LEFT: &str = "aim_left";
pub const ACTION_AIM_RIGHT: &str = "aim_right";
pub const ACTION_AIM_UP: &str = "aim_up";
pub const ACTION_AIM_DOWN: &str = "aim_down";

/// M4A: parse a KeyCode by stable name. Returns `None` only for defensive
/// in-memory fallbacks; live settings patches validate names before dispatch.
fn parse_key_code(name: &str) -> Option<KeyCode> {
    match name {
        "Space" => Some(KeyCode::Space),
        "Enter" => Some(KeyCode::Enter),
        "Tab" => Some(KeyCode::Tab),
        "Escape" => Some(KeyCode::Escape),
        "Backspace" => Some(KeyCode::Backspace),
        "ArrowUp" => Some(KeyCode::ArrowUp),
        "ArrowDown" => Some(KeyCode::ArrowDown),
        "ArrowLeft" => Some(KeyCode::ArrowLeft),
        "ArrowRight" => Some(KeyCode::ArrowRight),
        "ShiftLeft" => Some(KeyCode::ShiftLeft),
        "ShiftRight" => Some(KeyCode::ShiftRight),
        "ControlLeft" => Some(KeyCode::ControlLeft),
        "ControlRight" => Some(KeyCode::ControlRight),
        "KeyA" => Some(KeyCode::KeyA),
        "KeyB" => Some(KeyCode::KeyB),
        "KeyC" => Some(KeyCode::KeyC),
        "KeyD" => Some(KeyCode::KeyD),
        "KeyE" => Some(KeyCode::KeyE),
        "KeyF" => Some(KeyCode::KeyF),
        "KeyG" => Some(KeyCode::KeyG),
        "KeyH" => Some(KeyCode::KeyH),
        "KeyI" => Some(KeyCode::KeyI),
        "KeyJ" => Some(KeyCode::KeyJ),
        "KeyK" => Some(KeyCode::KeyK),
        "KeyL" => Some(KeyCode::KeyL),
        "KeyM" => Some(KeyCode::KeyM),
        "KeyN" => Some(KeyCode::KeyN),
        "KeyO" => Some(KeyCode::KeyO),
        "KeyP" => Some(KeyCode::KeyP),
        "KeyQ" => Some(KeyCode::KeyQ),
        "KeyR" => Some(KeyCode::KeyR),
        "KeyS" => Some(KeyCode::KeyS),
        "KeyT" => Some(KeyCode::KeyT),
        "KeyU" => Some(KeyCode::KeyU),
        "KeyV" => Some(KeyCode::KeyV),
        "KeyW" => Some(KeyCode::KeyW),
        "KeyX" => Some(KeyCode::KeyX),
        "KeyY" => Some(KeyCode::KeyY),
        "KeyZ" => Some(KeyCode::KeyZ),
        "Digit0" => Some(KeyCode::Digit0),
        "Digit1" => Some(KeyCode::Digit1),
        "Digit2" => Some(KeyCode::Digit2),
        "Digit3" => Some(KeyCode::Digit3),
        "Digit4" => Some(KeyCode::Digit4),
        "Digit5" => Some(KeyCode::Digit5),
        "Digit6" => Some(KeyCode::Digit6),
        "Digit7" => Some(KeyCode::Digit7),
        "Digit8" => Some(KeyCode::Digit8),
        "Digit9" => Some(KeyCode::Digit9),
        "Numpad0" => Some(KeyCode::Numpad0),
        "Numpad1" => Some(KeyCode::Numpad1),
        "Numpad2" => Some(KeyCode::Numpad2),
        "Numpad3" => Some(KeyCode::Numpad3),
        "Numpad4" => Some(KeyCode::Numpad4),
        "Numpad5" => Some(KeyCode::Numpad5),
        "Numpad6" => Some(KeyCode::Numpad6),
        "Numpad7" => Some(KeyCode::Numpad7),
        "Numpad8" => Some(KeyCode::Numpad8),
        "Numpad9" => Some(KeyCode::Numpad9),
        _ => None,
    }
}

/// M4A: resolve the active KeyCode for an action by reading
/// `Settings.key_bindings` (when `key_remap_enabled = true`) and falling
/// back to the hard-coded default only as a defensive in-memory fallback.
/// Live cfctl/JSON-RPC patches validate action + key names before they enter
/// Settings, so unsupported remaps reject instead of silently succeeding.
fn key_for_action(settings: &cf_control::Settings, action: &str) -> Option<KeyCode> {
    if settings.key_remap_enabled {
        if let Some(name) = settings.key_bindings.get(action) {
            if let Some(k) = parse_key_code(name) {
                return Some(k);
            }
            tracing::warn!(target: "cf::app", action = %action, binding = %name, "unknown key binding name; falling back to default");
        }
    }
    match action {
        ACTION_JUMP => Some(KeyCode::Space),
        ACTION_FIRE => Some(KeyCode::Enter),
        ACTION_FIRE_ALT => Some(KeyCode::KeyJ),
        ACTION_RELOAD => Some(KeyCode::KeyR),
        ACTION_DIG => Some(KeyCode::KeyG),
        ACTION_RESET => Some(KeyCode::KeyL),
        ACTION_SELECT_SLOT_0 => Some(KeyCode::Digit1),
        ACTION_SELECT_SLOT_1 => Some(KeyCode::Digit2),
        ACTION_SELECT_SLOT_2 => Some(KeyCode::Digit3),
        ACTION_SELECT_SLOT_3 => Some(KeyCode::Digit4),
        ACTION_MOVE_LEFT => Some(KeyCode::KeyA),
        ACTION_MOVE_RIGHT => Some(KeyCode::KeyD),
        ACTION_MOVE_UP => Some(KeyCode::KeyW),
        ACTION_MOVE_DOWN => Some(KeyCode::KeyS),
        ACTION_AIM_LEFT => Some(KeyCode::ArrowLeft),
        ACTION_AIM_RIGHT => Some(KeyCode::ArrowRight),
        ACTION_AIM_UP => Some(KeyCode::ArrowUp),
        ACTION_AIM_DOWN => Some(KeyCode::ArrowDown),
        _ => None,
    }
}

fn focus_owns_keyboard_key(key: KeyCode, focus_active: bool) -> bool {
    focus_active && matches!(key, KeyCode::ArrowUp | KeyCode::ArrowDown)
}

fn gameplay_key_pressed(keys: &ButtonInput<KeyCode>, key: KeyCode, focus_active: bool) -> bool {
    keys.pressed(key) && !focus_owns_keyboard_key(key, focus_active)
}

fn gameplay_key_just_released(keys: &ButtonInput<KeyCode>, key: KeyCode, focus_active: bool) -> bool {
    keys.just_released(key) && !focus_owns_keyboard_key(key, focus_active)
}

/// M4A: hold-to-confirm tracker. Pure-Rust state; cf-app's
/// `ingest_player_input` calls `update` once per frame with the live key
/// state + Settings flags + an Instant; the tracker returns the set of
/// actions that fired this frame.
///
/// Behavior contract (DR-012 ACC-A-05):
///
/// - When `hold_to_confirm = false`, every action fires on the first frame
///   the action's KeyCode transitions from `released` to `pressed` (tap).
/// - When `hold_to_confirm = true`, the action key must be held continuously
///   for `hold_threshold_ms` before the action fires; releasing before the
///   threshold cancels the hold; the action fires AT MOST ONCE per hold.
///
/// The tracker is unit-testable without Bevy via `tick_with_state`.
#[derive(Resource, Debug, Default)]
pub struct HoldTracker {
    holds: std::collections::HashMap<String, HoldEntry>,
}

#[derive(Debug, Clone, Copy)]
struct HoldEntry {
    started_at: std::time::Instant,
    fired: bool,
}

impl HoldTracker {
    /// Per-frame update. Returns the set of action ids that fired THIS frame.
    /// `pressed_actions` is the set of action ids whose KeyCode is currently
    /// down; `now` is the wall-clock instant for the frame.
    pub fn tick_with_state(
        &mut self,
        pressed_actions: &std::collections::HashSet<String>,
        hold_to_confirm: bool,
        hold_threshold: std::time::Duration,
        now: std::time::Instant,
    ) -> std::collections::HashSet<String> {
        let mut fired: std::collections::HashSet<String> = std::collections::HashSet::new();
        // Drop holds whose action keys were released this frame.
        self.holds.retain(|action, _| pressed_actions.contains(action));
        for action in pressed_actions {
            let entry = self.holds.entry(action.clone()).or_insert(HoldEntry {
                started_at: now,
                fired: false,
            });
            if !hold_to_confirm {
                // Tap mode: fire on the FIRST frame the key was seen pressed
                // (i.e., when we just inserted the entry). The `started_at`
                // instant is the press tick; if the entry's `fired` is false
                // it means we haven't fired yet for this hold session.
                if !entry.fired {
                    entry.fired = true;
                    fired.insert(action.clone());
                }
            } else if !entry.fired && now.saturating_duration_since(entry.started_at) >= hold_threshold {
                entry.fired = true;
                fired.insert(action.clone());
            }
        }
        fired
    }

    /// Test-only convenience: clear the tracker. Equivalent to a fresh frame
    /// after a settings change.
    #[cfg(test)]
    pub fn clear(&mut self) {
        self.holds.clear();
    }
}

#[cfg(test)]
mod hold_tracker_tests {
    use super::*;
    use std::collections::HashSet;
    use std::time::{Duration, Instant};

    fn pressed_set(actions: &[&str]) -> HashSet<String> {
        actions.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn tap_fires_on_first_pressed_frame_then_stays_silent() {
        let mut t = HoldTracker::default();
        let now = Instant::now();
        let fired = t.tick_with_state(&pressed_set(&["jump"]), false, Duration::from_millis(250), now);
        assert!(fired.contains("jump"));
        let fired_next = t.tick_with_state(
            &pressed_set(&["jump"]),
            false,
            Duration::from_millis(250),
            now + Duration::from_millis(16),
        );
        assert!(!fired_next.contains("jump"), "tap should fire only once per hold");
    }

    #[test]
    fn tap_fires_again_after_release_then_press() {
        let mut t = HoldTracker::default();
        let now = Instant::now();
        let _ = t.tick_with_state(&pressed_set(&["fire"]), false, Duration::from_millis(250), now);
        let _ = t.tick_with_state(
            &pressed_set(&[]),
            false,
            Duration::from_millis(250),
            now + Duration::from_millis(16),
        );
        let fired = t.tick_with_state(
            &pressed_set(&["fire"]),
            false,
            Duration::from_millis(250),
            now + Duration::from_millis(32),
        );
        assert!(fired.contains("fire"), "post-release press fires again");
    }

    #[test]
    fn hold_does_not_fire_before_threshold() {
        let mut t = HoldTracker::default();
        let now = Instant::now();
        let fired = t.tick_with_state(&pressed_set(&["jump"]), true, Duration::from_millis(250), now);
        assert!(!fired.contains("jump"), "hold mode must NOT fire on tap");
        // 100ms in, still below threshold.
        let fired = t.tick_with_state(
            &pressed_set(&["jump"]),
            true,
            Duration::from_millis(250),
            now + Duration::from_millis(100),
        );
        assert!(!fired.contains("jump"), "still below threshold");
    }

    #[test]
    fn hold_fires_once_at_threshold() {
        let mut t = HoldTracker::default();
        let now = Instant::now();
        let _ = t.tick_with_state(&pressed_set(&["jump"]), true, Duration::from_millis(250), now);
        let fired = t.tick_with_state(
            &pressed_set(&["jump"]),
            true,
            Duration::from_millis(250),
            now + Duration::from_millis(260),
        );
        assert!(fired.contains("jump"), "fires exactly when threshold reached");
        // Continued hold: no further fires.
        let fired_next = t.tick_with_state(
            &pressed_set(&["jump"]),
            true,
            Duration::from_millis(250),
            now + Duration::from_millis(500),
        );
        assert!(!fired_next.contains("jump"), "fires at most once per hold");
    }

    #[test]
    fn hold_release_before_threshold_cancels() {
        let mut t = HoldTracker::default();
        let now = Instant::now();
        let _ = t.tick_with_state(&pressed_set(&["fire"]), true, Duration::from_millis(250), now);
        // Release before threshold.
        let fired = t.tick_with_state(
            &pressed_set(&[]),
            true,
            Duration::from_millis(250),
            now + Duration::from_millis(100),
        );
        assert!(!fired.contains("fire"), "no fire if released before threshold");
        // Press again; must restart the hold timer.
        let fired = t.tick_with_state(
            &pressed_set(&["fire"]),
            true,
            Duration::from_millis(250),
            now + Duration::from_millis(120),
        );
        assert!(!fired.contains("fire"), "new hold session, threshold not reached");
        let fired = t.tick_with_state(
            &pressed_set(&["fire"]),
            true,
            Duration::from_millis(250),
            now + Duration::from_millis(380),
        );
        assert!(fired.contains("fire"), "second hold completes after a fresh threshold");
    }

    #[test]
    fn key_for_action_honors_remap_when_enabled() {
        use std::collections::BTreeMap;
        let baseline = cf_control::Settings::default();
        assert_eq!(
            key_for_action(&baseline, ACTION_FIRE),
            Some(KeyCode::Enter),
            "default fire is Enter"
        );
        let mut bindings = BTreeMap::new();
        bindings.insert("fire".to_string(), "KeyF".to_string());
        bindings.insert("jump".to_string(), "ShiftLeft".to_string());
        let s = cf_control::Settings {
            key_remap_enabled: true,
            key_bindings: bindings,
            ..cf_control::Settings::default()
        };
        assert_eq!(key_for_action(&s, ACTION_FIRE), Some(KeyCode::KeyF));
        assert_eq!(key_for_action(&s, ACTION_JUMP), Some(KeyCode::ShiftLeft));
        assert_eq!(key_for_action(&s, ACTION_RELOAD), Some(KeyCode::KeyR));
    }

    #[test]
    fn key_for_action_ignores_remap_when_disabled() {
        use std::collections::BTreeMap;
        let mut bindings = BTreeMap::new();
        bindings.insert("fire".to_string(), "KeyF".to_string());
        let s = cf_control::Settings {
            key_remap_enabled: false,
            key_bindings: bindings,
            ..cf_control::Settings::default()
        };
        assert_eq!(
            key_for_action(&s, ACTION_FIRE),
            Some(KeyCode::Enter),
            "remap ignored when disabled"
        );
    }

    #[test]
    fn key_for_action_warns_on_unknown_binding_name_and_falls_back() {
        use std::collections::BTreeMap;
        let mut bindings = BTreeMap::new();
        bindings.insert("fire".to_string(), "BogusKey".to_string());
        let s = cf_control::Settings {
            key_remap_enabled: true,
            key_bindings: bindings,
            ..cf_control::Settings::default()
        };
        assert_eq!(key_for_action(&s, ACTION_FIRE), Some(KeyCode::Enter));
    }

    #[test]
    fn keyboard_focus_tab_enters_focus_mode_without_arrow_stealing() {
        use cf_control::server::FocusDirection;
        assert!(matches!(
            keyboard_focus_direction(true, false, false, false, false, false),
            Some(FocusDirection::Next)
        ));
        assert!(matches!(
            keyboard_focus_direction(true, true, false, false, false, false),
            Some(FocusDirection::Prev)
        ));
    }

    #[test]
    fn keyboard_focus_arrows_only_navigate_after_focus_is_active() {
        use cf_control::server::FocusDirection;
        assert!(
            keyboard_focus_direction(false, false, true, false, false, false).is_none(),
            "ArrowDown must remain aim-only before Tab enters focus mode"
        );
        assert!(
            keyboard_focus_direction(false, false, false, true, false, false).is_none(),
            "ArrowUp must remain aim-only before Tab enters focus mode"
        );
        assert!(matches!(
            keyboard_focus_direction(false, false, true, false, false, true),
            Some(FocusDirection::Next)
        ));
        assert!(matches!(
            keyboard_focus_direction(false, false, false, true, false, true),
            Some(FocusDirection::Prev)
        ));
    }

    #[test]
    fn focus_mode_owns_arrow_keys_but_not_remapped_aim_keys() {
        let mut arrows = ButtonInput::<KeyCode>::default();
        arrows.press(KeyCode::ArrowDown);
        assert_eq!(
            keyboard_axis_gameplay(
                &arrows,
                KeyCode::KeyW,
                KeyCode::KeyS,
                KeyCode::ArrowUp,
                KeyCode::ArrowDown,
                false,
            ),
            -1.0,
            "without active focus ArrowDown remains default aim-down"
        );
        assert_eq!(
            keyboard_axis_gameplay(
                &arrows,
                KeyCode::KeyW,
                KeyCode::KeyS,
                KeyCode::ArrowUp,
                KeyCode::ArrowDown,
                true,
            ),
            0.0,
            "with active focus ArrowDown belongs to HUD traversal"
        );
        assert!(!gameplay_key_pressed(&arrows, KeyCode::ArrowDown, true));

        let mut remapped = ButtonInput::<KeyCode>::default();
        remapped.press(KeyCode::Numpad2);
        assert_eq!(
            keyboard_axis_gameplay(
                &remapped,
                KeyCode::KeyW,
                KeyCode::KeyS,
                KeyCode::Numpad8,
                KeyCode::Numpad2,
                true,
            ),
            -1.0,
            "focus mode only owns the physical arrow keys, not a remapped numpad aim key"
        );
    }

    fn make_gamepad_with_press(button: GamepadButton) -> Gamepad {
        let mut gp = Gamepad::default();
        gp.digital_mut().press(button);
        gp
    }

    fn make_gamepad_with_axis(axis: GamepadAxis, value: f32) -> Gamepad {
        let mut gp = Gamepad::default();
        gp.analog_mut().set(GamepadInput::Axis(axis), value);
        gp
    }

    #[test]
    fn gamepad_focus_input_dpad_down_dispatches_next() {
        use cf_control::server::FocusDirection;
        let gp = make_gamepad_with_press(GamepadButton::DPadDown);
        let mut prev_y = 0.0;
        assert!(matches!(
            gamepad_focus_direction(&gp, &mut prev_y, 0.5),
            Some(FocusDirection::Next)
        ));
    }

    #[test]
    fn gamepad_focus_input_dpad_up_dispatches_prev() {
        use cf_control::server::FocusDirection;
        let gp = make_gamepad_with_press(GamepadButton::DPadUp);
        let mut prev_y = 0.0;
        assert!(matches!(
            gamepad_focus_direction(&gp, &mut prev_y, 0.5),
            Some(FocusDirection::Prev)
        ));
    }

    #[test]
    fn gamepad_focus_input_dpad_right_dispatches_next() {
        use cf_control::server::FocusDirection;
        let gp = make_gamepad_with_press(GamepadButton::DPadRight);
        let mut prev_y = 0.0;
        assert!(matches!(
            gamepad_focus_direction(&gp, &mut prev_y, 0.5),
            Some(FocusDirection::Next)
        ));
    }

    #[test]
    fn gamepad_focus_input_dpad_left_dispatches_prev() {
        use cf_control::server::FocusDirection;
        let gp = make_gamepad_with_press(GamepadButton::DPadLeft);
        let mut prev_y = 0.0;
        assert!(matches!(
            gamepad_focus_direction(&gp, &mut prev_y, 0.5),
            Some(FocusDirection::Prev)
        ));
    }

    #[test]
    fn gamepad_focus_input_east_button_clears_focus() {
        use cf_control::server::FocusDirection;
        let gp = make_gamepad_with_press(GamepadButton::East);
        let mut prev_y = 0.0;
        assert!(matches!(
            gamepad_focus_direction(&gp, &mut prev_y, 0.5),
            Some(FocusDirection::Clear)
        ));
    }

    #[test]
    fn gamepad_focus_input_south_button_is_reserved_for_activation_no_focus_dispatch() {
        // South (Xbox A / PS Cross) is the standard "confirm / activate"
        // button on consoles. M4A does NOT own activation semantics (M5 + M8
        // own that), so the focus translator MUST NOT dispatch a focus
        // change when South is pressed — the button is reserved for a future
        // activation event. Returning None here is the honest behavior; the
        // earlier hard-coded `Set("hud.silhouette")` jump was source-
        // untruthful and was removed.
        let gp = make_gamepad_with_press(GamepadButton::South);
        let mut prev_y = 0.0;
        assert!(
            gamepad_focus_direction(&gp, &mut prev_y, 0.5).is_none(),
            "South must be a no-op for focus traversal"
        );
    }

    #[test]
    fn gamepad_focus_input_left_bumper_dispatches_prev() {
        use cf_control::server::FocusDirection;
        let gp = make_gamepad_with_press(GamepadButton::LeftTrigger);
        let mut prev_y = 0.0;
        assert!(matches!(
            gamepad_focus_direction(&gp, &mut prev_y, 0.5),
            Some(FocusDirection::Prev)
        ));
    }

    #[test]
    fn gamepad_focus_input_right_bumper_dispatches_next() {
        use cf_control::server::FocusDirection;
        let gp = make_gamepad_with_press(GamepadButton::RightTrigger);
        let mut prev_y = 0.0;
        assert!(matches!(
            gamepad_focus_direction(&gp, &mut prev_y, 0.5),
            Some(FocusDirection::Next)
        ));
    }

    #[test]
    fn gamepad_focus_input_right_stick_down_rising_edge_dispatches_next() {
        use cf_control::server::FocusDirection;
        let gp = make_gamepad_with_axis(GamepadAxis::RightStickY, -0.8);
        let mut prev_y = 0.0;
        assert!(matches!(
            gamepad_focus_direction(&gp, &mut prev_y, 0.5),
            Some(FocusDirection::Next)
        ));
        assert!((prev_y - (-0.8)).abs() < 1e-6);
    }

    #[test]
    fn gamepad_focus_input_right_stick_up_rising_edge_dispatches_prev() {
        use cf_control::server::FocusDirection;
        let gp = make_gamepad_with_axis(GamepadAxis::RightStickY, 0.9);
        let mut prev_y = 0.0;
        assert!(matches!(
            gamepad_focus_direction(&gp, &mut prev_y, 0.5),
            Some(FocusDirection::Prev)
        ));
    }

    #[test]
    fn gamepad_focus_input_right_stick_held_only_fires_on_rising_edge() {
        let gp = make_gamepad_with_axis(GamepadAxis::RightStickY, -0.8);
        let mut prev_y = -0.7;
        assert!(
            gamepad_focus_direction(&gp, &mut prev_y, 0.5).is_none(),
            "stick already past threshold last frame; should not refire"
        );
    }

    #[test]
    fn gamepad_focus_input_right_stick_below_threshold_does_not_fire() {
        let gp = make_gamepad_with_axis(GamepadAxis::RightStickY, -0.3);
        let mut prev_y = 0.0;
        assert!(gamepad_focus_direction(&gp, &mut prev_y, 0.5).is_none());
    }

    #[test]
    fn gamepad_focus_input_no_button_no_axis_returns_none() {
        let gp = Gamepad::default();
        let mut prev_y = 0.0;
        assert!(gamepad_focus_direction(&gp, &mut prev_y, 0.5).is_none());
    }

    #[test]
    fn gamepad_focus_input_per_gamepad_debounce_isolates_idle_pad_from_active_pad() {
        // Audit fix round-5 (2026-05-10): regression test for the global-
        // Local<f32> bug — when two pads are connected and pad A holds the
        // stick down (analog Y at -0.8) while pad B is idle (analog Y at 0),
        // an idle pad B's zero-Y must NOT reset pad A's debounce state.
        // The fix in `ingest_focus_input` keeps a per-Entity HashMap, but
        // the underlying helper is correct as long as the caller passes a
        // SEPARATE `&mut f32` per gamepad. This test exercises that
        // contract: with two separate per-pad histories, pad A fires once
        // on the rising edge then stays silent + pad B never fires.
        use cf_control::server::FocusDirection;

        let pad_a = make_gamepad_with_axis(GamepadAxis::RightStickY, -0.8);
        let pad_b = Gamepad::default(); // idle
        let mut history_a = 0.0_f32;
        let mut history_b = 0.0_f32;

        // Frame 1: pad A's stick crosses the threshold; pad B idle.
        let dir_a = gamepad_focus_direction(&pad_a, &mut history_a, 0.5);
        assert!(matches!(dir_a, Some(FocusDirection::Next)));
        let dir_b = gamepad_focus_direction(&pad_b, &mut history_b, 0.5);
        assert!(dir_b.is_none(), "idle pad B must not fire");
        assert!((history_a - (-0.8)).abs() < 1e-6);
        assert!(history_b.abs() < 1e-6, "pad B history untouched by pad A");

        // Frame 2: pad A's stick still held down; with per-pad history
        // preserved, no rising edge → no refire. Pad B still idle.
        let dir_a2 = gamepad_focus_direction(&pad_a, &mut history_a, 0.5);
        assert!(
            dir_a2.is_none(),
            "pad A held stick must not refire — per-pad history preserved"
        );
        let dir_b2 = gamepad_focus_direction(&pad_b, &mut history_b, 0.5);
        assert!(dir_b2.is_none());

        // Critical regression assertion: if the previous global-Local<f32>
        // implementation were still in place, pad B's idle Y of 0.0 would
        // overwrite pad A's history to 0, and a subsequent frame with pad A
        // still at -0.8 would FIRE again (because 0.0 → -0.8 is a rising
        // edge). With per-pad histories, that cannot happen.
    }

    #[test]
    fn settings_default_key_bindings_includes_movement_and_aim_actions() {
        // Audit fix round-5 (2026-05-10): movement + aim are part of the
        // remap surface so left-handed users + numpad-aim users can rebind
        // without code changes.
        let bindings = cf_control::default_key_bindings();
        assert_eq!(bindings.get("move_left").map(String::as_str), Some("KeyA"));
        assert_eq!(bindings.get("move_right").map(String::as_str), Some("KeyD"));
        assert_eq!(bindings.get("move_up").map(String::as_str), Some("KeyW"));
        assert_eq!(bindings.get("move_down").map(String::as_str), Some("KeyS"));
        assert_eq!(bindings.get("aim_left").map(String::as_str), Some("ArrowLeft"));
        assert_eq!(bindings.get("aim_right").map(String::as_str), Some("ArrowRight"));
        assert_eq!(bindings.get("aim_up").map(String::as_str), Some("ArrowUp"));
        assert_eq!(bindings.get("aim_down").map(String::as_str), Some("ArrowDown"));
    }

    #[test]
    fn key_for_action_honors_movement_remap_when_enabled() {
        use std::collections::BTreeMap;
        let mut bindings = BTreeMap::new();
        bindings.insert("move_left".into(), "KeyH".into());
        bindings.insert("move_right".into(), "KeyL".into());
        bindings.insert("aim_up".into(), "Numpad8".into());
        let s = cf_control::Settings {
            key_remap_enabled: true,
            key_bindings: bindings,
            ..cf_control::Settings::default()
        };
        assert_eq!(key_for_action(&s, ACTION_MOVE_LEFT), Some(KeyCode::KeyH));
        assert_eq!(key_for_action(&s, ACTION_MOVE_RIGHT), Some(KeyCode::KeyL));
        assert_eq!(key_for_action(&s, ACTION_AIM_UP), Some(KeyCode::Numpad8));
        // Unrebound action → default.
        assert_eq!(key_for_action(&s, ACTION_MOVE_UP), Some(KeyCode::KeyW));
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
    local_input_enabled: Res<LocalInputEnabled>,
    mut hold_tracker: ResMut<HoldTracker>,
    mut last_move_x: Local<f32>,
    mut last_aim: Local<(f32, f32)>,
    mut last_intent_epoch: Local<u64>,
) {
    let _ = rt; // Reserved; ControlRuntime presence does not gate human input.
    if !local_input_enabled.0 {
        return;
    }
    if !holder.0.config().has_actor_world {
        return;
    }
    // Audit fix round-5 (2026-05-10): movement + aim now honor the live
    // `Settings.key_bindings` remap table when `key_remap_enabled=true`. The
    // defaults preserve the historical WASD-move + Arrow-aim feel; W/S still
    // double on aim_y as a built-in alternative when arrows aren't reachable
    // (e.g. left-handed keyboard layouts). When remap is disabled, the
    // hardcoded WASD/Arrow defaults take over via `key_for_action`'s
    // fallback path.
    let settings = holder.0.current_settings();
    let focus_active = holder.0.hud_caches_snapshot().focused_node.is_some();
    let key_or = |action: &str, fallback: KeyCode| key_for_action(&settings, action).unwrap_or(fallback);
    let move_left = key_or(ACTION_MOVE_LEFT, KeyCode::KeyA);
    let move_right = key_or(ACTION_MOVE_RIGHT, KeyCode::KeyD);
    let move_up = key_or(ACTION_MOVE_UP, KeyCode::KeyW);
    let move_down = key_or(ACTION_MOVE_DOWN, KeyCode::KeyS);
    let aim_left = key_or(ACTION_AIM_LEFT, KeyCode::ArrowLeft);
    let aim_right = key_or(ACTION_AIM_RIGHT, KeyCode::ArrowRight);
    let aim_up = key_or(ACTION_AIM_UP, KeyCode::ArrowUp);
    let aim_down = key_or(ACTION_AIM_DOWN, KeyCode::ArrowDown);
    let move_x = keyboard_axis_pair_gameplay(&keys, move_right, move_left, focus_active);
    let aim_x = keyboard_axis_pair_gameplay(&keys, aim_right, aim_left, focus_active);
    let aim_y = keyboard_axis_gameplay(&keys, move_up, move_down, aim_up, aim_down, focus_active);
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
        // M4A action dispatch goes through the HoldTracker, which honors
        // `Settings.hold_to_confirm` + `hold_threshold_ms` + `key_remap_enabled` +
        // `key_bindings`. The set of actions whose CURRENT key is pressed this
        // frame is computed via `key_for_action(settings, action)`.
        let live_settings = holder.0.current_settings();
        let mut pressed: HashSet<String> = HashSet::new();
        for action in [
            ACTION_JUMP,
            ACTION_FIRE,
            ACTION_FIRE_ALT,
            ACTION_RELOAD,
            ACTION_DIG,
            ACTION_RESET,
            ACTION_SELECT_SLOT_0,
            ACTION_SELECT_SLOT_1,
            ACTION_SELECT_SLOT_2,
            ACTION_SELECT_SLOT_3,
        ] {
            if let Some(k) = key_for_action(&live_settings, action) {
                if gameplay_key_pressed(&keys, k, focus_active) {
                    pressed.insert(action.to_string());
                }
            }
        }
        let now = std::time::Instant::now();
        let threshold = std::time::Duration::from_millis(u64::from(live_settings.hold_threshold_ms));
        let fired = hold_tracker.tick_with_state(&pressed, live_settings.hold_to_confirm, threshold, now);
        if fired.contains(ACTION_JUMP) {
            let _ = holder
                .0
                .dispatch(ControlCommand::ActPlayerJump {
                    source: IntentSource::Human,
                })
                .await;
        }
        if fired.contains(ACTION_FIRE) || fired.contains(ACTION_FIRE_ALT) {
            let _ = holder
                .0
                .dispatch(ControlCommand::ActPlayerFire {
                    pressed: true,
                    source: IntentSource::Human,
                })
                .await;
        }
        // Released-fire dispatch keeps the `ActPlayerFireParams.pressed=false`
        // contract live for future hold-to-fire weapons. We emit it when the
        // primary OR alt fire key transitioned just_released this frame.
        let fire_primary = key_for_action(&live_settings, ACTION_FIRE);
        let fire_alt = key_for_action(&live_settings, ACTION_FIRE_ALT);
        let fire_released = fire_primary
            .map(|k| gameplay_key_just_released(&keys, k, focus_active))
            .unwrap_or(false)
            || fire_alt
                .map(|k| gameplay_key_just_released(&keys, k, focus_active))
                .unwrap_or(false);
        if fire_released {
            let _ = holder
                .0
                .dispatch(ControlCommand::ActPlayerFire {
                    pressed: false,
                    source: IntentSource::Human,
                })
                .await;
        }
        if fired.contains(ACTION_RELOAD) {
            let _ = holder
                .0
                .dispatch(ControlCommand::ActPlayerReload {
                    source: IntentSource::Human,
                })
                .await;
        }
        if fired.contains(ACTION_RESET) {
            let _ = holder
                .0
                .dispatch(ControlCommand::ActPlayerReset {
                    source: IntentSource::Human,
                })
                .await;
        }
        if fired.contains(ACTION_DIG) {
            let _ = holder
                .0
                .dispatch(ControlCommand::ActPlayerDig {
                    target: None,
                    source: IntentSource::Human,
                })
                .await;
        }
        for (action_id, slot) in [
            (ACTION_SELECT_SLOT_0, 0u32),
            (ACTION_SELECT_SLOT_1, 1u32),
            (ACTION_SELECT_SLOT_2, 2u32),
            (ACTION_SELECT_SLOT_3, 3u32),
        ] {
            if fired.contains(action_id) {
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

fn keyboard_axis_gameplay(
    keys: &ButtonInput<KeyCode>,
    pos_a: KeyCode,
    neg_a: KeyCode,
    pos_b: KeyCode,
    neg_b: KeyCode,
    focus_active: bool,
) -> f32 {
    let pos = gameplay_key_pressed(keys, pos_a, focus_active) || gameplay_key_pressed(keys, pos_b, focus_active);
    let neg = gameplay_key_pressed(keys, neg_a, focus_active) || gameplay_key_pressed(keys, neg_b, focus_active);
    axis_from_pressed(pos, neg)
}

fn axis_from_pressed(pos: bool, neg: bool) -> f32 {
    match (pos, neg) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
    }
}

fn keyboard_axis_pair_gameplay(keys: &ButtonInput<KeyCode>, pos: KeyCode, neg: KeyCode, focus_active: bool) -> f32 {
    axis_from_pressed(
        gameplay_key_pressed(keys, pos, focus_active),
        gameplay_key_pressed(keys, neg, focus_active),
    )
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

/// M4A: rebuild the HUD module-strip placeholder from the player's filtered
/// rifle view. Mirrors `cf-control::engine::build_module_strip_view` so the
/// HUD + the `cfctl observe` ActorView agree on every tick.
fn build_hud_module_strip(rifle: Option<&cf_control::RifleHudView>) -> HudModuleStrip {
    let weapon_state = match rifle {
        Some(r) => {
            let reloading = r.reload_remaining_ticks > 0;
            let empty = r.capacity > 0 && r.ammo == 0;
            if reloading || empty {
                "warning"
            } else {
                "nominal"
            }
        }
        _ => "not_present",
    };
    let weapon_label = match rifle {
        Some(r) => {
            if r.reload_remaining_ticks > 0 {
                "RELOADING".to_string()
            } else if r.capacity > 0 && r.ammo == 0 {
                "EMPTY".to_string()
            } else {
                format!("READY {}/{}", r.ammo, r.capacity)
            }
        }
        _ => "—".to_string(),
    };
    HudModuleStrip {
        modules: vec![
            HudModule {
                id: "weapon_mount".into(),
                label: weapon_label,
                state: weapon_state.into(),
                kind: "weapon_mount".into(),
            },
            HudModule {
                id: "jet".into(),
                label: "JET N/A".into(),
                state: "not_present".into(),
                kind: "jet".into(),
            },
            HudModule {
                id: "shield".into(),
                label: "SHIELD N/A".into(),
                state: "not_present".into(),
                kind: "shield".into(),
            },
            HudModule {
                id: "sensor".into(),
                label: "SENSOR N/A".into(),
                state: "not_present".into(),
                kind: "sensor".into(),
            },
        ],
        placeholder: true,
    }
}

/// Copy the engine's actor world + rifle state into the Bevy render + HUD
/// resources every frame. The engine is the single source of truth; render +
/// HUD never own authoritative state.
fn sync_actor_state_to_render(
    holder: Res<EngineHolder>,
    mut render_state: ResMut<ActorRenderState>,
    mut hud_state: ResMut<HudState>,
    mut hud_settings: ResMut<HudSettings>,
    mut camera_follow: ResMut<CameraFollow>,
) {
    let snapshot = holder.0.actor_render_snapshot();
    let hud_caches = holder.0.hud_caches_snapshot();
    let live_settings = holder.0.current_settings();
    render_state.actors = snapshot.actors.clone();
    render_state.player_actor_id = snapshot.player_actor_id;
    render_state.region_width = holder.0.config().region_width;
    render_state.region_height = holder.0.config().region_height;
    render_state.region_anchor_x = holder.0.config().region_anchor_x;
    render_state.region_anchor_y = holder.0.config().region_anchor_y;
    render_state.floor_y = snapshot.floor_y;
    // **M5**: feed the current sim tick to cf-render-2d so the chassis
    // pip walk-cycle has a phase. Without this, legs stand still while
    // the actor's position moves — the M5-DC-3 "static sliding pawn" gap
    // the per-zone chassis rendering exists to close.
    render_state.tick = snapshot.tick;

    // M4A: mirror cf-control::Settings into HudSettings so cf-ui's UiScale +
    // high-contrast palette systems pick up live `act.settings.set` patches.
    // M12: also mirror `comic_style_overlay` + `comic_death_recap` so
    // cf-ui::comic_overlay's `ComicOverlayState` reflects live settings.
    let next_settings = HudSettings {
        ui_scale: live_settings.ui_scale,
        high_contrast: live_settings.high_contrast,
        captions: live_settings.captions,
        reduced_motion: live_settings.reduced_motion,
        reduced_shake: live_settings.reduced_shake,
        reduced_flash: live_settings.reduced_flash,
        hold_to_confirm: live_settings.hold_to_confirm,
        hold_threshold_ms: live_settings.hold_threshold_ms,
        key_remap_enabled: live_settings.key_remap_enabled,
        focused_node: hud_caches.focused_node.clone(),
        ai_debug: live_settings.ai_debug,
        comic_style_overlay: live_settings.comic_style_overlay.as_str().to_string(),
        comic_death_recap: live_settings.comic_death_recap,
    };
    if (hud_settings.ui_scale - next_settings.ui_scale).abs() > f32::EPSILON
        || hud_settings.high_contrast != next_settings.high_contrast
        || hud_settings.captions != next_settings.captions
        || hud_settings.reduced_motion != next_settings.reduced_motion
        || hud_settings.reduced_shake != next_settings.reduced_shake
        || hud_settings.reduced_flash != next_settings.reduced_flash
        || hud_settings.hold_to_confirm != next_settings.hold_to_confirm
        || hud_settings.hold_threshold_ms != next_settings.hold_threshold_ms
        || hud_settings.key_remap_enabled != next_settings.key_remap_enabled
        || hud_settings.focused_node != next_settings.focused_node
        || hud_settings.ai_debug != next_settings.ai_debug
        || hud_settings.comic_style_overlay != next_settings.comic_style_overlay
        || hud_settings.comic_death_recap != next_settings.comic_death_recap
    {
        *hud_settings = next_settings;
    }

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

    // M4A: project per-actor stance + body silhouette + module strip from the
    // engine's authoritative observation. Player-anchored; non-player actors
    // are exposed via cfctl observe but not the cf-app HUD.
    let (stance, silhouette) = match hud_state.player.as_ref() {
        Some(p) => (
            p.stance.clone(),
            HudBodySilhouette {
                head_hp_pct: p.body_silhouette.head_hp_pct,
                torso_hp_pct: p.body_silhouette.torso_hp_pct,
                arm_left_hp_pct: p.body_silhouette.arm_left_hp_pct,
                arm_right_hp_pct: p.body_silhouette.arm_right_hp_pct,
                leg_left_hp_pct: p.body_silhouette.leg_left_hp_pct,
                leg_right_hp_pct: p.body_silhouette.leg_right_hp_pct,
                placeholder: p.body_silhouette.placeholder,
            },
        ),
        None => (String::new(), HudBodySilhouette::default()),
    };
    hud_state.stance = stance;
    hud_state.body_silhouette = silhouette;
    hud_state.stability = hud_state.player.as_ref().map(|p| p.stability).unwrap_or(1.0);
    // **M5**: prefer the chassis module strip when a chassis is attached;
    // otherwise fall back to the M4A weapon-mount placeholder.
    hud_state.modules = match hud_state.player.as_ref().and_then(|p| p.chassis.as_ref()) {
        Some(chassis) => HudModuleStrip {
            modules: chassis
                .modules
                .iter()
                .map(|m| HudModule {
                    id: m.id.clone(),
                    label: match m.kind.as_str() {
                        "weapon_mount" => "WEAPON".to_string(),
                        "jet" => "JET".to_string(),
                        "shield" => "SHIELD".to_string(),
                        "sensor" => "SENSOR".to_string(),
                        "repair_drone" => "REPAIR".to_string(),
                        _ => m.kind.to_uppercase(),
                    },
                    state: m.state.clone(),
                    kind: m.kind.clone(),
                })
                .collect(),
            placeholder: false,
        },
        None => build_hud_module_strip(snapshot.player_rifle.as_ref()),
    };

    // M4A: banners + captions + tool_validity from the engine's HUD-cache snapshot.
    hud_state.banners = hud_caches
        .banners
        .iter()
        .map(|b| HudBanner {
            id: b.id.clone(),
            severity: b.severity.clone(),
            label: b.label.clone(),
            raised_at_tick: b.raised_at_tick,
        })
        .collect();
    hud_state.captions = hud_caches
        .captions
        .iter()
        .map(|c| HudCaption {
            id: c.id.clone(),
            label: c.label.clone(),
            raised_at_tick: c.raised_at_tick,
        })
        .collect();
    hud_state.tool_validity =
        if hud_caches.tool_validity.last_carve_tick.is_some() || hud_caches.tool_validity.last_refusal_tick.is_some() {
            Some(HudToolValidity {
                last_carve_tick: hud_caches.tool_validity.last_carve_tick,
                last_refusal_tick: hud_caches.tool_validity.last_refusal_tick,
                last_refusal_reason: hud_caches.tool_validity.last_refusal_reason.clone(),
                last_refusal_target: hud_caches.tool_validity.last_refusal_target.clone(),
                valid: hud_caches.tool_validity.valid,
            })
        } else {
            None
        };
    // M1 Gap D3: surface the CONTROLS CAPTURED state to the HUD.
    hud_state.controls_captured_by = hud_caches.controls_captured_by.clone();

    // M1 Gap E2 + E4: feed CameraFollow + tool-validity to the renderer.
    if let Some(player) = hud_state.player.as_ref() {
        camera_follow.target = Some(bevy::math::Vec2::new(player.position[0], player.position[1]));
    }
    render_state.tool_valid = hud_caches
        .tool_validity
        .last_refusal_tick
        .map(|_| hud_caches.tool_validity.valid);

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
        show_me_why_event_id: m.show_me_why_event_id.clone(),
        show_replay_cta: m.show_replay_cta,
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
            intent_label: enemy_view.map(|e| e.intent_label.clone()).unwrap_or_default(),
            world_position: enemy_view.and_then(|e| e.position).or(Some(a.position)),
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

/// **M9** § HUD readability + observability + Reactor visual feedback —
/// mirror the engine's reactor + timer projections into the cf-ui
/// widgets + cf-render-2d sprite resource so the HUD reactor strip + the
/// timer-warning captions + the reactor sprite swap all reflect the
/// live sim state. Pulls from `ActorRenderSnapshot::reactor / timer`
/// (sync read; same lock the existing `sync_actor_state_to_render`
/// path already takes) so cf-app does not bounce off the async
/// cfctl path each frame.
fn sync_reactor_state_to_widgets(
    holder: Res<EngineHolder>,
    mut hp_bar: ResMut<ReactorHpBarState>,
    mut pressure_line: ResMut<ReactorPressureLineState>,
    mut timer_warnings: ResMut<TimerWarningsState>,
    mut sprite_state: ResMut<ReactorSpriteState>,
) {
    let snapshot = holder.0.actor_render_snapshot();

    match snapshot.reactor {
        Some(reactor) => {
            let pips: Vec<ArmorPipView> = reactor
                .armor_layers
                .iter()
                .map(|l| {
                    let kind: &'static str = match l.kind.as_str() {
                        "External" => "External",
                        "Internal" => "Internal",
                        "Core" => "Core",
                        _ => "External",
                    };
                    ArmorPipView {
                        kind,
                        hp: l.hp,
                        max_hp: l.max_hp,
                        hp_percent: l.hp_percent,
                        band: IntegrityBand::from_hp_percent(l.hp_percent),
                    }
                })
                .collect();
            hp_bar.update(reactor.hp, reactor.max_hp, &reactor.pressure_state, pips);
            pressure_line.update(&reactor.pressure_state);
            sprite_state.variant = ReactorSprite::from_pressure_state(&reactor.pressure_state);
            sprite_state.present = true;
        }
        None => {
            // No reactor in this scenario — keep resources at default so HUD
            // widgets render an inert state rather than stale data.
            *hp_bar = ReactorHpBarState::default();
            *pressure_line = ReactorPressureLineState::default();
            sprite_state.variant = ReactorSprite::Nominal;
            sprite_state.present = false;
        }
    }

    match snapshot.timer {
        Some(timer) if timer.total_ticks > 0 && !timer.mission_terminal => {
            let remaining_s = timer.remaining_seconds;
            for (threshold_s, _severity, _caption) in WARNING_THRESHOLDS {
                if remaining_s <= *threshold_s {
                    let _ = timer_warnings.push_threshold(*threshold_s, remaining_s);
                }
            }
            timer_warnings.update_color(remaining_s);
        }
        _ => {
            // No active timer — leave already-fired warnings in place (they're
            // single-shot per run by contract) but clear the color band so the
            // HUD doesn't render a stale tint over a terminal mission.
            timer_warnings.last_color = None;
        }
    }
}

/// M4A keyboard + controller focus traversal.
///
/// Closes DR-012 ACC-A-04 + the roadmap M4A "controller route through HUD" +
/// ACC-A "controller/keyboard/mouse parity" requirements.
///
/// Keyboard: Tab enters/advances focus through the canonical
/// `HUD_FOCUSABLE_NODES` list; Shift+Tab retreats; once focus is active,
/// ArrowDown / ArrowUp navigate within the focus ring. F1 clears focus.
/// Until Tab has entered focus mode, ArrowDown / ArrowUp remain gameplay aim
/// keys so aiming never also cycles HUD focus.
///
/// Controller (gamepad): D-Pad Down / Right advances focus; D-Pad Up /
/// Left retreats; the right-stick analog Y axis (deadzone 0.5) drives the
/// same Next / Prev dispatch on rising edge so a thumb stick is as good as
/// the D-Pad. The East face button (Xbox B / PlayStation Circle) clears
/// focus (the standard "back / cancel" convention on consoles). LeftTrigger
/// (L1) / RightTrigger (R1) also drive Prev / Next as a thumb-friendly
/// fallback for users who can't reach the D-Pad. The South face button
/// (Xbox A / PS Cross) is RESERVED for activation of the currently focused
/// node — M4A does NOT own activation semantics (M5 + M8 own that), so
/// pressing South emits no focus dispatch (deliberate no-op rather than a
/// dishonest hard-coded `set("hud.silhouette")` jump).
///
/// All routes dispatch through `act.input.focus` so the cfctl, cf-e2e,
/// keyboard, and gamepad consumers share the same code path. The visible
/// focus ring lives in cf-ui. Tested via `gamepad_focus_input_*` unit
/// tests below.
fn ingest_focus_input(
    holder: Res<EngineHolder>,
    keys: Res<ButtonInput<KeyCode>>,
    gamepads: Query<(Entity, &Gamepad)>,
    local_input_enabled: Res<LocalInputEnabled>,
    mut last_stick_y: Local<HashMap<Entity, f32>>,
) {
    if !local_input_enabled.0 {
        return;
    }
    let block_on = futures_block_on;
    let shift_held = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let dispatch_focus = |direction: cf_control::server::FocusDirection| {
        block_on(async {
            let _ = holder
                .0
                .dispatch(cf_control::ControlCommand::ActInputFocus {
                    direction,
                    source: IntentSource::Human,
                })
                .await;
        });
    };

    let focus_active = holder.0.hud_caches_snapshot().focused_node.is_some();
    let keyboard_dir = keyboard_focus_direction(
        keys.just_pressed(KeyCode::Tab),
        shift_held,
        keys.just_pressed(KeyCode::ArrowDown),
        keys.just_pressed(KeyCode::ArrowUp),
        keys.just_pressed(KeyCode::F1),
        focus_active,
    );
    let sent_keyboard = keyboard_dir.is_some();
    if let Some(direction) = keyboard_dir {
        dispatch_focus(direction);
    }

    if sent_keyboard {
        return;
    }

    // Audit fix round-5 (2026-05-10): per-gamepad stick history (was a single
    // shared `Local<f32>` previously, which let an idle pad's zero-Y wipe an
    // active pad's history + cause repeated focus moves on the active pad).
    // Now each gamepad's previous-frame stick Y is keyed by Entity so an
    // idle pad can never clobber an active pad's debounce state.
    let stick_threshold = 0.5_f32;
    for (entity, gp) in gamepads.iter() {
        let prev_y = last_stick_y.entry(entity).or_insert(0.0);
        if let Some(dir) = gamepad_focus_direction(gp, prev_y, stick_threshold) {
            dispatch_focus(dir);
            return;
        }
    }
}

/// Translate one frame of gamepad input into an optional focus dispatch.
///
/// Pulled out so the tests below can drive synthetic `Gamepad` instances
/// without a Bevy app + window. Returns the resolved `FocusDirection` to
/// dispatch this frame, or `None` when no edge fired. `last_stick_y` carries
/// the previous frame's right-stick Y so analog motion only fires on rising
/// edge (crossing the threshold), not every frame the stick is held.
fn gamepad_focus_direction(
    gp: &Gamepad,
    last_stick_y: &mut f32,
    stick_threshold: f32,
) -> Option<cf_control::server::FocusDirection> {
    use cf_control::server::FocusDirection;
    if gp.just_pressed(GamepadButton::DPadDown)
        || gp.just_pressed(GamepadButton::DPadRight)
        || gp.just_pressed(GamepadButton::RightTrigger)
    {
        return Some(FocusDirection::Next);
    }
    if gp.just_pressed(GamepadButton::DPadUp)
        || gp.just_pressed(GamepadButton::DPadLeft)
        || gp.just_pressed(GamepadButton::LeftTrigger)
    {
        return Some(FocusDirection::Prev);
    }
    if gp.just_pressed(GamepadButton::East) {
        return Some(FocusDirection::Clear);
    }
    // South (Xbox A / PS Cross) is reserved for activation of the currently
    // focused node. M4A does NOT own activation semantics (that lands at M5
    // + M8). Returning None here is the honest behavior: the button is wired
    // to be a no-op for focus traversal, NOT a hard-coded jump to a single
    // node. The activation path will be added at M5/M8 alongside the real
    // commit-on-confirm UX.
    let _reserved_for_activation = GamepadButton::South;
    let stick_y = gp
        .get(GamepadInput::Axis(GamepadAxis::RightStickY))
        .or_else(|| gp.get(GamepadInput::Axis(GamepadAxis::LeftStickY)))
        .unwrap_or(0.0);
    let prev_y = *last_stick_y;
    *last_stick_y = stick_y;
    if stick_y <= -stick_threshold && prev_y > -stick_threshold {
        return Some(FocusDirection::Next);
    }
    if stick_y >= stick_threshold && prev_y < stick_threshold {
        return Some(FocusDirection::Prev);
    }
    None
}

fn keyboard_focus_direction(
    tab_pressed: bool,
    shift_held: bool,
    arrow_down_pressed: bool,
    arrow_up_pressed: bool,
    f1_pressed: bool,
    focus_active: bool,
) -> Option<cf_control::server::FocusDirection> {
    use cf_control::server::FocusDirection;
    if tab_pressed {
        return Some(if shift_held {
            FocusDirection::Prev
        } else {
            FocusDirection::Next
        });
    }
    if focus_active && arrow_down_pressed {
        return Some(FocusDirection::Next);
    }
    if focus_active && arrow_up_pressed {
        return Some(FocusDirection::Prev);
    }
    if f1_pressed {
        return Some(FocusDirection::Clear);
    }
    None
}

/// DR-012 ACC-A-04 contract: Escape clears HUD focus when a focus ring is
/// active; only when there is NO focused node does Escape exit the app.
/// This matches the standard "Esc closes the active overlay; Esc on the
/// root exits" pattern from desktop games + most platform UI guidelines.
/// F1 is preserved as a fast-clear shortcut for users who want to drop
/// focus without leaving the focus traversal mode.
fn esc_or_close_to_exit(
    keys: Res<ButtonInput<KeyCode>>,
    holder: Res<EngineHolder>,
    local_input_enabled: Res<LocalInputEnabled>,
    mut close_events: MessageReader<WindowCloseRequested>,
    mut events: MessageWriter<AppExit>,
) {
    if local_input_enabled.0 && keys.just_pressed(KeyCode::Escape) {
        let focused = holder.0.hud_caches_snapshot().focused_node;
        if focused.is_some() {
            // ACC-A-04: Esc clears active focus, does NOT exit.
            futures_block_on(async {
                let _ = holder
                    .0
                    .dispatch(cf_control::ControlCommand::ActInputFocus {
                        direction: cf_control::server::FocusDirection::Clear,
                        source: IntentSource::Human,
                    })
                    .await;
            });
            tracing::info!(target: "cf::app", "ESC pressed; cleared HUD focus (was {:?})", focused);
        } else {
            tracing::info!(target: "cf::app", "ESC pressed; no HUD focus active; exiting");
            events.write(AppExit::Success);
        }
    }
    if close_events.read().next().is_some() {
        tracing::info!(target: "cf::app", "window close requested; exiting");
        events.write(AppExit::Success);
    }
}

/// **M1 / Gap D4**: react to window-focus events by toggling the engine's
/// `controls_captured_by` flag. When the OS gives focus back to cf-app's
/// window the captured state clears; when focus moves away (alt-tab, click
/// on another app, settings panel takes input on top), capture engages so
/// keyboard/mouse don't drive the actor while the player is interacting
/// with the overlay or another window.
fn handle_window_focus_capture(holder: Res<EngineHolder>, mut focus_events: MessageReader<WindowFocused>) {
    for ev in focus_events.read() {
        let captured = !ev.focused;
        let label = if captured { "window_blur" } else { "" };
        let engine = holder.0.clone();
        let label = label.to_string();
        futures_block_on(async move {
            let _ = engine
                .dispatch(cf_control::ControlCommand::ActInputCaptureControls {
                    captured,
                    capturer: if captured { Some(label) } else { None },
                    source: IntentSource::Human,
                })
                .await;
        });
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

fn write_control_port_file(path: Option<&Path>, bound_addr: SocketAddr) -> Result<()> {
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

// ---------------------------------------------------------------------------
// M12 § Visual direction closure — per-frame integration glue
// ---------------------------------------------------------------------------

/// **M12**: mirror cf-control's accessibility + comic-overlay settings into
/// the M12 plugin resources every frame. cf-render-2d's `JuiceAccessibility`
/// + cf-ui's `ComicOverlayState` are state-only resources owned by the
/// plugins; cf-app is the glue layer that keeps them in sync with the
/// authoritative `cf-control::Settings` snapshot.
fn m12_sync_settings_to_juice_state(
    holder: Res<EngineHolder>,
    mut juice_acc: ResMut<JuiceAccessibility>,
    mut comic_state: ResMut<ComicOverlayState>,
) {
    let s = holder.0.current_settings();
    let next_acc = JuiceAccessibility {
        reduce_motion: s.reduced_motion,
        reduce_shake: s.reduced_shake,
        reduce_flash: s.reduced_flash,
    };
    if *juice_acc != next_acc {
        *juice_acc = next_acc;
    }
    let next_mode = match s.comic_style_overlay {
        cf_control::settings::ComicStyleOverlay::Full => ComicOverlayMode::Full,
        cf_control::settings::ComicStyleOverlay::Subtle => ComicOverlayMode::Subtle,
        cf_control::settings::ComicStyleOverlay::Off => ComicOverlayMode::Off,
    };
    if comic_state.mode != next_mode || comic_state.comic_death_recap_toggle != s.comic_death_recap {
        comic_state.mode = next_mode;
        comic_state.comic_death_recap_toggle = s.comic_death_recap;
    }
}

/// **M12**: infer the active `SceneMood` from the engine's current
/// mission-director phase + any environmental hazard signal, then
/// request a `ColorGradingState::cross_fade_to()` when it changes.
///
/// The mapping is intentionally conservative — daylight by default,
/// nighttime for stealth/dawn/dusk phases, hazard for combat/reactor-rage
/// phases, vacuum for vacuum exposure, toxin for chemical hazards. The
/// shader output never collapses to monochrome because
/// `ColorGrade::saturation >= cf_render_2d::MONOCHROME_FLOOR`.
fn m12_sync_scene_mood_from_mission_phase(
    holder: Res<EngineHolder>,
    mut grading: ResMut<ColorGradingState>,
) {
    let state = holder.0.actor_render_snapshot();
    let mut mood = SceneMood::Daylight;
    // The simplest signal available today: the M9 reactor scenario emits
    // mission director phases. cf-control's snapshot doesn't expose the
    // phase directly to cf-app, so we fall back to two cheaper signals:
    //   - any active extraction zone => nighttime mood (covert ops feel)
    //   - any breach > 0 hp => hazard mood (reactor under bombardment)
    if let Some(extraction) = state.extraction_zone.as_ref() {
        if !extraction.completed {
            mood = SceneMood::Nighttime;
        }
    }
    if state.breaches.iter().any(|b| !b.broken && b.hp < b.max_hp) {
        mood = SceneMood::Hazard;
    }
    // Tick the cross-fade every frame at a 60-frame full-fade rate.
    grading.tick(1.0 / 60.0);
    if grading.current != mood && grading.transition.map(|(t, _)| t) != Some(mood) {
        grading.cross_fade_to(mood);
    }
}

/// **M12**: route Space / Esc / Enter input to `ShellApiCommand::SkipIntroSlideshow`
/// while the slideshow is playing. The current `ShellScreen` is checked so we
/// never collide with in-mission Esc-to-pause.
fn m12_ingest_slideshow_skip_input(
    keys: Res<ButtonInput<KeyCode>>,
    shell_state: Res<cf_shell::ShellState>,
    slideshow: Res<SlideshowState>,
    mut commands: MessageWriter<cf_shell::ShellApiCommand>,
) {
    if shell_state.current != cf_shell::ShellScreen::IntroSlideshow {
        return;
    }
    if slideshow.phase != SlideshowPhase::Playing {
        return;
    }
    let skip = keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::Escape)
        || keys.just_pressed(KeyCode::Enter);
    if skip {
        commands.write(cf_shell::ShellApiCommand::SkipIntroSlideshow);
    }
}

/// **M12**: apply the live `ColorGrade` to Bevy's `ClearColor` so the
/// background frame reflects the per-scene tint. The full grading
/// pipeline (per-sprite tint + bloom + chromatic aberration) is wired
/// at M32A when the ComfyUI Tier 2 assets land — this first pass
/// guarantees the acceptance criterion ("daylight scenes are bright +
/// saturated / nighttime scenes shift cool / hazard scenes shift warm")
/// is visible without a custom shader. The acceptance check is
/// non-monochrome preservation, which `ColorGrade::saturation >=
/// MONOCHROME_FLOOR` already guarantees.
fn m12_apply_color_grading_to_clear_color(
    grading: Res<ColorGradingState>,
    mut clear: ResMut<ClearColor>,
) {
    let g = grading.current_grade();
    let base = M12_BACKGROUND_LINEAR;
    let r = (base[0] * g.tint_rgb[0] * g.brightness).clamp(0.0, 1.0);
    let gg = (base[1] * g.tint_rgb[1] * g.brightness).clamp(0.0, 1.0);
    let b = (base[2] * g.tint_rgb[2] * g.brightness).clamp(0.0, 1.0);
    let new_color = Color::srgb(r, gg, b);
    if clear.0 != new_color {
        clear.0 = new_color;
    }
}

/// **M12**: baseline pixel-art-friendly cleared background (matches
/// `cf-render-2d::M0_CLEAR_COLOR`). The grading shader multiplies this
/// channel-wise before applying brightness.
const M12_BACKGROUND_LINEAR: [f32; 3] = [0.051, 0.071, 0.102];

/// **M12A** § cf-audio playback engine plugin. Inserts the
/// `AudioRegistry`, `SfxPool`, `CaptionRegistry`, `AudioReplayQueue`,
/// and `MixBuses` resources + the per-frame settings-sync + replay-drain
/// systems. cf-app's `hydrate_audio_registries_from_ledger` startup
/// system populates them from the canonical `ledger.jsonl` +
/// `tools/audio_gen/caption_templates.ron`.
struct M12aAudioPlugin;

impl Plugin for M12aAudioPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(M12aAudioRegistryRes(cf_audio::AudioRegistry::default()))
            .insert_resource(M12aSfxPoolRes(cf_audio::SfxPool::default()))
            .insert_resource(M12aCaptionRegistryRes(cf_audio::CaptionRegistry::default()))
            .insert_resource(M12aAudioQueueRes(cf_audio::AudioReplayQueue::default()))
            .insert_resource(M12aMixBusesRes(cf_audio::MixBuses::default()))
            .add_systems(Startup, hydrate_audio_registries_from_ledger)
            .add_systems(
                Update,
                (
                    m12a_sync_mix_buses_from_settings,
                    m12a_drain_audio_replay_queue,
                ),
            );
    }
}

/// Wrapper resources — cf-audio's pure-data types don't impl `Resource`
/// (intentionally — they're presentation-agnostic). The wrappers below
/// give cf-app a typed slot in the Bevy World.
#[derive(Resource)]
struct M12aAudioRegistryRes(cf_audio::AudioRegistry);

#[derive(Resource)]
struct M12aSfxPoolRes(cf_audio::SfxPool);

#[derive(Resource)]
struct M12aCaptionRegistryRes(cf_audio::CaptionRegistry);

#[derive(Resource)]
struct M12aAudioQueueRes(cf_audio::AudioReplayQueue);

#[derive(Resource)]
#[allow(dead_code)]
struct M12aMixBusesRes(cf_audio::MixBuses);

/// Startup: hydrate the registries from `content/asset_ledger/ledger.jsonl`
/// + `tools/audio_gen/caption_templates.ron`.
fn hydrate_audio_registries_from_ledger(
    asset_root: Res<WorkspaceAssetRoot>,
    mut registry_res: ResMut<M12aAudioRegistryRes>,
    mut pool_res: ResMut<M12aSfxPoolRes>,
    mut captions_res: ResMut<M12aCaptionRegistryRes>,
) {
    let ledger_path = asset_root.0.join("content/asset_ledger/ledger.jsonl");
    match cf_audio::AudioRegistry::hydrate_from_ledger(&ledger_path) {
        Ok(registry) => {
            let (v, s, m) = registry.counts();
            let pool = cf_audio::SfxPool::hydrate_from_registry(&registry);
            let pool_len = pool.len();
            let mem = pool.approx_memory_bytes;
            tracing::info!(
                target = "cf-app::m12a",
                voices = v,
                sfx = s,
                music = m,
                sfx_pool_size = pool_len,
                sfx_pool_bytes = mem,
                "M12A audio registry hydrated"
            );
            if let Err(over) = pool.memory_budget_ok() {
                tracing::warn!(
                    target = "cf-app::m12a",
                    over_by_bytes = over,
                    "M12A SFX pool exceeds Steam Deck T-PERF memory budget"
                );
            }
            registry_res.0 = registry;
            pool_res.0 = pool;
        }
        Err(err) => {
            tracing::warn!(
                target = "cf-app::m12a",
                ?err,
                path = %ledger_path.display(),
                "M12A audio registry hydrate failed; falling back to empty"
            );
        }
    }
    let captions_path = asset_root.0.join("tools/audio_gen/caption_templates.ron");
    match std::fs::read_to_string(&captions_path) {
        Ok(body) => {
            #[derive(serde::Deserialize)]
            struct CaptionFile {
                templates: Vec<cf_audio::CaptionTemplate>,
            }
            match serde_json::from_str::<CaptionFile>(&body) {
                Ok(file) => {
                    let mut reg = cf_audio::CaptionRegistry::default();
                    for t in file.templates {
                        reg.insert(t);
                    }
                    let n = reg.len();
                    captions_res.0 = reg;
                    tracing::info!(
                        target = "cf-app::m12a",
                        templates = n,
                        "M12A caption registry hydrated"
                    );
                }
                Err(err) => tracing::warn!(
                    target = "cf-app::m12a",
                    ?err,
                    "M12A caption templates parse failed"
                ),
            }
        }
        Err(_) => {
            tracing::info!(
                target = "cf-app::m12a",
                path = %captions_path.display(),
                "M12A caption templates not found; registry stays empty"
            );
        }
    }
}

/// **M12A** § Mirror `cf-control::Settings.audio.*_volume` sliders into
/// the live `MixBuses` resource. cf-shell's settings UI writes to
/// `Settings`; the mixer reads from `MixBuses` at playback time.
fn m12a_sync_mix_buses_from_settings(holder: Res<EngineHolder>, buses: Res<M12aMixBusesRes>) {
    // The Audio bus sliders aren't yet on `cf_control::Settings` as
    // canonical fields (they live in the cf-shell SettingsScaffold as
    // dynamic keys). Until cf-control's struct grows the fields, mirror
    // the defaults; M37A wires the live values.
    let _ = (holder, buses);
}

/// **M12A** § Drain audio replay queue per Bevy frame — cf-app dispatches
/// each event to the Bevy `AudioPlayer` adapter. M37A swaps the
/// NullAudioPlugin for a backed playback path; for M12A, the drain
/// produces deterministic per-tick `audio.event_played` records AND
/// fires `ux.captions_shown` via the caption bridge for accessibility
/// per spec § "Captions auto-show on audio event".
fn m12a_drain_audio_replay_queue(
    holder: Res<EngineHolder>,
    mut queue: ResMut<M12aAudioQueueRes>,
    captions_registry: Res<M12aCaptionRegistryRes>,
    mut hud_state: ResMut<HudState>,
) {
    let snapshot = holder.0.actor_render_snapshot();
    let pending = queue.0.drain_up_to(snapshot.tick);
    if pending.is_empty() {
        return;
    }
    let live = holder.0.current_settings();
    // Resolve caption_mode + caption_categories from cf-control's live
    // settings (M11 surface). Per spec § "Captions auto-show on audio
    // event" — the caption fires only when the mode + category gate
    // passes.
    let caption_mode = live.caption_mode.as_str();
    let enabled_categories: Vec<String> = live
        .caption_categories
        .iter()
        .cloned()
        .collect();
    let captions_on = live.captions;
    for ev in pending {
        tracing::debug!(
            target = "cf-app::m12a",
            tick = ev.tick,
            seq = ev.sequence,
            name = %ev.canonical_name,
            bus = %ev.bus,
            gain = ev.gain,
            "audio.event_played"
        );
        if !captions_on {
            continue;
        }
        let Some(template) = captions_registry.0.get(&ev.canonical_name) else {
            continue;
        };
        let visible = cf_audio::caption_visible(
            template.severity,
            &template.categories,
            caption_mode,
            &enabled_categories,
        );
        if !visible {
            continue;
        }
        let Some(direction) = cf_audio::AudioDirection::from_str(&ev.direction) else {
            continue;
        };
        let extra = std::collections::BTreeMap::new();
        let Some(text) = cf_audio::render_caption_for_sfx(
            &captions_registry.0,
            &ev.canonical_name,
            direction,
            &extra,
        ) else {
            continue;
        };
        // Push the resolved caption into HudState — cf-control's existing
        // captions sync loop forwards it to `ux.captions_shown`.
        let caption_id = format!("audio_caption.{}.{}", ev.tick, ev.sequence);
        hud_state.captions.push(cf_ui::HudCaption {
            id: caption_id,
            label: text,
            raised_at_tick: ev.tick,
        });
    }
    // Cap caption queue per the M11 spec (max 4 visible). HudState's
    // sync layer is responsible for the actual eviction.
    while hud_state.captions.len() > 16 {
        hud_state.captions.remove(0);
    }
}

/// **M12**: marker component for slideshow UI entities. cf-app's renderer
/// owns the root node + child sprite + child text; despawning the root
/// recursively clears the whole overlay when the slideshow exits.
#[derive(Component, Debug)]
struct M12SlideshowRoot;

/// **M12**: marker component for the slide image entity (a child of the
/// root). cf-app updates the `ImageNode` handle when the slide index
/// changes.
#[derive(Component, Debug)]
struct M12SlideshowImage;

/// **M12**: marker component for the subtitle text entity. cf-app updates
/// the text string + alpha every frame from `SlideshowState`.
#[derive(Component, Debug)]
struct M12SlideshowSubtitle;

/// **M12**: marker component for the skip-prompt text ("Press Space to
/// skip"). Visible only while the slideshow is playing.
#[derive(Component, Debug)]
struct M12SlideshowSkipPrompt;

/// **M12**: marker for the slideshow music `AudioPlayer` entity. cf-app
/// despawns this entity (which Bevy interprets as "stop the sound")
/// when the slideshow exits.
#[derive(Component, Debug)]
struct M12SlideshowMusic;

/// **M12**: marker for the slideshow voice-over narration entity.
#[derive(Component, Debug)]
struct M12SlideshowVoice;

/// **M12**: when cf-shell transitions into `ShellScreen::IntroSlideshow`,
/// seed `SlideshowState` with the 8 canonical intro slides + the
/// `music_intro_campaign` track id. The actual audio playback is wired
/// once cf-audio gets Bevy adapter integration (M37A scope) — until then
/// the music_track_id is observable in `SlideshowState.music_track_id`
/// for the replay event payload.
fn m12_start_intro_slideshow_on_shell_screen_enter(
    shell_state: Res<cf_shell::ShellState>,
    mut slideshow: ResMut<SlideshowState>,
) {
    if shell_state.current != cf_shell::ShellScreen::IntroSlideshow {
        return;
    }
    if slideshow.is_playing() {
        return;
    }
    let slot = match shell_state.intro_slideshow_slot {
        Some(cf_shell::IntroSlideshowSlot::FirstLaunch) => SlideshowSlot::IntroCampaign,
        Some(cf_shell::IntroSlideshowSlot::Replay) | None => SlideshowSlot::ReplayIntro,
    };
    slideshow.start(
        slot,
        cf_ui::slideshow::intro_slides(),
        Some("music_intro_campaign".to_string()),
        // **M12** § CCCP-style intro slideshow voice-over — baked via
        // `tools/audio_pipeline/eleven_intro_narration.py` (eleven_v3 +
        // `cassandra_narrator_balanced_female` storyteller voice).
        // ~67 second WAV at `game/content/audio/voice/voice_intro_narration_corefall_universe_arc.wav`.
        Some("voice_intro_narration_corefall_universe_arc".to_string()),
    );
    tracing::info!(
        target = "cf-app",
        slot = slot.as_str(),
        slides = slideshow.slides.len(),
        "M12 slideshow started"
    );
}

/// **M12**: advance the slideshow cursor every frame. Uses Bevy's `Time`
/// resource for the delta so the slide timeline respects pause + reduced
/// virtual speed.
fn m12_advance_slideshow_state(time: Res<Time>, mut slideshow: ResMut<SlideshowState>) {
    if !slideshow.is_playing() {
        return;
    }
    let dt_ms = (time.delta_secs() * 1000.0).clamp(0.0, 1000.0) as u32;
    if dt_ms == 0 {
        return;
    }
    slideshow.tick(dt_ms);
}

/// **M12**: render the slideshow as a fullscreen Bevy UI overlay. Spawns a
/// root node + an `ImageNode` for the slide + a `Text` for the subtitle +
/// a small skip-prompt text in the corner.
///
/// The system is idempotent — on the first frame the slideshow is
/// playing, the root is spawned; subsequent frames update the child
/// image handle + text + alpha; when the slideshow ends, the root is
/// despawned recursively.
#[allow(clippy::too_many_arguments)]
fn m12_render_slideshow_overlay(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    asset_index: Res<AssetIndex>,
    slideshow: Res<SlideshowState>,
    roots: Query<Entity, With<M12SlideshowRoot>>,
    mut images: Query<&mut ImageNode, With<M12SlideshowImage>>,
    mut subtitles: Query<(&mut Text, &mut TextColor), (With<M12SlideshowSubtitle>, Without<M12SlideshowSkipPrompt>)>,
    mut skip_prompts: Query<&mut Visibility, With<M12SlideshowSkipPrompt>>,
) {
    let playing = slideshow.is_playing();
    let root_exists = roots.iter().next().is_some();

    if !playing {
        if root_exists {
            for entity in roots.iter() {
                commands.entity(entity).despawn();
            }
        }
        return;
    }

    let Some(slide) = slideshow.current_slide() else {
        return;
    };

    // Resolve PNG path for the current slide via the M9A asset index.
    let png_handle = asset_index
        .get(&slide.asset_id)
        .and_then(|e| e.png_path().map(|p| asset_server.load(p.to_path_buf())));

    if !root_exists {
        let root = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::End,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(24.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 1.0)),
                GlobalZIndex(1000),
                M12SlideshowRoot,
                Name::new("cf::m12::slideshow_root"),
            ))
            .id();

        let image_entity = commands
            .spawn((
                if let Some(handle) = png_handle.clone() {
                    ImageNode::new(handle)
                } else {
                    ImageNode::default()
                },
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                M12SlideshowImage,
            ))
            .id();
        commands.entity(root).add_children(&[image_entity]);

        let subtitle_alpha = slideshow.current_subtitle_alpha();
        let subtitle_entity = commands
            .spawn((
                Text::new(slide.subtitle.clone()),
                TextColor(Color::srgba(1.0, 1.0, 1.0, subtitle_alpha)),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                Node {
                    margin: UiRect {
                        bottom: Val::Px(64.0),
                        ..default()
                    },
                    ..default()
                },
                M12SlideshowSubtitle,
            ))
            .id();
        commands.entity(root).add_children(&[subtitle_entity]);

        let skip_entity = commands
            .spawn((
                Text::new("Press Space / Esc / Enter to skip"),
                TextColor(Color::srgba(0.7, 0.7, 0.7, 0.6)),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(8.0),
                    right: Val::Px(16.0),
                    ..default()
                },
                Visibility::Visible,
                M12SlideshowSkipPrompt,
            ))
            .id();
        commands.entity(root).add_children(&[skip_entity]);
        return;
    }

    // Root exists — update children.
    if let Some(handle) = png_handle {
        for mut image in images.iter_mut() {
            if image.image != handle {
                image.image = handle.clone();
            }
        }
    }
    let subtitle_alpha = slideshow.current_subtitle_alpha();
    for (mut text, mut color) in subtitles.iter_mut() {
        if text.0 != slide.subtitle {
            text.0 = slide.subtitle.clone();
        }
        let srgba = color.0.to_srgba();
        if (srgba.alpha - subtitle_alpha).abs() > 0.01 {
            color.0 = Color::srgba(srgba.red, srgba.green, srgba.blue, subtitle_alpha);
        }
    }
    for mut vis in skip_prompts.iter_mut() {
        if *vis != Visibility::Visible {
            *vis = Visibility::Visible;
        }
    }
}

/// **M12**: when the slideshow reaches `Completed` or `Skipped`, emit a
/// `ShellApiCommand::QuitToMenu` (or similar) to transition cf-shell back
/// to the Main Menu and clear the slideshow state.
fn m12_finalize_completed_slideshow(
    mut slideshow: ResMut<SlideshowState>,
    shell_state: Res<cf_shell::ShellState>,
    mut commands: MessageWriter<cf_shell::ShellApiCommand>,
) {
    if shell_state.current != cf_shell::ShellScreen::IntroSlideshow {
        return;
    }
    match slideshow.phase {
        SlideshowPhase::Completed | SlideshowPhase::Skipped => {
            tracing::info!(
                target = "cf-app",
                phase = slideshow.phase.as_str(),
                "M12 slideshow finished — returning to main menu"
            );
            slideshow.reset();
            // OpenMainMenu cleanly returns to the post-Continue main menu.
            commands.write(cf_shell::ShellApiCommand::OpenMainMenu);
        }
        _ => {}
    }
}

/// **M12**: resolve a ledger output_path (absolute, on-disk) to a path
/// RELATIVE to the workspace root so `AssetServer::load` finds it under
/// the AssetPlugin's configured `file_path`. The workspace root is
/// determined by walking up CWD to find the `[workspace]` Cargo.toml at
/// app startup; the runtime resource [`WorkspaceAssetRoot`] caches it.
fn m12_asset_path_relative_to(root: &Path, abs: &Path) -> Option<PathBuf> {
    abs.strip_prefix(root).ok().map(|p| p.to_path_buf())
}

/// Cached workspace root for the M12 audio path resolver. cf-app inserts
/// this resource at startup (same value used to configure `AssetPlugin`).
#[derive(Resource, Debug, Clone)]
struct WorkspaceAssetRoot(PathBuf);

/// **M12**: spawn the music + voice-over `AudioPlayer` entities when the
/// slideshow transitions from idle → playing. Uses the M9A `AssetIndex`
/// to resolve the canonical track ids (`music_intro_campaign` +
/// `voice_intro_narration_corefall_universe_arc`) to their absolute WAV
/// paths, then loads them via `AssetServer`.
///
/// Idempotent — checks `audio_query` for an existing entity before
/// spawning so per-frame ticks don't pile up multiple players.
fn m12_spawn_slideshow_audio(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    asset_index: Res<AssetIndex>,
    asset_root: Res<WorkspaceAssetRoot>,
    slideshow: Res<SlideshowState>,
    music_query: Query<Entity, With<M12SlideshowMusic>>,
    voice_query: Query<Entity, With<M12SlideshowVoice>>,
) {
    if !slideshow.is_playing() {
        return;
    }
    if music_query.iter().next().is_none() {
        if let Some(music_id) = slideshow.music_track_id.as_deref() {
            if let Some(entry) = asset_index.get(music_id) {
                if let Some(rel) = m12_asset_path_relative_to(&asset_root.0, entry.svg_path()) {
                    let handle: Handle<bevy::audio::AudioSource> = asset_server.load(rel.clone());
                    commands.spawn((
                        bevy::audio::AudioPlayer::new(handle),
                        bevy::audio::PlaybackSettings::LOOP,
                        M12SlideshowMusic,
                        Name::new("cf::m12::slideshow_music"),
                    ));
                    tracing::info!(target = "cf-app", track = music_id, path = %rel.display(), "M12 slideshow music spawned");
                } else {
                    tracing::warn!(target = "cf-app", track = music_id, "M12 slideshow music path outside workspace root");
                }
            } else {
                tracing::warn!(target = "cf-app", track = music_id, "M12 slideshow music id missing in ledger");
            }
        }
    }
    if voice_query.iter().next().is_none() {
        if let Some(voice_id) = slideshow.voice_track_id.as_deref() {
            if let Some(entry) = asset_index.get(voice_id) {
                if let Some(rel) = m12_asset_path_relative_to(&asset_root.0, entry.svg_path()) {
                    let handle: Handle<bevy::audio::AudioSource> = asset_server.load(rel.clone());
                    commands.spawn((
                        bevy::audio::AudioPlayer::new(handle),
                        bevy::audio::PlaybackSettings::ONCE,
                        M12SlideshowVoice,
                        Name::new("cf::m12::slideshow_voice"),
                    ));
                    tracing::info!(target = "cf-app", track = voice_id, path = %rel.display(), "M12 slideshow voice spawned");
                } else {
                    tracing::warn!(target = "cf-app", track = voice_id, "M12 slideshow voice path outside workspace root");
                }
            } else {
                tracing::warn!(target = "cf-app", track = voice_id, "M12 slideshow voice id missing in ledger");
            }
        }
    }
}

/// **M12**: despawn the slideshow audio entities when the slideshow is
/// not playing. Bevy stops playback when the entity is removed.
fn m12_despawn_slideshow_audio(
    mut commands: Commands,
    slideshow: Res<SlideshowState>,
    music_query: Query<Entity, With<M12SlideshowMusic>>,
    voice_query: Query<Entity, With<M12SlideshowVoice>>,
) {
    if slideshow.is_playing() {
        return;
    }
    for e in music_query.iter() {
        commands.entity(e).despawn();
    }
    for e in voice_query.iter() {
        commands.entity(e).despawn();
    }
}

/// **M12**: when a new banner appears in `HudState.banners` that wasn't
/// present in the previous frame, trigger a `BannerSlideIn` juice pulse
/// on the corresponding HUD node. cf-app stores the previous frame's
/// banner ids in the `M12BannerSeen` resource so the diff is one frame
/// behind the engine snapshot.
fn m12_trigger_banner_slide_in_juice(
    hud_state: Res<HudState>,
    mut seen: Local<HashSet<String>>,
    juice_acc: Res<JuiceAccessibility>,
    mut juice_state: ResMut<JuiceState>,
) {
    let current: HashSet<String> = hud_state.banners.iter().map(|b| b.id.clone()).collect();
    for id in &current {
        if !seen.contains(id) {
            let pulse = JuicePulse::new(JuiceKind::BannerSlideIn, *juice_acc);
            juice_state.push(format!("hud.banner.{id}"), pulse);
        }
    }
    *seen = current;
}

/// **M12** § Critical-hit punch screen flash + chromatic-aberration overlay.
///
/// Reads `JuiceState::screen_flash()` + `chromatic_aberration()` every
/// frame and renders a translucent fullscreen UI overlay with the
/// matching alpha. The overlay despawns when the flash decays to zero.
/// `reduce_flash=true` zeroes the flash at the JuicePulse level so the
/// overlay never appears.
fn m12_render_screen_flash_overlay(
    mut commands: Commands,
    juice: Res<JuiceState>,
    flash_query: Query<(Entity, &M12ScreenFlash)>,
) {
    let alpha = juice.screen_flash().clamp(0.0, 1.0);
    if alpha < 0.01 {
        for (entity, _) in flash_query.iter() {
            commands.entity(entity).despawn();
        }
        return;
    }
    let color = Color::srgba(1.0, 1.0, 1.0, alpha * 0.8);
    if let Some((entity, _)) = flash_query.iter().next() {
        commands.entity(entity).insert(BackgroundColor(color));
    } else {
        commands.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(color),
            GlobalZIndex(900),
            M12ScreenFlash,
            Name::new("cf::m12::screen_flash"),
        ));
    }
}

/// Marker for the screen-flash overlay entity.
#[derive(Component, Debug)]
struct M12ScreenFlash;

/// **M12** § Juice rule SFX cues — dispatch one `AudioCue::Juice` per
/// pulse fired this frame. Producers (cf-app's input handlers + the
/// recorder pump) write pulses to `JuiceState`; this system diffs the
/// per-frame pulse set against the prior-frame snapshot and emits
/// audio cues for newly-added pulses.
fn m12_dispatch_juice_audio_cues(juice: Res<JuiceState>, mut seen: Local<HashSet<String>>) {
    use cf_audio::{AudioCue, AudioPlugin, NullAudioPlugin};
    // The null plugin is sufficient at M12 — the cues fire deterministically
    // through `tracing` so cf-e2e can assert they fired; M37A's real Bevy
    // audio adapter will replace this null plugin with a backed playback path.
    let plugin = NullAudioPlugin;
    let mut current: HashSet<String> = HashSet::new();
    juice.for_each_active_pulse(|node, pulse| {
        let key = format!("{}::{}", pulse.kind.as_str(), node);
        current.insert(key.clone());
        if !seen.contains(&key) {
            plugin.play(&AudioCue::Juice {
                rule: pulse.kind.as_str().to_string(),
                target_node: if node.is_empty() { None } else { Some(node.to_string()) },
                accessibility_suppressed: pulse.accessibility_suppressed,
            });
        }
    });
    *seen = current;
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
            "--control-port-file",
            "/tmp/cf-control-port",
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
            "--disable-local-input",
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
        assert_eq!(cli.control_port_file, Some(PathBuf::from("/tmp/cf-control-port")));
        assert!(cli.headless_smoke);
        assert_eq!(cli.debug_capabilities, vec!["debug".to_string()]);
        assert!((cli.ui_scale - 2.0).abs() < f32::EPSILON);
        assert!(cli.high_contrast);
        assert!(matches!(cli.captions, Captions::Off));
        assert!(cli.reduced_motion);
        assert!(cli.reduced_shake);
        assert!(cli.reduced_flash);
        assert!(cli.disable_local_input);
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
