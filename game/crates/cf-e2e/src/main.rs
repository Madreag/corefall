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
    time::Duration,
};

use anyhow::{Context, Result};
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::{net::TcpStream, process::Child, process::Command as TokioCommand, time::sleep};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use tracing_subscriber::EnvFilter;

use cf_control::SCHEMA_VERSION;
use cf_replay::diagnostics;

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
    #[arg(long)]
    save_load_roundtrip: bool,
    #[arg(long)]
    verify_checksums: bool,
    #[arg(long, default_value_t = 60)]
    timeout_seconds: u64,
    #[arg(long, default_value_t = 17900u16)]
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
    /// Path to `python3` used to invoke the grid composer. Defaults to `python3`.
    #[arg(long, default_value = "python3")]
    python_bin: String,
    /// Path to `capture_grid.py`. Defaults to `<repo>/game/tools/capture_grid.py`.
    #[arg(long)]
    composer_script: Option<PathBuf>,
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
    let _ = (
        cli.ui_scale,
        cli.high_contrast,
        cli.verify_focus,
        cli.save_load_roundtrip,
    );
    let _ = cli.verify_checksums;

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

    let mut child = launch_cf_app(LaunchOptions {
        port: cli.control_port,
        scenario: &scenario,
        write_run_bundle: cli.write_run_bundle,
        capture_grid: cli.capture_grid,
        capture_frames_hz: cli.capture_frames_hz,
        no_capture_events: cli.no_capture_events,
    })?;
    let url = format!("ws://127.0.0.1:{}", cli.control_port);
    let mut session = match wait_for_ws(&url, Duration::from_secs(8)).await {
        Ok(ws) => Session {
            ws,
            next_id: 1,
            child: Some(child),
        },
        Err(e) => {
            let _ = child.start_kill();
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

    if cli.capture_grid {
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
    if cli.capture_grid {
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
    async fn shutdown_app_only(&mut self) {
        let _ = self.send("system.shutdown", json!({})).await;
        let _ = self.ws.close(None).await;
        if let Some(mut child) = self.child.take() {
            let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
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
}

fn launch_cf_app(opts: LaunchOptions<'_>) -> Result<Child> {
    let bin = locate_cf_app_binary()?;
    let mut args: Vec<String> = vec![
        "--scenario".into(),
        opts.scenario.into(),
        "--control-api".into(),
        "--control-port".into(),
        opts.port.to_string(),
        "--ticks".into(),
        "0".into(),
    ];
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
    let child = TokioCommand::new(&bin)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn {}", bin.display()))?;
    Ok(child)
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
    let mut node = value;
    for seg in &parts {
        node = node.get(seg)?;
    }
    Some(node.clone())
}
