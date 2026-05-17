//! M3B: replay viewer + cause-chain + debrief library.
//!
//! Loads a run bundle written by `cf-replay::write_run_bundle`, and exposes
//! three views over the same data:
//!
//! - [`viewer`] — event tail / category filter / tick scrubber / pause-step
//!   state machine, rendered as markdown (M3B-001).
//! - [`cause_chain`] — `parent_event_id` walk for terminal events (M3B-002).
//! - [`debrief`] — outcome / objectives / key events / damage recap / terrain
//!   changes / checksum status summary (M3B-003).
//!
//! All output is deterministic markdown so golden tests + corefall-review
//! evidence + BP3 closure notes can compare bundles offline.

pub mod bundle;
pub mod cause_chain;
pub mod debrief;
pub mod delta_reconstructor;
pub mod editor;
pub mod renderer;
pub mod summary;
mod text;
pub mod thinking_timeline;
pub mod viewer;

pub use bundle::{Bundle, BundleError};
pub use editor::{
    const_scene_for_tick, EditorError, EditorState, ExportSelectionResult, ScrubResult, TrimSelection, PREVIEW_HEIGHT,
    PREVIEW_WIDTH, SCRUB_LATENCY_BUDGET_MS,
};
pub use renderer::{render_event_body, render_event_plain, MAX_SENTENCE_LEN};
pub use summary::SweepSummary;
pub use thinking_timeline::{build_timeline, slice_window, ThinkingTimelineEntry};
