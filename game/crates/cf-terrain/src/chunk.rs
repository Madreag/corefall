//! M3 — Chunk struct (per `specs/done/M3.md` "## Files" path).
//!
//! Canonical implementation lives in `cf-terrain/src/chunked.rs`; this module
//! re-exports the public surface so consumers that import per the spec path
//! `cf_terrain::chunk::*` resolve cleanly.

pub use crate::chunked::{Chunk, ChunkedTerrain, ChunkedTerrainSnapshot, ChunkedTerrainSnapshotChunk};
