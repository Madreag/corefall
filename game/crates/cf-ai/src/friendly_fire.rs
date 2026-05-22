//! M7-A: friendly fire avoidance.
//!
//! Bots reposition before firing if a friendly is in the line-of-fire OR
//! within a grenade blast radius. The avoidance event fires once when the
//! reposition decision is taken.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FriendlyFireKind {
    LineOfFire,
    BlastRadius,
}

impl FriendlyFireKind {
    pub fn as_str(self) -> &'static str {
        match self {
            FriendlyFireKind::LineOfFire => "line_of_fire",
            FriendlyFireKind::BlastRadius => "blast_radius",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FriendlyFireAvoidanceEvent {
    pub actor_id: u64,
    pub friendly_actor_id: u64,
    pub kind: FriendlyFireKind,
}

/// the friendly's position, decide whether the friendly sits in the line
/// of fire. Uses a 1-unit lateral tolerance.
pub fn is_friendly_in_line_of_fire(
    self_pos: [f32; 2],
    aim: [f32; 2],
    friendly_pos: [f32; 2],
    max_range: f32,
    lateral_tolerance: f32,
) -> bool {
    let dx = friendly_pos[0] - self_pos[0];
    let dy = friendly_pos[1] - self_pos[1];
    let aim_len = (aim[0] * aim[0] + aim[1] * aim[1]).sqrt();
    if aim_len < f32::EPSILON {
        return false;
    }
    let along = (dx * aim[0] + dy * aim[1]) / aim_len;
    if along < 0.0 || along > max_range {
        return false;
    }
    let perp = (dx * aim[1] - dy * aim[0]) / aim_len;
    perp.abs() <= lateral_tolerance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directly_ahead_is_in_line() {
        assert!(is_friendly_in_line_of_fire(
            [0.0, 0.0],
            [1.0, 0.0],
            [10.0, 0.0],
            100.0,
            1.0
        ));
    }

    #[test]
    fn behind_is_not_in_line() {
        assert!(!is_friendly_in_line_of_fire(
            [0.0, 0.0],
            [1.0, 0.0],
            [-10.0, 0.0],
            100.0,
            1.0
        ));
    }

    #[test]
    fn far_lateral_is_not_in_line() {
        assert!(!is_friendly_in_line_of_fire(
            [0.0, 0.0],
            [1.0, 0.0],
            [10.0, 5.0],
            100.0,
            1.0
        ));
    }
}
