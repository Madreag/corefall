//! M3 — Dirty-region tracker (per `specs/done/M3.md` "## Files" path).
//!
//! Canonical implementation lives in `cf-terrain/src/chunked.rs` (the
//! per-tick coalescing is in `cf-control/src/engine.rs::flush_pending_dirty_batch`).
//! This module re-exports the public surface so consumers that import per
//! the spec path `cf_terrain::dirty::*` resolve cleanly.

pub use crate::chunked::DirtyRect;
