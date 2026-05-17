//! **M12B** § cf-app audio backend — Bevy + bevy_audio HRIR convolution
//! adapter + convolution-reverb send bus.
//!
//! Per M12B spec § Files:
//!
//! > `game/crates/cf-app/src/audio_backend/hrtf_convolution.rs` (NEW:
//! > bevy_audio HRIR convolution adapter)
//! > `game/crates/cf-app/src/audio_backend/reverb_send.rs` (NEW:
//! > convolution reverb send bus)
//!
//! The determinism surface lives in `cf-audio` (pure math); cf-app
//! consumes the per-source `SpatialEnvelope` + per-room
//! `ReverbProfile` and applies the HRIR convolution + reverb send at
//! playback time.
//!
//! Per spec § Notes for the implementer:
//!
//! > HRIR convolution at playback time only: `cf-audio::spatial`
//! > produces a `SpatialEnvelope` descriptor; the actual convolution
//! > lives in `cf-app::audio_backend::hrtf_convolution` (Bevy +
//! > bevy_audio + rustfft). Determinism surface stays in `cf-audio`;
//! > convolution stays out of the sim.

pub mod hrtf_convolution;
pub mod reverb_send;

pub use hrtf_convolution::{HrirConvolutionAdapter, HrirConvolutionFrame, HRIR_FADE_MS};
pub use reverb_send::{
    cross_fade_alpha, current_ir_id_for, ReverbSendBus, ReverbSendFrame, IR_CROSS_FADE_MS,
};
