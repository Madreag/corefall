//! M6C-3: ATGM Javelin top-attack lock state machine.
//!
//! Gherkin scenario M6C-3:
//! ```text
//! Scenario: M6C-3 ATGM Javelin top-attack lock
//!   Given player aims atgm_javelin at tank chassis
//!   When lock_acquired fires after 3s
//!   And player releases:
//!     Then projectile arcs to top of target
//!     And HEAT-tandem penetrates top armor
//! ```
//!
//! The lock is acquired by holding the trigger on a valid target for
//! [`ATGM_LOCK_ACQUISITION_SECONDS`]; the trigger release fires a
//! top-attack profile. Breaking the line of sight before acquisition
//! cancels the lock (count resets when target id changes).

use serde::{Deserialize, Serialize};

pub const ATGM_LOCK_ACQUISITION_SECONDS: f32 = 3.0;

/// State of the per-actor ATGM lock-on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AtgmLockState {
    Idle = 0,
    Acquiring = 1,
    Locked = 2,
}

impl AtgmLockState {
    pub fn as_str(self) -> &'static str {
        match self {
            AtgmLockState::Idle => "idle",
            AtgmLockState::Acquiring => "acquiring",
            AtgmLockState::Locked => "locked",
        }
    }
}

impl Default for AtgmLockState {
    fn default() -> Self {
        AtgmLockState::Idle
    }
}

/// Outcome of one tick of [`AtgmLockTracker::tick`].
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AtgmLockOutcome {
    /// State transitioned from Acquiring → Locked this tick.
    pub lock_acquired_this_tick: bool,
    /// Lost the lock (target id changed or line of sight broken).
    pub lock_lost_this_tick: bool,
    /// Player fired (trigger released while Locked) — engine should spawn a
    /// top-attack arcing projectile against the locked target.
    pub fired_top_attack_this_tick: bool,
    /// Current acquisition progress in [0, 1].
    pub progress: f32,
}

/// Per-actor ATGM lock-on tracker. Determinism guaranteed: the same input
/// sequence always produces the same outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AtgmLockTracker {
    pub state: AtgmLockState,
    /// Seconds spent acquiring (resets when target changes or LOS lost).
    pub acquiring_seconds: f32,
    /// Currently-locked target opaque id (0 = none).
    pub target_id: u64,
}

impl AtgmLockTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// One fixed-tick step. `target_id` is the currently-painted target
    /// (0 = no valid target / line of sight lost). `trigger_held` true
    /// while the player is holding the lock-on button. `trigger_released`
    /// is a 1-tick edge — true on the tick the player releases.
    /// `dt_seconds` is the tick duration in seconds (e.g. 1.0/60.0 at 60 Hz).
    pub fn tick(&mut self, target_id: u64, trigger_held: bool, trigger_released: bool, dt_seconds: f32) -> AtgmLockOutcome {
        let mut out = AtgmLockOutcome::default();
        let dt = dt_seconds.max(0.0);

        // Target lost or trigger released → cancel acquisition.
        if target_id == 0 || !trigger_held {
            if self.state == AtgmLockState::Acquiring {
                out.lock_lost_this_tick = true;
            }
            // Fire on trigger release while LOCKED.
            if self.state == AtgmLockState::Locked && trigger_released {
                out.fired_top_attack_this_tick = true;
            }
            // Reset on release: a fresh lock requires a fresh hold.
            if trigger_released || target_id == 0 {
                self.state = AtgmLockState::Idle;
                self.acquiring_seconds = 0.0;
                self.target_id = 0;
            }
            out.progress = (self.acquiring_seconds / ATGM_LOCK_ACQUISITION_SECONDS).clamp(0.0, 1.0);
            return out;
        }

        // Target changed mid-acquisition → reset accumulator.
        if self.target_id != target_id {
            if self.state == AtgmLockState::Acquiring || self.state == AtgmLockState::Locked {
                out.lock_lost_this_tick = true;
            }
            self.target_id = target_id;
            self.acquiring_seconds = 0.0;
            self.state = AtgmLockState::Acquiring;
        } else if self.state == AtgmLockState::Idle {
            self.state = AtgmLockState::Acquiring;
        }

        if self.state == AtgmLockState::Acquiring {
            self.acquiring_seconds += dt;
            if self.acquiring_seconds >= ATGM_LOCK_ACQUISITION_SECONDS {
                self.state = AtgmLockState::Locked;
                out.lock_acquired_this_tick = true;
            }
        }
        out.progress = (self.acquiring_seconds / ATGM_LOCK_ACQUISITION_SECONDS).clamp(0.0, 1.0);
        out
    }

    pub fn is_locked(&self) -> bool {
        matches!(self.state, AtgmLockState::Locked)
    }

    pub fn reset(&mut self) {
        self.state = AtgmLockState::Idle;
        self.acquiring_seconds = 0.0;
        self.target_id = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_acquired_after_3_seconds() {
        // M6C-3 Scenario:
        //   Given player aims atgm_javelin at tank chassis
        //   When lock_acquired fires after 3s
        let mut t = AtgmLockTracker::new();
        let dt = 1.0 / 60.0;
        let mut acquired = false;
        let mut elapsed = 0.0;
        // Tick at 60 Hz for slightly over 3s.
        for _ in 0..(60 * 4) {
            let out = t.tick(42, true, false, dt);
            elapsed += dt;
            if out.lock_acquired_this_tick {
                acquired = true;
                break;
            }
        }
        assert!(acquired);
        // Must take at least 3.0 - one-tick of slop.
        assert!(elapsed >= ATGM_LOCK_ACQUISITION_SECONDS - dt);
        // Locked state persists until release.
        assert!(t.is_locked());
    }

    #[test]
    fn release_after_lock_fires_top_attack() {
        // M6C-3 Scenario continued:
        //   And player releases:
        //     Then projectile arcs to top of target
        let mut t = AtgmLockTracker::new();
        let dt = 1.0 / 60.0;
        for _ in 0..(60 * 4) {
            let _ = t.tick(7, true, false, dt);
            if t.is_locked() {
                break;
            }
        }
        assert!(t.is_locked());
        let release = t.tick(7, false, true, dt);
        assert!(release.fired_top_attack_this_tick);
        assert_eq!(t.state, AtgmLockState::Idle);
    }

    #[test]
    fn lost_los_resets_acquisition() {
        let mut t = AtgmLockTracker::new();
        let dt = 1.0 / 60.0;
        // Acquire for 1s.
        for _ in 0..60 {
            let _ = t.tick(7, true, false, dt);
        }
        assert_eq!(t.state, AtgmLockState::Acquiring);
        // LoS broken (target_id = 0).
        let out = t.tick(0, true, false, dt);
        assert!(out.lock_lost_this_tick);
        assert_eq!(t.state, AtgmLockState::Idle);
        assert_eq!(t.acquiring_seconds, 0.0);
    }

    #[test]
    fn target_swap_resets_accumulator() {
        let mut t = AtgmLockTracker::new();
        let dt = 1.0 / 60.0;
        for _ in 0..60 {
            let _ = t.tick(7, true, false, dt);
        }
        let elapsed_before = t.acquiring_seconds;
        assert!(elapsed_before > 0.5);
        let out = t.tick(99, true, false, dt);
        assert!(out.lock_lost_this_tick);
        // Now acquiring fresh target.
        assert_eq!(t.state, AtgmLockState::Acquiring);
        assert!(t.acquiring_seconds < elapsed_before);
    }

    #[test]
    fn release_before_lock_does_not_fire() {
        let mut t = AtgmLockTracker::new();
        let dt = 1.0 / 60.0;
        for _ in 0..30 {
            let _ = t.tick(7, true, false, dt);
        }
        let out = t.tick(7, false, true, dt);
        assert!(!out.fired_top_attack_this_tick);
    }
}
