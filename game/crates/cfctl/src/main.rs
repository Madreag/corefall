//! `cfctl` — AI/dev control client.
//!
//! Subcommands either:
//!   - run a self-contained inline sim (`run`, `observe --once --inline`); or
//!   - connect to a running `cf-app --control-api` server over WebSocket (`scenario`, `pause`,
//!     `step`, `observe --stream`, `script run`, `act`).
//!
//! Stub/fake-success responses are forbidden. If the server is unreachable, commands
//! fail with a non-zero exit code and a structured JSON error.

use anyhow::{Context, Result};
use clap::Parser;
use serde_json::json;

use cf_replay::diagnostics;

mod cli;
mod client;
mod handlers;

use cli::{Cli, Cmd};
use handlers::{
    cmd_act, cmd_inspect, cmd_ledger_summary, cmd_observe, cmd_replay, cmd_run, cmd_save, cmd_scenario, cmd_script,
    cmd_simple, cmd_version,
};

fn main() -> Result<()> {
    diagnostics::init("cf::ctl");
    let cli = Cli::parse();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime for cfctl")?;
    runtime.block_on(async move { dispatch(cli).await })
}

async fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Cmd::Observe {
            once,
            stream,
            hz,
            scenario,
            seed,
            tick_rate_hz,
            settings,
            format,
            inline,
        } => {
            cmd_observe(
                &cli.connect,
                cli.auto_launch_port,
                cli.no_auto_launch,
                once,
                stream,
                hz,
                scenario,
                seed,
                tick_rate_hz,
                settings,
                format,
                inline,
            )
            .await
        }
        Cmd::Run {
            scenario,
            ticks,
            seed,
            tick_rate_hz,
            write_run_bundle,
            run_bundle_dir,
            ui_scale,
            high_contrast,
            paced,
            ledger_chain,
            delta_baseline_cadence_ticks,
        } => cmd_run(
            scenario,
            ticks,
            seed,
            tick_rate_hz,
            write_run_bundle,
            run_bundle_dir,
            ui_scale,
            high_contrast,
            paced,
            ledger_chain,
            delta_baseline_cadence_ticks,
        ),
        Cmd::Scenario { action } => cmd_scenario(&cli.connect, cli.auto_launch_port, cli.no_auto_launch, action).await,
        Cmd::Pause => {
            cmd_simple(
                &cli.connect,
                cli.auto_launch_port,
                cli.no_auto_launch,
                "sim.pause",
                json!({}),
            )
            .await
        }
        Cmd::Step { ticks } => {
            cmd_simple(
                &cli.connect,
                cli.auto_launch_port,
                cli.no_auto_launch,
                "sim.step",
                json!({"ticks": ticks}),
            )
            .await
        }
        Cmd::Act { action } => cmd_act(&cli.connect, cli.auto_launch_port, cli.no_auto_launch, action).await,
        Cmd::Script { action } => cmd_script(&cli.connect, cli.auto_launch_port, cli.no_auto_launch, action).await,
        Cmd::Inspect { action } => cmd_inspect(&cli.connect, cli.auto_launch_port, cli.no_auto_launch, action).await,
        Cmd::Replay { action } => cmd_replay(action),
        Cmd::LedgerSummary { format, inline } => {
            cmd_ledger_summary(&cli.connect, cli.auto_launch_port, cli.no_auto_launch, format, inline).await
        }
        Cmd::Save { action } => cmd_save(&cli.connect, cli.auto_launch_port, cli.no_auto_launch, action).await,
        Cmd::Version => cmd_version(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::want_inline_default;
    use clap::Parser;

    #[test]
    fn version_command_prints_schema_version() {
        cmd_version().unwrap();
    }

    #[test]
    fn want_inline_logic() {
        assert!(want_inline_default(false, true, false, false));
        assert!(want_inline_default(false, false, false, true));
        assert!(!want_inline_default(false, false, true, false));
        assert!(want_inline_default(true, false, true, false));
    }

    #[test]
    fn run_seed_defaults_to_none_so_manifest_seed_wins() {
        let cli = Cli::try_parse_from(["cfctl", "run", "--scenario", "m0_blank"]).unwrap();
        match cli.command {
            Cmd::Run { seed, .. } => assert_eq!(
                seed, None,
                "cfctl run --seed must default to None so the scenario manifest's seed flows \
                 unchanged into build_engine_config (shared determinism contract with cf-app). \
                 Any default value here force-passes Some(default) and rejects scenarios whose \
                 manifest seed differs (M0.2-F3 reject path)."
            ),
            other => panic!("expected Cmd::Run, got {other:?}"),
        }
    }

    #[test]
    fn run_seed_explicit_value_passes_through() {
        let cli = Cli::try_parse_from(["cfctl", "run", "--scenario", "m0_blank", "--seed", "7"]).unwrap();
        match cli.command {
            Cmd::Run { seed, .. } => assert_eq!(seed, Some(7)),
            other => panic!("expected Cmd::Run, got {other:?}"),
        }
    }

    #[test]
    fn observe_seed_defaults_to_none_so_manifest_seed_wins() {
        let cli = Cli::try_parse_from(["cfctl", "observe", "--once", "--scenario", "m0_blank"]).unwrap();
        match cli.command {
            Cmd::Observe { seed, .. } => assert_eq!(
                seed, None,
                "cfctl observe --seed must default to None so the scenario manifest's seed flows \
                 unchanged into build_engine_config (shared determinism contract with cf-app). \
                 Any default value here force-passes Some(default) and rejects scenarios whose \
                 manifest seed differs (M0.2-F3 reject path)."
            ),
            other => panic!("expected Cmd::Observe, got {other:?}"),
        }
    }

    #[test]
    fn observe_seed_explicit_value_passes_through() {
        let cli = Cli::try_parse_from(["cfctl", "observe", "--once", "--scenario", "m0_blank", "--seed", "7"]).unwrap();
        match cli.command {
            Cmd::Observe { seed, .. } => assert_eq!(seed, Some(7)),
            other => panic!("expected Cmd::Observe, got {other:?}"),
        }
    }
}
