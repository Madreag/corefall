//! M2 / M3 — Digger tool spec.
//!
//! Per the M2 + M3 spec "## Files" sections, `cf-equipment/src/digger.rs` is
//! the canonical home for the data-driven dig tool. M2 ships a minimal
//! parameter set used by the micro_breach + dig scenarios; M3's chunked
//! carving consumes the same params via `ChunkedTerrain::try_carve`.
//!
//! The current implementation stores dig parameters inline at the call
//! sites in `cf-control/src/engine.rs` (radius is supplied per-act); the
//! type below is a forward-compat surface for M5+ tool inventory where
//! actors carry multiple dig tools with different specs.

use serde::{Deserialize, Serialize};

/// **M2 / M3**: data-driven dig tool spec.
///
/// - `dig_strength` scales how much material a single act removes (placeholder;
///   M3's `try_carve` currently uses a flat carve radius — strength enters at
///   M5+ when material hardness ladders).
/// - `dig_radius` is the carve mask radius in world units.
/// - `dig_cooldown_ms` is the minimum interval between successive dig acts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiggerTool {
    pub id: String,
    #[serde(default = "default_dig_strength")]
    pub dig_strength: f32,
    #[serde(default = "default_dig_radius")]
    pub dig_radius: f32,
    #[serde(default = "default_dig_cooldown_ms")]
    pub dig_cooldown_ms: u32,
}

fn default_dig_strength() -> f32 {
    1.0
}
fn default_dig_radius() -> f32 {
    16.0
}
fn default_dig_cooldown_ms() -> u32 {
    150
}

impl Default for DiggerTool {
    fn default() -> Self {
        Self {
            id: "digger".into(),
            dig_strength: default_dig_strength(),
            dig_radius: default_dig_radius(),
            dig_cooldown_ms: default_dig_cooldown_ms(),
        }
    }
}
