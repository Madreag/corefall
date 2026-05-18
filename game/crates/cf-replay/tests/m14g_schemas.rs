//! **M14G** — Schema registration + payload-validation acceptance tests.
//!
//! Covers VAL-M14G-009 (5 schemas registered) + VAL-M14G-010/036/037/038/039
//! (per-event required-field enforcement) + VAL-CROSS-011/027 (17 schemas
//! load with non-colliding names; integer-id collision-free if cf-replay
//! uses an id table).

use cf_replay::schemas::{event_schema_for, validate_event_payload};
use serde_json::json;

const M14G_SCHEMA_PAIRS: &[(&str, &str)] = &[
    ("wound", "created"),
    ("wound", "escalated"),
    ("wound", "aged"),
    ("wound", "scabbed"),
    ("wound", "scarred"),
];

/// VAL-M14G-009: all five M14G replay schemas register under their
/// canonical event names.
#[test]
fn val_m14g_009_wound_event_schemas_registered() {
    for (cat, ty) in M14G_SCHEMA_PAIRS {
        let raw = event_schema_for(cat, ty);
        assert!(raw.is_some(), "missing schema {cat}.{ty}");
        let s = raw.unwrap();
        let v: serde_json::Value = serde_json::from_str(s).expect("schema is json");
        let sv = v
            .pointer("/properties/schema_version/const")
            .and_then(|x| x.as_str())
            .unwrap_or_else(|| panic!("{cat}.{ty} missing properties.schema_version.const"));
        assert_eq!(sv, "prototype-recorder-event.v0.1");
        let cat_const = v
            .pointer("/properties/category/const")
            .and_then(|x| x.as_str())
            .unwrap_or_else(|| panic!("{cat}.{ty} missing properties.category.const"));
        assert_eq!(cat_const, *cat);
        let ty_const = v
            .pointer("/properties/event_type/const")
            .and_then(|x| x.as_str())
            .unwrap_or_else(|| panic!("{cat}.{ty} missing properties.event_type.const"));
        assert_eq!(ty_const, *ty);
    }
}

/// VAL-M14G-010: wound.created payload schema requires
/// {kind, zone, severity, actor_id, tick}.
#[test]
fn val_m14g_010_wound_created_payload_required_fields() {
    let complete = json!({
        "actor_id": 1,
        "tick": 100,
        "kind": "GunshotEntry",
        "zone": "torso_front",
        "severity": 0.4,
    });
    assert!(validate_event_payload("wound", "created", &complete).is_ok());
    for skip in ["actor_id", "tick", "kind", "zone", "severity"] {
        let mut p = complete.clone();
        p.as_object_mut().unwrap().remove(skip);
        let v = validate_event_payload("wound", "created", &p);
        assert!(v.is_err(), "removing `{skip}` must reject");
    }
    let bad_kind = json!({
        "actor_id": 1,
        "tick": 100,
        "kind": "Gunshot",
        "zone": "torso_front",
        "severity": 0.4,
    });
    assert!(
        validate_event_payload("wound", "created", &bad_kind).is_err(),
        "non-canonical kind must reject"
    );
    let over_severity = json!({
        "actor_id": 1,
        "tick": 100,
        "kind": "GunshotEntry",
        "zone": "torso_front",
        "severity": 2.0,
    });
    assert!(
        validate_event_payload("wound", "created", &over_severity).is_err(),
        "severity > 1.0 must reject"
    );
}

/// VAL-M14G-036: wound.escalated payload requires the seven fields.
#[test]
fn val_m14g_036_wound_escalated_payload_required_fields() {
    let complete = json!({
        "actor_id": 1,
        "tick": 100,
        "zone": "foot_right",
        "old_kind": "Burn1st",
        "new_kind": "Burn2nd",
        "old_severity": 0.2,
        "new_severity": 0.5,
    });
    assert!(validate_event_payload("wound", "escalated", &complete).is_ok());
    for skip in ["actor_id", "tick", "zone", "old_kind", "new_kind", "old_severity", "new_severity"] {
        let mut p = complete.clone();
        p.as_object_mut().unwrap().remove(skip);
        assert!(
            validate_event_payload("wound", "escalated", &p).is_err(),
            "removing `{skip}` must reject"
        );
    }
}

/// VAL-M14G-037: wound.aged payload requires the five fields and the
/// new_state enum is closed.
#[test]
fn val_m14g_037_wound_aged_payload_required_fields_and_enum() {
    let complete = json!({
        "actor_id": 1,
        "tick": 100,
        "zone": "torso_front",
        "wound_id": 42,
        "new_state": "bandage_soaked",
    });
    assert!(validate_event_payload("wound", "aged", &complete).is_ok());
    for skip in ["actor_id", "tick", "zone", "wound_id", "new_state"] {
        let mut p = complete.clone();
        p.as_object_mut().unwrap().remove(skip);
        assert!(
            validate_event_payload("wound", "aged", &p).is_err(),
            "removing `{skip}` must reject"
        );
    }
    for canonical in ["bandage_soaked", "scab_forming", "scab_complete", "scar_forming", "necrotic"] {
        let mut p = complete.clone();
        p["new_state"] = json!(canonical);
        assert!(
            validate_event_payload("wound", "aged", &p).is_ok(),
            "canonical state `{canonical}` must validate"
        );
    }
    let mut bad = complete.clone();
    bad["new_state"] = json!("garbage");
    assert!(validate_event_payload("wound", "aged", &bad).is_err(), "garbage state must reject");
}

/// VAL-M14G-038: wound.scabbed payload requires the five fields.
#[test]
fn val_m14g_038_wound_scabbed_payload_required_fields() {
    let complete = json!({
        "actor_id": 1,
        "tick": 100,
        "zone": "arm_right",
        "wound_id": 42,
        "kind": "LacerationLight",
    });
    assert!(validate_event_payload("wound", "scabbed", &complete).is_ok());
    for skip in ["actor_id", "tick", "zone", "wound_id", "kind"] {
        let mut p = complete.clone();
        p.as_object_mut().unwrap().remove(skip);
        assert!(
            validate_event_payload("wound", "scabbed", &p).is_err(),
            "removing `{skip}` must reject"
        );
    }
}

/// VAL-M14G-039: wound.scarred payload requires the five fields.
#[test]
fn val_m14g_039_wound_scarred_payload_required_fields() {
    let complete = json!({
        "actor_id": 1,
        "tick": 100,
        "zone": "torso_front",
        "wound_id": 42,
        "kind": "LacerationModerate",
    });
    assert!(validate_event_payload("wound", "scarred", &complete).is_ok());
    for skip in ["actor_id", "tick", "zone", "wound_id", "kind"] {
        let mut p = complete.clone();
        p.as_object_mut().unwrap().remove(skip);
        assert!(
            validate_event_payload("wound", "scarred", &p).is_err(),
            "removing `{skip}` must reject"
        );
    }
}

/// VAL-CROSS-011: the 17 new schemas load with unique event names. We
/// verify the 5 M14G names are unique among themselves AND collision-free
/// against the 12 prior-milestone names.
#[test]
fn val_cross_011_17_new_schemas_load_with_non_colliding_names() {
    let new_names: Vec<(&str, &str)> = vec![
        // M14C × 3
        ("armor", "heat_jet_traversed"),
        ("armor", "apfsds_long_rod_through"),
        ("armor", "era_pre_detonated"),
        // M14D × 1
        ("collision", "projectile_pair_contact"),
        // M14E × 4
        ("terrain", "structural_integrity_low"),
        ("terrain", "cave_in_triggered"),
        ("terrain", "support_beam_placed"),
        ("terrain", "support_beam_destroyed"),
        // M14F × 4
        ("terrain", "wall_bulging"),
        ("terrain", "wall_crack_advanced"),
        ("terrain", "wall_rupture"),
        ("terrain", "brace_strut_placed"),
        // M14G × 5
        ("wound", "created"),
        ("wound", "escalated"),
        ("wound", "aged"),
        ("wound", "scabbed"),
        ("wound", "scarred"),
    ];
    assert_eq!(new_names.len(), 17, "expected 17 cross-mission new schemas");
    let mut seen: std::collections::HashSet<(&str, &str)> = std::collections::HashSet::new();
    for pair in &new_names {
        assert!(seen.insert(*pair), "duplicate event-name pair {:?}", pair);
        assert!(event_schema_for(pair.0, pair.1).is_some(), "missing schema {:?}", pair);
    }
}

/// VAL-CROSS-027: cf-replay does not use an integer-id table — name
/// uniqueness alone is the registry's collision-safety contract. This test
/// documents that the registry shape is name-keyed and that all 17 new
/// schemas are present + uniquely named.
#[test]
fn val_cross_027_event_name_registry_collision_free() {
    let pairs = [
        ("armor", "heat_jet_traversed"),
        ("armor", "apfsds_long_rod_through"),
        ("armor", "era_pre_detonated"),
        ("collision", "projectile_pair_contact"),
        ("terrain", "structural_integrity_low"),
        ("terrain", "cave_in_triggered"),
        ("terrain", "support_beam_placed"),
        ("terrain", "support_beam_destroyed"),
        ("terrain", "wall_bulging"),
        ("terrain", "wall_crack_advanced"),
        ("terrain", "wall_rupture"),
        ("terrain", "brace_strut_placed"),
        ("wound", "created"),
        ("wound", "escalated"),
        ("wound", "aged"),
        ("wound", "scabbed"),
        ("wound", "scarred"),
    ];
    let mut schemas_seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for (cat, ty) in pairs {
        let s = event_schema_for(cat, ty).unwrap_or_else(|| panic!("missing schema {cat}.{ty}"));
        schemas_seen.insert(s);
    }
    assert_eq!(
        schemas_seen.len(),
        17,
        "schemas must point to distinct files; got {}",
        schemas_seen.len()
    );
}
