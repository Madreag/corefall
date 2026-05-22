use std::{fs, path::Path};

use crate::report::ValidationReport;

pub(crate) mod difficulty;
pub(crate) mod event_schema;
pub(crate) mod fortification;
pub(crate) mod inventory;
pub(crate) mod ledger;
pub(crate) mod loadout;
pub(crate) mod m14a;
pub(crate) mod m6_registry;
pub(crate) mod material;
pub(crate) mod medical;
pub(crate) mod roles;
pub(crate) mod scenario;
pub(crate) mod trench;

use event_schema::{is_envelope_schema_file, is_event_schema_file, validate_envelope_schema_file, validate_event_schema_file};

/// BP4 + BP5 content surfaces. Paths under any of these directories must FAIL
/// validation (not WARN) because the content owners will start landing real
/// manifests as the listed milestones ship, and a silent WARN would let
/// half-broken or schema-drifted manifests sneak into run-bundle evidence.
/// (path-component, owning_milestone).
///
/// (cf-material loader) since M2 ships the v1 schema. Other BP4+ content
/// types remain strict-fail until their milestones land.
const STRICT_FAIL_CONTENT_CATEGORIES: &[(&str, &str)] = &[
    ("chassis", "M5"),
    ("atmospheres", "M5.9 / M7.5"),
    ("worlds", "M5.10"),
    ("origins", "M5"),
];

pub(crate) fn walk(dir: &Path, report: &mut ValidationReport) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(err) => {
            report.add_error(dir.to_path_buf(), format!("read_dir failed: {err}"));
            return;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, report);
        } else if path.extension().and_then(|s| s.to_str()) == Some("ron")
            || (path.extension().and_then(|s| s.to_str()) == Some("json")
                && path.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str()) == Some("ai")
                && path.file_name().and_then(|s| s.to_str()) == Some("difficulty.json"))
            || (path.extension().and_then(|s| s.to_str()) == Some("json")
                && path.components().any(|c| c.as_os_str() == "materials"))
            || path.file_name().and_then(|s| s.to_str()) == Some("ledger.jsonl")
            // descriptors validated against `cf_equipment::LoadoutFile`.
            || (path.extension().and_then(|s| s.to_str()) == Some("json")
                && path.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str()) == Some("loadouts"))
            || (path.extension().and_then(|s| s.to_str()) == Some("json")
                && path.file_name().and_then(|s| s.to_str()) == Some("roles.json")
                && path.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str()) == Some("equipment"))
            // JSON schemas — every <family>_<type>.json under schemas/event/
            // plus the envelope schemas under schemas/v0_1/ + schemas/v1/.
            || is_event_schema_file(&path)
            || is_envelope_schema_file(&path)
        {
            validate_one(&path, report);
        }
    }
}

pub(crate) fn validate_one(path: &Path, report: &mut ValidationReport) {
    if path.file_name().and_then(|s| s.to_str()) == Some("ledger.jsonl") {
        ledger::validate_ledger_jsonl(path, report);
        return;
    }
    // `content/asset_ledger/regen_manifest.ron` against its locked v1.0.0
    // schema (`cf-asset-ledger/schemas/v1/regen_manifest.schema.json`). Each
    // pipeline entry must declare pipeline_id / regen_command / model_version
    // / deterministic; the rest of the schema's fields are optional.
    if path.file_name().and_then(|s| s.to_str()) == Some("regen_manifest.ron") {
        medical::validate_regen_manifest(path, report);
        return;
    }
    // `content/balance/ttd_floors_interim.ron` against the minimal
    // shape locked in cf-actor::ttd. The validator only confirms the
    // schema_version and that floors/compound_modifiers are non-empty
    // tagged tuples per the file's documented schema; the canonical
    // structural validator is the live cf-actor M17 loader.
    if path.file_name().and_then(|s| s.to_str()) == Some("ttd_floors_interim.ron") {
        medical::validate_ttd_floors_interim(path, report);
        return;
    }
    if is_event_schema_file(path) {
        validate_event_schema_file(path, report);
        return;
    }
    if is_envelope_schema_file(path) {
        validate_envelope_schema_file(path, report);
        return;
    }
    if path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        == Some("wound_specs")
        && path.extension().and_then(|s| s.to_str()) == Some("ron")
    {
        medical::validate_wound_spec_ron(path, report);
        return;
    }
    if path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        == Some("treatments")
        && path.extension().and_then(|s| s.to_str()) == Some("ron")
    {
        medical::validate_treatment_spec_ron(path, report);
        return;
    }
    if path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        == Some("prosthetics")
        && path.extension().and_then(|s| s.to_str()) == Some("ron")
    {
        medical::validate_prosthetic_spec_ron(path, report);
        return;
    }
    // This must come BEFORE the scenarios fallthrough so the registry RONs
    // aren't mis-routed to `validate_scenario`.
    if path.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str()) == Some("equipment") {
        match path.file_name().and_then(|s| s.to_str()) {
            Some("weapon_registry.ron") => {
                m6_registry::validate_weapon_registry(path, report);
                return;
            }
            Some("grenade_registry.ron") => {
                m6_registry::validate_grenade_registry(path, report);
                return;
            }
            Some("melee_registry.ron") => {
                m6_registry::validate_melee_registry(path, report);
                return;
            }
            Some("tool_registry.ron") => {
                m6_registry::validate_tool_registry(path, report);
                return;
            }
            Some("roles.json") => {
                // M13: roles.json mirrors cf_equipment::role_records(). The
                // file is presentation-only (mod tools + scenario editors);
                // structural validation only checks well-formed JSON +
                // top-level shape so an edit doesn't accidentally break the
                // export tool. Live role definitions remain authoritative
                // in cf_equipment::role_records().
                roles::validate_roles_json(path, report);
                return;
            }
            _ => {}
        }
    }
    // canonical [`cf_equipment::LoadoutFile`] schema (schema_version, role-id
    // resolution, id↔filename parity). See spec § "Equipment loadouts are
    // data-driven".
    if path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        == Some("loadouts")
        && path
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            == Some("equipment")
        && path.extension().and_then(|s| s.to_str()) == Some("json")
    {
        loadout::validate_loadout_file(path, report);
        return;
    }
    // "Validate `content/equipment/items/*.ron` against ItemSpec
    // schema". `manifest.ron` is validated against the canonical
    // `cf_equipment::item_spec` registry (mirror drift detection);
    // any other `*.ron` file in the items dir is validated as a
    // standalone `cf_equipment::ItemSpec` definition.
    //
    // per-category folder. The per-category folders
    // (`content/equipment/<category>/*.ron`) hold standalone
    // `cf_equipment::ItemSpec` files split per-category for modder
    // ergonomics; the validator routes them through the same
    // `validate_item_spec_ron` path used by `items/<id>.ron` so the
    // schema lock applies uniformly.
    let parent_name = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str());
    let grandparent_name = path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str());
    if grandparent_name == Some("equipment")
        && path.extension().and_then(|s| s.to_str()) == Some("ron")
    {
        if parent_name == Some("items") {
            if path.file_name().and_then(|s| s.to_str()) == Some("manifest.ron") {
                inventory::validate_item_manifest(path, report);
            } else {
                inventory::validate_item_spec_ron(path, report);
            }
            return;
        }
        if matches!(
            parent_name,
            Some("firearms")
                | Some("melee")
                | Some("grenades")
                | Some("heavy")
                | Some("medical")
                | Some("survival")
                | Some("sensors")
                | Some("ppe")
        ) {
            inventory::validate_item_spec_ron(path, report);
            return;
        }
    }
    // `scenarios` fallthrough so `content/trench_segments/*.ron`,
    // `content/trench_modules/*.ron`, and `content/trench_templates/*.trench.ron`
    // are validated through the cf-trench / cf-content loaders (which
    // reject unknown enums + negative depth with typed errors per
    // VAL-M9B-MOD-SEGMENT-001 / VAL-M9B-TEMPLATE-003).
    if path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        == Some("trench_segments")
        && path.extension().and_then(|s| s.to_str()) == Some("ron")
    {
        trench::validate_trench_segment(path, report);
        return;
    }
    if path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        == Some("trench_modules")
        && path.extension().and_then(|s| s.to_str()) == Some("ron")
    {
        trench::validate_trench_module(path, report);
        return;
    }
    if path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        == Some("trench_templates")
        && path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|n| n.ends_with(".trench.ron"))
            .unwrap_or(false)
    {
        trench::validate_trench_template(path, report);
        return;
    }
    // route fortification RONs under `content/fortifications/*.ron`
    // through the cf-fortification FortificationSpec loader. Unknown
    // FortificationKind / wire_kind / mine_kind / anti_tank_kind enum
    // values fail with typed errors; references to dependencies that
    // are not yet shipped (e.g. M9B trench segment ids when M9B is
    // absent) surface as WARN entries with kind `missing_dependency`
    // rather than FAIL.
    if path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        == Some("fortifications")
        && path.extension().and_then(|s| s.to_str()) == Some("ron")
    {
        fortification::validate_fortification(path, report);
        return;
    }
    // content/jetpacks/*.ron + content/quick_action_layouts/*.ron".
    if path.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str()) == Some("limb_paths")
        && path.extension().and_then(|s| s.to_str()) == Some("ron")
    {
        m14a::validate_m14a_limb_path(path, report);
        return;
    }
    if path.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str()) == Some("jetpacks")
        && path.extension().and_then(|s| s.to_str()) == Some("ron")
    {
        m14a::validate_m14a_jetpack(path, report);
        return;
    }
    if path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        == Some("quick_action_layouts")
        && path.extension().and_then(|s| s.to_str()) == Some("ron")
    {
        m14a::validate_m14a_quick_action_layout(path, report);
        return;
    }
    if path.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str()) == Some("scenarios")
        || path
            .components()
            .any(|c| c.as_os_str().to_string_lossy().contains("scenarios"))
    {
        scenario::validate_scenario(path, report);
        return;
    }
    if path.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str()) == Some("ai")
        && path.file_name().and_then(|s| s.to_str()) == Some("difficulty.json")
    {
        difficulty::validate_difficulty_json(path, report);
        return;
    }
    let path_components: Vec<String> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    let is_material_file =
        path_components.iter().any(|c| c == "materials") && path.extension().and_then(|s| s.to_str()) == Some("json");
    if is_material_file {
        material::validate_material_registry(path, report);
        return;
    }
    if let Some((category, milestone)) = STRICT_FAIL_CONTENT_CATEGORIES
        .iter()
        .find(|(cat, _)| path_components.iter().any(|c| c == cat))
    {
        report.add_error(
            path.to_path_buf(),
            format!(
                "cf-mod validator does not yet support {category}/* content — owning milestone is {milestone}. \
                 Until that lands, content/{category}/ files cannot be validated. Move them out or remove them."
            ),
        );
        return;
    }
    report.add_warn(
        path.to_path_buf(),
        "no validator wired for this content type yet (M0 only validates content/scenarios/*.ron)".to_string(),
    );
}
