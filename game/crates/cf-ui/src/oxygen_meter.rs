//! M16 § Swim oxygen meter HUD widget.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Severity band for the oxygen meter color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OxygenBand {
    Full,
    Low,
    Critical,
    Drowning,
}

#[derive(Debug, Clone, Default, Resource, Serialize, Deserialize)]
pub struct OxygenMeterState {
    /// Oxygen reservoir [0, max].
    pub oxygen_seconds: f32,
    pub oxygen_max_seconds: f32,
    /// True when the actor is currently submerged.
    pub submerged: bool,
    /// True when the player is actively drowning (oxygen at 0).
    pub drowning: bool,
}

impl OxygenMeterState {
    pub fn refresh(&mut self, oxygen_seconds: f32, oxygen_max_seconds: f32, submerged: bool, drowning: bool) {
        self.oxygen_seconds = oxygen_seconds.max(0.0);
        self.oxygen_max_seconds = oxygen_max_seconds.max(1.0);
        self.submerged = submerged;
        self.drowning = drowning;
    }

    #[must_use]
    pub fn band(&self) -> OxygenBand {
        if self.drowning {
            OxygenBand::Drowning
        } else {
            let pct = self.oxygen_seconds / self.oxygen_max_seconds.max(1.0);
            if pct < 0.15 {
                OxygenBand::Critical
            } else if pct < 0.40 {
                OxygenBand::Low
            } else {
                OxygenBand::Full
            }
        }
    }

    /// One-line summary for accessibility readouts.
    #[must_use]
    pub fn summary_line(&self) -> String {
        if !self.submerged {
            format!("O2 {:.0}/{:.0}", self.oxygen_seconds, self.oxygen_max_seconds)
        } else if self.drowning {
            "DROWNING — surface NOW".to_string()
        } else {
            format!("O2 {:.0} s left", self.oxygen_seconds)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_oxygen_is_full_band() {
        let mut state = OxygenMeterState::default();
        state.refresh(30.0, 30.0, true, false);
        assert_eq!(state.band(), OxygenBand::Full);
    }

    #[test]
    fn drowning_overrides_band() {
        let mut state = OxygenMeterState::default();
        state.refresh(0.0, 30.0, true, true);
        assert_eq!(state.band(), OxygenBand::Drowning);
    }

    #[test]
    fn critical_band_under_15_percent() {
        let mut state = OxygenMeterState::default();
        state.refresh(3.0, 30.0, true, false);
        assert_eq!(state.band(), OxygenBand::Critical);
    }

    #[test]
    fn low_band_under_40_percent() {
        let mut state = OxygenMeterState::default();
        state.refresh(10.0, 30.0, true, false);
        assert_eq!(state.band(), OxygenBand::Low);
    }
}
