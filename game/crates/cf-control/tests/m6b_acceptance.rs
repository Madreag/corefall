//! **M6B**: ItemSpec canonicalization + per-item weight + grid dimensions +
//! inventory encumbrance — acceptance tests.
//!
//! Each test mirrors one Gherkin scenario from `specs/active/M6B.md`
//! § "Acceptance criteria". The tests exercise the cf-equipment +
//! cf-actor primitives directly (pure helpers) and verify per-actor
//! grid + encumbrance state contracts.

use cf_actor::{ActorId, ActorState, Inventory, InventoryGrid, Vec2};
use cf_equipment::{
    encumbrance_band, liquid_fill_mass, max_carry_kg_for_origin, spec_for_id, try_nest_depth, walk_speed_multiplier,
    BackpackTier, EncumbranceBand, MAX_CONTAINER_NEST_DEPTH, MAX_DEPTH_EXCEEDED,
};

// ============================================================================
// Scenario: Item declares mass + dimensions
// ============================================================================

#[test]
fn scenario_item_declares_mass_and_dimensions() {
    // Given rifle_m1 spec with mass=3.5 kg + dimensions 2×4
    let spec = spec_for_id("rifle_m1").expect("rifle_m1 in registry");
    assert!((spec.mass_kg - 3.5).abs() < 1e-6);
    assert_eq!(spec.dimensions.w, 2);
    assert_eq!(spec.dimensions.h, 4);
    assert_eq!(spec.category, cf_equipment::ItemCategory::Weapon);

    // When player picks up rifle → inventory.total_mass += 3.5
    let inv = Inventory::with_rifle("rifle_m1_default");
    let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
    actor.inventory_grid_attach();
    let pre_mass = actor.inventory_grid_total_mass_kg();
    let _ = actor
        .inventory_grid_mut()
        .unwrap()
        .add_top_level("rifle_m1", 1, 0.0);
    actor.recompute_inventory_encumbrance();
    let post_mass = actor.inventory_grid_total_mass_kg();
    assert!((post_mass - pre_mass - 3.5).abs() < 1e-6);

    // actor.M14A.total_mass updated → cf_actor::mass_aggregator::total_mass
    // sees the new inventory contribution.
    let total = cf_actor::total_mass(&actor);
    // chassis(80) + inventory(3.5) = 83.5
    assert!((total - 83.5).abs() < 1e-6);
}

// ============================================================================
// Scenario: Encumbrance at 100% reduces walk speed
// ============================================================================

#[test]
fn scenario_encumbrance_at_100_percent_reduces_walk_speed() {
    // Given a human at max_carry_kg=50 + carrying 50kg
    let mult = walk_speed_multiplier(50.0, 50.0);
    // Then walk_speed_multiplier = 0.5
    assert!((mult - 0.5).abs() < 1e-6);

    // And HUD shows "ENCUMBERED" warning → band = Heavy
    assert_eq!(encumbrance_band(50.0, 50.0), EncumbranceBand::Heavy);

    // The per-actor envelope behavior matches.
    let inv = Inventory::with_rifle("rifle_m1_default");
    let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
    actor.inventory_grid_attach();
    // Fill inventory with 50 kg = ~15 rifles (3.5 kg each → 52.5 kg).
    for _ in 0..15 {
        actor.inventory_grid_mut().unwrap().add_top_level("rifle_m1", 1, 0.0);
    }
    actor.recompute_inventory_encumbrance();
    assert!(actor.is_encumbered(), "actor must be encumbered at 50+ kg / 50 cap");
    assert_eq!(actor.encumbrance_band(), EncumbranceBand::Heavy);
    // Walk speed multiplier clamped at 0.5.
    assert!((actor.encumbrance_walk_speed_multiplier() - 0.5).abs() < 1e-6);
}

#[test]
fn scenario_encumbrance_table_lerp_curve() {
    // Spec § Tunable defaults: walk-speed at 100% = 0.5, at 50% = 0.75, at 0% = 1.0.
    assert!((walk_speed_multiplier(0.0, 50.0) - 1.0).abs() < 1e-6);
    assert!((walk_speed_multiplier(25.0, 50.0) - 0.75).abs() < 1e-6);
    assert!((walk_speed_multiplier(50.0, 50.0) - 0.5).abs() < 1e-6);
}

// ============================================================================
// Scenario: Per-origin scaling applies
// ============================================================================

#[test]
fn scenario_per_origin_scaling_applies() {
    // Given a heavy_biomech with same load → max_carry_kg = 75 (1.5× baseline)
    assert!((max_carry_kg_for_origin("heavy_biomech") - 75.0).abs() < 1e-6);

    // Spec table:
    // | heavy_biomech | 1.5× |
    // | robot         | 1.2× |
    // | drone         | 0.3× |
    // | android/human | 1.0× |
    // 1.2 + 0.3 cannot be represented exactly in IEEE 754 → use a wider
    // tolerance for those (1e-4 is comfortably within 0.01%).
    assert!((max_carry_kg_for_origin("robot") - 60.0).abs() < 1e-4);
    assert!((max_carry_kg_for_origin("drone") - 15.0).abs() < 1e-4);
    assert!((max_carry_kg_for_origin("android") - 50.0).abs() < 1e-6);
    assert!((max_carry_kg_for_origin("human") - 50.0).abs() < 1e-6);

    // And encumbrance threshold differs accordingly: same 50 kg load is
    // Heavy on a human (50/50=1.0) but only Moderate on a heavy_biomech
    // (50/75=0.67) and "None" (light load) on a robot (50/60=0.83).
    assert_eq!(encumbrance_band(50.0, 50.0), EncumbranceBand::Heavy);
    assert_eq!(encumbrance_band(50.0, 75.0), EncumbranceBand::Light);
    assert_eq!(encumbrance_band(50.0, 60.0), EncumbranceBand::Moderate);

    // Walk-speed multiplier differs accordingly.
    assert!((walk_speed_multiplier(50.0, 50.0) - 0.5).abs() < 1e-6);
    let biomech_mult = walk_speed_multiplier(50.0, 75.0);
    assert!(biomech_mult > 0.6 && biomech_mult < 0.7);
}

#[test]
fn scenario_per_origin_actor_envelope_seeded_from_origin() {
    // Per-actor envelope auto-baselines from `origin_id`.
    let inv = Inventory::with_rifle("rifle_m1_default");
    let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
    actor.origin_id = "heavy_biomech".to_string();
    actor.inventory_grid_attach();
    assert!((actor.max_carry_kg() - 75.0).abs() < 1e-6);
}

// ============================================================================
// Scenario: Container nesting depth-limited
// ============================================================================

#[test]
fn scenario_container_nesting_depth_limited_rejects_max_depth_exceeded() {
    // Given a chest (level 1) containing a crate (level 2) — depth 2 is the
    // M6B-locked cap.
    let mut grid = InventoryGrid::default();
    let chest_id = grid.add_top_level("chest", 1, 0.0);
    let crate_id = grid
        .try_nest_container(chest_id, "crate")
        .expect("crate nests in chest");
    assert!(crate_id > 0);

    // When player tries to nest another container inside the crate (would
    // be depth 3), the call rejects with "max_depth_exceeded".
    let result = grid.try_nest_container(crate_id, "crate");
    assert_eq!(result, Err(MAX_DEPTH_EXCEEDED));
}

#[test]
fn scenario_container_nesting_depth_helper_matches_spec() {
    // try_nest_depth pure helper — depth 2 + container child = rejection.
    let result = try_nest_depth(2, MAX_CONTAINER_NEST_DEPTH, true);
    assert_eq!(result, Err(MAX_DEPTH_EXCEEDED));

    // Non-container child at depth 2 + 1 = depth 3 is allowed (items live
    // inside containers without bumping the cap).
    let result = try_nest_depth(2, MAX_CONTAINER_NEST_DEPTH, false);
    assert!(result.is_ok());
}

// ============================================================================
// Scenario: Liquid container full vs empty mass
// ============================================================================

#[test]
fn scenario_liquid_container_full_vs_empty_mass() {
    // Given an empty water_bottle (0.2 kg empty + 1L capacity)
    let spec = spec_for_id("water_bottle").expect("water_bottle in registry");
    assert!((spec.mass_kg - 0.2).abs() < 1e-6);
    assert_eq!(spec.liquid_capacity_l, Some(1.0));

    // When player fills with 1L water → item mass = 1.2 kg
    let mass_full = liquid_fill_mass(&spec, 1.0);
    assert!((mass_full - 1.2).abs() < 1e-6);

    // When player drinks 500ml → item mass = 0.7 kg
    let mass_half = liquid_fill_mass(&spec, 0.5);
    assert!((mass_half - 0.7).abs() < 1e-6);

    // Same behavior at the grid level (so observe.actor.inventory + M14A
    // mass aggregator see live transitions).
    let mut grid = InventoryGrid::default();
    let id = grid.add_top_level("water_bottle", 1, 0.0);
    assert!((grid.total_mass_kg() - 0.2).abs() < 1e-6);
    let new_mass = grid.adjust_liquid(id, 1.0).unwrap();
    assert!((new_mass - 1.2).abs() < 1e-6);
    assert!((grid.total_mass_kg() - 1.2).abs() < 1e-6);
    let after_drink = grid.adjust_liquid(id, -0.5).unwrap();
    assert!((after_drink - 0.7).abs() < 1e-6);
}

// ============================================================================
// Scenario: Determinism — encumbrance reproduced
// ============================================================================

#[test]
fn scenario_determinism_encumbrance_reproduced() {
    // Given identical seed + identical inventory load
    // When 100 ticks of movement elapse
    // Then identical actor positions per tick
    //
    // The encumbrance compute is pure (no RNG, no wall clock). We verify
    // determinism by reproducing the same compute twice and asserting
    // bit-identical outputs.
    let inv = Inventory::with_rifle("rifle_m1_default");
    let mut actor_a = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv.clone());
    let mut actor_b = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
    actor_a.inventory_grid_attach();
    actor_b.inventory_grid_attach();

    // Identical load.
    for _ in 0..5 {
        actor_a.inventory_grid_mut().unwrap().add_top_level("rifle_m1", 1, 0.0);
        actor_b.inventory_grid_mut().unwrap().add_top_level("rifle_m1", 1, 0.0);
    }
    let id_a = actor_a
        .inventory_grid_mut()
        .unwrap()
        .add_top_level("water_bottle", 1, 0.5);
    let id_b = actor_b
        .inventory_grid_mut()
        .unwrap()
        .add_top_level("water_bottle", 1, 0.5);
    assert_eq!(id_a, id_b);
    actor_a.recompute_inventory_encumbrance();
    actor_b.recompute_inventory_encumbrance();

    // After 100 "computed-once" reads, the per-actor envelope values must
    // be bit-identical (no drift / accumulation).
    for _ in 0..100 {
        actor_a.recompute_inventory_encumbrance();
        actor_b.recompute_inventory_encumbrance();
    }
    let a = actor_a.inventory_encumbrance.unwrap();
    let b = actor_b.inventory_encumbrance.unwrap();
    assert_eq!(a, b, "encumbrance envelope must be deterministic");
    assert!((cf_actor::total_mass(&actor_a) - cf_actor::total_mass(&actor_b)).abs() < 1e-6);
}

// ============================================================================
// Schema-locked spec field surfaces
// ============================================================================

#[test]
fn backpack_tier_dimensions_match_spec_table() {
    // Spec § Tunable defaults
    assert_eq!(BackpackTier::Small.dimensions(), (4, 6));
    assert_eq!(BackpackTier::Medium.dimensions(), (6, 8));
    assert_eq!(BackpackTier::Large.dimensions(), (8, 10));
    assert_eq!(BackpackTier::Industrial.dimensions(), (10, 12));
}

#[test]
fn item_spec_schema_round_trips() {
    // Schema lock: serde round-trip must preserve every field.
    let spec = spec_for_id("rifle_m1").unwrap();
    let json = serde_json::to_string(&spec).expect("serialize");
    let back: cf_equipment::ItemSpec = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(spec, back);
}
