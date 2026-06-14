//! conditions.rs — the 8-condition mental-health lifecycle FSM, the per-actor
//! mental-health state, the trigger evaluators (witness-death window + stim-
//! dose window), and the per-tick advance pass (panic rolls, treatment
//! resolution, natural progression, relapse).
//!
//! FSM: `Triggered → Acute → (Subacute | Chronic) → (Remission | Refractory)`
//! — isomorphic to the cf-disease `DiseaseStage` lifecycle and shared with the
//! M16B `mental_illness` disease entry. All transitions are deterministic and
//! tick-driven; stochastic outcomes use the seeded `crate::mh_roll`.

use std::collections::{BTreeMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::{
    mh_roll, ticks_to_seconds, treatment::TreatmentPlan, treatment::PsychMedClass, OriginId,
    ADDICTION_DOSE_THRESHOLD, ADDICTION_WINDOW_SECONDS, PANIC_FREEZE_MAX_SECONDS,
    PANIC_FREEZE_MIN_SECONDS, SALT_OUTCOME, SALT_PANIC, SALT_PANIC_FREEZE, SALT_RELAPSE,
    WITNESS_DEATH_THRESHOLD, WITNESS_WINDOW_SECONDS,
};

/// The 8 launch mental-health conditions (spec § Player-facing behavior).
/// `#[repr(u8)]` with explicit discriminants so `kind as u64` keys `mh_roll`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum ConditionKind {
    Ptsd = 0,
    AnxietyDisorder = 1,
    Depression = 2,
    Addiction = 3,
    Withdrawal = 4,
    Insomnia = 5,
    PanicDisorder = 6,
    AcuteStressReaction = 7,
}

impl ConditionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ConditionKind::Ptsd => "ptsd",
            ConditionKind::AnxietyDisorder => "anxiety_disorder",
            ConditionKind::Depression => "depression",
            ConditionKind::Addiction => "addiction",
            ConditionKind::Withdrawal => "withdrawal",
            ConditionKind::Insomnia => "insomnia",
            ConditionKind::PanicDisorder => "panic_disorder",
            ConditionKind::AcuteStressReaction => "acute_stress_reaction",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "ptsd" => ConditionKind::Ptsd,
            "anxiety_disorder" => ConditionKind::AnxietyDisorder,
            "depression" => ConditionKind::Depression,
            "addiction" => ConditionKind::Addiction,
            "withdrawal" => ConditionKind::Withdrawal,
            "insomnia" => ConditionKind::Insomnia,
            "panic_disorder" => ConditionKind::PanicDisorder,
            "acute_stress_reaction" => ConditionKind::AcuteStressReaction,
            _ => return None,
        })
    }

    pub fn all() -> &'static [ConditionKind] {
        &[
            ConditionKind::Ptsd,
            ConditionKind::AnxietyDisorder,
            ConditionKind::Depression,
            ConditionKind::Addiction,
            ConditionKind::Withdrawal,
            ConditionKind::Insomnia,
            ConditionKind::PanicDisorder,
            ConditionKind::AcuteStressReaction,
        ]
    }

    /// `recovered_from_<condition>` trait id granted on remission.
    pub fn recovered_trait(self) -> String {
        format!("recovered_from_{}", self.as_str())
    }

    /// `chronic_<condition>` trait id granted on chronic entry.
    pub fn chronic_trait(self) -> String {
        format!("chronic_{}", self.as_str())
    }

    /// `refractory_<condition>` trait id granted on refractory entry.
    pub fn refractory_trait(self) -> String {
        format!("refractory_{}", self.as_str())
    }
}

/// Lifecycle stage. `as_str` mirrors the `from`/`to` strings on
/// `psych.stage_changed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionStage {
    Triggered,
    Acute,
    Subacute,
    Chronic,
    Remission,
    Refractory,
}

impl ConditionStage {
    pub fn as_str(self) -> &'static str {
        match self {
            ConditionStage::Triggered => "triggered",
            ConditionStage::Acute => "acute",
            ConditionStage::Subacute => "subacute",
            ConditionStage::Chronic => "chronic",
            ConditionStage::Remission => "remission",
            ConditionStage::Refractory => "refractory",
        }
    }

    /// FSM end states (the clock stops; only relapse / re-treatment moves out).
    pub fn is_terminal(self) -> bool {
        matches!(self, ConditionStage::Remission | ConditionStage::Refractory)
    }

    /// Stages where the actor is actively symptomatic (everything but the
    /// recovered Remission state — Refractory still has symptoms).
    pub fn is_symptomatic(self) -> bool {
        !matches!(self, ConditionStage::Remission)
    }
}

/// Why a condition triggered (combat trauma, drug use, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerReason {
    WitnessDeaths,
    SurviveCriticalWound,
    ConcussionAtKo,
    SustainedStress,
    LossOfSquadmate,
    DrugDoses,
    DrugAbsence,
    PanicThreshold,
    AnxietyChronic,
    ImminentTrauma,
    SleepDeprivation,
    /// Secondary onset driven by the comorbidity matrix (a comorbid partner of
    /// an already-triggered condition).
    Comorbidity,
}

impl TriggerReason {
    pub fn as_str(self) -> &'static str {
        match self {
            TriggerReason::WitnessDeaths => "witness_deaths",
            TriggerReason::SurviveCriticalWound => "survive_critical_wound",
            TriggerReason::ConcussionAtKo => "concussion_at_ko",
            TriggerReason::SustainedStress => "sustained_stress",
            TriggerReason::LossOfSquadmate => "loss_of_squadmate",
            TriggerReason::DrugDoses => "drug_doses",
            TriggerReason::DrugAbsence => "drug_absence",
            TriggerReason::PanicThreshold => "panic_threshold",
            TriggerReason::AnxietyChronic => "anxiety_chronic",
            TriggerReason::ImminentTrauma => "imminent_trauma",
            TriggerReason::SleepDeprivation => "sleep_deprivation",
            TriggerReason::Comorbidity => "comorbidity",
        }
    }
}

/// Per-condition tuning. Authored as `content/psych_conditions/*.ron`; the
/// hardcoded `ConditionRegistry::default_registry` is the boot source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConditionSpec {
    pub kind: ConditionKind,
    /// Acute → Subacute clock (seconds), untreated.
    pub acute_window_seconds: f32,
    /// Subacute → Chronic/Remission clock (seconds), untreated.
    pub subacute_window_seconds: f32,
    /// Probability the resolution lands on Chronic (vs Remission). For a
    /// Chronic condition under treatment, the probability of Refractory.
    pub chronic_chance: f32,
    /// Therapy sessions required for a treated resolution.
    pub therapy_sessions_required: u32,
    /// Indicated medication class (None → therapy/rest only).
    pub medication: Option<PsychMedClass>,
    /// Seconds on medication before it counts toward remission.
    pub medication_onset_seconds: f32,
    /// Per-tick panic-attack probability while symptomatic (0 = no panics).
    pub panic_chance_per_tick: f32,
    /// Per-tick relapse probability while in Remission (0 = no relapse).
    pub relapse_chance_per_tick: f32,
    /// True for conditions that resolve on their own over `natural_resolve`.
    pub resolves_naturally: bool,
    /// Acute → Remission clock (seconds) for naturally-resolving conditions.
    pub natural_resolve_seconds: f32,
}

/// The condition registry (8 specs), keyed by snake_case condition id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConditionRegistry {
    pub specs: BTreeMap<String, ConditionSpec>,
}

impl ConditionRegistry {
    /// The 8 launch condition specs (spec § Player-facing behavior + Tunable
    /// defaults).
    pub fn default_registry() -> Self {
        let mut specs = BTreeMap::new();
        for spec in [
            ConditionSpec {
                kind: ConditionKind::Ptsd,
                acute_window_seconds: 7.0 * crate::DAY_SECONDS,
                subacute_window_seconds: 30.0 * crate::DAY_SECONDS,
                chronic_chance: 0.30,
                therapy_sessions_required: crate::PTSD_THERAPY_SESSIONS,
                medication: Some(PsychMedClass::Ssri),
                medication_onset_seconds: crate::SSRI_ONSET_SECONDS,
                panic_chance_per_tick: 0.000_05,
                relapse_chance_per_tick: 0.000_001,
                resolves_naturally: false,
                natural_resolve_seconds: 0.0,
            },
            ConditionSpec {
                kind: ConditionKind::AnxietyDisorder,
                acute_window_seconds: 7.0 * crate::DAY_SECONDS,
                subacute_window_seconds: 30.0 * crate::DAY_SECONDS,
                chronic_chance: 0.10,
                therapy_sessions_required: 6,
                medication: Some(PsychMedClass::Benzo),
                medication_onset_seconds: 7.0 * crate::DAY_SECONDS,
                panic_chance_per_tick: 0.0,
                relapse_chance_per_tick: 0.000_001,
                resolves_naturally: false,
                natural_resolve_seconds: 0.0,
            },
            ConditionSpec {
                kind: ConditionKind::Depression,
                acute_window_seconds: 30.0 * crate::DAY_SECONDS,
                subacute_window_seconds: 60.0 * crate::DAY_SECONDS,
                chronic_chance: 0.40,
                therapy_sessions_required: 8,
                medication: Some(PsychMedClass::Ssri),
                medication_onset_seconds: crate::SSRI_ONSET_SECONDS,
                panic_chance_per_tick: 0.0,
                relapse_chance_per_tick: 0.000_002,
                resolves_naturally: false,
                natural_resolve_seconds: 0.0,
            },
            ConditionSpec {
                kind: ConditionKind::Addiction,
                acute_window_seconds: crate::DAY_SECONDS,
                subacute_window_seconds: 14.0 * crate::DAY_SECONDS,
                // Addiction is a permanent vulnerability — treatment reaches
                // remission but the chronic pull stays high.
                chronic_chance: 0.50,
                therapy_sessions_required: 8,
                medication: Some(PsychMedClass::WithdrawalAssist),
                medication_onset_seconds: 7.0 * crate::DAY_SECONDS,
                panic_chance_per_tick: 0.0,
                relapse_chance_per_tick: 0.000_010,
                resolves_naturally: false,
                natural_resolve_seconds: 0.0,
            },
            ConditionSpec {
                kind: ConditionKind::Withdrawal,
                acute_window_seconds: 3.0 * crate::DAY_SECONDS,
                subacute_window_seconds: 0.0,
                chronic_chance: 0.0,
                therapy_sessions_required: 2,
                medication: Some(PsychMedClass::WithdrawalAssist),
                medication_onset_seconds: 3.0 * crate::DAY_SECONDS,
                panic_chance_per_tick: 0.0,
                relapse_chance_per_tick: 0.0,
                resolves_naturally: true,
                // Resolves over 2 in-game weeks (spec § Tunable defaults).
                natural_resolve_seconds: crate::WITHDRAWAL_RESOLVE_SECONDS,
            },
            ConditionSpec {
                kind: ConditionKind::Insomnia,
                acute_window_seconds: 7.0 * crate::DAY_SECONDS,
                subacute_window_seconds: 0.0,
                chronic_chance: 0.15,
                therapy_sessions_required: 3,
                medication: Some(PsychMedClass::SleepAid),
                medication_onset_seconds: 3.0 * crate::DAY_SECONDS,
                panic_chance_per_tick: 0.0,
                relapse_chance_per_tick: 0.0,
                resolves_naturally: true,
                natural_resolve_seconds: 7.0 * crate::DAY_SECONDS,
            },
            ConditionSpec {
                kind: ConditionKind::PanicDisorder,
                acute_window_seconds: 7.0 * crate::DAY_SECONDS,
                subacute_window_seconds: 30.0 * crate::DAY_SECONDS,
                chronic_chance: 0.20,
                therapy_sessions_required: 8,
                medication: Some(PsychMedClass::Benzo),
                medication_onset_seconds: 7.0 * crate::DAY_SECONDS,
                // Acute panic-attack spikes (freeze 3-8s).
                panic_chance_per_tick: 0.000_50,
                relapse_chance_per_tick: 0.000_002,
                resolves_naturally: false,
                natural_resolve_seconds: 0.0,
            },
            ConditionSpec {
                kind: ConditionKind::AcuteStressReaction,
                acute_window_seconds: crate::DAY_SECONDS,
                subacute_window_seconds: 0.0,
                chronic_chance: 0.0,
                therapy_sessions_required: 1,
                medication: None,
                medication_onset_seconds: 0.0,
                // Mild panic spikes during the acute window.
                panic_chance_per_tick: 0.000_02,
                relapse_chance_per_tick: 0.0,
                resolves_naturally: true,
                // Short window: resolves within 7 in-game days if not
                // converted to PTSD.
                natural_resolve_seconds: 7.0 * crate::DAY_SECONDS,
            },
        ] {
            specs.insert(spec.kind.as_str().to_string(), spec);
        }
        Self { specs }
    }

    pub fn get(&self, kind: ConditionKind) -> Option<&ConditionSpec> {
        self.specs.get(kind.as_str())
    }

    /// Like [`get`](Self::get) but panics on a missing spec (registry is the
    /// canonical source — a missing kind is a content bug).
    pub fn lookup(&self, kind: ConditionKind) -> &ConditionSpec {
        self.get(kind).unwrap_or_else(|| panic!("no condition spec for {}", kind.as_str()))
    }

    /// Load `content/psych_conditions/*.ron`, overlaying the defaults. Missing
    /// dir → defaults. `_comorbidity.ron` and other `_`-prefixed files are
    /// skipped (handled by the comorbidity module).
    pub fn load_dir(dir: &std::path::Path) -> Result<Self, ConditionLoadError> {
        let mut reg = Self::default_registry();
        if !dir.exists() {
            return Ok(reg);
        }
        for entry in std::fs::read_dir(dir)
            .map_err(|e| ConditionLoadError::Io(dir.to_path_buf(), e.to_string()))?
            .flatten()
        {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("ron") {
                continue;
            }
            if path
                .file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|n| n.starts_with('_'))
            {
                continue;
            }
            let body =
                std::fs::read_to_string(&path).map_err(|e| ConditionLoadError::Io(path.clone(), e.to_string()))?;
            match ron::from_str::<ConditionSpec>(&body) {
                Ok(spec) => {
                    reg.specs.insert(spec.kind.as_str().to_string(), spec);
                }
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "condition spec parse failed");
                    return Err(ConditionLoadError::Parse(path.clone(), e.to_string()));
                }
            }
        }
        Ok(reg)
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ConditionLoadError {
    #[error("io error reading {0:?}: {1}")]
    Io(std::path::PathBuf, String),
    #[error("parse error in {0:?}: {1}")]
    Parse(std::path::PathBuf, String),
}

/// One active mental-health condition on an actor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorCondition {
    pub kind: ConditionKind,
    pub stage: ConditionStage,
    pub triggered_at_tick: u64,
    pub stage_entered_tick: u64,
    pub treatment: TreatmentPlan,
    /// The comorbid partner condition, when this one arose as a comorbidity.
    pub comorbid_with: Option<ConditionKind>,
    /// Tick until which a panic attack freezes the actor (0 = not frozen).
    pub panic_frozen_until_tick: u64,
}

impl ActorCondition {
    fn new(kind: ConditionKind, tick: u64) -> Self {
        Self {
            kind,
            stage: ConditionStage::Triggered,
            triggered_at_tick: tick,
            stage_entered_tick: tick,
            treatment: TreatmentPlan::default(),
            comorbid_with: None,
            panic_frozen_until_tick: 0,
        }
    }

    fn enter(&mut self, stage: ConditionStage, tick: u64) {
        self.stage = stage;
        self.stage_entered_tick = tick;
    }

    /// True when a panic attack currently freezes the actor at `tick`.
    pub fn is_panic_frozen(&self, tick: u64) -> bool {
        tick < self.panic_frozen_until_tick
    }
}

/// Sliding window of witnessed-death ticks. Declares a PTSD-eligible trauma
/// once `threshold` deaths land within `window_seconds`. Latches `fired` so
/// the trauma is declared exactly once per run.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WitnessWindow {
    pub deaths: VecDeque<u64>,
    pub fired: bool,
}

impl WitnessWindow {
    /// Record one witnessed death at `tick`. Returns `true` exactly once — on
    /// the death that crosses the threshold within the window.
    pub fn record_death(&mut self, tick: u64, tick_rate_hz: u32) -> bool {
        let window_ticks = (WITNESS_WINDOW_SECONDS * tick_rate_hz.max(1) as f32) as u64;
        self.deaths.push_back(tick);
        while let Some(&front) = self.deaths.front() {
            if tick.saturating_sub(front) > window_ticks {
                self.deaths.pop_front();
            } else {
                break;
            }
        }
        if !self.fired && self.deaths.len() as u32 >= WITNESS_DEATH_THRESHOLD {
            self.fired = true;
            return true;
        }
        false
    }
}

/// Sliding window of stim-dose ticks. Declares an Addiction once `threshold`
/// doses land within `window_seconds`. Tracks the last-dose tick for the
/// withdrawal-absence check.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DoseWindow {
    pub doses: VecDeque<u64>,
    pub last_dose_tick: Option<u64>,
    pub fired: bool,
}

impl DoseWindow {
    /// Record one stim dose at `tick`. Returns `true` exactly once — on the
    /// dose that crosses the addiction threshold within the 30-day window.
    pub fn record_dose(&mut self, tick: u64, tick_rate_hz: u32) -> bool {
        let window_ticks = (ADDICTION_WINDOW_SECONDS * tick_rate_hz.max(1) as f32) as u64;
        self.doses.push_back(tick);
        self.last_dose_tick = Some(tick);
        while let Some(&front) = self.doses.front() {
            if tick.saturating_sub(front) > window_ticks {
                self.doses.pop_front();
            } else {
                break;
            }
        }
        if !self.fired && self.doses.len() as u32 >= ADDICTION_DOSE_THRESHOLD {
            self.fired = true;
            return true;
        }
        false
    }

    /// Seconds since the last dose (`f32::INFINITY` if none ever taken).
    pub fn seconds_since_last_dose(&self, tick: u64, tick_rate_hz: u32) -> f32 {
        match self.last_dose_tick {
            Some(last) => ticks_to_seconds(tick.saturating_sub(last), tick_rate_hz),
            None => f32::INFINITY,
        }
    }
}

// ----- Events -----

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConditionTriggeredEvent {
    pub actor_id: u64,
    pub tick: u64,
    pub condition: ConditionKind,
    pub reason: TriggerReason,
    /// Stage the condition occupies right after triggering (Acute).
    pub stage: ConditionStage,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PsychStageChangedEvent {
    pub actor_id: u64,
    pub tick: u64,
    pub condition: ConditionKind,
    pub from: ConditionStage,
    pub to: ConditionStage,
    /// Trait id granted by entering `to` (chronic_* / refractory_*), if any.
    pub trait_granted: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanicAttackEvent {
    pub actor_id: u64,
    pub tick: u64,
    pub condition: ConditionKind,
    pub freeze_seconds: f32,
    pub freeze_until_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemissionAchievedEvent {
    pub actor_id: u64,
    pub tick: u64,
    pub condition: ConditionKind,
    /// Whether a treatment course (vs natural resolution) drove the remission.
    pub treated: bool,
    pub trait_granted: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PsychRelapsedEvent {
    pub actor_id: u64,
    pub tick: u64,
    pub condition: ConditionKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AddictionDevelopedEvent {
    pub actor_id: u64,
    pub tick: u64,
    pub dose_count: u32,
    /// `chronic_addiction` — the permanent vulnerability trait.
    pub trait_granted: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WithdrawalStartedEvent {
    pub actor_id: u64,
    pub tick: u64,
    pub hours_since_dose: f32,
    pub aim_wobble_multiplier: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TherapySessionEvent {
    pub actor_id: u64,
    pub tick: u64,
    pub condition: ConditionKind,
    pub session_index: u32,
    pub sessions_required: u32,
    pub efficacy: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MedicationStartedEvent {
    pub actor_id: u64,
    pub tick: u64,
    pub condition: ConditionKind,
    pub medication: PsychMedClass,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComorbidityDetectedEvent {
    pub actor_id: u64,
    pub tick: u64,
    pub primary: ConditionKind,
    pub comorbid: ConditionKind,
}

/// Aggregated output of one mental-health tick for an actor.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PsychOutput {
    pub triggered: Vec<ConditionTriggeredEvent>,
    pub stage_changed: Vec<PsychStageChangedEvent>,
    pub panic_attacks: Vec<PanicAttackEvent>,
    pub remissions: Vec<RemissionAchievedEvent>,
    pub relapses: Vec<PsychRelapsedEvent>,
}

/// Per-actor mental-health state: the active conditions, the witness-death
/// trigger window, and the stim-dose trigger window.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ActorMentalHealth {
    pub origin: OriginId,
    pub active: Vec<ActorCondition>,
    pub witness: WitnessWindow,
    pub doses: DoseWindow,
}

impl ActorMentalHealth {
    pub fn with_origin(origin: OriginId) -> Self {
        Self {
            origin,
            ..Default::default()
        }
    }

    pub fn find(&self, kind: ConditionKind) -> Option<&ActorCondition> {
        self.active.iter().find(|c| c.kind == kind)
    }

    pub fn find_mut(&mut self, kind: ConditionKind) -> Option<&mut ActorCondition> {
        self.active.iter_mut().find(|c| c.kind == kind)
    }

    /// True when a condition of `kind` exists in any stage.
    pub fn has(&self, kind: ConditionKind) -> bool {
        self.active.iter().any(|c| c.kind == kind)
    }

    /// True when a condition of `kind` is actively symptomatic.
    pub fn is_symptomatic(&self, kind: ConditionKind) -> bool {
        self.find(kind).is_some_and(|c| c.stage.is_symptomatic())
    }

    /// `Some("insomnia")` when an active Insomnia condition blocks sleep — the
    /// `act.player.sleep` rejection reason.
    pub fn can_sleep(&self) -> Option<&'static str> {
        if self.is_symptomatic(ConditionKind::Insomnia) {
            Some("insomnia")
        } else {
            None
        }
    }

    /// True when any active condition currently freezes the actor (panic).
    pub fn is_panic_frozen(&self, tick: u64) -> bool {
        self.active.iter().any(|c| c.is_panic_frozen(tick))
    }

    /// Trigger a condition. Returns the `condition_triggered` event (the
    /// condition is created at Triggered and immediately advanced to Acute) —
    /// or `None` when:
    ///   - the origin is synthetic (robots/drones have no mental health), or
    ///   - a condition of this kind already exists (no re-trigger).
    pub fn trigger(
        &mut self,
        actor_id: u64,
        kind: ConditionKind,
        reason: TriggerReason,
        tick: u64,
    ) -> Option<ConditionTriggeredEvent> {
        if self.origin.is_synthetic() || self.has(kind) {
            return None;
        }
        let mut condition = ActorCondition::new(kind, tick);
        condition.enter(ConditionStage::Acute, tick);
        self.active.push(condition);
        Some(ConditionTriggeredEvent {
            actor_id,
            tick,
            condition: kind,
            reason,
            stage: ConditionStage::Acute,
        })
    }

    /// Record a witnessed squadmate death. When 3 land within 60s and the
    /// origin is non-synthetic, triggers PTSD and returns the trigger event.
    pub fn record_witnessed_death(
        &mut self,
        actor_id: u64,
        tick: u64,
        tick_rate_hz: u32,
    ) -> Option<ConditionTriggeredEvent> {
        if self.witness.record_death(tick, tick_rate_hz) {
            return self.trigger(actor_id, ConditionKind::Ptsd, TriggerReason::WitnessDeaths, tick);
        }
        None
    }

    /// Record a combat-stim dose. When 7 land within 30 days and the origin is
    /// non-synthetic, develops an Addiction (permanent vulnerability) and
    /// returns the event.
    pub fn record_stim_dose(
        &mut self,
        actor_id: u64,
        tick: u64,
        tick_rate_hz: u32,
    ) -> Option<AddictionDevelopedEvent> {
        let crossed = self.doses.record_dose(tick, tick_rate_hz);
        if !crossed || self.origin.is_synthetic() {
            return None;
        }
        let dose_count = self.doses.doses.len() as u32;
        // The Addiction condition itself (drives the lifecycle + withdrawal).
        self.trigger(actor_id, ConditionKind::Addiction, TriggerReason::DrugDoses, tick);
        Some(AddictionDevelopedEvent {
            actor_id,
            tick,
            dose_count,
            trait_granted: ConditionKind::Addiction.chronic_trait(),
        })
    }

    /// Check the withdrawal-absence trigger: an addicted actor whose last stim
    /// dose was more than 12h ago starts Withdrawal (2× aim wobble). Returns
    /// the event the first time it fires.
    pub fn check_withdrawal(
        &mut self,
        actor_id: u64,
        tick: u64,
        tick_rate_hz: u32,
    ) -> Option<WithdrawalStartedEvent> {
        // Only an addicted, non-synthetic actor not already withdrawing.
        if !self.has(ConditionKind::Addiction) || self.has(ConditionKind::Withdrawal) {
            return None;
        }
        let secs = self.doses.seconds_since_last_dose(tick, tick_rate_hz);
        if secs <= crate::WITHDRAWAL_ABSENCE_SECONDS {
            return None;
        }
        self.trigger(actor_id, ConditionKind::Withdrawal, TriggerReason::DrugAbsence, tick)?;
        Some(WithdrawalStartedEvent {
            actor_id,
            tick,
            hours_since_dose: secs / crate::HOUR_SECONDS,
            aim_wobble_multiplier: crate::WITHDRAWAL_AIM_WOBBLE_MULTIPLIER,
        })
    }

    /// Begin or resume a therapy course for `kind`, recording one session.
    /// Returns the session event (with the deterministic efficacy roll).
    pub fn record_therapy_session(
        &mut self,
        actor_id: u64,
        kind: ConditionKind,
        registry: &ConditionRegistry,
        seed: u64,
        tick: u64,
    ) -> Option<TherapySessionEvent> {
        let required = registry.get(kind)?.therapy_sessions_required;
        let condition = self.find_mut(kind)?;
        condition.treatment.record_therapy_session();
        let session_index = condition.treatment.therapy_sessions_completed;
        let efficacy = crate::therapy_efficacy_roll(seed, actor_id, kind, session_index);
        Some(TherapySessionEvent {
            actor_id,
            tick,
            condition: kind,
            session_index,
            sessions_required: required,
            efficacy,
        })
    }

    /// Begin a medication course for `kind`. Returns the event, or `None` when
    /// the condition is absent.
    pub fn start_medication(
        &mut self,
        actor_id: u64,
        kind: ConditionKind,
        medication: PsychMedClass,
        tick: u64,
    ) -> Option<MedicationStartedEvent> {
        let condition = self.find_mut(kind)?;
        condition.treatment.start_medication(medication, tick);
        Some(MedicationStartedEvent {
            actor_id,
            tick,
            condition: kind,
            medication,
        })
    }

    /// Advance every active condition by one tick: panic rolls, treatment
    /// resolution, natural progression, and relapse. Deterministic.
    pub fn tick(
        &mut self,
        actor_id: u64,
        registry: &ConditionRegistry,
        tick: u64,
        tick_rate_hz: u32,
        seed: u64,
    ) -> PsychOutput {
        let mut out = PsychOutput::default();
        let kinds: Vec<ConditionKind> = self.active.iter().map(|c| c.kind).collect();
        for kind in kinds {
            let Some(spec) = registry.get(kind).cloned() else {
                continue;
            };
            advance_one(self, actor_id, kind, &spec, tick, tick_rate_hz, seed, &mut out);
        }
        out
    }
}

/// Resolve a treated condition (therapy + medication both met). Returns true
/// when the condition resolved this tick (so the natural clock is skipped).
fn resolve_treatment(
    state: &mut ActorMentalHealth,
    actor_id: u64,
    kind: ConditionKind,
    spec: &ConditionSpec,
    tick: u64,
    tick_rate_hz: u32,
    seed: u64,
    out: &mut PsychOutput,
) -> bool {
    let ready;
    let from;
    {
        let Some(condition) = state.find_mut(kind) else {
            return false;
        };
        from = condition.stage;
        if condition.treatment.resolved
            || matches!(condition.stage, ConditionStage::Remission | ConditionStage::Refractory)
        {
            return false;
        }
        let therapy_ok = condition.treatment.therapy_sessions_completed >= spec.therapy_sessions_required;
        let med_ok = match spec.medication {
            None => true,
            Some(_) => {
                condition.treatment.medication_started()
                    && condition.treatment.medication_active_for(tick, tick_rate_hz) >= spec.medication_onset_seconds
            }
        };
        ready = therapy_ok && med_ok;
    }
    if !ready {
        return false;
    }
    let outcome_roll = mh_roll(seed, actor_id, kind, SALT_OUTCOME);
    let Some(condition) = state.find_mut(kind) else {
        return false;
    };
    condition.treatment.resolved = true;
    // From Chronic, the failure band lands on Refractory (treatment-resistant);
    // from Acute/Subacute it lands on Chronic.
    if outcome_roll < spec.chronic_chance {
        let to = if from == ConditionStage::Chronic {
            ConditionStage::Refractory
        } else {
            ConditionStage::Chronic
        };
        condition.enter(to, tick);
        let trait_granted = if to == ConditionStage::Refractory {
            kind.refractory_trait()
        } else {
            kind.chronic_trait()
        };
        push_stage(out, actor_id, tick, kind, from, to, Some(trait_granted));
    } else {
        condition.enter(ConditionStage::Remission, tick);
        let trait_granted = kind.recovered_trait();
        push_stage(out, actor_id, tick, kind, from, ConditionStage::Remission, None);
        out.remissions.push(RemissionAchievedEvent {
            actor_id,
            tick,
            condition: kind,
            treated: true,
            trait_granted,
        });
    }
    true
}

fn advance_one(
    state: &mut ActorMentalHealth,
    actor_id: u64,
    kind: ConditionKind,
    spec: &ConditionSpec,
    tick: u64,
    tick_rate_hz: u32,
    seed: u64,
    out: &mut PsychOutput,
) {
    // 1. Panic roll (symptomatic + has panic chance + not already frozen).
    panic_pass(state, actor_id, kind, spec, tick, tick_rate_hz, seed, out);

    // 2. Treatment resolution takes priority over the natural clock.
    if resolve_treatment(state, actor_id, kind, spec, tick, tick_rate_hz, seed, out) {
        return;
    }

    let Some(condition) = state.find_mut(kind) else {
        return;
    };
    let elapsed = ticks_to_seconds(tick.saturating_sub(condition.stage_entered_tick), tick_rate_hz);
    let from = condition.stage;
    match condition.stage {
        ConditionStage::Triggered => {
            condition.enter(ConditionStage::Acute, tick);
            push_stage(out, actor_id, tick, kind, from, ConditionStage::Acute, None);
        }
        ConditionStage::Acute => {
            if spec.resolves_naturally {
                if elapsed >= spec.natural_resolve_seconds {
                    condition.enter(ConditionStage::Remission, tick);
                    push_stage(out, actor_id, tick, kind, from, ConditionStage::Remission, None);
                    out.remissions.push(RemissionAchievedEvent {
                        actor_id,
                        tick,
                        condition: kind,
                        treated: false,
                        trait_granted: kind.recovered_trait(),
                    });
                }
            } else if elapsed >= spec.acute_window_seconds {
                condition.enter(ConditionStage::Subacute, tick);
                push_stage(out, actor_id, tick, kind, from, ConditionStage::Subacute, None);
            }
        }
        ConditionStage::Subacute => {
            if elapsed >= spec.subacute_window_seconds {
                // Spontaneous (untreated) resolution: Chronic vs Remission.
                let roll = mh_roll(seed, actor_id, kind, SALT_OUTCOME);
                if roll < spec.chronic_chance {
                    condition.enter(ConditionStage::Chronic, tick);
                    push_stage(
                        out,
                        actor_id,
                        tick,
                        kind,
                        from,
                        ConditionStage::Chronic,
                        Some(kind.chronic_trait()),
                    );
                } else {
                    condition.enter(ConditionStage::Remission, tick);
                    push_stage(out, actor_id, tick, kind, from, ConditionStage::Remission, None);
                    out.remissions.push(RemissionAchievedEvent {
                        actor_id,
                        tick,
                        condition: kind,
                        treated: false,
                        trait_granted: kind.recovered_trait(),
                    });
                }
            }
        }
        ConditionStage::Remission => {
            // Relapse risk: a per-tick roll returns the condition to Acute.
            if spec.relapse_chance_per_tick > 0.0 {
                let roll = mh_roll(seed, actor_id, kind, SALT_RELAPSE ^ tick);
                if roll < spec.relapse_chance_per_tick {
                    condition.treatment.resolved = false;
                    condition.enter(ConditionStage::Acute, tick);
                    push_stage(out, actor_id, tick, kind, from, ConditionStage::Acute, None);
                    out.relapses.push(PsychRelapsedEvent {
                        actor_id,
                        tick,
                        condition: kind,
                    });
                }
            }
        }
        // Chronic stays until a fresh treatment course resolves it; Refractory
        // is the treatment-resistant terminal state.
        ConditionStage::Chronic | ConditionStage::Refractory => {}
    }
}

/// Per-tick panic-attack roll. Sets the freeze window and emits the event.
fn panic_pass(
    state: &mut ActorMentalHealth,
    actor_id: u64,
    kind: ConditionKind,
    spec: &ConditionSpec,
    tick: u64,
    tick_rate_hz: u32,
    seed: u64,
    out: &mut PsychOutput,
) {
    if spec.panic_chance_per_tick <= 0.0 {
        return;
    }
    let Some(condition) = state.find_mut(kind) else {
        return;
    };
    if !condition.stage.is_symptomatic() || condition.is_panic_frozen(tick) {
        return;
    }
    // Per-tick independent stream (salt folds the tick) for reproducible timing.
    let roll = mh_roll(seed, actor_id, kind, SALT_PANIC ^ tick);
    if roll >= spec.panic_chance_per_tick {
        return;
    }
    let dur_roll = mh_roll(seed, actor_id, kind, SALT_PANIC_FREEZE ^ tick);
    let freeze_seconds = PANIC_FREEZE_MIN_SECONDS + dur_roll * (PANIC_FREEZE_MAX_SECONDS - PANIC_FREEZE_MIN_SECONDS);
    let freeze_ticks = (freeze_seconds * tick_rate_hz.max(1) as f32) as u64;
    let freeze_until_tick = tick.saturating_add(freeze_ticks.max(1));
    condition.panic_frozen_until_tick = freeze_until_tick;
    out.panic_attacks.push(PanicAttackEvent {
        actor_id,
        tick,
        condition: kind,
        freeze_seconds,
        freeze_until_tick,
    });
}

fn push_stage(
    out: &mut PsychOutput,
    actor_id: u64,
    tick: u64,
    kind: ConditionKind,
    from: ConditionStage,
    to: ConditionStage,
    trait_granted: Option<String>,
) {
    out.stage_changed.push(PsychStageChangedEvent {
        actor_id,
        tick,
        condition: kind,
        from,
        to,
        trait_granted,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg() -> ConditionRegistry {
        ConditionRegistry::default_registry()
    }

    #[test]
    fn eight_conditions_round_trip() {
        assert_eq!(ConditionKind::all().len(), 8);
        for &k in ConditionKind::all() {
            assert_eq!(ConditionKind::from_str(k.as_str()), Some(k));
            assert!(reg().get(k).is_some(), "no spec for {}", k.as_str());
        }
    }

    #[test]
    fn trait_strings_match_convention() {
        assert_eq!(ConditionKind::Ptsd.recovered_trait(), "recovered_from_ptsd");
        assert_eq!(ConditionKind::Depression.chronic_trait(), "chronic_depression");
        assert_eq!(ConditionKind::Ptsd.refractory_trait(), "refractory_ptsd");
    }

    #[test]
    fn trigger_creates_acute_condition() {
        let mut s = ActorMentalHealth::with_origin(OriginId::Human);
        let ev = s
            .trigger(7, ConditionKind::Ptsd, TriggerReason::WitnessDeaths, 100)
            .unwrap();
        assert_eq!(ev.condition, ConditionKind::Ptsd);
        assert_eq!(ev.stage, ConditionStage::Acute);
        assert_eq!(s.find(ConditionKind::Ptsd).unwrap().stage, ConditionStage::Acute);
    }

    #[test]
    fn double_trigger_is_ignored() {
        let mut s = ActorMentalHealth::with_origin(OriginId::Human);
        assert!(s.trigger(7, ConditionKind::Ptsd, TriggerReason::WitnessDeaths, 0).is_some());
        assert!(s.trigger(7, ConditionKind::Ptsd, TriggerReason::WitnessDeaths, 1).is_none());
    }

    #[test]
    fn synthetic_origin_never_triggers() {
        for origin in [OriginId::Robot, OriginId::Drone] {
            let mut s = ActorMentalHealth::with_origin(origin);
            assert!(s.trigger(7, ConditionKind::Ptsd, TriggerReason::WitnessDeaths, 0).is_none());
            assert!(s.active.is_empty());
        }
    }

    #[test]
    fn android_is_not_synthetic_and_can_trigger() {
        let mut s = ActorMentalHealth::with_origin(OriginId::Android);
        assert!(s.trigger(7, ConditionKind::Ptsd, TriggerReason::WitnessDeaths, 0).is_some());
    }

    #[test]
    fn witness_window_fires_on_third_death_within_60s() {
        let mut s = ActorMentalHealth::with_origin(OriginId::Human);
        // 60 Hz. Three deaths within 60s (3600 ticks).
        assert!(s.record_witnessed_death(7, 0, 60).is_none());
        assert!(s.record_witnessed_death(7, 1000, 60).is_none());
        let ev = s.record_witnessed_death(7, 2000, 60).expect("third death triggers PTSD");
        assert_eq!(ev.condition, ConditionKind::Ptsd);
        assert_eq!(s.find(ConditionKind::Ptsd).unwrap().stage, ConditionStage::Acute);
    }

    #[test]
    fn witness_window_prunes_outside_60s() {
        let mut s = ActorMentalHealth::with_origin(OriginId::Human);
        // Deaths spaced > 60s apart never accumulate to 3 in-window.
        let step = 3601u64; // just over 60s at 60 Hz
        assert!(s.record_witnessed_death(7, 0, 60).is_none());
        assert!(s.record_witnessed_death(7, step, 60).is_none());
        assert!(s.record_witnessed_death(7, step * 2, 60).is_none());
        assert!(!s.has(ConditionKind::Ptsd));
    }

    #[test]
    fn addiction_develops_after_seven_doses() {
        let mut s = ActorMentalHealth::with_origin(OriginId::Human);
        let day = (crate::DAY_SECONDS * 60.0) as u64;
        let mut developed = None;
        for d in 0..7u64 {
            developed = s.record_stim_dose(7, d * day, 60);
        }
        let ev = developed.expect("seventh dose develops addiction");
        assert_eq!(ev.dose_count, 7);
        assert_eq!(ev.trait_granted, "chronic_addiction");
        assert!(s.has(ConditionKind::Addiction));
    }

    #[test]
    fn withdrawal_starts_after_12h_absence() {
        let mut s = ActorMentalHealth::with_origin(OriginId::Human);
        let day = (crate::DAY_SECONDS * 60.0) as u64;
        for d in 0..7u64 {
            s.record_stim_dose(7, d * day, 60);
        }
        let last = 6 * day;
        // 13h after the last dose → withdrawal.
        let t = last + (13.0 * 3600.0 * 60.0) as u64;
        let ev = s.check_withdrawal(7, t, 60).expect("withdrawal after 12h absence");
        assert!((ev.aim_wobble_multiplier - 2.0).abs() < 1e-6);
        assert!(ev.hours_since_dose >= 12.0);
        // The withdrawal now degrades the actor's aim through the effects
        // consumer (the 2× wobble → additive aim spread the sim applies).
        assert!(crate::effects::condition_aim_spread_bonus_radians(&s) > 0.0);
    }

    #[test]
    fn ptsd_reaches_remission_after_therapy_plus_ssri() {
        // Scenario 4: 10 sessions + 14d SSRI → remission + recovered_from_ptsd.
        // Search an actor whose outcome roll lands in the remission band (>=0.30).
        let registry = reg();
        let mut actor = None;
        for a in 0..200u64 {
            if mh_roll(0xC0FFEE, a, ConditionKind::Ptsd, SALT_OUTCOME) >= 0.30 {
                actor = Some(a);
                break;
            }
        }
        let actor_id = actor.expect("some actor lands in remission band");
        let mut s = ActorMentalHealth::with_origin(OriginId::Human);
        s.trigger(actor_id, ConditionKind::Ptsd, TriggerReason::WitnessDeaths, 0);
        for _ in 0..10 {
            s.record_therapy_session(actor_id, ConditionKind::Ptsd, &registry, 0xC0FFEE, 0);
        }
        s.start_medication(actor_id, ConditionKind::Ptsd, PsychMedClass::Ssri, 0);
        // Jump 14 days on SSRI.
        let t = (crate::SSRI_ONSET_SECONDS * 60.0) as u64 + 1;
        let out = s.tick(actor_id, &registry, t, 60, 0xC0FFEE);
        let rem = out.remissions.iter().find(|r| r.condition == ConditionKind::Ptsd).expect("remission");
        assert!(rem.treated);
        assert_eq!(rem.trait_granted, "recovered_from_ptsd");
        assert_eq!(s.find(ConditionKind::Ptsd).unwrap().stage, ConditionStage::Remission);
    }

    #[test]
    fn ptsd_can_resolve_to_chronic() {
        // The 30% band lands on Chronic with chronic_ptsd trait.
        let registry = reg();
        let mut actor = None;
        for a in 0..200u64 {
            if mh_roll(0xBEEF, a, ConditionKind::Ptsd, SALT_OUTCOME) < 0.30 {
                actor = Some(a);
                break;
            }
        }
        let actor_id = actor.expect("some actor lands in chronic band");
        let mut s = ActorMentalHealth::with_origin(OriginId::Human);
        s.trigger(actor_id, ConditionKind::Ptsd, TriggerReason::WitnessDeaths, 0);
        for _ in 0..10 {
            s.record_therapy_session(actor_id, ConditionKind::Ptsd, &registry, 0xBEEF, 0);
        }
        s.start_medication(actor_id, ConditionKind::Ptsd, PsychMedClass::Ssri, 0);
        let t = (crate::SSRI_ONSET_SECONDS * 60.0) as u64 + 1;
        let out = s.tick(actor_id, &registry, t, 60, 0xBEEF);
        let sc = out
            .stage_changed
            .iter()
            .find(|e| e.to == ConditionStage::Chronic)
            .expect("chronic transition");
        assert_eq!(sc.trait_granted.as_deref(), Some("chronic_ptsd"));
    }

    #[test]
    fn insomnia_blocks_sleep() {
        let mut s = ActorMentalHealth::with_origin(OriginId::Human);
        assert_eq!(s.can_sleep(), None);
        s.trigger(7, ConditionKind::Insomnia, TriggerReason::SleepDeprivation, 0);
        assert_eq!(s.can_sleep(), Some("insomnia"));
    }

    #[test]
    fn panic_disorder_freezes_for_3_to_8s() {
        let registry = reg();
        let mut s = ActorMentalHealth::with_origin(OriginId::Human);
        s.trigger(7, ConditionKind::PanicDisorder, TriggerReason::PanicThreshold, 0);
        // Find a tick where the panic roll fires.
        let mut fired = None;
        for t in 1..2_000_000u64 {
            let out = s.tick(7, &registry, t, 60, 999);
            if let Some(p) = out.panic_attacks.first() {
                fired = Some(p.clone());
                break;
            }
        }
        let p = fired.expect("a panic attack must fire within a day of ticks");
        assert!((3.0..=8.0).contains(&p.freeze_seconds), "freeze {} out of range", p.freeze_seconds);
        assert!(s.find(ConditionKind::PanicDisorder).unwrap().is_panic_frozen(p.tick));
    }

    #[test]
    fn panic_timing_is_deterministic_for_same_seed() {
        // Scenario 9: identical seed reproduces panic-attack ticks over a day.
        let registry = reg();
        let run = || {
            let mut s = ActorMentalHealth::with_origin(OriginId::Human);
            s.trigger(7, ConditionKind::PanicDisorder, TriggerReason::PanicThreshold, 0);
            let mut ticks = Vec::new();
            // 1 in-game day at 1 Hz for test speed (determinism is tick-keyed).
            for t in 1..=86_400u64 {
                let out = s.tick(7, &registry, t, 1, 0xD00D);
                for p in &out.panic_attacks {
                    ticks.push(p.tick);
                }
            }
            ticks
        };
        let a = run();
        let b = run();
        assert_eq!(a, b);
        assert!(!a.is_empty(), "at least one panic attack in a day");
    }

    #[test]
    fn withdrawal_resolves_naturally_over_two_weeks() {
        let registry = reg();
        let mut s = ActorMentalHealth::with_origin(OriginId::Human);
        s.trigger(7, ConditionKind::Withdrawal, TriggerReason::DrugAbsence, 0);
        // Jump 14 days; Acute → Remission (natural).
        let t = (crate::WITHDRAWAL_RESOLVE_SECONDS * 60.0) as u64 + 1;
        let out = s.tick(7, &registry, t, 60, 1);
        assert!(out.remissions.iter().any(|r| r.condition == ConditionKind::Withdrawal && !r.treated));
        assert_eq!(s.find(ConditionKind::Withdrawal).unwrap().stage, ConditionStage::Remission);
    }

    #[test]
    fn untreated_acute_progresses_to_subacute() {
        let registry = reg();
        let mut s = ActorMentalHealth::with_origin(OriginId::Human);
        s.trigger(7, ConditionKind::Depression, TriggerReason::SustainedStress, 0);
        // Depression acute window = 30 days.
        let t = (30.0 * crate::DAY_SECONDS * 60.0) as u64 + 1;
        let out = s.tick(7, &registry, t, 60, 1);
        assert!(out.stage_changed.iter().any(|e| e.to == ConditionStage::Subacute));
    }

    #[test]
    fn registry_round_trips_through_ron() {
        for &k in ConditionKind::all() {
            let spec = reg().lookup(k).clone();
            let str = ron::to_string(&spec).unwrap();
            let back: ConditionSpec = ron::from_str(&str).unwrap();
            assert_eq!(spec, back);
        }
    }
}
