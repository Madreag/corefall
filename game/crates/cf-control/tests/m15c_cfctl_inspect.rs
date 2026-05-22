//! M15C § "Per-material properties accessible via cfctl":
//! `cfctl inspect.material iron` — by-name lookup resolves to the canonical
//! id and returns the full MaterialDef property dump (hardness=8,
//! density=7870, specific_heat=449, thermal_conductivity=80.4).

use cf_control::{server::EngineHandle, M0Engine, M0EngineConfig, Scenario};

fn locate_scenario(id: &str) -> std::path::PathBuf {
    let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    workspace.join("content").join("scenarios").join(format!("{id}.ron"))
}

fn engine_for(id: &str) -> M0Engine {
    let path = locate_scenario(id);
    let scenario = Scenario::load_from_file(&path).expect("scenario parses");
    let config = M0EngineConfig::for_loaded_scenario(&scenario, path);
    M0Engine::new(config)
}

/// VAL-M15C-CFCTL-001: `cfctl inspect.material iron` resolves the name to
/// id 68 and returns the full MaterialDef dump with M15C ONI-parity
/// numbers (hardness=8, density_kg_per_m3=7870, specific_heat=449,
/// thermal_conductivity=80.4).
#[tokio::test]
async fn m15c_cfctl_inspect_material_by_name_iron() {
    let engine = engine_for("m14e_tunnel_collapse_drill");
    let resolved = engine.resolve_material_id_by_name("iron").await;
    assert_eq!(resolved, Some(68), "iron name must resolve to id 68");
    let value = engine
        .inspect_material(resolved.unwrap())
        .await
        .expect("inspect.material returns a payload");
    let obj = value.as_object().expect("payload is an object");
    assert_eq!(obj.get("hardness").and_then(|v| v.as_f64()), Some(8.0));
    assert_eq!(obj.get("density_kg_per_m3").and_then(|v| v.as_f64()), Some(7870.0));
    assert_eq!(
        obj.get("specific_heat_capacity_j_per_kg_k").and_then(|v| v.as_f64()),
        Some(449.0)
    );
    let k = obj.get("thermal_conductivity_w_per_m_k").and_then(|v| v.as_f64()).unwrap();
    assert!((k - 80.4).abs() < 1e-3);
}

/// VAL-M15C-CFCTL-002: `resolve_material_id_by_name("steel")` resolves to
/// id 69 and the dump carries the spec hardness=12 + density=7800.
#[tokio::test]
async fn m15c_cfctl_inspect_material_by_name_steel() {
    let engine = engine_for("m14e_tunnel_collapse_drill");
    let resolved = engine.resolve_material_id_by_name("steel").await;
    assert_eq!(resolved, Some(69));
    let value = engine.inspect_material(resolved.unwrap()).await.expect("payload");
    let obj = value.as_object().expect("object");
    assert_eq!(obj.get("hardness").and_then(|v| v.as_f64()), Some(12.0));
    assert_eq!(obj.get("density_kg_per_m3").and_then(|v| v.as_f64()), Some(7800.0));
    assert_eq!(obj.get("state").and_then(|v| v.as_str()), Some("solid"));
}

/// VAL-M15C-CFCTL-003: unknown material names resolve to `None`.
#[tokio::test]
async fn m15c_cfctl_inspect_material_by_name_unknown() {
    let engine = engine_for("m14e_tunnel_collapse_drill");
    let resolved = engine.resolve_material_id_by_name("definitely_not_a_real_material").await;
    assert!(resolved.is_none());
}

/// VAL-M15C-CFCTL-004: cross-reference id-based and name-based lookups —
/// every canonical M15C-roster name resolves to a stable id, and the same
/// id round-trips back through the inspect call to return identical
/// metadata.
#[tokio::test]
async fn m15c_cfctl_name_id_roundtrip_for_spec_roster() {
    let engine = engine_for("m14e_tunnel_collapse_drill");
    for name in [
        "iron", "steel", "stainless_steel", "brass", "bronze", "aluminum",
        "titanium", "copper", "nickel", "lead", "tin", "zinc", "magnesium",
        "lithium", "silver", "platinum", "tungsten", "uranium_fuel_rod",
        "depleted_uranium", "plutonium", "diamond", "graphene", "vacuum",
        "lime", "quicklime", "phosphorite", "cement_powder", "flour",
        "compost", "dirt_fine", "plasma_jet", "welding_plasma",
        "sunlight", "em_field", "magnetic_field", "ir_signature",
        "radioactive_emission",
    ] {
        let id = engine
            .resolve_material_id_by_name(name)
            .await
            .unwrap_or_else(|| panic!("name {name} must resolve"));
        let payload = engine
            .inspect_material(id)
            .await
            .unwrap_or_else(|| panic!("inspect by id {id} must return a payload"));
        let p_obj = payload.as_object().unwrap();
        assert_eq!(
            p_obj.get("id").and_then(|v| v.as_u64()),
            Some(id as u64),
            "id mismatch for {name}"
        );
        assert_eq!(
            p_obj.get("name").and_then(|v| v.as_str()),
            Some(name),
            "name mismatch for {name}"
        );
    }
}
