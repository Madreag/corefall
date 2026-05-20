//! **M14I** replay-schema validation tests — long-term-consequence
//! event surface registers correctly and enforces required fields.

use cf_replay::schemas::{event_schema_for, validate_event_payload};
use serde_json::json;

const M14I_SCHEMA_PAIRS: &[(&str, &str)] = &[
    ("scar", "acquired"),
    ("phantom_limb", "acquired"),
    ("phantom_limb", "panic_attack"),
    ("memory_loss", "minor_acquired"),
    ("memory_loss", "major_acquired"),
    ("age", "year_advanced"),
    ("age", "retirement_offered"),
    ("age", "terminal_roll"),
    ("prosthetic", "installed"),
    ("prosthetic", "malfunctioned"),
    ("prosthetic", "maintained"),
    ("disease", "exposed"),
    ("veteran", "retired"),
];

#[test]
fn m14i_event_schemas_registered() {
    for (cat, ty) in M14I_SCHEMA_PAIRS {
        let raw = event_schema_for(cat, ty);
        assert!(raw.is_some(), "missing schema {cat}.{ty}");
        let v: serde_json::Value = serde_json::from_str(raw.unwrap()).expect("schema is json");
        let sv = v
            .pointer("/properties/schema_version/const")
            .and_then(|x| x.as_str())
            .unwrap_or_else(|| panic!("{cat}.{ty} missing schema_version"));
        assert_eq!(sv, "prototype-recorder-event.v0.1");
    }
}

#[test]
fn scar_acquired_required_fields() {
    let complete = json!({
        "actor_id": 1,
        "tick": 100,
        "scar_id": 0,
        "kind": "LacerationSevere",
        "zone": "arm_left",
        "severity_at_close": 0.8,
        "closure_method": "suture_kit",
        "functional_debuff": "reduced_zone_strength",
    });
    assert!(validate_event_payload("scar", "acquired", &complete).is_ok());
    for skip in [
        "actor_id",
        "tick",
        "scar_id",
        "kind",
        "zone",
        "severity_at_close",
        "closure_method",
        "functional_debuff",
    ] {
        let mut p = complete.clone();
        p.as_object_mut().unwrap().remove(skip);
        assert!(
            validate_event_payload("scar", "acquired", &p).is_err(),
            "removing `{skip}` must reject"
        );
    }
}

#[test]
fn age_terminal_roll_outcome_enum() {
    for outcome in ["survived", "death"] {
        let p = json!({
            "actor_id": 1,
            "tick": 100,
            "probability_x1000": 100,
            "outcome": outcome,
        });
        assert!(validate_event_payload("age", "terminal_roll", &p).is_ok());
    }
    let p_bad = json!({
        "actor_id": 1,
        "tick": 100,
        "probability_x1000": 100,
        "outcome": "blasted",
    });
    assert!(validate_event_payload("age", "terminal_roll", &p_bad).is_err());
}

#[test]
fn prosthetic_installed_tier_enum() {
    for tier in ["t1", "t2", "t3"] {
        let p = json!({
            "actor_id": 1,
            "tick": 100,
            "kind": "prosthetic_leg_t1",
            "tier": tier,
            "zone": "leg_right",
            "functional_restoration": 0.7,
        });
        assert!(validate_event_payload("prosthetic", "installed", &p).is_ok());
    }
    let p_bad = json!({
        "actor_id": 1,
        "tick": 100,
        "kind": "prosthetic_leg_t1",
        "tier": "t9",
        "zone": "leg_right",
    });
    assert!(validate_event_payload("prosthetic", "installed", &p_bad).is_err());
}

#[test]
fn memory_loss_minor_required_fields() {
    let p = json!({
        "actor_id": 1,
        "tick": 100,
        "concussion_count": 5,
    });
    assert!(validate_event_payload("memory_loss", "minor_acquired", &p).is_ok());
    let p_bad = json!({
        "tick": 100,
        "concussion_count": 5,
    });
    assert!(validate_event_payload("memory_loss", "minor_acquired", &p_bad).is_err());
}

#[test]
fn disease_exposed_required_fields() {
    let p = json!({
        "actor_id": 1,
        "tick": 100,
        "vector": "long_term_radiation",
        "cumulative_dose": 7.0,
        "threshold": 6.0,
    });
    assert!(validate_event_payload("disease", "exposed", &p).is_ok());
    let p_bad = json!({
        "tick": 100,
        "vector": "long_term_radiation",
    });
    assert!(validate_event_payload("disease", "exposed", &p_bad).is_err());
}
