//! M7-A: squad communication with 0.5s relay delay.
//!
//! When one bot detects the player, the threat propagates to the rest of
//! the squad with a fixed 0.5-second delay (spec § Squad communication).
//! The relay event fires when the latency expires.

use serde::{Deserialize, Serialize};

use crate::constants::{seconds_to_ticks_for, SQUAD_COMM_RELAY_DELAY_SECONDS};

/// **M7-A**: pending squad-comm relay. The originator queues one entry per
/// detection event; the engine ticks the latch each tick and fires the
/// `ai.squad_comm_relayed` event when the delay elapses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SquadCommPending {
    pub originator_actor_id: u64,
    pub target_actor_id: u64,
    pub target_position: [f32; 2],
    pub trigger_tick: u64,
    pub relay_tick: u64,
}

impl SquadCommPending {
    pub fn new(originator: u64, target: u64, target_pos: [f32; 2], trigger_tick: u64, tick_rate_hz: u32) -> Self {
        let delay = seconds_to_ticks_for(SQUAD_COMM_RELAY_DELAY_SECONDS, tick_rate_hz).max(1);
        Self {
            originator_actor_id: originator,
            target_actor_id: target,
            target_position: target_pos,
            trigger_tick,
            relay_tick: trigger_tick.saturating_add(delay as u64),
        }
    }

    pub fn is_ready(&self, current_tick: u64) -> bool {
        current_tick >= self.relay_tick
    }
}

/// **M7-A**: squad-comm relay event payload (consumed by cf-control).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SquadCommRelayedEvent {
    pub originator_actor_id: u64,
    pub receiver_actor_ids: Vec<u64>,
    pub target_actor_id: u64,
    pub target_position: [f32; 2],
    pub delay_ticks: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_delay_is_30_ticks_at_60hz() {
        let p = SquadCommPending::new(1, 2, [0.0, 0.0], 100, 60);
        assert_eq!(p.relay_tick, 130);
    }

    #[test]
    fn is_ready_only_after_relay_tick() {
        let p = SquadCommPending::new(1, 2, [0.0, 0.0], 100, 60);
        assert!(!p.is_ready(125));
        assert!(p.is_ready(130));
    }
}
