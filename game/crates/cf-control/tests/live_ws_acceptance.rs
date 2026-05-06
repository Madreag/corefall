//! M0.2 acceptance: real WebSocket round-trips against the production `ControlServer`.
//!
//! These tests bind `127.0.0.1:0` (ephemeral port), spin up the real `ControlServer`
//! against an `M0Engine`, and drive it via raw `tokio_tungstenite` WebSocket frames so
//! we exercise the same code path `cfctl` does — including the mandatory
//! `schema_version` check and the `scenario.load --seed` rejection logic.

use std::{path::PathBuf, sync::Arc, time::Duration};

use cf_control::{
    engine::M0Engine, runtime::build_engine_config, server::ControlServer, ConfigInputs, ControlServerConfig, Settings,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio_tungstenite::{connect_async, tungstenite::Message};

static WS_TEST_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn write_blank_scenario() -> PathBuf {
    let mut p = std::env::temp_dir();
    let seq = WS_TEST_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let nanos = cf_sim_core::WallClock
        .now_utc()
        .timestamp_nanos_opt()
        .unwrap_or_default();
    p.push(format!(
        "cf_control_ws_test_{}_{}_{}.ron",
        std::process::id(),
        seq,
        nanos
    ));
    std::fs::write(
        &p,
        r#"(
  schema_version: 1,
  id: "m0_blank",
  display_name: "M0 Blank",
  description: "WS acceptance fixture",
  seed: 42,
  duration_ticks: Some(60),
  region: (anchor: (0.0, 0.0), width: 1280.0, height: 720.0),
  gravity: -980.0,
  teams: [],
  actors: [],
  objectives: [],
  director: None,
  capabilities: (debug: false, control_api: true, save_load: false),
  save_fields: [],
  expected_tests: ["M0-SMOKE-01"],
  notes: "",
)"#,
    )
    .unwrap();
    p
}

async fn spawn_server(seed: u64) -> (String, tokio::task::JoinHandle<std::io::Result<()>>) {
    let scenario_path = write_blank_scenario();
    let inputs = ConfigInputs {
        scenario_id: "m0_blank".into(),
        scenario_path,
        run_mode: "ws-test".into(),
        run_bundle_root: std::env::temp_dir(),
        write_run_bundle: false,
        control_api_enabled: true,
        debug_capabilities: vec![],
        tick_rate_hz: 60,
        paced: false,
        settings: Settings::default(),
        seed_override: Some(seed),
        duration_ticks_override: None,
        debug_inject_panic_at_tick: None,
    };
    let config = build_engine_config(inputs).expect("build_engine_config");
    let engine = Arc::new(M0Engine::new(config));
    engine.record_run_started();
    let server = ControlServer::new(ControlServerConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        max_observe_hz: 240,
        ..Default::default()
    });
    let (listener, bound) = server.bind().await.expect("bind");
    let url = format!("ws://{}", bound.bind);
    let handle = tokio::spawn(async move { ControlServer::serve_listener(listener, engine, 240).await });
    // Yield so the listener task is registered before we connect.
    tokio::time::sleep(Duration::from_millis(50)).await;
    (url, handle)
}

async fn send_and_recv(url: &str, request: Value) -> Value {
    let (mut ws, _resp) = connect_async(url).await.expect("ws connect");
    ws.send(Message::Text(request.to_string())).await.expect("ws send");
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(3), ws.next())
            .await
            .expect("timed out waiting for response")
            .expect("ws stream closed")
            .expect("ws read error");
        if let Message::Text(text) = msg {
            let parsed: Value = serde_json::from_str(&text).expect("response is valid json");
            // Skip notifications (e.g. observe.frame); only return responses with id.
            if parsed.get("id").is_some() {
                let _ = ws.close(None).await;
                return parsed;
            }
        }
    }
}

#[tokio::test]
async fn live_ws_missing_schema_version_rejects_act_player_move() {
    let (url, handle) = spawn_server(42).await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.player.move",
            "params": {"x": 1.0, "y": 0.0}
        }),
    )
    .await;
    handle.abort();

    let error = response
        .get("error")
        .expect("missing schema_version must produce a JSON-RPC error");
    assert_eq!(error["code"], -32602);
    assert_eq!(error["data"]["reason"], "schema_version_missing");
    assert!(response.get("result").is_none(), "must NOT be accepted");
}

#[tokio::test]
async fn live_ws_missing_schema_version_rejects_runbundle_write() {
    let (url, handle) = spawn_server(42).await;
    let response = send_and_recv(
        &url,
        json!({"jsonrpc": "2.0", "id": 2, "method": "runbundle.write", "params": {}}),
    )
    .await;
    handle.abort();
    let error = response
        .get("error")
        .expect("missing schema_version must produce error");
    assert_eq!(error["code"], -32602);
    assert_eq!(error["data"]["reason"], "schema_version_missing");
}

#[tokio::test]
async fn live_ws_unknown_param_field_rejected() {
    let (url, handle) = spawn_server(42).await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 20,
            "method": "sim.step",
            "params": {"schema_version": 1, "ticks": 1, "typo_ticks": 99}
        }),
    )
    .await;
    handle.abort();
    let error = response
        .get("error")
        .expect("unknown param field must produce an InvalidParams error");
    assert_eq!(error["code"], -32602);
    assert!(
        error["data"]["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("unknown field"),
        "unknown-field rejection should expose serde's unknown field reason: {response}"
    );
}

#[tokio::test]
async fn live_ws_step_zero_rejected() {
    let (url, handle) = spawn_server(42).await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 21,
            "method": "sim.step",
            "params": {"schema_version": 1, "ticks": 0}
        }),
    )
    .await;
    handle.abort();
    let error = response
        .get("error")
        .expect("step zero must produce an InvalidParams error");
    assert_eq!(error["code"], -32602);
    assert_eq!(error["data"]["reason"], "ticks_must_be_positive");
}

#[tokio::test]
async fn live_ws_act_player_move_rejected_until_m1() {
    let (url, handle) = spawn_server(42).await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 22,
            "method": "act.player.move",
            "params": {"schema_version": 1, "x": 1.0, "y": 0.0}
        }),
    )
    .await;
    handle.abort();
    let error = response
        .get("error")
        .expect("M0 act.player.move must reject because no player actor exists yet");
    assert_eq!(error["message"], "command_rejected");
    assert_eq!(error["data"]["reason"], "act_player_move_not_available_in_m0");
}

#[tokio::test]
async fn live_ws_runbundle_id_override_rejected() {
    let (url, handle) = spawn_server(42).await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 23,
            "method": "runbundle.write",
            "params": {"schema_version": 1, "id_override": "manual"}
        }),
    )
    .await;
    handle.abort();
    let error = response
        .get("error")
        .expect("id_override must reject until M0 supports custom run ids");
    assert_eq!(error["code"], -32602);
    assert_eq!(error["data"]["reason"], "runbundle_id_override_not_supported_in_m0");
}

#[tokio::test]
async fn live_ws_scenario_load_with_mismatched_seed_rejected() {
    // M0.2-F3 over the wire: the engine seed is 42; client requests seed 7. Server must
    // return a JSON-RPC error (`command_rejected` → -32004) NOT result.status="accepted".
    let (url, handle) = spawn_server(42).await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "scenario.load",
            "params": {"schema_version": 1, "scenario": "m0_blank", "seed": 7}
        }),
    )
    .await;
    handle.abort();

    // The server returns a JSON-RPC error envelope for rejected commands:
    //   { error: { message: "command_rejected", data: { reason: "<actual>", tick: N } } }
    let error = response
        .get("error")
        .expect("rejected command must produce error envelope");
    assert_eq!(error["message"], "command_rejected");
    assert_eq!(error["data"]["reason"], "seed_override_not_supported_in_m0");
    assert!(
        response.get("result").is_none(),
        "rejected scenario.load must NOT carry a `result` field; got {response}"
    );
}

#[tokio::test]
async fn live_ws_scenario_load_with_matching_seed_accepted() {
    let (url, handle) = spawn_server(42).await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "scenario.load",
            "params": {"schema_version": 1, "scenario": "m0_blank", "seed": 42}
        }),
    )
    .await;
    handle.abort();
    let result = response.get("result").expect("matching seed should accept");
    assert_eq!(result["status"], "accepted");
}

#[tokio::test]
async fn live_ws_scenario_load_unknown_scenario_rejected() {
    let (url, handle) = spawn_server(42).await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "scenario.load",
            "params": {"schema_version": 1, "scenario": "some_other_scene"}
        }),
    )
    .await;
    handle.abort();
    let error = response
        .get("error")
        .expect("unknown scenario must produce error envelope");
    assert_eq!(error["message"], "command_rejected");
    assert_eq!(error["data"]["reason"], "scenario_swap_not_supported_in_m0");
}
