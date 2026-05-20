//! **M14H** § Surgery 5-phase minigame UX.
//!
//! Per-phase progress display + per-step skill check toast.

use bevy::prelude::*;

pub use cf_treatment::{SurgeryEvent, SurgeryFailureReason, SurgeryPhase};

/// Bevy resource projection of the M14H 5-phase surgery UX.
///
/// cf-app's bridge writes this per frame from the engine's
/// per-surgery state machine (`SurgerySession`).
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct SurgeryPanelState {
    /// True if a surgery session is currently active.
    pub active: bool,
    /// Currently-running phase.
    pub current_phase: Option<SurgeryPhase>,
    /// Elapsed seconds in the current phase.
    pub phase_elapsed_seconds: f32,
    /// Total seconds for the current phase.
    pub phase_total_seconds: f32,
    /// Patient actor id (0 if none).
    pub patient_actor_id: u64,
    /// Surgeon actor id (0 if none).
    pub surgeon_actor_id: u64,
    /// Steps completed so far (Operate phase counter).
    pub steps_completed: u32,
    /// Steps that passed their skill check.
    pub steps_passed: u32,
    /// Total wounds being treated (drives Operate-phase repeat count).
    pub wounds_to_treat: u32,
    /// Last skill-check toast line for HUD overlay.
    pub last_skill_check: Option<String>,
}

impl SurgeryPanelState {
    /// Headline for the HUD overlay — `"SURGERY: <Phase> N/M  T+Xs"`.
    #[must_use]
    pub fn headline(&self) -> String {
        if !self.active {
            return String::new();
        }
        let phase = self
            .current_phase
            .map(|p| p.as_str().to_string())
            .unwrap_or_else(|| "?".to_string());
        format!(
            "SURGERY: {phase} {}/{}  T+{:.0}s",
            self.steps_completed, self.wounds_to_treat, self.phase_elapsed_seconds
        )
    }

    /// True when the current phase has elapsed past its total duration —
    /// callers should advance the phase.
    #[must_use]
    pub fn phase_complete(&self) -> bool {
        self.active && self.phase_elapsed_seconds >= self.phase_total_seconds
    }

    /// Apply a [`SurgeryEvent`] to update the panel state.
    pub fn apply(&mut self, ev: &SurgeryEvent) {
        match ev {
            SurgeryEvent::PhaseStarted {
                phase,
                duration_seconds,
                ..
            } => {
                self.active = true;
                self.current_phase = Some(*phase);
                self.phase_elapsed_seconds = 0.0;
                self.phase_total_seconds = *duration_seconds;
            }
            SurgeryEvent::PhaseCompleted { .. } => {
                // Wait for next PhaseStarted; intentional no-op here.
            }
            SurgeryEvent::SkillCheck {
                step_index,
                passed,
                ..
            } => {
                self.steps_completed = step_index.saturating_add(1);
                if *passed {
                    self.steps_passed = self.steps_passed.saturating_add(1);
                    self.last_skill_check = Some(format!("Step {} PASS", step_index + 1));
                } else {
                    self.last_skill_check = Some(format!("Step {} FAIL", step_index + 1));
                }
            }
            SurgeryEvent::Completed { .. } => {
                self.active = false;
                self.current_phase = Some(SurgeryPhase::Completed);
            }
            SurgeryEvent::Failed { .. } => {
                self.active = false;
                self.current_phase = Some(SurgeryPhase::Failed);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headline_inactive_is_empty() {
        let s = SurgeryPanelState::default();
        assert_eq!(s.headline(), "");
    }

    #[test]
    fn apply_phase_started_activates_panel() {
        let mut s = SurgeryPanelState::default();
        s.apply(&SurgeryEvent::PhaseStarted {
            actor_id: 1,
            phase: SurgeryPhase::Open,
            tick: 0,
            duration_seconds: 10.0,
        });
        assert!(s.active);
        assert_eq!(s.current_phase, Some(SurgeryPhase::Open));
        assert_eq!(s.phase_total_seconds, 10.0);
    }

    #[test]
    fn skill_check_updates_steps_counter() {
        let mut s = SurgeryPanelState::default();
        s.wounds_to_treat = 3;
        s.apply(&SurgeryEvent::SkillCheck {
            actor_id: 1,
            tick: 0,
            step_index: 0,
            passed: true,
            roll_x1000: 100,
            threshold_x1000: 900,
        });
        assert_eq!(s.steps_completed, 1);
        assert_eq!(s.steps_passed, 1);
        assert_eq!(s.last_skill_check.as_deref(), Some("Step 1 PASS"));
    }

    #[test]
    fn completed_deactivates_panel() {
        let mut s = SurgeryPanelState::default();
        s.active = true;
        s.apply(&SurgeryEvent::Completed {
            actor_id: 1,
            tick: 0,
            wounds_treated: 3,
            steps_passed: 3,
        });
        assert!(!s.active);
        assert_eq!(s.current_phase, Some(SurgeryPhase::Completed));
    }
}
