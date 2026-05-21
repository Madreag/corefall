use std::{fs, path::Path};

use crate::report::ValidationReport;

/// **M6B**: ItemSpec manifest entry shape (mirrors
/// `cf_equipment::ItemSpec::{id, category}`).
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct M6BItemManifestEntry {
    pub(crate) id: String,
    pub(crate) category: String,
}

/// **M6B**: full item manifest envelope. Schema_version is locked at 1
/// per spec § Files.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct M6BItemManifest {
    pub(crate) schema_version: u32,
    pub(crate) items: Vec<M6BItemManifestEntry>,
}

/// **M6B**: validate the on-disk
/// `content/equipment/items/manifest.ron` against the canonical
/// `cf_equipment::item_spec` registry. Each manifest entry MUST resolve
/// through `spec_for_id()` and the declared `category` MUST match the
/// runtime spec's `category.as_str()`. Manifest must list at least every
/// registered item id (drift-detection: a hardcoded registry entry that
/// authors forgot to list in the manifest is a bug).
pub(crate) fn validate_item_manifest(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let parsed: M6BItemManifest = match ron::from_str(&raw) {
        Ok(v) => v,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("ron parse failed: {err}"));
            return;
        }
    };
    let mut messages: Vec<String> = Vec::new();
    if parsed.schema_version != 1 {
        messages.push(format!(
            "item_manifest.schema_version must be 1 (got {})",
            parsed.schema_version
        ));
    }
    if parsed.items.is_empty() {
        messages.push("item_manifest must declare at least 1 item".to_string());
    }
    let mut manifest_ids: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for (i, entry) in parsed.items.iter().enumerate() {
        if entry.id.trim().is_empty() {
            messages.push(format!("items[{i}].id must be non-empty"));
            continue;
        }
        if !manifest_ids.insert(entry.id.clone()) {
            messages.push(format!("items[{i}].id `{}` duplicated", entry.id));
        }
        match cf_equipment::spec_for_id(&entry.id) {
            Some(spec) => {
                if spec.category.as_str() != entry.category {
                    messages.push(format!(
                        "items[{i}].id `{}` category mismatch (manifest={}, registry={})",
                        entry.id,
                        entry.category,
                        spec.category.as_str()
                    ));
                }
            }
            None => {
                messages.push(format!(
                    "items[{i}].id `{}` is not registered in cf_equipment::item_spec",
                    entry.id
                ));
            }
        }
    }
    let registry_ids: std::collections::BTreeSet<String> = cf_equipment::item_registered_ids().into_iter().collect();
    for missing in registry_ids.difference(&manifest_ids) {
        messages.push(format!(
            "registered item `{missing}` is missing from manifest.ron (mirror drift)"
        ));
    }
    if messages.is_empty() {
        report.add_pass(
            path.to_path_buf(),
            format!("item_manifest ({} entries)", parsed.items.len()),
        );
    } else {
        report.add_error(path.to_path_buf(), messages.join("; "));
    }
}

/// **M6B**: validate a standalone `content/equipment/items/<id>.ron`
/// ItemSpec definition. The file must parse as a `cf_equipment::ItemSpec`
/// (via serde) and the canonical id MUST already be registered in the
/// runtime registry (so mods can't ship arbitrary undeclared ids while
/// the lock window stays narrow). Per spec § "Validate
/// `content/equipment/items/*.ron` against ItemSpec schema".
pub(crate) fn validate_item_spec_ron(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let spec: cf_equipment::ItemSpec = match ron::from_str(&raw) {
        Ok(v) => v,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("item_spec ron parse failed: {err}"));
            return;
        }
    };
    let mut messages: Vec<String> = Vec::new();
    if spec.id.trim().is_empty() {
        messages.push("item_spec.id must be non-empty".to_string());
    }
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    if !stem.is_empty() && stem != spec.id {
        messages.push(format!(
            "item_spec.id `{}` mismatches filename stem `{stem}`",
            spec.id
        ));
    }
    if spec.mass_kg < 0.0 || !spec.mass_kg.is_finite() {
        messages.push(format!("item_spec.mass_kg must be finite and >= 0 (got {})", spec.mass_kg));
    }
    if spec.dimensions.w == 0 || spec.dimensions.h == 0 {
        messages.push(format!(
            "item_spec.dimensions must be > 0 in both axes (got {}×{})",
            spec.dimensions.w, spec.dimensions.h
        ));
    }
    if spec.bulk_volume_l < 0.0 || !spec.bulk_volume_l.is_finite() {
        messages.push(format!(
            "item_spec.bulk_volume_l must be finite and >= 0 (got {})",
            spec.bulk_volume_l
        ));
    }
    if spec.stackable && spec.max_stack == 0 {
        messages.push("stackable items must declare max_stack > 0".to_string());
    }
    if let Some(cap) = &spec.container_capacity {
        if cap.max_nest_depth > cf_equipment::MAX_CONTAINER_NEST_DEPTH {
            messages.push(format!(
                "container_capacity.max_nest_depth ({}) exceeds engine cap ({})",
                cap.max_nest_depth,
                cf_equipment::MAX_CONTAINER_NEST_DEPTH
            ));
        }
    }
    if let Some(liquid_cap) = spec.liquid_capacity_l {
        if liquid_cap < 0.0 || !liquid_cap.is_finite() {
            messages.push(format!(
                "liquid_capacity_l must be finite and >= 0 (got {liquid_cap})"
            ));
        }
    }
    if cf_equipment::spec_for_id(&spec.id).is_none() {
        messages.push(format!(
            "item_spec.id `{}` is not registered in cf_equipment::item_spec (M6B locked the registry; new ids land in M6C+)",
            spec.id
        ));
    }
    if messages.is_empty() {
        report.add_pass(
            path.to_path_buf(),
            format!("item_spec `{}` ({} kg, {}×{}, {} L)", spec.id, spec.mass_kg, spec.dimensions.w, spec.dimensions.h, spec.bulk_volume_l),
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
    fn item_manifest_accepts_minimal_synced_manifest() {
        let registry_ids = cf_equipment::item_registered_ids();
        let mut items = String::new();
        for id in &registry_ids {
            let spec = cf_equipment::spec_for_id(id).unwrap();
            items.push_str(&format!("(id: \"{}\", category: \"{}\"),\n", id, spec.category.as_str()));
        }
        let body = format!("(schema_version: 1, items: [{items}])");
        let path = write_tmp("manifest.ron", &body);
        let mut report = ValidationReport::default();
        validate_item_manifest(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.pass(), 1, "report: {:?}", report.entries);
        assert_eq!(report.fail(), 0);
    }

    #[test]
    fn item_manifest_rejects_unknown_id() {
        let body = r#"(
  schema_version: 1,
  items: [
    (id: "rifle_m1", category: "weapon"),
    (id: "this_id_does_not_exist", category: "weapon"),
  ],
)"#;
        let path = write_tmp("manifest.ron", body);
        let mut report = ValidationReport::default();
        validate_item_manifest(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("not registered"));
    }

    #[test]
    fn item_manifest_rejects_category_drift() {
        let body = r#"(
  schema_version: 1,
  items: [
    (id: "rifle_m1", category: "consumable"),
  ],
)"#;
        let path = write_tmp("manifest.ron", body);
        let mut report = ValidationReport::default();
        validate_item_manifest(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("category mismatch"));
    }

    #[test]
    fn item_manifest_rejects_missing_registered_id() {
        let body = r#"(
  schema_version: 1,
  items: [
    (id: "rifle_m1", category: "weapon"),
  ],
)"#;
        let path = write_tmp("manifest.ron", body);
        let mut report = ValidationReport::default();
        validate_item_manifest(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("mirror drift"));
    }

    #[test]
    fn item_manifest_rejects_bad_schema_version() {
        let body = r#"(
  schema_version: 2,
  items: [
    (id: "rifle_m1", category: "weapon"),
  ],
)"#;
        let path = write_tmp("manifest.ron", body);
        let mut report = ValidationReport::default();
        validate_item_manifest(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
    }

    #[test]
    fn item_spec_ron_accepts_registered_id() {
        let body = r#"(
  id: "rifle_m1",
  display_name: "Rifle (M1)",
  mass_kg: 3.5,
  dimensions: (w: 2, h: 4),
  bulk_volume_l: 3.0,
  stackable: false,
  max_stack: 1,
  category: weapon,
  container_capacity: None,
  liquid_capacity_l: None,
  rotation_allowed: true,
  quick_slot_eligible: true,
  durability_max: Some(1000),
  repair_recipe: Some("repair.rifle_m1"),
  material_weight_breakdown: {},
  crafting_yield_count: 1,
  origin_compatibility: [],
  forbid_for_origin: [],
)"#;
        let path = write_tmp("rifle_m1.ron", body);
        let mut report = ValidationReport::default();
        validate_item_spec_ron(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.pass(), 1, "report: {:?}", report.entries);
        assert_eq!(report.fail(), 0);
    }

    #[test]
    fn item_spec_ron_rejects_unknown_id() {
        let body = r#"(
  id: "made_up_item",
  display_name: "Made Up",
  mass_kg: 1.0,
  dimensions: (w: 1, h: 1),
  bulk_volume_l: 1.0,
  stackable: false,
  max_stack: 1,
  category: weapon,
  container_capacity: None,
  liquid_capacity_l: None,
  rotation_allowed: true,
  quick_slot_eligible: false,
  durability_max: None,
  repair_recipe: None,
  material_weight_breakdown: {},
  crafting_yield_count: 1,
  origin_compatibility: [],
  forbid_for_origin: [],
)"#;
        let path = write_tmp("made_up_item.ron", body);
        let mut report = ValidationReport::default();
        validate_item_spec_ron(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("not registered"));
    }

    #[test]
    fn item_spec_ron_rejects_filename_mismatch() {
        let body = r#"(
  id: "rifle_m1",
  display_name: "Rifle (M1)",
  mass_kg: 3.5,
  dimensions: (w: 2, h: 4),
  bulk_volume_l: 3.0,
  stackable: false,
  max_stack: 1,
  category: weapon,
  container_capacity: None,
  liquid_capacity_l: None,
  rotation_allowed: true,
  quick_slot_eligible: true,
  durability_max: Some(1000),
  repair_recipe: None,
  material_weight_breakdown: {},
  crafting_yield_count: 1,
  origin_compatibility: [],
  forbid_for_origin: [],
)"#;
        let path = write_tmp("wrong_filename.ron", body);
        let mut report = ValidationReport::default();
        validate_item_spec_ron(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("mismatches filename"));
    }

    #[test]
    fn item_spec_ron_rejects_zero_dimensions() {
        let body = r#"(
  id: "rifle_m1",
  display_name: "Rifle (M1)",
  mass_kg: 3.5,
  dimensions: (w: 0, h: 4),
  bulk_volume_l: 3.0,
  stackable: false,
  max_stack: 1,
  category: weapon,
  container_capacity: None,
  liquid_capacity_l: None,
  rotation_allowed: true,
  quick_slot_eligible: true,
  durability_max: None,
  repair_recipe: None,
  material_weight_breakdown: {},
  crafting_yield_count: 1,
  origin_compatibility: [],
  forbid_for_origin: [],
)"#;
        let path = write_tmp("rifle_m1.ron", body);
        let mut report = ValidationReport::default();
        validate_item_spec_ron(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("dimensions"));
    }
}
