//! M10B timeline scrub + trim primitives.
//!
//! Spec § "Files":
//!
//! > `game/crates/cf-tools-replay-viewer/src/timeline.rs` (NEW:
//! > frame-accurate scrub + trim controls)
//!
//! This module re-exports the frame-accurate scrub + trim primitives
//! the editor UI consumes:
//!
//! - [`ScrubResult`] — per-frame preview produced by
//!   `EditorState::scrub_to(tick)`; carries the rendered RGBA frame +
//!   the wall-clock latency for the scrub (VAL-M10B-028's `latency ≤
//!   16 ms` test reads `latency` off the result).
//! - [`TrimSelection`] — frame-accurate trim selection updated by the
//!   editor's "Set In" / "Set Out" buttons; consumed by
//!   `EditorState::export_selection` (VAL-M10B-029's frame-accurate
//!   trim contract).
//! - [`SCRUB_LATENCY_BUDGET_MS`] — VAL-M10B-028's 16 ms scrub-latency
//!   budget.
//! - [`PREVIEW_WIDTH`] / [`PREVIEW_HEIGHT`] — default editor preview
//!   resolution (320 × 180); the final `export_selection` job uses
//!   the preset's full resolution.
//!
//! The state-machine implementation lives in [`crate::editor`] (and
//! its [`crate::editor_ui`] re-export) so all unit tests can exercise
//! the timeline behavior without an egui test harness. This module
//! exists as the spec-named entry point per M10B § Files.

pub use crate::editor::{
    ScrubResult, TrimSelection, PREVIEW_HEIGHT, PREVIEW_WIDTH, SCRUB_LATENCY_BUDGET_MS,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_selection_is_valid_when_end_after_start() {
        let sel = TrimSelection {
            start_tick: 60,
            end_tick: 180,
        };
        assert!(sel.is_valid());
        assert_eq!(sel.len_ticks(), 120);
    }

    #[test]
    fn trim_selection_is_invalid_when_empty() {
        let sel = TrimSelection {
            start_tick: 100,
            end_tick: 100,
        };
        assert!(!sel.is_valid());
        assert_eq!(sel.len_ticks(), 0);
    }

    #[test]
    fn scrub_latency_budget_is_sixteen_ms() {
        assert_eq!(SCRUB_LATENCY_BUDGET_MS, 16);
    }

    /// Default preview resolution is 16:9.
    #[test]
    fn preview_resolution_is_sixteen_by_nine() {
        // 320 * 9 = 180 * 16, so the aspect ratio holds.
        assert_eq!(PREVIEW_WIDTH * 9, PREVIEW_HEIGHT * 16);
    }
}
