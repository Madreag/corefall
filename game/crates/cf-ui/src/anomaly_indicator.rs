//! M16 § Anomaly indicator HUD widget.
//!
//! Per spec § "Anomaly detector reveals positions":
//! - When the player carries an anomaly detector (or a Compass artifact),
//!   nearby anomalies surface on the minimap.
//! - On-screen radial indicator shows the closest anomaly's bearing.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// One on-minimap anomaly marker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnomalyIndicatorMarker {
    pub anomaly_id: u64,
    pub kind: String,
    pub world_position: [f32; 2],
    /// Bearing in radians from the player. North = 0, clockwise.
    pub bearing_radians: f32,
    /// Distance in meters from the player.
    pub distance_m: f32,
    /// True when the anomaly required a detector to surface (bloodsucker
    /// lair, psy storm).
    pub detector_required: bool,
}

#[derive(Debug, Clone, Default, Resource, Serialize, Deserialize)]
pub struct AnomalyIndicatorState {
    pub markers: Vec<AnomalyIndicatorMarker>,
    /// True when the player carries an anomaly detector item (or an
    /// artifact with `reveals_anomalies`).
    pub detector_active: bool,
}

impl AnomalyIndicatorState {
    pub fn refresh(
        &mut self,
        player_pos: [f32; 2],
        detector_active: bool,
        anomalies: &[(u64, String, [f32; 2], bool)],
    ) {
        self.detector_active = detector_active;
        let mut out = Vec::new();
        for (id, kind, pos, requires_detector) in anomalies {
            if *requires_detector && !detector_active {
                continue;
            }
            let dx = pos[0] - player_pos[0];
            let dy = pos[1] - player_pos[1];
            let dist = (dx * dx + dy * dy).sqrt();
            let bearing = dy.atan2(dx);
            out.push(AnomalyIndicatorMarker {
                anomaly_id: *id,
                kind: kind.clone(),
                world_position: *pos,
                bearing_radians: bearing,
                distance_m: dist,
                detector_required: *requires_detector,
            });
        }
        out.sort_by(|a, b| a.distance_m.partial_cmp(&b.distance_m).unwrap_or(std::cmp::Ordering::Equal));
        self.markers = out;
    }

    /// Closest anomaly bearing (radians). None when the list is empty.
    #[must_use]
    pub fn closest_bearing(&self) -> Option<f32> {
        self.markers.first().map(|m| m.bearing_radians)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detector_off_hides_detector_required_anomalies() {
        let mut state = AnomalyIndicatorState::default();
        let anomalies = vec![
            (1u64, "electric_anomaly".to_string(), [5.0, 0.0], false),
            (2u64, "bloodsucker_lair".to_string(), [3.0, 0.0], true),
        ];
        state.refresh([0.0, 0.0], false, &anomalies);
        assert_eq!(state.markers.len(), 1, "detector-required anomalies must hide w/o detector");
        assert_eq!(state.markers[0].anomaly_id, 1);
    }

    #[test]
    fn detector_on_reveals_all_anomalies() {
        let mut state = AnomalyIndicatorState::default();
        let anomalies = vec![
            (1u64, "electric_anomaly".to_string(), [5.0, 0.0], false),
            (2u64, "bloodsucker_lair".to_string(), [3.0, 0.0], true),
        ];
        state.refresh([0.0, 0.0], true, &anomalies);
        assert_eq!(state.markers.len(), 2);
    }

    #[test]
    fn markers_sorted_by_distance() {
        let mut state = AnomalyIndicatorState::default();
        let anomalies = vec![
            (1u64, "electric_anomaly".to_string(), [10.0, 0.0], false),
            (2u64, "chemical_anomaly".to_string(), [3.0, 0.0], false),
            (3u64, "time_anomaly".to_string(), [7.0, 0.0], false),
        ];
        state.refresh([0.0, 0.0], true, &anomalies);
        assert_eq!(state.markers[0].anomaly_id, 2);
        assert_eq!(state.markers[1].anomaly_id, 3);
        assert_eq!(state.markers[2].anomaly_id, 1);
    }
}
