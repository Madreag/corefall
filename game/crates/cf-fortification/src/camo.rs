//! M9C § "Camo netting": 4×4 overlay placed on any structure /
//! vehicle / actor cluster.
//!
//! Per the spec table:
//!
//! > `camo_netting`: a 4×4 overlay placed on any structure / vehicle /
//! > actor cluster; gives `concealed: true` status against M22 visual
//! > detection at > 8 tiles range (only matters against AI/observers;
//! > not against spotlight or thermal scope).
//! >
//! > HP 100; flammable (M15D fire propagates fast).
//!
//! Per the spec § Notes for the implementer:
//!
//! > Camo netting concealment is a soft check, not a hard block — M22
//! > visual detection rolls `(distance, netting_present,
//! > observer_thermal_capable)`. Bypass rules: thermal scope (ARV),
//! > spotlight cone, distance < 8 tiles, motion-while-firing.
//!
//! VAL-M9C-048 / VAL-M9C-049 / VAL-M9C-CAMO-BASELINE-HOLDS land here.

use serde::{Deserialize, Serialize};

use crate::common::FortificationId;

/// HP cap of a placed `camo_netting` per spec table.
pub const CAMO_NETTING_HP: u32 = 100;
/// Footprint of a placed `camo_netting`: 4 tiles × 4 tiles per spec.
pub const CAMO_NETTING_TILE_FOOTPRINT: (u32, u32) = (4, 4);
/// Distance threshold for the "< 8 tiles" bypass rule (spec § Notes
/// for the implementer + spec table). At observer-range strictly less
/// than 8 tiles, ground-level visual detection bypasses the netting.
pub const BYPASS_RANGE_TILES: u32 = 8;

/// Placed `camo_netting` overlay in the world.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CamoNetting {
    pub id: FortificationId,
    pub hp: u32,
}

impl CamoNetting {
    #[must_use]
    pub fn new(id: FortificationId) -> Self {
        Self { id, hp: CAMO_NETTING_HP }
    }

    #[must_use]
    pub fn is_destroyed(&self) -> bool {
        self.hp == 0
    }
}

/// Inputs to the per-observer camo concealment check.
///
/// Field names match the spec § Notes for the implementer four bypass
/// rules verbatim so cf-control + cf-ai callers can wire them without
/// renaming.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct CamoConcealmentInputs {
    /// Range from observer to the concealed actor in tiles. `< 8` is
    /// one of the four bypass rules.
    pub observer_range_tiles: u32,
    /// True when the observer has a thermal scope (M44D ARV reference).
    pub observer_has_thermal: bool,
    /// True when the concealed actor is inside an active spotlight
    /// cone.
    pub illuminated_by_spotlight: bool,
    /// True when the concealed actor is firing while in motion (the
    /// "motion-while-firing" bypass rule).
    pub actor_motion_while_firing: bool,
}

impl CamoConcealmentInputs {
    /// Baseline: every bypass rule is FALSE. Used by
    /// `camo_baseline_concealment_holds` per VAL-M9C-CAMO-BASELINE-
    /// HOLDS.
    #[must_use]
    pub const fn baseline() -> Self {
        Self {
            observer_range_tiles: BYPASS_RANGE_TILES + 8,
            observer_has_thermal: false,
            illuminated_by_spotlight: false,
            actor_motion_while_firing: false,
        }
    }
}

/// Enumerates which (if any) bypass rule fired. Returned by
/// [`camo_concealed`] for logging + AI introspection. The
/// `None`-shape means concealment HOLDS.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CamoBypassReason {
    /// Observer is < 8 tiles away.
    ShortRange = 0,
    /// Observer has a thermal scope.
    ThermalScope = 1,
    /// Concealed actor is in an active spotlight cone.
    SpotlightCone = 2,
    /// Concealed actor is firing while moving.
    MotionWhileFiring = 3,
}

impl CamoBypassReason {
    pub const ALL: [CamoBypassReason; 4] = [
        CamoBypassReason::ShortRange,
        CamoBypassReason::ThermalScope,
        CamoBypassReason::SpotlightCone,
        CamoBypassReason::MotionWhileFiring,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            CamoBypassReason::ShortRange => "short_range",
            CamoBypassReason::ThermalScope => "thermal_scope",
            CamoBypassReason::SpotlightCone => "spotlight_cone",
            CamoBypassReason::MotionWhileFiring => "motion_while_firing",
        }
    }
}

/// Outcome of the per-observer camo concealment check.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CamoConcealment {
    /// All bypass rules are absent → observer sees the netting; the
    /// concealed actor remains `concealed=true`.
    Concealed,
    /// One or more bypass rules fired → observer sees the actor
    /// through the netting; concealment is broken. The reason field
    /// names the first-fired bypass rule (BFS order, used for
    /// telemetry / AI introspection).
    Bypassed(CamoBypassReason),
}

impl CamoConcealment {
    #[must_use]
    pub const fn is_concealed(self) -> bool {
        matches!(self, CamoConcealment::Concealed)
    }

    #[must_use]
    pub const fn bypass_reason(self) -> Option<CamoBypassReason> {
        match self {
            CamoConcealment::Concealed => None,
            CamoConcealment::Bypassed(r) => Some(r),
        }
    }
}

/// Pure helper that returns whether camo netting conceals the actor
/// against a given observer. Spec § Notes for the implementer is the
/// reference:
///
/// > Bypass rules: thermal scope (ARV), spotlight cone, distance < 8
/// > tiles, motion-while-firing.
///
/// Bypass priority (BFS, for telemetry only — concealment outcome
/// is identical regardless of priority since ANY bypass fires):
///
/// 1. Thermal scope
/// 2. Spotlight cone
/// 3. Short range (< 8 tiles)
/// 4. Motion while firing
#[must_use]
pub fn camo_concealed(inputs: CamoConcealmentInputs) -> CamoConcealment {
    if inputs.observer_has_thermal {
        return CamoConcealment::Bypassed(CamoBypassReason::ThermalScope);
    }
    if inputs.illuminated_by_spotlight {
        return CamoConcealment::Bypassed(CamoBypassReason::SpotlightCone);
    }
    if inputs.observer_range_tiles < BYPASS_RANGE_TILES {
        return CamoConcealment::Bypassed(CamoBypassReason::ShortRange);
    }
    if inputs.actor_motion_while_firing {
        return CamoConcealment::Bypassed(CamoBypassReason::MotionWhileFiring);
    }
    CamoConcealment::Concealed
}

#[cfg(test)]
mod tests {
    use super::*;

    /// VAL-M9C-CAMO-BASELINE-HOLDS: baseline returns concealed=true.
    #[test]
    fn camo_baseline_concealment_holds() {
        let result = camo_concealed(CamoConcealmentInputs::baseline());
        assert_eq!(result, CamoConcealment::Concealed);
        assert!(result.is_concealed());
        assert!(result.bypass_reason().is_none());
    }

    /// Alias matching `expectedBehavior.camo_concealment_baseline_holds`
    /// in the feature definition.
    #[test]
    fn camo_concealment_baseline_holds() {
        camo_baseline_concealment_holds();
    }

    /// VAL-M9C-048: thermal scope (ARV) bypasses concealment.
    #[test]
    fn camo_bypass_thermal_scope() {
        let inputs = CamoConcealmentInputs {
            observer_has_thermal: true,
            ..CamoConcealmentInputs::baseline()
        };
        assert_eq!(
            camo_concealed(inputs),
            CamoConcealment::Bypassed(CamoBypassReason::ThermalScope)
        );
    }

    /// VAL-M9C-023 helper: spotlight cone bypasses concealment.
    #[test]
    fn camo_bypass_spotlight_cone() {
        let inputs = CamoConcealmentInputs {
            illuminated_by_spotlight: true,
            ..CamoConcealmentInputs::baseline()
        };
        assert_eq!(
            camo_concealed(inputs),
            CamoConcealment::Bypassed(CamoBypassReason::SpotlightCone)
        );
    }

    /// VAL-M9C-049 part 1: observer range < 8 tiles bypasses
    /// concealment.
    #[test]
    fn camo_bypass_short_range() {
        let inputs = CamoConcealmentInputs {
            observer_range_tiles: BYPASS_RANGE_TILES - 1,
            ..CamoConcealmentInputs::baseline()
        };
        assert_eq!(
            camo_concealed(inputs),
            CamoConcealment::Bypassed(CamoBypassReason::ShortRange)
        );
    }

    /// VAL-M9C-049 part 2: motion-while-firing bypasses concealment.
    #[test]
    fn camo_bypass_motion_while_firing() {
        let inputs = CamoConcealmentInputs {
            actor_motion_while_firing: true,
            ..CamoConcealmentInputs::baseline()
        };
        assert_eq!(
            camo_concealed(inputs),
            CamoConcealment::Bypassed(CamoBypassReason::MotionWhileFiring)
        );
    }

    /// Boundary: observer at exactly 8 tiles remains concealed
    /// (bypass rule is strictly `< 8`, not `<= 8`).
    #[test]
    fn camo_concealed_at_exactly_threshold_range() {
        let inputs = CamoConcealmentInputs {
            observer_range_tiles: BYPASS_RANGE_TILES,
            ..CamoConcealmentInputs::baseline()
        };
        assert_eq!(camo_concealed(inputs), CamoConcealment::Concealed);
    }

    /// When multiple bypass rules fire, thermal scope wins by BFS
    /// priority.
    #[test]
    fn camo_bypass_thermal_dominates() {
        let inputs = CamoConcealmentInputs {
            observer_has_thermal: true,
            illuminated_by_spotlight: true,
            observer_range_tiles: 1,
            actor_motion_while_firing: true,
        };
        assert_eq!(
            camo_concealed(inputs),
            CamoConcealment::Bypassed(CamoBypassReason::ThermalScope)
        );
    }

    #[test]
    fn camo_netting_construction() {
        let net = CamoNetting::new(FortificationId(7));
        assert_eq!(net.hp, CAMO_NETTING_HP);
        assert!(!net.is_destroyed());
        let dead = CamoNetting {
            id: FortificationId(7),
            hp: 0,
        };
        assert!(dead.is_destroyed());
    }

    #[test]
    fn camo_bypass_reason_as_str_round_trips() {
        for r in CamoBypassReason::ALL {
            let s = r.as_str();
            let parsed: CamoBypassReason = ron::from_str(s).expect("ron round-trip");
            assert_eq!(parsed, r);
        }
    }
}
