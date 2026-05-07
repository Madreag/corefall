//! Local control server implementing the M0 method catalog over WebSocket.
//!
//! M0 methods:
//!   - `scenario.load`             (params: ScenarioLoadParams) -> CommandAck
//!   - `scenario.reset`            -> CommandAck
//!   - `sim.pause` / `sim.resume`  -> CommandAck
//!   - `sim.step`                  (params: StepParams) -> CommandAck
//!   - `sim.run_for_ticks`         (params: RunForTicksParams) -> CommandAck
//!   - `observe.once`              (params: ObserveOnceParams) -> ObserveFrame
//!   - `observe.subscribe`         (params: ObserveSubscribeParams) -> CommandAck
//!     followed by `observe.frame` notifications (server -> client).
//!   - `observe.unsubscribe`       -> CommandAck
//!   - `observe.settings`          -> ObserveSettings
//!   - `act.player.move`           (params: ActPlayerMoveParams) -> CommandAck
//!   - `act.settings.set`          (params: SettingsPatch) -> CommandAck
//!   - `runbundle.write`           (params: RunBundleWriteParams) -> CommandAck
//!   - `system.shutdown`           (params: SystemShutdownParams) -> CommandAck
//!
//! Schema mismatches (`params.schema_version != 1`) reply with -32602 + fix-hint.
//! Unknown methods reply with -32601 (`MethodNotFound`).

use std::{net::SocketAddr, sync::Arc, time::Duration};

use futures_util::{SinkExt, StreamExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{
    net::TcpListener,
    sync::{mpsc, Mutex},
};
use tokio_tungstenite::{accept_async, tungstenite::Message};

use cf_actor::IntentSource;

use crate::{
    envelope::{
        error_codes, JsonRpcError, JsonRpcId, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
        METHOD_OBSERVE_FRAME,
    },
    schemas::SCHEMA_VERSION,
    schemas::{
        ActPlayerAimParams, ActPlayerFireParams, ActPlayerJumpParams, ActPlayerMoveParams, ActPlayerReloadParams,
        ActPlayerResetParams, ActPlayerSelectItemParams, ObserveOnceParams, ObserveSubscribeParams,
        RunBundleWriteParams, RunForTicksParams, ScenarioLoadParams, StepParams, SystemShutdownParams,
    },
    state::{ControlEnvelopeStatus, ObserveFrame, ObserveSettings},
    Settings,
};

#[derive(Debug, Clone, Copy)]
pub struct ControlServerConfig {
    pub bind: SocketAddr,
    pub heartbeat: Duration,
    /// Maximum Hz the server will allow `observe.subscribe` to request.
    /// Bound to the engine's tick rate at server-construction time.
    pub max_observe_hz: u32,
}

impl Default for ControlServerConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:17890".parse().expect("static loopback bind parses"),
            heartbeat: Duration::from_secs(5),
            max_observe_hz: 240,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SettingsPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_scale: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub high_contrast: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub captions: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reduced_motion: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reduced_shake: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reduced_flash: Option<bool>,
}

impl SettingsPatch {
    pub fn is_empty(&self) -> bool {
        self.ui_scale.is_none()
            && self.high_contrast.is_none()
            && self.captions.is_none()
            && self.reduced_motion.is_none()
            && self.reduced_shake.is_none()
            && self.reduced_flash.is_none()
    }
}

#[derive(Debug, Clone)]
pub enum ControlCommand {
    ScenarioLoad { scenario: String, seed: Option<u64> },
    ScenarioReset,
    Pause,
    Resume,
    Step { ticks: u64 },
    RunForTicks { ticks: u64, write_run_bundle: bool },
    ActPlayerMove { x: f32, y: f32, source: IntentSource },
    ActPlayerJump { source: IntentSource },
    ActPlayerAim { x: f32, y: f32, source: IntentSource },
    ActPlayerFire { pressed: bool, source: IntentSource },
    ActPlayerReload { source: IntentSource },
    ActPlayerSelectItem { slot: u32, source: IntentSource },
    ActPlayerReset { source: IntentSource },
    SettingsSet { changes: SettingsPatch },
    RunBundleWrite { id_override: Option<String> },
    Shutdown { write_run_bundle: bool },
}

/// Trait that the engine implements so the server stays decoupled from `cf-app`.
#[async_trait::async_trait]
pub trait EngineHandle: Send + Sync + 'static {
    async fn snapshot(&self, filter: Option<&str>) -> ObserveFrame;
    async fn settings_snapshot(&self) -> Settings;
    async fn dispatch(&self, command: ControlCommand) -> CommandResult;
}

#[derive(Debug, Clone)]
pub struct CommandResult {
    pub status: ControlEnvelopeStatus,
    pub effective_tick: u64,
    pub reason: Option<String>,
}

impl CommandResult {
    pub fn accepted(effective_tick: u64) -> Self {
        Self {
            status: ControlEnvelopeStatus::Accepted,
            effective_tick,
            reason: None,
        }
    }
    pub fn rejected(reason: impl Into<String>, effective_tick: u64) -> Self {
        Self {
            status: ControlEnvelopeStatus::Rejected,
            effective_tick,
            reason: Some(reason.into()),
        }
    }
}

pub struct ControlServer {
    config: ControlServerConfig,
}

impl ControlServer {
    pub fn new(config: ControlServerConfig) -> Self {
        Self { config }
    }

    pub async fn serve<E: EngineHandle>(self, engine: Arc<E>) -> std::io::Result<()> {
        let listener = TcpListener::bind(self.config.bind).await?;
        tracing::info!(target: "cf::ctl", bind = %self.config.bind, "control server listening");
        let max_hz = self.config.max_observe_hz;
        loop {
            let (stream, peer) = listener.accept().await?;
            let engine = engine.clone();
            tokio::spawn(async move {
                if let Err(err) = handle_connection(stream, peer, engine, max_hz).await {
                    tracing::warn!(target: "cf::ctl", %peer, error = %err, "control connection ended with error");
                }
            });
        }
    }

    /// Bind without serving so callers can inspect the actual port (useful for
    /// tests with `127.0.0.1:0` ephemeral binding).
    pub async fn bind(self) -> std::io::Result<(TcpListener, ControlServerConfig)> {
        let listener = TcpListener::bind(self.config.bind).await?;
        let mut cfg = self.config;
        cfg.bind = listener.local_addr()?;
        Ok((listener, cfg))
    }

    pub async fn serve_listener<E: EngineHandle>(
        listener: TcpListener,
        engine: Arc<E>,
        max_observe_hz: u32,
    ) -> std::io::Result<()> {
        loop {
            let (stream, peer) = listener.accept().await?;
            let engine = engine.clone();
            let max_hz = max_observe_hz;
            tokio::spawn(async move {
                if let Err(err) = handle_connection(stream, peer, engine, max_hz).await {
                    tracing::warn!(target: "cf::ctl", %peer, error = %err, "control connection ended with error");
                }
            });
        }
    }
}

async fn handle_connection<E: EngineHandle>(
    stream: tokio::net::TcpStream,
    peer: SocketAddr,
    engine: Arc<E>,
    max_observe_hz: u32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ws_stream = accept_async(stream).await?;
    tracing::info!(target: "cf::ctl", %peer, "control connection accepted");
    let (sink, mut source) = ws_stream.split();
    let sink = Arc::new(Mutex::new(sink));
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Message>();

    let sink_for_writer = sink.clone();
    let writer_handle = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            let mut guard = sink_for_writer.lock().await;
            if guard.send(msg).await.is_err() {
                break;
            }
        }
    });

    let subscribe_hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
    let subscribe_filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    spawn_observation_loop(
        engine.clone(),
        out_tx.clone(),
        subscribe_hz.clone(),
        subscribe_filter.clone(),
    );

    while let Some(message) = source.next().await {
        let message = message?;
        match message {
            Message::Text(text) => {
                let response =
                    process_request(&text, engine.as_ref(), &subscribe_hz, &subscribe_filter, max_observe_hz).await;
                if let Some(payload) = response {
                    let _ = out_tx.send(Message::Text(payload.into()));
                }
            }
            Message::Ping(payload) => {
                let _ = out_tx.send(Message::Pong(payload));
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    drop(out_tx);
    let _ = writer_handle.await;
    tracing::info!(target: "cf::ctl", %peer, "control connection closed");
    Ok(())
}

fn spawn_observation_loop<E: EngineHandle>(
    engine: Arc<E>,
    out_tx: mpsc::UnboundedSender<Message>,
    subscribe_hz: Arc<Mutex<Option<u32>>>,
    subscribe_filter: Arc<Mutex<Option<String>>>,
) {
    tokio::spawn(async move {
        loop {
            let hz_now = { *subscribe_hz.lock().await };
            match hz_now {
                None => {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                Some(hz) => {
                    let hz = hz.max(1);
                    let filter = subscribe_filter.lock().await.clone();
                    let frame = engine.snapshot(filter.as_deref()).await;
                    let notification = JsonRpcNotification {
                        jsonrpc: "2.0".to_string(),
                        method: METHOD_OBSERVE_FRAME.to_string(),
                        params: serde_json::to_value(&frame).unwrap_or(serde_json::Value::Null),
                    };
                    if let Ok(payload) = serde_json::to_string(&notification) {
                        if out_tx.send(Message::Text(payload.into())).is_err() {
                            break;
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(1000 / u64::from(hz))).await;
                }
            }
        }
    });
}

async fn process_request<E: EngineHandle>(
    text: &str,
    engine: &E,
    subscribe_hz: &Arc<Mutex<Option<u32>>>,
    subscribe_filter: &Arc<Mutex<Option<String>>>,
    max_observe_hz: u32,
) -> Option<String> {
    let request: JsonRpcRequest = match serde_json::from_str(text) {
        Ok(r) => r,
        Err(err) => {
            tracing::warn!(target: "cf::ctl", error = %err, "invalid jsonrpc request");
            return Some(error_response(
                JsonRpcId::Null,
                error_codes::INVALID_PARAMS,
                "InvalidRequest",
                json!({"reason": err.to_string()}),
            ));
        }
    };
    if request.jsonrpc != "2.0" {
        return Some(error_response(
            request.id,
            error_codes::INVALID_PARAMS,
            "InvalidRequest",
            json!({"reason": "jsonrpc must be \"2.0\""}),
        ));
    }

    let method = request.method.clone();
    let params = request.params.clone();
    if let Err(err) = check_schema_version(&params) {
        return Some(error_response(
            request.id,
            error_codes::INVALID_PARAMS,
            "InvalidParams",
            err,
        ));
    }

    match method.as_str() {
        "scenario.load" => {
            let p: ScenarioLoadParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ScenarioLoad {
                    scenario: p.scenario,
                    seed: p.seed,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "scenario.reset" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let result = engine.dispatch(ControlCommand::ScenarioReset).await;
            Some(ack_response(request.id, &result))
        }
        "sim.pause" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let result = engine.dispatch(ControlCommand::Pause).await;
            Some(ack_response(request.id, &result))
        }
        "sim.resume" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let result = engine.dispatch(ControlCommand::Resume).await;
            Some(ack_response(request.id, &result))
        }
        "sim.step" => {
            let p: StepParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if p.ticks == 0 {
                return Some(invalid_param_reason(request.id, "ticks_must_be_positive"));
            }
            let result = engine.dispatch(ControlCommand::Step { ticks: p.ticks }).await;
            Some(ack_response(request.id, &result))
        }
        "sim.run_for_ticks" => {
            let p: RunForTicksParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if p.ticks == 0 {
                return Some(invalid_param_reason(request.id, "ticks_must_be_positive"));
            }
            let result = engine
                .dispatch(ControlCommand::RunForTicks {
                    ticks: p.ticks,
                    write_run_bundle: p.write_run_bundle.unwrap_or(false),
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "observe.once" => {
            let p: ObserveOnceParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if p.filter.is_some() {
                return Some(invalid_param_reason(request.id, "observe_filter_not_supported_in_m0"));
            }
            let frame = engine.snapshot(p.filter.as_deref()).await;
            Some(success_response(
                request.id,
                serde_json::to_value(&frame).unwrap_or(serde_json::Value::Null),
            ))
        }
        "observe.subscribe" => {
            let p: ObserveSubscribeParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if p.filter.is_some() {
                return Some(invalid_param_reason(request.id, "observe_filter_not_supported_in_m0"));
            }
            let hz = p.hz.unwrap_or(10);
            if hz == 0 || hz > max_observe_hz {
                return Some(error_response(
                    request.id,
                    error_codes::INVALID_PARAMS,
                    "InvalidParams",
                    json!({
                        "reason": "observe_hz_out_of_range",
                        "min_hz": 1,
                        "max_hz": max_observe_hz,
                    }),
                ));
            }
            *subscribe_hz.lock().await = Some(hz);
            *subscribe_filter.lock().await = p.filter;
            Some(ack_response(request.id, &CommandResult::accepted(0)))
        }
        "observe.unsubscribe" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            *subscribe_hz.lock().await = None;
            *subscribe_filter.lock().await = None;
            Some(ack_response(request.id, &CommandResult::accepted(0)))
        }
        "observe.settings" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let settings = engine.settings_snapshot().await;
            let view = ObserveSettings {
                schema_version: SCHEMA_VERSION,
                settings,
            };
            Some(success_response(
                request.id,
                serde_json::to_value(&view).unwrap_or(serde_json::Value::Null),
            ))
        }
        "act.player.move" => {
            let p: ActPlayerMoveParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if !p.x.is_finite() || !p.y.is_finite() {
                return Some(invalid_param_reason(request.id, "axis_must_be_finite"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerMove {
                    x: p.x,
                    y: p.y,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.jump" => {
            let _p: ActPlayerJumpParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerJump {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.aim" => {
            let p: ActPlayerAimParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if !p.x.is_finite() || !p.y.is_finite() {
                return Some(invalid_param_reason(request.id, "aim_must_be_finite"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerAim {
                    x: p.x,
                    y: p.y,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.fire" => {
            let p: ActPlayerFireParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerFire {
                    pressed: p.pressed,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.reload" => {
            let _p: ActPlayerReloadParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerReload {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.select_item" => {
            let p: ActPlayerSelectItemParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerSelectItem {
                    slot: p.slot,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.reset" => {
            let _p: ActPlayerResetParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerReset {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.settings.set" => {
            // Accept either a flat object {schema_version, ui_scale, ...} or a wrapped {schema_version, patch:{...}}.
            let patch_value = if params.get("patch").is_some() {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct WrappedPatch {
                    schema_version: u32,
                    patch: SettingsPatch,
                }
                let wrapped: WrappedPatch = match serde_json::from_value(params) {
                    Ok(v) => v,
                    Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
                };
                let _ = wrapped.schema_version;
                serde_json::to_value(wrapped.patch).unwrap_or(serde_json::Value::Null)
            } else {
                let mut p = params.clone();
                if let Some(o) = p.as_object_mut() {
                    o.remove("schema_version");
                }
                p
            };
            let patch: SettingsPatch = match serde_json::from_value(patch_value) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if patch.is_empty() {
                return Some(invalid_param_reason(request.id, "settings_patch_empty"));
            }
            let result = engine.dispatch(ControlCommand::SettingsSet { changes: patch }).await;
            Some(ack_response(request.id, &result))
        }
        "runbundle.write" => {
            let p: RunBundleWriteParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if p.id_override.is_some() {
                return Some(invalid_param_reason(
                    request.id,
                    "runbundle_id_override_not_supported_in_m0",
                ));
            }
            let result = engine
                .dispatch(ControlCommand::RunBundleWrite {
                    id_override: p.id_override,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "system.shutdown" => {
            let p: SystemShutdownParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::Shutdown {
                    write_run_bundle: p.write_run_bundle.unwrap_or(false),
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        _ => Some(error_response(
            request.id,
            error_codes::METHOD_NOT_FOUND,
            "MethodNotFound",
            json!({"method": method, "fix_hint": "see spec/ai-control-observability-layer.md M0 method catalog"}),
        )),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SchemaOnlyParams {
    schema_version: u32,
}

fn parse_schema_only(id: JsonRpcId, params: serde_json::Value) -> Result<(), String> {
    let parsed: SchemaOnlyParams =
        serde_json::from_value(params).map_err(|err| missing_param_error(id, &err.to_string()))?;
    let _ = parsed.schema_version;
    Ok(())
}

/// `schema_version` is MANDATORY on every M0 client→server JSON-RPC request. Missing,
/// non-numeric, or mismatched values reject with `-32602` (`InvalidParams`). There are no
/// notification methods in the M0 catalog; if a future milestone adds one, document it
/// here as an explicit exception.
fn check_schema_version(params: &serde_json::Value) -> Result<(), serde_json::Value> {
    let obj = match params.as_object() {
        Some(o) => o,
        None => {
            return Err(json!({
                "reason": "schema_version_missing",
                "server_version": SCHEMA_VERSION,
                "fix_hint": format!("All M0 control methods require an object `params` containing `schema_version: {SCHEMA_VERSION}`."),
            }));
        }
    };
    match obj.get("schema_version").and_then(|v| v.as_u64()) {
        None => Err(json!({
            "reason": "schema_version_missing",
            "server_version": SCHEMA_VERSION,
            "fix_hint": format!("All M0 control methods require `params.schema_version: {SCHEMA_VERSION}`."),
        })),
        Some(v) if v as u32 == SCHEMA_VERSION => Ok(()),
        Some(other) => Err(json!({
            "reason": "schema_version_mismatch",
            "server_version": SCHEMA_VERSION,
            "client_version": other,
            "fix_hint": format!("Upgrade cfctl or pin client schema_version: {SCHEMA_VERSION}"),
        })),
    }
}

fn ack_response(id: JsonRpcId, result: &CommandResult) -> String {
    let (status_str, error_obj) = match result.status {
        ControlEnvelopeStatus::Accepted => ("accepted", None),
        ControlEnvelopeStatus::Queued => ("queued", None),
        ControlEnvelopeStatus::Rejected => (
            "rejected",
            Some(JsonRpcError {
                code: error_codes::COMMAND_REJECTED,
                message: "command_rejected".to_string(),
                data: Some(json!({
                    "reason": result.reason.clone().unwrap_or_else(|| "unknown".to_string()),
                    "tick": result.effective_tick
                })),
            }),
        ),
    };
    if let Some(error) = error_obj {
        let resp = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        };
        return serde_json::to_string(&resp).unwrap_or_default();
    }
    let payload = json!({
        "schema_version": SCHEMA_VERSION,
        "status": status_str,
        "effective_tick": result.effective_tick,
        "reason": result.reason
    });
    success_response(id, payload)
}

fn success_response(id: JsonRpcId, value: serde_json::Value) -> String {
    let resp = JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(value),
        error: None,
    };
    serde_json::to_string(&resp).unwrap_or_default()
}

fn error_response(id: JsonRpcId, code: i32, message: &str, data: serde_json::Value) -> String {
    let resp = JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(JsonRpcError {
            code,
            message: message.to_string(),
            data: Some(data),
        }),
    };
    serde_json::to_string(&resp).unwrap_or_default()
}

fn missing_param_error(id: JsonRpcId, reason: &str) -> String {
    error_response(
        id,
        error_codes::INVALID_PARAMS,
        "InvalidParams",
        json!({"reason": reason, "fix_hint": "see spec/prototype-roadmap CLI Reference for the M0 method catalog"}),
    )
}

fn invalid_param_reason(id: JsonRpcId, reason: &str) -> String {
    error_response(
        id,
        error_codes::INVALID_PARAMS,
        "InvalidParams",
        json!({"reason": reason, "fix_hint": "see spec/prototype-roadmap CLI Reference for the M0 method catalog"}),
    )
}

pub use async_trait::async_trait;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::RunStatus;

    #[derive(Default)]
    struct StubEngine;

    #[async_trait::async_trait]
    impl EngineHandle for StubEngine {
        async fn snapshot(&self, _filter: Option<&str>) -> ObserveFrame {
            ObserveFrame {
                schema_version: SCHEMA_VERSION,
                run_id: "stub".to_string(),
                tick: 0,
                sim_time_ms: 0.0,
                run_status: RunStatus::Paused,
                scenario: "m0_blank".to_string(),
                events_since: 0,
                events: vec![],
                settings: ObserveSettings {
                    schema_version: SCHEMA_VERSION,
                    settings: Settings::default(),
                },
                actors: vec![],
                player_actor_id: None,
            }
        }
        async fn settings_snapshot(&self) -> Settings {
            Settings::default()
        }
        async fn dispatch(&self, _command: ControlCommand) -> CommandResult {
            CommandResult::accepted(0)
        }
    }

    #[tokio::test]
    async fn schema_mismatch_returns_invalid_params() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.player.move",
            "params": {"schema_version": 99, "x": 1.0}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let error = parsed.error.expect("schema mismatch produces an error");
        assert_eq!(error.code, error_codes::INVALID_PARAMS);
        let data = error.data.unwrap();
        assert_eq!(data.get("reason").unwrap(), "schema_version_mismatch");
        assert_eq!(data.get("server_version").unwrap(), 1);
        assert_eq!(data.get("client_version").unwrap(), 99);
    }

    /// M0.2-F2: Every M0 method must reject a request with missing `params.schema_version`.
    /// Pre-fix, several handlers (`act.settings.set`, `runbundle.write`, `system.shutdown`,
    /// `observe.subscribe/unsubscribe`, `observe.once`) silently defaulted when the field
    /// was absent. Now `check_schema_version` requires it before any handler runs.
    #[tokio::test]
    async fn missing_schema_version_rejects_every_m0_method() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let methods_with_params: &[(&str, serde_json::Value)] = &[
            ("scenario.load", json!({"scenario": "m0_blank"})),
            ("scenario.reset", json!({})),
            ("sim.pause", json!({})),
            ("sim.resume", json!({})),
            ("sim.step", json!({"ticks": 1})),
            ("sim.run_for_ticks", json!({"ticks": 30})),
            ("observe.once", json!({})),
            ("observe.subscribe", json!({"hz": 10})),
            ("observe.unsubscribe", json!({})),
            ("observe.settings", json!({})),
            ("act.player.move", json!({"x": 1.0, "y": 0.0})),
            ("act.settings.set", json!({"ui_scale": 2.0})),
            ("runbundle.write", json!({})),
            ("system.shutdown", json!({})),
        ];
        for (method, params) in methods_with_params {
            let req = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
            let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
                .await
                .unwrap();
            let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
            let error = parsed
                .error
                .unwrap_or_else(|| panic!("{method} must reject missing schema_version"));
            assert_eq!(
                error.code,
                error_codes::INVALID_PARAMS,
                "{method} must return -32602 (InvalidParams) on missing schema_version, got code {}",
                error.code
            );
            let data = error.data.unwrap();
            assert_eq!(
                data.get("reason").unwrap(),
                "schema_version_missing",
                "{method} returned wrong reason on missing schema_version"
            );
        }
    }

    /// M0.2-F2: A request that omits `params` entirely must also reject. (clap-driven
    /// JSON-RPC clients commonly send `{"method": "...", "id": ...}` without params for
    /// no-arg methods; the server must still demand schema_version.)
    #[tokio::test]
    async fn missing_params_object_rejects() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({"jsonrpc": "2.0", "id": 1, "method": "sim.pause"});
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let error = parsed.error.expect("missing params must reject");
        assert_eq!(error.code, error_codes::INVALID_PARAMS);
        assert_eq!(error.data.unwrap().get("reason").unwrap(), "schema_version_missing");
    }

    #[tokio::test]
    async fn unknown_method_returns_method_not_found() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.player.fly",
            "params": {"schema_version": 1}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let error = parsed.error.expect("unknown method returns error");
        assert_eq!(error.code, error_codes::METHOD_NOT_FOUND);
    }

    #[tokio::test]
    async fn observe_once_returns_frame() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "observe.once",
            "params": {"schema_version": 1}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let result = parsed.result.expect("observe.once returns success");
        assert_eq!(result.get("scenario").unwrap(), "m0_blank");
        assert_eq!(result.get("schema_version").unwrap(), 1);
    }

    #[tokio::test]
    async fn settings_set_dispatches_patch() {
        struct CaptureEngine {
            patch: Mutex<Option<SettingsPatch>>,
        }
        #[async_trait::async_trait]
        impl EngineHandle for CaptureEngine {
            async fn snapshot(&self, _filter: Option<&str>) -> ObserveFrame {
                StubEngine.snapshot(_filter).await
            }
            async fn settings_snapshot(&self) -> Settings {
                Settings::default()
            }
            async fn dispatch(&self, command: ControlCommand) -> CommandResult {
                if let ControlCommand::SettingsSet { changes } = command {
                    *self.patch.lock().await = Some(changes);
                }
                CommandResult::accepted(0)
            }
        }
        let engine = CaptureEngine {
            patch: Mutex::new(None),
        };
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.settings.set",
            "params": {"schema_version": 1, "ui_scale": 1.5, "high_contrast": true}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        assert!(parsed.result.is_some());
        let captured = engine.patch.lock().await.clone().unwrap();
        assert_eq!(captured.ui_scale, Some(1.5));
        assert_eq!(captured.high_contrast, Some(true));
        assert_eq!(captured.captions, None);
    }

    /// Contract-integrity regression: every M0 handler must reject unknown fields instead
    /// of accepting a request whose extra data was silently ignored.
    #[tokio::test]
    async fn unknown_params_reject_every_m0_method() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let methods_with_params: &[(&str, serde_json::Value)] = &[
            (
                "scenario.load",
                json!({"schema_version": 1, "scenario": "m0_blank", "unexpected": true}),
            ),
            ("scenario.reset", json!({"schema_version": 1, "unexpected": true})),
            ("sim.pause", json!({"schema_version": 1, "unexpected": true})),
            ("sim.resume", json!({"schema_version": 1, "unexpected": true})),
            ("sim.step", json!({"schema_version": 1, "ticks": 1, "unexpected": true})),
            (
                "sim.run_for_ticks",
                json!({"schema_version": 1, "ticks": 30, "unexpected": true}),
            ),
            ("observe.once", json!({"schema_version": 1, "unexpected": true})),
            (
                "observe.subscribe",
                json!({"schema_version": 1, "hz": 10, "unexpected": true}),
            ),
            ("observe.unsubscribe", json!({"schema_version": 1, "unexpected": true})),
            ("observe.settings", json!({"schema_version": 1, "unexpected": true})),
            (
                "act.player.move",
                json!({"schema_version": 1, "x": 1.0, "unexpected": true}),
            ),
            (
                "act.settings.set",
                json!({"schema_version": 1, "ui_scale": 2.0, "unexpected": true}),
            ),
            ("runbundle.write", json!({"schema_version": 1, "unexpected": true})),
            ("system.shutdown", json!({"schema_version": 1, "unexpected": true})),
        ];
        for (method, params) in methods_with_params {
            let req = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
            let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
                .await
                .unwrap();
            let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
            let error = parsed
                .error
                .unwrap_or_else(|| panic!("{method} must reject unknown params"));
            assert_eq!(
                error.code,
                error_codes::INVALID_PARAMS,
                "{method} must reject unknown params"
            );
            assert!(
                parsed.result.is_none(),
                "{method} must not return success with unknown params"
            );
        }
    }

    #[tokio::test]
    async fn unsupported_m0_params_reject_before_dispatch() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let cases: &[(&str, serde_json::Value, &str)] = &[
            (
                "sim.step",
                json!({"schema_version": 1, "ticks": 0}),
                "ticks_must_be_positive",
            ),
            (
                "sim.run_for_ticks",
                json!({"schema_version": 1, "ticks": 0}),
                "ticks_must_be_positive",
            ),
            (
                "observe.once",
                json!({"schema_version": 1, "filter": "system"}),
                "observe_filter_not_supported_in_m0",
            ),
            (
                "observe.subscribe",
                json!({"schema_version": 1, "hz": 0}),
                "observe_hz_out_of_range",
            ),
            (
                "observe.subscribe",
                json!({"schema_version": 1, "hz": 241}),
                "observe_hz_out_of_range",
            ),
            (
                "observe.subscribe",
                json!({"schema_version": 1, "filter": "system"}),
                "observe_filter_not_supported_in_m0",
            ),
            ("act.settings.set", json!({"schema_version": 1}), "settings_patch_empty"),
            (
                "runbundle.write",
                json!({"schema_version": 1, "id_override": "manual"}),
                "runbundle_id_override_not_supported_in_m0",
            ),
        ];
        for (method, params, reason) in cases {
            let req = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
            let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
                .await
                .unwrap();
            let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
            let error = parsed.error.unwrap_or_else(|| panic!("{method} must reject {params}"));
            assert_eq!(error.code, error_codes::INVALID_PARAMS, "{method} wrong error code");
            assert_eq!(
                error.data.unwrap().get("reason").unwrap(),
                reason,
                "{method} wrong reason"
            );
        }
    }
}
