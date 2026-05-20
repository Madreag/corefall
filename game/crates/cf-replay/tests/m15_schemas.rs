//! **M15** § Active Material Kernel — replay-schema validation.
//!
//! Covers VAL-M15-replay-001 (7 schemas registered + parse cleanly) +
//! VAL-M15-replay-002..008 (per-event payload required-field
//! enforcement).

use cf_replay::schemas::{event_schema_for, validate_event_payload};
use serde_json::json;

const M15_SCHEMA_PAIRS: &[(&str, &str)] = &[
    ("material", "reaction_triggered"),
    ("material", "phase_transition"),
    ("material", "cellular_step"),
    ("flask", "thrown"),
    ("flask", "consumed"),
    ("alchemy", "recipe_invoked"),
    ("alchemy", "recipe_completed"),
];

/// VAL-M15-replay-001: all seven M15 replay schemas register under their
/// canonical event names.
#[test]
fn val_m15_replay_001_schemas_registered() {
    for (cat, ty) in M15_SCHEMA_PAIRS {
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

/// VAL-M15-replay-002: material.reaction_triggered payload required fields.
#[test]
fn val_m15_replay_002_reaction_triggered_payload() {
    let complete = json!({
        "reaction_id": "rxn.corrosion.acid_iron",
        "material_a": 21,
        "material_b": 68,
        "output": 38,
        "pos": [10, 20],
        "energy_release_j": 89000.0,
        "auto_ignite": false
    });
    assert!(validate_event_payload("material", "reaction_triggered", &complete).is_ok());
    for skip in [
        "reaction_id",
        "material_a",
        "material_b",
        "output",
        "pos",
        "energy_release_j",
        "auto_ignite",
    ] {
        let mut p = complete.clone();
        p.as_object_mut().unwrap().remove(skip);
        assert!(
            validate_event_payload("material", "reaction_triggered", &p).is_err(),
            "removing `{skip}` should reject"
        );
    }
}

/// VAL-M15-replay-003: material.phase_transition payload required fields.
#[test]
fn val_m15_replay_003_phase_transition_payload() {
    let complete = json!({
        "material": 13,
        "product_material": 50,
        "from_state": "liquid",
        "to_state": "gas",
        "pos": [10, 20],
        "temperature_k": 380.0,
        "latent_heat_j_per_kg": 2260000.0,
        "direction": "forward"
    });
    assert!(validate_event_payload("material", "phase_transition", &complete).is_ok());
    let bad_state = json!({
        "material": 13,
        "product_material": 50,
        "from_state": "ham",
        "to_state": "gas",
        "pos": [10, 20],
        "temperature_k": 380.0,
        "latent_heat_j_per_kg": 2260000.0,
        "direction": "forward"
    });
    assert!(
        validate_event_payload("material", "phase_transition", &bad_state).is_err(),
        "non-canonical from_state must reject"
    );
}

/// VAL-M15-replay-004: material.cellular_step payload required fields.
#[test]
fn val_m15_replay_004_cellular_step_payload() {
    let complete = json!({
        "parity": 0,
        "pixels_moved": 42,
        "dirty_chunks": [[0, 0], [1, 0]]
    });
    assert!(validate_event_payload("material", "cellular_step", &complete).is_ok());
}

/// VAL-M15-replay-005: flask.thrown payload required fields.
#[test]
fn val_m15_replay_005_flask_thrown_payload() {
    let complete = json!({
        "flask_id": 7,
        "thrower_id": 1,
        "kind": "water",
        "contents_material": 13,
        "impact_pos": [100.0, 50.0],
        "volume_ml": 200.0,
        "splash_radius_px": 20.0,
        "splash_pixel_budget": 1600
    });
    assert!(validate_event_payload("flask", "thrown", &complete).is_ok());
    let bad_kind = json!({
        "flask_id": 7,
        "thrower_id": 1,
        "kind": "soda",
        "contents_material": 13,
        "impact_pos": [100.0, 50.0],
        "volume_ml": 200.0,
        "splash_radius_px": 20.0,
        "splash_pixel_budget": 1600
    });
    assert!(
        validate_event_payload("flask", "thrown", &bad_kind).is_err(),
        "non-canonical flask kind must reject"
    );
}

/// VAL-M15-replay-006: flask.consumed payload required fields.
#[test]
fn val_m15_replay_006_flask_consumed_payload() {
    let complete = json!({
        "flask_id": 7,
        "drinker_id": 1,
        "kind": "heal_potion",
        "health_delta": 50.0
    });
    assert!(validate_event_payload("flask", "consumed", &complete).is_ok());
}

/// VAL-M15-replay-007: alchemy.recipe_invoked payload required fields.
#[test]
fn val_m15_replay_007_alchemy_recipe_invoked_payload() {
    let complete = json!({
        "recipe_id": "recipe.steel",
        "station_id": 1,
        "invoked_tick": 100
    });
    assert!(validate_event_payload("alchemy", "recipe_invoked", &complete).is_ok());
}

/// VAL-M15-replay-008: alchemy.recipe_completed payload required fields.
#[test]
fn val_m15_replay_008_alchemy_recipe_completed_payload() {
    let complete = json!({
        "recipe_id": "recipe.steel",
        "station_id": 1,
        "output": 69,
        "output_units": 1,
        "completed_tick": 160
    });
    assert!(validate_event_payload("alchemy", "recipe_completed", &complete).is_ok());
}

/// VAL-M15-replay-009: schemas describe additive payload (additionalProperties: true).
#[test]
fn val_m15_replay_009_schemas_are_additive() {
    for (cat, ty) in M15_SCHEMA_PAIRS {
        let raw = event_schema_for(cat, ty).expect("registered");
        let v: serde_json::Value = serde_json::from_str(raw).unwrap();
        let payload_obj = v.pointer("/properties/payload").and_then(|p| p.as_object());
        if let Some(payload) = payload_obj {
            if let Some(b) = payload.get("additionalProperties").and_then(|v| v.as_bool()) {
                assert!(b, "{cat}.{ty} payload should be additive (additionalProperties: true)");
            }
        }
    }
}
