//! **M14J** § "grappling-hook gun".
//!
//! T2 ranged equipment that fires a rope-tethered hook projectile. On
//! embed, a verlet rope (cf-physics::rope) deploys between the gun's
//! muzzle (actor hand bone) and the embedded anchor point.
//!
//! Build cost: `20 steel + 5 motor + 10 rope` at the T2 fabricator
//! (per spec § "Fire a grappling-hook gun").
//!
//! Pure / deterministic: every helper takes state in and returns state
//! out. No clock reads.

use serde::{Deserialize, Serialize};

pub const GRAPPLE_GUN_T2_ID: &str = "grapple_gun_t2";

/// motor + 10 rope; built at T2 fabricator) at any surface within 30 m".
pub const GRAPPLE_MAX_RANGE_M: f32 = 30.0;

pub const ROPE_CLIMB_SPEED_M_PER_S: f32 = 0.8;

pub const ROPE_RAPPEL_SPEED_M_PER_S: f32 = 1.5;

/// ≥ 25 m) registers as an M25 story-grade event".
pub const GRAPPLE_LONG_DISTANCE_M: f32 = 25.0;

/// or `Missed` (out of range or non-anchorable material).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GrappleFireOutcome {
    /// Hook embedded at the target. Carries the anchor point + the rope
    /// length so the engine can spawn the verlet rope.
    Embedded {
        anchor: [f32; 2],
        rope_length_m: f32,
        long_distance: bool,
    },
    /// Hook missed (too far or non-anchorable surface).
    Missed { reason: &'static str },
}

/// resolve a fire attempt. Pure / deterministic. Caller supplies the
/// `material_anchorable(x, y)` predicate (chunked terrain anchor lookup).
#[must_use]
pub fn fire_grapple<F>(
    origin: [f32; 2],
    target: [f32; 2],
    material_anchorable: F,
) -> GrappleFireOutcome
where
    F: Fn(f32, f32) -> bool,
{
    let dx = target[0] - origin[0];
    let dy = target[1] - origin[1];
    let dist = (dx * dx + dy * dy).sqrt();
    if !dist.is_finite() || dist > GRAPPLE_MAX_RANGE_M {
        return GrappleFireOutcome::Missed {
            reason: "out_of_range",
        };
    }
    if !material_anchorable(target[0], target[1]) {
        return GrappleFireOutcome::Missed {
            reason: "material_not_anchorable",
        };
    }
    GrappleFireOutcome::Embedded {
        anchor: target,
        rope_length_m: dist.max(1.0),
        long_distance: dist >= GRAPPLE_LONG_DISTANCE_M,
    }
}

pub const SADDLE_UNIVERSAL_ID: &str = "saddle_universal";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fire_grapple_embeds_at_anchorable_target_within_range() {
        let anchor = |_x: f32, _y: f32| true;
        let outcome = fire_grapple([0.0, 0.0], [18.0, 0.0], anchor);
        match outcome {
            GrappleFireOutcome::Embedded {
                anchor,
                rope_length_m,
                long_distance,
            } => {
                assert_eq!(anchor, [18.0, 0.0]);
                assert!((rope_length_m - 18.0).abs() < 1e-3);
                assert!(!long_distance);
            }
            _ => panic!("expected Embedded"),
        }
    }

    #[test]
    fn fire_grapple_rejects_out_of_range() {
        let anchor = |_x: f32, _y: f32| true;
        let outcome = fire_grapple([0.0, 0.0], [40.0, 0.0], anchor);
        assert!(matches!(outcome, GrappleFireOutcome::Missed { reason } if reason == "out_of_range"));
    }

    #[test]
    fn fire_grapple_rejects_non_anchorable() {
        let anchor = |_x: f32, _y: f32| false;
        let outcome = fire_grapple([0.0, 0.0], [10.0, 0.0], anchor);
        assert!(matches!(
            outcome,
            GrappleFireOutcome::Missed { reason } if reason == "material_not_anchorable"
        ));
    }

    #[test]
    fn fire_grapple_marks_long_distance_shot() {
        let anchor = |_x: f32, _y: f32| true;
        let outcome = fire_grapple([0.0, 0.0], [27.0, 0.0], anchor);
        assert!(matches!(
            outcome,
            GrappleFireOutcome::Embedded {
                long_distance: true,
                ..
            }
        ));
    }
}
