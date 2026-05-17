//! M8B § Rollback prediction model + resimulation.
//!
//! Per M8B spec § Rollback prediction model:
//!
//! - **Ring buffer (6 frames)**: per-tick input snapshots + per-tick
//!   world-state hash so the driver can detect first-divergent-frame.
//! - **Prediction (last-input-repeat)**: M8B explicitly forbids
//!   extrapolated aim / decayed move because of determinism risk.
//! - **Resimulation driver**: replays the deterministic sim core over
//!   the rollback window; budget = 6 frames @ p99 ≤ 8 ms total.
//!
//! This module ships the FOUNDATION + budget arithmetic; the live
//! sim-core entry point is wired by cf-control at M9+.

pub mod prediction;
pub mod resimulate;
pub mod ring_buffer;

pub use prediction::{InputPredictor, PredictionMode};
pub use resimulate::{ResimulateBudget, ResimulateOutcome, ResimulateRequest};
pub use ring_buffer::{InputFrame, RollbackRingBuffer, ROLLBACK_WINDOW_FRAMES};

use serde::{Deserialize, Serialize};

/// **M8A § locked**: rollback budget in frames (M8B inherits).
pub const ROLLBACK_BUDGET_FRAMES: u32 = ROLLBACK_WINDOW_FRAMES as u32;

/// **M8A § locked**: total resimulation budget at p99 (milliseconds)
/// — M8B preserves this from M8A.
pub const ROLLBACK_RESIM_BUDGET_MS: f32 = 8.0;

/// **M8B § rollback**: descriptor for a rollback resimulation request.
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
