//! M9C: minesweeper — handheld mine-detection tool.
//!
//! Spec § "Minesweeper tool (T1 handheld)": held in equipment slot;
//! emits 3-tile-radius detection ping every 2s; revealed mines render
//! with yellow marker visible only to the sweeping faction;
//! `minesweeper_detected` event fires.
//!
//! Per the minefield kernel:
//! - 3-tile radius covers `mine_proximity` + `tripwire_mine`.
//! - 2-tile radius covers `mine_pressure` + `ied_chain` (hidden mines).
//!
//! The tool is registered alongside the M9B entrenching tool so the
//! cfctl handler can dispatch detection pings + the AI engineer
//! doctrine (AI-ENG-A-03) can decide to use a minesweeper to disarm
//! enemy minefields in the squad's path.
//!
//! VAL-M9C-054 + VAL-M9C-025 + VAL-M9C-046 land here.

use serde::{Deserialize, Serialize};

/// Canonical id under which the minesweeper tool is registered in the
/// M9C tool catalog.
pub const MINESWEEPER_ID: &str = "minesweeper";
/// Spec § "Minesweeper": 2-second detection ping interval.
pub const MINESWEEPER_PING_SECONDS: u32 = 2;
/// Spec § "Minesweeper": 3-tile radius for proximity / tripwire mines.
pub const MINESWEEPER_RADIUS_PROXIMITY_TRIPWIRE: u32 = 3;
/// Spec § "Minesweeper": 2-tile radius for pressure / IED mines.
pub const MINESWEEPER_RADIUS_PRESSURE_IED: u32 = 2;

/// On-disk + in-code spec for the minesweeper tool. Mirrors the
/// [`super::entrenching::EntrenchingToolSpec`] minimal shape so the
/// catalog stays uniform.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MinesweeperToolSpec {
    pub id: String,
    pub display_name: String,
    /// Detection ping interval in seconds (spec: 2 s).
    pub ping_interval_seconds: u32,
    /// Detection radius for proximity / tripwire mines (spec: 3 tiles).
    pub radius_proximity_tripwire_tiles: u32,
    /// Detection radius for pressure / IED mines (spec: 2 tiles).
    pub radius_pressure_ied_tiles: u32,
    /// Mining tier — minesweeper is T1 (entry-level handheld).
    pub tier: u8,
    pub mass_kg: f32,
    pub max_durability: f32,
}

#[must_use]
pub fn minesweeper_m9c_default() -> MinesweeperToolSpec {
    MinesweeperToolSpec {
        id: MINESWEEPER_ID.to_string(),
        display_name: "Minesweeper".to_string(),
        ping_interval_seconds: MINESWEEPER_PING_SECONDS,
        radius_proximity_tripwire_tiles: MINESWEEPER_RADIUS_PROXIMITY_TRIPWIRE,
        radius_pressure_ied_tiles: MINESWEEPER_RADIUS_PRESSURE_IED,
        tier: 1,
        mass_kg: 2.5,
        max_durability: 60.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minesweeper_default_matches_spec() {
        let m = minesweeper_m9c_default();
        assert_eq!(m.id, MINESWEEPER_ID);
        assert_eq!(m.tier, 1);
        assert_eq!(m.ping_interval_seconds, 2);
        assert_eq!(m.radius_proximity_tripwire_tiles, 3);
        assert_eq!(m.radius_pressure_ied_tiles, 2);
    }
}
