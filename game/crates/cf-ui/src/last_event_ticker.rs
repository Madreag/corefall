//! M2 — Last-event ticker HUD widget (most recent significant event with
//! parent reason).
//!
//! Per the M2 spec's "## Files" section, `cf-ui/src/last_event_ticker.rs` is
//! the canonical home for the EVENT ticker zone. The current implementation
//! lives in `cf-ui/src/lib.rs` as the `last_event: Option<String>` field on
//! `HudBindParams` + the Bevy text query in `cf-app::main`. This module is
//! kept thin so consumers that import per the spec path
//! `cf_ui::last_event_ticker::*` resolve cleanly.

pub use crate::HudBanner;
