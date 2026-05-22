//! **M14H** § Surgery 5-phase minigame state machine.
//!
//! Phases (spec § Surgery minigame):
//! - **Open** (10s) — exposes the wound; bleed risk during.
//! - **Diagnose** (15s) — confirms internal damage extent.
//! - **Operate** (30–120s; per-wound) — per-step skill check.
//! - **Close** (20s) — suture + seal; conversion to scar.
//! - **Recover** (30 min in-game in hospital bed) — monitored healing.
//!
//! Failure modes: bleed-out during Open/Operate; infection from dirty tools;
//! iatrogenic Wound.
//!
//! Determinism: skill checks use a seeded `Xoshiro256StarStar` so identical
//! seed + identical patient + identical surgeon reproduce identical phase
//! outcomes (M14H Gherkin scenario 7).

use rand_core::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256StarStar;
use serde::{Deserialize, Serialize};

use crate::producers::{MEDIC_T1_SKILL_PASS_RATE_X1000, SURGEON_T1_SKILL_PASS_RATE_X1000};

pub const SURGERY_PHASE_OPEN_SECONDS: f32 = 10.0;
pub const SURGERY_PHASE_DIAGNOSE_SECONDS: f32 = 15.0;
pub const SURGERY_PHASE_OPERATE_SECONDS_PER_STEP: f32 = 30.0;
pub const SURGERY_PHASE_CLOSE_SECONDS: f32 = 20.0;
pub const SURGERY_PHASE_RECOVER_SECONDS: f32 = 30.0 * 60.0;

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SurgeryPhase {
    Open = 0,
    Diagnose = 1,
    Operate = 2,
    Close = 3,
    Recover = 4,
    Completed = 5,
    Failed = 6,
}

impl SurgeryPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            SurgeryPhase::Open => "Open",
            SurgeryPhase::Diagnose => "Diagnose",
            SurgeryPhase::Operate => "Operate",
            SurgeryPhase::Close => "Close",
            SurgeryPhase::Recover => "Recover",
            SurgeryPhase::Completed => "Completed",
            SurgeryPhase::Failed => "Failed",
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurgeryFailureReason {
    BleedOutOpen,
    BleedOutOperate,
    SkillCheckFailed,
    InfectionFromDirtyTool,
    IatrogenicWound,
    Cancelled,
}

impl SurgeryFailureReason {
    pub fn as_str(self) -> &'static str {
        match self {
            SurgeryFailureReason::BleedOutOpen => "bleed_out_open",
            SurgeryFailureReason::BleedOutOperate => "bleed_out_operate",
            SurgeryFailureReason::SkillCheckFailed => "skill_check_failed",
            SurgeryFailureReason::InfectionFromDirtyTool => "infection_from_dirty_tool",
            SurgeryFailureReason::IatrogenicWound => "iatrogenic_wound",
            SurgeryFailureReason::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "outcome")]
pub enum SurgeryOutcome {
    Completed {
        actor_id: u64,
        wounds_treated: u32,
        steps_passed: u32,
    },
    Failed {
        actor_id: u64,
        reason: SurgeryFailureReason,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "event")]
pub enum SurgeryEvent {
    PhaseStarted {
        actor_id: u64,
        phase: SurgeryPhase,
        tick: u64,
        duration_seconds: f32,
    },
    PhaseCompleted {
        actor_id: u64,
        phase: SurgeryPhase,
        tick: u64,
    },
    SkillCheck {
        actor_id: u64,
        tick: u64,
        step_index: u32,
        passed: bool,
        roll_x1000: u32,
        threshold_x1000: u32,
    },
    Completed {
        actor_id: u64,
        tick: u64,
        wounds_treated: u32,
        steps_passed: u32,
    },
    Failed {
        actor_id: u64,
        tick: u64,
        reason: SurgeryFailureReason,
    },
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct SurgeryStepResult {
    pub step_index: u32,
    pub passed: bool,
    pub roll_x1000: u32,
    pub threshold_x1000: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SurgerySession {
    pub actor_id: u64,
    pub phase: SurgeryPhase,
    pub wounds_to_treat: u32,
    pub steps_completed: u32,
    pub steps_passed: u32,
    pub phase_seconds_remaining: f32,
    /// True if the surgeon has surgeon_t1 (90% pass rate); false → medic_t1
    /// (70% pass rate).
    pub surgeon_t1: bool,
    /// Patient bleed-out rate during Open / Operate. > 0 enables risk roll.
    pub bleed_ml_per_s: f32,
    /// Cumulative bleed in ml. Crosses the threshold → BleedOut failure.
    pub cumulative_bleed_ml: f32,
    /// Bleed-out threshold in ml.
    pub bleed_out_threshold_ml: f32,
    /// True if tools are dirty (>0.5 dirt_pct).
    pub tools_dirty: bool,
    pub cancelled: bool,
    pub outcome: Option<SurgeryOutcome>,
    rng: Xoshiro256StarStar,
}

impl SurgerySession {
    pub fn new(actor_id: u64, wounds_to_treat: u32, surgeon_t1: bool, seed: u64) -> Self {
        let phase = if wounds_to_treat == 0 {
            SurgeryPhase::Completed
        } else {
            SurgeryPhase::Open
        };
        let phase_seconds = SURGERY_PHASE_OPEN_SECONDS;
        Self {
            actor_id,
            phase,
            wounds_to_treat: wounds_to_treat.max(1),
            steps_completed: 0,
            steps_passed: 0,
            phase_seconds_remaining: phase_seconds,
            surgeon_t1,
            bleed_ml_per_s: 0.0,
            cumulative_bleed_ml: 0.0,
            bleed_out_threshold_ml: 1500.0,
            tools_dirty: false,
            cancelled: false,
            outcome: None,
            rng: Xoshiro256StarStar::seed_from_u64(seed),
        }
    }

    /// True if the session ended (Completed or Failed).
    pub fn is_terminal(&self) -> bool {
        matches!(self.phase, SurgeryPhase::Completed | SurgeryPhase::Failed)
    }

    fn phase_duration(&self) -> f32 {
        match self.phase {
            SurgeryPhase::Open => SURGERY_PHASE_OPEN_SECONDS,
            SurgeryPhase::Diagnose => SURGERY_PHASE_DIAGNOSE_SECONDS,
            SurgeryPhase::Operate => SURGERY_PHASE_OPERATE_SECONDS_PER_STEP,
            SurgeryPhase::Close => SURGERY_PHASE_CLOSE_SECONDS,
            SurgeryPhase::Recover => SURGERY_PHASE_RECOVER_SECONDS,
            SurgeryPhase::Completed | SurgeryPhase::Failed => 0.0,
        }
    }

    fn advance_phase(&mut self) -> SurgeryPhase {
        let next = match self.phase {
            SurgeryPhase::Open => SurgeryPhase::Diagnose,
            SurgeryPhase::Diagnose => SurgeryPhase::Operate,
            SurgeryPhase::Operate => {
                // Stay in Operate until every wound has had its skill check.
                if self.steps_completed >= self.wounds_to_treat {
                    SurgeryPhase::Close
                } else {
                    SurgeryPhase::Operate
                }
            }
            SurgeryPhase::Close => SurgeryPhase::Recover,
            SurgeryPhase::Recover => SurgeryPhase::Completed,
            SurgeryPhase::Completed | SurgeryPhase::Failed => self.phase,
        };
        self.phase = next;
        self.phase_seconds_remaining = self.phase_duration();
        next
    }

    fn skill_threshold_x1000(&self) -> u32 {
        if self.surgeon_t1 {
            SURGEON_T1_SKILL_PASS_RATE_X1000
        } else {
            MEDIC_T1_SKILL_PASS_RATE_X1000
        }
    }

    fn roll_skill_check(&mut self) -> SurgeryStepResult {
        let roll = (self.rng.next_u64() % 1000) as u32;
        let threshold = self.skill_threshold_x1000();
        SurgeryStepResult {
            step_index: self.steps_completed,
            passed: roll < threshold,
            roll_x1000: roll,
            threshold_x1000: threshold,
        }
    }

    /// Advance time by `dt_seconds` and return events fired this tick.
    /// Emits PhaseStarted on first entry into a new phase, PhaseCompleted
    /// at end of phase, SkillCheck per Operate-step, Completed/Failed at
    /// terminal.
    pub fn tick(&mut self, dt_seconds: f32, sim_tick: u64) -> Vec<SurgeryEvent> {
        let mut events = Vec::new();
        if self.is_terminal() {
            return events;
        }
        if self.cancelled {
            self.phase = SurgeryPhase::Failed;
            self.outcome = Some(SurgeryOutcome::Failed {
                actor_id: self.actor_id,
                reason: SurgeryFailureReason::Cancelled,
            });
            events.push(SurgeryEvent::Failed {
                actor_id: self.actor_id,
                tick: sim_tick,
                reason: SurgeryFailureReason::Cancelled,
            });
            return events;
        }
        // On the very first tick of a session (and every phase entry) emit
        // PhaseStarted. We track this by checking whether the remaining
        // time equals the full duration AND no events have fired for this
        // phase yet.
        if (self.phase_seconds_remaining - self.phase_duration()).abs() < 1e-6 {
            events.push(SurgeryEvent::PhaseStarted {
                actor_id: self.actor_id,
                phase: self.phase,
                tick: sim_tick,
                duration_seconds: self.phase_duration(),
            });
        }
        // Bleed-out risk during Open / Operate.
        if matches!(self.phase, SurgeryPhase::Open | SurgeryPhase::Operate)
            && self.bleed_ml_per_s > 0.0
        {
            self.cumulative_bleed_ml += self.bleed_ml_per_s * dt_seconds;
            if self.cumulative_bleed_ml >= self.bleed_out_threshold_ml {
                let reason = match self.phase {
                    SurgeryPhase::Open => SurgeryFailureReason::BleedOutOpen,
                    _ => SurgeryFailureReason::BleedOutOperate,
                };
                self.phase = SurgeryPhase::Failed;
                self.outcome = Some(SurgeryOutcome::Failed {
                    actor_id: self.actor_id,
                    reason,
                });
                events.push(SurgeryEvent::Failed {
                    actor_id: self.actor_id,
                    tick: sim_tick,
                    reason,
                });
                return events;
            }
        }
        self.phase_seconds_remaining -= dt_seconds;
        if self.phase_seconds_remaining > 0.0 {
            return events;
        }
        // Phase end — emit PhaseCompleted and dispatch per-phase exit logic.
        let prev = self.phase;
        events.push(SurgeryEvent::PhaseCompleted {
            actor_id: self.actor_id,
            phase: prev,
            tick: sim_tick,
        });
        match prev {
            SurgeryPhase::Operate => {
                let step = self.roll_skill_check();
                self.steps_completed += 1;
                if step.passed {
                    self.steps_passed += 1;
                }
                events.push(SurgeryEvent::SkillCheck {
                    actor_id: self.actor_id,
                    tick: sim_tick,
                    step_index: step.step_index,
                    passed: step.passed,
                    roll_x1000: step.roll_x1000,
                    threshold_x1000: step.threshold_x1000,
                });
                if !step.passed && self.tools_dirty {
                    self.phase = SurgeryPhase::Failed;
                    self.outcome = Some(SurgeryOutcome::Failed {
                        actor_id: self.actor_id,
                        reason: SurgeryFailureReason::InfectionFromDirtyTool,
                    });
                    events.push(SurgeryEvent::Failed {
                        actor_id: self.actor_id,
                        tick: sim_tick,
                        reason: SurgeryFailureReason::InfectionFromDirtyTool,
                    });
                    return events;
                }
            }
            _ => {}
        }
        let next = self.advance_phase();
        if matches!(next, SurgeryPhase::Completed) {
            self.outcome = Some(SurgeryOutcome::Completed {
                actor_id: self.actor_id,
                wounds_treated: self.steps_completed,
                steps_passed: self.steps_passed,
            });
            events.push(SurgeryEvent::Completed {
                actor_id: self.actor_id,
                tick: sim_tick,
                wounds_treated: self.steps_completed,
                steps_passed: self.steps_passed,
            });
        }
        events
    }

    pub fn cancel(&mut self) {
        if !self.is_terminal() {
            self.cancelled = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Given 3 ShrapnelEmbedded wounds + surgery_table + surgeon, when the
    /// full 5-phase sequence completes then 3× treatment.applied (one per
    /// shrapnel removed) — modeled here as 3 Operate skill-check passes.
    #[test]
    fn surgery_5_phases_complete() {
        let mut s = SurgerySession::new(99, 3, true, 7);
        let mut all_events = Vec::new();
        // Run for plenty of sim time (30s open + 15s diag + 30s × 3 ops + 20s close + 1800s recover).
        let total_seconds = (SURGERY_PHASE_OPEN_SECONDS
            + SURGERY_PHASE_DIAGNOSE_SECONDS
            + SURGERY_PHASE_OPERATE_SECONDS_PER_STEP * 3.0
            + SURGERY_PHASE_CLOSE_SECONDS
            + SURGERY_PHASE_RECOVER_SECONDS) as u64
            + 5;
        for t in 0..=total_seconds {
            all_events.extend(s.tick(1.0, t));
            if s.is_terminal() {
                break;
            }
        }
        // Phase ordering: PhaseStarted(Open) first.
        let started_open = all_events
            .iter()
            .position(|e| matches!(e, SurgeryEvent::PhaseStarted { phase: SurgeryPhase::Open, .. }));
        assert!(started_open.is_some(), "must start with Open phase");
        // 3 skill checks during Operate.
        let skill_checks = all_events
            .iter()
            .filter(|e| matches!(e, SurgeryEvent::SkillCheck { .. }))
            .count();
        assert_eq!(skill_checks, 3, "expected 3 skill checks, got {skill_checks}");
        // Completed at end.
        assert!(matches!(
            all_events.last(),
            Some(SurgeryEvent::Completed { .. })
        ));
    }

    /// Determinism — same seed produces identical event stream.
    #[test]
    fn surgery_determinism_same_seed() {
        let mut s1 = SurgerySession::new(99, 3, true, 42);
        let mut s2 = SurgerySession::new(99, 3, true, 42);
        let mut a = Vec::new();
        let mut b = Vec::new();
        for t in 0..2200u64 {
            a.extend(s1.tick(1.0, t));
            b.extend(s2.tick(1.0, t));
            if s1.is_terminal() && s2.is_terminal() {
                break;
            }
        }
        assert_eq!(a, b);
    }

    /// Surgeon_t1 has higher pass-rate than medic_t1.
    #[test]
    fn surgeon_t1_pass_rate_higher() {
        let s_surg = SurgerySession::new(1, 100, true, 1);
        let s_med = SurgerySession::new(1, 100, false, 1);
        assert!(s_surg.skill_threshold_x1000() > s_med.skill_threshold_x1000());
    }

    /// Bleed-out kills the surgery during Open if too much bleed.
    #[test]
    fn surgery_bleeds_out_during_open() {
        let mut s = SurgerySession::new(1, 1, true, 0);
        s.bleed_ml_per_s = 200.0;
        s.bleed_out_threshold_ml = 100.0;
        let mut last = None;
        for t in 0..=20u64 {
            for e in s.tick(1.0, t) {
                last = Some(e);
            }
            if s.is_terminal() {
                break;
            }
        }
        assert!(matches!(
            last,
            Some(SurgeryEvent::Failed {
                reason: SurgeryFailureReason::BleedOutOpen,
                ..
            })
        ));
    }
}
