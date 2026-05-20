//! **M14J** § "mount — rider chassis pairing".
//!
//! When a rider mounts a saddled critter, both actors enter a coupled state:
//!  - The rider's [`Stance`](crate::Stance) becomes `Mounted`.
//!  - The critter chassis takes both masses (rider.mass + critter.mass) for
//!    its M14A walk-speed + jetpack-vs-mass math.
//!  - Rider weapon inputs are dispatched to the rider's arm sim
//!    independently of the critter's gait selection.
//!  - The rider tracks `ride_direction` (dx, dy) which the critter AI
//!    consumes as its locomotion goal.
//!
//! Dismount semantics:
//!  - Stationary critter → instant dismount.
//!  - Mid-gallop → 1-tick stagger + rider inherits 70% of critter's
//!    instantaneous velocity.
//!
//! Pure / deterministic: every helper takes state as parameters and returns
//! the new state. No clock reads. No `thread_rng()`.

use serde::{Deserialize, Serialize};

use crate::ActorId;

/// **M14J** § "mounted-pairing combined mass" — fraction of the critter's
/// top speed retained when carrying a human-mass rider. Spec § "notes for
/// the implementer": "The critter loses ~20% top speed when carrying a
/// human-mass rider; tune in content RON".
pub const MOUNT_TOP_SPEED_RETAINED: f32 = 0.80;

/// **M14J** § "Dismount mid-gallop staggers the actor" — fraction of the
/// critter's instantaneous velocity the rider inherits on mid-motion
/// dismount.
pub const DISMOUNT_VELOCITY_INHERIT_FRACTION: f32 = 0.70;

/// **M14J** § "Dismount mid-gallop" — speed threshold above which the
/// dismount is considered "mid-motion" and triggers a stagger. The critter
/// is "stationary" below this (units of world / s).
pub const DISMOUNT_STATIONARY_SPEED_THRESHOLD: f32 = 0.5;

/// **M14J** § "Dismount mid-gallop" — stagger window in milliseconds.
pub const DISMOUNT_MID_MOTION_STAGGER_MS: u32 = 200;

/// **M14J** § "Mount weapon aim spread penalty" — extra spread (radians)
/// added when a mounted rider fires a one-handed weapon at gallop.
pub const MOUNT_MOTION_AIM_SPREAD_RAD: f32 = 0.1;

/// **M14J** § per-rider mount state. Lives on the rider's
/// [`ActorState`](crate::ActorState) so save/load round-trips preserve
/// the pairing. `None` when the rider is not mounted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct MountState {
    /// Critter actor id the rider is paired with.
    pub critter_id: ActorId,
    /// Last `ride_direction` input the rider provided (dx, dy normalized).
    pub ride_direction: [f32; 2],
    /// Combined mass (rider + critter) cached for the critter's M14A
    /// walk-speed curves.
    pub combined_mass_kg: f32,
    /// True when the rider's last fire event was during gait motion (used
    /// to apply [`MOUNT_MOTION_AIM_SPREAD_RAD`]).
    pub firing_during_motion: bool,
}

impl MountState {
    /// Construct a fresh mount pairing.
    #[must_use]
    pub fn new(critter_id: ActorId, rider_mass_kg: f32, critter_mass_kg: f32) -> Self {
        Self {
            critter_id,
            ride_direction: [0.0, 0.0],
            combined_mass_kg: rider_mass_kg.max(0.0) + critter_mass_kg.max(0.0),
            firing_during_motion: false,
        }
    }

    /// Update the rider's ride direction. Caller normalizes the (dx, dy)
    /// vector if the magnitude exceeds 1.
    pub fn set_ride_direction(&mut self, dx: f32, dy: f32) {
        let mag = (dx * dx + dy * dy).sqrt();
        if mag > 1.0 {
            self.ride_direction = [dx / mag, dy / mag];
        } else {
            self.ride_direction = [dx, dy];
        }
    }
}

/// **M14J** § "Mount weapon aim spread" — compute the effective aim spread
/// from the base spread + the mount-motion penalty + critter speed.
#[must_use]
pub fn mounted_aim_spread(base_spread_rad: f32, critter_speed: f32) -> f32 {
    let penalty = if critter_speed.abs() > DISMOUNT_STATIONARY_SPEED_THRESHOLD {
        MOUNT_MOTION_AIM_SPREAD_RAD
    } else {
        0.0
    };
    base_spread_rad + penalty
}

/// **M14J** § "Dismount mid-gallop staggers the actor" — outcome of a
/// dismount attempt. `inherited_velocity` is what the rider's velocity
/// should become after the dismount (zero on stationary; 70% of critter
/// velocity on mid-motion). `stagger_ms` is the stagger window applied
/// when the dismount is mid-motion.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DismountOutcome {
    pub mid_motion: bool,
    pub inherited_velocity: [f32; 2],
    pub stagger_ms: u32,
}

/// Resolve a dismount. Pure / deterministic.
#[must_use]
pub fn resolve_dismount(critter_velocity: [f32; 2]) -> DismountOutcome {
    let speed = (critter_velocity[0] * critter_velocity[0] + critter_velocity[1] * critter_velocity[1]).sqrt();
    if speed <= DISMOUNT_STATIONARY_SPEED_THRESHOLD {
        DismountOutcome {
            mid_motion: false,
            inherited_velocity: [0.0, 0.0],
            stagger_ms: 0,
        }
    } else {
        DismountOutcome {
            mid_motion: true,
            inherited_velocity: [
                critter_velocity[0] * DISMOUNT_VELOCITY_INHERIT_FRACTION,
                critter_velocity[1] * DISMOUNT_VELOCITY_INHERIT_FRACTION,
            ],
            stagger_ms: DISMOUNT_MID_MOTION_STAGGER_MS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mount_state_caches_combined_mass() {
        let m = MountState::new(ActorId(42), 80.0, 350.0);
        assert!((m.combined_mass_kg - 430.0).abs() < 1e-6);
    }

    #[test]
    fn dismount_stationary_is_instant() {
        let outcome = resolve_dismount([0.1, 0.0]);
        assert!(!outcome.mid_motion);
        assert_eq!(outcome.stagger_ms, 0);
        assert_eq!(outcome.inherited_velocity, [0.0, 0.0]);
    }

    #[test]
    fn dismount_mid_gallop_inherits_70pct_velocity() {
        let outcome = resolve_dismount([7.0, 0.0]);
        assert!(outcome.mid_motion);
        assert_eq!(outcome.stagger_ms, DISMOUNT_MID_MOTION_STAGGER_MS);
        assert!((outcome.inherited_velocity[0] - 4.9).abs() < 1e-3);
    }

    #[test]
    fn ride_direction_normalizes() {
        let mut m = MountState::new(ActorId(1), 80.0, 200.0);
        m.set_ride_direction(3.0, 4.0);
        let mag = (m.ride_direction[0] * m.ride_direction[0] + m.ride_direction[1] * m.ride_direction[1]).sqrt();
        assert!((mag - 1.0).abs() < 1e-3);
    }

    #[test]
    fn mounted_aim_spread_penalty_at_speed() {
        let s = mounted_aim_spread(0.05, 8.0);
        assert!((s - (0.05 + MOUNT_MOTION_AIM_SPREAD_RAD)).abs() < 1e-6);
        let s_idle = mounted_aim_spread(0.05, 0.0);
        assert!((s_idle - 0.05).abs() < 1e-6);
    }
}
