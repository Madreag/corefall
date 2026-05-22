//! M7-A: auto-repair first-class contract (Engineer).
//!
//! When a chassis module enters DEGRADED/FAILED state, the closest Engineer
//! archetype within `ENGINEER_AUTO_REPAIR_REACH_SECONDS` (= 6 s) begins
//! moving toward the target; the first repair tick lands within
//! `ENGINEER_AUTO_REPAIR_FIRST_TICK_SECONDS` (= 8 s) of the transition.
//! Spec § Auto-repair Gherkin.

use serde::{Deserialize, Serialize};

use crate::constants::{
    seconds_to_ticks_for, ENGINEER_AUTO_REPAIR_FIRST_TICK_SECONDS, ENGINEER_AUTO_REPAIR_REACH_SECONDS,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoRepairMission {
    pub engineer_actor_id: u64,
    pub target_actor_id: u64,
    pub target_module_id: String,
    pub triggered_tick: u64,
    pub reach_deadline_tick: u64,
    pub first_tick_deadline_tick: u64,
    pub state: AutoRepairState,
    pub progressed_ticks: u32,
}

impl AutoRepairMission {
    pub fn new(engineer: u64, target: u64, module: impl Into<String>, trigger_tick: u64, tick_rate_hz: u32) -> Self {
        let reach = seconds_to_ticks_for(ENGINEER_AUTO_REPAIR_REACH_SECONDS, tick_rate_hz).max(1) as u64;
        let first = seconds_to_ticks_for(ENGINEER_AUTO_REPAIR_FIRST_TICK_SECONDS, tick_rate_hz).max(1) as u64;
        Self {
            engineer_actor_id: engineer,
            target_actor_id: target,
            target_module_id: module.into(),
            triggered_tick: trigger_tick,
            reach_deadline_tick: trigger_tick.saturating_add(reach),
            first_tick_deadline_tick: trigger_tick.saturating_add(first),
            state: AutoRepairState::Initiated,
            progressed_ticks: 0,
        }
    }

    pub fn within_reach_window(&self, current_tick: u64) -> bool {
        current_tick <= self.reach_deadline_tick
    }

    pub fn within_first_tick_window(&self, current_tick: u64) -> bool {
        current_tick <= self.first_tick_deadline_tick
    }

    pub fn mark_reached(&mut self) {
        if matches!(self.state, AutoRepairState::Initiated) {
            self.state = AutoRepairState::Reached;
        }
    }

    pub fn record_repair_tick(&mut self) {
        if matches!(self.state, AutoRepairState::Initiated | AutoRepairState::Reached) {
            self.state = AutoRepairState::Progressing;
        }
        self.progressed_ticks = self.progressed_ticks.saturating_add(1);
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self.state, AutoRepairState::Completed | AutoRepairState::Failed)
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoRepairState {
    Initiated,
    Reached,
    Progressing,
    Completed,
    Failed,
}

impl AutoRepairState {
    pub fn as_str(self) -> &'static str {
        match self {
            AutoRepairState::Initiated => "initiated",
            AutoRepairState::Reached => "reached",
            AutoRepairState::Progressing => "progressing",
            AutoRepairState::Completed => "completed",
            AutoRepairState::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoRepairInitiatedEvent {
    pub engineer_actor_id: u64,
    pub target_actor_id: u64,
    pub target_module_id: String,
    pub triggered_tick: u64,
    pub reach_deadline_tick: u64,
    pub first_tick_deadline_tick: u64,
    pub reach_seconds: f32,
    pub first_tick_seconds: f32,
}

impl AutoRepairInitiatedEvent {
    pub fn from_mission(m: &AutoRepairMission) -> Self {
        Self {
            engineer_actor_id: m.engineer_actor_id,
            target_actor_id: m.target_actor_id,
            target_module_id: m.target_module_id.clone(),
            triggered_tick: m.triggered_tick,
            reach_deadline_tick: m.reach_deadline_tick,
            first_tick_deadline_tick: m.first_tick_deadline_tick,
            reach_seconds: ENGINEER_AUTO_REPAIR_REACH_SECONDS,
            first_tick_seconds: ENGINEER_AUTO_REPAIR_FIRST_TICK_SECONDS,
        }
    }
}

/// or per per-second sample, engine's choice).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoRepairProgressedEvent {
    pub engineer_actor_id: u64,
    pub target_actor_id: u64,
    pub target_module_id: String,
    pub tick: u64,
    pub repair_amount: f32,
    pub total_progressed_ticks: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_are_six_and_eight_seconds_at_60hz() {
        let m = AutoRepairMission::new(1, 2, "leg_left", 1000, 60);
        assert_eq!(m.reach_deadline_tick, 1000 + 6 * 60);
        assert_eq!(m.first_tick_deadline_tick, 1000 + 8 * 60);
    }

    #[test]
    fn record_repair_tick_advances_state() {
        let mut m = AutoRepairMission::new(1, 2, "head", 0, 60);
        m.record_repair_tick();
        assert_eq!(m.state, AutoRepairState::Progressing);
        assert_eq!(m.progressed_ticks, 1);
    }
}
