use serde::{Deserialize, Serialize};

use crate::defaults::{
    default_bipod_equipped, default_bloom_factor, default_dying_dwell_ticks,
    default_grenade_kind, default_mass_dirty, default_mass_kg, default_origin_id,
    default_recoil_decay_rate, default_sharp_aim_build_ticks, default_speed_factor,
    default_stability, default_stability_recovery_rate, default_swim_breath_seconds,
    default_swim_drain_multiplier, default_walk_threshold,
};
use crate::{
    cardiac, m14h_state, ActorId, Affliction, AtmosphereSample, BodyArmorSlot,
    BodySilhouette, ChassisModuleView, ChassisView, ChassisZoneView, CoverState,
    FacingDirection, Inventory, ItemSlot, LeanState, LimbLossFlags, ModuleState,
    ModuleStrip, ResourceAccumulators, Stamina, Stance, Status, Vec2,
};

/// Per-actor authoritative state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorState {
    pub id: ActorId,
    pub team: String,
    pub spawn: Vec2,
    pub position: Vec2,
    pub velocity: Vec2,
    pub aim: Vec2,
    pub on_ground: bool,
    pub status: Status,
    pub hp: f32,
    pub hp_max: f32,
    pub hp_unstable_threshold: f32,
    pub hp_downed_threshold: f32,
    pub inventory: Inventory,
    /// True if this actor accepts player intent (only one in M1 scenarios).
    pub controllable: bool,
    /// Half-extents of the AABB used for ground collision + future limb proxies.
    /// M1 uses a chunky 8x16 actor footprint; M5 replaces this with chassis half-extents.
    pub half_extents: Vec2,
    /// Physical mass in kg. Affects recoil magnitude, knockdown resistance,
    /// and future collision impulse routing. Chassis kind overrides this
    /// when attached (Infantry 80kg, PoweredArmor 200kg, LightMech 600kg).
    #[serde(default = "default_mass_kg")]
    pub mass_kg: f32,
    /// `None` for legacy M1 / M1.5 actors that haven't been promoted; `Some` for
    /// M5+ chassis-grade actors (infantry / powered_armor / light_mech).
    #[serde(default)]
    pub chassis: Option<cf_chassis::ChassisState>,
    /// Defaults to `"human"`.
    #[serde(default = "default_origin_id")]
    pub origin_id: String,
    /// events. `crouch_active` is sticky (toggle by act.player.crouch); the
    /// others are edge-driven and cleared after consumption.
    #[serde(default)]
    pub crouch_active: bool,
    #[serde(default)]
    pub climb_active: bool,
    #[serde(default)]
    pub jet_active: bool,
    /// W1.3: knockdown recovery ticks remaining. When > 0, actor cannot accept
    /// input (same as Downed) but status stays Stable/Unstable. Decrements each
    /// tick; at zero the actor regains control. Triggered when stability < 0.1
    /// and the actor takes a destabilizing event (landing, recoil, damage).
    #[serde(default)]
    pub knockdown_ticks_remaining: u32,
    /// contribution returns `drop_gear=true`. Used to gate single-shot
    /// `actor.gear_dropped` event emission + clear the rifle/inventory slot
    /// the next tick. Roadmap §M5 done-criterion: "dropped gear".
    #[serde(default)]
    pub gear_dropped_by_limb_loss: bool,
    /// `PilotState::Ejected` (post-eject) so the engine knows to call
    /// `detach_chassis` once. Roadmap §M5 done-criterion: "Pilot eject works:
    /// player ejects from a wrecked mech and continues as foot infantry."
    #[serde(default)]
    pub chassis_detached: bool,
    /// player's brain (mission-critical=true; cannot be gibbed instantly).
    /// Brain death = `LossReason::BrainDestroyed`.
    #[serde(default)]
    pub is_brain: bool,
    /// 0 when this actor has never been the brain target.
    #[serde(default)]
    pub last_brain_hop_tick: u64,
    /// world-space position recorded the last time the brain was in this
    /// actor. Persists across brain hops so AI fallback / squad cohesion
    /// can reason about where the brain WAS.
    #[serde(default)]
    pub brain_last_position: [f32; 2],
    /// runtime state. `Some(...)` for drone-spec actors; `None` for everything else.
    #[serde(default)]
    pub drone_ally: Option<cf_chassis::DroneAllyState>,
    /// is currently boarding (boarding is an attribute of the boarder, not
    /// the chassis being boarded). `None` when no boarding is in flight.
    /// Set when `act.player.board` is accepted; cleared when
    /// `boarding_ticks_remaining` decrements to 0 + the pilot transfer
    /// completes (or when the boarding is otherwise cancelled).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_boarding_target: Option<u64>,
    /// boarding transition. Ticked on the player actor in
    /// `tick_chassis_eject_for_all` so input lock + HUD banner read off
    /// the player, not the target chassis.
    #[serde(default)]
    pub boarding_ticks_remaining: u32,
    /// currently-active hit-reaction window. Drives speed reduction +
    /// aim-wobble multipliers per `HitReaction`.
    #[serde(default)]
    pub hit_reaction_ticks_remaining: u32,
    /// reaction ("stagger_stun" / "limp" / "drop_weapon" / ...).
    #[serde(default)]
    pub hit_reaction_kind: String,
    /// active reaction (1.0 = no reduction).
    #[serde(default = "default_speed_factor")]
    pub hit_reaction_speed_factor: f32,
    /// **W1.3**: stability scalar (0.0 = fully disrupted, 1.0 = stable).
    /// Decremented by recoil impulse, fall impact, explosion knockback.
    /// Recovers toward 1.0 at `stability_recovery_rate` per tick when on ground.
    /// When stability < 0.3, aim bloom increases; when < 0.1, actor is knockdown-vulnerable.
    /// Feeds future DR-003 HUD readability + A-FEEL-06 damage-cause explanation.
    #[serde(default = "default_stability")]
    pub stability: f32,
    /// Recovery rate per tick when grounded and not taking impulse damage.
    /// Default 0.02 = full recovery in ~50 ticks (0.83s at 60Hz).
    #[serde(default = "default_stability_recovery_rate")]
    pub stability_recovery_rate: f32,
    /// Decrements each tick after status enters DYING; at zero the status
    /// transitions to DEAD. Default 60 ticks (1000 ms at 60Hz).
    #[serde(default)]
    pub dying_dwell_ticks_remaining: u32,
    /// Default value used when seeding `dying_dwell_ticks_remaining` on the
    /// DYING transition. Configurable per-scenario for tutorial / iron-man modes.
    #[serde(default = "default_dying_dwell_ticks")]
    pub dying_dwell_ticks_default: u32,
    /// `act.player.sharp_aim` while STABLE, slow, and equipped (CCCP
    /// `AHuman.cpp:1779`). Snaps to 0 on jump/reload/swap/knockdown/over-walk.
    #[serde(default)]
    pub sharp_aim_progress: f32,
    /// True while the player holds the sharp-aim input. Sticky; consumed every tick.
    #[serde(default)]
    pub sharp_aim_active: bool,
    /// Number of ticks to fully build sharp aim from 0 → 1.0. Default 30 ticks (0.5s @ 60Hz).
    #[serde(default = "default_sharp_aim_build_ticks")]
    pub sharp_aim_build_ticks: u32,
    /// recoil drift over multiple shots; decays per tick. Positive / negative
    /// values alternate so the muzzle climbs predictably rather than chaotically.
    #[serde(default)]
    pub recoil_accumulator: f32,
    /// Recoil accumulator decay per tick. Default 0.05 (subtract 0.05 per tick
    /// toward zero).
    #[serde(default = "default_recoil_decay_rate")]
    pub recoil_decay_rate: f32,
    /// Alternates sign of the next recoil contribution (so the muzzle climbs
    /// in a tight zig-zag rather than always biasing one way).
    #[serde(default)]
    pub recoil_alternation_sign: i8,
    /// at DYING (does NOT transition to DEAD via dwell). The mission director
    /// (M1.5+) reacts to "critical actor downed" before final loss.
    #[serde(default)]
    pub mission_critical: bool,
    /// engine fires a one-shot `actor.inventory_dropped` event + clears the
    /// rifle slot. Cleared on `reset()`.
    #[serde(default)]
    pub inventory_dropped_on_dying: bool,
    /// equivalent lethal cause) so the engine can thread the cause-chain
    /// across the DYING dwell:
    ///   `inventory_dropped` -> `status_changed(DEAD)` -> `status_changed(DYING)`
    ///   -> `wound_added` -> `projectile_hit` -> `projectile_spawned`
    ///   -> `weapon_fired` -> `input.intent_received`
    /// without losing the parent_event_id chain when the DYING/DEAD events
    /// fire on a different tick than the lethal projectile_hit. `None` when
    /// the actor has not entered DYING (yet) or the lethal cause did not
    /// come from a recordable event (e.g. seed-time spawn).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_lethal_cause_event_id: Option<String>,
    /// actor counts as "slow enough" for sharp aim to keep building.
    /// Default 1.5 (essentially "barely moving" — see OpenSoldat
    /// `Sprites.pas:4870` for the calibration anchor).
    #[serde(default = "default_walk_threshold")]
    pub walk_threshold: f32,
    /// reticle renderer, and AI consumers all read the same value the sim
    /// computed this tick (rather than each duplicating the formula).
    #[serde(default = "default_bloom_factor")]
    pub bloom_factor: f32,
    /// accumulators on every actor; `#[serde(default)]` keeps M5 bundles
    /// readable while M5.8 wires actual driver values later. Save-bundle
    /// checksum layout is stable from M5 onward — M5.8 will fill these without
    /// a schema bump.
    #[serde(default)]
    pub resources: ResourceAccumulators,
    /// state set populated by hazards (M5.7) + origin reactions (M5.8). Empty
    /// at M5 baseline; serde-default preserves backward compat.
    #[serde(default)]
    pub afflictions: Vec<Affliction>,
    /// that have not taken a typed wound yet. Survives save/load via
    /// `checksum_bytes` (VAL-CROSS-029).
    #[serde(default)]
    pub m14g_wound_list: cf_wound::ActorWoundList,
    /// M5/M16 — transient per-tick affliction-derived combat/movement modifiers
    /// (the M16 affliction → aim/move-speed consumer). The engine recomputes
    /// these every tick from the M16 affliction state; the actor sim consumes
    /// them in `effective_max_speed` (walk-speed × multiplier) and the
    /// weapon-fire spread cone (+ aim-spread bonus radians). Identity defaults
    /// (1.0 / 0.0) keep an unafflicted actor byte-for-byte unchanged. Not
    /// serialized and not part of `checksum_bytes` — they are derived, not
    /// authoritative state.
    #[serde(skip, default = "crate::defaults::default_affliction_speed_multiplier")]
    pub affliction_speed_multiplier: f32,
    #[serde(skip)]
    pub affliction_aim_spread_bonus_rad: f32,
    #[serde(default)]
    pub m14h_cardiac: cardiac::ActorCardiacComponent,
    /// trigger (spec § "Necrosis if not removed").
    #[serde(default)]
    pub m14h_tourniquets: std::collections::BTreeMap<cf_wound::registry::ZoneId, u64>,
    /// expiry tick; per-tick pass clears expired buffs.
    #[serde(default)]
    pub m14h_buffs: Vec<m14h_state::ActiveBuff>,
    /// 8s recharge interval per spec table.
    #[serde(default)]
    pub m14h_last_defib_tick: Option<u64>,
    #[serde(default)]
    pub m14h_antibiotic_course: Option<m14h_state::AntibioticCourseState>,
    #[serde(default)]
    pub facing: FacingDirection,
    #[serde(default)]
    pub stamina: Stamina,
    #[serde(default)]
    pub lean_state: LeanState,
    #[serde(default)]
    pub cover_state: CoverState,
    #[serde(default)]
    pub sprint_active: bool,
    #[serde(default)]
    pub prone_active: bool,
    /// 0 when no animation is playing.
    #[serde(default)]
    pub cinematic_ticks_remaining: u32,
    #[serde(default)]
    pub cinematic_kind: Option<Stance>,
    #[serde(default)]
    pub stealth_meter: f32,
    /// M13 chassis fills physics-driven limb damage.
    #[serde(default)]
    pub limb_loss: LimbLossFlags,
    /// forces Walk per spec § "Weight system".
    #[serde(default)]
    pub inventory_weight_kg: f32,
    /// When deployed, the firing path multiplies recoil by
    /// [`cf_equipment::BIPOD_RECOIL_FACTOR`] and bloom by
    /// [`cf_equipment::BIPOD_BLOOM_FACTOR`].
    #[serde(default = "default_bipod_equipped")]
    pub bipod: cf_equipment::Bipod,
    /// attached, the firing path multiplies loudness by
    /// [`cf_equipment::SUPPRESSOR_LOUDNESS_FACTOR`].
    #[serde(default)]
    pub suppressor: cf_equipment::Suppressor,
    /// lazily on first `act.player.use_tool` so existing scenarios serialize
    /// cleanly.
    #[serde(default)]
    pub tool_durability: std::collections::BTreeMap<String, cf_equipment::Durability>,
    /// this actor. The firing path in `sim::fire_actor` gates on this so
    /// fire/reload intent is rejected during the swap window.
    #[serde(default)]
    pub weapon_swap_in_progress: bool,
    /// grenade. Reset on throw or grenade swap.
    #[serde(default)]
    pub grenade_cook_seconds: f32,
    /// when no grenade slot is selected. Defaults to Frag at spawn so the
    /// spec's "Cook grenade for shorter fuse" Gherkin reproduces.
    #[serde(default = "default_grenade_kind")]
    pub grenade_held_kind: Option<cf_equipment::GrenadeKind>,
    /// value is the grenade's base fuse; cook_grenade subtracts elapsed
    /// cook time.
    #[serde(default)]
    pub grenade_held_fuse_remaining: f32,
    /// `cf_equipment::DRILL_JAM_HEAT_THRESHOLD` the drill jams + emits
    /// `equipment.drill_overheated`. Decays at
    /// `cf_equipment::DRILL_HEAT_DECAY_PER_S` per second when idle.
    #[serde(default)]
    pub drill_heat: f32,
    /// 0 when not revealed. Set by `act.player.use_tool { kind:
    /// "sensor_pulse" }` for hostile actors within the reveal radius.
    #[serde(default)]
    pub reveal_until_tick: u64,
    /// selected weapon. `act.player.cycle_fire_mode` rotates this through
    /// the weapon's `FireModeSet::available` list; the firing path consults
    /// it to gate Burst3 / Charge / Arc semantics. Default `Single` matches
    /// the cold-start weapon state.
    #[serde(default)]
    pub weapon_fire_mode: cf_equipment::AdvancedFireMode,
    /// the trigger is held under `AdvancedFireMode::Charge`; clamps to 1.0
    /// at full charge (per [`cf_equipment::SNIPER_CHARGE_MAX_SECONDS`]).
    /// Reset to 0 on trigger release.
    #[serde(default)]
    pub weapon_charge_fraction: f32,
    /// to `BURST3_ROUND_COUNT - 1` when the first round of a 3-round burst
    /// fires through the M1 path; decremented as each follow-up round
    /// fires from the M6 tick scheduler.
    #[serde(default)]
    pub burst3_remaining_shots: u32,
    /// fires. Reset to [`cf_equipment::BURST3_INTER_SHOT_SECONDS`] each
    /// time a follow-up round leaves the muzzle so the 3-round burst
    /// completes within 100 ms per spec § "SMG burst-3 fire mode".
    #[serde(default)]
    pub burst3_next_fire_at_seconds: f32,
    /// `true` the tick after the player begins holding fire; transitions
    /// to `false` on release, at which point the engine fires one
    /// charge-scaled shot.
    #[serde(default)]
    pub fire_held_prev: bool,
    /// container nesting + liquid-mass tracking). `None` for legacy
    /// actors that still drive purely off `inventory` /
    /// `inventory_weight_kg`; `Some(...)` for M6B+ actors so the M14A
    /// mass aggregator can read a single canonical inventory surface.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory_grid: Option<crate::inventory::InventoryGrid>,
    /// derived walk-speed multiplier + band). Populated lazily on
    /// `inventory_grid_attach` or at construction time via
    /// `seed_inventory_encumbrance_for_origin`. `None` for legacy actors;
    /// `Some(...)` for M6B+ actors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory_encumbrance: Option<crate::inventory::InventoryEncumbrance>,
    /// boots + knee/elbow pads). Separate from the M13 chassis armor.
    /// Default = empty slots so legacy actors serialize cleanly.
    #[serde(default)]
    pub body_armor: BodyArmorSlot,
    /// `Stance::Crewing`. Stored externally so the `Stance` enum stays
    /// a flat `#[repr(u8)]` while expressing the spec-shape
    /// `Stance::Crewing { fortification_id }`. `None` when not crewing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crewing_fortification_id: Option<u32>,
    /// the implementer": *don't store wire crossing state on the wire
    /// (one wire, many actors); store it on the actor's `crossing:
    /// Option<WireId>`. Avoid the O(n×m) interaction-matrix trap.*
    /// `Some(wire_id)` while the actor is currently crossing /
    /// snagged on a wire; `None` otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crossing: Option<cf_fortification::WireId>,
    #[serde(default)]
    pub move_state: crate::move_state::MoveState,
    #[serde(default)]
    pub prone_state: crate::move_state::ProneState,
    #[serde(default)]
    pub upper_body_state: crate::move_state::UpperBodyState,
    #[serde(default)]
    pub attitude: crate::attitude::AttitudeState,
    #[serde(default)]
    pub walk_angle: crate::attitude::WalkAngleState,
    #[serde(default)]
    pub walk_path_offset: crate::attitude::WalkPathOffset,
    #[serde(default)]
    pub arm_sway: crate::arm_sway::ArmSwayState,
    #[serde(default)]
    pub stride_frame: bool,
    #[serde(default)]
    pub stride_start: bool,
    #[serde(default)]
    pub stride_timer_ms: u32,
    #[serde(default)]
    pub last_stride_side_fg: bool,
    /// `default_infantry_registry()`. Owned on actor for serde round-trip.
    #[serde(default = "crate::limb_path::default_infantry_registry")]
    pub limb_paths: crate::limb_path::LimbPathRegistry,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jetpack: Option<cf_equipment::Jetpack>,
    #[serde(default)]
    pub quick_action_bar: crate::quick_action::QuickActionBarState,
    #[serde(default)]
    pub total_mass_cached: f32,
    /// stale and `total_mass()` must recompute.
    #[serde(default = "default_mass_dirty")]
    pub total_mass_dirty: bool,
    #[serde(default)]
    pub wound_mass_kg: f32,
    /// (consumers like cf-perception read this to emit hearing signals).
    #[serde(default)]
    pub last_stride_tick: u64,
    /// disambiguate tap-Q from hold-Q.
    #[serde(default)]
    pub q_press_tick: u64,
    #[serde(default)]
    pub q_held: bool,
    #[serde(default)]
    pub last_used_quick_slot: u8,
    /// Level 1 = 0-30% External lost; 2 = 30-60%; 3 = 60-100%.
    #[serde(default)]
    pub armor_scratch_level: std::collections::BTreeMap<String, u8>,
    #[serde(default)]
    pub atmosphere_sample: AtmosphereSample,
    /// per-zone hazard contact (zone_name → tick).
    #[serde(default)]
    pub last_hazard_contact_tick: std::collections::BTreeMap<String, u64>,
    /// biological age, prosthetics, traits, severed-limb tracking,
    /// concussion count, chronic pain baseline, radiation dose).
    #[serde(default)]
    pub m14i_long_term: crate::long_term::LongTermState,
    #[serde(default)]
    pub parkour_signal: crate::parkour::ParkourSignal,
    /// with a critter; `None` when unmounted. Living on the rider keeps the
    /// save/load round trip simple (the critter side just notes whether it
    /// has any rider via `is_being_ridden`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mount: Option<crate::mount::MountState>,
    /// (mirrored on the critter actor for fast lookups during gait
    /// selection + AI doctrine).
    #[serde(default)]
    pub is_being_ridden: bool,
    /// far end of the embedded grapple line). `None` when the actor is not
    /// holding any rope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holding_rope: Option<cf_physics::RopeId>,
    /// rider. `None` when the actor is not zip-lining.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zipline_attached: Option<cf_physics::RopeId>,
    /// decelerates at -3 m/s² per spec.
    #[serde(default)]
    pub zipline_brake_engaged: bool,
    /// (200 ms). 0 when not in a wall-jump animation.
    #[serde(default)]
    pub wall_jump_ticks_remaining_ms: u32,
    /// `Stance::SwimSubmerged`. Reset on surface contact. 0 triggers
    /// drowning per spec § "Submerged swim dive transitions to drowning".
    #[serde(default = "default_swim_breath_seconds")]
    pub swim_breath_seconds: f32,
    /// Distinct from the legacy `Stance::Swim` so the M14A walking loop
    /// can swap limb paths via `move_state` + a small swim-state machine.
    #[serde(default)]
    pub swim_kind: crate::move_state::SwimKind,
    /// the actor's origin: Human 1.0×, Aqueous 0.5×, Robotic = sinks
    /// (drain rate has no effect since they don't actually swim).
    #[serde(default = "default_swim_drain_multiplier")]
    pub swim_drain_multiplier: f32,
    /// Mirror flag for fast checks.
    #[serde(default)]
    pub swim_disabled_sinks: bool,
}

impl ActorState {
    /// Create a default M1 player actor at `spawn` with `inventory` and full HP.
    pub fn player(id: ActorId, team: impl Into<String>, spawn: Vec2, hp_max: f32, inventory: Inventory) -> Self {
        Self {
            id,
            team: team.into(),
            spawn,
            position: spawn,
            velocity: Vec2::ZERO,
            aim: Vec2::new(1.0, 0.0),
            on_ground: false,
            status: Status::Stable,
            hp: hp_max,
            hp_max,
            hp_unstable_threshold: hp_max * 0.5,
            hp_downed_threshold: hp_max * 0.1,
            inventory,
            controllable: true,
            half_extents: Vec2::new(8.0, 16.0),
            mass_kg: 80.0,
            chassis: None,
            origin_id: default_origin_id(),
            crouch_active: false,
            climb_active: false,
            jet_active: false,
            gear_dropped_by_limb_loss: false,
            chassis_detached: false,
            is_brain: false,
            last_brain_hop_tick: 0,
            brain_last_position: [0.0, 0.0],
            drone_ally: None,
            pending_boarding_target: None,
            boarding_ticks_remaining: 0,
            hit_reaction_ticks_remaining: 0,
            hit_reaction_kind: String::new(),
            hit_reaction_speed_factor: 1.0,
            stability: 1.0,
            stability_recovery_rate: 0.02,
            knockdown_ticks_remaining: 0,
            dying_dwell_ticks_remaining: 0,
            dying_dwell_ticks_default: default_dying_dwell_ticks(),
            sharp_aim_progress: 0.0,
            sharp_aim_active: false,
            sharp_aim_build_ticks: default_sharp_aim_build_ticks(),
            recoil_accumulator: 0.0,
            recoil_decay_rate: default_recoil_decay_rate(),
            recoil_alternation_sign: 1,
            mission_critical: false,
            inventory_dropped_on_dying: false,
            last_lethal_cause_event_id: None,
            walk_threshold: default_walk_threshold(),
            bloom_factor: default_bloom_factor(),
            resources: ResourceAccumulators::default(),
            afflictions: Vec::new(),
            m14g_wound_list: cf_wound::ActorWoundList::new(),
            affliction_speed_multiplier: 1.0,
            affliction_aim_spread_bonus_rad: 0.0,
            m14h_cardiac: cardiac::ActorCardiacComponent::new(),
            m14h_tourniquets: std::collections::BTreeMap::new(),
            m14h_buffs: Vec::new(),
            m14h_last_defib_tick: None,
            m14h_antibiotic_course: None,
            facing: FacingDirection::Right,
            stamina: Stamina::full(),
            lean_state: LeanState::default(),
            cover_state: CoverState::open(),
            sprint_active: false,
            prone_active: false,
            cinematic_ticks_remaining: 0,
            cinematic_kind: None,
            stealth_meter: 0.0,
            limb_loss: LimbLossFlags::default(),
            inventory_weight_kg: 0.0,
            bipod: cf_equipment::Bipod::equipped_default(),
            suppressor: cf_equipment::Suppressor::default(),
            tool_durability: std::collections::BTreeMap::new(),
            weapon_swap_in_progress: false,
            grenade_cook_seconds: 0.0,
            grenade_held_kind: Some(cf_equipment::GrenadeKind::Frag),
            grenade_held_fuse_remaining: 5.0,
            drill_heat: 0.0,
            reveal_until_tick: 0,
            weapon_fire_mode: cf_equipment::AdvancedFireMode::Single,
            weapon_charge_fraction: 0.0,
            burst3_remaining_shots: 0,
            burst3_next_fire_at_seconds: 0.0,
            fire_held_prev: false,
            inventory_grid: None,
            inventory_encumbrance: None,
            body_armor: BodyArmorSlot::default(),
            crewing_fortification_id: None,
            crossing: None,
            move_state: crate::move_state::MoveState::default(),
            prone_state: crate::move_state::ProneState::default(),
            upper_body_state: crate::move_state::UpperBodyState::default(),
            attitude: crate::attitude::AttitudeState::default(),
            walk_angle: crate::attitude::WalkAngleState::default(),
            walk_path_offset: crate::attitude::WalkPathOffset::default(),
            arm_sway: crate::arm_sway::ArmSwayState::default(),
            stride_frame: false,
            stride_start: true,
            stride_timer_ms: 0,
            last_stride_side_fg: false,
            limb_paths: crate::limb_path::default_infantry_registry(),
            jetpack: None,
            quick_action_bar: crate::quick_action::QuickActionBarState::infantry_default(),
            total_mass_cached: 80.0,
            total_mass_dirty: true,
            wound_mass_kg: 0.0,
            last_stride_tick: 0,
            q_press_tick: 0,
            q_held: false,
            last_used_quick_slot: 0,
            armor_scratch_level: std::collections::BTreeMap::new(),
            atmosphere_sample: AtmosphereSample::default(),
            last_hazard_contact_tick: std::collections::BTreeMap::new(),
            m14i_long_term: crate::long_term::LongTermState::default(),
            parkour_signal: crate::parkour::ParkourSignal::default(),
            mount: None,
            is_being_ridden: false,
            holding_rope: None,
            zipline_attached: None,
            zipline_brake_engaged: false,
            wall_jump_ticks_remaining_ms: 0,
            swim_breath_seconds: default_swim_breath_seconds(),
            swim_kind: crate::move_state::SwimKind::None,
            swim_drain_multiplier: default_swim_drain_multiplier(),
            swim_disabled_sinks: false,
        }
    }

    /// callers (cf-control engine) can spawn a `Wreck` static actor on the
    /// battlefield for `act.chassis.salvage` to operate on. Resets
    /// `half_extents` to the infantry baseline so the now-foot-infantry pilot
    /// uses the right collision proxy. Closes M5-DC-6 ("continues as foot
    /// infantry") and DR-014 ("Pilot rescue / ejection: Pilots/operators can
    /// survive a chassis loss. Eject, crawl out, get carried.").
    pub fn detach_chassis(&mut self) -> Option<cf_chassis::ChassisState> {
        let chassis = self.chassis.take()?;
        self.half_extents = Vec2::new(8.0, 16.0);
        // Reset movement flags since the now-foot pilot doesn't have jet/climb anymore.
        self.jet_active = false;
        self.climb_active = false;
        // Mark the detach so cf-control can emit `actor.chassis_detached` once.
        self.chassis_detached = true;
        Some(chassis)
    }

    /// M5: attach a chassis to this actor. Resizes the half_extents to fit the
    /// chassis silhouette for the given chassis kind so M5.5 collision proxies
    /// match the visible silhouette. **M13** adds CrabQuadruped + Drone
    /// archetypes — half-extents per spec § "Chassis archetypes — M13 ships 5".
    pub fn attach_chassis(&mut self, chassis: cf_chassis::ChassisState) {
        let (half_extents, mass) = match chassis.kind {
            cf_chassis::ChassisKind::Infantry => (Vec2::new(8.0, 16.0), 80.0),
            cf_chassis::ChassisKind::PoweredArmor => (Vec2::new(10.0, 20.0), 200.0),
            cf_chassis::ChassisKind::LightMech => (Vec2::new(18.0, 36.0), 600.0),
            // Crab: 4 legs spread it out laterally, shorter in y axis.
            cf_chassis::ChassisKind::CrabQuadruped => (Vec2::new(16.0, 14.0), 350.0),
            // Drone: small target on all sides.
            cf_chassis::ChassisKind::Drone => (Vec2::new(6.0, 6.0), 30.0),
            cf_chassis::ChassisKind::HeavyTrooper => (Vec2::new(11.0, 22.0), 380.0),
        };
        self.half_extents = half_extents;
        self.mass_kg = mass;
        // the attached chassis is a drone so cfctl + AI consumers see the
        // mode/fuel surface immediately.
        if chassis.kind == cf_chassis::ChassisKind::Drone && self.drone_ally.is_none() {
            self.drone_ally = Some(cf_chassis::DroneAllyState::default());
        }
        self.chassis = Some(chassis);
    }

    /// `InventoryEncumbrance` envelope. Idempotent — safe to call on
    /// already-initialized actors. Sets the encumbrance baseline from
    /// the actor's current `origin_id` so heavy_biomech / drone /
    /// robot get the spec-mandated `1.5× / 0.3× / 1.2×` carry scaling.
    pub fn inventory_grid_attach(&mut self) {
        if self.inventory_grid.is_none() {
            self.inventory_grid = Some(crate::inventory::InventoryGrid::default());
        }
        if self.inventory_encumbrance.is_none() {
            self.inventory_encumbrance = Some(crate::inventory::InventoryEncumbrance::for_origin(&self.origin_id));
        }
    }

    pub fn inventory_grid(&self) -> Option<&crate::inventory::InventoryGrid> {
        self.inventory_grid.as_ref()
    }

    pub fn inventory_grid_mut(&mut self) -> Option<&mut crate::inventory::InventoryGrid> {
        self.inventory_grid.as_mut()
    }

    /// (recursive). Returns `0.0` when no grid is attached.
    pub fn inventory_grid_total_mass_kg(&self) -> f32 {
        if let Some(grid) = &self.inventory_grid {
            grid.total_mass_kg()
        } else {
            // Fallback to the legacy M6 slot-weight surface so
            // mass_aggregator::total_mass still reports a sensible
            // value for chassis-less / pre-M6B actors.
            self.inventory_weight_kg.max(0.0)
        }
    }

    /// inventory grid (recursive). Returns `0.0` when no grid is
    /// attached.
    pub fn inventory_grid_total_bulk_l(&self) -> f32 {
        self.inventory_grid
            .as_ref()
            .map_or(0.0, crate::inventory::InventoryGrid::total_bulk_l)
    }

    /// live `inventory_grid` totals AND from the actor's current
    /// `origin_id`. Caller drives this from the engine tick (and after
    /// pickup / drop / liquid-fill / set_origin events). Returns the
    /// new envelope (or `None` when no envelope is attached).
    ///
    /// **Important**: rebasing on `origin_id` here means that an actor
    /// whose origin changes mid-game (M17 origin swap, brain hop) sees
    /// the per-origin `max_carry_kg` modifier applied automatically on
    /// the next tick — no extra plumbing in cfctl required.
    pub fn recompute_inventory_encumbrance(&mut self) -> Option<crate::inventory::InventoryEncumbrance> {
        let total_mass = self.inventory_grid_total_mass_kg();
        let total_bulk = self.inventory_grid_total_bulk_l();
        let origin_id = self.origin_id.clone();
        let env = self.inventory_encumbrance.as_mut()?;
        env.rebaseline_for_origin(&origin_id);
        env.set_carried(total_mass, total_bulk);
        Some(*env)
    }

    /// Falls back to the human baseline when no envelope is attached.
    pub fn max_carry_kg(&self) -> f32 {
        self.inventory_encumbrance
            .map_or(cf_equipment::HUMAN_BASELINE_MAX_CARRY_KG, |e| e.max_carry_kg)
    }

    pub fn max_carry_volume_l(&self) -> f32 {
        self.inventory_encumbrance
            .map_or(cf_equipment::HUMAN_BASELINE_MAX_CARRY_VOLUME_L, |e| {
                e.max_carry_volume_l
            })
    }

    /// envelope (`1.0` empty, `0.5` at 100%). Returns `1.0` when no
    /// envelope is attached.
    pub fn encumbrance_walk_speed_multiplier(&self) -> f32 {
        self.inventory_encumbrance.map_or(1.0, |e| e.walk_speed_multiplier)
    }

    /// (`None` / `Light` / `Moderate` / `Heavy`).
    pub fn encumbrance_band(&self) -> cf_equipment::EncumbranceBand {
        self.inventory_encumbrance
            .map_or(cf_equipment::EncumbranceBand::None, |e| e.band)
    }

    /// "ENCUMBERED" warning per spec § "Encumbrance at 100% reduces
    /// walk speed").
    pub fn is_encumbered(&self) -> bool {
        self.inventory_encumbrance.is_some_and(|e| e.encumbered())
    }

    /// to this actor based on the zone struck. Replaces any in-flight
    /// reaction. `tick_rate_hz` is used to derive the duration in ticks.
    pub fn apply_hit_reaction(&mut self, zone: cf_chassis::BodyZone, tick_rate_hz: u32) -> cf_chassis::HitReaction {
        let reaction = cf_chassis::HitReaction::for_zone(zone);
        self.hit_reaction_ticks_remaining = reaction.duration_ticks(tick_rate_hz);
        self.hit_reaction_kind = reaction.kind.to_string();
        self.hit_reaction_speed_factor = reaction.speed_factor;
        reaction
    }

    /// one tick. Returns `true` on the tick the window expires (clears state).
    pub fn tick_hit_reaction(&mut self) -> bool {
        if self.hit_reaction_ticks_remaining == 0 {
            return false;
        }
        self.hit_reaction_ticks_remaining -= 1;
        if self.hit_reaction_ticks_remaining == 0 {
            self.hit_reaction_kind.clear();
            self.hit_reaction_speed_factor = 1.0;
            return true;
        }
        false
    }

    /// `total_mass_cached` if dirty, then return it.
    pub fn total_mass_kg(&mut self) -> f32 {
        if self.total_mass_dirty {
            self.total_mass_cached = crate::mass_aggregator::total_mass(self);
            self.total_mass_dirty = false;
        }
        self.total_mass_cached
    }

    pub fn walk_speed_mass_factor(&mut self) -> f32 {
        let _ = self.total_mass_kg();
        crate::mass_aggregator::mass_factor(self)
    }

    pub fn mark_mass_dirty(&mut self) {
        self.total_mass_dirty = true;
    }

    /// Marks mass dirty so the next `total_mass_kg()` call recomputes.
    pub fn equip_jetpack(&mut self, jetpack: cf_equipment::Jetpack) {
        self.jetpack = Some(jetpack);
        self.mark_mass_dirty();
    }

    pub fn drop_jetpack(&mut self) -> Option<cf_equipment::Jetpack> {
        self.mark_mass_dirty();
        self.jetpack.take()
    }

    /// default layout. Called after `attach_chassis`.
    pub fn reseat_quick_action_bar_for_chassis(&mut self) {
        let kind = self.chassis.as_ref().map(|c| c.kind);
        self.quick_action_bar = match kind {
            Some(cf_chassis::ChassisKind::PoweredArmor) => {
                crate::quick_action::QuickActionBarState::powered_armor_default()
            }
            Some(cf_chassis::ChassisKind::LightMech) => {
                crate::quick_action::QuickActionBarState::light_mech_default()
            }
            Some(cf_chassis::ChassisKind::HeavyTrooper) => {
                crate::quick_action::QuickActionBarState::heavy_trooper_default()
            }
            _ => crate::quick_action::QuickActionBarState::infantry_default(),
        };
    }

    /// Sets `stride_frame=true` (consumed end-of-tick), advances
    /// `last_stride_side_fg`, records `last_stride_tick`.
    pub fn emit_stride(&mut self, tick: u64, side_fg: bool) {
        self.stride_frame = true;
        self.last_stride_side_fg = side_fg;
        self.last_stride_tick = tick;
        self.stride_start = false;
        self.stride_timer_ms = 0;
    }

    /// active hazard zones (EM disruption disables slots 6/7/8 by default).
    pub fn apply_em_disruption(&mut self, em_disrupted: bool) {
        if em_disrupted {
            self.quick_action_bar.apply_hazard_disabled_slots(&[5, 6, 7]);
        } else {
            self.quick_action_bar.apply_hazard_disabled_slots(&[]);
        }
    }

    /// per-zone scratch level when external HP drops past thresholds.
    /// Returns `true` when the level changed (caller emits decal-spawn event).
    pub fn maybe_advance_armor_scratch(&mut self, zone_name: &str, external_pct: f32) -> bool {
        let new_level = if external_pct >= 0.7 {
            1
        } else if external_pct >= 0.4 {
            2
        } else if external_pct > 0.0 {
            3
        } else {
            0
        };
        let prev = self.armor_scratch_level.get(zone_name).copied().unwrap_or(0);
        if new_level > prev {
            self.armor_scratch_level.insert(zone_name.to_string(), new_level);
            true
        } else {
            false
        }
    }

    /// exceeds the stagger threshold, scaled by per-zone `stagger_factor`.
    pub fn knockdown_check(&mut self, incoming_impulse_n_s: f32, zone: cf_chassis::BodyZone) -> crate::attitude::KnockdownOutcome {
        let total_mass = self.total_mass_kg();
        let stagger_factor = self
            .chassis
            .as_ref()
            .and_then(|c| c.zones.iter().find(|z| z.zone == zone))
            .map(|z| z.stagger_factor)
            .unwrap_or(1.0);
        let effective_impulse = incoming_impulse_n_s / stagger_factor.max(0.05);
        crate::attitude::evaluate_knockdown(effective_impulse, total_mass)
    }

    /// per-material hazard contact via a debounce window.
    pub fn maybe_emit_hazard_contact(&mut self, zone_name: &str, current_tick: u64, debounce_ticks: u64) -> bool {
        let last = self.last_hazard_contact_tick.get(zone_name).copied().unwrap_or(0);
        if current_tick.saturating_sub(last) >= debounce_ticks {
            self.last_hazard_contact_tick.insert(zone_name.to_string(), current_tick);
            true
        } else {
            false
        }
    }

    pub fn mark_brain(&mut self, tick: u64) {
        self.is_brain = true;
        self.mission_critical = true;
        self.last_brain_hop_tick = tick;
        self.brain_last_position = [self.position.x, self.position.y];
    }

    /// player hops out of this actor). Records the last known position so
    /// the AI fallback can reason about where the brain WAS.
    ///
    /// `mark_brain` sets BOTH `is_brain = true` and
    /// `mission_critical = true`; without clearing both here, every actor
    /// the player ever hopped through would remain capped-at-Dying
    /// indefinitely, accumulating "zombie" actors over a mission.
    pub fn clear_brain(&mut self) {
        self.is_brain = false;
        self.mission_critical = false;
        self.brain_last_position = [self.position.x, self.position.y];
    }

    /// Reset the actor back to its spawn state. Position, velocity, aim, on-ground,
    /// status, and HP all return to defaults; the selected inventory slot is cleared
    /// to `0` so the actor can fire its rifle again after `act.player.reset`.
    /// Inventory items themselves are not rewound (slot contents are immutable in M1).
    pub fn reset(&mut self) {
        self.position = self.spawn;
        self.velocity = Vec2::ZERO;
        self.aim = Vec2::new(1.0, 0.0);
        self.on_ground = false;
        self.status = Status::Stable;
        self.hp = self.hp_max;
        self.inventory.selected = ItemSlot(0);
        self.stability = 1.0;
        self.knockdown_ticks_remaining = 0;
        self.dying_dwell_ticks_remaining = 0;
        self.sharp_aim_progress = 0.0;
        self.sharp_aim_active = false;
        self.recoil_accumulator = 0.0;
        self.recoil_alternation_sign = 1;
        self.inventory_dropped_on_dying = false;
        self.last_lethal_cause_event_id = None;
        self.bloom_factor = default_bloom_factor();
        self.crouch_active = false;
        self.climb_active = false;
        self.jet_active = false;
        if let Some(chassis) = self.chassis.as_mut() {
            chassis.reset();
        }
        self.facing = FacingDirection::Right;
        self.stamina.reset();
        self.lean_state.reset();
        self.cover_state = CoverState::open();
        self.sprint_active = false;
        self.prone_active = false;
        self.cinematic_ticks_remaining = 0;
        self.cinematic_kind = None;
        self.stealth_meter = 0.0;
        self.inventory_weight_kg = 0.0;
        self.bipod = cf_equipment::Bipod::equipped_default();
        self.suppressor = cf_equipment::Suppressor::default();
        self.tool_durability.clear();
        self.weapon_swap_in_progress = false;
        self.grenade_cook_seconds = 0.0;
        self.grenade_held_kind = Some(cf_equipment::GrenadeKind::Frag);
        self.grenade_held_fuse_remaining = 5.0;
        self.drill_heat = 0.0;
        self.reveal_until_tick = 0;
        self.weapon_fire_mode = cf_equipment::AdvancedFireMode::Single;
        self.weapon_charge_fraction = 0.0;
        self.burst3_remaining_shots = 0;
        self.burst3_next_fire_at_seconds = 0.0;
        self.fire_held_prev = false;
        // reset actor's encumbrance band returns to `None` and HUD widgets
        // clear the "ENCUMBERED" warning.
        if let Some(grid) = self.inventory_grid.as_mut() {
            grid.items.clear();
            grid.next_instance_id = 1;
        }
        if let Some(env) = self.inventory_encumbrance.as_mut() {
            env.set_carried(0.0, 0.0);
        }
    }

    /// Apply damage with a cause string. Returns the new status if it changed.
    ///
    /// the chassis pipeline (armor layers → wound → actor HP) via
    /// [`ActorState::apply_zone_damage`]. The legacy direct-HP path is preserved
    /// for actors without a chassis.
    ///
    /// (cutscene safety).
    ///
    /// DYING (per spec literal "caps at DYING (does not reach DEAD)"). HP
    /// can reach 0; the actor enters DYING; the DYING dwell never elapses
    /// to DEAD while `mission_critical=true` (the dwell-elapsed branch in
    /// `cf-actor::sim::step_one_actor` honors `dying_cap_in_effect`,
    /// which is true for mission_critical actors). Prior behaviour
    /// clamped HP at the Downed threshold; that was spec drift.
    pub fn apply_damage(&mut self, amount: f32) -> Option<Status> {
        if amount <= 0.0 || self.status.is_dead() || matches!(self.status, Status::Dying | Status::Inactive) {
            return None;
        }
        self.hp = (self.hp - amount).max(0.0);
        let new_status = self.derived_status();
        if new_status != self.status {
            self.status = new_status;
            // Seed DYING dwell on first entry.
            if matches!(new_status, Status::Dying) {
                self.dying_dwell_ticks_remaining = self.dying_dwell_ticks_default;
            }
            Some(new_status)
        } else {
            None
        }
    }

    /// callers consume the return for event emission.
    pub fn set_inactive(&mut self, inactive: bool) -> Option<Status> {
        if inactive && !matches!(self.status, Status::Inactive) {
            self.status = Status::Inactive;
            Some(Status::Inactive)
        } else if !inactive && matches!(self.status, Status::Inactive) {
            // Reactivate using the derived status from HP.
            self.status = if self.hp <= 0.0 { Status::Dying } else { Status::Stable };
            Some(self.status)
        } else {
            None
        }
    }

    /// M5: apply damage routed through a specific body zone (chassis grammar).
    /// Returns `(new_status_if_changed, zone_damage_outcome)`. The outcome
    /// describes every layer/module transition so the engine emits replay events.
    pub fn apply_zone_damage(
        &mut self,
        zone: cf_chassis::BodyZone,
        amount: f32,
        cause: &str,
    ) -> (Option<Status>, cf_chassis::ZoneDamageOutcome) {
        if amount <= 0.0 || !amount.is_finite() || self.status.is_dead() {
            return (None, cf_chassis::ZoneDamageOutcome::default());
        }
        //   - Bound / Injured (still inside chassis): damage routes through
        //     the chassis layered armor.
        //   - Ejecting: pilot is mid-bail in a sealed eject capsule; damage
        //     is heavily reduced (10%) to give the player a tactical window
        //     to escape without instantly dying mid-eject.
        //   - Ejected: pilot is mid-air infantry, briefly under fire from
        //     the wreck. Damage routes to actor HP at quarter rate (the
        //     parachute drop offers some concealment / fall trajectory).
        //   - Extracted: pilot is in safety zone, no damage.
        //   - BailedTooLate / Lost: chassis is gone; damage routes to actor
        //     HP directly.
        let pilot_state = self.chassis.as_ref().map(|c| c.pilot_state);
        let route_through_chassis = matches!(
            pilot_state,
            Some(cf_chassis::PilotState::Bound | cf_chassis::PilotState::Injured | cf_chassis::PilotState::Ejecting)
        );
        let damage_scale = match pilot_state {
            Some(cf_chassis::PilotState::Extracted) => 0.0,
            Some(cf_chassis::PilotState::Ejecting) => 0.1,
            Some(cf_chassis::PilotState::Ejected) => 0.25,
            _ => 1.0,
        };
        let effective_amount = amount * damage_scale;
        if effective_amount <= 0.0 {
            return (None, cf_chassis::ZoneDamageOutcome::default());
        }
        let outcome = if route_through_chassis {
            self.chassis
                .as_mut()
                .map(|c| c.apply_zone_damage(zone, effective_amount, cause))
                .unwrap_or_default()
        } else {
            // No chassis OR pilot is outside the chassis: damage routes to
            // actor HP directly (at the scaled amount).
            cf_chassis::ZoneDamageOutcome {
                zone: Some(zone),
                cause: cause.to_string(),
                actor_hp_damage: effective_amount,
                ..Default::default()
            }
        };
        // Spill actor_hp_damage (overflow past every chassis layer + wound) to
        // actor.hp. Wound damage absorbed by the chassis does NOT spill to
        // actor.hp — the chassis IS the armor, and the wound container is the
        // last buffer before the pilot is hit. This keeps powered armor /
        // light mech chassis a meaningful HP buffer (220 hp torso = ~6 mech
        // autocannon hits before any actor.hp loss).
        let spill = outcome.actor_hp_damage;
        let prev = self.status;
        if spill > 0.0 {
            self.hp = (self.hp - spill).max(0.0);
        }
        // Re-derive status.
        let new_status = self.derived_status();
        if new_status != prev {
            self.status = new_status;
            (Some(new_status), outcome)
        } else {
            (None, outcome)
        }
    }

    fn derived_status(&self) -> Status {
        // Inactive is a sticky external state — never derive into it from HP.
        if matches!(self.status, Status::Inactive) {
            return Status::Inactive;
        }
        // Dying / Dead are sticky once entered: the DYING dwell tick logic owns
        // the DYING→DEAD transition; status-from-hp must not snap back to alive
        // if HP somehow rises during dwell.
        if matches!(self.status, Status::Dying | Status::Dead) {
            return self.status;
        }
        // lethal impulse". `is_robot_origin()` returns true for synth /
        // robot origin_id. Robot actors at 0 HP go straight to DEAD; they
        // never enter UNSTABLE / DOWNED / DYING.
        if self.is_robot_origin() {
            if self.hp <= 0.0 {
                return Status::Dead;
            }
            return Status::Stable;
        }
        if self.hp <= 0.0 {
            // CCCP Actor.cpp:1229 — HP=0 enters DYING, NOT DEAD directly.
            // Mission-critical actors cap here so the mission director (M1.5+)
            // can react before final loss.
            Status::Dying
        } else if self.hp <= self.hp_downed_threshold {
            Status::Downed
        } else if self.hp <= self.hp_unstable_threshold {
            Status::Unstable
        } else {
            Status::Stable
        }
    }

    pub fn is_robot_origin(&self) -> bool {
        matches!(self.origin_id.as_str(), "robot" | "synth")
    }

    /// Derived stance for HUD + `cfctl observe`. M5 routes through
    /// [`Stance::from_chassis`] so crouch / climb / jet / eject signals propagate.
    ///
    /// rope hanging / swinging, zip-lining, mount, and swim surface / submerged.
    pub fn stance(&self) -> Stance {
        if self.knockdown_ticks_remaining > 0 {
            return Stance::KnockedDown;
        }
        // kinematic stance derivation because they pin the actor's locomotion
        // mode independent of velocity / on_ground.
        if self.parkour_signal.vault_ticks_remaining_ms > 0 {
            return Stance::Vault;
        }
        if self.wall_jump_ticks_remaining_ms > 0 {
            return Stance::WallJump;
        }
        if self.zipline_attached.is_some() {
            return Stance::Ziplining;
        }
        if self.mount.is_some() {
            return Stance::Mounted;
        }
        if self.holding_rope.is_some() {
            // If the actor has lateral velocity past a threshold along the
            // rope-hanging axis we treat it as a swing.
            let swing_speed = self.velocity.x.abs();
            if swing_speed > 1.5 {
                return Stance::RopeSwinging;
            }
            return Stance::RopeHanging;
        }
        match self.swim_kind {
            crate::move_state::SwimKind::SurfaceBreast | crate::move_state::SwimKind::SurfaceFreestyle => {
                return Stance::SwimSurface;
            }
            crate::move_state::SwimKind::Dive | crate::move_state::SwimKind::Tread => {
                return Stance::SwimSubmerged;
            }
            crate::move_state::SwimKind::None => {}
        }
        let ejecting = self
            .chassis
            .as_ref()
            .is_some_and(|c| matches!(c.pilot_state, cf_chassis::PilotState::Ejecting));
        Stance::from_chassis(
            self.velocity,
            self.on_ground,
            self.status,
            self.crouch_active,
            self.climb_active,
            self.jet_active,
            ejecting,
        )
    }

    /// Body silhouette per-zone hp percentage. M5 reads from the chassis when
    /// present; otherwise falls back to the M4A flat-HP projection.
    /// `placeholder = false` when sourced from a real chassis body graph.
    pub fn body_silhouette(&self) -> BodySilhouette {
        if let Some(chassis) = self.chassis.as_ref() {
            let pct = |zone: cf_chassis::BodyZone| -> f32 {
                chassis.zone(zone).map_or(0.0, cf_chassis::ZoneState::zone_integrity)
            };
            BodySilhouette {
                head_hp_pct: pct(cf_chassis::BodyZone::Head),
                torso_hp_pct: pct(cf_chassis::BodyZone::Torso),
                arm_left_hp_pct: pct(cf_chassis::BodyZone::ArmLeft),
                arm_right_hp_pct: pct(cf_chassis::BodyZone::ArmRight),
                leg_left_hp_pct: pct(cf_chassis::BodyZone::LegLeft),
                leg_right_hp_pct: pct(cf_chassis::BodyZone::LegRight),
                placeholder: false,
            }
        } else {
            let pct = if self.hp_max > 0.0 {
                (self.hp / self.hp_max).clamp(0.0, 1.0)
            } else {
                0.0
            };
            BodySilhouette {
                head_hp_pct: pct,
                torso_hp_pct: pct,
                arm_left_hp_pct: pct,
                arm_right_hp_pct: pct,
                leg_left_hp_pct: pct,
                leg_right_hp_pct: pct,
                placeholder: true,
            }
        }
    }

    /// a `ModuleStrip` with `placeholder=false`. For chassis-less actors
    /// callers should fall back to the M4A placeholder strip.
    pub fn chassis_module_strip(&self) -> Option<ModuleStrip> {
        let chassis = self.chassis.as_ref()?;
        let modules = chassis
            .modules
            .iter()
            .map(|m| ModuleState {
                id: m.id.clone(),
                label: match m.kind {
                    cf_chassis::ModuleKind::WeaponMount => "WEAPON".to_string(),
                    cf_chassis::ModuleKind::Jet => "JET".to_string(),
                    cf_chassis::ModuleKind::Shield => "SHIELD".to_string(),
                    cf_chassis::ModuleKind::Sensor => "SENSOR".to_string(),
                    cf_chassis::ModuleKind::RepairDrone => "REPAIR".to_string(),
                    cf_chassis::ModuleKind::Cockpit => "COCKPIT".to_string(),
                    cf_chassis::ModuleKind::AmmoRack => "AMMO".to_string(),
                    cf_chassis::ModuleKind::Engine => "ENGINE".to_string(),
                    cf_chassis::ModuleKind::Optics => "OPTICS".to_string(),
                    cf_chassis::ModuleKind::Transmission => "TRANS".to_string(),
                    cf_chassis::ModuleKind::Reactor => "REACTOR".to_string(),
                    cf_chassis::ModuleKind::PowerCore => "POWER".to_string(),
                    cf_chassis::ModuleKind::FuelTank => "FUEL".to_string(),
                    cf_chassis::ModuleKind::TargetingComputer => "TARGET".to_string(),
                    cf_chassis::ModuleKind::CommRelay => "COMM".to_string(),
                    cf_chassis::ModuleKind::MotorController => "MOTOR".to_string(),
                    cf_chassis::ModuleKind::Era => "ERA".to_string(),
                },
                state: m.state.as_str().to_string(),
                kind: m.kind.as_str().to_string(),
            })
            .collect();
        Some(ModuleStrip {
            modules,
            placeholder: false,
        })
    }

    /// Returns `None` if no chassis is attached.
    pub fn chassis_view(&self) -> Option<ChassisView> {
        let c = self.chassis.as_ref()?;
        let zones = c
            .zones
            .iter()
            .map(|z| ChassisZoneView {
                zone: z.zone.as_str().to_string(),
                external_integrity: z.external_integrity(),
                internal_integrity: z.internal_integrity(),
                core_integrity: z.core_integrity(),
                wound_integrity: z.wound_integrity(),
                destroyed: z.destroyed,
                zone_integrity: z.zone_integrity(),
            })
            .collect();
        let modules = c
            .modules
            .iter()
            .map(|m| ChassisModuleView {
                id: m.id.clone(),
                kind: m.kind.as_str().to_string(),
                state: m.state.as_str().to_string(),
                bound_zone: m.bound_zone.as_str().to_string(),
                integrity: m.integrity(),
                last_reason: m.last_reason.clone(),
            })
            .collect();
        Some(ChassisView {
            spec_id: c.spec_id.clone(),
            kind: c.kind.as_str().to_string(),
            stage: c.stage.as_str().to_string(),
            pilot_state: c.pilot_state.as_str().to_string(),
            weapon_jammed: c.weapon_jammed,
            tutorial_safety: c.tutorial_safety,
            mass_kg: c.mass_kg,
            zones,
            modules,
            integrity: c.integrity(),
            eject_ticks_remaining: c.eject_window.ticks_remaining,
            eject_ticks_total: c.eject_window.ticks_total,
            destroyed_zones: c.destroyed_zones().iter().map(|z| z.as_str().to_string()).collect(),
            salvaged_module_ids: c.salvaged_modules.iter().map(|m| m.id.clone()).collect(),
        })
    }

    /// Hash bytes for the M1 deterministic checksum extension. Layout-stable; future
    /// milestones append fields without bumping the schema. Field encodings are picked
    /// to round-trip the full source domain — the inventory slot writes its full `u32`
    /// (`ItemSlot.0.to_le_bytes()`) so growing the inventory beyond 255 slots in a
    /// future milestone cannot silently collide divergent states into the same hash.
    /// M1 bytes so existing checksums stay byte-stable for chassis-less actors;
    /// chassis-grade actors get a richer checksum.
    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(96);
        out.extend_from_slice(&self.id.0.to_le_bytes());
        out.extend_from_slice(&quantize_f32(self.position.x).to_le_bytes());
        out.extend_from_slice(&quantize_f32(self.position.y).to_le_bytes());
        out.extend_from_slice(&quantize_f32(self.velocity.x).to_le_bytes());
        out.extend_from_slice(&quantize_f32(self.velocity.y).to_le_bytes());
        out.extend_from_slice(&quantize_f32(self.aim.x).to_le_bytes());
        out.extend_from_slice(&quantize_f32(self.aim.y).to_le_bytes());
        out.extend_from_slice(&quantize_f32(self.hp).to_le_bytes());
        out.push(self.status as u8);
        out.push(u8::from(self.on_ground));
        out.extend_from_slice(&self.inventory.selected.0.to_le_bytes());
        out.extend_from_slice(&quantize_f32(self.stability).to_le_bytes());
        out.extend_from_slice(&self.knockdown_ticks_remaining.to_le_bytes());
        // adds: sharp_aim_q16, mass_q16, origin_id_u8. Without these
        // three fields, two actors with identical pos/vel/aim/hp but
        // different sharp_aim charge or origin hash identically, hiding
        // a real determinism drift. Origin defaults to 0 (Human) at M4
        // since the multi-origin model lands at M9/M17.
        out.extend_from_slice(&quantize_f32(self.sharp_aim_progress).to_le_bytes());
        out.extend_from_slice(&quantize_f32(self.mass_kg).to_le_bytes());
        out.push(0u8); // origin_id placeholder; M9+ fills with origin discriminator
                       // M5: append chassis bytes only when a chassis is attached; legacy actors
                       // remain byte-identical for cross-milestone determinism comparisons.
        if let Some(chassis) = &self.chassis {
            out.push(1);
            out.extend_from_slice(&chassis.checksum_bytes());
            out.push(u8::from(self.crouch_active));
            out.push(u8::from(self.climb_active));
            out.push(u8::from(self.jet_active));
        }
        // save/load round-trips preserve M14G simulation state. Append-only
        // (never reorder existing fields) to keep cross-milestone
        // determinism comparisons stable.
        if !self.m14g_wound_list.wounds_by_zone.is_empty()
            || !self.m14g_wound_list.necrotic_zones.is_empty()
            || self.m14g_wound_list.next_id != 0
        {
            out.push(1);
            out.extend_from_slice(&self.m14g_wound_list.checksum_bytes());
        }
        // buff + antibiotic-course state when any field deviates from
        // default. Append-only.
        let cardiac_default = cardiac::ActorCardiacComponent::default();
        if self.m14h_cardiac != cardiac_default
            || !self.m14h_tourniquets.is_empty()
            || !self.m14h_buffs.is_empty()
            || self.m14h_last_defib_tick.is_some()
            || self.m14h_antibiotic_course.is_some()
        {
            out.push(1);
            out.push(if self.m14h_cardiac.in_arrest { 1 } else { 0 });
            out.extend_from_slice(&self.m14h_cardiac.onset_tick.to_le_bytes());
            out.extend_from_slice(&self.m14h_cardiac.cpr_rounds_total.to_le_bytes());
            out.extend_from_slice(&self.m14h_cardiac.consecutive_cpr_rounds.to_le_bytes());
            out.extend_from_slice(&self.m14h_cardiac.charges_remaining.to_le_bytes());
            out.extend_from_slice(&self.m14h_cardiac.defib_shocks.to_le_bytes());
            out.push(if self.m14h_cardiac.chest_bruised { 1 } else { 0 });
            out.extend_from_slice(&(self.m14h_tourniquets.len() as u64).to_le_bytes());
            for (zone, tick) in &self.m14h_tourniquets {
                out.extend_from_slice(zone.as_str().as_bytes());
                out.push(0);
                out.extend_from_slice(&tick.to_le_bytes());
            }
            out.extend_from_slice(&(self.m14h_buffs.len() as u64).to_le_bytes());
            for buff in &self.m14h_buffs {
                out.push(buff.kind as u8);
                out.extend_from_slice(&buff.applied_tick.to_le_bytes());
                out.extend_from_slice(&buff.expires_tick.to_le_bytes());
            }
            match self.m14h_last_defib_tick {
                Some(t) => {
                    out.push(1);
                    out.extend_from_slice(&t.to_le_bytes());
                }
                None => out.push(0),
            }
            match self.m14h_antibiotic_course.as_ref() {
                Some(s) => {
                    out.push(1);
                    out.push(s.tier);
                    out.extend_from_slice(&s.doses_taken.to_le_bytes());
                    out.extend_from_slice(&s.doses_required.to_le_bytes());
                    out.extend_from_slice(&s.dose_interval_hours.to_le_bytes());
                    out.extend_from_slice(&s.next_dose_tick.to_le_bytes());
                    out.push(if s.resistance_risk { 1 } else { 0 });
                }
                None => out.push(0),
            }
        }
        // long-term state (scars, biological age, prosthetics, traits,
        // severed-limb tracking, concussion count, chronic-pain
        // baseline, radiation dose). Append-only.
        if !self.m14i_long_term.is_empty() {
            out.push(1);
            out.extend_from_slice(&self.m14i_long_term.checksum_bytes());
        }
        out
    }
}

/// Quantize an `f32` to a deterministic `i32` representation for cross-platform checksum
/// stability. Per-pixel resolution is plenty for the M1 actor; finer scales can append.
pub(crate) fn quantize_f32(value: f32) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    (value * 1024.0).round() as i32
}
