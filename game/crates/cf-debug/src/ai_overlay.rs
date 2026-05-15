//! F1 — AI state overlay. Per-AI sight cone + hearing radius + memory
//! grid + state label render data. The overlay produces typed structs the
//! renderer consumes; cf-debug stays render-agnostic.

use serde::{Deserialize, Serialize};

/// Per-AI render data for the AI state overlay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiOverlayEntry {
    /// AI actor id.
    pub actor_id: u64,
    /// World-space position used as the cone apex.
    pub position: (f32, f32),
    /// Sight-cone half-angle in radians.
    pub sight_half_angle_rad: f32,
    /// Sight-cone range (world units).
    pub sight_range: f32,
    /// Hearing radius (world units).
    pub hearing_radius: f32,
    /// Aim direction in radians (cone bisector).
    pub facing_rad: f32,
    /// State label rendered above the actor.
    pub state_label: String,
    /// Memory grid cells the AI currently retains (each entry is a tile
    /// coord). Empty for actors with no memory state.
    pub memory_cells: Vec<(i32, i32)>,
}

/// Aggregated overlay payload.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AiOverlayData {
    /// Per-AI overlay entries.
    pub entries: Vec<AiOverlayEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_round_trips_serde() {
        let e = AiOverlayEntry {
            actor_id: 7,
            position: (10.0, 5.0),
            sight_half_angle_rad: 0.6,
            sight_range: 200.0,
            hearing_radius: 100.0,
            facing_rad: 1.5,
            state_label: "ALERT".into(),
            memory_cells: vec![(1, 2), (3, 4)],
        };
        let json = serde_json::to_string(&e).unwrap();
        let back: AiOverlayEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(e, back);
    }
}
