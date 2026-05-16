//! M6: stealth kill / takedown.
//!
//! Spec § "Stealth kill instant-kill from behind: only available when
//! `stealth_meter < 30%`".

use serde::{Deserialize, Serialize};

/// M6 § stealth-meter threshold below which stealth kills are permitted.
pub const STEALTH_KILL_METER_MAX: f32 = 0.3;

/// Spec § "Visible 1.2s animation": stealth-kill stance dwell time.
pub const STEALTH_KILL_ANIMATION_SECONDS: f32 = 1.2;

/// One stealth-kill attempt + outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StealthKillAttempt {
    pub attacker_actor: u64,
    pub victim_actor: u64,
    pub attacker_facing_x: f32,
    pub victim_facing_x: f32,
    pub stealth_meter: f32,
    pub distance: f32,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StealthKillRejection {
    NotBehindTarget,
    TooFar,
    Spotted,
    InvalidGeometry,
}

impl StealthKillRejection {
    pub fn as_str(self) -> &'static str {
        match self {
            StealthKillRejection::NotBehindTarget => "not_behind_target",
            StealthKillRejection::TooFar => "too_far",
            StealthKillRejection::Spotted => "spotted",
            StealthKillRejection::InvalidGeometry => "invalid_geometry",
        }
    }
}

/// Maximum reach for stealth kill (world units).
pub const STEALTH_KILL_REACH: f32 = 20.0;

/// Evaluate an attempt. Returns `Ok` if the stealth kill should land.
pub fn evaluate_attempt(a: &StealthKillAttempt) -> Result<(), StealthKillRejection> {
    if !a.stealth_meter.is_finite()
        || !a.attacker_facing_x.is_finite()
        || !a.victim_facing_x.is_finite()
        || !a.distance.is_finite()
    {
        return Err(StealthKillRejection::InvalidGeometry);
    }
    if a.stealth_meter >= STEALTH_KILL_METER_MAX {
        return Err(StealthKillRejection::Spotted);
    }
    if a.distance > STEALTH_KILL_REACH {
        return Err(StealthKillRejection::TooFar);
    }
    let same_dir = a.attacker_facing_x.signum() == a.victim_facing_x.signum();
    if !same_dir {
        return Err(StealthKillRejection::NotBehindTarget);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_when_behind_and_unseen() {
        let a = StealthKillAttempt {
            attacker_actor: 1,
            victim_actor: 2,
            attacker_facing_x: 1.0,
            victim_facing_x: 1.0,
            stealth_meter: 0.2,
            distance: 10.0,
        };
        assert!(evaluate_attempt(&a).is_ok());
    }

    #[test]
    fn rejects_when_face_to_face() {
        let a = StealthKillAttempt {
            attacker_actor: 1,
            victim_actor: 2,
            attacker_facing_x: 1.0,
            victim_facing_x: -1.0,
            stealth_meter: 0.1,
            distance: 5.0,
        };
        assert_eq!(evaluate_attempt(&a), Err(StealthKillRejection::NotBehindTarget));
    }

    #[test]
    fn rejects_when_spotted() {
        let a = StealthKillAttempt {
            attacker_actor: 1,
            victim_actor: 2,
            attacker_facing_x: 1.0,
            victim_facing_x: 1.0,
            stealth_meter: 0.5,
            distance: 5.0,
        };
        assert_eq!(evaluate_attempt(&a), Err(StealthKillRejection::Spotted));
    }

    #[test]
    fn rejects_when_too_far() {
        let a = StealthKillAttempt {
            attacker_actor: 1,
            victim_actor: 2,
            attacker_facing_x: 1.0,
            victim_facing_x: 1.0,
            stealth_meter: 0.1,
            distance: 50.0,
        };
        assert_eq!(evaluate_attempt(&a), Err(StealthKillRejection::TooFar));
    }

    #[test]
    fn rejects_nan_geometry() {
        let a = StealthKillAttempt {
            attacker_actor: 1,
            victim_actor: 2,
            attacker_facing_x: f32::NAN,
            victim_facing_x: 1.0,
            stealth_meter: 0.1,
            distance: 5.0,
        };
        assert_eq!(evaluate_attempt(&a), Err(StealthKillRejection::InvalidGeometry));
    }
}
