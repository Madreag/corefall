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
    clippy::needless_pass_by_value
)]

pub mod arm_sway;
pub mod atmosphere_contact;
pub mod attachable;
pub mod attitude;
pub mod body_armor_slot;
pub mod components;
pub mod constants;
pub mod cover;
pub mod gib;
pub mod inventory;
pub mod lean;
pub mod limb_path;
pub mod mass_aggregator;
pub mod material_contact;
pub mod move_state;
pub mod quick_action;
pub mod resource_drain;
pub mod sim;
pub mod sim_overlay;
pub mod stamina;
pub mod stance;
pub mod systems;
pub mod ttd;
pub mod walking_sim;

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
pub use move_state::{MoveState, ProneState, UpperBodyState};
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

/// **M6**: side-view facing direction. Updated when the player aims; the
/// sprite renderer flips horizontally on change. M13 chassis adds armor
/// zone visibility per facing direction (spec § "Side-view facing direction
/// + limb-loss action restrictions (M13 forward-compat)").
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FacingDirection {
    Left = 0,
    Right = 1,
}

impl Default for FacingDirection {
    fn default() -> Self {
        FacingDirection::Right
    }
}

impl FacingDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            FacingDirection::Left => "left",
            FacingDirection::Right => "right",
        }
    }

    /// Flip the facing direction.
    pub fn flipped(self) -> Self {
        match self {
            FacingDirection::Left => FacingDirection::Right,
            FacingDirection::Right => FacingDirection::Left,
        }
    }

    /// Derive facing from an aim vector. Right-half-plane returns Right.
    pub fn from_aim(aim: Vec2) -> Self {
        if aim.x >= 0.0 {
            FacingDirection::Right
        } else {
            FacingDirection::Left
        }
    }

    pub fn sign(self) -> f32 {
        match self {
            FacingDirection::Left => -1.0,
            FacingDirection::Right => 1.0,
        }
    }
}

/// **M6 (M13 forward-compat)**: per-limb loss tracking. M6 only sets these
/// flags via scenario seed / debug commands so the action-rejection surface
/// can be tested before M13 chassis ships full limb damage routing. Each
/// flag rejects a specific action category per spec § "Limb-loss action
/// restrictions".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LimbLossFlags {
    pub both_arms_lost: bool,
    pub single_arm_lost: bool,
    pub both_legs_lost: bool,
    pub single_leg_lost: bool,
    pub both_hands_lost: bool,
    pub backpack_lost: bool,
    pub head_destroyed: bool,
    pub torso_destroyed: bool,
}

impl LimbLossFlags {
    /// True when the actor cannot fire any weapon (both arms lost or both
    /// hands lost).
    pub fn weapon_fire_disabled(self) -> bool {
        self.both_arms_lost || self.both_hands_lost
    }

    /// True when two-hand weapons are rejected (single arm lost).
    pub fn two_hand_weapon_disabled(self) -> bool {
        self.single_arm_lost
    }

    /// True when sprint / jump / vault / climb are rejected.
    pub fn movement_disabled(self) -> bool {
        self.both_legs_lost
    }

    /// True when sprint / jump are rejected (but reduced-mobility movement still ok).
    pub fn sprint_disabled(self) -> bool {
        self.both_legs_lost || self.single_leg_lost
    }

    /// True for instant-death conditions.
    pub fn instant_death(self) -> bool {
        self.head_destroyed || self.torso_destroyed
    }

    /// Short reason label per spec § "Limb-loss action restrictions".
    /// Returns the most-specific reason that applies for the given action.
    pub fn reject_reason_for(self, action: &str) -> Option<&'static str> {
        match action {
            "fire" | "throw_grenade" | "knife_throw" => {
                if self.both_hands_lost {
                    Some("no_hands_for_grip")
                } else if self.both_arms_lost {
                    Some("no_arms_for_weapon")
                } else {
                    None
                }
            }
            "two_hand_fire" => {
                if self.single_arm_lost {
                    Some("single_arm_two_hand_weapon_rejected")
                } else {
                    None
                }
            }
            "sprint" | "jump" | "vault" | "climb_up" | "climb_down" => {
                if self.both_legs_lost {
                    Some("no_legs_for_movement")
                } else if self.single_leg_lost {
                    Some("single_leg_reduced_mobility")
                } else {
                    None
                }
            }
            "deploy_jet" => {
                if self.backpack_lost {
                    Some("backpack_lost_no_jet")
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use cf_equipment::{AdvancedFireMode, BipodState};

/// Stable per-actor id. Allocated by the scenario loader; future networking will
/// reuse the same id space across the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActorId(pub u64);

impl ActorId {
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Body status state machine (M1 surface; M3/M4/M5 expand wounds + chassis layers).
///
/// Lifecycle per CCCP `Actor.h:33`:
/// `Stable → Unstable → Downed → Dying → Dead` with `Inactive` as the
/// tutorial/cutscene escape hatch that pauses the state machine entirely.
///
/// `#[repr(u8)]` with explicit discriminants pins the layout used by
/// [`ActorState::checksum_bytes`]. New variants (`Inactive=4`, `Dying=5`) are
/// **appended after `Dead=3`** to preserve existing checksum byte layout —
/// inserting them between `Downed` and `Dead` would silently shift every
/// pre-M1 bundle's checksum. Order in the enum body is for readability; the
/// numeric tag is what `checksum_bytes` records.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Healthy and acting normally.
    Stable = 0,
    /// Below `hp_unstable_threshold`; movement still works but HUD warns.
    Unstable = 1,
    /// At/below `hp_downed_threshold`; player loses control but is not yet dying.
    Downed = 2,
    /// HP at zero; cannot recover this run.
    Dead = 3,
    /// Tutorial/cutscene pause. Inputs ignored; state machine frozen.
    /// Per CCCP `Actor.h:33` INACTIVE branch.
    Inactive = 4,
    /// HP reached zero; entering the DYING dwell window before transitioning
    /// to DEAD (CCCP `Actor.cpp:1229`; default 1000 ms / 60 ticks at 60 Hz).
    /// Inventory drop fires on entry to DYING.
    Dying = 5,
}

impl Status {
    /// True for the terminal "no more sim involvement" state.
    pub fn is_dead(self) -> bool {
        matches!(self, Status::Dead)
    }

    /// True if the actor can accept new control input (move / aim / fire / reload).
    /// `Dying` ignores input (death animation playing); `Inactive` ignores input
    /// (cutscene / tutorial pause).
    pub fn accepts_input(self) -> bool {
        matches!(self, Status::Stable | Status::Unstable)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Status::Stable => "stable",
            Status::Unstable => "unstable",
            Status::Downed => "downed",
            Status::Dying => "dying",
            Status::Dead => "dead",
            Status::Inactive => "inactive",
        }
    }
}

/// Readable stance/locomotion state derived from per-tick actor state.
///
/// M4A introduced the derivation surface; **M5 extends it with explicit
/// chassis-aware stances** (Crouching, Climbing, Jetting, Ejecting) so the
/// HUD + animation events + AI doctrine speak the same vocabulary.
/// `Stance::from_state` still derives the core six from velocity/grounded/status;
/// the chassis-aware extensions are surfaced via `Stance::from_chassis_state`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stance {
    /// On ground, stationary, accepting input.
    Idle = 0,
    /// On ground, horizontal velocity > walk threshold and < run threshold.
    Walking = 1,
    /// On ground, horizontal velocity >= run threshold.
    Running = 2,
    /// Off ground (jumping, falling).
    Airborne = 3,
    /// Status::Downed but still alive.
    Downed = 4,
    /// Status::Dead.
    Dead = 5,
    /// M5: ducked/crouching (set by `act.player.crouch`).
    Crouching = 6,
    /// M5: scaling a vertical surface (set by climb intent; placeholder cue).
    Climbing = 7,
    /// M5: airborne with jet thrust active (set by jet intent when module nominal).
    Jetting = 8,
    /// M5: pilot is mid-eject sequence (ChassisState.eject_window active).
    Ejecting = 9,
    /// W1.3: temporarily stunned by stability loss. Actor cannot accept input
    /// but is not Downed (will recover in knockdown_ticks_remaining ticks).
    KnockedDown = 10,
    /// M6: explicit upright-stationary stance (modern tactical shooter
    /// surface; distinct from `Idle` so HUD/AI can name it directly).
    Stand = 11,
    /// M6: sprinting (faster than running; depletes stamina).
    Sprint = 12,
    /// M6: crouch-walking (speed below run threshold while crouched).
    CrouchWalk = 13,
    /// M6: prone-stationary (lowest silhouette; bipod-deployable).
    Prone = 14,
    /// M6: prone-crawling (moving while prone).
    ProneWalk = 15,
    /// M6: sprint-to-crouch transition (600 ms with i-frames).
    Slide = 16,
    /// M6: vaulting over low cover (800 ms; transitions to other side).
    Vault = 17,
    /// M6: forward-roll evasive (600 ms; brief i-frames).
    Dive = 18,
    /// M6: leaning around corner (lean_angle in `ActorState::lean_angle`).
    Lean = 19,
    /// M6: explicit DYING dwell stance (visually distinct from Downed).
    Dying = 20,
    /// M6: rope climbing (reserved; ladder takes priority for M6).
    RopeClimb = 21,
    /// M6: ladder climbing (1500 ms transition to top).
    LadderClimb = 22,
    /// M6: pipe climbing.
    PipeClimb = 23,
    /// M6: stealth-kill animation in progress (instant kill once landed).
    StealthAttack = 24,
    /// M6: knife throw windup + release.
    KnifeThrow = 25,
    /// M16+ reserved: aquatic locomotion.
    Swim = 26,
    /// **M9C**: crewing a static fortification (MG nest / tripod / bunker
    /// firing slit). The bound fortification id is stored on
    /// `ActorState::crewing_fortification_id` so the flat enum stays
    /// binary-compatible while satisfying the spec note:
    /// `Stance::Crewing { fortification_id }` (M9C § Notes for the
    /// implementer). While in this stance, `cover_state` is Full,
    /// movement inputs are suspended, and primary fire is rebound to
    /// the fortification's mounted weapon.
    Crewing = 27,
}

impl Stance {
    /// Threshold below which horizontal velocity counts as `Idle` (world units / s).
    /// Mirrors `cf-physics::WALK_SPEED_FLOOR` in spirit; we keep the literal here so
    /// `cf-actor` does not depend on physics constants.
    pub const WALK_THRESHOLD: f32 = 8.0;
    /// Threshold at/above which horizontal velocity counts as `Running`.
    pub const RUN_THRESHOLD: f32 = 60.0;

    pub fn as_str(self) -> &'static str {
        match self {
            Stance::Idle => "idle",
            Stance::Walking => "walking",
            Stance::Running => "running",
            Stance::Airborne => "airborne",
            Stance::Downed => "downed",
            Stance::Dead => "dead",
            Stance::Crouching => "crouching",
            Stance::Climbing => "climbing",
            Stance::Jetting => "jetting",
            Stance::Ejecting => "ejecting",
            Stance::KnockedDown => "knocked_down",
            Stance::Stand => "stand",
            Stance::Sprint => "sprint",
            Stance::CrouchWalk => "crouch_walk",
            Stance::Prone => "prone",
            Stance::ProneWalk => "prone_walk",
            Stance::Slide => "slide",
            Stance::Vault => "vault",
            Stance::Dive => "dive",
            Stance::Lean => "lean",
            Stance::Dying => "dying",
            Stance::RopeClimb => "rope_climb",
            Stance::LadderClimb => "ladder_climb",
            Stance::PipeClimb => "pipe_climb",
            Stance::StealthAttack => "stealth_attack",
            Stance::KnifeThrow => "knife_throw",
            Stance::Swim => "swim",
            Stance::Crewing => "crewing",
        }
    }

    /// M6: returns true when this stance prevents the actor from firing
    /// (slide/vault/dive/stealth attack/knife throw + dead/downed/dying/knockdown).
    pub fn locks_fire(self) -> bool {
        matches!(
            self,
            Stance::Slide
                | Stance::Vault
                | Stance::Dive
                | Stance::StealthAttack
                | Stance::KnifeThrow
                | Stance::Dead
                | Stance::Downed
                | Stance::Dying
                | Stance::KnockedDown
                | Stance::Ejecting
        )
    }

    /// Derive stance from kinematic + status state. Pure; no clock reads.
    pub fn from_state(velocity: Vec2, on_ground: bool, status: Status) -> Stance {
        match status {
            Status::Dead => Stance::Dead,
            Status::Dying | Status::Downed => Stance::Downed,
            Status::Inactive => Stance::Idle,
            Status::Stable | Status::Unstable => {
                if !on_ground {
                    Stance::Airborne
                } else {
                    let speed = velocity.x.abs();
                    if speed >= Self::RUN_THRESHOLD {
                        Stance::Running
                    } else if speed >= Self::WALK_THRESHOLD {
                        Stance::Walking
                    } else {
                        Stance::Idle
                    }
                }
            }
        }
    }

    /// M5: derive stance from kinematic + status + chassis cues. Overrides the
    /// base stance with `Crouching` / `Climbing` / `Jetting` / `Ejecting` when
    /// the actor's chassis or movement-intent flags say so.
    #[allow(clippy::fn_params_excessive_bools)]
    pub fn from_chassis(
        velocity: Vec2,
        on_ground: bool,
        status: Status,
        crouch_active: bool,
        climb_active: bool,
        jet_active: bool,
        ejecting: bool,
    ) -> Stance {
        if ejecting {
            return Stance::Ejecting;
        }
        match status {
            Status::Dead => Stance::Dead,
            Status::Dying | Status::Downed => Stance::Downed,
            Status::Inactive => Stance::Idle,
            Status::Stable | Status::Unstable => {
                if jet_active {
                    return Stance::Jetting;
                }
                if climb_active {
                    return Stance::Climbing;
                }
                if !on_ground {
                    return Stance::Airborne;
                }
                if crouch_active {
                    return Stance::Crouching;
                }
                let speed = velocity.x.abs();
                if speed >= Self::RUN_THRESHOLD {
                    Stance::Running
                } else if speed >= Self::WALK_THRESHOLD {
                    Stance::Walking
                } else {
                    Stance::Idle
                }
            }
        }
    }
}

/// Item id used by the M1 inventory. Maps 1:1 to a slot index in [`Inventory::items`].
/// Resolved against per-actor item presets (`cf-equipment::RIFLE_M1_DEFAULT_ID`, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ItemSlot(pub u32);

impl ItemSlot {
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// 2D vector used by sim systems. We do NOT depend on `glam` here so this crate stays
/// dependency-light. The Bevy bridge converts to `Vec2`.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    /// Returns a unit vector. If the input is the zero vector OR contains a non-finite
    /// component (NaN / Inf), returns `Vec2::new(1.0, 0.0)` so consumers (e.g. weapon
    /// muzzle origin, projectile velocity, recoil) never produce NaNs. NaN comparisons
    /// always return false, so a plain `len < 1e-6` guard is NOT sufficient — we must
    /// explicitly check `is_finite()` on every component.
    pub fn normalize_or_x(self) -> Vec2 {
        if !self.x.is_finite() || !self.y.is_finite() {
            return Vec2::new(1.0, 0.0);
        }
        let len = self.length();
        // 1e-6 tolerance picked because: at f32 precision, vector lengths
        // below ~1.2e-38 underflow to zero outright (subnormal); below
        // ~1e-6 the per-component division `self.x / len` loses ~6
        // significant digits and the resulting "normalized" vector points
        // in essentially-arbitrary directions. 1e-6 is safely above that
        // floor at the canonical aim/muzzle-velocity scale (1 unit = 1 m,
        // typical aim magnitudes 0.1-1.0). When BP4-BP5 introduce
        // sub-millimeter precision physics (e.g., particle systems), this
        // should become scale-relative (issue #19 follow-up).
        if !len.is_finite() || len < 1e-6 {
            Vec2::new(1.0, 0.0)
        } else {
            Vec2::new(self.x / len, self.y / len)
        }
    }
}

impl std::ops::Add for Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl std::ops::Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, rhs: Vec2) -> Vec2 {
        Vec2::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, rhs: f32) -> Vec2 {
        Vec2::new(self.x * rhs, self.y * rhs)
    }
}

/// One inventory slot with a fixed item kind and (optional) ammo state.
///
/// Kept simple in M1: each actor has up to 4 slots. The "selected" slot drives
/// `weapon_fired` / `weapon_reloaded`. Slots beyond the rifle are placeholders for M5.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InventoryItem {
    /// Nothing in the slot.
    Empty,
    /// One M1 rifle preset. Concrete fire/reload state lives in `cf-equipment::RifleState`.
    /// We keep the slot shape here so the scenario manifest stays decoupled from rifle internals.
    Rifle { preset: String },
}

impl InventoryItem {
    pub fn label(&self) -> &str {
        match self {
            InventoryItem::Empty => "empty",
            InventoryItem::Rifle { .. } => "rifle",
        }
    }

    /// M4 § snapshot_inventory payload: high-level kind label for the
    /// slot. Currently mirrors `label()` but is distinct so future
    /// kinds (melee, tools, grenades) can expose a richer per-slot
    /// kind taxonomy without mutating the existing `label()` strings.
    pub fn kind_label(&self) -> &str {
        match self {
            InventoryItem::Empty => "empty",
            InventoryItem::Rifle { .. } => "rifle",
        }
    }

    pub fn is_rifle(&self) -> bool {
        matches!(self, InventoryItem::Rifle { .. })
    }
}

/// Up to four inventory slots. M1 ships one rifle; remaining slots are `Empty`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Inventory {
    pub items: Vec<InventoryItem>,
    pub selected: ItemSlot,
}

impl Default for Inventory {
    fn default() -> Self {
        // M6: 8 active slots per spec § Inventory. M1 / M1.5 / M2 scenarios
        // populate slot 0 with the rifle and leave 1..=7 empty; M6+ scenarios
        // can hand-author all 8 active slots. The 3 reserved tank slots are
        // surfaced only through `ActorObservation::inventory_extended`
        // (locked at M6; M17 fills GasTank instances).
        Self {
            items: vec![InventoryItem::Empty; 8],
            selected: ItemSlot(0),
        }
    }
}

impl Inventory {
    pub fn with_rifle(preset: &str) -> Self {
        let mut inv = Self::default();
        inv.items[0] = InventoryItem::Rifle {
            preset: preset.to_string(),
        };
        inv
    }

    pub fn selected_item(&self) -> &InventoryItem {
        self.items
            .get(self.selected.0 as usize)
            .unwrap_or(&InventoryItem::Empty)
    }

    /// Set `selected` to the requested slot iff the slot exists. Returns true if the
    /// selection changed.
    pub fn try_select(&mut self, slot: ItemSlot) -> bool {
        if (slot.0 as usize) < self.items.len() && self.selected != slot {
            self.selected = slot;
            true
        } else {
            false
        }
    }

    pub fn rifle_slot(&self) -> Option<ItemSlot> {
        self.items
            .iter()
            .enumerate()
            .find_map(|(i, it)| if it.is_rifle() { Some(ItemSlot(i as u32)) } else { None })
    }
}

/// Source of a `ControlIntent` for replay/audit.
/// **M14 audit pass 3 (GAP-M1-04)**: the M1 spec lists IntentSource as
/// `{Player, Ai, Replay, Script}`. The original implementation diverged
/// into `{Human, Cfctl, Ai, Replay}`. We accept both the spec names AND
/// the legacy names on the input side via serde aliases, but emit only
/// the spec-canonical names on the output side via `rename` so every
/// recorded event uses the published vocabulary.
///
/// **M14 audit pass 4 (Finding 6)**: serialization now actually emits
/// "player" / "script" — previously `#[serde(alias = ...)]` only
/// affected the input side, so serialized events kept emitting
/// "human" / "cfctl".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentSource {
    /// Keyboard / mouse / gamepad inside `cf-app`. Spec literal: `Player`.
    #[serde(rename = "player", alias = "human")]
    Human,
    /// JSON-RPC `act.player.*` calls coming from `cfctl` / scripted E2E /
    /// future bots. Spec literal: `Script`.
    #[serde(rename = "script", alias = "cfctl")]
    Cfctl,
    /// AI-driven intent from `cf-ai` reactive guard or future commander layer.
    Ai,
    /// Replay-driven intent from `cf-headless` replay verifier.
    Replay,
}

impl IntentSource {
    /// Spec-canonical name per M1 spec § "IntentSource = {Player, Ai, Replay, Script}".
    /// Mirrors the `#[serde(rename = ...)]` output strings exactly so
    /// engine sites that emit raw strings (HUD captions, log lines)
    /// match what serde would produce.
    pub fn spec_canonical_name(self) -> &'static str {
        match self {
            IntentSource::Human => "player",
            IntentSource::Cfctl => "script",
            IntentSource::Ai => "ai",
            IntentSource::Replay => "replay",
        }
    }
}

/// One tick's worth of player input. Produced by `cf-control` and applied by
/// [`ActorWorld::tick`]. Sticky vs. edge-triggered semantics matter:
///
/// - `move_x`, `aim`: continuous (latest value wins).
/// - `jump`, `fire`, `reload`, `selected_item`, `reset`: edge-triggered (true only on
///   the tick the button was pressed; cleared by the engine after consumption).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ControlIntent {
    pub actor: ActorId,
    pub source: IntentSource,
    pub move_x: f32,
    pub jump: bool,
    pub aim: Vec2,
    pub fire: bool,
    pub reload: bool,
    pub selected_item: Option<ItemSlot>,
    pub reset: bool,
    /// Edge-triggered interact (door/lever/panel). BP4+ implementation.
    #[serde(default)]
    pub interact: bool,
    /// Edge-triggered use-tool (medkit/repair). BP4+ implementation.
    #[serde(default)]
    pub use_tool: bool,
    /// Toggle crouch stance. M5 added via separate method; unified here.
    #[serde(default)]
    pub crouch: bool,
    /// Toggle prone stance. BP4+ implementation.
    #[serde(default)]
    pub prone: bool,
    /// **M1**: continuous sharp-aim hold (per CCCP `AHuman.cpp:1779`). Sticky;
    /// sim consumes this every tick to advance `sharp_aim_progress`. Edge-
    /// trigger semantics live in the engine (the press/release events convert
    /// to a continuous bool here).
    #[serde(default)]
    pub sharp_aim: bool,
    /// **M1**: sticky "fire is held" flag. Edge-triggered `fire` above is
    /// cleared by `clear_edges` each tick; `fire_held` survives until the
    /// player releases. The sim treats `fire || fire_held` as the trigger
    /// signal, so FullAuto weapons auto-repeat at their fire interval while
    /// held. Semi-mode weapons use `RifleState::semi_latched` to fire exactly
    /// once per held press regardless of `fire_held`.
    #[serde(default)]
    pub fire_held: bool,
}

impl Default for IntentSource {
    fn default() -> Self {
        IntentSource::Human
    }
}

impl ControlIntent {
    pub fn new(actor: ActorId, source: IntentSource) -> Self {
        Self {
            actor,
            source,
            ..Self::default()
        }
    }

    /// Reset edge-triggered fields. Continuous fields (move_x, aim) are preserved.
    pub fn clear_edges(&mut self) {
        self.jump = false;
        self.fire = false;
        self.reload = false;
        self.selected_item = None;
        self.reset = false;
        self.interact = false;
        self.use_tool = false;
        self.crouch = false;
        self.prone = false;
    }

    /// Returns true when no actively-driven input is present. `aim` is a
    /// continuous field that persists across ticks (not cleared by
    /// [`clear_edges`](Self::clear_edges)), so it is intentionally excluded
    /// here — a sticky aim direction does not indicate the player is
    /// currently providing input. `sharp_aim` is also sticky/continuous; it
    /// is not treated as active input pressure for idle detection.
    pub fn is_idle(&self) -> bool {
        self.move_x.abs() < f32::EPSILON
            && !self.jump
            && !self.fire
            && !self.reload
            && self.selected_item.is_none()
            && !self.reset
            && !self.interact
            && !self.use_tool
            && !self.crouch
            && !self.prone
    }
}

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
    /// **M5**: full chassis state (body graph + armor zones + modules + pilot binding).
    /// `None` for legacy M1 / M1.5 actors that haven't been promoted; `Some` for
    /// M5+ chassis-grade actors (infantry / powered_armor / light_mech).
    #[serde(default)]
    pub chassis: Option<cf_chassis::ChassisState>,
    /// **M5**: opaque origin-id tag for DR-014 / M5.8 origin-gated equipment.
    /// Defaults to `"human"`.
    #[serde(default = "default_origin_id")]
    pub origin_id: String,
    /// **M5**: actor-level movement-intent flags surfaced for HUD + animation
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
    /// **M5**: latched flag set on first tick when destroyed-zone movement
    /// contribution returns `drop_gear=true`. Used to gate single-shot
    /// `actor.gear_dropped` event emission + clear the rifle/inventory slot
    /// the next tick. Roadmap §M5 done-criterion: "dropped gear".
    #[serde(default)]
    pub gear_dropped_by_limb_loss: bool,
    /// **M5**: latched flag set when the chassis transitions through
    /// `PilotState::Ejected` (post-eject) so the engine knows to call
    /// `detach_chassis` once. Roadmap §M5 done-criterion: "Pilot eject works:
    /// player ejects from a wrecked mech and continues as foot infantry."
    #[serde(default)]
    pub chassis_detached: bool,
    /// **M13** § "Brain hopping / multi-actor control" — actor flagged as the
    /// player's brain (mission-critical=true; cannot be gibbed instantly).
    /// Brain death = `LossReason::BrainDestroyed`.
    #[serde(default)]
    pub is_brain: bool,
    /// **M13** § "Brain hopping" — tick of the last brain hop into this actor.
    /// 0 when this actor has never been the brain target.
    #[serde(default)]
    pub last_brain_hop_tick: u64,
    /// **M13** § "Brain memory tracks last-known position + last-hop tick" —
    /// world-space position recorded the last time the brain was in this
    /// actor. Persists across brain hops so AI fallback / squad cohesion
    /// can reason about where the brain WAS.
    #[serde(default)]
    pub brain_last_position: [f32; 2],
    /// **M13** § "Drone allies — 4 modes + autonomous behavior" — drone-only
    /// runtime state. `Some(...)` for drone-spec actors; `None` for everything else.
    #[serde(default)]
    pub drone_ally: Option<cf_chassis::DroneAllyState>,
    /// **M14 audit pass 4 (Finding 1)**: target chassis actor id this actor
    /// is currently boarding (boarding is an attribute of the boarder, not
    /// the chassis being boarded). `None` when no boarding is in flight.
    /// Set when `act.player.board` is accepted; cleared when
    /// `boarding_ticks_remaining` decrements to 0 + the pilot transfer
    /// completes (or when the boarding is otherwise cancelled).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_boarding_target: Option<u64>,
    /// **M14 audit pass 4 (Finding 1)**: ticks remaining in the 1500ms
    /// boarding transition. Ticked on the player actor in
    /// `tick_chassis_eject_for_all` so input lock + HUD banner read off
    /// the player, not the target chassis.
    #[serde(default)]
    pub boarding_ticks_remaining: u32,
    /// **M13** § "Hit reactions per body part" — ticks remaining in the
    /// currently-active hit-reaction window. Drives speed reduction +
    /// aim-wobble multipliers per `HitReaction`.
    #[serde(default)]
    pub hit_reaction_ticks_remaining: u32,
    /// **M13** § "Hit reactions per body part" — label of the currently-active
    /// reaction ("stagger_stun" / "limp" / "drop_weapon" / ...).
    #[serde(default)]
    pub hit_reaction_kind: String,
    /// **M13** § "Hit reactions per body part" — speed factor enforced by the
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
    /// **M1**: DYING dwell countdown (CCCP `Actor.cpp:1229`). 0 when not dying.
    /// Decrements each tick after status enters DYING; at zero the status
    /// transitions to DEAD. Default 60 ticks (1000 ms at 60Hz).
    #[serde(default)]
    pub dying_dwell_ticks_remaining: u32,
    /// Default value used when seeding `dying_dwell_ticks_remaining` on the
    /// DYING transition. Configurable per-scenario for tutorial / iron-man modes.
    #[serde(default = "default_dying_dwell_ticks")]
    pub dying_dwell_ticks_default: u32,
    /// **M1**: sharp-aim progress scalar (0..1). Builds when the player holds
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
    /// **M1**: recoil accumulator (CCCP `HDFirearm.cpp:891`). Tracks angular
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
    /// **M1**: mission-critical actor cap. When true, `derived_status` caps
    /// at DYING (does NOT transition to DEAD via dwell). The mission director
    /// (M1.5+) reacts to "critical actor downed" before final loss.
    #[serde(default)]
    pub mission_critical: bool,
    /// **M1**: latched flag set on the tick the actor enters DYING so the
    /// engine fires a one-shot `actor.inventory_dropped` event + clears the
    /// rifle slot. Cleared on `reset()`.
    #[serde(default)]
    pub inventory_dropped_on_dying: bool,
    /// **M1 (Gap C3)**: latched parent event id (`combat.projectile_hit` or
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
    /// **M1**: horizontal-speed threshold (world units / s) below which the
    /// actor counts as "slow enough" for sharp aim to keep building.
    /// Default 1.5 (essentially "barely moving" — see OpenSoldat
    /// `Sprites.pas:4870` for the calibration anchor).
    #[serde(default = "default_walk_threshold")]
    pub walk_threshold: f32,
    /// **M1**: most-recent computed bloom multiplier. Latched here so HUD,
    /// reticle renderer, and AI consumers all read the same value the sim
    /// computed this tick (rather than each duplicating the formula).
    #[serde(default = "default_bloom_factor")]
    pub bloom_factor: f32,
    /// **M5.8 forward-hook (DR-040 ResourceAccumulators)**: 7 resource
    /// accumulators on every actor; `#[serde(default)]` keeps M5 bundles
    /// readable while M5.8 wires actual driver values later. Save-bundle
    /// checksum layout is stable from M5 onward — M5.8 will fill these without
    /// a schema bump.
    #[serde(default)]
    pub resources: ResourceAccumulators,
    /// **M5.7/M5.8 forward-hook (DR-036 AfflictionKind)**: per-actor systemic
    /// state set populated by hazards (M5.7) + origin reactions (M5.8). Empty
    /// at M5 baseline; serde-default preserves backward compat.
    #[serde(default)]
    pub afflictions: Vec<Affliction>,
    /// **M6**: side-view facing direction. Updates on aim; flips sprite.
    #[serde(default)]
    pub facing: FacingDirection,
    /// **M6**: stamina pool driving Sprint stance.
    #[serde(default)]
    pub stamina: Stamina,
    /// **M6**: lean angle + direction (for "lean around corner").
    #[serde(default)]
    pub lean_state: LeanState,
    /// **M6**: current cover state (side + effectiveness).
    #[serde(default)]
    pub cover_state: CoverState,
    /// **M6**: sticky sprint intent (toggled by act.player.sprint).
    #[serde(default)]
    pub sprint_active: bool,
    /// **M6**: sticky prone intent (toggled by act.player.prone).
    #[serde(default)]
    pub prone_active: bool,
    /// **M6**: animation-bound stance ticks remaining (slide / vault / dive / climb).
    /// 0 when no animation is playing.
    #[serde(default)]
    pub cinematic_ticks_remaining: u32,
    /// **M6**: which cinematic stance is currently playing.
    #[serde(default)]
    pub cinematic_kind: Option<Stance>,
    /// **M6**: latched stealth-meter value (0..1).
    #[serde(default)]
    pub stealth_meter: f32,
    /// **M6 (M13 forward-compat)**: limb-loss tracking + action restriction surface.
    /// Per spec § "Limb-loss action restrictions": M6 reserves the surface;
    /// M13 chassis fills physics-driven limb damage.
    #[serde(default)]
    pub limb_loss: LimbLossFlags,
    /// **M6**: total inventory weight (kg). Recomputed each tick; > 30 kg
    /// forces Walk per spec § "Weight system".
    #[serde(default)]
    pub inventory_weight_kg: f32,
    /// **M6**: bipod attachment + deployment state on the equipped weapon.
    /// When deployed, the firing path multiplies recoil by
    /// [`cf_equipment::BIPOD_RECOIL_FACTOR`] and bloom by
    /// [`cf_equipment::BIPOD_BLOOM_FACTOR`].
    #[serde(default = "default_bipod_equipped")]
    pub bipod: cf_equipment::Bipod,
    /// **M6**: suppressor attachment state on the equipped weapon. When
    /// attached, the firing path multiplies loudness by
    /// [`cf_equipment::SUPPRESSOR_LOUDNESS_FACTOR`].
    #[serde(default)]
    pub suppressor: cf_equipment::Suppressor,
    /// **M6**: per-tool durability map (tool kind → Durability). Filled
    /// lazily on first `act.player.use_tool` so existing scenarios serialize
    /// cleanly.
    #[serde(default)]
    pub tool_durability: std::collections::BTreeMap<String, cf_equipment::Durability>,
    /// **M6**: true while a [`cf_equipment::WeaponSwap`] is in flight for
    /// this actor. The firing path in `sim::fire_actor` gates on this so
    /// fire/reload intent is rejected during the swap window.
    #[serde(default)]
    pub weapon_swap_in_progress: bool,
    /// **M6**: cook-time accumulator (seconds) for the currently-held
    /// grenade. Reset on throw or grenade swap.
    #[serde(default)]
    pub grenade_cook_seconds: f32,
    /// **M6**: which grenade kind is currently equipped (held). `None`
    /// when no grenade slot is selected. Defaults to Frag at spawn so the
    /// spec's "Cook grenade for shorter fuse" Gherkin reproduces.
    #[serde(default = "default_grenade_kind")]
    pub grenade_held_kind: Option<cf_equipment::GrenadeKind>,
    /// **M6**: remaining fuse on the held grenade after cooking. Initial
    /// value is the grenade's base fuse; cook_grenade subtracts elapsed
    /// cook time.
    #[serde(default)]
    pub grenade_held_fuse_remaining: f32,
    /// **M6**: drill heat accumulator (0..1). Above
    /// `cf_equipment::DRILL_JAM_HEAT_THRESHOLD` the drill jams + emits
    /// `equipment.drill_overheated`. Decays at
    /// `cf_equipment::DRILL_HEAT_DECAY_PER_S` per second when idle.
    #[serde(default)]
    pub drill_heat: f32,
    /// **M6**: tick at which sensor-pulse reveal expires for this actor.
    /// 0 when not revealed. Set by `act.player.use_tool { kind:
    /// "sensor_pulse" }` for hostile actors within the reveal radius.
    #[serde(default)]
    pub reveal_until_tick: u64,
    /// **M6**: cached [`AdvancedFireMode`] for the actor's currently
    /// selected weapon. `act.player.cycle_fire_mode` rotates this through
    /// the weapon's `FireModeSet::available` list; the firing path consults
    /// it to gate Burst3 / Charge / Arc semantics. Default `Single` matches
    /// the cold-start weapon state.
    #[serde(default)]
    pub weapon_fire_mode: cf_equipment::AdvancedFireMode,
    /// **M6**: charge-mode accumulator scalar in `0..=1`. Ticks up while
    /// the trigger is held under `AdvancedFireMode::Charge`; clamps to 1.0
    /// at full charge (per [`cf_equipment::SNIPER_CHARGE_MAX_SECONDS`]).
    /// Reset to 0 on trigger release.
    #[serde(default)]
    pub weapon_charge_fraction: f32,
    /// **M6**: queued follow-up shots for `AdvancedFireMode::Burst3`. Seeded
    /// to `BURST3_ROUND_COUNT - 1` when the first round of a 3-round burst
    /// fires through the M1 path; decremented as each follow-up round
    /// fires from the M6 tick scheduler.
    #[serde(default)]
    pub burst3_remaining_shots: u32,
    /// **M6**: seconds remaining until the next burst-3 follow-up shot
    /// fires. Reset to [`cf_equipment::BURST3_INTER_SHOT_SECONDS`] each
    /// time a follow-up round leaves the muzzle so the 3-round burst
    /// completes within 100 ms per spec § "SMG burst-3 fire mode".
    #[serde(default)]
    pub burst3_next_fire_at_seconds: f32,
    /// **M6**: trigger-state edge tracker for Charge-mode release detection.
    /// `true` the tick after the player begins holding fire; transitions
    /// to `false` on release, at which point the engine fires one
    /// charge-scaled shot.
    #[serde(default)]
    pub fire_held_prev: bool,
    /// **M6B**: per-actor inventory grid (Tetris-style placement +
    /// container nesting + liquid-mass tracking). `None` for legacy
    /// actors that still drive purely off `inventory` /
    /// `inventory_weight_kg`; `Some(...)` for M6B+ actors so the M14A
    /// mass aggregator can read a single canonical inventory surface.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory_grid: Option<crate::inventory::InventoryGrid>,
    /// **M6B**: per-actor inventory encumbrance envelope (carry cap +
    /// derived walk-speed multiplier + band). Populated lazily on
    /// `inventory_grid_attach` or at construction time via
    /// `seed_inventory_encumbrance_for_origin`. `None` for legacy actors;
    /// `Some(...)` for M6B+ actors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory_encumbrance: Option<crate::inventory::InventoryEncumbrance>,
    /// **M6C**: per-actor body armor slot (helmet + body + gloves +
    /// boots + knee/elbow pads). Separate from the M13 chassis armor.
    /// Default = empty slots so legacy actors serialize cleanly.
    #[serde(default)]
    pub body_armor: BodyArmorSlot,
    /// **M9C**: bound fortification id when this actor is in
    /// `Stance::Crewing`. Stored externally so the `Stance` enum stays
    /// a flat `#[repr(u8)]` while expressing the spec-shape
    /// `Stance::Crewing { fortification_id }`. `None` when not crewing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crewing_fortification_id: Option<u32>,
    /// **M9C**: per-actor wire-crossing state. Per spec § "Notes for
    /// the implementer": *don't store wire crossing state on the wire
    /// (one wire, many actors); store it on the actor's `crossing:
    /// Option<WireId>`. Avoid the O(n×m) interaction-matrix trap.*
    /// `Some(wire_id)` while the actor is currently crossing /
    /// snagged on a wire; `None` otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crossing: Option<cf_fortification::WireId>,
    /// **M14A** § "MoveState" — current locomotion state (CCCP analog).
    #[serde(default)]
    pub move_state: crate::move_state::MoveState,
    /// **M14A** § "ProneState".
    #[serde(default)]
    pub prone_state: crate::move_state::ProneState,
    /// **M14A** § "UpperBodyState".
    #[serde(default)]
    pub upper_body_state: crate::move_state::UpperBodyState,
    /// **M14A** § "Rotational balancing spring".
    #[serde(default)]
    pub attitude: crate::attitude::AttitudeState,
    /// **M14A** § "Per-leg WalkAngle slope adapter".
    #[serde(default)]
    pub walk_angle: crate::attitude::WalkAngleState,
    /// **M14A** § "UpdateCrouching — crouch lean + WalkPathOffset".
    #[serde(default)]
    pub walk_path_offset: crate::attitude::WalkPathOffset,
    /// **M14A** § "Arm sway state".
    #[serde(default)]
    pub arm_sway: crate::arm_sway::ArmSwayState,
    /// **M14A** § "Stride alternation algorithm" — true on the tick a foot plants.
    #[serde(default)]
    pub stride_frame: bool,
    /// **M14A** § "Stride alternation algorithm" — `true` at start of stride.
    #[serde(default)]
    pub stride_start: bool,
    /// **M14A** § "stride_timer_ms" — wall-clock-stable per-tick counter.
    #[serde(default)]
    pub stride_timer_ms: u32,
    /// **M14A** § "Stride alternation" — last side that plant'd (true = fg).
    #[serde(default)]
    pub last_stride_side_fg: bool,
    /// **M14A** § "Per-actor limb-path registry" — RON-loadable, defaults to
    /// `default_infantry_registry()`. Owned on actor for serde round-trip.
    #[serde(default = "crate::limb_path::default_infantry_registry")]
    pub limb_paths: crate::limb_path::LimbPathRegistry,
    /// **M14A** § "Jetpack physics" — equipped jetpack instance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jetpack: Option<cf_equipment::Jetpack>,
    /// **M14A** § "Quick Action UX" — 8-slot bar + radial state.
    #[serde(default)]
    pub quick_action_bar: crate::quick_action::QuickActionBarState,
    /// **M14A** § "Mass aggregation — every kg matters" — cached total mass.
    #[serde(default)]
    pub total_mass_cached: f32,
    /// **M14A** § "Mass × everything matrix" — true when the cached mass is
    /// stale and `total_mass()` must recompute.
    #[serde(default = "default_mass_dirty")]
    pub total_mass_dirty: bool,
    /// **M14A** § "Wound mass from lodged pixels" — accumulated lodged-pixel mass.
    #[serde(default)]
    pub wound_mass_kg: f32,
    /// **M14A** § "actor.on_stride" — last tick a stride event fired
    /// (consumers like cf-perception read this to emit hearing signals).
    #[serde(default)]
    pub last_stride_tick: u64,
    /// **M14A** § "Hold-Q radial flow" — tick a Q-press began on; used to
    /// disambiguate tap-Q from hold-Q.
    #[serde(default)]
    pub q_press_tick: u64,
    /// **M14A** § "Hold-Q radial flow" — `true` while Q is held.
    #[serde(default)]
    pub q_held: bool,
    /// **M14A** § "Quick action UX last-used slot".
    #[serde(default)]
    pub last_used_quick_slot: u8,
    /// **M14A** § "armor scratch" — per-zone advance (0..3 decal level).
    /// Level 1 = 0-30% External lost; 2 = 30-60%; 3 = 60-100%.
    #[serde(default)]
    pub armor_scratch_level: std::collections::BTreeMap<String, u8>,
    /// **M14A** § "Atmospheric overlay" — last sampled atmosphere at actor pos.
    #[serde(default)]
    pub atmosphere_sample: AtmosphereSample,
    /// **M14A** § "Per-stride hazard.actor_contact debouncing" — tick of last
    /// per-zone hazard contact (zone_name → tick).
    #[serde(default)]
    pub last_hazard_contact_tick: std::collections::BTreeMap<String, u64>,
}

/// **M14A** § "Atmospheric overlay" — re-export of [`cf_atmos::AtmosphereSample`]
/// so callers don't need to depend on cf-atmos directly.
pub use cf_atmos::AtmosphereSample;

fn default_mass_dirty() -> bool {
    true
}

fn default_bipod_equipped() -> cf_equipment::Bipod {
    cf_equipment::Bipod::equipped_default()
}

#[allow(clippy::unnecessary_wraps)]
fn default_grenade_kind() -> Option<cf_equipment::GrenadeKind> {
    Some(cf_equipment::GrenadeKind::Frag)
}

/// **M5.8 forward-hook (DR-040 ResourceAccumulators)**: per-actor resource
/// values driven by origin reaction matrix at M5.8. Reserved layout slot at
/// M5 so save bundles + observe frames serialize the slot now and M5.8 can
/// fill values without a checksum byte-layout shift.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ResourceAccumulators {
    pub caloric_energy: f32,
    pub battery_charge: f32,
    pub power: f32,
    pub heat: f32,
    pub oxygen_supply: f32,
    pub g_load_dose: f32,
    pub concussion_dose: f32,
}

/// **M5.7/M5.8 forward-hook (DR-036 affliction layer)**: per-actor systemic
/// state. Spec-locked enum prevents typos across BP4 milestones. Carried as
/// `Vec<Affliction>` on `ActorState` so multiple afflictions can stack.
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AfflictionKind {
    Wetness,
    Burning,
    Corroded,
    Electrified,
    Poisoned,
    Asphyxiating,
    Suffocating,
    Drowning,
    Depressurizing,
    InternalShock,
    CoolantLeaking,
    OilLeaking,
    Overheating,
    LowBattery,
    PowerStarved,
    Weak,
    Exhausted,
    Hypoxia,
    Downclocked,
    HeatExhaustion,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Affliction {
    pub kind: AfflictionKind,
    #[serde(default)]
    pub intensity: f32,
    #[serde(default)]
    pub expires_tick: Option<u64>,
}

fn default_origin_id() -> String {
    "human".to_string()
}

fn default_stability() -> f32 {
    1.0
}

fn default_stability_recovery_rate() -> f32 {
    0.02
}

fn default_speed_factor() -> f32 {
    1.0
}

fn default_mass_kg() -> f32 {
    80.0 // human infantry default
}

fn default_dying_dwell_ticks() -> u32 {
    60 // CCCP Actor.cpp:1229 — 1000 ms at 60 Hz
}

fn default_sharp_aim_build_ticks() -> u32 {
    30 // ~0.5 s at 60 Hz to fully build sharp aim
}

fn default_recoil_decay_rate() -> f32 {
    0.05
}

fn default_walk_threshold() -> f32 {
    1.5
}

fn default_bloom_factor() -> f32 {
    1.0
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
        }
    }

    /// **M5**: detach the chassis from this actor and return it as a Wreck so
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
            // **M14A** § "Heavy Trooper" — 380 kg base loaded.
            cf_chassis::ChassisKind::HeavyTrooper => (Vec2::new(11.0, 22.0), 380.0),
        };
        self.half_extents = half_extents;
        self.mass_kg = mass;
        // **M13** § "Drone allies — 4 modes" — auto-seed `DroneAllyState` when
        // the attached chassis is a drone so cfctl + AI consumers see the
        // mode/fuel surface immediately.
        if chassis.kind == cf_chassis::ChassisKind::Drone && self.drone_ally.is_none() {
            self.drone_ally = Some(cf_chassis::DroneAllyState::default());
        }
        self.chassis = Some(chassis);
    }

    /// **M6B**: attach a default `InventoryGrid` and per-origin
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

    /// **M6B**: borrow the per-actor inventory grid (read-only).
    pub fn inventory_grid(&self) -> Option<&crate::inventory::InventoryGrid> {
        self.inventory_grid.as_ref()
    }

    /// **M6B**: borrow the per-actor inventory grid (mutable).
    pub fn inventory_grid_mut(&mut self) -> Option<&mut crate::inventory::InventoryGrid> {
        self.inventory_grid.as_mut()
    }

    /// **M6B**: total mass of every placed item in the inventory grid
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

    /// **M6B**: total bulk volume of every placed item in the
    /// inventory grid (recursive). Returns `0.0` when no grid is
    /// attached.
    pub fn inventory_grid_total_bulk_l(&self) -> f32 {
        self.inventory_grid
            .as_ref()
            .map_or(0.0, inventory::InventoryGrid::total_bulk_l)
    }

    /// **M6B**: refresh the `InventoryEncumbrance` envelope from the
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

    /// **M6B**: per-actor `max_carry_kg` baseline (50 × origin modifier).
    /// Falls back to the human baseline when no envelope is attached.
    pub fn max_carry_kg(&self) -> f32 {
        self.inventory_encumbrance
            .map_or(cf_equipment::HUMAN_BASELINE_MAX_CARRY_KG, |e| e.max_carry_kg)
    }

    /// **M6B**: per-actor `max_carry_volume_l` baseline (60 × origin modifier).
    pub fn max_carry_volume_l(&self) -> f32 {
        self.inventory_encumbrance
            .map_or(cf_equipment::HUMAN_BASELINE_MAX_CARRY_VOLUME_L, |e| {
                e.max_carry_volume_l
            })
    }

    /// **M6B**: derived walk-speed multiplier from the encumbrance
    /// envelope (`1.0` empty, `0.5` at 100%). Returns `1.0` when no
    /// envelope is attached.
    pub fn encumbrance_walk_speed_multiplier(&self) -> f32 {
        self.inventory_encumbrance.map_or(1.0, |e| e.walk_speed_multiplier)
    }

    /// **M6B**: derived encumbrance band from the envelope
    /// (`None` / `Light` / `Moderate` / `Heavy`).
    pub fn encumbrance_band(&self) -> cf_equipment::EncumbranceBand {
        self.inventory_encumbrance
            .map_or(cf_equipment::EncumbranceBand::None, |e| e.band)
    }

    /// **M6B**: true when the actor is at or past 100% load (HUD shows
    /// "ENCUMBERED" warning per spec § "Encumbrance at 100% reduces
    /// walk speed").
    pub fn is_encumbered(&self) -> bool {
        self.inventory_encumbrance.is_some_and(|e| e.encumbered())
    }

    /// **M13** § "Hit reactions per body part" — apply a hit-reaction window
    /// to this actor based on the zone struck. Replaces any in-flight
    /// reaction. `tick_rate_hz` is used to derive the duration in ticks.
    pub fn apply_hit_reaction(&mut self, zone: cf_chassis::BodyZone, tick_rate_hz: u32) -> cf_chassis::HitReaction {
        let reaction = cf_chassis::HitReaction::for_zone(zone);
        self.hit_reaction_ticks_remaining = reaction.duration_ticks(tick_rate_hz);
        self.hit_reaction_kind = reaction.kind.to_string();
        self.hit_reaction_speed_factor = reaction.speed_factor;
        reaction
    }

    /// **M13** § "Hit reactions per body part" — advance the reaction timer
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

    /// **M14A** § "Mass aggregation system" — recompute the cached
    /// `total_mass_cached` if dirty, then return it.
    pub fn total_mass_kg(&mut self) -> f32 {
        if self.total_mass_dirty {
            self.total_mass_cached = crate::mass_aggregator::total_mass(self);
            self.total_mass_dirty = false;
        }
        self.total_mass_cached
    }

    /// **M14A** § "Mass × everything matrix" — current walk-speed mass factor.
    pub fn walk_speed_mass_factor(&mut self) -> f32 {
        let _ = self.total_mass_kg();
        crate::mass_aggregator::mass_factor(self)
    }

    /// **M14A** § "Live recalculation hooks" — invalidate the cached total mass.
    pub fn mark_mass_dirty(&mut self) {
        self.total_mass_dirty = true;
    }

    /// **M14A** § "Mass aggregation system" — equip a jetpack on this actor.
    /// Marks mass dirty so the next `total_mass_kg()` call recomputes.
    pub fn equip_jetpack(&mut self, jetpack: cf_equipment::Jetpack) {
        self.jetpack = Some(jetpack);
        self.mark_mass_dirty();
    }

    /// **M14A** § "Backpack severance → jetpack failure" — drop the jetpack.
    pub fn drop_jetpack(&mut self) -> Option<cf_equipment::Jetpack> {
        self.mark_mass_dirty();
        self.jetpack.take()
    }

    /// **M14A** § "Quick Action UX" — reseat the QAB to the per-chassis
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

    /// **M14A** § "actor.on_stride" — note a stride event on this tick.
    /// Sets `stride_frame=true` (consumed end-of-tick), advances
    /// `last_stride_side_fg`, records `last_stride_tick`.
    pub fn emit_stride(&mut self, tick: u64, side_fg: bool) {
        self.stride_frame = true;
        self.last_stride_side_fg = side_fg;
        self.last_stride_tick = tick;
        self.stride_start = false;
        self.stride_timer_ms = 0;
    }

    /// **M14A** § "Apply hazard overlay disabled slots" — gate the QAB by
    /// active hazard zones (EM disruption disables slots 6/7/8 by default).
    pub fn apply_em_disruption(&mut self, em_disrupted: bool) {
        if em_disrupted {
            self.quick_action_bar.apply_hazard_disabled_slots(&[5, 6, 7]);
        } else {
            self.quick_action_bar.apply_hazard_disabled_slots(&[]);
        }
    }

    /// **M14A** § "Heavy Armor — visible armor scratches" — advance the
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

    /// **M14A** § "Knockdown threshold" — check whether incoming impulse
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

    /// **M14A** § "Per-stride material contact resolver" — apply per-stride
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

    /// **M13** § "Brain hopping" — mark this actor as the player's brain.
    pub fn mark_brain(&mut self, tick: u64) {
        self.is_brain = true;
        self.mission_critical = true;
        self.last_brain_hop_tick = tick;
        self.brain_last_position = [self.position.x, self.position.y];
    }

    /// **M13** § "Brain hopping" — clear the brain flag (called when the
    /// player hops out of this actor). Records the last known position so
    /// the AI fallback can reason about where the brain WAS.
    ///
    /// **M14 audit pass 4 (Finding 7)**: also clear `mission_critical`.
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
    /// **M5**: chassis state (zones / modules / pilot binding) also resets.
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
        // **M6B**: reset the inventory grid + encumbrance envelope so a
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
    /// **M5**: when the actor has a chassis attached, damage is routed through
    /// the chassis pipeline (armor layers → wound → actor HP) via
    /// [`ActorState::apply_zone_damage`]. The legacy direct-HP path is preserved
    /// for actors without a chassis.
    ///
    /// **M1**: `Dying` and `Dead` reject further damage; `Inactive` also rejects
    /// (cutscene safety).
    ///
    /// **M1 audit pass 6 (2026-05-13)**: mission-critical actors now cap at
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

    /// **M1**: set INACTIVE state explicitly (tutorial / cutscene). Pure setter;
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
        // **M5**: pilot lifecycle vs damage routing.
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

    /// Derived stance for HUD + `cfctl observe`. M5 routes through
    /// [`Stance::from_chassis`] so crouch / climb / jet / eject signals propagate.
    pub fn stance(&self) -> Stance {
        if self.knockdown_ticks_remaining > 0 {
            return Stance::KnockedDown;
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

    /// **M5**: real chassis module strip when a chassis is attached. Returns
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
                    // **M13** § "Critical chassis modules with full mechanics" — short HUD labels.
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

    /// **M5**: chassis projection (per-zone integrity + stage + pilot state + modules).
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
    /// **M5**: chassis state (zones / modules / pilot state) is appended after the
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
        // **M4 § Checksum scope sim_state_v1** — element #3 spec literal
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

/// The set of [`ActorState`]s in a scenario, plus the player actor id (if any).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ActorWorld {
    pub actors: BTreeMap<ActorId, ActorState>,
    pub player: Option<ActorId>,
    /// Y coordinate of the world floor (sand-table simplification for M1 — the M2 chunked
    /// terrain replaces this).
    pub floor_y: f32,
    /// Gravity in world units / s² (negative = pulls toward floor). Defaults to scenario's
    /// `gravity`. Sim systems apply this each tick.
    pub gravity: f32,
    /// **M1.5 G8**: when true, lethal damage to controllable actors caps
    /// at `Status::Dying` — the DYING dwell does NOT promote to DEAD so a
    /// tutorial player can finish their first session without restarting.
    /// Per DR-023 onboarding policy; sourced from the scenario manifest's
    /// `tutorial_safety` flag.
    #[serde(default)]
    pub tutorial_safety: bool,
}

impl ActorWorld {
    pub fn new(floor_y: f32, gravity: f32) -> Self {
        Self {
            actors: BTreeMap::new(),
            player: None,
            floor_y,
            gravity,
            tutorial_safety: false,
        }
    }

    pub fn insert(&mut self, actor: ActorState) {
        if actor.controllable && self.player.is_none() {
            self.player = Some(actor.id);
        }
        self.actors.insert(actor.id, actor);
    }

    pub fn player_actor(&self) -> Option<&ActorState> {
        self.player.and_then(|id| self.actors.get(&id))
    }

    pub fn player_actor_mut(&mut self) -> Option<&mut ActorState> {
        let id = self.player?;
        self.actors.get_mut(&id)
    }

    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.actors.len() * 96 + 16);
        out.extend_from_slice(&quantize_f32(self.floor_y).to_le_bytes());
        out.extend_from_slice(&quantize_f32(self.gravity).to_le_bytes());
        for (_, actor) in &self.actors {
            out.extend_from_slice(&actor.checksum_bytes());
        }
        out
    }
}

/// M4A body silhouette projection. Per-zone hp percentages clamped to `[0, 1]`.
/// `placeholder = true` until M5 lands the real body graph; HUD + AI consumers
/// must treat the layout as stable but the per-zone values as derived (not
/// individually targetable yet).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodySilhouette {
    pub head_hp_pct: f32,
    pub torso_hp_pct: f32,
    pub arm_left_hp_pct: f32,
    pub arm_right_hp_pct: f32,
    pub leg_left_hp_pct: f32,
    pub leg_right_hp_pct: f32,
    pub placeholder: bool,
}

impl Default for BodySilhouette {
    fn default() -> Self {
        Self {
            head_hp_pct: 1.0,
            torso_hp_pct: 1.0,
            arm_left_hp_pct: 1.0,
            arm_right_hp_pct: 1.0,
            leg_left_hp_pct: 1.0,
            leg_right_hp_pct: 1.0,
            placeholder: true,
        }
    }
}

/// M4A module strip placeholder. M5's chassis grammar replaces this with real
/// per-module state (see [[spec/chassis-armor-mechs-and-origins]]); M4A ships
/// the surface so HUD + `cfctl observe` consumers + accessibility tooling can
/// rely on the contract early.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleStrip {
    /// Module slots, each with a stable id + textual state. Empty when no
    /// chassis is bound. M4A populates `weapon_mount` from the selected
    /// rifle's status (READY / RELOADING / EMPTY / NO RIFLE) and stubs
    /// `jet`, `shield`, and `sensor` as `not_present` so consumers can
    /// distinguish "no module" from "module destroyed".
    pub modules: Vec<ModuleState>,
    pub placeholder: bool,
}

impl Default for ModuleStrip {
    fn default() -> Self {
        Self {
            modules: Vec::new(),
            placeholder: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleState {
    pub id: String,
    pub label: String,
    /// One of: `nominal`, `degraded`, `warning`, `failed`, `not_present`.
    pub state: String,
    /// One of: `weapon_mount`, `jet`, `shield`, `sensor`, `repair_drone`.
    pub kind: String,
}

/// Public projection of an actor for the cf-control observe envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorObservation {
    pub id: u64,
    pub team: String,
    pub controllable: bool,
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub aim: [f32; 2],
    pub on_ground: bool,
    pub status: String,
    pub hp: f32,
    pub hp_max: f32,
    pub selected_slot: u32,
    pub selected_item: String,
    /// **M1 re-audit (2026-05-13)**: full inventory contents as an array of
    /// item labels in slot order (length 4 for M1; matches `Inventory.items`).
    /// Closes the M1 spec drift item — spec said the observation includes
    /// `inventory[]` but code only surfaced `selected_slot + selected_item`.
    #[serde(default)]
    pub inventory: Vec<String>,
    /// M4A: derived stance label (idle/walking/running/airborne/downed/dead/crouching/...).
    pub stance: String,
    /// M4A: per-zone body silhouette projection. `placeholder=false` when sourced
    /// from a real M5 chassis body graph.
    pub body_silhouette: BodySilhouette,
    /// **M5**: full chassis projection when the actor has a chassis attached.
    #[serde(default)]
    pub chassis: Option<ChassisView>,
    /// **M5**: actor origin tag (`human`, `robot`, `android`, ...).
    #[serde(default = "default_origin_id")]
    pub origin_id: String,
    /// W1.3: stability scalar (0.0 = fully disrupted, 1.0 = stable).
    #[serde(default = "default_stability")]
    pub stability: f32,
    /// W1.3: stability recovery rate per tick when grounded.
    #[serde(default = "default_stability_recovery_rate")]
    pub stability_recovery_rate: f32,
    /// Physical mass in kg. Affects movement feel, stability resistance, knockdown.
    #[serde(default = "default_mass_kg")]
    pub mass_kg: f32,
    /// **M5**: per-tick movement-intent mirror.
    #[serde(default)]
    pub crouch_active: bool,
    #[serde(default)]
    pub climb_active: bool,
    #[serde(default)]
    pub jet_active: bool,
    /// **M1**: sharp-aim progress scalar (0..1) — CCCP `AHuman.cpp:1779`.
    #[serde(default)]
    pub sharp_aim_progress: f32,
    /// **M1**: most-recent recoil accumulator value (CCCP `HDFirearm.cpp:891`).
    #[serde(default)]
    pub recoil_accumulator: f32,
    /// **M1**: knockdown recovery ticks remaining. Zero when not in knockdown.
    #[serde(default)]
    pub knockdown_ticks_remaining: u32,
    /// **M1**: DYING dwell countdown ticks remaining (CCCP `Actor.cpp:1229`).
    #[serde(default)]
    pub dying_dwell_ticks_remaining: u32,
    /// **M1**: mission-critical flag (caps damage at DOWNED).
    #[serde(default)]
    pub mission_critical: bool,
    /// **M1**: most-recent computed reticle bloom multiplier (Soldat `Sprites.pas:4870`).
    /// 1.0 = standing/walking; >1 = movement / airborne / sharp-aim breakup.
    #[serde(default = "default_bloom_factor")]
    pub bloom_factor: f32,
    /// **M6**: side-view facing direction.
    #[serde(default)]
    pub facing: String,
    /// **M6**: stamina pool current value (0..1).
    #[serde(default)]
    pub stamina: f32,
    /// **M6**: stamina pool capacity (>=1.0 if extended).
    #[serde(default)]
    pub stamina_max: f32,
    /// **M6**: sticky sprint intent.
    #[serde(default)]
    pub sprint_active: bool,
    /// **M6**: sticky prone intent.
    #[serde(default)]
    pub prone_active: bool,
    /// **M6**: current lean angle (degrees; negative=left, positive=right).
    #[serde(default)]
    pub lean_angle_degrees: f32,
    /// **M6**: current lean direction (none/left/right).
    #[serde(default)]
    pub lean_direction: String,
    /// **M6**: latched stealth meter (0..1).
    #[serde(default)]
    pub stealth_meter: f32,
    /// **M6**: HUD spotted-caption flag.
    #[serde(default)]
    pub spotted: bool,
    /// **M6**: cover side (none/left/right/both).
    #[serde(default)]
    pub cover_side: String,
    /// **M6**: cover effectiveness 0..1.
    #[serde(default)]
    pub cover_effectiveness: f32,
    /// **M6**: total inventory weight (kg).
    #[serde(default)]
    pub inventory_weight_kg: f32,
    /// **M6**: true when weight forces walking (>30kg).
    #[serde(default)]
    pub weight_forces_walk: bool,
    /// **M6 (M13 forward-compat)**: per-limb loss flags surfaced for the HUD
    /// + action-rejection contract.
    #[serde(default)]
    pub limb_loss: LimbLossFlags,
    /// **M6**: extended inventory slots (8 active + 3 reserved tank).
    /// Each entry includes kind + state ("empty" / "occupied" / "locked")
    /// + the locked tooltip on the reserved slots.
    #[serde(default)]
    pub inventory_extended: Vec<ExtendedInventorySlotView>,
    /// **M6**: weapon-state projection (mag_remaining, fire_mode, bipod_state,
    /// suppressor_attached, reload_state, charge_fraction). See
    /// [`WeaponStateView`] for the shape contract.
    #[serde(default)]
    pub weapon_state: WeaponStateView,
    /// **M13** § "Brain hopping / multi-actor control" — true when the
    /// player's brain currently resides in this actor.
    #[serde(default)]
    pub is_brain: bool,
    /// **M13** § "Hit reactions per body part" — currently-active hit
    /// reaction label + ticks remaining + speed factor. Empty label when no
    /// reaction is active.
    #[serde(default)]
    pub hit_reaction_kind: String,
    #[serde(default)]
    pub hit_reaction_ticks_remaining: u32,
    /// **M13** § "Drone allies" — drone mode + fuel (only populated when
    /// the actor is a drone).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drone_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drone_fuel: Option<f32>,
    /// **M6B**: per-actor `max_carry_kg` (50 × origin modifier).
    #[serde(default)]
    pub max_carry_kg: f32,
    /// **M6B**: per-actor `max_carry_volume_l` (60 × origin modifier).
    #[serde(default)]
    pub max_carry_volume_l: f32,
    /// **M6B**: total carried mass in kg (sum of inventory grid).
    /// Distinct from the legacy `inventory_weight_kg` (M6 slot sum) so
    /// the M14A mass aggregator can consume a per-item canonical surface.
    #[serde(default)]
    pub total_carried_kg: f32,
    /// **M6B**: total carried bulk in liters.
    #[serde(default)]
    pub total_carried_volume_l: f32,
    /// **M6B**: walk-speed multiplier from the encumbrance curve
    /// (`1.0` empty, `0.5` at 100% carry).
    #[serde(default = "default_bloom_factor")]
    pub encumbrance_walk_speed_multiplier: f32,
    /// **M6B**: discrete encumbrance band (none/light/moderate/heavy).
    #[serde(default)]
    pub encumbrance_band: String,
    /// **M6B**: true when the actor is at or past 100% load (HUD shows
    /// "ENCUMBERED" warning per spec § "Encumbrance at 100% reduces
    /// walk speed").
    #[serde(default)]
    pub encumbered: bool,
    /// **M6B**: per-actor inventory grid (Tetris placements + per-item
    /// mass + bulk + nested container counts). `None` for pre-M6B
    /// legacy actors; `Some(...)` for any actor with an attached
    /// `inventory_grid`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory_grid: Option<InventoryGridView>,
}

/// **M6**: extended-inventory slot projection. Mirrors
/// `cf_equipment::inventory::ExtendedSlot` but lives here so observe.actor
/// stays a pure cf-actor projection.
///
/// **M6B**: extended with `bulk_volume_l` so `observe.actor.inventory`
/// surfaces both per-slot mass + bulk per spec § Crates / modules touched
/// (cf-control MODIFY — observe.actor.inventory extended with mass + bulk).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ExtendedInventorySlotView {
    pub kind: String,
    pub state: String,
    #[serde(default)]
    pub item_id: String,
    #[serde(default)]
    pub weight_kg: f32,
    /// **M6B**: per-slot bulk volume in liters (from
    /// `cf_equipment::ItemSpec.bulk_volume_l`). Zero for empty slots.
    #[serde(default)]
    pub bulk_volume_l: f32,
    #[serde(default)]
    pub locked_tooltip: Option<String>,
}

/// **M6B**: per-placement projection of the actor's inventory grid for
/// `observe.actor`. Surfaces the canonical mass + bulk per item so the
/// HUD + M27 Tetris UX + M14A mass aggregator all see one source of
/// truth without each having to recompute from the spec registry.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InventoryGridPlacementView {
    pub instance_id: u64,
    pub item_id: String,
    pub category: String,
    pub origin: [u8; 2],
    pub dimensions: [u8; 2],
    pub rotated: bool,
    pub stack_count: u16,
    pub mass_kg: f32,
    pub bulk_volume_l: f32,
    pub is_container: bool,
    pub nested_count: u16,
    pub current_liquid_l: f32,
    pub liquid_capacity_l: f32,
    pub quick_slot_eligible: bool,
}

/// **M6B**: full inventory-grid projection surfaced via
/// `observe.actor.inventory_grid`. Mirrors
/// `cf_actor::InventoryGrid` with derived totals so cfctl consumers see
/// the canonical M6B surface without consulting the engine binary.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InventoryGridView {
    pub tier: String,
    pub grid_w: u8,
    pub grid_h: u8,
    pub placements: Vec<InventoryGridPlacementView>,
    pub total_mass_kg: f32,
    pub total_bulk_l: f32,
}

/// **M6**: high-level reload-state projection for [`WeaponStateView`].
///
/// `Idle` covers both "ready to fire" and "between shots / pump-action
/// chamber" — anything that is not actively in a multi-tick reload animation.
/// `Reloading` covers the multi-tick reload window driven by the M1
/// `cf_equipment::RifleState::reload_remaining_ticks` counter (plus any
/// future weapon-specific reload state machines).
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReloadState {
    #[default]
    Idle = 0,
    Reloading = 1,
}

impl ReloadState {
    pub fn as_str(self) -> &'static str {
        match self {
            ReloadState::Idle => "idle",
            ReloadState::Reloading => "reloading",
        }
    }
}

/// **M6**: per-actor weapon-state observation projection (the spec § "Crates
/// / modules touched / cf-actor" bullet "ActorObservation extensions
/// (cover_state, stamina, lean_angle, weapon_state)" weapon_state field).
///
/// Six fields are surfaced, all live:
///
/// - `mag_remaining` — current rounds in the chambered magazine, read from
///   the live [`cf_equipment::RifleState::ammo_in_mag`] when the engine
///   passes a rifle handle; falls back to the rifle preset's `mag_capacity`
///   when no rifle state is available (e.g. test paths).
/// - `fire_mode` — extended fire-mode discriminator
///   ([`cf_equipment::AdvancedFireMode`]) reflecting the live
///   `ActorState::weapon_fire_mode`. Rotated by
///   `act.player.cycle_fire_mode`.
/// - `bipod_state` — current [`cf_equipment::BipodState`] (`Stowed` /
///   `Deployed`) read from `ActorState::bipod.state`.
/// - `suppressor_attached` — true when `ActorState::suppressor.attached`
///   is set on the actor's currently-equipped weapon.
/// - `reload_state` — [`ReloadState`] reload-window discriminator (`Idle`
///   vs `Reloading`), derived from
///   [`cf_equipment::RifleState::reload_remaining_ticks`] (`Reloading` when
///   `> 0`, otherwise `Idle`). Defaults to `Idle` when no rifle handle is
///   threaded through.
/// - `charge_fraction` — charge-mode (e.g. sniper) accumulator scalar
///   `0..1` from `ActorState::weapon_charge_fraction`. 0.0 when the
///   weapon is not in `AdvancedFireMode::Charge` mode or the trigger
///   has not been held this trigger cycle.
///
/// The shape is deliberately additive — every field carries a default so the
/// observation surface remains consistent for the conversion paths that
/// only see [`ActorState`]. Engine code that has the per-actor
/// [`cf_equipment::RifleState`] in hand should use
/// [`ActorObservation::from_actor_and_rifle`] so the magazine + reload
/// fields reflect the live tick state.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WeaponStateView {
    pub mag_remaining: u32,
    pub fire_mode: AdvancedFireMode,
    pub bipod_state: BipodState,
    pub suppressor_attached: bool,
    pub reload_state: ReloadState,
    pub charge_fraction: f32,
}

impl ActorObservation {
    /// **M6**: build an [`ActorObservation`] with the per-actor rifle state
    /// threaded through so [`WeaponStateView::mag_remaining`] and
    /// [`WeaponStateView::reload_state`] reflect the live tick values from
    /// the engine's [`crate::sim::RifleStates`] map. Pass `None` from
    /// contexts that don't track rifle state (tests, benches, and any
    /// pre-rifle-allocation path); the magazine field falls back to the
    /// rifle preset's `mag_capacity` and reload reports `Idle`.
    pub fn from_actor_and_rifle(actor: &ActorState, rifle: Option<&cf_equipment::RifleState>) -> Self {
        Self {
            id: actor.id.0,
            team: actor.team.clone(),
            controllable: actor.controllable,
            position: [actor.position.x, actor.position.y],
            velocity: [actor.velocity.x, actor.velocity.y],
            aim: [actor.aim.x, actor.aim.y],
            on_ground: actor.on_ground,
            status: actor.status.as_str().to_string(),
            hp: actor.hp,
            hp_max: actor.hp_max,
            selected_slot: actor.inventory.selected.0,
            selected_item: actor.inventory.selected_item().label().to_string(),
            inventory: actor.inventory.items.iter().map(|i| i.label().to_string()).collect(),
            stance: actor.stance().as_str().to_string(),
            body_silhouette: actor.body_silhouette(),
            chassis: actor.chassis_view(),
            origin_id: actor.origin_id.clone(),
            stability: actor.stability,
            stability_recovery_rate: actor.stability_recovery_rate,
            mass_kg: actor.mass_kg,
            crouch_active: actor.crouch_active,
            climb_active: actor.climb_active,
            jet_active: actor.jet_active,
            sharp_aim_progress: actor.sharp_aim_progress,
            recoil_accumulator: actor.recoil_accumulator,
            knockdown_ticks_remaining: actor.knockdown_ticks_remaining,
            dying_dwell_ticks_remaining: actor.dying_dwell_ticks_remaining,
            mission_critical: actor.mission_critical,
            bloom_factor: actor.bloom_factor,
            facing: actor.facing.as_str().to_string(),
            stamina: actor.stamina.current,
            stamina_max: actor.stamina.max,
            sprint_active: actor.sprint_active,
            prone_active: actor.prone_active,
            lean_angle_degrees: actor.lean_state.angle_degrees,
            lean_direction: actor.lean_state.direction.as_str().to_string(),
            stealth_meter: actor.stealth_meter,
            spotted: actor.stealth_meter >= 0.5,
            cover_side: actor.cover_state.side.as_str().to_string(),
            cover_effectiveness: actor.cover_state.effectiveness,
            inventory_weight_kg: actor.inventory_weight_kg,
            weight_forces_walk: actor.inventory_weight_kg > 30.0,
            limb_loss: actor.limb_loss,
            inventory_extended: actor.extended_inventory_view(),
            weapon_state: actor.weapon_state_view(rifle),
            is_brain: actor.is_brain,
            hit_reaction_kind: actor.hit_reaction_kind.clone(),
            hit_reaction_ticks_remaining: actor.hit_reaction_ticks_remaining,
            drone_mode: actor.drone_ally.as_ref().map(|d| d.mode.as_str().to_string()),
            drone_fuel: actor.drone_ally.as_ref().map(|d| d.fuel),
            max_carry_kg: actor.max_carry_kg(),
            max_carry_volume_l: actor.max_carry_volume_l(),
            total_carried_kg: actor.inventory_grid_total_mass_kg(),
            total_carried_volume_l: actor.inventory_grid_total_bulk_l(),
            encumbrance_walk_speed_multiplier: actor.encumbrance_walk_speed_multiplier(),
            encumbrance_band: actor.encumbrance_band().as_str().to_string(),
            encumbered: actor.is_encumbered(),
            inventory_grid: actor.inventory_grid_view(),
        }
    }
}

impl From<&ActorState> for ActorObservation {
    fn from(actor: &ActorState) -> Self {
        Self::from_actor_and_rifle(actor, None)
    }
}

impl ActorState {
    /// **M6**: build the extended-inventory projection (8 active slots + 3
    /// tank slots, with the tank slots reporting `state="locked"`).
    ///
    /// Slots 0..=7 mirror the actor's `Inventory.items` (8-slot vec on
    /// M6+; legacy 4-slot vecs naturally project as empty for the upper
    /// 4 slots). Slots 8..=10 are the M17 forward-compat tank slots and
    /// always report `state="locked"`.
    pub fn extended_inventory_view(&self) -> Vec<ExtendedInventorySlotView> {
        let slot_kinds = [
            "primary",
            "secondary",
            "sidearm",
            "tool1",
            "tool2",
            "grenade",
            "medical",
            "special",
        ];
        let tank_kinds = [
            ("tank_primary", "Reserved — see M17 for tank ladder"),
            ("tank_secondary", "Reserved — see M17 for tank ladder"),
            ("tank_utility", "Reserved — see M17 for tank ladder"),
        ];
        let mut out = Vec::with_capacity(slot_kinds.len() + tank_kinds.len());
        for (i, name) in slot_kinds.iter().enumerate() {
            let item = self.inventory.items.get(i).cloned().unwrap_or(InventoryItem::Empty);
            // **M6B**: per-slot mass + bulk derive from the canonical
            // `cf_equipment::ItemSpec` registry. Unknown ids fall back
            // to the M6 hardcoded weight (3.5) for legacy compat.
            let (state, item_id, weight, bulk) = match &item {
                InventoryItem::Empty => ("empty", String::new(), 0.0, 0.0),
                InventoryItem::Rifle { preset } => {
                    let spec = cf_equipment::spec_for_id(preset);
                    let mass = spec.as_ref().map_or(3.5, |s| s.mass_kg);
                    let bulk = spec.as_ref().map_or(0.0, |s| s.bulk_volume_l);
                    ("occupied", preset.clone(), mass, bulk)
                }
            };
            out.push(ExtendedInventorySlotView {
                kind: (*name).to_string(),
                state: state.to_string(),
                item_id,
                weight_kg: weight,
                bulk_volume_l: bulk,
                locked_tooltip: None,
            });
        }
        for (name, tooltip) in &tank_kinds {
            out.push(ExtendedInventorySlotView {
                kind: (*name).to_string(),
                state: "locked".to_string(),
                item_id: String::new(),
                weight_kg: 0.0,
                bulk_volume_l: 0.0,
                locked_tooltip: Some((*tooltip).to_string()),
            });
        }
        out
    }

    /// **M6B**: build the per-actor inventory-grid projection used by
    /// `observe.actor.inventory_grid`. Walks every top-level placement
    /// in the grid and emits a [`InventoryGridPlacementView`] with the
    /// canonical mass + bulk + category + nested-count derived from
    /// the [`cf_equipment::ItemSpec`] registry. Returns `None` when no
    /// grid is attached (pre-M6B legacy actors).
    pub fn inventory_grid_view(&self) -> Option<InventoryGridView> {
        let grid = self.inventory_grid.as_ref()?;
        let (w, h) = grid.dimensions();
        let placements = grid
            .items
            .iter()
            .map(|p| {
                let spec = cf_equipment::spec_for_id(&p.item_id);
                let (mass, bulk, dims, category, is_container, liquid_cap, quick_slot) = match spec {
                    Some(s) => (
                        p.mass_kg(&s),
                        p.bulk_volume_l(&s),
                        if p.rotated {
                            s.dimensions.rotated()
                        } else {
                            s.dimensions
                        },
                        s.category.as_str().to_string(),
                        s.is_container(),
                        s.liquid_capacity_l.unwrap_or(0.0),
                        s.quick_slot_eligible,
                    ),
                    None => (
                        0.0,
                        0.0,
                        cf_equipment::GridDim::new(1, 1),
                        String::new(),
                        false,
                        0.0,
                        false,
                    ),
                };
                let nested_count = p.container.as_ref().map(|c| c.items.len() as u16).unwrap_or(0);
                InventoryGridPlacementView {
                    instance_id: p.instance_id,
                    item_id: p.item_id.clone(),
                    category,
                    origin: [p.origin.0, p.origin.1],
                    dimensions: [dims.w, dims.h],
                    rotated: p.rotated,
                    stack_count: p.count,
                    mass_kg: mass,
                    bulk_volume_l: bulk,
                    is_container,
                    nested_count,
                    current_liquid_l: p.current_liquid_l,
                    liquid_capacity_l: liquid_cap,
                    quick_slot_eligible: quick_slot,
                }
            })
            .collect();
        Some(InventoryGridView {
            tier: grid.tier.as_str().to_string(),
            grid_w: w,
            grid_h: h,
            placements,
            total_mass_kg: grid.total_mass_kg(),
            total_bulk_l: grid.total_bulk_l(),
        })
    }

    /// **M6**: project a [`WeaponStateView`] for the actor's currently
    /// selected weapon. All six fields are now live:
    ///
    /// - `mag_remaining` reads from
    ///   [`cf_equipment::RifleState::ammo_in_mag`] when the caller threads
    ///   the per-actor rifle state; otherwise falls back to the rifle
    ///   preset's `mag_capacity` (and `0` when the active slot is not a
    ///   rifle).
    /// - `fire_mode` reads [`ActorState::weapon_fire_mode`] (rotated by
    ///   `act.player.cycle_fire_mode`).
    /// - `bipod_state` reads [`ActorState::bipod.state`].
    /// - `suppressor_attached` reads [`ActorState::suppressor.attached`].
    /// - `reload_state` is derived from
    ///   [`cf_equipment::RifleState::reload_remaining_ticks`] (`Reloading`
    ///   when `> 0`, `Idle` otherwise). When the caller passes `None`,
    ///   `Idle` is reported.
    /// - `charge_fraction` reads
    ///   [`ActorState::weapon_charge_fraction`] (filled by the
    ///   Charge-mode firing path).
    ///
    /// See the [`WeaponStateView`] doc comment for the full shape
    /// contract.
    pub fn weapon_state_view(&self, rifle: Option<&cf_equipment::RifleState>) -> WeaponStateView {
        let mag_remaining = match (self.inventory.selected_item(), rifle) {
            (InventoryItem::Rifle { .. }, Some(r)) => r.ammo_in_mag,
            (InventoryItem::Rifle { preset }, None) => {
                cf_equipment::rifle_preset(preset).map_or(0, |spec| spec.mag_capacity)
            }
            (InventoryItem::Empty, _) => 0,
        };
        let reload_state = match rifle {
            Some(r) if r.reload_remaining_ticks > 0 => ReloadState::Reloading,
            _ => ReloadState::Idle,
        };
        WeaponStateView {
            mag_remaining,
            fire_mode: self.weapon_fire_mode,
            bipod_state: self.bipod.state,
            suppressor_attached: self.suppressor.attached,
            reload_state,
            charge_fraction: self.weapon_charge_fraction.clamp(0.0, 1.0),
        }
    }
}

/// **M5**: chassis projection for `cfctl observe` / `cfctl inspect chassis`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChassisView {
    pub spec_id: String,
    pub kind: String,
    pub stage: String,
    pub pilot_state: String,
    pub weapon_jammed: bool,
    pub tutorial_safety: bool,
    pub mass_kg: f32,
    pub zones: Vec<ChassisZoneView>,
    pub modules: Vec<ChassisModuleView>,
    pub integrity: f32,
    pub eject_ticks_remaining: u32,
    pub eject_ticks_total: u32,
    pub destroyed_zones: Vec<String>,
    pub salvaged_module_ids: Vec<String>,
}

/// Per-zone chassis view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChassisZoneView {
    pub zone: String,
    pub external_integrity: f32,
    pub internal_integrity: f32,
    pub core_integrity: f32,
    pub wound_integrity: f32,
    pub destroyed: bool,
    pub zone_integrity: f32,
}

/// Per-module chassis view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChassisModuleView {
    pub id: String,
    pub kind: String,
    pub state: String,
    pub bound_zone: String,
    pub integrity: f32,
    pub last_reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod body_a;
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
