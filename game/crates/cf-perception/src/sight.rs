//! M6: line-of-sight checks for the perception kernel.
//!
//! Sight is the cheaper of the two perception channels: a single ray from
//! observer to target, attenuated by intervening cover and by the target's
//! current stealth profile. The actual ray-cast against terrain pixels is
//! delegated to the engine (the perception crate stays terrain-agnostic so
//! it can be reused by integration tests with synthetic occluders).

use serde::{Deserialize, Serialize};

use cf_actor::Vec2;

/// One sight-line query.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SightCheck {
    pub observer: Vec2,
    pub observer_facing_x: f32,
    pub target: Vec2,
    /// Half-angle (radians) of the observer's view cone. 0 = pinhole;
    /// `std::f32::consts::PI` = omnidirectional. Default Infantry cone ≈ 1.0 rad.
    pub view_cone_half_angle: f32,
    /// Maximum sight range (world units).
    pub max_range: f32,
    /// Cumulative occlusion factor along the ray (0.0 = fully blocked,
    /// 1.0 = clear). Engine computes by sampling terrain.
    pub occlusion_factor: f32,
}

impl Default for SightCheck {
    fn default() -> Self {
        Self {
            observer: Vec2::ZERO,
            observer_facing_x: 1.0,
            target: Vec2::ZERO,
            view_cone_half_angle: 1.0,
            max_range: 240.0,
            occlusion_factor: 1.0,
        }
    }
}

/// Outcome of a sight check.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SightResult {
    pub in_cone: bool,
    pub in_range: bool,
    /// Visibility multiplier 0..1; combine with target's stealth profile.
    pub visibility: f32,
    pub distance: f32,
}

impl SightResult {
    pub fn is_visible(self) -> bool {
        self.in_cone && self.in_range && self.visibility > 0.05
    }
}

#[must_use]
pub fn compute_sightline(check: SightCheck) -> SightResult {
    let dx = check.target.x - check.observer.x;
    let dy = check.target.y - check.observer.y;
    if !dx.is_finite() || !dy.is_finite() {
        return SightResult {
            in_cone: false,
            in_range: false,
            visibility: 0.0,
            distance: f32::INFINITY,
        };
    }
    let distance = (dx * dx + dy * dy).sqrt();
    let in_range = distance <= check.max_range;
    let facing_sign = if check.observer_facing_x >= 0.0 { 1.0 } else { -1.0 };
    let aligned = (dx * facing_sign).max(0.0);
    let in_cone = if distance < 1e-3 {
        true
    } else {
        let cone_cos = check.view_cone_half_angle.cos().clamp(-1.0, 1.0);
        (aligned / distance) >= cone_cos
    };
    let range_factor = if check.max_range > 0.0 {
        (1.0 - (distance / check.max_range)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let visibility = check.occlusion_factor.clamp(0.0, 1.0) * range_factor;
    SightResult {
        in_cone,
        in_range,
        visibility,
        distance,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directly_in_front_visible() {
        let check = SightCheck {
            observer: Vec2::new(0.0, 0.0),
            observer_facing_x: 1.0,
            target: Vec2::new(50.0, 0.0),
            view_cone_half_angle: 1.0,
            max_range: 200.0,
            occlusion_factor: 1.0,
        };
        let r = compute_sightline(check);
        assert!(r.in_cone);
        assert!(r.in_range);
        assert!(r.is_visible());
    }

    #[test]
    fn behind_back_not_in_cone() {
        let check = SightCheck {
            observer: Vec2::new(0.0, 0.0),
            observer_facing_x: 1.0,
            target: Vec2::new(-50.0, 0.0),
            view_cone_half_angle: 1.0,
            max_range: 200.0,
            occlusion_factor: 1.0,
        };
        let r = compute_sightline(check);
        assert!(!r.in_cone);
    }

    #[test]
    fn out_of_range_returns_zero_visibility() {
        let check = SightCheck {
            observer: Vec2::new(0.0, 0.0),
            observer_facing_x: 1.0,
            target: Vec2::new(500.0, 0.0),
            view_cone_half_angle: 1.0,
            max_range: 200.0,
            occlusion_factor: 1.0,
        };
        let r = compute_sightline(check);
        assert!(!r.in_range);
        assert_eq!(r.visibility, 0.0);
    }

    #[test]
    fn nan_target_rejected() {
        let check = SightCheck {
            observer: Vec2::new(0.0, 0.0),
            observer_facing_x: 1.0,
            target: Vec2::new(f32::NAN, 0.0),
            ..SightCheck::default()
        };
        let r = compute_sightline(check);
        assert_eq!(r.visibility, 0.0);
    }
}
