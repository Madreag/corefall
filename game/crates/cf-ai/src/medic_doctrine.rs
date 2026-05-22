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
}
