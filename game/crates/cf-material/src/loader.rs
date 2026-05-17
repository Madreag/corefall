//! Material registry JSON loader + validator.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{MaterialId, MaterialRegistry, LAUNCH_MATERIAL_NAMES, MATERIAL_SCHEMA_VERSION};

/// One validation finding. The validator returns ALL errors at once so the
/// caller can present them to a content editor without round-trips.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryValidationError {
    /// Stable kind for cfctl / cf-mod scripting. One of: `schema_version_mismatch`,
    /// `duplicate_id`, `unknown_field`, `missing_required_field`,
    /// `launch_set_mismatch`, `integrity_overflow`, `affordance_conflict`,
    /// `color_format`, `unknown_spawn_material`.
    pub kind: String,
    pub path: String,
    pub message: String,
    /// Hint surfaced to the user (one line — keep it actionable).
    pub hint: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RegistryValidationReport {
    pub schema_version: u32,
    pub errors: Vec<RegistryValidationError>,
    pub warnings: Vec<RegistryValidationError>,
    pub material_count: usize,
}

/// Load + validate a registry from disk. Returns the parsed registry plus the
/// validation report. If `errors.is_empty()` the registry is safe to use.
pub fn load_registry_from_file(
    path: impl AsRef<Path>,
) -> Result<(MaterialRegistry, RegistryValidationReport), RegistryLoadError> {
    let path_ref = path.as_ref();
    let raw = fs::read_to_string(path_ref).map_err(|source| RegistryLoadError::Io {
        path: path_ref.to_path_buf(),
        source,
    })?;
    let json_value: serde_json::Value = serde_json::from_str(&raw).map_err(|source| RegistryLoadError::Parse {
        path: path_ref.to_path_buf(),
        source,
    })?;
    let report = validate_registry_json(&json_value);
    let registry: MaterialRegistry = serde_json::from_value(json_value).map_err(|source| RegistryLoadError::Parse {
        path: path_ref.to_path_buf(),
        source,
    })?;
    Ok((registry, report))
}

/// Run the validator over a parsed `serde_json::Value` so callers can also
/// validate registry data that hasn't been promoted to `MaterialRegistry`
/// yet (the cf-mod CLI uses this path).
pub fn validate_registry_json(value: &serde_json::Value) -> RegistryValidationReport {
    let mut report = RegistryValidationReport {
        schema_version: 0,
        errors: Vec::new(),
        warnings: Vec::new(),
        material_count: 0,
    };

    // schema_version check.
    match value.get("schema_version").and_then(|v| v.as_u64()) {
        Some(v) if v as u32 == MATERIAL_SCHEMA_VERSION => {
            report.schema_version = v as u32;
        }
        Some(other) => {
            report.errors.push(RegistryValidationError {
                kind: "schema_version_mismatch".to_string(),
                path: "schema_version".to_string(),
                message: format!("schema_version must be {MATERIAL_SCHEMA_VERSION}; got {other}"),
                hint: format!(
                    "Set schema_version to {MATERIAL_SCHEMA_VERSION} or run a migration if you have a newer registry."
                ),
            });
        }
        None => {
            report.errors.push(RegistryValidationError {
                kind: "schema_version_mismatch".to_string(),
                path: "schema_version".to_string(),
                message: "schema_version field missing".to_string(),
                hint: format!("Add `\"schema_version\": {MATERIAL_SCHEMA_VERSION}` at the root."),
            });
        }
    }

    // materials array check.
    let materials = match value.get("materials").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => {
            report.errors.push(RegistryValidationError {
                kind: "missing_required_field".to_string(),
                path: "materials".to_string(),
                message: "materials[] must be an array".to_string(),
                hint: "Wrap the eight launch materials in a `materials: [...]` array.".to_string(),
            });
            return report;
        }
    };
    report.material_count = materials.len();

    // Per-material checks.
    let mut seen_ids: std::collections::BTreeMap<MaterialId, usize> = std::collections::BTreeMap::new();
    let mut seen_names: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let allowed_fields = known_material_fields();
    let required_fields = [
        "id",
        "name",
        "display_name",
        "hardness",
        "diggable",
        "anchorable",
        "hazard",
        "path_cost",
        "density",
        "color_hex",
        "description",
    ];

    for (idx, mat) in materials.iter().enumerate() {
        let path_prefix = format!("materials[{idx}]");
        let obj = match mat.as_object() {
            Some(o) => o,
            None => {
                report.errors.push(RegistryValidationError {
                    kind: "missing_required_field".to_string(),
                    path: path_prefix.clone(),
                    message: "expected object".to_string(),
                    hint: "Each entry in materials[] must be an object.".to_string(),
                });
                continue;
            }
        };

        // Unknown fields.
        for key in obj.keys() {
            if !allowed_fields.contains(&key.as_str()) {
                report.errors.push(RegistryValidationError {
                    kind: "unknown_field".to_string(),
                    path: format!("{path_prefix}.{key}"),
                    message: format!("unknown field `{key}`"),
                    hint: format!(
                        "Allowed fields: {}. Drop the unknown field or open an RFC to extend the schema.",
                        allowed_fields_csv()
                    ),
                });
            }
        }

        // Required fields.
        for req in required_fields {
            if !obj.contains_key(req) {
                report.errors.push(RegistryValidationError {
                    kind: "missing_required_field".to_string(),
                    path: format!("{path_prefix}.{req}"),
                    message: format!("required field `{req}` missing"),
                    hint: format!("Add `\"{req}\": ...` to this material entry."),
                });
            }
        }

        // Duplicate id / name.
        if let Some(id) = obj.get("id").and_then(|v| v.as_u64()) {
            let id_u8 = id as MaterialId;
            if let Some(prev) = seen_ids.insert(id_u8, idx) {
                report.errors.push(RegistryValidationError {
                    kind: "duplicate_id".to_string(),
                    path: format!("{path_prefix}.id"),
                    message: format!("duplicate material id {id_u8} (previously seen at index {prev})"),
                    hint: "Each material id must be unique across the registry.".to_string(),
                });
            }
        }
        if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
            if let Some(prev) = seen_names.insert(name.to_string(), idx) {
                report.errors.push(RegistryValidationError {
                    kind: "duplicate_id".to_string(),
                    path: format!("{path_prefix}.name"),
                    message: format!("duplicate material name `{name}` (previously seen at index {prev})"),
                    hint: "Each material name must be unique.".to_string(),
                });
            }
        }

        // Hardness sanity (integrity_overflow).
        if let Some(hardness) = obj.get("hardness").and_then(|v| v.as_f64()) {
            if !hardness.is_finite() {
                report.errors.push(RegistryValidationError {
                    kind: "integrity_overflow".to_string(),
                    path: format!("{path_prefix}.hardness"),
                    message: format!("hardness {hardness} is not finite"),
                    hint: "Use a finite, positive number; -1 reserved for unbreakable in CCCP.".to_string(),
                });
            } else if hardness < 0.0 && (hardness - -1.0).abs() > 1e-6 {
                report.errors.push(RegistryValidationError {
                    kind: "integrity_overflow".to_string(),
                    path: format!("{path_prefix}.hardness"),
                    message: format!("hardness {hardness} is negative (only -1 reserved for unbreakable)"),
                    hint: "Non-negative integrity values only (or -1 for unbreakable).".to_string(),
                });
            }
        }

        // Affordance conflict: hazard=true + actor_passable explicitly true is suspicious.
        let hazard = obj.get("hazard").and_then(|v| v.as_bool()).unwrap_or(false);
        let actor_passable = obj.get("actor_passable").and_then(|v| v.as_bool());
        if hazard && actor_passable == Some(true) {
            let path_cost = obj.get("path_cost").and_then(|v| v.as_f64()).unwrap_or(1.0);
            if path_cost <= 1.0 {
                report.warnings.push(RegistryValidationError {
                    kind: "affordance_conflict".to_string(),
                    path: format!("{path_prefix}"),
                    message: "hazard=true + actor_passable=true + low path_cost is suspicious (AI may walk into it)"
                        .to_string(),
                    hint: "Raise path_cost (>= 5) for hazards the AI should route around.".to_string(),
                });
            }
        }

        // Color format check (6-hex).
        if let Some(color) = obj.get("color_hex").and_then(|v| v.as_str()) {
            if !is_valid_hex_color(color) {
                report.errors.push(RegistryValidationError {
                    kind: "color_format".to_string(),
                    path: format!("{path_prefix}.color_hex"),
                    message: format!("color_hex `{color}` must be 6 hex chars (no #)"),
                    hint: "Use `RRGGBB` with hex digits, e.g. `8B6914`.".to_string(),
                });
            }
        }
    }

    // Launch-set check: must contain the 8 DR-007 ids with the correct
    // canonical names (id=0 -> air, etc.).
    let strict_launch_set = report
        .errors
        .iter()
        .all(|e| !matches!(e.kind.as_str(), "schema_version_mismatch" | "missing_required_field"));
    if strict_launch_set {
        for (id, name) in LAUNCH_MATERIAL_NAMES {
            match seen_names.get(*name) {
                Some(idx) => {
                    if let Some(found_id) = materials[*idx].get("id").and_then(|v| v.as_u64()) {
                        if found_id as MaterialId != *id {
                            report.errors.push(RegistryValidationError {
                                kind: "launch_set_mismatch".to_string(),
                                path: format!("materials[{idx}].id"),
                                message: format!(
                                    "launch material `{name}` must have id={id}; got {found_id}"
                                ),
                                hint: "DR-007 launch set: 0=air, 1=dirt, 2=concrete, 3=metal_nohook, 4=hazard, 5=loose_fill, 6=repair_fill, 7=anchor.".to_string(),
                            });
                        }
                    }
                }
                None => {
                    report.errors.push(RegistryValidationError {
                        kind: "launch_set_mismatch".to_string(),
                        path: "materials".to_string(),
                        message: format!("required launch material `{name}` (id={id}) missing"),
                        hint: "The DR-007 launch set requires all 8 materials; add the missing entry.".to_string(),
                    });
                }
            }
        }
    }

    report
}

/// Validate an already-parsed [`MaterialRegistry`]. Useful when the JSON has
/// already been parsed elsewhere and you want the same per-rule findings.
pub fn validate_registry(registry: &MaterialRegistry) -> Vec<RegistryValidationError> {
    let value = serde_json::to_value(registry).unwrap_or(serde_json::Value::Null);
    let report = validate_registry_json(&value);
    let mut all = report.errors;
    all.extend(report.warnings);
    all
}

fn known_material_fields() -> &'static [&'static str] {
    &[
        "id",
        "name",
        "display_name",
        "hardness",
        "diggable",
        "anchorable",
        "hazard",
        "path_cost",
        "density",
        "color_hex",
        "description",
        "drillable",
        "blastable",
        "beam_cuttable",
        "projectile_passable",
        "actor_passable",
        "blocks_line_of_sight",
        "damage_per_tick",
        "damage_kind",
        "stickiness",
        "restitution",
        "friction",
        "structural_integrity",
        "priority",
        "piling",
        "settle_material",
        "spawn_material",
        "is_scrap",
        "uses_own_color",
        "ui_overlay_color",
        "render_priority",
        "heat_capacity",
        "thermal_conductivity",
        "temperature",
        "ignition_temperature",
        "burn_rate",
        "oxygen_requirement",
        "burn_products",
        "phase_changes",
        "conductivity",
        "wetting",
        "reaction_tags",
        "ai_affordances",
        // M12B § acoustic registry fields (echo + decay + transmission + low-pass).
        "echo_coefficient",
        "decay_band",
        "acoustic_transmission_loss_db",
        "low_pass_cutoff_hz",
    ]
}

fn allowed_fields_csv() -> String {
    known_material_fields().join(", ")
}

fn is_valid_hex_color(s: &str) -> bool {
    s.len() == 6 && s.chars().all(|c| c.is_ascii_hexdigit())
}

#[derive(Debug, thiserror::Error)]
pub enum RegistryLoadError {
    #[error("io error reading {path:?}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("json parse error in {path:?}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_launch_registry() -> serde_json::Value {
        serde_json::json!({
            "schema_version": 1,
            "materials": [
                {"id": 0, "name": "air", "display_name": "Air", "hardness": 0.0, "diggable": false, "anchorable": false, "hazard": false, "path_cost": 1.0, "density": 0.0, "color_hex": "000000", "description": "Empty"},
                {"id": 1, "name": "dirt", "display_name": "Dirt", "hardness": 10.0, "diggable": true, "anchorable": true, "hazard": false, "path_cost": 1.0, "density": 1.5, "color_hex": "8B6914", "description": "Default destructible terrain"},
                {"id": 2, "name": "concrete", "display_name": "Concrete", "hardness": 40.0, "diggable": true, "anchorable": true, "hazard": false, "path_cost": 1.0, "density": 2.3, "color_hex": "808080", "description": "Bunker wall"},
                {"id": 3, "name": "metal_nohook", "display_name": "Reinforced Metal", "hardness": 100.0, "diggable": false, "anchorable": false, "hazard": false, "path_cost": 999.0, "density": 7.8, "color_hex": "4A4A4A", "description": "Refuse-by-default metal"},
                {"id": 4, "name": "hazard", "display_name": "Hazard Tile", "hardness": 50.0, "diggable": false, "anchorable": false, "hazard": true, "path_cost": 10.0, "density": 3.0, "color_hex": "FF4444", "description": "Damage-on-touch surface"},
                {"id": 5, "name": "loose_fill", "display_name": "Loose Rubble", "hardness": 5.0, "diggable": true, "anchorable": false, "hazard": false, "path_cost": 2.0, "density": 1.2, "color_hex": "C8A864", "description": "Soft fill"},
                {"id": 6, "name": "repair_fill", "display_name": "Repair Foam", "hardness": 15.0, "diggable": true, "anchorable": true, "hazard": false, "path_cost": 1.0, "density": 0.8, "color_hex": "44FF44", "description": "Player-placed repair"},
                {"id": 7, "name": "anchor", "display_name": "Anchor Rock", "hardness": 60.0, "diggable": false, "anchorable": true, "hazard": false, "path_cost": 1.0, "density": 2.6, "color_hex": "6B4226", "description": "Hard anchorable surface"}
            ]
        })
    }

    #[test]
    fn full_launch_set_validates_cleanly() {
        let v = full_launch_registry();
        let report = validate_registry_json(&v);
        assert!(report.errors.is_empty(), "errors: {:?}", report.errors);
        assert_eq!(report.material_count, 8);
    }

    #[test]
    fn missing_required_field_rejects() {
        let mut v = full_launch_registry();
        v["materials"][1].as_object_mut().unwrap().remove("hardness");
        let report = validate_registry_json(&v);
        assert!(report
            .errors
            .iter()
            .any(|e| e.kind == "missing_required_field" && e.path.ends_with("hardness")));
    }

    #[test]
    fn unknown_field_rejects_with_hint() {
        let mut v = full_launch_registry();
        v["materials"][1]["rainbow_color"] = serde_json::json!("red");
        let report = validate_registry_json(&v);
        let err = report
            .errors
            .iter()
            .find(|e| e.kind == "unknown_field")
            .expect("expected unknown_field error");
        assert!(err.hint.contains("Allowed fields"));
    }

    #[test]
    fn duplicate_id_rejects() {
        let mut v = full_launch_registry();
        v["materials"][1]["id"] = serde_json::json!(0);
        let report = validate_registry_json(&v);
        assert!(report.errors.iter().any(|e| e.kind == "duplicate_id"));
    }

    #[test]
    fn schema_version_mismatch_rejects() {
        let mut v = full_launch_registry();
        v["schema_version"] = serde_json::json!(99);
        let report = validate_registry_json(&v);
        assert!(report.errors.iter().any(|e| e.kind == "schema_version_mismatch"));
    }

    #[test]
    fn launch_set_id_mismatch_rejects() {
        let mut v = full_launch_registry();
        v["materials"][1]["id"] = serde_json::json!(42);
        let report = validate_registry_json(&v);
        assert!(report
            .errors
            .iter()
            .any(|e| e.kind == "duplicate_id" || e.kind == "launch_set_mismatch"));
    }

    #[test]
    fn integrity_overflow_rejects() {
        let mut v = full_launch_registry();
        v["materials"][1]["hardness"] = serde_json::json!(-5.0);
        let report = validate_registry_json(&v);
        assert!(report.errors.iter().any(|e| e.kind == "integrity_overflow"));
    }

    #[test]
    fn bad_color_hex_rejects() {
        let mut v = full_launch_registry();
        v["materials"][1]["color_hex"] = serde_json::json!("not-hex!");
        let report = validate_registry_json(&v);
        assert!(report.errors.iter().any(|e| e.kind == "color_format"));
    }
}
