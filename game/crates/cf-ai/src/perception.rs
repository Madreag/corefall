//! M2 — Reactive guard perception.
//!
//! Per the M2 spec's "## Files" section, `cf-ai/src/perception.rs` is the
//! canonical home for sight_cone + hearing + memory_grid. The current
//! implementation lives in `cf-ai/src/lib.rs` for compactness; this module
//! re-exports the public surface so consumers that import per the spec path
//! `cf_ai::perception::*` resolve cleanly.

pub use crate::PerceptionRecord;
