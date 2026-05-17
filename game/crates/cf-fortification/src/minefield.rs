//! M9C § "Minefield system (4 mine kinds + minesweeper + bomb
//! disposal)": placeholder for the full surface that lands in feature
//! m9c-4.
//!
//! This module is a deliberate scaffold for m9c-1: it owns the
//! [`MineKind`] enum (referenced by `content/fortifications/*.ron` +
//! `content/mine_fields/*.minefield.ron` loaders + cf-mod validation)
//! without implementing the trigger-evaluation state machine. The full
//! kernel (proximity / pressure / tripwire / IED chain) ships in
//! `m9c-4-minefield-suite`.
//!
//! VAL-M9C-002 + VAL-M9C-007 (mine_kind enum surface) land here for
//! m9c-1.

use serde::{Deserialize, Serialize};

/// Four mine kinds enumerated in the spec § Minefield system table.
/// The RON loader rejects unknown enum values up-front via the
/// standard `serde` `#[serde(rename_all = "snake_case")]` shape.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MineKind {
    MineProximity = 0,
    MinePressure = 1,
    TripwireMine = 2,
    IedChain = 3,
}

impl MineKind {
    pub const ALL: [MineKind; 4] = [
        MineKind::MineProximity,
        MineKind::MinePressure,
        MineKind::TripwireMine,
        MineKind::IedChain,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            MineKind::MineProximity => "mine_proximity",
            MineKind::MinePressure => "mine_pressure",
            MineKind::TripwireMine => "tripwire_mine",
            MineKind::IedChain => "ied_chain",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mine_kind_round_trips_via_ron() {
        for kind in MineKind::ALL {
            let s = kind.as_str();
            let parsed: MineKind = ron::from_str(s).expect("ron round-trip");
            assert_eq!(parsed, kind);
        }
    }

    /// Unknown enum values are rejected at parse time per the
    /// VAL-M9C-007 contract. A garbled mine_kind string MUST fail with
    /// a typed error before the loader sees the value.
    #[test]
    fn unknown_mine_kind_rejected_at_parse() {
        let result: Result<MineKind, _> = ron::from_str("\"definitely_not_a_real_mine\"");
        assert!(result.is_err(), "unknown enum must reject");
    }
}
