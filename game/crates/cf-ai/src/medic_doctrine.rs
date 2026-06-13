//! **M14H** § Field-medic doctrine extension to the M7 Medic role.
//!
//! Wraps [`cf_treatment::FieldMedicDecisionTree`] in an AI-side wrapper
//! that the M22 utility scorer + thinking stack can drive.
//!
//! Integration contract (Gherkin scenario 5): when the M7 Utility scorer
//! picks `TaskType::TriageDownedAlly` for a Medic archetype, the engine
//! calls [`MedicDoctrineState::resolve_next_action`] with a snapshot of
//! every triage-eligible patient + a per-target reachability predicate.
//! The resolver:
//!
//! 1. **Assesses** the candidate patient list (Step 1).
//! 2. **Triages** by compound TTD + wound severity + cardiac arrest
//!    (Step 2).
//! 3. **Stabilizes** life-threatening conditions first (Step 3).
//! 4. **Treats** the highest-priority wound (Step 4).
//! 5. **Monitors** by re-scanning every N ticks (Step 5).
//!
//! The returned [`MedicAction`] is dispatched by the cf-control engine
//! (MoveTo / Apply / Cpr / Defib / Scan), and a `patient.assessed`
//! replay event is emitted with the current decision-tree step.

use serde::{Deserialize, Serialize};

pub use cf_treatment::{
    DecisionStep, FieldMedicDecisionTree, MedicAction, PatientSnapshot, WoundPriority,
};
use cf_treatment::TreatmentKind;

/// decision tree's `last_treatment_applied` + `last_monitor_tick` state
/// survive between AI updates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MedicDoctrineState {
    pub medic_actor_id: u64,
    pub step: DecisionStep,
    pub current_target: Option<u64>,
    pub last_treatment_kind: Option<String>,
    pub last_monitor_tick: u64,
}

impl MedicDoctrineState {
    pub fn new(medic_actor_id: u64) -> Self {
        Self {
            medic_actor_id,
            step: DecisionStep::Assess,
            current_target: None,
            last_treatment_kind: None,
            last_monitor_tick: 0,
        }
    }

    pub fn tree(&self) -> FieldMedicDecisionTree {
        let mut t = FieldMedicDecisionTree::new(self.medic_actor_id);
        t.step = self.step;
        t.current_target = self.current_target;
        t.last_treatment_applied = self
            .last_treatment_kind
            .as_deref()
            .and_then(TreatmentKind::from_str);
        t.last_monitor_tick = self.last_monitor_tick;
        t
    }

    pub fn write_back(&mut self, tree: &FieldMedicDecisionTree) {
        self.step = tree.step;
        self.current_target = tree.current_target;
        self.last_treatment_kind = tree
            .last_treatment_applied
            .map(|k| k.as_str().to_string());
        self.last_monitor_tick = tree.last_monitor_tick;
    }

    /// tree is a finite-state machine; AI medic doctrine consumes via M22
    /// utility scorer". This resolver is the canonical M22 entry point:
    /// the M7 thinking stack invokes it after Utility scores
    /// `TriageDownedAlly` highest.
    ///
    /// Side effects: mutates `self.step`, `self.current_target`,
    /// `self.last_treatment_kind`, `self.last_monitor_tick`.
    ///
    /// Returns:
    /// - The chosen [`MedicAction`].
    /// - A [`MedicAssessment`] snapshot suitable for the
    ///   `patient.assessed` replay event payload.
    pub fn resolve_next_action(
        &mut self,
        sim_tick: u64,
        patients: &[PatientSnapshot],
        medic_within_reach_of: impl Fn(u64) -> bool,
        last_scan_age_ticks: impl Fn(u64) -> u64,
    ) -> (MedicAction, MedicAssessment) {
        let mut tree = self.tree();
        let action = tree.next_action(
            sim_tick,
            patients,
            medic_within_reach_of,
            last_scan_age_ticks,
        );
        self.write_back(&tree);
        let target = self.current_target.unwrap_or(0);
        let snapshot = patients.iter().find(|p| p.actor_id == target);
        let assessment = MedicAssessment {
            medic_actor_id: self.medic_actor_id,
            target_actor_id: target,
            step: self.step,
            compound_ttd_seconds: snapshot.map(|p| p.compound_ttd_seconds).unwrap_or(f32::INFINITY),
            wound_count: snapshot.map(|p| p.wounds.len() as u32).unwrap_or(0),
            highest_priority_treatment: match &action {
                MedicAction::Apply { kind, .. } => Some(*kind),
                _ => None,
            },
        };
        (action, assessment)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MedicAssessment {
    pub medic_actor_id: u64,
    pub target_actor_id: u64,
    pub step: DecisionStep,
    pub compound_ttd_seconds: f32,
    pub wound_count: u32,
    pub highest_priority_treatment: Option<TreatmentKind>,
}

// ===========================================================================
// M16C — Medic `TreatPsych` priority + psych-emergency triage.
//
// The Medic role gains a parallel psych-triage pass: given an ally's
// `cf_mental_health::ActorMentalHealth` state it picks the highest-priority
// psychological intervention (calm a panicking actor, administer the indicated
// medication, escort to a therapy NPC, or monitor an in-progress course). The
// M22 utility scorer weights the `TreatPsych` task by [`psych_emergency_level`]
// and adds [`TREAT_PSYCH_UTILITY_BONUS`] when an emergency is active.
// ===========================================================================

use cf_mental_health::{ActorMentalHealth, ConditionKind, ConditionRegistry, PsychMedClass};

/// Utility scorer bonus the Medic adds to `TreatPsych(target)` when the target
/// is in an active psych emergency (mirrors [`super::auto_triage`]'s triage
/// bonus shape).
pub const TREAT_PSYCH_UTILITY_BONUS: f32 = 0.4;

/// How urgent an ally's psych state is for the Medic's `TreatPsych` priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PsychEmergencyLevel {
    /// No symptomatic condition — nothing to treat.
    None,
    /// Symptomatic but treatment is underway / non-critical.
    Routine,
    /// Untreated high-risk condition (withdrawal HP-drain, depression suicide
    /// risk) — treat soon.
    Urgent,
    /// Actor is incapacitated by an active panic-attack freeze — treat now.
    Critical,
}

/// The psych intervention the Medic should take for one ally.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PsychTreatAction {
    /// Nothing to do.
    None,
    /// Treatment is underway (med active + therapy in progress) — keep watch.
    Monitor { target: u64, condition: ConditionKind },
    /// Stay with and calm an actor frozen by a panic attack.
    Calm { target: u64, condition: ConditionKind },
    /// Administer the indicated medication (not yet started).
    AdministerMedication {
        target: u64,
        condition: ConditionKind,
        medication: PsychMedClass,
    },
    /// Escort the actor to a therapy NPC (medication done / not required, but
    /// therapy sessions remain).
    EscortToTherapy { target: u64, condition: ConditionKind },
}

/// Triage rank for a symptomatic condition (higher = more urgent). Drives both
/// the emergency level and which condition the Medic addresses first.
fn psych_triage_rank(kind: ConditionKind) -> u8 {
    match kind {
        // HP-draining / life-risk conditions first.
        ConditionKind::Withdrawal => 9,
        ConditionKind::Depression => 8,
        ConditionKind::PanicDisorder => 7,
        ConditionKind::Ptsd => 6,
        ConditionKind::AnxietyDisorder => 5,
        ConditionKind::Addiction => 4,
        ConditionKind::AcuteStressReaction => 3,
        ConditionKind::Insomnia => 2,
    }
}

/// Conditions whose untreated symptomatic presence is an *urgent* (vs routine)
/// psych emergency.
fn is_high_risk(kind: ConditionKind) -> bool {
    matches!(kind, ConditionKind::Withdrawal | ConditionKind::Depression)
}

/// The most urgent symptomatic condition on `mh`, if any.
fn top_symptomatic(mh: &ActorMentalHealth) -> Option<ConditionKind> {
    mh.active
        .iter()
        .filter(|c| c.stage.is_symptomatic())
        .map(|c| c.kind)
        .max_by_key(|&k| psych_triage_rank(k))
}

/// Classify an ally's psych emergency for the Medic's `TreatPsych` weighting.
pub fn psych_emergency_level(mh: &ActorMentalHealth, tick: u64) -> PsychEmergencyLevel {
    if mh.is_panic_frozen(tick) {
        return PsychEmergencyLevel::Critical;
    }
    match top_symptomatic(mh) {
        None => PsychEmergencyLevel::None,
        Some(kind) if is_high_risk(kind) => PsychEmergencyLevel::Urgent,
        Some(_) => PsychEmergencyLevel::Routine,
    }
}

/// Resolve the Medic's `TreatPsych` action for one ally. Deterministic and
/// side-effect free — the engine dispatches the returned action.
pub fn psych_triage(
    mh: &ActorMentalHealth,
    target: u64,
    tick: u64,
    tick_rate_hz: u32,
    registry: &ConditionRegistry,
) -> PsychTreatAction {
    // 1. A frozen actor is incapacitated — calm them first.
    if let Some(frozen) = mh.active.iter().find(|c| c.is_panic_frozen(tick)) {
        return PsychTreatAction::Calm {
            target,
            condition: frozen.kind,
        };
    }
    // 2. Otherwise address the most urgent symptomatic condition.
    let Some(kind) = top_symptomatic(mh) else {
        return PsychTreatAction::None;
    };
    let Some(condition) = mh.find(kind) else {
        return PsychTreatAction::None;
    };
    let Some(spec) = registry.get(kind) else {
        return PsychTreatAction::None;
    };
    // Medication indicated but not yet started → administer it.
    if let Some(med) = spec.medication {
        if !condition.treatment.medication_started() {
            return PsychTreatAction::AdministerMedication {
                target,
                condition: kind,
                medication: med,
            };
        }
        // Med running but not yet at onset, and therapy still pending →
        // monitor; therapy complete + med active → monitor (awaiting onset).
        if condition.treatment.therapy_sessions_completed < spec.therapy_sessions_required {
            return PsychTreatAction::EscortToTherapy { target, condition: kind };
        }
        let _ = tick_rate_hz;
        return PsychTreatAction::Monitor { target, condition: kind };
    }
    // No medication indicated (rest/therapy only).
    if condition.treatment.therapy_sessions_completed < spec.therapy_sessions_required {
        return PsychTreatAction::EscortToTherapy { target, condition: kind };
    }
    PsychTreatAction::Monitor { target, condition: kind }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_state() {
        let mut s = MedicDoctrineState::new(7);
        let mut t = s.tree();
        t.step = DecisionStep::Treat;
        t.current_target = Some(42);
        t.last_treatment_applied = Some(TreatmentKind::TourniquetV1);
        t.last_monitor_tick = 120;
        s.write_back(&t);
        let t2 = s.tree();
        assert_eq!(t2.step, DecisionStep::Treat);
        assert_eq!(t2.current_target, Some(42));
        assert_eq!(
            t2.last_treatment_applied,
            Some(TreatmentKind::TourniquetV1)
        );
        assert_eq!(t2.last_monitor_tick, 120);
    }

    fn ally(actor_id: u64, ttd: f32, arterial: bool) -> PatientSnapshot {
        PatientSnapshot {
            actor_id,
            compound_ttd_seconds: ttd,
            wound_severity_sum: 1.0,
            mission_critical: false,
            cardiac_arrest: false,
            hypoxia: false,
            wounds: vec![WoundPriority {
                arterial_bleed: arterial,
                bleed_ml_per_s: if arterial { 12.0 } else { 1.0 },
                severity: 0.6,
                is_fracture: false,
                shrapnel_embedded: false,
                burn3rd: false,
                laceration: !arterial,
            }],
        }
    }

    /// resolver produces a concrete MedicAction + MedicAssessment.
    #[test]
    fn resolve_next_action_produces_apply_for_arterial_bleed() {
        let mut s = MedicDoctrineState::new(1);
        let patients = vec![ally(42, 10.0, true)];
        // Step 1: not within reach yet — MoveTo.
        let (action, _) = s.resolve_next_action(0, &patients, |_| false, |_| 100);
        assert!(matches!(action, MedicAction::MoveTo { .. }));
        // Step 2: within reach — Apply Tourniquet.
        let (action, assessment) =
            s.resolve_next_action(10, &patients, |_| true, |_| 100);
        match action {
            MedicAction::Apply { kind, .. } => {
                assert_eq!(kind, TreatmentKind::TourniquetV1);
            }
            other => panic!("expected Apply, got {other:?}"),
        }
        assert_eq!(assessment.target_actor_id, 42);
        assert_eq!(
            assessment.highest_priority_treatment,
            Some(TreatmentKind::TourniquetV1)
        );
    }

    #[test]
    fn resolve_next_action_idle_with_no_patients() {
        let mut s = MedicDoctrineState::new(1);
        let (action, _) = s.resolve_next_action(0, &[], |_| false, |_| 100);
        assert_eq!(action, MedicAction::Idle);
    }

    // ---- M16C psych-triage ----
    use cf_mental_health::{OriginId, TriggerReason};

    #[test]
    fn psych_triage_calms_a_panic_frozen_actor() {
        let reg = ConditionRegistry::default_registry();
        let mut mh = ActorMentalHealth::with_origin(OriginId::Human);
        mh.trigger(42, ConditionKind::PanicDisorder, TriggerReason::PanicThreshold, 0);
        // Freeze the actor until tick 300.
        mh.find_mut(ConditionKind::PanicDisorder).unwrap().panic_frozen_until_tick = 300;
        assert_eq!(psych_emergency_level(&mh, 100), PsychEmergencyLevel::Critical);
        assert_eq!(
            psych_triage(&mh, 42, 100, 60, &reg),
            PsychTreatAction::Calm { target: 42, condition: ConditionKind::PanicDisorder }
        );
    }

    #[test]
    fn psych_triage_administers_medication_for_untreated_ptsd() {
        let reg = ConditionRegistry::default_registry();
        let mut mh = ActorMentalHealth::with_origin(OriginId::Human);
        mh.trigger(42, ConditionKind::Ptsd, TriggerReason::WitnessDeaths, 0);
        assert_eq!(psych_emergency_level(&mh, 10), PsychEmergencyLevel::Routine);
        assert_eq!(
            psych_triage(&mh, 42, 10, 60, &reg),
            PsychTreatAction::AdministerMedication {
                target: 42,
                condition: ConditionKind::Ptsd,
                medication: PsychMedClass::Ssri,
            }
        );
    }

    #[test]
    fn psych_triage_escorts_to_therapy_once_medication_started() {
        let reg = ConditionRegistry::default_registry();
        let mut mh = ActorMentalHealth::with_origin(OriginId::Human);
        mh.trigger(42, ConditionKind::Ptsd, TriggerReason::WitnessDeaths, 0);
        mh.start_medication(42, ConditionKind::Ptsd, PsychMedClass::Ssri, 0);
        assert_eq!(
            psych_triage(&mh, 42, 10, 60, &reg),
            PsychTreatAction::EscortToTherapy { target: 42, condition: ConditionKind::Ptsd }
        );
    }

    #[test]
    fn psych_triage_prioritizes_withdrawal_as_urgent() {
        let reg = ConditionRegistry::default_registry();
        let mut mh = ActorMentalHealth::with_origin(OriginId::Human);
        // Insomnia (low rank) + Withdrawal (top rank) both symptomatic.
        mh.trigger(42, ConditionKind::Insomnia, TriggerReason::SleepDeprivation, 0);
        mh.trigger(42, ConditionKind::Withdrawal, TriggerReason::DrugAbsence, 0);
        assert_eq!(psych_emergency_level(&mh, 10), PsychEmergencyLevel::Urgent);
        match psych_triage(&mh, 42, 10, 60, &reg) {
            PsychTreatAction::AdministerMedication { condition, medication, .. } => {
                assert_eq!(condition, ConditionKind::Withdrawal);
                assert_eq!(medication, PsychMedClass::WithdrawalAssist);
            }
            other => panic!("expected withdrawal medication, got {other:?}"),
        }
    }

    #[test]
    fn psych_triage_none_when_healthy() {
        let reg = ConditionRegistry::default_registry();
        let mh = ActorMentalHealth::with_origin(OriginId::Human);
        assert_eq!(psych_emergency_level(&mh, 0), PsychEmergencyLevel::None);
        assert_eq!(psych_triage(&mh, 42, 0, 60, &reg), PsychTreatAction::None);
    }
}
