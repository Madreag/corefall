//! M6C-5: Defibrillator revive state machine.
//!
//! Gherkin scenario M6C-5:
//! ```text
//! Scenario: M6C-5 Defibrillator revives downed actor (M14H consumer)
//!   Given actor in Downed state
//!   When ally uses defibrillator within 30s window:
//!     Then actor.revived fires
//!     And HP restored to 25%
//! ```
//!
//! The revive window starts the moment an actor enters the Downed state.
//! Outside the window, applying the defibrillator becomes a no-op
//! (engine emits `actor.revive_failed reason="window_expired"`).

use serde::{Deserialize, Serialize};

/// Revive window after Downed onset (seconds).
pub const DEFIB_REVIVE_WINDOW_SECONDS: f32 = 30.0;

/// HP fraction restored on successful revive (0.25 = 25%).
pub const DEFIB_REVIVE_HP_FRACTION: f32 = 0.25;

/// Outcome of a defibrillator application attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefibOutcome {
    /// Successful revive — engine should emit `actor.revived` and restore HP.
    Revived { hp_restore_fraction_x1000: u32 },
    /// Revive window already expired (>30s since Downed).
    WindowExpired { seconds_since_downed: u32 },
    /// Target is not in Downed state.
    NotDowned,
    /// Already dead — no revive possible.
    AlreadyDead,
}

impl DefibOutcome {
    pub fn is_revive(&self) -> bool {
        matches!(self, DefibOutcome::Revived { .. })
    }
}

/// Target snapshot accepted by [`apply_defibrillator`]. The full ActorState
/// lives in cf-actor — this struct keeps the per-revive contract local so
/// the cf-equipment crate stays dependency-free.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DefibTarget {
    /// True when the target is currently Downed.
    pub is_downed: bool,
    /// True when the target is irreversibly dead.
    pub is_dead: bool,
    /// Seconds elapsed since target entered the Downed state.
    pub seconds_since_downed: f32,
}

/// Evaluate a defibrillator application. Returns the [`DefibOutcome`] the
/// engine should record + apply to the target.
pub fn apply_defibrillator(target: DefibTarget) -> DefibOutcome {
    if target.is_dead {
        return DefibOutcome::AlreadyDead;
    }
    if !target.is_downed {
        return DefibOutcome::NotDowned;
    }
    if target.seconds_since_downed > DEFIB_REVIVE_WINDOW_SECONDS {
        return DefibOutcome::WindowExpired {
            seconds_since_downed: target.seconds_since_downed.round() as u32,
        };
    }
    DefibOutcome::Revived {
        hp_restore_fraction_x1000: (DEFIB_REVIVE_HP_FRACTION * 1000.0).round() as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revives_downed_within_30s_window() {
        // M6C-5 Scenario:
        //   Given actor in Downed state
        //   When ally uses defibrillator within 30s window:
        //     Then actor.revived fires
        //     And HP restored to 25%
        let out = apply_defibrillator(DefibTarget {
            is_downed: true,
            is_dead: false,
            seconds_since_downed: 15.0,
        });
        match out {
            DefibOutcome::Revived { hp_restore_fraction_x1000 } => {
                assert_eq!(hp_restore_fraction_x1000, 250);
            }
            _ => panic!("expected revive, got {out:?}"),
        }
    }

    #[test]
    fn outside_window_rejected_as_expired() {
        let out = apply_defibrillator(DefibTarget {
            is_downed: true,
            is_dead: false,
            seconds_since_downed: 31.0,
        });
        match out {
            DefibOutcome::WindowExpired { seconds_since_downed } => {
                assert_eq!(seconds_since_downed, 31);
            }
            _ => panic!("expected window expired, got {out:?}"),
        }
    }

    #[test]
    fn non_downed_target_rejected() {
        let out = apply_defibrillator(DefibTarget {
            is_downed: false,
            is_dead: false,
            seconds_since_downed: 0.0,
        });
        assert_eq!(out, DefibOutcome::NotDowned);
    }

    #[test]
    fn dead_target_cannot_be_revived() {
        let out = apply_defibrillator(DefibTarget {
            is_downed: true,
            is_dead: true,
            seconds_since_downed: 5.0,
        });
        assert_eq!(out, DefibOutcome::AlreadyDead);
    }

    #[test]
    fn boundary_at_30s_still_revives() {
        let out = apply_defibrillator(DefibTarget {
            is_downed: true,
            is_dead: false,
            seconds_since_downed: 30.0,
        });
        assert!(out.is_revive());
    }
}
