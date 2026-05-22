use std::time::Duration;

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::{net::TcpStream, process::Child, time::sleep};
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

use cf_control::SCHEMA_VERSION;

pub(crate) type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub(crate) struct Session {
    pub(crate) ws: WsStream,
    pub(crate) next_id: i64,
    pub(crate) child: Option<Child>,
}

impl Session {
    pub(crate) async fn send(&mut self, method: &str, mut params: Value) -> Result<Value> {
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

    pub(crate) async fn shutdown(mut self) {
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
    pub(crate) async fn shutdown_app_only(&mut self) {
        let _ = self.send("system.shutdown", json!({})).await;
        let _ = self.ws.close(None).await;
        if let Some(mut child) = self.child.take() {
            let _ = tokio::time::timeout(Duration::from_secs(30), child.wait()).await;
            let _ = child.start_kill();
        }
    }
}

pub(crate) async fn wait_for_ws(url: &str, total_timeout: Duration) -> Result<WsStream> {
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
