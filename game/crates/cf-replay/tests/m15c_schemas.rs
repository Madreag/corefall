//! M15C § Files: 2 new replay schemas (material.registered +
//! material.registry_validation_failed) register against the v0.1
//! envelope + enforce the spec-mandated payload-required-field set.

use cf_replay::schemas::{event_schema_for, validate_event_payload};
use serde_json::json;

const M15C_SCHEMA_PAIRS: &[(&str, &str)] = &[
    ("material", "registered"),
    ("material", "registry_validation_failed"),
];

/// VAL-M15C-replay-001: both M15C schemas register under their canonical
/// event names + carry the v0.1 envelope const.
#[test]
fn val_m15c_replay_001_schemas_registered() {
    for (cat, ty) in M15C_SCHEMA_PAIRS {
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

/// VAL-M15C-replay-002: material.registered payload required fields.
#[test]
fn val_m15c_replay_002_registered_payload_required_fields() {
    let complete = json!({
        "id": 68,
        "name": "iron",
        "display_name": "Iron",
        "state": "solid",
        "hardness": 8,
        "density_kg_per_m3": 7870.0,
        "specific_heat_capacity_j_per_kg_k": 449.0,
        "thermal_conductivity_w_per_m_k": 80.4,
        "color_hex": "888880"
    });
    assert!(validate_event_payload("material", "registered", &complete).is_ok());
    for skip in [
        "id",
        "name",
        "display_name",
        "state",
        "hardness",
        "density_kg_per_m3",
        "specific_heat_capacity_j_per_kg_k",
        "thermal_conductivity_w_per_m_k",
        "color_hex",
    ] {
        let mut incomplete = complete.clone();
        incomplete.as_object_mut().unwrap().remove(skip);
        let err = validate_event_payload("material", "registered", &incomplete);
        assert!(err.is_err(), "must reject missing `{skip}`: got {:?}", err);
    }
}

/// VAL-M15C-replay-003: material.registered.state enum honors the canonical
/// labels (solid/liquid/gas/powder/plasma/energy_field).
#[test]
fn val_m15c_replay_003_registered_state_enum_accepts_canonical() {
    for state in ["solid", "liquid", "gas", "powder", "plasma", "energy_field"] {
        let payload = json!({
            "id": 0,
            "name": "x",
            "display_name": "X",
            "state": state,
            "hardness": 0.0,
            "density_kg_per_m3": 0.0,
            "specific_heat_capacity_j_per_kg_k": 0.0,
            "thermal_conductivity_w_per_m_k": 0.0,
            "color_hex": "000000"
        });
        assert!(
            validate_event_payload("material", "registered", &payload).is_ok(),
            "state `{state}` must validate cleanly"
        );
    }
    let bad = json!({
        "id": 0,
        "name": "x",
        "display_name": "X",
        "state": "garbage_state",
        "hardness": 0.0,
        "density_kg_per_m3": 0.0,
        "specific_heat_capacity_j_per_kg_k": 0.0,
        "thermal_conductivity_w_per_m_k": 0.0,
        "color_hex": "000000"
    });
    assert!(validate_event_payload("material", "registered", &bad).is_err());
}

/// VAL-M15C-replay-004: material.registry_validation_failed payload
/// required fields (kind, path, message, hint).
#[test]
fn val_m15c_replay_004_validation_failed_payload_required_fields() {
    let complete = json!({
        "kind": "missing_required_field",
        "path": "materials[42].specific_heat_capacity_j_per_kg_k",
        "message": "M15C: field 'specific_heat_capacity_j_per_kg_k' required",
        "hint": "Add the field with the ONI/Stationeers canonical value."
    });
    assert!(validate_event_payload("material", "registry_validation_failed", &complete).is_ok());
    for skip in ["kind", "path", "message", "hint"] {
        let mut incomplete = complete.clone();
        incomplete.as_object_mut().unwrap().remove(skip);
        let err = validate_event_payload("material", "registry_validation_failed", &incomplete);
        assert!(err.is_err(), "must reject missing `{skip}`: got {:?}", err);
    }
}

/// VAL-M15C-replay-005: material.registry_validation_failed.kind enum
/// honors the cf-material::loader::RegistryValidationError.kind set.
#[test]
fn val_m15c_replay_005_validation_failed_kind_enum_accepts_canonical() {
    for kind in [
        "schema_version_mismatch",
        "duplicate_id",
        "unknown_field",
        "missing_required_field",
        "launch_set_mismatch",
        "integrity_overflow",
        "affordance_conflict",
        "color_format",
        "unknown_spawn_material",
    ] {
        let payload = json!({
            "kind": kind,
            "path": "materials[0]",
            "message": "msg",
            "hint": "hint"
        });
        assert!(
            validate_event_payload("material", "registry_validation_failed", &payload).is_ok(),
            "kind `{kind}` must validate cleanly"
        );
    }
    let bad = json!({
        "kind": "garbage_kind",
        "path": "materials[0]",
        "message": "msg",
        "hint": "hint"
    });
    assert!(validate_event_payload("material", "registry_validation_failed", &bad).is_err());
}
