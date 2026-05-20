//! M6: stamina state machine (drives Sprint stance auto-cancel).
//!
//! Per spec § "Sprint depletes stamina at 0.2/s while sprinting; +0.3/s recovery
//! when not." Drains are tick-rate-aware; the engine threads `tick_rate_hz`
//! into [`Stamina::step`] so the same simulation behaves identically at 60 Hz
//! and 120 Hz.

use serde::{Deserialize, Serialize};

/// M6 § Sprint stamina drain rate (units of stamina per second; 0..1 scale).
pub const SPRINT_STAMINA_DRAIN_PER_S: f32 = 0.2;
/// M6 § Sprint stamina recovery rate (units of stamina per second).
pub const SPRINT_STAMINA_RECOVERY_PER_S: f32 = 0.3;
/// Threshold below which a sprint intent is rejected and active sprint
/// auto-cancels (per spec: "auto-cancels at 0").
pub const SPRINT_MIN_STAMINA: f32 = 0.0;

/// Per-actor stamina state. Owned by `ActorState` from M6 onward; serde-default
/// keeps M1 / M5 bundles forward-compatible.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Stamina {
    /// Current stamina pool (0..1).
    pub current: f32,
    /// Maximum stamina pool (typically 1.0; future origin-scalar applies here).
    pub max: f32,
    /// True if the actor is actively sprinting (drains).
    pub sprinting: bool,
}

impl Default for Stamina {
    fn default() -> Self {
        Self {
            current: 1.0,
            max: 1.0,
            sprinting: false,
        }
    }
}

impl Stamina {
    pub fn full() -> Self {
        Self::default()
    }

    /// Tick the stamina one simulation step. `tick_rate_hz` is the configured
    /// sim Hz (60 by default; configurable per spec lock).
    pub fn step(&mut self, tick_rate_hz: u32) {
        let rate = tick_rate_hz.max(1) as f32;
        if !self.current.is_finite() || !self.max.is_finite() {
            self.current = 0.0;
            self.max = 1.0;
            return;
        }
        if self.sprinting {
            self.current -= SPRINT_STAMINA_DRAIN_PER_S / rate;
        } else {
            self.current += SPRINT_STAMINA_RECOVERY_PER_S / rate;
        }
        self.current = self.current.clamp(0.0, self.max);
    }

    /// True if a fresh sprint intent should be accepted.
    pub fn can_sprint(self) -> bool {
        self.current > SPRINT_MIN_STAMINA
    }

    /// Returns true if the sprint stance should auto-cancel this tick.
    pub fn should_auto_cancel_sprint(self) -> bool {
        self.sprinting && self.current <= SPRINT_MIN_STAMINA
    }

    pub fn reset(&mut self) {
        self.current = self.max;
        self.sprinting = false;
    }

    /// **M14J** § "stroke rate consumes M16 swim-stamina" — drain `amount`
    /// from the pool. Clamps to `[0, max]`.
    pub fn consume(&mut self, amount: f32) {
        if !amount.is_finite() || amount <= 0.0 {
            return;
        }
        self.current = (self.current - amount).clamp(0.0, self.max);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drain_over_five_seconds_at_60hz() {
        let mut s = Stamina::full();
        s.sprinting = true;
        for _ in 0..(5 * 60) {
            s.step(60);
        }
        assert!(s.current <= 0.001, "expected drained to 0, got {}", s.current);
    }

    #[test]
    fn recovery_over_three_seconds() {
        let mut s = Stamina {
            current: 0.0,
            max: 1.0,
            sprinting: false,
        };
        for _ in 0..(3 * 60) {
            s.step(60);
        }
        assert!(s.current >= 0.89, "expected ~0.9, got {}", s.current);
    }

    #[test]
    fn auto_cancel_at_zero() {
        let s = Stamina {
            current: 0.0,
            max: 1.0,
            sprinting: true,
        };
        assert!(s.should_auto_cancel_sprint());
    }

    #[test]
    fn cannot_sprint_when_drained() {
        let s = Stamina {
            current: 0.0,
            max: 1.0,
            sprinting: false,
        };
        assert!(!s.can_sprint());
    }

    #[test]
    fn nan_state_resets() {
        let mut s = Stamina {
            current: f32::NAN,
            max: 1.0,
            sprinting: false,
        };
        s.step(60);
        assert_eq!(s.current, 0.0);
    }

    #[test]
    fn invariant_60_120_match_after_one_second() {
        let mut a = Stamina::full();
        a.sprinting = true;
        let mut b = a;
        for _ in 0..60 {
            a.step(60);
        }
        for _ in 0..120 {
            b.step(120);
        }
        assert!((a.current - b.current).abs() < 1e-3);
    }
}
