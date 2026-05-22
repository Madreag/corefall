//! Per-event JSON schema validator. Walks the schema source returned by
//! `crate::schemas_lookup::event_schema_for` against a payload value and
//! checks required fields + per-property type/enum/range constraints plus
//! the M5-A2 extensions for `oneOf`, `items`, and nested-object recursion.

use serde::Deserialize;
use serde_json::Value;

use crate::schemas_lookup::event_schema_for;

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
    #[serde(default)]
    maximum: Option<f64>,
    /// **M5-A1**: `oneOf` lets a property accept one of several alternative
    /// type/enum branches (e.g. `origin_id` accepts either an integer OR a
    /// canonical Origin-enum string). The minimal validator walks each
    /// branch and passes if ANY branch accepts the value.
    #[serde(default, rename = "oneOf")]
    one_of: Option<Vec<Value>>,
    /// **M5-A2**: `items` lets an array property constrain its items'
    /// type + enum (e.g. `applied_afflictions: array<affliction-kind-string>`).
    /// Only the simple form `items: { type, enum }` is honored — tuple form
    /// `items: [...]` is left unenforced (the projectile-spawned schemas
    /// use that form for fixed-arity tuples; their arity is already gated
    /// by `minItems`/`maxItems`).
    #[serde(default)]
    items: Option<Value>,
    /// **M5-A2**: `properties` + `required` on a nested object property
    /// (e.g. `signal: { properties: { schema_version, active_hazards }, required: [...] }`).
    /// The validator recurses into the nested object and enforces both.
    #[serde(default)]
    properties: Option<serde_json::Map<String, Value>>,
    #[serde(default)]
    required: Option<Vec<String>>,
}

/// Validate that `payload` matches the schema registered for
/// `(category, event_type)`. Returns `Ok(())` when there is no registered
/// schema or the payload satisfies the schema's required-field + type +
/// range constraints.
///
/// Supports two schema shapes:
/// 1. **Legacy payload-only** (M2/M3/M4 schemas in `schemas/event/`): the
///    schema describes the payload object directly; `required` /
///    `properties` apply to the payload value.
/// 2. **M5 envelope-shaped**: the schema describes the full event envelope
///    with `properties.schema_version.const = "prototype-recorder-event.v0.1"`,
///    `category` const, `event_type` const, and `payload` nested under
///    `properties.payload`. For these schemas the validator extracts the
///    `payload` sub-schema and validates the supplied `payload` argument
///    against it. The canonical literal matches `EVENT_SCHEMA_VERSION` in
///    `lib.rs` so producer events emitted by the recorder will satisfy
///    strict JSON Schema validators reading these schemas externally.
pub fn validate_event_payload(category: &str, event_type: &str, payload: &Value) -> ValidationResult {
    let Some(raw) = event_schema_for(category, event_type) else {
        return Ok(());
    };
    let full_value: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| format!("schema parse error for {category}.{event_type}: {e}"))?;
    let payload_schema_source: Value = if let Some(props) = full_value.get("properties").and_then(|v| v.as_object()) {
        let sv = props
            .get("schema_version")
            .and_then(|v| v.get("const"))
            .and_then(|v| v.as_str());
        if matches!(sv, Some("prototype-recorder-event.v0.1") | Some("0.1")) {
            props
                .get("payload")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({"type": "object"}))
        } else {
            full_value.clone()
        }
    } else {
        full_value.clone()
    };
    let schema: RawSchema = serde_json::from_value(payload_schema_source)
        .map_err(|e| format!("payload schema parse error for {category}.{event_type}: {e}"))?;
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
        if let Some(max) = constraint.maximum {
            if let Some(n) = value.as_f64() {
                if n > max {
                    return Err(format!("{category}.{event_type}::{key} value {n} > maximum {max}"));
                }
            }
        }
        if let Some(branches) = &constraint.one_of {
            let mut any_match = false;
            let mut branch_errors: Vec<String> = Vec::new();
            for (i, branch) in branches.iter().enumerate() {
                match check_one_of_branch(category, event_type, key, branch, value) {
                    Ok(()) => {
                        any_match = true;
                        break;
                    }
                    Err(e) => branch_errors.push(format!("branch[{i}]: {e}")),
                }
            }
            if !any_match {
                return Err(format!(
                    "{category}.{event_type}::{key} value {value} did not satisfy any oneOf branch — {}",
                    branch_errors.join("; ")
                ));
            }
        }
        if let Some(items_schema) = &constraint.items {
            if items_schema.is_object() {
                if let Some(arr) = value.as_array() {
                    for (i, item) in arr.iter().enumerate() {
                        check_array_item(category, event_type, key, i, items_schema, item)?;
                    }
                }
            }
        }
        if let Some(nested_props) = &constraint.properties {
            if let Some(obj) = value.as_object() {
                if let Some(nested_required) = &constraint.required {
                    for req in nested_required {
                        if !obj.contains_key(req) {
                            return Err(format!("{category}.{event_type}::{key}.{req} required field missing"));
                        }
                    }
                }
                for (nested_key, nested_raw) in nested_props {
                    let Some(nested_value) = obj.get(nested_key) else {
                        continue;
                    };
                    check_nested_property(category, event_type, key, nested_key, nested_raw, nested_value)?;
                }
            }
        }
    }
    Ok(())
}

/// **M5-A2**: validate one element of an array against the schema's
/// `items` constraint. Supports `type` (string or array union) and `enum`.
fn check_array_item(
    category: &str,
    event_type: &str,
    key: &str,
    index: usize,
    items_schema: &Value,
    item: &Value,
) -> ValidationResult {
    let items_obj = items_schema
        .as_object()
        .ok_or_else(|| format!("{category}.{event_type}::{key}.items must be a JSON object schema"))?;
    if let Some(ty) = items_obj.get("type") {
        let key_with_index = format!("{key}[{index}]");
        check_type(category, event_type, &key_with_index, ty, item)?;
    }
    if let Some(enum_values) = items_obj.get("enum").and_then(|v| v.as_array()) {
        if !enum_values.contains(item) {
            return Err(format!(
                "{category}.{event_type}::{key}[{index}] value {item} not in enum {enum_values:?}"
            ));
        }
    }
    Ok(())
}

/// **M5-A2**: validate a single nested-object property as a mini-schema
/// (type + enum + minimum + maximum). Used for sub-structs like
/// environment.signal_aggregated.signal.{schema_version,active_hazards}.
/// Supports recursion one level deep.
fn check_nested_property(
    category: &str,
    event_type: &str,
    parent_key: &str,
    nested_key: &str,
    nested_schema: &Value,
    value: &Value,
) -> ValidationResult {
    let nested_obj = nested_schema
        .as_object()
        .ok_or_else(|| format!("{category}.{event_type}::{parent_key}.{nested_key} schema must be an object"))?;
    let qualified_key = format!("{parent_key}.{nested_key}");
    if let Some(ty) = nested_obj.get("type") {
        check_type(category, event_type, &qualified_key, ty, value)?;
    }
    if let Some(enum_values) = nested_obj.get("enum").and_then(|v| v.as_array()) {
        if !enum_values.contains(value) {
            return Err(format!(
                "{category}.{event_type}::{qualified_key} value {value} not in enum {enum_values:?}"
            ));
        }
    }
    if let Some(min) = nested_obj.get("minimum").and_then(|v| v.as_f64()) {
        if let Some(n) = value.as_f64() {
            if n < min {
                return Err(format!(
                    "{category}.{event_type}::{qualified_key} value {n} < minimum {min}"
                ));
            }
        }
    }
    if let Some(max) = nested_obj.get("maximum").and_then(|v| v.as_f64()) {
        if let Some(n) = value.as_f64() {
            if n > max {
                return Err(format!(
                    "{category}.{event_type}::{qualified_key} value {n} > maximum {max}"
                ));
            }
        }
    }
    if let Some(items_schema) = nested_obj.get("items") {
        if items_schema.is_object() {
            if let Some(arr) = value.as_array() {
                for (i, item) in arr.iter().enumerate() {
                    check_array_item(category, event_type, &qualified_key, i, items_schema, item)?;
                }
            }
        }
    }
    Ok(())
}

/// **M5-A1**: validate a single `oneOf` branch as a mini-schema (type +
/// enum). Returns `Ok` if the value satisfies the branch. The minimal
/// validator only supports `type` and `enum` constraints inside `oneOf`
/// branches; richer JSON-Schema features inside `oneOf` are not needed by
/// any M5 schema today.
fn check_one_of_branch(category: &str, event_type: &str, key: &str, branch: &Value, value: &Value) -> ValidationResult {
    let branch_obj = branch
        .as_object()
        .ok_or_else(|| format!("oneOf branch for {category}.{event_type}::{key} must be an object"))?;
    if let Some(ty) = branch_obj.get("type") {
        check_type(category, event_type, key, ty, value)?;
    }
    if let Some(enum_values) = branch_obj.get("enum").and_then(|v| v.as_array()) {
        if !enum_values.contains(value) {
            return Err(format!(
                "{category}.{event_type}::{key} value {value} not in enum {enum_values:?}"
            ));
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
