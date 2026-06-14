//! M17 — robot overclock (voluntary boost) + downclock (involuntary thermal
//! throttle) + the shared heat accumulator.
//!
//! Overclock trades heat + power for action speed (move / aim / fire / reload).
//! When heat crosses the throttle band the chassis involuntarily downclocks
//! (action speed × 0.5) regardless of overclock intent; sustained heat past the
//! critical band damages modules and risks meltdown.

use serde::{Deserialize, Serialize};

/// Heat fraction (0-1) at which involuntary downclock engages.
pub const THROTTLE_BAND: f32 = 0.70;
/// Heat fraction at which module damage / meltdown risk begins.
pub const CRITICAL_BAND: f32 = 0.90;
/// Heat fraction that triggers meltdown.
pub const MELTDOWN_BAND: f32 = 1.0;

/// Action-speed multiplier applied while involuntarily throttled.
pub const DOWNCLOCK_ACTION_SPEED: f32 = 0.5;

/// Per-tier action-speed multiplier when overclocking (tier 1-3).
pub fn overclock_action_speed(tier: u8) -> f32 {
    match tier {
        0 => 1.0,
        1 => 1.15,
        2 => 1.30,
        3 => 1.50,
        _ => 1.50,
    }
}

/// Heat added per second per overclock tier.
pub fn overclock_heat_per_s(tier: u8) -> f32 {
    match tier {
        0 => 0.0,
        1 => 0.04,
        2 => 0.08,
        3 => 0.14,
        _ => 0.14,
    }
}

/// Power (kWh) drained per second per overclock tier (spec § "+2 kWh/s").
pub fn overclock_power_drain_per_s(tier: u8) -> f32 {
    match tier {
        0 => 0.0,
        1 => 1.0,
        2 => 2.0,
        3 => 3.0,
        _ => 3.0,
    }
}

/// Active overclock / downclock state for one actor.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OverclockState {
    /// Requested overclock tier (0 = off).
    pub tier: u8,
    /// Ticks the actor has held the current overclock tier (sustained-heat KO).
    pub sustained_ticks: u32,
    /// True while heat forces an involuntary downclock.
    pub throttled: bool,
}

impl OverclockState {
    pub fn is_boosting(&self) -> bool {
        self.tier > 0 && !self.throttled
    }
}

/// The effective action-speed multiplier given overclock tier + heat throttle.
/// Throttle (involuntary) always wins over boost.
pub fn effective_action_speed(state: &OverclockState, heat: f32) -> f32 {
    if heat >= THROTTLE_BAND {
        DOWNCLOCK_ACTION_SPEED
    } else {
        overclock_action_speed(state.tier)
    }
}

/// Thermal band an actor is in (for HUD + AI doctrine + events).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThermalBand {
    Nominal,
    Throttle,
    Critical,
    Meltdown,
}

impl ThermalBand {
    pub fn as_str(self) -> &'static str {
        match self {
            ThermalBand::Nominal => "nominal",
            ThermalBand::Throttle => "throttle",
            ThermalBand::Critical => "critical",
            ThermalBand::Meltdown => "meltdown",
        }
    }

    pub fn from_heat(heat: f32) -> Self {
        if heat >= MELTDOWN_BAND {
            ThermalBand::Meltdown
        } else if heat >= CRITICAL_BAND {
            ThermalBand::Critical
        } else if heat >= THROTTLE_BAND {
            ThermalBand::Throttle
        } else {
            ThermalBand::Nominal
        }
    }
}

/// Passive heat dissipation per second. Vacuum kills convective cooling
/// (only radiation), so robots cook in vacuum unless actively cooled.
pub fn heat_dissipation_per_s(in_vacuum: bool, active_cooling: bool) -> f32 {
    let base = if in_vacuum { 0.005 } else { 0.03 };
    if active_cooling {
        base + 0.05
    } else {
        base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overclock_tier_2_is_1_3x() {
        assert!((overclock_action_speed(2) - 1.30).abs() < 1e-6);
    }

    #[test]
    fn throttle_overrides_boost() {
        let s = OverclockState {
            tier: 2,
            sustained_ticks: 0,
            throttled: false,
        };
        // Below the throttle band: boost applies.
        assert!((effective_action_speed(&s, 0.5) - 1.30).abs() < 1e-6);
        // At the throttle band: involuntary downclock wins.
        assert!((effective_action_speed(&s, 0.70) - DOWNCLOCK_ACTION_SPEED).abs() < 1e-6);
    }

    #[test]
    fn thermal_bands_from_heat() {
        assert_eq!(ThermalBand::from_heat(0.5), ThermalBand::Nominal);
        assert_eq!(ThermalBand::from_heat(0.70), ThermalBand::Throttle);
        assert_eq!(ThermalBand::from_heat(0.90), ThermalBand::Critical);
        assert_eq!(ThermalBand::from_heat(1.0), ThermalBand::Meltdown);
    }

    #[test]
    fn vacuum_reduces_dissipation() {
        assert!(heat_dissipation_per_s(true, false) < heat_dissipation_per_s(false, false));
        assert!(heat_dissipation_per_s(true, true) > heat_dissipation_per_s(true, false));
    }
}
