//! M16 + M16A § cf-mod validators for `content/hazards/*.ron`,
//! `content/anomalies/*.ron`, `content/artifacts/*.ron`, and
//! `content/afflictions/env/*.ron`.
//!
//! Each validator parses the RON into the canonical spec type and reports
//! a FAIL when the parse fails OR the values fall outside the
//! schema-locked ranges (e.g. severity > 1.0, negative damage, etc.).

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

pub(crate) fn validate_env_affliction_ron(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let spec: cf_affliction::EnvAfflictionSpec = match ron::from_str(&raw) {
        Ok(s) => s,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("ron parse failed: {err}"));
            return;
        }
    };
    if spec.mild_threshold > spec.moderate_threshold
        || spec.moderate_threshold > spec.severe_threshold
        || spec.severe_threshold > spec.lethal_threshold
    {
        report.add_error(
            path.to_path_buf(),
            "thresholds must be monotonically increasing (mild <= moderate <= severe <= lethal)"
                .to_string(),
        );
    }
    if spec.accumulator_rate_per_s < 0.0 {
        report.add_error(
            path.to_path_buf(),
            format!(
                "accumulator_rate_per_s must be >= 0; got {}",
                spec.accumulator_rate_per_s
            ),
        );
    }
    if spec.decay_per_s < 0.0 {
        report.add_error(
            path.to_path_buf(),
            format!("decay_per_s must be >= 0; got {}", spec.decay_per_s),
        );
    }
    if spec.hp_per_second_at_threshold < 0.0 {
        report.add_error(
            path.to_path_buf(),
            format!(
                "hp_per_second_at_threshold must be >= 0; got {}",
                spec.hp_per_second_at_threshold
            ),
        );
    }
    if !(0.0..=1.0).contains(&spec.speed_multiplier) {
        report.add_error(
            path.to_path_buf(),
            format!("speed_multiplier must be in [0,1]; got {}", spec.speed_multiplier),
        );
    }
    if spec.aim_wobble_multiplier < 0.0 {
        report.add_error(
            path.to_path_buf(),
            format!(
                "aim_wobble_multiplier must be >= 0; got {}",
                spec.aim_wobble_multiplier
            ),
        );
    }
    if spec.stamina_drain_multiplier < 0.0 {
        report.add_error(
            path.to_path_buf(),
            format!(
                "stamina_drain_multiplier must be >= 0; got {}",
                spec.stamina_drain_multiplier
            ),
        );
    }
    if spec.clear_cooldown_s < 0.0 {
        report.add_error(
            path.to_path_buf(),
            format!("clear_cooldown_s must be >= 0; got {}", spec.clear_cooldown_s),
        );
    }
    report.add_pass(path.to_path_buf(), format!("env_affliction {}", spec.kind.as_str()));
}

pub(crate) fn validate_disease_ron(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let spec: cf_disease::DiseaseSpec = match ron::from_str(&raw) {
        Ok(s) => s,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("ron parse failed: {err}"));
            return;
        }
    };
    let prob = |v: f32| (0.0..=1.0).contains(&v);
    if !prob(spec.lethality_untreated) {
        report.add_error(
            path.to_path_buf(),
            format!("lethality_untreated must be in [0,1]; got {}", spec.lethality_untreated),
        );
    }
    if !prob(spec.cure.success_chance) {
        report.add_error(
            path.to_path_buf(),
            format!("cure.success_chance must be in [0,1]; got {}", spec.cure.success_chance),
        );
    }
    if !prob(spec.cure.partial_course_consequence.drives_resistance_chance) {
        report.add_error(
            path.to_path_buf(),
            "cure.partial_course_consequence.drives_resistance_chance must be in [0,1]".to_string(),
        );
    }
    if spec.incubation_seconds < 0.0 || spec.prodromal_seconds < 0.0 || spec.manifest_seconds < 0.0 {
        report.add_error(
            path.to_path_buf(),
            "lifecycle stage durations must be >= 0".to_string(),
        );
    }
    if spec.r0_per_exposure_event < 0.0 {
        report.add_error(
            path.to_path_buf(),
            format!("r0_per_exposure_event must be >= 0; got {}", spec.r0_per_exposure_event),
        );
    }
    if spec.transmission_vector_table.is_empty() {
        report.add_error(
            path.to_path_buf(),
            "transmission_vector_table must declare at least one vector".to_string(),
        );
    }
    if !spec.has_vector(spec.primary_vector) {
        report.add_error(
            path.to_path_buf(),
            "transmission_vector_table must include the primary_vector".to_string(),
        );
    }
    if spec.transmission_vector_table.iter().any(|e| e.relative_r0 < 0.0) {
        report.add_error(
            path.to_path_buf(),
            "transmission_vector_table relative_r0 values must be >= 0".to_string(),
        );
    }
    if let Some(v) = &spec.vaccine {
        if !prob(v.side_effect_chance) {
            report.add_error(
                path.to_path_buf(),
                format!("vaccine.side_effect_chance must be in [0,1]; got {}", v.side_effect_chance),
            );
        }
        if v.immunity_duration_seconds < 0.0 {
            report.add_error(
                path.to_path_buf(),
                "vaccine.immunity_duration_seconds must be >= 0".to_string(),
            );
        }
    }
    report.add_pass(path.to_path_buf(), format!("disease {}", spec.kind.as_str()));
}

pub(crate) fn validate_susceptibility_matrix_ron(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let matrix: cf_disease::SusceptibilityMatrix = match ron::from_str(&raw) {
        Ok(m) => m,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("ron parse failed: {err}"));
            return;
        }
    };
    if matrix.grid.is_empty() {
        report.add_error(path.to_path_buf(), "susceptibility grid must be non-empty".to_string());
    }
    for (origin, row) in &matrix.grid {
        for (disease, mult) in row {
            if *mult < 0.0 {
                report.add_error(
                    path.to_path_buf(),
                    format!("multiplier for ({origin}, {disease}) must be >= 0; got {mult}"),
                );
            }
        }
    }
    report.add_pass(
        path.to_path_buf(),
        format!("susceptibility_matrix ({} origins)", matrix.grid.len()),
    );
}

pub(crate) fn validate_cure_ron(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let spec: cf_equipment::CureItemSpec = match ron::from_str(&raw) {
        Ok(s) => s,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("ron parse failed: {err}"));
            return;
        }
    };
    if spec.item_id.is_empty() {
        report.add_error(path.to_path_buf(), "item_id must be non-empty".to_string());
    }
    if spec.treats.is_empty() {
        report.add_error(path.to_path_buf(), "cure must treat at least one disease".to_string());
    }
    if spec.dose_interval_hours < 0.0 {
        report.add_error(
            path.to_path_buf(),
            format!("dose_interval_hours must be >= 0; got {}", spec.dose_interval_hours),
        );
    }
    report.add_pass(path.to_path_buf(), format!("cure {}", spec.item_id));
}

pub(crate) fn validate_vaccine_ron(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let spec: cf_equipment::VaccineItemSpec = match ron::from_str(&raw) {
        Ok(s) => s,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("ron parse failed: {err}"));
            return;
        }
    };
    if spec.item_id.is_empty() {
        report.add_error(path.to_path_buf(), "item_id must be non-empty".to_string());
    }
    if !(0.0..=1.0).contains(&spec.side_effect_chance) {
        report.add_error(
            path.to_path_buf(),
            format!("side_effect_chance must be in [0,1]; got {}", spec.side_effect_chance),
        );
    }
    if spec.immunity_duration_seconds < 0.0 {
        report.add_error(
            path.to_path_buf(),
            "immunity_duration_seconds must be >= 0".to_string(),
        );
    }
    if spec.doses_required == 0 {
        report.add_error(path.to_path_buf(), "doses_required must be >= 1".to_string());
    }
    report.add_pass(path.to_path_buf(), format!("vaccine {}", spec.item_id));
}

#[cfg(test)]
mod m16b_tests {
    use super::*;
    use std::path::PathBuf;

    fn content(sub: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content").join(sub)
    }

    #[test]
    fn all_disease_content_passes() {
        let dir = content("diseases");
        if !dir.exists() {
            return;
        }
        let mut report = ValidationReport::default();
        for entry in fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("ron") {
                continue;
            }
            if path.file_name().and_then(|s| s.to_str()).map(|n| n.starts_with('_')) == Some(true) {
                validate_susceptibility_matrix_ron(&path, &mut report);
            } else {
                validate_disease_ron(&path, &mut report);
            }
        }
        assert_eq!(report.fail(), 0, "disease content must validate cleanly");
        assert!(report.pass() >= 18, "17 diseases + matrix must pass");
    }

    #[test]
    fn all_cure_and_vaccine_content_passes() {
        for (sub, is_cure) in [("cures", true), ("vaccines", false)] {
            let dir = content(sub);
            if !dir.exists() {
                continue;
            }
            let mut report = ValidationReport::default();
            for entry in fs::read_dir(&dir).unwrap().flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) != Some("ron") {
                    continue;
                }
                if is_cure {
                    validate_cure_ron(&path, &mut report);
                } else {
                    validate_vaccine_ron(&path, &mut report);
                }
            }
            assert_eq!(report.fail(), 0, "{sub} content must validate cleanly");
            assert!(report.pass() >= 9, "{sub} must have >= 9 passing files");
        }
    }

    #[test]
    fn malformed_disease_fails() {
        let mut path = std::env::temp_dir();
        path.push(format!("cf_m16b_bad_disease_{}.ron", std::process::id()));
        // lethality 2.0 is out of [0,1].
        fs::write(
            &path,
            "(kind: flu, pathogen_class: viral, primary_vector: airborne, incubation_seconds: 1.0, prodromal_seconds: 0.0, manifest_seconds: 1.0, lethality_untreated: 2.0, becomes_chronic: false, can_become_carrier: false, human_to_human: true, isolation_class: class_a, r0_per_exposure_event: 1.0, cure: (treatment_kind: none, item_required: None, dose_count: 0, dose_interval_hours: 0.0, success_chance: 0.5, partial_course_consequence: (relapses: false, drives_resistance_chance: 0.0), origin_compatibility: []), vaccine: None, cure_only_pre_manifest: false)",
        )
        .unwrap();
        let mut report = ValidationReport::default();
        validate_disease_ron(&path, &mut report);
        assert!(report.fail() >= 1, "out-of-range lethality must FAIL");
        let _ = fs::remove_file(&path);
    }
}
