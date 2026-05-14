//! M6: centralized perception kernel.
//!
//! Replaces M2's per-AI perception code path with a unified, deterministic
//! kernel that handles sight (line-of-sight against terrain), hearing
//! (distance-attenuated loudness), occlusion (walls partially block sound),
//! footsteps (per-surface loudness modifier), and the per-actor stealth meter.
//!
//! Every public function in this crate is pure: it takes input state and
//! returns derived signals. The cf-control engine threads the signals into
//! per-tick replay events (`perception.*`).
//!
//! All distance and time units are world units / seconds; the engine adapts
//! to its configured tick rate via `tick_rate_hz` parameters.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::needless_pass_by_value
)]

use serde::{Deserialize, Serialize};

pub mod footstep;
pub mod hearing;
pub mod occlusion;
pub mod sight;
pub mod stealth_meter;

pub use footstep::{footstep_loudness, FootstepEmission, SurfaceKind};
pub use hearing::{distance_attenuation, hearing_reaction, HearingProbe};
pub use occlusion::{apply_occlusion, OcclusionResult};
pub use sight::{compute_sightline, SightCheck, SightResult};
pub use stealth_meter::{StealthMeter, StealthVisibility};

/// Locked at M6: the baseline alarm radius scalar (per CCCP `HDFirearm.cpp:948`
/// and M1 spec § Sim numbers locked).
///
/// Used as `loudness_radius = ALARM_RADIUS_BASE * (damage / 10).clamp(1, 3)`.
pub const ALARM_RADIUS_BASE: f32 = 480.0;

/// M6 § "Suppressor effect reduces alarm propagation": loudness × 0.4 when a
/// suppressor is attached. Mirrors the same number in
/// [`cf_equipment::suppressor::SUPPRESSOR_LOUDNESS_FACTOR`].
pub const SUPPRESSOR_LOUDNESS_FACTOR: f32 = 0.4;

/// M6 § "Echo behavior in enclosed areas": when an emitter is inside a
/// reverberant volume, the perceived loudness is multiplied by this factor
/// for receivers also inside the same volume (simple model — full IRs land in
/// the cf-audio M14 pass).
pub const ECHO_BOOST_ENCLOSED: f32 = 1.25;

/// Categorical loudness band returned by [`distance_attenuation`]. Discrete
/// bands keep AI doctrine decisions deterministic across CPUs.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoudnessBand {
    Inaudible = 0,
    Faint = 1,
    Moderate = 2,
    Loud = 3,
    Deafening = 4,
}

impl LoudnessBand {
    /// Convert a normalized 0..1 loudness signal into the discrete band the
    /// AI doctrine consumes. Cutoffs are spec-locked from M6 § perception kernel.
    pub fn from_intensity(intensity: f32) -> Self {
        if !intensity.is_finite() || intensity <= 0.05 {
            LoudnessBand::Inaudible
        } else if intensity < 0.2 {
            LoudnessBand::Faint
        } else if intensity < 0.5 {
            LoudnessBand::Moderate
        } else if intensity < 0.85 {
            LoudnessBand::Loud
        } else {
            LoudnessBand::Deafening
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            LoudnessBand::Inaudible => "inaudible",
            LoudnessBand::Faint => "faint",
            LoudnessBand::Moderate => "moderate",
            LoudnessBand::Loud => "loud",
            LoudnessBand::Deafening => "deafening",
        }
    }
}

/// Aggregate perception signal for one (emitter, receiver) pair. Fed into
/// `cf-ai` doctrine via the snapshot path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerceptionSignal {
    pub source_actor: u64,
    pub receiver_actor: u64,
    pub raw_loudness: f32,
    pub distance: f32,
    pub occlusion_factor: f32,
    pub effective_loudness: f32,
    pub band: LoudnessBand,
}

impl PerceptionSignal {
    /// Combine raw loudness, distance attenuation, and occlusion factor into
    /// a single signal record.
    pub fn new(
        source_actor: u64,
        receiver_actor: u64,
        raw_loudness: f32,
        distance: f32,
        hearing_range: f32,
        occlusion_factor: f32,
    ) -> Self {
        let distance_factor = distance_attenuation(distance, hearing_range);
        let occ = occlusion_factor.clamp(0.0, 1.0);
        let effective = (raw_loudness * distance_factor * occ).clamp(0.0, 1.0);
        Self {
            source_actor,
            receiver_actor,
            raw_loudness,
            distance,
            occlusion_factor: occ,
            effective_loudness: effective,
            band: LoudnessBand::from_intensity(effective),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn band_intensity_thresholds() {
        assert_eq!(LoudnessBand::from_intensity(0.0), LoudnessBand::Inaudible);
        assert_eq!(LoudnessBand::from_intensity(0.1), LoudnessBand::Faint);
        assert_eq!(LoudnessBand::from_intensity(0.3), LoudnessBand::Moderate);
        assert_eq!(LoudnessBand::from_intensity(0.7), LoudnessBand::Loud);
        assert_eq!(LoudnessBand::from_intensity(0.95), LoudnessBand::Deafening);
        assert_eq!(LoudnessBand::from_intensity(f32::NAN), LoudnessBand::Inaudible);
    }

    #[test]
    fn signal_attenuates_with_distance() {
        let close = PerceptionSignal::new(1, 2, 1.0, 20.0, 100.0, 1.0);
        let far = PerceptionSignal::new(1, 2, 1.0, 80.0, 100.0, 1.0);
        assert!(close.effective_loudness > far.effective_loudness);
    }

    #[test]
    fn occlusion_reduces_signal() {
        let open = PerceptionSignal::new(1, 2, 1.0, 50.0, 100.0, 1.0);
        let occluded = PerceptionSignal::new(1, 2, 1.0, 50.0, 100.0, 0.5);
        assert!(open.effective_loudness > occluded.effective_loudness);
    }
}
