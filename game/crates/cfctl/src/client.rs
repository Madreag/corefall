use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command as TokioCommand},
    time::sleep,
};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

use cf_control::{runtime::resolve_run_bundle_root, SCHEMA_VERSION};

pub type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// JSON-RPC session over WebSocket. Auto-launches `cf-app --headless-smoke
/// --control-api` if `--connect` is not provided and `--no-auto-launch` is not
/// set.
pub struct Session {
    ws: WsStream,
    next_id: i64,
    spawned_child: Option<Child>,
}

impl Session {
    pub async fn open(connect: &Option<String>, auto_launch_port: u16, no_auto_launch: bool) -> Result<Self> {
        Self::open_with(connect, auto_launch_port, no_auto_launch, AutoLaunchOpts::default()).await
    }

    /// Open with explicit auto-launch options (used by `script run --write-run-bundle` and
    /// `runbundle write` so only those subcommands ask the auto-launched cf-app to write a
    /// bundle on exit).
    pub async fn open_with(
        connect: &Option<String>,
        auto_launch_port: u16,
        no_auto_launch: bool,
        opts: AutoLaunchOpts,
    ) -> Result<Self> {
        let (addr, child) = if let Some(addr) = connect.clone() {
            (addr, None)
        } else if no_auto_launch {
            anyhow::bail!("no --connect supplied and --no-auto-launch is set; nothing to talk to");
        } else {
            let child = launch_cf_app(auto_launch_port, opts).await?;
            (format!("127.0.0.1:{auto_launch_port}"), Some(child))
        };
        let url = format!("ws://{}", addr);
        let ws = wait_for_ws(&url, Duration::from_secs(8)).await?;
        Ok(Self {
            ws,
            next_id: 1,
            spawned_child: child,
        })
    }

    pub async fn send_request(&mut self, method: &str, params_no_schema: Value) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        let mut params = match params_no_schema {
            Value::Object(_) | Value::Null => params_no_schema,
            other => json!({"_unwrapped": other}),
        };
        if let Value::Object(ref mut m) = params {
            m.insert("schema_version".to_string(), json!(SCHEMA_VERSION));
        } else {
            params = json!({"schema_version": SCHEMA_VERSION});
        }
        let req = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
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

    pub async fn subscribe(&mut self, hz: u32, _scenario: Option<String>) -> Result<()> {
        let _ = self.send_request("observe.subscribe", json!({"hz": hz})).await?;
        Ok(())
    }

    pub async fn unsubscribe(&mut self) -> Result<()> {
        let _ = self.send_request("observe.unsubscribe", json!({})).await?;
        Ok(())
    }

    pub async fn next_observe_frame(&mut self, timeout: Duration) -> Result<Value> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let msg = tokio::time::timeout_at(deadline, self.ws.next())
                .await
                .context("observe.frame timeout")?;
            let msg = msg.ok_or_else(|| anyhow::anyhow!("ws stream closed before frame"))??;
            if let Message::Text(text) = msg {
                let v: Value = serde_json::from_str(&text)?;
                if v.get("method").and_then(|x| x.as_str()) == Some("observe.frame") {
                    return Ok(v.get("params").cloned().unwrap_or(Value::Null));
                }
            }
        }
    }

    pub async fn close(mut self) -> Result<()> {
        if self.spawned_child.is_some() {
            let _ = self.send_request("system.shutdown", json!({})).await;
        }
        let _ = self.ws.close(None).await;
        if let Some(mut child) = self.spawned_child.take() {
            let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
            let _ = child.start_kill();
        }
        Ok(())
    }
}

/// Options for auto-launching `cf-app` from a `cfctl` subcommand.
#[derive(Debug, Clone, Default)]
pub struct AutoLaunchOpts {
    /// Pass `--write-run-bundle` to the auto-launched cf-app. Only set this for subcommands
    /// that actually need bundle evidence (`script run --write-run-bundle`, `runbundle write`,
    /// observability scripts that call `runbundle.write`). For `observe --once`, `pause`, etc.,
    /// leave it false to keep `prototype_runs/native/` clean. (L4 fix.)
    pub write_run_bundle: bool,
    /// Scenario id to load in the auto-launched cf-app. Defaults to `m0_blank`. M1 scripts
    /// that need an actor world override this with `m1_actor_range` so `act.player.*` works.
    pub scenario: Option<String>,
}

async fn launch_cf_app(port: u16, opts: AutoLaunchOpts) -> Result<Child> {
    let bin = locate_cf_app_binary()?;
    let scenario = opts.scenario.clone().unwrap_or_else(|| "m0_blank".into());
    let mut args: Vec<String> = vec![
        "--scenario".into(),
        scenario,
        "--headless-smoke".into(),
        "--control-api".into(),
        "--control-port".into(),
        port.to_string(),
        "--ticks".into(),
        "0".into(),
    ];
    if opts.write_run_bundle {
        args.push("--write-run-bundle".into());
        args.push("--run-bundle-dir".into());
        args.push(resolve_run_bundle_root(None).display().to_string());
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
    let dir = exe.parent().context("cfctl binary has no parent dir")?;
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
    let output = std::process::Command::new("cargo")
        .args(["build", "-p", "cf-app", "--message-format=json"])
        .output();
    if let Ok(o) = output {
        if o.status.success() {
            for line in String::from_utf8_lossy(&o.stdout).lines() {
                if line.contains("\"target\"") && line.contains("\"cf-app\"") && line.contains("\"executable\"") {
                    if let Ok(v) = serde_json::from_str::<Value>(line) {
                        if let Some(exe) = v.get("executable").and_then(|x| x.as_str()) {
                            return Ok(PathBuf::from(exe));
                        }
                    }
                }
            }
        }
    }
    anyhow::bail!("could not locate cf-app binary; set CF_APP_BIN or run `cargo build -p cf-app` first")
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

#[allow(dead_code)]
pub async fn drain_subprocess_stderr(child: &mut Child) -> Result<()> {
    if let Some(stderr) = child.stderr.take() {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            eprintln!("[cf-app] {line}");
        }
    }
    Ok(())
}
