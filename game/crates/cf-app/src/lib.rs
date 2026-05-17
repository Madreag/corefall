//! `cf-app` library surface.
//!
//! Most of the cf-app codebase lives in `main.rs` (the Bevy binary).
//! This library target exposes the headlessly-testable bits the
//! mission-validation tests reach for — currently the M10B
//! post-mission debrief modal CTA per VAL-M10B-DEBRIEF-CTA.

#![deny(unsafe_code)]

pub mod debrief_modal;

pub use debrief_modal::{
    build_debrief_modal, DebriefModal, DebriefModalButton, ExportCtaDispatch, EXPORT_LAST_REPLAY_BUTTON_ID,
    EXPORT_LAST_REPLAY_BUTTON_LABEL,
};
