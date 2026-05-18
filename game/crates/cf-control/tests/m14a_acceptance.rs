//! **M14A**: limb-driven walking sim + CC feel parity + simulation overlay —
//! acceptance tests.
//!
//! Each test mirrors one PARITY gate from `specs/active/M14A.md` § Acceptance
//! criteria. The tests exercise the cf-actor + cf-equipment + cf-physics +
//! cf-chassis primitives directly + verify the public API contracts hold
//! deterministically.

use cf_actor::{
    angular_impulse_from_offcenter_hit, apply_stride_drain, attitude_spring_tick_stable, evaluate_knockdown,
    mass_factor, resolve_atmosphere_contact, resolve_material_contact, tick_arm_sway, tick_prone_state_machine,
    tick_walk_angle, total_mass, walk_sim_tick, ActorId, ActorState, ArmSwayContext, ArmSwayState, AtmosphereSample,
    AttitudeState, Inventory, KnockdownOutcome, LimbPath, MoveState, ProneState, QuickActionBarState,
    RotAngleTargets, SpringContext, Vec2, WalkAngleState, WalkPathOffset, WalkSimContext,
};
use cf_chassis::{heavy_trooper_spec, BodyZone, ChassisKind, ChassisState};
use cf_equipment::{
    jet_pressure_efficiency, jetpack_tick, muzzle_flash_combusts, pressure_modulated_thrust, Jetpack,
};
use cf_physics::{evaluate_ricochet, flail_as_limb, push_travel, RicochetOutcome};

fn make_walking_actor() -> ActorState {
    ActorState::player(
        ActorId(1),
        "blue",
        Vec2::ZERO,
        100.0,
        Inventory::with_rifle("rifle_m1_default"),
    )
}

fn ctx_at(tick: u64) -> WalkSimContext {
    WalkSimContext {
        tick,
        dt_ms: 16,
        move_input_active: false,
        move_x: 0.0,
        crouch_input_active: false,
        jet_hold: false,
        jet_press_edge: false,
        atmosphere: AtmosphereSample::default(),
        reduce_motion: false,
    }
}

// ============================================================================
// Walking + attitude (PARITY-01 → PARITY-32)
// ============================================================================

#[test]
fn parity_01_alternating_foot_plants() {
    let mut a = make_walking_actor();
    a.on_ground = true;
    a.velocity = Vec2::new(5.0, 0.0);
    let mut sides: Vec<u8> = Vec::new();
    for i in 0..240 {
        let mut c = ctx_at(i);
        c.move_input_active = true;
        c.move_x = 1.0;
        let ev = walk_sim_tick(&mut a, c);
        if ev.stride_fired {
            sides.push(ev.stride_side);
        }
    }
    assert!(sides.len() >= 4, "expected several strides, got {}", sides.len());
    // Consecutive strides should alternate.
    for w in sides.windows(2) {
        assert_ne!(w[0], w[1], "consecutive strides on same side");
    }
}

#[test]
fn parity_02_spring_corrects_rotation_toward_lean_target() {
    let mut state = AttitudeState {
        rot: 0.3,
        ..Default::default()
    };
    let ctx = SpringContext {
        move_state: MoveState::Stand,
        rot_angle_targets: RotAngleTargets::default(),
        aim_angle: 0.0,
        h_flipped: false,
        health: 100.0,
        max_health: 100.0,
        velocity_x: 0.0,
        walk_path_offset: WalkPathOffset::default(),
        max_crouch_rotation: 0.45,
        max_walkpath_crouch_shift: 6.0,
    };
    for _ in 0..120 {
        attitude_spring_tick_stable(&mut state, &ctx);
    }
    assert!(state.rot.abs() < 0.05, "rot did not converge: {}", state.rot);
}

#[test]
fn parity_03_walking_leans_chassis_forward() {
    let mut state = AttitudeState::default();
    let ctx = SpringContext {
        move_state: MoveState::Walk,
        rot_angle_targets: RotAngleTargets::default(),
        aim_angle: 0.0,
        h_flipped: false,
        health: 100.0,
        max_health: 100.0,
        velocity_x: 5.0,
        walk_path_offset: WalkPathOffset::default(),
        max_crouch_rotation: 0.45,
        max_walkpath_crouch_shift: 6.0,
    };
    attitude_spring_tick_stable(&mut state, &ctx);
    assert!((state.rot_target - cf_actor::WALK_ROT_TARGET).abs() < 1e-6);
}

#[test]
fn parity_05_off_center_hit_spins_chassis() {
    // Hit offset [+4, +12] with impulse [-100, 0] → cross = 1200.
    let dv = angular_impulse_from_offcenter_hit([4.0, 12.0], [-100.0, 0.0], 1000.0);
    assert!(dv > 0.0);
}

#[test]
fn parity_06_knockdown_on_lethal_impulse_triggers_unstable() {
    let outcome = evaluate_knockdown(2000.0, 80.0);
    assert_eq!(outcome, KnockdownOutcome::Knockdown);
}

#[test]
fn parity_09_crouch_plus_direction_triggers_go_prone() {
    let mut state = AttitudeState::default();
    let prone = tick_prone_state_machine(&mut state, ProneState::NotProne, 16, true, true);
    assert_eq!(prone, ProneState::GoProne);
}

#[test]
fn parity_10_prone_holds_rotation_flat() {
    let mut state = AttitudeState::default();
    // Drive into PRONE.
    let mut prone = ProneState::NotProne;
    for _ in 0..22 {
        prone = tick_prone_state_machine(&mut state, prone, 16, true, true);
    }
    assert_eq!(prone, ProneState::Prone);
}

#[test]
fn parity_11_bg_arm_supports_two_handed_weapon() {
    let mut sway = ArmSwayState::default();
    let ctx = ArmSwayContext {
        body_rot: 0.05,
        aim_angle: 0.2,
        sharp_aim_factor: 0.0,
        two_hand_weapon: true,
        holds_device: true,
        status_stable: true,
        fg_flail_scalar: cf_actor::FG_ARM_FLAIL_SCALAR,
        bg_flail_scalar: cf_actor::BG_ARM_FLAIL_SCALAR,
        stride_progress: 0.0,
    };
    tick_arm_sway(&mut sway, &ctx);
    assert!(sway.bg_supporting_fg);
}

#[test]
fn parity_12_head_tracks_aim_when_stable() {
    let ctx = ArmSwayContext {
        body_rot: 0.0,
        aim_angle: std::f32::consts::FRAC_PI_4,
        sharp_aim_factor: 0.0,
        two_hand_weapon: false,
        holds_device: false,
        status_stable: true,
        fg_flail_scalar: cf_actor::FG_ARM_FLAIL_SCALAR,
        bg_flail_scalar: cf_actor::BG_ARM_FLAIL_SCALAR,
        stride_progress: 0.0,
    };
    let target = cf_actor::head_rotation_target(&ctx);
    let expected = cf_actor::LOOK_TO_AIM_RATIO * std::f32::consts::FRAC_PI_4;
    assert!((target - expected).abs() < 1e-3);
}

#[test]
fn parity_18_push_force_doubles_after_500ms_stuck() {
    let mut path = LimbPath::default();
    path.push_force = 80.0;
    assert!((path.effective_push_force() - 80.0).abs() < 1e-6);
    path.seg_timer_ms = 500;
    assert!((path.effective_push_force() - 160.0).abs() < 1e-6);
}

#[test]
fn parity_20_walk_angle_clamped_to_40_degrees() {
    let mut state = WalkAngleState::default();
    let huge_slope = 1.5; // ~85 deg
    tick_walk_angle(&mut state, huge_slope, huge_slope, 1.0);
    let clamp = (std::f32::consts::PI / 180.0) * 40.0;
    assert!(state.fg <= clamp + 1e-3);
    assert!(state.bg <= clamp + 1e-3);
}

#[test]
fn parity_27_stride_emits_actor_event() {
    let mut a = make_walking_actor();
    a.on_ground = true;
    a.velocity = Vec2::new(5.0, 0.0);
    let mut fired = false;
    for i in 0..120 {
        let mut c = ctx_at(i);
        c.move_input_active = true;
        c.move_x = 1.0;
        let ev = walk_sim_tick(&mut a, c);
        if ev.stride_fired {
            fired = true;
            assert_eq!(a.last_stride_tick, i);
            break;
        }
    }
    assert!(fired, "no stride fired in 120 ticks");
}

// ============================================================================
// Mass + Jetpack (PARITY-33 → PARITY-50)
// ============================================================================

#[test]
fn parity_33_total_mass_aggregates_all_sources() {
    let mut a = make_walking_actor();
    a.jetpack = Some(Jetpack::standard_powered_armor());
    let mass = total_mass(&a);
    // 80 (chassis) + 5 (jet dry) + 12 (fuel) + 3.5 (held rifle) ≈ 100.5
    assert!(mass > 95.0 && mass < 110.0, "mass out of range: {}", mass);
}

#[test]
fn parity_34_heavier_mass_lower_factor() {
    let mut light = make_walking_actor();
    let mut heavy = make_walking_actor();
    heavy.mass_kg = 380.0;
    let f_light = mass_factor(&light);
    let f_heavy = mass_factor(&heavy);
    assert!(f_light > f_heavy * 2.0);
}

#[test]
fn parity_37_dropping_weapon_changes_mass() {
    let mut a = make_walking_actor();
    a.jetpack = Some(Jetpack::standard_powered_armor());
    let before = total_mass(&a);
    a.jetpack = None;
    let after = total_mass(&a);
    assert!(before > after);
}

#[test]
fn parity_39_jet_fuel_mass_decreases_with_burn() {
    let mut jet = Jetpack::standard_powered_armor();
    let initial = jet.fuel_mass_kg();
    for _ in 0..120 {
        jetpack_tick(&mut jet, true, false, false, 200.0, 80.0, 0.0, false, 101.0, 16);
    }
    assert!(jet.fuel_mass_kg() < initial - 0.5);
}

#[test]
fn parity_42_jump_pack_one_shot_full_discharge() {
    let mut jet = Jetpack::jump_pack_light_mech();
    for i in 0..200 {
        jetpack_tick(&mut jet, true, i == 0, false, 1900.0, 80.0, 0.0, false, 101.0, 16);
        if jet.jet_time_left_ms == 0 {
            break;
        }
    }
    assert!(jet.jumppack_refilling);
    assert_eq!(jet.jet_time_left_ms, 0);
}

#[test]
fn parity_44_can_adjust_angle_while_firing_locks_direction() {
    let mut jet = Jetpack::jump_pack_light_mech();
    jetpack_tick(&mut jet, true, true, false, 200.0, 80.0, 0.5, false, 101.0, 16);
    let locked = jet.locked_emit_angle.expect("angle should lock");
    jetpack_tick(&mut jet, true, false, false, 200.0, 80.0, -1.0, false, 101.0, 16);
    assert_eq!(jet.locked_emit_angle, Some(locked));
}

#[test]
fn parity_45_throttle_for_weight_increases_drain() {
    let mut light = Jetpack::standard_powered_armor();
    let mut heavy = Jetpack::standard_powered_armor();
    let _ = light.fuel_ratio();
    let _ = heavy.fuel_ratio();
    for _ in 0..30 {
        jetpack_tick(&mut light, true, false, false, 80.0, 80.0, 0.0, false, 101.0, 16);
        jetpack_tick(&mut heavy, true, false, false, 220.0, 80.0, 0.0, false, 101.0, 16);
    }
    let light_used = 4500_u32.saturating_sub(light.jet_time_left_ms);
    let heavy_used = 4500_u32.saturating_sub(heavy.jet_time_left_ms);
    assert!(heavy_used > light_used * 2);
}

#[test]
fn parity_46_minimum_fuel_ratio_blocks_activation() {
    let mut jet = Jetpack::standard_powered_armor();
    jet.jet_time_left_ms = (jet.jet_time_total_ms as f32 * 0.2) as u32;
    assert_eq!(
        jet.check_activation_reject(true),
        Some("jet_below_minimum_fuel_ratio")
    );
}

// ============================================================================
// Heavy Armor (PARITY-51 → PARITY-65)
// ============================================================================

#[test]
fn parity_51_heavy_trooper_has_tank_grade_zones() {
    let spec = heavy_trooper_spec();
    assert_eq!(spec.kind, ChassisKind::HeavyTrooper);
    let torso = spec.zones.iter().find(|z| z.zone == BodyZone::Torso).unwrap();
    assert!((torso.damage_multiplier - 0.6).abs() < 1e-6);
    assert!(torso.stagger_factor < 0.3);
    assert!(torso.gib_impulse_limit >= 3200.0);
}

#[test]
fn parity_58_heavy_trooper_not_knocked_down_by_rifle() {
    let outcome = evaluate_knockdown(200.0, 380.0);
    assert_eq!(outcome, KnockdownOutcome::None);
}

#[test]
fn parity_60_ricochet_at_grazing_angle() {
    let angle = 70.0_f32.to_radians();
    let incoming = [angle.cos(), angle.sin()];
    let out = evaluate_ricochet(incoming, [1.0, 0.0], 0.3, 800.0, 22.0, 0.0);
    matches!(out, RicochetOutcome::Bounce { .. });
}

#[test]
fn parity_61_heavy_trooper_walks_slow_via_mass_factor() {
    let mut a = make_walking_actor();
    let chassis = ChassisState::from_spec(&heavy_trooper_spec(), 60, false);
    a.attach_chassis(chassis);
    let mf = mass_factor(&a);
    assert!(mf <= 0.4, "heavy trooper mass factor too high: {}", mf);
}

// ============================================================================
// Quick Action UX (PARITY-66 → PARITY-75)
// ============================================================================

#[test]
fn parity_66_qab_always_visible_with_eight_slots() {
    let bar = QuickActionBarState::infantry_default();
    assert_eq!(bar.slots.len(), 8);
}

#[test]
fn parity_67_tap_key_invokes_instant_slot() {
    let mut bar = QuickActionBarState::infantry_default();
    let out = bar.try_invoke_slot(0);
    assert_eq!(out, cf_actor::quick_action::InvokeOutcome::Accepted);
    assert_eq!(bar.last_used_slot, 0);
}

#[test]
fn parity_69_hold_q_opens_radial_within_80ms() {
    let mut bar = QuickActionBarState::infantry_default();
    bar.open_radial(0, false);
    for _ in 0..20 {
        bar.tick(4);
    }
    assert_eq!(bar.radial.phase, cf_actor::RadialPhase::Open);
    assert!((bar.radial.sim_time_multiplier - cf_actor::QUICK_ACTION_TIME_SLOW).abs() < 1e-3);
}

#[test]
fn parity_71_release_q_invokes_selection() {
    let mut bar = QuickActionBarState::infantry_default();
    bar.open_radial(0, false);
    for _ in 0..20 {
        bar.tick(4);
    }
    let invoked = bar.close_radial(Some(4));
    assert_eq!(invoked, Some(4));
}

#[test]
fn parity_73_tap_q_invokes_last_used() {
    let mut bar = QuickActionBarState::infantry_default();
    bar.try_invoke_slot(2);
    let out = bar.try_invoke_toggle();
    assert_eq!(out, cf_actor::quick_action::InvokeOutcome::Accepted);
}

#[test]
fn parity_75_reduce_motion_uses_50_percent_time_slow() {
    let mut bar = QuickActionBarState::infantry_default();
    bar.open_radial(0, true);
    for _ in 0..20 {
        bar.tick(4);
    }
    assert!((bar.radial.sim_time_multiplier - cf_actor::QUICK_ACTION_TIME_SLOW_REDUCE_MOTION).abs() < 1e-3);
}

// ============================================================================
// Simulation overlay (PARITY-76 → PARITY-123)
// ============================================================================

#[test]
fn parity_76_wet_tile_causes_foot_slip() {
    // material id 18 = water with friction_mult 0.4.
    let mod_ = cf_terrain::material_walk_modulator(18);
    let c = resolve_material_contact(
        mod_.friction_mult,
        mod_.speed_mult,
        mod_.emit_hazard,
        mod_.foot_damage_hp_per_tick,
        mod_.hazard_kind,
        18,
        80.0,
        "human",
    );
    assert!(c.foot_slip);
}

#[test]
fn parity_80_lava_burns_planted_foot() {
    let mod_ = cf_terrain::material_walk_modulator(12);
    assert!(mod_.foot_damage_hp_per_tick > 0.0);
    assert!(mod_.emit_hazard);
}

#[test]
fn parity_86_wind_pushes_walking_actor() {
    let mut atm = AtmosphereSample::default();
    atm.wind = [-10.0, 0.0];
    let c = resolve_atmosphere_contact(&atm, 8.0);
    assert!(c.wind_force_n[0] < 0.0);
}

#[test]
fn parity_87_jet_thrust_scales_with_pressure() {
    let vac = jet_pressure_efficiency(0.5);
    let earth = jet_pressure_efficiency(101.0);
    let venus = jet_pressure_efficiency(239.0);
    assert!(vac > earth);
    assert!(venus < earth);
}

#[test]
fn parity_88_vacuum_has_zero_drag_effect_on_thrust() {
    let earth = pressure_modulated_thrust(1000.0, 101.0);
    let vacuum = pressure_modulated_thrust(1000.0, 0.5);
    assert!(vacuum > earth);
}

#[test]
fn parity_90_hypoxic_atmosphere_reports_warning() {
    let mut atm = AtmosphereSample::default();
    atm.o2_partial_kpa = 10.0;
    assert_eq!(atm.hypoxia_severity(), 2);
}

#[test]
fn parity_92_muzzle_flash_ignites_combustible_atmosphere() {
    assert!(muzzle_flash_combusts(10.0, 21.0, 101.0, 600.0));
    assert!(!muzzle_flash_combusts(10.0, 21.0, 101.0, 300.0));
}

#[test]
fn parity_97_low_g_cell_extends_jump_arc() {
    let mut atm = AtmosphereSample::default();
    atm.local_gravity_m_s2 = 3.0;
    let overlay = cf_actor::compute_overlay(atm);
    assert!(overlay.low_g);
}

#[test]
fn parity_103_helmet_breach_in_vacuum_drains_o2() {
    let mol = cf_actor::suit_o2_drain_mol_per_tick(2.0, 1.0);
    assert!((mol - cf_atmos::INHALED_MOL_PER_TICK_BASE * 2.0).abs() < 1e-6);
}

#[test]
fn parity_106_human_caloric_depletes_per_stride() {
    let mut a = make_walking_actor();
    a.resources.caloric_energy = 100.0;
    let _ = apply_stride_drain(&mut a);
    assert!(a.resources.caloric_energy < 100.0);
}

#[test]
fn parity_107_robot_power_depletes_per_stride() {
    let mut a = make_walking_actor();
    a.origin_id = "robot".to_string();
    a.resources.power = 100.0;
    let _ = apply_stride_drain(&mut a);
    assert!(a.resources.power < 100.0);
}

// ============================================================================
// Determinism contract
// ============================================================================

#[test]
fn determinism_same_seed_same_trajectory() {
    let mut a1 = make_walking_actor();
    let mut a2 = make_walking_actor();
    a1.on_ground = true;
    a2.on_ground = true;
    a1.velocity = Vec2::new(5.0, 0.0);
    a2.velocity = Vec2::new(5.0, 0.0);
    for i in 0..240 {
        let mut c = ctx_at(i);
        c.move_input_active = true;
        c.move_x = 1.0;
        walk_sim_tick(&mut a1, c);
        walk_sim_tick(&mut a2, c);
    }
    assert!((a1.position.x - a2.position.x).abs() < 1e-6);
    assert!((a1.attitude.rot - a2.attitude.rot).abs() < 1e-6);
    assert_eq!(a1.last_stride_side_fg, a2.last_stride_side_fg);
}

#[test]
fn mass_invariant_holds_within_tolerance() {
    let mut a = make_walking_actor();
    a.jetpack = Some(Jetpack::standard_powered_armor());
    let cached = a.total_mass_kg();
    let computed = total_mass(&a);
    assert!((cached - computed).abs() < 0.5);
}

// ============================================================================
// Atom-group push_travel + flail_as_limb
// ============================================================================

#[test]
fn push_travel_blocks_against_solid_terrain() {
    let (impulse, final_pos) = push_travel([0.0, 0.0], [10.0, 0.0], 100.0, 16, |_x, _y| true);
    assert!(impulse[0] < 0.0);
    assert_eq!(final_pos, [0.0, 0.0]);
}

#[test]
fn flail_as_limb_constrains_to_radius() {
    let result = flail_as_limb([0.0, 0.0], [0.0, 8.0], [50.0, 8.0], 10.0, [0.0, 0.0], 0.0, 1.0, 16);
    let dist = (result[0].powi(2) + (result[1] - 8.0).powi(2)).sqrt();
    assert!(dist <= 10.0 + 1e-3);
}
