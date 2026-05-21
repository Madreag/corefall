use std::{fs, path::Path};

use crate::report::ValidationReport;

/// **M13**: validate the `content/equipment/roles.json` mod-tooling export.
/// Authoritative roles live in `cf_equipment::role_records()`; the JSON file
/// is presentation-only. Validation only checks well-formed JSON with a
/// top-level `schema_version` integer + a `roles` array.
pub(crate) fn validate_roles_json(path: &Path, report: &mut ValidationReport) {
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
    let Some(obj) = value.as_object() else {
        report.add_error(path.to_path_buf(), "roles.json must be a JSON object".to_string());
        return;
    };
    if obj.get("schema_version").and_then(|v| v.as_u64()).is_none() {
        report.add_error(
            path.to_path_buf(),
            "roles.json must declare an integer schema_version".to_string(),
        );
        return;
    }
    let Some(roles) = obj.get("roles").and_then(|v| v.as_array()) else {
        report.add_error(path.to_path_buf(), "roles.json must declare a `roles` array".to_string());
        return;
    };
    let mut unknown: Vec<String> = Vec::new();
    for entry in roles {
        if let Some(id) = entry.get("id").and_then(|v| v.as_str()) {
            if cf_equipment::role_record(id).is_none() {
                unknown.push(id.to_string());
            }
        }
    }
    if !unknown.is_empty() {
        report.add_warn(
            path.to_path_buf(),
            format!(
                "roles.json references {} role id(s) not present in cf_equipment::role_records(): {}",
                unknown.len(),
                unknown.join(", ")
            ),
        );
    }
    report.add_pass(path.to_path_buf(), format!("roles.json ({} entries)", roles.len()));
}
