use std::{fs, path::Path};

use crate::report::ValidationReport;

/// **M1.5 (baseline) + M7 (extension)**: cf-mod validator for
/// `content/ai/difficulty.json`. Two registry shapes are accepted:
///
/// 1. **M2 baseline shape** — entries carry the three legacy ids
///    (`cakewalk`, `tough_crowd`, `veteran`) and the M2 AI tuning fields
///    (hp, aim_settle_ticks, miss_chance, sight_range, hearing_radius,
///    memory_decay_ticks, reload_ms). This shape pre-dates the M7 friend-
///    feedback expansion; the schema check stays in place so older mods
///    keep validating.
///
/// 2. **M7 extension shape** — entries declare `archetype_id` +
///    `difficulty_id` and the registry contains exactly **5 archetypes ×
///    3 difficulties = 15 entries**. The spec at `specs/active/M7.md`
///    § "AI difficulty preset registry" mandates each entry carries the
///    M2 fields plus the three new M7 fields: `cover_seek_radius`,
///    `retreat_hp_threshold` (0.15..=0.50), `squad_comm_delay_ticks`.
///    The five archetype ids are rifleman / sniper / assault / engineer /
///    medic; the three difficulty ids are the legacy trio. Every
///    (archetype_id, difficulty_id) pair MUST appear exactly once.
pub(crate) fn validate_difficulty_json(path: &Path, report: &mut ValidationReport) {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let value: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("json parse failed: {err}"));
            return;
        }
    };
    let mut messages: Vec<String> = Vec::new();
    match value.get("schema").and_then(|v| v.as_u64()) {
        Some(1) => {}
        Some(other) => messages.push(format!("difficulty.schema must be 1 (got {other})")),
        None => messages.push("difficulty.schema missing or not an integer".to_string()),
    }
    let presets = match value.get("presets").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => {
            messages.push("difficulty.presets missing or not an array".to_string());
            report.add_error(path.to_path_buf(), messages.join("; "));
            return;
        }
    };
    let shape = detect_difficulty_shape(presets);
    match shape {
        DifficultyRegistryShape::M7Extension => {
            validate_difficulty_m7_shape(presets, &mut messages);
        }
        DifficultyRegistryShape::M2Baseline => {
            validate_difficulty_m2_shape(presets, &mut messages);
        }
    }
    if messages.is_empty() {
        let shape_label = match shape {
            DifficultyRegistryShape::M7Extension => "M7 extension shape",
            DifficultyRegistryShape::M2Baseline => "M2 baseline shape",
        };
        report.add_pass(
            path.to_path_buf(),
            format!("difficulty.json ({} presets, {shape_label})", presets.len()),
        );
    } else {
        report.add_error(path.to_path_buf(), messages.join("; "));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DifficultyRegistryShape {
    M2Baseline,
    M7Extension,
}

pub(crate) fn detect_difficulty_shape(presets: &[serde_json::Value]) -> DifficultyRegistryShape {
    if presets
        .iter()
        .any(|p| p.get("archetype_id").and_then(|v| v.as_str()).is_some())
    {
        DifficultyRegistryShape::M7Extension
    } else {
        DifficultyRegistryShape::M2Baseline
    }
}

const M2_DIFFICULTY_REQUIRED_IDS: &[&str] = &["cakewalk", "tough_crowd", "veteran"];
const M2_DIFFICULTY_REQUIRED_FIELDS: &[&str] = &[
    "hp",
    "aim_settle_ticks",
    "miss_chance",
    "sight_range",
    "hearing_radius",
    "memory_decay_ticks",
    "reload_ms",
];

pub(crate) fn validate_difficulty_m2_shape(presets: &[serde_json::Value], messages: &mut Vec<String>) {
    let mut found_ids: Vec<String> = Vec::new();
    for (i, preset) in presets.iter().enumerate() {
        let id = preset.get("id").and_then(|v| v.as_str()).unwrap_or("");
        if id.is_empty() {
            messages.push(format!("presets[{i}].id missing or not a string"));
            continue;
        }
        found_ids.push(id.to_string());
        for field in M2_DIFFICULTY_REQUIRED_FIELDS {
            if preset.get(*field).is_none() {
                messages.push(format!("presets[{i}={id}].{field} missing"));
            }
        }
    }
    for required in M2_DIFFICULTY_REQUIRED_IDS {
        if !found_ids.iter().any(|id| id == required) {
            messages.push(format!("required preset id `{required}` missing"));
        }
    }
}

const M7_DIFFICULTY_ARCHETYPE_IDS: &[&str] = &["rifleman", "sniper", "assault", "engineer", "medic"];
const M7_DIFFICULTY_DIFFICULTY_IDS: &[&str] = &["cakewalk", "tough_crowd", "veteran"];
const M7_DIFFICULTY_REQUIRED_FIELDS: &[&str] = &[
    "id",
    "archetype_id",
    "difficulty_id",
    "name",
    "hp",
    "hp_multiplier",
    "damage_multiplier",
    "aim_settle_ticks",
    "reaction_time_ticks",
    "miss_chance",
    "sight_range",
    "sight_fov_degrees",
    "fov_degrees",
    "hearing_radius",
    "hearing_range",
    "memory_decay_ticks",
    "reload_ms",
    "retreat_hp_threshold",
    "cover_seek_radius",
    "squad_comm_delay_ticks",
];

pub(crate) fn validate_difficulty_m7_shape(presets: &[serde_json::Value], messages: &mut Vec<String>) {
    let expected_total = M7_DIFFICULTY_ARCHETYPE_IDS.len() * M7_DIFFICULTY_DIFFICULTY_IDS.len();
    if presets.len() != expected_total {
        messages.push(format!(
            "M7 difficulty registry must have exactly {expected_total} entries \
             ({} archetypes x {} difficulties); got {}",
            M7_DIFFICULTY_ARCHETYPE_IDS.len(),
            M7_DIFFICULTY_DIFFICULTY_IDS.len(),
            presets.len()
        ));
    }
    let mut seen_pairs: Vec<(String, String)> = Vec::new();
    for (i, preset) in presets.iter().enumerate() {
        let arch = preset.get("archetype_id").and_then(|v| v.as_str()).unwrap_or("");
        let diff = preset.get("difficulty_id").and_then(|v| v.as_str()).unwrap_or("");
        let id = preset.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let label = if id.is_empty() {
            format!("[{i}]")
        } else {
            format!("[{i}={id}]")
        };
        if arch.is_empty() {
            messages.push(format!("presets{label}.archetype_id missing"));
        } else if !M7_DIFFICULTY_ARCHETYPE_IDS.contains(&arch) {
            messages.push(format!(
                "presets{label}.archetype_id `{arch}` not in {M7_DIFFICULTY_ARCHETYPE_IDS:?}"
            ));
        }
        if diff.is_empty() {
            messages.push(format!("presets{label}.difficulty_id missing"));
        } else if !M7_DIFFICULTY_DIFFICULTY_IDS.contains(&diff) {
            messages.push(format!(
                "presets{label}.difficulty_id `{diff}` not in {M7_DIFFICULTY_DIFFICULTY_IDS:?}"
            ));
        }
        if !arch.is_empty() && !diff.is_empty() {
            let pair = (arch.to_string(), diff.to_string());
            if seen_pairs.contains(&pair) {
                messages.push(format!(
                    "presets{label}: duplicate (archetype_id, difficulty_id) pair `({arch}, {diff})`"
                ));
            } else {
                seen_pairs.push(pair);
            }
        }
        for field in M7_DIFFICULTY_REQUIRED_FIELDS {
            if preset.get(*field).is_none() {
                messages.push(format!("presets{label}.{field} missing"));
            }
        }
        if let Some(rht) = preset.get("retreat_hp_threshold").and_then(|v| v.as_f64()) {
            if !(0.15..=0.50).contains(&rht) {
                messages.push(format!("presets{label}.retreat_hp_threshold {rht} outside 0.15..=0.50"));
            }
        }
        if let Some(csr) = preset.get("cover_seek_radius").and_then(|v| v.as_f64()) {
            if csr <= 0.0 {
                messages.push(format!("presets{label}.cover_seek_radius {csr} must be > 0"));
            }
        }
        if let Some(delay) = preset.get("squad_comm_delay_ticks").and_then(|v| v.as_i64()) {
            if delay < 0 {
                messages.push(format!("presets{label}.squad_comm_delay_ticks {delay} must be >= 0"));
            }
        }
        if let Some(mc) = preset.get("miss_chance").and_then(|v| v.as_f64()) {
            if !(0.0..=1.0).contains(&mc) {
                messages.push(format!("presets{label}.miss_chance {mc} outside 0.0..=1.0"));
            }
        }
        if let Some(hpm) = preset.get("hp_multiplier").and_then(|v| v.as_f64()) {
            if hpm <= 0.0 {
                messages.push(format!("presets{label}.hp_multiplier {hpm} must be > 0"));
            }
        }
        if let Some(dm) = preset.get("damage_multiplier").and_then(|v| v.as_f64()) {
            if dm <= 0.0 {
                messages.push(format!("presets{label}.damage_multiplier {dm} must be > 0"));
            }
        }
    }
    for arch in M7_DIFFICULTY_ARCHETYPE_IDS {
        for diff in M7_DIFFICULTY_DIFFICULTY_IDS {
            let pair = ((*arch).to_string(), (*diff).to_string());
            if !seen_pairs.contains(&pair) {
                messages.push(format!(
                    "required (archetype_id, difficulty_id) pair `({arch}, {diff})` missing"
                ));
            }
        }
    }
}
