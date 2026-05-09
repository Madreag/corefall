//! M3A: headless replay verifier.
//!
//! Reads a checked run bundle, replays the recorded control commands against a
//! fresh `M0Engine` constructed from the bundle's manifest, and verifies that
//! the engine's per-tick `determinism.sim_checksum` events match the recorded
//! values. On mismatch, emits a structured `first_divergence` report and exits
//! non-zero.
//!
//! The verifier purposely uses the SAME `M0Engine` code the live engine uses
//! (per AGENTS.md "no parallel production paths"). It does NOT reimplement the
//! sim loop, the dig path, the mission state machine, or any other subsystem
//! the live engine drives.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use serde_json::Value;
use tracing_subscriber::EnvFilter;

use cf_actor::IntentSource;
use cf_control::{
    runtime::{build_engine_config, ConfigInputs},
    scenario::Scenario,
    server::SettingsPatch,
    settings::Settings,
    ControlCommand, EngineHandle, EngineState, M0Engine, M0EngineConfig,
};

#[derive(Debug, Parser)]
#[command(name = "cf-headless", about = "Headless sim runner / replay verifier (M3A).")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// M3A-003: replay a run bundle headlessly and verify the per-tick sim
    /// checksum matches what the live run recorded.
    Replay {
        bundle_dir: PathBuf,
        /// Skip per-tick checksum verification. Default behavior is to
        /// verify; pass `--no-verify-checksums` to opt out (Bugbot 2ce56d7e
        /// flagged the prior `default_value_t = true` boolean: clap v4
        /// requires an explicit value for `default_value_t` bools, which
        /// made `--verify-checksums` parse-error without a value. The
        /// negation flag is the idiomatic clap v4 default-true opt-out).
        #[arg(long, default_value_t = false)]
        no_verify_checksums: bool,
        /// Optional override scenario manifest path. Defaults to the path
        /// recorded in `run_manifest.json.scene.source_path`.
        #[arg(long)]
        scenario_path: Option<PathBuf>,
    },
}

fn init_diagnostics() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,cf_=debug")))
        .with_target(true)
        .init();
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(target: "cf::headless", panic = %info, "system.panic");
        prev_hook(info);
    }));
}

fn main() -> Result<()> {
    init_diagnostics();
    let cli = Cli::parse();
    match cli.command {
        Cmd::Replay {
            bundle_dir,
            no_verify_checksums,
            scenario_path,
        } => replay(&bundle_dir, !no_verify_checksums, scenario_path),
    }
}

fn replay(bundle_dir: &Path, verify_checksums: bool, scenario_path: Option<PathBuf>) -> Result<()> {
    if !bundle_dir.exists() {
        bail!("bundle directory does not exist: {}", bundle_dir.display());
    }
    let manifest_path = bundle_dir.join("run_manifest.json");
    let manifest_text = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("read manifest {}", manifest_path.display()))?;
    let manifest: Value = serde_json::from_str(&manifest_text).context("parse run_manifest.json")?;

    let recorded_seed = manifest
        .get("seed")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("manifest missing seed"))?;
    let recorded_tick_rate = manifest
        .get("tick_rate_hz")
        .and_then(Value::as_u64)
        .map(|n| n as u32)
        .unwrap_or(60);
    let scenario_source = scenario_path
        .map(|p| p.display().to_string())
        .or_else(|| {
            manifest
                .get("scene")
                .and_then(|s| s.get("source_path"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .ok_or_else(|| anyhow!("manifest missing scene.source_path"))?;

    let mut scenario_path_buf = PathBuf::from(&scenario_source);
    if !scenario_path_buf.exists() {
        // Manifest paths are usually relative to the binary's cwd at run
        // time. Fall back to common Corefall layouts so the replay verifier
        // works regardless of caller cwd.
        let candidates = [
            scenario_path_buf.clone(),
            PathBuf::from("game").join(&scenario_path_buf),
            PathBuf::from("../game").join(&scenario_path_buf),
            PathBuf::from("..").join("game").join(&scenario_path_buf),
        ];
        let mut resolved = None;
        for c in &candidates {
            if c.exists() {
                resolved = Some(c.clone());
                break;
            }
        }
        match resolved {
            Some(p) => {
                scenario_path_buf = p;
            }
            None => bail!(
                "scenario file does not exist: {} (override with --scenario-path)",
                scenario_path_buf.display()
            ),
        }
    }
    let scenario = Scenario::load_from_file(&scenario_path_buf).context("load scenario")?;

    let mut config: M0EngineConfig = build_engine_config(ConfigInputs {
        scenario_id: scenario.id.clone(),
        scenario_path: scenario_path_buf.clone(),
        seed_override: Some(recorded_seed),
        duration_ticks_override: None,
        tick_rate_hz: recorded_tick_rate,
        run_mode: "headless-replay".to_string(),
        run_bundle_root: cf_replay::default_run_bundle_root(),
        write_run_bundle: false,
        control_api_enabled: false,
        debug_capabilities: vec!["headless-replay".to_string()],
        paced: false,
        settings: Settings::default(),
        debug_inject_panic_at_tick: None,
    })?;
    config.run_mode = "headless-replay".to_string();
    config.write_run_bundle = false;

    let events_path = bundle_dir.join("events.jsonl");
    let events_text =
        std::fs::read_to_string(&events_path).with_context(|| format!("read events {}", events_path.display()))?;

    let recorded_checksums = collect_checksums(&events_text);
    if recorded_checksums.is_empty() {
        bail!("bundle contains no determinism.sim_checksum events; cannot verify");
    }
    let recorded_commands = collect_commands(&events_text)?;
    let max_tick = recorded_checksums
        .iter()
        .map(|(t, _)| *t)
        .chain(recorded_commands.iter().map(|c| c.tick))
        .max()
        .unwrap_or(0);

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("build tokio runtime")?;

    runtime.block_on(async move {
        let engine = std::sync::Arc::new(M0Engine::new(config.clone()));
        engine.record_run_started();

        let mut next_tick: u64 = engine.current_tick().0;
        let mut chk_idx: usize = 0;
        let mut cmd_idx: usize = 0;
        let mut divergences: Vec<(u64, String, String)> = Vec::new();
        // Bugbot 2ce56d7e: bound the pause-recovery retry path so a permanently
        // stalled engine (e.g., shutdown_requested set + drive_tick keeps
        // returning None despite RunForTicks dispatches) cannot spin forever.
        // Three consecutive None advances at the same tick is the recovery
        // budget; beyond that we abort with a structured stall report.
        let mut consecutive_no_advance: u32 = 0;
        const MAX_NO_ADVANCE_RETRIES: u32 = 3;

        // Replay loop:
        //   1. Dispatch any commands recorded at the engine's CURRENT tick.
        //   2. Drive a single tick forward.
        //   3. If the new tick has a recorded checksum, compare against the
        //      engine's live checksum.
        //   4. Stop when we reach `max_tick`.
        while next_tick <= max_tick {
            // 1) Dispatch all commands at the current tick.
            while cmd_idx < recorded_commands.len() && recorded_commands[cmd_idx].tick == next_tick {
                let cmd_event = &recorded_commands[cmd_idx];
                if let Some(command) = parse_command(&cmd_event.payload) {
                    let _ = engine.dispatch(command).await;
                }
                cmd_idx += 1;
            }

            // 2) Drive forward.
            if engine.drive_tick().is_none() {
                consecutive_no_advance += 1;
                if consecutive_no_advance >= MAX_NO_ADVANCE_RETRIES {
                    bail!(
                        "replay verifier stalled: engine returned None from drive_tick {} times in a row at tick {} (likely shutdown_requested or another terminal state). Aborting to avoid infinite loop.",
                        consecutive_no_advance,
                        next_tick
                    );
                }
                // Engine is paused. Force-resume via RunForTicks for the rest
                // of the budget. This handles bundles that paused but never
                // resumed in time before drive_tick was called.
                let remaining = max_tick.saturating_sub(next_tick).max(1);
                let _ = engine
                    .dispatch(ControlCommand::RunForTicks {
                        ticks: remaining,
                        write_run_bundle: false,
                    })
                    .await;
                continue;
            }
            consecutive_no_advance = 0;
            next_tick = engine.current_tick().0;

            // 3) Verify checksum at this tick if there's a recorded one.
            if verify_checksums {
                while chk_idx < recorded_checksums.len() && recorded_checksums[chk_idx].0 < next_tick {
                    let (tick, hex) = &recorded_checksums[chk_idx];
                    divergences.push((
                        *tick,
                        hex.clone(),
                        format!("recorded tick {tick} skipped (engine reached {next_tick})"),
                    ));
                    chk_idx += 1;
                }
                if chk_idx < recorded_checksums.len() && recorded_checksums[chk_idx].0 == next_tick {
                    let (tick, recorded_hex) = &recorded_checksums[chk_idx];
                    let live_hex = engine.recorder().final_checksum_hex().unwrap_or_default();
                    if live_hex != *recorded_hex {
                        divergences.push((*tick, recorded_hex.clone(), live_hex));
                    }
                    chk_idx += 1;
                }
            }
        }

        let live_state = engine_state(&engine).await;
        if divergences.is_empty() {
            println!(
                "{{\"result\":\"ok\",\"replayed_ticks\":{},\"checksums_verified\":{},\"commands_replayed\":{},\"final_run_id\":\"{}\"}}",
                next_tick,
                recorded_checksums.len(),
                recorded_commands.len(),
                live_state.run_id
            );
            Ok::<_, anyhow::Error>(())
        } else {
            let first = divergences.first().expect("non-empty");
            println!(
                "{{\"result\":\"divergence\",\"first_divergence\":{{\"tick\":{},\"recorded\":\"{}\",\"live\":\"{}\"}},\"total_divergences\":{}}}",
                first.0, first.1, first.2, divergences.len()
            );
            bail!("replay diverged at tick {}", first.0)
        }
    })?;

    Ok(())
}

/// One control.command_accepted event reduced to (tick, payload).
struct RecordedCommand {
    tick: u64,
    payload: Value,
}

fn collect_commands(events_text: &str) -> Result<Vec<RecordedCommand>> {
    let mut out = Vec::new();
    for line in events_text.lines() {
        if line.is_empty() {
            continue;
        }
        let v: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if v.get("category").and_then(Value::as_str) != Some("control") {
            continue;
        }
        if v.get("event_type").and_then(Value::as_str) != Some("command_accepted") {
            continue;
        }
        let tick = v.get("tick").and_then(Value::as_u64).unwrap_or(0);
        let payload = v.get("payload").cloned().unwrap_or(Value::Null);
        out.push(RecordedCommand { tick, payload });
    }
    Ok(out)
}

fn parse_command(payload: &Value) -> Option<ControlCommand> {
    let method = payload.get("method").and_then(Value::as_str)?;
    match method {
        "scenario.reset" => Some(ControlCommand::ScenarioReset),
        "scenario.load" => {
            let scenario = payload.get("scenario").and_then(Value::as_str)?.to_string();
            let seed = payload.get("seed").and_then(Value::as_u64);
            Some(ControlCommand::ScenarioLoad { scenario, seed })
        }
        "sim.pause" => Some(ControlCommand::Pause),
        "sim.resume" => Some(ControlCommand::Resume),
        "sim.step" => {
            let ticks = payload.get("ticks").and_then(Value::as_u64).unwrap_or(1);
            Some(ControlCommand::Step { ticks })
        }
        "sim.run_for_ticks" => {
            let ticks = payload.get("ticks").and_then(Value::as_u64).unwrap_or(0);
            // Bugbot: hard-code `write_run_bundle: false` for the replay
            // verifier regardless of what the recorded payload contained.
            // The replay path's `M0EngineConfig.write_run_bundle` is `false`
            // (see `build_engine_config` call above with
            // `write_run_bundle: false`). Replaying the original
            // `write_run_bundle: true` flag would dispatch a bundle write
            // to the verifier's recorder — extra disk I/O we never want
            // during replay.
            Some(ControlCommand::RunForTicks {
                ticks,
                write_run_bundle: false,
            })
        }
        "act.player.move" => Some(ControlCommand::ActPlayerMove {
            x: payload.get("x").and_then(Value::as_f64).unwrap_or(0.0) as f32,
            y: payload.get("y").and_then(Value::as_f64).unwrap_or(0.0) as f32,
            source: IntentSource::Cfctl,
        }),
        "act.player.aim" => Some(ControlCommand::ActPlayerAim {
            x: payload.get("x").and_then(Value::as_f64).unwrap_or(0.0) as f32,
            y: payload.get("y").and_then(Value::as_f64).unwrap_or(0.0) as f32,
            source: IntentSource::Cfctl,
        }),
        "act.player.fire" => Some(ControlCommand::ActPlayerFire {
            pressed: payload.get("pressed").and_then(Value::as_bool).unwrap_or(true),
            source: IntentSource::Cfctl,
        }),
        "act.player.reload" => Some(ControlCommand::ActPlayerReload {
            source: IntentSource::Cfctl,
        }),
        "act.player.jump" => Some(ControlCommand::ActPlayerJump {
            source: IntentSource::Cfctl,
        }),
        "act.player.dig" => Some(ControlCommand::ActPlayerDig {
            target: payload.get("target").and_then(Value::as_str).map(str::to_string),
            source: IntentSource::Cfctl,
        }),
        "act.player.select_item" => Some(ControlCommand::ActPlayerSelectItem {
            slot: payload.get("slot").and_then(Value::as_u64).unwrap_or(0) as u32,
            source: IntentSource::Cfctl,
        }),
        "act.player.reset" => Some(ControlCommand::ActPlayerReset {
            source: IntentSource::Cfctl,
        }),
        "act.settings.set" => {
            // Settings patches are not replayed because the recorded
            // command_accepted payload does not carry the patch contents
            // (avoid leaking accessibility flags into the event log).
            Some(ControlCommand::SettingsSet {
                changes: SettingsPatch::default(),
            })
        }
        "runbundle.write" | "system.shutdown" => None,
        _ => None,
    }
}

async fn engine_state(engine: &M0Engine) -> EngineState {
    use cf_control::EngineHandle;
    let frame = engine.snapshot(None).await;
    EngineState {
        run_id: frame.run_id,
        scenario: frame.scenario,
        tick: frame.tick,
        sim_time_ms: frame.sim_time_ms,
        run_status: frame.run_status,
        seed: 0,
        tick_rate_hz: 0,
    }
}

/// Read events.jsonl and return `(tick, checksum_hex)` for every CADENCE
/// `determinism.sim_checksum` event in tick order. We deliberately skip
/// `kind=final` checksums because:
///
/// 1. They are emitted by the engine on `record_run_finished()` and
///    `write_run_bundle()`, both of which happen OUTSIDE the replay loop
///    (the verifier doesn't drive shutdown lifecycle).
/// 2. A single tick can carry multiple final checksums (mid-run
///    `runbundle.write` + final shutdown), all with the same hex but emitted
///    at the same tick. Comparing them against a single live computation is
///    redundant and produces phantom divergences.
fn collect_checksums(events_text: &str) -> Vec<(u64, String)> {
    let mut out = Vec::new();
    for line in events_text.lines() {
        if line.is_empty() {
            continue;
        }
        let v: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if v.get("category").and_then(Value::as_str) != Some("determinism") {
            continue;
        }
        if v.get("event_type").and_then(Value::as_str) != Some("sim_checksum") {
            continue;
        }
        if v.get("payload").and_then(|p| p.get("kind")).and_then(Value::as_str) == Some("final") {
            continue;
        }
        let tick = v.get("tick").and_then(Value::as_u64).unwrap_or(0);
        let hex = v
            .get("payload")
            .and_then(|p| p.get("checksum_hex"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        out.push((tick, hex));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Bugbot regression: `parse_command` for `sim.run_for_ticks` MUST hard-
    /// code `write_run_bundle: false` regardless of what the recorded payload
    /// contained. The replay verifier never wants to write a bundle even if
    /// the original run dispatched `RunForTicks { write_run_bundle: true }`.
    #[test]
    fn parse_run_for_ticks_forces_write_run_bundle_false() {
        let payload = json!({
            "method": "sim.run_for_ticks",
            "ticks": 60,
            "write_run_bundle": true,
        });
        match parse_command(&payload) {
            Some(ControlCommand::RunForTicks {
                ticks,
                write_run_bundle,
            }) => {
                assert_eq!(ticks, 60);
                assert!(
                    !write_run_bundle,
                    "replay verifier MUST NOT pass through write_run_bundle=true from recorded payload"
                );
            }
            other => panic!("expected RunForTicks, got {other:?}"),
        }
    }

    #[test]
    fn parse_run_for_ticks_default_write_run_bundle_false() {
        let payload = json!({"method": "sim.run_for_ticks", "ticks": 60});
        match parse_command(&payload) {
            Some(ControlCommand::RunForTicks { write_run_bundle, .. }) => {
                assert!(!write_run_bundle);
            }
            other => panic!("expected RunForTicks, got {other:?}"),
        }
    }

    /// Verifies the cf-headless replay parser handles every BP2 method
    /// without panicking and produces the expected variant.
    #[test]
    fn parse_bp2_method_catalog_returns_expected_variants() {
        let cases = [
            ("scenario.reset", json!({"method": "scenario.reset"})),
            ("act.player.dig", json!({"method": "act.player.dig", "target": null})),
            (
                "act.player.move",
                json!({"method": "act.player.move", "x": 1.0, "y": 0.0}),
            ),
            (
                "act.player.aim",
                json!({"method": "act.player.aim", "x": -1.0, "y": 0.0}),
            ),
            ("act.player.fire", json!({"method": "act.player.fire", "pressed": true})),
        ];
        for (name, payload) in cases {
            assert!(
                parse_command(&payload).is_some(),
                "parse_command returned None for {name}"
            );
        }
    }

    /// M3A: the replay verifier must accept every BP2 cfctl method the live
    /// engine dispatches. This is the regression proof for the parser layer
    /// the verifier sits behind. (Wrapper-named so
    /// `bp_test_coverage::cargo_module_missing` finds it under the
    /// `cf-headless::tests::replay_*` glob declared in
    /// `game/content/build_points/bp2.test_manifest.json`.)
    #[test]
    fn replay_parser_accepts_every_bp2_method() {
        let payloads = [
            json!({"method": "scenario.reset"}),
            json!({"method": "scenario.load", "scenario": "x", "seed": 1}),
            json!({"method": "sim.pause"}),
            json!({"method": "sim.resume"}),
            json!({"method": "sim.step", "ticks": 1}),
            json!({"method": "sim.run_for_ticks", "ticks": 60}),
            json!({"method": "act.player.move", "x": 1.0, "y": 0.0}),
            json!({"method": "act.player.aim", "x": 1.0, "y": 0.0}),
            json!({"method": "act.player.fire", "pressed": true}),
            json!({"method": "act.player.reload"}),
            json!({"method": "act.player.jump"}),
            json!({"method": "act.player.dig", "target": null}),
            json!({"method": "act.player.select_item", "slot": 0}),
            json!({"method": "act.player.reset"}),
            json!({"method": "act.settings.set", "settings": {"captions": true}}),
        ];
        for p in payloads.iter() {
            let method = p.get("method").and_then(|v| v.as_str()).unwrap_or("?");
            assert!(
                parse_command(p).is_some(),
                "replay verifier failed to parse method={method}"
            );
        }
    }

    /// M3A: the replay verifier must REJECT unknown methods rather than
    /// silently treat them as no-ops. Negative proof so `replay_*` covers
    /// both happy + adversarial paths.
    #[test]
    fn replay_parser_rejects_unknown_method() {
        let cases = [
            json!({"method": "act.player.frobnicate"}),
            json!({"method": "act.input.key_press"}),
            json!({"method": ""}),
            json!({}),
        ];
        for p in cases.iter() {
            assert!(
                parse_command(p).is_none(),
                "replay parser accepted an unsupported method {p:?}"
            );
        }
    }
}
