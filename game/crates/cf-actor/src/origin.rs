//! M17 — canonical per-origin reaction + resource model.
//!
//! This module owns the **origin reaction matrix**: the per-race table that
//! turns a chassis-bearing actor's origin into concrete survival resources
//! (blood / oil / power / caloric / bio_fluid / oxygen), shot-force-feedback
//! content (pain_jolt vs servo_jolt + frame_ring), concussion susceptibility,
//! environmental resistances, and breathing / temperature / pressure / gravity
//! envelopes. The engine (`cf-control::m17_origin`) consumes [`OriginProfile`]
//! every tick to drain resources, gate death, and emit `resource.*` /
//! `origin.*` events.
//!
//! Determinism: pure data + pure functions. No RNG, no clock. The hardcoded
//! [`OriginProfile::canonical`] is the boot fallback; `content/origins/*.json`
//! overrides it via the engine loader.

use serde::{Deserialize, Serialize};

use crate::ResourceAccumulators;

/// The launch races / origins. Discriminants 0-9 mirror
/// `cf_disease::OriginId` / `cf_mental_health::OriginId`; `PoweredOrganic`
/// (10) is the M17 cyber-human hybrid the resource + power matrices treat
/// distinctly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Origin {
    Human = 0,
    Android = 1,
    Robot = 2,
    Drone = 3,
    HeavyBiomech = 4,
    MethaneBreather = 5,
    Crystalline = 6,
    Aqueous = 7,
    Photosynthetic = 8,
    Insectoid = 9,
    PoweredOrganic = 10,
}

impl Default for Origin {
    fn default() -> Self {
        Origin::Human
    }
}

impl Origin {
    /// Canonical snake_case identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            Origin::Human => "human",
            Origin::Android => "android",
            Origin::Robot => "robot",
            Origin::Drone => "drone",
            Origin::HeavyBiomech => "heavy_biomech",
            Origin::MethaneBreather => "methane_breather",
            Origin::Crystalline => "crystalline",
            Origin::Aqueous => "aqueous",
            Origin::Photosynthetic => "photosynthetic",
            Origin::Insectoid => "insectoid",
            Origin::PoweredOrganic => "powered_organic",
        }
    }

    /// PascalCase identifier the replay `origin_id` enum uses
    /// (origin_shot_force_feedback / concussion schemas).
    pub fn replay_id(self) -> &'static str {
        match self {
            Origin::Human => "Human",
            Origin::Android => "Android",
            Origin::Robot => "Robot",
            Origin::Drone => "Robot",
            Origin::HeavyBiomech => "HeavyBiomech",
            Origin::PoweredOrganic => "PoweredOrganic",
            // The 5 newer races are not in the v0.1 replay enum; they map to
            // their nearest reaction class so the schema stays valid.
            Origin::MethaneBreather
            | Origin::Crystalline
            | Origin::Aqueous
            | Origin::Photosynthetic
            | Origin::Insectoid => "Human",
        }
    }

    /// Tolerant parse — folds FRE / world aliases onto the canonical set.
    pub fn from_str(s: &str) -> Self {
        match s {
            "android" | "android_synthetic" | "hybrid" => Origin::Android,
            "robot" | "robotic_drone" | "synth" => Origin::Robot,
            "drone" | "construct" => Origin::Drone,
            "heavy_biomech" | "biomech" => Origin::HeavyBiomech,
            "powered_organic" | "cyber_human" | "powered" => Origin::PoweredOrganic,
            "methane" | "methane_breather" => Origin::MethaneBreather,
            "crystalline" | "crystalline_helios" | "silica_xenofauna" | "silicon" => Origin::Crystalline,
            "aqueous" | "aqueous_kindred" => Origin::Aqueous,
            "photosynthetic" | "photosynth" => Origin::Photosynthetic,
            "insectoid" | "insectoid_swarm" => Origin::Insectoid,
            _ => Origin::Human,
        }
    }

    pub fn all() -> &'static [Origin] {
        &[
            Origin::Human,
            Origin::Android,
            Origin::Robot,
            Origin::Drone,
            Origin::HeavyBiomech,
            Origin::MethaneBreather,
            Origin::Crystalline,
            Origin::Aqueous,
            Origin::Photosynthetic,
            Origin::Insectoid,
            Origin::PoweredOrganic,
        ]
    }

    /// Fully-synthetic origins (no flesh; routed through internal_shock, not
    /// concussion; INERT on power loss rather than DEAD on blood loss).
    pub fn is_synthetic(self) -> bool {
        matches!(self, Origin::Robot | Origin::Drone)
    }

    /// True for origins whose primary survival resource is power (a depleted
    /// battery offlines them — recoverable, not killed).
    pub fn is_power_survival(self) -> bool {
        matches!(self, Origin::Robot | Origin::Drone | Origin::Crystalline)
    }

    /// True for origins that roll impacts onto internal modules + accrue an
    /// internal-shock dose instead of a concussion dose.
    pub fn uses_internal_shock(self) -> bool {
        matches!(self, Origin::Robot | Origin::Drone | Origin::Crystalline)
    }
}

/// What a shot's body-force-feedback feels like for an origin (the *content*
/// of the always-emitted `origin.shot_force_feedback`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackKind {
    /// Organic pain spike (humans / biomech / the newer organic races).
    PainJolt,
    /// Synthetic servo jolt + chassis frame ring (robots / drones / crystalline).
    ServoJolt,
    /// Hybrid — reduced pain plus a servo jolt (androids / powered organics).
    Hybrid,
}

impl FeedbackKind {
    pub fn as_str(self) -> &'static str {
        match self {
            FeedbackKind::PainJolt => "pain_jolt",
            FeedbackKind::ServoJolt => "servo_jolt",
            FeedbackKind::Hybrid => "hybrid",
        }
    }
}

/// How much of an actor's body is power-dependent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyPowerNeed {
    /// Organic body runs on caloric + blood + oxygen; only equipment needs power.
    None,
    /// Synthetic side needs power; organic side runs without it (androids).
    Partial,
    /// Whole body is power-dependent; empty battery = INERT (robots).
    Full,
}

impl BodyPowerNeed {
    pub fn as_str(self) -> &'static str {
        match self {
            BodyPowerNeed::None => "none",
            BodyPowerNeed::Partial => "partial",
            BodyPowerNeed::Full => "full",
        }
    }
}

/// What an origin breathes (gates oxygen-supply + atmosphere poisoning).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BreathGas {
    None,
    Oxygen,
    Co2,
    Argon,
    Methane,
    /// Versatile — O2 or CO2-rich (insectoid).
    OxygenOrCo2,
    /// Dissolved O2 in a water medium (aqueous).
    DissolvedOxygen,
}

impl BreathGas {
    pub fn as_str(self) -> &'static str {
        match self {
            BreathGas::None => "none",
            BreathGas::Oxygen => "oxygen",
            BreathGas::Co2 => "co2",
            BreathGas::Argon => "argon",
            BreathGas::Methane => "methane",
            BreathGas::OxygenOrCo2 => "oxygen_or_co2",
            BreathGas::DissolvedOxygen => "dissolved_oxygen",
        }
    }
}

/// Per-environmental-factor resistance band. `0.0` = baseline,
/// `+1.0` = immune, negative = extra-vulnerable.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EnvResistance {
    pub radiation: f32,
    pub acid: f32,
    pub electric: f32,
    pub heat: f32,
    pub cold: f32,
    pub toxic: f32,
    pub impact: f32,
}

/// The canonical per-origin reaction + resource profile (the M17 matrix row).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OriginProfile {
    pub origin: Origin,
    // --- survival resources (max / initial; 0 = origin does not use it) ---
    pub blood_max_ml: f32,
    pub oil_max_ml: f32,
    pub bio_fluid_max_ml: f32,
    pub power_max_kwh: f32,
    pub caloric_max: f32,
    pub stamina_max: f32,
    pub oxygen_supply_seconds: f32,
    /// Natural clot rate (mL/s) that offsets bleed/leak (oil never clots → 0).
    pub clot_rate_ml_per_s: f32,
    // --- reaction matrix ---
    pub feedback_kind: FeedbackKind,
    /// True when impacts accumulate g_load + concussion (organic); robots emit
    /// `g_load_delta = 0` always and route to internal_shock instead.
    pub accumulates_g_load: bool,
    /// Concussion susceptibility multiplier (1.0 = full human curve, 0.5 =
    /// android, 0.0 = none).
    pub concussion_susceptibility: f32,
    /// Concussion / internal-shock dose decay per second.
    pub dose_decay_per_s: f32,
    /// Robots / drones / crystalline roll impacts onto internal modules
    /// (internal_shock) rather than stacking a concussion dose.
    pub uses_internal_shock: bool,
    // --- power + oxygen contract ---
    pub body_power_need: BodyPowerNeed,
    pub equipment_needs_power: bool,
    pub breathes: BreathGas,
    pub oxygen_required: bool,
    pub vacuum_immune: bool,
    /// Oxygen is lethal in atmosphere (methane breather).
    pub oxygen_toxic: bool,
    // --- environment envelope ---
    pub temp_min_c: f32,
    pub temp_max_c: f32,
    pub pressure_min_kpa: f32,
    pub pressure_max_kpa: f32,
    pub gravity_min_g: f32,
    pub gravity_max_g: f32,
    pub resist: EnvResistance,
}

impl OriginProfile {
    /// True when this origin has a blood (or bio-fluid) survival pool.
    pub fn has_blood(&self) -> bool {
        self.blood_max_ml > 0.0
    }

    pub fn has_bio_fluid(&self) -> bool {
        self.bio_fluid_max_ml > 0.0
    }

    pub fn has_oil(&self) -> bool {
        self.oil_max_ml > 0.0
    }

    pub fn has_power(&self) -> bool {
        self.power_max_kwh > 0.0
    }

    pub fn has_caloric(&self) -> bool {
        self.caloric_max > 0.0
    }

    /// Seed a fresh [`ResourceAccumulators`] at this origin's full reserves.
    pub fn seed_resources(&self) -> ResourceAccumulators {
        ResourceAccumulators {
            caloric_energy: self.caloric_max,
            battery_charge: if self.has_power() { 100.0 } else { 0.0 },
            power: self.power_max_kwh,
            heat: 0.0,
            oxygen_supply: self.oxygen_supply_seconds,
            g_load_dose: 0.0,
            concussion_dose: 0.0,
            blood: self.blood_max_ml,
            oil: self.oil_max_ml,
            bio_fluid: self.bio_fluid_max_ml,
            internal_shock_dose: 0.0,
        }
    }

    /// The hardcoded canonical profile (per-origin resource matrix, the 10
    /// launch races, and the environment resistance matrix). Boot fallback;
    /// `content/origins/*.json` overrides it.
    pub fn canonical(origin: Origin) -> Self {
        // Shared organic env baseline (overridden per race below).
        let organic = EnvResistance::default();
        match origin {
            Origin::Human => OriginProfile {
                origin,
                blood_max_ml: 5000.0,
                oil_max_ml: 0.0,
                bio_fluid_max_ml: 0.0,
                power_max_kwh: 0.0,
                caloric_max: 100.0,
                stamina_max: 1.0,
                oxygen_supply_seconds: 1800.0,
                clot_rate_ml_per_s: 1.0,
                feedback_kind: FeedbackKind::PainJolt,
                accumulates_g_load: true,
                concussion_susceptibility: 1.0,
                dose_decay_per_s: 5.0,
                uses_internal_shock: false,
                body_power_need: BodyPowerNeed::None,
                equipment_needs_power: true,
                breathes: BreathGas::Oxygen,
                oxygen_required: true,
                vacuum_immune: false,
                oxygen_toxic: false,
                temp_min_c: 18.0,
                temp_max_c: 25.0,
                pressure_min_kpa: 80.0,
                pressure_max_kpa: 110.0,
                gravity_min_g: 0.8,
                gravity_max_g: 1.2,
                resist: organic,
            },
            Origin::Android => OriginProfile {
                origin,
                blood_max_ml: 4000.0,
                oil_max_ml: 3000.0,
                bio_fluid_max_ml: 0.0,
                power_max_kwh: 60.0,
                caloric_max: 50.0,
                stamina_max: 0.7,
                oxygen_supply_seconds: 1800.0,
                clot_rate_ml_per_s: 0.7,
                feedback_kind: FeedbackKind::Hybrid,
                accumulates_g_load: true,
                concussion_susceptibility: 0.5,
                dose_decay_per_s: 5.0,
                uses_internal_shock: false,
                body_power_need: BodyPowerNeed::Partial,
                equipment_needs_power: true,
                breathes: BreathGas::Oxygen,
                oxygen_required: true,
                vacuum_immune: false,
                oxygen_toxic: false,
                temp_min_c: 15.0,
                temp_max_c: 30.0,
                pressure_min_kpa: 70.0,
                pressure_max_kpa: 120.0,
                gravity_min_g: 0.5,
                gravity_max_g: 1.5,
                resist: EnvResistance {
                    acid: 0.3,
                    toxic: 0.3,
                    ..organic
                },
            },
            Origin::Robot => OriginProfile {
                origin,
                blood_max_ml: 0.0,
                oil_max_ml: 5000.0,
                bio_fluid_max_ml: 0.0,
                power_max_kwh: 100.0,
                caloric_max: 0.0,
                stamina_max: 0.0,
                oxygen_supply_seconds: 0.0,
                clot_rate_ml_per_s: 0.0,
                feedback_kind: FeedbackKind::ServoJolt,
                accumulates_g_load: false,
                concussion_susceptibility: 0.0,
                dose_decay_per_s: 2.0,
                uses_internal_shock: true,
                body_power_need: BodyPowerNeed::Full,
                equipment_needs_power: true,
                breathes: BreathGas::None,
                oxygen_required: false,
                vacuum_immune: true,
                oxygen_toxic: false,
                temp_min_c: -50.0,
                temp_max_c: 60.0,
                pressure_min_kpa: 0.0,
                pressure_max_kpa: 200.0,
                gravity_min_g: 0.0,
                gravity_max_g: 3.0,
                resist: EnvResistance {
                    heat: 0.5,
                    cold: 0.5,
                    acid: -0.5,
                    electric: -0.5,
                    toxic: 0.8,
                    ..organic
                },
            },
            Origin::Drone => OriginProfile {
                origin,
                blood_max_ml: 0.0,
                oil_max_ml: 500.0,
                bio_fluid_max_ml: 0.0,
                power_max_kwh: 20.0,
                caloric_max: 0.0,
                stamina_max: 0.0,
                oxygen_supply_seconds: 0.0,
                clot_rate_ml_per_s: 0.0,
                feedback_kind: FeedbackKind::ServoJolt,
                accumulates_g_load: false,
                concussion_susceptibility: 0.0,
                dose_decay_per_s: 2.0,
                uses_internal_shock: true,
                body_power_need: BodyPowerNeed::Full,
                equipment_needs_power: true,
                breathes: BreathGas::None,
                oxygen_required: false,
                vacuum_immune: true,
                oxygen_toxic: false,
                temp_min_c: -40.0,
                temp_max_c: 60.0,
                pressure_min_kpa: 0.0,
                pressure_max_kpa: 200.0,
                gravity_min_g: 0.0,
                gravity_max_g: 3.0,
                resist: EnvResistance {
                    heat: 0.4,
                    cold: 0.4,
                    acid: -0.5,
                    electric: -0.5,
                    toxic: 0.8,
                    ..organic
                },
            },
            Origin::PoweredOrganic => OriginProfile {
                origin,
                blood_max_ml: 5000.0,
                oil_max_ml: 0.0,
                bio_fluid_max_ml: 0.0,
                power_max_kwh: 40.0,
                caloric_max: 100.0,
                stamina_max: 1.0,
                oxygen_supply_seconds: 1800.0,
                clot_rate_ml_per_s: 1.0,
                feedback_kind: FeedbackKind::Hybrid,
                accumulates_g_load: true,
                concussion_susceptibility: 0.85,
                dose_decay_per_s: 5.0,
                uses_internal_shock: false,
                body_power_need: BodyPowerNeed::Partial,
                equipment_needs_power: true,
                breathes: BreathGas::Oxygen,
                oxygen_required: true,
                vacuum_immune: false,
                oxygen_toxic: false,
                temp_min_c: 18.0,
                temp_max_c: 25.0,
                pressure_min_kpa: 80.0,
                pressure_max_kpa: 110.0,
                gravity_min_g: 0.8,
                gravity_max_g: 1.2,
                resist: EnvResistance {
                    impact: 0.2,
                    ..organic
                },
            },
            Origin::HeavyBiomech => OriginProfile {
                origin,
                blood_max_ml: 0.0,
                oil_max_ml: 0.0,
                bio_fluid_max_ml: 8000.0,
                power_max_kwh: 0.0,
                caloric_max: 150.0,
                stamina_max: 1.2,
                oxygen_supply_seconds: 2400.0,
                clot_rate_ml_per_s: 0.3,
                feedback_kind: FeedbackKind::PainJolt,
                accumulates_g_load: true,
                concussion_susceptibility: 0.9,
                dose_decay_per_s: 4.0,
                uses_internal_shock: false,
                body_power_need: BodyPowerNeed::None,
                equipment_needs_power: true,
                breathes: BreathGas::Oxygen,
                oxygen_required: true,
                vacuum_immune: false,
                oxygen_toxic: false,
                temp_min_c: 5.0,
                temp_max_c: 35.0,
                pressure_min_kpa: 50.0,
                pressure_max_kpa: 300.0,
                gravity_min_g: 0.5,
                gravity_max_g: 2.0,
                resist: EnvResistance {
                    impact: 0.3,
                    radiation: 0.2,
                    ..organic
                },
            },
            Origin::Insectoid => OriginProfile {
                origin,
                blood_max_ml: 3500.0,
                oil_max_ml: 0.0,
                bio_fluid_max_ml: 0.0,
                power_max_kwh: 0.0,
                caloric_max: 90.0,
                stamina_max: 1.0,
                oxygen_supply_seconds: 1200.0,
                clot_rate_ml_per_s: 1.5,
                feedback_kind: FeedbackKind::PainJolt,
                accumulates_g_load: true,
                concussion_susceptibility: 0.6,
                dose_decay_per_s: 5.0,
                uses_internal_shock: false,
                body_power_need: BodyPowerNeed::None,
                equipment_needs_power: true,
                breathes: BreathGas::OxygenOrCo2,
                oxygen_required: true,
                vacuum_immune: false,
                oxygen_toxic: false,
                temp_min_c: 5.0,
                temp_max_c: 45.0,
                pressure_min_kpa: 50.0,
                pressure_max_kpa: 200.0,
                gravity_min_g: 0.2,
                gravity_max_g: 2.0,
                resist: EnvResistance {
                    impact: 0.4,
                    cold: -0.5,
                    toxic: 0.3,
                    ..organic
                },
            },
            Origin::Crystalline => OriginProfile {
                origin,
                blood_max_ml: 0.0,
                oil_max_ml: 0.0,
                bio_fluid_max_ml: 0.0,
                power_max_kwh: 80.0,
                caloric_max: 0.0,
                stamina_max: 0.0,
                oxygen_supply_seconds: 0.0,
                clot_rate_ml_per_s: 0.0,
                feedback_kind: FeedbackKind::ServoJolt,
                accumulates_g_load: false,
                concussion_susceptibility: 0.0,
                dose_decay_per_s: 2.0,
                uses_internal_shock: true,
                body_power_need: BodyPowerNeed::Full,
                equipment_needs_power: true,
                breathes: BreathGas::Argon,
                oxygen_required: false,
                vacuum_immune: true,
                oxygen_toxic: false,
                temp_min_c: -100.0,
                temp_max_c: 400.0,
                pressure_min_kpa: 0.0,
                pressure_max_kpa: 1000.0,
                gravity_min_g: 0.0,
                gravity_max_g: 5.0,
                resist: EnvResistance {
                    radiation: 1.0,
                    electric: 1.0,
                    acid: -0.7,
                    impact: -0.5,
                    heat: 0.6,
                    cold: 0.6,
                    ..organic
                },
            },
            Origin::Photosynthetic => OriginProfile {
                origin,
                blood_max_ml: 4000.0,
                oil_max_ml: 0.0,
                bio_fluid_max_ml: 0.0,
                power_max_kwh: 0.0,
                caloric_max: 100.0,
                stamina_max: 0.8,
                oxygen_supply_seconds: 1500.0,
                clot_rate_ml_per_s: 0.8,
                feedback_kind: FeedbackKind::PainJolt,
                accumulates_g_load: true,
                concussion_susceptibility: 0.6,
                dose_decay_per_s: 4.0,
                uses_internal_shock: false,
                body_power_need: BodyPowerNeed::None,
                equipment_needs_power: true,
                breathes: BreathGas::Co2,
                oxygen_required: true,
                vacuum_immune: false,
                oxygen_toxic: false,
                temp_min_c: 10.0,
                temp_max_c: 35.0,
                pressure_min_kpa: 50.0,
                pressure_max_kpa: 150.0,
                gravity_min_g: 0.3,
                gravity_max_g: 1.5,
                resist: EnvResistance {
                    radiation: 1.0,
                    toxic: 0.9,
                    cold: -0.5,
                    ..organic
                },
            },
            Origin::Aqueous => OriginProfile {
                origin,
                blood_max_ml: 4500.0,
                oil_max_ml: 0.0,
                bio_fluid_max_ml: 0.0,
                power_max_kwh: 0.0,
                caloric_max: 100.0,
                stamina_max: 1.0,
                oxygen_supply_seconds: 1500.0,
                clot_rate_ml_per_s: 1.0,
                feedback_kind: FeedbackKind::PainJolt,
                accumulates_g_load: true,
                concussion_susceptibility: 0.6,
                dose_decay_per_s: 5.0,
                uses_internal_shock: false,
                body_power_need: BodyPowerNeed::None,
                equipment_needs_power: true,
                breathes: BreathGas::DissolvedOxygen,
                oxygen_required: true,
                vacuum_immune: false,
                oxygen_toxic: false,
                temp_min_c: 5.0,
                temp_max_c: 25.0,
                pressure_min_kpa: 50.0,
                pressure_max_kpa: 500.0,
                gravity_min_g: 0.0,
                gravity_max_g: 2.0,
                resist: EnvResistance {
                    cold: 0.5,
                    heat: -0.5,
                    ..organic
                },
            },
            Origin::MethaneBreather => OriginProfile {
                origin,
                blood_max_ml: 4000.0,
                oil_max_ml: 0.0,
                bio_fluid_max_ml: 0.0,
                power_max_kwh: 0.0,
                caloric_max: 100.0,
                stamina_max: 1.0,
                oxygen_supply_seconds: 1800.0,
                clot_rate_ml_per_s: 1.0,
                feedback_kind: FeedbackKind::PainJolt,
                accumulates_g_load: true,
                concussion_susceptibility: 1.0,
                dose_decay_per_s: 5.0,
                uses_internal_shock: false,
                body_power_need: BodyPowerNeed::None,
                equipment_needs_power: true,
                breathes: BreathGas::Methane,
                oxygen_required: false,
                vacuum_immune: true,
                oxygen_toxic: true,
                temp_min_c: -200.0,
                temp_max_c: -100.0,
                pressure_min_kpa: 50.0,
                pressure_max_kpa: 200.0,
                gravity_min_g: 0.01,
                gravity_max_g: 1.0,
                resist: EnvResistance {
                    cold: 1.0,
                    heat: -0.7,
                    ..organic
                },
            },
        }
    }
}

/// Per-origin profile registry: the 11 canonical rows, overridable from
/// `content/origins/*.json`. Lookup is O(1) by discriminant.
#[derive(Debug, Clone)]
pub struct OriginRegistry {
    profiles: [OriginProfile; 11],
}

impl Default for OriginRegistry {
    fn default() -> Self {
        Self::canonical()
    }
}

impl OriginRegistry {
    /// The hardcoded canonical registry (boot fallback).
    pub fn canonical() -> Self {
        let all = Origin::all();
        let mut profiles = [OriginProfile::canonical(Origin::Human); 11];
        for (i, &o) in all.iter().enumerate() {
            profiles[i] = OriginProfile::canonical(o);
        }
        Self { profiles }
    }

    pub fn profile(&self, origin: Origin) -> &OriginProfile {
        // `Origin` discriminants are 0..=10, matching the array index.
        &self.profiles[origin as usize]
    }

    pub fn profile_for_id(&self, origin_id: &str) -> &OriginProfile {
        self.profile(Origin::from_str(origin_id))
    }

    /// Replace one origin's profile (used by the content loader / mods).
    pub fn set_profile(&mut self, profile: OriginProfile) {
        let idx = profile.origin as usize;
        if idx < self.profiles.len() {
            self.profiles[idx] = profile;
        }
    }

    /// Overlay per-origin profiles from `content/origins/*.json` onto the
    /// canonical registry. Missing dir → canonical (boot fallback). Each JSON
    /// file is one full [`OriginProfile`]; `_`-prefixed files are skipped.
    /// `tracing::warn!`s and continues on a per-file parse failure (never
    /// silently drops a malformed override).
    pub fn load_dir(dir: &std::path::Path) -> Self {
        let mut reg = Self::canonical();
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return reg,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with('_'))
            {
                continue;
            }
            match std::fs::read_to_string(&path) {
                Ok(text) => match serde_json::from_str::<OriginProfile>(&text) {
                    Ok(profile) => reg.set_profile(profile),
                    Err(err) => tracing::warn!(
                        target: "cf_actor::origin",
                        ?path,
                        %err,
                        "origin profile parse failed; keeping canonical"
                    ),
                },
                Err(err) => tracing::warn!(
                    target: "cf_actor::origin",
                    ?path,
                    %err,
                    "origin profile read failed; keeping canonical"
                ),
            }
        }
        reg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eleven_origins_round_trip() {
        assert_eq!(Origin::all().len(), 11);
        for &o in Origin::all() {
            assert_eq!(Origin::from_str(o.as_str()), o);
        }
        assert_eq!(Origin::from_str("synth"), Origin::Robot);
        assert_eq!(Origin::from_str("cyber_human"), Origin::PoweredOrganic);
        assert_eq!(Origin::from_str("unknown"), Origin::Human);
    }

    #[test]
    fn resource_matrix_matches_spec() {
        let human = OriginProfile::canonical(Origin::Human);
        assert!(human.has_blood() && !human.has_oil() && !human.has_power());
        assert_eq!(human.blood_max_ml, 5000.0);

        let robot = OriginProfile::canonical(Origin::Robot);
        assert!(!robot.has_blood() && robot.has_oil() && robot.has_power());
        assert_eq!(robot.power_max_kwh, 100.0);
        assert!(robot.uses_internal_shock && !robot.accumulates_g_load);
        assert_eq!(robot.concussion_susceptibility, 0.0);

        let android = OriginProfile::canonical(Origin::Android);
        assert!(android.has_blood() && android.has_oil() && android.has_power());
        assert_eq!(android.concussion_susceptibility, 0.5);

        let biomech = OriginProfile::canonical(Origin::HeavyBiomech);
        assert!(biomech.has_bio_fluid());
        assert_eq!(biomech.bio_fluid_max_ml, 8000.0);
    }

    #[test]
    fn seed_resources_fills_only_live_pools() {
        let r = OriginProfile::canonical(Origin::Robot).seed_resources();
        assert_eq!(r.blood, 0.0);
        assert_eq!(r.oil, 5000.0);
        assert_eq!(r.power, 100.0);
        let h = OriginProfile::canonical(Origin::Human).seed_resources();
        assert_eq!(h.blood, 5000.0);
        assert_eq!(h.oil, 0.0);
        assert_eq!(h.power, 0.0);
    }

    #[test]
    fn breathing_and_vacuum_contract() {
        assert!(OriginProfile::canonical(Origin::Robot).vacuum_immune);
        assert!(OriginProfile::canonical(Origin::MethaneBreather).oxygen_toxic);
        assert!(OriginProfile::canonical(Origin::MethaneBreather).vacuum_immune);
        assert!(OriginProfile::canonical(Origin::Human).oxygen_required);
        assert!(!OriginProfile::canonical(Origin::Robot).oxygen_required);
        assert_eq!(
            OriginProfile::canonical(Origin::Crystalline).resist.radiation,
            1.0
        );
    }

    #[test]
    fn registry_lookup_is_stable() {
        let reg = OriginRegistry::canonical();
        for &o in Origin::all() {
            assert_eq!(reg.profile(o).origin, o);
        }
    }

    #[test]
    fn canonical_profiles_round_trip_through_json() {
        for &o in Origin::all() {
            let p = OriginProfile::canonical(o);
            let json = serde_json::to_string(&p).unwrap();
            let back: OriginProfile = serde_json::from_str(&json).unwrap();
            assert_eq!(p, back);
        }
    }

    #[test]
    #[ignore = "content emitter — run explicitly to (re)generate content/origins/*.json"]
    fn emit_origin_content() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("content")
            .join("origins");
        std::fs::create_dir_all(&root).unwrap();
        for &o in Origin::all() {
            let p = OriginProfile::canonical(o);
            let json = serde_json::to_string_pretty(&p).unwrap();
            let path = root.join(format!("{}.json", o.as_str()));
            std::fs::write(&path, json + "\n").unwrap();
        }
    }
}
