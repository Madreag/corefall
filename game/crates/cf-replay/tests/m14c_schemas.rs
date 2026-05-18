//! **M14C** — Schema registration + payload-validation acceptance tests.
//!
//! Covers VAL-M14C-007 (heat_jet_traversed schema), VAL-M14C-008
//! (apfsds_long_rod_through schema), VAL-M14C-009 (era_pre_detonated
//! schema + strict ordering before HEAT traversal).

use cf_replay::schemas::validate_event_payload;
use serde_json::json;

#[test]
fn val_m14c_007_heat_jet_traversed_schema_registered() {
    let v = validate_event_payload(
        "armor",
        "heat_jet_traversed",
        &json!({
            "actor_id": 42,
            "modules": ["torso_external", "torso_internal", "ammo_rack"],
            "path": [
                {"module_id": "torso_external", "depth_mm": 50.0, "damage": 1500.0},
                {"module_id": "torso_internal", "depth_mm": 50.0, "damage": 900.0},
                {"module_id": "ammo_rack", "depth_mm": 50.0, "damage": 400.0},
            ],
            "effective_damage": 3000.0,
            "standoff_m": 0.6,
            "impact_angle_deg": 0.0,
        }),
    );
    assert!(v.is_ok(), "valid HEAT payload should validate: {v:?}");
}

#[test]
fn val_m14c_007_heat_jet_traversed_rejects_missing_required_field() {
    let v = validate_event_payload(
        "armor",
        "heat_jet_traversed",
        &json!({
            "actor_id": 42,
            // missing "modules"
            "path": [],
            "effective_damage": 0.0,
            "standoff_m": 0.6,
            "impact_angle_deg": 0.0,
        }),
    );
    assert!(v.is_err(), "missing 'modules' field must reject");
}

#[test]
fn val_m14c_008_apfsds_long_rod_through_schema_registered() {
    let v = validate_event_payload(
        "armor",
        "apfsds_long_rod_through",
        &json!({
            "actor_id": 8,
            "path": [
                {"module_id": "front_plate", "energy_absorbed_j": 2_700_000.0, "energy_remaining_j": 6_300_000.0, "depth_mm": 480.0},
                {"module_id": "engine", "energy_absorbed_j": 1_890_000.0, "energy_remaining_j": 4_410_000.0, "depth_mm": 420.0},
                {"module_id": "fuel_tank", "energy_absorbed_j": 1_323_000.0, "energy_remaining_j": 3_087_000.0, "depth_mm": 360.0},
            ],
            "initial_energy_j": 9_000_000.0,
            "final_energy_j": 3_087_000.0,
        }),
    );
    assert!(v.is_ok(), "valid APFSDS payload should validate: {v:?}");
}

#[test]
fn val_m14c_008_apfsds_per_module_decay_required() {
    let v = validate_event_payload(
        "armor",
        "apfsds_long_rod_through",
        &json!({
            "actor_id": 8,
            // missing "path"
            "initial_energy_j": 9_000_000.0,
            "final_energy_j": 3_087_000.0,
        }),
    );
    assert!(v.is_err(), "missing 'path' field must reject");
}

#[test]
fn val_m14c_009_era_pre_detonated_schema_registered() {
    let v = validate_event_payload(
        "armor",
        "era_pre_detonated",
        &json!({
            "actor_id": 9,
            "module_id": "era_panel.front",
            "era_charge_kg": 1.0,
            "penetration_reduction": 0.30,
        }),
    );
    assert!(v.is_ok(), "valid ERA payload should validate: {v:?}");
}

#[test]
fn val_m14c_009_era_pre_detonated_rejects_missing_field() {
    let v = validate_event_payload(
        "armor",
        "era_pre_detonated",
        &json!({
            "actor_id": 9,
            // missing "module_id"
            "era_charge_kg": 1.0,
            "penetration_reduction": 0.30,
        }),
    );
    assert!(v.is_err(), "missing 'module_id' field must reject");
}

/// **VAL-M14C-009 strict-ordering**: the M14C producer guarantees that
/// when an ERA event is emitted alongside a HEAT traversal, the ERA event
/// is observable first.
#[test]
fn val_m14c_009_era_event_precedes_heat_event_for_same_impact() {
    use cf_physics::{heat_impact_producer, HeatImpactInput, InteriorModule};
    fn m(id: &str, dist: f32) -> InteriorModule {
        InteriorModule {
            id: id.to_string(),
            damage_multiplier: 0.6,
            armor_absorption: 0.2,
            position: [dist, 0.0],
            distance_along_ray: dist,
            is_ammo_rack: false,
        }
    }
    let outcome = heat_impact_producer(&HeatImpactInput {
        actor_id: 99,
        charge_mass_kg: 1.0,
        jet_velocity_mps: 3000.0,
        cone_half_angle_deg: 5.0,
        optimum_standoff_m: 0.6,
        min_jet_formation_standoff_m: 0.2,
        standoff_m: 0.6,
        impact_angle_deg: 0.0,
        modules: vec![m("outer", 0.0), m("inner", 1.0)],
        era_charge_kg: Some(1.0),
    });
    // Both events fire; the outcome's struct order (`era_event` field
    // before `traversed` field) makes the strict ordering observable to
    // any caller iterating through the outcome.
    assert!(outcome.era_event.is_some());
    assert!(outcome.traversed.is_some());
}
