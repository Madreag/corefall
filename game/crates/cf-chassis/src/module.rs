use serde::{Deserialize, Serialize};

use crate::{ArmorLayerKind, BodyZone};

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
    Cockpit = 5,
    AmmoRack = 6,
    Engine = 7,
    Optics = 8,
    Transmission = 9,
    Reactor = 10,
    PowerCore = 11,
    FuelTank = 12,
    TargetingComputer = 13,
    CommRelay = 14,
    MotorController = 15,
    /// panel that pre-detonates incoming HEAT jets. ERA panels are
    /// HEAT-specific (APFSDS bypasses them per VAL-M14C-024).
    Era = 16,
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
            ModuleKind::Era => "era",
        }
    }

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
    /// traversal". Chassis-local AABB describing this module's hitbox for
    /// ray traversal. Empty (`is_positioned() == false`) when the module
    /// has no positioned geometry; ray traversal then falls back to
    /// bound-zone presence.
    #[serde(default)]
    pub local_aabb: Aabb,
    /// module reaches `Failed`.
    #[serde(default)]
    pub failure_cascade: FailureCascade,
    /// `module.ammo_rack_cooking` / `module.ammo_rack_detonated` event
    /// cascade. Zero for modules whose kind != `AmmoRack`.
    #[serde(default)]
    pub ammo_quantity_remaining: u32,
    #[serde(default)]
    pub rounds_cooked_off: u32,
    /// raises fire-risk on engine penetration.
    #[serde(default = "default_fluid_level")]
    pub oil_level: f32,
    #[serde(default = "default_fluid_level")]
    pub coolant_level: f32,
    /// 0 = nominal, 4 = critical (volatile release imminent).
    #[serde(default)]
    pub pressure_state: u8,
    /// this module has already emitted its tier-crossing cascade events
    /// (AmmoCooking / EngineOilLeak / EngineFire / ReactorPressureAdvanced /
    /// OpticsImpaired / MobilityReduced). Used to prevent redundant
    /// cascade emission when multiple zone hits in the same tick each
    /// trigger `apply_critical_module_damage` for the same module while
    /// the module's state hasn't actually crossed a tier this call.
    /// Initialized to `Nominal` (no cascade has fired yet).
    #[serde(default = "default_module_state_kind")]
    pub last_cascade_emitted_state: ModuleStateKind,
    /// `true` = ERA panel intact, `false` = ERA has already pre-detonated
    /// against a prior HEAT impact and can no longer disrupt a jet (per
    /// VAL-M14C-002). Initialized true for ERA modules, false for all
    /// other kinds (irrelevant).
    #[serde(default = "default_era_consumable")]
    pub era_consumable: bool,
    /// `era_charge_kg × 0.7` HEAT penetration reduction formula per
    /// VAL-M14C-025. Defaults to 1.0 kg (~70% reduction) which matches
    /// Gherkin-2's "~70% reduction" band.
    #[serde(default = "default_era_charge_kg")]
    pub era_charge_kg: f32,
}

pub(crate) fn default_era_consumable() -> bool {
    true
}

pub(crate) fn default_era_charge_kg() -> f32 {
    1.0
}

pub(crate) fn default_module_state_kind() -> ModuleStateKind {
    ModuleStateKind::Nominal
}

pub(crate) fn default_fluid_level() -> f32 {
    1.0
}

impl ChassisModule {
    pub fn new(id: impl Into<String>, kind: ModuleKind, bound_zone: BodyZone, hp_max: f32) -> Self {
        let is_era = matches!(kind, ModuleKind::Era);
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
            era_consumable: is_era,
            era_charge_kg: if is_era { default_era_charge_kg() } else { 0.0 },
        }
    }

    #[must_use]
    pub fn with_local_aabb(mut self, aabb: Aabb) -> Self {
        self.local_aabb = aabb;
        self
    }

    #[must_use]
    pub fn with_failure_cascade(mut self, cascade: FailureCascade) -> Self {
        self.failure_cascade = cascade;
        self
    }

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
            era_consumable: false,
            era_charge_kg: 0.0,
        }
    }

    /// `era_charge_kg` for an ERA panel module. Other module kinds ignore
    /// the call (the helper returns `self` unchanged).
    #[must_use]
    pub fn with_era(mut self, era_charge_kg: f32, consumable: bool) -> Self {
        if matches!(self.kind, ModuleKind::Era) {
            self.era_charge_kg = era_charge_kg.max(0.0);
            self.era_consumable = consumable;
        }
        self
    }

    /// panel and is still consumable (intact), mark it spent and return
    /// the panel's `era_charge_kg`. Returns `None` for non-ERA modules or
    /// already-spent ERA panels (per VAL-M14C-002 one-shot rule).
    pub fn consume_era_panel(&mut self) -> Option<f32> {
        if matches!(self.kind, ModuleKind::Era) && self.era_consumable {
            self.era_consumable = false;
            Some(self.era_charge_kg.max(0.0))
        } else {
            None
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
