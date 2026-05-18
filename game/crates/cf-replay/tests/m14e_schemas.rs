//! **M14E** — Schema registration + payload-validation acceptance tests.
//!
//! Covers VAL-M14E-022 (4 replay schemas + 2 scenario RONs exist and
//! round-trip), VAL-M14E-004 (cave_in_triggered payload required fields),
//! and VAL-M14E-026 (terrain.terrain_cascade schema registration).

use cf_replay::schemas::validate_event_payload;
use serde_json::json;

#[test]
fn val_m14e_022_structural_integrity_low_schema_registered() {
    let v = validate_event_payload(
        "terrain",
        "structural_integrity_low",
        &json!({
            "chunk_id": [2, 1],
            "min_integrity": 180,
            "unsupported_span_px": 24,
            "unstable_cells": 4,
            "level": "l1",
        }),
    );
    assert!(v.is_ok(), "structural_integrity_low must validate: {v:?}");
}

#[test]
fn val_m14e_022_structural_integrity_low_rejects_missing_chunk_id() {
    let v = validate_event_payload(
        "terrain",
        "structural_integrity_low",
        &json!({
            "min_integrity": 100,
            "unsupported_span_px": 24,
            "unstable_cells": 4,
            "level": "l1",
        }),
    );
    assert!(v.is_err(), "missing chunk_id must reject");
}

/// **VAL-M14E-004**: cave_in_triggered payload contains chunk_id, bbox,
/// and falling_debris_count.
#[test]
fn val_m14e_004_cave_in_triggered_payload_required_fields() {
    let ok = validate_event_payload(
        "terrain",
        "cave_in_triggered",
        &json!({
            "chunk_id": [3, 1],
            "bbox": { "min": [256, 64], "max": [288, 80] },
            "falling_debris_count": 128,
            "unsupported_span_px": 32,
            "cascade_primary": true,
        }),
    );
    assert!(ok.is_ok(), "cave_in_triggered must validate: {ok:?}");

    let no_chunk = validate_event_payload(
        "terrain",
        "cave_in_triggered",
        &json!({
            "bbox": { "min": [256, 64], "max": [288, 80] },
            "falling_debris_count": 128,
            "unsupported_span_px": 32,
            "cascade_primary": true,
        }),
    );
    assert!(no_chunk.is_err(), "missing chunk_id must reject");

    let no_bbox = validate_event_payload(
        "terrain",
        "cave_in_triggered",
        &json!({
            "chunk_id": [3, 1],
            "falling_debris_count": 128,
            "unsupported_span_px": 32,
            "cascade_primary": true,
        }),
    );
    assert!(no_bbox.is_err(), "missing bbox must reject");

    let no_debris = validate_event_payload(
        "terrain",
        "cave_in_triggered",
        &json!({
            "chunk_id": [3, 1],
            "bbox": { "min": [256, 64], "max": [288, 80] },
            "unsupported_span_px": 32,
            "cascade_primary": true,
        }),
    );
    assert!(no_debris.is_err(), "missing falling_debris_count must reject");
}

#[test]
fn val_m14e_022_support_beam_placed_schema_registered() {
    let v = validate_event_payload(
        "terrain",
        "support_beam_placed",
        &json!({
            "actor_id": 1,
            "world_pos": [120.0, 32.0],
            "chunk_id": [0, 0],
            "cost": { "iron": 2, "wood": 1 },
            "footprint_half_px": 8,
        }),
    );
    assert!(v.is_ok(), "support_beam_placed must validate: {v:?}");
}

#[test]
fn val_m14e_022_support_beam_placed_rejects_bad_cost() {
    let v = validate_event_payload(
        "terrain",
        "support_beam_placed",
        &json!({
            "actor_id": 1,
            "world_pos": [120.0, 32.0],
            "chunk_id": [0, 0],
            // missing wood key
            "cost": { "iron": 2 },
        }),
    );
    assert!(v.is_err(), "cost missing wood must reject");
}

#[test]
fn val_m14e_022_support_beam_destroyed_schema_registered() {
    let v = validate_event_payload(
        "terrain",
        "support_beam_destroyed",
        &json!({
            "world_pos": [120.0, 32.0],
            "chunk_id": [0, 0],
            "cause": "demolish",
            "actor_id": 1,
        }),
    );
    assert!(v.is_ok(), "support_beam_destroyed must validate: {v:?}");

    let bad_cause = validate_event_payload(
        "terrain",
        "support_beam_destroyed",
        &json!({
            "world_pos": [120.0, 32.0],
            "chunk_id": [0, 0],
            "cause": "garbage",
        }),
    );
    assert!(bad_cause.is_err(), "non-enum cause must reject");
}

/// **VAL-M14E-026**: cascade family event registers under
/// `terrain.terrain_cascade` with `cascade_kind="cave_in"`.
#[test]
fn val_m14e_026_terrain_cascade_schema_registered() {
    let v = validate_event_payload(
        "terrain",
        "terrain_cascade",
        &json!({
            "primary_chunk_id": [1, 0],
            "secondary_chunk_id": [2, 0],
            "cascade_kind": "cave_in",
            "tick_delta": 8,
        }),
    );
    assert!(v.is_ok(), "terrain.terrain_cascade must validate: {v:?}");

    let bad_kind = validate_event_payload(
        "terrain",
        "terrain_cascade",
        &json!({
            "primary_chunk_id": [1, 0],
            "secondary_chunk_id": [2, 0],
            "cascade_kind": "not_a_kind",
        }),
    );
    assert!(bad_kind.is_err(), "non-enum cascade_kind must reject");
}
