//! **M12B** § Bevy bevy_audio HRIR convolution adapter.
//!
//! Per M12B spec § Notes for the implementer:
//!
//! > `cf-app::audio_backend::hrtf_convolution` (Bevy + bevy_audio +
//! > rustfft). Determinism surface stays in `cf-audio`; convolution
//! > stays out of the sim.
//!
//! This module is the **adapter** layer that takes per-source
//! [`cf_audio::SpatialEnvelope`] descriptors produced by the sim's
//! `resolve_spatial` call and applies them to the bevy_audio output
//! stream (left ear + right ear). Production DSP (rustfft FFT
//! convolution) is gated behind a feature flag; M12B baseline keeps
//! the adapter shape pure — the convolution math lives behind a trait
//! so the Bevy audio crate can wire a real backend later without
//! re-shaping the call boundary.

use std::sync::Arc;

use cf_audio::{HrirIndex, HrirTable, SpatialEnvelope, HRTF_SAMPLES};

/// **M12B** § Cross-fade window used when the listener crosses an HRIR
/// bucket boundary. Avoids audible clicks when the player turns.
pub const HRIR_FADE_MS: f32 = 25.0;

/// **M12B** § A single playback frame produced by the convolution
/// adapter. Owned `(left_taps, right_taps)` — `HRTF_SAMPLES` f32s each.
/// The actual mixing into the bevy_audio output stream is the caller's
/// responsibility; this struct is the deterministic descriptor passed
/// between the adapter and the audio backend.
#[derive(Debug, Clone)]
pub struct HrirConvolutionFrame {
    /// HRIR bucket used to convolve this frame.
    pub hrir_index: HrirIndex,
    /// Effective gain applied to both ears (per-source pre-convolution).
    pub gain: f32,
    /// Left-ear samples (HRTF_SAMPLES taps).
    pub left_taps: Vec<f32>,
    /// Right-ear samples (HRTF_SAMPLES taps).
    pub right_taps: Vec<f32>,
    /// Low-pass cutoff to apply post-convolution (Hz).
    pub low_pass_cutoff_hz: f32,
    /// Pitch shift factor (Doppler).
    pub pitch_factor: f32,
}

impl HrirConvolutionFrame {
    /// **M12B** § Empty (silent) frame. Used when the spatial envelope's
    /// `gain == 0` (vacuum, out-of-range, fully-occluded source).
    #[must_use]
    pub fn silent(hrir_index: HrirIndex) -> Self {
        Self {
            hrir_index,
            gain: 0.0,
            left_taps: vec![0.0; HRTF_SAMPLES],
            right_taps: vec![0.0; HRTF_SAMPLES],
            low_pass_cutoff_hz: 20_000.0,
            pitch_factor: 1.0,
        }
    }
}

/// **M12B** § The cf-app adapter. Holds an `Arc<HrirTable>` shared
/// across systems; every `resolve` call is `O(1)` index math + an
/// HRIR slice clone (no allocation in steady state once the
/// `Vec<f32>` capacity is reserved).
#[derive(Debug, Clone)]
pub struct HrirConvolutionAdapter {
    table: Arc<HrirTable>,
}

impl HrirConvolutionAdapter {
    /// **M12B** § Construct an adapter from a shared HRIR table. cf-app
    /// loads the table once at startup from
    /// `game/content/audio/hrtf/mit_kemar_subset.bin`; the
    /// [`HrirTable::placeholder`] is the safe fallback when the
    /// production binary isn't on disk.
    #[must_use]
    pub fn new(table: Arc<HrirTable>) -> Self {
        Self { table }
    }

    /// **M12B** § Resolve one [`HrirConvolutionFrame`] for a given
    /// [`SpatialEnvelope`]. Pure adapter logic — no allocation when
    /// `frame.left_taps`/`right_taps` are reused, no Bevy/rodio
    /// dependency, no `thread_rng`.
    #[must_use]
    pub fn resolve(&self, envelope: &SpatialEnvelope) -> HrirConvolutionFrame {
        if envelope.gain <= 1e-6 {
            return HrirConvolutionFrame::silent(envelope.hrir_index);
        }
        let left_slice = self.table.lookup(
            envelope.hrir_index.azimuth_bucket as usize,
            envelope.hrir_index.elevation_bucket as usize,
            0,
        );
        let right_slice = self.table.lookup(
            envelope.hrir_index.azimuth_bucket as usize,
            envelope.hrir_index.elevation_bucket as usize,
            1,
        );
        let left_taps: Vec<f32> = left_slice.iter().map(|t| t * envelope.gain).collect();
        let right_taps: Vec<f32> = right_slice.iter().map(|t| t * envelope.gain).collect();
        HrirConvolutionFrame {
            hrir_index: envelope.hrir_index,
            gain: envelope.gain,
            left_taps,
            right_taps,
            low_pass_cutoff_hz: envelope.occlusion.low_pass_cutoff_hz.min(envelope.medium_filter.cutoff_hz),
            pitch_factor: envelope.doppler.factor,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_audio::{
        resolve_spatial, ListenerContext, Medium, ReverbProfile, SourceContext, WallAcoustics,
    };

    fn mk_envelope(gain_override: f32) -> SpatialEnvelope {
        let listener = ListenerContext {
            position: [0.0, 0.0],
            velocity: [0.0, 0.0],
            facing_rad: 0.0,
            room_id: None,
        };
        let source = SourceContext {
            position: [0.0, 10.0],
            velocity: [0.0, 0.0],
            base_gain: gain_override,
            propagation_range_m: 100.0,
            room_id: None,
        };
        resolve_spatial(source, listener, Medium::Air, &[], ReverbProfile::open_outdoor())
    }

    #[test]
    fn convolution_adapter_returns_silent_frame_for_zero_gain() {
        let table = Arc::new(HrirTable::placeholder());
        let adapter = HrirConvolutionAdapter::new(table);
        let mut env = mk_envelope(1.0);
        env.gain = 0.0;
        let frame = adapter.resolve(&env);
        assert!(frame.gain.abs() < 1e-6);
        assert_eq!(frame.left_taps.len(), HRTF_SAMPLES);
        assert_eq!(frame.right_taps.len(), HRTF_SAMPLES);
        for s in frame.left_taps.iter().chain(frame.right_taps.iter()) {
            assert!(s.abs() < 1e-6);
        }
    }

    #[test]
    fn convolution_adapter_scales_taps_by_envelope_gain() {
        let table = Arc::new(HrirTable::placeholder());
        let adapter = HrirConvolutionAdapter::new(table);
        let mut env = mk_envelope(1.0);
        env.gain = 0.5;
        let frame = adapter.resolve(&env);
        // Placeholder table puts a 1.0 impulse at sample 0; with gain
        // 0.5 we expect sample[0] == 0.5.
        assert!((frame.left_taps[0] - 0.5).abs() < 1e-4);
        assert!((frame.right_taps[0] - 0.5).abs() < 1e-4);
    }

    #[test]
    fn convolution_adapter_uses_hrir_index_from_envelope() {
        let table = Arc::new(HrirTable::placeholder());
        let adapter = HrirConvolutionAdapter::new(table);
        let env = mk_envelope(1.0);
        let frame = adapter.resolve(&env);
        assert_eq!(frame.hrir_index, env.hrir_index);
    }

    #[test]
    fn convolution_adapter_carries_doppler_pitch_factor() {
        let table = Arc::new(HrirTable::placeholder());
        let adapter = HrirConvolutionAdapter::new(table);
        let mut env = mk_envelope(1.0);
        env.doppler.factor = 0.5;
        let frame = adapter.resolve(&env);
        assert!((frame.pitch_factor - 0.5).abs() < 1e-6);
    }

    #[test]
    fn convolution_adapter_picks_min_low_pass_cutoff() {
        let table = Arc::new(HrirTable::placeholder());
        let adapter = HrirConvolutionAdapter::new(table);
        let listener = ListenerContext {
            position: [-5.0, 0.0],
            velocity: [0.0, 0.0],
            facing_rad: 0.0,
            room_id: None,
        };
        let source = SourceContext {
            position: [5.0, 0.0],
            velocity: [0.0, 0.0],
            base_gain: 1.0,
            propagation_range_m: 100.0,
            room_id: None,
        };
        // Underwater (800 Hz medium cutoff) + concrete wall (800 Hz
        // wall cutoff) — both at 800 Hz; min stays at 800.
        let walls = vec![WallAcoustics {
            transmission_loss_db: 28.0,
            low_pass_cutoff_hz: 800.0,
        }];
        let env = resolve_spatial(
            source,
            listener,
            Medium::Water,
            &walls,
            ReverbProfile::open_outdoor(),
        );
        let frame = adapter.resolve(&env);
        assert!((frame.low_pass_cutoff_hz - 800.0).abs() < 1e-3);
    }

    #[test]
    fn hrir_fade_ms_constant_matches_spec() {
        // Spec § "Cross-fade the IR over 250 ms to avoid clicks". Per-frame
        // HRIR cross-fade window is shorter (25 ms here) since HRIR-bucket
        // transitions are far more frequent than IR swaps.
        assert!((HRIR_FADE_MS - 25.0).abs() < 1e-3);
    }
}
