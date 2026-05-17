//! M10B chapter-marker derivation integration tests.
//!
//! Verification command: `cargo test -p cf-replay-export chapter_markers`
//! (expect: 26-chapter fixture PASS; frame_accuracy PASS).
//!
//! VAL-M10B-022 — 30-min fixture: 12 objectives + 3 reactor armor +
//! 7 deaths + 4 breaches = 26 chapter markers.
//! VAL-M10B-023 — chapter timecodes frame-accurate to ≤ 1 frame at 60 fps.

use cf_replay::Event;
use cf_replay_export::chapter_derivation::{counts_by_event_type, ChapterDerivation};
use cf_replay_export::chapter_markers::ChapterRuleSet;
use std::path::PathBuf;

fn default_rules() -> ChapterRuleSet {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join("content/replay_export/chapter_rules/default.ron");
    ChapterRuleSet::load(&path).expect("default chapter rules must load")
}

fn event(tick: u64, idx: usize, event_type: &str, payload: serde_json::Value, actor_id: Option<u64>) -> Event {
    Event {
        schema_version: cf_replay::EVENT_SCHEMA_VERSION.to_string(),
        run_id: "chapter_markers_test".into(),
        tick,
        sim_time_ms: tick as f64 / 60.0 * 1000.0,
        event_id: format!("{event_type}_{idx}"),
        category: event_type.split('.').next().unwrap_or(event_type).into(),
        event_type: event_type.into(),
        payload,
        parent_event_id: None,
        actor_id,
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
fn chapter_markers_26_fixture_produces_exactly_26() {
    let rules = default_rules();
    let derivation = ChapterDerivation {
        rules: &rules,
        tick_rate_hz: 60,
    };
    let mut events: Vec<Event> = Vec::new();
    // 12 objective transitions (4 started + 4 completed + 4 failed).
    for i in 0..4u64 {
        events.push(event(
            1_000 + i * 2_000,
            i as usize,
            "mission.objective_started",
            serde_json::json!({"objective_name": format!("obj_{i}")}),
            None,
        ));
        events.push(event(
            1_500 + i * 2_000,
            i as usize,
            "mission.objective_completed",
            serde_json::json!({"objective_name": format!("obj_{i}")}),
            None,
        ));
        events.push(event(
            1_700 + i * 2_000,
            i as usize,
            "mission.objective_failed",
            serde_json::json!({"objective_name": format!("obj_{i}")}),
            None,
        ));
    }
    // 3 reactor armor_layer_destroyed.
    for i in 0..3u64 {
        events.push(event(
            20_000 + i * 1_000,
            i as usize,
            "reactor.armor_layer_destroyed",
            serde_json::json!({}),
            None,
        ));
    }
    // 7 actor deaths.
    for i in 0..7u64 {
        events.push(event(
            40_000 + i * 1_500,
            i as usize,
            "actor_status_changed",
            serde_json::json!({"status": "killed", "actor_name": "player"}),
            Some(100 + i),
        ));
    }
    // 4 atmos breaches.
    for i in 0..4u64 {
        events.push(event(
            80_000 + i * 1_000,
            i as usize,
            "atmos.breach_detected",
            serde_json::json!({"region": "med_bay"}),
            None,
        ));
    }
    let markers = derivation.derive(&events);
    assert_eq!(markers.len(), 26, "VAL-M10B-022: 12+3+7+4 must equal 26");
    let counts = counts_by_event_type(&markers);
    assert_eq!(counts.get("mission.objective_started"), Some(&4));
    assert_eq!(counts.get("mission.objective_completed"), Some(&4));
    assert_eq!(counts.get("mission.objective_failed"), Some(&4));
    assert_eq!(counts.get("reactor.armor_layer_destroyed"), Some(&3));
    assert_eq!(counts.get("actor_status_changed"), Some(&7));
    assert_eq!(counts.get("atmos.breach_detected"), Some(&4));
}

#[test]
fn chapter_markers_titles_match_default_templates() {
    let rules = default_rules();
    let derivation = ChapterDerivation {
        rules: &rules,
        tick_rate_hz: 60,
    };
    let events = vec![
        event(
            1_000,
            0,
            "mission.objective_started",
            serde_json::json!({"objective_name": "secure_alpha"}),
            None,
        ),
        event(
            5_000,
            0,
            "atmos.breach_detected",
            serde_json::json!({"region": "engineering"}),
            None,
        ),
    ];
    let markers = derivation.derive(&events);
    let titles: Vec<&str> = markers.iter().map(|m| m.title.as_str()).collect();
    assert_eq!(titles[0], "Objective started: secure_alpha");
    assert_eq!(titles[1], "Atmospheric breach: engineering");
}

#[test]
fn chapter_markers_frame_accuracy_under_1_frame_at_60_fps() {
    let rules = default_rules();
    let derivation = ChapterDerivation {
        rules: &rules,
        tick_rate_hz: 60,
    };
    // Synthesize 50 deaths at varied prime-offset ticks to exercise
    // floor / rounding edges.
    let events: Vec<Event> = (0..50u64)
        .map(|i| {
            event(
                i * 1009 + 13,
                i as usize,
                "actor_status_changed",
                serde_json::json!({"status": "killed", "actor_name": "player"}),
                Some(i),
            )
        })
        .collect();
    let markers = derivation.derive(&events);
    assert_eq!(markers.len(), 50);
    for marker in &markers {
        let preset_fps = 60u32;
        let frame_idx = marker.frame_index(preset_fps);
        let expected_frame = marker.tick_index as i64; // 60 Hz → 1 frame per tick
        let diff = (frame_idx - expected_frame).abs();
        assert!(
            diff <= 1,
            "marker @ tick {} produced frame {} (Δ={})",
            marker.tick_index,
            frame_idx,
            diff
        );
    }
}
