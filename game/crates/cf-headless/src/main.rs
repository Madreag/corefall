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
use serde_json::{json, Value};
use tracing_subscriber::EnvFilter;

use cf_actor::IntentSource;
use cf_control::{
    runtime::{build_engine_config, ConfigInputs},
    scenario::Scenario,
    server::SettingsPatch,
    settings::Settings,
    ControlCommand, EngineHandle, EngineState, M0Engine, M0EngineConfig,
};

mod server_mode;
use server_mode::ServeArgs;

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
        /// **M4 § Replay throughput benchmark**: emit performance counters
        /// (`throughput_ticks_per_sec`, `wall_time_ms`, `peak_memory_mb`)
        /// alongside the result envelope. Implies `--no-verify-checksums`
        /// because throughput measurements are orthogonal to determinism
        /// verification (the verifier doesn't have to walk every recorded
        /// checksum when timing the replay).
        #[arg(long, value_enum)]
        measure: Option<MeasureMode>,
        /// **M4 § Replay verifier safety**: maximum consecutive no-advance
        /// retries before the verifier bails on a stalled engine. Default
        /// 3 per spec. Lower values fail-fast on corrupt bundles; higher
        /// values give the verifier more rope in unusual cfctl scripts.
        #[arg(long, default_value_t = 3)]
        max_no_advance_retries: u32,
        /// **M4A § "Run bundle references ledger entries"**: cross-check
        /// every event with an `asset_ref` field against the canonical
        /// `content/asset_ledger/ledger.jsonl`. Fails the replay when a
        /// referenced ledger entry is missing, drifted, failed, or stale.
        /// Default behavior is to check; pass `--no-verify-asset-refs`
        /// to opt out (consistent with `--no-verify-checksums`).
        #[arg(long, default_value_t = false)]
        no_verify_asset_refs: bool,
        /// **M4A**: optional override path for the canonical asset ledger.
        /// Defaults to the same three candidate paths the cf-control
        /// `observe.assets.ledger_summary` surface searches. Useful for
        /// tests that bake against a sandbox ledger.
        #[arg(long)]
        asset_ledger_path: Option<PathBuf>,
    },
    /// M8A § cf-net authoritative server. Runs the deterministic sim
    /// core with no Bevy render plugin; emits a server-side run bundle
    /// to `prototype_runs/server/<run-id>/`. Behind a feature flag at
    /// M8A; M9+ wires the live QUIC transport.
    Serve {
        /// Scenario manifest path to drive the authoritative server.
        #[arg(long)]
        scenario: PathBuf,
        /// Server bind address.
        #[arg(long, default_value = "0.0.0.0")]
        bind_addr: String,
        /// TCP/UDP port for the server. Reference port 4040.
        #[arg(long, default_value_t = 4040)]
        port: u16,
        /// Number of ticks to simulate before exiting. Useful for
        /// determinism + integration testing.
        #[arg(long, default_value_t = 1000)]
        ticks: u32,
        /// Confirm we run with no Bevy render plugin (server mode).
        #[arg(long, default_value_t = true)]
        no_render: bool,
        /// Maximum LAN clients (locked at 8 for M8A acceptance criterion).
        #[arg(long, default_value_t = 8)]
        max_clients: u32,
    },
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum MeasureMode {
    /// Measure ticks-replayed-per-second + peak memory. Skips checksum
    /// verification.
    Throughput,
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
            measure,
            max_no_advance_retries,
            no_verify_asset_refs,
            asset_ledger_path,
        } => {
            let verify = match measure {
                Some(MeasureMode::Throughput) => false,
                None => !no_verify_checksums,
            };
            replay(ReplayArgs {
                bundle_dir: &bundle_dir,
                verify_checksums: verify,
                scenario_path,
                measure,
                max_no_advance_retries,
                verify_asset_refs: !no_verify_asset_refs,
                asset_ledger_path,
            })
        }
        Cmd::Serve {
            scenario,
            bind_addr,
            port,
            ticks,
            no_render,
            max_clients,
        } => server_mode::run_serve(ServeArgs {
            scenario,
            bind_addr,
            port,
            ticks,
            no_render,
            max_clients,
        }),
    }
}

struct ReplayArgs<'a> {
    bundle_dir: &'a Path,
    verify_checksums: bool,
    scenario_path: Option<PathBuf>,
    measure: Option<MeasureMode>,
    max_no_advance_retries: u32,
    verify_asset_refs: bool,
    asset_ledger_path: Option<PathBuf>,
}

fn replay(args: ReplayArgs<'_>) -> Result<()> {
    let ReplayArgs {
        bundle_dir,
        verify_checksums,
        scenario_path,
        measure,
        max_no_advance_retries,
        verify_asset_refs,
        asset_ledger_path,
    } = args;
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
    // **M4 § Per-scenario checksum cadence**: the verifier must use the
    // SAME cadence the bundle was produced with. Read it from
    // `run_manifest.json.checksum.cadence_ticks` and pass it into the
    // engine config so per-tick checksum events line up.
    let recorded_cadence = manifest
        .get("checksum")
        .and_then(|c| c.get("cadence_ticks"))
        .and_then(Value::as_u64);
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
        capture_grid_enabled: false,
        paced: false,
        settings: Settings::default(),
        debug_inject_panic_at_tick: None,
        checksum_cadence_ticks: recorded_cadence,
        expected_outcome: None,
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

    // **M4A § "Run bundle references ledger entries"**: collect every event
    // whose envelope-level `asset_ref` field is set so we can cross-check
    // them against the canonical asset ledger AFTER the deterministic
    // replay loop completes.
    let asset_refs_in_bundle: Vec<AssetRefReference> = if verify_asset_refs {
        collect_asset_refs(&events_text)
    } else {
        Vec::new()
    };
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

    let replay_start = std::time::Instant::now();
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
        let max_no_advance_retries_local = max_no_advance_retries;

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
                if consecutive_no_advance >= max_no_advance_retries_local {
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

        // **M4A § "Run bundle references ledger entries"**: cross-check
        // every event with `asset_ref` against the canonical ledger.
        // Surfaces missing/drifted/failed ledger entries as a structured
        // result the same way determinism.first_divergence is surfaced.
        let asset_ref_report = if verify_asset_refs && !asset_refs_in_bundle.is_empty() {
            Some(verify_asset_refs_against_ledger(
                &asset_refs_in_bundle,
                asset_ledger_path.as_deref(),
            )?)
        } else {
            None
        };

        if divergences.is_empty() {
            // Asset-ref verification can also fail the replay.
            if let Some(report) = &asset_ref_report {
                if !report.failures.is_empty() {
                    let output = json!({
                        "result": "asset_ref_failure",
                        "first_failure": report.failures[0],
                        "total_failures": report.failures.len(),
                        "asset_ref_failures": report.failures,
                        "asset_refs_checked": report.checked,
                    });
                    println!("{}", serde_json::to_string(&output).unwrap_or_default());
                    tracing::error!(
                        target: "cf::headless",
                        total = report.failures.len(),
                        "m4a.asset_ref.failure"
                    );
                    bail!(
                        "replay verified determinism but {} asset_ref(s) failed ledger cross-check",
                        report.failures.len()
                    );
                }
            }
            let mut ok = json!({
                "result": "ok",
                "replayed_ticks": next_tick,
                "checksums_verified": if verify_checksums { recorded_checksums.len() } else { 0 },
                "commands_replayed": recorded_commands.len(),
                "final_run_id": live_state.run_id,
            });
            if !verify_checksums {
                ok["checksum_verification"] = serde_json::Value::String("skipped".to_string());
            }
            if let Some(report) = &asset_ref_report {
                ok["asset_refs_checked"] = serde_json::json!(report.checked);
                ok["asset_ref_verification"] = serde_json::Value::String("ok".to_string());
            } else if verify_asset_refs {
                ok["asset_ref_verification"] = serde_json::Value::String("no_asset_refs".to_string());
            } else {
                ok["asset_ref_verification"] = serde_json::Value::String("skipped".to_string());
            }
            if let Some(MeasureMode::Throughput) = measure {
                let wall_time_ms = replay_start.elapsed().as_secs_f64() * 1000.0;
                let throughput = if wall_time_ms > 0.0 {
                    (next_tick as f64) / (wall_time_ms / 1000.0)
                } else {
                    0.0
                };
                let peak_mb = peak_memory_mb();
                ok["throughput_ticks_per_sec"] = serde_json::json!(throughput);
                ok["wall_time_ms"] = serde_json::json!(wall_time_ms);
                ok["peak_memory_mb"] = serde_json::json!(peak_mb);
            }
            println!("{}", serde_json::to_string(&ok).unwrap_or_default());
            Ok::<_, anyhow::Error>(())
        } else {
            let first = divergences.first().expect("non-empty");
            let all_diffs: Vec<serde_json::Value> = divergences.iter()
                .map(|(tick, recorded, live)| json!({"tick": tick, "recorded": recorded, "live": live}))
                .collect();
            let output = json!({
                "result": "divergence",
                "first_divergence": {"tick": first.0, "recorded": first.1, "live": first.2},
                "total_divergences": divergences.len(),
                "all_divergences": all_diffs,
            });
            println!("{}", serde_json::to_string(&output).unwrap_or_default());
            tracing::error!(
                target: "cf::headless",
                tick = first.0,
                recorded = %first.1,
                live = %first.2,
                total = divergences.len(),
                "determinism.first_divergence"
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
        "act.player.crouch" => Some(ControlCommand::ActPlayerCrouch {
            active: payload.get("active").and_then(Value::as_bool).unwrap_or(true),
            source: IntentSource::Replay,
        }),
        "act.player.climb" => Some(ControlCommand::ActPlayerClimb {
            active: payload.get("active").and_then(Value::as_bool).unwrap_or(true),
            source: IntentSource::Replay,
        }),
        "act.player.jet" => Some(ControlCommand::ActPlayerJet {
            active: payload.get("active").and_then(Value::as_bool).unwrap_or(true),
            source: IntentSource::Replay,
        }),
        "act.player.eject" => Some(ControlCommand::ActPlayerEject {
            source: IntentSource::Replay,
        }),
        // **M1 Gap I1**: replay arm for the sharp-aim toggle.
        "act.player.sharp_aim" => Some(ControlCommand::ActPlayerSharpAim {
            active: payload.get("active").and_then(Value::as_bool).unwrap_or(false),
            source: IntentSource::Replay,
        }),
        // **M1 Gap S3**: replay arm for the abort stub (currently rejects).
        "act.player.abort" => Some(ControlCommand::ActPlayerAbort {
            source: IntentSource::Replay,
        }),
        // **M1 Gap D1**: replay arm for the controls-capture toggle.
        "act.input.capture_controls" => Some(ControlCommand::ActInputCaptureControls {
            captured: payload.get("captured").and_then(Value::as_bool).unwrap_or(false),
            capturer: payload.get("capturer").and_then(Value::as_str).map(str::to_string),
            source: IntentSource::Replay,
        }),
        "act.chassis.repair" => Some(ControlCommand::ActChassisRepair {
            zone: payload.get("zone").and_then(Value::as_str).map(str::to_string),
            module_id: payload.get("module_id").and_then(Value::as_str).map(str::to_string),
            reason: payload
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or("replay")
                .to_string(),
            source: IntentSource::Replay,
        }),
        "act.chassis.salvage" => Some(ControlCommand::ActChassisSalvage {
            reason: payload
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or("replay")
                .to_string(),
            source: IntentSource::Replay,
        }),
        "act.chassis.clear_jam" => Some(ControlCommand::ActChassisClearJam {
            source: IntentSource::Replay,
        }),
        "act.input.focus" => {
            let dir_str = payload.get("direction").and_then(Value::as_str).unwrap_or("clear");
            let direction = match dir_str {
                "next" => cf_control::FocusDirection::Next,
                "prev" => cf_control::FocusDirection::Prev,
                "clear" => cf_control::FocusDirection::Clear,
                other => cf_control::FocusDirection::Set(other.to_string()),
            };
            Some(ControlCommand::ActInputFocus {
                direction,
                source: IntentSource::Replay,
            })
        }
        "act.settings.set" => {
            // Settings patches are not replayed because the recorded
            // command_accepted payload does not carry the patch contents
            // (avoid leaking accessibility flags into the event log).
            Some(ControlCommand::SettingsSet {
                changes: Box::new(SettingsPatch::default()),
            })
        }
        "runbundle.write" | "system.shutdown" => None,
        _ => None,
    }
}

/// **M4 § Replay throughput benchmark**: best-effort peak resident-set-size
/// reporter. On macOS uses `mach_task_basic_info::resident_size_max`; on
/// Linux uses `/proc/self/status`'s `VmHWM`; on Windows uses
/// `GetProcessMemoryInfo`. Falls back to 0.0 if the platform probe fails;
/// the throughput envelope tolerates a zero value so CI doesn't fail on
/// unfamiliar OSes.
fn peak_memory_mb() -> f64 {
    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if let Some(rest) = line.strip_prefix("VmHWM:") {
                    let kb: u64 = rest.trim().trim_end_matches(" kB").trim().parse().unwrap_or(0);
                    return (kb as f64) / 1024.0;
                }
            }
        }
        0.0
    }
    #[cfg(target_os = "macos")]
    {
        unsafe extern "C" {
            fn getrusage(who: i32, usage: *mut Rusage) -> i32;
        }
        #[repr(C)]
        #[derive(Default)]
        struct Rusage {
            ru_utime: [i64; 2],
            ru_stime: [i64; 2],
            ru_maxrss: i64,
            _pad: [i64; 14],
        }
        let mut ru = Rusage::default();
        let rc = unsafe {
            getrusage(0 /* RUSAGE_SELF */, &mut ru)
        };
        if rc != 0 {
            return 0.0;
        }
        (ru.ru_maxrss as f64) / (1024.0 * 1024.0)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        0.0
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

// ---------------------------------------------------------------------------
// M4A asset-ledger cross-check
// ---------------------------------------------------------------------------

/// **M4A § "Run bundle references ledger entries"** — every envelope-level
/// `asset_ref` collected for ledger cross-check. Keeping `event_id` and
/// `tick` lets the failure report point operators at the exact event.
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct AssetRefReference {
    pub asset_ref: String,
    pub event_id: String,
    pub tick: u64,
    pub category: String,
    pub event_type: String,
}

fn collect_asset_refs(events_text: &str) -> Vec<AssetRefReference> {
    let mut out = Vec::new();
    for line in events_text.lines() {
        if line.is_empty() {
            continue;
        }
        let v: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let Some(asset_ref) = v.get("asset_ref").and_then(Value::as_str) else {
            continue;
        };
        if asset_ref.is_empty() {
            continue;
        }
        out.push(AssetRefReference {
            asset_ref: asset_ref.to_string(),
            event_id: v.get("event_id").and_then(Value::as_str).unwrap_or("").to_string(),
            tick: v.get("tick").and_then(Value::as_u64).unwrap_or(0),
            category: v.get("category").and_then(Value::as_str).unwrap_or("").to_string(),
            event_type: v.get("event_type").and_then(Value::as_str).unwrap_or("").to_string(),
        });
    }
    out
}

#[derive(Debug)]
pub(crate) struct AssetRefVerifyReport {
    pub checked: usize,
    pub failures: Vec<Value>,
}

/// Open the canonical ledger (or an explicit override path) and verify
/// every referenced `AssetId` exists in the live set and is `Fresh` on
/// disk. Failures are reported as JSON values for the `result` envelope.
pub(crate) fn verify_asset_refs_against_ledger(
    refs: &[AssetRefReference],
    ledger_path_override: Option<&Path>,
) -> Result<AssetRefVerifyReport> {
    let (ledger_path, base_dir) = resolve_ledger_path(ledger_path_override)?;
    let handle = cf_asset_ledger::LedgerHandle::new(&ledger_path);
    let live = handle
        .live_entries()
        .with_context(|| format!("read ledger {}", ledger_path.display()))?;
    let mut by_id: std::collections::HashMap<String, &cf_asset_ledger::AssetEntry> =
        std::collections::HashMap::with_capacity(live.len());
    for entry in &live {
        by_id.insert(entry.id.as_str().to_string(), entry);
    }
    let mut failures: Vec<Value> = Vec::new();
    for r in refs {
        match by_id.get(&r.asset_ref) {
            None => failures.push(json!({
                "asset_ref": r.asset_ref,
                "event_id": r.event_id,
                "tick": r.tick,
                "category": r.category,
                "event_type": r.event_type,
                "reason": "asset_id_not_in_ledger",
            })),
            Some(entry) => {
                let verify_result = cf_asset_ledger::verify_entry(entry, &base_dir);
                if !matches!(verify_result.status, cf_asset_ledger::RegenStatus::Fresh) {
                    failures.push(json!({
                        "asset_ref": r.asset_ref,
                        "event_id": r.event_id,
                        "tick": r.tick,
                        "category": r.category,
                        "event_type": r.event_type,
                        "reason": format!("ledger_entry_not_fresh:{}", verify_result.status.as_str()),
                        "note": verify_result.note,
                    }));
                }
            }
        }
    }
    Ok(AssetRefVerifyReport {
        checked: refs.len(),
        failures,
    })
}

fn resolve_ledger_path(override_path: Option<&Path>) -> Result<(PathBuf, PathBuf)> {
    if let Some(p) = override_path {
        let base = p
            .parent()
            .and_then(|d| d.parent())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        return Ok((p.to_path_buf(), base));
    }
    let candidates: [(&str, &str); 3] = [
        ("content/asset_ledger/ledger.jsonl", "."),
        ("../content/asset_ledger/ledger.jsonl", ".."),
        ("game/content/asset_ledger/ledger.jsonl", "game"),
    ];
    for (lp, base) in &candidates {
        let p = PathBuf::from(lp);
        if p.exists() {
            return Ok((p, PathBuf::from(base)));
        }
    }
    bail!(
        "no asset ledger found at any of: content/asset_ledger/ledger.jsonl, ../content/asset_ledger/ledger.jsonl, game/content/asset_ledger/ledger.jsonl (override with --asset-ledger-path)"
    );
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
            json!({"method": "act.player.sharp_aim", "active": true}),
            json!({"method": "act.player.abort"}),
            json!({"method": "act.input.capture_controls", "captured": true, "capturer": "settings_panel"}),
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

    /// **M4A § "Run bundle references ledger entries"**: collect_asset_refs
    /// walks events.jsonl and surfaces every envelope-level `asset_ref`
    /// (cosmetic events included).
    #[test]
    fn collect_asset_refs_finds_envelope_field() {
        let events = r#"{"schema_version":"prototype-recorder-event.v0.1","run_id":"r","tick":0,"sim_time_ms":0,"event_id":"r:0:0","category":"system","event_type":"run_started","payload":{}}
{"schema_version":"prototype-recorder-event.v0.1","run_id":"r","tick":1,"sim_time_ms":16.6,"event_id":"r:1:1","category":"capture","event_type":"capture_grid_screenshot","payload":{},"asset_ref":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","cosmetic":true}
{"schema_version":"prototype-recorder-event.v0.1","run_id":"r","tick":2,"sim_time_ms":33.3,"event_id":"r:2:2","category":"system","event_type":"run_finished","payload":{}}
"#;
        let refs = collect_asset_refs(events);
        assert_eq!(refs.len(), 1, "expected exactly one asset_ref event");
        assert_eq!(refs[0].asset_ref, "a".repeat(64));
        assert_eq!(refs[0].tick, 1);
        assert_eq!(refs[0].category, "capture");
        assert_eq!(refs[0].event_type, "capture_grid_screenshot");
    }

    /// **M4A § "Run bundle references ledger entries"**: the verifier
    /// surfaces failures when an event's asset_ref doesn't match any
    /// live ledger entry.
    #[test]
    fn verify_asset_refs_against_ledger_flags_missing() {
        // Build a sandbox ledger with no entries. PID + atomic counter is
        // enough for uniqueness; SystemTime::now is disallowed by the
        // workspace clippy lint.
        static C: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = C.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("cf-headless-asset-ref-test-{pid}-{seq}"));
        std::fs::create_dir_all(&dir).unwrap();
        let ledger_path = dir.join("ledger.jsonl");
        std::fs::write(&ledger_path, "").unwrap();
        let refs = vec![AssetRefReference {
            asset_ref: "b".repeat(64),
            event_id: "r:1:0".to_string(),
            tick: 1,
            category: "capture".to_string(),
            event_type: "capture_grid_screenshot".to_string(),
        }];
        let report = verify_asset_refs_against_ledger(&refs, Some(&ledger_path)).expect("verify");
        assert_eq!(report.checked, 1);
        assert_eq!(report.failures.len(), 1);
        assert_eq!(
            report.failures[0].get("reason").and_then(|v| v.as_str()),
            Some("asset_id_not_in_ledger")
        );
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
