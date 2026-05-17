//! M9B: schema validation gate for `cf-replay/schemas/event/*.json`.
//!
//! Per VAL-M9B-AICOVER-001 + VAL-M9B-CFCTL-003 the closure-feature
//! worker invokes `cargo test -p cf-replay schema_dump_check` to assert
//! every event schema parses as valid JSON Schema (draft-07+ shape) and
//! declares the required keys (`$id`, `title`, `type`, `properties`).
//!
//! This test file is **additive only** — it does not gate the
//! pre-existing 300+ event schemas (those have already shipped); it
//! exists so the m9b-3 worker (this feature) and the m9b-4 worker can
//! assert their new schemas parse cleanly without relying on the
//! larger M9B event-cosmetic-gate test that lands with m9b-4.

use std::path::PathBuf;

fn schemas_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("schemas/event")
}

fn parse_schema(name: &str) -> serde_json::Value {
    let path = schemas_dir().join(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn assert_required_keys(v: &serde_json::Value, name: &str) {
    let obj = v
        .as_object()
        .unwrap_or_else(|| panic!("{name} root must be JSON object"));
    for key in ["$id", "title", "type", "properties"] {
        assert!(obj.contains_key(key), "{name} missing required key `{key}`");
    }
}

/// VAL-M9B-AICOVER-001: ai_cover_decision.json parses + declares the
/// 4-variant reason_label enum + non-cosmetic.
#[test]
fn schema_dump_check_ai_cover_decision_valid() {
    let v = parse_schema("ai_cover_decision.json");
    assert_required_keys(&v, "ai_cover_decision.json");
    let props = v.get("properties").and_then(|p| p.as_object()).unwrap();
    let reason = props
        .get("reason_label")
        .and_then(|r| r.as_object())
        .expect("reason_label property exists");
    let enum_vals = reason
        .get("enum")
        .and_then(|e| e.as_array())
        .expect("reason_label.enum exists");
    let strings: Vec<&str> = enum_vals.iter().filter_map(|v| v.as_str()).collect();
    for required in [
        "step_up_for_shot",
        "step_down_to_reload",
        "hold_full_cover",
        "reload_safe",
    ] {
        assert!(
            strings.contains(&required),
            "ai_cover_decision.json reason_label.enum missing `{required}`; have {strings:?}"
        );
    }
    // Cosmetic gate: schema must declare cosmetic=false (either via const
    // or default) so the recorder treats it as a non-cosmetic event.
    let cosmetic = props
        .get("cosmetic")
        .and_then(|c| c.as_object())
        .expect("cosmetic property exists");
    let cosmetic_const = cosmetic.get("const").and_then(|v| v.as_bool());
    assert_eq!(
        cosmetic_const,
        Some(false),
        "ai_cover_decision cosmetic.const must be false"
    );
}

/// VAL-M9B-COVER-002: trench_cover_state_changed.json parses + declares
/// segment_boundary / stance_change enum on `cause`.
#[test]
fn schema_dump_check_trench_cover_state_changed_valid() {
    let v = parse_schema("trench_cover_state_changed.json");
    assert_required_keys(&v, "trench_cover_state_changed.json");
    let props = v.get("properties").and_then(|p| p.as_object()).unwrap();
    let cause = props
        .get("cause")
        .and_then(|r| r.as_object())
        .expect("cause property exists");
    let enum_vals = cause
        .get("enum")
        .and_then(|e| e.as_array())
        .expect("cause.enum exists");
    let strings: Vec<&str> = enum_vals.iter().filter_map(|v| v.as_str()).collect();
    for required in ["segment_boundary", "stance_change"] {
        assert!(
            strings.contains(&required),
            "trench_cover_state_changed.json cause.enum missing `{required}`"
        );
    }
}

#[test]
fn schema_dump_check_trench_segment_dug_valid() {
    let v = parse_schema("trench_segment_dug.json");
    assert_required_keys(&v, "trench_segment_dug.json");
    let props = v.get("properties").and_then(|p| p.as_object()).unwrap();
    let variant = props
        .get("variant")
        .and_then(|r| r.as_object())
        .expect("variant property exists");
    let enum_vals = variant
        .get("enum")
        .and_then(|e| e.as_array())
        .expect("variant.enum exists");
    let strings: Vec<&str> = enum_vals.iter().filter_map(|v| v.as_str()).collect();
    for required in [
        "shallow_scrape",
        "standard",
        "deep",
        "communication",
        "fire_step",
        "parapet_raised",
    ] {
        assert!(
            strings.contains(&required),
            "trench_segment_dug.json variant.enum missing `{required}`"
        );
    }
}

#[test]
fn schema_dump_check_trench_module_placed_valid() {
    let v = parse_schema("trench_module_placed.json");
    assert_required_keys(&v, "trench_module_placed.json");
}

#[test]
fn schema_dump_check_trench_module_repaired_valid() {
    let v = parse_schema("trench_module_repaired.json");
    assert_required_keys(&v, "trench_module_repaired.json");
}

#[test]
fn schema_dump_check_trench_segment_variant_downgraded_valid() {
    let v = parse_schema("trench_segment_variant_downgraded.json");
    assert_required_keys(&v, "trench_segment_variant_downgraded.json");
}

/// VAL-M9B-REPLAY-002 sub-check: every M9B-owned schema authored by
/// this feature declares `cosmetic=false` so the recorder treats them
/// as non-droppable under M4 backpressure.
#[test]
fn schema_dump_check_m9b_event_cosmetic_gate() {
    let m9b_schemas = [
        "ai_cover_decision.json",
        "trench_cover_state_changed.json",
        "trench_segment_dug.json",
        "trench_module_placed.json",
        "trench_module_repaired.json",
        "trench_segment_variant_downgraded.json",
    ];
    for name in m9b_schemas {
        let v = parse_schema(name);
        let props = v
            .get("properties")
            .and_then(|p| p.as_object())
            .unwrap_or_else(|| panic!("{name} missing properties"));
        let cosmetic = props
            .get("cosmetic")
            .and_then(|c| c.as_object())
            .unwrap_or_else(|| panic!("{name} missing cosmetic property"));
        let const_val = cosmetic.get("const").and_then(|v| v.as_bool());
        assert_eq!(
            const_val,
            Some(false),
            "{name} cosmetic.const must be false (non-droppable)"
        );
    }
}
