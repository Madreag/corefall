//! M8A § cf-net::snapshot — delta-encoded world snapshots.
//!
//! Per M8A spec § Snapshot field contract: snapshots adopt the
//! Powder-Toy-derived field shape; M8A reuses `cf_sim_core::snapshot`
//! for the determinism-anchor envelope. cf-net's snapshot module wraps
//! that with per-cadence delta encoding.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotCadence {
    pub keyframe_every_ticks: u64,
    pub delta_every_ticks: u64,
}

impl Default for SnapshotCadence {
    fn default() -> Self {
        Self {
            keyframe_every_ticks: crate::protocol::SNAPSHOT_CADENCE_TICKS,
            delta_every_ticks: 1,
        }
    }
}

/// Re-export the canonical snapshot envelope from cf-sim-core.
pub use cf_sim_core::snapshot::{Snapshot, SnapshotDelta};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_cadence_keyframe_every_64() {
        let c = SnapshotCadence::default();
        assert_eq!(c.keyframe_every_ticks, 64);
        assert_eq!(c.delta_every_ticks, 1);
    }
}
