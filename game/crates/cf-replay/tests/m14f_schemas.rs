//! **M14F** — Schema registration + payload-validation acceptance tests.
//!
//! Covers VAL-M14F-019 (four schemas exist and validate) + VAL-M14F-027
//! (wall_rupture payload contract) + VAL-M14F-004 (brace_strut_placed
//! schema contract).

use cf_replay::schemas::validate_event_payload;
use serde_json::json;

/// VAL-M14F-019 + VAL-M14F-002: terrain.wall_bulging schema registers
/// and validates payloads with the L1 contract fields.
#[test]
fn val_m14f_019_wall_bulging_schema_registered() {
    let v = validate_event_payload(
        "terrain",
        "wall_bulging",
        &json!({
            "chunk_id": [0, 0],
            "bbox": { "min": [64, 60], "max": [88, 76] },
            "unsupported_span_px": 24,
            "min_integrity": 180,
            "lateral_yield_strength": 30,
            "vibration_modifier": 1.0,
            "level": "l1",
        }),
    );
    assert!(v.is_ok(), "wall_bulging must validate: {v:?}");
}

#[test]
fn val_m14f_019_wall_bulging_rejects_missing_chunk_id() {
    let v = validate_event_payload(
        "terrain",
        "wall_bulging",
        &json!({
            "bbox": { "min": [0, 0], "max": [16, 16] },
            "unsupported_span_px": 24,
        }),
    );
    assert!(v.is_err(), "missing chunk_id must reject");
}

/// VAL-M14F-019 + VAL-M14F-012: terrain.wall_crack_advanced schema
/// registers and validates L2 escalation payloads.
#[test]
fn val_m14f_019_wall_crack_advanced_schema_registered() {
    let v = validate_event_payload(
        "terrain",
        "wall_crack_advanced",
        &json!({
            "chunk_id": [0, 0],
            "bbox": { "min": [64, 60], "max": [88, 76] },
            "unsupported_span_px": 24,
            "min_integrity": 110,
            "lateral_yield_strength": 30,
            "vibration_modifier": 1.0,
            "level": "l2",
        }),
    );
    assert!(v.is_ok(), "wall_crack_advanced must validate: {v:?}");
}

/// VAL-M14F-019 + VAL-M14F-027: terrain.wall_rupture payload must
/// carry the three required fields chunk_id + bbox + falling_debris_count.
#[test]
fn val_m14f_027_wall_rupture_payload_required_fields() {
    let ok = validate_event_payload(
        "terrain",
        "wall_rupture",
        &json!({
            "chunk_id": [1, 0],
            "bbox": { "min": [256, 60], "max": [288, 200] },
            "falling_debris_count": 128,
            "unsupported_span_px": 32,
            "lateral_yield_strength": 50,
            "vibration_modifier": 1.0,
            "cascade_primary": true,
            "trigger": "integrity_decay",
        }),
    );
    assert!(ok.is_ok(), "wall_rupture must validate: {ok:?}");

    let no_chunk = validate_event_payload(
        "terrain",
        "wall_rupture",
        &json!({
            "bbox": { "min": [256, 60], "max": [288, 200] },
            "falling_debris_count": 128,
        }),
    );
    assert!(no_chunk.is_err(), "missing chunk_id must reject");

    let no_bbox = validate_event_payload(
        "terrain",
        "wall_rupture",
        &json!({
            "chunk_id": [1, 0],
            "falling_debris_count": 128,
        }),
    );
    assert!(no_bbox.is_err(), "missing bbox must reject");

    let no_debris = validate_event_payload(
        "terrain",
        "wall_rupture",
        &json!({
            "chunk_id": [1, 0],
            "bbox": { "min": [256, 60], "max": [288, 200] },
        }),
    );
    assert!(no_debris.is_err(), "missing falling_debris_count must reject");
}

/// VAL-M14F-019: wall_rupture validates the three rupture-trigger
/// classifications (integrity_decay / pressure_blowout /
/// explosive_damage).
#[test]
fn val_m14f_019_wall_rupture_validates_each_trigger_enum() {
    for trigger in ["integrity_decay", "pressure_blowout", "explosive_damage"] {
        let v = validate_event_payload(
            "terrain",
            "wall_rupture",
            &json!({
                "chunk_id": [1, 0],
                "bbox": { "min": [0, 0], "max": [16, 16] },
                "falling_debris_count": 32,
                "trigger": trigger,
            }),
        );
        assert!(v.is_ok(), "trigger {trigger} must validate: {v:?}");
    }
    let bad = validate_event_payload(
        "terrain",
        "wall_rupture",
        &json!({
            "chunk_id": [1, 0],
            "bbox": { "min": [0, 0], "max": [16, 16] },
            "falling_debris_count": 32,
            "trigger": "garbage",
        }),
    );
    assert!(bad.is_err(), "unknown trigger value must reject");
}

/// VAL-M14F-019 + VAL-M14F-004: terrain.brace_strut_placed schema
/// registers; payload validates with required tier + cost fields.
#[test]
fn val_m14f_004_brace_strut_placed_schema_registered() {
    let v = validate_event_payload(
        "terrain",
        "brace_strut_placed",
        &json!({
            "actor_id": 1,
            "tier": "t1",
            "world_pos": [120.0, 32.0],
            "chunk_id": [0, 0],
            "lateral_cell": [8, 8],
            "cost": { "iron": 2, "wood": 1 },
            "lock_radius_px": 8,
        }),
    );
    assert!(v.is_ok(), "brace_strut_placed must validate: {v:?}");
}

#[test]
fn val_m14f_004_brace_strut_placed_rejects_bad_tier() {
    let v = validate_event_payload(
        "terrain",
        "brace_strut_placed",
        &json!({
            "actor_id": 1,
            "tier": "t4",
            "world_pos": [120.0, 32.0],
            "chunk_id": [0, 0],
            "cost": { "iron": 2, "wood": 1 },
        }),
    );
    assert!(v.is_err(), "non-enum tier must reject");
}

#[test]
fn val_m14f_004_brace_strut_placed_rejects_missing_cost_wood() {
    let v = validate_event_payload(
        "terrain",
        "brace_strut_placed",
        &json!({
            "actor_id": 1,
            "tier": "t1",
            "world_pos": [120.0, 32.0],
            "chunk_id": [0, 0],
            // missing wood key
            "cost": { "iron": 2 },
        }),
    );
    assert!(v.is_err(), "missing wood in cost must reject");
}

/// VAL-M14F-019 + VAL-M14F-027: schema validation roundtrip on a
/// payload constructed via the cf-terrain wall_collapse module —
/// VAL-M14F-027 surface assertion.
#[test]
fn val_m14f_027_terrain_payload_validates_against_wall_rupture_schema() {
    let payload = cf_terrain::WallCollapsePayload::rupture((1, 0), [256, 60], [288, 200], 32, 4, 1.0);
    let body = json!({
        "chunk_id": [payload.chunk_id.0, payload.chunk_id.1],
        "bbox": { "min": payload.bbox_min, "max": payload.bbox_max },
        "falling_debris_count": payload.falling_debris_count,
        "unsupported_span_px": payload.unsupported_span_px,
        "lateral_yield_strength": 50,
        "vibration_modifier": payload.vibration_modifier,
        "cascade_primary": payload.cascade_primary,
        "trigger": "integrity_decay",
    });
    let v = validate_event_payload("terrain", "wall_rupture", &body);
    assert!(v.is_ok(), "wall_collapse::WallCollapsePayload must satisfy the schema: {v:?}");
}
