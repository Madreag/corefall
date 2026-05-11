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

/// One of the launch chassis archetypes. `Infantry` is the "no chassis" baseline a
/// pilot ejects INTO; `PoweredArmor` is the Spartan-ish bulky-but-still-human-shaped
/// suit; `LightMech` is the ~3x-human bipedal walker.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChassisKind {
    Infantry = 0,
    PoweredArmor = 1,
    LightMech = 2,
}

impl ChassisKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ChassisKind::Infantry => "infantry",
            ChassisKind::PoweredArmor => "powered_armor",
            ChassisKind::LightMech => "light_mech",
        }
    }

    /// Reference ids (`POWERED_ARMOR_ID`, `LIGHT_MECH_ID`, `INFANTRY_ID`).
    pub fn default_spec_id(self) -> &'static str {
        match self {
            ChassisKind::Infantry => INFANTRY_ID,
            ChassisKind::PoweredArmor => POWERED_ARMOR_ID,
            ChassisKind::LightMech => LIGHT_MECH_ID,
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
        }
    }

    /// Canonical iteration order for events + checksums. Stable across milestones.
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
        ]
    }

    /// Parent zone in the kinematic chain. `Head` and `Torso` and `Backpack`
    /// have no parent; the chains are torso → arm/leg (upper) → forearm/shin →
    /// hand/foot.
    pub fn parent(self) -> Option<BodyZone> {
        match self {
            BodyZone::Head | BodyZone::Torso | BodyZone::Backpack => None,
            BodyZone::ArmLeft | BodyZone::ArmRight | BodyZone::LegLeft | BodyZone::LegRight => Some(BodyZone::Torso),
            BodyZone::ForearmLeft => Some(BodyZone::ArmLeft),
            BodyZone::ForearmRight => Some(BodyZone::ArmRight),
            BodyZone::HandLeft => Some(BodyZone::ForearmLeft),
            BodyZone::HandRight => Some(BodyZone::ForearmRight),
            BodyZone::ShinLeft => Some(BodyZone::LegLeft),
            BodyZone::ShinRight => Some(BodyZone::LegRight),
            BodyZone::FootLeft => Some(BodyZone::ShinLeft),
            BodyZone::FootRight => Some(BodyZone::ShinRight),
        }
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
    /// `true` once `wound_hp <= 0`; the zone is destroyed and emits
    /// `armor_zone_destroyed`. Limb destruction has mechanical consequences listed
    /// in [`BodyGraph::movement_contributions`].
    pub destroyed: bool,
}

impl ZoneState {
    pub fn new(zone: BodyZone, layers: Vec<ArmorLayer>, wound_hp: f32) -> Self {
        Self {
            zone,
            layers,
            wound_hp: wound_hp.max(0.0),
            wound_hp_max: wound_hp.max(0.0),
            destroyed: false,
        }
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
/// zone so module health follows the zone.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleKind {
    WeaponMount = 0,
    Jet = 1,
    Shield = 2,
    Sensor = 3,
    RepairDrone = 4,
}

impl ModuleKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ModuleKind::WeaponMount => "weapon_mount",
            ModuleKind::Jet => "jet",
            ModuleKind::Shield => "shield",
            ModuleKind::Sensor => "sensor",
            ModuleKind::RepairDrone => "repair_drone",
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
        }
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
}

impl ChassisState {
    /// Build a runtime state from a spec at the given tick rate.
    pub fn from_spec(spec: &ChassisSpec, tick_rate_hz: u32, tutorial_safety: bool) -> Self {
        let tick_rate = tick_rate_hz.max(1);
        let eject_ticks = ((spec.eject_window_seconds.max(0.1)) * tick_rate as f32).round() as u32;
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
        }
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
    let zones = BodyZone::all().to_vec();
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
    }
}

/// Stable registry of every launch chassis spec.
pub fn chassis_specs() -> BTreeMap<&'static str, ChassisSpec> {
    let mut m = BTreeMap::new();
    m.insert(INFANTRY_ID, infantry_spec());
    m.insert(POWERED_ARMOR_ID, powered_armor_spec());
    m.insert(LIGHT_MECH_ID, light_mech_spec());
    m
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
}
