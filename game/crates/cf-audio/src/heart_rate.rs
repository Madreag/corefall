//! M17 § "Heart-rate audio mix per concussion band".
//!
//! The heart-rate layer mixes LOUDER as the concussion band rises and ducks the
//! ambient bus at Severe+. Bands + thresholds + gain bonuses mirror
//! `cf_actor::concussion::ConcussionBand`; the mirror lives here so cf-audio
//! keeps no dependency on cf-actor.

use serde::{Deserialize, Serialize};

/// Ambient bus multiplier applied while the band ducks ambient (Severe+).
pub const AMBIENT_DUCK_FACTOR: f32 = 0.5;

/// Heartbeat playback BPM at the Clear band (dose 0).
pub const HEART_RATE_BPM_CLEAR: f32 = 70.0;

/// Heartbeat playback BPM at the KO band (dose 100).
pub const HEART_RATE_BPM_KO: f32 = 160.0;

/// Concussion severity bands mirroring `cf_actor::concussion::ConcussionBand`
/// (same thresholds + heart-rate gain bonuses; duplicated to avoid the dep).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeartRateBand {
    Clear = 0,
    Mild = 1,
    Moderate = 2,
    Severe = 3,
    KoImminent = 4,
    Ko = 5,
}

impl HeartRateBand {
    /// Dose (0-100) → band (Mild 20 / Moderate 40 / Severe 60 / KO_Imminent 80 / KO 100).
    pub fn from_dose(dose: f32) -> Self {
        if dose >= 100.0 {
            HeartRateBand::Ko
        } else if dose >= 80.0 {
            HeartRateBand::KoImminent
        } else if dose >= 60.0 {
            HeartRateBand::Severe
        } else if dose >= 40.0 {
            HeartRateBand::Moderate
        } else if dose >= 20.0 {
            HeartRateBand::Mild
        } else {
            HeartRateBand::Clear
        }
    }

    /// Additive heart-rate gain bonus per band (Mild +0.20, Severe +0.60, KO +1.0).
    pub fn heart_rate_gain_bonus(self) -> f32 {
        match self {
            HeartRateBand::Clear => 0.0,
            HeartRateBand::Mild => 0.20,
            HeartRateBand::Moderate => 0.40,
            HeartRateBand::Severe => 0.60,
            HeartRateBand::KoImminent => 0.80,
            HeartRateBand::Ko => 1.0,
        }
    }

    /// True once the band ducks the ambient bus (Severe and above).
    pub fn ducks_ambient(self) -> bool {
        self >= HeartRateBand::Severe
    }

    /// Schema label matching `cf_actor::concussion::ConcussionBand::as_str`.
    pub fn as_str(self) -> &'static str {
        match self {
            HeartRateBand::Clear => "Clear",
            HeartRateBand::Mild => "Mild",
            HeartRateBand::Moderate => "Moderate",
            HeartRateBand::Severe => "Severe",
            HeartRateBand::KoImminent => "KO_Imminent",
            HeartRateBand::Ko => "KO",
        }
    }
}

/// Heart-rate layer gain for `base_gain` at `dose`: louder as the band rises.
#[must_use]
pub fn heart_rate_gain(base_gain: f32, dose: f32) -> f32 {
    let band = HeartRateBand::from_dose(dose);
    base_gain * (1.0 + band.heart_rate_gain_bonus())
}

/// Ambient bus multiplier at `dose`: 1.0 normally, `AMBIENT_DUCK_FACTOR` at Severe+.
#[must_use]
pub fn ambient_duck_factor(dose: f32) -> f32 {
    if HeartRateBand::from_dose(dose).ducks_ambient() {
        AMBIENT_DUCK_FACTOR
    } else {
        1.0
    }
}

/// Heartbeat playback BPM at `dose`, linear from 70 (Clear) to 160 (KO).
#[must_use]
pub fn heart_rate_bpm(dose: f32) -> f32 {
    let t = dose.clamp(0.0, 100.0) / 100.0;
    HEART_RATE_BPM_CLEAR + (HEART_RATE_BPM_KO - HEART_RATE_BPM_CLEAR) * t
}

/// Accessibility caption for the heart-rate layer, surfaced at Severe+ when
/// captions are on. `None` below Severe (nothing pounding to caption).
#[must_use]
pub fn heart_rate_caption(dose: f32) -> Option<&'static str> {
    let band = HeartRateBand::from_dose(dose);
    if band >= HeartRateBand::Ko {
        Some("heartbeat flatlining")
    } else if band >= HeartRateBand::KoImminent {
        Some("heartbeat racing")
    } else if band >= HeartRateBand::Severe {
        Some("heartbeat pounding")
    } else {
        None
    }
}

/// Resolved heart-rate mix snapshot the audio engine reads per frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeartRateMix {
    /// Band the dose resolved to.
    pub band: HeartRateBand,
    /// Heart-rate layer gain after the per-band bonus.
    pub gain: f32,
    /// Ambient bus multiplier (1.0 or ducked).
    pub ambient_duck: f32,
    /// Heartbeat playback BPM driving the loop rate.
    pub bpm: f32,
    /// Accessibility caption (Severe+), else `None`.
    pub caption: Option<&'static str>,
}

impl HeartRateMix {
    /// Resolve the full heart-rate mix for `base_gain` at concussion `dose`.
    #[must_use]
    pub fn resolve(base_gain: f32, dose: f32) -> Self {
        Self {
            band: HeartRateBand::from_dose(dose),
            gain: heart_rate_gain(base_gain, dose),
            ambient_duck: ambient_duck_factor(dose),
            bpm: heart_rate_bpm(dose),
            caption: heart_rate_caption(dose),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dose_maps_to_band() {
        assert_eq!(HeartRateBand::from_dose(0.0), HeartRateBand::Clear);
        assert_eq!(HeartRateBand::from_dose(25.0), HeartRateBand::Mild);
        assert_eq!(HeartRateBand::from_dose(45.0), HeartRateBand::Moderate);
        assert_eq!(HeartRateBand::from_dose(65.0), HeartRateBand::Severe);
        assert_eq!(HeartRateBand::from_dose(85.0), HeartRateBand::KoImminent);
        assert_eq!(HeartRateBand::from_dose(100.0), HeartRateBand::Ko);
    }

    #[test]
    fn mild_gain_is_base_times_1_2() {
        let g = heart_rate_gain(1.0, 25.0);
        assert!((g - 1.2).abs() < 1e-6, "mild gain = {g}");
        let scaled = heart_rate_gain(0.5, 25.0);
        assert!((scaled - 0.6).abs() < 1e-6, "mild gain @0.5 = {scaled}");
    }

    #[test]
    fn severe_gain_is_base_times_1_6() {
        let g = heart_rate_gain(1.0, 65.0);
        assert!((g - 1.6).abs() < 1e-6, "severe gain = {g}");
    }

    #[test]
    fn ducking_inactive_at_mild_active_at_severe() {
        assert!(!HeartRateBand::from_dose(25.0).ducks_ambient());
        assert!((ambient_duck_factor(25.0) - 1.0).abs() < 1e-6);
        assert!(HeartRateBand::from_dose(65.0).ducks_ambient());
        assert!((ambient_duck_factor(65.0) - AMBIENT_DUCK_FACTOR).abs() < 1e-6);
    }

    #[test]
    fn bpm_runs_from_70_clear_to_160_ko() {
        assert!((heart_rate_bpm(0.0) - 70.0).abs() < 1e-6);
        assert!((heart_rate_bpm(100.0) - 160.0).abs() < 1e-6);
        let mid = heart_rate_bpm(50.0);
        assert!(mid > 70.0 && mid < 160.0, "mid bpm = {mid}");
        assert!((heart_rate_bpm(-10.0) - 70.0).abs() < 1e-6);
        assert!((heart_rate_bpm(150.0) - 160.0).abs() < 1e-6);
    }

    #[test]
    fn caption_only_at_severe_and_above() {
        assert_eq!(heart_rate_caption(25.0), None);
        assert_eq!(heart_rate_caption(45.0), None);
        assert_eq!(heart_rate_caption(65.0), Some("heartbeat pounding"));
        assert!(heart_rate_caption(85.0).is_some());
        assert!(heart_rate_caption(100.0).is_some());
    }

    #[test]
    fn resolve_bundles_the_layer() {
        let mix = HeartRateMix::resolve(1.0, 65.0);
        assert_eq!(mix.band, HeartRateBand::Severe);
        assert!((mix.gain - 1.6).abs() < 1e-6);
        assert!((mix.ambient_duck - AMBIENT_DUCK_FACTOR).abs() < 1e-6);
        assert_eq!(mix.caption, Some("heartbeat pounding"));
    }
}
