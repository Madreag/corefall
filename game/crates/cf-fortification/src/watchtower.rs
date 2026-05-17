//! M9C § "Watchtower (3 height tiers) + spotlight + observation post":
//! placeholder for the full surface that lands in feature m9c-3.
//!
//! This module is a deliberate scaffold for m9c-1: it owns the
//! type-shape contract for the watchtower tier-ladder (so the
//! cf-fortification public API can be frozen at workspace-registration
//! time) without implementing the spotter mark / spotlight cone /
//! lateral-collapse logic. Those land in `m9c-3-watchtower-suite`.
//!
//! VAL-M9C-002 is the only contract this file satisfies for m9c-1.

use serde::{Deserialize, Serialize};

use crate::common::FortificationId;

/// Spec table row HP per watchtower tier.
pub const WATCHTOWER_T1_MAX_HP: u32 = 600;
pub const WATCHTOWER_T2_MAX_HP: u32 = 1200;
pub const WATCHTOWER_T3_MAX_HP: u32 = 2400;
/// Spec table row HP for a `spotlight` mounted on a watchtower.
pub const SPOTLIGHT_MAX_HP: u32 = 100;
/// Spec table row HP for an `observation_post`.
pub const OBSERVATION_POST_MAX_HP: u32 = 400;
/// Spec table row HP for a `radio_repeater`.
pub const RADIO_REPEATER_MAX_HP: u32 = 200;

/// One of the three watchtower height tiers (m9c-3 fills in the full
/// per-tier kernel + spotter + radio_repeater integration).
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchtowerTier {
    T1 = 1,
    T2 = 2,
    T3 = 3,
}

impl WatchtowerTier {
    pub const ALL: [WatchtowerTier; 3] = [
        WatchtowerTier::T1,
        WatchtowerTier::T2,
        WatchtowerTier::T3,
    ];

    #[must_use]
    pub const fn max_hp(self) -> u32 {
        match self {
            WatchtowerTier::T1 => WATCHTOWER_T1_MAX_HP,
            WatchtowerTier::T2 => WATCHTOWER_T2_MAX_HP,
            WatchtowerTier::T3 => WATCHTOWER_T3_MAX_HP,
        }
    }
}

/// Placed watchtower instance (m9c-1 placeholder).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Watchtower {
    pub id: FortificationId,
    pub tier: WatchtowerTier,
    pub hp: u32,
}

impl Watchtower {
    #[must_use]
    pub fn new_built(id: FortificationId, tier: WatchtowerTier) -> Self {
        Self {
            id,
            tier,
            hp: tier.max_hp(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watchtower_tier_max_hp_matches_spec_table() {
        assert_eq!(WatchtowerTier::T1.max_hp(), 600);
        assert_eq!(WatchtowerTier::T2.max_hp(), 1200);
        assert_eq!(WatchtowerTier::T3.max_hp(), 2400);
    }

    #[test]
    fn watchtower_t3_built_pins_max_hp() {
        let tower = Watchtower::new_built(FortificationId(1), WatchtowerTier::T3);
        assert_eq!(tower.hp, WATCHTOWER_T3_MAX_HP);
    }
}
