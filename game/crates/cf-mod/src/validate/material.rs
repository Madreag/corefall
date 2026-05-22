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

    fn launch_entry(
        id: u16,
        name: &str,
        display_name: &str,
        hardness: f64,
        diggable: bool,
        anchorable: bool,
        hazard: bool,
        path_cost: f64,
        density: f64,
        color_hex: &str,
        state: &str,
        density_kg_per_m3: f64,
        cp: f64,
        k: f64,
    ) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "name": name,
            "display_name": display_name,
            "hardness": hardness,
            "diggable": diggable,
            "anchorable": anchorable,
            "hazard": hazard,
            "path_cost": path_cost,
            "density": density,
            "color_hex": color_hex,
            "description": display_name,
            "state": state,
            "density_kg_per_m3": density_kg_per_m3,
            "specific_heat_capacity_j_per_kg_k": cp,
            "thermal_conductivity_w_per_m_k": k,
            "molar_mass_g_per_mol": 0.0,
            "toxicity": 0.0,
            "corrosiveness": 0.0,
            "radioactivity": 0.0,
            "electrical_conductivity": 0.0,
            "viscosity_pa_s": 0.0,
            "surface_tension_n_per_m": 0.0,
            "default_mass_per_tile_kg": 0.0,
            "max_mass_per_tile_kg": 0.0
        })
    }

    fn launch_set_with_m15c() -> serde_json::Value {
        serde_json::json!({
            "schema_version": 1,
            "materials": [
                launch_entry(0, "air", "Air", 0.0, false, false, false, 1.0, 0.0, "000000", "gas", 1.225, 1005.0, 0.026),
                launch_entry(1, "dirt", "Dirt", 10.0, true, true, false, 1.0, 1.5, "8B6914", "solid", 1500.0, 800.0, 0.5),
                launch_entry(2, "concrete", "Concrete", 40.0, true, true, false, 1.0, 2.3, "808080", "solid", 2300.0, 880.0, 1.7),
                launch_entry(3, "metal_nohook", "Metal", 100.0, false, false, false, 999.0, 7.8, "4A4A4A", "solid", 7800.0, 466.0, 50.0),
                launch_entry(4, "hazard", "Hazard", 50.0, false, false, true, 10.0, 3.0, "FF4444", "solid", 3000.0, 700.0, 1.0),
                launch_entry(5, "loose_fill", "Loose Rubble", 5.0, true, false, false, 2.0, 1.2, "C8A864", "solid", 1200.0, 800.0, 0.4),
                launch_entry(6, "repair_fill", "Repair", 15.0, true, true, false, 1.0, 0.8, "44FF44", "solid", 800.0, 1500.0, 0.05),
                launch_entry(7, "anchor", "Anchor", 60.0, false, true, false, 1.0, 2.6, "6B4226", "solid", 2600.0, 790.0, 2.5)
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
