//! M2 — BreachStrip (soft-breach barrier proxy).
//!
//! Per the M2 spec's "## Files" section, `cf-terrain/src/breach.rs` is the
//! canonical home for `BreachStrip` + `BreachWorld` — the M2 stand-in for M3
//! chunked terrain. The current implementation lives in
//! `cf-terrain/src/lib.rs`; this module re-exports the public surface so
//! consumers that import per the spec path `cf_terrain::breach::*` resolve
//! cleanly.
//!
//! The implementation is throwaway (M3 chunked terrain replaces it) but the
//! EVENT contract is durable — replay consumers never see a schema bump.

pub use crate::{BreachStrip, BreachWorld};
