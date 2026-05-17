//! `cf-app` library surface.
//!
//! Most of the cf-app codebase lives in `main.rs` (the Bevy binary).
//! This library target exposes the headlessly-testable bits the
//! mission-validation tests reach for — currently the M10B
//! post-mission debrief modal CTA per VAL-M10B-DEBRIEF-CTA + the
//! M12B audio backend (HRIR convolution + reverb send).

#![deny(unsafe_code)]

pub mod audio_backend;
pub mod debrief_modal;

pub use audio_backend::{
    cross_fade_alpha, current_ir_id_for, HrirConvolutionAdapter, HrirConvolutionFrame, ReverbSendBus, ReverbSendFrame,
    HRIR_FADE_MS, IR_CROSS_FADE_MS,
};
pub use debrief_modal::{
    build_debrief_modal, DebriefModal, DebriefModalButton, ExportCtaDispatch, EXPORT_LAST_REPLAY_BUTTON_ID,
    EXPORT_LAST_REPLAY_BUTTON_LABEL,
};
