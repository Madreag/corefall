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
    let schema: RawSchema = serde_json::from_str(raw)
        .map_err(|e| format!("schema parse error for {category}.{event_type}: {e}"))?;
    let obj = payload
        .as_object()
        .ok_or_else(|| format!("payload for {category}.{event_type} must be an object"))?;
    for req in &schema.required {
        if !obj.contains_key(req) {
            return Err(format!(
                "{category}.{event_type}: required field `{req}` missing"
            ));
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
                    return Err(format!(
                        "{category}.{event_type}::{key} value {n} < minimum {min}"
                    ));
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
        ] {
            let raw = event_schema_for(cat, ty).unwrap_or_else(|| panic!("no schema for {cat}.{ty}"));
            let _parsed: RawSchema = serde_json::from_str(raw)
                .unwrap_or_else(|e| panic!("schema parse error for {cat}.{ty}: {e}"));
        }
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
