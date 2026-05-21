use serde::{Deserialize, Serialize};

use crate::{ArmorLayerKind, BodyZone, ChassisStage, ModuleKind, ModuleStateKind};

/// **M13** § "Critical chassis modules with full mechanics" — typed cascade
/// event surfaced by [`crate::ChassisState::apply_critical_module_damage`].
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

/// Aggregate outcome from [`crate::ChassisState::apply_critical_module_damage`].
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

/// Outcome of [`crate::ChassisState::attempt_eject`].
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

/// Outcome of [`crate::ChassisState::salvage`].
#[derive(Debug, Clone, PartialEq)]
pub struct SalvageOutcome {
    pub salvaged_module_ids: Vec<String>,
    pub reason: String,
}

/// Outcome of [`crate::ChassisState::repair_zone`].
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

/// Aggregate outcome of [`crate::ChassisState::apply_zone_damage`].
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
pub(crate) fn stage_from_integrity(integrity: f32) -> ModuleStateKind {
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
