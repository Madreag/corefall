use std::{fs, path::Path};

use crate::report::ValidationReport;

/// **M5**: identifies a per-event schema file living at
/// `<.../schemas/event/<family>_<type>.json>`. Used by `walk()` to pick the
/// file up and by `validate_one()` to route it to `validate_event_schema_file`.
pub(crate) fn is_event_schema_file(path: &Path) -> bool {
    if path.extension().and_then(|s| s.to_str()) != Some("json") {
        return false;
    }
    let Some(parent) = path.parent() else {
        return false;
    };
    if parent.file_name().and_then(|s| s.to_str()) != Some("event") {
        return false;
    }
    parent.parent().and_then(|gp| gp.file_name()).and_then(|s| s.to_str()) == Some("schemas")
}

/// **M5**: identifies an envelope schema file under
/// `<.../schemas/v0_1/*.schema.json>` or `<.../schemas/v1/*.schema.json>`.
pub(crate) fn is_envelope_schema_file(path: &Path) -> bool {
    if path.extension().and_then(|s| s.to_str()) != Some("json") {
        return false;
    }
    let Some(parent) = path.parent() else {
        return false;
    };
    let Some(parent_name) = parent.file_name().and_then(|s| s.to_str()) else {
        return false;
    };
    if !is_envelope_version_dir(parent_name) {
        return false;
    }
    parent.parent().and_then(|gp| gp.file_name()).and_then(|s| s.to_str()) == Some("schemas")
}

/// **M5-A1**: matches a version-suffixed envelope directory like `v0_1`,
/// `v1`, `v0_2`, `v2_5`. Strictly: `^v[0-9]+(_[0-9]+)?$`. Widens the legacy
/// `v0_1`/`v1` literal match so future M4 envelope-bump migration directories
/// (BP6+) are picked up automatically.
pub(crate) fn is_envelope_version_dir(name: &str) -> bool {
    let Some(rest) = name.strip_prefix('v') else {
        return false;
    };
    if rest.is_empty() {
        return false;
    }
    let mut seen_underscore = false;
    let mut current_segment_has_digit = false;
    for ch in rest.chars() {
        if ch == '_' {
            if seen_underscore || !current_segment_has_digit {
                return false;
            }
            seen_underscore = true;
            current_segment_has_digit = false;
        } else if ch.is_ascii_digit() {
            current_segment_has_digit = true;
        } else {
            return false;
        }
    }
    current_segment_has_digit
}

/// **M5**: validate a per-event JSON schema file under
/// `cf-replay/schemas/event/`. Two shapes are accepted:
///
/// 1. **M5 envelope-shaped schemas** (new at M5): MUST declare
///    `properties.schema_version.const = "0.1"`, `properties.category.const`
///    matching the filename family, `properties.event_type.const` matching
///    the filename suffix, `properties.payload` as an object sub-schema,
///    and top-level `required` containing the standard envelope fields
///    `schema_version`, `category`, `event_type`, `tick`, `payload`.
///    Verified against the spec scenario "each schema declares
///    `schema_version=\"0.1\"` matching the M4 locked envelope".
///
/// 2. **Legacy payload-only schemas** (M2-M4): the file pre-dates M5 and
///    describes the payload object directly without an envelope wrapper.
///    Only well-formed-JSON + presence of a `type` or `properties` field
///    is required.
pub(crate) fn validate_event_schema_file(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let value: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("json parse failed: {err}"));
            return;
        }
    };
    let messages = validate_event_schema_value(path, &value);
    if messages.is_empty() {
        let shape = if value.pointer("/properties/schema_version/const").is_some() {
            "M5 envelope-shape"
        } else {
            "legacy payload-only"
        };
        report.add_pass(path.to_path_buf(), format!("event schema ({shape})"));
    } else {
        report.add_error(path.to_path_buf(), messages.join("; "));
    }
}

/// Pure-function half of `validate_event_schema_file` so tests can drive it
/// directly. Returns the empty Vec on success and a list of human-readable
/// error messages on failure.
pub(crate) fn validate_event_schema_value(path: &Path, value: &serde_json::Value) -> Vec<String> {
    let mut messages: Vec<String> = Vec::new();
    let Some(obj) = value.as_object() else {
        messages.push("schema must be a JSON object".to_string());
        return messages;
    };
    match obj.get("type").and_then(|v| v.as_str()) {
        Some("object") => {}
        Some(other) => messages.push(format!("schema.type must be \"object\" (got {other})")),
        None => {
            if value.pointer("/properties/schema_version/const").is_some() {
                messages.push("schema.type missing (M5 envelope shape requires type=object)".to_string());
            }
        }
    }
    let is_m5 = value.pointer("/properties/schema_version/const").is_some();
    if !is_m5 {
        if obj.get("properties").is_none() && obj.get("type").is_none() {
            messages.push("legacy schema must define either `properties` or `type`".to_string());
        }
        return messages;
    }
    let sv = value
        .pointer("/properties/schema_version/const")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if sv != "prototype-recorder-event.v0.1" {
        messages.push(format!(
            "properties.schema_version.const must be \"prototype-recorder-event.v0.1\" (got \"{sv}\")"
        ));
    }
    let file_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string();
    let title = value.get("title").and_then(|v| v.as_str()).unwrap_or_default();
    if title.is_empty() {
        messages.push("title must be set to \"<family>.<event_type>\"".to_string());
    } else if !title.contains('.') {
        messages.push(format!("title \"{title}\" must contain a `.` separator"));
    }
    let cat_const = value
        .pointer("/properties/category/const")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if cat_const.is_empty() {
        messages.push("properties.category.const must be set".to_string());
    } else if !file_stem.starts_with(&format!("{cat_const}_")) && file_stem != cat_const {
        messages.push(format!(
            "filename `{file_stem}` does not start with category `{cat_const}_`"
        ));
    }
    let ty_const = value
        .pointer("/properties/event_type/const")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if ty_const.is_empty() {
        messages.push("properties.event_type.const must be set".to_string());
    } else {
        let expected_stem = format!("{cat_const}_{ty_const}");
        if file_stem != expected_stem {
            messages.push(format!(
                "filename `{file_stem}` does not match expected `{expected_stem}` (from category+event_type consts)"
            ));
        }
    }
    if !title.is_empty() && !cat_const.is_empty() && !ty_const.is_empty() {
        let expected_title = format!("{cat_const}.{ty_const}");
        if title != expected_title {
            messages.push(format!(
                "title `{title}` must equal `{expected_title}` (from category+event_type consts)"
            ));
        }
    }
    let payload_schema = value.pointer("/properties/payload");
    match payload_schema {
        Some(serde_json::Value::Object(p)) => {
            if let Some(ty) = p.get("type").and_then(|v| v.as_str()) {
                if ty != "object" {
                    messages.push(format!("properties.payload.type must be \"object\" (got \"{ty}\")"));
                }
            } else {
                messages.push("properties.payload.type must be set to \"object\"".to_string());
            }
            if let Some(serde_json::Value::Bool(false)) = p.get("additionalProperties") {
                messages.push(
                    "properties.payload.additionalProperties must NOT be `false` — M4 envelope is additive-only per DR-002; future producers must be able to add fields without an envelope bump"
                        .to_string(),
                );
            }
        }
        Some(_) => messages.push("properties.payload must be a JSON object schema".to_string()),
        None => messages.push("properties.payload must be defined".to_string()),
    }
    let req: Vec<&str> = value
        .get("required")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str()).collect())
        .unwrap_or_default();
    for must in ["schema_version", "category", "event_type", "tick", "payload"] {
        if !req.contains(&must) {
            messages.push(format!("top-level `required` must include `{must}` (got {req:?})"));
        }
    }
    if value.pointer("/properties/tick").is_none() {
        messages.push("properties.tick must be declared".to_string());
    }
    messages
}

/// **M5**: validate an envelope schema file under
/// `cf-replay/schemas/v0_1/` or `cf-replay/schemas/v1/`. These pre-date M5 —
/// the validator just confirms well-formed JSON.
pub(crate) fn validate_envelope_schema_file(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let value: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("json parse failed: {err}"));
            return;
        }
    };
    if !value.is_object() {
        report.add_error(path.to_path_buf(), "schema must be a JSON object".to_string());
        return;
    }
    let title = value.get("title").and_then(|v| v.as_str()).unwrap_or("(no title)");
    report.add_pass(path.to_path_buf(), format!("envelope schema: {title}"));
}
