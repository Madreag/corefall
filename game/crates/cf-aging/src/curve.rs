//! **M14I** § per-origin aging-degradation curve.
//!
//! The spec specifies per-year decay coefficients for `caloric_max`,
//! `max_speed`, and `wound_heal_rate`. Humans decay fastest; heavy biomech
//! decays slowest. Robots / crystallines never sample this curve (the
//! caller skips them via [`AgingOrigin::is_biological`]).

use serde::{Deserialize, Serialize};

use super::AgingOrigin;

/// Default per-year caloric_max decay coefficient (human, post-30).
pub const CALORIC_MAX_DECAY_PER_YEAR_DEFAULT: f32 = 0.005;

/// Default per-year max_speed decay coefficient (human, post-30).
pub const MAX_SPEED_DECAY_PER_YEAR_DEFAULT: f32 = 0.004;

/// Default per-year wound_heal_rate decay coefficient (human, post-30).
pub const HEAL_RATE_DECAY_PER_YEAR_DEFAULT: f32 = 0.006;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AgingCurve {
    pub caloric_max_decay_per_year: f32,
    pub max_speed_decay_per_year: f32,
    pub heal_rate_decay_per_year: f32,
}

impl AgingCurve {
    pub fn human() -> Self {
        Self {
            caloric_max_decay_per_year: CALORIC_MAX_DECAY_PER_YEAR_DEFAULT,
            max_speed_decay_per_year: MAX_SPEED_DECAY_PER_YEAR_DEFAULT,
            heal_rate_decay_per_year: HEAL_RATE_DECAY_PER_YEAR_DEFAULT,
        }
    }
}

#[must_use]
pub fn age_curve_for_origin(origin: AgingOrigin) -> AgingCurve {
    match origin {
        AgingOrigin::Human => AgingCurve::human(),
        AgingOrigin::PoweredOrganic => AgingCurve {
            caloric_max_decay_per_year: 0.003,
            max_speed_decay_per_year: 0.003,
            heal_rate_decay_per_year: 0.004,
        },
        AgingOrigin::AndroidOrganicSide => AgingCurve {
            caloric_max_decay_per_year: 0.0025,
            max_speed_decay_per_year: 0.002,
            heal_rate_decay_per_year: 0.003,
        },
        AgingOrigin::HeavyBiomech => AgingCurve {
            // Per spec scenario: 0.2%/yr ≈ 0.002.
            caloric_max_decay_per_year: 0.002,
            max_speed_decay_per_year: 0.0015,
            heal_rate_decay_per_year: 0.0025,
        },
        AgingOrigin::Robot | AgingOrigin::Crystalline => AgingCurve {
            caloric_max_decay_per_year: 0.0,
            max_speed_decay_per_year: 0.0,
            heal_rate_decay_per_year: 0.0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_curve_defaults() {
        let c = age_curve_for_origin(AgingOrigin::Human);
        assert_eq!(c.caloric_max_decay_per_year, CALORIC_MAX_DECAY_PER_YEAR_DEFAULT);
    }

    #[test]
    fn robot_curve_is_zero() {
        let c = age_curve_for_origin(AgingOrigin::Robot);
        assert_eq!(c.caloric_max_decay_per_year, 0.0);
        assert_eq!(c.max_speed_decay_per_year, 0.0);
        assert_eq!(c.heal_rate_decay_per_year, 0.0);
    }

    #[test]
    fn biomech_curve_below_human() {
        let h = age_curve_for_origin(AgingOrigin::Human);
        let b = age_curve_for_origin(AgingOrigin::HeavyBiomech);
        assert!(b.caloric_max_decay_per_year < h.caloric_max_decay_per_year);
    }
}
