//! M2 — Objective banner HUD widget (title + sub-line).
//!
//! Per the M2 spec's "## Files" section, `cf-ui/src/objective_banner.rs` is
//! the canonical home for the OBJECTIVE banner zone. The current
//! implementation lives in `cf-ui/src/lib.rs::objective_line`; this module
//! re-exports the function so consumers that import per the spec path
//! `cf_ui::objective_banner::*` resolve cleanly.

pub use crate::{objective_line, HudMission};
