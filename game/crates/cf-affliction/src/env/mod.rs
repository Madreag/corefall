//! M16A § Environment-driven affliction depth layer.
//!
//! 11 launch env-driven afflictions: stuffiness, heatstroke, hypothermia,
//! asphyxiation, refrigerant_inhalation, electrocution, illuminated,
//! laceration, trench_foot, stamina_movement_cost, panic_freeze_env.
//!
//! Each kind owns an accumulator + threshold transition + per-tick effect
//! + clear condition. The kernel is deterministic: per-actor-id ASC then
//!   kind-enum order; no HashMap iteration; no `thread_rng`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub mod asphyxiation;
pub mod electrocution;
pub mod heatstroke;
pub mod hypothermia;
pub mod illuminated;
pub mod laceration;
pub mod panic_freeze_env;
pub mod refrigerant_inhalation;
pub mod stamina_movement_cost;
pub mod stuffiness;
pub mod trench_foot;

/// 11 env-driven affliction kinds per M16A spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvAfflictionKind {
    Stuffiness,
    Heatstroke,
    Hypothermia,
    Asphyxiation,
    RefrigerantInhalation,
    Electrocution,
    Illuminated,
    Laceration,
    TrenchFoot,
    StaminaMovementCost,
    PanicFreezeEnv,
}

impl EnvAfflictionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EnvAfflictionKind::Stuffiness => "stuffiness",
            EnvAfflictionKind::Heatstroke => "heatstroke",
            EnvAfflictionKind::Hypothermia => "hypothermia",
            EnvAfflictionKind::Asphyxiation => "asphyxiation",
            EnvAfflictionKind::RefrigerantInhalation => "refrigerant_inhalation",
            EnvAfflictionKind::Electrocution => "electrocution",
            EnvAfflictionKind::Illuminated => "illuminated",
            EnvAfflictionKind::Laceration => "laceration",
            EnvAfflictionKind::TrenchFoot => "trench_foot",
            EnvAfflictionKind::StaminaMovementCost => "stamina_movement_cost",
            EnvAfflictionKind::PanicFreezeEnv => "panic_freeze_env",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "stuffiness" => EnvAfflictionKind::Stuffiness,
            "heatstroke" => EnvAfflictionKind::Heatstroke,
            "hypothermia" => EnvAfflictionKind::Hypothermia,
            "asphyxiation" => EnvAfflictionKind::Asphyxiation,
            "refrigerant_inhalation" => EnvAfflictionKind::RefrigerantInhalation,
            "electrocution" => EnvAfflictionKind::Electrocution,
            "illuminated" => EnvAfflictionKind::Illuminated,
            "laceration" => EnvAfflictionKind::Laceration,
            "trench_foot" => EnvAfflictionKind::TrenchFoot,
            "stamina_movement_cost" => EnvAfflictionKind::StaminaMovementCost,
            "panic_freeze_env" => EnvAfflictionKind::PanicFreezeEnv,
            _ => return None,
        })
    }

    pub fn all() -> &'static [EnvAfflictionKind] {
        &[
            EnvAfflictionKind::Stuffiness,
            EnvAfflictionKind::Heatstroke,
            EnvAfflictionKind::Hypothermia,
            EnvAfflictionKind::Asphyxiation,
            EnvAfflictionKind::RefrigerantInhalation,
            EnvAfflictionKind::Electrocution,
            EnvAfflictionKind::Illuminated,
            EnvAfflictionKind::Laceration,
            EnvAfflictionKind::TrenchFoot,
            EnvAfflictionKind::StaminaMovementCost,
            EnvAfflictionKind::PanicFreezeEnv,
        ]
    }
}

/// Severity bands per spec § "color-coded by severity (yellow / orange / red)".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvSeverity {
    None,
    Mild,
    Moderate,
    Severe,
    Lethal,
}

impl EnvSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            EnvSeverity::None => "none",
            EnvSeverity::Mild => "mild",
            EnvSeverity::Moderate => "moderate",
            EnvSeverity::Severe => "severe",
            EnvSeverity::Lethal => "lethal",
        }
    }

    pub fn from_severity_0_1(severity: f32) -> Self {
        if severity <= 0.0 {
            EnvSeverity::None
        } else if severity < 0.4 {
            EnvSeverity::Mild
        } else if severity < 0.7 {
            EnvSeverity::Moderate
        } else if severity < 1.0 {
            EnvSeverity::Severe
        } else {
            EnvSeverity::Lethal
        }
    }
}

/// M16A consumer slice of the M17 origin susceptibility matrix per spec
/// § "Per-origin susceptibility". `multiplier == 0.0` short-circuits the
/// accumulator at function entry (immunity); `multiplier == 1.0` is the
/// baseline human rate.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AtmosphericSusceptibility {
    pub origin_id: OriginId,
    pub stuffiness_multiplier: f32,
    pub heat_comfort_max_k: f32,
    pub cold_comfort_min_k: f32,
    pub asphyxiation_ttd_s: f32,
    pub refrigerant_inhalation_multiplier: f32,
    pub electrocution_resistance: f32,
    pub trench_foot_multiplier: f32,
    pub stamina_cost_multiplier: f32,
    pub oxygen_toxic: bool,
}

/// M17 origin identifier. Mirrors `cf_affliction::Race` plus the methane
/// breather slot used by the M16A spec scenarios.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginId {
    Human,
    MethaneBreather,
    Crystalline,
    Aqueous,
    Robot,
    Android,
}

impl OriginId {
    pub fn as_str(self) -> &'static str {
        match self {
            OriginId::Human => "human",
            OriginId::MethaneBreather => "methane_breather",
            OriginId::Crystalline => "crystalline",
            OriginId::Aqueous => "aqueous",
            OriginId::Robot => "robot",
            OriginId::Android => "android",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "methane" | "methane_breather" => OriginId::MethaneBreather,
            "crystalline" => OriginId::Crystalline,
            "aqueous" => OriginId::Aqueous,
            "robot" | "synth" => OriginId::Robot,
            "android" | "hybrid" => OriginId::Android,
            _ => OriginId::Human,
        }
    }
}

impl AtmosphericSusceptibility {
    /// M17 atmospheric slice for the 6 launch origins.
    pub fn for_origin(origin_id: OriginId) -> Self {
        match origin_id {
            OriginId::Human => Self {
                origin_id,
                stuffiness_multiplier: 1.0,
                heat_comfort_max_k: 322.0,
                cold_comfort_min_k: 273.0,
                asphyxiation_ttd_s: 30.0,
                refrigerant_inhalation_multiplier: 1.0,
                electrocution_resistance: 0.0,
                trench_foot_multiplier: 1.0,
                stamina_cost_multiplier: 1.0,
                oxygen_toxic: false,
            },
            OriginId::MethaneBreather => Self {
                origin_id,
                stuffiness_multiplier: 0.5,
                heat_comfort_max_k: 280.0,
                cold_comfort_min_k: 90.0,
                asphyxiation_ttd_s: f32::INFINITY,
                refrigerant_inhalation_multiplier: 1.2,
                electrocution_resistance: 0.2,
                trench_foot_multiplier: 1.0,
                stamina_cost_multiplier: 1.0,
                oxygen_toxic: true,
            },
            OriginId::Crystalline => Self {
                origin_id,
                stuffiness_multiplier: 0.4,
                heat_comfort_max_k: 423.0,
                cold_comfort_min_k: 200.0,
                asphyxiation_ttd_s: 120.0,
                refrigerant_inhalation_multiplier: 0.5,
                electrocution_resistance: 0.4,
                trench_foot_multiplier: 0.2,
                stamina_cost_multiplier: 0.8,
                oxygen_toxic: false,
            },
            OriginId::Aqueous => Self {
                origin_id,
                stuffiness_multiplier: 1.2,
                heat_comfort_max_k: 310.0,
                cold_comfort_min_k: 263.0,
                asphyxiation_ttd_s: 60.0,
                refrigerant_inhalation_multiplier: 1.0,
                electrocution_resistance: -0.5,
                trench_foot_multiplier: 0.0,
                stamina_cost_multiplier: 1.0,
                oxygen_toxic: false,
            },
            OriginId::Robot => Self {
                origin_id,
                stuffiness_multiplier: 0.0,
                heat_comfort_max_k: 450.0,
                cold_comfort_min_k: 90.0,
                asphyxiation_ttd_s: f32::INFINITY,
                refrigerant_inhalation_multiplier: 0.0,
                electrocution_resistance: 0.6,
                trench_foot_multiplier: 0.3,
                stamina_cost_multiplier: 0.6,
                oxygen_toxic: false,
            },
            OriginId::Android => Self {
                origin_id,
                stuffiness_multiplier: 0.7,
                heat_comfort_max_k: 350.0,
                cold_comfort_min_k: 250.0,
                asphyxiation_ttd_s: 90.0,
                refrigerant_inhalation_multiplier: 0.6,
                electrocution_resistance: 0.3,
                trench_foot_multiplier: 0.6,
                stamina_cost_multiplier: 0.9,
                oxygen_toxic: false,
            },
        }
    }
}

/// Aggregated per-tick environment signals consumed by the M16A kernel.
/// Mirrors the atmospheric / thermal / em / electric / wet slices of
/// M20's EnvironmentSignal — M16A is a *consumer*, never a producer.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EnvSignal {
    pub humidity_pct: f32,
    pub co2_partial_kpa: f32,
    pub o2_partial_kpa: f32,
    pub occupant_count: u32,
    pub room_temp_k: f32,
    pub refrigerant_partial_kpa: f32,
    pub electric_shock_event_j: f32,
    pub spotlight_lit: bool,
    pub razor_wire_contact: bool,
    pub bladed_hit_severity: f32,
    pub wet_duckboard_contact: bool,
    pub feet_dry_and_warm: bool,
    pub heavy_weapon_kg: f32,
    pub baseline_carry_kg: f32,
    pub analyzer_alarm_unaddressed: bool,
    pub extreme_breach_event: bool,
    pub stabilize_assist: bool,
}

/// Per-affliction accumulator state. Each kind owns its own integrator
/// (seconds * value, count, partial-pressure-seconds, etc.).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct EnvAccumulator {
    pub kind_value: f32,
    pub arc_count: u32,
    pub knockdown_ticks_remaining: u32,
    pub stunned_ticks_remaining: u32,
    pub panic_freeze_ticks_remaining: u32,
    pub cooldown_seconds: f32,
    pub bleed_stack: u32,
}

/// Per-actor M16A env affliction state.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EnvAfflictionState {
    pub accumulators: BTreeMap<String, EnvAccumulator>,
    pub severities: BTreeMap<String, f32>,
    pub last_threshold: BTreeMap<String, EnvSeverity>,
    /// Origin-id immune kinds (one-shot emit). Marker is "kind:reason".
    pub immune_emitted: BTreeMap<String, bool>,
    pub bleed_stack_total: u32,
    pub m16b_sepsis_feed: bool,
}

impl EnvAfflictionState {
    pub fn accumulator(&self, kind: EnvAfflictionKind) -> EnvAccumulator {
        self.accumulators.get(kind.as_str()).copied().unwrap_or_default()
    }

    pub fn severity(&self, kind: EnvAfflictionKind) -> f32 {
        self.severities.get(kind.as_str()).copied().unwrap_or(0.0)
    }

    pub fn last_threshold(&self, kind: EnvAfflictionKind) -> EnvSeverity {
        self.last_threshold
            .get(kind.as_str())
            .copied()
            .unwrap_or(EnvSeverity::None)
    }

    pub fn set_accumulator(&mut self, kind: EnvAfflictionKind, value: EnvAccumulator) {
        self.accumulators.insert(kind.as_str().to_string(), value);
    }

    pub fn set_severity(&mut self, kind: EnvAfflictionKind, severity: f32) {
        self.severities
            .insert(kind.as_str().to_string(), severity.clamp(0.0, 1.0));
    }

    pub fn set_last_threshold(&mut self, kind: EnvAfflictionKind, band: EnvSeverity) {
        self.last_threshold
            .insert(kind.as_str().to_string(), band);
    }

    pub fn clear(&mut self, kind: EnvAfflictionKind) {
        self.accumulators.remove(kind.as_str());
        self.severities.remove(kind.as_str());
        self.last_threshold.remove(kind.as_str());
    }

    pub fn mark_immune(&mut self, kind: EnvAfflictionKind, reason: &str) -> bool {
        let key = format!("{}:{}", kind.as_str(), reason);
        if self.immune_emitted.contains_key(&key) {
            return false;
        }
        self.immune_emitted.insert(key, true);
        true
    }
}

/// Per-kind tuning parameters loaded from `content/afflictions/env/<kind>.ron`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EnvAfflictionSpec {
    pub kind: EnvAfflictionKind,
    pub mild_threshold: f32,
    pub moderate_threshold: f32,
    pub severe_threshold: f32,
    pub lethal_threshold: f32,
    pub accumulator_rate_per_s: f32,
    pub decay_per_s: f32,
    pub hp_per_second_at_threshold: f32,
    pub speed_multiplier: f32,
    pub aim_wobble_multiplier: f32,
    pub stamina_drain_multiplier: f32,
    pub clear_cooldown_s: f32,
    pub feeds_m16b_sepsis: bool,
}

impl EnvAfflictionSpec {
    pub fn default_for(kind: EnvAfflictionKind) -> Self {
        match kind {
            EnvAfflictionKind::Stuffiness => Self {
                kind,
                mild_threshold: 600.0,
                moderate_threshold: 1800.0,
                severe_threshold: 3600.0,
                lethal_threshold: f32::INFINITY,
                accumulator_rate_per_s: 1.0,
                decay_per_s: 0.5,
                hp_per_second_at_threshold: 0.0,
                speed_multiplier: 1.0,
                aim_wobble_multiplier: 1.0,
                stamina_drain_multiplier: 1.2,
                clear_cooldown_s: 0.0,
                feeds_m16b_sepsis: false,
            },
            EnvAfflictionKind::Heatstroke => Self {
                kind,
                mild_threshold: 300.0,
                moderate_threshold: 600.0,
                severe_threshold: 900.0,
                lethal_threshold: 1800.0,
                accumulator_rate_per_s: 1.0,
                decay_per_s: 0.5,
                hp_per_second_at_threshold: 1.0 / 30.0,
                speed_multiplier: 0.85,
                aim_wobble_multiplier: 1.4,
                stamina_drain_multiplier: 1.1,
                clear_cooldown_s: 300.0,
                feeds_m16b_sepsis: false,
            },
            EnvAfflictionKind::Hypothermia => Self {
                kind,
                mild_threshold: 300.0,
                moderate_threshold: 600.0,
                severe_threshold: 900.0,
                lethal_threshold: 1800.0,
                accumulator_rate_per_s: 1.0,
                decay_per_s: 0.5,
                hp_per_second_at_threshold: 1.0 / 60.0,
                speed_multiplier: 0.85,
                aim_wobble_multiplier: 1.4,
                stamina_drain_multiplier: 1.1,
                clear_cooldown_s: 300.0,
                feeds_m16b_sepsis: false,
            },
            EnvAfflictionKind::Asphyxiation => Self {
                kind,
                mild_threshold: 30.0,
                moderate_threshold: 60.0,
                severe_threshold: 80.0,
                lethal_threshold: 90.0,
                accumulator_rate_per_s: 1.0,
                decay_per_s: 2.0,
                hp_per_second_at_threshold: 2.0,
                speed_multiplier: 0.7,
                aim_wobble_multiplier: 1.6,
                stamina_drain_multiplier: 1.5,
                clear_cooldown_s: 0.0,
                feeds_m16b_sepsis: false,
            },
            EnvAfflictionKind::RefrigerantInhalation => Self {
                kind,
                mild_threshold: 50.0,
                moderate_threshold: 120.0,
                severe_threshold: 200.0,
                lethal_threshold: 400.0,
                accumulator_rate_per_s: 1.0,
                decay_per_s: 0.5,
                hp_per_second_at_threshold: 0.1,
                speed_multiplier: 0.9,
                aim_wobble_multiplier: 1.6,
                stamina_drain_multiplier: 1.2,
                clear_cooldown_s: 600.0,
                feeds_m16b_sepsis: true,
            },
            EnvAfflictionKind::Electrocution => Self {
                kind,
                mild_threshold: 1.0,
                moderate_threshold: 2.0,
                severe_threshold: 3.0,
                lethal_threshold: 5.0,
                accumulator_rate_per_s: 0.0,
                decay_per_s: 0.0,
                hp_per_second_at_threshold: 0.5,
                speed_multiplier: 1.0,
                aim_wobble_multiplier: 1.0,
                stamina_drain_multiplier: 1.0,
                clear_cooldown_s: 0.0,
                feeds_m16b_sepsis: false,
            },
            EnvAfflictionKind::Illuminated => Self {
                kind,
                mild_threshold: 0.0,
                moderate_threshold: 0.0,
                severe_threshold: 0.0,
                lethal_threshold: f32::INFINITY,
                accumulator_rate_per_s: 1.0,
                decay_per_s: 0.0,
                hp_per_second_at_threshold: 0.0,
                speed_multiplier: 1.0,
                aim_wobble_multiplier: 1.0,
                stamina_drain_multiplier: 1.0,
                clear_cooldown_s: 0.0,
                feeds_m16b_sepsis: false,
            },
            EnvAfflictionKind::Laceration => Self {
                kind,
                mild_threshold: 1.0,
                moderate_threshold: 3.0,
                severe_threshold: 5.0,
                lethal_threshold: 8.0,
                accumulator_rate_per_s: 0.0,
                decay_per_s: 0.0,
                hp_per_second_at_threshold: 1.0 / 60.0,
                speed_multiplier: 1.0,
                aim_wobble_multiplier: 1.0,
                stamina_drain_multiplier: 1.0,
                clear_cooldown_s: 30.0,
                feeds_m16b_sepsis: true,
            },
            EnvAfflictionKind::TrenchFoot => Self {
                kind,
                mild_threshold: 7200.0,
                moderate_threshold: 14400.0,
                severe_threshold: 21600.0,
                lethal_threshold: f32::INFINITY,
                accumulator_rate_per_s: 1.0,
                decay_per_s: 1.0,
                hp_per_second_at_threshold: 0.0,
                speed_multiplier: 0.9,
                aim_wobble_multiplier: 1.0,
                stamina_drain_multiplier: 1.0,
                clear_cooldown_s: 86400.0,
                feeds_m16b_sepsis: true,
            },
            EnvAfflictionKind::StaminaMovementCost => Self {
                kind,
                mild_threshold: 0.0,
                moderate_threshold: 100.0,
                severe_threshold: 250.0,
                lethal_threshold: f32::INFINITY,
                accumulator_rate_per_s: 1.0,
                decay_per_s: 0.0,
                hp_per_second_at_threshold: 0.0,
                speed_multiplier: 0.95,
                aim_wobble_multiplier: 1.0,
                stamina_drain_multiplier: 1.5,
                clear_cooldown_s: 0.0,
                feeds_m16b_sepsis: false,
            },
            EnvAfflictionKind::PanicFreezeEnv => Self {
                kind,
                mild_threshold: 1.0,
                moderate_threshold: 1.0,
                severe_threshold: 1.0,
                lethal_threshold: 1.0,
                accumulator_rate_per_s: 0.0,
                decay_per_s: 0.0,
                hp_per_second_at_threshold: 0.0,
                speed_multiplier: 0.0,
                aim_wobble_multiplier: 1.0,
                stamina_drain_multiplier: 1.0,
                clear_cooldown_s: 5.0,
                feeds_m16b_sepsis: false,
            },
        }
    }
}

/// Registry of env affliction specs, keyed by kind id.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvAfflictionRegistry {
    pub specs: BTreeMap<String, EnvAfflictionSpec>,
}

impl EnvAfflictionRegistry {
    pub fn default_registry() -> Self {
        let mut specs = BTreeMap::new();
        for k in EnvAfflictionKind::all().iter().copied() {
            specs.insert(k.as_str().to_string(), EnvAfflictionSpec::default_for(k));
        }
        Self { specs }
    }

    pub fn lookup(&self, kind: EnvAfflictionKind) -> EnvAfflictionSpec {
        self.specs
            .get(kind.as_str())
            .copied()
            .unwrap_or_else(|| EnvAfflictionSpec::default_for(kind))
    }
}

/// Reason carried on `affliction.env_cleared`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvClearReason {
    ConditionCleared,
    CooldownElapsed,
    SquadmateStabilized,
    Death,
}

impl EnvClearReason {
    pub fn as_str(self) -> &'static str {
        match self {
            EnvClearReason::ConditionCleared => "condition_cleared",
            EnvClearReason::CooldownElapsed => "cooldown_elapsed",
            EnvClearReason::SquadmateStabilized => "squadmate_stabilized",
            EnvClearReason::Death => "death",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvThresholdCrossedEvent {
    pub actor_id: u64,
    pub kind: EnvAfflictionKind,
    pub severity: EnvSeverity,
    pub severity_0_1: f32,
    pub accumulator_value: f32,
    pub source_event_id: Option<String>,
    pub origin_id: OriginId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvSeverityChangedEvent {
    pub actor_id: u64,
    pub kind: EnvAfflictionKind,
    pub from_severity: f32,
    pub to_severity: f32,
    pub accumulator_value: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvClearedEvent {
    pub actor_id: u64,
    pub kind: EnvAfflictionKind,
    pub reason: EnvClearReason,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvOriginImmuneEvent {
    pub actor_id: u64,
    pub kind: EnvAfflictionKind,
    pub origin_id: OriginId,
    pub reason: String,
    pub alt_kind: Option<EnvAfflictionKind>,
}

/// One tick of producer output across all 11 env afflictions for a single actor.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EnvTickOutput {
    pub threshold_crossed: Vec<EnvThresholdCrossedEvent>,
    pub severity_changed: Vec<EnvSeverityChangedEvent>,
    pub cleared: Vec<EnvClearedEvent>,
    pub origin_immune: Vec<EnvOriginImmuneEvent>,
    pub hp_damage: f32,
    pub speed_multiplier: f32,
    pub aim_wobble_multiplier: f32,
    pub stamina_drain_multiplier: f32,
    pub knockdown_ticks: u32,
    pub panic_freeze_ticks: u32,
    pub reveal_to_ai: bool,
    pub m16b_sepsis_feed: bool,
}

impl EnvTickOutput {
    fn new() -> Self {
        Self {
            speed_multiplier: 1.0,
            aim_wobble_multiplier: 1.0,
            stamina_drain_multiplier: 1.0,
            ..Default::default()
        }
    }
}

/// Run one M16A env affliction tick across all 11 kinds for `actor_id`.
/// Each kind's per-kind module owns its accumulator math; this entry
/// point batches the calls and aggregates the output. Per spec § "all 11
/// affliction accumulators share a single tick_all entry point — runs at
/// 1/N=5 ticks cadence (perf-bound)".
pub fn tick_all(
    state: &mut EnvAfflictionState,
    actor_id: u64,
    susceptibility: AtmosphericSusceptibility,
    signal: &EnvSignal,
    registry: &EnvAfflictionRegistry,
    dt_seconds: f32,
    source_event_id: Option<String>,
) -> EnvTickOutput {
    let mut out = EnvTickOutput::new();
    stuffiness::tick(state, actor_id, &susceptibility, signal, &registry.lookup(EnvAfflictionKind::Stuffiness), dt_seconds, source_event_id.clone(), &mut out);
    heatstroke::tick(state, actor_id, &susceptibility, signal, &registry.lookup(EnvAfflictionKind::Heatstroke), dt_seconds, source_event_id.clone(), &mut out);
    hypothermia::tick(state, actor_id, &susceptibility, signal, &registry.lookup(EnvAfflictionKind::Hypothermia), dt_seconds, source_event_id.clone(), &mut out);
    asphyxiation::tick(state, actor_id, &susceptibility, signal, &registry.lookup(EnvAfflictionKind::Asphyxiation), dt_seconds, source_event_id.clone(), &mut out);
    refrigerant_inhalation::tick(state, actor_id, &susceptibility, signal, &registry.lookup(EnvAfflictionKind::RefrigerantInhalation), dt_seconds, source_event_id.clone(), &mut out);
    electrocution::tick(state, actor_id, &susceptibility, signal, &registry.lookup(EnvAfflictionKind::Electrocution), dt_seconds, source_event_id.clone(), &mut out);
    illuminated::tick(state, actor_id, &susceptibility, signal, &registry.lookup(EnvAfflictionKind::Illuminated), dt_seconds, source_event_id.clone(), &mut out);
    laceration::tick(state, actor_id, &susceptibility, signal, &registry.lookup(EnvAfflictionKind::Laceration), dt_seconds, source_event_id.clone(), &mut out);
    trench_foot::tick(state, actor_id, &susceptibility, signal, &registry.lookup(EnvAfflictionKind::TrenchFoot), dt_seconds, source_event_id.clone(), &mut out);
    stamina_movement_cost::tick(state, actor_id, &susceptibility, signal, &registry.lookup(EnvAfflictionKind::StaminaMovementCost), dt_seconds, source_event_id.clone(), &mut out);
    panic_freeze_env::tick(state, actor_id, &susceptibility, signal, &registry.lookup(EnvAfflictionKind::PanicFreezeEnv), dt_seconds, source_event_id, &mut out);
    out.m16b_sepsis_feed = state.m16b_sepsis_feed;
    out
}

/// Shared helper: record severity transitions and threshold crossings on a
/// per-kind accumulator value. Returns the produced events into `out`.
pub(crate) fn evaluate_threshold(
    state: &mut EnvAfflictionState,
    actor_id: u64,
    kind: EnvAfflictionKind,
    accumulator: f32,
    spec: &EnvAfflictionSpec,
    origin_id: OriginId,
    source_event_id: Option<String>,
    out: &mut EnvTickOutput,
) -> f32 {
    let severity_0_1 = severity_from_thresholds(accumulator, spec).clamp(0.0, 1.0);
    let from_severity = state.severity(kind);
    if (severity_0_1 - from_severity).abs() > f32::EPSILON {
        state.set_severity(kind, severity_0_1);
        out.severity_changed.push(EnvSeverityChangedEvent {
            actor_id,
            kind,
            from_severity,
            to_severity: severity_0_1,
            accumulator_value: accumulator,
        });
    }
    let band = EnvSeverity::from_severity_0_1(severity_0_1);
    let last = state.last_threshold(kind);
    if band > last && band != EnvSeverity::None {
        state.set_last_threshold(kind, band);
        out.threshold_crossed.push(EnvThresholdCrossedEvent {
            actor_id,
            kind,
            severity: band,
            severity_0_1,
            accumulator_value: accumulator,
            source_event_id: source_event_id.clone(),
            origin_id,
        });
    } else if band < last {
        state.set_last_threshold(kind, band);
    }
    if spec.feeds_m16b_sepsis && band >= EnvSeverity::Mild {
        state.m16b_sepsis_feed = true;
    }
    severity_0_1
}

pub(crate) fn severity_from_thresholds(accumulator: f32, spec: &EnvAfflictionSpec) -> f32 {
    if accumulator <= 0.0 {
        return 0.0;
    }
    if accumulator >= spec.lethal_threshold {
        return 1.0;
    }
    if accumulator >= spec.severe_threshold {
        let span = (spec.lethal_threshold - spec.severe_threshold).max(1e-3);
        return 0.7 + 0.3 * ((accumulator - spec.severe_threshold) / span).min(1.0);
    }
    if accumulator >= spec.moderate_threshold {
        let span = (spec.severe_threshold - spec.moderate_threshold).max(1e-3);
        return 0.4 + 0.3 * ((accumulator - spec.moderate_threshold) / span);
    }
    if accumulator >= spec.mild_threshold {
        let span = (spec.moderate_threshold - spec.mild_threshold).max(1e-3);
        return 0.1 + 0.3 * ((accumulator - spec.mild_threshold) / span);
    }
    0.1 * (accumulator / spec.mild_threshold.max(1e-3))
}

pub(crate) fn emit_clear(
    state: &mut EnvAfflictionState,
    actor_id: u64,
    kind: EnvAfflictionKind,
    reason: EnvClearReason,
    out: &mut EnvTickOutput,
) {
    let had_history = state.severity(kind) > 0.0
        || state.last_threshold(kind) != EnvSeverity::None
        || state.accumulators.contains_key(kind.as_str());
    if !had_history {
        return;
    }
    state.clear(kind);
    out.cleared.push(EnvClearedEvent {
        actor_id,
        kind,
        reason,
    });
}

pub(crate) fn check_origin_immune(
    state: &mut EnvAfflictionState,
    actor_id: u64,
    kind: EnvAfflictionKind,
    susceptibility: &AtmosphericSusceptibility,
    multiplier: f32,
    reason: &str,
    alt_kind: Option<EnvAfflictionKind>,
    out: &mut EnvTickOutput,
) -> bool {
    if multiplier <= 0.0 {
        if state.mark_immune(kind, reason) {
            out.origin_immune.push(EnvOriginImmuneEvent {
                actor_id,
                kind,
                origin_id: susceptibility.origin_id,
                reason: reason.to_string(),
                alt_kind,
            });
        }
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_all_eleven_kinds() {
        let reg = EnvAfflictionRegistry::default_registry();
        assert_eq!(reg.specs.len(), 11);
        for k in EnvAfflictionKind::all() {
            assert!(reg.specs.contains_key(k.as_str()));
        }
    }

    #[test]
    fn severity_band_progression() {
        assert_eq!(EnvSeverity::from_severity_0_1(0.0), EnvSeverity::None);
        assert_eq!(EnvSeverity::from_severity_0_1(0.2), EnvSeverity::Mild);
        assert_eq!(EnvSeverity::from_severity_0_1(0.5), EnvSeverity::Moderate);
        assert_eq!(EnvSeverity::from_severity_0_1(0.85), EnvSeverity::Severe);
        assert_eq!(EnvSeverity::from_severity_0_1(1.0), EnvSeverity::Lethal);
    }

    #[test]
    fn human_baseline_susceptibility() {
        let s = AtmosphericSusceptibility::for_origin(OriginId::Human);
        assert!((s.heat_comfort_max_k - 322.0).abs() < 1e-3);
        assert!((s.cold_comfort_min_k - 273.0).abs() < 1e-3);
        assert_eq!(s.asphyxiation_ttd_s, 30.0);
    }

    #[test]
    fn methane_breather_immune_to_asphyxiation() {
        let s = AtmosphericSusceptibility::for_origin(OriginId::MethaneBreather);
        assert!(s.asphyxiation_ttd_s.is_infinite());
        assert!(s.oxygen_toxic);
    }

    #[test]
    fn robot_immune_to_stuffiness_and_refrigerant() {
        let s = AtmosphericSusceptibility::for_origin(OriginId::Robot);
        assert_eq!(s.stuffiness_multiplier, 0.0);
        assert_eq!(s.refrigerant_inhalation_multiplier, 0.0);
    }
}
