//! **M15B** § Schema validation acceptance tests.
//!
//! Covers VAL-M15B-replay-001 (3 new schemas registered + parse cleanly)
//! + per-event payload required-field enforcement against the schemas
//! in `cf-replay/schemas/event/material_phase_nucleated.json`,
//! `material_precipitation_started.json`,
//! `material_gpu_cpu_divergence_detected.json`.

use cf_replay::schemas::{event_schema_for, validate_event_payload};
use serde_json::json;

const M15B_SCHEMA_PAIRS: &[(&str, &str)] = &[
    ("material", "phase_nucleated"),
    ("material", "precipitation_started"),
    ("material", "gpu_cpu_divergence_detected"),
];

/// VAL-M15B-replay-001: all three M15B schemas register under their
/// canonical event names + carry the v0.1 envelope const.
#[test]
fn val_m15b_replay_001_schemas_registered() {
    for (cat, ty) in M15B_SCHEMA_PAIRS {
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

/// VAL-M15B-replay-002: material.phase_nucleated payload required fields.
#[test]
fn val_m15b_replay_002_phase_nucleated_payload() {
    let complete = json!({
        "from_material": 50,
        "to_material": 71,
        "from": "steam",
        "to": "cloud",
        "pos": [128, 32],
        "altitude_px": 100.0,
        "temperature_k": 290.0
    });
    assert!(validate_event_payload("material", "phase_nucleated", &complete).is_ok());
    for skip in [
        "from_material",
        "to_material",
        "pos",
        "altitude_px",
        "temperature_k",
    ] {
        let mut p = complete.clone();
        p.as_object_mut().unwrap().remove(skip);
        assert!(
            validate_event_payload("material", "phase_nucleated", &p).is_err(),
            "removing `{skip}` should reject"
        );
    }
}

/// VAL-M15B-replay-003: material.precipitation_started payload required fields.
#[test]
fn val_m15b_replay_003_precipitation_started_payload() {
    let complete = json!({
        "material": 87,
        "pos": [10, 20],
        "saturation": 0.85,
        "pollutant_fraction": 0.02,
        "ambient": "earth"
    });
    assert!(validate_event_payload("material", "precipitation_started", &complete).is_ok());
    for skip in [
        "material",
        "pos",
        "saturation",
        "pollutant_fraction",
        "ambient",
    ] {
        let mut p = complete.clone();
        p.as_object_mut().unwrap().remove(skip);
        assert!(
            validate_event_payload("material", "precipitation_started", &p).is_err(),
            "removing `{skip}` should reject"
        );
    }
}

/// VAL-M15B-replay-004: material.gpu_cpu_divergence_detected payload
/// required fields.
#[test]
fn val_m15b_replay_004_gpu_cpu_divergence_detected_payload() {
    let complete = json!({
        "gpu_backend": "gpu",
        "cpu_backend": "cpu_fallback",
        "gpu_checksum_hex": "ab".repeat(32),
        "cpu_checksum_hex": "cd".repeat(32),
        "reason": "byte_mismatch"
    });
    assert!(validate_event_payload("material", "gpu_cpu_divergence_detected", &complete).is_ok());
    for skip in [
        "gpu_backend",
        "cpu_backend",
        "gpu_checksum_hex",
        "cpu_checksum_hex",
        "reason",
    ] {
        let mut p = complete.clone();
        p.as_object_mut().unwrap().remove(skip);
        assert!(
            validate_event_payload("material", "gpu_cpu_divergence_detected", &p).is_err(),
            "removing `{skip}` should reject"
        );
    }
}

/// VAL-M15B-replay-005: spec literal ambient enum is enforced —
/// "earth"/"vulcan"/"mimas"/"mars" pass; junk strings fail.
#[test]
fn val_m15b_replay_005_ambient_enum_enforced() {
    for valid in ["earth", "vulcan", "mimas", "mars"] {
        let p = json!({
            "material": 87,
            "pos": [0, 0],
            "saturation": 0.85,
            "pollutant_fraction": 0.02,
            "ambient": valid
        });
        assert!(
            validate_event_payload("material", "precipitation_started", &p).is_ok(),
            "ambient='{valid}' must pass"
        );
    }
    let invalid = json!({
        "material": 87,
        "pos": [0, 0],
        "saturation": 0.85,
        "pollutant_fraction": 0.02,
        "ambient": "junk"
    });
    assert!(
        validate_event_payload("material", "precipitation_started", &invalid).is_err(),
        "junk ambient must reject"
    );
}

/// VAL-M15B-replay-006: spec literal reason enum is enforced —
/// "byte_mismatch"/"tick_skew"/"length_mismatch" pass; others fail.
#[test]
fn val_m15b_replay_006_reason_enum_enforced() {
    for valid in ["byte_mismatch", "tick_skew", "length_mismatch"] {
        let p = json!({
            "gpu_backend": "gpu",
            "cpu_backend": "cpu_fallback",
            "gpu_checksum_hex": "ab".repeat(32),
            "cpu_checksum_hex": "cd".repeat(32),
            "reason": valid
        });
        assert!(
            validate_event_payload("material", "gpu_cpu_divergence_detected", &p).is_ok(),
            "reason='{valid}' must pass"
        );
    }
    let invalid = json!({
        "gpu_backend": "gpu",
        "cpu_backend": "cpu_fallback",
        "gpu_checksum_hex": "ab".repeat(32),
        "cpu_checksum_hex": "cd".repeat(32),
        "reason": "made_up"
    });
    assert!(
        validate_event_payload("material", "gpu_cpu_divergence_detected", &invalid).is_err(),
        "junk reason must reject"
    );
}

/// VAL-M15B-replay-007: spec literal "from='steam' to='cloud'" — verify
/// the producer-side payload builder emits the spec literal names so
/// downstream consumers can filter by string.
#[test]
fn val_m15b_replay_007_phase_nucleated_payload_carries_spec_literal_names() {
    use cf_material::precipitation::{
        evaluate_steam_nucleation, ids, AmbientWorld, PrecipitationInputs,
    };
    let inputs = PrecipitationInputs::with_default_pressure(
        ids::STEAM,
        8,
        4,
        200.0,
        290.0,
        AmbientWorld::Earth,
        0.0,
        1,
    );
    let evt = evaluate_steam_nucleation(inputs).expect("fires");
    let payload = evt.to_recorder_payload();
    assert_eq!(payload["from"].as_str(), Some("steam"));
    assert_eq!(payload["to"].as_str(), Some("cloud"));
    assert_eq!(payload["from_material"].as_u64(), Some(50));
    assert_eq!(payload["to_material"].as_u64(), Some(71));
    // Verify the payload validates against the schema.
    assert!(validate_event_payload("material", "phase_nucleated", &payload).is_ok());
}

/// VAL-M15B-replay-008: precipitation_started payload builder
/// produces a schema-valid payload.
#[test]
fn val_m15b_replay_008_precipitation_started_payload_builder_schema_valid() {
    use cf_material::precipitation::{ids, PrecipitationStartedEvent};
    let evt = PrecipitationStartedEvent {
        pos: [10, 20],
        material: ids::ACID_DROPLET,
        saturation: 0.85,
        pollutant_fraction: 0.10,
        ambient: "vulcan".to_string(),
        tick: 60,
    };
    let payload = evt.to_recorder_payload();
    assert!(validate_event_payload("material", "precipitation_started", &payload).is_ok());
    assert_eq!(payload["ambient"].as_str(), Some("vulcan"));
    assert_eq!(payload["material"].as_u64(), Some(88));
}

/// VAL-M15B-replay-009: gpu_cpu_divergence_detected payload builder
/// produces a schema-valid payload.
#[test]
fn val_m15b_replay_009_divergence_payload_builder_schema_valid() {
    use cf_physics::determinism::DivergenceEvent;
    let evt = DivergenceEvent {
        tick: 42,
        gpu_backend: "gpu".to_string(),
        cpu_backend: "cpu_fallback".to_string(),
        gpu_bytes: [0xab; 32],
        cpu_bytes: [0xcd; 32],
        reason: "byte_mismatch".to_string(),
    };
    let payload = evt.to_recorder_payload();
    assert!(validate_event_payload("material", "gpu_cpu_divergence_detected", &payload).is_ok());
    assert_eq!(payload["gpu_checksum_hex"].as_str().unwrap().len(), 64);
    assert_eq!(payload["cpu_checksum_hex"].as_str().unwrap().len(), 64);
}
