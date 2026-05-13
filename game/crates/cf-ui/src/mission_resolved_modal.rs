//! M2 — Mission resolved modal (win/loss screen + "show me why" CTA).
//!
//! Per the M2 spec's "## Files" section, `cf-ui/src/mission_resolved_modal.rs`
//! is the canonical home for the outcome modal. The current implementation
//! lives in `cf-ui/src/lib.rs` (`show_replay_cta_event_id` + the
//! `HudMission.show_me_why_event_id` field consumed by `cf-app::main`).
//! This module re-exports the public surface so consumers that import per
//! the spec path `cf_ui::mission_resolved_modal::*` resolve cleanly.

pub use crate::{show_replay_cta_event_id, HudMission};
