//! M3 — Per-chunk checksum (per `specs/done/M3.md` "## Files" path).
//!
//! Per the M3 spec, every `Chunk` has a stable blake3 checksum over its
//! `material_grid`. The canonical implementation lives on
//! `cf-terrain/src/chunked.rs::Chunk::checksum`. This module exists per the
//! spec's "## Files" enumeration so future consumers that import
//! `cf_terrain::checksum::*` resolve cleanly.

pub use crate::chunked::Chunk;
