//! M7-A: high-ground preference (Sniper / Spotter).
//!
//! Sniper + Spotter archetypes actively move to elevated terrain before
//! firing. `ai.high_ground_preference_applied` fires when the bot picks a
//! high-ground waypoint.

use serde::{Deserialize, Serialize};

/// **M7-A**: high-ground event payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HighGroundEvent {
    pub actor_id: u64,
    pub target_position: [f32; 2],
    pub elevation_gain: f32,
}

/// **M7-A**: pure helper — pick the highest elevation among candidates that
/// is reachable within `max_range`. Returns the candidate position + the
/// elevation gain over `self_pos.y`.
pub fn pick_high_ground(self_pos: [f32; 2], candidates: &[[f32; 2]], max_range: f32) -> Option<([f32; 2], f32)> {
    let mut best: Option<([f32; 2], f32)> = None;
    for c in candidates {
        let dx = c[0] - self_pos[0];
        let dy = c[1] - self_pos[1];
        let d2 = dx * dx + dy * dy;
        if d2 > max_range * max_range {
            continue;
        }
        let gain = c[1] - self_pos[1];
        if gain <= 0.0 {
            continue;
        }
        if best.map(|(_, bg)| gain > bg).unwrap_or(true) {
            best = Some((*c, gain));
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_highest_in_range() {
        let pick = pick_high_ground([0.0, 0.0], &[[5.0, 3.0], [4.0, 8.0], [200.0, 50.0]], 50.0);
        assert!(pick.is_some());
        let (pos, gain) = pick.unwrap();
        assert!((pos[1] - 8.0).abs() < f32::EPSILON);
        assert!((gain - 8.0).abs() < f32::EPSILON);
    }

    #[test]
    fn none_when_no_elevation() {
        let pick = pick_high_ground([0.0, 5.0], &[[3.0, 0.0]], 10.0);
        assert!(pick.is_none());
    }
}
