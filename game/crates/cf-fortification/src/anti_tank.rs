//! M9C § "Anti-tank ditch + dragon's teeth + tank trap": placeholder
//! for the full surface that lands in feature m9c-5.
//!
//! This module is a deliberate scaffold for m9c-1: it owns the
//! [`AntiTankKind`] enum (referenced by `content/fortifications/*.ron`
//! loaders + cf-mod validation) without implementing the
//! vehicle-collision state machine. The full kernel (anti-tank ditch
//! stuck-chance, dragon's teeth per-component damage routing, tank
//! trap X destructibility) ships in `m9c-5-wire-anti-tank-suite`.
//!
//! VAL-M9C-002 + VAL-M9C-007 (anti-tank enum surface) land here for
//! m9c-1.

use serde::{Deserialize, Serialize};

/// Four anti-tank kinds enumerated in the spec § "Anti-tank ditch +
/// dragon's teeth + tank trap" table.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AntiTankKind {
    AntiTankDitch = 0,
    DragonsTeeth = 1,
    TankTrapX = 2,
    BollardConcrete = 3,
}

impl AntiTankKind {
    pub const ALL: [AntiTankKind; 4] = [
        AntiTankKind::AntiTankDitch,
        AntiTankKind::DragonsTeeth,
        AntiTankKind::TankTrapX,
        AntiTankKind::BollardConcrete,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            AntiTankKind::AntiTankDitch => "anti_tank_ditch",
            AntiTankKind::DragonsTeeth => "dragons_teeth",
            AntiTankKind::TankTrapX => "tank_trap_x",
            AntiTankKind::BollardConcrete => "bollard_concrete",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anti_tank_kind_round_trips_via_ron() {
        for kind in AntiTankKind::ALL {
            let s = kind.as_str();
            let parsed: AntiTankKind = ron::from_str(s).expect("ron round-trip");
            assert_eq!(parsed, kind);
        }
    }

    #[test]
    fn unknown_anti_tank_kind_rejected_at_parse() {
        let result: Result<AntiTankKind, _> = ron::from_str("\"definitely_not_real_at\"");
        assert!(result.is_err(), "unknown enum must reject");
    }
}
