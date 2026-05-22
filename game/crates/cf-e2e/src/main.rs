//! M1.5 scripted E2E runner.
//!
//! `cf-e2e` is the agent-driven harness layered on top of `cfctl`. It:
//!
//! 1. Resolves a scenario id and a script name.
//! 2. Auto-launches `cf-app --headless-smoke --control-api` against an
//!    ephemeral port.
//! 3. Replays the named cfctl script.
//! 4. After the final `observe.once`, asserts on `--expect <key>=<value>` pairs
//!    (e.g. `objective.result=win`, `mission.result=lost`,
//!    `mission.loss_reason=player_dead`, `objective.<id>=completed`).
//! 5. Optionally writes a run bundle through `runbundle.write`.
//!
//! Exit code is `0` when every expectation passes and `1` otherwise. The harness
//! only depends on `cf-control` types for the JSON-RPC envelope; everything else
//! is parsed via `serde_json::Value` so the runner stays tolerant to schema
//! growth.

mod cli;
mod composer;
mod expect;
mod launcher;
mod lookup;
mod script;
mod session;

use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use serde_json::{json, Value};
use tracing_subscriber::EnvFilter;

use cf_control::SCHEMA_VERSION;
use cf_replay::diagnostics;

use crate::cli::Cli;
use crate::composer::{default_composer_script, invoke_composer};
use crate::expect::{json_as_f64, parse_expect, ExpectOp};
use crate::launcher::{launch_cf_app, wait_for_control_port_file, LaunchOptions};
use crate::lookup::lookup;
use crate::script::{locate_script, ticks_to_wait_for, ControlScript};
use crate::session::{wait_for_ws, Session};

fn init_diagnostics() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,cf_=debug")))
        .with_target(true)
        .init();
    diagnostics::init("cf::e2e");
}

#[tokio::main]
async fn main() -> Result<()> {
    init_diagnostics();
    let cli = Cli::parse();
    tracing::info!(target: "cf::e2e", scenario = %cli.scenario, script = ?cli.script, ai_harness = ?cli.ai_harness, "starting cf-e2e");
    let _ = (cli.save_load_roundtrip, cli.verify_checksums);
    // the runner to emit an `ai_test_result {status, duration_seconds,
    // replay_path}` block alongside the standard stdout JSON. Capture the
    // wall-clock start now so the printout at the end can include the
    // measured duration.
    let ai_harness_started_at = cli.ai_harness.as_ref().map(|_| std::time::Instant::now());

    // M2 re-audit (2026-05-13): `--ai-harness` is a spec-canonical alias for
    // `--script` so AI-H-NN scenarios can be invoked with the spec wording.
    // `conflicts_with` on the clap arg already rejects passing both.
    let script_source = cli.script.as_deref().or(cli.ai_harness.as_deref());
    let script_path = match script_source {
        Some(name) => locate_script(name)?,
        None => anyhow::bail!(
            "--script <name> (or --ai-harness <name>) is required for M1.5; M0/M1 inline runs use cfctl run"
        ),
    };
    let script: ControlScript = serde_json::from_str(&std::fs::read_to_string(&script_path)?)
        .with_context(|| format!("parse {}", script_path.display()))?;

    let scenario = if let Some(s) = &script.scenario {
        s.clone()
    } else {
        cli.scenario.clone()
    };
    if scenario != cli.scenario {
        tracing::warn!(target: "cf::e2e", "script scenario {scenario} overrides --scenario {}", cli.scenario);
    }

    // --capture-each-action implies --capture-grid AND raises the baseline
    // frames-per-second so per-action keyframes are distinguishable.
    let effective_capture_grid = cli.capture_grid || cli.capture_each_action;
    let effective_capture_frames_hz = if cli.capture_each_action && cli.capture_frames_hz < 30.0 {
        30.0
    } else {
        cli.capture_frames_hz
    };

    let mut launched = launch_cf_app(LaunchOptions {
        port: cli.control_port,
        scenario: &scenario,
        write_run_bundle: cli.write_run_bundle,
        capture_grid: effective_capture_grid,
        capture_frames_hz: effective_capture_frames_hz,
        no_capture_events: cli.no_capture_events,
        tick_rate_hz: cli.tick_rate_hz,
        ui_scale: cli.ui_scale,
        high_contrast: cli.high_contrast,
        captions: cli.captions.as_bool(),
        reduced_motion: cli.reduced_motion,
        reduced_shake: cli.reduced_shake,
        reduced_flash: cli.reduced_flash,
        unpaced: cli.unpaced,
    })?;
    let control_port = if let Some(port_file) = launched.control_port_file.as_ref() {
        match wait_for_control_port_file(port_file, Duration::from_secs(8)).await {
            Ok(port) => port,
            Err(e) => {
                let _ = launched.child.start_kill();
                anyhow::bail!("cf-app did not report its ephemeral control port: {e}");
            }
        }
    } else {
        cli.control_port
    };
    let url = format!("ws://127.0.0.1:{control_port}");
    let mut session = match wait_for_ws(&url, Duration::from_secs(8)).await {
        Ok(ws) => Session {
            ws,
            next_id: 1,
            child: Some(launched.child),
        },
        Err(e) => {
            let _ = launched.child.start_kill();
            anyhow::bail!("ws connect failed: {e}");
        }
    };

    let mut last_observe: Option<Value> = None;
    let deadline = std::time::Instant::now() + Duration::from_secs(cli.timeout_seconds);
    for step in &script.steps {
        if std::time::Instant::now() > deadline {
            session.shutdown().await;
            anyhow::bail!(
                "script {} timed out after {}s",
                script_path.display(),
                cli.timeout_seconds
            );
        }
        let result = session.send(&step.method, step.params.clone()).await?;
        if step.method == "observe.once" {
            last_observe = Some(result.clone());
        }
        if let Some(extra_ticks) = ticks_to_wait_for(&step.method, &step.params) {
            let target_tick = result.get("effective_tick").and_then(|v| v.as_u64()).unwrap_or(0) + extra_ticks;
            let poll_deadline = std::time::Instant::now() + Duration::from_millis((extra_ticks * 50).max(2_000));
            loop {
                if std::time::Instant::now() > poll_deadline {
                    break;
                }
                let frame = session.send("observe.once", json!({})).await?;
                let live_tick = frame.get("tick").and_then(|t| t.as_u64()).unwrap_or(0);
                if live_tick >= target_tick {
                    last_observe = Some(frame);
                    break;
                }
                tokio::time::sleep(Duration::from_millis(15)).await;
            }
        }
    }
    if cli.write_run_bundle {
        let _ = session.send("runbundle.write", json!({})).await?;
    }

    // M4A: --verify-focus must run BEFORE the cf-app shutdown that
    // capture-grid composition triggers. Drive a full focus cycle through
    // act.input.focus + observe.once so the FINAL observation snapshot we
    // pass to --expect already carries the post-focus state.
    //
    // Single source: `cf_control::HUD_FOCUSABLE_NODES` is the canonical
    // 12-id list; engine + cf-e2e + live_ws_acceptance + cf-app all read
    // from it. Any regression dropping a node fails all consumers together.
    if cli.verify_focus {
        let total_nodes = cf_control::HUD_FOCUSABLE_NODES.len();
        for _ in 0..total_nodes {
            let _ = session.send("act.input.focus", json!({"direction": "next"})).await?;
        }
        let _ = session.send("act.input.focus", json!({"direction": "clear"})).await?;
        let _ = session.send("act.input.focus", json!({"direction": "next"})).await?;
        let post_focus = session.send("observe.once", json!({})).await?;
        last_observe = Some(post_focus);
    }

    if effective_capture_grid {
        // Capture composition needs the engine's run_id; force a final observe.once
        // even if the script never asked for one.
        let final_obs = session.send("observe.once", json!({})).await?;
        last_observe = Some(final_obs);
    }

    let mut observation = last_observe.context("script never executed observe.once")?;

    // Run grid composition. The composer needs `capture_manifest.json` to exist on
    // disk — and that manifest is only written when cf-app's `app.run()` returns
    // (i.e., when cf-app has shut down). So we MUST shut down the cf-app process
    // BEFORE invoking the composer; otherwise the composer fires before the
    // manifest exists and the entire --capture-grid pipeline silently no-ops.
    // Discovered during the T-RELEASE rehearsal (PR #7); the BP1 acceptance bundle
    // was produced by cf-app directly, never through cf-e2e --capture-grid.
    if effective_capture_grid {
        let run_id = observation
            .get("run_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        if let Some(run_id) = run_id {
            // Drain the WS + tear down cf-app so the manifest gets written.
            session.shutdown_app_only().await;
            let bundle_root = cf_replay::resolve_run_bundle_root(None);
            let run_dir = bundle_root.join(&run_id);
            // Belt-and-braces filesystem flush after process exit; on macOS in
            // particular the bundle dir entries can take a tick to propagate.
            tokio::time::sleep(Duration::from_millis(250)).await;
            let composer = cli.composer_script.clone().unwrap_or_else(default_composer_script);
            match invoke_composer(&cli.python_bin, &composer, &run_dir) {
                Ok(stats) => {
                    if let Value::Object(map) = &mut observation {
                        map.insert("capture".into(), stats);
                    }
                }
                Err(e) => {
                    tracing::warn!(target: "cf::e2e", "capture_grid composer failed: {e}");
                }
            }
        } else {
            tracing::warn!(target: "cf::e2e", "observe.once did not return run_id; skipping capture_grid composition");
        }
    }

    let mut all_pass = true;

    // M4A: post-run --verify-focus assertion against the snapshot taken above.
    if cli.verify_focus {
        let total_nodes = cf_control::HUD_FOCUSABLE_NODES.len();

        let required_focusables: Vec<String> = cf_control::HUD_FOCUSABLE_NODES.iter().map(|s| s.to_string()).collect();
        let nodes = observation
            .get("accessibility")
            .and_then(|v| v.get("focusable_nodes"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let nodes_str: Vec<String> = nodes.iter().filter_map(|v| v.as_str().map(String::from)).collect();
        let mut missing: Vec<&str> = Vec::new();
        for req in &required_focusables {
            if !nodes_str.iter().any(|s| s == req) {
                missing.push(req.as_str());
            }
        }
        let focus_cycle = observation
            .get("accessibility")
            .and_then(|v| v.get("focus_cycle"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let focused_node = observation
            .get("accessibility")
            .and_then(|v| v.get("focused_node"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        if missing.is_empty() && focus_cycle >= (total_nodes as u64 + 2) && focused_node.is_some() {
            tracing::info!(
                target: "cf::e2e",
                focusable_count = nodes_str.len(),
                focus_cycle,
                focused_node = ?focused_node,
                "verify_focus PASS"
            );
        } else {
            tracing::error!(
                target: "cf::e2e",
                missing = ?missing,
                actual = ?nodes_str,
                focus_cycle,
                focused_node = ?focused_node,
                expected_focus_cycle = (total_nodes as u64 + 2),
                "verify_focus FAIL"
            );
            all_pass = false;
        }
    }

    for expect in &cli.expect {
        let parsed = match parse_expect(expect) {
            Some(p) => p,
            None => {
                tracing::error!(target: "cf::e2e", expect = %expect, "expectation must be `key=value`, `key>=value`, or `key<=value`");
                all_pass = false;
                continue;
            }
        };
        let actual = lookup(&observation, parsed.key);
        let actual_str = match &actual {
            Some(Value::String(s)) => s.clone(),
            Some(v) => v.to_string(),
            None => "<missing>".to_string(),
        };
        let pass = match parsed.op {
            ExpectOp::Eq => actual_str.trim_matches('"') == parsed.value,
            ExpectOp::Ge | ExpectOp::Le => {
                let lhs = actual.as_ref().and_then(json_as_f64);
                let rhs: Option<f64> = parsed.value.parse().ok();
                match (lhs, rhs, parsed.op) {
                    (Some(a), Some(b), ExpectOp::Ge) => a >= b,
                    (Some(a), Some(b), ExpectOp::Le) => a <= b,
                    _ => false,
                }
            }
        };
        if pass {
            tracing::info!(target: "cf::e2e", expect = %expect, "PASS");
        } else {
            tracing::error!(target: "cf::e2e", expect = %expect, actual = %actual_str, "FAIL");
            all_pass = false;
        }
    }

    session.shutdown().await;
    if !all_pass {
        anyhow::bail!("cf-e2e expectations failed");
    }
    // block when --ai-harness was used. Spec literal: "the runner emits
    // ai_test_result with status, duration, replay_path".
    let mut payload = json!({
        "schema_version": SCHEMA_VERSION,
        "scenario": cli.scenario,
        "script": script_path.display().to_string(),
        "expectations_pass": cli.expect,
        "result": "pass",
    });
    if let (Some(name), Some(started)) = (cli.ai_harness.as_deref(), ai_harness_started_at) {
        let duration_seconds = started.elapsed().as_secs_f64();
        let replay_path = cf_replay::resolve_run_bundle_root(None).display().to_string();
        if let Some(obj) = payload.as_object_mut() {
            // `duration`; `duration_seconds` retained as alias.
            obj.insert(
                "ai_test_result".into(),
                json!({
                    "harness": name,
                    "status": "pass",
                    "duration": duration_seconds,
                    "duration_seconds": duration_seconds,
                    "replay_path": replay_path,
                }),
            );
        }
    }
    println!("{}", serde_json::to_string(&payload).unwrap());
    Ok(())
}
