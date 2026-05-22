//! **M14**: side-view facing × hit routing — front/back/top/bottom zone
//! visibility resolution per spec § "Side-view facing direction × hit
//! routing".
//!
//! The 2D side-view layout exposes different zones depending on which
//! direction the projectile arrives from. A front-facing shot hits the
//! actor's front-arm / front-leg / torso_front; a back-facing shot hits
//! the back-arm (which the actor's front-arm normally hides) / back-leg /
//! torso_back / backpack; top-down hits land on head_top / shoulders;
//! bottom-up hits land on feet / abdomen.
//!
//! All functions pure / deterministic. No clocks; no RNG.

use serde::{Deserialize, Serialize};

/// Hit direction relative to the actor's facing direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HitDirection {
    Front,
    Back,
    Top,
    Bottom,
}

impl HitDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            HitDirection::Front => "front",
            HitDirection::Back => "back",
            HitDirection::Top => "top",
            HitDirection::Bottom => "bottom",
        }
    }
}

/// relative to the target's facing sign (+1.0 right, -1.0 left).
///
/// Algorithm: compute the dot of the velocity with the facing vector
/// (left/right = ±x). If the projectile is approaching head-on it's
/// front; from behind it's back; vertical components dominate at >0.7
/// magnitude → top/bottom.
#[must_use]
pub fn classify_hit_direction(velocity: (f32, f32), facing_sign: f32) -> HitDirection {
    let (vx, vy) = velocity;
    let mag = (vx * vx + vy * vy).sqrt().max(f32::EPSILON);
    let nx = vx / mag;
    let ny = vy / mag;
    // Vertical dominance check first.
    if ny.abs() > 0.7 {
        return if ny > 0.0 { HitDirection::Bottom } else { HitDirection::Top };
    }
    // Horizontal: hit direction relative to facing.
    // Projectile traveling +x hits an actor facing +x from BEHIND (the
    // projectile catches up).
    // Projectile traveling +x hits an actor facing -x from the FRONT.
    let facing = if facing_sign >= 0.0 { 1.0_f32 } else { -1.0_f32 };
    // signum() returns ±1.0 exactly for finite non-zero inputs; strict
    // equality is the intended check here.
    #[allow(clippy::float_cmp)]
    let matches = nx.signum() == facing;
    if matches {
        HitDirection::Back
    } else {
        HitDirection::Front
    }
}

/// engine consults this when resolving which zone the projectile damages
/// (a back-facing hit can damage the back_arm + backpack; a front-facing
/// hit cannot, because the actor's body blocks them).
///
/// Spec § "Determine exposed zones":
/// - Front: front_arm, front_leg, torso_front
/// - Back: back_arm, back_leg, torso_back, backpack
/// - Top: head_top, torso_top, shoulders
/// - Bottom: feet, legs, abdomen
#[must_use]
pub fn exposed_zones(direction: HitDirection) -> &'static [&'static str] {
    match direction {
        HitDirection::Front => &["arm_left", "leg_left", "torso", "head", "hand_left", "forearm_left"],
        HitDirection::Back => &["arm_right", "leg_right", "torso", "backpack", "hand_right", "forearm_right"],
        HitDirection::Top => &["head", "torso", "arm_left", "arm_right"],
        HitDirection::Bottom => &[
            "foot_left",
            "foot_right",
            "shin_left",
            "shin_right",
            "leg_left",
            "leg_right",
        ],
    }
}

/// pipeline step 4 says "Mirror local_x by Actor::facing". Right-facing
/// actor: identity. Left-facing actor: flip x. Local-space y stays the
/// same (the side-view is top-down on the vertical axis).
#[must_use]
pub fn mirror_local_x(local_x: f32, facing_sign: f32) -> f32 {
    if facing_sign >= 0.0 {
        local_x
    } else {
        -local_x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_horizontal_projectile_back_when_aligned_with_facing() {
        // Right-facing actor; projectile traveling +x → back hit.
        let d = classify_hit_direction((1.0, 0.0), 1.0);
        assert_eq!(d, HitDirection::Back);
    }

    #[test]
    fn classify_horizontal_projectile_front_when_opposed() {
        // Right-facing actor; projectile traveling -x → front hit.
        let d = classify_hit_direction((-1.0, 0.0), 1.0);
        assert_eq!(d, HitDirection::Front);
    }

    #[test]
    fn classify_left_facing_inverts_front_back() {
        // Left-facing actor; projectile +x → front hit.
        let d = classify_hit_direction((1.0, 0.0), -1.0);
        assert_eq!(d, HitDirection::Front);
    }

    #[test]
    fn classify_top_when_vertical_dominant_downward() {
        // Projectile traveling -y (top to bottom): hits TOP of actor.
        let d = classify_hit_direction((0.1, -1.0), 1.0);
        assert_eq!(d, HitDirection::Top);
    }

    #[test]
    fn classify_bottom_when_vertical_dominant_upward() {
        let d = classify_hit_direction((0.1, 1.0), 1.0);
        assert_eq!(d, HitDirection::Bottom);
    }

    #[test]
    fn exposed_zones_front_includes_torso() {
        let zones = exposed_zones(HitDirection::Front);
        assert!(zones.contains(&"torso"));
    }

    #[test]
    fn exposed_zones_back_includes_backpack() {
        let zones = exposed_zones(HitDirection::Back);
        assert!(zones.contains(&"backpack"));
    }

    #[test]
    fn exposed_zones_bottom_includes_feet() {
        let zones = exposed_zones(HitDirection::Bottom);
        assert!(zones.contains(&"foot_left"));
        assert!(zones.contains(&"foot_right"));
    }

    #[test]
    fn mirror_left_facing_flips_local_x() {
        assert!((mirror_local_x(5.0, -1.0) - -5.0).abs() < f32::EPSILON);
        assert!((mirror_local_x(5.0, 1.0) - 5.0).abs() < f32::EPSILON);
    }
}
