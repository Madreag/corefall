//! M17 — G-Force / concussion band model.
//!
//! Turns a `concussion_dose` (0-100) into a [`ConcussionBand`], the HUD
//! vignette / blackout fraction, and the heart-rate audio mix gain. Per-origin
//! capping (human = full curve to KO, android = capped at Moderate, robot =
//! never) lives here so the combat emitter + HUD + accessibility path share one
//! source of truth.

use serde::{Deserialize, Serialize};

use crate::origin::Origin;

/// Concussion severity bands (spec § "G-Force HUD blackout").
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConcussionBand {
    Clear = 0,
    Mild = 1,
    Moderate = 2,
    Severe = 3,
    KoImminent = 4,
    Ko = 5,
}

impl ConcussionBand {
    /// Schema label (matches concussion.band_changed `to_band` enum).
    pub fn as_str(self) -> &'static str {
        match self {
            ConcussionBand::Clear => "Clear",
            ConcussionBand::Mild => "Mild",
            ConcussionBand::Moderate => "Moderate",
            ConcussionBand::Severe => "Severe",
            ConcussionBand::KoImminent => "KO_Imminent",
            ConcussionBand::Ko => "KO",
        }
    }

    /// Dose → band per the locked thresholds
    /// (Mild 20 / Moderate 40 / Severe 60 / KO_Imminent 80 / KO 100).
    pub fn from_dose(dose: f32) -> Self {
        if dose >= 100.0 {
            ConcussionBand::Ko
        } else if dose >= 80.0 {
            ConcussionBand::KoImminent
        } else if dose >= 60.0 {
            ConcussionBand::Severe
        } else if dose >= 40.0 {
            ConcussionBand::Moderate
        } else if dose >= 20.0 {
            ConcussionBand::Mild
        } else {
            ConcussionBand::Clear
        }
    }

    /// HUD vignette / blackout fraction (0.0-1.0) per spec § band table.
    pub fn vignette_fraction(self) -> f32 {
        match self {
            ConcussionBand::Clear => 0.0,
            ConcussionBand::Mild => 0.10,
            ConcussionBand::Moderate => 0.30,
            ConcussionBand::Severe => 0.60,
            ConcussionBand::KoImminent => 0.85,
            ConcussionBand::Ko => 1.0,
        }
    }

    /// Heart-rate audio mix gain bonus (additive fraction) per band — Mild
    /// +20%, Severe +60% (spec § "Heart-rate audio mix per concussion band").
    pub fn heart_rate_gain_bonus(self) -> f32 {
        match self {
            ConcussionBand::Clear => 0.0,
            ConcussionBand::Mild => 0.20,
            ConcussionBand::Moderate => 0.40,
            ConcussionBand::Severe => 0.60,
            ConcussionBand::KoImminent => 0.80,
            ConcussionBand::Ko => 1.0,
        }
    }

    /// True once the band ducks ambient audio under the heart-rate layer
    /// (Severe and above).
    pub fn ducks_ambient(self) -> bool {
        self >= ConcussionBand::Severe
    }

    /// The dose at this band's floor (used when a dose is capped to a band).
    pub fn dose_floor(self) -> f32 {
        match self {
            ConcussionBand::Clear => 0.0,
            ConcussionBand::Mild => 20.0,
            ConcussionBand::Moderate => 40.0,
            ConcussionBand::Severe => 60.0,
            ConcussionBand::KoImminent => 80.0,
            ConcussionBand::Ko => 100.0,
        }
    }
}

/// The per-origin band ceiling. Human reaches KO; android caps at Moderate
/// (synthetic side resists); robots never accumulate a band (0). Honors the
/// `reduced_g_force_blackout` accessibility toggle (caps everyone at Moderate).
pub fn band_cap(origin: Origin, reduced_g_force_blackout: bool) -> ConcussionBand {
    let base = match origin {
        Origin::Robot | Origin::Drone | Origin::Crystalline => ConcussionBand::Clear,
        Origin::Android => ConcussionBand::Moderate,
        _ => ConcussionBand::Ko,
    };
    if reduced_g_force_blackout && base > ConcussionBand::Moderate {
        ConcussionBand::Moderate
    } else {
        base
    }
}

/// Apply the per-origin susceptibility multiplier + band cap to a raw dose,
/// returning `(effective_dose, capped_band)`. The dose itself is scaled by
/// susceptibility so an android accrues half the human dose for the same hit.
pub fn effective_band(
    raw_dose: f32,
    susceptibility: f32,
    origin: Origin,
    reduced_g_force_blackout: bool,
) -> (f32, ConcussionBand) {
    let scaled = (raw_dose * susceptibility).clamp(0.0, 100.0);
    let cap = band_cap(origin, reduced_g_force_blackout);
    let raw_band = ConcussionBand::from_dose(scaled);
    let band = if raw_band > cap { cap } else { raw_band };
    // When the band is capped, clamp the reported dose to the cap's floor so
    // the HUD vignette never exceeds the capped band's fraction.
    let dose = if raw_band > cap { cap.dose_floor() } else { scaled };
    (dose, band)
}

/// KO blackout duration (seconds) for a dose at the KO threshold — 5-10s,
/// scaling with how far past 100 the raw (pre-cap) dose pushed.
pub fn ko_duration_seconds(raw_dose: f32) -> f32 {
    let over = (raw_dose - 100.0).clamp(0.0, 100.0);
    (5.0 + over * 0.05).clamp(5.0, 10.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dose_thresholds_map_to_bands() {
        assert_eq!(ConcussionBand::from_dose(0.0), ConcussionBand::Clear);
        assert_eq!(ConcussionBand::from_dose(25.0), ConcussionBand::Mild);
        assert_eq!(ConcussionBand::from_dose(85.0), ConcussionBand::KoImminent);
        assert_eq!(ConcussionBand::from_dose(100.0), ConcussionBand::Ko);
    }

    #[test]
    fn human_reaches_85_pct_vignette_android_caps_at_60() {
        // Scenario: human dose 85 → KO_Imminent → 85% vignette + tunneling.
        let (_d, band) = effective_band(85.0, 1.0, Origin::Human, false);
        assert_eq!(band, ConcussionBand::KoImminent);
        assert!((band.vignette_fraction() - 0.85).abs() < 1e-6);

        // Android: same impulse capped at Moderate (≤60% → 30% vignette).
        let (_d, aband) = effective_band(85.0, 0.5, Origin::Android, false);
        assert_eq!(aband, ConcussionBand::Moderate);
        assert!(aband <= ConcussionBand::Moderate);
    }

    #[test]
    fn robot_never_accumulates_a_band() {
        let (dose, band) = effective_band(100.0, 0.0, Origin::Robot, false);
        assert_eq!(band, ConcussionBand::Clear);
        assert_eq!(dose, 0.0);
    }

    #[test]
    fn reduced_blackout_caps_humans_at_moderate() {
        let (_d, band) = effective_band(90.0, 1.0, Origin::Human, true);
        assert_eq!(band, ConcussionBand::Moderate);
    }

    #[test]
    fn heart_rate_gain_and_duck_per_band() {
        assert!((ConcussionBand::Mild.heart_rate_gain_bonus() - 0.20).abs() < 1e-6);
        assert!((ConcussionBand::Severe.heart_rate_gain_bonus() - 0.60).abs() < 1e-6);
        assert!(!ConcussionBand::Mild.ducks_ambient());
        assert!(ConcussionBand::Severe.ducks_ambient());
    }
}
