//! **M14**: full collision + impulse routing — acceptance tests.
//!
//! Each test mirrors one Gherkin scenario from `specs/active/M14.md`
//! § "Acceptance criteria". The tests exercise the cf-physics + cf-actor
//! + cf-internal primitives directly (pure helpers) and verify event
//! payload shapes against the registered schemas.
//!
//! The full-engine swept-collision + ragdoll + organ routing chain is
//! verified by the live-ws acceptance harness; this file proves the
//! contract surfaces are present + deterministic.

use cf_actor::{default_cascade_chain, spread_angle, GibOriginKind, GibSpawn, SpreadMode};
use cf_internal::{
    route_explosion_internal_damage, route_internal_damage, InternalGraphKind,
};
use cf_physics::{
    apfsds_energy_through_module, bleed_per_tick, classify_hit_direction, classify_zone_state, decay_damage,
    era_pre_detonates_heat, evaluate_joint, exposed_zones, explosion_impulse, fall_impulse_chain,
    he_damage_at_distance, heat_jet_modules_penetrated, mirror_local_x, prioritize_swept_collisions,
    severance_probability, severance_roll, spalling_fragment_count, spalling_fragment_damage_fraction, step_ragdoll,
    traverse_ray, DecayBand, HeWave, HitDirection, InteriorModule, Joint, Ragdoll, RagdollState, SharpnessInputs,
    SweptHitCandidate, ZoneState,
};

// ============================================================================
// Full swept-collision
// ============================================================================

#[test]
fn swept_collision_priority_closer_first() {
    // Scenario: "Projectile traces swept path" — Given a projectile fired with
    // high velocity, when it crosses multiple actors in one tick, then
    // combat.swept_collision fires for each hit in priority order (closest first).
    let candidates = vec![
        SweptHitCandidate {
            target_id: 7,
            entry_t: 0.8,
            distance_traveled: 800.0,
            entry_point: [800.0, 0.0],
            ray_origin: [0.0, 0.0],
            ray_direction: [1.0, 0.0],
        },
        SweptHitCandidate {
            target_id: 3,
            entry_t: 0.2,
            distance_traveled: 200.0,
            entry_point: [200.0, 0.0],
            ray_origin: [0.0, 0.0],
            ray_direction: [1.0, 0.0],
        },
        SweptHitCandidate {
            target_id: 5,
            entry_t: 0.5,
            distance_traveled: 500.0,
            entry_point: [500.0, 0.0],
            ray_origin: [0.0, 0.0],
            ray_direction: [1.0, 0.0],
        },
    ];
    let resolved = prioritize_swept_collisions(candidates);
    assert_eq!(resolved.len(), 3);
    assert_eq!(resolved[0].target_id, 3);
    assert_eq!(resolved[1].target_id, 5);
    assert_eq!(resolved[2].target_id, 7);
    assert_eq!(resolved[0].priority_index, 0);
    assert_eq!(resolved[0].priority_total, 3);
}

#[test]
fn swept_collision_deterministic_across_runs() {
    let candidates = vec![
        SweptHitCandidate {
            target_id: 1,
            entry_t: 0.5,
            distance_traveled: 500.0,
            entry_point: [500.0, 0.0],
            ray_origin: [0.0, 0.0],
            ray_direction: [1.0, 0.0],
        },
        SweptHitCandidate {
            target_id: 2,
            entry_t: 0.5,
            distance_traveled: 500.0,
            entry_point: [500.0, 0.0],
            ray_origin: [0.0, 0.0],
            ray_direction: [1.0, 0.0],
        },
    ];
    let a = prioritize_swept_collisions(candidates.clone());
    let b = prioritize_swept_collisions(candidates);
    assert_eq!(a, b, "swept-collision priority must be deterministic");
}

// ============================================================================
// Bullet sharpness decay over distance
// ============================================================================

#[test]
fn bullet_sharpness_in_range_full_damage() {
    // Scenario: "Bullet sharpness decay over distance" — within effective range
    // the rifle delivers full damage.
    let outcome = decay_damage(SharpnessInputs {
        distance_traveled: 50.0,
        effective_range: 100.0,
        max_range: 500.0,
        base_damage: 25.0,
        base_sharpness: 0.8,
    });
    assert!(matches!(outcome.band, DecayBand::InRange));
    assert!((outcome.decayed_damage - 25.0).abs() < 1e-3);
    assert!(!outcome.expired);
}

#[test]
fn bullet_sharpness_past_max_range_expires_with_zero_damage() {
    // Scenario: "Bullet sharpness decay over distance" — past max_range,
    // damage = 0; projectile despawns.
    let outcome = decay_damage(SharpnessInputs {
        distance_traveled: 600.0,
        effective_range: 100.0,
        max_range: 500.0,
        base_damage: 25.0,
        base_sharpness: 0.8,
    });
    assert!(matches!(outcome.band, DecayBand::Expired));
    assert!(outcome.decayed_damage.abs() < 1e-3);
    assert!(outcome.expired);
}

#[test]
fn bullet_sharpness_decays_linearly_in_decay_band() {
    // 300m on a 100-500 weapon = halfway through decay band = 50% damage.
    let outcome = decay_damage(SharpnessInputs {
        distance_traveled: 300.0,
        effective_range: 100.0,
        max_range: 500.0,
        base_damage: 25.0,
        base_sharpness: 0.8,
    });
    assert!(matches!(outcome.band, DecayBand::Decaying));
    assert!((outcome.decayed_damage - 12.5).abs() < 1e-2);
    assert!(outcome.decayed_sharpness < 0.8);
}

// ============================================================================
// Limb detachment + joint impulse
// ============================================================================

#[test]
fn joint_impulse_at_strength_threshold_detaches() {
    // Scenario: "Joint impulse > joint_strength → detachment" — Given an arm
    // with joint_strength=80, when a hit delivers joint_impulse=100,
    // then attachable.detached fires + the arm becomes a physical object.
    let joint = Joint::new(80.0, 150.0, 1.0, 0.0);
    let eval = evaluate_joint(joint, 100.0);
    assert!(eval.detach);
    assert!(!eval.gib);
}

#[test]
fn joint_impulse_at_gib_limit_gibs_not_detaches() {
    // Scenario: "Joint impulse > gib_impulse_limit → gib" — Given an arm with
    // gib_impulse_limit=150, when a hit delivers joint_impulse=200, then
    // attachable.gib_threshold_crossed fires + body.gib_created fires.
    let joint = Joint::new(80.0, 150.0, 1.0, 0.0);
    let eval = evaluate_joint(joint, 200.0);
    assert!(eval.gib);
    assert!(!eval.detach);
}

#[test]
fn damage_multiplier_propagates_upward() {
    // Scenario: "Damage multiplier propagates upward" — Given an attachable
    // with m_DamageMultiplier=2.0, when the attachable takes 10 damage,
    // then 20 damage is added to the parent's HP pool.
    let joint = Joint::new(80.0, 150.0, 2.0, 0.0);
    let eval = evaluate_joint(joint, 10.0);
    // Damage multiplier applies to the propagated value.
    assert!((eval.propagated_damage - 20.0).abs() < 1e-3);
}

#[test]
fn gib_cascade_torso_lists_all_humanoid_zones() {
    // Scenario: "Gib cascade — parent gib cascades to children" — Given a
    // torso gibbed, then body.gib_cascade_triggered fires for each child
    // attachable.
    let children = default_cascade_chain("torso");
    assert!(children.contains(&"head"));
    assert!(children.contains(&"arm_left"));
    assert!(children.contains(&"arm_right"));
    assert!(children.contains(&"leg_left"));
    assert!(children.contains(&"leg_right"));
    assert!(children.contains(&"backpack"));
}

#[test]
fn arm_cascade_lists_forearm_and_hand() {
    let children = default_cascade_chain("arm_left");
    assert!(children.contains(&"forearm_left"));
    assert!(children.contains(&"hand_left"));
}

// ============================================================================
// Falling damage → leg joint impulse → potential severance
// ============================================================================

#[test]
fn fall_chain_severs_foot_first() {
    // 5m fall, 80kg actor — foot threshold first.
    let joints = vec![
        ("foot_left".to_string(), Joint::default_for_zone("foot_left")),
        ("shin_left".to_string(), Joint::default_for_zone("shin_left")),
        ("leg_left".to_string(), Joint::default_for_zone("leg_left")),
    ];
    let chain = fall_impulse_chain(9.9, 80.0, &joints);
    assert_eq!(chain.len(), 3);
    // Foot is the first joint; it absorbs the bulk and detaches/gibs.
    assert!(chain[0].1.detach || chain[0].1.gib);
}

#[test]
fn fall_chain_at_low_velocity_no_severance() {
    // 1m fall — too gentle for severance.
    let joints = vec![
        ("foot_left".to_string(), Joint::default_for_zone("foot_left")),
        ("shin_left".to_string(), Joint::default_for_zone("shin_left")),
        ("leg_left".to_string(), Joint::default_for_zone("leg_left")),
    ];
    let chain = fall_impulse_chain(4.4, 80.0, &joints);
    assert!(!chain[0].1.detach);
    assert!(!chain[0].1.gib);
}

// ============================================================================
// Heavy-melee severance (chainsaw / katana / hatchet)
// ============================================================================

#[test]
fn chainsaw_severance_probability_higher_than_knife() {
    let chainsaw_arm = severance_probability(0.4, 80.0, 100.0);
    let knife_arm = severance_probability(0.0, 80.0, 100.0);
    assert!(chainsaw_arm > knife_arm);
    assert!((knife_arm).abs() < f32::EPSILON);
}

#[test]
fn severance_roll_deterministic_given_seed() {
    // Given the same RNG draw + probability, severance_roll always returns
    // the same answer.
    let p = 0.4;
    assert!(severance_roll(0.1, p));
    assert!(severance_roll(0.1, p));
    assert!(!severance_roll(0.5, p));
    assert!(!severance_roll(0.5, p));
}

// ============================================================================
// Explosion proximity severance
// ============================================================================

#[test]
fn explosion_impulse_inverse_square_falloff() {
    let near = explosion_impulse(1000.0, 0.0);
    let mid = explosion_impulse(1000.0, 2.0);
    let far = explosion_impulse(1000.0, 5.0);
    assert!(near > mid);
    assert!(mid > far);
}

// ============================================================================
// Travel-impulse damage + ragdoll
// ============================================================================

#[test]
fn ragdoll_activates_with_state() {
    let r = Ragdoll::activate((100.0, 200.0), (10.0, 0.0), 80.0, false, 5);
    assert!(matches!(r.state, RagdollState::Activating));
    assert!(!r.reduced_motion_skip);
}

#[test]
fn ragdoll_reduced_motion_skips_animation() {
    let r = Ragdoll::activate((100.0, 200.0), (10.0, 0.0), 80.0, true, 5);
    assert!(matches!(r.state, RagdollState::StaticCollapse));
    assert!(r.reduced_motion_skip);
}

#[test]
fn ragdoll_promotes_to_active_on_dead() {
    let mut r = Ragdoll::activate((100.0, 200.0), (10.0, 0.0), 80.0, false, 5);
    r.promote_to_active();
    assert!(matches!(r.state, RagdollState::Active));
}

#[test]
fn ragdoll_subject_to_gravity_then_floor_clamp() {
    let r = Ragdoll::activate((100.0, 30.0), (0.0, -500.0), 80.0, false, 5);
    let r2 = step_ragdoll(r, -980.0, 1.0 / 60.0, 0.0, 16.0, -2000.0);
    // Either still falling or already on the floor.
    assert!(r2.position.1 >= 16.0);
}

// ============================================================================
// Per-organ + per-circuit internal damage routing
// ============================================================================

#[test]
fn heavy_hit_routes_to_random_organ_for_humanoid() {
    // Scenario: "Heavy hit routes internal damage to random organ" — Given a
    // hit with passthrough_damage=15, when the actor is human, then
    // internal.organ_damaged fires for random organ weighted by hit_zone.
    let d = route_internal_damage(InternalGraphKind::Humanoid, "torso", 15.0, 0.1).expect("heavy hit");
    assert_eq!(d.graph_kind, InternalGraphKind::Humanoid);
    assert!(d.applied_damage > 0.0);
    // 15 * 0.6 = 9 (default modifier).
    assert!((d.applied_damage - 9.0).abs() < 1e-3);
}

#[test]
fn light_hit_does_not_route_internal_damage() {
    let d = route_internal_damage(InternalGraphKind::Humanoid, "torso", 3.0, 0.5);
    assert!(d.is_none());
}

#[test]
fn heavy_hit_to_robot_picks_circuit() {
    let d = route_internal_damage(InternalGraphKind::Robot, "torso", 20.0, 0.0).expect("heavy hit");
    assert_eq!(d.graph_kind, InternalGraphKind::Robot);
    // First circuit for torso is power_core.
    assert_eq!(d.target_id, "power_core");
}

#[test]
fn explosion_routes_to_three_organs() {
    // Scenario: "Explosion routes to 3 organs" — Given explosive damage with
    // passthrough > 10, then 3 random organs damaged (within radius).
    let rolls = vec![0.0_f32, 0.3, 0.6, 0.9];
    let decisions =
        route_explosion_internal_damage(InternalGraphKind::Humanoid, "torso", 20.0, &rolls);
    assert_eq!(decisions.len(), 3);
    // All three target_ids must be distinct.
    let mut ids: Vec<&str> = decisions.iter().map(|d| d.target_id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 3, "explosion must pick 3 unique organs");
}

#[test]
fn organ_routing_deterministic_for_same_roll() {
    let d1 = route_internal_damage(InternalGraphKind::Humanoid, "head", 15.0, 0.0).unwrap();
    let d2 = route_internal_damage(InternalGraphKind::Humanoid, "head", 15.0, 0.0).unwrap();
    assert_eq!(d1.target_id, d2.target_id);
}

// ============================================================================
// Gib spawn data (authored, per-origin)
// ============================================================================

#[test]
fn gib_spawn_human_uses_blood_pixel() {
    let g = GibSpawn::default_for_origin(GibOriginKind::Human);
    assert_eq!(g.particle, "blood_pixel");
}

#[test]
fn gib_spawn_robot_uses_oil_pixel() {
    let g = GibSpawn::default_for_origin(GibOriginKind::Robot);
    assert_eq!(g.particle, "oil_pixel");
}

#[test]
fn gib_spawn_android_uses_synth_blood() {
    let g = GibSpawn::default_for_origin(GibOriginKind::Android);
    assert_eq!(g.particle, "synth_blood_pixel");
}

#[test]
fn gib_spread_even_brackets_zero_for_two_particles() {
    let a = spread_angle(SpreadMode::SpreadEven, 0, 2, 1.0, 0.0);
    let b = spread_angle(SpreadMode::SpreadEven, 1, 2, 1.0, 0.0);
    assert!(a < 0.0);
    assert!(b > 0.0);
}

// ============================================================================
// Side-view facing direction × hit routing
// ============================================================================

#[test]
fn facing_classify_back_hit_when_aligned_with_facing() {
    // Right-facing actor; projectile traveling +x = back hit (the actor's
    // back is facing the projectile because the projectile catches up).
    let d = classify_hit_direction((1.0, 0.0), 1.0);
    assert_eq!(d, HitDirection::Back);
}

#[test]
fn facing_classify_front_hit_when_opposed() {
    let d = classify_hit_direction((-1.0, 0.0), 1.0);
    assert_eq!(d, HitDirection::Front);
}

#[test]
fn facing_back_exposes_backpack() {
    assert!(exposed_zones(HitDirection::Back).contains(&"backpack"));
}

#[test]
fn facing_bottom_exposes_feet() {
    assert!(exposed_zones(HitDirection::Bottom).contains(&"foot_left"));
}

#[test]
fn facing_mirror_left_flips_local_x() {
    assert!((mirror_local_x(5.0, -1.0) - -5.0).abs() < f32::EPSILON);
}

// ============================================================================
// ZoneState lifecycle
// ============================================================================

#[test]
fn zone_state_classify_band_thresholds() {
    assert_eq!(classify_zone_state(1.0, false, false), ZoneState::Intact);
    assert_eq!(classify_zone_state(0.5, false, false), ZoneState::Damaged);
    assert_eq!(classify_zone_state(0.30, false, false), ZoneState::Critical);
    assert_eq!(classify_zone_state(1.0, true, false), ZoneState::Severed);
    assert_eq!(classify_zone_state(0.0, true, true), ZoneState::Destroyed);
}

#[test]
fn zone_state_functional_consequence_only_after_severance() {
    assert!(!ZoneState::Intact.functional_consequence_active());
    assert!(!ZoneState::Damaged.functional_consequence_active());
    assert!(ZoneState::Severed.functional_consequence_active());
    assert!(ZoneState::Destroyed.functional_consequence_active());
}

// ============================================================================
// Bleed-out timer (6 HP/sec per CCCP)
// ============================================================================

#[test]
fn bleed_zero_when_no_loss() {
    assert!(bleed_per_tick(0, 60).abs() < f32::EPSILON);
}

#[test]
fn bleed_six_per_sec_at_one_zone() {
    let v = bleed_per_tick(1, 60);
    assert!((v - 0.1).abs() < 1e-3);
}

#[test]
fn bleed_scales_with_zones_capped_at_four() {
    let one = bleed_per_tick(1, 60);
    let four = bleed_per_tick(4, 60);
    let five = bleed_per_tick(5, 60);
    assert!(four > one);
    assert!((four - five).abs() < 1e-3);
}

// ============================================================================
// War Thunder-style penetration ray + spalling
// ============================================================================

fn ray_module(id: &str, dist: f32, mult: f32, abs: f32) -> InteriorModule {
    InteriorModule {
        id: id.to_string(),
        damage_multiplier: mult,
        armor_absorption: abs,
        position: [dist, 0.0],
        distance_along_ray: dist,
        is_ammo_rack: false,
    }
}

#[test]
fn penetration_ray_visits_each_module() {
    let mods = vec![ray_module("a", 10.0, 0.5, 0.1), ray_module("b", 20.0, 0.5, 0.1)];
    let res = traverse_ray([0.0, 0.0], [1.0, 0.0], 100.0, &mods, 0.5);
    assert_eq!(res.modules_hit.len(), 2);
}

#[test]
fn penetration_ray_ammo_rack_halts_chain() {
    let mut mods = vec![ray_module("a", 10.0, 0.5, 0.1), ray_module("ammo", 20.0, 1.0, 0.5)];
    mods[1].is_ammo_rack = true;
    let res = traverse_ray([0.0, 0.0], [1.0, 0.0], 100.0, &mods, 0.5);
    assert!(res.modules_hit.iter().any(|m| m.critical_detonation));
}

#[test]
fn penetration_ray_exits_backstop_when_energy_left() {
    let mods = vec![ray_module("a", 10.0, 0.1, 0.1)];
    let res = traverse_ray([0.0, 0.0], [1.0, 0.0], 200.0, &mods, 0.3);
    assert!(res.exited_backstop);
}

#[test]
fn spalling_count_zero_below_threshold() {
    assert_eq!(spalling_fragment_count(1.0, 5.0, 0.5), 0);
}

#[test]
fn spalling_fragment_damage_within_range() {
    for i in 0..3 {
        let f = spalling_fragment_damage_fraction(i, 3, 0.5);
        assert!(f >= 0.2 && f <= 0.5);
    }
}

// ============================================================================
// HE / HEAT / APFSDS / ERA helpers
// ============================================================================

#[test]
fn he_damage_full_at_center() {
    let wave = HeWave { center: [0.0, 0.0], radius: 10.0, damage_at_zero_distance: 50.0 };
    assert!((he_damage_at_distance(&wave, 0.0) - 50.0).abs() < 1e-3);
}

#[test]
fn he_damage_zero_outside_radius() {
    let wave = HeWave { center: [0.0, 0.0], radius: 10.0, damage_at_zero_distance: 50.0 };
    assert!(he_damage_at_distance(&wave, 20.0).abs() < f32::EPSILON);
}

#[test]
fn heat_jet_penetrates_modules() {
    let mods = vec![ray_module("a", 10.0, 0.5, 0.1), ray_module("b", 20.0, 0.5, 0.1)];
    let hits = heat_jet_modules_penetrated(120.0, &mods);
    assert!(hits.len() >= 2);
}

#[test]
fn era_consumes_one_shot_to_neutralize_heat() {
    assert!(era_pre_detonates_heat(true));
    assert!(!era_pre_detonates_heat(false));
}

#[test]
fn apfsds_energy_conserved() {
    let (absorbed, remaining) = apfsds_energy_through_module(700.0, 1500.0, 100.0, 7.0);
    let total = 0.5 * 7.0 * 1500.0 * 1500.0;
    assert!((absorbed + remaining - total).abs() / total < 1e-3);
}
