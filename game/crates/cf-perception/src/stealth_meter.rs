//! M6: per-actor stealth meter (detection probability 0..1).
//!
//! The stealth meter is the smoothed integral of "how visible am I right now?".
//! When low (< 0.3) the actor can stealth-kill; when high (> 0.5) the HUD
//! flashes the "Spotted" caption. Visualization is the eye icon next to the
//! reticle.

use serde::{Deserialize, Serialize};

/// M6 § "Stealth kill instant-kill from behind": only available when meter < 0.3
/// (= 30%).
pub const STEALTH_KILL_THRESHOLD: f32 = 0.3;

/// M6 § "When detection > 50%: caption 'Spotted'": HUD threshold.
pub const SPOTTED_CAPTION_THRESHOLD: f32 = 0.5;

/// Per-actor visibility profile feeding the stealth meter integrator.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StealthVisibility {
    /// 0..1 instantaneous visibility from sight kernel (cone × range × occlusion).
    pub instantaneous: f32,
    /// 0..1 noise contribution from hearing (gunshots / footsteps).
    pub noise: f32,
    /// True when crouched (-30% multiplier).
    pub crouched: bool,
    /// True when prone (-50% multiplier).
    pub prone: bool,
    /// True when stationary (-20% multiplier).
    pub stationary: bool,
}

impl Default for StealthVisibility {
    fn default() -> Self {
        Self {
            instantaneous: 0.0,
            noise: 0.0,
            crouched: false,
            prone: false,
            stationary: false,
        }
    }
}

impl StealthVisibility {
    /// Effective visibility score after stance multipliers.
    pub fn effective(self) -> f32 {
        if !self.instantaneous.is_finite() || !self.noise.is_finite() {
            return 0.0;
        }
        let base = (self.instantaneous + self.noise).clamp(0.0, 1.0);
        let mut k = 1.0;
        if self.crouched {
            k *= 0.7;
        }
        if self.prone {
            k *= 0.5;
        }
        if self.stationary {
            k *= 0.8;
        }
        (base * k).clamp(0.0, 1.0)
    }
}

/// Smoothed stealth meter (0..1). Integrator with separate rise / fall rates
/// so being seen briefly registers but doesn't lock the actor permanently into
/// "spotted" state.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StealthMeter {
    /// Current value (0..1).
    pub value: f32,
    /// Smoothing factor toward the target when the target is rising. Per tick.
    pub rise_per_tick: f32,
    /// Smoothing factor toward the target when the target is falling. Per tick.
    pub fall_per_tick: f32,
}

impl Default for StealthMeter {
    fn default() -> Self {
        Self {
            value: 0.0,
            rise_per_tick: 0.05,
            fall_per_tick: 0.02,
        }
    }
}

impl StealthMeter {
    /// Step the meter toward the new visibility target. Returns the new value.
    pub fn step_toward(&mut self, target: f32) -> f32 {
        let t = if target.is_finite() {
            target.clamp(0.0, 1.0)
        } else {
            self.value
        };
        if t > self.value {
            self.value = (self.value + self.rise_per_tick).min(t);
        } else if t < self.value {
            self.value = (self.value - self.fall_per_tick).max(t);
        }
        self.value
    }

    /// True when the actor can execute a stealth kill.
    pub fn can_stealth_kill(self) -> bool {
        self.value < STEALTH_KILL_THRESHOLD
    }

    /// True when the HUD should display the "Spotted" caption.
    pub fn is_spotted(self) -> bool {
        self.value >= SPOTTED_CAPTION_THRESHOLD
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meter_rises_toward_high_visibility() {
        let mut m = StealthMeter::default();
        for _ in 0..20 {
            m.step_toward(1.0);
        }
        assert!(m.value >= 0.5);
    }

    #[test]
    fn meter_falls_when_target_lower() {
        let mut m = StealthMeter {
            value: 0.8,
            ..StealthMeter::default()
        };
        for _ in 0..30 {
            m.step_toward(0.0);
        }
        assert!(m.value < 0.5);
    }

    #[test]
    fn stealth_kill_below_threshold() {
        let m = StealthMeter {
            value: 0.2,
            ..StealthMeter::default()
        };
        assert!(m.can_stealth_kill());
    }

    #[test]
    fn spotted_above_threshold() {
        let m = StealthMeter {
            value: 0.6,
            ..StealthMeter::default()
        };
        assert!(m.is_spotted());
    }

    #[test]
    fn crouch_reduces_visibility() {
        let v = StealthVisibility {
            instantaneous: 0.6,
            noise: 0.0,
            crouched: true,
            prone: false,
            stationary: false,
        };
        assert!(v.effective() < 0.6);
    }

    #[test]
    fn prone_reduces_more_than_crouch() {
        let c = StealthVisibility {
            instantaneous: 1.0,
            noise: 0.0,
            crouched: true,
            prone: false,
            stationary: false,
        };
        let p = StealthVisibility {
            crouched: false,
            prone: true,
            ..c
        };
        assert!(p.effective() < c.effective());
    }
}
