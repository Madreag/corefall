//! M7-A: auto-triage first-class contract (Medic).
//!
//! When a squadmate enters DYING, the closest Medic archetype within
//! `MEDIC_AUTO_TRIAGE_REACH_SECONDS` (= 6 s) begins moving toward the
//! target; on arrival applies a medkit within `MEDIC_AUTO_TRIAGE_APPLY_SECONDS`
//! (= 8 s) of the DYING transition. Spec § Auto-triage Gherkin.

use serde::{Deserialize, Serialize};

use crate::constants::{seconds_to_ticks_for, MEDIC_AUTO_TRIAGE_APPLY_SECONDS, MEDIC_AUTO_TRIAGE_REACH_SECONDS};

/// stack picks `TriageDownedAlly`; the engine instantiates an
/// `AutoTriageMission` and ticks it each frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoTriageMission {
    pub medic_actor_id: u64,
    pub target_actor_id: u64,
    pub dying_transition_tick: u64,
    pub reach_deadline_tick: u64,
    pub apply_deadline_tick: u64,
    pub state: AutoTriageState,
    pub initiated_tick: Option<u64>,
    pub applied_tick: Option<u64>,
}

impl AutoTriageMission {
    pub fn new(medic: u64, target: u64, dying_tick: u64, tick_rate_hz: u32) -> Self {
        let reach = seconds_to_ticks_for(MEDIC_AUTO_TRIAGE_REACH_SECONDS, tick_rate_hz).max(1) as u64;
        let apply = seconds_to_ticks_for(MEDIC_AUTO_TRIAGE_APPLY_SECONDS, tick_rate_hz).max(1) as u64;
        Self {
            medic_actor_id: medic,
            target_actor_id: target,
            dying_transition_tick: dying_tick,
            reach_deadline_tick: dying_tick.saturating_add(reach),
            apply_deadline_tick: dying_tick.saturating_add(apply),
            state: AutoTriageState::Initiated,
            initiated_tick: Some(dying_tick),
            applied_tick: None,
        }
    }

    /// Returns true if the medic still has time to reach the target.
    pub fn within_reach_window(&self, current_tick: u64) -> bool {
        current_tick <= self.reach_deadline_tick
    }

    /// Returns true if the medic still has time to apply stabilization.
    pub fn within_apply_window(&self, current_tick: u64) -> bool {
        current_tick <= self.apply_deadline_tick
    }

    /// Transition to `Reached` once the medic arrives.
    pub fn mark_reached(&mut self, tick: u64) {
        if matches!(self.state, AutoTriageState::Initiated) {
            self.state = AutoTriageState::Reached;
            let _ = tick;
        }
    }

    /// Transition to `Applied` when stabilization lands.
    pub fn mark_applied(&mut self, tick: u64) {
        if matches!(self.state, AutoTriageState::Initiated | AutoTriageState::Reached) {
            self.state = AutoTriageState::Applied;
            self.applied_tick = Some(tick);
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self.state, AutoTriageState::Applied | AutoTriageState::Failed)
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoTriageState {
    Initiated,
    Reached,
    Applied,
    Failed,
}

impl AutoTriageState {
    pub fn as_str(self) -> &'static str {
        match self {
            AutoTriageState::Initiated => "initiated",
            AutoTriageState::Reached => "reached",
            AutoTriageState::Applied => "applied",
            AutoTriageState::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoTriageInitiatedEvent {
    pub medic_actor_id: u64,
    pub target_actor_id: u64,
    pub dying_tick: u64,
    pub reach_deadline_tick: u64,
    pub apply_deadline_tick: u64,
    pub reach_seconds: f32,
    pub apply_seconds: f32,
}

impl AutoTriageInitiatedEvent {
    pub fn from_mission(m: &AutoTriageMission) -> Self {
        Self {
            medic_actor_id: m.medic_actor_id,
            target_actor_id: m.target_actor_id,
            dying_tick: m.dying_transition_tick,
            reach_deadline_tick: m.reach_deadline_tick,
            apply_deadline_tick: m.apply_deadline_tick,
            reach_seconds: MEDIC_AUTO_TRIAGE_REACH_SECONDS,
            apply_seconds: MEDIC_AUTO_TRIAGE_APPLY_SECONDS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoTriageAppliedEvent {
    pub medic_actor_id: u64,
    pub target_actor_id: u64,
    pub dying_tick: u64,
    pub applied_tick: u64,
    pub elapsed_seconds: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadlines_are_six_and_eight_seconds_at_60hz() {
        let m = AutoTriageMission::new(1, 2, 1000, 60);
        assert_eq!(m.reach_deadline_tick, 1000 + 6 * 60);
        assert_eq!(m.apply_deadline_tick, 1000 + 8 * 60);
    }

    #[test]
    fn mark_applied_terminates() {
        let mut m = AutoTriageMission::new(1, 2, 0, 60);
        m.mark_applied(120);
        assert!(m.is_terminal());
        assert_eq!(m.state, AutoTriageState::Applied);
    }
}
