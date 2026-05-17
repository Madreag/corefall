//! **M12B** § Per-environment low-pass filter — per-medium filter
//! coefficients used by the spatial-resolution pipeline.
//!
//! Per spec acceptance:
//!
//! ```text
//! Scenario: Per-environment low-pass when listener is underwater
//!   Given the listener submerged in M19F water at world (0, 0)
//!   And a gunshot fires above the waterline at world (0, 2)
//!   When the SFX fires
//!   Then medium_at(midpoint) returns medium="water"
//!   And SpatialEnvelope.medium_filter applies 800 Hz cutoff + 0.6 gain
//!   And the listener hears the characteristic muffled-underwater report
//! ```
//!
//! And:
//!
//! ```text
//! Scenario: Vacuum blocks audio entirely (DR-014 vacuum_no_voice)
//!   ...
//!   Then medium_at returns medium="vacuum"
//!   And SpatialEnvelope.gain == 0
//!   And no waveform reaches the listener's headphones
//! ```
//!
//! The module is pure math: no rodio, no Bevy, no `thread_rng`. Numerical
//! constants are the spec-locked defaults from the acceptance scenarios.

use serde::{Deserialize, Serialize};

/// Standard speed of sound in dry Earth air at STP, in m/s. Used as the
/// default `c` term in the Doppler formula when no medium is supplied.
pub const SPEED_OF_SOUND_AIR_M_PER_S: f32 = 343.0;

/// Speed of sound in fresh water at ~20 °C, in m/s (~1480 m/s commonly
/// cited; rounded for compact replay-event payloads).
pub const SPEED_OF_SOUND_WATER_M_PER_S: f32 = 1480.0;

/// Speed of sound in ammonia at Mimas-ambient conditions, in m/s. Per
/// the M12B acceptance scenario "Doppler atmosphere-corrected speed of
/// sound" — ammonia is ≈ 415 m/s at the listener's environment.
pub const SPEED_OF_SOUND_AMMONIA_M_PER_S: f32 = 415.0;

/// Vacuum: no propagation. Used as the zero short-circuit before
/// Doppler math runs.
pub const SPEED_OF_SOUND_VACUUM_M_PER_S: f32 = 0.0;

/// **M12B** § Audio-propagation medium discriminant. Names match the
/// `audio.spatial_resolved` schema enum so the replay verifier can round-
/// trip the wire string.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Medium {
    /// Standard dry air (Earth-like atmosphere).
    Air,
    /// Water (underwater audio scenario).
    Water,
    /// Smoke / fog / dense particulate atmosphere.
    Smoke,
    /// Ammonia atmosphere (Mimas, per M12B doppler scenario).
    Ammonia,
    /// Vacuum (no sound propagation — DR-014 `vacuum_no_voice`).
    Vacuum,
}

impl Medium {
    /// Snake_case wire identifier used in replay events.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Medium::Air => "air",
            Medium::Water => "water",
            Medium::Smoke => "smoke",
            Medium::Ammonia => "ammonia",
            Medium::Vacuum => "vacuum",
        }
    }

    /// Parse from a snake_case wire string. Returns `None` on unknown
    /// strings — the caller is expected to fall back to [`Medium::Air`]
    /// with a tracing warning.
    #[must_use]
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Medium> {
        Some(match s {
            "air" => Medium::Air,
            "water" => Medium::Water,
            "smoke" => Medium::Smoke,
            "ammonia" => Medium::Ammonia,
            "vacuum" => Medium::Vacuum,
            _ => return None,
        })
    }
}

/// **M12B** § Resolved per-medium filter coefficients. Wired into
/// `SpatialEnvelope.medium_filter`. The cf-app HRIR convolution adapter
/// applies the low-pass + gain at playback time; the sim only carries
/// the coefficient triple.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MediumFilter {
    /// Medium discriminant for replay-event surfacing.
    pub medium: Medium,
    /// Low-pass cutoff frequency in Hz. Cutoff `>= 20_000` is treated as
    /// "no low-pass" by the playback adapter.
    pub cutoff_hz: f32,
    /// Multiplicative gain applied to the audio source — `[0.0, 1.0]`.
    /// Vacuum produces `gain=0.0`, short-circuiting playback.
    pub gain: f32,
    /// Speed of sound in this medium, in m/s. Used as `c` in the Doppler
    /// formula. Vacuum returns `0.0` (the doppler resolver short-circuits
    /// to a gain-zero envelope rather than dividing by zero).
    pub speed_of_sound_m_per_s: f32,
}

impl MediumFilter {
    /// Standard Earth-air filter (no attenuation, no low-pass).
    #[must_use]
    pub const fn air() -> Self {
        Self {
            medium: Medium::Air,
            cutoff_hz: 20_000.0,
            gain: 1.0,
            speed_of_sound_m_per_s: SPEED_OF_SOUND_AIR_M_PER_S,
        }
    }

    /// Underwater filter — 800 Hz cutoff + 60% gain per M12B acceptance.
    #[must_use]
    pub const fn water() -> Self {
        Self {
            medium: Medium::Water,
            cutoff_hz: 800.0,
            gain: 0.60,
            speed_of_sound_m_per_s: SPEED_OF_SOUND_WATER_M_PER_S,
        }
    }

    /// Thick smoke — 1.5 kHz cutoff + 80% gain per spec § Per-environment
    /// low-pass filter.
    #[must_use]
    pub const fn smoke() -> Self {
        Self {
            medium: Medium::Smoke,
            cutoff_hz: 1500.0,
            gain: 0.80,
            speed_of_sound_m_per_s: SPEED_OF_SOUND_AIR_M_PER_S,
        }
    }

    /// Mimas-ammonia atmosphere — -3 dB attenuation + slight high-cut +
    /// medium-corrected speed of sound per spec § "Doppler
    /// atmosphere-corrected speed of sound".
    ///
    /// -3 dB ≈ multiplicative gain of `10^(-3/20) ≈ 0.7079`.
    #[must_use]
    pub fn ammonia() -> Self {
        Self {
            medium: Medium::Ammonia,
            cutoff_hz: 9_000.0,
            gain: 0.7079458_f32,
            speed_of_sound_m_per_s: SPEED_OF_SOUND_AMMONIA_M_PER_S,
        }
    }

    /// Vacuum — DR-014 `vacuum_no_voice`. Gain is zero so the playback
    /// adapter dispatches no waveform.
    #[must_use]
    pub const fn vacuum() -> Self {
        Self {
            medium: Medium::Vacuum,
            cutoff_hz: 0.0,
            gain: 0.0,
            speed_of_sound_m_per_s: SPEED_OF_SOUND_VACUUM_M_PER_S,
        }
    }

    /// Resolve the filter triple for a given medium.
    #[must_use]
    pub fn for_medium(medium: Medium) -> Self {
        match medium {
            Medium::Air => MediumFilter::air(),
            Medium::Water => MediumFilter::water(),
            Medium::Smoke => MediumFilter::smoke(),
            Medium::Ammonia => MediumFilter::ammonia(),
            Medium::Vacuum => MediumFilter::vacuum(),
        }
    }

    /// `true` when the medium blocks audio entirely (vacuum). The
    /// spatial-resolve pipeline short-circuits the envelope to `gain=0`
    /// without divide-by-zero risk in the Doppler math.
    #[must_use]
    pub const fn is_silent(self) -> bool {
        matches!(self.medium, Medium::Vacuum)
    }
}

/// **M12B** § `medium_at(pos)` stub. Real implementation reads from
/// `cf-atmos::medium_at(pos)`; for headless / determinism tests the
/// caller supplies the medium directly. This helper is the canonical
/// "pick a medium between two points" predicate — it samples at the
/// midpoint of `source` ↔ `listener` per the acceptance scenario.
///
/// `samples_at_midpoint` is the live atmosphere probe (e.g.
/// `cf_atmos::probe_medium`). The closure returns `None` when the
/// atmosphere subsystem isn't loaded; we fall back to [`Medium::Air`]
/// with a tracing-friendly `unknown_medium` reason recorded by the
/// envelope.
#[must_use]
pub fn medium_at_midpoint<F>(source: [f32; 2], listener: [f32; 2], samples_at_midpoint: F) -> Medium
where
    F: FnOnce([f32; 2]) -> Option<Medium>,
{
    let mid = [
        (source[0] + listener[0]) * 0.5,
        (source[1] + listener[1]) * 0.5,
    ];
    samples_at_midpoint(mid).unwrap_or(Medium::Air)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn medium_round_trips_through_str() {
        for m in [Medium::Air, Medium::Water, Medium::Smoke, Medium::Ammonia, Medium::Vacuum] {
            assert_eq!(Medium::from_str(m.as_str()), Some(m));
        }
        assert!(Medium::from_str("not_a_medium").is_none());
    }

    #[test]
    fn medium_serde_round_trips() {
        let m = Medium::Water;
        let s = serde_json::to_string(&m).unwrap();
        assert_eq!(s, "\"water\"");
        let back: Medium = serde_json::from_str(&s).unwrap();
        assert_eq!(back, m);
    }

    #[test]
    fn water_filter_matches_spec_acceptance() {
        let f = MediumFilter::for_medium(Medium::Water);
        // Spec: "800 Hz cutoff + 0.6 gain".
        assert!((f.cutoff_hz - 800.0).abs() < 1e-3);
        assert!((f.gain - 0.6).abs() < 1e-3);
        assert!(f.speed_of_sound_m_per_s > SPEED_OF_SOUND_AIR_M_PER_S);
    }

    #[test]
    fn vacuum_filter_silences_audio() {
        let f = MediumFilter::for_medium(Medium::Vacuum);
        assert!(f.gain.abs() < 1e-6);
        assert!(f.is_silent());
        assert_eq!(f.speed_of_sound_m_per_s, 0.0);
    }

    #[test]
    fn smoke_filter_matches_spec_acceptance() {
        let f = MediumFilter::for_medium(Medium::Smoke);
        assert!((f.cutoff_hz - 1500.0).abs() < 1e-3);
        assert!((f.gain - 0.8).abs() < 1e-3);
    }

    #[test]
    fn ammonia_filter_uses_medium_corrected_speed_of_sound() {
        let f = MediumFilter::for_medium(Medium::Ammonia);
        // Spec § "Doppler atmosphere-corrected speed of sound" — ≈ 415 m/s.
        assert!((f.speed_of_sound_m_per_s - SPEED_OF_SOUND_AMMONIA_M_PER_S).abs() < 1e-3);
        // -3 dB ≈ 0.7079.
        assert!((f.gain - 0.7079).abs() < 1e-3);
        assert!(!f.is_silent());
    }

    #[test]
    fn air_filter_does_not_attenuate() {
        let f = MediumFilter::for_medium(Medium::Air);
        assert!((f.gain - 1.0).abs() < 1e-6);
        assert!(f.cutoff_hz >= 20_000.0);
        assert!((f.speed_of_sound_m_per_s - SPEED_OF_SOUND_AIR_M_PER_S).abs() < 1e-6);
    }

    #[test]
    fn medium_at_midpoint_uses_actual_midpoint() {
        let source = [0.0, 0.0];
        let listener = [10.0, 0.0];
        let probed_pos = std::cell::Cell::new([0.0_f32, 0.0_f32]);
        let m = medium_at_midpoint(source, listener, |p| {
            probed_pos.set(p);
            Some(Medium::Water)
        });
        assert_eq!(m, Medium::Water);
        let mid = probed_pos.get();
        assert!((mid[0] - 5.0).abs() < 1e-6);
        assert!((mid[1] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn medium_at_midpoint_falls_back_to_air_when_probe_returns_none() {
        let m = medium_at_midpoint([0.0, 0.0], [10.0, 0.0], |_| None);
        assert_eq!(m, Medium::Air);
    }

    #[test]
    fn medium_filter_round_trips_through_serde() {
        let f = MediumFilter::for_medium(Medium::Smoke);
        let s = serde_json::to_string(&f).unwrap();
        let back: MediumFilter = serde_json::from_str(&s).unwrap();
        assert_eq!(back, f);
    }
}
