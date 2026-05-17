//! M10B frame ticker integration tests.
//!
//! VAL-M10B-NO-LIVE-SIM `frame_ticker_no_live_sim` test:
//!
//! > The frame ticker module (`cf-replay-export::frame_ticker`)
//! > reconstructs per-tick state from the bundle's M4B baseline +
//! > delta chain and never instantiates a live `cf-sim-core`
//! > Simulation. PASS = ticker source contains no
//! > `Simulation::new` / live-sim spinup AND the reconstruction
//! > path is exercised by a test; FAIL = live sim instantiated OR
//! > no test coverage of the reconstruction path.
//!
//! This integration test asserts the M4B reconstruction path is
//! actually exercised (the unit test in `frame_ticker.rs` covers the
//! init-counter contract; here we additionally walk the full
//! delta-chain code path end-to-end).

use cf_replay::Event;
use cf_replay_export::{BundleSource, FrameTicker, FrameTickerConfig};

fn snapshot_event(tick: u64, event_id: &str, event_type: &str, payload: serde_json::Value) -> Event {
    Event {
        schema_version: cf_replay::EVENT_SCHEMA_VERSION.to_string(),
        run_id: "frame_ticker_test".into(),
        tick,
        sim_time_ms: tick as f64 * 16.6,
        event_id: event_id.into(),
        category: "snapshot".into(),
        event_type: event_type.into(),
        payload,
        parent_event_id: None,
        actor_id: None,
        source_id: None,
        team: None,
        pos: None,
        bbox: None,
        dropped_count: None,
        cosmetic: None,
        asset_ref: None,
        prev_event_hash: None,
        chained_hash_hex: None,
    }
}

#[test]
fn frame_ticker_no_live_sim() {
    let events = vec![
        snapshot_event(
            0,
            "b0",
            "baseline_emitted",
            serde_json::json!({"state": {"hp": 100, "ammo": 30}}),
        ),
        snapshot_event(
            1,
            "d1",
            "delta_emitted",
            serde_json::json!({
                "baseline_event_id": "b0",
                "ops": [{"op": "set", "path": ["hp"], "value": 90}]
            }),
        ),
        snapshot_event(
            2,
            "d2",
            "delta_emitted",
            serde_json::json!({
                "baseline_event_id": "b0",
                "ops": [{"op": "set", "path": ["ammo"], "value": 28}]
            }),
        ),
    ];

    let ticker = FrameTicker::new(FrameTickerConfig {
        fps: 60,
        tick_rate_hz: 60,
        start_tick: 0,
        end_tick: 3,
    })
    .expect("ticker config valid");
    let frames = ticker
        .run(BundleSource::Events(&events), None)
        .expect("frame walk succeeds without live sim");
    assert_eq!(frames.len(), 3);
    assert_eq!(frames[0].source_tick, 0);
    assert_eq!(frames[2].source_tick, 2);
    assert_eq!(frames[2].snapshot.get("hp").and_then(|v| v.as_i64()), Some(90));
    assert_eq!(frames[2].snapshot.get("ammo").and_then(|v| v.as_i64()), Some(28));
}
