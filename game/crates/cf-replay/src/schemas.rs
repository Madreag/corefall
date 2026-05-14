//! **M1 Gap H**: per-event JSON schema validation for the prototype-recorder
//! event payloads.
//!
//! The full schemas live under `cf-replay/schemas/event/` (one JSON file per
//! `(category, event_type)` pair). The validator here is intentionally a
//! minimal "required field present + type matches" check rather than a
//! full draft-2020-12 implementation: pulling in a JSON Schema crate just
//! to assert payload shapes would balloon the dependency surface for a
//! benefit M1 doesn't need. The validator handles:
//!
//! - `required` array (every listed field MUST exist in the payload).
//! - per-field `type` (`object`, `array`, `string`, `number`, `integer`,
//!   `boolean`; arrays of types are interpreted as a union).
//! - `minItems` + `maxItems` on arrays.
//! - `minimum` on numeric values.
//! - `enum` on strings.
//!
//! `additionalProperties: true` is implicit — payloads may carry extra
//! fields beyond the schema without rejection (the recorder envelope is
//! intentionally extensible).
//!
//! `cf-mod validate-bundle` calls `validate_event_payload` on every event
//! in a run bundle; the workspace test under `cf-replay/tests` walks a
//! freshly-recorded smoke bundle to prove the schemas accept real events.

use serde::Deserialize;
use serde_json::Value;

const SCHEMA_INPUT_INTENT_RECEIVED: &str = include_str!("../schemas/event/input_intent_received.json");
const SCHEMA_WEAPON_FIRED: &str = include_str!("../schemas/event/weapon_fired.json");
const SCHEMA_PROJECTILE_SPAWNED: &str = include_str!("../schemas/event/projectile_spawned.json");
const SCHEMA_WOUND_ADDED: &str = include_str!("../schemas/event/wound_added.json");
const SCHEMA_INVENTORY_DROPPED: &str = include_str!("../schemas/event/inventory_dropped.json");
const SCHEMA_ALARM_REGISTERED: &str = include_str!("../schemas/event/alarm_registered.json");
const SCHEMA_TERRAIN_CARVED: &str = include_str!("../schemas/event/terrain_carved.json");
const SCHEMA_TERRAIN_PENETRATION_THRESHOLD: &str = include_str!("../schemas/event/terrain_penetration_threshold.json");
const SCHEMA_TERRAIN_DIRTY_REGION_BATCH: &str = include_str!("../schemas/event/terrain_dirty_region_batch.json");
const SCHEMA_TERRAIN_PIXEL_DISLODGED: &str = include_str!("../schemas/event/terrain_pixel_dislodged.json");
const SCHEMA_HAZARD_CONTACT_OR_AVOIDANCE: &str = include_str!("../schemas/event/hazard_contact_or_avoidance.json");
const SCHEMA_ANCHOR_MATERIAL_RESULT: &str = include_str!("../schemas/event/anchor_material_result.json");
const SCHEMA_TERRAIN_MATERIAL_PROBE: &str = include_str!("../schemas/event/terrain_material_probe.json");
const SCHEMA_TERRAIN_FILL_OR_REPAIR: &str = include_str!("../schemas/event/terrain_fill_or_repair.json");
const SCHEMA_FORCED_REFRESH_REQUESTED: &str = include_str!("../schemas/event/forced_refresh_requested.json");
// M3 audit pass 7 (2026-05-13): schemas for terrain.* events that were
// previously recorded without a schema.
const SCHEMA_TERRAIN_DEBRIS_CAPPED: &str = include_str!("../schemas/event/debris_capped.json");
const SCHEMA_TERRAIN_TOOL_REFUSED: &str = include_str!("../schemas/event/tool_refused.json");
const SCHEMA_TERRAIN_TOOL_ACTION_STARTED: &str = include_str!("../schemas/event/tool_action_started.json");
const SCHEMA_EQUIPMENT_TOOL_ACTION_COMPLETED: &str = include_str!("../schemas/event/tool_action_completed.json");
const SCHEMA_TERRAIN_PATH_INVALIDATED: &str = include_str!("../schemas/event/path_invalidated.json");
// M2 re-audit (2026-05-13): mission + AI event schemas the spec lists in
// "## Files" / "## Crates / modules touched" but were never created.
const SCHEMA_MISSION_STARTED: &str = include_str!("../schemas/event/mission_started.json");
const SCHEMA_OBJECTIVE_STARTED: &str = include_str!("../schemas/event/objective_started.json");
const SCHEMA_OBJECTIVE_UPDATED: &str = include_str!("../schemas/event/objective_updated.json");
const SCHEMA_OBJECTIVE_COMPLETED: &str = include_str!("../schemas/event/objective_completed.json");
const SCHEMA_OBJECTIVE_FAILED: &str = include_str!("../schemas/event/objective_failed.json");
const SCHEMA_MISSION_RESOLVED: &str = include_str!("../schemas/event/mission_resolved.json");
const SCHEMA_AI_STATE_CHANGED: &str = include_str!("../schemas/event/ai_state_changed.json");
const SCHEMA_AI_PERCEPTION_SIGNAL: &str = include_str!("../schemas/event/ai_perception_signal.json");
const SCHEMA_AI_TACTIC_CHOSEN: &str = include_str!("../schemas/event/ai_tactic_chosen.json");
const SCHEMA_AI_MISSED_SHOT_REASON: &str = include_str!("../schemas/event/ai_missed_shot_reason.json");
const SCHEMA_AI_STUCK_STATE_CHANGED: &str = include_str!("../schemas/event/ai_stuck_state_changed.json");
const SCHEMA_AI_RECOVERY_ACTION: &str = include_str!("../schemas/event/ai_recovery_action.json");
// M4 (2026-05-13): system/determinism/snapshot event schemas locked at v0.1.
const SCHEMA_SYSTEM_RUN_STARTED: &str = include_str!("../schemas/event/system_run_started.json");
const SCHEMA_SYSTEM_RUN_FINISHED: &str = include_str!("../schemas/event/system_run_finished.json");
const SCHEMA_SYSTEM_CATEGORY_BASELINE: &str = include_str!("../schemas/event/system_category_baseline.json");
const SCHEMA_DETERMINISM_SIM_CHECKSUM: &str = include_str!("../schemas/event/determinism_sim_checksum.json");
const SCHEMA_DETERMINISM_FIRST_DIVERGENCE: &str = include_str!("../schemas/event/determinism_first_divergence.json");
const SCHEMA_SNAPSHOT_ACTOR: &str = include_str!("../schemas/event/snapshot_actor.json");
const SCHEMA_SNAPSHOT_INVENTORY: &str = include_str!("../schemas/event/snapshot_inventory.json");
const SCHEMA_SNAPSHOT_TERRAIN_CHUNK: &str = include_str!("../schemas/event/snapshot_terrain_chunk.json");
const SCHEMA_SNAPSHOT_TERRAIN_SUMMARY: &str = include_str!("../schemas/event/snapshot_terrain_summary.json");
const SCHEMA_SNAPSHOT_CHASSIS: &str = include_str!("../schemas/event/snapshot_chassis.json");

/// Look up the schema source by `(category, event_type)`. Returns `None` if
/// no schema exists for this pair (callers treat as "no validation
/// constraint"; the recorder envelope itself is checked by the bundle
/// checker separately).
pub fn event_schema_for(category: &str, event_type: &str) -> Option<&'static str> {
    match (category, event_type) {
        ("input", "intent_received") => Some(SCHEMA_INPUT_INTENT_RECEIVED),
        ("equipment", "weapon_fired") => Some(SCHEMA_WEAPON_FIRED),
        ("combat", "projectile_spawned") => Some(SCHEMA_PROJECTILE_SPAWNED),
        ("combat", "wound_added") => Some(SCHEMA_WOUND_ADDED),
        ("actor", "inventory_dropped") => Some(SCHEMA_INVENTORY_DROPPED),
        ("equipment", "alarm_registered") => Some(SCHEMA_ALARM_REGISTERED),
        ("terrain", "terrain_carved") => Some(SCHEMA_TERRAIN_CARVED),
        ("terrain", "terrain_penetration_threshold") => Some(SCHEMA_TERRAIN_PENETRATION_THRESHOLD),
        ("terrain", "terrain_dirty_region_batch") => Some(SCHEMA_TERRAIN_DIRTY_REGION_BATCH),
        ("terrain", "terrain_pixel_dislodged") => Some(SCHEMA_TERRAIN_PIXEL_DISLODGED),
        ("terrain", "hazard_contact_or_avoidance") => Some(SCHEMA_HAZARD_CONTACT_OR_AVOIDANCE),
        ("terrain", "anchor_material_result") => Some(SCHEMA_ANCHOR_MATERIAL_RESULT),
        ("terrain", "terrain_material_probe") => Some(SCHEMA_TERRAIN_MATERIAL_PROBE),
        ("terrain", "terrain_fill_or_repair") => Some(SCHEMA_TERRAIN_FILL_OR_REPAIR),
        ("terrain", "forced_refresh_requested") => Some(SCHEMA_FORCED_REFRESH_REQUESTED),
        // M3 audit pass 7 (2026-05-13): newly-registered schemas.
        ("terrain", "debris_capped") => Some(SCHEMA_TERRAIN_DEBRIS_CAPPED),
        ("terrain", "tool_refused") => Some(SCHEMA_TERRAIN_TOOL_REFUSED),
        ("terrain", "tool_action_started") => Some(SCHEMA_TERRAIN_TOOL_ACTION_STARTED),
        ("equipment", "tool_action_completed") => Some(SCHEMA_EQUIPMENT_TOOL_ACTION_COMPLETED),
        ("terrain", "path_invalidated") => Some(SCHEMA_TERRAIN_PATH_INVALIDATED),
        // M2 re-audit (2026-05-13): mission + AI event schemas.
        ("mission", "mission_started") => Some(SCHEMA_MISSION_STARTED),
        ("mission", "objective_started") => Some(SCHEMA_OBJECTIVE_STARTED),
        ("mission", "objective_updated") => Some(SCHEMA_OBJECTIVE_UPDATED),
        ("mission", "objective_completed") => Some(SCHEMA_OBJECTIVE_COMPLETED),
        ("mission", "objective_failed") => Some(SCHEMA_OBJECTIVE_FAILED),
        ("mission", "mission_resolved") => Some(SCHEMA_MISSION_RESOLVED),
        ("ai", "state_changed") => Some(SCHEMA_AI_STATE_CHANGED),
        ("ai", "perception_signal") => Some(SCHEMA_AI_PERCEPTION_SIGNAL),
        ("ai", "tactic_chosen") => Some(SCHEMA_AI_TACTIC_CHOSEN),
        ("ai", "missed_shot_reason") => Some(SCHEMA_AI_MISSED_SHOT_REASON),
        ("ai", "stuck_state_changed") => Some(SCHEMA_AI_STUCK_STATE_CHANGED),
        ("ai", "recovery_action") => Some(SCHEMA_AI_RECOVERY_ACTION),
        // M4: system/determinism/snapshot event schemas locked at v0.1.
        ("system", "run_started") => Some(SCHEMA_SYSTEM_RUN_STARTED),
        ("system", "run_finished") => Some(SCHEMA_SYSTEM_RUN_FINISHED),
        ("system", "category_baseline") => Some(SCHEMA_SYSTEM_CATEGORY_BASELINE),
        ("determinism", "sim_checksum") => Some(SCHEMA_DETERMINISM_SIM_CHECKSUM),
        ("determinism", "first_divergence") => Some(SCHEMA_DETERMINISM_FIRST_DIVERGENCE),
        ("snapshot", "snapshot_actor") => Some(SCHEMA_SNAPSHOT_ACTOR),
        ("snapshot", "snapshot_inventory") => Some(SCHEMA_SNAPSHOT_INVENTORY),
        ("snapshot", "snapshot_terrain_chunk") => Some(SCHEMA_SNAPSHOT_TERRAIN_CHUNK),
        ("snapshot", "snapshot_terrain_summary") => Some(SCHEMA_SNAPSHOT_TERRAIN_SUMMARY),
        ("snapshot", "snapshot_chassis") => Some(SCHEMA_SNAPSHOT_CHASSIS),
        _ => None,
    }
}

/// Result of a schema validation.
pub type ValidationResult = Result<(), String>;

#[derive(Deserialize)]
struct RawSchema {
    #[serde(default)]
    required: Vec<String>,
    #[serde(default)]
    properties: serde_json::Map<String, Value>,
}

#[derive(Deserialize)]
struct PropConstraint {
    #[serde(default, rename = "type")]
    ty: Option<Value>,
    #[serde(default, rename = "enum")]
    enum_values: Option<Vec<Value>>,
    #[serde(default, rename = "minItems")]
    min_items: Option<usize>,
    #[serde(default, rename = "maxItems")]
    max_items: Option<usize>,
    #[serde(default)]
    minimum: Option<f64>,
}

/// Validate that `payload` matches the schema registered for
/// `(category, event_type)`. Returns `Ok(())` when there is no registered
/// schema or the payload satisfies the schema's required-field + type +
/// range constraints.
pub fn validate_event_payload(category: &str, event_type: &str, payload: &Value) -> ValidationResult {
    let Some(raw) = event_schema_for(category, event_type) else {
        return Ok(());
    };
    let schema: RawSchema =
        serde_json::from_str(raw).map_err(|e| format!("schema parse error for {category}.{event_type}: {e}"))?;
    let obj = payload
        .as_object()
        .ok_or_else(|| format!("payload for {category}.{event_type} must be an object"))?;
    for req in &schema.required {
        if !obj.contains_key(req) {
            return Err(format!("{category}.{event_type}: required field `{req}` missing"));
        }
    }
    for (key, raw_constraint) in &schema.properties {
        let Some(value) = obj.get(key) else {
            continue;
        };
        let constraint: PropConstraint = serde_json::from_value(raw_constraint.clone())
            .map_err(|e| format!("{category}.{event_type}::{key} constraint parse error: {e}"))?;
        if let Some(ty) = &constraint.ty {
            check_type(category, event_type, key, ty, value)?;
        }
        if let Some(enum_values) = &constraint.enum_values {
            if !enum_values.contains(value) {
                return Err(format!(
                    "{category}.{event_type}::{key} value {value} not in enum {enum_values:?}"
                ));
            }
        }
        if let Some(min_items) = constraint.min_items {
            if let Some(arr) = value.as_array() {
                if arr.len() < min_items {
                    return Err(format!(
                        "{category}.{event_type}::{key} array length {} < minItems {}",
                        arr.len(),
                        min_items
                    ));
                }
            }
        }
        if let Some(max_items) = constraint.max_items {
            if let Some(arr) = value.as_array() {
                if arr.len() > max_items {
                    return Err(format!(
                        "{category}.{event_type}::{key} array length {} > maxItems {}",
                        arr.len(),
                        max_items
                    ));
                }
            }
        }
        if let Some(min) = constraint.minimum {
            if let Some(n) = value.as_f64() {
                if n < min {
                    return Err(format!("{category}.{event_type}::{key} value {n} < minimum {min}"));
                }
            }
        }
    }
    Ok(())
}

fn check_type(category: &str, event_type: &str, key: &str, ty: &Value, value: &Value) -> ValidationResult {
    let types: Vec<&str> = match ty {
        Value::String(s) => vec![s.as_str()],
        Value::Array(arr) => arr.iter().filter_map(|v| v.as_str()).collect(),
        _ => Vec::new(),
    };
    let matches = types.iter().any(|t| match *t {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "number" => value.is_f64() || value.is_i64() || value.is_u64(),
        "integer" => value.is_i64() || value.is_u64(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        _ => true,
    });
    if !matches {
        return Err(format!(
            "{category}.{event_type}::{key} expected type {types:?}, got {value}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn schemas_load_for_every_registered_event_type() {
        for (cat, ty) in [
            ("input", "intent_received"),
            ("equipment", "weapon_fired"),
            ("combat", "projectile_spawned"),
            ("combat", "wound_added"),
            ("actor", "inventory_dropped"),
            ("equipment", "alarm_registered"),
            ("terrain", "terrain_carved"),
            ("terrain", "terrain_penetration_threshold"),
            ("terrain", "terrain_dirty_region_batch"),
            ("terrain", "terrain_pixel_dislodged"),
            ("terrain", "hazard_contact_or_avoidance"),
            ("terrain", "anchor_material_result"),
            ("terrain", "terrain_material_probe"),
            ("terrain", "terrain_fill_or_repair"),
            // M4 schemas
            ("system", "run_started"),
            ("system", "run_finished"),
            ("system", "category_baseline"),
            ("determinism", "sim_checksum"),
            ("determinism", "first_divergence"),
            ("snapshot", "snapshot_actor"),
            ("snapshot", "snapshot_inventory"),
            ("snapshot", "snapshot_terrain_chunk"),
            ("snapshot", "snapshot_terrain_summary"),
            ("snapshot", "snapshot_chassis"),
        ] {
            let raw = event_schema_for(cat, ty).unwrap_or_else(|| panic!("no schema for {cat}.{ty}"));
            let _parsed: RawSchema =
                serde_json::from_str(raw).unwrap_or_else(|e| panic!("schema parse error for {cat}.{ty}: {e}"));
        }
    }

    #[test]
    fn terrain_carved_event_validates_minimum_payload() {
        let payload = json!({
            "bbox": { "min": [0, 0], "max": [10, 10] },
            "count": 12u32,
            "removed_count": 12u32,
            "debris_count": 12u32,
            "material_ids": [1u32],
        });
        validate_event_payload("terrain", "terrain_carved", &payload).expect("valid payload");
    }

    #[test]
    fn terrain_penetration_threshold_event_validates() {
        let payload = json!({
            "projectile_id": 7u32,
            "material_id": 1u32,
            "passed": true,
            "impulse_squared": 256.0,
            "integrity_squared": 100.0,
        });
        validate_event_payload("terrain", "terrain_penetration_threshold", &payload).expect("valid");
    }

    #[test]
    fn unknown_event_type_is_ok_by_default() {
        let payload = json!({});
        assert!(validate_event_payload("not", "registered", &payload).is_ok());
    }

    #[test]
    fn validates_input_intent_received_required_fields() {
        let mut payload = json!({
            "actor": 1,
            "source": "cfctl",
            "move_x": 0.0,
            "aim_x": 1.0,
            "aim_y": 0.0,
            "jump": false,
            "fire": false,
            "reload": false,
        });
        assert!(validate_event_payload("input", "intent_received", &payload).is_ok());
        payload.as_object_mut().unwrap().remove("actor");
        let err = validate_event_payload("input", "intent_received", &payload).unwrap_err();
        assert!(err.contains("`actor` missing"), "got: {err}");
    }

    #[test]
    fn validates_projectile_spawned_array_arity() {
        let bad = json!({
            "id": 1,
            "owner": 2,
            "origin": [0.0],
            "velocity": [1.0, 0.0],
            "damage": 12.0,
        });
        let err = validate_event_payload("combat", "projectile_spawned", &bad).unwrap_err();
        assert!(err.contains("origin"), "got: {err}");
    }
}
