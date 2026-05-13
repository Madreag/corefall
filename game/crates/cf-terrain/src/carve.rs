//! M3 — Carve / blast / fill helpers (per `specs/done/M3.md` "## Files" path).
//!
//! Canonical implementation lives in `cf-terrain/src/chunked.rs`; this module
//! re-exports the public surface so consumers that import per the spec path
//! `cf_terrain::carve::*` resolve cleanly.

pub use crate::chunked::DirtyRect;
