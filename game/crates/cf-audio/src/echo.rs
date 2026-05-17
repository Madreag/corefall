//! **M12B** § Per-material echo response. The cf-material registry adds
//! four acoustic fields per material row (`echo_coefficient`, `decay_band`,
//! `acoustic_transmission_loss_db`, `low_pass_cutoff_hz`); this module
//! consumes the first two to compute a per-surface echo descriptor and the
//! per-reflection delay used by the reverb module.
//!
//! Per spec acceptance:
//!
//! ```text
//! Scenario: Cloth-lined room dampens echo to near-zero
//!   ...
//!   Then ReverbProfile.decay_coefficient ≤ 0.15
//!   And ReverbProfile.wet_dry_mix ≤ 0.25 (mostly dry)
//!   And the SFX sounds nearly anechoic to the listener
//! ```
//!
//! The module is pure math; deterministic surface, no DSP. The DSP lives
//! in `cf-app::audio_backend::reverb_send`.

use serde::{Deserialize, Serialize};

/// **M12B** § Canonical decay-band tilt labels per spec table.
///
/// - `bright` — high-frequency-favored (concrete, rock).
/// - `bright_ringing` — bright + sustained sustained ring (steel).
/// - `bright_short` — short, sharp, bright tail (glass, ice).
/// - `warm_mid` — mid-band-favored (wood, dirt).
/// - `warm_low` — low-band-favored (water).
/// - `dampened` — strongly absorbed (cloth, cardboard, foam_insulation).
/// - `anechoic` — practically no reflection (snow, sand, air).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum DecayBand {
    Bright,
    BrightRinging,
    BrightShort,
    WarmMid,
    WarmLow,
    Dampened,
    Anechoic,
}

impl DecayBand {
    /// Snake_case wire identifier (matches `cf-material` decay_band strings).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            DecayBand::Bright => "bright",
            DecayBand::BrightRinging => "bright_ringing",
            DecayBand::BrightShort => "bright_short",
            DecayBand::WarmMid => "warm_mid",
            DecayBand::WarmLow => "warm_low",
            DecayBand::Dampened => "dampened",
            DecayBand::Anechoic => "anechoic",
        }
    }

    /// Parse from a snake_case wire string. Unknown strings fall back to
    /// [`DecayBand::WarmMid`] (the canonical "default dirt" tilt).
    #[must_use]
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> DecayBand {
        match s {
            "bright" => DecayBand::Bright,
            "bright_ringing" => DecayBand::BrightRinging,
            "bright_short" => DecayBand::BrightShort,
            "warm_mid" => DecayBand::WarmMid,
            "warm_low" => DecayBand::WarmLow,
            "dampened" => DecayBand::Dampened,
            "anechoic" => DecayBand::Anechoic,
            _ => DecayBand::WarmMid,
        }
    }

    /// Coarse high/mid/low spectral-tilt summary. Used by the reverb send
    /// module to pick an IR with the correct spectral character when
    /// multiple IRs match the volume + decay coefficient.
    #[must_use]
    pub fn tilt_label(self) -> &'static str {
        match self {
            DecayBand::Bright | DecayBand::BrightRinging | DecayBand::BrightShort => "high",
            DecayBand::WarmMid => "mid",
            DecayBand::WarmLow => "low",
            DecayBand::Dampened | DecayBand::Anechoic => "neutral",
        }
    }

    /// `true` when the band is in the spec-locked "dampened" family
    /// (cloth, foam, anechoic). Drives the "cloth-lined room dampens echo
    /// to near-zero" acceptance scenario.
    #[must_use]
    pub const fn is_dampened(self) -> bool {
        matches!(self, DecayBand::Dampened | DecayBand::Anechoic)
    }
}

/// **M12B** § Per-material echo response — first-reflection amplitude +
/// spectral tilt. Returned by [`echo_response_for`] given a material's
/// canonical acoustic fields.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EchoResponse {
    /// 0..=1 first-reflection amplitude.
    pub coefficient: f32,
    /// Spectral-tilt label.
    pub band: DecayBand,
    /// First-reflection delay floor in milliseconds. The room-level
    /// reverb profile adds the volume-dependent term; this is the
    /// per-surface contribution (~the time the wavefront takes to
    /// bounce off this surface and re-arrive at the listener).
    pub first_reflection_delay_ms: f32,
}

impl EchoResponse {
    /// `true` when the surface is effectively anechoic (cloth, foam,
    /// snow). Spec § "Cloth-lined room dampens echo to near-zero".
    #[must_use]
    pub fn is_nearly_anechoic(&self) -> bool {
        self.coefficient <= 0.15 || self.band.is_dampened()
    }
}

/// **M12B** § Resolve the per-material echo response.
///
/// `echo_coefficient` and `decay_band` come from `cf-material::registry`
/// (defaults to the "dirt" row if missing). Returns a fully populated
/// [`EchoResponse`].
#[must_use]
pub fn echo_response_for(echo_coefficient: f32, decay_band: &str) -> EchoResponse {
    let coefficient = echo_coefficient.clamp(0.0, 1.0);
    let band = DecayBand::from_str(decay_band);
    // First-reflection delay is band-dependent: bright surfaces ring back
    // fast (~3 ms), dampened surfaces blur the first reflection further.
    let first_reflection_delay_ms = match band {
        DecayBand::Bright | DecayBand::BrightRinging | DecayBand::BrightShort => 3.0,
        DecayBand::WarmMid => 6.0,
        DecayBand::WarmLow => 8.0,
        DecayBand::Dampened => 10.0,
        DecayBand::Anechoic => 12.0,
    };
    EchoResponse {
        coefficient,
        band,
        first_reflection_delay_ms,
    }
}

/// **M12B** § Surface-area-weighted echo coefficient. Used by the
/// reverb-profile derivation: the room's reflective character is the
/// weighted mean of every wall surface's echo coefficient, where the
/// weight is the surface area of that wall.
///
/// Per spec § "Per-room reverb profile derivation":
///
/// ```text
/// decay_coefficient: weighted_mean(wall_material.echo_coefficient, by_surface_area)
/// ```
///
/// Returns `0.0` when the input slice is empty (open outdoor — `wet_dry_mix`
/// goes dry in the caller).
#[must_use]
pub fn weighted_mean_coefficient(samples: &[(f32, f32)]) -> f32 {
    let total_area: f32 = samples.iter().map(|(_, area)| area.max(0.0)).sum();
    if total_area <= 0.0 {
        return 0.0;
    }
    let weighted: f32 = samples
        .iter()
        .map(|(coef, area)| coef.clamp(0.0, 1.0) * area.max(0.0))
        .sum();
    (weighted / total_area).clamp(0.0, 1.0)
}

/// **M12B** § Dominant decay-band across multiple wall surfaces. Picks the
/// band that owns the largest weighted surface area. Ties resolve in the
/// canonical-tilt order: `bright_ringing > bright > bright_short > warm_mid >
/// warm_low > dampened > anechoic` so two runs with identical inputs pick
/// the same band.
#[must_use]
pub fn dominant_band(samples: &[(DecayBand, f32)]) -> DecayBand {
    if samples.is_empty() {
        return DecayBand::WarmMid;
    }
    let mut weights: [f32; 7] = [0.0; 7];
    for (band, area) in samples {
        let idx = match band {
            DecayBand::Bright => 1,
            DecayBand::BrightRinging => 0,
            DecayBand::BrightShort => 2,
            DecayBand::WarmMid => 3,
            DecayBand::WarmLow => 4,
            DecayBand::Dampened => 5,
            DecayBand::Anechoic => 6,
        };
        weights[idx] += area.max(0.0);
    }
    // Deterministic tie-break: prefer lower indices (canonical-tilt order).
    let mut best_idx = 0usize;
    let mut best_w = weights[0];
    for (i, w) in weights.iter().enumerate().skip(1) {
        if *w > best_w {
            best_idx = i;
            best_w = *w;
        }
    }
    match best_idx {
        0 => DecayBand::BrightRinging,
        1 => DecayBand::Bright,
        2 => DecayBand::BrightShort,
        3 => DecayBand::WarmMid,
        4 => DecayBand::WarmLow,
        5 => DecayBand::Dampened,
        _ => DecayBand::Anechoic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decay_band_round_trips_through_str() {
        for band in [
            DecayBand::Bright,
            DecayBand::BrightRinging,
            DecayBand::BrightShort,
            DecayBand::WarmMid,
            DecayBand::WarmLow,
            DecayBand::Dampened,
            DecayBand::Anechoic,
        ] {
            assert_eq!(DecayBand::from_str(band.as_str()), band);
        }
    }

    #[test]
    fn unknown_decay_band_falls_back_to_warm_mid() {
        assert_eq!(DecayBand::from_str("garbage"), DecayBand::WarmMid);
    }

    #[test]
    fn echo_response_clamps_coefficient() {
        let resp = echo_response_for(5.0, "bright");
        assert!((resp.coefficient - 1.0).abs() < 1e-6);
        let resp = echo_response_for(-0.5, "bright");
        assert!(resp.coefficient.abs() < 1e-6);
    }

    #[test]
    fn echo_response_concrete_matches_spec_table() {
        let resp = echo_response_for(0.85, "bright");
        assert_eq!(resp.band, DecayBand::Bright);
        assert!((resp.coefficient - 0.85).abs() < 1e-6);
        assert!((resp.first_reflection_delay_ms - 3.0).abs() < 1e-6);
        assert!(!resp.is_nearly_anechoic());
    }

    #[test]
    fn echo_response_cloth_is_nearly_anechoic() {
        let resp = echo_response_for(0.08, "dampened");
        assert!(resp.is_nearly_anechoic());
    }

    #[test]
    fn echo_response_snow_is_nearly_anechoic() {
        let resp = echo_response_for(0.05, "anechoic");
        assert!(resp.is_nearly_anechoic());
    }

    #[test]
    fn weighted_mean_coefficient_returns_zero_for_empty() {
        assert!(weighted_mean_coefficient(&[]).abs() < 1e-6);
    }

    #[test]
    fn weighted_mean_coefficient_uses_areas_as_weights() {
        // 75% concrete wall (echo=0.85), 25% cloth wall (echo=0.08).
        let m = weighted_mean_coefficient(&[(0.85, 75.0), (0.08, 25.0)]);
        // (0.85 * 75 + 0.08 * 25) / 100 = (63.75 + 2.0) / 100 = 0.6575
        assert!((m - 0.6575).abs() < 1e-4);
    }

    #[test]
    fn weighted_mean_coefficient_clamps_inputs() {
        let m = weighted_mean_coefficient(&[(5.0, 50.0), (-3.0, 50.0)]);
        // (1.0 * 50 + 0.0 * 50) / 100 = 0.5
        assert!((m - 0.5).abs() < 1e-4);
    }

    #[test]
    fn dominant_band_returns_largest_area() {
        let band = dominant_band(&[
            (DecayBand::Bright, 75.0),
            (DecayBand::Dampened, 25.0),
        ]);
        assert_eq!(band, DecayBand::Bright);
    }

    #[test]
    fn dominant_band_ties_break_canonically() {
        let band = dominant_band(&[
            (DecayBand::BrightRinging, 50.0),
            (DecayBand::Bright, 50.0),
        ]);
        // First in canonical order wins on tie.
        assert_eq!(band, DecayBand::BrightRinging);
    }

    #[test]
    fn dominant_band_returns_warm_mid_for_empty() {
        assert_eq!(dominant_band(&[]), DecayBand::WarmMid);
    }

    #[test]
    fn dominant_band_for_cloth_warehouse() {
        // 80% cloth + 20% wood — cloth wins.
        let band = dominant_band(&[
            (DecayBand::Dampened, 80.0),
            (DecayBand::WarmMid, 20.0),
        ]);
        assert_eq!(band, DecayBand::Dampened);
    }

    #[test]
    fn tilt_label_categorises_bright_family_as_high() {
        assert_eq!(DecayBand::Bright.tilt_label(), "high");
        assert_eq!(DecayBand::BrightRinging.tilt_label(), "high");
        assert_eq!(DecayBand::BrightShort.tilt_label(), "high");
    }
}
