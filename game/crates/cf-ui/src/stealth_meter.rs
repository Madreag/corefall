//! M8 — Stealth meter HUD widget (eye icon near reticle).
//!
//! Per spec § UX widgets: eye icon with detection %; "Spotted" caption
//! surfaces when detection > 50%.

use bevy::prelude::*;

/// Spec-mandated detection threshold (0..1) above which "Spotted" caption
/// surfaces.
pub const SPOTTED_THRESHOLD: f32 = 0.5;

/// Stealth meter widget Bevy resource.
#[derive(Resource, Debug, Clone)]
pub struct StealthMeterState {
    /// Detection fraction in `[0, 1]`.
    pub detection: f32,
    /// Whether the meter is rendered (Settings.perception_overlay_enabled).
    pub enabled: bool,
}

impl Default for StealthMeterState {
    fn default() -> Self {
        Self {
            detection: 0.0,
            enabled: true,
        }
    }
}

impl StealthMeterState {
    /// Set the current detection fraction.
    pub fn set(&mut self, fraction: f32) {
        self.detection = fraction.clamp(0.0, 1.0);
    }

    /// Whether the "Spotted" caption should surface this frame.
    pub fn is_spotted(&self) -> bool {
        self.detection > SPOTTED_THRESHOLD
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_spotted_above_threshold() {
        let s = StealthMeterState {
            detection: 0.6,
            enabled: true,
        };
        assert!(s.is_spotted());
    }

    #[test]
    fn is_not_spotted_at_threshold() {
        let s = StealthMeterState {
            detection: 0.5,
            enabled: true,
        };
        assert!(!s.is_spotted());
    }

    #[test]
    fn set_clamps() {
        let mut s = StealthMeterState::default();
        s.set(2.0);
        assert_eq!(s.detection, 1.0);
    }
}
