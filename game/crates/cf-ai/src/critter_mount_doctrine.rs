//! **M14J** § "AI critter gait coordination when ridden vs free".
//!
//! When a critter is being ridden by a player or AI rider, the critter
//! AI swaps its locomotion goal from "free roam" (M27 wandering doctrine)
//! to "obey rider ride_direction". This module owns the small per-tick
//! gait selector that picks (walk / trot / canter / gallop) from the
//! magnitude of the rider's `ride_direction` input.
//!
//! Pure / deterministic. Self-contained: no cross-crate state.

use serde::{Deserialize, Serialize};

/// **M14J** § "rider provides `ride_direction` (dx, dy as input to
/// critter's locomotion goal); critter AI handles gait selection (walk /
/// trot / canter / gallop) via its own M14A limb paths".
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CritterGait {
    /// Standing in place; no forward motion.
    #[default]
    Idle = 0,
    /// Slow walk (~1.5 m/s).
    Walk = 1,
    /// Medium trot (~3 m/s).
    Trot = 2,
    /// Fast canter (~6 m/s).
    Canter = 3,
    /// Top-speed gallop (~9 m/s).
    Gallop = 4,
}

impl CritterGait {
    pub fn as_str(self) -> &'static str {
        match self {
            CritterGait::Idle => "idle",
            CritterGait::Walk => "walk",
            CritterGait::Trot => "trot",
            CritterGait::Canter => "canter",
            CritterGait::Gallop => "gallop",
        }
    }

    /// Top speed (m/s) corresponding to this gait. Used by the M14A
    /// walking sim to target locomotion velocity.
    pub fn top_speed_m_s(self) -> f32 {
        match self {
            CritterGait::Idle => 0.0,
            CritterGait::Walk => 1.5,
            CritterGait::Trot => 3.0,
            CritterGait::Canter => 6.0,
            CritterGait::Gallop => 9.0,
        }
    }
}

/// **M14J** § "Mounted-pairing combined mass: rider.mass + critter.mass →
/// critter chassis aggregates both for M14A speed curves" — per-tick
/// gait + locomotion goal output.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CritterMountGoal {
    pub gait: CritterGait,
    /// Locomotion goal direction (normalized) the critter pursues this tick.
    pub goal_direction: [f32; 2],
    /// Effective top speed after the rider-mass penalty (spec ~20%).
    pub effective_top_speed: f32,
}

impl Default for CritterMountGoal {
    fn default() -> Self {
        Self {
            gait: CritterGait::Idle,
            goal_direction: [0.0, 0.0],
            effective_top_speed: 0.0,
        }
    }
}

/// **M14J** § "rider weapon inputs go to rider's own arm sim —
/// independent of critter motion" — pick a gait from the rider's
/// `ride_direction` magnitude. Pure / deterministic.
///
/// `rider_dx`, `rider_dy`: rider's ride_direction (clamped at unit).
/// `mount_speed_multiplier`: the critter's M14A speed retention factor
/// when carrying a rider (e.g. `MOUNT_TOP_SPEED_RETAINED = 0.80`).
#[must_use]
pub fn select_gait_for_ride_input(
    rider_dx: f32,
    rider_dy: f32,
    mount_speed_multiplier: f32,
) -> CritterMountGoal {
    let mag = (rider_dx * rider_dx + rider_dy * rider_dy).sqrt();
    let gait = if mag < 0.01 {
        CritterGait::Idle
    } else if mag < 0.3 {
        CritterGait::Walk
    } else if mag < 0.6 {
        CritterGait::Trot
    } else if mag < 0.9 {
        CritterGait::Canter
    } else {
        CritterGait::Gallop
    };
    let goal = if mag > 1e-6 {
        [rider_dx / mag, rider_dy / mag]
    } else {
        [0.0, 0.0]
    };
    CritterMountGoal {
        gait,
        goal_direction: goal,
        effective_top_speed: gait.top_speed_m_s() * mount_speed_multiplier.clamp(0.1, 1.0),
    }
}

/// **M14J** § "critter AI swaps free roam → obey rider" — pick the gait
/// for a free / un-ridden critter. M27's wandering doctrine produces the
/// rider-direction input via its own pathing; for the unmounted case the
/// AI returns Idle by default (M27 free-roam is layered above this).
#[must_use]
pub fn select_gait_for_free_critter() -> CritterMountGoal {
    CritterMountGoal::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_input_yields_idle() {
        let g = select_gait_for_ride_input(0.0, 0.0, 0.8);
        assert_eq!(g.gait, CritterGait::Idle);
        assert!(g.effective_top_speed.abs() < 1e-6);
    }

    #[test]
    fn full_input_yields_gallop_with_mass_penalty() {
        let g = select_gait_for_ride_input(1.0, 0.0, 0.8);
        assert_eq!(g.gait, CritterGait::Gallop);
        let raw = CritterGait::Gallop.top_speed_m_s();
        assert!((g.effective_top_speed - raw * 0.8).abs() < 1e-3);
    }

    #[test]
    fn mid_input_yields_trot() {
        let g = select_gait_for_ride_input(0.4, 0.0, 1.0);
        assert_eq!(g.gait, CritterGait::Trot);
    }

    #[test]
    fn free_critter_returns_idle() {
        let g = select_gait_for_free_critter();
        assert_eq!(g.gait, CritterGait::Idle);
    }

    #[test]
    fn deterministic() {
        let a = select_gait_for_ride_input(0.5, 0.0, 0.8);
        let b = select_gait_for_ride_input(0.5, 0.0, 0.8);
        assert_eq!(a, b);
    }
}
