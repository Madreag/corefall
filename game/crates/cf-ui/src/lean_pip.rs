//! M8 — Lean angle pip HUD widget (-45° to +45° near reticle).

use bevy::prelude::*;

/// Lean min/max in degrees per spec § Lean angle indicator.
pub const LEAN_MIN_DEGREES: f32 = -45.0;
/// Maximum lean angle in degrees.
pub const LEAN_MAX_DEGREES: f32 = 45.0;

/// Lean pip widget Bevy resource.
#[derive(Resource, Debug, Clone, Default)]
pub struct LeanPipState {
    /// Current lean angle in degrees, clamped to `[-45, 45]`.
    pub angle_degrees: f32,
}

impl LeanPipState {
    /// Set the lean angle (clamped).
    pub fn set(&mut self, degrees: f32) {
        self.angle_degrees = degrees.clamp(LEAN_MIN_DEGREES, LEAN_MAX_DEGREES);
    }

    /// Player-facing label.
    pub fn label(&self) -> String {
        if self.angle_degrees.abs() < 0.5 {
            "Lean: Center".to_string()
        } else if self.angle_degrees < 0.0 {
            format!("Lean: {:.0}° L", self.angle_degrees.abs())
        } else {
            format!("Lean: {:.0}° R", self.angle_degrees)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_clamps() {
        let mut s = LeanPipState::default();
        s.set(100.0);
        assert!((s.angle_degrees - LEAN_MAX_DEGREES).abs() < f32::EPSILON);
        s.set(-100.0);
        assert!((s.angle_degrees - LEAN_MIN_DEGREES).abs() < f32::EPSILON);
    }

    #[test]
    fn label_centers_near_zero() {
        let s = LeanPipState { angle_degrees: 0.1 };
        assert_eq!(s.label(), "Lean: Center");
    }

    #[test]
    fn label_indicates_direction() {
        let mut s = LeanPipState::default();
        s.set(30.0);
        assert!(s.label().contains("R"));
        s.set(-30.0);
        assert!(s.label().contains("L"));
    }
}
