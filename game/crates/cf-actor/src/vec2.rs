use serde::{Deserialize, Serialize};

/// 2D vector used by sim systems. We do NOT depend on `glam` here so this crate stays
/// dependency-light. The Bevy bridge converts to `Vec2`.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    /// Returns a unit vector. If the input is the zero vector OR contains a non-finite
    /// component (NaN / Inf), returns `Vec2::new(1.0, 0.0)` so consumers (e.g. weapon
    /// muzzle origin, projectile velocity, recoil) never produce NaNs. NaN comparisons
    /// always return false, so a plain `len < 1e-6` guard is NOT sufficient — we must
    /// explicitly check `is_finite()` on every component.
    pub fn normalize_or_x(self) -> Vec2 {
        if !self.x.is_finite() || !self.y.is_finite() {
            return Vec2::new(1.0, 0.0);
        }
        let len = self.length();
        if !len.is_finite() || len < 1e-6 {
            Vec2::new(1.0, 0.0)
        } else {
            Vec2::new(self.x / len, self.y / len)
        }
    }
}

impl std::ops::Add for Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl std::ops::Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, rhs: Vec2) -> Vec2 {
        Vec2::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, rhs: f32) -> Vec2 {
        Vec2::new(self.x * rhs, self.y * rhs)
    }
}
