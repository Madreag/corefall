//! F7 — Squad command overlay. Waypoint pins + doctrine labels per squad.

use serde::{Deserialize, Serialize};

/// One squad-member waypoint pin.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WaypointPin {
    /// Owning squad member actor id.
    pub member_id: u64,
    /// Pin world position.
    pub position: (f32, f32),
    /// Doctrine label (e.g. `defend_point`, `flank_left`).
    pub doctrine_label: String,
}

/// Aggregated overlay payload.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SquadOverlayData {
    /// All waypoint pins to draw this frame.
    pub pins: Vec<WaypointPin>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_round_trips() {
        let p = WaypointPin {
            member_id: 9,
            position: (44.0, 12.0),
            doctrine_label: "defend_point".into(),
        };
        let s = serde_json::to_string(&p).unwrap();
        let back: WaypointPin = serde_json::from_str(&s).unwrap();
        assert_eq!(p, back);
    }
}
