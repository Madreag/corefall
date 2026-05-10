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
pub mod viewer;

pub use bundle::{Bundle, BundleError};
