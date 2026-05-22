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
        let is_energy = matches!(m.state, MaterialState::EnergyField);
        let is_vacuum = m.name == "vacuum";
        assert!(
            m.color_hex.len() == 6 && m.color_hex.chars().all(|c| c.is_ascii_hexdigit()),
            "material `{}` has invalid color_hex: {}",
            m.name,
            m.color_hex
        );
        let strict = !is_energy && !is_vacuum;
        if strict {
            assert!(
                m.density_kg_per_m3 > 0.0,
                "material `{}` must have non-default density_kg_per_m3",
                m.name
            );
            assert!(
                m.specific_heat_capacity_j_per_kg_k > 0.0,
                "material `{}` must have non-default specific_heat_capacity_j_per_kg_k",
                m.name
            );
            assert!(
                m.thermal_conductivity_w_per_m_k > 0.0,
                "material `{}` must have non-default thermal_conductivity_w_per_m_k",
                m.name
            );
        }
        assert!(m.satisfies_m15c_schema(), "material `{}` failed M15C schema check", m.name);
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
        (iron.density_kg_per_m3 - 7870.0).abs() < 1e-1,
        "iron density_kg_per_m3 must equal 7870 (ONI parity)"
    );
    assert!(
        (iron.specific_heat_capacity_j_per_kg_k - 449.0).abs() < 1e-3,
        "iron specific_heat_capacity_j_per_kg_k must equal 449 (ONI parity)"
    );
    assert!(
        (iron.thermal_conductivity_w_per_m_k - 80.4).abs() < 1e-3,
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
    assert!((iron.specific_heat_capacity_j_per_kg_k - 449.0).abs() < 1e-3);
    assert!((iron.thermal_conductivity_w_per_m_k - 80.4).abs() < 1e-3);
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
    let phase_owned: cf_material::MaterialState = water.state;
    assert_eq!(phase_owned, MaterialState::Liquid, "water must be a liquid");

    let phase = cf_material::default_phase_registry();
    let (t, dir) = phase.evaluate(13, 370.0, 380.0).expect("water boil fires");
    assert_eq!(dir, cf_material::PhaseDirection::Forward);
    let (resulting_material, _) = t.resolve(dir);
    assert_eq!(resulting_material, 50, "water transforms to steam");
}

/// minimal valid M15C launch-set entry: id, name, display_name + the 8
/// pre-M15C required fields + all M15C required-non-Option fields with
/// canonical values.
fn entry(id: u16, name: &str, display: &str, density_kg_per_m3: f64, cp: f64, k: f64) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "name": name,
        "display_name": display,
        "hardness": 1.0,
        "diggable": false,
        "anchorable": false,
        "hazard": false,
        "path_cost": 1.0,
        "density": 1.0,
        "color_hex": "888888",
        "description": display,
        "state": "solid",
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

fn launch_registry_fixture() -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "materials": [
            entry(0, "air", "Air", 1.225, 1005.0, 0.026),
            entry(1, "dirt", "Dirt", 1500.0, 800.0, 0.5),
            entry(2, "concrete", "Concrete", 2300.0, 880.0, 1.7),
            entry(3, "metal_nohook", "Metal", 7800.0, 466.0, 50.0),
            entry(4, "hazard", "Hazard", 3000.0, 700.0, 1.0),
            entry(5, "loose_fill", "Loose", 1200.0, 800.0, 0.4),
            entry(6, "repair_fill", "Repair", 800.0, 1500.0, 0.05),
            entry(7, "anchor", "Anchor", 2600.0, 790.0, 2.5)
        ]
    })
}

/// Scenario: Mod validation catches incomplete material entries.
/// Given a mod author adds material with no specific_heat_capacity
/// When cf-mod validate runs:
///   Then validation fails with "field 'specific_heat_capacity_j_per_kg_k' required".
#[test]
fn m15c_validator_rejects_missing_specific_heat_capacity() {
    let mut body = launch_registry_fixture();
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
        (steel.density_kg_per_m3 - 7800.0).abs() < 1e-1,
        "steel density_kg_per_m3 must equal 7800"
    );
    assert_eq!(steel.state, MaterialState::Solid);
    assert!(
        steel.default_mass_per_tile_kg > 0.0,
        "steel must surface a non-zero default_mass_per_tile_kg"
    );
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
        seen.insert(m.state);
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
    let mut body = launch_registry_fixture();
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
    let mut body = launch_registry_fixture();
    body["materials"][0]["container_rules"] = serde_json::json!({
        "sealable": true,
        "max_capacity_l": 100.0
    });
    let reg: cf_material::MaterialRegistry = serde_json::from_value(body).expect("parse");
    let m: &MaterialDef = &reg.materials[0];
    assert!(m.container_rules.sealable);
    assert!((m.container_rules.max_capacity_l.unwrap() - 100.0).abs() < 1e-3);
}

/// VAL-M15C-009: validator rejects entries missing the M15C non-Option
/// numeric fields (molar_mass, toxicity, corrosiveness, radioactivity,
/// electrical_conductivity, viscosity_pa_s, surface_tension_n_per_m,
/// default/max_mass_per_tile_kg) even when set to 0.
#[test]
fn m15c_validator_requires_all_non_option_numeric_fields() {
    for required_field in [
        "molar_mass_g_per_mol",
        "toxicity",
        "corrosiveness",
        "radioactivity",
        "electrical_conductivity",
        "viscosity_pa_s",
        "surface_tension_n_per_m",
        "default_mass_per_tile_kg",
        "max_mass_per_tile_kg",
    ] {
        let mut body = launch_registry_fixture();
        body["materials"][3].as_object_mut().unwrap().remove(required_field);
        let report = validate_registry_json(&body);
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.path.contains(required_field) && e.message.contains("required")),
            "validator must reject missing `{required_field}`; got: {:?}",
            report.errors
        );
    }
}

/// VAL-M15C-010: every entry in the canonical launch registry exposes the
/// full M15C scalar block as non-Option fields.
#[test]
fn m15c_material_def_fields_are_non_option_per_spec() {
    let path = locate_registry();
    let (reg, _report) = load_registry_from_file(&path).expect("registry loads");
    for m in &reg.materials {
        let _state: cf_material::MaterialState = m.state;
        let _: f32 = m.density_kg_per_m3;
        let _: f32 = m.specific_heat_capacity_j_per_kg_k;
        let _: f32 = m.thermal_conductivity_w_per_m_k;
        let _: f32 = m.molar_mass_g_per_mol;
        let _: f32 = m.toxicity;
        let _: f32 = m.corrosiveness;
        let _: f32 = m.radioactivity;
        let _: f32 = m.electrical_conductivity;
        let _: f32 = m.viscosity_pa_s;
        let _: f32 = m.surface_tension_n_per_m;
        let _: f32 = m.default_mass_per_tile_kg;
        let _: f32 = m.max_mass_per_tile_kg;
        let _: &cf_material::ContainerRules = &m.container_rules;
    }
    let iron = reg.find_by_name("iron").expect("iron must exist");
    assert!((iron.density_kg_per_m3 - 7870.0).abs() < 1e-1);
    assert!((iron.specific_heat_capacity_j_per_kg_k - 449.0).abs() < 1e-3);
    assert!((iron.thermal_conductivity_w_per_m_k - 80.4).abs() < 1e-3);
    assert!((iron.electrical_conductivity - 1.0e7).abs() < 1e3);
}

/// VAL-M15C-011: state assignments match the spec roster — loose_fill is
/// SOLID (not powder), charcoal is POWDER (not solid), sulfur is SOLID.
#[test]
fn m15c_state_assignments_match_spec_roster() {
    let path = locate_registry();
    let (reg, _report) = load_registry_from_file(&path).expect("registry loads");
    assert_eq!(
        reg.find_by_name("loose_fill").unwrap().state,
        MaterialState::Solid,
        "loose_fill must be solid per M15C § Solids roster"
    );
    assert_eq!(
        reg.find_by_name("charcoal").unwrap().state,
        MaterialState::Powder,
        "charcoal must be powder per M15C § Powders roster"
    );
    assert_eq!(
        reg.find_by_name("sulfur").unwrap().state,
        MaterialState::Solid,
        "sulfur must be solid per M15C § Solids roster"
    );
}
