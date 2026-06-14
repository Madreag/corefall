//! M17 — G-force concussion vignette widget.
//!
//! Maps a [`HudConcussion`] projection onto a screen-blackout alpha and the
//! tunnel-vision flag. The `reduced_g_force_blackout` accessibility toggle caps
//! the alpha at the Moderate band so the screen never fully blacks out.

use crate::hud_model::HudConcussion;

/// Alpha cap applied when `reduced_g_force_blackout` is on (Moderate band).
const REDUCED_CAP: f32 = 0.30;
/// Tunnel-vision kicks in at Severe (vignette fraction ≥ 0.60).
const TUNNELING_FRACTION: f32 = 0.60;

/// Blackout / vignette alpha in `[0, 1]`. With `reduced_g_force_blackout` on,
/// the alpha is capped at the Moderate band fraction (0.30).
#[must_use]
pub fn vignette_alpha(c: &HudConcussion, reduced_g_force_blackout: bool) -> f32 {
    let frac = c.vignette_fraction.clamp(0.0, 1.0);
    if reduced_g_force_blackout {
        frac.min(REDUCED_CAP)
    } else {
        frac
    }
}

/// True once the vignette tunnels (Severe band and above).
#[must_use]
pub fn tunneling_active(c: &HudConcussion) -> bool {
    c.vignette_fraction >= TUNNELING_FRACTION
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_actor::ConcussionBand;

    fn from_dose(dose: f32) -> HudConcussion {
        let band = ConcussionBand::from_dose(dose);
        HudConcussion {
            dose,
            band: band.as_str().to_string(),
            vignette_fraction: band.vignette_fraction(),
            ducks_ambient: band.ducks_ambient(),
        }
    }

    #[test]
    fn human_dose_85_blacks_out_and_tunnels() {
        let c = from_dose(85.0);
        assert_eq!(ConcussionBand::from_dose(85.0), ConcussionBand::KoImminent);
        assert!((c.vignette_fraction - 0.85).abs() < 1e-6);
        assert!(tunneling_active(&c));
        assert!((vignette_alpha(&c, false) - 0.85).abs() < 1e-6);
    }

    #[test]
    fn reduced_toggle_caps_alpha_at_moderate() {
        let c = from_dose(85.0);
        assert!((vignette_alpha(&c, true) - 0.30).abs() < 1e-6);
    }

    #[test]
    fn clear_band_has_no_vignette_or_tunneling() {
        let c = from_dose(0.0);
        assert_eq!(vignette_alpha(&c, false), 0.0);
        assert!(!tunneling_active(&c));
    }

    #[test]
    fn moderate_band_does_not_tunnel() {
        let c = from_dose(45.0);
        assert_eq!(ConcussionBand::from_dose(45.0), ConcussionBand::Moderate);
        assert!(!tunneling_active(&c));
        assert!((vignette_alpha(&c, false) - 0.30).abs() < 1e-6);
    }
}
