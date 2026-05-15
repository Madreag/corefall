//! cf-photo — M8 photo mode (basic surface; M33+ codex extends).
//!
//! Spec § Photo mode: F12 enters photo mode (sim pauses), free camera
//! (pan/zoom/rotate/FOV), 4 launch filters (sepia / B&W / color grading
//! / cyberpunk neon), export to PNG. The crate keeps deterministic,
//! headless filter math; cf-app supplies the source RGB buffer + the
//! file path.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod export;
pub mod filters;

pub use export::{export_png, ExportError};
pub use filters::{apply_filter, PhotoFilter};

use serde::{Deserialize, Serialize};

/// Free camera state inside photo mode. Independent of cf-camera's
/// gameplay-time state; the player drives this with mouse/keyboard while
/// the sim is paused.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhotoCamera {
    /// World-space camera position.
    pub position: (f32, f32),
    /// Render zoom factor (1.0 = base; > 1.0 = zoomed in).
    pub zoom: f32,
    /// Rotation angle in radians.
    pub rotation_rad: f32,
    /// FOV in degrees.
    pub fov_degrees: f32,
}

impl Default for PhotoCamera {
    fn default() -> Self {
        Self {
            position: (0.0, 0.0),
            zoom: 1.0,
            rotation_rad: 0.0,
            fov_degrees: 60.0,
        }
    }
}

/// Photo mode state machine. cf-control owns one instance per session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PhotoModeState {
    /// Whether photo mode is currently entered.
    pub active: bool,
    /// Active filter.
    pub filter: PhotoFilter,
    /// Free-camera state.
    pub camera: PhotoCamera,
    /// Total shots taken in this session.
    pub shot_count: u32,
}

impl PhotoModeState {
    /// Enter photo mode. Returns false when already active.
    pub fn enter(&mut self) -> bool {
        if self.active {
            return false;
        }
        self.active = true;
        true
    }

    /// Exit photo mode. Returns false when already inactive.
    pub fn exit(&mut self) -> bool {
        if !self.active {
            return false;
        }
        self.active = false;
        true
    }

    /// Cycle through the 4 filters + None. Returns the new filter.
    pub fn cycle_filter(&mut self) -> PhotoFilter {
        self.filter = self.filter.next();
        self.filter
    }

    /// Increment the shot counter.
    pub fn record_shot(&mut self) -> u32 {
        self.shot_count = self.shot_count.saturating_add(1);
        self.shot_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enter_exit_round_trips() {
        let mut s = PhotoModeState::default();
        assert!(s.enter());
        assert!(s.active);
        assert!(!s.enter(), "second enter is a no-op");
        assert!(s.exit());
        assert!(!s.active);
        assert!(!s.exit(), "second exit is a no-op");
    }

    #[test]
    fn cycle_filter_walks_5_states() {
        let mut s = PhotoModeState::default();
        let order: Vec<PhotoFilter> = (0..5).map(|_| s.cycle_filter()).collect();
        assert_eq!(order.len(), 5);
        let mut sorted = order.clone();
        sorted.sort_by_key(|f| f.as_str());
        sorted.dedup();
        assert_eq!(sorted.len(), 5, "cycle covers every variant exactly once");
    }

    #[test]
    fn record_shot_increments() {
        let mut s = PhotoModeState::default();
        assert_eq!(s.record_shot(), 1);
        assert_eq!(s.record_shot(), 2);
    }
}
