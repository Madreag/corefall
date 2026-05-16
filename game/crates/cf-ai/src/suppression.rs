//! M7-A: suppression sub-plan + event helpers.
//!
//! `ai.suppression_started` fires when a Rifleman / Assault chooses
//! SuppressFire AND a squadmate is flanking the same target. The mechanic
//! is "low-accuracy high-rate-of-fire while a teammate maneuvers."

use serde::{Deserialize, Serialize};

/// **M7-A**: suppression event payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuppressionEvent {
    pub actor_id: u64,
    pub target_actor_id: u64,
    pub flanker_actor_id: Option<u64>,
    /// Suppression duration in ticks the bot intends to maintain. M7-A
    /// uses a fixed 4-second budget at the configured tick rate; M7-B's
    /// chatter cooldown ladder may tune this.
    pub duration_ticks: u32,
}

impl SuppressionEvent {
    pub fn build(actor_id: u64, target_actor_id: u64, flanker_actor_id: Option<u64>, tick_rate_hz: u32) -> Self {
        let duration_ticks = (tick_rate_hz * 4).max(60);
        Self {
            actor_id,
            target_actor_id,
            flanker_actor_id,
            duration_ticks,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_at_60hz_gives_4_second_budget() {
        let e = SuppressionEvent::build(1, 2, Some(3), 60);
        assert_eq!(e.duration_ticks, 240);
    }
}
