//! **M14H** — Schema registration + payload-validation acceptance tests.
//!
//! Covers VAL-M14H-001 (18 schemas registered) + per-event required-field
//! enforcement. Mirrors the M14G test scaffolding.

use cf_replay::schemas::{event_schema_for, validate_event_payload};
use serde_json::json;

const M14H_SCHEMA_PAIRS: &[(&str, &str)] = &[
    ("treatment", "applied"),
    ("treatment", "completed"),
    ("treatment", "failed"),
    ("treatment", "cancelled"),
    ("cardiac", "arrested"),
    ("cardiac", "cpr_round"),
    ("cardiac", "defib_attempted"),
    ("cardiac", "restored"),
    ("cardiac", "expired"),
    ("surgery", "phase_started"),
    ("surgery", "phase_completed"),
    ("surgery", "skill_check"),
    ("surgery", "completed"),
    ("surgery", "failed"),
    ("scan", "started"),
    ("scan", "completed"),
    ("triage", "queue_changed"),
    ("patient", "assessed"),
];

/// VAL-M14H-001: all 18 M14H replay schemas register under their canonical
/// event names.
#[test]
fn val_m14h_001_event_schemas_registered() {
    assert_eq!(M14H_SCHEMA_PAIRS.len(), 18, "M14H spec requires 18 events");
    for (cat, ty) in M14H_SCHEMA_PAIRS {
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

/// treatment.applied requires actor_id + tick + kind + apply_seconds.
#[test]
fn treatment_applied_required_fields() {
    let complete = json!({
        "actor_id": 1,
        "tick": 100,
        "kind": "field_bandage_v1",
        "apply_seconds": 5.0
    });
    assert!(validate_event_payload("treatment", "applied", &complete).is_ok());
    for skip in ["actor_id", "tick", "kind", "apply_seconds"] {
        let mut p = complete.clone();
        p.as_object_mut().unwrap().remove(skip);
        assert!(
            validate_event_payload("treatment", "applied", &p).is_err(),
            "removing `{skip}` must reject"
        );
    }
}

/// treatment.failed accepts the spec-locked reason set.
#[test]
fn treatment_failed_reason_enum() {
    let reasons = [
        "wrong_origin",
        "missing_tool",
        "missing_skill",
        "out_of_charges",
        "dirty_wound_failure",
        "blood_incompatibility",
        "cancelled",
    ];
    for r in reasons {
        let p = json!({
            "actor_id": 1,
            "tick": 1,
            "kind": "sutures_v1",
            "reason": r,
        });
        assert!(
            validate_event_payload("treatment", "failed", &p).is_ok(),
            "expected ok for reason={r}"
        );
    }
    let bad = json!({
        "actor_id": 1,
        "tick": 1,
        "kind": "sutures_v1",
        "reason": "unknown_reason",
    });
    assert!(validate_event_payload("treatment", "failed", &bad).is_err());
}

/// cardiac.defib_attempted accepts the success probability payload shape.
#[test]
fn cardiac_defib_attempted_shape() {
    let p = json!({
        "actor_id": 1,
        "tick": 1,
        "success_probability_x1000": 700,
        "roll_x1000": 500,
        "passed": true,
        "charges_remaining": 3,
    });
    assert!(validate_event_payload("cardiac", "defib_attempted", &p).is_ok());
    let missing = json!({
        "actor_id": 1,
        "tick": 1,
        "passed": true,
    });
    assert!(validate_event_payload("cardiac", "defib_attempted", &missing).is_err());
}

/// surgery.phase_started enforces phase enum.
#[test]
fn surgery_phase_started_phase_enum() {
    for phase in &["Open", "Diagnose", "Operate", "Close", "Recover"] {
        let p = json!({
            "actor_id": 1,
            "tick": 1,
            "phase": phase,
            "duration_seconds": 10.0,
        });
        assert!(
            validate_event_payload("surgery", "phase_started", &p).is_ok(),
            "phase {phase} must accept"
        );
    }
    let bad = json!({
        "actor_id": 1,
        "tick": 1,
        "phase": "Other",
        "duration_seconds": 10.0,
    });
    assert!(validate_event_payload("surgery", "phase_started", &bad).is_err());
}

/// triage.queue_changed enforces an array of actor ids.
#[test]
fn triage_queue_changed_required_array() {
    let p = json!({
        "tick": 100,
        "row_count": 3,
        "selected_actor_id": 7,
        "actor_ids_sorted": [7, 8, 9],
    });
    assert!(validate_event_payload("triage", "queue_changed", &p).is_ok());
    let missing = json!({
        "tick": 100,
        "row_count": 3,
        "selected_actor_id": 7,
    });
    assert!(validate_event_payload("triage", "queue_changed", &missing).is_err());
}

/// patient.assessed enforces step enum.
#[test]
fn patient_assessed_step_enum() {
    for step in &["assess", "triage", "stabilize", "treat", "monitor"] {
        let p = json!({
            "medic_actor_id": 1,
            "target_actor_id": 2,
            "tick": 100,
            "step": step,
        });
        assert!(
            validate_event_payload("patient", "assessed", &p).is_ok(),
            "step {step} must accept"
        );
    }
}
