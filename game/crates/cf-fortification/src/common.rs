//! M9C: shared types referenced across the seven fortification
//! submodules.
//!
//! Per the spec § Notes for the implementer:
//!
//! > A crewed fortification has a 1:1 actor→fortification binding. The
//! > actor's stance becomes `Crewing { fortification_id }`.
//!
//! [`FortificationId`] is the engine-wide handle used for that binding
//! and for replay-event references. The kernel deliberately uses a
//! newtype around `u32` so cf-control (and cf-replay) can use the same
//! id space without leaking the actor / chassis numbering.
//!
//! M9C-1 ships only the type plumbing; per-fortification storage lives
//! in cf-control (added in m9c-2..m9c-6).

use serde::{Deserialize, Serialize};

/// Newtype handle for any placed M9C fortification.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct FortificationId(pub u32);

/// Convenience alias the spec calls `FortId` in places.
pub type FortId = FortificationId;

/// The 23 enumerated fortification kinds the M9C spec promises (24th
/// kind `bunker_firing_slit` reuses the bunker-template grammar from
/// M28F but is still authored under `content/fortifications/`). The
/// `as_str` mapping is what the per-asset RON files declare under
/// the top-level `kind` field.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FortificationKind {
    MgNestStatic = 0,
    AmmoBoxMg = 1,
    MgTripodPortable = 2,
    SpotterScope = 3,
    BunkerFiringSlit = 4,
    SandbagLow = 5,
    SandbagMid = 6,
    SandbagHigh = 7,
    WatchtowerT1 = 8,
    WatchtowerT2 = 9,
    WatchtowerT3 = 10,
    Spotlight = 11,
    ObservationPost = 12,
    RadioRepeater = 13,
    BarbedWire = 14,
    RazorWire = 15,
    ElectrifiedFence = 16,
    ConcertinaRoll = 17,
    AntiTankDitch = 18,
    DragonsTeeth = 19,
    TankTrapX = 20,
    BollardConcrete = 21,
    CamoNetting = 22,
}

impl FortificationKind {
    pub const ALL: [FortificationKind; 23] = [
        FortificationKind::MgNestStatic,
        FortificationKind::AmmoBoxMg,
        FortificationKind::MgTripodPortable,
        FortificationKind::SpotterScope,
        FortificationKind::BunkerFiringSlit,
        FortificationKind::SandbagLow,
        FortificationKind::SandbagMid,
        FortificationKind::SandbagHigh,
        FortificationKind::WatchtowerT1,
        FortificationKind::WatchtowerT2,
        FortificationKind::WatchtowerT3,
        FortificationKind::Spotlight,
        FortificationKind::ObservationPost,
        FortificationKind::RadioRepeater,
        FortificationKind::BarbedWire,
        FortificationKind::RazorWire,
        FortificationKind::ElectrifiedFence,
        FortificationKind::ConcertinaRoll,
        FortificationKind::AntiTankDitch,
        FortificationKind::DragonsTeeth,
        FortificationKind::TankTrapX,
        FortificationKind::BollardConcrete,
        FortificationKind::CamoNetting,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            FortificationKind::MgNestStatic => "mg_nest_static",
            FortificationKind::AmmoBoxMg => "ammo_box_mg",
            FortificationKind::MgTripodPortable => "mg_tripod_portable",
            FortificationKind::SpotterScope => "spotter_scope",
            FortificationKind::BunkerFiringSlit => "bunker_firing_slit",
            FortificationKind::SandbagLow => "sandbag_low",
            FortificationKind::SandbagMid => "sandbag_mid",
            FortificationKind::SandbagHigh => "sandbag_high",
            FortificationKind::WatchtowerT1 => "watchtower_t1",
            FortificationKind::WatchtowerT2 => "watchtower_t2",
            FortificationKind::WatchtowerT3 => "watchtower_t3",
            FortificationKind::Spotlight => "spotlight",
            FortificationKind::ObservationPost => "observation_post",
            FortificationKind::RadioRepeater => "radio_repeater",
            FortificationKind::BarbedWire => "barbed_wire",
            FortificationKind::RazorWire => "razor_wire",
            FortificationKind::ElectrifiedFence => "electrified_fence",
            FortificationKind::ConcertinaRoll => "concertina_roll",
            FortificationKind::AntiTankDitch => "anti_tank_ditch",
            FortificationKind::DragonsTeeth => "dragons_teeth",
            FortificationKind::TankTrapX => "tank_trap_x",
            FortificationKind::BollardConcrete => "bollard_concrete",
            FortificationKind::CamoNetting => "camo_netting",
        }
    }
}

/// Owning faction of a placed fortification. Used by crewing-permission
/// checks + detection-mask filtering (minesweeper marker visible only
/// to the sweeping faction, etc.).
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FortificationFaction {
    Player = 0,
    Enemy = 1,
    Neutral = 2,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fortification_kind_round_trips_via_ron() {
        for kind in FortificationKind::ALL {
            let s = kind.as_str();
            let parsed: FortificationKind =
                ron::from_str(s).expect("ron parse via as_str");
            assert_eq!(parsed, kind, "round-trip diverged for {kind:?}");
        }
    }

    #[test]
    fn fortification_id_is_orderable_and_hashable() {
        use std::collections::HashSet;
        let mut set: HashSet<FortificationId> = HashSet::new();
        set.insert(FortificationId(1));
        set.insert(FortificationId(1));
        set.insert(FortificationId(2));
        assert_eq!(set.len(), 2);
        let mut v = vec![FortificationId(3), FortificationId(1), FortificationId(2)];
        v.sort();
        assert_eq!(v, vec![FortificationId(1), FortificationId(2), FortificationId(3)]);
    }
}
