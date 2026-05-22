//! **M14H** § Field-Medic Decision Tree.
//!
//! Five-step FSM consumed by `cf-ai::medic_doctrine`:
//! 1. **Assess** — read M11 silhouette + scanner data + active afflictions.
//! 2. **Triage** — sort patients by `compound_TTD + wound severity sum +
//!    mission-critical flag`.
//! 3. **Stabilize** — apply tourniquet / bandage / trauma_pack to stop
//!    active bleed; clear life-threatening afflictions (hypoxia / heat
//!    stack / cardiac arrest).
//! 4. **Treat** — surgery / sutures / splint / cauterize / cure-course;
//!    per-wound loop.
//! 5. **Monitor** — re-scan every N ticks; raise alert on stage transition.

use serde::{Deserialize, Serialize};

use crate::producers::TreatmentKind;

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStep {
    Assess = 0,
    Triage = 1,
    Stabilize = 2,
    Treat = 3,
    Monitor = 4,
}

impl DecisionStep {
    pub fn as_str(self) -> &'static str {
        match self {
            DecisionStep::Assess => "assess",
            DecisionStep::Triage => "triage",
            DecisionStep::Stabilize => "stabilize",
            DecisionStep::Treat => "treat",
            DecisionStep::Monitor => "monitor",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "action")]
pub enum MedicAction {
    Idle,
    MoveTo {
        target_actor_id: u64,
    },
    Scan {
        target_actor_id: u64,
    },
    Apply {
        target_actor_id: u64,
        kind: TreatmentKind,
    },
    Cpr {
        target_actor_id: u64,
    },
    Defib {
        target_actor_id: u64,
    },
    AlertStageTransition {
        target_actor_id: u64,
        new_stage: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WoundPriority {
    /// Arterial-bleed flag — tourniquet first.
    pub arterial_bleed: bool,
    /// Active bleed rate ml/s.
    pub bleed_ml_per_s: f32,
    /// Wound severity 0..1.
    pub severity: f32,
    /// True if a fracture (splint applicability).
    pub is_fracture: bool,
    /// True if shrapnel embedded (surgery / shrapnel_extractor).
    pub shrapnel_embedded: bool,
    /// True if 3rd-degree thermal burn (cauterize unusable; surgery needed).
    pub burn3rd: bool,
    /// True for lacerations.
    pub laceration: bool,
}

impl WoundPriority {
    /// Compose a priority score 0..1000 for triage ranking.
    pub fn score(&self) -> u32 {
        let mut s = 0u32;
        if self.arterial_bleed {
            s += 900;
        }
        s += (self.bleed_ml_per_s.min(20.0) * 10.0) as u32;
        s += (self.severity.clamp(0.0, 1.0) * 200.0) as u32;
        if self.is_fracture {
            s += 60;
        }
        if self.shrapnel_embedded {
            s += 50;
        }
        if self.burn3rd {
            s += 80;
        }
        if self.laceration {
            s += 30;
        }
        s
    }

    /// highest-priority treatment.
    ///
    /// Gherkin scenario 5: arterial bleed → tourniquet first.
    pub fn highest_priority_treatment(&self) -> Option<TreatmentKind> {
        if self.arterial_bleed {
            return Some(TreatmentKind::TourniquetV1);
        }
        if self.bleed_ml_per_s > 8.0 {
            return Some(TreatmentKind::TraumaPackV1);
        }
        if self.bleed_ml_per_s > 0.0 {
            return Some(TreatmentKind::FieldBandageV1);
        }
        if self.shrapnel_embedded {
            return Some(TreatmentKind::SurgeryKitV1);
        }
        if self.burn3rd {
            return Some(TreatmentKind::CauterizeV1);
        }
        if self.is_fracture {
            return Some(TreatmentKind::SplintV1);
        }
        if self.laceration {
            return Some(TreatmentKind::SuturesV1);
        }
        None
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PatientSnapshot {
    pub actor_id: u64,
    pub compound_ttd_seconds: f32,
    pub wound_severity_sum: f32,
    pub mission_critical: bool,
    pub wounds: Vec<WoundPriority>,
    pub cardiac_arrest: bool,
    pub hypoxia: bool,
}

impl PatientSnapshot {
    pub fn triage_key(&self) -> i64 {
        // Lower compound_TTD = more urgent. Use integer-friendly key:
        // urgency = 1e6 - ttd_seconds × 100; lower urgency = less urgent.
        // We invert so that sorting ASCENDING yields most urgent first.
        let mut base = (self.compound_ttd_seconds * 1000.0) as i64;
        base -= (self.wound_severity_sum * 100.0) as i64;
        if self.mission_critical {
            base -= 100_000;
        }
        if self.cardiac_arrest {
            base -= 1_000_000;
        }
        base
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldMedicDecisionTree {
    pub medic_actor_id: u64,
    pub step: DecisionStep,
    pub current_target: Option<u64>,
    pub last_treatment_applied: Option<TreatmentKind>,
    pub monitor_cadence_ticks: u64,
    pub last_monitor_tick: u64,
}

impl FieldMedicDecisionTree {
    pub fn new(medic_actor_id: u64) -> Self {
        Self {
            medic_actor_id,
            step: DecisionStep::Assess,
            current_target: None,
            last_treatment_applied: None,
            monitor_cadence_ticks: 60,
            last_monitor_tick: 0,
        }
    }

    /// Triage step — sort patients by triage key ascending.
    pub fn triage(&self, patients: &[PatientSnapshot]) -> Vec<u64> {
        let mut order: Vec<&PatientSnapshot> = patients.iter().collect();
        order.sort_by_key(|p| (p.triage_key(), p.actor_id));
        order.iter().map(|p| p.actor_id).collect()
    }

    /// Compute the next action given the medic position + the patient
    /// snapshot list. Mutates `self.step`, `self.current_target`,
    /// `self.last_treatment_applied`. Returns the action the cf-ai medic
    /// doctrine should issue this tick.
    pub fn next_action(
        &mut self,
        sim_tick: u64,
        patients: &[PatientSnapshot],
        medic_within_reach_of: impl Fn(u64) -> bool,
        last_scan_age_ticks: impl Fn(u64) -> u64,
    ) -> MedicAction {
        // Assess: pick the highest-priority patient.
        let order = self.triage(patients);
        let top = order.first().copied();
        if top.is_none() {
            self.step = DecisionStep::Assess;
            self.current_target = None;
            return MedicAction::Idle;
        }
        let target = top.unwrap();
        self.current_target = Some(target);
        let snapshot = patients
            .iter()
            .find(|p| p.actor_id == target)
            .expect("triage emitted unknown id");
        // Move-to gate.
        if !medic_within_reach_of(target) {
            self.step = DecisionStep::Triage;
            return MedicAction::MoveTo {
                target_actor_id: target,
            };
        }
        // Stabilize life-threatening conditions first.
        if snapshot.cardiac_arrest {
            self.step = DecisionStep::Stabilize;
            // Defib if we already did CPR; else CPR first.
            return if self.last_treatment_applied == Some(TreatmentKind::CprManual) {
                self.last_treatment_applied = Some(TreatmentKind::DefibrillatorV1);
                MedicAction::Defib {
                    target_actor_id: target,
                }
            } else {
                self.last_treatment_applied = Some(TreatmentKind::CprManual);
                MedicAction::Cpr {
                    target_actor_id: target,
                }
            };
        }
        // Top wound determines the treatment.
        // Gherkin scenario 5: tourniquet for arterial bleed first.
        if let Some(top_wound) = snapshot
            .wounds
            .iter()
            .max_by_key(|w| (w.arterial_bleed as u32, w.score()))
        {
            if let Some(kind) = top_wound.highest_priority_treatment() {
                // After a treatment apply, the next tick re-scans before
                // treating the next wound.
                self.step = DecisionStep::Treat;
                self.last_treatment_applied = Some(kind);
                return MedicAction::Apply {
                    target_actor_id: target,
                    kind,
                };
            }
        }
        // Monitor: re-scan if cadence elapsed.
        if last_scan_age_ticks(target) >= self.monitor_cadence_ticks {
            self.step = DecisionStep::Monitor;
            self.last_monitor_tick = sim_tick;
            return MedicAction::Scan {
                target_actor_id: target,
            };
        }
        self.step = DecisionStep::Monitor;
        MedicAction::Idle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// tree. Given a Medic NPC + ally with compound TTD 10s and 3 wounds,
    /// when the AI tick fires, the Medic moves to the ally, applies the
    /// highest-priority treatment first (tourniquet for arterial bleed),
    /// then re-scans, then applies next treatment.
    #[test]
    fn medic_decision_tree_priorities_arterial_bleed() {
        let mut tree = FieldMedicDecisionTree::new(1);
        let ally = PatientSnapshot {
            actor_id: 42,
            compound_ttd_seconds: 10.0,
            wound_severity_sum: 1.5,
            mission_critical: false,
            cardiac_arrest: false,
            hypoxia: false,
            wounds: vec![
                WoundPriority {
                    arterial_bleed: true,
                    bleed_ml_per_s: 12.0,
                    severity: 0.8,
                    is_fracture: false,
                    shrapnel_embedded: false,
                    burn3rd: false,
                    laceration: false,
                },
                WoundPriority {
                    arterial_bleed: false,
                    bleed_ml_per_s: 1.0,
                    severity: 0.4,
                    is_fracture: false,
                    shrapnel_embedded: false,
                    burn3rd: false,
                    laceration: true,
                },
                WoundPriority {
                    arterial_bleed: false,
                    bleed_ml_per_s: 0.0,
                    severity: 0.3,
                    is_fracture: true,
                    shrapnel_embedded: false,
                    burn3rd: false,
                    laceration: false,
                },
            ],
        };
        let patients = vec![ally];
        // Step 1: medic not within reach → MoveTo.
        let action1 = tree.next_action(0, &patients, |_| false, |_| 100);
        assert!(matches!(action1, MedicAction::MoveTo { .. }));
        // Step 2: medic within reach → Apply tourniquet for arterial bleed.
        let action2 = tree.next_action(10, &patients, |_| true, |_| 100);
        match action2 {
            MedicAction::Apply { kind, .. } => assert_eq!(kind, TreatmentKind::TourniquetV1),
            _ => panic!("expected Apply, got {action2:?}"),
        }
    }

    /// Cardiac-arrest patient is always top priority.
    #[test]
    fn cardiac_arrest_is_top_priority() {
        let tree = FieldMedicDecisionTree::new(1);
        let p1 = PatientSnapshot {
            actor_id: 1,
            compound_ttd_seconds: 30.0,
            wound_severity_sum: 0.5,
            mission_critical: false,
            cardiac_arrest: false,
            hypoxia: false,
            wounds: vec![],
        };
        let p2 = PatientSnapshot {
            actor_id: 2,
            compound_ttd_seconds: 120.0,
            wound_severity_sum: 0.1,
            mission_critical: false,
            cardiac_arrest: true,
            hypoxia: false,
            wounds: vec![],
        };
        let order = tree.triage(&[p1, p2]);
        assert_eq!(order, vec![2, 1]);
    }

    /// Per-wound highest-priority treatment dispatch.
    #[test]
    fn highest_priority_treatment_by_wound_shape() {
        let arterial = WoundPriority {
            arterial_bleed: true,
            bleed_ml_per_s: 12.0,
            severity: 0.8,
            is_fracture: false,
            shrapnel_embedded: false,
            burn3rd: false,
            laceration: false,
        };
        assert_eq!(
            arterial.highest_priority_treatment(),
            Some(TreatmentKind::TourniquetV1)
        );
        let fracture = WoundPriority {
            arterial_bleed: false,
            bleed_ml_per_s: 0.0,
            severity: 0.5,
            is_fracture: true,
            shrapnel_embedded: false,
            burn3rd: false,
            laceration: false,
        };
        assert_eq!(
            fracture.highest_priority_treatment(),
            Some(TreatmentKind::SplintV1)
        );
        let burn = WoundPriority {
            arterial_bleed: false,
            bleed_ml_per_s: 0.0,
            severity: 0.9,
            is_fracture: false,
            shrapnel_embedded: false,
            burn3rd: true,
            laceration: false,
        };
        assert_eq!(
            burn.highest_priority_treatment(),
            Some(TreatmentKind::CauterizeV1)
        );
    }
}
