//! M7-A: patrol waypoint following with idle pauses.
//!
//! Idle bots with a configured patrol route walk between waypoints, pausing
//! for 5-10 seconds at each. The pause duration uses the seeded RNG to
//! preserve replay determinism.

use serde::{Deserialize, Serialize};

use crate::constants::{seconds_to_ticks_for, PATROL_IDLE_MAX_SECONDS, PATROL_IDLE_MIN_SECONDS};

/// **M7-A**: one patrol route (list of waypoints + cursor).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatrolRoute {
    pub waypoints: Vec<[f32; 2]>,
    pub cursor: usize,
    pub idle_remaining_ticks: u32,
}

impl PatrolRoute {
    pub fn new(waypoints: Vec<[f32; 2]>) -> Self {
        Self {
            waypoints,
            cursor: 0,
            idle_remaining_ticks: 0,
        }
    }

    pub fn current(&self) -> Option<[f32; 2]> {
        self.waypoints.get(self.cursor).copied()
    }

    pub fn advance(&mut self, tick_rate_hz: u32, rng_roll: f32) {
        if self.waypoints.is_empty() {
            return;
        }
        self.cursor = (self.cursor + 1) % self.waypoints.len();
        let span = PATROL_IDLE_MAX_SECONDS - PATROL_IDLE_MIN_SECONDS;
        let seconds = PATROL_IDLE_MIN_SECONDS + rng_roll.clamp(0.0, 1.0) * span;
        self.idle_remaining_ticks = seconds_to_ticks_for(seconds, tick_rate_hz).max(1);
    }

    pub fn tick_idle(&mut self) -> bool {
        if self.idle_remaining_ticks == 0 {
            return false;
        }
        self.idle_remaining_ticks -= 1;
        true
    }
}

/// **M7-A**: emitted when a bot reaches a patrol waypoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatrolWaypointReachedEvent {
    pub actor_id: u64,
    pub waypoint_index: usize,
    pub position: [f32; 2],
    pub idle_seconds: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_wraps_and_sets_idle() {
        let mut p = PatrolRoute::new(vec![[0.0, 0.0], [10.0, 0.0]]);
        p.advance(60, 0.0);
        assert_eq!(p.cursor, 1);
        // 5s minimum at 60 Hz → 300 ticks.
        assert_eq!(p.idle_remaining_ticks, 300);
    }

    #[test]
    fn idle_decrements_to_zero() {
        let mut p = PatrolRoute::new(vec![[0.0, 0.0]]);
        p.idle_remaining_ticks = 3;
        assert!(p.tick_idle());
        assert!(p.tick_idle());
        assert!(p.tick_idle());
        assert!(!p.tick_idle());
    }
}
