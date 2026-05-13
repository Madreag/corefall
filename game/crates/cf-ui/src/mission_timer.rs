//! M2 — Mission timer HUD widget (MM:SS countdown with color states).
//!
//! Per the M2 spec's "## Files" section, `cf-ui/src/mission_timer.rs` is the
//! canonical home for the TIMER zone. The current implementation lives in
//! `cf-ui/src/lib.rs::mission_line` (which renders both the objective
//! progress + timer countdown together); this module re-exports the
//! function so consumers that import per the spec path
//! `cf_ui::mission_timer::*` resolve cleanly.
//!
//! Color states: green > 30s, yellow 10-30s, red < 10s (rendered by the
//! Bevy bind layer at `cf-app::main` since `cf-ui` is text-mode at M2).

pub use crate::{mission_line, HudMission};
