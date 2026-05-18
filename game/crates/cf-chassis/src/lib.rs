//! M5: chassis grammar, body graph, armor layers, modules, damage stages.
//!
//! This crate is the runtime contract that turns the M1 "single rifle + flat HP bar"
//! actor into the M5 contract from
//! `spec/chassis-armor-mechs-and-origins` + DR-014 + DR-021:
//!
//! - **Body graph** (`BodyGraph`): named limbs (head/torso/arms/legs/backpack),
//!   attachment joints between them, equipment sockets (`hand_right`, `back_mount`,
//!   etc.), wound containers per zone, armor-coverage map per zone, and per-limb
//!   movement-contribution fields (does losing this limb disable jet/jump/climb/aim/etc.).
//! - **Layered armor** (`ArmorLayer`): each zone has an external + internal + core
//!   layer with hp/max_hp/hardness/integrity. Damage strips layers in order; once
//!   the core is breached the zone becomes a wound container and routes damage to
//!   the actor's HP.
//! - **Modules** (`ChassisModule`): jet / shield / sensor / weapon_mount / repair_drone
//!   with `nominal → degraded → warning → failed` state machines bound to a body zone
//!   so module health follows the zone it sits on (failing a torso destroys the jet
//!   when bound there; surviving with one arm degrades weapon_mount).
//! - **Damage stages** (`ChassisStage`): the 11-stage pipeline from the roadmap M5
//!   done-criteria: nominal → degraded → module-warning → module-failed →
//!   weapon-jammed → armor-cracked → disabled → pilot-injured → eject → bail-too-late
//!   → wreck → gibbed. Transitions emit replay events with reason labels.
//! - **Pilot binding** (`PilotState`): pilot lives inside a chassis until it wrecks;
//!   `attempt_eject` moves to Ejecting then Ejected → Extracted; missing the eject
//!   window flips to BailedTooLate → Lost.
//! - **Tutorial-safety policy**: when `tutorial_safety = true` lethal damage caps
//!   at Disabled / PilotInjured (no Wreck / Gibbed), and `attempt_eject` cannot
//!   transition to BailedTooLate during the tutorial window.
//!
//! Reference chassis are provided as `powered_armor_default`, `light_mech_default`,
//! and `infantry_default`. Modders can clone them via `ChassisSpec::clone` and
//! mutate before insertion.
//!
//! Determinism contract: every public mutator is pure (state in → state out via
//! `&mut self`); no clock reads; no `rand::thread_rng()`. The engine seeds any RNG
//! it needs for jam-chance rolls and feeds it in explicitly.

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
    clippy::cast_lossless,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::if_not_else,
    clippy::needless_pass_by_value,
    clippy::needless_continue
)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Stable id for a chassis spec preset. Scenarios reference these by id and the
/// runtime resolves them through [`chassis_spec`].
pub const POWERED_ARMOR_ID: &str = "powered_armor_v1";
pub const LIGHT_MECH_ID: &str = "light_mech_v1";
pub const INFANTRY_ID: &str = "infantry_v1";
/// **M13** § "Chassis archetypes — M13 ships 5" — non-humanoid quadruped
/// archetype: 4 legs + 2 claws + carapace + sensor cluster; no jet.
pub const CRAB_QUADRUPED_ID: &str = "crab_quadruped_v1";
/// **M13** § "Chassis archetypes — M13 ships 5" — autonomous miniature
/// chassis: 4 zones (chassis core + 2 arms + sensor pod); no pilot.
pub const DRONE_ID: &str = "drone_v1";
/// **M14A** § "Heavy Armor — `heavy_trooper_v1`" — tank-grade infantry
/// chassis. 380 kg base loaded; rifles glance, only AP/HE breach.
pub const HEAVY_TROOPER_ID: &str = "heavy_trooper_v1";

/// **M13** § "Chassis archetypes — M13 ships 5". Discriminants `Infantry=0`
/// through `LightMech=2` are pinned for cross-milestone determinism; the new
/// `CrabQuadruped=3` + `Drone=4` are appended so the repr(u8) byte layout
/// stays stable.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChassisKind {
    Infantry = 0,
    PoweredArmor = 1,
    LightMech = 2,
    /// **M13** non-humanoid quadruped (crab-like) chassis.
    CrabQuadruped = 3,
    /// **M13** autonomous drone chassis (no pilot binding).
    Drone = 4,
    /// **M14A** § "Heavy Armor" — tank-grade infantry; small arms barely
    /// scratch; 380 kg loaded; throttle-for-weight jet visibly struggles.
    HeavyTrooper = 5,
}

impl ChassisKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ChassisKind::Infantry => "infantry",
            ChassisKind::PoweredArmor => "powered_armor",
            ChassisKind::LightMech => "light_mech",
            ChassisKind::CrabQuadruped => "crab_quadruped",
            ChassisKind::Drone => "drone",
            ChassisKind::HeavyTrooper => "heavy_trooper",
        }
    }

    /// Reference ids (`POWERED_ARMOR_ID`, `LIGHT_MECH_ID`, `INFANTRY_ID`,
    /// `CRAB_QUADRUPED_ID`, `DRONE_ID`, `HEAVY_TROOPER_ID`).
    pub fn default_spec_id(self) -> &'static str {
        match self {
            ChassisKind::Infantry => INFANTRY_ID,
            ChassisKind::PoweredArmor => POWERED_ARMOR_ID,
            ChassisKind::LightMech => LIGHT_MECH_ID,
            ChassisKind::CrabQuadruped => CRAB_QUADRUPED_ID,
            ChassisKind::Drone => DRONE_ID,
            ChassisKind::HeavyTrooper => HEAVY_TROOPER_ID,
        }
    }

    /// **M13** § "Per-chassis ability slot count (Light=1, Medium=2, Heavy=3,
    /// Drone=1)". Used by [`ChassisAbilitySlots`] to bound the active
    /// ability roster.
    pub fn ability_slot_count(self) -> u8 {
        match self {
            ChassisKind::Infantry | ChassisKind::Drone => 1,
            ChassisKind::PoweredArmor | ChassisKind::CrabQuadruped => 2,
            ChassisKind::LightMech | ChassisKind::HeavyTrooper => 3,
        }
    }

    /// **M13** § "Weapon modifier slots (Noita-style combinatorial)". Per
    /// the spec's per-chassis-tier table (Infantry 0-1, Powered armor 1-2,
    /// Light mech 2-3, Heavy mech future 3-4). We surface the max for each.
    pub fn weapon_modifier_slot_count(self) -> u8 {
        match self {
            ChassisKind::Infantry | ChassisKind::Drone => 1,
            ChassisKind::PoweredArmor | ChassisKind::CrabQuadruped => 2,
            ChassisKind::LightMech | ChassisKind::HeavyTrooper => 3,
        }
    }

    /// **M13** § "Pilot-inside-chassis dual silhouette" — weight class drives
    /// the pilot silhouette size scaling (Light 60% / Medium 40% / Heavy 25%).
    /// Returns the scale factor (0..1) for the pilot inset overlay.
    pub fn pilot_silhouette_scale(self) -> f32 {
        match self {
            ChassisKind::Infantry | ChassisKind::Drone => 1.0,
            ChassisKind::PoweredArmor => 0.6,
            ChassisKind::CrabQuadruped | ChassisKind::HeavyTrooper => 0.4,
            ChassisKind::LightMech => 0.25,
        }
    }

    /// **M13** § "Cockpit camera anchor" — only Medium + Heavy classes
    /// support the cockpit anchor (Light has third-person only). Returns
    /// `true` when [`crate::CameraAnchor::Cockpit`] is a valid request.
    pub fn supports_cockpit_anchor(self) -> bool {
        matches!(
            self,
            ChassisKind::LightMech | ChassisKind::CrabQuadruped | ChassisKind::HeavyTrooper
        )
    }
}

/// **M13** § "Cockpit camera anchor (first-person mech view)". Tracks the
/// current camera anchor request driven by `act.input.camera_anchor`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraAnchor {
    /// Default third-person follow camera.
    #[default]
    Default = 0,
    /// First-person inside chassis cockpit (Medium + Heavy classes only).
    Cockpit = 1,
}

impl CameraAnchor {
    pub fn as_str(self) -> &'static str {
        match self {
            CameraAnchor::Default => "default",
            CameraAnchor::Cockpit => "cockpit",
        }
    }

    pub fn parse(s: &str) -> Option<CameraAnchor> {
        match s.to_ascii_lowercase().as_str() {
            "default" | "follow" | "third_person" => Some(CameraAnchor::Default),
            "cockpit" | "first_person" => Some(CameraAnchor::Cockpit),
            _ => None,
        }
    }
}

/// Named body zones. M5 ships the full 14-zone limb model the roadmap M5
/// section requires: head, torso, upper-arm / forearm / hand (left + right),
/// thigh / shin / foot (left + right), and a backpack/jetpack slot.
/// `ArmLeft` / `ArmRight` / `LegLeft` / `LegRight` remain as **composite**
/// aliases (the upper limb in their respective chain) so legacy zone-from-hit
/// resolution + AI heuristics + checksum byte layout stay stable; the new
/// granular variants are appended to the enum so cross-milestone determinism
/// is preserved per the `repr(u8)` discriminant order.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyZone {
    Head = 0,
    Torso = 1,
    /// Composite alias for the upper-arm chain (left). Treated as the upper
    /// arm during zone resolution; granular `ForearmLeft`/`HandLeft` are
    /// addressable separately.
    ArmLeft = 2,
    ArmRight = 3,
    LegLeft = 4,
    LegRight = 5,
    Backpack = 6,
    ForearmLeft = 7,
    ForearmRight = 8,
    HandLeft = 9,
    HandRight = 10,
    ShinLeft = 11,
    ShinRight = 12,
    FootLeft = 13,
    FootRight = 14,
    // **M13** § "Quadruped=11 zones (4 legs + 2 claws + torso + sensor cluster + carapace)".
    // Front-left/right + rear-left/right legs; left/right claws; carapace shell; sensor cluster.
    LegFrontLeft = 15,
    LegFrontRight = 16,
    LegRearLeft = 17,
    LegRearRight = 18,
    ClawLeft = 19,
    ClawRight = 20,
    Carapace = 21,
    SensorCluster = 22,
    // **M13** § "Drone=4 zones (chassis + 2 arms + sensor pod)".
    DroneCore = 23,
    DroneArmLeft = 24,
    DroneArmRight = 25,
    DroneSensorPod = 26,
}

impl BodyZone {
    pub fn as_str(self) -> &'static str {
        match self {
            BodyZone::Head => "head",
            BodyZone::Torso => "torso",
            BodyZone::ArmLeft => "arm_left",
            BodyZone::ArmRight => "arm_right",
            BodyZone::LegLeft => "leg_left",
            BodyZone::LegRight => "leg_right",
            BodyZone::Backpack => "backpack",
            BodyZone::ForearmLeft => "forearm_left",
            BodyZone::ForearmRight => "forearm_right",
            BodyZone::HandLeft => "hand_left",
            BodyZone::HandRight => "hand_right",
            BodyZone::ShinLeft => "shin_left",
            BodyZone::ShinRight => "shin_right",
            BodyZone::FootLeft => "foot_left",
            BodyZone::FootRight => "foot_right",
            BodyZone::LegFrontLeft => "leg_front_left",
            BodyZone::LegFrontRight => "leg_front_right",
            BodyZone::LegRearLeft => "leg_rear_left",
            BodyZone::LegRearRight => "leg_rear_right",
            BodyZone::ClawLeft => "claw_left",
            BodyZone::ClawRight => "claw_right",
            BodyZone::Carapace => "carapace",
            BodyZone::SensorCluster => "sensor_cluster",
            BodyZone::DroneCore => "drone_core",
            BodyZone::DroneArmLeft => "drone_arm_left",
            BodyZone::DroneArmRight => "drone_arm_right",
            BodyZone::DroneSensorPod => "drone_sensor_pod",
        }
    }

    /// Canonical iteration order for events + checksums. Humanoid zones first
    /// (stable since M5); quadruped + drone zones appended at the end so
    /// existing checksums for chassis-less + humanoid actors stay byte-identical.
    pub fn all() -> &'static [BodyZone] {
        &[
            BodyZone::Head,
            BodyZone::Torso,
            BodyZone::ArmLeft,
            BodyZone::ArmRight,
            BodyZone::LegLeft,
            BodyZone::LegRight,
            BodyZone::Backpack,
            BodyZone::ForearmLeft,
            BodyZone::ForearmRight,
            BodyZone::HandLeft,
            BodyZone::HandRight,
            BodyZone::ShinLeft,
            BodyZone::ShinRight,
            BodyZone::FootLeft,
            BodyZone::FootRight,
            BodyZone::LegFrontLeft,
            BodyZone::LegFrontRight,
            BodyZone::LegRearLeft,
            BodyZone::LegRearRight,
            BodyZone::ClawLeft,
            BodyZone::ClawRight,
            BodyZone::Carapace,
            BodyZone::SensorCluster,
            BodyZone::DroneCore,
            BodyZone::DroneArmLeft,
            BodyZone::DroneArmRight,
            BodyZone::DroneSensorPod,
        ]
    }

    /// Parent zone in the kinematic chain. `Head` and `Torso` and `Backpack`
    /// have no parent; the chains are torso → arm/leg (upper) → forearm/shin →
    /// hand/foot. Quadruped legs root in carapace; claws root in their leg.
    /// Drone arms root in the drone core; sensor pod is a leaf on the core.
    pub fn parent(self) -> Option<BodyZone> {
        match self {
            BodyZone::Head
            | BodyZone::Torso
            | BodyZone::Backpack
            | BodyZone::Carapace
            | BodyZone::SensorCluster
            | BodyZone::DroneCore => None,
            BodyZone::ArmLeft | BodyZone::ArmRight | BodyZone::LegLeft | BodyZone::LegRight => Some(BodyZone::Torso),
            BodyZone::ForearmLeft => Some(BodyZone::ArmLeft),
            BodyZone::ForearmRight => Some(BodyZone::ArmRight),
            BodyZone::HandLeft => Some(BodyZone::ForearmLeft),
            BodyZone::HandRight => Some(BodyZone::ForearmRight),
            BodyZone::ShinLeft => Some(BodyZone::LegLeft),
            BodyZone::ShinRight => Some(BodyZone::LegRight),
            BodyZone::FootLeft => Some(BodyZone::ShinLeft),
            BodyZone::FootRight => Some(BodyZone::ShinRight),
            BodyZone::LegFrontLeft
            | BodyZone::LegFrontRight
            | BodyZone::LegRearLeft
            | BodyZone::LegRearRight => Some(BodyZone::Carapace),
            BodyZone::ClawLeft => Some(BodyZone::LegFrontLeft),
            BodyZone::ClawRight => Some(BodyZone::LegFrontRight),
            BodyZone::DroneArmLeft | BodyZone::DroneArmRight | BodyZone::DroneSensorPod => Some(BodyZone::DroneCore),
        }
    }

    /// True iff this zone is one of the M13 quadruped-only zones.
    pub fn is_quadruped_zone(self) -> bool {
        matches!(
            self,
            BodyZone::LegFrontLeft
                | BodyZone::LegFrontRight
                | BodyZone::LegRearLeft
                | BodyZone::LegRearRight
                | BodyZone::ClawLeft
                | BodyZone::ClawRight
                | BodyZone::Carapace
                | BodyZone::SensorCluster
        )
    }

    /// True iff this zone is one of the M13 drone-only zones.
    pub fn is_drone_zone(self) -> bool {
        matches!(
            self,
            BodyZone::DroneCore | BodyZone::DroneArmLeft | BodyZone::DroneArmRight | BodyZone::DroneSensorPod
        )
    }

    /// True for the right-side arm chain (upper / forearm / hand).
    pub fn is_right_arm_chain(self) -> bool {
        matches!(self, BodyZone::ArmRight | BodyZone::ForearmRight | BodyZone::HandRight)
    }

    /// True for the left-side arm chain.
    pub fn is_left_arm_chain(self) -> bool {
        matches!(self, BodyZone::ArmLeft | BodyZone::ForearmLeft | BodyZone::HandLeft)
    }

    /// True for the right-side leg chain.
    pub fn is_right_leg_chain(self) -> bool {
        matches!(self, BodyZone::LegRight | BodyZone::ShinRight | BodyZone::FootRight)
    }

    /// True for the left-side leg chain.
    pub fn is_left_leg_chain(self) -> bool {
        matches!(self, BodyZone::LegLeft | BodyZone::ShinLeft | BodyZone::FootLeft)
    }
}

/// Equipment socket id. Sockets are addressed by stable string so modders can
/// reference them in role records (`hand_right`, `back_mount`, etc.). M5 ships a
/// canonical set; mods can add more.
pub const SOCKET_HAND_RIGHT: &str = "hand_right";
pub const SOCKET_HAND_LEFT: &str = "hand_left";
pub const SOCKET_BACK_MOUNT: &str = "back_mount";
pub const SOCKET_HEAD_MOUNT: &str = "head_mount";
pub const SOCKET_TORSO_HARDPOINT: &str = "torso_hardpoint";

/// Layer ordering within a zone. Damage strips `External` first, `Internal` next,
/// then breaches into `Core`. Once `Core.hp == 0` the zone is considered breached
/// and routes damage to the actor HP via [`ZoneDamageOutcome::actor_hp_damage`].
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArmorLayerKind {
    External = 0,
    Internal = 1,
    Core = 2,
}

impl ArmorLayerKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ArmorLayerKind::External => "external",
            ArmorLayerKind::Internal => "internal",
            ArmorLayerKind::Core => "core",
        }
    }
}

/// One layer of armor on a zone. `hardness` reduces incoming damage; `integrity` is
/// a 0..1 derived field surfaced for HUD + AI ("75% external integrity left").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmorLayer {
    pub kind: ArmorLayerKind,
    pub hp: f32,
    pub hp_max: f32,
    /// Flat damage reduction subtracted from incoming damage before HP is touched.
    /// Clamped at 0 so a Hardness > damage produces a no-op (ricochet).
    pub hardness: f32,
}

impl ArmorLayer {
    pub fn new(kind: ArmorLayerKind, hp_max: f32, hardness: f32) -> Self {
        Self {
            kind,
            hp: hp_max.max(0.0),
            hp_max: hp_max.max(0.0),
            hardness: hardness.max(0.0),
        }
    }

    pub fn integrity(&self) -> f32 {
        if self.hp_max <= 0.0 {
            0.0
        } else {
            (self.hp / self.hp_max).clamp(0.0, 1.0)
        }
    }

    pub fn is_breached(&self) -> bool {
        self.hp <= 0.0
    }

    pub fn reset(&mut self) {
        self.hp = self.hp_max;
    }
}

/// Per-zone armor + wound container. `wound_hp` accumulates AFTER all three armor
/// layers are breached; when `wound_hp` hits zero the zone is considered destroyed
/// and emits `armor_zone_destroyed`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZoneState {
    pub zone: BodyZone,
    pub layers: Vec<ArmorLayer>,
    pub wound_hp: f32,
    pub wound_hp_max: f32,
    /// **M14A** § "Per-attachable `damage_multiplier`" — scales incoming
    /// damage on this zone (< 1.0 = tougher). Default 1.0.
    #[serde(default = "default_damage_multiplier")]
    pub damage_multiplier: f32,
    /// **M14A** § "Per-zone `gib_impulse_limit`" — impulse threshold (N·s)
    /// below which the zone cannot be gibbed off. Default 800 N·s; heavy
    /// chassis raises this to 1600..3200 N·s.
    #[serde(default = "default_gib_impulse_limit")]
    pub gib_impulse_limit: f32,
    /// **M14A** § "Per-zone `stagger_factor`" — multiplier on hit-reaction
    /// duration + knockdown probability (0.2 = heavy; 1.0 = baseline).
    #[serde(default = "default_stagger_factor")]
    pub stagger_factor: f32,
    /// `true` once `wound_hp <= 0`; the zone is destroyed and emits
    /// `armor_zone_destroyed`. Limb destruction has mechanical consequences listed
    /// in [`BodyGraph::movement_contributions`].
    pub destroyed: bool,
}

fn default_damage_multiplier() -> f32 {
    1.0
}

fn default_gib_impulse_limit() -> f32 {
    800.0
}

fn default_stagger_factor() -> f32 {
    1.0
}

impl ZoneState {
    pub fn new(zone: BodyZone, layers: Vec<ArmorLayer>, wound_hp: f32) -> Self {
        Self {
            zone,
            layers,
            wound_hp: wound_hp.max(0.0),
            wound_hp_max: wound_hp.max(0.0),
            damage_multiplier: default_damage_multiplier(),
            gib_impulse_limit: default_gib_impulse_limit(),
            stagger_factor: default_stagger_factor(),
            destroyed: false,
        }
    }

    /// **M14A** § "Per-zone tunings" — chain-able builder for heavy-armor archetypes.
    #[must_use]
    pub fn with_damage_multiplier(mut self, mult: f32) -> Self {
        self.damage_multiplier = mult;
        self
    }

    #[must_use]
    pub fn with_gib_impulse_limit(mut self, limit: f32) -> Self {
        self.gib_impulse_limit = limit;
        self
    }

    #[must_use]
    pub fn with_stagger_factor(mut self, factor: f32) -> Self {
        self.stagger_factor = factor;
        self
    }

    pub fn reset(&mut self) {
        for layer in &mut self.layers {
            layer.reset();
        }
        self.wound_hp = self.wound_hp_max;
        self.destroyed = false;
    }

    pub fn external_integrity(&self) -> f32 {
        self.layers
            .iter()
            .find(|l| l.kind == ArmorLayerKind::External)
            .map_or(0.0, ArmorLayer::integrity)
    }

    pub fn internal_integrity(&self) -> f32 {
        self.layers
            .iter()
            .find(|l| l.kind == ArmorLayerKind::Internal)
            .map_or(0.0, ArmorLayer::integrity)
    }

    pub fn core_integrity(&self) -> f32 {
        self.layers
            .iter()
            .find(|l| l.kind == ArmorLayerKind::Core)
            .map_or(0.0, ArmorLayer::integrity)
    }

    pub fn wound_integrity(&self) -> f32 {
        if self.wound_hp_max <= 0.0 {
            0.0
        } else {
            (self.wound_hp / self.wound_hp_max).clamp(0.0, 1.0)
        }
    }

    /// Composite "how OK is this zone" — averages external/internal/core/wound. Used
    /// for HUD silhouette tinting + AI utility scoring.
    pub fn zone_integrity(&self) -> f32 {
        let mut sum = 0.0;
        let mut n = 0.0;
        for layer in &self.layers {
            sum += layer.integrity();
            n += 1.0;
        }
        sum += self.wound_integrity();
        n += 1.0;
        if n > 0.0 {
            sum / n
        } else {
            0.0
        }
    }
}

/// Kind of chassis module. Each module has a state machine + can be bound to a body
/// zone so module health follows the zone. Discriminants 0..4 are the M5 set
/// (stable for cross-milestone determinism). M13 appends six "critical chassis
/// modules" per the spec § "Critical chassis modules with full mechanics".
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleKind {
    WeaponMount = 0,
    Jet = 1,
    Shield = 2,
    Sensor = 3,
    RepairDrone = 4,
    /// **M13** § "Cockpit module (pilot inside; mech weight class only)".
    Cockpit = 5,
    /// **M13** § "Ammo rack module (explosive cascade)".
    AmmoRack = 6,
    /// **M13** § "Engine module (fire risk)".
    Engine = 7,
    /// **M13** § "Optics module (vision impairment)".
    Optics = 8,
    /// **M13** § "Transmission module (mobility)".
    Transmission = 9,
    /// **M13** § "Reactor module (catastrophic if destroyed)".
    Reactor = 10,
    /// **M13** § "Per-chassis module positions" — power core (drone + powered armor).
    PowerCore = 11,
    /// **M13** § "Per-chassis module positions" — internal fuel tank.
    FuelTank = 12,
    /// **M13** § "Per-chassis module positions" — targeting computer.
    TargetingComputer = 13,
    /// **M13** § "Per-chassis module positions" — comm relay.
    CommRelay = 14,
    /// **M13** § "Per-chassis module positions" — per-leg motor controller.
    MotorController = 15,
}

impl ModuleKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ModuleKind::WeaponMount => "weapon_mount",
            ModuleKind::Jet => "jet",
            ModuleKind::Shield => "shield",
            ModuleKind::Sensor => "sensor",
            ModuleKind::RepairDrone => "repair_drone",
            ModuleKind::Cockpit => "cockpit",
            ModuleKind::AmmoRack => "ammo_rack",
            ModuleKind::Engine => "engine",
            ModuleKind::Optics => "optics",
            ModuleKind::Transmission => "transmission",
            ModuleKind::Reactor => "reactor",
            ModuleKind::PowerCore => "power_core",
            ModuleKind::FuelTank => "fuel_tank",
            ModuleKind::TargetingComputer => "targeting_computer",
            ModuleKind::CommRelay => "comm_relay",
            ModuleKind::MotorController => "motor_controller",
        }
    }

    /// **M13** § "Critical chassis modules with full mechanics". True for
    /// modules whose destruction triggers a chassis-wide catastrophic
    /// cascade (`AmmoRack` cook-off, `Reactor` overpressure, `Cockpit`
    /// pilot loss, `Engine` immobilization + fire). The engine wires
    /// these through `ChassisState::apply_module_damage`.
    pub fn is_critical(self) -> bool {
        matches!(
            self,
            ModuleKind::Cockpit
                | ModuleKind::AmmoRack
                | ModuleKind::Engine
                | ModuleKind::Reactor
                | ModuleKind::Optics
                | ModuleKind::Transmission
        )
    }
}

/// **M13** § "Per-module positioning + War Thunder-style module ray traversal".
/// Axis-aligned bounding box in chassis local space (origin = chassis center;
/// units = world pixels). Used to resolve which module a penetrating ray
/// strikes; identity Aabb (size 0) means "module has no positioned hitbox
/// and is treated as bound-zone-coincident".
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Aabb {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl Aabb {
    pub fn new(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Self {
        Self { min_x, min_y, max_x, max_y }
    }

    /// True iff the local point `(x, y)` lies inside (boundary inclusive).
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }

    /// True iff the box has non-zero area.
    pub fn is_positioned(&self) -> bool {
        (self.max_x - self.min_x).abs() > f32::EPSILON && (self.max_y - self.min_y).abs() > f32::EPSILON
    }
}

/// **M13** § "Critical chassis modules with full mechanics". Per-module
/// behavior the engine triggers when a module reaches `Failed`. The
/// `engine.rs` event emitters key off these to surface module-specific
/// `chassis.*` / `module.*` events.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCascade {
    /// No special cascade — module simply stops functioning.
    #[default]
    None = 0,
    /// **AmmoRack**: first-failure cooks 1/3 of remaining ammo; severe
    /// failure detonates the rack catastrophically.
    AmmoCookoff = 1,
    /// **Engine**: oil leak fires + cascading fuel ignition if fuel tank
    /// adjacent + chassis immobilized when fully destroyed.
    EngineFire = 2,
    /// **Cockpit**: cockpit penetration deals damage directly to the pilot.
    PilotDirectDamage = 3,
    /// **Optics**: damaged → sight × 0.5; destroyed → blind.
    SightImpairment = 4,
    /// **Transmission**: damaged → speed × 0.6; destroyed → immobile.
    MobilityLoss = 5,
    /// **Reactor**: overpressure cascade per M9 reactor model.
    ReactorOverpressure = 6,
}

impl FailureCascade {
    pub fn as_str(self) -> &'static str {
        match self {
            FailureCascade::None => "none",
            FailureCascade::AmmoCookoff => "ammo_cookoff",
            FailureCascade::EngineFire => "engine_fire",
            FailureCascade::PilotDirectDamage => "pilot_direct_damage",
            FailureCascade::SightImpairment => "sight_impairment",
            FailureCascade::MobilityLoss => "mobility_loss",
            FailureCascade::ReactorOverpressure => "reactor_overpressure",
        }
    }
}

/// State of one chassis module. Follows the canonical
/// `nominal → degraded → warning → failed` ramp from DR-014/021. `degraded` is the
/// first reduction in capability; `warning` means imminent failure (HUD raises
/// banner); `failed` means the module is inoperative until repaired.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleStateKind {
    Nominal = 0,
    Degraded = 1,
    Warning = 2,
    Failed = 3,
    /// The module is not present on this chassis at all (Infantry has no jet,
    /// `not_present` keeps the HUD module strip stable across chassis kinds).
    NotPresent = 4,
}

impl ModuleStateKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ModuleStateKind::Nominal => "nominal",
            ModuleStateKind::Degraded => "degraded",
            ModuleStateKind::Warning => "warning",
            ModuleStateKind::Failed => "failed",
            ModuleStateKind::NotPresent => "not_present",
        }
    }

    pub fn is_failed(self) -> bool {
        matches!(self, ModuleStateKind::Failed)
    }

    pub fn is_present(self) -> bool {
        !matches!(self, ModuleStateKind::NotPresent)
    }
}

/// One chassis module instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChassisModule {
    pub id: String,
    pub kind: ModuleKind,
    pub bound_zone: BodyZone,
    pub state: ModuleStateKind,
    pub hp: f32,
    pub hp_max: f32,
    /// Reason the module last transitioned (for replay + HUD banner). One of
    /// `bound_zone_destroyed`, `armor_breached`, `direct_hit`, `overheated`,
    /// `jammed`, `repaired`, `salvaged`, or a mod-supplied string.
    pub last_reason: String,
    /// **M13** § "Per-module positioning + War Thunder-style module ray
    /// traversal". Chassis-local AABB describing this module's hitbox for
    /// ray traversal. Empty (`is_positioned() == false`) when the module
    /// has no positioned geometry; ray traversal then falls back to
    /// bound-zone presence.
    #[serde(default)]
    pub local_aabb: Aabb,
    /// **M13** § "Critical chassis modules" — what cascade fires when this
    /// module reaches `Failed`.
    #[serde(default)]
    pub failure_cascade: FailureCascade,
    /// **M13** AmmoRack-only: rounds remaining in the rack. Drives the
    /// `module.ammo_rack_cooking` / `module.ammo_rack_detonated` event
    /// cascade. Zero for modules whose kind != `AmmoRack`.
    #[serde(default)]
    pub ammo_quantity_remaining: u32,
    /// **M13** AmmoRack-only: cumulative `rounds_cooked_off` counter.
    #[serde(default)]
    pub rounds_cooked_off: u32,
    /// **M13** Engine-only: oil reservoir (0..1). 1.0 = full; below 0.3
    /// raises fire-risk on engine penetration.
    #[serde(default = "default_fluid_level")]
    pub oil_level: f32,
    /// **M13** Engine-only: coolant reservoir (0..1).
    #[serde(default = "default_fluid_level")]
    pub coolant_level: f32,
    /// **M13** Reactor-only: pressure tier 0..4 (per M9 5-tier signature).
    /// 0 = nominal, 4 = critical (volatile release imminent).
    #[serde(default)]
    pub pressure_state: u8,
    /// **M14 audit pass 4 (Finding 8)**: highest `ModuleStateKind` for which
    /// this module has already emitted its tier-crossing cascade events
    /// (AmmoCooking / EngineOilLeak / EngineFire / ReactorPressureAdvanced /
    /// OpticsImpaired / MobilityReduced). Used to prevent redundant
    /// cascade emission when multiple zone hits in the same tick each
    /// trigger `apply_critical_module_damage` for the same module while
    /// the module's state hasn't actually crossed a tier this call.
    /// Initialized to `Nominal` (no cascade has fired yet).
    #[serde(default = "default_module_state_kind")]
    pub last_cascade_emitted_state: ModuleStateKind,
}

fn default_module_state_kind() -> ModuleStateKind {
    ModuleStateKind::Nominal
}

fn default_fluid_level() -> f32 {
    1.0
}

impl ChassisModule {
    pub fn new(id: impl Into<String>, kind: ModuleKind, bound_zone: BodyZone, hp_max: f32) -> Self {
        Self {
            id: id.into(),
            kind,
            bound_zone,
            state: ModuleStateKind::Nominal,
            hp: hp_max.max(0.0),
            hp_max: hp_max.max(0.0),
            last_reason: String::new(),
            local_aabb: Aabb::default(),
            failure_cascade: FailureCascade::None,
            ammo_quantity_remaining: 0,
            rounds_cooked_off: 0,
            oil_level: default_fluid_level(),
            coolant_level: default_fluid_level(),
            pressure_state: 0,
            last_cascade_emitted_state: ModuleStateKind::Nominal,
        }
    }

    /// **M13**: builder helper that attaches a chassis-local hitbox to the module.
    #[must_use]
    pub fn with_local_aabb(mut self, aabb: Aabb) -> Self {
        self.local_aabb = aabb;
        self
    }

    /// **M13**: builder helper that wires the failure cascade onto the module.
    #[must_use]
    pub fn with_failure_cascade(mut self, cascade: FailureCascade) -> Self {
        self.failure_cascade = cascade;
        self
    }

    /// **M13** AmmoRack-only: pre-seed the rounds remaining + cascade.
    #[must_use]
    pub fn with_ammo(mut self, rounds: u32) -> Self {
        self.ammo_quantity_remaining = rounds;
        if self.kind == ModuleKind::AmmoRack && self.failure_cascade == FailureCascade::None {
            self.failure_cascade = FailureCascade::AmmoCookoff;
        }
        self
    }

    pub fn not_present(id: impl Into<String>, kind: ModuleKind) -> Self {
        Self {
            id: id.into(),
            kind,
            bound_zone: BodyZone::Torso,
            state: ModuleStateKind::NotPresent,
            hp: 0.0,
            hp_max: 0.0,
            last_reason: String::new(),
            local_aabb: Aabb::default(),
            failure_cascade: FailureCascade::None,
            ammo_quantity_remaining: 0,
            rounds_cooked_off: 0,
            oil_level: 0.0,
            coolant_level: 0.0,
            pressure_state: 0,
            last_cascade_emitted_state: ModuleStateKind::NotPresent,
        }
    }

    pub fn integrity(&self) -> f32 {
        if self.hp_max <= 0.0 {
            0.0
        } else {
            (self.hp / self.hp_max).clamp(0.0, 1.0)
        }
    }

    pub fn reset(&mut self) {
        if self.state != ModuleStateKind::NotPresent {
            self.hp = self.hp_max;
            self.state = ModuleStateKind::Nominal;
            self.last_reason.clear();
            self.rounds_cooked_off = 0;
            self.oil_level = default_fluid_level();
            self.coolant_level = default_fluid_level();
            self.pressure_state = 0;
        }
    }
}

/// 11-stage chassis damage pipeline from the roadmap M5 done-criteria. Stages
/// monotonically advance (except via repair which can step back at most one level).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChassisStage {
    Nominal = 0,
    Degraded = 1,
    ModuleWarning = 2,
    ModuleFailed = 3,
    WeaponJammed = 4,
    ArmorCracked = 5,
    Disabled = 6,
    PilotInjured = 7,
    Eject = 8,
    BailTooLate = 9,
    Wreck = 10,
    Gibbed = 11,
}

impl ChassisStage {
    pub fn as_str(self) -> &'static str {
        match self {
            ChassisStage::Nominal => "nominal",
            ChassisStage::Degraded => "degraded",
            ChassisStage::ModuleWarning => "module_warning",
            ChassisStage::ModuleFailed => "module_failed",
            ChassisStage::WeaponJammed => "weapon_jammed",
            ChassisStage::ArmorCracked => "armor_cracked",
            ChassisStage::Disabled => "disabled",
            ChassisStage::PilotInjured => "pilot_injured",
            ChassisStage::Eject => "eject",
            ChassisStage::BailTooLate => "bail_too_late",
            ChassisStage::Wreck => "wreck",
            ChassisStage::Gibbed => "gibbed",
        }
    }

    /// True for the terminal stages where the chassis is no longer pilotable.
    pub fn is_terminal(self) -> bool {
        matches!(self, ChassisStage::Wreck | ChassisStage::Gibbed)
    }

    pub fn is_ejecting(self) -> bool {
        matches!(self, ChassisStage::Eject | ChassisStage::BailTooLate)
    }
}

/// Pilot state inside the chassis. Lifecycle:
/// `Bound → Injured? → Ejecting → Ejected → Extracted` (success path) or
/// `Bound → Ejecting? → BailedTooLate → Lost` (failure path) or
/// `Bound → ... → Lost` (gibbed without ejecting).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PilotState {
    /// Pilot is bound to the chassis (default while flying / walking the mech).
    Bound = 0,
    /// Pilot is injured but still bound (decel/jet capability reduced).
    Injured = 1,
    /// Pilot has triggered eject; eject sequence is mid-flight.
    Ejecting = 2,
    /// Pilot has ejected and is now controlled as foot infantry.
    Ejected = 3,
    /// Pilot has reached a safe extraction zone / objective.
    Extracted = 4,
    /// Pilot tried to eject too late; sequence failed (chassis already wrecked).
    BailedTooLate = 5,
    /// Pilot is lost (chassis wrecked + no eject OR bail-too-late).
    Lost = 6,
}

impl PilotState {
    pub fn as_str(self) -> &'static str {
        match self {
            PilotState::Bound => "bound",
            PilotState::Injured => "injured",
            PilotState::Ejecting => "ejecting",
            PilotState::Ejected => "ejected",
            PilotState::Extracted => "extracted",
            PilotState::BailedTooLate => "bailed_too_late",
            PilotState::Lost => "lost",
        }
    }

    pub fn is_in_chassis(self) -> bool {
        matches!(self, PilotState::Bound | PilotState::Injured)
    }

    pub fn is_lost(self) -> bool {
        matches!(self, PilotState::Lost | PilotState::BailedTooLate)
    }
}

/// One joint in the body graph. Joints connect zones and propagate physical
/// disruption (e.g., destroying the elbow joint between `arm_right` and the hand
/// means the hand can no longer grip). M5 ships a fixed set for the launch
/// chassis; M5.5 collision filters reference joint names for parent-linked
/// limb-collision events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Joint {
    pub id: String,
    pub parent: BodyZone,
    pub child: BodyZone,
    /// True iff this joint is intact. Destroying the parent zone severs the
    /// joint; the runtime updates this on `apply_zone_damage`.
    pub intact: bool,
}

/// Equipment socket on the body graph. Sockets are mount points where role-record
/// items are attached. Each socket is bound to a zone so dropping the zone drops
/// the gear.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquipmentSocket {
    pub id: String,
    pub zone: BodyZone,
    /// True iff a piece of equipment is currently mounted at the socket.
    pub occupied: bool,
    /// Role-record id of the mounted equipment, if any. M5 reuses
    /// `RIFLE_M1_DEFAULT_ID` for the canonical rifle socket; mods can add new.
    pub mounted_role: Option<String>,
}

/// Per-limb movement-contribution flags. When the zone is destroyed the flags say
/// what the actor loses (jet, jump, climb, two-handed grip, aim stability).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MovementContribution {
    pub zone: BodyZone,
    /// Multiplicative factor on movement speed if zone destroyed (1.0 = no impact).
    pub move_speed_factor_when_destroyed: f32,
    /// Multiplicative factor on jump impulse if zone destroyed.
    pub jump_impulse_factor_when_destroyed: f32,
    /// Whether destroying this zone disables the rifle (e.g., right arm gone).
    pub disables_rifle_when_destroyed: bool,
    /// Whether destroying this zone forces a crawl state.
    pub forces_crawl_when_destroyed: bool,
    /// Whether destroying this zone drops carried gear.
    pub drops_gear_when_destroyed: bool,
    /// Whether destroying this zone disables jet/jump (e.g., backpack housing the
    /// jet module, or legs gone).
    pub disables_jet_when_destroyed: bool,
}

impl MovementContribution {
    pub fn neutral(zone: BodyZone) -> Self {
        Self {
            zone,
            move_speed_factor_when_destroyed: 1.0,
            jump_impulse_factor_when_destroyed: 1.0,
            disables_rifle_when_destroyed: false,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: false,
            disables_jet_when_destroyed: false,
        }
    }
}

/// Body graph for a chassis. Lists every zone, joint, socket, and movement
/// contribution. The runtime walks this graph to resolve animation events and
/// damage consequences.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodyGraph {
    pub zones: Vec<BodyZone>,
    pub joints: Vec<Joint>,
    pub sockets: Vec<EquipmentSocket>,
    pub movement_contributions: Vec<MovementContribution>,
}

impl BodyGraph {
    pub fn movement_factor(&self, destroyed_zones: &[BodyZone]) -> (f32, f32, bool, bool, bool, bool) {
        // Returns (move_speed_factor, jump_factor, disable_rifle, force_crawl, drop_gear, disable_jet).
        let mut move_factor: f32 = 1.0;
        let mut jump_factor: f32 = 1.0;
        let mut disable_rifle = false;
        let mut force_crawl = false;
        let mut drop_gear = false;
        let mut disable_jet = false;
        for zone in destroyed_zones {
            if let Some(c) = self.movement_contributions.iter().find(|c| c.zone == *zone) {
                move_factor = move_factor.min(c.move_speed_factor_when_destroyed);
                jump_factor = jump_factor.min(c.jump_impulse_factor_when_destroyed);
                disable_rifle = disable_rifle || c.disables_rifle_when_destroyed;
                force_crawl = force_crawl || c.forces_crawl_when_destroyed;
                drop_gear = drop_gear || c.drops_gear_when_destroyed;
                disable_jet = disable_jet || c.disables_jet_when_destroyed;
            }
        }
        (
            move_factor,
            jump_factor,
            disable_rifle,
            force_crawl,
            drop_gear,
            disable_jet,
        )
    }
}

/// Eject window state. Eject is a multi-tick sequence: triggered at tick `T`, the
/// chassis spends `eject_ticks` blowing the canopy and clearing the pilot, then
/// transitions Ejected → Extracted when the pilot reaches a safe spot (engine
/// drives the extraction check).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EjectWindow {
    /// Ticks remaining in the active eject sequence. `0` = not ejecting OR completed.
    pub ticks_remaining: u32,
    /// Ticks the full eject takes once triggered. Defaults to 1 second at 60 Hz.
    pub ticks_total: u32,
    /// Tick at which the eject was triggered. Used by replay for event ordering.
    pub triggered_at_tick: u64,
}

impl Default for EjectWindow {
    fn default() -> Self {
        Self {
            ticks_remaining: 0,
            ticks_total: 60,
            triggered_at_tick: 0,
        }
    }
}

/// **M13** § "Armor mounting angles per chassis archetype". Per-zone mount
/// angles drive the M9 angled-armor math: incoming projectiles that strike
/// at a glancing angle effectively thicken the armor.
#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ArmorMountAngles {
    /// Forward-facing armor angle (degrees from vertical).
    pub front_degrees: f32,
    /// Lateral / side armor angle (degrees).
    pub side_degrees: f32,
    /// Rear armor angle (degrees).
    pub back_degrees: f32,
}

impl ArmorMountAngles {
    pub const fn new(front: f32, side: f32, back: f32) -> Self {
        Self {
            front_degrees: front,
            side_degrees: side,
            back_degrees: back,
        }
    }
}

/// **M13** § "Chassis ability slots — Time stop, Time slow, and 6 other launch
/// abilities". Eight launch abilities + per-chassis slot count.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChassisAbility {
    TimeStop = 0,
    TimeSlow = 1,
    ShieldBurst = 2,
    Overdrive = 3,
    RepairPulse = 4,
    Cloak = 5,
    EmpPulse = 6,
    GravityWell = 7,
}

impl ChassisAbility {
    pub fn as_str(self) -> &'static str {
        match self {
            ChassisAbility::TimeStop => "time_stop",
            ChassisAbility::TimeSlow => "time_slow",
            ChassisAbility::ShieldBurst => "shield_burst",
            ChassisAbility::Overdrive => "overdrive",
            ChassisAbility::RepairPulse => "repair_pulse",
            ChassisAbility::Cloak => "cloak",
            ChassisAbility::EmpPulse => "EMP_pulse",
            ChassisAbility::GravityWell => "gravity_well",
        }
    }

    pub fn parse(s: &str) -> Option<ChassisAbility> {
        match s {
            "time_stop" => Some(ChassisAbility::TimeStop),
            "time_slow" => Some(ChassisAbility::TimeSlow),
            "shield_burst" => Some(ChassisAbility::ShieldBurst),
            "overdrive" => Some(ChassisAbility::Overdrive),
            "repair_pulse" => Some(ChassisAbility::RepairPulse),
            "cloak" => Some(ChassisAbility::Cloak),
            "EMP_pulse" | "emp_pulse" => Some(ChassisAbility::EmpPulse),
            "gravity_well" => Some(ChassisAbility::GravityWell),
            _ => None,
        }
    }

    /// Per spec § "Chassis ability slots" table. (effect_seconds, cooldown_seconds).
    pub fn defaults(self) -> (f32, f32) {
        match self {
            ChassisAbility::TimeStop => (1.5, 30.0),
            ChassisAbility::TimeSlow => (5.0, 25.0),
            ChassisAbility::ShieldBurst => (8.0, 20.0),
            ChassisAbility::Overdrive => (6.0, 30.0),
            ChassisAbility::RepairPulse => (0.1, 45.0),
            ChassisAbility::Cloak => (5.0, 60.0),
            ChassisAbility::EmpPulse => (4.0, 40.0),
            ChassisAbility::GravityWell => (4.0, 50.0),
        }
    }

    /// Canonical iteration order for events + checksums.
    pub fn all() -> &'static [ChassisAbility] {
        &[
            ChassisAbility::TimeStop,
            ChassisAbility::TimeSlow,
            ChassisAbility::ShieldBurst,
            ChassisAbility::Overdrive,
            ChassisAbility::RepairPulse,
            ChassisAbility::Cloak,
            ChassisAbility::EmpPulse,
            ChassisAbility::GravityWell,
        ]
    }
}

/// **M13** § "Chassis ability slots" — one slot's runtime state.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AbilitySlotState {
    pub ability: ChassisAbility,
    /// Ticks remaining on the cooldown (0 = ready).
    pub cooldown_remaining_ticks: u32,
    /// Cooldown duration in ticks at the chassis tick rate.
    pub cooldown_total_ticks: u32,
    /// Ticks remaining in the active effect window (0 = effect ended).
    pub effect_remaining_ticks: u32,
    /// Effect duration in ticks at the chassis tick rate.
    pub effect_total_ticks: u32,
}

impl AbilitySlotState {
    pub fn new(ability: ChassisAbility, tick_rate_hz: u32) -> Self {
        let (effect_s, cooldown_s) = ability.defaults();
        let tr = tick_rate_hz.max(1) as f32;
        Self {
            ability,
            cooldown_remaining_ticks: 0,
            cooldown_total_ticks: (cooldown_s * tr).round() as u32,
            effect_remaining_ticks: 0,
            effect_total_ticks: (effect_s * tr).round() as u32,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.cooldown_remaining_ticks == 0 && self.effect_remaining_ticks == 0
    }

    pub fn is_active(&self) -> bool {
        self.effect_remaining_ticks > 0
    }
}

/// **M13** § "Chassis ability slots" — per-chassis active ability roster.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChassisAbilitySlots {
    /// Maximum ability slot count for this chassis (derived from kind).
    pub max_slots: u8,
    /// Active slot roster (length ≤ max_slots).
    pub slots: Vec<AbilitySlotState>,
}

impl ChassisAbilitySlots {
    pub fn new(kind: ChassisKind, tick_rate_hz: u32) -> Self {
        let max_slots = kind.ability_slot_count();
        // Default loadout per chassis kind. Light mech = 3 most-versatile slots;
        // powered armor = 2; infantry = 1; drone = 1; crab = 2.
        let default_loadout: &[ChassisAbility] = match kind {
            ChassisKind::Infantry => &[ChassisAbility::ShieldBurst],
            ChassisKind::PoweredArmor => &[ChassisAbility::Overdrive, ChassisAbility::ShieldBurst],
            ChassisKind::LightMech => &[ChassisAbility::TimeSlow, ChassisAbility::Overdrive, ChassisAbility::ShieldBurst],
            ChassisKind::CrabQuadruped => &[ChassisAbility::ShieldBurst, ChassisAbility::EmpPulse],
            ChassisKind::Drone => &[ChassisAbility::Cloak],
            ChassisKind::HeavyTrooper => &[
                ChassisAbility::ShieldBurst,
                ChassisAbility::Overdrive,
                ChassisAbility::EmpPulse,
            ],
        };
        let slots: Vec<AbilitySlotState> = default_loadout
            .iter()
            .take(max_slots as usize)
            .map(|a| AbilitySlotState::new(*a, tick_rate_hz))
            .collect();
        Self { max_slots, slots }
    }

    /// Find a slot by ability kind.
    pub fn find(&self, ability: ChassisAbility) -> Option<&AbilitySlotState> {
        self.slots.iter().find(|s| s.ability == ability)
    }

    pub fn find_mut(&mut self, ability: ChassisAbility) -> Option<&mut AbilitySlotState> {
        self.slots.iter_mut().find(|s| s.ability == ability)
    }

    /// Tick every slot's cooldown + effect timers. Returns the abilities whose
    /// effect window ended this tick (for `ability.effect_ended` events) and
    /// the abilities whose cooldown ended this tick (for `ability.cooldown_expired`).
    pub fn tick(&mut self) -> AbilityTickOutcome {
        let mut outcome = AbilityTickOutcome::default();
        for slot in &mut self.slots {
            if slot.effect_remaining_ticks > 0 {
                slot.effect_remaining_ticks -= 1;
                if slot.effect_remaining_ticks == 0 {
                    outcome.effects_ended.push(slot.ability);
                    // Effect-ended starts the cooldown.
                    slot.cooldown_remaining_ticks = slot.cooldown_total_ticks;
                }
            } else if slot.cooldown_remaining_ticks > 0 {
                slot.cooldown_remaining_ticks -= 1;
                if slot.cooldown_remaining_ticks == 0 {
                    outcome.cooldowns_expired.push(slot.ability);
                }
            }
        }
        outcome
    }

    /// Attempt to activate `ability`. Returns `Ok(slot_state)` on success or
    /// the typed reason on rejection.
    pub fn activate(&mut self, ability: ChassisAbility) -> Result<AbilitySlotState, AbilityRejectReason> {
        let slot = self.find_mut(ability).ok_or(AbilityRejectReason::NotEquipped)?;
        if slot.cooldown_remaining_ticks > 0 {
            return Err(AbilityRejectReason::OnCooldown);
        }
        if slot.effect_remaining_ticks > 0 {
            return Err(AbilityRejectReason::AlreadyActive);
        }
        slot.effect_remaining_ticks = slot.effect_total_ticks.max(1);
        Ok(*slot)
    }
}

/// Per-tick outcome of [`ChassisAbilitySlots::tick`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AbilityTickOutcome {
    pub effects_ended: Vec<ChassisAbility>,
    pub cooldowns_expired: Vec<ChassisAbility>,
}

/// Typed rejection reasons surfaced by [`ChassisAbilitySlots::activate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbilityRejectReason {
    NotEquipped,
    OnCooldown,
    AlreadyActive,
}

impl AbilityRejectReason {
    pub fn as_str(self) -> &'static str {
        match self {
            AbilityRejectReason::NotEquipped => "ability_not_equipped",
            AbilityRejectReason::OnCooldown => "ability_on_cooldown",
            AbilityRejectReason::AlreadyActive => "ability_already_active",
        }
    }
}

/// **M13** § "Drone allies — 4 modes + autonomous behavior".
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DroneMode {
    #[default]
    Follow = 0,
    AutoMine = 1,
    AutoRepair = 2,
    AutoCarry = 3,
}

impl DroneMode {
    pub fn as_str(self) -> &'static str {
        match self {
            DroneMode::Follow => "follow",
            DroneMode::AutoMine => "auto_mine",
            DroneMode::AutoRepair => "auto_repair",
            DroneMode::AutoCarry => "auto_carry",
        }
    }

    pub fn parse(s: &str) -> Option<DroneMode> {
        match s.to_ascii_lowercase().as_str() {
            "follow" => Some(DroneMode::Follow),
            "auto_mine" | "auto-mine" | "mine" => Some(DroneMode::AutoMine),
            "auto_repair" | "auto-repair" | "repair" => Some(DroneMode::AutoRepair),
            "auto_carry" | "auto-carry" | "carry" => Some(DroneMode::AutoCarry),
            _ => None,
        }
    }
}

/// **M13** § "Drone allies — Drone has limited fuel + battery (drains while
/// active; ~5 minutes per full charge)". Runtime drone ally state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DroneAllyState {
    pub mode: DroneMode,
    /// Fuel level 0..1. Drains roughly 1.0 / 300 s while active.
    pub fuel: f32,
    /// True after at least one `drone.task_completed` event has been emitted.
    #[serde(default)]
    pub task_completed: bool,
    /// True after the drone took terminal damage.
    #[serde(default)]
    pub destroyed: bool,
}

impl Default for DroneAllyState {
    fn default() -> Self {
        Self {
            mode: DroneMode::Follow,
            fuel: 1.0,
            task_completed: false,
            destroyed: false,
        }
    }
}

impl DroneAllyState {
    /// Drain fuel by one tick; returns `true` iff the drone just crossed the
    /// 0.2 low-fuel threshold (emit `drone.fuel_low` once).
    pub fn tick_fuel(&mut self, tick_rate_hz: u32) -> bool {
        if self.destroyed {
            return false;
        }
        let prev = self.fuel;
        // Full charge = 300s (~5 minutes); per-tick drain = 1.0 / (300 * tick_rate).
        let drain = 1.0 / (300.0 * tick_rate_hz.max(1) as f32);
        self.fuel = (self.fuel - drain).max(0.0);
        prev > 0.2 && self.fuel <= 0.2
    }
}

/// **M13** § "Weapon modifier slots (Noita-style combinatorial)" — 30+ launch
/// modifiers stackable on the same weapon. Discriminants are stable so the
/// modifier registry remains deterministic across milestones.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeaponModifier {
    Homing = 0,
    Explosive = 1,
    Freezing = 2,
    Electric = 3,
    Poisoning = 4,
    Bouncing = 5,
    Piercing = 6,
    Ricochet = 7,
    Bleed = 8,
    Stun = 9,
    FireChain = 10,
    DoubleTap = 11,
    TripleShot = 12,
    FastFire = 13,
    SlowFire = 14,
    SlowMotionOnKill = 15,
    SummonMinion = 16,
    GravityWell = 17,
    Vortex = 18,
    Magnet = 19,
    TimeSlowOnHit = 20,
    HealingBurst = 21,
    LifeSteal = 22,
    ManaBurst = 23,
    ShieldBreak = 24,
    ArmorPiercingRandom = 25,
    Knockback = 26,
    Weighted = 27,
    Magnetic = 28,
    ChainLightning = 29,
    FrostAura = 30,
}

impl WeaponModifier {
    pub fn as_str(self) -> &'static str {
        match self {
            WeaponModifier::Homing => "homing",
            WeaponModifier::Explosive => "explosive",
            WeaponModifier::Freezing => "freezing",
            WeaponModifier::Electric => "electric",
            WeaponModifier::Poisoning => "poisoning",
            WeaponModifier::Bouncing => "bouncing",
            WeaponModifier::Piercing => "piercing",
            WeaponModifier::Ricochet => "ricochet",
            WeaponModifier::Bleed => "bleed",
            WeaponModifier::Stun => "stun",
            WeaponModifier::FireChain => "fire_chain",
            WeaponModifier::DoubleTap => "double_tap",
            WeaponModifier::TripleShot => "triple_shot",
            WeaponModifier::FastFire => "fast_fire",
            WeaponModifier::SlowFire => "slow_fire",
            WeaponModifier::SlowMotionOnKill => "slow_motion_on_kill",
            WeaponModifier::SummonMinion => "summon_minion",
            WeaponModifier::GravityWell => "gravity_well",
            WeaponModifier::Vortex => "vortex",
            WeaponModifier::Magnet => "magnet",
            WeaponModifier::TimeSlowOnHit => "time_slow_on_hit",
            WeaponModifier::HealingBurst => "healing_burst",
            WeaponModifier::LifeSteal => "life_steal",
            WeaponModifier::ManaBurst => "mana_burst",
            WeaponModifier::ShieldBreak => "shield_break",
            WeaponModifier::ArmorPiercingRandom => "armor_piercing_random",
            WeaponModifier::Knockback => "knockback",
            WeaponModifier::Weighted => "weighted",
            WeaponModifier::Magnetic => "magnetic",
            WeaponModifier::ChainLightning => "chain_lightning",
            WeaponModifier::FrostAura => "frost_aura",
        }
    }

    pub fn parse(s: &str) -> Option<WeaponModifier> {
        for m in WeaponModifier::all() {
            if m.as_str() == s {
                return Some(*m);
            }
        }
        None
    }

    pub fn all() -> &'static [WeaponModifier] {
        &[
            WeaponModifier::Homing,
            WeaponModifier::Explosive,
            WeaponModifier::Freezing,
            WeaponModifier::Electric,
            WeaponModifier::Poisoning,
            WeaponModifier::Bouncing,
            WeaponModifier::Piercing,
            WeaponModifier::Ricochet,
            WeaponModifier::Bleed,
            WeaponModifier::Stun,
            WeaponModifier::FireChain,
            WeaponModifier::DoubleTap,
            WeaponModifier::TripleShot,
            WeaponModifier::FastFire,
            WeaponModifier::SlowFire,
            WeaponModifier::SlowMotionOnKill,
            WeaponModifier::SummonMinion,
            WeaponModifier::GravityWell,
            WeaponModifier::Vortex,
            WeaponModifier::Magnet,
            WeaponModifier::TimeSlowOnHit,
            WeaponModifier::HealingBurst,
            WeaponModifier::LifeSteal,
            WeaponModifier::ManaBurst,
            WeaponModifier::ShieldBreak,
            WeaponModifier::ArmorPiercingRandom,
            WeaponModifier::Knockback,
            WeaponModifier::Weighted,
            WeaponModifier::Magnetic,
            WeaponModifier::ChainLightning,
            WeaponModifier::FrostAura,
        ]
    }
}

/// **M13** § "Weapon modifier slots" — per-weapon modifier set bounded by the
/// chassis tier's slot count (see `ChassisKind::weapon_modifier_slot_count`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WeaponModifierSet {
    pub max_slots: u8,
    pub modifiers: Vec<WeaponModifier>,
}

impl WeaponModifierSet {
    pub fn new(kind: ChassisKind) -> Self {
        Self {
            max_slots: kind.weapon_modifier_slot_count(),
            modifiers: Vec::new(),
        }
    }

    pub fn attach(&mut self, m: WeaponModifier) -> Result<(), &'static str> {
        if self.modifiers.contains(&m) {
            return Err("modifier_already_attached");
        }
        if self.modifiers.len() as u8 >= self.max_slots {
            return Err("modifier_slots_full");
        }
        self.modifiers.push(m);
        Ok(())
    }

    pub fn detach(&mut self, m: WeaponModifier) -> bool {
        if let Some(idx) = self.modifiers.iter().position(|x| *x == m) {
            self.modifiers.remove(idx);
            true
        } else {
            false
        }
    }

    pub fn contains(&self, m: WeaponModifier) -> bool {
        self.modifiers.contains(&m)
    }

    pub fn is_combined(&self) -> bool {
        self.modifiers.len() >= 2
    }
}

/// **M13** § "Hit zone determination — how a 2D side-view projectile picks a
/// limb". Per-stance AABB tables that map (`local_x`, `local_y`) in normalized
/// actor-local space to a `BodyZone`. The lookup is fully deterministic — no
/// RNG, just AABB containment tests in spec-locked iteration order.
pub mod hit_zone {
    use super::BodyZone;

    /// Resolver result for a single hit.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct HitZoneResolution {
        pub zone: BodyZone,
        pub local_x: f32,
        pub local_y: f32,
    }

    /// Stance discriminator used for AABB-table lookup. Mirrors the M6
    /// `cf_actor::Stance` taxonomy for the four stances the spec tabulates.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum HitZoneStance {
        Standing,
        Crouching,
        Prone,
        Crawl,
    }

    /// One (zone, x_range, y_range) entry from the M13 spec.
    #[derive(Debug, Clone, Copy)]
    struct ZoneAabb {
        zone: BodyZone,
        x_min: f32,
        y_min: f32,
        x_max: f32,
        y_max: f32,
    }

    impl ZoneAabb {
        const fn new(zone: BodyZone, x_min: f32, x_max: f32, y_min: f32, y_max: f32) -> Self {
            Self {
                zone,
                x_min,
                y_min,
                x_max,
                y_max,
            }
        }

        fn contains(&self, x: f32, y: f32) -> bool {
            x >= self.x_min && x <= self.x_max && y >= self.y_min && y <= self.y_max
        }
    }

    // **STANDING** zone AABB table (per M13 spec § "STANDING:"). Order matters
    // — smaller / higher-priority zones come first so the first containment
    // hit wins. Local space convention: `local_x` ∈ [-0.5, +0.5] (negative =
    // near side after facing flip); `local_y` ∈ [0.0, 1.0] (0 = feet, 1 = head crown).
    const STANDING_TABLE: &[ZoneAabb] = &[
        ZoneAabb::new(BodyZone::Head, -0.15, 0.15, 0.85, 1.00),
        ZoneAabb::new(BodyZone::Backpack, 0.15, 0.30, 0.55, 0.85),
        ZoneAabb::new(BodyZone::HandLeft, -0.40, -0.25, 0.30, 0.50),
        ZoneAabb::new(BodyZone::HandRight, 0.25, 0.40, 0.30, 0.50),
        ZoneAabb::new(BodyZone::ForearmLeft, -0.40, -0.20, 0.35, 0.55),
        ZoneAabb::new(BodyZone::ForearmRight, 0.20, 0.40, 0.35, 0.55),
        ZoneAabb::new(BodyZone::ArmLeft, -0.40, -0.15, 0.45, 0.80),
        ZoneAabb::new(BodyZone::ArmRight, 0.15, 0.40, 0.45, 0.80),
        ZoneAabb::new(BodyZone::Torso, -0.30, 0.30, 0.45, 0.85),
        ZoneAabb::new(BodyZone::ShinLeft, -0.18, 0.0, 0.08, 0.15),
        ZoneAabb::new(BodyZone::ShinRight, 0.0, 0.18, 0.08, 0.15),
        ZoneAabb::new(BodyZone::FootLeft, -0.18, 0.0, 0.0, 0.08),
        ZoneAabb::new(BodyZone::FootRight, 0.0, 0.18, 0.0, 0.08),
        ZoneAabb::new(BodyZone::LegLeft, -0.20, 0.0, 0.10, 0.45),
        ZoneAabb::new(BodyZone::LegRight, 0.0, 0.20, 0.10, 0.45),
    ];

    const CROUCHING_TABLE: &[ZoneAabb] = &[
        ZoneAabb::new(BodyZone::Head, -0.15, 0.15, 0.70, 0.85),
        ZoneAabb::new(BodyZone::ArmRight, 0.15, 0.40, 0.50, 0.70),
        ZoneAabb::new(BodyZone::ArmLeft, -0.40, -0.15, 0.50, 0.70),
        ZoneAabb::new(BodyZone::Torso, -0.30, 0.30, 0.40, 0.70),
        ZoneAabb::new(BodyZone::FootLeft, -0.18, 0.0, 0.0, 0.10),
        ZoneAabb::new(BodyZone::FootRight, 0.0, 0.18, 0.0, 0.10),
        ZoneAabb::new(BodyZone::LegLeft, -0.20, 0.0, 0.10, 0.40),
        ZoneAabb::new(BodyZone::LegRight, 0.0, 0.20, 0.10, 0.40),
    ];

    const PRONE_TABLE: &[ZoneAabb] = &[
        ZoneAabb::new(BodyZone::Head, -0.40, -0.30, 0.05, 0.25),
        ZoneAabb::new(BodyZone::Backpack, -0.20, 0.20, 0.20, 0.30),
        ZoneAabb::new(BodyZone::Torso, -0.30, 0.30, 0.05, 0.30),
        ZoneAabb::new(BodyZone::ArmLeft, -0.30, 0.0, 0.10, 0.25),
        ZoneAabb::new(BodyZone::ArmRight, 0.0, 0.30, 0.10, 0.25),
        ZoneAabb::new(BodyZone::FootLeft, 0.30, 0.40, 0.0, 0.10),
        ZoneAabb::new(BodyZone::FootRight, 0.30, 0.40, 0.0, 0.10),
        ZoneAabb::new(BodyZone::LegLeft, 0.10, 0.40, 0.05, 0.25),
        ZoneAabb::new(BodyZone::LegRight, 0.10, 0.40, 0.05, 0.25),
    ];

    const CRAWL_TABLE: &[ZoneAabb] = &[
        ZoneAabb::new(BodyZone::Head, -0.40, -0.30, 0.05, 0.20),
        ZoneAabb::new(BodyZone::Torso, -0.30, 0.30, 0.05, 0.30),
        ZoneAabb::new(BodyZone::LegLeft, 0.10, 0.40, 0.05, 0.20),
        ZoneAabb::new(BodyZone::LegRight, 0.10, 0.40, 0.05, 0.20),
    ];

    fn table_for(stance: HitZoneStance) -> &'static [ZoneAabb] {
        match stance {
            HitZoneStance::Standing => STANDING_TABLE,
            HitZoneStance::Crouching => CROUCHING_TABLE,
            HitZoneStance::Prone => PRONE_TABLE,
            HitZoneStance::Crawl => CRAWL_TABLE,
        }
    }

    /// **M13** § "PROJECTILE-VS-ACTOR HIT DETECTION (deterministic; no RNG)".
    /// Resolves the body zone at the given local-space coordinate.
    /// `local_x` is post-facing-flip (positive = near side); `local_y` is
    /// normalized 0..1 from feet to crown.
    pub fn resolve(stance: HitZoneStance, local_x: f32, local_y: f32) -> Option<HitZoneResolution> {
        let table = table_for(stance);
        for entry in table {
            if entry.contains(local_x, local_y) {
                return Some(HitZoneResolution {
                    zone: entry.zone,
                    local_x,
                    local_y,
                });
            }
        }
        None
    }

    /// Spec § "Per-stance hit probability distributions (designer reference)".
    /// Tabulated expected distribution percentages — used by tests + AI
    /// hint surfaces; NOT used at runtime to bias the resolver (which is
    /// purely AABB-driven).
    pub fn expected_distribution_standing_horizontal() -> [(BodyZone, f32); 5] {
        [
            (BodyZone::Head, 0.12),
            (BodyZone::Torso, 0.50),
            (BodyZone::ArmRight, 0.15),
            (BodyZone::LegRight, 0.20),
            (BodyZone::FootRight, 0.03),
        ]
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn standing_head_zone_resolves() {
            let r = resolve(HitZoneStance::Standing, 0.0, 0.92).unwrap();
            assert_eq!(r.zone, BodyZone::Head);
        }

        #[test]
        fn standing_torso_zone_resolves_at_mid_height() {
            let r = resolve(HitZoneStance::Standing, 0.0, 0.65).unwrap();
            assert_eq!(r.zone, BodyZone::Torso);
        }

        #[test]
        fn standing_leg_resolves_at_low_height() {
            let r = resolve(HitZoneStance::Standing, -0.10, 0.25).unwrap();
            assert_eq!(r.zone, BodyZone::LegLeft);
        }

        #[test]
        fn crouching_head_is_lower_than_standing() {
            let r = resolve(HitZoneStance::Crouching, 0.0, 0.80).unwrap();
            assert_eq!(r.zone, BodyZone::Head);
            // A shot at standing-head height (0.92) would miss the crouching actor.
            assert!(resolve(HitZoneStance::Crouching, 0.0, 0.95).is_none());
        }

        #[test]
        fn prone_head_is_at_facing_front() {
            let r = resolve(HitZoneStance::Prone, -0.35, 0.10).unwrap();
            assert_eq!(r.zone, BodyZone::Head);
        }

        #[test]
        fn crawl_table_has_minimal_silhouette() {
            // Crawl skips arms — those points return None.
            let r = resolve(HitZoneStance::Crawl, 0.0, 0.50);
            assert!(r.is_none(), "crawl has flat profile; mid-height arms gap");
        }
    }
}

/// Chassis spec — the immutable design data for one archetype. Scenarios reference
/// these by id; the runtime clones to a [`ChassisState`] for each actor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChassisSpec {
    pub id: String,
    pub kind: ChassisKind,
    pub display_name: String,
    pub body_graph: BodyGraph,
    pub zones: Vec<ZoneState>,
    pub modules: Vec<ChassisModule>,
    /// Default eject window length (ticks). 60 ticks at 60 Hz = 1 second window.
    /// At 120 Hz this becomes 120 ticks, preserving real-time semantics.
    pub eject_window_seconds: f32,
    /// Tick rate this spec was instantiated with (used to size [`EjectWindow`]).
    /// Independent of the runtime tick rate; resolved on insertion.
    pub mass_kg: f32,
    /// **M13** § "Armor mounting angles per chassis archetype". Per-zone
    /// armor mount angles drive M9 angled-armor math.
    #[serde(default)]
    pub armor_angles: ArmorMountAngles,
}

/// Runtime mutable chassis state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChassisState {
    pub spec_id: String,
    pub kind: ChassisKind,
    pub stage: ChassisStage,
    pub pilot_state: PilotState,
    pub body_graph: BodyGraph,
    pub zones: Vec<ZoneState>,
    pub modules: Vec<ChassisModule>,
    pub eject_window: EjectWindow,
    /// Tick rate this chassis was instantiated with. Used to scale eject ticks
    /// from seconds — same chassis at 60 Hz vs 120 Hz produce identical real-time
    /// eject windows.
    pub tick_rate_hz: u32,
    /// Tutorial safety: lethal damage capped at Disabled / PilotInjured.
    pub tutorial_safety: bool,
    /// Mass of the chassis in kilograms (drives M5.5 impulse-to-damage routing).
    pub mass_kg: f32,
    /// True iff the weapon mounted at SOCKET_HAND_RIGHT is currently jammed.
    /// Distinct from module Failed because the rifle ITSELF jams (mechanism fault)
    /// rather than the chassis weapon-mount module failing.
    pub weapon_jammed: bool,
    /// Last reason for stage transition (for the next event emit). Cleared by
    /// the engine on read.
    pub last_stage_reason: String,
    /// Modules salvaged after wreck (populated by [`ChassisState::salvage`]).
    pub salvaged_modules: Vec<ChassisModule>,
    /// **M13** § "Armor mounting angles per chassis archetype".
    #[serde(default)]
    pub armor_angles: ArmorMountAngles,
    /// **M13** § "Chassis ability slots" — active ability roster.
    #[serde(default)]
    pub abilities: ChassisAbilitySlots,
    /// **M13** § "Weapon modifier slots (Noita-style combinatorial)".
    #[serde(default)]
    pub weapon_modifiers: WeaponModifierSet,
    /// **M13** § "Cockpit camera anchor" — current camera anchor request.
    #[serde(default)]
    pub camera_anchor: CameraAnchor,
    /// **M13** § "Boarding / disembarking transitions" — ticks remaining in
    /// the 1500ms boarding transition (0 = idle).
    #[serde(default)]
    pub boarding_ticks_remaining: u32,
    /// **M13** § "Boarding / disembarking transitions" — ticks remaining in
    /// the 1500ms disembarking transition.
    #[serde(default)]
    pub disembarking_ticks_remaining: u32,
    /// **M13** § "Boarding / disembarking transitions" — full transition
    /// budget in ticks at the chassis tick rate (1500ms).
    #[serde(default)]
    pub transition_ticks_total: u32,
}

impl ChassisState {
    /// Build a runtime state from a spec at the given tick rate.
    pub fn from_spec(spec: &ChassisSpec, tick_rate_hz: u32, tutorial_safety: bool) -> Self {
        let tick_rate = tick_rate_hz.max(1);
        let eject_ticks = ((spec.eject_window_seconds.max(0.1)) * tick_rate as f32).round() as u32;
        // 1500ms transition window per spec § "Boarding / disembarking transitions".
        let transition_ticks_total = ((1.5_f32) * tick_rate as f32).round() as u32;
        Self {
            spec_id: spec.id.clone(),
            kind: spec.kind,
            stage: ChassisStage::Nominal,
            pilot_state: PilotState::Bound,
            body_graph: spec.body_graph.clone(),
            zones: spec.zones.clone(),
            modules: spec.modules.clone(),
            eject_window: EjectWindow {
                ticks_remaining: 0,
                ticks_total: eject_ticks.max(1),
                triggered_at_tick: 0,
            },
            tick_rate_hz: tick_rate,
            tutorial_safety,
            mass_kg: spec.mass_kg,
            weapon_jammed: false,
            last_stage_reason: String::new(),
            salvaged_modules: Vec::new(),
            armor_angles: spec.armor_angles,
            abilities: ChassisAbilitySlots::new(spec.kind, tick_rate),
            weapon_modifiers: WeaponModifierSet::new(spec.kind),
            camera_anchor: CameraAnchor::Default,
            boarding_ticks_remaining: 0,
            disembarking_ticks_remaining: 0,
            transition_ticks_total: transition_ticks_total.max(1),
        }
    }

    /// **M13** § "Cockpit camera anchor" — switch the active camera anchor.
    /// Returns `Ok(prev_anchor)` on success or a typed reason on rejection.
    pub fn set_camera_anchor(&mut self, anchor: CameraAnchor) -> Result<CameraAnchor, &'static str> {
        if anchor == CameraAnchor::Cockpit && !self.kind.supports_cockpit_anchor() {
            return Err("camera_anchor_not_supported_by_chassis_class");
        }
        let prev = self.camera_anchor;
        self.camera_anchor = anchor;
        Ok(prev)
    }

    /// **M13** § "Boarding / disembarking transitions" — kick off the
    /// 1500ms boarding transition. Returns `true` when accepted (was idle).
    pub fn begin_boarding(&mut self) -> bool {
        if self.boarding_ticks_remaining > 0 || self.disembarking_ticks_remaining > 0 {
            return false;
        }
        self.boarding_ticks_remaining = self.transition_ticks_total;
        true
    }

    /// **M13** § "Boarding / disembarking transitions" — kick off the 1500ms
    /// disembarking transition.
    pub fn begin_disembarking(&mut self) -> bool {
        if self.boarding_ticks_remaining > 0 || self.disembarking_ticks_remaining > 0 {
            return false;
        }
        self.disembarking_ticks_remaining = self.transition_ticks_total;
        true
    }

    /// **M13** § "Boarding / disembarking transitions" — true iff the chassis
    /// is mid-transition (input rejected during).
    pub fn is_in_transition(&self) -> bool {
        self.boarding_ticks_remaining > 0 || self.disembarking_ticks_remaining > 0
    }

    /// **M13** § "Boarding / disembarking transitions" — tick the transition
    /// timers. Returns the side that just completed (if any).
    pub fn tick_transitions(&mut self) -> Option<TransitionCompleted> {
        if self.boarding_ticks_remaining > 0 {
            self.boarding_ticks_remaining -= 1;
            if self.boarding_ticks_remaining == 0 {
                return Some(TransitionCompleted::Boarded);
            }
        } else if self.disembarking_ticks_remaining > 0 {
            self.disembarking_ticks_remaining -= 1;
            if self.disembarking_ticks_remaining == 0 {
                return Some(TransitionCompleted::Disembarked);
            }
        }
        None
    }

    /// **M13** § "Chassis ability slots" — activate one ability slot.
    pub fn activate_ability(&mut self, ability: ChassisAbility) -> Result<AbilitySlotState, AbilityRejectReason> {
        self.abilities.activate(ability)
    }

    /// **M13** § "Chassis ability slots" — tick every slot's cooldown + effect.
    pub fn tick_abilities(&mut self) -> AbilityTickOutcome {
        self.abilities.tick()
    }

    /// **M13** § "Weapon modifier slots" — attach a modifier to the active weapon.
    pub fn attach_weapon_modifier(&mut self, m: WeaponModifier) -> Result<bool, &'static str> {
        let before_len = self.weapon_modifiers.modifiers.len();
        self.weapon_modifiers.attach(m)?;
        Ok(self.weapon_modifiers.modifiers.len() > before_len)
    }

    /// **M13** § "Weapon modifier slots" — detach a modifier.
    pub fn detach_weapon_modifier(&mut self, m: WeaponModifier) -> bool {
        self.weapon_modifiers.detach(m)
    }

    pub fn zone(&self, zone: BodyZone) -> Option<&ZoneState> {
        self.zones.iter().find(|z| z.zone == zone)
    }

    pub fn zone_mut(&mut self, zone: BodyZone) -> Option<&mut ZoneState> {
        self.zones.iter_mut().find(|z| z.zone == zone)
    }

    pub fn module(&self, id: &str) -> Option<&ChassisModule> {
        self.modules.iter().find(|m| m.id == id)
    }

    pub fn module_mut(&mut self, id: &str) -> Option<&mut ChassisModule> {
        self.modules.iter_mut().find(|m| m.id == id)
    }

    pub fn module_by_kind(&self, kind: ModuleKind) -> Option<&ChassisModule> {
        self.modules.iter().find(|m| m.kind == kind)
    }

    pub fn destroyed_zones(&self) -> Vec<BodyZone> {
        self.zones.iter().filter(|z| z.destroyed).map(|z| z.zone).collect()
    }

    /// Composite chassis integrity — averages every zone's zone_integrity. Drives
    /// HUD silhouette + stage transitions ("ArmorCracked" when avg drops below 0.6).
    pub fn integrity(&self) -> f32 {
        if self.zones.is_empty() {
            return 1.0;
        }
        let sum: f32 = self.zones.iter().map(ZoneState::zone_integrity).sum();
        sum / self.zones.len() as f32
    }

    /// Reset state to spec defaults (used by scenario.reset).
    pub fn reset(&mut self) {
        for zone in &mut self.zones {
            zone.reset();
        }
        for module in &mut self.modules {
            module.reset();
        }
        self.stage = ChassisStage::Nominal;
        self.pilot_state = PilotState::Bound;
        self.eject_window.ticks_remaining = 0;
        self.eject_window.triggered_at_tick = 0;
        self.weapon_jammed = false;
        self.last_stage_reason.clear();
        self.salvaged_modules.clear();
    }

    /// Apply damage to a specific zone with a typed cause label. Returns a
    /// [`ZoneDamageOutcome`] describing every layer/module transition the engine
    /// must emit as events.
    pub fn apply_zone_damage(&mut self, zone: BodyZone, damage: f32, cause: &str) -> ZoneDamageOutcome {
        let mut outcome = ZoneDamageOutcome::default();
        if damage <= 0.0 || !damage.is_finite() {
            return outcome;
        }
        outcome.zone = Some(zone);
        outcome.cause = cause.to_string();

        let mut remaining = damage;
        let mut layers_breached: Vec<(ArmorLayerKind, f32)> = Vec::new();
        let mut zone_destroyed = false;
        let mut wound_damage_taken = 0.0_f32;
        let mut wound_destroyed = false;

        if let Some(zs) = self.zone_mut(zone) {
            // Drain layers in canonical order.
            for kind in [ArmorLayerKind::External, ArmorLayerKind::Internal, ArmorLayerKind::Core] {
                if remaining <= 0.0 {
                    break;
                }
                let Some(layer) = zs.layers.iter_mut().find(|l| l.kind == kind) else {
                    continue;
                };
                if layer.hp <= 0.0 {
                    continue;
                }
                let effective = (remaining - layer.hardness).max(0.0);
                if effective <= 0.0 {
                    // Hardness absorbed the hit; record a glance event.
                    outcome.glances.push(LayerGlance {
                        layer: kind,
                        absorbed: remaining,
                    });
                    remaining = 0.0;
                    break;
                }
                let taken = effective.min(layer.hp);
                layer.hp -= taken;
                outcome.layer_damage.push(LayerDamage {
                    layer: kind,
                    damage: taken,
                    hp_after: layer.hp,
                    breached: layer.is_breached(),
                });
                if layer.is_breached() {
                    layers_breached.push((kind, layer.hp_max));
                }
                remaining -= taken;
            }
            // Spill into wound HP if all layers breached.
            if remaining > 0.0 {
                let wound_take = remaining.min(zs.wound_hp);
                zs.wound_hp -= wound_take;
                wound_damage_taken = wound_take;
                remaining -= wound_take;
                if zs.wound_hp <= 0.0 && !zs.destroyed {
                    zs.destroyed = true;
                    zone_destroyed = true;
                    wound_destroyed = true;
                }
            }
        }
        outcome.layers_breached = layers_breached;
        outcome.wound_damage = wound_damage_taken;
        outcome.zone_destroyed = zone_destroyed;
        let _ = wound_destroyed; // tracked for future routing into actor HP coefficient
        outcome.actor_hp_damage = remaining.max(0.0);
        // **M13** § "Limb loss functional consequences" — head/torso loss is
        // INSTANT DEATH per CCCP decapitation rule. Tutorial-safety overrides
        // (`tutorial_safety=true` caps damage at PilotInjured) suppress lethal.
        if zone_destroyed && !self.tutorial_safety && matches!(zone, BodyZone::Head | BodyZone::Torso) {
            outcome.lethal = true;
        }

        // Propagate to module health bound to this zone.
        if zone_destroyed {
            let modules_to_update: Vec<(String, ModuleStateKind, String)> = self
                .modules
                .iter_mut()
                .filter(|m| m.bound_zone == zone && m.state != ModuleStateKind::NotPresent)
                .map(|m| {
                    m.hp = 0.0;
                    m.state = ModuleStateKind::Failed;
                    m.last_reason = "bound_zone_destroyed".to_string();
                    (m.id.clone(), ModuleStateKind::Failed, m.last_reason.clone())
                })
                .collect();
            outcome.module_transitions.extend(
                modules_to_update
                    .into_iter()
                    .map(|(id, state, reason)| ModuleTransition { id, state, reason }),
            );
            // Sever joints connected to this zone.
            for joint in &mut self.body_graph.joints {
                if (joint.parent == zone || joint.child == zone) && joint.intact {
                    joint.intact = false;
                    outcome.joints_severed.push(joint.id.clone());
                }
            }
        } else {
            // Non-destroying damage to a zone with low integrity also degrades modules
            // bound to it (e.g., torso cracked → jet warning).
            let integrity = self.zone(zone).map_or(1.0, ZoneState::zone_integrity);
            let new_state = stage_from_integrity(integrity);
            if new_state != ModuleStateKind::Nominal && new_state != ModuleStateKind::NotPresent {
                let updates: Vec<(String, ModuleStateKind, String)> = self
                    .modules
                    .iter_mut()
                    .filter(|m| m.bound_zone == zone && m.state.is_present())
                    .filter_map(|m| {
                        if (new_state as u8) > (m.state as u8) {
                            m.state = new_state;
                            m.last_reason = "bound_zone_damaged".to_string();
                            // Drain module HP proportional to its bound zone.
                            m.hp = (m.hp_max * integrity).clamp(0.0, m.hp_max);
                            Some((m.id.clone(), new_state, m.last_reason.clone()))
                        } else {
                            None
                        }
                    })
                    .collect();
                outcome
                    .module_transitions
                    .extend(
                        updates
                            .into_iter()
                            .map(|(id, state, reason)| ModuleTransition { id, state, reason }),
                    );
            }
        }

        outcome
    }

    /// Apply damage directly to a module (e.g., direct hit on the jet module).
    pub fn apply_module_damage(&mut self, module_id: &str, damage: f32, cause: &str) -> Option<ModuleTransition> {
        if damage <= 0.0 || !damage.is_finite() {
            return None;
        }
        let module = self.module_mut(module_id)?;
        if !module.state.is_present() {
            return None;
        }
        module.hp = (module.hp - damage).max(0.0);
        let new_state = stage_from_integrity(module.integrity());
        if new_state == module.state && new_state != ModuleStateKind::Nominal {
            return None;
        }
        if (new_state as u8) > (module.state as u8) {
            module.state = new_state;
            module.last_reason = cause.to_string();
            Some(ModuleTransition {
                id: module.id.clone(),
                state: new_state,
                reason: module.last_reason.clone(),
            })
        } else {
            None
        }
    }

    /// **M13** § "Critical chassis modules with full mechanics" — apply
    /// damage to a module and surface its cascade outcome (ammo cookoff,
    /// engine fire, optics blind, etc.). The engine wires this into
    /// `module.ammo_rack_cooking` / `module.ammo_rack_detonated` /
    /// `module.spalling_damage` event emitters.
    pub fn apply_critical_module_damage(
        &mut self,
        module_id: &str,
        damage: f32,
        cause: &str,
    ) -> Option<CriticalModuleOutcome> {
        let transition = self.apply_module_damage(module_id, damage, cause);
        let module = self.module(module_id)?;
        let mut cascade_events: Vec<CriticalModuleEvent> = Vec::new();
        let module_id = module.id.clone();
        let module_kind = module.kind;
        let module_state = module.state;
        let cascade = module.failure_cascade;
        let ammo_remaining = module.ammo_quantity_remaining;
        // **M14 audit pass 4 (Finding 8)**: tier-crossing cascades fire
        // ONLY when the module's `state` has advanced past its previous
        // `last_cascade_emitted_state` — otherwise multiple zone hits in a
        // single tick (or any other rapid succession of calls while
        // already inside Warning) would re-cook ammo, re-leak oil,
        // re-advance pressure, etc. once per call. PilotDirectHit is
        // intentionally per-hit (damage-amount-bearing event) and gates
        // on damage > 0 instead.
        let last_emitted = module
            .last_cascade_emitted_state;
        let tier_advanced = (module_state as u8) > (last_emitted as u8);
        // Per spec § "Ammo rack module (explosive cascade)" — first-hit cooks
        // 1/3 of remaining ammo; severe-hit (Failed state) detonates the rack.
        if cascade == FailureCascade::AmmoCookoff && tier_advanced {
            match module_state {
                ModuleStateKind::Warning if ammo_remaining > 0 => {
                    let cook = ammo_remaining / 3;
                    // borrow again to mutate counters
                    if let Some(m) = self.module_mut(&module_id) {
                        m.rounds_cooked_off = m.rounds_cooked_off.saturating_add(cook);
                        m.ammo_quantity_remaining = m.ammo_quantity_remaining.saturating_sub(cook);
                    }
                    cascade_events.push(CriticalModuleEvent::AmmoCooking { rounds_cooked: cook });
                }
                ModuleStateKind::Failed => {
                    let detonated = ammo_remaining;
                    if let Some(m) = self.module_mut(&module_id) {
                        m.rounds_cooked_off = m.rounds_cooked_off.saturating_add(detonated);
                        m.ammo_quantity_remaining = 0;
                    }
                    cascade_events.push(CriticalModuleEvent::AmmoDetonated {
                        rounds_detonated: detonated,
                    });
                    // Catastrophic — flag chassis as gibbed unless tutorial-safe.
                    if !self.tutorial_safety {
                        self.stage = ChassisStage::Gibbed;
                        self.pilot_state = PilotState::Lost;
                        self.last_stage_reason = "ammo_rack_detonated".to_string();
                    }
                }
                _ => {}
            }
        }
        // Per spec § "Engine module (fire risk)" — penetrated engine spills
        // oil; destroyed engine cascades fire. Tier-gated.
        if cascade == FailureCascade::EngineFire && tier_advanced {
            if matches!(module_state, ModuleStateKind::Warning | ModuleStateKind::Failed) {
                if let Some(m) = self.module_mut(&module_id) {
                    m.oil_level = (m.oil_level - 0.5).max(0.0);
                }
                cascade_events.push(CriticalModuleEvent::EngineOilLeak);
            }
            if module_state == ModuleStateKind::Failed {
                cascade_events.push(CriticalModuleEvent::EngineFire);
            }
        }
        // Per spec § "Reactor module" — pressure_state advances per damage tier.
        // Reactor pressure has its own internal crossed flag below (it only
        // emits when `pressure` > prior `pressure_state`); leave that as the
        // authoritative dedupe for reactors.
        if cascade == FailureCascade::ReactorOverpressure {
            let pressure = match module_state {
                ModuleStateKind::Degraded => 1,
                ModuleStateKind::Warning => 2,
                ModuleStateKind::Failed => 4,
                ModuleStateKind::Nominal | ModuleStateKind::NotPresent => 0,
            };
            let mut crossed = false;
            if let Some(m) = self.module_mut(&module_id) {
                if pressure > m.pressure_state {
                    m.pressure_state = pressure;
                    crossed = true;
                }
            }
            if crossed {
                cascade_events.push(CriticalModuleEvent::ReactorPressureAdvanced { tier: pressure });
            }
        }
        // Per spec § "Cockpit module" — penetration deals direct damage to pilot.
        // Per-hit cascade (carries damage payload); gates on damage>0 instead
        // of tier_advanced so multiple hits all surface their damage.
        if cascade == FailureCascade::PilotDirectDamage
            && damage > 0.0
            && matches!(module_state, ModuleStateKind::Warning | ModuleStateKind::Failed)
        {
            cascade_events.push(CriticalModuleEvent::PilotDirectHit { damage });
            // Promote pilot to Injured when cockpit takes damage.
            if matches!(self.pilot_state, PilotState::Bound) {
                self.pilot_state = PilotState::Injured;
            }
        }
        // Per spec § "Optics module" — damaged → sight × 0.5; destroyed → blind.
        if cascade == FailureCascade::SightImpairment
            && tier_advanced
            && matches!(module_state, ModuleStateKind::Warning | ModuleStateKind::Failed)
        {
            cascade_events.push(CriticalModuleEvent::OpticsImpaired {
                blind: module_state == ModuleStateKind::Failed,
            });
        }
        // Per spec § "Transmission module" — damaged → speed × 0.6; destroyed → immobile.
        if cascade == FailureCascade::MobilityLoss
            && tier_advanced
            && matches!(module_state, ModuleStateKind::Warning | ModuleStateKind::Failed)
        {
            cascade_events.push(CriticalModuleEvent::MobilityReduced {
                immobile: module_state == ModuleStateKind::Failed,
            });
        }
        // **M14 audit pass 4 (Finding 8)**: latch the high-water mark so
        // subsequent same-state calls don't refire the tier-gated cascades.
        if tier_advanced {
            if let Some(m) = self.module_mut(&module_id) {
                m.last_cascade_emitted_state = module_state;
            }
        }
        if transition.is_none() && cascade_events.is_empty() {
            return None;
        }
        Some(CriticalModuleOutcome {
            module_id,
            module_kind,
            transition,
            cascade_events,
        })
    }

    /// **M13** § "Spalling integration with chassis modules" — given an
    /// impact point in chassis-local space, fire 1-3 deterministic spalling
    /// fragments into the chassis and report each fragment's module hit.
    /// `seed` is the caller-supplied deterministic PRNG seed (NO thread_rng).
    pub fn spawn_spalling_fragments(
        &mut self,
        impact_local: (f32, f32),
        fragment_count: u32,
        original_damage: f32,
        seed: u64,
    ) -> Vec<SpallingFragmentOutcome> {
        let mut outcomes: Vec<SpallingFragmentOutcome> = Vec::new();
        let count = fragment_count.clamp(1, 3);
        for i in 0..count {
            // Deterministic fragment direction within ±30° cone (per spec).
            let frag_seed = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(i as u64);
            let angle_norm = ((frag_seed % 1024) as f32 / 1024.0) - 0.5; // -0.5..+0.5
            let angle_rad = angle_norm * std::f32::consts::PI * (60.0 / 180.0);
            let dx = angle_rad.cos();
            let dy = angle_rad.sin();
            // Per spec: per-fragment damage = 20-50% of original.
            let damage_frac = 0.2 + ((frag_seed >> 10) % 30) as f32 / 100.0;
            let damage = original_damage * damage_frac;
            // Pick the first module whose local_aabb is on the ray. Stub
            // walks the module list and returns the first positioned hit.
            let target_id: Option<String> = self
                .modules
                .iter()
                .find(|m| m.state.is_present() && m.local_aabb.is_positioned())
                .map(|m| m.id.clone());
            let _ = (dx, dy, impact_local);
            if let Some(id) = target_id {
                let transition = self.apply_module_damage(&id, damage, "spalling_fragment");
                outcomes.push(SpallingFragmentOutcome {
                    fragment_id: format!("frag_{i}"),
                    module_id: id,
                    damage,
                    transition,
                });
            }
        }
        outcomes
    }

    /// Stage transition pass — call once per tick (or right after damage application).
    /// Updates `self.stage` based on aggregate damage, module health, and pilot state.
    /// Returns `Some(new_stage)` iff the stage advanced.
    pub fn recompute_stage(&mut self) -> Option<ChassisStage> {
        let prev = self.stage;
        let mut next = prev;

        // Composite cues.
        let core_integrity_min = self.zones.iter().map(ZoneState::core_integrity).fold(1.0_f32, f32::min);
        let any_zone_destroyed = self.zones.iter().any(|z| z.destroyed);
        let any_module_failed = self
            .modules
            .iter()
            .any(|m| m.state == ModuleStateKind::Failed && m.kind != ModuleKind::WeaponMount);
        let any_module_warning = self.modules.iter().any(|m| m.state == ModuleStateKind::Warning);
        let weapon_mount_failed = self
            .module_by_kind(ModuleKind::WeaponMount)
            .is_some_and(|m| m.state == ModuleStateKind::Failed);
        let armor_cracked = self.integrity() <= 0.5;
        let disabled = core_integrity_min <= 0.0 && any_zone_destroyed;
        let pilot_injured = matches!(self.pilot_state, PilotState::Injured);
        let pilot_ejected = matches!(self.pilot_state, PilotState::Ejected | PilotState::Extracted);
        let pilot_lost = self.pilot_state.is_lost();
        let chassis_wrecked = self.zone(BodyZone::Torso).is_some_and(|z| z.destroyed) && disabled;

        // Advance stage by precedence (last-wins for "more severe"). Never step
        // backwards except via explicit repair.
        if prev <= ChassisStage::Nominal && self.integrity() < 1.0 {
            next = ChassisStage::Degraded;
        }
        if any_module_warning && next < ChassisStage::ModuleWarning {
            next = ChassisStage::ModuleWarning;
        }
        if any_module_failed && next < ChassisStage::ModuleFailed {
            next = ChassisStage::ModuleFailed;
        }
        if (self.weapon_jammed || weapon_mount_failed) && next < ChassisStage::WeaponJammed {
            next = ChassisStage::WeaponJammed;
        }
        if armor_cracked && next < ChassisStage::ArmorCracked {
            next = ChassisStage::ArmorCracked;
        }
        if disabled && next < ChassisStage::Disabled {
            next = ChassisStage::Disabled;
        }
        if pilot_injured && next < ChassisStage::PilotInjured {
            next = ChassisStage::PilotInjured;
        }
        if matches!(self.pilot_state, PilotState::Ejecting) && next < ChassisStage::Eject {
            next = ChassisStage::Eject;
        }
        if matches!(self.pilot_state, PilotState::BailedTooLate) && next < ChassisStage::BailTooLate {
            next = ChassisStage::BailTooLate;
        }
        // Wreck stage requires either disable + ejected_or_lost OR torso destroyed.
        if ((disabled && (pilot_ejected || pilot_lost)) || chassis_wrecked) && next < ChassisStage::Wreck {
            next = ChassisStage::Wreck;
        }
        // Gibbed is reserved for explicit catastrophic damage flagged via
        // [`ChassisState::mark_gibbed`].

        // Tutorial-safety floor: never advance beyond PilotInjured.
        if self.tutorial_safety && next > ChassisStage::PilotInjured {
            next = ChassisStage::PilotInjured;
        }
        if next != prev {
            self.stage = next;
            Some(next)
        } else {
            None
        }
    }

    /// Mark this chassis as gibbed (catastrophic explosion). Used by M5.6+ reactions.
    pub fn mark_gibbed(&mut self, reason: &str) {
        if !self.tutorial_safety {
            self.stage = ChassisStage::Gibbed;
            self.pilot_state = PilotState::Lost;
            self.last_stage_reason = reason.to_string();
        }
    }

    /// Trigger an eject sequence. Returns `Some(EjectAccepted { ticks_total })` if
    /// the chassis accepted the eject; `None` if the pilot is already ejected/lost
    /// or the chassis stage forbids it.
    pub fn attempt_eject(&mut self, tick: u64) -> Option<EjectAccepted> {
        // Cannot eject if already out of the chassis.
        if !self.pilot_state.is_in_chassis() {
            return None;
        }
        // Tutorial safety blocks "real" eject; it returns a no-op extracted instead.
        if self.tutorial_safety {
            self.pilot_state = PilotState::Extracted;
            self.eject_window.triggered_at_tick = tick;
            self.eject_window.ticks_remaining = 0;
            return Some(EjectAccepted {
                ticks_total: 0,
                tutorial_extract: true,
            });
        }
        self.pilot_state = PilotState::Ejecting;
        self.eject_window.triggered_at_tick = tick;
        self.eject_window.ticks_remaining = self.eject_window.ticks_total;
        self.last_stage_reason = "pilot_ejected".to_string();
        Some(EjectAccepted {
            ticks_total: self.eject_window.ticks_total,
            tutorial_extract: false,
        })
    }

    /// Tick the eject sequence. Returns `Some(EjectProgress)` when the sequence
    /// transitions (started→ejected, ejected→bail-too-late) so the engine emits
    /// events.
    pub fn tick_eject(&mut self) -> Option<EjectProgress> {
        if !matches!(self.pilot_state, PilotState::Ejecting) {
            return None;
        }
        if self.eject_window.ticks_remaining > 0 {
            self.eject_window.ticks_remaining -= 1;
        }
        if self.eject_window.ticks_remaining == 0 {
            // If the chassis is already wrecked / gibbed before the sequence
            // completed, the pilot bailed too late.
            if matches!(self.stage, ChassisStage::Wreck | ChassisStage::Gibbed) {
                self.pilot_state = PilotState::BailedTooLate;
                return Some(EjectProgress::BailedTooLate);
            }
            self.pilot_state = PilotState::Ejected;
            return Some(EjectProgress::Ejected);
        }
        None
    }

    /// Mark the pilot as extracted (reached safety zone).
    pub fn mark_pilot_extracted(&mut self) -> bool {
        if matches!(self.pilot_state, PilotState::Ejected) {
            self.pilot_state = PilotState::Extracted;
            true
        } else {
            false
        }
    }

    /// Mark the pilot as lost (chassis exploded with pilot inside).
    pub fn mark_pilot_lost(&mut self, reason: &str) -> bool {
        if !self.pilot_state.is_lost() {
            self.pilot_state = PilotState::Lost;
            self.last_stage_reason = reason.to_string();
            true
        } else {
            false
        }
    }

    /// Repair a zone (heal all its layers + wound back to spec). Stage may step
    /// back at most one level.
    pub fn repair_zone(&mut self, zone: BodyZone, reason: &str) -> Option<RepairOutcome> {
        let zs = self.zone_mut(zone)?;
        let was_destroyed = zs.destroyed;
        zs.reset();
        // Resurrect modules whose bound zone is the repaired one — they go back to
        // Nominal with full HP.
        let restored: Vec<String> = self
            .modules
            .iter_mut()
            .filter(|m| m.bound_zone == zone && m.state.is_present())
            .filter_map(|m| {
                let prev = m.state;
                m.hp = m.hp_max;
                m.state = ModuleStateKind::Nominal;
                // **M14 audit pass 4 (Finding 8)**: reset cascade
                // emission high-water mark so re-damage re-fires its
                // tier-crossing cascades.
                m.last_cascade_emitted_state = ModuleStateKind::Nominal;
                m.pressure_state = 0;
                m.oil_level = 1.0;
                m.coolant_level = 1.0;
                m.last_reason = format!("repaired_via:{reason}");
                if prev != ModuleStateKind::Nominal {
                    Some(m.id.clone())
                } else {
                    None
                }
            })
            .collect();
        // Joint mend.
        for joint in &mut self.body_graph.joints {
            if joint.parent == zone || joint.child == zone {
                joint.intact = true;
            }
        }
        // Step stage back one slot if any progress.
        let prev_stage = self.stage;
        self.stage = match self.stage {
            ChassisStage::Degraded => ChassisStage::Nominal,
            ChassisStage::ModuleWarning => ChassisStage::Degraded,
            ChassisStage::ModuleFailed => ChassisStage::ModuleWarning,
            ChassisStage::WeaponJammed => ChassisStage::ModuleFailed,
            ChassisStage::ArmorCracked => ChassisStage::WeaponJammed,
            ChassisStage::Disabled => ChassisStage::ArmorCracked,
            ChassisStage::PilotInjured => ChassisStage::Disabled,
            other => other,
        };
        Some(RepairOutcome {
            zone,
            was_destroyed,
            modules_restored: restored,
            prev_stage,
            new_stage: self.stage,
            reason: reason.to_string(),
        })
    }

    /// Repair a specific module (e.g., field-deployed repair drone).
    pub fn repair_module(&mut self, module_id: &str, reason: &str) -> Option<ModuleTransition> {
        let module = self.module_mut(module_id)?;
        if !module.state.is_present() {
            return None;
        }
        let prev = module.state;
        module.hp = module.hp_max;
        module.state = ModuleStateKind::Nominal;
        // **M14 audit pass 4 (Finding 8)**: reset the cascade-emitted
        // high-water mark on repair so future damage that re-crosses a
        // tier emits its cascade event again.
        module.last_cascade_emitted_state = ModuleStateKind::Nominal;
        // Repair also restores reactor pressure_state + oil/coolant so
        // the cascade pipeline resumes at full nominal reserves.
        module.pressure_state = 0;
        module.oil_level = 1.0;
        module.coolant_level = 1.0;
        module.last_reason = format!("repaired:{reason}");
        if prev != ModuleStateKind::Nominal {
            Some(ModuleTransition {
                id: module.id.clone(),
                state: ModuleStateKind::Nominal,
                reason: module.last_reason.clone(),
            })
        } else {
            None
        }
    }

    /// **M5 scenario manifest** path: set the stage directly. Used by
    /// `ScenarioChassis::initial_stage` so a scenario can spawn a chassis
    /// already in `Wreck` / `Disabled` for salvage proof. Does NOT recompute
    /// from zone/module integrity — callers that need integrity-driven stage
    /// should use [`Self::recompute_stage`] instead.
    pub fn force_stage(&mut self, stage: ChassisStage) {
        self.stage = stage;
        self.last_stage_reason = format!("scenario_force:{stage:?}");
    }

    /// Salvage a wrecked chassis: pull every non-Failed module into
    /// `salvaged_modules` and emit a [`SalvageOutcome`]. Returns `None` if the
    /// chassis is not wreck-stage.
    pub fn salvage(&mut self, reason: &str) -> Option<SalvageOutcome> {
        if !matches!(
            self.stage,
            ChassisStage::Wreck | ChassisStage::Disabled | ChassisStage::Gibbed
        ) {
            return None;
        }
        let mut salvaged_ids: Vec<String> = Vec::new();
        for module in &mut self.modules {
            if !module.state.is_present() {
                continue;
            }
            // Modules below 25% integrity are too broken to salvage.
            if module.integrity() < 0.25 {
                continue;
            }
            module.last_reason = format!("salvaged:{reason}");
            self.salvaged_modules.push(module.clone());
            salvaged_ids.push(module.id.clone());
        }
        // Move the chassis into Wreck if it wasn't already.
        if self.stage != ChassisStage::Gibbed {
            self.stage = ChassisStage::Wreck;
        }
        Some(SalvageOutcome {
            salvaged_module_ids: salvaged_ids,
            reason: reason.to_string(),
        })
    }

    /// Mark the rifle as jammed. Distinct from the weapon-mount module's `Failed`
    /// state (which is structural). A jam clears on `clear_jam`.
    pub fn jam_weapon(&mut self, reason: &str) -> bool {
        if self.weapon_jammed {
            return false;
        }
        self.weapon_jammed = true;
        self.last_stage_reason = format!("weapon_jammed:{reason}");
        true
    }

    pub fn clear_jam(&mut self) -> bool {
        if !self.weapon_jammed {
            return false;
        }
        self.weapon_jammed = false;
        true
    }

    /// Hash bytes for the deterministic checksum extension. Layout-stable.
    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(256);
        out.push(self.kind as u8);
        out.push(self.stage as u8);
        out.push(self.pilot_state as u8);
        out.push(u8::from(self.weapon_jammed));
        out.extend_from_slice(&self.eject_window.ticks_remaining.to_le_bytes());
        for zone in BodyZone::all() {
            let z = self
                .zones
                .iter()
                .find(|z| z.zone == *zone)
                .cloned()
                .unwrap_or_else(|| ZoneState::new(*zone, Vec::new(), 0.0));
            for layer in [ArmorLayerKind::External, ArmorLayerKind::Internal, ArmorLayerKind::Core] {
                let l = z
                    .layers
                    .iter()
                    .find(|l| l.kind == layer)
                    .cloned()
                    .unwrap_or(ArmorLayer {
                        kind: layer,
                        hp: 0.0,
                        hp_max: 0.0,
                        hardness: 0.0,
                    });
                out.extend_from_slice(&(l.hp * 1024.0).round().to_bits().to_le_bytes());
            }
            out.extend_from_slice(&(z.wound_hp * 1024.0).round().to_bits().to_le_bytes());
            out.push(u8::from(z.destroyed));
        }
        let mut module_ids: Vec<&ChassisModule> = self.modules.iter().collect();
        module_ids.sort_by(|a, b| a.id.cmp(&b.id));
        out.extend_from_slice(&(module_ids.len() as u32).to_le_bytes());
        for m in module_ids {
            out.push(m.state as u8);
            out.extend_from_slice(&(m.hp * 1024.0).round().to_bits().to_le_bytes());
        }
        out
    }
}

/// **M13** § "Critical chassis modules with full mechanics" — typed cascade
/// event surfaced by [`ChassisState::apply_critical_module_damage`].
#[derive(Debug, Clone, PartialEq)]
pub enum CriticalModuleEvent {
    AmmoCooking { rounds_cooked: u32 },
    AmmoDetonated { rounds_detonated: u32 },
    EngineOilLeak,
    EngineFire,
    ReactorPressureAdvanced { tier: u8 },
    PilotDirectHit { damage: f32 },
    OpticsImpaired { blind: bool },
    MobilityReduced { immobile: bool },
}

impl CriticalModuleEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            CriticalModuleEvent::AmmoCooking { .. } => "ammo_rack_cooking",
            CriticalModuleEvent::AmmoDetonated { .. } => "ammo_rack_detonated",
            CriticalModuleEvent::EngineOilLeak => "engine_oil_leak",
            CriticalModuleEvent::EngineFire => "engine_fire",
            CriticalModuleEvent::ReactorPressureAdvanced { .. } => "reactor_pressure_advanced",
            CriticalModuleEvent::PilotDirectHit { .. } => "pilot_direct_hit",
            CriticalModuleEvent::OpticsImpaired { .. } => "optics_impaired",
            CriticalModuleEvent::MobilityReduced { .. } => "mobility_reduced",
        }
    }
}

/// Aggregate outcome from [`ChassisState::apply_critical_module_damage`].
#[derive(Debug, Clone, PartialEq)]
pub struct CriticalModuleOutcome {
    pub module_id: String,
    pub module_kind: ModuleKind,
    pub transition: Option<ModuleTransition>,
    pub cascade_events: Vec<CriticalModuleEvent>,
}

/// **M13** § "Spalling integration with chassis modules" — per-fragment outcome.
#[derive(Debug, Clone, PartialEq)]
pub struct SpallingFragmentOutcome {
    pub fragment_id: String,
    pub module_id: String,
    pub damage: f32,
    pub transition: Option<ModuleTransition>,
}

/// **M13** § "Boarding / disembarking transitions" — which side of the
/// transition completed this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransitionCompleted {
    Boarded,
    Disembarked,
}

impl TransitionCompleted {
    pub fn as_str(self) -> &'static str {
        match self {
            TransitionCompleted::Boarded => "boarded",
            TransitionCompleted::Disembarked => "disembarked",
        }
    }
}

/// **M13** § "Hit reactions per body part (per CCCP MOSRotating::CollideAtPoint)".
/// Tabulated per-zone reaction (kind label + duration in seconds + concussion dose).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HitReaction {
    pub kind: &'static str,
    pub duration_seconds: f32,
    pub concussion_dose: u32,
    pub drop_chance: f32,
    pub speed_factor: f32,
}

impl HitReaction {
    pub const fn new(kind: &'static str, duration_seconds: f32) -> Self {
        Self {
            kind,
            duration_seconds,
            concussion_dose: 0,
            drop_chance: 0.0,
            speed_factor: 1.0,
        }
    }

    /// Hit reactions per body zone (per spec § "Hit reactions per body part" table).
    pub fn for_zone(zone: BodyZone) -> Self {
        match zone {
            BodyZone::Head => HitReaction {
                kind: "stagger_stun",
                duration_seconds: 0.5,
                concussion_dose: 15,
                drop_chance: 0.0,
                speed_factor: 1.0,
            },
            BodyZone::Torso => HitReaction {
                kind: "knockback",
                duration_seconds: 0.8,
                concussion_dose: 0,
                drop_chance: 0.0,
                speed_factor: 1.0,
            },
            BodyZone::ArmLeft | BodyZone::ArmRight => HitReaction {
                kind: "reduced_grip",
                duration_seconds: 1.2,
                concussion_dose: 0,
                drop_chance: 0.0,
                speed_factor: 1.0,
            },
            BodyZone::ForearmLeft | BodyZone::ForearmRight => HitReaction {
                kind: "grip_penalty",
                duration_seconds: 0.8,
                concussion_dose: 0,
                drop_chance: 0.10,
                speed_factor: 1.0,
            },
            BodyZone::HandLeft | BodyZone::HandRight => HitReaction {
                kind: "drop_weapon",
                duration_seconds: 0.6,
                concussion_dose: 0,
                drop_chance: 0.40,
                speed_factor: 1.0,
            },
            BodyZone::LegLeft | BodyZone::LegRight => HitReaction {
                kind: "limp",
                duration_seconds: 2.0,
                concussion_dose: 0,
                drop_chance: 0.0,
                speed_factor: 0.7,
            },
            BodyZone::ShinLeft | BodyZone::ShinRight => HitReaction {
                kind: "brief_limp",
                duration_seconds: 0.8,
                concussion_dose: 0,
                drop_chance: 0.0,
                speed_factor: 0.85,
            },
            BodyZone::FootLeft | BodyZone::FootRight => HitReaction {
                kind: "minimal",
                duration_seconds: 0.2,
                concussion_dose: 0,
                drop_chance: 0.0,
                speed_factor: 1.0,
            },
            BodyZone::Backpack => HitReaction {
                kind: "module_damage",
                duration_seconds: 0.4,
                concussion_dose: 0,
                drop_chance: 0.0,
                speed_factor: 1.0,
            },
            // Quadruped + drone zones — generic reactions; M14+ tunes.
            BodyZone::LegFrontLeft
            | BodyZone::LegFrontRight
            | BodyZone::LegRearLeft
            | BodyZone::LegRearRight => HitReaction {
                kind: "limp",
                duration_seconds: 1.5,
                concussion_dose: 0,
                drop_chance: 0.0,
                speed_factor: 0.7,
            },
            BodyZone::ClawLeft | BodyZone::ClawRight => HitReaction {
                kind: "grip_penalty",
                duration_seconds: 0.8,
                concussion_dose: 0,
                drop_chance: 0.30,
                speed_factor: 1.0,
            },
            BodyZone::Carapace | BodyZone::SensorCluster => HitReaction {
                kind: "knockback",
                duration_seconds: 0.6,
                concussion_dose: 0,
                drop_chance: 0.0,
                speed_factor: 1.0,
            },
            BodyZone::DroneCore => HitReaction {
                kind: "destabilize",
                duration_seconds: 0.5,
                concussion_dose: 0,
                drop_chance: 0.0,
                speed_factor: 0.6,
            },
            BodyZone::DroneArmLeft | BodyZone::DroneArmRight | BodyZone::DroneSensorPod => HitReaction {
                kind: "minimal",
                duration_seconds: 0.3,
                concussion_dose: 0,
                drop_chance: 0.0,
                speed_factor: 1.0,
            },
        }
    }

    /// Duration in ticks at the actor's tick rate.
    pub fn duration_ticks(self, tick_rate_hz: u32) -> u32 {
        (self.duration_seconds * tick_rate_hz.max(1) as f32).round() as u32
    }
}

/// Outcome of [`ChassisState::attempt_eject`].
#[derive(Debug, Clone, PartialEq)]
pub struct EjectAccepted {
    pub ticks_total: u32,
    /// True when the eject was demoted to an instant tutorial-extract.
    pub tutorial_extract: bool,
}

/// Tick-level eject progress signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EjectProgress {
    /// The pilot has separated from the chassis and is now foot infantry.
    Ejected,
    /// The eject window expired while the chassis was already wrecked.
    BailedTooLate,
}

/// Outcome of [`ChassisState::salvage`].
#[derive(Debug, Clone, PartialEq)]
pub struct SalvageOutcome {
    pub salvaged_module_ids: Vec<String>,
    pub reason: String,
}

/// Outcome of [`ChassisState::repair_zone`].
#[derive(Debug, Clone, PartialEq)]
pub struct RepairOutcome {
    pub zone: BodyZone,
    pub was_destroyed: bool,
    pub modules_restored: Vec<String>,
    pub prev_stage: ChassisStage,
    pub new_stage: ChassisStage,
    pub reason: String,
}

/// Damage routed to one armor layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerDamage {
    pub layer: ArmorLayerKind,
    pub damage: f32,
    pub hp_after: f32,
    pub breached: bool,
}

/// Layer "glance" — the layer's hardness fully absorbed the hit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerGlance {
    pub layer: ArmorLayerKind,
    pub absorbed: f32,
}

/// Module transition recorded in [`ZoneDamageOutcome`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleTransition {
    pub id: String,
    pub state: ModuleStateKind,
    pub reason: String,
}

/// Aggregate outcome of [`ChassisState::apply_zone_damage`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ZoneDamageOutcome {
    pub zone: Option<BodyZone>,
    pub cause: String,
    pub layer_damage: Vec<LayerDamage>,
    pub layers_breached: Vec<(ArmorLayerKind, f32)>,
    pub glances: Vec<LayerGlance>,
    pub wound_damage: f32,
    pub zone_destroyed: bool,
    pub module_transitions: Vec<ModuleTransition>,
    pub joints_severed: Vec<String>,
    pub actor_hp_damage: f32,
    /// **M13** § "Limb loss functional consequences" — head/torso loss is
    /// INSTANT DEATH (per CCCP decapitation rule). True iff the destroyed
    /// zone is `Head` or `Torso` and the chassis is NOT tutorial-safe.
    /// Engine consumers should set actor.hp = 0 immediately.
    #[serde(default)]
    pub lethal: bool,
}

impl ZoneDamageOutcome {
    pub fn any_event(&self) -> bool {
        !self.layer_damage.is_empty()
            || !self.layers_breached.is_empty()
            || !self.module_transitions.is_empty()
            || !self.joints_severed.is_empty()
            || !self.glances.is_empty()
            || self.zone_destroyed
            || self.wound_damage > 0.0
            || self.actor_hp_damage > 0.0
            || self.lethal
    }
}

/// Map an integrity 0..1 to a module state.
fn stage_from_integrity(integrity: f32) -> ModuleStateKind {
    if integrity <= 0.0 {
        ModuleStateKind::Failed
    } else if integrity <= 0.25 {
        ModuleStateKind::Warning
    } else if integrity <= 0.6 {
        ModuleStateKind::Degraded
    } else {
        ModuleStateKind::Nominal
    }
}

// ---------------------- reference chassis builders ----------------------

/// Build the canonical Infantry body graph (no chassis, just the body).
fn infantry_body_graph() -> BodyGraph {
    // **M13** preserves the M5 15-zone humanoid contract — quadruped + drone
    // zones do NOT appear in the humanoid body graph (they belong to the
    // crab_body_graph / drone_body_graph functions instead).
    let zones: Vec<BodyZone> = BodyZone::all()
        .iter()
        .filter(|z| !z.is_quadruped_zone() && !z.is_drone_zone())
        .copied()
        .collect();
    let joints = vec![
        Joint {
            id: "neck".to_string(),
            parent: BodyZone::Torso,
            child: BodyZone::Head,
            intact: true,
        },
        Joint {
            id: "shoulder_left".to_string(),
            parent: BodyZone::Torso,
            child: BodyZone::ArmLeft,
            intact: true,
        },
        Joint {
            id: "shoulder_right".to_string(),
            parent: BodyZone::Torso,
            child: BodyZone::ArmRight,
            intact: true,
        },
        Joint {
            id: "elbow_left".to_string(),
            parent: BodyZone::ArmLeft,
            child: BodyZone::ForearmLeft,
            intact: true,
        },
        Joint {
            id: "elbow_right".to_string(),
            parent: BodyZone::ArmRight,
            child: BodyZone::ForearmRight,
            intact: true,
        },
        Joint {
            id: "wrist_left".to_string(),
            parent: BodyZone::ForearmLeft,
            child: BodyZone::HandLeft,
            intact: true,
        },
        Joint {
            id: "wrist_right".to_string(),
            parent: BodyZone::ForearmRight,
            child: BodyZone::HandRight,
            intact: true,
        },
        Joint {
            id: "hip_left".to_string(),
            parent: BodyZone::Torso,
            child: BodyZone::LegLeft,
            intact: true,
        },
        Joint {
            id: "hip_right".to_string(),
            parent: BodyZone::Torso,
            child: BodyZone::LegRight,
            intact: true,
        },
        Joint {
            id: "knee_left".to_string(),
            parent: BodyZone::LegLeft,
            child: BodyZone::ShinLeft,
            intact: true,
        },
        Joint {
            id: "knee_right".to_string(),
            parent: BodyZone::LegRight,
            child: BodyZone::ShinRight,
            intact: true,
        },
        Joint {
            id: "ankle_left".to_string(),
            parent: BodyZone::ShinLeft,
            child: BodyZone::FootLeft,
            intact: true,
        },
        Joint {
            id: "ankle_right".to_string(),
            parent: BodyZone::ShinRight,
            child: BodyZone::FootRight,
            intact: true,
        },
        Joint {
            id: "back_mount".to_string(),
            parent: BodyZone::Torso,
            child: BodyZone::Backpack,
            intact: true,
        },
    ];
    let sockets = vec![
        EquipmentSocket {
            id: SOCKET_HAND_RIGHT.to_string(),
            // Hand socket is on the granular Hand zone (M5 spec) so dropping
            // the hand drops the rifle.
            zone: BodyZone::HandRight,
            occupied: true,
            mounted_role: Some(cf_equipment::RIFLE_M1_DEFAULT_ID.to_string()),
        },
        EquipmentSocket {
            id: SOCKET_HAND_LEFT.to_string(),
            zone: BodyZone::HandLeft,
            occupied: false,
            mounted_role: None,
        },
        EquipmentSocket {
            id: SOCKET_BACK_MOUNT.to_string(),
            zone: BodyZone::Backpack,
            occupied: false,
            mounted_role: None,
        },
        EquipmentSocket {
            id: SOCKET_HEAD_MOUNT.to_string(),
            zone: BodyZone::Head,
            occupied: false,
            mounted_role: None,
        },
        EquipmentSocket {
            id: SOCKET_TORSO_HARDPOINT.to_string(),
            zone: BodyZone::Torso,
            occupied: false,
            mounted_role: None,
        },
    ];
    let movement_contributions = vec![
        MovementContribution {
            zone: BodyZone::Head,
            move_speed_factor_when_destroyed: 0.0,
            jump_impulse_factor_when_destroyed: 0.0,
            disables_rifle_when_destroyed: true,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: true,
            disables_jet_when_destroyed: true,
        },
        MovementContribution {
            zone: BodyZone::Torso,
            move_speed_factor_when_destroyed: 0.0,
            jump_impulse_factor_when_destroyed: 0.0,
            disables_rifle_when_destroyed: true,
            forces_crawl_when_destroyed: true,
            drops_gear_when_destroyed: true,
            disables_jet_when_destroyed: true,
        },
        MovementContribution {
            zone: BodyZone::ArmRight,
            move_speed_factor_when_destroyed: 0.95,
            jump_impulse_factor_when_destroyed: 0.95,
            disables_rifle_when_destroyed: true,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: true,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::ArmLeft,
            move_speed_factor_when_destroyed: 0.95,
            jump_impulse_factor_when_destroyed: 0.95,
            disables_rifle_when_destroyed: false,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: false,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::LegRight,
            move_speed_factor_when_destroyed: 0.5,
            jump_impulse_factor_when_destroyed: 0.4,
            disables_rifle_when_destroyed: false,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: false,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::LegLeft,
            move_speed_factor_when_destroyed: 0.5,
            jump_impulse_factor_when_destroyed: 0.4,
            disables_rifle_when_destroyed: false,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: false,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::Backpack,
            move_speed_factor_when_destroyed: 1.0,
            jump_impulse_factor_when_destroyed: 1.0,
            disables_rifle_when_destroyed: false,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: false,
            disables_jet_when_destroyed: true,
        },
        // Granular forearm/hand consequences: destroying the right hand drops
        // the rifle entirely; destroying the right forearm reduces aim
        // stability + disables fine-control rifle handling.
        MovementContribution {
            zone: BodyZone::ForearmRight,
            move_speed_factor_when_destroyed: 0.95,
            jump_impulse_factor_when_destroyed: 0.95,
            disables_rifle_when_destroyed: true,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: true,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::HandRight,
            move_speed_factor_when_destroyed: 0.95,
            jump_impulse_factor_when_destroyed: 0.95,
            disables_rifle_when_destroyed: true,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: true,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::ForearmLeft,
            move_speed_factor_when_destroyed: 0.95,
            jump_impulse_factor_when_destroyed: 0.95,
            disables_rifle_when_destroyed: false,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: false,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::HandLeft,
            move_speed_factor_when_destroyed: 0.95,
            jump_impulse_factor_when_destroyed: 0.95,
            disables_rifle_when_destroyed: false,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: false,
            disables_jet_when_destroyed: false,
        },
        // Granular leg consequences: destroying a shin or foot cripples
        // movement on that side (limp); losing both feet forces a crawl.
        MovementContribution {
            zone: BodyZone::ShinRight,
            move_speed_factor_when_destroyed: 0.4,
            jump_impulse_factor_when_destroyed: 0.3,
            disables_rifle_when_destroyed: false,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: false,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::ShinLeft,
            move_speed_factor_when_destroyed: 0.4,
            jump_impulse_factor_when_destroyed: 0.3,
            disables_rifle_when_destroyed: false,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: false,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::FootRight,
            move_speed_factor_when_destroyed: 0.6,
            jump_impulse_factor_when_destroyed: 0.5,
            disables_rifle_when_destroyed: false,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: false,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::FootLeft,
            move_speed_factor_when_destroyed: 0.6,
            jump_impulse_factor_when_destroyed: 0.5,
            disables_rifle_when_destroyed: false,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: false,
            disables_jet_when_destroyed: false,
        },
    ];
    BodyGraph {
        zones,
        joints,
        sockets,
        movement_contributions,
    }
}

fn make_zone(
    zone: BodyZone,
    external_hp: f32,
    external_hardness: f32,
    internal_hp: f32,
    internal_hardness: f32,
    core_hp: f32,
    wound_hp: f32,
) -> ZoneState {
    let layers = vec![
        ArmorLayer::new(ArmorLayerKind::External, external_hp, external_hardness),
        ArmorLayer::new(ArmorLayerKind::Internal, internal_hp, internal_hardness),
        ArmorLayer::new(ArmorLayerKind::Core, core_hp, 0.0),
    ];
    ZoneState::new(zone, layers, wound_hp)
}

/// Build the canonical Infantry chassis spec — minimal armor, just a body.
pub fn infantry_spec() -> ChassisSpec {
    let zones = vec![
        make_zone(BodyZone::Head, 4.0, 2.0, 4.0, 0.0, 6.0, 12.0),
        make_zone(BodyZone::Torso, 8.0, 2.0, 6.0, 0.0, 12.0, 30.0),
        make_zone(BodyZone::ArmRight, 4.0, 1.0, 4.0, 0.0, 6.0, 12.0),
        make_zone(BodyZone::ArmLeft, 4.0, 1.0, 4.0, 0.0, 6.0, 12.0),
        make_zone(BodyZone::LegRight, 5.0, 1.0, 5.0, 0.0, 8.0, 16.0),
        make_zone(BodyZone::LegLeft, 5.0, 1.0, 5.0, 0.0, 8.0, 16.0),
        make_zone(BodyZone::Backpack, 0.0, 0.0, 0.0, 0.0, 0.0, 4.0),
        make_zone(BodyZone::ForearmRight, 3.0, 1.0, 3.0, 0.0, 4.0, 8.0),
        make_zone(BodyZone::ForearmLeft, 3.0, 1.0, 3.0, 0.0, 4.0, 8.0),
        make_zone(BodyZone::HandRight, 2.0, 0.0, 2.0, 0.0, 3.0, 6.0),
        make_zone(BodyZone::HandLeft, 2.0, 0.0, 2.0, 0.0, 3.0, 6.0),
        make_zone(BodyZone::ShinRight, 4.0, 1.0, 4.0, 0.0, 5.0, 10.0),
        make_zone(BodyZone::ShinLeft, 4.0, 1.0, 4.0, 0.0, 5.0, 10.0),
        make_zone(BodyZone::FootRight, 3.0, 0.0, 3.0, 0.0, 4.0, 8.0),
        make_zone(BodyZone::FootLeft, 3.0, 0.0, 3.0, 0.0, 4.0, 8.0),
    ];
    let modules = vec![
        ChassisModule::new("weapon_mount.rifle", ModuleKind::WeaponMount, BodyZone::ArmRight, 30.0),
        ChassisModule::not_present("jet.none", ModuleKind::Jet),
        ChassisModule::not_present("shield.none", ModuleKind::Shield),
        ChassisModule::not_present("sensor.none", ModuleKind::Sensor),
        ChassisModule::not_present("repair_drone.none", ModuleKind::RepairDrone),
    ];
    ChassisSpec {
        id: INFANTRY_ID.to_string(),
        kind: ChassisKind::Infantry,
        display_name: "Infantry (Foot)".to_string(),
        body_graph: infantry_body_graph(),
        zones,
        modules,
        eject_window_seconds: 0.0,
        mass_kg: 90.0,
        // Infantry has no armor mount slope (per spec table: 0° / 0° / 0°).
        armor_angles: ArmorMountAngles::new(0.0, 0.0, 0.0),
    }
}

/// Build the canonical Powered Armor chassis spec — Spartan-ish, jet pack,
/// shield generator, full body armor.
pub fn powered_armor_spec() -> ChassisSpec {
    let zones = vec![
        make_zone(BodyZone::Head, 30.0, 6.0, 18.0, 3.0, 24.0, 12.0),
        make_zone(BodyZone::Torso, 80.0, 8.0, 50.0, 4.0, 60.0, 30.0),
        make_zone(BodyZone::ArmRight, 36.0, 5.0, 24.0, 2.0, 30.0, 12.0),
        make_zone(BodyZone::ArmLeft, 36.0, 5.0, 24.0, 2.0, 30.0, 12.0),
        make_zone(BodyZone::LegRight, 40.0, 5.0, 24.0, 2.0, 36.0, 16.0),
        make_zone(BodyZone::LegLeft, 40.0, 5.0, 24.0, 2.0, 36.0, 16.0),
        make_zone(BodyZone::Backpack, 30.0, 4.0, 20.0, 2.0, 18.0, 4.0),
        make_zone(BodyZone::ForearmRight, 24.0, 4.0, 16.0, 2.0, 20.0, 8.0),
        make_zone(BodyZone::ForearmLeft, 24.0, 4.0, 16.0, 2.0, 20.0, 8.0),
        make_zone(BodyZone::HandRight, 18.0, 3.0, 12.0, 1.0, 14.0, 6.0),
        make_zone(BodyZone::HandLeft, 18.0, 3.0, 12.0, 1.0, 14.0, 6.0),
        make_zone(BodyZone::ShinRight, 28.0, 4.0, 18.0, 2.0, 22.0, 10.0),
        make_zone(BodyZone::ShinLeft, 28.0, 4.0, 18.0, 2.0, 22.0, 10.0),
        make_zone(BodyZone::FootRight, 20.0, 3.0, 14.0, 1.0, 16.0, 8.0),
        make_zone(BodyZone::FootLeft, 20.0, 3.0, 14.0, 1.0, 16.0, 8.0),
    ];
    let modules = vec![
        ChassisModule::new("weapon_mount.rifle", ModuleKind::WeaponMount, BodyZone::ArmRight, 60.0),
        ChassisModule::new("jet.pack", ModuleKind::Jet, BodyZone::Backpack, 40.0),
        ChassisModule::new("shield.bubble", ModuleKind::Shield, BodyZone::Torso, 50.0),
        ChassisModule::new("sensor.scope", ModuleKind::Sensor, BodyZone::Head, 25.0),
        ChassisModule::not_present("repair_drone.none", ModuleKind::RepairDrone),
        // **M13** § "Per-chassis module positions" — Powered Armor: +
        // power_core (torso center), targeting_computer (head),
        // gun_mount (arm), shield_emitter (chest).
        ChassisModule::new("power_core.cell", ModuleKind::PowerCore, BodyZone::Torso, 50.0)
            .with_local_aabb(Aabb::new(-3.0, 4.0, 3.0, 12.0)),
        ChassisModule::new("targeting_computer.optics", ModuleKind::Optics, BodyZone::Head, 25.0)
            .with_local_aabb(Aabb::new(-2.0, 16.0, 2.0, 20.0))
            .with_failure_cascade(FailureCascade::SightImpairment),
        ChassisModule::new("targeting_computer.cpu", ModuleKind::TargetingComputer, BodyZone::Head, 20.0)
            .with_local_aabb(Aabb::new(-1.5, 14.0, 1.5, 17.0)),
    ];
    ChassisSpec {
        id: POWERED_ARMOR_ID.to_string(),
        kind: ChassisKind::PoweredArmor,
        display_name: "Powered Armor MK-I".to_string(),
        body_graph: infantry_body_graph(),
        zones,
        modules,
        eject_window_seconds: 1.0,
        mass_kg: 350.0,
        // Powered Armor (Spartan-ish): 15° front slope, 0° side, 15° back slope.
        armor_angles: ArmorMountAngles::new(15.0, 0.0, 15.0),
    }
}

/// Build the canonical Light Mech chassis spec — ~3x human, heavier armor,
/// repair drone, jet pack.
pub fn light_mech_spec() -> ChassisSpec {
    let zones = vec![
        make_zone(BodyZone::Head, 60.0, 10.0, 30.0, 5.0, 36.0, 12.0),
        make_zone(BodyZone::Torso, 180.0, 12.0, 100.0, 6.0, 120.0, 30.0),
        make_zone(BodyZone::ArmRight, 80.0, 8.0, 50.0, 4.0, 60.0, 12.0),
        make_zone(BodyZone::ArmLeft, 80.0, 8.0, 50.0, 4.0, 60.0, 12.0),
        make_zone(BodyZone::LegRight, 100.0, 8.0, 60.0, 4.0, 80.0, 16.0),
        make_zone(BodyZone::LegLeft, 100.0, 8.0, 60.0, 4.0, 80.0, 16.0),
        make_zone(BodyZone::Backpack, 60.0, 6.0, 40.0, 3.0, 30.0, 4.0),
        make_zone(BodyZone::ForearmRight, 50.0, 6.0, 30.0, 3.0, 36.0, 8.0),
        make_zone(BodyZone::ForearmLeft, 50.0, 6.0, 30.0, 3.0, 36.0, 8.0),
        make_zone(BodyZone::HandRight, 30.0, 4.0, 20.0, 2.0, 24.0, 6.0),
        make_zone(BodyZone::HandLeft, 30.0, 4.0, 20.0, 2.0, 24.0, 6.0),
        make_zone(BodyZone::ShinRight, 70.0, 6.0, 40.0, 3.0, 54.0, 10.0),
        make_zone(BodyZone::ShinLeft, 70.0, 6.0, 40.0, 3.0, 54.0, 10.0),
        make_zone(BodyZone::FootRight, 40.0, 4.0, 28.0, 2.0, 32.0, 8.0),
        make_zone(BodyZone::FootLeft, 40.0, 4.0, 28.0, 2.0, 32.0, 8.0),
    ];
    let modules = vec![
        ChassisModule::new("weapon_mount.rifle", ModuleKind::WeaponMount, BodyZone::ArmRight, 100.0),
        ChassisModule::new("jet.heavy", ModuleKind::Jet, BodyZone::Backpack, 80.0),
        ChassisModule::new("shield.heavy", ModuleKind::Shield, BodyZone::Torso, 100.0),
        ChassisModule::new("sensor.array", ModuleKind::Sensor, BodyZone::Head, 50.0),
        ChassisModule::new("repair_drone.bay", ModuleKind::RepairDrone, BodyZone::Torso, 50.0),
        // **M13** § "Per-chassis module positions" — Light Mech (cockpit + frame):
        // cockpit (top-front), reactor (torso center), fuel_tank (torso back),
        // ammo_rack (torso side; explosive), engine (torso back; fire-risk),
        // transmission (between torso + leg), motor_controller_per_leg,
        // optics_pod (head), comm_relay (head), targeting_computer (chest).
        ChassisModule::new("cockpit.main", ModuleKind::Cockpit, BodyZone::Head, 120.0)
            .with_local_aabb(Aabb::new(-4.0, 28.0, 4.0, 36.0))
            .with_failure_cascade(FailureCascade::PilotDirectDamage),
        ChassisModule::new("reactor.core", ModuleKind::Reactor, BodyZone::Torso, 180.0)
            .with_local_aabb(Aabb::new(-6.0, 8.0, 6.0, 20.0))
            .with_failure_cascade(FailureCascade::ReactorOverpressure),
        ChassisModule::new("fuel_tank.rear", ModuleKind::FuelTank, BodyZone::Torso, 80.0)
            .with_local_aabb(Aabb::new(2.0, 4.0, 8.0, 16.0)),
        ChassisModule::new("ammo_rack.main", ModuleKind::AmmoRack, BodyZone::Torso, 60.0)
            .with_local_aabb(Aabb::new(-8.0, 6.0, -3.0, 14.0))
            .with_ammo(15),
        ChassisModule::new("engine.main", ModuleKind::Engine, BodyZone::Torso, 100.0)
            .with_local_aabb(Aabb::new(0.0, 2.0, 6.0, 10.0))
            .with_failure_cascade(FailureCascade::EngineFire),
        ChassisModule::new("transmission.main", ModuleKind::Transmission, BodyZone::Torso, 60.0)
            .with_local_aabb(Aabb::new(-3.0, -2.0, 3.0, 4.0))
            .with_failure_cascade(FailureCascade::MobilityLoss),
        ChassisModule::new("motor_controller.left", ModuleKind::MotorController, BodyZone::LegLeft, 40.0)
            .with_failure_cascade(FailureCascade::MobilityLoss),
        ChassisModule::new("motor_controller.right", ModuleKind::MotorController, BodyZone::LegRight, 40.0)
            .with_failure_cascade(FailureCascade::MobilityLoss),
        ChassisModule::new("optics_pod.main", ModuleKind::Optics, BodyZone::Head, 35.0)
            .with_local_aabb(Aabb::new(-2.5, 30.0, 2.5, 34.0))
            .with_failure_cascade(FailureCascade::SightImpairment),
        ChassisModule::new("comm_relay.head", ModuleKind::CommRelay, BodyZone::Head, 25.0)
            .with_local_aabb(Aabb::new(-2.0, 33.0, 2.0, 36.0)),
        ChassisModule::new("targeting_computer.chest", ModuleKind::TargetingComputer, BodyZone::Torso, 35.0)
            .with_local_aabb(Aabb::new(-3.0, 18.0, 3.0, 22.0)),
    ];
    ChassisSpec {
        id: LIGHT_MECH_ID.to_string(),
        kind: ChassisKind::LightMech,
        display_name: "Light Mech LM-1".to_string(),
        body_graph: infantry_body_graph(),
        zones,
        modules,
        eject_window_seconds: 1.5,
        mass_kg: 1800.0,
        // Light Mech (cockpit): 30° front slope, 0° side, 15° back slope.
        armor_angles: ArmorMountAngles::new(30.0, 0.0, 15.0),
    }
}

/// **M13** § "Chassis archetypes — M13 ships 5" — non-humanoid quadruped
/// body graph: 4 legs + 2 claws + torso + sensor cluster + carapace = 11 zones.
fn crab_body_graph() -> BodyGraph {
    let zones = vec![
        BodyZone::Torso,
        BodyZone::Carapace,
        BodyZone::SensorCluster,
        BodyZone::LegFrontLeft,
        BodyZone::LegFrontRight,
        BodyZone::LegRearLeft,
        BodyZone::LegRearRight,
        BodyZone::ClawLeft,
        BodyZone::ClawRight,
        BodyZone::Head,
        BodyZone::Backpack,
    ];
    let joints = vec![
        Joint {
            id: "carapace_to_torso".to_string(),
            parent: BodyZone::Torso,
            child: BodyZone::Carapace,
            intact: true,
        },
        Joint {
            id: "sensor_to_carapace".to_string(),
            parent: BodyZone::Carapace,
            child: BodyZone::SensorCluster,
            intact: true,
        },
        Joint {
            id: "leg_front_left".to_string(),
            parent: BodyZone::Carapace,
            child: BodyZone::LegFrontLeft,
            intact: true,
        },
        Joint {
            id: "leg_front_right".to_string(),
            parent: BodyZone::Carapace,
            child: BodyZone::LegFrontRight,
            intact: true,
        },
        Joint {
            id: "leg_rear_left".to_string(),
            parent: BodyZone::Carapace,
            child: BodyZone::LegRearLeft,
            intact: true,
        },
        Joint {
            id: "leg_rear_right".to_string(),
            parent: BodyZone::Carapace,
            child: BodyZone::LegRearRight,
            intact: true,
        },
        Joint {
            id: "claw_left".to_string(),
            parent: BodyZone::LegFrontLeft,
            child: BodyZone::ClawLeft,
            intact: true,
        },
        Joint {
            id: "claw_right".to_string(),
            parent: BodyZone::LegFrontRight,
            child: BodyZone::ClawRight,
            intact: true,
        },
    ];
    let sockets = vec![
        EquipmentSocket {
            id: "claw_left".to_string(),
            zone: BodyZone::ClawLeft,
            occupied: false,
            mounted_role: None,
        },
        EquipmentSocket {
            id: "claw_right".to_string(),
            zone: BodyZone::ClawRight,
            occupied: false,
            mounted_role: None,
        },
        EquipmentSocket {
            id: SOCKET_TORSO_HARDPOINT.to_string(),
            zone: BodyZone::Torso,
            occupied: false,
            mounted_role: None,
        },
    ];
    let movement_contributions = vec![
        MovementContribution {
            zone: BodyZone::Carapace,
            move_speed_factor_when_destroyed: 0.0,
            jump_impulse_factor_when_destroyed: 0.0,
            disables_rifle_when_destroyed: true,
            forces_crawl_when_destroyed: true,
            drops_gear_when_destroyed: true,
            disables_jet_when_destroyed: true,
        },
        MovementContribution {
            zone: BodyZone::LegFrontLeft,
            move_speed_factor_when_destroyed: 0.75,
            jump_impulse_factor_when_destroyed: 0.75,
            ..MovementContribution::neutral(BodyZone::LegFrontLeft)
        },
        MovementContribution {
            zone: BodyZone::LegFrontRight,
            move_speed_factor_when_destroyed: 0.75,
            jump_impulse_factor_when_destroyed: 0.75,
            ..MovementContribution::neutral(BodyZone::LegFrontRight)
        },
        MovementContribution {
            zone: BodyZone::LegRearLeft,
            move_speed_factor_when_destroyed: 0.75,
            jump_impulse_factor_when_destroyed: 0.75,
            ..MovementContribution::neutral(BodyZone::LegRearLeft)
        },
        MovementContribution {
            zone: BodyZone::LegRearRight,
            move_speed_factor_when_destroyed: 0.75,
            jump_impulse_factor_when_destroyed: 0.75,
            ..MovementContribution::neutral(BodyZone::LegRearRight)
        },
        MovementContribution {
            zone: BodyZone::ClawLeft,
            move_speed_factor_when_destroyed: 1.0,
            jump_impulse_factor_when_destroyed: 1.0,
            disables_rifle_when_destroyed: true,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: true,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::ClawRight,
            move_speed_factor_when_destroyed: 1.0,
            jump_impulse_factor_when_destroyed: 1.0,
            disables_rifle_when_destroyed: true,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: true,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::SensorCluster,
            move_speed_factor_when_destroyed: 1.0,
            jump_impulse_factor_when_destroyed: 1.0,
            disables_rifle_when_destroyed: false,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: false,
            disables_jet_when_destroyed: false,
        },
    ];
    BodyGraph {
        zones,
        joints,
        sockets,
        movement_contributions,
    }
}

/// **M13** § "Crab / quadruped" chassis spec. 11-zone non-humanoid; no jet.
pub fn crab_quadruped_spec() -> ChassisSpec {
    let zones = vec![
        make_zone(BodyZone::Torso, 120.0, 8.0, 80.0, 4.0, 100.0, 30.0),
        make_zone(BodyZone::Carapace, 200.0, 12.0, 100.0, 6.0, 140.0, 40.0),
        make_zone(BodyZone::SensorCluster, 30.0, 4.0, 18.0, 2.0, 24.0, 10.0),
        make_zone(BodyZone::LegFrontLeft, 60.0, 6.0, 40.0, 3.0, 50.0, 16.0),
        make_zone(BodyZone::LegFrontRight, 60.0, 6.0, 40.0, 3.0, 50.0, 16.0),
        make_zone(BodyZone::LegRearLeft, 60.0, 6.0, 40.0, 3.0, 50.0, 16.0),
        make_zone(BodyZone::LegRearRight, 60.0, 6.0, 40.0, 3.0, 50.0, 16.0),
        make_zone(BodyZone::ClawLeft, 40.0, 5.0, 26.0, 2.0, 30.0, 12.0),
        make_zone(BodyZone::ClawRight, 40.0, 5.0, 26.0, 2.0, 30.0, 12.0),
        // Head zone retained for HUD silhouette parity; "head" zone on a crab
        // is the sensor-tower top.
        make_zone(BodyZone::Head, 25.0, 4.0, 15.0, 2.0, 20.0, 10.0),
        // No backpack on crab; emit empty zone so iteration order remains stable.
        make_zone(BodyZone::Backpack, 0.0, 0.0, 0.0, 0.0, 0.0, 4.0),
    ];
    let modules = vec![
        ChassisModule::new("sensor_cluster.array", ModuleKind::Sensor, BodyZone::SensorCluster, 60.0),
        ChassisModule::new("carapace_core.reactor", ModuleKind::Reactor, BodyZone::Carapace, 140.0)
            .with_local_aabb(Aabb::new(-6.0, 4.0, 6.0, 14.0))
            .with_failure_cascade(FailureCascade::ReactorOverpressure),
        ChassisModule::new("motor_controller.fl", ModuleKind::MotorController, BodyZone::LegFrontLeft, 40.0)
            .with_failure_cascade(FailureCascade::MobilityLoss),
        ChassisModule::new("motor_controller.fr", ModuleKind::MotorController, BodyZone::LegFrontRight, 40.0)
            .with_failure_cascade(FailureCascade::MobilityLoss),
        ChassisModule::new("motor_controller.rl", ModuleKind::MotorController, BodyZone::LegRearLeft, 40.0)
            .with_failure_cascade(FailureCascade::MobilityLoss),
        ChassisModule::new("motor_controller.rr", ModuleKind::MotorController, BodyZone::LegRearRight, 40.0)
            .with_failure_cascade(FailureCascade::MobilityLoss),
        ChassisModule::new("fuel_tank.back", ModuleKind::FuelTank, BodyZone::Carapace, 60.0)
            .with_local_aabb(Aabb::new(2.0, 6.0, 8.0, 12.0)),
        ChassisModule::new("weapon_mount.left", ModuleKind::WeaponMount, BodyZone::ClawLeft, 50.0),
        ChassisModule::new("weapon_mount.right", ModuleKind::WeaponMount, BodyZone::ClawRight, 50.0),
        // No jet on crab per spec.
        ChassisModule::not_present("jet.none", ModuleKind::Jet),
        ChassisModule::not_present("shield.none", ModuleKind::Shield),
        ChassisModule::not_present("repair_drone.none", ModuleKind::RepairDrone),
    ];
    ChassisSpec {
        id: CRAB_QUADRUPED_ID.to_string(),
        kind: ChassisKind::CrabQuadruped,
        display_name: "Crab Quadruped CQ-1".to_string(),
        body_graph: crab_body_graph(),
        zones,
        modules,
        eject_window_seconds: 1.2,
        mass_kg: 1200.0,
        // Crab / quadruped: 30° front, 30° side, 30° back.
        armor_angles: ArmorMountAngles::new(30.0, 30.0, 30.0),
    }
}

/// **M13** § "Drone" body graph: 4 zones (chassis core + 2 arms + sensor pod).
fn drone_body_graph() -> BodyGraph {
    let zones = vec![
        BodyZone::DroneCore,
        BodyZone::DroneArmLeft,
        BodyZone::DroneArmRight,
        BodyZone::DroneSensorPod,
    ];
    let joints = vec![
        Joint {
            id: "arm_left_to_core".to_string(),
            parent: BodyZone::DroneCore,
            child: BodyZone::DroneArmLeft,
            intact: true,
        },
        Joint {
            id: "arm_right_to_core".to_string(),
            parent: BodyZone::DroneCore,
            child: BodyZone::DroneArmRight,
            intact: true,
        },
        Joint {
            id: "sensor_pod_to_core".to_string(),
            parent: BodyZone::DroneCore,
            child: BodyZone::DroneSensorPod,
            intact: true,
        },
    ];
    let sockets = vec![
        EquipmentSocket {
            id: "drone_arm_left".to_string(),
            zone: BodyZone::DroneArmLeft,
            occupied: false,
            mounted_role: None,
        },
        EquipmentSocket {
            id: "drone_arm_right".to_string(),
            zone: BodyZone::DroneArmRight,
            occupied: false,
            mounted_role: None,
        },
    ];
    let movement_contributions = vec![
        MovementContribution {
            zone: BodyZone::DroneCore,
            move_speed_factor_when_destroyed: 0.0,
            jump_impulse_factor_when_destroyed: 0.0,
            disables_rifle_when_destroyed: true,
            forces_crawl_when_destroyed: true,
            drops_gear_when_destroyed: true,
            disables_jet_when_destroyed: true,
        },
        MovementContribution {
            zone: BodyZone::DroneArmLeft,
            move_speed_factor_when_destroyed: 1.0,
            jump_impulse_factor_when_destroyed: 1.0,
            disables_rifle_when_destroyed: false,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: true,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::DroneArmRight,
            move_speed_factor_when_destroyed: 1.0,
            jump_impulse_factor_when_destroyed: 1.0,
            disables_rifle_when_destroyed: true,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: true,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::DroneSensorPod,
            ..MovementContribution::neutral(BodyZone::DroneSensorPod)
        },
    ];
    BodyGraph {
        zones,
        joints,
        sockets,
        movement_contributions,
    }
}

/// **M13** § "Drone" chassis spec. 4-zone autonomous miniature chassis; no pilot.
pub fn drone_spec() -> ChassisSpec {
    let zones = vec![
        make_zone(BodyZone::DroneCore, 40.0, 4.0, 24.0, 2.0, 30.0, 16.0),
        make_zone(BodyZone::DroneArmLeft, 20.0, 2.0, 12.0, 1.0, 14.0, 8.0),
        make_zone(BodyZone::DroneArmRight, 20.0, 2.0, 12.0, 1.0, 14.0, 8.0),
        make_zone(BodyZone::DroneSensorPod, 15.0, 2.0, 10.0, 1.0, 12.0, 6.0),
    ];
    let modules = vec![
        ChassisModule::new("power_core.cell", ModuleKind::PowerCore, BodyZone::DroneCore, 40.0)
            .with_local_aabb(Aabb::new(-2.0, 0.0, 2.0, 4.0)),
        ChassisModule::new("sensor_pod.main", ModuleKind::Sensor, BodyZone::DroneSensorPod, 25.0),
        ChassisModule::new("motor_controller.l", ModuleKind::MotorController, BodyZone::DroneArmLeft, 18.0)
            .with_failure_cascade(FailureCascade::MobilityLoss),
        ChassisModule::new("motor_controller.r", ModuleKind::MotorController, BodyZone::DroneArmRight, 18.0)
            .with_failure_cascade(FailureCascade::MobilityLoss),
        ChassisModule::new("comm_relay.main", ModuleKind::CommRelay, BodyZone::DroneCore, 18.0),
        ChassisModule::not_present("jet.none", ModuleKind::Jet),
        ChassisModule::not_present("weapon_mount.none", ModuleKind::WeaponMount),
        ChassisModule::not_present("shield.none", ModuleKind::Shield),
        ChassisModule::not_present("repair_drone.none", ModuleKind::RepairDrone),
    ];
    ChassisSpec {
        id: DRONE_ID.to_string(),
        kind: ChassisKind::Drone,
        display_name: "Recon Drone DR-1".to_string(),
        body_graph: drone_body_graph(),
        zones,
        modules,
        eject_window_seconds: 0.5,
        mass_kg: 60.0,
        // Drone: small target; flat armor mount on all faces.
        armor_angles: ArmorMountAngles::new(0.0, 0.0, 0.0),
    }
}

/// **M14A** § "Heavy Armor — `heavy_trooper_v1`" — tank-grade infantry chassis.
///
/// Per-zone External HP, hardness, and `damage_multiplier` / `gib_impulse_limit`
/// / `stagger_factor` are spec-locked so rifles glance + heavy never knocks down
/// on small-arms hits.
pub fn heavy_trooper_spec() -> ChassisSpec {
    let zones = vec![
        // Head: 240 HP / hardness 18 / dmg×0.6 / gib 1600 / stagger 0.2
        make_zone(BodyZone::Head, 240.0, 18.0, 80.0, 8.0, 120.0, 30.0)
            .with_damage_multiplier(0.6)
            .with_gib_impulse_limit(1600.0)
            .with_stagger_factor(0.2),
        // Torso: 400 HP / hardness 22 / dmg×0.6 / gib 3200 / stagger 0.2
        make_zone(BodyZone::Torso, 400.0, 22.0, 200.0, 12.0, 240.0, 60.0)
            .with_damage_multiplier(0.6)
            .with_gib_impulse_limit(3200.0)
            .with_stagger_factor(0.2),
        // Arms: 180 HP / hardness 16 / dmg×0.75 / gib 2400 / stagger 0.3
        make_zone(BodyZone::ArmRight, 180.0, 16.0, 80.0, 7.0, 100.0, 24.0)
            .with_damage_multiplier(0.75)
            .with_gib_impulse_limit(2400.0)
            .with_stagger_factor(0.3),
        make_zone(BodyZone::ArmLeft, 180.0, 16.0, 80.0, 7.0, 100.0, 24.0)
            .with_damage_multiplier(0.75)
            .with_gib_impulse_limit(2400.0)
            .with_stagger_factor(0.3),
        // Legs: 220 HP / hardness 16 / dmg×0.75 / gib 2400 / stagger 0.3
        make_zone(BodyZone::LegRight, 220.0, 16.0, 100.0, 8.0, 140.0, 32.0)
            .with_damage_multiplier(0.75)
            .with_gib_impulse_limit(2400.0)
            .with_stagger_factor(0.3),
        make_zone(BodyZone::LegLeft, 220.0, 16.0, 100.0, 8.0, 140.0, 32.0)
            .with_damage_multiplier(0.75)
            .with_gib_impulse_limit(2400.0)
            .with_stagger_factor(0.3),
        // Backpack: 140 HP / hardness 12 / dmg×0.8 / gib 1600 / stagger 0.5
        make_zone(BodyZone::Backpack, 140.0, 12.0, 80.0, 6.0, 60.0, 16.0)
            .with_damage_multiplier(0.8)
            .with_gib_impulse_limit(1600.0)
            .with_stagger_factor(0.5),
        make_zone(BodyZone::ForearmRight, 100.0, 14.0, 50.0, 5.0, 60.0, 12.0)
            .with_damage_multiplier(0.8)
            .with_gib_impulse_limit(1600.0)
            .with_stagger_factor(0.5),
        make_zone(BodyZone::ForearmLeft, 100.0, 14.0, 50.0, 5.0, 60.0, 12.0)
            .with_damage_multiplier(0.8)
            .with_gib_impulse_limit(1600.0)
            .with_stagger_factor(0.5),
        make_zone(BodyZone::HandRight, 80.0, 12.0, 40.0, 4.0, 50.0, 12.0)
            .with_damage_multiplier(0.8)
            .with_gib_impulse_limit(1600.0)
            .with_stagger_factor(0.5),
        make_zone(BodyZone::HandLeft, 80.0, 12.0, 40.0, 4.0, 50.0, 12.0)
            .with_damage_multiplier(0.8)
            .with_gib_impulse_limit(1600.0)
            .with_stagger_factor(0.5),
        make_zone(BodyZone::ShinRight, 120.0, 14.0, 60.0, 5.0, 80.0, 16.0)
            .with_damage_multiplier(0.8)
            .with_gib_impulse_limit(1600.0)
            .with_stagger_factor(0.5),
        make_zone(BodyZone::ShinLeft, 120.0, 14.0, 60.0, 5.0, 80.0, 16.0)
            .with_damage_multiplier(0.8)
            .with_gib_impulse_limit(1600.0)
            .with_stagger_factor(0.5),
        make_zone(BodyZone::FootRight, 100.0, 12.0, 50.0, 4.0, 60.0, 14.0)
            .with_damage_multiplier(0.8)
            .with_gib_impulse_limit(1600.0)
            .with_stagger_factor(0.5),
        make_zone(BodyZone::FootLeft, 100.0, 12.0, 50.0, 4.0, 60.0, 14.0)
            .with_damage_multiplier(0.8)
            .with_gib_impulse_limit(1600.0)
            .with_stagger_factor(0.5),
    ];
    let modules = vec![
        ChassisModule::new("weapon_mount.heavy", ModuleKind::WeaponMount, BodyZone::ArmRight, 200.0),
        ChassisModule::new("jet.heavy_trooper", ModuleKind::Jet, BodyZone::Backpack, 100.0),
        ChassisModule::new("shield.heavy_plate", ModuleKind::Shield, BodyZone::Torso, 150.0),
        ChassisModule::not_present("sensor.none", ModuleKind::Sensor),
        ChassisModule::not_present("repair_drone.none", ModuleKind::RepairDrone),
    ];
    ChassisSpec {
        id: HEAVY_TROOPER_ID.to_string(),
        kind: ChassisKind::HeavyTrooper,
        display_name: "Heavy Trooper HT-1".to_string(),
        body_graph: infantry_body_graph(),
        zones,
        modules,
        eject_window_seconds: 1.2,
        mass_kg: 380.0,
        // Heavy Trooper armor mount angles: 40° front, 15° side, 30° back.
        armor_angles: ArmorMountAngles::new(40.0, 15.0, 30.0),
    }
}

/// Stable registry of every launch chassis spec. **M14A** ships 6 archetypes:
/// Infantry, Powered Armor, Light Mech, Crab Quadruped, Drone, Heavy Trooper.
pub fn chassis_specs() -> BTreeMap<&'static str, ChassisSpec> {
    let mut m = BTreeMap::new();
    m.insert(INFANTRY_ID, infantry_spec());
    m.insert(POWERED_ARMOR_ID, powered_armor_spec());
    m.insert(LIGHT_MECH_ID, light_mech_spec());
    m.insert(CRAB_QUADRUPED_ID, crab_quadruped_spec());
    m.insert(DRONE_ID, drone_spec());
    m.insert(HEAVY_TROOPER_ID, heavy_trooper_spec());
    m
}

/// **M13** § "Engineer auto-repair contract" — per-module-class repair time
/// table (seconds per HP point + tool requirement + Engineer priority weight).
/// Consumed by M7's Engineer-role utility scorer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModuleRepairCost {
    /// Module classification id.
    pub class: &'static str,
    /// Seconds of repair work per HP point restored.
    pub seconds_per_hp: f32,
    /// Tool requirement, comma-joined ("welder+plate", "toolkit", etc.).
    pub tool_required: &'static str,
    /// Engineer auto-repair priority weight (0..10).
    pub engineer_priority: u8,
}

impl ModuleRepairCost {
    /// Spec § "Engineer auto-repair contract" canonical table.
    pub fn for_module(kind: ModuleKind) -> Self {
        match kind {
            ModuleKind::Jet => ModuleRepairCost {
                class: "jet",
                seconds_per_hp: 0.6,
                tool_required: "toolkit+power",
                engineer_priority: 8,
            },
            ModuleKind::WeaponMount => ModuleRepairCost {
                class: "weapon_mount",
                seconds_per_hp: 0.4,
                tool_required: "toolkit",
                engineer_priority: 7,
            },
            ModuleKind::Sensor | ModuleKind::Optics => ModuleRepairCost {
                class: "sensor",
                seconds_per_hp: 0.3,
                tool_required: "toolkit",
                engineer_priority: 6,
            },
            ModuleKind::PowerCore | ModuleKind::Reactor => ModuleRepairCost {
                class: "power_cell",
                seconds_per_hp: 1.2,
                tool_required: "toolkit+capacitor",
                engineer_priority: 9,
            },
            _ => ModuleRepairCost {
                class: "generic",
                seconds_per_hp: 0.5,
                tool_required: "toolkit",
                engineer_priority: 5,
            },
        }
    }

    /// Armor-zone repair cost per layer (Engineer auto-repair contract table).
    pub fn for_armor_layer(layer: ArmorLayerKind) -> Self {
        match layer {
            ArmorLayerKind::External => ModuleRepairCost {
                class: "armor_external",
                seconds_per_hp: 0.3,
                tool_required: "welder+plate",
                engineer_priority: 9,
            },
            ArmorLayerKind::Internal => ModuleRepairCost {
                class: "armor_internal",
                seconds_per_hp: 0.5,
                tool_required: "welder+plate",
                engineer_priority: 8,
            },
            ArmorLayerKind::Core => ModuleRepairCost {
                class: "armor_core",
                seconds_per_hp: 0.8,
                tool_required: "welder+plate+power",
                engineer_priority: 7,
            },
        }
    }
}

pub fn chassis_spec(spec_id: &str) -> Option<ChassisSpec> {
    chassis_specs().get(spec_id).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    mod chassis_a;

    #[test]
    fn powered_armor_spec_has_canonical_zones_and_modules() {
        let s = powered_armor_spec();
        assert_eq!(s.kind, ChassisKind::PoweredArmor);
        // 15-zone full body graph: 7 base zones (head/torso/arms/legs/backpack)
        // + 8 granular limbs (forearms/hands + shins/feet pairs) per M5 spec.
        assert_eq!(s.zones.len(), 15);
        // Granular limbs verified:
        for zone in [
            BodyZone::ForearmRight,
            BodyZone::ForearmLeft,
            BodyZone::HandRight,
            BodyZone::HandLeft,
            BodyZone::ShinRight,
            BodyZone::ShinLeft,
            BodyZone::FootRight,
            BodyZone::FootLeft,
        ] {
            assert!(s.zones.iter().any(|z| z.zone == zone), "missing granular zone {zone:?}");
        }
        assert!(s.modules.iter().any(|m| m.kind == ModuleKind::WeaponMount));
        assert!(s.modules.iter().any(|m| m.kind == ModuleKind::Jet));
        assert!(s.modules.iter().any(|m| m.kind == ModuleKind::Shield));
        assert!(s.modules.iter().any(|m| m.kind == ModuleKind::Sensor));
    }

    #[test]
    fn destroying_hand_right_disables_rifle_via_movement_contribution() {
        let graph = infantry_body_graph();
        let (_m, _j, disable_rifle, _, drop_gear, _) = graph.movement_factor(&[BodyZone::HandRight]);
        assert!(disable_rifle, "destroyed right hand must disable rifle");
        assert!(drop_gear, "destroyed right hand must drop gear");
    }

    #[test]
    fn destroying_shin_left_reduces_movement_speed() {
        let graph = infantry_body_graph();
        let (move_factor, jump_factor, _, _, _, _) = graph.movement_factor(&[BodyZone::ShinLeft]);
        assert!(move_factor <= 0.5, "destroyed left shin must reduce move speed");
        assert!(jump_factor <= 0.4, "destroyed left shin must reduce jump");
    }

    #[test]
    fn parent_chain_resolves_correctly() {
        assert_eq!(BodyZone::HandRight.parent(), Some(BodyZone::ForearmRight));
        assert_eq!(BodyZone::ForearmRight.parent(), Some(BodyZone::ArmRight));
        assert_eq!(BodyZone::ArmRight.parent(), Some(BodyZone::Torso));
        assert_eq!(BodyZone::Torso.parent(), None);
        assert_eq!(BodyZone::FootLeft.parent(), Some(BodyZone::ShinLeft));
        assert!(BodyZone::HandLeft.is_left_arm_chain());
        assert!(BodyZone::FootRight.is_right_leg_chain());
    }

    #[test]
    fn light_mech_spec_has_repair_drone() {
        let s = light_mech_spec();
        assert!(s.modules.iter().any(|m| m.kind == ModuleKind::RepairDrone));
    }

    #[test]
    fn infantry_has_no_jet_module_present() {
        let s = infantry_spec();
        let jet = s.modules.iter().find(|m| m.kind == ModuleKind::Jet).unwrap();
        assert_eq!(jet.state, ModuleStateKind::NotPresent);
    }

    #[test]
    fn chassis_state_initializes_to_nominal() {
        let spec = powered_armor_spec();
        let state = ChassisState::from_spec(&spec, 60, false);
        assert_eq!(state.stage, ChassisStage::Nominal);
        assert_eq!(state.pilot_state, PilotState::Bound);
        assert!((state.integrity() - 1.0).abs() < 1e-3);
    }

    #[test]
    fn external_layer_glances_low_damage() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, false);
        let outcome = state.apply_zone_damage(BodyZone::Torso, 3.0, "projectile_hit");
        assert!(!outcome.glances.is_empty(), "expected hardness glance");
        assert!(outcome.layer_damage.is_empty(), "no layer should take damage");
    }

    #[test]
    fn damage_breaches_external_then_internal_then_core() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, false);
        let total = 80.0 + 50.0 + 60.0 + 30.0 + 10.0; // overkill torso
        let _ = state.apply_zone_damage(BodyZone::Torso, total, "projectile_hit");
        let torso = state.zone(BodyZone::Torso).unwrap();
        assert!(torso.destroyed, "torso must be destroyed");
        for layer in &torso.layers {
            assert!(layer.is_breached(), "layer {} should be breached", layer.kind.as_str());
        }
    }

    #[test]
    fn destroying_backpack_fails_jet_module() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, false);
        let total = 100.0 + 60.0 + 30.0; // overkill backpack
        let outcome = state.apply_zone_damage(BodyZone::Backpack, total, "projectile_hit");
        assert!(outcome.zone_destroyed);
        let jet = state.module_by_kind(ModuleKind::Jet).unwrap();
        assert_eq!(jet.state, ModuleStateKind::Failed);
    }

    #[test]
    fn stage_advances_to_armor_cracked_at_50_percent() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, false);
        let _ = state.apply_zone_damage(BodyZone::Torso, 200.0, "projectile_hit");
        let _ = state.apply_zone_damage(BodyZone::ArmRight, 150.0, "projectile_hit");
        let _ = state.recompute_stage();
        assert!(state.stage >= ChassisStage::ArmorCracked);
    }

    #[test]
    fn attempt_eject_starts_window() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, false);
        let accepted = state.attempt_eject(120).unwrap();
        assert!(!accepted.tutorial_extract);
        assert_eq!(state.pilot_state, PilotState::Ejecting);
        assert!(state.eject_window.ticks_remaining > 0);
    }

    #[test]
    fn tick_eject_completes_to_ejected() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, false);
        state.attempt_eject(0).unwrap();
        let ticks = state.eject_window.ticks_total;
        for _ in 0..ticks - 1 {
            let _ = state.tick_eject();
        }
        let final_progress = state.tick_eject();
        assert_eq!(final_progress, Some(EjectProgress::Ejected));
        assert_eq!(state.pilot_state, PilotState::Ejected);
    }

    #[test]
    fn bail_too_late_when_wreck_first() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, false);
        state.attempt_eject(0).unwrap();
        // Wreck the chassis mid-eject.
        let _ = state.apply_zone_damage(BodyZone::Torso, 1000.0, "projectile_hit");
        let _ = state.recompute_stage();
        // Force-set stage to Wreck (recompute may not catch it without all zones gone).
        state.stage = ChassisStage::Wreck;
        let ticks = state.eject_window.ticks_total;
        for _ in 0..ticks - 1 {
            let _ = state.tick_eject();
        }
        let final_progress = state.tick_eject();
        assert_eq!(final_progress, Some(EjectProgress::BailedTooLate));
        assert_eq!(state.pilot_state, PilotState::BailedTooLate);
    }

    #[test]
    fn tutorial_safety_caps_at_pilot_injured() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, true);
        state.pilot_state = PilotState::Injured;
        let _ = state.apply_zone_damage(BodyZone::Torso, 1000.0, "projectile_hit");
        let _ = state.recompute_stage();
        assert!(
            state.stage <= ChassisStage::PilotInjured,
            "tutorial safety must cap stage; got {:?}",
            state.stage
        );
    }

    #[test]
    fn tutorial_safety_blocks_eject_to_lost() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, true);
        let outcome = state.attempt_eject(10).unwrap();
        assert!(outcome.tutorial_extract);
        assert_eq!(state.pilot_state, PilotState::Extracted);
    }

    #[test]
    fn repair_zone_restores_modules() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, false);
        let _ = state.apply_zone_damage(BodyZone::Backpack, 500.0, "projectile_hit");
        let outcome = state.repair_zone(BodyZone::Backpack, "field_kit").unwrap();
        assert!(outcome.was_destroyed);
        assert!(!outcome.modules_restored.is_empty());
        let jet = state.module_by_kind(ModuleKind::Jet).unwrap();
        assert_eq!(jet.state, ModuleStateKind::Nominal);
    }

    #[test]
    fn salvage_pulls_surviving_modules() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, false);
        // Wreck the torso so the chassis is salvageable.
        let _ = state.apply_zone_damage(BodyZone::Torso, 1000.0, "projectile_hit");
        let _ = state.recompute_stage();
        state.stage = ChassisStage::Wreck;
        let outcome = state.salvage("ejected_pilot_returns").unwrap();
        // Shield was bound to torso — should NOT be salvaged.
        assert!(!outcome.salvaged_module_ids.iter().any(|id| id.starts_with("shield")));
        // Sensor (head) + jet (backpack) survive.
        assert!(!outcome.salvaged_module_ids.is_empty());
        assert!(!state.salvaged_modules.is_empty());
    }

    #[test]
    fn weapon_jam_and_clear_round_trip() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, false);
        assert!(state.jam_weapon("debris_in_action"));
        assert!(state.weapon_jammed);
        let _ = state.recompute_stage();
        assert!(state.stage >= ChassisStage::WeaponJammed);
        assert!(state.clear_jam());
        assert!(!state.weapon_jammed);
    }

    #[test]
    fn movement_factor_reflects_destroyed_zones() {
        let graph = infantry_body_graph();
        let (m, j, dr, fc, dg, dj) = graph.movement_factor(&[BodyZone::LegRight, BodyZone::Backpack]);
        assert!(m <= 0.5);
        assert!(j <= 0.4);
        assert!(!dr);
        assert!(!fc);
        assert!(!dg);
        assert!(dj, "destroyed backpack must disable jet");
    }

    #[test]
    fn checksum_layout_is_stable() {
        let spec = powered_armor_spec();
        let s1 = ChassisState::from_spec(&spec, 60, false);
        let s2 = ChassisState::from_spec(&spec, 60, false);
        assert_eq!(s1.checksum_bytes(), s2.checksum_bytes());
    }

    #[test]
    fn checksum_distinguishes_zone_damage() {
        let spec = powered_armor_spec();
        let mut a = ChassisState::from_spec(&spec, 60, false);
        let mut b = ChassisState::from_spec(&spec, 60, false);
        let _ = a.apply_zone_damage(BodyZone::Torso, 50.0, "hit");
        let _ = b.apply_zone_damage(BodyZone::LegLeft, 50.0, "hit");
        assert_ne!(a.checksum_bytes(), b.checksum_bytes());
    }

    #[test]
    fn registry_resolves_canonical_ids() {
        assert!(chassis_spec(POWERED_ARMOR_ID).is_some());
        assert!(chassis_spec(LIGHT_MECH_ID).is_some());
        assert!(chassis_spec(INFANTRY_ID).is_some());
        assert!(chassis_spec("nonexistent").is_none());
    }

    /// **M13** § "Chassis archetypes — M13 ships 5": crab + drone archetypes
    /// must be in the canonical registry alongside the 3 humanoid kinds.
    #[test]
    fn registry_ships_six_chassis_archetypes() {
        assert!(chassis_spec(CRAB_QUADRUPED_ID).is_some());
        assert!(chassis_spec(DRONE_ID).is_some());
        // **M14A** ships the 6th archetype — Heavy Trooper.
        assert!(chassis_spec(HEAVY_TROOPER_ID).is_some());
        assert_eq!(chassis_specs().len(), 6, "M14A ships 6 chassis archetypes");
    }

    /// **M14A** § "Heavy Armor" — heavy trooper spec contract.
    #[test]
    fn heavy_trooper_spec_has_tank_grade_zones() {
        let s = heavy_trooper_spec();
        assert_eq!(s.kind, ChassisKind::HeavyTrooper);
        assert!((s.mass_kg - 380.0).abs() < 1e-3);
        // Torso External ≥ 400 HP at hardness ≥ 22.
        let torso = s.zones.iter().find(|z| z.zone == BodyZone::Torso).unwrap();
        let torso_ext = torso.layers.iter().find(|l| l.kind == ArmorLayerKind::External).unwrap();
        assert!(torso_ext.hp >= 400.0);
        assert!(torso_ext.hardness >= 22.0);
        // Per-zone tunings: torso dmg_multiplier=0.6, stagger=0.2, gib≥3200.
        assert!((torso.damage_multiplier - 0.6).abs() < 1e-6);
        assert!((torso.stagger_factor - 0.2).abs() < 1e-6);
        assert!(torso.gib_impulse_limit >= 3200.0);
    }

    /// **M13** § "Quadruped=11 zones": crab body graph zone count contract.
    #[test]
    fn crab_quadruped_has_eleven_zones() {
        let s = crab_quadruped_spec();
        assert_eq!(s.kind, ChassisKind::CrabQuadruped);
        assert_eq!(s.zones.len(), 11);
        for zone in [
            BodyZone::Carapace,
            BodyZone::SensorCluster,
            BodyZone::LegFrontLeft,
            BodyZone::LegFrontRight,
            BodyZone::LegRearLeft,
            BodyZone::LegRearRight,
            BodyZone::ClawLeft,
            BodyZone::ClawRight,
        ] {
            assert!(s.zones.iter().any(|z| z.zone == zone), "missing crab zone {zone:?}");
        }
        // No jet on crab.
        let jet = s.modules.iter().find(|m| m.kind == ModuleKind::Jet).unwrap();
        assert_eq!(jet.state, ModuleStateKind::NotPresent);
    }

    /// **M13** § "Drone=4 zones": drone body graph zone count contract.
    #[test]
    fn drone_has_four_zones() {
        let s = drone_spec();
        assert_eq!(s.kind, ChassisKind::Drone);
        assert_eq!(s.zones.len(), 4);
        for zone in [
            BodyZone::DroneCore,
            BodyZone::DroneArmLeft,
            BodyZone::DroneArmRight,
            BodyZone::DroneSensorPod,
        ] {
            assert!(s.zones.iter().any(|z| z.zone == zone), "missing drone zone {zone:?}");
        }
    }

    /// **M13** § "Armor mounting angles per chassis archetype": per-chassis
    /// angles match the spec table.
    #[test]
    fn armor_mount_angles_match_spec() {
        assert_eq!(infantry_spec().armor_angles, ArmorMountAngles::new(0.0, 0.0, 0.0));
        assert_eq!(powered_armor_spec().armor_angles, ArmorMountAngles::new(15.0, 0.0, 15.0));
        assert_eq!(light_mech_spec().armor_angles, ArmorMountAngles::new(30.0, 0.0, 15.0));
        assert_eq!(
            crab_quadruped_spec().armor_angles,
            ArmorMountAngles::new(30.0, 30.0, 30.0)
        );
        assert_eq!(drone_spec().armor_angles, ArmorMountAngles::new(0.0, 0.0, 0.0));
    }

    /// **M13** § "Chassis ability slots": per-chassis slot count + activate.
    #[test]
    fn chassis_ability_slots_count_per_kind() {
        assert_eq!(ChassisKind::Infantry.ability_slot_count(), 1);
        assert_eq!(ChassisKind::PoweredArmor.ability_slot_count(), 2);
        assert_eq!(ChassisKind::LightMech.ability_slot_count(), 3);
        assert_eq!(ChassisKind::Drone.ability_slot_count(), 1);
    }

    #[test]
    fn chassis_ability_activate_advances_and_cools_down() {
        let mut state = ChassisState::from_spec(&powered_armor_spec(), 60, false);
        let result = state.activate_ability(ChassisAbility::Overdrive);
        assert!(result.is_ok());
        // Activating again while active fails.
        let err = state.activate_ability(ChassisAbility::Overdrive).unwrap_err();
        assert_eq!(err, AbilityRejectReason::AlreadyActive);
        // Tick out the effect — engine should drain effect ticks.
        for _ in 0..state.abilities.find(ChassisAbility::Overdrive).unwrap().effect_total_ticks {
            state.tick_abilities();
        }
        // Now cooling down.
        let err = state.activate_ability(ChassisAbility::Overdrive).unwrap_err();
        assert_eq!(err, AbilityRejectReason::OnCooldown);
    }

    /// **M13** § "Weapon modifier slots": attach/detach respects max slot count.
    #[test]
    fn weapon_modifier_slot_count_per_chassis_tier() {
        let mut set = WeaponModifierSet::new(ChassisKind::Infantry);
        assert_eq!(set.max_slots, 1);
        set.attach(WeaponModifier::Homing).unwrap();
        assert!(set.attach(WeaponModifier::Explosive).is_err());
        let mut mech = WeaponModifierSet::new(ChassisKind::LightMech);
        assert_eq!(mech.max_slots, 3);
        mech.attach(WeaponModifier::Homing).unwrap();
        mech.attach(WeaponModifier::Explosive).unwrap();
        mech.attach(WeaponModifier::Freezing).unwrap();
        assert!(mech.is_combined());
        assert!(mech.attach(WeaponModifier::ChainLightning).is_err());
    }

    /// **M13** § "30+ launch modifiers": registry has at least 30 modifiers.
    #[test]
    fn weapon_modifier_registry_has_thirty_plus() {
        assert!(WeaponModifier::all().len() >= 30);
        assert_eq!(WeaponModifier::parse("homing"), Some(WeaponModifier::Homing));
        assert_eq!(WeaponModifier::parse("nonsense"), None);
    }

    /// **M13** § "Drone allies — 4 modes": parse + fuel drain.
    #[test]
    fn drone_modes_round_trip_and_drain_fuel() {
        assert_eq!(DroneMode::parse("auto_mine"), Some(DroneMode::AutoMine));
        assert_eq!(DroneMode::parse("auto-carry"), Some(DroneMode::AutoCarry));
        let mut drone = DroneAllyState::default();
        assert!((drone.fuel - 1.0).abs() < 1e-6);
        for _ in 0..18000 {
            drone.tick_fuel(60);
        }
        assert!(drone.fuel < 1.0, "5 minutes of fuel drain should reduce charge");
    }

    /// **M13** § "Cockpit camera anchor — Medium + Heavy classes only".
    #[test]
    fn cockpit_anchor_rejects_unsupported_chassis() {
        let mut infantry = ChassisState::from_spec(&infantry_spec(), 60, false);
        assert!(infantry.set_camera_anchor(CameraAnchor::Cockpit).is_err());
        let mut mech = ChassisState::from_spec(&light_mech_spec(), 60, false);
        assert!(mech.set_camera_anchor(CameraAnchor::Cockpit).is_ok());
        assert_eq!(mech.camera_anchor, CameraAnchor::Cockpit);
    }

    /// **M13** § "Boarding / disembarking transitions": 1500ms transitions
    /// are tick-rate-stable.
    #[test]
    fn boarding_transitions_match_1500ms_at_any_tick_rate() {
        let mut at_60 = ChassisState::from_spec(&powered_armor_spec(), 60, false);
        let at_120 = ChassisState::from_spec(&powered_armor_spec(), 120, false);
        assert_eq!(at_60.transition_ticks_total, 90); // 1.5s * 60Hz
        assert_eq!(at_120.transition_ticks_total, 180); // 1.5s * 120Hz
        assert!(at_60.begin_boarding());
        assert!(at_60.is_in_transition());
        // Cannot start a second transition while one is in flight.
        assert!(!at_60.begin_disembarking());
        // Tick until completion.
        let mut completed = None;
        for _ in 0..at_60.transition_ticks_total {
            completed = at_60.tick_transitions();
        }
        assert_eq!(completed, Some(TransitionCompleted::Boarded));
        assert!(!at_60.is_in_transition());
        let _ = at_120;
    }

    /// **M13** § "Hit reactions per body part": tabulated reactions per zone.
    #[test]
    fn hit_reactions_match_spec_table() {
        let head = HitReaction::for_zone(BodyZone::Head);
        assert_eq!(head.kind, "stagger_stun");
        assert!((head.duration_seconds - 0.5).abs() < 1e-6);
        assert_eq!(head.concussion_dose, 15);
        let hand = HitReaction::for_zone(BodyZone::HandRight);
        assert_eq!(hand.kind, "drop_weapon");
        assert!((hand.drop_chance - 0.40).abs() < 1e-6);
        let leg = HitReaction::for_zone(BodyZone::LegLeft);
        assert!((leg.speed_factor - 0.7).abs() < 1e-6);
        let head_ticks = head.duration_ticks(60);
        assert_eq!(head_ticks, 30);
    }

    /// **M13** § "Critical chassis modules with full mechanics": ammo rack
    /// cooking + detonation cascade.
    #[test]
    fn ammo_rack_cascade_cooks_then_detonates() {
        let mut state = ChassisState::from_spec(&light_mech_spec(), 60, false);
        // Apply damage to drive AmmoRack toward Warning then Failed.
        let warn = state.apply_critical_module_damage("ammo_rack.main", 50.0, "shaped_charge").unwrap();
        assert!(matches!(
            warn.cascade_events.first(),
            Some(CriticalModuleEvent::AmmoCooking { .. })
        ));
        // Now finish off the rack.
        let finish = state.apply_critical_module_damage("ammo_rack.main", 50.0, "second_hit").unwrap();
        assert!(finish
            .cascade_events
            .iter()
            .any(|e| matches!(e, CriticalModuleEvent::AmmoDetonated { .. })));
        assert_eq!(state.stage, ChassisStage::Gibbed);
    }

    /// **M13** § "Spalling integration": deterministic fragment routing.
    #[test]
    fn spalling_fragments_are_deterministic_given_same_seed() {
        let mut a = ChassisState::from_spec(&light_mech_spec(), 60, false);
        let mut b = ChassisState::from_spec(&light_mech_spec(), 60, false);
        let frags_a = a.spawn_spalling_fragments((0.0, 0.0), 3, 30.0, 42);
        let frags_b = b.spawn_spalling_fragments((0.0, 0.0), 3, 30.0, 42);
        assert_eq!(frags_a.len(), 3);
        assert_eq!(frags_a.len(), frags_b.len());
        for (fa, fb) in frags_a.iter().zip(frags_b.iter()) {
            assert_eq!(fa.module_id, fb.module_id);
            assert!((fa.damage - fb.damage).abs() < 1e-3);
        }
    }

    /// **M13** § "Limb loss functional consequences" — head destruction
    /// flags `lethal=true` (instant death per CCCP decapitation rule).
    #[test]
    fn head_destruction_flags_lethal_when_not_tutorial_safe() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, false);
        let outcome = state.apply_zone_damage(BodyZone::Head, 1000.0, "headshot");
        assert!(outcome.zone_destroyed, "head zone must be destroyed by 1000 dmg");
        assert!(outcome.lethal, "head destruction must flag lethal=true");
    }

    /// **M13** § "Tutorial-safety scenario variant" — head destruction does
    /// NOT flag lethal when tutorial_safety=true.
    #[test]
    fn head_destruction_skips_lethal_in_tutorial_safety() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, true);
        let outcome = state.apply_zone_damage(BodyZone::Head, 1000.0, "headshot");
        assert!(outcome.zone_destroyed);
        assert!(!outcome.lethal, "tutorial_safety must suppress lethal");
    }

    /// **M13** § "Torso loss = INSTANT DEATH": torso destruction flags lethal.
    #[test]
    fn torso_destruction_flags_lethal() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, false);
        let outcome = state.apply_zone_damage(BodyZone::Torso, 2000.0, "shaped_charge");
        assert!(outcome.zone_destroyed);
        assert!(outcome.lethal);
    }

    /// **M13** § "Arm loss" — destroying an arm does NOT flag lethal (only
    /// head/torso do).
    #[test]
    fn arm_destruction_does_not_flag_lethal() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, false);
        let outcome = state.apply_zone_damage(BodyZone::ArmLeft, 1000.0, "explosion");
        assert!(outcome.zone_destroyed);
        assert!(!outcome.lethal, "arm destruction must NOT flag lethal");
    }

    /// **M13** § "Engineer auto-repair contract" — per-module repair cost
    /// table matches the spec values.
    #[test]
    fn engineer_auto_repair_table_matches_spec() {
        let jet = ModuleRepairCost::for_module(ModuleKind::Jet);
        assert!((jet.seconds_per_hp - 0.6).abs() < 1e-6);
        assert_eq!(jet.engineer_priority, 8);
        let core = ModuleRepairCost::for_module(ModuleKind::PowerCore);
        assert_eq!(core.engineer_priority, 9);
        let ext = ModuleRepairCost::for_armor_layer(ArmorLayerKind::External);
        assert!((ext.seconds_per_hp - 0.3).abs() < 1e-6);
        assert_eq!(ext.engineer_priority, 9);
    }
}
