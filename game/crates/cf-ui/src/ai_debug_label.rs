//! M2 — AI debug label (floating intent text above guard sprite).
//!
//! Per the M2 spec's "## Files" section, `cf-ui/src/ai_debug_label.rs` is
//! the canonical home for the `--ai-debug` floating label. The current
//! implementation lives in `cf-ui/src/lib.rs::ai_debug_label`; this module
//! re-exports the function so consumers that import per the spec path
//! `cf_ui::ai_debug_label::*` resolve cleanly.

pub use crate::{ai_debug_label, HudEnemy, HudSettings};
