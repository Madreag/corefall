//! M1: actor sim primitives.
//!
//! This crate owns the cross-binary types used by `cf-control`'s engine, `cf-app`'s
//! Bevy bridge, and the future networking/AI crates:
//!
//! - [`ActorId`], [`Status`], [`Inventory`], [`ActorState`]: components in the M1
//!   data model. Bevy ECS components are NOT defined here; the renderer wraps these
//!   types with `#[derive(Component)]` newtypes in `cf-app`/`cf-render-2d`.
//! - [`ActorWorld`]: the authoritative simulation state. Owned by the `cf-control`
//!   engine, drained per fixed tick.
//! - [`ControlIntent`]: a single tick's worth of player input. Produced by `cf-control`
//!   from JSON-RPC commands or Bevy keyboard/mouse input, consumed by [`ActorWorld::tick`].
//! - [`ActorObservation`]: the snapshot shape exposed via `observe.once`/`observe.frame`.
//!
//! Determinism contract: every public mutator is pure (state in → state out via `&mut self`)
//! and never reads a wall clock or `rand::thread_rng`. The engine's seeded RNG is the only
//! source of nondeterminism allowed inside a tick, and it is wired in by callers.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::struct_excessive_bools,
    clippy::derivable_impls,
    clippy::missing_const_for_fn,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::items_after_statements,
    clippy::return_self_not_must_use,
    clippy::float_cmp,
    clippy::if_not_else,
    clippy::cast_lossless,
    clippy::for_kv_map,
    clippy::too_many_lines,
    clippy::needless_pass_by_value,
    clippy::option_if_let_else,
    clippy::similar_names,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::single_match_else,
    clippy::field_reassign_with_default,
    clippy::collapsible_if,
    clippy::match_same_arms,
    clippy::branches_sharing_code,
    clippy::bool_to_int_with_if,
    clippy::redundant_closure_for_method_calls,
    clippy::redundant_closure,
    clippy::map_unwrap_or,
    clippy::manual_unwrap_or,
    clippy::option_map_unit_fn,
    clippy::option_as_ref_deref,
    clippy::if_same_then_else,
    clippy::unnecessary_wraps,
    clippy::unnecessary_map_or,
    clippy::wildcard_imports,
    clippy::ptr_arg,
    clippy::enum_variant_names,
    clippy::match_wildcard_for_single_variants,
    clippy::large_enum_variant,
    clippy::default_trait_access,
    clippy::implicit_hasher,
    clippy::uninlined_format_args
)]

pub mod arm_sway;
pub mod atmosphere_contact;
pub mod attachable;
pub mod attitude;
pub mod body_armor_slot;
pub mod cardiac;
pub mod components;
pub mod m14h_state;
pub mod constants;
pub mod cover;
pub mod gib;
pub mod inventory;
pub mod lean;
pub mod limb_path;
pub mod long_term;
pub mod m14a_constants;
pub mod m14j_sim;
pub mod mass;
pub mod mass_aggregator;
pub mod material_contact;
pub mod mount;
pub mod move_state;
pub mod parkour;
pub mod quick_action;
pub mod resource_drain;
pub mod sim;
pub mod sim_deps;
pub mod sim_outcomes;
pub mod sim_state;
pub mod sim_step_actor;
pub mod sim_step_loose;
pub mod sim_step_projectile;
#[cfg(test)]
mod sim_tests;
pub mod sim_overlay;
pub mod stamina;
pub mod stance;
pub mod systems;
pub mod traits;
pub mod ttd;
pub mod walking_sim;

mod actor_state;
mod actor_world;
mod affliction;
mod defaults;
mod identity;
mod intent;
mod inventory_top;
mod observation;
mod silhouette;
mod status_stance;
mod vec2;

pub use m14a_constants::*;

pub use walking_sim::{walk_sim_tick, WalkSimContext, WalkSimEvents};

pub use atmosphere_contact::{
    resolve_atmosphere_contact, suit_o2_drain_mol_per_tick, wind_force_for_actor, AtmosphereContact,
};
pub use material_contact::{resolve_material_contact, MaterialContact};
pub use resource_drain::{apply_stride_drain, drain_per_stride, resource_speed_mult};
pub use sim_overlay::{
    compute_overlay, OverlayOutcome, WALK_SPEED_HYPERTHERMIC_MULT, WALK_SPEED_HYPOTHERMIC_MULT,
    WALK_SPEED_HYPOXIA_MULT, WALK_SPEED_TOXIC_STAMINA_MULT,
};

pub use arm_sway::{
    bg_arm_rotation, empty_arm_swing, fg_arm_rotation, head_rotation_target, tick_arm_sway,
    ArmSwayContext, ArmSwayState, ARM_SWING_RATE, BG_ARM_FLAIL_SCALAR, DEVICE_ARM_SWAY_RATE,
    FG_ARM_FLAIL_SCALAR, HEAD_SMOOTHING, LOOK_TO_AIM_RATIO,
};
pub use attitude::{
    angular_impulse_from_offcenter_hit, attitude_spring_tick_dying, attitude_spring_tick_stable,
    attitude_spring_tick_unstable, evaluate_knockdown, tick_prone_state_machine, tick_walk_angle,
    AttitudeState, AttitudeStatus, KnockdownOutcome, RotAngleTargets, SpringContext,
    WalkAngleState, WalkPathOffset, CROUCH_ROT_TARGET, DYING_DURATION_MS, DYING_SPRING_K_SCALAR,
    JUMP_ROT_TARGET, MAX_CROUCH_ROTATION, MAX_WALKPATH_CROUCH_SHIFT, PRONE_DAMP_FACTOR,
    PRONE_GOSPRING_K, PRONE_HOLD_SPRING_K, PRONE_TRANSITION_MS, SPRING_DAMPING_BASE,
    SPRING_DAMPING_HEALTH_COEF, SPRING_STRENGTH, STABLE_RECOVER_MS, STAND_ROT_TARGET,
    UNSTABLE_SPRING_K, WALK_ROT_TARGET,
};
pub use limb_path::{
    default_infantry_arm_crawl, default_infantry_climb, default_infantry_crawl,
    default_infantry_crouch, default_infantry_dislodge, default_infantry_jump,
    default_infantry_registry, default_infantry_stand, default_infantry_walk_bg,
    default_infantry_walk_fg, LimbPath, LimbPathRegistry, LimbPathSpeed, PathSide,
};
pub use move_state::{MoveState, ProneState, SwimKind, UpperBodyState};
pub use mount::{
    mounted_aim_spread, resolve_dismount, DismountOutcome, MountState, DISMOUNT_MID_MOTION_STAGGER_MS,
    DISMOUNT_STATIONARY_SPEED_THRESHOLD, DISMOUNT_VELOCITY_INHERIT_FRACTION, MOUNT_MOTION_AIM_SPREAD_RAD,
    MOUNT_TOP_SPEED_RETAINED,
};
pub use parkour::{
    apply_vault, detect_vault, detect_wall, wall_jump_velocity_delta, ParkourSignal, VaultCandidate,
    WallCandidate, MAX_CHAINED_WALL_JUMPS, VAULT_DURATION_MS, VAULT_FORWARD_SWEEP_M,
    VAULT_MAX_OBSTACLE_HEIGHT_M, WALL_CONTACT_GRACE_MS, WALL_JUMP_DURATION_MS, WALL_JUMP_PERPENDICULAR_FRACTION,
};
pub use m14j_sim::{
    actor_has_dive_suit, actor_has_helmet_seal, mount_motion_aim_penalty, populate_vault_candidate,
    populate_wall_candidate, tick_m14j_actor, tick_m14j_full, M14jTickEvents,
    CLIMB_PATH_PROGRESS_DIVISOR_MS, CLIMB_VERTICAL_SPEED_M_PER_S, SWIM_BREATH_DRAIN_SECONDS_PER_SEC,
    SWIM_MAX_BREATH_SECONDS, SWIM_STAMINA_PER_STROKE, SWIM_STROKE_PERIOD_MS,
};
pub use quick_action::{
    InvokeOutcome, QuickActionBarState, QuickActionSlot, QuickActionSlotKind, RadialPhase,
    RadialState, QUICK_ACTION_DEADZONE_PX, QUICK_ACTION_OPEN_MS, QUICK_ACTION_SLOT_COUNT,
    QUICK_ACTION_TAP_MAX_MS, QUICK_ACTION_TIME_SLOW, QUICK_ACTION_TIME_SLOW_REDUCE_MOTION,
};

pub use inventory::{
    Container, InventoryBreakdown, InventoryEncumbrance, InventoryGrid, PlacedItem,
};
pub use mass_aggregator::{breakdown as mass_breakdown, mass_factor, total_mass, MassBreakdown};

pub use attachable::{apply_damage as apply_attachable_damage, Attachable};
pub use body_armor_slot::{ArmorHitOutcome, ArmorSlotState, BodyArmorSlot, EquipReject as BodyArmorEquipReject, HitZone};
pub use cover::{CoverSide, CoverState};
pub use gib::{default_cascade_chain, spread_angle, GibOriginKind, GibSpawn, SpreadMode};
pub use lean::{LeanDirection, LeanState, LEAN_MAX_DEGREES};
pub use stamina::{Stamina, SPRINT_STAMINA_DRAIN_PER_S, SPRINT_STAMINA_RECOVERY_PER_S};
pub use stance::{derive_stance, fire_allowed_in_stance, is_cinematic, stance_bloom_factor, StanceInputs};
pub use ttd::{AiDifficulty, InterimTtdContract, TtdAfflictionKind, TtdContract, TtdOrigin};

pub use actor_state::ActorState;
pub use actor_world::ActorWorld;
pub use affliction::{Affliction, AfflictionKind, ResourceAccumulators};
pub use identity::{ActorId, FacingDirection, LimbLossFlags};
pub use intent::{ControlIntent, IntentSource};
pub use inventory_top::{Inventory, InventoryItem, ItemSlot};
pub use observation::{
    ActorObservation, ChassisModuleView, ChassisView, ChassisZoneView,
    ExtendedInventorySlotView, InventoryGridPlacementView, InventoryGridView, ReloadState,
    WeaponStateView,
};
pub use silhouette::{BodySilhouette, ModuleState, ModuleStrip};
pub use status_stance::{Stance, Status};
pub use vec2::Vec2;

pub(crate) use actor_state::quantize_f32;

/// **M14A** § "Atmospheric overlay" — re-export of [`cf_atmos::AtmosphereSample`]
/// so callers don't need to depend on cf-atmos directly.
pub use cf_atmos::AtmosphereSample;

#[cfg(test)]
mod tests {
    use super::*;

    mod body_a;
    mod m14g_wound_list;
    mod m9b_damage_routing;

    #[test]
    fn status_thresholds() {
        let inv = Inventory::with_rifle("rifle_m1_default");
        let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
        assert_eq!(actor.status, Status::Stable);
        actor.apply_damage(60.0);
        assert_eq!(actor.status, Status::Unstable);
        actor.apply_damage(35.0);
        assert_eq!(actor.status, Status::Downed);
        actor.apply_damage(10.0);
        // HP=0 enters DYING first (CCCP Actor.cpp:1229); DEAD only after dwell.
        assert_eq!(actor.status, Status::Dying);
        assert!(actor.dying_dwell_ticks_remaining > 0);
        // Damage during DYING is a no-op.
        let no_change = actor.apply_damage(10.0);
        assert!(no_change.is_none());
    }

    #[test]
    fn mission_critical_caps_at_dying() {
        // M1 audit pass 6 (2026-05-13): spec literal "caps at DYING (does
        // not reach DEAD)". HP can reach 0; the actor enters DYING; the
        // DYING dwell never elapses to DEAD while mission_critical=true
        // (the dwell-elapsed branch in cf-actor::sim::step_one_actor
        // honors `dying_cap_in_effect`).
        let inv = Inventory::with_rifle("rifle_m1_default");
        let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
        actor.mission_critical = true;
        actor.apply_damage(1000.0);
        assert_eq!(actor.status, Status::Dying);
        assert!(actor.dying_dwell_ticks_remaining > 0);
    }

    #[test]
    fn inactive_pauses_state_machine() {
        let inv = Inventory::with_rifle("rifle_m1_default");
        let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
        actor.set_inactive(true);
        assert_eq!(actor.status, Status::Inactive);
        // Damage during INACTIVE is a no-op (cutscene safety).
        let change = actor.apply_damage(200.0);
        assert!(change.is_none());
        assert_eq!(actor.status, Status::Inactive);
    }

    #[test]
    fn reset_returns_full_health() {
        let inv = Inventory::with_rifle("rifle_m1_default");
        let mut actor = ActorState::player(ActorId(1), "blue", Vec2::new(10.0, 20.0), 100.0, inv);
        actor.apply_damage(70.0);
        actor.position = Vec2::new(50.0, 50.0);
        actor.reset();
        assert_eq!(actor.position, Vec2::new(10.0, 20.0));
        assert_eq!(actor.status, Status::Stable);
        assert!((actor.hp - actor.hp_max).abs() < f32::EPSILON);
    }

    #[test]
    fn inventory_select_only_advances_when_slot_exists() {
        let mut inv = Inventory::with_rifle("rifle_m1_default");
        assert!(!inv.try_select(ItemSlot(0)));
        assert!(inv.try_select(ItemSlot(1)));
        assert!(!inv.try_select(ItemSlot(99)));
        assert_eq!(inv.selected, ItemSlot(1));
    }

    #[test]
    fn intent_clear_edges_drops_buttons_keeps_axes() {
        let mut intent = ControlIntent::new(ActorId(1), IntentSource::Human);
        intent.move_x = 1.0;
        intent.aim = Vec2::new(0.0, 1.0);
        intent.jump = true;
        intent.fire = true;
        intent.reload = true;
        intent.selected_item = Some(ItemSlot(2));
        intent.reset = true;
        intent.clear_edges();
        assert!((intent.move_x - 1.0).abs() < f32::EPSILON);
        assert_eq!(intent.aim, Vec2::new(0.0, 1.0));
        assert!(!intent.jump);
        assert!(!intent.fire);
        assert!(!intent.reload);
        assert!(intent.selected_item.is_none());
        assert!(!intent.reset);
    }

    #[test]
    fn quantize_handles_nonfinite() {
        assert_eq!(quantize_f32(f32::NAN), 0);
        assert_eq!(quantize_f32(f32::INFINITY), 0);
        assert_eq!(quantize_f32(0.5), 512);
    }

    #[test]
    fn normalize_or_x_rejects_nonfinite_components() {
        // NaN/Inf must NOT pass through `len < 1e-6` (NaN comparisons return false),
        // otherwise the division produces poison values that propagate to muzzle origin,
        // projectile velocity, and recoil. Defense-in-depth fallback to (1, 0).
        for (x, y) in [
            (f32::NAN, 0.0),
            (0.0, f32::NAN),
            (f32::NAN, f32::NAN),
            (f32::INFINITY, 0.0),
            (0.0, f32::NEG_INFINITY),
            (f32::INFINITY, f32::INFINITY),
        ] {
            let n = Vec2::new(x, y).normalize_or_x();
            assert_eq!(n, Vec2::new(1.0, 0.0), "non-finite ({x}, {y}) must normalize to (1, 0)");
        }
        // Finite zero stays at (1, 0).
        assert_eq!(Vec2::new(0.0, 0.0).normalize_or_x(), Vec2::new(1.0, 0.0));
        // Finite unit vectors normalize correctly.
        let n = Vec2::new(3.0, 4.0).normalize_or_x();
        assert!((n.x - 0.6).abs() < 1e-6);
        assert!((n.y - 0.8).abs() < 1e-6);
    }

    #[test]
    fn actor_world_inserts_player_id_once() {
        let mut world = ActorWorld::new(0.0, -980.0);
        let inv = Inventory::with_rifle("rifle_m1_default");
        world.insert(ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv.clone()));
        let mut second = ActorState::player(ActorId(2), "blue", Vec2::new(5.0, 0.0), 100.0, inv);
        second.controllable = true;
        world.insert(second);
        assert_eq!(world.player, Some(ActorId(1)), "first controllable actor wins");
    }

    #[test]
    fn checksum_bytes_are_layout_stable() {
        let inv = Inventory::with_rifle("rifle_m1_default");
        let actor = ActorState::player(ActorId(7), "blue", Vec2::new(1.0, 2.0), 100.0, inv);
        let bytes = actor.checksum_bytes();
        // 8 (id u64) + 4*7 (position.x/y, velocity.x/y, aim.x/y, hp as i32) + 1 (status u8)
        // + 1 (on_ground u8) + 4 (selected slot u32) + 4 (stability i32)
        // + 4 (knockdown_ticks_remaining u32) = 50 bytes.
        // **M4 § Checksum scope sim_state_v1** appends 9 more bytes:
        // 4 (sharp_aim_progress i32) + 4 (mass_kg i32) + 1 (origin_id u8) = 9.
        // Total = 59 bytes.
        assert_eq!(bytes.len(), 59);
    }

    #[test]
    fn stance_derives_idle_when_grounded_and_still() {
        let inv = Inventory::with_rifle("rifle_m1_default");
        let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
        actor.on_ground = true;
        actor.velocity = Vec2::new(0.0, 0.0);
        assert_eq!(actor.stance(), Stance::Idle);
    }

    #[test]
    fn stance_derives_walking_running_airborne() {
        let inv = Inventory::with_rifle("rifle_m1_default");
        let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
        actor.on_ground = true;
        actor.velocity = Vec2::new(20.0, 0.0);
        assert_eq!(actor.stance(), Stance::Walking);
        actor.velocity = Vec2::new(80.0, 0.0);
        assert_eq!(actor.stance(), Stance::Running);
        actor.on_ground = false;
        actor.velocity = Vec2::new(20.0, 100.0);
        assert_eq!(actor.stance(), Stance::Airborne);
    }

    #[test]
    fn stance_derives_downed_and_dead_from_status() {
        let inv = Inventory::with_rifle("rifle_m1_default");
        let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
        actor.apply_damage(95.0);
        assert!(matches!(actor.stance(), Stance::Downed));
        actor.apply_damage(100.0);
        // HP=0 lands at DYING which projects to Stance::Downed (death animation).
        assert!(matches!(actor.status, Status::Dying));
        // Force dwell expiry → DEAD.
        actor.status = Status::Dead;
        assert_eq!(actor.stance(), Stance::Dead);
    }

    #[test]
    fn body_silhouette_clamps_hp_to_unit_range() {
        let inv = Inventory::with_rifle("rifle_m1_default");
        let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
        actor.hp = 60.0;
        let s = actor.body_silhouette();
        assert!((s.head_hp_pct - 0.6).abs() < 1e-6);
        assert!(s.placeholder);
        actor.hp = -50.0;
        let s = actor.body_silhouette();
        assert!(s.head_hp_pct >= 0.0);
        actor.hp = 200.0;
        let s = actor.body_silhouette();
        assert!(s.head_hp_pct <= 1.0);
    }

    #[test]
    fn actor_observation_carries_stance_and_silhouette() {
        let inv = Inventory::with_rifle("rifle_m1_default");
        let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
        actor.on_ground = true;
        actor.velocity = Vec2::new(80.0, 0.0);
        let obs = ActorObservation::from(&actor);
        assert_eq!(obs.stance, "running");
        assert!(obs.body_silhouette.placeholder);
        assert!((obs.body_silhouette.torso_hp_pct - 1.0).abs() < 1e-6);
    }

    #[test]
    fn attach_chassis_resizes_half_extents_for_mech() {
        let inv = Inventory::with_rifle("rifle_m1_default");
        let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
        let spec = cf_chassis::light_mech_spec();
        let chassis = cf_chassis::ChassisState::from_spec(&spec, 60, false);
        actor.attach_chassis(chassis);
        assert!(actor.half_extents.x > 10.0, "mech should be wider than infantry");
        assert!(actor.half_extents.y > 20.0, "mech should be taller than infantry");
    }

    #[test]
    fn apply_zone_damage_routes_through_chassis_layers() {
        let inv = Inventory::with_rifle("rifle_m1_default");
        let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
        let spec = cf_chassis::powered_armor_spec();
        let chassis = cf_chassis::ChassisState::from_spec(&spec, 60, false);
        actor.attach_chassis(chassis);
        let (status_change, outcome) = actor.apply_zone_damage(cf_chassis::BodyZone::Torso, 20.0, "test");
        assert!(status_change.is_none(), "small hit shouldn't change status");
        assert!(
            !outcome.layer_damage.is_empty() || !outcome.glances.is_empty(),
            "expected layer damage or glance"
        );
    }

    #[test]
    fn body_silhouette_reads_from_chassis_when_attached() {
        let inv = Inventory::with_rifle("rifle_m1_default");
        let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
        let spec = cf_chassis::powered_armor_spec();
        let chassis = cf_chassis::ChassisState::from_spec(&spec, 60, false);
        actor.attach_chassis(chassis);
        // Heavy damage to right arm so silhouette zones diverge.
        let _ = actor.apply_zone_damage(cf_chassis::BodyZone::ArmRight, 500.0, "test");
        let s = actor.body_silhouette();
        assert!(!s.placeholder, "silhouette must be sourced from chassis");
        assert!(
            s.arm_right_hp_pct < s.head_hp_pct,
            "right arm should be lower than head"
        );
    }

    #[test]
    fn chassis_view_serializes_full_zone_set() {
        let inv = Inventory::with_rifle("rifle_m1_default");
        let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
        let spec = cf_chassis::powered_armor_spec();
        let chassis = cf_chassis::ChassisState::from_spec(&spec, 60, false);
        actor.attach_chassis(chassis);
        let view = actor.chassis_view().unwrap();
        // M5 full body graph: 15 zones (head/torso/arms/legs/backpack + granular forearms/hands + shins/feet).
        assert_eq!(view.zones.len(), 15);
        // M13 powered-armor adds 3 critical modules (power_core, optics, targeting_computer)
        // on top of the M5 5-slot strip (weapon_mount/jet/shield/sensor/repair_drone), totaling 8.
        assert_eq!(view.modules.len(), 8);
        assert_eq!(view.stage, "nominal");
    }

    #[test]
    fn stance_from_chassis_yields_ejecting_when_pilot_ejecting() {
        let inv = Inventory::with_rifle("rifle_m1_default");
        let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
        let spec = cf_chassis::powered_armor_spec();
        let mut chassis = cf_chassis::ChassisState::from_spec(&spec, 60, false);
        chassis.pilot_state = cf_chassis::PilotState::Ejecting;
        actor.attach_chassis(chassis);
        assert_eq!(actor.stance(), Stance::Ejecting);
    }

    #[test]
    fn stance_from_chassis_yields_jetting_when_active() {
        let inv = Inventory::with_rifle("rifle_m1_default");
        let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
        actor.jet_active = true;
        actor.on_ground = false;
        assert_eq!(actor.stance(), Stance::Jetting);
    }

    #[test]
    fn checksum_distinguishes_high_inventory_slots() {
        // Regression: inventory.selected used to be cast `as u8`, silently truncating
        // the u32 ItemSlot. Slots 256 and 0 collided into the same checksum byte. Now
        // the full u32 is serialized so growing the inventory beyond 255 slots can't
        // hide divergent state behind identical bytes.
        let inv = Inventory::with_rifle("rifle_m1_default");
        let mut actor_a = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv.clone());
        let mut actor_b = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
        actor_a.inventory.selected = ItemSlot(0);
        actor_b.inventory.selected = ItemSlot(256);
        assert_ne!(
            actor_a.checksum_bytes(),
            actor_b.checksum_bytes(),
            "slot 0 and slot 256 must produce different checksum bytes"
        );
    }
}
