//! M8 — Mini-map / radar HUD widget (256×256 top-right).
//!
//! Per spec § UX widgets: player + enemies + waypoints + zoom adjustable.
//! cf-app's renderer reads this state each frame.

use bevy::prelude::*;

/// Spec-mandated default mini-map width/height in display pixels.
pub const MINIMAP_SIZE_PX: u32 = 256;

/// One marker on the mini-map.
#[derive(Debug, Clone, PartialEq)]
pub enum MinimapMarker {
    /// Local player.
    Player {
        /// World position in unit-space.
        position: (f32, f32),
        /// Facing in radians.
        facing_rad: f32,
    },
    /// Enemy actor.
    Enemy {
        /// World position.
        position: (f32, f32),
        /// Display label (short).
        label: String,
    },
    /// Persistent waypoint.
    Waypoint {
        /// World position.
        position: (f32, f32),
        /// Display label.
        label: String,
    },
    /// Squadmate (friendly bot).
    Squadmate {
        /// World position.
        position: (f32, f32),
        /// Display label.
        label: String,
    },
}

/// Mini-map widget Bevy resource.
#[derive(Resource, Debug, Clone)]
pub struct MinimapState {
    /// Display side length in px.
    pub size_px: u32,
    /// Zoom factor (1.0 = base; higher = closer).
    pub zoom: f32,
    /// All markers to render this frame.
    pub markers: Vec<MinimapMarker>,
    /// Whether the mini-map is enabled (Settings.mini_map_enabled).
    pub enabled: bool,
}

impl Default for MinimapState {
    fn default() -> Self {
        Self {
            size_px: MINIMAP_SIZE_PX,
            zoom: 1.0,
            markers: Vec::new(),
            enabled: true,
        }
    }
}

impl MinimapState {
    /// Update the zoom (clamped to `[0.25, 4.0]`).
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(0.25, 4.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_uses_spec_size() {
        let s = MinimapState::default();
        assert_eq!(s.size_px, MINIMAP_SIZE_PX);
    }

    #[test]
    fn set_zoom_clamps() {
        let mut s = MinimapState::default();
        s.set_zoom(100.0);
        assert_eq!(s.zoom, 4.0);
        s.set_zoom(0.0);
        assert_eq!(s.zoom, 0.25);
    }

    #[test]
    fn marker_kinds_round_trip() {
        let mut s = MinimapState::default();
        s.markers.push(MinimapMarker::Player {
            position: (1.0, 2.0),
            facing_rad: 0.5,
        });
        s.markers.push(MinimapMarker::Enemy {
            position: (3.0, 4.0),
            label: "Sniper".into(),
        });
        assert_eq!(s.markers.len(), 2);
    }
}
