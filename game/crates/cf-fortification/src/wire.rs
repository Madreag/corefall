//! M9C § "Barbed wire + razor wire + electrified fence": placeholder
//! for the full surface that lands in feature m9c-5.
//!
//! This module is a deliberate scaffold for m9c-1: it owns the
//! [`WireKind`] enum (referenced by `content/fortifications/*.ron`
//! loaders + cf-mod validation) without implementing the per-actor
//! crossing state machine. The full kernel (crossing / snag /
//! electrified-power coupling / wire_cut) ships in
//! `m9c-5-wire-anti-tank-suite`.
//!
//! VAL-M9C-002 + VAL-M9C-007 (wire_kind enum surface) land here for
//! m9c-1.

use serde::{Deserialize, Serialize};

/// Four wire kinds enumerated in the spec § "Barbed wire + razor wire
/// + electrified fence" table.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WireKind {
    BarbedWire = 0,
    RazorWire = 1,
    ElectrifiedFence = 2,
    ConcertinaRoll = 3,
}

impl WireKind {
    pub const ALL: [WireKind; 4] = [
        WireKind::BarbedWire,
        WireKind::RazorWire,
        WireKind::ElectrifiedFence,
        WireKind::ConcertinaRoll,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            WireKind::BarbedWire => "barbed_wire",
            WireKind::RazorWire => "razor_wire",
            WireKind::ElectrifiedFence => "electrified_fence",
            WireKind::ConcertinaRoll => "concertina_roll",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_kind_round_trips_via_ron() {
        for kind in WireKind::ALL {
            let s = kind.as_str();
            let parsed: WireKind = ron::from_str(s).expect("ron round-trip");
            assert_eq!(parsed, kind);
        }
    }

    /// VAL-M9C-007: unknown enum values are rejected at parse time.
    #[test]
    fn unknown_wire_kind_rejected_at_parse() {
        let result: Result<WireKind, _> = ron::from_str("\"definitely_not_real_wire\"");
        assert!(result.is_err(), "unknown enum must reject");
    }
}
