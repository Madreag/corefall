//! M2 — AI difficulty presets.
//!
//! Per the M2 spec's "## Files" section, `cf-ai/src/difficulty.rs` is the
//! canonical home for the 3 difficulty presets (`cakewalk`, `tough_crowd`,
//! `veteran`) loaded from `content/ai/difficulty.json`. The current
//! implementation lives in `cf-ai/src/lib.rs`; this module re-exports the
//! public surface so consumers that import per the spec path
//! `cf_ai::difficulty::*` resolve cleanly.

pub use crate::DifficultyPreset;
