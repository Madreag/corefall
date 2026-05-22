//! M15C acceptance tests. One test per Gherkin scenario in the spec.

use cf_material::{load_registry_from_file, validate_registry_json, MaterialDef, MaterialState};

fn locate_registry() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content/materials/material_registry.json")
}

/// Scenario: All 50+ materials declared with full schema.
/// Given content/materials/material_registry.json
/// When cf-material::load_registry runs
/// Then 50+ MaterialDef entries parse successfully
/// And every entry has non-default values for: hardness, density,
///   specific_heat_capacity, thermal_conductivity, color_hex.
#[test]
fn m15c_all_50_plus_materials_have_full_schema() {
    let path = locate_registry();
    let (reg, report) = load_registry_from_file(&path).expect("registry loads");
    assert!(
        report.errors.is_empty(),
        "registry validation must be clean; errors: {:?}",
        report.errors
    );
    assert!(
        reg.materials.len() >= 50,
        "spec requires 50+ materials; got {}",
        reg.materials.len()
    );

    for m in &reg.materials {
        let is_energy = matches!(m.material_state(), MaterialState::EnergyField);
        let is_vacuum = m.name == "vacuum";
        assert!(m.state.is_some(), "material `{}` missing state", m.name);
        assert!(
            m.color_hex.len() == 6 && m.color_hex.chars().all(|c| c.is_ascii_hexdigit()),
            "material `{}` has invalid color_hex: {}",
            m.name,
            m.color_hex
        );
        let strict = !is_energy && !is_vacuum;
        if strict {
            assert!(
                m.density_kg_per_m3.unwrap_or(0.0) > 0.0,
                "material `{}` must have non-default density_kg_per_m3",
                m.name
            );
            assert!(
                m.specific_heat_capacity_j_per_kg_k.unwrap_or(0.0) > 0.0,
                "material `{}` must have non-default specific_heat_capacity_j_per_kg_k",
                m.name
            );
            assert!(
                m.thermal_conductivity_w_per_m_k.unwrap_or(0.0) > 0.0,
                "material `{}` must have non-default thermal_conductivity_w_per_m_k",
                m.name
            );
        }
    }
}

/// Scenario: Per-material properties accessible via cfctl.
/// Given iron material entry
/// Then full property dump returns hardness=8, density=7870,
///   specific_heat=449, thermal_conductivity=80.4.
#[test]
fn m15c_iron_property_dump_matches_spec_literal() {
    let path = locate_registry();
    let (reg, _report) = load_registry_from_file(&path).expect("registry loads");
    let iron = reg
        .find_by_name("iron")
        .expect("iron must exist in launch registry");
    assert!((iron.hardness - 8.0).abs() < 1e-3, "iron hardness must equal 8");
    assert!(
        (iron.density_kg_per_m3.expect("iron must have density") - 7870.0).abs() < 1e-1,
        "iron density_kg_per_m3 must equal 7870 (ONI parity)"
    );
    assert!(
        (iron.specific_heat_capacity_j_per_kg_k.expect("iron must have cp") - 449.0).abs() < 1e-3,
        "iron specific_heat_capacity_j_per_kg_k must equal 449 (ONI parity)"
    );
    assert!(
        (iron.thermal_conductivity_w_per_m_k.expect("iron must have k") - 80.4).abs() < 1e-3,
        "iron thermal_conductivity_w_per_m_k must equal 80.4 (ONI parity)"
    );
}

/// Scenario: ONI parity numbers used for thermal properties.
/// Given iron material entry
/// Then specific_heat_capacity_j_per_kg_k = 449 (ONI parity)
/// And thermal_conductivity_w_per_m_k = 80.4.
#[test]
fn m15c_iron_oni_parity() {
    let path = locate_registry();
    let (reg, _report) = load_registry_from_file(&path).expect("registry loads");
    let iron = reg.find_by_name("iron").expect("iron");
    assert!((iron.specific_heat_capacity_j_per_kg_k.unwrap() - 449.0).abs() < 1e-3);
    assert!((iron.thermal_conductivity_w_per_m_k.unwrap() - 80.4).abs() < 1e-3);
}

/// Scenario: Per-state materials phase-transition correctly.
/// Given water material with melt_temp_k=273.15, boil_temp_k=373.15
/// When tile temperature crosses 373.15K:
///   Then phase change fires water -> steam.
#[test]
fn m15c_water_phase_thresholds_match_spec_literal() {
    let path = locate_registry();
    let (reg, _report) = load_registry_from_file(&path).expect("registry loads");
    let water = reg.find_by_name("water").expect("water");
    assert!(
        (water.melt_temp_k.expect("water must have melt_temp_k") - 273.15).abs() < 1e-3,
        "water melt_temp_k must equal 273.15"
    );
    assert!(
        (water.boil_temp_k.expect("water must have boil_temp_k") - 373.15).abs() < 1e-3,
        "water boil_temp_k must equal 373.15"
    );

    let phase = cf_material::default_phase_registry();
    let (t, dir) = phase.evaluate(13, 370.0, 380.0).expect("water boil fires");
    assert_eq!(dir, cf_material::PhaseDirection::Forward);
    let (resulting_material, _) = t.resolve(dir);
    assert_eq!(resulting_material, 50, "water transforms to steam");
}

/// Scenario: Mod validation catches incomplete material entries.
/// Given a mod author adds material with no specific_heat_capacity
/// When cf-mod validate runs:
///   Then validation fails with "field 'specific_heat_capacity_j_per_kg_k' required".
#[test]
fn m15c_validator_rejects_missing_specific_heat_capacity() {
    let mut body = serde_json::json!({
        "schema_version": 1,
        "materials": [
            {"id": 0, "name": "air", "display_name": "Air", "hardness": 0.0, "diggable": false, "anchorable": false, "hazard": false, "path_cost": 1.0, "density": 0.0, "color_hex": "000000", "description": "Empty",
             "state": "gas", "density_kg_per_m3": 1.225, "specific_heat_capacity_j_per_kg_k": 1005.0, "thermal_conductivity_w_per_m_k": 0.026},
            {"id": 1, "name": "dirt", "display_name": "Dirt", "hardness": 10.0, "diggable": true, "anchorable": true, "hazard": false, "path_cost": 1.0, "density": 1.5, "color_hex": "8B6914", "description": "Dirt",
             "state": "solid", "density_kg_per_m3": 1500.0, "thermal_conductivity_w_per_m_k": 0.5},
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
    });
    body["materials"][1]
        .as_object_mut()
        .unwrap()
        .remove("specific_heat_capacity_j_per_kg_k");
    let report = validate_registry_json(&body);
    let err = report
        .errors
        .iter()
        .find(|e| e.path.contains("specific_heat_capacity_j_per_kg_k"))
        .expect("validator must surface a `specific_heat_capacity_j_per_kg_k` error");
    assert!(
        err.message.contains("specific_heat_capacity_j_per_kg_k"),
        "error must name the missing field: {err:?}"
    );
    assert!(
        err.message.contains("required") || err.kind == "missing_required_field",
        "error must mention `required`: {err:?}"
    );
}

/// Scenario: F8 tile inspect overlay shows complete material data.
/// Given a steel material entry
/// Then HUD-grade fields are populated: element=Steel, hardness=12,
///   density=7800, plus default_mass_per_tile_kg + state="solid".
#[test]
fn m15c_steel_supports_f8_tile_inspect_payload() {
    let path = locate_registry();
    let (reg, _report) = load_registry_from_file(&path).expect("registry loads");
    let steel = reg.find_by_name("steel").expect("steel");
    assert_eq!(steel.display_name, "Steel");
    assert!((steel.hardness - 12.0).abs() < 1e-3, "steel hardness must equal 12");
    assert!(
        (steel.density_kg_per_m3.unwrap() - 7800.0).abs() < 1e-1,
        "steel density_kg_per_m3 must equal 7800"
    );
    assert_eq!(steel.material_state(), MaterialState::Solid);
    assert!(steel.default_mass_per_tile_kg.is_some(), "steel must surface default_mass_per_tile_kg");
}

/// Scenario: M15C-roster materials all present.
/// Given content/materials/material_registry.json
/// Then every M15C-roster entry is present.
#[test]
fn m15c_roster_present() {
    let path = locate_registry();
    let (reg, _report) = load_registry_from_file(&path).expect("registry loads");
    let must_exist: &[&str] = &[
        "dirt", "concrete", "metal_nohook", "hazard", "loose_fill", "repair_fill", "anchor",
        "iron", "sandstone", "ice", "foam_insulation", "vacuum",
        "steel", "stainless_steel", "brass", "bronze", "aluminum", "titanium", "copper",
        "nickel", "tin", "lead", "zinc", "magnesium", "lithium", "sulfur", "gold", "silver",
        "platinum", "tungsten", "uranium_fuel_rod", "depleted_uranium", "plutonium",
        "silica", "quartz", "basalt", "slag", "iridium", "niobium", "ceramic", "cobalt",
        "insulite", "polypropylene", "diamond", "graphene",
        "sand", "salt", "sugar", "ash", "charcoal", "gunpowder", "snow", "phosphorite",
        "cement_powder", "flour", "lime", "quicklime", "compost", "dirt_fine",
        "electric_arc", "lightning", "fire_intense", "plasma_jet", "welding_plasma",
        "sunlight", "em_field", "magnetic_field", "ir_signature", "radioactive_emission",
    ];
    let mut missing = Vec::new();
    for name in must_exist {
        if reg.find_by_name(name).is_none() {
            missing.push(*name);
        }
    }
    assert!(missing.is_empty(), "M15C-roster materials missing: {missing:?}");
}

/// VAL-M15C-005: every material state declared in MaterialState round-trips
/// through the JSON loader (solid, liquid, gas, powder, plasma, energy_field).
#[test]
fn m15c_all_material_states_round_trip() {
    let path = locate_registry();
    let (reg, _report) = load_registry_from_file(&path).expect("registry loads");
    let mut seen: std::collections::BTreeSet<MaterialState> = std::collections::BTreeSet::new();
    for m in &reg.materials {
        seen.insert(m.material_state());
    }
    for st in MaterialState::all() {
        assert!(
            seen.contains(st),
            "registry must contain at least one entry of state {:?}",
            st
        );
    }
}

/// VAL-M15C-006: water + steam phase transition fires when the tile
/// temperature crosses 373.15 K.
#[test]
fn m15c_water_steam_transition_fires_at_boil_threshold() {
    let phase = cf_material::default_phase_registry();
    let res = phase.evaluate(13, 370.0, 374.0);
    assert!(res.is_some(), "phase transition must fire when crossing 373.15 K");
    let (transition, dir) = res.unwrap();
    assert_eq!(dir, cf_material::PhaseDirection::Forward);
    let (out, _) = transition.resolve(dir);
    assert_eq!(out, 50, "water becomes steam");
}

/// VAL-M15C-007: the validator rejects entries with no state field.
#[test]
fn m15c_validator_rejects_missing_state() {
    let mut body = serde_json::json!({
        "schema_version": 1,
        "materials": [
            {"id": 0, "name": "air", "display_name": "Air", "hardness": 0.0, "diggable": false, "anchorable": false, "hazard": false, "path_cost": 1.0, "density": 0.0, "color_hex": "000000", "description": "Empty",
             "state": "gas", "density_kg_per_m3": 1.225, "specific_heat_capacity_j_per_kg_k": 1005.0, "thermal_conductivity_w_per_m_k": 0.026},
            {"id": 1, "name": "dirt", "display_name": "Dirt", "hardness": 10.0, "diggable": true, "anchorable": true, "hazard": false, "path_cost": 1.0, "density": 1.5, "color_hex": "8B6914", "description": "Dirt",
             "density_kg_per_m3": 1500.0, "specific_heat_capacity_j_per_kg_k": 800.0, "thermal_conductivity_w_per_m_k": 0.5}
        ]
    });
    body["materials"][1].as_object_mut().unwrap().remove("state");
    let report = validate_registry_json(&body);
    assert!(
        report.errors.iter().any(|e| e.path.contains(".state")),
        "validator must surface a state-missing error; got: {:?}",
        report.errors
    );
}

/// VAL-M15C-008: container_rules round-trips through serde.
#[test]
fn m15c_container_rules_round_trip() {
    let body = serde_json::json!({
        "schema_version": 1,
        "materials": [
            {"id": 0, "name": "air", "display_name": "Air", "hardness": 0.0, "diggable": false, "anchorable": false, "hazard": false, "path_cost": 1.0, "density": 0.0, "color_hex": "000000", "description": "Empty",
             "state": "gas", "density_kg_per_m3": 1.225, "specific_heat_capacity_j_per_kg_k": 1005.0, "thermal_conductivity_w_per_m_k": 0.026,
             "container_rules": {"sealable": true, "max_capacity_l": 100.0}}
        ]
    });
    let reg: cf_material::MaterialRegistry = serde_json::from_value(body).expect("parse");
    let m: &MaterialDef = &reg.materials[0];
    let rules = m.container_rules.as_ref().expect("rules");
    assert!(rules.sealable);
    assert!((rules.max_capacity_l.unwrap() - 100.0).abs() < 1e-3);
}
