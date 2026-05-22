//! M9C § Files: on-disk RON schema for the 24 authored fortifications
//! under `content/fortifications/<kind>.ron`.
//!
//! Each `.ron` file deserializes into a [`FortificationSpec`] that
//! captures the spec's asset-table row verbatim:
//!
//! - `kind` — one of the 23 [`FortificationKind`] enum values + the
//!   `BunkerFiringSlit` overlap with M28F.
//! - `hp` — max HP per spec table.
//! - `footprint_tiles` — `(width, height)` per spec table.
//! - `build_time_seconds` — per spec table.
//! - `material_cost` — ordered cost map per spec table (e.g. `4 iron`).
//! - `provides` — short prose description (the spec's "Provides" column).
//! - Optional per-kind fields:
//!   - `cover_state` — for sandbag walls (3 tiers each with per-stance
//!     cover values).
//!   - `mine_kind` — for the 4 minefield template overlap rows.
//!   - `wire_kind` — for the 4 wire rows.
//!   - `anti_tank_kind` — for the 4 anti-tank rows.
//!   - `damage_per_tick` — per-tick damage applied while crossing wire.
//!   - `speed_multiplier_on_cross` — speed multiplier for crossing wire.
//! - `depends_on` — optional list of asset IDs this fortification
//!   references (e.g. `"trench_segment:fire_step"`). cf-mod validation
//!   uses this to emit `fortification_missing_dependency` warning
//!   events when the dependency is not yet shipped (forward-compat
//!   for cross-milestone references).
//!
//! VAL-M9C-005 + VAL-M9C-007 + VAL-M9C-MOD-MISSING-DEPENDENCY land here.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::anti_tank::AntiTankKind;
use crate::common::FortificationKind;
use crate::minefield::MineKind;
use crate::sandbag::SandbagTier;
use crate::wire::WireKind;

/// Three-state cover for a sandbag wall. Mirrors the `cf-trench`
/// `CoverState` enum but stays in cf-fortification so the crate has no
/// circular dependency on cf-trench (M9C is M9B's overlay, not its
/// peer). The string ids are the spec's table values verbatim.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum FortCoverLevel {
    None,
    Partial,
    Full,
}

impl FortCoverLevel {
    pub const ALL: [FortCoverLevel; 3] = [
        FortCoverLevel::None,
        FortCoverLevel::Partial,
        FortCoverLevel::Full,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            FortCoverLevel::None => "None",
            FortCoverLevel::Partial => "Partial",
            FortCoverLevel::Full => "Full",
        }
    }
}

/// Per-stance cover map authored on each sandbag-wall RON. Spec
/// columns: `Cover (Standing)` + `Cover (Crouched)` + (implicit prone
/// per Cortex-Command-pattern Full).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandbagCoverByStance {
    pub standing: FortCoverLevel,
    pub crouched: FortCoverLevel,
    #[serde(default = "default_prone_cover")]
    pub prone: FortCoverLevel,
}

const fn default_prone_cover() -> FortCoverLevel {
    FortCoverLevel::Full
}

/// One-stop schema for every fortification RON under
/// `content/fortifications/`. Per-kind fields are optional so a
/// `mg_nest_static` RON doesn't need `wire_kind` and vice versa.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FortificationSpec {
    pub kind: FortificationKind,
    pub hp: u32,
    pub footprint_tiles: (u32, u32),
    pub build_time_seconds: u32,
    pub material_cost: BTreeMap<String, u32>,
    #[serde(default)]
    pub provides: String,
    /// Cross-asset references (`asset_id:value`) the loader should
    /// check at validation time. Spec § Notes for the implementer
    /// describes the missing-dependency warning event surface.
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Sandbag-wall per-stance cover; only populated for the three
    /// `sandbag_*` rows.
    #[serde(default)]
    pub cover_state: Option<SandbagCoverByStance>,
    /// Sandbag-wall pixel height (4 / 8 / 12 per tier); only
    /// populated for the three `sandbag_*` rows.
    #[serde(default)]
    pub sandbag_tier: Option<SandbagTier>,
    /// Mine_kind selector for the (declarative) minefield template
    /// overlap rows. (The 24 fortifications are non-mine assets; this
    /// field exists so cf-mod can reject `unknown mine_kind` enum
    /// values authored in `content/mine_fields/*.minefield.ron`.)
    #[serde(default)]
    pub mine_kind: Option<MineKind>,
    /// Wire_kind selector for the four wire rows.
    #[serde(default)]
    pub wire_kind: Option<WireKind>,
    /// Anti-tank kind selector for the four anti-tank rows.
    #[serde(default)]
    pub anti_tank_kind: Option<AntiTankKind>,
    /// Damage per tick applied while an actor is crossing a wire
    /// without cutters (spec table column).
    #[serde(default)]
    pub damage_per_tick: Option<u32>,
    /// Speed multiplier (0.0–1.0) applied to an actor crossing a wire
    /// without cutters (spec table column).
    #[serde(default)]
    pub speed_multiplier_on_cross: Option<f32>,
}

impl FortificationSpec {
    pub fn from_ron_str(text: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str::<FortificationSpec>(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fortifications_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content/fortifications")
    }

    fn load(name: &str) -> FortificationSpec {
        let path = fortifications_dir().join(format!("{name}.ron"));
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        FortificationSpec::from_ron_str(&raw)
            .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
    }

    #[test]
    fn fortifications_load_all() {
        for kind in FortificationKind::ALL {
            let spec = load(kind.as_str());
            assert_eq!(spec.kind, kind, "kind field mismatches filename");
        }
    }

    #[test]
    fn sandbag_high_matches_spec_table() {
        let s = load("sandbag_high");
        assert_eq!(s.kind, FortificationKind::SandbagHigh);
        assert_eq!(s.hp, 600);
        assert_eq!(s.build_time_seconds, 12);
        assert_eq!(s.material_cost.get("sandbag"), Some(&12));
        let cover = s.cover_state.as_ref().expect("sandbag carries cover_state");
        assert_eq!(cover.standing, FortCoverLevel::Full);
        assert_eq!(cover.crouched, FortCoverLevel::Full);
        assert_eq!(s.sandbag_tier, Some(SandbagTier::High));
    }

    #[test]
    fn sandbag_mid_matches_spec_table() {
        let s = load("sandbag_mid");
        assert_eq!(s.kind, FortificationKind::SandbagMid);
        assert_eq!(s.hp, 400);
        assert_eq!(s.build_time_seconds, 8);
        assert_eq!(s.material_cost.get("sandbag"), Some(&8));
        let cover = s.cover_state.as_ref().expect("sandbag carries cover_state");
        assert_eq!(cover.standing, FortCoverLevel::Partial);
        assert_eq!(cover.crouched, FortCoverLevel::Full);
        assert_eq!(s.sandbag_tier, Some(SandbagTier::Mid));
    }

    #[test]
    fn sandbag_low_matches_spec_table() {
        let s = load("sandbag_low");
        assert_eq!(s.kind, FortificationKind::SandbagLow);
        assert_eq!(s.hp, 200);
        assert_eq!(s.build_time_seconds, 4);
        assert_eq!(s.material_cost.get("sandbag"), Some(&4));
        let cover = s.cover_state.as_ref().expect("sandbag carries cover_state");
        assert_eq!(cover.standing, FortCoverLevel::None);
        assert_eq!(cover.crouched, FortCoverLevel::Partial);
        assert_eq!(s.sandbag_tier, Some(SandbagTier::Low));
    }

    #[test]
    fn mg_nest_static_matches_spec_table() {
        let s = load("mg_nest_static");
        assert_eq!(s.kind, FortificationKind::MgNestStatic);
        assert_eq!(s.hp, 800);
        assert_eq!(s.footprint_tiles, (4, 2));
        assert_eq!(s.build_time_seconds, 60);
        assert_eq!(s.material_cost.get("sandbag"), Some(&60));
        assert_eq!(s.material_cost.get("iron"), Some(&40));
        assert_eq!(s.material_cost.get("heavy_mg_7_62mm"), Some(&1));
    }

    #[test]
    fn camo_netting_matches_spec_table() {
        let s = load("camo_netting");
        assert_eq!(s.kind, FortificationKind::CamoNetting);
        assert_eq!(s.hp, 100);
        assert_eq!(s.footprint_tiles, (4, 4));
    }

    #[test]
    fn watchtower_t3_matches_spec_table() {
        let s = load("watchtower_t3");
        assert_eq!(s.kind, FortificationKind::WatchtowerT3);
        assert_eq!(s.hp, 2400);
        assert_eq!(s.footprint_tiles, (5, 5));
        assert_eq!(s.build_time_seconds, 240);
    }

    #[test]
    fn electrified_fence_matches_spec_table() {
        let s = load("electrified_fence");
        assert_eq!(s.kind, FortificationKind::ElectrifiedFence);
        assert_eq!(s.hp, 400);
        assert_eq!(s.wire_kind, Some(WireKind::ElectrifiedFence));
    }

    #[test]
    fn dragons_teeth_matches_spec_table() {
        let s = load("dragons_teeth");
        assert_eq!(s.kind, FortificationKind::DragonsTeeth);
        assert_eq!(s.hp, 1200);
        assert_eq!(s.anti_tank_kind, Some(AntiTankKind::DragonsTeeth));
    }

    /// VAL-M9C-005 round-trip: every authored RON round-trips byte-
    /// identically through ron's pretty serializer.
    #[test]
    fn every_fortification_ron_round_trips() {
        for kind in FortificationKind::ALL {
            let spec = load(kind.as_str());
            let serialized = ron::ser::to_string_pretty(&spec, ron::ser::PrettyConfig::default())
                .expect("serialize spec");
            let parsed = FortificationSpec::from_ron_str(&serialized).expect("re-parse");
            assert_eq!(spec, parsed, "round-trip diverged for {kind:?}");
        }
    }

    /// loader at parse time with a typed error.
    #[test]
    fn fortification_kind_unknown_enum_rejected() {
        let bad = "(kind: definitely_not_a_kind, hp: 100, footprint_tiles: (1, 1), build_time_seconds: 1, material_cost: {})";
        let result = FortificationSpec::from_ron_str(bad);
        assert!(result.is_err(), "unknown kind must reject");
    }

    #[test]
    fn fortification_unknown_wire_kind_rejected() {
        let bad = "(kind: barbed_wire, hp: 200, footprint_tiles: (1, 1), build_time_seconds: 4, material_cost: {}, wire_kind: Some(not_a_wire_kind))";
        let result = FortificationSpec::from_ron_str(bad);
        assert!(result.is_err(), "unknown wire_kind must reject");
    }

    #[test]
    fn fortification_unknown_mine_kind_rejected() {
        let bad = "(kind: barbed_wire, hp: 200, footprint_tiles: (1, 1), build_time_seconds: 4, material_cost: {}, mine_kind: Some(not_a_mine_kind))";
        let result = FortificationSpec::from_ron_str(bad);
        assert!(result.is_err(), "unknown mine_kind must reject");
    }
}
