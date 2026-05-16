//! M8 — Compass HUD widget (top-center, 360° with cardinals + waypoint
//! bearings).

use bevy::prelude::*;

/// Cardinal direction labels.
pub const CARDINALS: [(&str, f32); 8] = [
    ("N", 0.0),
    ("NE", 45.0),
    ("E", 90.0),
    ("SE", 135.0),
    ("S", 180.0),
    ("SW", 225.0),
    ("W", 270.0),
    ("NW", 315.0),
];

/// One waypoint bearing rendered along the compass strip.
#[derive(Debug, Clone, PartialEq)]
pub struct CompassBearing {
    /// Bearing in degrees (0..360, 0 = north, clockwise).
    pub bearing_degrees: f32,
    /// Player-facing label.
    pub label: String,
}

/// Compass widget Bevy resource.
#[derive(Resource, Debug, Clone)]
pub struct CompassState {
    /// Player-facing bearing (degrees).
    pub heading_degrees: f32,
    /// Waypoint bearings to render.
    pub bearings: Vec<CompassBearing>,
    /// Whether the compass is enabled (Settings.compass_enabled).
    pub enabled: bool,
}

impl Default for CompassState {
    fn default() -> Self {
        Self {
            heading_degrees: 0.0,
            bearings: Vec::new(),
            enabled: true,
        }
    }
}

impl CompassState {
    /// Set the player heading (clamped to `[0, 360)`).
    pub fn set_heading(&mut self, deg: f32) {
        let mut h = deg.rem_euclid(360.0);
        if h < 0.0 {
            h += 360.0;
        }
        self.heading_degrees = h;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cardinals_count_is_8() {
        assert_eq!(CARDINALS.len(), 8);
    }

    #[test]
    fn set_heading_wraps() {
        let mut s = CompassState::default();
        s.set_heading(370.0);
        assert!((s.heading_degrees - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn set_heading_negative_wraps() {
        let mut s = CompassState::default();
        s.set_heading(-90.0);
        assert!((s.heading_degrees - 270.0).abs() < f32::EPSILON);
    }
}
