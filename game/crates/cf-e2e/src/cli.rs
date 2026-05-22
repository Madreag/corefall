use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "cf-e2e", about = "M1.5 scripted end-to-end runner.")]
pub(crate) struct Cli {
    #[arg(long)]
    pub(crate) scenario: String,
    /// Path to a `.cfctl.json` script file (or unqualified script name from
    /// `game/scripts/cfctl/`). M1.5 baseline name.
    ///
    /// `--script <name>` so AI-trust-harness scenarios (AI-H-01..) can be
    /// invoked by the spec-canonical flag name. Both flags accept the same
    /// values; specifying both is a CLI error.
    #[arg(long, conflicts_with = "ai_harness")]
    pub(crate) script: Option<String>,
    /// Use this when invoking AI-H-NN test scenarios so the invocation reads
    /// `cargo run -p cf-e2e -- --ai-harness ai_h_01_sentry_hears_threat`
    /// per the M2 spec text.
    #[arg(long, conflicts_with = "script")]
    pub(crate) ai_harness: Option<String>,
    /// Expected post-run state in `key=value` form. May be repeated.
    #[arg(long, action = clap::ArgAction::Append)]
    pub(crate) expect: Vec<String>,
    #[arg(long)]
    pub(crate) write_run_bundle: bool,
    #[arg(long, default_value_t = 1.0)]
    pub(crate) ui_scale: f32,
    #[arg(long)]
    pub(crate) high_contrast: bool,
    #[arg(long)]
    pub(crate) verify_focus: bool,
    /// M4A: ACC-A floor — captions on/off (mirrors cf-app's `--captions on|off`).
    #[arg(long, value_enum, default_value_t = CaptionsArg::On)]
    pub(crate) captions: CaptionsArg,
    /// M4A: ACC-A floor — pass through `--reduced-motion` to the spawned cf-app.
    #[arg(long)]
    pub(crate) reduced_motion: bool,
    /// M4A: ACC-A floor — pass through `--reduced-shake` to the spawned cf-app.
    #[arg(long)]
    pub(crate) reduced_shake: bool,
    /// M4A: ACC-A floor — pass through `--reduced-flash` to the spawned cf-app.
    #[arg(long)]
    pub(crate) reduced_flash: bool,
    #[arg(long)]
    pub(crate) save_load_roundtrip: bool,
    #[arg(long)]
    pub(crate) verify_checksums: bool,
    /// Wall-clock timeout for the script runner. M0/M1/M1.5 scripts complete
    /// quickly (mission-win at low ticks); BP2 fun slices (M2.5 micro reactor
    /// defense) require the engine to run a 60s mission timer at the default
    /// 60Hz pace, so the default is now 180s. Pass a smaller value via
    /// `--timeout-seconds` for fast tests if needed.
    #[arg(long, default_value_t = 180)]
    pub(crate) timeout_seconds: u64,
    /// Control API port for the spawned cf-app. 0 chooses an ephemeral free
    /// port so concurrent sweep rows do not collide.
    #[arg(long, default_value_t = 0u16)]
    pub(crate) control_port: u16,
    /// T-CAPTURE: enable cf-capture frame readback + grid composition.
    /// When set, the spawned cf-app runs in windowed mode (NOT --headless-smoke)
    /// so the wgpu swapchain is available for screenshot readback. After the
    /// run, `game/tools/capture_grid.py <run_dir>` is invoked to compose grids.
    #[arg(long)]
    pub(crate) capture_grid: bool,
    #[arg(long, default_value_t = 10.0)]
    pub(crate) capture_frames_hz: f32,
    #[arg(long)]
    pub(crate) no_capture_events: bool,
    /// AI-Agent Self-Test Report Gate (per `.claude/skills/corefall-review/SKILL.md`):
    /// force a cf-capture event keyframe at each cfctl action's tick so the agent
    /// can write per-action visual prose without manually correlating tick → frame
    /// indices. Implies `--capture-grid`. Each `act.*` / `scenario.*` /
    /// `runbundle.*` command's `control.command_accepted` event already triggers
    /// a keyframe via cf-app's `CaptureKeyframeRequested` event hook, so this flag
    /// is a documentation + enable shortcut: it sets capture-grid AND raises
    /// capture-frames-hz to 30 Hz so even tightly-spaced commands get distinct
    /// frames.
    #[arg(long)]
    pub(crate) capture_each_action: bool,
    /// Self-Play Validation Rule "make it possible" clause: lets the harness
    /// drive the spawned cf-app at a non-default sim tick rate so the
    /// "60 Hz default + 120 Hz validation" rate-coverage requirement in the
    /// canonical roadmap can be exercised through a single cf-e2e command
    /// (instead of forcing the agent to drop down to direct cf-app
    /// invocation). 0 = use cf-app's default (60 Hz).
    #[arg(long, default_value_t = 0)]
    pub(crate) tick_rate_hz: u32,
    /// `sim.run_for_ticks` budgets resolve in a handful of Bevy frames
    /// instead of pacing 1 tick per Bevy frame (~60Hz wall-clock).
    /// Required for the m1_5min_endurance script (18000 ticks) which
    /// otherwise takes 300s of wall clock and exceeds the default 180s
    /// timeout. Determinism is preserved — the sim is deterministic
    /// per-tick, only the wall-clock pacing changes.
    #[arg(long)]
    pub(crate) unpaced: bool,
    /// Path to `python3` used to invoke the grid composer. Defaults to `python3`.
    #[arg(long, default_value = "python3")]
    pub(crate) python_bin: String,
    /// Path to `capture_grid.py`. Defaults to `<repo>/game/tools/capture_grid.py`.
    #[arg(long)]
    pub(crate) composer_script: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum CaptionsArg {
    On,
    Off,
}

impl CaptionsArg {
    pub(crate) const fn as_bool(self) -> bool {
        matches!(self, Self::On)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captions_arg_accepts_on_and_off_values() {
        let on = Cli::try_parse_from(["cf-e2e", "--scenario", "m0_blank", "--captions", "on"])
            .expect("captions on should parse");
        let off = Cli::try_parse_from(["cf-e2e", "--scenario", "m0_blank", "--captions", "off"])
            .expect("captions off should parse");

        assert!(on.captions.as_bool());
        assert!(!off.captions.as_bool());
    }

    #[test]
    fn captions_arg_defaults_to_on() {
        let cli = Cli::try_parse_from(["cf-e2e", "--scenario", "m0_blank"]).expect("default captions should parse");

        assert_eq!(cli.captions, CaptionsArg::On);
        assert!(cli.captions.as_bool());
        assert_eq!(cli.control_port, 0);
    }
}
