//! F2 — Pathfinding overlay. Forward-compat M22; M8 ships the data shape.

use serde::{Deserialize, Serialize};

/// Per-actor path render data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PathfindingOverlayEntry {
    /// Actor whose path is rendered.
    pub actor_id: u64,
    /// Current path waypoints (in world units).
    pub current_path: Vec<(f32, f32)>,
    /// Alternate path candidates (one inner Vec per alternate).
    pub alternates: Vec<Vec<(f32, f32)>>,
    /// Cost label for the chosen path.
    pub current_cost: f32,
}

/// Aggregated overlay payload.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PathfindingOverlayData {
    /// Per-actor entries.
    pub entries: Vec<PathfindingOverlayEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_default_is_empty() {
        let d = PathfindingOverlayData::default();
        assert!(d.entries.is_empty());
    }

    #[test]
    fn entry_carries_path_and_alternates() {
        let e = PathfindingOverlayEntry {
            actor_id: 1,
            current_path: vec![(0.0, 0.0), (10.0, 0.0)],
            alternates: vec![vec![(0.0, 0.0), (5.0, 5.0)]],
            current_cost: 12.5,
        };
        assert_eq!(e.current_path.len(), 2);
        assert_eq!(e.alternates.len(), 1);
    }
}
