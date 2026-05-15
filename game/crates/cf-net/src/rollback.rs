//! M8A § cf-net::rollback — rollback netcode primitives.
//!
//! Per M8A spec: 6-frame rollback budget at p99 ≤ 8 ms total
//! resimulation. The deterministic sim core is reused (no separate code
//! path); on confirmed-input mismatch, roll back to first divergent
//! frame and resimulate forward.

use serde::{Deserialize, Serialize};

/// **M8A § locked**: rollback budget in frames.
pub const ROLLBACK_BUDGET_FRAMES: u32 = 6;

/// **M8A § locked**: total resimulation budget at p99 (milliseconds).
pub const ROLLBACK_RESIM_BUDGET_MS: f32 = 8.0;

/// **M8A § rollback**: descriptor for a rollback resimulation request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackRequest {
    pub from_tick: u64,
    pub to_tick: u64,
    pub reason: String,
}

impl RollbackRequest {
    pub fn span_frames(&self) -> u64 {
        self.to_tick.saturating_sub(self.from_tick)
    }

    /// **M8A § acceptance**: rollbacks larger than the locked budget
    /// trigger a full snapshot resync (Gaffer pattern).
    pub fn within_budget(&self) -> bool {
        self.span_frames() <= u64::from(ROLLBACK_BUDGET_FRAMES)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollback_under_budget_passes() {
        let r = RollbackRequest {
            from_tick: 100,
            to_tick: 105,
            reason: "input mismatch".into(),
        };
        assert!(r.within_budget());
    }

    #[test]
    fn rollback_over_budget_fails() {
        let r = RollbackRequest {
            from_tick: 100,
            to_tick: 110,
            reason: "huge mismatch".into(),
        };
        assert!(!r.within_budget());
    }
}
