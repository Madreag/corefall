//! M6: lean angle tracking for "lean around corner".
//!
//! Spec: lean range -45° to +45° (M6 § "Lean left / Lean right"). The lean
//! angle is sticky (held while the player keeps the input down); release sets
//! the angle back toward 0. The engine threads the angle into projectile
//! origin offset + aim cone offset.

use serde::{Deserialize, Serialize};

/// M6 § max lean degrees. Stored as degrees in cfctl I/O; radians internally.
pub const LEAN_MAX_DEGREES: f32 = 45.0;
/// M6 § per-tick lean approach rate (degrees / tick at 60 Hz).
pub const LEAN_APPROACH_DEG_PER_S: f32 = 180.0;

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeanDirection {
    None = 0,
    Left = 1,
    Right = 2,
}

impl Default for LeanDirection {
    fn default() -> Self {
        LeanDirection::None
    }
}

impl LeanDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            LeanDirection::None => "none",
            LeanDirection::Left => "left",
            LeanDirection::Right => "right",
        }
    }

    pub fn sign(self) -> f32 {
        match self {
            LeanDirection::None => 0.0,
            LeanDirection::Left => -1.0,
            LeanDirection::Right => 1.0,
        }
    }
}

/// Per-actor lean state. Threaded into [`crate::ActorState::lean_state`] from M6.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LeanState {
    /// Direction the player is requesting (sticky while held).
    pub direction: LeanDirection,
    /// Current lean angle (degrees; signed; -LEAN_MAX_DEGREES..LEAN_MAX_DEGREES).
    pub angle_degrees: f32,
}

impl Default for LeanState {
    fn default() -> Self {
        Self {
            direction: LeanDirection::None,
            angle_degrees: 0.0,
        }
    }
}

impl LeanState {
    /// Step the lean angle toward its target this tick.
    pub fn step(&mut self, tick_rate_hz: u32) {
        let rate = tick_rate_hz.max(1) as f32;
        let step = LEAN_APPROACH_DEG_PER_S / rate;
        let target = self.direction.sign() * LEAN_MAX_DEGREES;
        if !self.angle_degrees.is_finite() {
            self.angle_degrees = 0.0;
        }
        if self.angle_degrees < target {
            self.angle_degrees = (self.angle_degrees + step).min(target);
        } else if self.angle_degrees > target {
            self.angle_degrees = (self.angle_degrees - step).max(target);
        }
    }

    pub fn is_leaning(self) -> bool {
        self.direction != LeanDirection::None || self.angle_degrees.abs() > 0.1
    }

    /// Convert the current angle to radians for sim use.
    pub fn angle_radians(self) -> f32 {
        self.angle_degrees.to_radians()
    }

    pub fn reset(&mut self) {
        self.direction = LeanDirection::None;
        self.angle_degrees = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lean_left_negative() {
        let mut l = LeanState {
            direction: LeanDirection::Left,
            ..LeanState::default()
        };
        for _ in 0..120 {
            l.step(60);
        }
        assert!(l.angle_degrees <= -LEAN_MAX_DEGREES + 0.5);
    }

    #[test]
    fn lean_right_positive() {
        let mut l = LeanState {
            direction: LeanDirection::Right,
            ..LeanState::default()
        };
        for _ in 0..120 {
            l.step(60);
        }
        assert!(l.angle_degrees >= LEAN_MAX_DEGREES - 0.5);
    }

    #[test]
    fn release_returns_to_zero() {
        let mut l = LeanState {
            direction: LeanDirection::None,
            angle_degrees: 30.0,
        };
        for _ in 0..120 {
            l.step(60);
        }
        assert!(l.angle_degrees.abs() < 0.5);
    }

    #[test]
    fn nan_resets() {
        let mut l = LeanState {
            direction: LeanDirection::None,
            angle_degrees: f32::NAN,
        };
        l.step(60);
        assert_eq!(l.angle_degrees, 0.0);
    }
}
