//! M6: weapon fire modes (Single / Burst-3 / Auto / Pump / Charge / Arc).
//!
//! M1 owned the Semi vs FullAuto enum in `lib.rs`; M6 expands the surface so
//! the launch weapon roster can express realistic firing characteristics:
//!
//! - **Single**: one shot per trigger press (Pistol, Sniper, Rifle in single mode).
//! - **Burst3**: three shots per trigger press within 100 ms (SMG, Rifle in burst mode).
//! - **Auto**: continuous fire while held (Rifle in auto, SMG, GrenadeLauncher in auto).
//! - **Pump**: manual pump-action chambering (Shotgun).
//! - **Charge**: hold to charge, release to fire at proportional damage (Sniper).
//! - **Arc**: arcing projectile with gravity (GrenadeLauncher in arc mode).
//!
//! Per spec § "6 launch weapons with multi-fire modes": each weapon's fire-mode
//! set determines which AdvancedFireMode variants are selectable via
//! `act.player.cycle_fire_mode`.

use serde::{Deserialize, Serialize};

/// Burst-3 inter-shot interval in seconds (spec § "3 projectiles spawn
/// within 100ms"). With 3 shots in 100 ms the cadence is 50 ms between shots.
pub const BURST3_INTER_SHOT_SECONDS: f32 = 0.05;

/// Number of rounds emitted per Burst-3 trigger press.
pub const BURST3_ROUND_COUNT: u32 = 3;

/// Maximum charge time for a Charge-mode weapon (Sniper) in seconds.
pub const SNIPER_CHARGE_MAX_SECONDS: f32 = 0.8;

/// Charge fraction below which the shot misfires (low damage).
pub const SNIPER_MISFIRE_BELOW: f32 = 0.5;

/// Extended fire-mode surface for M6 weapons. Distinct from M1's
/// [`crate::FireMode`] enum so existing M1 rifle state remains binary-compat;
/// M6 weapons that need richer modes use [`AdvancedFireMode`] in addition to
/// the M1 [`crate::FireMode`].
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdvancedFireMode {
    Single = 0,
    Burst3 = 1,
    Auto = 2,
    Pump = 3,
    Charge = 4,
    Arc = 5,
}

impl Default for AdvancedFireMode {
    fn default() -> Self {
        AdvancedFireMode::Single
    }
}

impl AdvancedFireMode {
    pub fn as_str(self) -> &'static str {
        match self {
            AdvancedFireMode::Single => "single",
            AdvancedFireMode::Burst3 => "burst3",
            AdvancedFireMode::Auto => "auto",
            AdvancedFireMode::Pump => "pump",
            AdvancedFireMode::Charge => "charge",
            AdvancedFireMode::Arc => "arc",
        }
    }

    pub fn rounds_per_trigger(self) -> u32 {
        match self {
            AdvancedFireMode::Burst3 => BURST3_ROUND_COUNT,
            _ => 1,
        }
    }

    pub fn requires_charge(self) -> bool {
        matches!(self, AdvancedFireMode::Charge)
    }

    pub fn requires_pump(self) -> bool {
        matches!(self, AdvancedFireMode::Pump)
    }

    pub fn is_continuous(self) -> bool {
        matches!(self, AdvancedFireMode::Auto)
    }
}

/// Cyclable fire-mode set for one weapon. Cycling is deterministic: the next
/// variant in `available` after the current. If `current` is not in
/// `available`, [`cycle_next`] resets to the first entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FireModeSet {
    pub available: Vec<AdvancedFireMode>,
    pub current: AdvancedFireMode,
}

impl FireModeSet {
    pub fn new(available: Vec<AdvancedFireMode>) -> Self {
        let current = available.first().copied().unwrap_or(AdvancedFireMode::Single);
        Self { available, current }
    }

    pub fn cycle_next(&mut self) -> AdvancedFireMode {
        if self.available.is_empty() {
            return self.current;
        }
        let idx = self.available.iter().position(|m| *m == self.current).unwrap_or(0);
        let next = (idx + 1) % self.available.len();
        self.current = self.available[next];
        self.current
    }

    pub fn set_current(&mut self, mode: AdvancedFireMode) -> bool {
        if self.available.contains(&mode) {
            self.current = mode;
            true
        } else {
            false
        }
    }
}

/// Compute charge fraction for a Charge-mode weapon given hold seconds.
/// Returns 0..1 clamped.
#[must_use]
pub fn charge_fraction(hold_seconds: f32) -> f32 {
    if !hold_seconds.is_finite() || hold_seconds <= 0.0 {
        return 0.0;
    }
    (hold_seconds / SNIPER_CHARGE_MAX_SECONDS).clamp(0.0, 1.0)
}

/// Returns the damage multiplier for a charged shot. Below misfire returns
/// 0.2; from misfire to full returns linear interpolation to 1.0.
#[must_use]
pub fn charge_damage_multiplier(charge: f32) -> f32 {
    let c = charge.clamp(0.0, 1.0);
    if c < SNIPER_MISFIRE_BELOW {
        0.2
    } else {
        let t = (c - SNIPER_MISFIRE_BELOW) / (1.0 - SNIPER_MISFIRE_BELOW);
        0.5 + 0.5 * t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn burst3_emits_three_rounds() {
        assert_eq!(AdvancedFireMode::Burst3.rounds_per_trigger(), 3);
    }

    #[test]
    fn cycle_wraps() {
        let mut s = FireModeSet::new(vec![
            AdvancedFireMode::Single,
            AdvancedFireMode::Burst3,
            AdvancedFireMode::Auto,
        ]);
        assert_eq!(s.cycle_next(), AdvancedFireMode::Burst3);
        assert_eq!(s.cycle_next(), AdvancedFireMode::Auto);
        assert_eq!(s.cycle_next(), AdvancedFireMode::Single);
    }

    #[test]
    fn charge_misfire_low() {
        assert!(charge_damage_multiplier(0.3) < 0.5);
    }

    #[test]
    fn charge_full_max() {
        assert!((charge_damage_multiplier(1.0) - 1.0).abs() < 1e-3);
    }

    #[test]
    fn nan_charge_zero_fraction() {
        assert_eq!(charge_fraction(f32::NAN), 0.0);
    }

    #[test]
    fn set_current_rejects_unavailable() {
        let mut s = FireModeSet::new(vec![AdvancedFireMode::Single]);
        assert!(!s.set_current(AdvancedFireMode::Auto));
    }
}
