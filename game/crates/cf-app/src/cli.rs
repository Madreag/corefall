use std::path::PathBuf;

use cf_capture::{CaptureConfig, CaptureMode};
use clap::Parser;

#[derive(Debug, Clone, clap::ValueEnum)]
pub(crate) enum Captions {
    On,
    Off,
}

impl Captions {
    pub(crate) fn as_bool(&self) -> bool {
        matches!(self, Captions::On)
    }
}

#[derive(Debug, Parser, Clone)]
#[command(
    name = "cf-app",
    about = "Corefall native app shell. Bevy app + cf-render-2d clear-screen + fixed-tick sim + cf-control loopback API."
)]
pub(crate) struct Cli {
    /// Scenario id. **Defaults to `m1_actor_range`** when the binary is launched
    /// with no `--scenario` flag — enables Finder/Explorer/Files double-click
    /// playability per the AGENTS.md Double-Click Playability Hard Gate. The
    /// default ships an actor + rifle + ground floor so the player sees a live
    /// game on launch; pass `--scenario m0_blank` for headless tests.
    #[arg(long, default_value = "m1_actor_range")]
    pub(crate) scenario: String,
    #[arg(long)]
    pub(crate) seed: Option<u64>,
    #[arg(long)]
    pub(crate) run_seconds: Option<f32>,
    #[arg(long)]
    pub(crate) ticks: Option<u64>,
    #[arg(long, default_value_t = 60)]
    pub(crate) tick_rate_hz: u32,
    #[arg(long)]
    pub(crate) write_run_bundle: bool,
    #[arg(long)]
    pub(crate) run_bundle_dir: Option<PathBuf>,
    #[arg(long)]
    pub(crate) control_api: bool,
    #[arg(long, default_value_t = 17890u16)]
    pub(crate) control_port: u16,
    #[arg(long)]
    pub(crate) control_uds: Option<PathBuf>,
    /// Write the actual bound control API port to this file after the listener
    /// is live. Used by cf-e2e with `--control-port 0` so the OS, not the
    /// harness, owns ephemeral port allocation.
    #[arg(long)]
    pub(crate) control_port_file: Option<PathBuf>,
    /// Skip window creation; runs the sim loop only. Useful for CI/scripted smoke.
    #[arg(long)]
    pub(crate) headless_smoke: bool,
    #[arg(long, value_delimiter = ',')]
    pub(crate) debug_capabilities: Vec<String>,
    #[arg(long, default_value_t = 1.0)]
    pub(crate) ui_scale: f32,
    #[arg(long)]
    pub(crate) high_contrast: bool,
    #[arg(long, value_enum, default_value_t = Captions::On)]
    pub(crate) captions: Captions,
    #[arg(long)]
    pub(crate) reduced_motion: bool,
    #[arg(long)]
    pub(crate) reduced_shake: bool,
    #[arg(long)]
    pub(crate) reduced_flash: bool,
    /// Automation mode for cf-e2e/cfctl-driven captures. When set, the Bevy
    /// window still renders, but keyboard/gamepad/escape input from the local
    /// desktop cannot inject player/focus commands into the control script.
    #[arg(long)]
    pub(crate) disable_local_input: bool,
    /// as many sim ticks per Bevy frame as the engine's clock budget allows
    /// (capped at 1024 per frame). cf-e2e passes this for cfctl scripts whose
    /// total sim ticks exceed the wall-clock window cf-e2e's default 180s
    /// timeout allows. Determinism is preserved because the sim is still
    /// deterministic per tick; only the wall-clock pacing changes.
    #[arg(long)]
    pub(crate) unpaced: bool,
    /// M4A: ACC-A-05 hold-to-press alternative for tap-to-press actions.
    #[arg(long)]
    pub(crate) hold_to_confirm: bool,
    /// M4A: ACC-A-05 hold threshold in milliseconds (50..2000).
    #[arg(long, default_value_t = 250)]
    pub(crate) hold_threshold_ms: u32,
    /// M4A: ACC-A-05 future remap UI surface flag (M8 ships the table editor).
    #[arg(long)]
    pub(crate) key_remap_enabled: bool,
    /// M3A: override the determinism checksum cadence (ticks between sim_checksum events).
    /// Default: 60. Set 0 to disable checksums.
    #[arg(long)]
    pub(crate) checksum_cadence_ticks: Option<u64>,
    /// `snapshot.baseline_emitted` events. Default 600 (10 s @ 60 Hz);
    /// 0 disables snapshot emission entirely. Lower values give more
    /// frequent baselines (faster reconstruction, larger bundle); higher
    /// values amortize over deltas (smaller bundle, slower reconstruction).
    #[arg(long)]
    pub(crate) delta_baseline_cadence_ticks: Option<u64>,
    /// recorder runs in chain mode: every event carries `prev_event_hash`
    /// + `chained_hash_hex`, and `RunManifest.ledger_chain_anchor` is the
    /// BLAKE3 chain anchor of the final event. Tournament organizers
    /// publish the anchor + the bundle so third parties can verify
    /// tamper-evidence via `cf-mod ledger verify --bundle`.
    #[arg(long)]
    pub(crate) ledger_chain: bool,
    /// **DEBUG-ONLY**: spawn a sub-thread that panics at the configured tick. Used to
    /// capture `system.panic` evidence in a real run bundle (M0-008 / M0.2-F5).
    /// Production runs should never set this.
    #[arg(long)]
    pub(crate) debug_inject_panic_at_tick: Option<u64>,
    /// T-CAPTURE: enable cf-capture frame readback. Defaults to off; pass with no value to
    /// turn on the windowed swapchain capture path at the default 10 Hz cadence.
    #[arg(long)]
    pub(crate) capture_grid: bool,
    /// T-CAPTURE baseline cadence. 10 Hz default = capture every 6 ticks at 60 Hz tick.
    /// Lower values reduce disk + LLM-input pressure; higher values increase motion fidelity.
    #[arg(long, default_value_t = 10.0)]
    pub(crate) capture_frames_hz: f32,
    /// T-CAPTURE: when present, suppress event-triggered keyframes (mission_*, terrain_carved,
    /// projectile_hit, actor_status_changed, weapon_fired, ai.state_changed, system.panic).
    /// Default is keyframes ON.
    #[arg(long)]
    pub(crate) no_capture_events: bool,
    /// T-CAPTURE: switch to offscreen RenderTarget::Image readback (true headless mode without
    /// an OS window). Currently scope-limited; the flag is accepted but the actual offscreen
    /// path is logged-only until the BP2 closure pass lands the wgpu readback wiring.
    /// Use windowed-hidden mode (default) for now.
    #[arg(long)]
    pub(crate) headless_capture: bool,
    /// intent label above every reactive guard's world position with the
    /// guard's current state + tactic ("ALERT: heard_shot", "ENGAGED",
    /// "RELOADING"). The label updates every tick. Without the flag the
    /// overlay is hidden. Acceptance criterion 'AI debug labels'.
    #[arg(long)]
    pub(crate) ai_debug: bool,
    /// the caller expects from this run. The canonical run-bundle checker
    /// (`prototype_run_check.py`) verifies that the actual outcome matches
    /// (`clean` requires exactly one `system.run_finished` + zero
    /// `system.panic`; `panic` requires at least one `system.panic`; `abort`
    /// is permissive). When omitted, defaults to `clean` (the M3A-005
    /// default). Used by cfctl scripts that intentionally produce panic /
    /// abort bundles to prove the checker rejects mismatches.
    #[arg(long, value_enum)]
    pub(crate) expected_outcome: Option<ExpectedOutcomeArg>,
}

/// CLI projection of `cf_replay::ExpectedOutcome` so clap can parse a string
/// value into the manifest enum without exposing cf-replay's serde wrapper
/// directly.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub(crate) enum ExpectedOutcomeArg {
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
pub(crate) struct CaptureOptions {
    pub(crate) enabled: bool,
    pub(crate) frames_hz: f32,
    pub(crate) event_keyframes: bool,
    pub(crate) headless: bool,
}

impl CaptureOptions {
    pub(crate) fn from_cli(cli: &Cli) -> Self {
        Self {
            enabled: cli.capture_grid,
            frames_hz: cli.capture_frames_hz,
            event_keyframes: !cli.no_capture_events,
            headless: cli.headless_capture,
        }
    }

    pub(crate) fn build_config(&self, output_dir: PathBuf, runtime_tick_rate_hz: u32) -> CaptureConfig {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
