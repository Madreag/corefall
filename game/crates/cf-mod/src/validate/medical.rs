use std::{fs, path::Path};

use crate::report::ValidationReport;

/// **M4A**: minimal structural mirror of `content/asset_ledger/regen_manifest.ron`,
/// kept in this binary so the validator does not pull a serde dep into
/// `cf-asset-ledger` itself. Matches the locked v1.0.0 schema at
/// `cf-asset-ledger/schemas/v1/regen_manifest.schema.json`.
#[derive(Debug, serde::Deserialize)]
pub(crate) struct RegenManifestV1 {
    pub(crate) schema_version: String,
    pub(crate) pipelines: Vec<RegenPipelineEntry>,
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
pub(crate) struct RegenPipelineEntry {
    pub(crate) pipeline_id: String,
    #[serde(default)]
    pub(crate) owner_milestone: String,
    pub(crate) regen_command: String,
    pub(crate) model_version: String,
    pub(crate) deterministic: bool,
    #[serde(default)]
    pub(crate) freeze_path_suffix: String,
    #[serde(default)]
    pub(crate) notes: String,
}

/// **M11**: minimal structural check for `content/balance/ttd_floors_interim.ron`.
/// Verifies the file is RON-parseable, declares `schema_version: "1.0.0"`,
/// and has at least one floor entry. The canonical M17 loader will replace
/// this with a strict validator once M17 ships.
pub(crate) fn validate_ttd_floors_interim(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    #[derive(serde::Deserialize)]
    struct FloorEntry {
        kind: String,
        origin: String,
        difficulty: String,
        seconds: f32,
    }
    #[derive(serde::Deserialize)]
    struct CompoundModifier {
        a: String,
        b: String,
        multiplier: f32,
    }
    #[derive(serde::Deserialize)]
    struct TtdFloorsInterim {
        schema_version: String,
        floors: Vec<FloorEntry>,
        #[serde(default)]
        compound_modifiers: Vec<CompoundModifier>,
    }
    let v: TtdFloorsInterim = match ron::from_str(&raw) {
        Ok(v) => v,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("ttd_floors_interim parse: {err}"));
            return;
        }
    };
    let mut messages: Vec<String> = Vec::new();
    if v.schema_version != "1.0.0" {
        messages.push(format!(
            "ttd_floors_interim.schema_version must be \"1.0.0\" (got {:?})",
            v.schema_version
        ));
    }
    if v.floors.is_empty() {
        messages.push("ttd_floors_interim.floors must contain at least one entry".to_string());
    }
    for (i, f) in v.floors.iter().enumerate() {
        if f.kind.trim().is_empty() {
            messages.push(format!("floors[{i}].kind must be non-empty"));
        }
        if f.origin.trim().is_empty() {
            messages.push(format!("floors[{i}].origin must be non-empty"));
        }
        if f.difficulty.trim().is_empty() {
            messages.push(format!("floors[{i}].difficulty must be non-empty"));
        }
        if !f.seconds.is_finite() || f.seconds < 0.0 {
            messages.push(format!(
                "floors[{i}].seconds must be a finite non-negative float (got {})",
                f.seconds
            ));
        }
    }
    for (i, cm) in v.compound_modifiers.iter().enumerate() {
        if !cm.multiplier.is_finite() || cm.multiplier < 0.0 {
            messages.push(format!(
                "compound_modifiers[{i}].multiplier must be finite and non-negative (got {})",
                cm.multiplier
            ));
        }
        if cm.a.trim().is_empty() || cm.b.trim().is_empty() {
            messages.push(format!("compound_modifiers[{i}].a and b must be non-empty"));
        }
    }
    if messages.is_empty() {
        report.add_pass(
            path.to_path_buf(),
            format!(
                "ttd_floors_interim v{} ({} floors, {} compound)",
                v.schema_version,
                v.floors.len(),
                v.compound_modifiers.len()
            ),
        );
    } else {
        report.add_error(path.to_path_buf(), messages.join("; "));
    }
}

/// **M14H § VAL-M14H-001**: validate one `content/treatments/<id>.ron`
/// file against the [`cf_treatment::TreatmentSpec`] schema. Rejects
/// files that reference an unknown `TreatmentKind`, omit any required
/// field, or carry a non-finite apply window.
pub(crate) fn validate_treatment_spec_ron(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let spec: cf_treatment::TreatmentSpec = match ron::from_str(&raw) {
        Ok(s) => s,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("treatment_spec parse: {err}"));
            return;
        }
    };
    let mut messages: Vec<String> = Vec::new();
    if !(spec.apply_seconds_min.is_finite() && spec.apply_seconds_min >= 0.0) {
        messages.push("apply_seconds_min must be finite + non-negative".to_string());
    }
    if !(spec.apply_seconds_max.is_finite() && spec.apply_seconds_max >= spec.apply_seconds_min) {
        messages.push("apply_seconds_max must be finite + >= min".to_string());
    }
    if spec.display_name.trim().is_empty() {
        messages.push("display_name must be non-empty".to_string());
    }
    if let Some(charges) = spec.charges {
        if charges == 0 {
            messages.push("charges must be > 0 when set".to_string());
        }
    }
    if let Some(doses) = spec.doses_per_course {
        if doses == 0 {
            messages.push("doses_per_course must be > 0 when set".to_string());
        }
    }
    if messages.is_empty() {
        report.add_pass(
            path.to_path_buf(),
            format!("treatment_spec kind={:?}", spec.kind),
        );
    } else {
        report.add_error(path.to_path_buf(), messages.join("; "));
    }
}

/// **M14I § VAL-M14I-PROSTHETIC**: validate one
/// `content/prosthetics/<name>.ron` file against the
/// [`cf_prosthetic::ProstheticSpec`] schema. Rejects files that reference
/// an unknown `ProstheticKind`, leave `target_zones` empty, or carry an
/// invalid functional-restoration / maintenance-interval / install-seconds.
pub(crate) fn validate_prosthetic_spec_ron(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let spec: cf_prosthetic::ProstheticSpec = match ron::from_str(&raw) {
        Ok(s) => s,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("prosthetic_spec parse: {err}"));
            return;
        }
    };
    let mut messages: Vec<String> = Vec::new();
    if spec.display_name.trim().is_empty() {
        messages.push("display_name must be non-empty".to_string());
    }
    if spec.target_zones.is_empty() {
        messages.push("target_zones must contain at least one zone".to_string());
    }
    if spec.compatible_origins.is_empty() {
        messages.push("compatible_origins must contain at least one origin".to_string());
    }
    if !(spec.functional_restoration.is_finite()
        && spec.functional_restoration >= 0.0
        && spec.functional_restoration <= 1.0)
    {
        messages.push("functional_restoration must be finite + in [0, 1]".to_string());
    }
    if !(spec.maintenance_interval_seconds.is_finite()
        && spec.maintenance_interval_seconds > 0.0)
    {
        messages.push("maintenance_interval_seconds must be finite + > 0".to_string());
    }
    if !(spec.install_seconds.is_finite() && spec.install_seconds >= 0.0) {
        messages.push("install_seconds must be finite + non-negative".to_string());
    }
    if messages.is_empty() {
        report.add_pass(
            path.to_path_buf(),
            format!("prosthetic_spec kind={:?} tier={:?}", spec.kind, spec.tier),
        );
    } else {
        report.add_error(path.to_path_buf(), messages.join("; "));
    }
}

/// **M14G § VAL-M14G-008 / VAL-CROSS-012 / VAL-CROSS-028**: validate one
/// `content/wound_specs/<name>.ron` file against the
/// [`cf_wound::WoundSpec`] schema. Rejects files that reference an unknown
/// `WoundKind`, omit any of the 11 required fields, or carry an
/// `heal_time_seconds_at_band` array of length ≠ 6.
pub(crate) fn validate_wound_spec_ron(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let spec: cf_wound::WoundSpec = match ron::from_str(&raw) {
        Ok(s) => s,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("wound_spec parse: {err}"));
            return;
        }
    };
    let mut messages: Vec<String> = Vec::new();
    if spec.heal_time_seconds_at_band.len() != 6 {
        messages.push(format!(
            "heal_time_seconds_at_band must have length 6, got {}",
            spec.heal_time_seconds_at_band.len()
        ));
    }
    if spec.decal_id.as_str().is_empty() {
        messages.push("decal_id must be non-empty".to_string());
    }
    if !(spec.bleed_rate_ml_per_s_per_severity.is_finite() && spec.bleed_rate_ml_per_s_per_severity >= 0.0) {
        messages.push("bleed_rate_ml_per_s_per_severity must be finite + non-negative".to_string());
    }
    if !(spec.pain_contribution_per_severity.is_finite() && spec.pain_contribution_per_severity >= 0.0) {
        messages.push("pain_contribution_per_severity must be finite + non-negative".to_string());
    }
    if !(spec.infection_base_chance_per_tick.is_finite() && spec.infection_base_chance_per_tick >= 0.0) {
        messages.push("infection_base_chance_per_tick must be finite + non-negative".to_string());
    }
    if messages.is_empty() {
        report.add_pass(
            path.to_path_buf(),
            format!("wound_spec kind={:?} decal_id={}", spec.kind, spec.decal_id.as_str()),
        );
    } else {
        report.add_error(path.to_path_buf(), messages.join("; "));
    }
}

pub(crate) fn validate_regen_manifest(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let manifest: RegenManifestV1 = match ron::from_str(&raw) {
        Ok(m) => m,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("regen_manifest parse: {err}"));
            return;
        }
    };
    let mut messages: Vec<String> = Vec::new();
    if manifest.schema_version != "1.0.0" {
        messages.push(format!(
            "regen_manifest.schema_version must be \"1.0.0\" (got {:?})",
            manifest.schema_version
        ));
    }
    if manifest.pipelines.is_empty() {
        messages.push("regen_manifest.pipelines must contain at least one entry".to_string());
    }
    for (i, entry) in manifest.pipelines.iter().enumerate() {
        if entry.pipeline_id.trim().is_empty() {
            messages.push(format!("pipelines[{i}].pipeline_id must be non-empty"));
        }
        if entry.regen_command.trim().is_empty() {
            messages.push(format!("pipelines[{i}].regen_command must be non-empty"));
        }
        if entry.model_version.trim().is_empty() {
            messages.push(format!("pipelines[{i}].model_version must be non-empty"));
        }
    }
    if messages.is_empty() {
        report.add_pass(
            path.to_path_buf(),
            format!(
                "regen_manifest v{} ({} pipelines)",
                manifest.schema_version,
                manifest.pipelines.len()
            ),
        );
    } else {
        report.add_error(path.to_path_buf(), messages.join("; "));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::ValidationReport;
    use crate::test_helpers::write_tmp;

    #[test]
    fn validate_regen_manifest_accepts_well_formed() {
        let body = r#"(
            schema_version: "1.0.0",
            pipelines: [
                (
                    pipeline_id: "M9A_svg_v1",
                    owner_milestone: "M9A",
                    regen_command: "cf-tools-svg-gen --asset-id $ASSET_ID",
                    model_version: "llm:gpt-4o-mini@2026-05",
                    deterministic: true,
                    freeze_path_suffix: ".frozen",
                    notes: "ok",
                ),
            ],
        )"#;
        let path = write_tmp("regen_manifest.ron", body);
        let mut report = ValidationReport::default();
        validate_regen_manifest(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.pass(), 1, "expected PASS, got {:?}", report.entries);
        assert_eq!(report.fail(), 0);
    }

    #[test]
    fn validate_regen_manifest_accepts_missing_optional_fields() {
        let body = r#"(
            schema_version: "1.0.0",
            pipelines: [
                (
                    pipeline_id: "minimal_v1",
                    regen_command: "cf-tools-minimal",
                    model_version: "v1",
                    deterministic: false,
                ),
            ],
        )"#;
        let path = write_tmp("regen_manifest.ron", body);
        let mut report = ValidationReport::default();
        validate_regen_manifest(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.pass(), 1, "expected PASS, got {:?}", report.entries);
    }

    #[test]
    fn validate_regen_manifest_rejects_wrong_schema_version() {
        let body = r#"(
            schema_version: "2.0.0",
            pipelines: [
                (
                    pipeline_id: "x",
                    regen_command: "y",
                    model_version: "z",
                    deterministic: true,
                ),
            ],
        )"#;
        let path = write_tmp("regen_manifest.ron", body);
        let mut report = ValidationReport::default();
        validate_regen_manifest(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("schema_version"));
    }

    #[test]
    fn validate_regen_manifest_rejects_empty_pipelines() {
        let body = r#"(
            schema_version: "1.0.0",
            pipelines: [],
        )"#;
        let path = write_tmp("regen_manifest.ron", body);
        let mut report = ValidationReport::default();
        validate_regen_manifest(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("pipelines"));
    }

    #[test]
    fn validate_regen_manifest_rejects_empty_pipeline_id() {
        let body = r#"(
            schema_version: "1.0.0",
            pipelines: [
                (
                    pipeline_id: "",
                    regen_command: "y",
                    model_version: "z",
                    deterministic: true,
                ),
            ],
        )"#;
        let path = write_tmp("regen_manifest.ron", body);
        let mut report = ValidationReport::default();
        validate_regen_manifest(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("pipeline_id"));
    }

    #[test]
    fn validate_regen_manifest_rejects_malformed_ron() {
        let body = "this is not valid ron";
        let path = write_tmp("regen_manifest.ron", body);
        let mut report = ValidationReport::default();
        validate_regen_manifest(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("regen_manifest parse"));
    }
}
