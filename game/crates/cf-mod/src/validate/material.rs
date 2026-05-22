use std::{fs, path::Path};

use crate::report::ValidationReport;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::ValidationReport;
    use crate::test_helpers::write_tmp;

    fn launch_set_with_m15c() -> serde_json::Value {
        serde_json::json!({
            "schema_version": 1,
            "materials": [
                {"id": 0, "name": "air", "display_name": "Air", "hardness": 0.0, "diggable": false, "anchorable": false, "hazard": false, "path_cost": 1.0, "density": 0.0, "color_hex": "000000", "description": "Empty",
                 "state": "gas", "density_kg_per_m3": 1.225, "specific_heat_capacity_j_per_kg_k": 1005.0, "thermal_conductivity_w_per_m_k": 0.026},
                {"id": 1, "name": "dirt", "display_name": "Dirt", "hardness": 10.0, "diggable": true, "anchorable": true, "hazard": false, "path_cost": 1.0, "density": 1.5, "color_hex": "8B6914", "description": "Dirt",
                 "state": "solid", "density_kg_per_m3": 1500.0, "specific_heat_capacity_j_per_kg_k": 800.0, "thermal_conductivity_w_per_m_k": 0.5},
                {"id": 2, "name": "concrete", "display_name": "Concrete", "hardness": 40.0, "diggable": true, "anchorable": true, "hazard": false, "path_cost": 1.0, "density": 2.3, "color_hex": "808080", "description": "Concrete",
                 "state": "solid", "density_kg_per_m3": 2300.0, "specific_heat_capacity_j_per_kg_k": 880.0, "thermal_conductivity_w_per_m_k": 1.7},
                {"id": 3, "name": "metal_nohook", "display_name": "Metal", "hardness": 100.0, "diggable": false, "anchorable": false, "hazard": false, "path_cost": 999.0, "density": 7.8, "color_hex": "4A4A4A", "description": "Metal",
                 "state": "solid", "density_kg_per_m3": 7800.0, "specific_heat_capacity_j_per_kg_k": 466.0, "thermal_conductivity_w_per_m_k": 50.0},
                {"id": 4, "name": "hazard", "display_name": "Hazard", "hardness": 50.0, "diggable": false, "anchorable": false, "hazard": true, "path_cost": 10.0, "density": 3.0, "color_hex": "FF4444", "description": "Hazard",
                 "state": "solid", "density_kg_per_m3": 3000.0, "specific_heat_capacity_j_per_kg_k": 700.0, "thermal_conductivity_w_per_m_k": 1.0},
                {"id": 5, "name": "loose_fill", "display_name": "Loose Rubble", "hardness": 5.0, "diggable": true, "anchorable": false, "hazard": false, "path_cost": 2.0, "density": 1.2, "color_hex": "C8A864", "description": "Loose",
                 "state": "powder", "density_kg_per_m3": 1200.0, "specific_heat_capacity_j_per_kg_k": 800.0, "thermal_conductivity_w_per_m_k": 0.4},
                {"id": 6, "name": "repair_fill", "display_name": "Repair", "hardness": 15.0, "diggable": true, "anchorable": true, "hazard": false, "path_cost": 1.0, "density": 0.8, "color_hex": "44FF44", "description": "Repair",
                 "state": "solid", "density_kg_per_m3": 800.0, "specific_heat_capacity_j_per_kg_k": 1500.0, "thermal_conductivity_w_per_m_k": 0.05},
                {"id": 7, "name": "anchor", "display_name": "Anchor", "hardness": 60.0, "diggable": false, "anchorable": true, "hazard": false, "path_cost": 1.0, "density": 2.6, "color_hex": "6B4226", "description": "Anchor",
                 "state": "solid", "density_kg_per_m3": 2600.0, "specific_heat_capacity_j_per_kg_k": 790.0, "thermal_conductivity_w_per_m_k": 2.5}
            ]
        })
    }

    #[test]
    fn material_registry_accepts_valid_registry() {
        let body = launch_set_with_m15c();
        let path = write_tmp("materials_pass.json", &body.to_string());
        let mut report = ValidationReport::default();
        validate_material_registry(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.pass(), 1);
        assert_eq!(report.fail(), 0);
    }

    #[test]
    fn material_registry_rejects_unknown_field() {
        let mut body = launch_set_with_m15c();
        body["materials"][1]["rainbow_color"] = serde_json::json!("red");
        let path = write_tmp("materials_unknown.json", &body.to_string());
        let mut report = ValidationReport::default();
        validate_material_registry(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("unknown_field"));
        assert!(report.entries[0].message.contains("rainbow_color"));
    }

    /// schema-required `specific_heat_capacity_j_per_kg_k` field on a
    /// material entry fails validation with the spec-mandated literal
    /// error message.
    #[test]
    fn material_registry_rejects_missing_specific_heat_capacity_j_per_kg_k() {
        let mut body = launch_set_with_m15c();
        body["materials"][1]
            .as_object_mut()
            .unwrap()
            .remove("specific_heat_capacity_j_per_kg_k");
        let path = write_tmp("materials_missing_cp.json", &body.to_string());
        let mut report = ValidationReport::default();
        validate_material_registry(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        let msg = &report.entries[0].message;
        assert!(
            msg.contains("specific_heat_capacity_j_per_kg_k"),
            "expected `specific_heat_capacity_j_per_kg_k` in error, got: {msg}"
        );
        assert!(
            msg.contains("required") || msg.contains("missing_required_field"),
            "expected `required` in error, got: {msg}"
        );
    }

    /// when the M15C `state` field is omitted.
    #[test]
    fn material_registry_rejects_missing_state() {
        let mut body = launch_set_with_m15c();
        body["materials"][1].as_object_mut().unwrap().remove("state");
        let path = write_tmp("materials_missing_state.json", &body.to_string());
        let mut report = ValidationReport::default();
        validate_material_registry(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("state"));
    }

    /// non-default value (zero is "not authored", flagged as `> 0` failure).
    #[test]
    fn material_registry_rejects_zero_density_kg_per_m3() {
        let mut body = launch_set_with_m15c();
        body["materials"][1]["density_kg_per_m3"] = serde_json::json!(0.0);
        let path = write_tmp("materials_zero_density.json", &body.to_string());
        let mut report = ValidationReport::default();
        validate_material_registry(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
    }

    #[test]
    fn material_registry_rejects_schema_version_mismatch() {
        let body = serde_json::json!({
            "schema_version": 42,
            "materials": []
        });
        let path = write_tmp("materials_schema_drift.json", &body.to_string());
        let mut report = ValidationReport::default();
        validate_material_registry(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("schema_version_mismatch"));
    }
}
