//! M16 § 22 affliction kinds — full producer + stacking + clear mechanics.
//!
//! 18 baseline + 4 survival kinds per spec § "22 affliction kinds — full
//! mechanics (18 baseline + 4 survival)". Kind names + payload field
//! enums mirror `cf-replay/schemas/event/affliction_*.json` exactly so
//! the engine's recorder feeds straight through.
//!
//! Existing AfflictionKind in `cf_actor::affliction` predates this crate
//! and uses a different naming + set (Wetness, Asphyxiating, etc.). This
//! crate provides the M16 v0.1-schema-aligned kind set + per-kind
//! mechanics; the actor crate continues to own its enum for backward
//! compatibility with M14J chassis flows.
//!
//! Stacking rule per spec § "Stacking rules: affliction severity stacks
//! (severity 0.5 → exposure → severity 0.8)" — severity adds, capped at
//! 1.0. Events `affliction.applied` / `affliction.escalated` /
//! `affliction.cleared` cover the lifecycle.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::struct_field_names,
    clippy::match_same_arms,
    clippy::unused_self,
    clippy::similar_names
)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// 23 affliction kinds locked in
/// `cf-replay/schemas/event/affliction_applied.json` enum: 18 baseline +
/// 4 survival + `blinded` (M5-A1 / M6 flash grenade).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum M16AfflictionKind {
    Burning,
    Wet,
    Electrified,
    Poisoned,
    Hypoxic,
    CombustibleAtmosphere,
    BreachDecomp,
    Hyperthermic,
    Hypothermic,
    Radiation,
    Concussed,
    Deafened,
    Blinded,
    Bleeding,
    InternalShock,
    LowBattery,
    CoolantLeaking,
    OilLeaking,
    Overheating,
    Hunger,
    Thirst,
    SleepDep,
    SanityLow,
}

impl M16AfflictionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            M16AfflictionKind::Burning => "burning",
            M16AfflictionKind::Wet => "wet",
            M16AfflictionKind::Electrified => "electrified",
            M16AfflictionKind::Poisoned => "poisoned",
            M16AfflictionKind::Hypoxic => "hypoxic",
            M16AfflictionKind::CombustibleAtmosphere => "combustible_atmosphere",
            M16AfflictionKind::BreachDecomp => "breach_decomp",
            M16AfflictionKind::Hyperthermic => "hyperthermic",
            M16AfflictionKind::Hypothermic => "hypothermic",
            M16AfflictionKind::Radiation => "radiation",
            M16AfflictionKind::Concussed => "concussed",
            M16AfflictionKind::Deafened => "deafened",
            M16AfflictionKind::Blinded => "blinded",
            M16AfflictionKind::Bleeding => "bleeding",
            M16AfflictionKind::InternalShock => "internal_shock",
            M16AfflictionKind::LowBattery => "low_battery",
            M16AfflictionKind::CoolantLeaking => "coolant_leaking",
            M16AfflictionKind::OilLeaking => "oil_leaking",
            M16AfflictionKind::Overheating => "overheating",
            M16AfflictionKind::Hunger => "hunger",
            M16AfflictionKind::Thirst => "thirst",
            M16AfflictionKind::SleepDep => "sleep_dep",
            M16AfflictionKind::SanityLow => "sanity_low",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "burning" => M16AfflictionKind::Burning,
            "wet" => M16AfflictionKind::Wet,
            "electrified" => M16AfflictionKind::Electrified,
            "poisoned" => M16AfflictionKind::Poisoned,
            "hypoxic" => M16AfflictionKind::Hypoxic,
            "combustible_atmosphere" => M16AfflictionKind::CombustibleAtmosphere,
            "breach_decomp" => M16AfflictionKind::BreachDecomp,
            "hyperthermic" => M16AfflictionKind::Hyperthermic,
            "hypothermic" => M16AfflictionKind::Hypothermic,
            "radiation" => M16AfflictionKind::Radiation,
            "concussed" => M16AfflictionKind::Concussed,
            "deafened" => M16AfflictionKind::Deafened,
            "blinded" => M16AfflictionKind::Blinded,
            "bleeding" => M16AfflictionKind::Bleeding,
            "internal_shock" => M16AfflictionKind::InternalShock,
            "low_battery" => M16AfflictionKind::LowBattery,
            "coolant_leaking" => M16AfflictionKind::CoolantLeaking,
            "oil_leaking" => M16AfflictionKind::OilLeaking,
            "overheating" => M16AfflictionKind::Overheating,
            "hunger" => M16AfflictionKind::Hunger,
            "thirst" => M16AfflictionKind::Thirst,
            "sleep_dep" => M16AfflictionKind::SleepDep,
            "sanity_low" => M16AfflictionKind::SanityLow,
            _ => return None,
        })
    }

    pub fn all_baseline() -> &'static [M16AfflictionKind] {
        &[
            M16AfflictionKind::Burning,
            M16AfflictionKind::Wet,
            M16AfflictionKind::Electrified,
            M16AfflictionKind::Poisoned,
            M16AfflictionKind::Hypoxic,
            M16AfflictionKind::CombustibleAtmosphere,
            M16AfflictionKind::BreachDecomp,
            M16AfflictionKind::Hyperthermic,
            M16AfflictionKind::Hypothermic,
            M16AfflictionKind::Radiation,
            M16AfflictionKind::Concussed,
            M16AfflictionKind::Deafened,
            M16AfflictionKind::Bleeding,
            M16AfflictionKind::InternalShock,
            M16AfflictionKind::LowBattery,
            M16AfflictionKind::CoolantLeaking,
            M16AfflictionKind::OilLeaking,
            M16AfflictionKind::Overheating,
        ]
    }

    pub fn all_survival() -> &'static [M16AfflictionKind] {
        &[
            M16AfflictionKind::Hunger,
            M16AfflictionKind::Thirst,
            M16AfflictionKind::SleepDep,
            M16AfflictionKind::SanityLow,
        ]
    }

    pub fn is_survival(self) -> bool {
        matches!(
            self,
            M16AfflictionKind::Hunger
                | M16AfflictionKind::Thirst
                | M16AfflictionKind::SleepDep
                | M16AfflictionKind::SanityLow
        )
    }
}

/// Per-kind mechanics. Mirrors the spec § "22 affliction kinds — full
/// mechanics" table.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AfflictionSpec {
    pub kind: M16AfflictionKind,
    /// HP per tick at severity 1.0; linear in severity.
    pub damage_per_tick_at_full: f32,
    /// True when the affliction is naturally cleared by time.
    pub clears_with_time: bool,
    /// Natural clear duration in seconds (relevant when `clears_with_time`).
    pub clear_duration_seconds: f32,
    /// Severity decay per second (negative). Applied even when not cleared
    /// (so wet evaporates, concussion fades). 0.0 = no decay.
    pub severity_decay_per_second: f32,
    /// True when the affliction is gated to PvE Survival mode per
    /// spec § "Survival afflictions only active in PvE Survival mode".
    pub survival_mode_only: bool,
}

impl AfflictionSpec {
    pub fn default_for(kind: M16AfflictionKind) -> Self {
        match kind {
            M16AfflictionKind::Burning => AfflictionSpec {
                kind,
                damage_per_tick_at_full: 0.083,
                clears_with_time: true,
                clear_duration_seconds: 60.0,
                severity_decay_per_second: 0.0167,
                survival_mode_only: false,
            },
            M16AfflictionKind::Wet => AfflictionSpec {
                kind,
                damage_per_tick_at_full: 0.0,
                clears_with_time: true,
                clear_duration_seconds: 30.0,
                severity_decay_per_second: 0.033,
                survival_mode_only: false,
            },
            M16AfflictionKind::Electrified => AfflictionSpec {
                kind,
                damage_per_tick_at_full: 0.05,
                clears_with_time: true,
                clear_duration_seconds: 5.0,
                severity_decay_per_second: 0.2,
                survival_mode_only: false,
            },
            M16AfflictionKind::Poisoned => AfflictionSpec {
                kind,
                damage_per_tick_at_full: 0.05,
                clears_with_time: true,
                clear_duration_seconds: 120.0,
                severity_decay_per_second: 0.0083,
                survival_mode_only: false,
            },
            M16AfflictionKind::Hypoxic => AfflictionSpec {
                kind,
                damage_per_tick_at_full: 0.033,
                clears_with_time: true,
                clear_duration_seconds: 30.0,
                severity_decay_per_second: 0.033,
                survival_mode_only: false,
            },
            M16AfflictionKind::CombustibleAtmosphere => AfflictionSpec {
                kind,
                damage_per_tick_at_full: 0.0,
                clears_with_time: false,
                clear_duration_seconds: 0.0,
                severity_decay_per_second: 0.0,
                survival_mode_only: false,
            },
            M16AfflictionKind::BreachDecomp => AfflictionSpec {
                kind,
                damage_per_tick_at_full: 0.017,
                clears_with_time: false,
                clear_duration_seconds: 0.0,
                severity_decay_per_second: 0.0,
                survival_mode_only: false,
            },
            M16AfflictionKind::Hyperthermic => AfflictionSpec {
                kind,
                damage_per_tick_at_full: 0.033,
                clears_with_time: true,
                clear_duration_seconds: 60.0,
                severity_decay_per_second: 0.0167,
                survival_mode_only: false,
            },
            M16AfflictionKind::Hypothermic => AfflictionSpec {
                kind,
                damage_per_tick_at_full: 0.017,
                clears_with_time: true,
                clear_duration_seconds: 60.0,
                severity_decay_per_second: 0.0167,
                survival_mode_only: false,
            },
            M16AfflictionKind::Radiation => AfflictionSpec {
                kind,
                damage_per_tick_at_full: 0.017,
                clears_with_time: true,
                clear_duration_seconds: 300.0,
                severity_decay_per_second: 0.0033,
                survival_mode_only: false,
            },
            M16AfflictionKind::Concussed => AfflictionSpec {
                kind,
                damage_per_tick_at_full: 0.0,
                clears_with_time: true,
                clear_duration_seconds: 3.0,
                severity_decay_per_second: 0.33,
                survival_mode_only: false,
            },
            M16AfflictionKind::Deafened => AfflictionSpec {
                kind,
                damage_per_tick_at_full: 0.0,
                clears_with_time: true,
                clear_duration_seconds: 5.0,
                severity_decay_per_second: 0.2,
                survival_mode_only: false,
            },
            M16AfflictionKind::Blinded => AfflictionSpec {
                kind,
                damage_per_tick_at_full: 0.0,
                clears_with_time: true,
                clear_duration_seconds: 3.0,
                severity_decay_per_second: 0.33,
                survival_mode_only: false,
            },
            M16AfflictionKind::Bleeding => AfflictionSpec {
                kind,
                damage_per_tick_at_full: 0.017,
                clears_with_time: false,
                clear_duration_seconds: 0.0,
                severity_decay_per_second: 0.0,
                survival_mode_only: false,
            },
            M16AfflictionKind::InternalShock => AfflictionSpec {
                kind,
                damage_per_tick_at_full: 0.05,
                clears_with_time: false,
                clear_duration_seconds: 0.0,
                severity_decay_per_second: 0.0,
                survival_mode_only: false,
            },
            M16AfflictionKind::LowBattery => AfflictionSpec {
                kind,
                damage_per_tick_at_full: 0.0,
                clears_with_time: false,
                clear_duration_seconds: 0.0,
                severity_decay_per_second: 0.0,
                survival_mode_only: false,
            },
            M16AfflictionKind::CoolantLeaking => AfflictionSpec {
                kind,
                damage_per_tick_at_full: 0.0,
                clears_with_time: false,
                clear_duration_seconds: 0.0,
                severity_decay_per_second: 0.0,
                survival_mode_only: false,
            },
            M16AfflictionKind::OilLeaking => AfflictionSpec {
                kind,
                damage_per_tick_at_full: 0.0,
                clears_with_time: false,
                clear_duration_seconds: 0.0,
                severity_decay_per_second: 0.0,
                survival_mode_only: false,
            },
            M16AfflictionKind::Overheating => AfflictionSpec {
                kind,
                damage_per_tick_at_full: 0.0,
                clears_with_time: true,
                clear_duration_seconds: 30.0,
                severity_decay_per_second: 0.033,
                survival_mode_only: false,
            },
            M16AfflictionKind::Hunger => AfflictionSpec {
                kind,
                damage_per_tick_at_full: 0.0017,
                clears_with_time: false,
                clear_duration_seconds: 0.0,
                severity_decay_per_second: 0.0,
                survival_mode_only: true,
            },
            M16AfflictionKind::Thirst => AfflictionSpec {
                kind,
                damage_per_tick_at_full: 0.0017,
                clears_with_time: false,
                clear_duration_seconds: 0.0,
                severity_decay_per_second: 0.0,
                survival_mode_only: true,
            },
            M16AfflictionKind::SleepDep => AfflictionSpec {
                kind,
                damage_per_tick_at_full: 0.0,
                clears_with_time: false,
                clear_duration_seconds: 0.0,
                severity_decay_per_second: 0.0,
                survival_mode_only: true,
            },
            M16AfflictionKind::SanityLow => AfflictionSpec {
                kind,
                damage_per_tick_at_full: 0.0,
                clears_with_time: false,
                clear_duration_seconds: 0.0,
                severity_decay_per_second: 0.0,
                survival_mode_only: true,
            },
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AfflictionRegistry {
    pub specs: BTreeMap<String, AfflictionSpec>,
}

impl AfflictionRegistry {
    pub fn default_registry() -> Self {
        let mut specs = BTreeMap::new();
        for k in M16AfflictionKind::all_baseline().iter().chain(M16AfflictionKind::all_survival().iter()).copied() {
            specs.insert(k.as_str().to_string(), AfflictionSpec::default_for(k));
        }
        specs.insert(
            M16AfflictionKind::Blinded.as_str().to_string(),
            AfflictionSpec::default_for(M16AfflictionKind::Blinded),
        );
        Self { specs }
    }

    pub fn lookup(&self, kind: M16AfflictionKind) -> &AfflictionSpec {
        self.specs
            .get(kind.as_str())
            .expect("affliction registry must contain every kind")
    }
}

/// One active affliction on an actor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveAffliction {
    pub kind: M16AfflictionKind,
    pub severity: f32,
    pub applied_at_tick: u64,
    pub expected_clear_tick: Option<u64>,
    pub source_event_id: Option<String>,
}

/// Per-actor affliction state.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ActorAfflictions {
    pub active: Vec<ActiveAffliction>,
}

impl ActorAfflictions {
    pub fn find(&self, kind: M16AfflictionKind) -> Option<&ActiveAffliction> {
        self.active.iter().find(|a| a.kind == kind)
    }

    pub fn find_mut(&mut self, kind: M16AfflictionKind) -> Option<&mut ActiveAffliction> {
        self.active.iter_mut().find(|a| a.kind == kind)
    }

    pub fn severity_of(&self, kind: M16AfflictionKind) -> f32 {
        self.find(kind).map(|a| a.severity).unwrap_or(0.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AfflictionAppliedEvent {
    pub actor_id: u64,
    pub kind: M16AfflictionKind,
    pub severity: f32,
    pub source_event_id: String,
    pub expected_duration_ticks: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AfflictionEscalatedEvent {
    pub actor_id: u64,
    pub kind: M16AfflictionKind,
    pub from_severity: f32,
    pub to_severity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClearReason {
    Time,
    Medikit,
    Environment,
    Death,
}

impl ClearReason {
    pub fn as_str(self) -> &'static str {
        match self {
            ClearReason::Time => "time",
            ClearReason::Medikit => "medikit",
            ClearReason::Environment => "environment",
            ClearReason::Death => "death",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AfflictionClearedEvent {
    pub actor_id: u64,
    pub kind: M16AfflictionKind,
    pub reason: ClearReason,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AfflictionTickEvent {
    pub actor_id: u64,
    pub kind: M16AfflictionKind,
    pub hp_delta: f32,
    pub tick: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProducerOutput {
    pub applied: Vec<AfflictionAppliedEvent>,
    pub escalated: Vec<AfflictionEscalatedEvent>,
    pub cleared: Vec<AfflictionClearedEvent>,
    pub tick: Vec<AfflictionTickEvent>,
    /// HP damage to apply to each actor this tick (sum across afflictions).
    pub hp_damage: BTreeMap<u64, f32>,
}

/// Apply an affliction to an actor. New afflictions emit
/// `affliction.applied`; existing afflictions of the same kind emit
/// `affliction.escalated` with severity stacked (capped at 1.0).
pub fn apply_affliction(
    state: &mut ActorAfflictions,
    actor_id: u64,
    kind: M16AfflictionKind,
    severity_to_add: f32,
    registry: &AfflictionRegistry,
    tick: u64,
    tick_rate_hz: u32,
    source_event_id: String,
) -> (Option<AfflictionAppliedEvent>, Option<AfflictionEscalatedEvent>) {
    let spec = registry.lookup(kind);
    let severity_to_add = severity_to_add.clamp(0.0, 1.0);
    if let Some(existing) = state.find_mut(kind) {
        let from = existing.severity;
        let to = (from + severity_to_add).clamp(0.0, 1.0);
        existing.severity = to;
        existing.expected_clear_tick = expected_clear_tick(spec, tick, tick_rate_hz);
        if (to - from).abs() < 1e-6 {
            return (None, None);
        }
        return (
            None,
            Some(AfflictionEscalatedEvent {
                actor_id,
                kind,
                from_severity: from,
                to_severity: to,
            }),
        );
    }
    let expected_clear = expected_clear_tick(spec, tick, tick_rate_hz);
    let expected_duration_ticks = match expected_clear {
        Some(c) => (c.saturating_sub(tick)).max(1) as u32,
        None => 1,
    };
    state.active.push(ActiveAffliction {
        kind,
        severity: severity_to_add,
        applied_at_tick: tick,
        expected_clear_tick: expected_clear,
        source_event_id: Some(source_event_id.clone()),
    });
    (
        Some(AfflictionAppliedEvent {
            actor_id,
            kind,
            severity: severity_to_add,
            source_event_id,
            expected_duration_ticks,
        }),
        None,
    )
}

fn expected_clear_tick(spec: &AfflictionSpec, tick: u64, tick_rate_hz: u32) -> Option<u64> {
    if !spec.clears_with_time {
        return None;
    }
    let dur_ticks = (spec.clear_duration_seconds * (tick_rate_hz.max(1) as f32)) as u64;
    Some(tick + dur_ticks)
}

/// Clear an affliction explicitly (medikit, environment, death).
pub fn clear_affliction(
    state: &mut ActorAfflictions,
    actor_id: u64,
    kind: M16AfflictionKind,
    reason: ClearReason,
) -> Option<AfflictionClearedEvent> {
    let prev_len = state.active.len();
    state.active.retain(|a| a.kind != kind);
    if state.active.len() == prev_len {
        return None;
    }
    Some(AfflictionClearedEvent {
        actor_id,
        kind,
        reason,
    })
}

/// Tick one actor's afflictions one sim tick. Applies severity decay,
/// time-based clears, and produces the per-tick damage roll-up. The
/// cosmetic `affliction.tick` event is emitted at the M4 cadence (one
/// per second per actor per kind).
pub fn tick_actor(
    state: &mut ActorAfflictions,
    actor_id: u64,
    registry: &AfflictionRegistry,
    tick: u64,
    tick_rate_hz: u32,
    survival_mode_active: bool,
) -> ProducerOutput {
    let dt_seconds = 1.0_f32 / (tick_rate_hz.max(1) as f32);
    let mut out = ProducerOutput::default();
    let mut total_damage = 0.0_f32;
    let mut to_clear: Vec<M16AfflictionKind> = Vec::new();
    for affl in state.active.iter_mut() {
        let spec = registry.lookup(affl.kind);
        if spec.survival_mode_only && !survival_mode_active {
            to_clear.push(affl.kind);
            continue;
        }
        // Damage accumulation.
        let dmg = spec.damage_per_tick_at_full * affl.severity;
        if dmg > 0.0 {
            total_damage += dmg;
            if tick % tick_rate_hz.max(1) as u64 == 0 {
                out.tick.push(AfflictionTickEvent {
                    actor_id,
                    kind: affl.kind,
                    hp_delta: -dmg,
                    tick,
                });
            }
        }
        // Severity decay.
        if spec.severity_decay_per_second > 0.0 {
            affl.severity = (affl.severity - spec.severity_decay_per_second * dt_seconds).max(0.0);
        }
        // Time clear.
        if let Some(clear_tick) = affl.expected_clear_tick {
            if tick >= clear_tick && spec.clears_with_time {
                to_clear.push(affl.kind);
                continue;
            }
        }
        if affl.severity <= 0.0 {
            to_clear.push(affl.kind);
        }
    }
    for kind in to_clear {
        if let Some(ev) = clear_affliction(state, actor_id, kind, ClearReason::Time) {
            out.cleared.push(ev);
        }
    }
    if total_damage > 0.0 {
        out.hp_damage.insert(actor_id, total_damage);
    }
    out
}

/// Auto-triage trigger thresholds per spec § "Per-affliction TTD floor +
/// auto-triage trigger thresholds". Returns true when the carrier's
/// affliction state crosses any threshold that warrants a Medic-role
/// utility scorer bonus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoTriageReason {
    BleedingStack3,
    SingleArterialWound,
    BurningAtLowHp,
    PoisonStackAtLowHp,
    ContinuousShock,
    FrozenAtLowHp,
    DrowningWithHelmetBreach,
    RadiationDoseHigh,
    CompoundTtdLow,
}

impl AutoTriageReason {
    pub fn as_str(self) -> &'static str {
        match self {
            AutoTriageReason::BleedingStack3 => "bleeding_stack_3",
            AutoTriageReason::SingleArterialWound => "single_arterial_wound",
            AutoTriageReason::BurningAtLowHp => "burning_at_low_hp",
            AutoTriageReason::PoisonStackAtLowHp => "poison_stack_at_low_hp",
            AutoTriageReason::ContinuousShock => "continuous_shock",
            AutoTriageReason::FrozenAtLowHp => "frozen_at_low_hp",
            AutoTriageReason::DrowningWithHelmetBreach => "drowning_with_helmet_breach",
            AutoTriageReason::RadiationDoseHigh => "radiation_dose_high",
            AutoTriageReason::CompoundTtdLow => "compound_ttd_low",
        }
    }
}

/// Per-affliction TTD floor at tough_crowd difficulty (default). Mirrors
/// spec § "Per-affliction TTD floor" table (single-instance values).
pub fn per_instance_ttd_seconds(kind: M16AfflictionKind) -> f32 {
    match kind {
        M16AfflictionKind::Bleeding => 90.0,
        M16AfflictionKind::Burning => 60.0,
        M16AfflictionKind::Concussed => 8.0,
        M16AfflictionKind::Poisoned => 120.0,
        M16AfflictionKind::Electrified => 30.0,
        M16AfflictionKind::Hypothermic => 40.0,
        M16AfflictionKind::Hypoxic => 30.0,
        M16AfflictionKind::BreachDecomp => 22.5,
        M16AfflictionKind::Radiation => 300.0,
        M16AfflictionKind::Hyperthermic => 100.0,
        M16AfflictionKind::Hunger => 600.0,
        M16AfflictionKind::Thirst => 360.0,
        M16AfflictionKind::SleepDep => 1200.0,
        M16AfflictionKind::SanityLow => 600.0,
        M16AfflictionKind::Blinded => f32::INFINITY,
        M16AfflictionKind::Deafened => f32::INFINITY,
        M16AfflictionKind::CombustibleAtmosphere => f32::INFINITY,
        M16AfflictionKind::Wet => f32::INFINITY,
        M16AfflictionKind::InternalShock => 30.0,
        M16AfflictionKind::LowBattery => 60.0,
        M16AfflictionKind::CoolantLeaking => 90.0,
        M16AfflictionKind::OilLeaking => 90.0,
        M16AfflictionKind::Overheating => 30.0,
    }
}

/// Auto-triage threshold gate. Returns the reason(s) firing for this
/// actor, given:
///   - their bleed wound count (M14 wound list)
///   - any arterial wound flag (M14H severity tier)
///   - HP percent (0..=1)
///   - radiation dose-rate (per-second band)
///   - whether the actor is in continuous shock (electric tile contact)
///   - whether the actor has a helmet breach + is drowning
///   - the compound TTD from `cf_actor::ttd::TtdContract::compound_ttd_seconds`
pub fn auto_triage_reasons(
    afflictions: &ActorAfflictions,
    bleed_wound_count: u32,
    arterial_wound: bool,
    hp_percent: f32,
    radiation_dose_rate: f32,
    continuous_shock: bool,
    helmet_breached_drowning: bool,
    compound_ttd_seconds: f32,
) -> Vec<AutoTriageReason> {
    let mut reasons = Vec::new();
    if bleed_wound_count >= 3 || afflictions.severity_of(M16AfflictionKind::Bleeding) >= 0.75 {
        reasons.push(AutoTriageReason::BleedingStack3);
    }
    if arterial_wound {
        reasons.push(AutoTriageReason::SingleArterialWound);
    }
    if afflictions.severity_of(M16AfflictionKind::Burning) > 0.0 && hp_percent < 0.40 {
        reasons.push(AutoTriageReason::BurningAtLowHp);
    }
    if afflictions.severity_of(M16AfflictionKind::Poisoned) >= 0.4 && hp_percent < 0.50 {
        reasons.push(AutoTriageReason::PoisonStackAtLowHp);
    }
    if continuous_shock {
        reasons.push(AutoTriageReason::ContinuousShock);
    }
    if afflictions.severity_of(M16AfflictionKind::Hypothermic) > 0.0 && hp_percent < 0.30 {
        reasons.push(AutoTriageReason::FrozenAtLowHp);
    }
    if helmet_breached_drowning {
        reasons.push(AutoTriageReason::DrowningWithHelmetBreach);
    }
    if radiation_dose_rate > 0.5 {
        reasons.push(AutoTriageReason::RadiationDoseHigh);
    }
    if compound_ttd_seconds < 12.0 {
        reasons.push(AutoTriageReason::CompoundTtdLow);
    }
    reasons
}

/// Utility scorer bonus per spec § "the Medic's utility scorer adds +0.4
/// to TriageDownedAlly(target)".
pub const TRIAGE_UTILITY_BONUS: f32 = 0.4;

/// Race-aware vacuum exposure TTD per spec § "Vacuum exposure with
/// helmet breach is race-aware".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Race {
    Human,
    Methane,
    Crystalline,
    Aqueous,
    Robotic,
}

pub fn vacuum_exposure_ttd_seconds(race: Race) -> f32 {
    match race {
        Race::Human => 15.0,
        Race::Methane => f32::INFINITY,
        Race::Crystalline => 60.0,
        Race::Aqueous => 20.0,
        Race::Robotic => 30.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_all_22_kinds_plus_blinded() {
        let reg = AfflictionRegistry::default_registry();
        let baseline = M16AfflictionKind::all_baseline().len();
        let survival = M16AfflictionKind::all_survival().len();
        assert_eq!(baseline + survival + 1, 23, "18 baseline + 4 survival + blinded");
        assert!(reg.specs.contains_key("blinded"));
        assert!(reg.specs.contains_key("hunger"));
        assert!(reg.specs.contains_key("sanity_low"));
    }

    #[test]
    fn applying_burning_emits_applied_event() {
        let reg = AfflictionRegistry::default_registry();
        let mut state = ActorAfflictions::default();
        let (applied, escalated) = apply_affliction(
            &mut state,
            7,
            M16AfflictionKind::Burning,
            0.3,
            &reg,
            0,
            60,
            "ev0".to_string(),
        );
        assert!(applied.is_some());
        assert!(escalated.is_none());
        assert_eq!(state.severity_of(M16AfflictionKind::Burning), 0.3);
    }

    #[test]
    fn re_applying_burning_stacks_severity_and_emits_escalated() {
        let reg = AfflictionRegistry::default_registry();
        let mut state = ActorAfflictions::default();
        apply_affliction(&mut state, 7, M16AfflictionKind::Burning, 0.3, &reg, 0, 60, "ev0".to_string());
        let (applied, escalated) = apply_affliction(
            &mut state,
            7,
            M16AfflictionKind::Burning,
            0.5,
            &reg,
            5,
            60,
            "ev1".to_string(),
        );
        assert!(applied.is_none());
        assert!(escalated.is_some());
        let ev = escalated.unwrap();
        assert!((ev.from_severity - 0.3).abs() < 1e-3);
        assert!((ev.to_severity - 0.8).abs() < 1e-3);
    }

    #[test]
    fn severity_caps_at_1_0() {
        let reg = AfflictionRegistry::default_registry();
        let mut state = ActorAfflictions::default();
        apply_affliction(&mut state, 7, M16AfflictionKind::Burning, 0.8, &reg, 0, 60, "ev0".to_string());
        apply_affliction(&mut state, 7, M16AfflictionKind::Burning, 0.8, &reg, 5, 60, "ev1".to_string());
        assert!((state.severity_of(M16AfflictionKind::Burning) - 1.0).abs() < 1e-3);
    }

    #[test]
    fn burning_dot_drains_hp() {
        let reg = AfflictionRegistry::default_registry();
        let mut state = ActorAfflictions::default();
        apply_affliction(&mut state, 7, M16AfflictionKind::Burning, 1.0, &reg, 0, 60, "ev0".to_string());
        let mut total_damage = 0.0_f32;
        for tick in 1..=60u64 {
            let out = tick_actor(&mut state, 7, &reg, tick, 60, false);
            if let Some(d) = out.hp_damage.get(&7) {
                total_damage += d;
            }
        }
        assert!(total_damage > 4.5, "burning at severity 1.0 should drain ~5 hp/sec");
    }

    #[test]
    fn medikit_clears_bleeding() {
        let reg = AfflictionRegistry::default_registry();
        let mut state = ActorAfflictions::default();
        apply_affliction(&mut state, 7, M16AfflictionKind::Bleeding, 1.0, &reg, 0, 60, "ev0".to_string());
        let cleared = clear_affliction(&mut state, 7, M16AfflictionKind::Bleeding, ClearReason::Medikit);
        assert!(cleared.is_some());
        assert_eq!(cleared.unwrap().reason, ClearReason::Medikit);
        assert!(state.find(M16AfflictionKind::Bleeding).is_none());
    }

    #[test]
    fn survival_afflictions_clear_outside_survival_mode() {
        let reg = AfflictionRegistry::default_registry();
        let mut state = ActorAfflictions::default();
        apply_affliction(&mut state, 7, M16AfflictionKind::Hunger, 0.5, &reg, 0, 60, "ev0".to_string());
        let out = tick_actor(&mut state, 7, &reg, 1, 60, false);
        assert!(!out.cleared.is_empty(), "hunger must clear when survival mode is off");
    }

    #[test]
    fn auto_triage_fires_on_bleed_stack_3() {
        let mut afflictions = ActorAfflictions::default();
        afflictions.active.push(ActiveAffliction {
            kind: M16AfflictionKind::Bleeding,
            severity: 1.0,
            applied_at_tick: 0,
            expected_clear_tick: None,
            source_event_id: None,
        });
        let reasons = auto_triage_reasons(&afflictions, 3, false, 0.9, 0.0, false, false, f32::INFINITY);
        assert!(reasons.contains(&AutoTriageReason::BleedingStack3));
    }

    #[test]
    fn auto_triage_fires_on_compound_ttd_low() {
        let afflictions = ActorAfflictions::default();
        let reasons = auto_triage_reasons(&afflictions, 0, false, 0.9, 0.0, false, false, 6.0);
        assert!(reasons.contains(&AutoTriageReason::CompoundTtdLow));
    }

    #[test]
    fn vacuum_exposure_ttd_is_race_aware() {
        assert!((vacuum_exposure_ttd_seconds(Race::Human) - 15.0).abs() < 1e-3);
        assert!(vacuum_exposure_ttd_seconds(Race::Methane).is_infinite());
        assert!((vacuum_exposure_ttd_seconds(Race::Crystalline) - 60.0).abs() < 1e-3);
    }

    #[test]
    fn bleeding_per_instance_ttd_is_90s() {
        assert!((per_instance_ttd_seconds(M16AfflictionKind::Bleeding) - 90.0).abs() < 1e-3);
    }

    #[test]
    fn burning_per_instance_ttd_is_60s() {
        assert!((per_instance_ttd_seconds(M16AfflictionKind::Burning) - 60.0).abs() < 1e-3);
    }

    #[test]
    fn concussion_ttd_within_5_to_10s_range() {
        let v = per_instance_ttd_seconds(M16AfflictionKind::Concussed);
        assert!(v >= 5.0 && v <= 10.0, "concussed TTD must be in 5-10s band, got {v}");
    }
}
