//! M8 — Stamina bar HUD widget (under stance pip).
//!
//! Per spec § UX widgets: cyan above 50%, yellow under 50%, red under 10%.

use bevy::prelude::*;

/// Color band the stamina bar should display.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum StaminaColor {
    /// Cyan — above 50%.
    Cyan,
    /// Yellow — between 10% and 50%.
    Yellow,
    /// Red — below 10%.
    Red,
}

/// Spec thresholds (fractions in `[0, 1]`).
pub const STAMINA_HIGH_THRESHOLD: f32 = 0.5;
/// Stamina threshold below which the bar turns red.
pub const STAMINA_CRITICAL_THRESHOLD: f32 = 0.1;

/// Stamina bar widget Bevy resource.
#[derive(Resource, Debug, Clone)]
pub struct StaminaBarState {
    /// Current stamina fraction in `[0, 1]`.
    pub fraction: f32,
}

impl Default for StaminaBarState {
    fn default() -> Self {
        Self { fraction: 1.0 }
    }
}

impl StaminaBarState {
    /// Set the current stamina fraction (clamped to `[0, 1]`).
    pub fn set(&mut self, fraction: f32) {
        self.fraction = fraction.clamp(0.0, 1.0);
    }

    /// Color band per spec thresholds.
    pub fn color(&self) -> StaminaColor {
        if self.fraction < STAMINA_CRITICAL_THRESHOLD {
            StaminaColor::Red
        } else if self.fraction < STAMINA_HIGH_THRESHOLD {
            StaminaColor::Yellow
        } else {
            StaminaColor::Cyan
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_is_cyan() {
        let s = StaminaBarState { fraction: 0.8 };
        assert_eq!(s.color(), StaminaColor::Cyan);
    }

    #[test]
    fn medium_is_yellow() {
        let s = StaminaBarState { fraction: 0.3 };
        assert_eq!(s.color(), StaminaColor::Yellow);
    }

    #[test]
    fn low_is_red() {
        let s = StaminaBarState { fraction: 0.05 };
        assert_eq!(s.color(), StaminaColor::Red);
    }

    #[test]
    fn set_clamps() {
        let mut s = StaminaBarState::default();
        s.set(2.0);
        assert_eq!(s.fraction, 1.0);
        s.set(-1.0);
        assert_eq!(s.fraction, 0.0);
    }
}
