//! M10B cause-chain overlay integration tests.
//!
//! Verification command: `cargo test -p cf-replay-export cause_chain_overlay`
//! (expect: chain renders + locale switch PASS).
//!
//! VAL-M10B-030 — every line is plain-language; locale switch
//! produces translated text.
//! VAL-CROSS-011 — cause-chain renders M9C `fence_shocked_actor` entry
//! (`electrified_fence` + `shock` substring).
//! VAL-CROSS-012 — cause-chain renders M9C `mine_triggered` entry
//! (`mine_` + numeric yield substring).

use cf_localization::LocalizationTable;
use cf_replay::Event;
use cf_replay_export::overlay_cause_chain::{render_event_plain, CauseChainOverlay};
use std::path::PathBuf;

fn synth_event(event_id: &str, event_type: &str, parent: Option<&str>, payload: serde_json::Value) -> Event {
    Event {
        schema_version: cf_replay::EVENT_SCHEMA_VERSION.to_string(),
        run_id: "cause_chain_overlay_test".into(),
        tick: 100,
        sim_time_ms: 100.0,
        event_id: event_id.into(),
        category: "test".into(),
        event_type: event_type.into(),
        payload,
        parent_event_id: parent.map(str::to_owned),
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

fn en_locale() -> LocalizationTable {
    LocalizationTable::english_baseline().expect("english baseline must load")
}

fn es_locale() -> LocalizationTable {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join("content/localization/es-ES.json");
    let txt = std::fs::read_to_string(&path).expect("es-ES.json must load");
    LocalizationTable::load_from_json(&txt).expect("es-ES table must parse")
}

#[test]
fn cause_chain_overlay_renders_plain_language() {
    let mut e = synth_event(
        "d",
        "actor_status_changed",
        None,
        serde_json::json!({"status": "killed"}),
    );
    e.actor_id = Some(7);
    let line = render_event_plain(&e, &en_locale());
    assert!(!line.contains("event_type:"));
    assert!(!line.contains("{")); // no curly-brace placeholders
    assert!(!line.contains("}"));
}

#[test]
fn cause_chain_overlay_locale_switch_translates_text() {
    let mut e = synth_event(
        "d",
        "actor_status_changed",
        None,
        serde_json::json!({"status": "killed"}),
    );
    e.actor_id = Some(99);
    let en_line = render_event_plain(&e, &en_locale());
    let es_line = render_event_plain(&e, &es_locale());
    assert!(en_line.contains("killed") || en_line.contains("died"));
    assert!(
        es_line.contains("matado") || es_line.contains("muri"),
        "Spanish line: {es_line}"
    );
    assert_ne!(en_line, es_line);
}

#[test]
fn cause_chain_overlay_renders_fence_shocked_with_substrings() {
    let events = vec![
        synth_event("a", "run_started", None, serde_json::json!({})),
        synth_event(
            "b",
            "fence_shocked_actor",
            Some("a"),
            serde_json::json!({
                "fence_id": 17,
                "actor_name": "scout",
            }),
        ),
        synth_event(
            "c",
            "actor_status_changed",
            Some("b"),
            serde_json::json!({"status": "killed"}),
        ),
    ];
    let overlay = CauseChainOverlay::new("c".into(), en_locale());
    let lines = overlay.render(&events).expect("chain renders");
    let combined = lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join(" | ");
    assert!(
        combined.contains("electrified_fence") || combined.contains("Electrified fence"),
        "cause-chain must include electrified_fence substring: {combined}"
    );
    assert!(
        combined.to_ascii_lowercase().contains("shock"),
        "cause-chain must include shock substring: {combined}"
    );
}

#[test]
fn cause_chain_overlay_renders_mine_triggered_with_yield() {
    let events = vec![
        synth_event("a", "run_started", None, serde_json::json!({})),
        synth_event(
            "b",
            "mine_triggered",
            Some("a"),
            serde_json::json!({
                "mine_id": "mine_pressure_004",
                "trigger_kind": "pressure",
                "yield_joules": 120,
            }),
        ),
        synth_event(
            "c",
            "actor_status_changed",
            Some("b"),
            serde_json::json!({"status": "killed"}),
        ),
    ];
    let overlay = CauseChainOverlay::new("c".into(), en_locale());
    let lines = overlay.render(&events).expect("chain renders");
    let combined = lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join(" | ");
    assert!(
        combined.contains("mine_") || combined.contains("Mine "),
        "cause-chain must include mine_ substring: {combined}"
    );
    assert!(combined.contains("120"), "cause-chain must include yield: {combined}");
}

#[test]
fn cause_chain_overlay_returns_none_for_missing_focus_id() {
    let events = vec![synth_event("a", "run_started", None, serde_json::json!({}))];
    let overlay = CauseChainOverlay::new("does_not_exist".into(), en_locale());
    assert!(overlay.render(&events).is_none());
}
