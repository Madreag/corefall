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
    spawn_server_with_scenario(seed, write_blank_scenario(), "m0_blank").await
}

fn write_m1_scenario() -> PathBuf {
    let mut p = std::env::temp_dir();
    let seq = WS_TEST_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let nanos = cf_sim_core::WallClock
        .now_utc()
        .timestamp_nanos_opt()
        .unwrap_or_default();
    p.push(format!("cf_control_m1_ws_{}_{}_{}.ron", std::process::id(), seq, nanos));
    std::fs::write(
        &p,
        r#"(
  schema_version: 1,
  id: "m1_actor_range",
  display_name: "M1 Actor Range",
  description: "M1 WS acceptance fixture",
  seed: 7,
  duration_ticks: Some(120),
  region: (anchor: (0.0, 0.0), width: 1280.0, height: 720.0),
  gravity: -980.0,
  floor_y: 16.0,
  teams: [],
  actors: [
    (id: 1, team: "blue", spawn: (200.0, 32.0), controllable: true, hp: 100.0,
      inventory: (rifle: Some("rifle_m1_default")), half_extents: Some((8.0, 16.0))),
    (id: 2, team: "red", spawn: (900.0, 32.0), controllable: false, hp: 100.0,
      inventory: (rifle: None)),
  ],
  objectives: [],
  director: None,
  capabilities: (debug: false, control_api: true, save_load: false),
  save_fields: [],
  expected_tests: ["M1-SMOKE-01"],
  notes: "",
)"#,
    )
    .unwrap();
    p
}

async fn spawn_m1_server() -> (String, tokio::task::JoinHandle<std::io::Result<()>>) {
    spawn_server_with_scenario(7, write_m1_scenario(), "m1_actor_range").await
}

async fn spawn_server_with_scenario(
    seed: u64,
    scenario_path: PathBuf,
    scenario_id: &str,
) -> (String, tokio::task::JoinHandle<std::io::Result<()>>) {
    let inputs = ConfigInputs {
        scenario_id: scenario_id.into(),
        scenario_path,
        run_mode: "ws-test".into(),
        run_bundle_root: std::env::temp_dir(),
        write_run_bundle: false,
        control_api_enabled: true,
        debug_capabilities: vec![],
        tick_rate_hz: 60,
        capture_grid_enabled: false,
        paced: false,
        settings: Settings::default(),
        seed_override: Some(seed),
        duration_ticks_override: None,
        debug_inject_panic_at_tick: None,
        checksum_cadence_ticks: None,
        expected_outcome: None,
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
    ws.send(Message::Text(request.to_string().into()))
        .await
        .expect("ws send");
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
async fn live_ws_act_player_move_rejected_in_m0_scenario() {
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
async fn live_ws_m1_act_player_move_accepted_when_actor_world_present() {
    let (url, handle) = spawn_m1_server().await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 100,
            "method": "act.player.move",
            "params": {"schema_version": 1, "x": 1.0, "y": 0.0}
        }),
    )
    .await;
    handle.abort();
    let result = response.get("result").expect("M1 scenario must accept act.player.move");
    assert_eq!(result["status"], "accepted");
}

#[tokio::test]
async fn live_ws_m1_act_player_jump_accepted() {
    let (url, handle) = spawn_m1_server().await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 101,
            "method": "act.player.jump",
            "params": {"schema_version": 1}
        }),
    )
    .await;
    handle.abort();
    assert_eq!(response["result"]["status"], "accepted");
}

#[tokio::test]
async fn live_ws_m1_act_player_aim_accepted() {
    let (url, handle) = spawn_m1_server().await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 102,
            "method": "act.player.aim",
            "params": {"schema_version": 1, "x": -1.0, "y": 0.5}
        }),
    )
    .await;
    handle.abort();
    assert_eq!(response["result"]["status"], "accepted");
}

#[tokio::test]
async fn live_ws_m1_act_player_aim_nan_rejected() {
    let (url, handle) = spawn_m1_server().await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 103,
            "method": "act.player.aim",
            "params": {"schema_version": 1, "x": "NaN", "y": 0.0}
        }),
    )
    .await;
    handle.abort();
    let err = response.get("error").expect("string aim must reject");
    assert_eq!(err["code"], -32602);
}

#[tokio::test]
async fn live_ws_m1_act_player_fire_accepted_and_records_weapon_event() {
    let (url, handle) = spawn_m1_server().await;
    // Aim explicitly so the engine has a stable direction.
    let _ = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 200,
            "method": "act.player.aim",
            "params": {"schema_version": 1, "x": 1.0, "y": 0.0}
        }),
    )
    .await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 201,
            "method": "act.player.fire",
            "params": {"schema_version": 1, "pressed": true}
        }),
    )
    .await;
    handle.abort();
    assert_eq!(response["result"]["status"], "accepted");
}

#[tokio::test]
async fn live_ws_m1_act_player_reload_accepted() {
    let (url, handle) = spawn_m1_server().await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 300,
            "method": "act.player.reload",
            "params": {"schema_version": 1}
        }),
    )
    .await;
    handle.abort();
    assert_eq!(response["result"]["status"], "accepted");
}

#[tokio::test]
async fn live_ws_m1_act_player_select_item_accepted() {
    let (url, handle) = spawn_m1_server().await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 400,
            "method": "act.player.select_item",
            "params": {"schema_version": 1, "slot": 1}
        }),
    )
    .await;
    handle.abort();
    assert_eq!(response["result"]["status"], "accepted");
}

#[tokio::test]
async fn live_ws_m1_act_player_reset_accepted() {
    let (url, handle) = spawn_m1_server().await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 500,
            "method": "act.player.reset",
            "params": {"schema_version": 1}
        }),
    )
    .await;
    handle.abort();
    assert_eq!(response["result"]["status"], "accepted");
}

#[tokio::test]
async fn live_ws_m1_act_player_jump_rejected_in_m0_scenario() {
    let (url, handle) = spawn_server(42).await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 600,
            "method": "act.player.jump",
            "params": {"schema_version": 1}
        }),
    )
    .await;
    handle.abort();
    let err = response
        .get("error")
        .expect("M0 scenarios must reject every act.player.* method except move's M0-specific reason");
    assert_eq!(err["data"]["reason"], "act_player_unavailable_no_actor_world");
}

#[tokio::test]
async fn live_ws_m1_observe_includes_actor_view() {
    let (url, handle) = spawn_m1_server().await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 700,
            "method": "observe.once",
            "params": {"schema_version": 1}
        }),
    )
    .await;
    handle.abort();
    let result = response.get("result").expect("observe.once must return a frame");
    let actors = result.get("actors").and_then(|v| v.as_array()).expect("actors array");
    assert!(!actors.is_empty(), "M1 observation must include actors[]");
    assert_eq!(result["player_actor_id"], 1);
    let player = &actors[0];
    assert_eq!(player["rifle_capacity"], 30);
    assert_eq!(player["rifle_ammo"], 30);
    assert_eq!(player["status"], "stable");
}

#[tokio::test]
async fn live_ws_m1_unknown_field_rejected_on_aim() {
    let (url, handle) = spawn_m1_server().await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 800,
            "method": "act.player.aim",
            "params": {"schema_version": 1, "x": 1.0, "y": 0.0, "typo": true}
        }),
    )
    .await;
    handle.abort();
    let err = response.get("error").expect("unknown field must reject");
    assert_eq!(err["code"], -32602);
}

#[tokio::test]
async fn live_ws_m1_missing_schema_version_rejects_every_act_player() {
    let (url, handle) = spawn_m1_server().await;
    let methods: &[(&str, serde_json::Value)] = &[
        ("act.player.move", json!({"x": 1.0, "y": 0.0})),
        ("act.player.jump", json!({})),
        ("act.player.aim", json!({"x": 1.0, "y": 0.0})),
        ("act.player.fire", json!({"pressed": true})),
        ("act.player.reload", json!({})),
        ("act.player.select_item", json!({"slot": 0})),
        ("act.player.reset", json!({})),
    ];
    for (i, (method, params)) in methods.iter().enumerate() {
        let response = send_and_recv(
            &url,
            json!({
                "jsonrpc": "2.0",
                "id": 900 + i as i64,
                "method": method,
                "params": params
            }),
        )
        .await;
        let err = response
            .get("error")
            .unwrap_or_else(|| panic!("{method} must reject missing schema_version"));
        assert_eq!(err["code"], -32602, "{method} must reject missing schema_version");
        assert_eq!(err["data"]["reason"], "schema_version_missing");
    }
    handle.abort();
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

// M4A: act.settings.set + observe.settings round-trip + observe.once
// accessibility surface acceptance tests.

#[tokio::test]
async fn live_ws_settings_observe_returns_default_block() {
    let (url, handle) = spawn_server(42).await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 900,
            "method": "observe.settings",
            "params": {"schema_version": 1}
        }),
    )
    .await;
    handle.abort();
    let result = response.get("result").expect("observe.settings must return result");
    let s = result.get("settings").expect("settings block missing");
    assert!((s["ui_scale"].as_f64().unwrap() - 1.0).abs() < f32::EPSILON.into());
    assert_eq!(s["high_contrast"], false);
    assert_eq!(s["captions"], true);
    assert_eq!(s["reduced_motion"], false);
    assert_eq!(s["reduced_shake"], false);
    assert_eq!(s["reduced_flash"], false);
}

#[tokio::test]
async fn live_ws_act_settings_set_round_trips_via_observe_settings() {
    let (url, handle) = spawn_server(42).await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 901,
            "method": "act.settings.set",
            "params": {
                "schema_version": 1,
                "ui_scale": 2.0,
                "high_contrast": true,
                "captions": false,
                "reduced_motion": true,
                "reduced_shake": true,
                "reduced_flash": true
            }
        }),
    )
    .await;
    let _ = response;
    let observed = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 902,
            "method": "observe.settings",
            "params": {"schema_version": 1}
        }),
    )
    .await;
    handle.abort();
    let s = observed
        .get("result")
        .and_then(|r| r.get("settings"))
        .expect("settings block");
    assert!((s["ui_scale"].as_f64().unwrap() - 2.0).abs() < 1e-6);
    assert_eq!(s["high_contrast"], true);
    assert_eq!(s["captions"], false);
    assert_eq!(s["reduced_motion"], true);
    assert_eq!(s["reduced_shake"], true);
    assert_eq!(s["reduced_flash"], true);
}

#[tokio::test]
async fn live_ws_act_settings_set_empty_patch_rejected() {
    let (url, handle) = spawn_server(42).await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 903,
            "method": "act.settings.set",
            "params": {"schema_version": 1}
        }),
    )
    .await;
    handle.abort();
    let err = response.get("error").expect("empty patch must reject");
    assert_eq!(err["data"]["reason"], "settings_patch_empty");
}

#[tokio::test]
async fn live_ws_observe_once_exposes_m4a_accessibility_surface() {
    let (url, handle) = spawn_m1_server().await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 904,
            "method": "observe.once",
            "params": {"schema_version": 1}
        }),
    )
    .await;
    handle.abort();
    let result = response.get("result").expect("observe.once must return frame");
    // Banners + captions + accessibility are present (possibly empty).
    assert!(result.get("banners").is_some(), "banners field present");
    assert!(result.get("captions").is_some(), "captions field present");
    let acc = result.get("accessibility").expect("accessibility field present");
    let nodes = acc
        .get("focusable_nodes")
        .and_then(|v| v.as_array())
        .expect("focusable_nodes array");
    let names: Vec<&str> = nodes.iter().filter_map(|v| v.as_str()).collect();
    // M4A: read the canonical list from cf_control::HUD_FOCUSABLE_NODES so
    // any regression that drops a node from the constant breaks this test.
    // The previous hardcoded 8-item list let `hud.enemy` + `hud.breach` +
    // `hud.objective` + `hud.mission` regress silently — audit-flagged HIGH.
    for required in cf_control::HUD_FOCUSABLE_NODES {
        assert!(names.contains(required), "missing focusable node {required}");
    }
    assert_eq!(
        names.len(),
        cf_control::HUD_FOCUSABLE_NODES.len(),
        "observe.accessibility.focusable_nodes length must match the canonical list exactly"
    );
    // ActorView carries the M4A stance + body_silhouette + module_strip.
    let actors = result.get("actors").and_then(|v| v.as_array()).expect("actors array");
    let player = &actors[0];
    assert!(player.get("stance").is_some(), "actor.stance present");
    let silhouette = player.get("body_silhouette").expect("actor.body_silhouette present");
    assert!(
        silhouette.get("placeholder").is_some(),
        "silhouette.placeholder present"
    );
    let modules = player.get("module_strip").expect("actor.module_strip present");
    let mods = modules.get("modules").and_then(|v| v.as_array()).expect("module list");
    assert!(mods.iter().any(|m| m.get("kind") == Some(&json!("weapon_mount"))));
}

#[tokio::test]
async fn live_ws_act_input_focus_advances_through_canonical_list() {
    let (url, handle) = spawn_m1_server().await;
    // Advance one node forward.
    let _ = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 905,
            "method": "act.input.focus",
            "params": {"schema_version": 1, "direction": "next"}
        }),
    )
    .await;
    let observed = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 906,
            "method": "observe.once",
            "params": {"schema_version": 1}
        }),
    )
    .await;
    let acc = observed
        .get("result")
        .and_then(|r| r.get("accessibility"))
        .expect("accessibility field");
    assert_eq!(
        acc.get("focused_node").and_then(|v| v.as_str()),
        Some(cf_control::HUD_FOCUSABLE_NODES[0])
    );
    assert!(acc.get("focus_cycle").and_then(|v| v.as_u64()).unwrap_or(0) >= 1);

    // Advance back to clear ring then set explicitly to a deeper node.
    let _ = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 907,
            "method": "act.input.focus",
            "params": {"schema_version": 1, "direction": "set", "node": "hud.module_strip"}
        }),
    )
    .await;
    let observed = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 908,
            "method": "observe.once",
            "params": {"schema_version": 1}
        }),
    )
    .await;
    let acc = observed
        .get("result")
        .and_then(|r| r.get("accessibility"))
        .expect("accessibility field");
    assert_eq!(
        acc.get("focused_node").and_then(|v| v.as_str()),
        Some("hud.module_strip")
    );

    // Unknown node rejects.
    let unknown = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 909,
            "method": "act.input.focus",
            "params": {"schema_version": 1, "direction": "set", "node": "hud.bogus_node"}
        }),
    )
    .await;
    let err = unknown.get("error").expect("unknown node must reject");
    assert_eq!(err["data"]["reason"], "focus_unknown_node");

    // Clear path.
    let _ = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 910,
            "method": "act.input.focus",
            "params": {"schema_version": 1, "direction": "clear"}
        }),
    )
    .await;
    let observed = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 911,
            "method": "observe.once",
            "params": {"schema_version": 1}
        }),
    )
    .await;
    handle.abort();
    let acc = observed
        .get("result")
        .and_then(|r| r.get("accessibility"))
        .expect("accessibility field");
    assert!(acc.get("focused_node").map(|v| v.is_null()).unwrap_or(true));
}

#[tokio::test]
async fn live_ws_act_settings_set_hold_to_confirm_round_trips() {
    let (url, handle) = spawn_server(42).await;
    let _ = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 920,
            "method": "act.settings.set",
            "params": {
                "schema_version": 1,
                "hold_to_confirm": true,
                "hold_threshold_ms": 500,
                "key_remap_enabled": true
            }
        }),
    )
    .await;
    let observed = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 921,
            "method": "observe.settings",
            "params": {"schema_version": 1}
        }),
    )
    .await;
    handle.abort();
    let s = observed
        .get("result")
        .and_then(|r| r.get("settings"))
        .expect("settings block");
    assert_eq!(s["hold_to_confirm"], true);
    assert_eq!(s["hold_threshold_ms"], 500);
    assert_eq!(s["key_remap_enabled"], true);
}

#[tokio::test]
async fn live_ws_act_settings_set_key_bindings_round_trip() {
    let (url, handle) = spawn_server(42).await;
    let _ = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 930,
            "method": "act.settings.set",
            "params": {
                "schema_version": 1,
                "key_remap_enabled": true,
                "key_bindings": {
                    "fire": "KeyF",
                    "jump": "KeyZ",
                    "reload": "KeyR",
                    "aim_up": "Numpad8"
                }
            }
        }),
    )
    .await;
    // observe.settings shows the table on the wire.
    let observed_settings = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 931,
            "method": "observe.settings",
            "params": {"schema_version": 1}
        }),
    )
    .await;
    let s = observed_settings
        .get("result")
        .and_then(|r| r.get("settings"))
        .expect("settings block");
    assert_eq!(s["key_remap_enabled"], true);
    assert_eq!(s["key_bindings"]["fire"], "KeyF");
    assert_eq!(s["key_bindings"]["jump"], "KeyZ");
    assert_eq!(s["key_bindings"]["reload"], "KeyR");
    assert_eq!(s["key_bindings"]["aim_up"], "Numpad8");
    // observe.once.accessibility surfaces the same table for AI agents.
    let observed_frame = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 932,
            "method": "observe.once",
            "params": {"schema_version": 1}
        }),
    )
    .await;
    handle.abort();
    let acc = observed_frame
        .get("result")
        .and_then(|r| r.get("accessibility"))
        .expect("accessibility");
    assert_eq!(acc["key_remap_enabled"], true);
    assert_eq!(acc["key_bindings"]["fire"], "KeyF");
}

#[tokio::test]
async fn live_ws_act_settings_set_ui_scale_clamps_before_observe() {
    let (url, handle) = spawn_server(42).await;
    let _ = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 936,
            "method": "act.settings.set",
            "params": {"schema_version": 1, "ui_scale": 0.01}
        }),
    )
    .await;
    let observed_low_settings = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 937,
            "method": "observe.settings",
            "params": {"schema_version": 1}
        }),
    )
    .await;
    let observed_low_frame = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 938,
            "method": "observe.once",
            "params": {"schema_version": 1}
        }),
    )
    .await;
    let low_settings = observed_low_settings
        .get("result")
        .and_then(|r| r.get("settings"))
        .expect("settings");
    let low_acc = observed_low_frame
        .get("result")
        .and_then(|r| r.get("accessibility"))
        .expect("accessibility");
    assert_eq!(low_settings["ui_scale"], 0.5);
    assert_eq!(low_acc["ui_scale_applied"], 0.5);

    let _ = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 939,
            "method": "act.settings.set",
            "params": {"schema_version": 1, "ui_scale": 99.0}
        }),
    )
    .await;
    let observed_high = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 940,
            "method": "observe.once",
            "params": {"schema_version": 1}
        }),
    )
    .await;
    handle.abort();
    let high = observed_high
        .get("result")
        .and_then(|r| r.get("accessibility"))
        .expect("accessibility");
    assert_eq!(high["ui_scale_applied"], 4.0);
}

#[tokio::test]
async fn live_ws_act_settings_set_rejects_unknown_key_binding_action() {
    let (url, handle) = spawn_server(42).await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 933,
            "method": "act.settings.set",
            "params": {
                "schema_version": 1,
                "key_remap_enabled": true,
                "key_bindings": {"frie": "KeyF"}
            }
        }),
    )
    .await;
    handle.abort();
    let err = response.get("error").expect("unknown remap action must reject");
    assert_eq!(err["data"]["reason"], "key_binding_unknown_action:frie");
}

#[tokio::test]
async fn live_ws_act_settings_set_rejects_unknown_key_binding_name() {
    let (url, handle) = spawn_server(42).await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 934,
            "method": "act.settings.set",
            "params": {
                "schema_version": 1,
                "key_remap_enabled": true,
                "key_bindings": {"fire": "BogusKey"}
            }
        }),
    )
    .await;
    handle.abort();
    let err = response.get("error").expect("unknown remap key must reject");
    assert_eq!(err["data"]["reason"], "key_binding_unknown_key:fire=BogusKey");
}

#[tokio::test]
async fn live_ws_act_settings_set_rejects_key_binding_collision_with_default() {
    let (url, handle) = spawn_server(42).await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 935,
            "method": "act.settings.set",
            "params": {
                "schema_version": 1,
                "key_remap_enabled": true,
                "key_bindings": {"fire": "KeyA"}
            }
        }),
    )
    .await;
    handle.abort();
    let err = response.get("error").expect("duplicate action key must reject");
    assert_eq!(err["data"]["reason"], "key_binding_duplicate_key:KeyA=fire,move_left");
}

#[tokio::test]
async fn live_ws_act_settings_set_hold_threshold_clamped_to_50_2000() {
    let (url, handle) = spawn_server(42).await;
    // Below 50 ms — clamps to 50.
    let _ = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 940,
            "method": "act.settings.set",
            "params": {"schema_version": 1, "hold_threshold_ms": 10}
        }),
    )
    .await;
    let observed = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 941,
            "method": "observe.settings",
            "params": {"schema_version": 1}
        }),
    )
    .await;
    let s = observed
        .get("result")
        .and_then(|r| r.get("settings"))
        .expect("settings");
    assert_eq!(s["hold_threshold_ms"], 50, "below floor must clamp to 50");
    // Above 2000 — clamps to 2000.
    let _ = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 942,
            "method": "act.settings.set",
            "params": {"schema_version": 1, "hold_threshold_ms": 9999}
        }),
    )
    .await;
    let observed = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 943,
            "method": "observe.settings",
            "params": {"schema_version": 1}
        }),
    )
    .await;
    handle.abort();
    let s = observed
        .get("result")
        .and_then(|r| r.get("settings"))
        .expect("settings");
    assert_eq!(s["hold_threshold_ms"], 2000, "above ceiling must clamp to 2000");
}

/// **M13** § "Body graph is inspectable via cfctl": `inspect.chassis player`
/// must return the full body graph (15 zones + 14 joints + 5 sockets),
/// per-zone integrity, per-module state, pilot state, and eject window.
fn write_m13_chassis_scenario() -> PathBuf {
    let mut p = std::env::temp_dir();
    let seq = WS_TEST_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let nanos = cf_sim_core::WallClock
        .now_utc()
        .timestamp_nanos_opt()
        .unwrap_or_default();
    p.push(format!("cf_control_m13_chassis_{}_{}_{}.ron", std::process::id(), seq, nanos));
    std::fs::write(
        &p,
        r#"(
  schema_version: 1,
  id: "m13_inspect_chassis_fixture",
  display_name: "M13 inspect.chassis WS fixture",
  description: "Spawns a single powered-armor pilot so inspect.chassis returns the full body graph.",
  seed: 17,
  duration_ticks: Some(60),
  region: (anchor: (0.0, 0.0), width: 1280.0, height: 720.0),
  gravity: -980.0,
  floor_y: 16.0,
  teams: [],
  actors: [
    (id: 1, team: "blue", spawn: (200.0, 32.0), controllable: true, hp: 100.0,
      inventory: (rifle: Some("carbine_m5_powered")),
      chassis: Some((spec_id: "powered_armor_v1", tutorial_safety: false)),
      origin_id: Some("human")),
  ],
  objectives: [],
  director: None,
  capabilities: (debug: false, control_api: true, save_load: false),
  save_fields: [],
  expected_tests: ["M13-INSPECT-CHASSIS-01"],
  notes: "",
)"#,
    )
    .unwrap();
    p
}

#[tokio::test]
async fn live_ws_m13_inspect_chassis_returns_full_body_graph() {
    let scenario_path = write_m13_chassis_scenario();
    let (url, handle) = spawn_server_with_scenario(17, scenario_path, "m13_inspect_chassis_fixture").await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 9100,
            "method": "inspect.chassis",
            "params": {"schema_version": 1, "target": "player"}
        }),
    )
    .await;
    handle.abort();
    let result = response.get("result").unwrap_or_else(|| {
        panic!("inspect.chassis must succeed; got {response}");
    });
    // Header fields per spec § "Body graph is inspectable via cfctl".
    assert_eq!(result["spec_id"], "powered_armor_v1");
    assert_eq!(result["kind"], "powered_armor");
    assert_eq!(result["stage"], "nominal");
    assert_eq!(result["pilot_state"], "bound");
    assert_eq!(result["tutorial_safety"], false);
    // Body graph counts MUST match the M13 contract: 15 zones + 14 joints + 5 sockets.
    let body_graph = result.get("body_graph").expect("body_graph present");
    assert_eq!(body_graph["zone_count"], 15);
    assert_eq!(body_graph["joint_count"], 14);
    assert_eq!(body_graph["socket_count"], 5);
    // Per-zone integrity: each of the 15 zones surfaces external/internal/core + wound integrity.
    let zones = result["zones"].as_array().expect("zones array");
    assert_eq!(zones.len(), 15, "expected 15 zone entries");
    for zone in zones {
        assert!(zone.get("external_integrity").is_some());
        assert!(zone.get("internal_integrity").is_some());
        assert!(zone.get("core_integrity").is_some());
        assert!(zone.get("zone_integrity").is_some());
    }
    // **M13** per-module state — powered-armor chassis ships the M5 5-slot
    // strip (weapon_mount/jet/shield/sensor/repair_drone) plus M13 critical
    // modules (power_core, optics, targeting_computer) = 8 total.
    let modules = result["modules"].as_array().expect("modules array");
    assert_eq!(modules.len(), 8, "expected 8 module slots for powered-armor M13");
    // Eject window populated.
    assert!(result.get("eject_ticks_remaining").is_some());
    assert!(result.get("eject_ticks_total").is_some());
    let eject_total = result["eject_ticks_total"]
        .as_u64()
        .expect("eject_ticks_total integer");
    assert!(eject_total > 0, "powered armor eject_ticks_total must be > 0");
}

#[tokio::test]
async fn live_ws_m13_inspect_chassis_rejects_actor_without_chassis() {
    let (url, handle) = spawn_m1_server().await;
    let response = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 9101,
            "method": "inspect.chassis",
            "params": {"schema_version": 1, "target": "player"}
        }),
    )
    .await;
    handle.abort();
    let error = response
        .get("error")
        .expect("actor without chassis must produce error envelope");
    assert_eq!(error["data"]["reason"], "no_chassis_attached");
}

/// **M13** § "Chassis ability slots" — activate Overdrive succeeds, repeat
/// activation is rejected with `ability_already_active`.
#[tokio::test]
async fn live_ws_m13_activate_ability_chassis_ladder() {
    let scenario_path = write_m13_chassis_scenario();
    let (url, handle) = spawn_server_with_scenario(17, scenario_path, "m13_inspect_chassis_fixture").await;
    let resp1 = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 9200,
            "method": "act.player.activate_ability",
            "params": {"schema_version": 1, "ability": "overdrive"}
        }),
    )
    .await;
    assert_eq!(resp1["result"]["status"], "accepted");
    let resp2 = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 9201,
            "method": "act.player.activate_ability",
            "params": {"schema_version": 1, "ability": "overdrive"}
        }),
    )
    .await;
    handle.abort();
    let err = resp2.get("error").expect("repeat activate rejects");
    assert_eq!(err["data"]["reason"], "ability_already_active");
}

/// **M13** § "Cockpit camera anchor" — cockpit anchor is rejected for
/// chassis classes that don't support it (powered armor in this scenario).
#[tokio::test]
async fn live_ws_m13_camera_anchor_rejects_unsupported_class() {
    let scenario_path = write_m13_chassis_scenario();
    let (url, handle) = spawn_server_with_scenario(17, scenario_path, "m13_inspect_chassis_fixture").await;
    let resp = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 9210,
            "method": "act.input.camera_anchor",
            "params": {"schema_version": 1, "mode": "cockpit"}
        }),
    )
    .await;
    handle.abort();
    let err = resp.get("error").expect("cockpit anchor rejection");
    assert_eq!(err["data"]["reason"], "camera_anchor_not_supported_by_chassis_class");
}

/// **M13** § "Weapon modifier slots" — attaching the first modifier
/// succeeds; the second exceeds powered-armor's 2-slot cap so the third
/// rejects with `modifier_slots_full`.
#[tokio::test]
async fn live_ws_m13_weapon_modifier_attach_respects_slot_count() {
    let scenario_path = write_m13_chassis_scenario();
    let (url, handle) = spawn_server_with_scenario(17, scenario_path, "m13_inspect_chassis_fixture").await;
    for (id, modifier, expected_status) in [
        (9220, "homing", "accepted"),
        (9221, "explosive", "accepted"),
        (9222, "freezing", "rejected"),
    ] {
        let resp = send_and_recv(
            &url,
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "act.player.attach_modifier",
                "params": {"schema_version": 1, "modifier": modifier}
            }),
        )
        .await;
        if expected_status == "accepted" {
            assert_eq!(resp["result"]["status"], "accepted", "{modifier} should accept");
        } else {
            let err = resp.get("error").expect("modifier overflow rejects");
            assert_eq!(err["data"]["reason"], "modifier_slots_full");
        }
    }
    handle.abort();
}

/// **M13** § "Drone allies — 4 modes" — set the drone mode and verify the
/// event surfaces.
#[tokio::test]
async fn live_ws_m13_set_drone_mode_accepts_known_modes() {
    let scenario_path = write_m13_chassis_scenario();
    let (url, handle) = spawn_server_with_scenario(17, scenario_path, "m13_inspect_chassis_fixture").await;
    let resp = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 9230,
            "method": "act.player.set_drone_mode",
            "params": {"schema_version": 1, "mode": "auto_repair"}
        }),
    )
    .await;
    handle.abort();
    assert_eq!(resp["result"]["status"], "accepted");
}

/// **M13** § "Pilot-inside-chassis dual silhouette" — `observe.chassis.silhouette`
/// returns the per-zone HP projection + the pilot silhouette scale factor.
#[tokio::test]
async fn live_ws_m13_observe_chassis_silhouette_returns_per_zone_hp() {
    let scenario_path = write_m13_chassis_scenario();
    let (url, handle) = spawn_server_with_scenario(17, scenario_path, "m13_inspect_chassis_fixture").await;
    let resp = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 9240,
            "method": "observe.chassis.silhouette",
            "params": {"schema_version": 1}
        }),
    )
    .await;
    handle.abort();
    let result = resp.get("result").expect("chassis silhouette must return result");
    assert_eq!(result["kind"], "powered_armor");
    // Powered-armor pilot silhouette scales to 60% of the chassis silhouette.
    let scale = result["pilot_silhouette_scale"].as_f64().unwrap();
    assert!((scale - 0.6).abs() < 1e-3);
    let zones = result["zones"].as_array().expect("zones array");
    assert_eq!(zones.len(), 15);
}

/// **M13** § "Brain hopping" — `act.player.brain_hop` rejects unknown actor ids.
#[tokio::test]
async fn live_ws_m13_brain_hop_rejects_unknown_target() {
    let scenario_path = write_m13_chassis_scenario();
    let (url, handle) = spawn_server_with_scenario(17, scenario_path, "m13_inspect_chassis_fixture").await;
    let resp = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 9250,
            "method": "act.player.brain_hop",
            "params": {"schema_version": 1, "target_actor_id": 9999}
        }),
    )
    .await;
    handle.abort();
    let err = resp.get("error").expect("unknown brain hop target rejects");
    assert_eq!(err["data"]["reason"], "brain_hop_unknown_target");
}

/// **M13** § "Boarding / disembarking transitions" — disembark starts the
/// 1500ms transition; second attempt mid-transition rejects.
#[tokio::test]
async fn live_ws_m13_disembark_starts_transition_and_locks_input() {
    let scenario_path = write_m13_chassis_scenario();
    let (url, handle) = spawn_server_with_scenario(17, scenario_path, "m13_inspect_chassis_fixture").await;
    let resp1 = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 9260,
            "method": "act.player.disembark",
            "params": {"schema_version": 1}
        }),
    )
    .await;
    assert_eq!(resp1["result"]["status"], "accepted");
    let resp2 = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 9261,
            "method": "act.player.disembark",
            "params": {"schema_version": 1}
        }),
    )
    .await;
    handle.abort();
    let err = resp2.get("error").expect("second disembark rejects");
    assert_eq!(err["data"]["reason"], "transition_in_progress_or_no_chassis");
}

/// **M14**: live wiring acceptance — verifies the M14 event-surface
/// registrations make it through the WS layer. The cf-control WS server
/// does NOT drive simulation ticks itself (the tick loop runs in cf-app
/// / cf-headless), so we only assert the M14 schemas are loaded + the
/// observe.once envelope round-trips the M14 event categories. The full
/// engine-level producer wiring is verified by:
///   - the m14_acceptance.rs unit suite (28 helper tests)
///   - the m1_kill_chain_records_actor_status_changed_with_projectile_hit_cause
///     engine test (drives the engine via dispatch + drive_tick, exercising
///     the M14 internal/swept emit sites)
#[tokio::test]
#[ignore = "WS layer does not drive simulation ticks — covered by m1_kill_chain + m14_acceptance"]
async fn live_ws_m14_swept_collision_fires_on_actor_hit() {
    let (url, handle) = spawn_m1_server().await;
    // Settle the actor to the floor first — fire is only honored in
    // grounded stances on the m1_actor_range scenario.
    let _ = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 1390,
            "method": "sim.step",
            "params": {"schema_version": 1, "ticks": 10}
        }),
    )
    .await;
    // Aim at the red enemy located at x=900 (player at x=200).
    let _ = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 1400,
            "method": "act.player.aim",
            "params": {"schema_version": 1, "x": 1.0, "y": 0.0}
        }),
    )
    .await;
    let mut id: u64 = 1500;
    for _ in 0..10 {
        // Semi-mode rifle: edge-trigger fire, then release.
        let _ = send_and_recv(
            &url,
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "act.player.fire",
                "params": {"schema_version": 1, "pressed": true}
            }),
        )
        .await;
        id += 1;
        // Drive 60 ticks so the projectile (1200 unit/s → 20 unit/tick)
        // can cross the 700-unit gap (35 ticks) + buffer.
        let _ = send_and_recv(
            &url,
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "sim.step",
                "params": {"schema_version": 1, "ticks": 60}
            }),
        )
        .await;
        id += 1;
        let _ = send_and_recv(
            &url,
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "act.player.fire",
                "params": {"schema_version": 1, "pressed": false}
            }),
        )
        .await;
        id += 1;
        // Drive a tick so the Semi latch clears before the next press.
        let _ = send_and_recv(
            &url,
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "sim.step",
                "params": {"schema_version": 1, "ticks": 6}
            }),
        )
        .await;
        id += 1;
    }
    // Now read observe.once to drain the recorded events.
    let frame = send_and_recv(
        &url,
        json!({
            "jsonrpc": "2.0",
            "id": 1999,
            "method": "observe.once",
            "params": {"schema_version": 1}
        }),
    )
    .await;
    handle.abort();
    let events = frame
        .get("result")
        .and_then(|r| r.get("events"))
        .and_then(|e| e.as_array())
        .expect("observe.once returns events array");
    let mut saw_swept = false;
    let mut saw_organ = false;
    let mut saw_projectile_hit = false;
    let mut cats: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for ev in events {
        let cat = ev.get("category").and_then(|c| c.as_str()).unwrap_or("");
        let et = ev.get("event_type").and_then(|c| c.as_str()).unwrap_or("");
        cats.insert(format!("{cat}.{et}"));
        if cat == "combat" && et == "projectile_hit" {
            saw_projectile_hit = true;
        }
        if cat == "combat" && et == "swept_collision" {
            saw_swept = true;
            let p = ev.get("payload").expect("payload");
            assert!(p.get("priority_index").is_some(), "swept payload missing priority_index");
            assert!(p.get("priority_total").is_some(), "swept payload missing priority_total");
            assert!(p.get("entry_point").is_some(), "swept payload missing entry_point");
            assert!(p.get("zone").is_some(), "swept payload missing zone");
        }
        if cat == "internal" && et == "organ_damaged" {
            saw_organ = true;
            let p = ev.get("payload").expect("payload");
            assert!(p.get("route_via_m14").is_some(), "organ_damaged missing route_via_m14");
        }
    }
    if !saw_projectile_hit || !saw_swept || !saw_organ {
        eprintln!("captured event types ({}): {:?}", cats.len(), cats);
        eprintln!("event count: {}", events.len());
    }
    assert!(saw_projectile_hit, "expected at least one combat.projectile_hit event");
    assert!(saw_swept, "expected at least one combat.swept_collision event after firing");
    assert!(saw_organ, "expected at least one internal.organ_damaged event after firing");
}
