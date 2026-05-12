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

use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::{net::TcpStream, process::Child, process::Command as TokioCommand, time::sleep};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing_subscriber::EnvFilter;

use cf_control::SCHEMA_VERSION;
use cf_replay::diagnostics;

static CONTROL_PORT_FILE_SEQ: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Parser)]
#[command(name = "cf-e2e", about = "M1.5 scripted end-to-end runner.")]
struct Cli {
    #[arg(long)]
    scenario: String,
    #[arg(long)]
    script: Option<String>,
    /// Expected post-run state in `key=value` form. May be repeated.
    #[arg(long, action = clap::ArgAction::Append)]
    expect: Vec<String>,
    #[arg(long)]
    write_run_bundle: bool,
    #[arg(long, default_value_t = 1.0)]
    ui_scale: f32,
    #[arg(long)]
    high_contrast: bool,
    #[arg(long)]
    verify_focus: bool,
    /// M4A: ACC-A floor — captions on/off (mirrors cf-app's `--captions on|off`).
    #[arg(long, value_enum, default_value_t = CaptionsArg::On)]
    captions: CaptionsArg,
    /// M4A: ACC-A floor — pass through `--reduced-motion` to the spawned cf-app.
    #[arg(long)]
    reduced_motion: bool,
    /// M4A: ACC-A floor — pass through `--reduced-shake` to the spawned cf-app.
    #[arg(long)]
    reduced_shake: bool,
    /// M4A: ACC-A floor — pass through `--reduced-flash` to the spawned cf-app.
    #[arg(long)]
    reduced_flash: bool,
    #[arg(long)]
    save_load_roundtrip: bool,
    #[arg(long)]
    verify_checksums: bool,
    /// Wall-clock timeout for the script runner. M0/M1/M1.5 scripts complete
    /// quickly (mission-win at low ticks); BP2 fun slices (M2.5 micro reactor
    /// defense) require the engine to run a 60s mission timer at the default
    /// 60Hz pace, so the default is now 180s. Pass a smaller value via
    /// `--timeout-seconds` for fast tests if needed.
    #[arg(long, default_value_t = 180)]
    timeout_seconds: u64,
    /// Control API port for the spawned cf-app. 0 chooses an ephemeral free
    /// port so concurrent sweep rows do not collide.
    #[arg(long, default_value_t = 0u16)]
    control_port: u16,
    /// T-CAPTURE: enable cf-capture frame readback + grid composition.
    /// When set, the spawned cf-app runs in windowed mode (NOT --headless-smoke)
    /// so the wgpu swapchain is available for screenshot readback. After the
    /// run, `game/tools/capture_grid.py <run_dir>` is invoked to compose grids.
    #[arg(long)]
    capture_grid: bool,
    #[arg(long, default_value_t = 10.0)]
    capture_frames_hz: f32,
    #[arg(long)]
    no_capture_events: bool,
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
    capture_each_action: bool,
    /// Self-Play Validation Rule "make it possible" clause: lets the harness
    /// drive the spawned cf-app at a non-default sim tick rate so the
    /// "60 Hz default + 120 Hz validation" rate-coverage requirement in the
    /// canonical roadmap can be exercised through a single cf-e2e command
    /// (instead of forcing the agent to drop down to direct cf-app
    /// invocation). 0 = use cf-app's default (60 Hz).
    #[arg(long, default_value_t = 0)]
    tick_rate_hz: u32,
    /// **M1 R2 / Blocker 3b**: drive the spawned cf-app in unpaced mode so
    /// `sim.run_for_ticks` budgets resolve in a handful of Bevy frames
    /// instead of pacing 1 tick per Bevy frame (~60Hz wall-clock).
    /// Required for the m1_5min_endurance script (18000 ticks) which
    /// otherwise takes 300s of wall clock and exceeds the default 180s
    /// timeout. Determinism is preserved — the sim is deterministic
    /// per-tick, only the wall-clock pacing changes.
    #[arg(long)]
    unpaced: bool,
    /// Path to `python3` used to invoke the grid composer. Defaults to `python3`.
    #[arg(long, default_value = "python3")]
    python_bin: String,
    /// Path to `capture_grid.py`. Defaults to `<repo>/game/tools/capture_grid.py`.
    #[arg(long)]
    composer_script: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CaptionsArg {
    On,
    Off,
}

impl CaptionsArg {
    const fn as_bool(self) -> bool {
        matches!(self, Self::On)
    }
}

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
    tracing::info!(target: "cf::e2e", scenario = %cli.scenario, script = ?cli.script, "starting cf-e2e");
    let _ = (cli.save_load_roundtrip, cli.verify_checksums);

    let script_path = match &cli.script {
        Some(name) => locate_script(name)?,
        None => anyhow::bail!("--script <name> is required for M1.5; M0/M1 inline runs use cfctl run"),
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
    println!(
        "{}",
        serde_json::to_string(&json!({
            "schema_version": SCHEMA_VERSION,
            "scenario": cli.scenario,
            "script": script_path.display().to_string(),
            "expectations_pass": cli.expect,
            "result": "pass",
        }))
        .unwrap()
    );
    Ok(())
}

fn ticks_to_wait_for(method: &str, params: &Value) -> Option<u64> {
    match method {
        "sim.step" | "sim.run_for_ticks" => params.get("ticks").and_then(|t| t.as_u64()),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
struct ControlScript {
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    scenario: Option<String>,
    steps: Vec<ScriptStep>,
}

impl ControlScript {
    #[allow(dead_code)]
    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

#[derive(Debug, Deserialize, Clone)]
struct ScriptStep {
    method: String,
    #[serde(default)]
    params: Value,
}

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

struct Session {
    ws: WsStream,
    next_id: i64,
    child: Option<Child>,
}

impl Session {
    async fn send(&mut self, method: &str, mut params: Value) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        if !params.is_object() {
            params = json!({});
        }
        if let Value::Object(ref mut m) = params {
            m.insert("schema_version".to_string(), json!(SCHEMA_VERSION));
        }
        let req = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        self.ws.send(Message::Text(req.to_string().into())).await?;
        loop {
            let msg = tokio::time::timeout(Duration::from_secs(8), self.ws.next())
                .await
                .context("server did not respond within 8s")?;
            let msg = msg.ok_or_else(|| anyhow::anyhow!("ws stream closed before response"))??;
            if let Message::Text(text) = msg {
                let v: Value = serde_json::from_str(&text)?;
                if v.get("id").and_then(|x| x.as_i64()) == Some(id) {
                    if let Some(err) = v.get("error") {
                        anyhow::bail!("server error: {err}");
                    }
                    return Ok(v.get("result").cloned().unwrap_or(Value::Null));
                }
            }
        }
    }

    async fn shutdown(mut self) {
        self.shutdown_app_only().await;
    }

    /// Send `system.shutdown`, close the WS, and wait for the spawned cf-app
    /// child process to exit. Used both as the normal end-of-script teardown
    /// AND as the pre-composer hook for --capture-grid runs (cf-capture only
    /// writes `capture_manifest.json` when cf-app exits, so the composer
    /// MUST run after this returns). Idempotent: calling twice is safe.
    ///
    /// Timeout: 30 s. cf-app's shutdown sequence is:
    ///
    ///   1. AppExit fires → Bevy's run loop returns from `app.run()`.
    ///   2. `wait_for_capture_pngs_flushed` polls the capture log until every
    ///      enqueued PNG has landed on disk (up to 5 s timeout).
    ///   3. `write_capture_manifest_from_handle` writes `capture_manifest.json`.
    ///   4. `finalize_engine` writes the run bundle.
    ///
    /// At ~120 frames/s capture cadence, step 2 alone can sit close to its
    /// 5 s ceiling on slower hardware. The previous 5 s timeout here let cf-e2e
    /// SIGKILL cf-app mid-step-2, leaving `capture_manifest.json` unwritten and
    /// the composer fail-closing on the missing file. Audit-flagged BLOCKER
    /// on 2026-05-09. The 30 s ceiling is comfortably above the 5 + 1 + 1 s
    /// worst-case shutdown plus margin for slower CI hardware.
    async fn shutdown_app_only(&mut self) {
        let _ = self.send("system.shutdown", json!({})).await;
        let _ = self.ws.close(None).await;
        if let Some(mut child) = self.child.take() {
            let _ = tokio::time::timeout(Duration::from_secs(30), child.wait()).await;
            let _ = child.start_kill();
        }
    }
}

struct LaunchOptions<'a> {
    port: u16,
    scenario: &'a str,
    write_run_bundle: bool,
    capture_grid: bool,
    capture_frames_hz: f32,
    no_capture_events: bool,
    /// Optional pass-through for `cf-app --tick-rate-hz`. 0 = use cf-app default.
    tick_rate_hz: u32,
    /// M4A: ACC-A flags forwarded to cf-app's `--ui-scale` / `--high-contrast` /
    /// `--captions on|off` / `--reduced-*`. Defaults match cf-app defaults so a
    /// caller that never set them passes the unmodified surface through.
    ui_scale: f32,
    high_contrast: bool,
    captions: bool,
    reduced_motion: bool,
    reduced_shake: bool,
    reduced_flash: bool,
    /// **M1 R2 / Blocker 3b**: forward `--unpaced` to cf-app so the engine
    /// races through sim.run_for_ticks budgets without per-tick wall-clock
    /// pacing.
    unpaced: bool,
}

struct LaunchedApp {
    child: Child,
    control_port_file: Option<PathBuf>,
}

fn launch_cf_app(opts: LaunchOptions<'_>) -> Result<LaunchedApp> {
    let bin = locate_cf_app_binary()?;
    let control_port_file = if opts.port == 0 {
        Some(unique_control_port_file())
    } else {
        None
    };
    let args = build_cf_app_args(&opts, control_port_file.as_deref());
    // Inherit stdio from the parent so cf-app's diagnostics (especially the
    // bevy_render screenshot INFO lines, ~10/sec under --capture-grid) flow
    // straight to the user's terminal. Piping with Stdio::piped() filled the
    // 64KB pipe buffer in seconds and deadlocked cf-app's render systems
    // when nobody was draining the pipe — the BP2 capture-grid freeze the
    // M2.5 win script kept hitting.
    let child = TokioCommand::new(&bin)
        .args(&args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("failed to spawn {}", bin.display()))?;
    Ok(LaunchedApp {
        child,
        control_port_file,
    })
}

fn build_cf_app_args(opts: &LaunchOptions<'_>, control_port_file: Option<&Path>) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "--scenario".into(),
        opts.scenario.into(),
        "--control-api".into(),
        "--control-port".into(),
        opts.port.to_string(),
        "--ticks".into(),
        "0".into(),
    ];
    if let Some(path) = control_port_file.as_ref() {
        args.push("--control-port-file".into());
        args.push(path.display().to_string());
    }
    if opts.tick_rate_hz != 0 {
        args.push("--tick-rate-hz".into());
        args.push(opts.tick_rate_hz.to_string());
    }
    if !opts.capture_grid {
        // Default: keep the legacy headless path the M0/M1/M1.5 cf-e2e scripts use.
        args.push("--headless-smoke".into());
    }
    if opts.capture_grid {
        args.push("--capture-grid".into());
        args.push("--capture-frames-hz".into());
        args.push(format!("{}", opts.capture_frames_hz));
        if opts.no_capture_events {
            args.push("--no-capture-events".into());
        }
    }
    if opts.write_run_bundle {
        args.push("--write-run-bundle".into());
        args.push("--run-bundle-dir".into());
        args.push(cf_replay::resolve_run_bundle_root(None).display().to_string());
    }
    // M4A ACC-A floor: forward accessibility flags so the spawned cf-app's
    // observe.settings + run_manifest.json + cf-ui HUD reflect the harness's
    // requested posture. cf-app defaults match cf-e2e defaults for ui_scale
    // (1.0), captions (on), high_contrast (false), and the three reduced-*
    // flags (false), so emitting only when non-default keeps the spawn line
    // tight for legacy tests.
    if (opts.ui_scale - 1.0).abs() > f32::EPSILON {
        args.push("--ui-scale".into());
        args.push(format!("{}", opts.ui_scale));
    }
    if opts.high_contrast {
        args.push("--high-contrast".into());
    }
    if !opts.captions {
        args.push("--captions".into());
        args.push("off".into());
    }
    if opts.reduced_motion {
        args.push("--reduced-motion".into());
    }
    if opts.reduced_shake {
        args.push("--reduced-shake".into());
    }
    if opts.reduced_flash {
        args.push("--reduced-flash".into());
    }
    // cf-e2e is the source of truth for scripted actions. Windowed capture
    // still opens a Bevy window, but it must not ingest ambient keyboard or
    // gamepad input from the developer machine and corrupt the scenario path.
    args.push("--disable-local-input".into());
    if opts.unpaced {
        args.push("--unpaced".into());
    }
    args
}

fn unique_control_port_file() -> PathBuf {
    let seq = CONTROL_PORT_FILE_SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("cf_e2e_control_port_{}_{}.txt", std::process::id(), seq))
}

async fn wait_for_control_port_file(path: &Path, timeout: Duration) -> Result<u16> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        match std::fs::read_to_string(path) {
            Ok(text) => {
                let port = text
                    .trim()
                    .parse::<u16>()
                    .with_context(|| format!("parse control port file {}", path.display()))?;
                if port == 0 {
                    anyhow::bail!("control port file {} reported port 0", path.display());
                }
                let _ = std::fs::remove_file(path);
                return Ok(port);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e).with_context(|| format!("read control port file {}", path.display())),
        }
        if std::time::Instant::now() >= deadline {
            anyhow::bail!("timed out waiting for {}", path.display());
        }
        sleep(Duration::from_millis(20)).await;
    }
}

fn locate_cf_app_binary() -> Result<PathBuf> {
    if let Ok(bin) = std::env::var("CF_APP_BIN") {
        if !bin.is_empty() {
            let p = PathBuf::from(bin);
            if p.exists() {
                return Ok(p);
            }
        }
    }
    let exe = std::env::current_exe().context("current_exe")?;
    let dir = exe.parent().context("cf-e2e binary has no parent dir")?;
    let candidates: Vec<PathBuf> = vec![
        dir.join("cf-app"),
        dir.join("cf-app.exe"),
        dir.parent().unwrap_or(Path::new("")).join("cf-app"),
        dir.parent().unwrap_or(Path::new("")).join("cf-app.exe"),
    ];
    for c in &candidates {
        if c.exists() {
            return Ok(c.clone());
        }
    }
    anyhow::bail!("could not locate cf-app binary; set CF_APP_BIN or build cf-app first")
}

fn locate_script(name: &str) -> Result<PathBuf> {
    let candidates = [
        PathBuf::from("scripts/cfctl").join(format!("{name}.cfctl.json")),
        PathBuf::from("../scripts/cfctl").join(format!("{name}.cfctl.json")),
        PathBuf::from("game/scripts/cfctl").join(format!("{name}.cfctl.json")),
    ];
    for c in &candidates {
        if c.exists() {
            return Ok(c.clone());
        }
    }
    anyhow::bail!("script {name} not found at scripts/cfctl/{name}.cfctl.json");
}

async fn wait_for_ws(url: &str, total_timeout: Duration) -> Result<WsStream> {
    let deadline = tokio::time::Instant::now() + total_timeout;
    let mut last_err: Option<anyhow::Error> = None;
    loop {
        if tokio::time::Instant::now() >= deadline {
            return Err(last_err.unwrap_or_else(|| anyhow::anyhow!("ws connect timeout: {url}")));
        }
        match connect_async(url).await {
            Ok((ws, _resp)) => return Ok(ws),
            Err(err) => {
                last_err = Some(anyhow::Error::from(err));
                sleep(Duration::from_millis(150)).await;
            }
        }
    }
}

/// Lookup `key` inside the observe.once result envelope. Supports a couple of
/// shortcuts on top of dotted paths:
///
/// - `mission.result` / `mission.loss_reason` / `mission.active_objective`
/// - `objective.<id>` => `mission.objectives[id==<id>].status`
/// - `breach.<id>.broken` / `breach.<id>.hp` etc.
#[derive(Debug, Clone, Copy)]
enum ExpectOp {
    Eq,
    Ge,
    Le,
}

#[derive(Debug)]
struct Expect<'a> {
    key: &'a str,
    op: ExpectOp,
    value: &'a str,
}

fn parse_expect(raw: &str) -> Option<Expect<'_>> {
    if let Some((k, v)) = raw.split_once(">=") {
        return Some(Expect {
            key: k.trim(),
            op: ExpectOp::Ge,
            value: v.trim(),
        });
    }
    if let Some((k, v)) = raw.split_once("<=") {
        return Some(Expect {
            key: k.trim(),
            op: ExpectOp::Le,
            value: v.trim(),
        });
    }
    raw.split_once('=').map(|(k, v)| Expect {
        key: k.trim(),
        op: ExpectOp::Eq,
        value: v.trim(),
    })
}

fn json_as_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn default_composer_script() -> PathBuf {
    if let Ok(repo) = std::env::var("CARGO_MANIFEST_DIR") {
        let p = PathBuf::from(repo)
            .join("..")
            .join("..")
            .join("tools")
            .join("capture_grid.py");
        if p.exists() {
            return p;
        }
    }
    let exe = std::env::current_exe().unwrap_or_default();
    let mut walk = exe.as_path();
    while let Some(parent) = walk.parent() {
        let candidate = parent.join("game").join("tools").join("capture_grid.py");
        if candidate.exists() {
            return candidate;
        }
        walk = parent;
    }
    PathBuf::from("game/tools/capture_grid.py")
}

fn invoke_composer(python_bin: &str, script: &Path, run_dir: &Path) -> Result<Value> {
    let captures_dir = run_dir.join("captures");
    if !captures_dir.exists() {
        anyhow::bail!(
            "captures dir {} does not exist (cf-app may not have produced any frames)",
            captures_dir.display()
        );
    }
    let output = std::process::Command::new(python_bin)
        .arg(script)
        .arg(run_dir)
        .output()
        .with_context(|| format!("spawn {python_bin} {}", script.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("composer exited with {}: {}", output.status, stderr.trim());
    }
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    serde_json::from_str(&stdout).with_context(|| format!("composer stdout was not JSON: {stdout}"))
}

/// - `enemy.<actor_id>.state` etc.
fn lookup(value: &Value, key: &str) -> Option<Value> {
    // **M1 R2 / Gap G3**: structured event-stream operators. These are useful
    // for cfctl scripts that need to assert "K events of type X with field
    // Y = Z fired during this run." The grammar:
    //
    // - `events.count` ........................ total event count.
    // - `events.<category>.count` ............. count by category.
    // - `events.<category>.<event_type>.count`  count by category+type.
    // - `events.<category>.<event_type>.last.payload.<field>`
    //                                          last matching event's payload field.
    // - `events.where(<f1=v1>,<f2=v2>).count`  count where field=value for
    //                                          all listed fields (and-of).
    // - `events.where(<f1=v1>).last.payload.<field>` analogous.
    //
    // Mission shorthands (deferred to the existing `mission.*` path) remain
    // unchanged.
    if let Some(rest) = key.strip_prefix("events.where(") {
        return lookup_events_where(value, rest);
    }
    if let Some(rest) = key.strip_prefix("events.") {
        return lookup_events_dotted(value, rest);
    }
    if key == "events.count" || key == "events" {
        // `events` alone returns the raw array. `events.count` falls through
        // to the generic walker below (handled there).
        if key == "events" {
            return value.get("events").cloned();
        }
    }
    let parts: Vec<&str> = key.split('.').collect();
    if parts.len() >= 2 && parts[0] == "objective" {
        let id = parts[1];
        let arr = value.get("mission")?.get("objectives")?.as_array()?;
        let obj = arr.iter().find(|o| o.get("id").and_then(|i| i.as_str()) == Some(id))?;
        if parts.len() == 2 {
            return obj.get("status").cloned();
        }
        let mut node = obj;
        for seg in &parts[2..] {
            node = node.get(seg)?;
        }
        return Some(node.clone());
    }
    if parts.len() >= 2 && parts[0] == "breach" {
        let id = parts[1];
        let arr = value.get("breaches")?.as_array()?;
        let strip = arr.iter().find(|s| s.get("id").and_then(|i| i.as_str()) == Some(id))?;
        if parts.len() == 2 {
            return Some(strip.clone());
        }
        let mut node = strip;
        for seg in &parts[2..] {
            node = node.get(seg)?;
        }
        return Some(node.clone());
    }
    if parts.len() >= 2 && parts[0] == "enemy" {
        let actor: u64 = parts[1].parse().ok()?;
        let arr = value.get("enemies")?.as_array()?;
        let enemy = arr
            .iter()
            .find(|e| e.get("actor").and_then(|i| i.as_u64()) == Some(actor))?;
        if parts.len() == 2 {
            return Some(enemy.clone());
        }
        let mut node = enemy;
        for seg in &parts[2..] {
            node = node.get(seg)?;
        }
        return Some(node.clone());
    }
    // M5: `actor.<id>.foo.bar` lookup against `actors[]` by id (`actor.player.*` also accepted).
    // **M1.5 fix**: the actor resolver only fires when the second segment is
    // either the literal "player" or a parseable u64 id. Otherwise the path
    // looks like `actor.<event_type>.count` (a bare event-stream expectation,
    // per the spec text) and we fall through to the event-stream passthrough
    // below. Same reasoning for `breach.<event_type>` and `enemy.<event_type>`.
    if parts.len() >= 2 && parts[0] == "actor" && (parts[1] == "player" || parts[1].parse::<u64>().is_ok()) {
        let arr = value.get("actors")?.as_array()?;
        let actor_match = if parts[1] == "player" {
            let pid = value.get("player_actor_id").and_then(|i| i.as_u64())?;
            arr.iter().find(|a| a.get("id").and_then(|i| i.as_u64()) == Some(pid))?
        } else {
            let pid: u64 = parts[1].parse().ok()?;
            arr.iter().find(|a| a.get("id").and_then(|i| i.as_u64()) == Some(pid))?
        };
        if parts.len() == 2 {
            return Some(actor_match.clone());
        }
        let mut current: Value = actor_match.clone();
        for seg in &parts[2..] {
            if *seg == "count" {
                if let Some(arr) = current.as_array() {
                    current = Value::from(arr.len() as u64);
                    continue;
                }
                return None;
            }
            current = current.get(*seg)?.clone();
        }
        return Some(current);
    }
    // **M1.5 / 10-line parser passthrough**: the M1.5 spec writes bare
    // `ai.state_changed.count>=N`, `terrain.terrain_carved.count>=N`,
    // `mission.objective_completed.count>=N` etc. (no `events.` prefix). If
    // the first segment matches a known event category AND the path's
    // intent is clearly event-stream (last segment ∈ {count, first, last}
    // OR contains a `.last.payload.` / `.first.payload.` drill-down), route
    // through the event-stream resolver. This keeps the spec text honest
    // without colliding with the existing `mission.result` / `mission.loss_reason`
    // /  `mission.objective.<id>.status` / `mission.timer_remaining_ticks`
    // shorthand paths the lookup walker resolves elsewhere.
    const KNOWN_EVENT_CATEGORIES: &[&str] = &[
        "accessibility",
        "actor",
        "ai",
        "chassis",
        "combat",
        "control",
        "determinism",
        "equipment",
        "input",
        "mission",
        "physics",
        "system",
        "terrain",
        "ux",
    ];
    let looks_like_event_stream = parts.last().is_some_and(|seg| {
        matches!(*seg, "count" | "first" | "last")
            || parts
                .windows(2)
                .any(|w| (w[0] == "first" || w[0] == "last") && (w[1] == "payload" || w[1] == "event_id"))
    });
    if parts.len() >= 2 && KNOWN_EVENT_CATEGORIES.contains(&parts[0]) && looks_like_event_stream {
        if let Some(v) = lookup_events_dotted(value, key) {
            return Some(v);
        }
    }
    let mut current: Value = value.clone();
    for seg in &parts {
        if *seg == "count" {
            if let Some(arr) = current.as_array() {
                current = Value::from(arr.len() as u64);
                continue;
            }
            return None;
        }
        let next = current.get(*seg)?.clone();
        current = next;
    }
    Some(current)
}

/// Resolve an `events.<category>[.<event_type>][.last.payload.<field>][.count]`
/// lookup against the observation snapshot's `events` array.
///
/// Grammar:
///   events.<cat>.count                          → matching count
///   events.<cat>.<type>.count                   → cat+type count
///   events.<cat>.<type>.last.payload.<field>    → payload field of last match
///   events.<cat>.<type>.last.event_id           → event_id of last match
fn lookup_events_dotted(observation: &Value, rest: &str) -> Option<Value> {
    let parts: Vec<&str> = rest.split('.').collect();
    if parts.is_empty() {
        return None;
    }
    let events = observation.get("events")?.as_array()?;
    // Single-segment "events.count" already handled by caller's generic walker
    // because it sits on `value.events`. Here we expect at least a category.
    if parts.len() == 1 && parts[0] == "count" {
        return Some(Value::from(events.len() as u64));
    }
    let category = parts[0];
    // Filter by category first.
    let mut filtered: Vec<&Value> = events
        .iter()
        .filter(|e| e.get("category").and_then(|v| v.as_str()) == Some(category))
        .collect();
    let tail = &parts[1..];
    if tail.is_empty() {
        return Some(Value::from(filtered.len() as u64));
    }
    if tail.len() == 1 && tail[0] == "count" {
        return Some(Value::from(filtered.len() as u64));
    }
    // tail[0] may be an event_type filter; if it doesn't look like a special
    // token (count/last/payload/where), treat it as a type filter.
    let mut tail_iter: &[&str] = tail;
    let reserved = ["count", "last", "payload", "first", "event_id"];
    if !reserved.contains(&tail[0]) {
        let event_type = tail[0];
        filtered.retain(|e| e.get("event_type").and_then(|v| v.as_str()) == Some(event_type));
        tail_iter = &tail[1..];
    }
    resolve_event_subpath(&filtered, tail_iter)
}

/// `events.where(category=actor,event_type=inventory_settled).count` style.
/// `rest` begins **after** `events.where(`.
fn lookup_events_where(observation: &Value, rest: &str) -> Option<Value> {
    let close = rest.find(')')?;
    let filter_expr = &rest[..close];
    let after = rest[close + 1..].trim_start_matches('.');
    let events = observation.get("events")?.as_array()?;
    let filters: Vec<(&str, &str)> = filter_expr
        .split(',')
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.trim(), v.trim()))
        .collect();
    if filters.is_empty() {
        return None;
    }
    let filtered: Vec<&Value> = events
        .iter()
        .filter(|e| {
            filters.iter().all(|(k, v)| {
                // The filter key can be a dotted payload path
                // (e.g. payload.zone=head).
                let candidate = if let Some(payload_field) = k.strip_prefix("payload.") {
                    e.get("payload").and_then(|p| p.get(payload_field))
                } else {
                    e.get(*k)
                };
                match candidate {
                    Some(Value::String(s)) => s == v,
                    Some(Value::Bool(b)) => *b == matches!(*v, "true" | "1"),
                    Some(Value::Number(n)) => v.parse::<f64>().is_ok_and(|x| n.as_f64() == Some(x)),
                    _ => false,
                }
            })
        })
        .collect();
    if after.is_empty() {
        return Some(Value::from(filtered.len() as u64));
    }
    let parts: Vec<&str> = after.split('.').collect();
    resolve_event_subpath(&filtered, &parts)
}

fn resolve_event_subpath(filtered: &[&Value], parts: &[&str]) -> Option<Value> {
    if parts.is_empty() || (parts.len() == 1 && parts[0] == "count") {
        return Some(Value::from(filtered.len() as u64));
    }
    if parts[0] == "first" || parts[0] == "last" {
        let target = if parts[0] == "first" {
            filtered.first()
        } else {
            filtered.last()
        };
        let target = target?;
        if parts.len() == 1 {
            return Some((*target).clone());
        }
        if parts[1] == "payload" {
            let payload = target.get("payload")?;
            if parts.len() == 2 {
                return Some(payload.clone());
            }
            let mut cur = payload;
            for seg in &parts[2..] {
                cur = cur.get(*seg)?;
            }
            return Some(cur.clone());
        }
        if parts[1] == "event_id" {
            return target.get("event_id").cloned();
        }
        // Generic dotted walk into the event object.
        let mut cur: &Value = target;
        for seg in &parts[1..] {
            cur = cur.get(*seg)?;
        }
        return Some(cur.clone());
    }
    None
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

    #[tokio::test]
    async fn wait_for_control_port_file_reads_bound_port() {
        let path = unique_control_port_file();
        let _ = std::fs::remove_file(&path);
        std::fs::write(&path, "41234\n").unwrap();

        let port = wait_for_control_port_file(&path, Duration::from_secs(1)).await.unwrap();

        assert_eq!(port, 41234);
        assert!(!path.exists());
    }

    #[test]
    fn cf_app_args_disable_local_input_for_scripted_runs() {
        let port_path = Path::new("/tmp/cf-e2e-port.txt");
        let args = build_cf_app_args(
            &LaunchOptions {
                port: 0,
                scenario: "m4a_micro_breach_readability",
                write_run_bundle: true,
                capture_grid: true,
                capture_frames_hz: 30.0,
                no_capture_events: false,
                tick_rate_hz: 120,
                ui_scale: 2.0,
                high_contrast: true,
                captions: true,
                reduced_motion: true,
                reduced_shake: true,
                reduced_flash: true,
                unpaced: false,
            },
            Some(port_path),
        );

        assert!(args.contains(&"--disable-local-input".to_string()));
        assert!(args.contains(&"--control-port-file".to_string()));
        assert!(!args.contains(&"--headless-smoke".to_string()));
    }

    #[test]
    fn cf_app_args_preserve_explicit_control_port() {
        let args = build_cf_app_args(
            &LaunchOptions {
                port: 17900,
                scenario: "m0_blank",
                write_run_bundle: false,
                capture_grid: false,
                capture_frames_hz: 10.0,
                no_capture_events: false,
                tick_rate_hz: 60,
                ui_scale: 1.0,
                high_contrast: false,
                captions: true,
                reduced_motion: false,
                reduced_shake: false,
                reduced_flash: false,
                unpaced: false,
            },
            None,
        );

        let port_arg = args
            .iter()
            .position(|arg| arg == "--control-port")
            .and_then(|idx| args.get(idx + 1))
            .expect("control port value");
        assert_eq!(port_arg, "17900");
        assert!(!args.contains(&"--control-port-file".to_string()));
    }

    fn fixture_observation() -> Value {
        serde_json::json!({
            "events": [
                {"category": "equipment", "event_type": "weapon_fired",
                 "event_id": "e1",
                 "payload": {"actor": 1, "loudness_radius": 480.0, "bloom_factor": 0.5}},
                {"category": "equipment", "event_type": "weapon_fired",
                 "event_id": "e2",
                 "payload": {"actor": 1, "loudness_radius": 480.0, "bloom_factor": 0.6}},
                {"category": "combat", "event_type": "projectile_hit",
                 "event_id": "e3",
                 "payload": {"shooter": 1, "target": 2, "zone": "torso"}},
                {"category": "combat", "event_type": "projectile_hit",
                 "event_id": "e4",
                 "payload": {"shooter": 1, "target": 2, "zone": "head"}},
                {"category": "actor", "event_type": "inventory_dropped",
                 "event_id": "e5",
                 "payload": {"actor": 2, "item_label": "rifle"}},
                {"category": "actor", "event_type": "inventory_settled",
                 "event_id": "e6",
                 "payload": {"loose_item_id": 0, "item_label": "rifle"}},
            ]
        })
    }

    #[test]
    fn events_dotted_count_filters_by_category_and_type() {
        let obs = fixture_observation();
        assert_eq!(
            lookup(&obs, "events.equipment.weapon_fired.count"),
            Some(Value::from(2u64))
        );
        assert_eq!(
            lookup(&obs, "events.combat.projectile_hit.count"),
            Some(Value::from(2u64))
        );
        assert_eq!(
            lookup(&obs, "events.actor.inventory_settled.count"),
            Some(Value::from(1u64))
        );
        assert_eq!(lookup(&obs, "events.actor.count"), Some(Value::from(2u64)));
    }

    #[test]
    fn events_dotted_last_payload_returns_last_match_field() {
        let obs = fixture_observation();
        assert_eq!(
            lookup(&obs, "events.combat.projectile_hit.last.payload.zone"),
            Some(Value::String("head".into()))
        );
        assert_eq!(
            lookup(&obs, "events.equipment.weapon_fired.last.payload.bloom_factor"),
            Some(Value::from(0.6))
        );
        assert_eq!(
            lookup(&obs, "events.actor.inventory_settled.last.payload.item_label"),
            Some(Value::String("rifle".into()))
        );
    }

    #[test]
    fn events_where_count_with_payload_filter() {
        let obs = fixture_observation();
        assert_eq!(
            lookup(&obs, "events.where(category=combat,payload.zone=head).count"),
            Some(Value::from(1u64))
        );
        assert_eq!(
            lookup(&obs, "events.where(category=combat,payload.zone=torso).count"),
            Some(Value::from(1u64))
        );
        assert_eq!(
            lookup(&obs, "events.where(category=actor,event_type=inventory_settled).count"),
            Some(Value::from(1u64))
        );
    }

    #[test]
    fn events_where_last_payload_drill_down() {
        let obs = fixture_observation();
        assert_eq!(
            lookup(
                &obs,
                "events.where(category=combat,event_type=projectile_hit).last.payload.zone"
            ),
            Some(Value::String("head".into()))
        );
        assert_eq!(
            lookup(&obs, "events.where(category=actor).last.payload.item_label"),
            Some(Value::String("rifle".into()))
        );
    }

    #[test]
    fn events_first_returns_first_match() {
        let obs = fixture_observation();
        assert_eq!(
            lookup(&obs, "events.combat.projectile_hit.first.payload.zone"),
            Some(Value::String("torso".into()))
        );
    }

    #[test]
    fn events_count_for_unknown_type_returns_zero() {
        let obs = fixture_observation();
        assert_eq!(lookup(&obs, "events.combat.nonexistent.count"), Some(Value::from(0u64)));
        assert_eq!(
            lookup(&obs, "events.where(category=unknown).count"),
            Some(Value::from(0u64))
        );
    }

    /// **M1.5 P1**: bare-prefix passthrough so the M1.5 spec's
    /// `ai.state_changed.count>=N` / `terrain.terrain_carved.count>=N`
    /// syntax resolves without an explicit `events.` prefix.
    #[test]
    fn bare_prefix_routes_to_event_stream_when_count_terminator() {
        let obs = fixture_observation();
        assert_eq!(lookup(&obs, "equipment.weapon_fired.count"), Some(Value::from(2u64)));
        assert_eq!(lookup(&obs, "combat.projectile_hit.count"), Some(Value::from(2u64)));
        assert_eq!(lookup(&obs, "actor.inventory_settled.count"), Some(Value::from(1u64)));
    }

    #[test]
    fn bare_prefix_preserves_actor_by_id_resolver() {
        let obs = serde_json::json!({
            "actors": [{"id": 7, "hp": 80}],
            "player_actor_id": 7,
            "events": [],
        });
        assert_eq!(lookup(&obs, "actor.7.hp"), Some(Value::from(80)));
        assert_eq!(lookup(&obs, "actor.player.hp"), Some(Value::from(80)));
    }

    #[test]
    fn bare_prefix_preserves_mission_field_paths() {
        let obs = serde_json::json!({
            "mission": {"result": "won", "loss_reason": null},
            "events": [],
        });
        assert_eq!(lookup(&obs, "mission.result"), Some(Value::String("won".into())));
    }

    #[test]
    fn bare_prefix_last_payload_drill_down() {
        let obs = fixture_observation();
        assert_eq!(
            lookup(&obs, "combat.projectile_hit.last.payload.zone"),
            Some(Value::String("head".into()))
        );
        assert_eq!(
            lookup(&obs, "actor.inventory_settled.last.payload.item_label"),
            Some(Value::String("rifle".into()))
        );
    }
}
