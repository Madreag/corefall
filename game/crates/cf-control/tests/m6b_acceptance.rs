//! **M6B**: ItemSpec canonicalization + per-item weight + grid dimensions +
//! inventory encumbrance — acceptance tests.
//!
//! Each test mirrors one Gherkin scenario from `specs/active/M6B.md`
//! § "Acceptance criteria". The tests exercise the cf-equipment +
//! cf-actor primitives directly (pure helpers) and verify per-actor
//! grid + encumbrance state contracts.

use cf_actor::{
    sim::{step_no_rng, ActorSimState, StepDeps},
    ActorId, ActorState, ControlIntent, IntentSource, Inventory, InventoryGrid, Vec2,
};
use cf_equipment::{
    encumbrance_band, liquid_fill_mass, max_carry_kg_for_origin, quick_slot_eligible_ids, spec_for_id,
    try_nest_depth, walk_speed_multiplier, BackpackTier, EncumbranceBand, ItemSpec, MAX_CONTAINER_NEST_DEPTH,
    MAX_DEPTH_EXCEEDED,
};
use std::collections::BTreeMap;

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

#[test]
fn scenario_origin_change_rebaselines_max_carry_on_next_recompute() {
    // M17 forward-compat: actor changes origin mid-game (e.g., brain
    // hops into a drone chassis). `recompute_inventory_encumbrance`
    // must refresh max_carry_kg from the new origin so the
    // walk-speed multiplier reflects the new envelope on the next
    // tick — no extra cfctl plumbing required.
    let inv = Inventory::with_rifle("rifle_m1_default");
    let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
    actor.inventory_grid_attach();
    assert!((actor.max_carry_kg() - 50.0).abs() < 1e-6);
    // Swap origin: human → drone (max_carry_kg becomes 15.0 = 50 × 0.3).
    actor.origin_id = "drone".to_string();
    actor.recompute_inventory_encumbrance();
    assert!((actor.max_carry_kg() - 15.0).abs() < 1e-6);
    // Swap origin: drone → heavy_biomech (max_carry_kg becomes 75.0).
    actor.origin_id = "heavy_biomech".to_string();
    actor.recompute_inventory_encumbrance();
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
    let back: ItemSpec = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(spec, back);
}

// ============================================================================
// Engine-level integration: 100-tick deterministic movement under load
// ============================================================================

fn make_step_deps() -> StepDeps {
    StepDeps {
        tick_dt: 1.0 / 60.0,
        region_min_x: 0.0,
        region_max_x: 4096.0,
        region_max_y: 4096.0,
        auto_reload_when_empty: false,
        tuning: None,
        tutorial_safety: false,
    }
}

fn make_loaded_actor(start_x: f32, encumbered: bool) -> ActorState {
    let inv = Inventory::with_rifle("rifle_m1_default");
    let mut a = ActorState::player(ActorId(1), "blue", Vec2::new(start_x, 16.0), 100.0, inv);
    a.on_ground = true;
    a.inventory_grid_attach();
    if encumbered {
        // Fill ~50 kg = full encumbrance for a human (max_carry_kg = 50).
        for _ in 0..15 {
            a.inventory_grid_mut().unwrap().add_top_level("rifle_m1", 1, 0.0);
        }
    }
    a.recompute_inventory_encumbrance();
    a
}

#[test]
fn scenario_determinism_engine_100_ticks_two_runs_identical_positions() {
    // **Spec § Acceptance criteria**: "Given identical seed + identical
    // inventory load, when 100 ticks of movement elapse, then identical
    // actor positions per tick".
    //
    // This is the engine-level proof: two independent ActorSimState
    // instances seeded with the same actor + same load + same control
    // intents produce bit-identical positions per tick across 100
    // ticks of horizontal movement.
    let mut state_a = ActorSimState::new(cf_actor::ActorWorld::new(0.0, -980.0));
    let mut state_b = ActorSimState::new(cf_actor::ActorWorld::new(0.0, -980.0));
    state_a.world.insert(make_loaded_actor(50.0, true));
    state_b.world.insert(make_loaded_actor(50.0, true));

    let deps = make_step_deps();
    let mut positions_a: Vec<(f32, f32)> = Vec::with_capacity(100);
    let mut positions_b: Vec<(f32, f32)> = Vec::with_capacity(100);
    for _ in 0..100 {
        // The sim drains intents each step (`intents.remove`), so we
        // re-insert the constant move-x intent every tick.
        let mut intents_a = BTreeMap::new();
        let mut intents_b = BTreeMap::new();
        let mut intent = ControlIntent::new(ActorId(1), IntentSource::Human);
        intent.move_x = 1.0;
        intents_a.insert(ActorId(1), intent.clone());
        intents_b.insert(ActorId(1), intent);
        let _ = step_no_rng(&mut state_a, &mut intents_a, deps);
        let _ = step_no_rng(&mut state_b, &mut intents_b, deps);
        let pa = state_a.world.actors[&ActorId(1)].position;
        let pb = state_b.world.actors[&ActorId(1)].position;
        positions_a.push((pa.x, pa.y));
        positions_b.push((pb.x, pb.y));
    }
    assert_eq!(positions_a, positions_b, "encumbered movement must be deterministic");
    // Sanity: actor moved (didn't stall).
    assert!(
        positions_a.last().unwrap().0 > 60.0,
        "actor must move forward (got x={})",
        positions_a.last().unwrap().0
    );
}

#[test]
fn scenario_encumbrance_reduces_actor_displacement_vs_unloaded() {
    // Cross-check on the sim path: an encumbered actor (50 kg → 0.5×
    // walk speed) MUST travel less far over 100 ticks than an unloaded
    // actor (1.0× walk speed) given identical move-x input.
    let mut state_loaded = ActorSimState::new(cf_actor::ActorWorld::new(0.0, -980.0));
    let mut state_empty = ActorSimState::new(cf_actor::ActorWorld::new(0.0, -980.0));
    state_loaded.world.insert(make_loaded_actor(50.0, true));
    state_empty.world.insert(make_loaded_actor(50.0, false));

    let deps = make_step_deps();
    for _ in 0..100 {
        // Re-insert the constant move-x intent every tick (the sim
        // drains intents via `remove` each step).
        let mut intents_l = BTreeMap::new();
        let mut intents_e = BTreeMap::new();
        let mut intent = ControlIntent::new(ActorId(1), IntentSource::Human);
        intent.move_x = 1.0;
        intents_l.insert(ActorId(1), intent.clone());
        intents_e.insert(ActorId(1), intent);
        let _ = step_no_rng(&mut state_loaded, &mut intents_l, deps);
        let _ = step_no_rng(&mut state_empty, &mut intents_e, deps);
    }
    let loaded_x = state_loaded.world.actors[&ActorId(1)].position.x;
    let empty_x = state_empty.world.actors[&ActorId(1)].position.x;
    assert!(
        loaded_x < empty_x,
        "encumbered actor must travel less far ({loaded_x} vs {empty_x})"
    );
    // Verify the ratio is roughly in the spec-mandated 0.5× zone (allow
    // ±15% slack for steady-state friction / ground-acceleration differences).
    let displacement_loaded = loaded_x - 50.0;
    let displacement_empty = empty_x - 50.0;
    let ratio = displacement_loaded / displacement_empty;
    assert!(
        (0.4..=0.65).contains(&ratio),
        "displacement ratio {ratio:.3} should be near 0.5 (got loaded={displacement_loaded:.2} / empty={displacement_empty:.2})"
    );
}

// ============================================================================
// observe.actor.inventory_grid surfaces canonical mass + bulk
// ============================================================================

#[test]
fn observe_actor_inventory_grid_surfaces_mass_and_bulk() {
    // Spec § Crates / modules touched: "cf-control MODIFY — observe.actor.inventory extended with mass + bulk".
    let inv = Inventory::with_rifle("rifle_m1_default");
    let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
    actor.inventory_grid_attach();
    actor.inventory_grid_mut().unwrap().add_top_level("rifle_m1", 1, 0.0);
    actor.inventory_grid_mut().unwrap().add_top_level("water_bottle", 1, 0.5);
    actor.recompute_inventory_encumbrance();
    let view = actor.inventory_grid_view().expect("grid view present");
    assert_eq!(view.tier, "small");
    assert_eq!(view.grid_w, 4);
    assert_eq!(view.grid_h, 6);
    assert_eq!(view.placements.len(), 2);
    // Per-placement mass + bulk surfaced from registry.
    let rifle_view = view.placements.iter().find(|p| p.item_id == "rifle_m1").unwrap();
    assert!((rifle_view.mass_kg - 3.5).abs() < 1e-4);
    assert!((rifle_view.bulk_volume_l - 3.0).abs() < 1e-4);
    assert!(rifle_view.quick_slot_eligible);
    assert_eq!(rifle_view.category, "weapon");
    let bottle_view = view.placements.iter().find(|p| p.item_id == "water_bottle").unwrap();
    assert!((bottle_view.mass_kg - 0.7).abs() < 1e-4);
    assert!((bottle_view.current_liquid_l - 0.5).abs() < 1e-4);
    assert!((view.total_mass_kg - 4.2).abs() < 1e-4);
}

#[test]
fn extended_inventory_view_carries_bulk_from_registry() {
    // Spec § Crates / modules touched: "observe.actor.inventory extended with mass + bulk"
    // — ExtendedInventorySlotView now carries `bulk_volume_l`.
    let inv = Inventory::with_rifle("rifle_m1");
    let actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
    let view = actor.extended_inventory_view();
    let occupied: Vec<_> = view.iter().filter(|s| s.state == "occupied").collect();
    assert_eq!(occupied.len(), 1);
    let slot = occupied[0];
    assert_eq!(slot.item_id, "rifle_m1");
    // From the ItemSpec registry: mass=3.5, bulk=3.0.
    assert!((slot.weight_kg - 3.5).abs() < 1e-4);
    assert!((slot.bulk_volume_l - 3.0).abs() < 1e-4);
}

// ============================================================================
// quick_slot_eligible filter for M14A QAB
// ============================================================================

#[test]
fn quick_slot_eligible_ids_excludes_containers() {
    // Spec § Player-facing behavior: "Hot-swap M14A QAB items declare
    // `quick_slot_eligible = true`".
    let qs = quick_slot_eligible_ids();
    assert!(qs.contains(&"rifle_m1".to_string()));
    assert!(qs.contains(&"medkit".to_string()));
    assert!(qs.contains(&"water_bottle".to_string()));
    assert!(!qs.contains(&"chest".to_string()));
    assert!(!qs.contains(&"backpack_small".to_string()));
}

// ============================================================================
// stack_mass helper
// ============================================================================

#[test]
fn stack_mass_matches_spec_formula() {
    // Spec § Player-facing behavior: "stack_mass = item_mass × count".
    let ammo = spec_for_id("ammo_5_56x45").unwrap();
    assert!((ammo.stack_mass(30) - 0.36).abs() < 1e-4);
    assert!((ammo.stack_mass(60) - 0.72).abs() < 1e-4);
}
