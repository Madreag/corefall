//! M7-A: cover-seeking BT sub-plan + event-emit helpers.
//!
//! When the Utility scorer picks `HoldCover` / `RetreatToCover` / `DigCover`,
//! or Layer 1 triggers an emergency cover move, the engine emits
//! `ai.cover_seeking_started` (one event per move). The actual movement
//! intent flows through the existing actor-step pipeline.

use serde::{Deserialize, Serialize};

use crate::archetype::Archetype;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverSeekingReason {
    /// Player / enemy fired at me.
    Fired,
    /// Emergency reactive layer (projectile incoming).
    EmergencyDodge,
    /// Low HP retreat to cover.
    LowHp,
    /// Squadmate flanking — I suppress / hold cover.
    SquadFlanking,
}

impl CoverSeekingReason {
    pub fn as_str(self) -> &'static str {
        match self {
            CoverSeekingReason::Fired => "fired",
            CoverSeekingReason::EmergencyDodge => "emergency_dodge",
            CoverSeekingReason::LowHp => "low_hp",
            CoverSeekingReason::SquadFlanking => "squad_flanking",
        }
    }
}

/// `ai.cover_seeking_started` with the exact shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoverSeekingEvent {
    pub actor_id: u64,
    pub archetype: Archetype,
    pub reason: CoverSeekingReason,
    pub target_position: [f32; 2],
    pub distance: f32,
}

/// the bot can move to within `max_distance`. Pure for unit-test seedability;
/// real engine integration uses the perception grid in `BotMemory`.
pub fn nearest_cover(self_pos: [f32; 2], candidates: &[[f32; 2]], max_distance: f32) -> Option<[f32; 2]> {
    let mut best: Option<([f32; 2], f32)> = None;
    for c in candidates {
        let dx = c[0] - self_pos[0];
        let dy = c[1] - self_pos[1];
        let d2 = dx * dx + dy * dy;
        if d2 > max_distance * max_distance {
            continue;
        }
        let d = d2.sqrt();
        if best.map(|(_, bd)| d < bd).unwrap_or(true) {
            best = Some((*c, d));
        }
    }
    best.map(|(p, _)| p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_cover_picks_closest() {
        let self_pos = [0.0, 0.0];
        let candidates = [[10.0, 0.0], [-3.0, 4.0], [50.0, 0.0]];
        let pick = nearest_cover(self_pos, &candidates, 100.0).expect("cover available");
        assert!((pick[0] - -3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn cover_beyond_range_skipped() {
        let pick = nearest_cover([0.0, 0.0], &[[100.0, 0.0]], 10.0);
        assert!(pick.is_none());
    }
}
