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

#[test]
fn schema_dump_check_trench_template_dropped_valid() {
    let v = parse_schema("trench_template_dropped.json");
    assert_required_keys(&v, "trench_template_dropped.json");
}

/// VAL-M9B-BREASTWORK-001: trench_breastwork_breached.json parses +
/// declares `module_id="breastwork"` + non-cosmetic.
#[test]
fn schema_dump_check_trench_breastwork_breached_valid() {
    let v = parse_schema("trench_breastwork_breached.json");
    assert_required_keys(&v, "trench_breastwork_breached.json");
    let props = v.get("properties").and_then(|p| p.as_object()).unwrap();
    let module = props
        .get("module_id")
        .and_then(|r| r.as_object())
        .expect("module_id property");
    let enum_vals = module
        .get("enum")
        .and_then(|e| e.as_array())
        .expect("module_id.enum");
    let strings: Vec<&str> = enum_vals.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        strings.contains(&"breastwork"),
        "module_id.enum must contain `breastwork`"
    );
}

/// VAL-M9B-DRAINAGE-001: trench_drainage_flushed.json parses + carries
/// water_depth_before/after.
#[test]
fn schema_dump_check_trench_drainage_flushed_valid() {
    let v = parse_schema("trench_drainage_flushed.json");
    assert_required_keys(&v, "trench_drainage_flushed.json");
    let props = v.get("properties").and_then(|p| p.as_object()).unwrap();
    assert!(
        props.contains_key("water_depth_before"),
        "missing water_depth_before"
    );
    assert!(
        props.contains_key("water_depth_after"),
        "missing water_depth_after"
    );
}

/// VAL-M9B-REVETMENT-001: trench_segment_collapsed.json parses + has
/// the variant enum + cause field.
#[test]
fn schema_dump_check_trench_segment_collapsed_valid() {
    let v = parse_schema("trench_segment_collapsed.json");
    assert_required_keys(&v, "trench_segment_collapsed.json");
    let props = v.get("properties").and_then(|p| p.as_object()).unwrap();
    let variant = props
        .get("variant")
        .and_then(|r| r.as_object())
        .expect("variant property");
    let enum_vals = variant
        .get("enum")
        .and_then(|e| e.as_array())
        .expect("variant.enum");
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
            "trench_segment_collapsed variant.enum missing `{required}`"
        );
    }
    let cause = props
        .get("cause")
        .and_then(|r| r.as_object())
        .expect("cause property");
    let cause_vals = cause
        .get("enum")
        .and_then(|e| e.as_array())
        .expect("cause.enum");
    let cause_strings: Vec<&str> = cause_vals.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        cause_strings.contains(&"no_revetment_in_soft_dirt"),
        "cause.enum must include `no_revetment_in_soft_dirt`"
    );
}

/// VAL-M9B-REPLAY-001 / VAL-M9B-REPLAY-002: all 8 M9B trench replay
/// event schemas exist + parse + declare cosmetic=false. The cosmetic
/// gate ensures the recorder treats them as non-droppable under M4
/// backpressure.
#[test]
fn schema_dump_check_m9b_event_cosmetic_gate() {
    let m9b_schemas = [
        "ai_cover_decision.json",
        "trench_cover_state_changed.json",
        "trench_segment_dug.json",
        "trench_module_placed.json",
        "trench_module_repaired.json",
        "trench_segment_variant_downgraded.json",
        "trench_template_dropped.json",
        "trench_breastwork_breached.json",
        "trench_drainage_flushed.json",
        "trench_segment_collapsed.json",
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

/// VAL-M9B-REPLAY-001: all 8 trench event schemas exist on disk.
#[test]
fn schema_dump_check_m9b_trench_event_schemas_present() {
    let m9b_trench_schemas = [
        "trench_segment_dug.json",
        "trench_module_placed.json",
        "trench_module_repaired.json",
        "trench_template_dropped.json",
        "trench_cover_state_changed.json",
        "trench_breastwork_breached.json",
        "trench_drainage_flushed.json",
        "trench_segment_collapsed.json",
    ];
    for name in m9b_trench_schemas {
        let path = schemas_dir().join(name);
        assert!(
            path.exists(),
            "trench schema {name} missing on disk at {}",
            path.display()
        );
    }
}

/// The 16 M9C event schemas authored by m9c-1 (placeholder shapes) +
/// hydrated to full payloads by m9c-2..m9c-6.
const M9C_EVENT_SCHEMAS: &[&str] = &[
    "mg_nest_crewed.json",
    "mg_nest_uncrewed.json",
    "mg_nest_fired_burst.json",
    "mg_tripod_deployed.json",
    "ammo_box_depleted.json",
    "sandbag_eroded.json",
    "watchtower_destroyed.json",
    "spotlight_dazzled.json",
    "spotter_target_marked.json",
    "mine_armed.json",
    "mine_triggered.json",
    "mine_disarmed.json",
    "minesweeper_detected.json",
    "wire_cut.json",
    "wire_crushed_by_vehicle.json",
    "fence_shocked_actor.json",
];

/// VAL-M9C-009: all 16 new replay event schemas exist on disk.
#[test]
fn schema_dump_check_m9c_event_schemas_present() {
    for name in M9C_EVENT_SCHEMAS {
        let path = schemas_dir().join(name);
        assert!(
            path.exists(),
            "M9C schema {name} missing on disk at {}",
            path.display()
        );
    }
    assert_eq!(M9C_EVENT_SCHEMAS.len(), 16, "expected 16 M9C event schemas");
}

/// VAL-M9C-COSMETIC-GATE: all 16 new M9C event schemas declare
/// `cosmetic=false` (either via `const` or `default`). Protocol /
/// sim-layer events are non-cosmetic and cannot be dropped under M4
/// backpressure.
#[test]
fn m9c_event_cosmetic_gate() {
    for name in M9C_EVENT_SCHEMAS {
        let v = parse_schema(name);
        assert_required_keys(&v, name);
        let props = v
            .get("properties")
            .and_then(|p| p.as_object())
            .unwrap_or_else(|| panic!("{name} missing properties"));
        let cosmetic = props
            .get("cosmetic")
            .and_then(|c| c.as_object())
            .unwrap_or_else(|| panic!("{name} missing cosmetic property"));
        let const_val = cosmetic.get("const").and_then(|v| v.as_bool());
        let default_val = cosmetic.get("default").and_then(|v| v.as_bool());
        assert!(
            const_val == Some(false) || default_val == Some(false),
            "{name} cosmetic must be const=false or default=false (non-droppable)"
        );
    }
}

/// Spec scenario field check: `sandbag_eroded` carries the `from`/`to`
/// fields with the `low | mid | high` enum values from the spec
/// scenario "sandbag_eroded event fires with from=high to=mid".
#[test]
fn schema_dump_check_sandbag_eroded_from_to_enum() {
    let v = parse_schema("sandbag_eroded.json");
    let props = v.get("properties").and_then(|p| p.as_object()).unwrap();
    for field in ["from", "to"] {
        let entry = props
            .get(field)
            .and_then(|r| r.as_object())
            .unwrap_or_else(|| panic!("sandbag_eroded.json missing `{field}` property"));
        let enum_vals = entry
            .get("enum")
            .and_then(|e| e.as_array())
            .unwrap_or_else(|| panic!("sandbag_eroded.json `{field}.enum` missing"));
        let strings: Vec<&str> = enum_vals.iter().filter_map(|v| v.as_str()).collect();
        for required in ["low", "mid", "high"] {
            assert!(
                strings.contains(&required),
                "sandbag_eroded.json {field}.enum missing `{required}`"
            );
        }
    }
}
