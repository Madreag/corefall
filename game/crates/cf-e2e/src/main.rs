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

    let mut child = launch_cf_app(cli.control_port, &scenario, cli.write_run_bundle)?;
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

    let observation = last_observe.context("script never executed observe.once")?;
    let mut all_pass = true;
    for expect in &cli.expect {
        let (key, expected_value) = match expect.split_once('=') {
            Some((k, v)) => (k, v),
            None => {
                tracing::error!(target: "cf::e2e", expect = %expect, "expectation must be `key=value`");
                all_pass = false;
                continue;
            }
        };
        let actual = lookup(&observation, key);
        let actual_str = match &actual {
            Some(Value::String(s)) => s.clone(),
            Some(v) => v.to_string(),
            None => "<missing>".to_string(),
        };
        if actual_str.trim_matches('"') == expected_value {
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
        let _ = self.send("system.shutdown", json!({})).await;
        let _ = self.ws.close(None).await;
        if let Some(mut child) = self.child.take() {
            let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
            let _ = child.start_kill();
        }
    }
}

fn launch_cf_app(port: u16, scenario: &str, write_run_bundle: bool) -> Result<Child> {
    let bin = locate_cf_app_binary()?;
    let mut args: Vec<String> = vec![
        "--scenario".into(),
        scenario.into(),
        "--headless-smoke".into(),
        "--control-api".into(),
        "--control-port".into(),
        port.to_string(),
        "--ticks".into(),
        "0".into(),
    ];
    if write_run_bundle {
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
