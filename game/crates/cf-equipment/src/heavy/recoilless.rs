//! M6C: Recoilless Rifle — anti-armor + back-blast.
//!
//! Recoilless weapons vent propellant gases rearward to cancel out the
//! firing impulse. The trade-off is a hazardous back-blast cone that
//! damages friendlies + flammable terrain behind the firer. Engine
//! consumers should refuse to chamber a round when the back-blast cone
//! intersects a friendly or a confined-space marker.

use serde::{Deserialize, Serialize};

/// Length of the back-blast cone in world units measured from the muzzle
/// rearward.
pub const BACK_BLAST_LENGTH_UNITS: f32 = 64.0;

/// Half-angle of the back-blast cone in radians (~30°).
pub const BACK_BLAST_HALF_ANGLE_RAD: f32 = 0.524;

/// Per-tick damage dealt to actors caught inside the back-blast cone
/// (applied only on the tick the weapon fires).
pub const BACK_BLAST_DAMAGE_AT_MUZZLE: f32 = 90.0;

/// One snapshot of an actor relative to a fired recoilless weapon. The
/// engine passes this into [`evaluate_back_blast_hit`] for every actor
/// within `BACK_BLAST_LENGTH_UNITS`; positive results take damage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BackBlastCandidate {
    /// Distance behind the muzzle (positive forward of muzzle ⇒ ignored).
    pub distance_behind_units: f32,
    /// Lateral offset from the back-blast centerline (positive = right).
    pub lateral_offset_units: f32,
}

/// Outcome of a back-blast check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackBlastOutcome {
    /// Actor outside cone — no damage.
    Outside,
    /// Actor inside cone — engine applies `damage` HP.
    Inside { damage_x1000: u32 },
}

/// Evaluate whether the candidate actor is inside the back-blast cone +
/// returns the damage to apply. Falls off linearly to zero at
/// `BACK_BLAST_LENGTH_UNITS`.
#[must_use]
pub fn evaluate_back_blast_hit(candidate: BackBlastCandidate) -> BackBlastOutcome {
    let d = candidate.distance_behind_units;
    if !d.is_finite() || d <= 0.0 || d >= BACK_BLAST_LENGTH_UNITS {
        return BackBlastOutcome::Outside;
    }
    let half_width_at_d = d * BACK_BLAST_HALF_ANGLE_RAD.tan();
    if candidate.lateral_offset_units.abs() > half_width_at_d {
        return BackBlastOutcome::Outside;
    }
    let falloff = (1.0 - d / BACK_BLAST_LENGTH_UNITS).clamp(0.0, 1.0);
    let dmg = BACK_BLAST_DAMAGE_AT_MUZZLE * falloff;
    BackBlastOutcome::Inside {
        damage_x1000: (dmg * 1000.0).round() as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_directly_behind_muzzle_takes_damage() {
        let r = evaluate_back_blast_hit(BackBlastCandidate {
            distance_behind_units: 20.0,
            lateral_offset_units: 0.0,
        });
        assert!(matches!(r, BackBlastOutcome::Inside { .. }));
    }

    #[test]
    fn actor_far_outside_cone_is_safe() {
        let r = evaluate_back_blast_hit(BackBlastCandidate {
            distance_behind_units: 20.0,
            lateral_offset_units: 200.0,
        });
        assert_eq!(r, BackBlastOutcome::Outside);
    }

    #[test]
    fn actor_in_front_of_muzzle_is_safe() {
        let r = evaluate_back_blast_hit(BackBlastCandidate {
            distance_behind_units: -10.0,
            lateral_offset_units: 0.0,
        });
        assert_eq!(r, BackBlastOutcome::Outside);
    }

    #[test]
    fn damage_falls_off_with_distance() {
        let BackBlastOutcome::Inside {
            damage_x1000: near, ..
        } = evaluate_back_blast_hit(BackBlastCandidate {
            distance_behind_units: 5.0,
            lateral_offset_units: 0.0,
        })
        else {
            panic!("expected inside");
        };
        let BackBlastOutcome::Inside {
            damage_x1000: far, ..
        } = evaluate_back_blast_hit(BackBlastCandidate {
            distance_behind_units: 50.0,
            lateral_offset_units: 0.0,
        })
        else {
            panic!("expected inside");
        };
        assert!(near > far, "near={near}, far={far}");
    }
}
