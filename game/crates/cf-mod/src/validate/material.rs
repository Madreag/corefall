use std::{fs, path::Path};

use crate::report::ValidationReport;

/// **M2**: validate a material registry JSON file (`content/materials/*.json`)
/// against the v1 schema. Aggregates `RegistryValidationError`s from the
/// cf-material loader and reports each as a FAIL entry with the structured
/// `kind` so cfctl + CI can pattern-match on `unknown_field`, `duplicate_id`,
/// `schema_version_mismatch`, `missing_required_field`, etc.
pub(crate) fn validate_material_registry(path: &Path, report: &mut ValidationReport) {
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
    let result = cf_material::validate_registry_json(&value);
    if result.errors.is_empty() {
        let summary = format!(
            "materials registry ({} materials; {} warning(s))",
            result.material_count,
            result.warnings.len()
        );
        report.add_pass(path.to_path_buf(), summary);
    } else {
        let messages: Vec<String> = result
            .errors
            .iter()
            .map(|e| format!("{} @ {}: {} [{}]", e.kind, e.path, e.message, e.hint))
            .collect();
        report.add_error(path.to_path_buf(), messages.join("; "));
    }
    for w in &result.warnings {
        let msg = format!("{} @ {}: {} [{}]", w.kind, w.path, w.message, w.hint);
        report.add_warn(path.to_path_buf(), msg);
    }
}
