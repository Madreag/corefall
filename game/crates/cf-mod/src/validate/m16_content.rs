//! M16 § cf-mod validators for `content/hazards/*.ron`,
//! `content/anomalies/*.ron`, and `content/artifacts/*.ron`.
//!
//! Each validator parses the RON into the canonical spec type
//! (`HazardSpec` / `AnomalySpec` / `ArtifactSpec`) and reports a FAIL
//! when the parse fails OR the values fall outside the schema-locked
//! ranges (e.g. severity > 1.0, negative damage, etc.).

use std::{fs, path::Path};

use crate::report::ValidationReport;

pub(crate) fn validate_hazard_ron(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let spec: cf_hazard::HazardSpec = match ron::from_str(&raw) {
        Ok(s) => s,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("ron parse failed: {err}"));
            return;
        }
    };
    if spec.spread_tiles_per_s < 0.0 {
        report.add_error(
            path.to_path_buf(),
            format!("spread_tiles_per_s must be >= 0; got {}", spec.spread_tiles_per_s),
        );
    }
    if spec.dissipation_seconds < 0.0 {
        report.add_error(
            path.to_path_buf(),
            format!("dissipation_seconds must be >= 0; got {}", spec.dissipation_seconds),
        );
    }
    if spec.contact_damage_per_tick < 0.0 {
        report.add_error(
            path.to_path_buf(),
            format!(
                "contact_damage_per_tick must be >= 0; got {}",
                spec.contact_damage_per_tick
            ),
        );
    }
    report.add_pass(path.to_path_buf(), format!("hazard {}", spec.kind.as_str()));
}

pub(crate) fn validate_anomaly_ron(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let spec: cf_anomaly::AnomalySpec = match ron::from_str(&raw) {
        Ok(s) => s,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("ron parse failed: {err}"));
            return;
        }
    };
    if spec.detection_radius_m <= 0.0 {
        report.add_error(
            path.to_path_buf(),
            format!("detection_radius_m must be > 0; got {}", spec.detection_radius_m),
        );
    }
    if spec.damage_per_tick < 0.0 {
        report.add_error(
            path.to_path_buf(),
            format!("damage_per_tick must be >= 0; got {}", spec.damage_per_tick),
        );
    }
    if spec.movement_multiplier <= 0.0 {
        report.add_error(
            path.to_path_buf(),
            format!("movement_multiplier must be > 0; got {}", spec.movement_multiplier),
        );
    }
    report.add_pass(path.to_path_buf(), format!("anomaly {}", spec.kind.as_str()));
}

pub(crate) fn validate_artifact_ron(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let spec: cf_artifact::ArtifactSpec = match ron::from_str(&raw) {
        Ok(s) => s,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("ron parse failed: {err}"));
            return;
        }
    };
    if spec.id.is_empty() {
        report.add_error(path.to_path_buf(), "id must be non-empty".to_string());
    }
    if spec.display_name.is_empty() {
        report.add_error(path.to_path_buf(), "display_name must be non-empty".to_string());
    }
    if spec.spawn_weight < 0.0 {
        report.add_error(
            path.to_path_buf(),
            format!("spawn_weight must be >= 0; got {}", spec.spawn_weight),
        );
    }
    let resist_clamp = |v: f32| (-0.95..=0.95).contains(&v);
    if !resist_clamp(spec.bonus.radiation_resistance)
        || !resist_clamp(spec.bonus.cold_resistance)
        || !resist_clamp(spec.bonus.fire_resistance)
        || !resist_clamp(spec.bonus.electric_resistance)
        || !resist_clamp(spec.bonus.toxic_resistance)
    {
        report.add_error(
            path.to_path_buf(),
            "resistance fields must be within [-0.95, 0.95]".to_string(),
        );
    }
    report.add_pass(path.to_path_buf(), format!("artifact {}", spec.id));
}
