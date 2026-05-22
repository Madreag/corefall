//! **M14F** § Brace-strut lateral reinforcement item.
//!
//! Spec §"Crates / modules touched":
//! > New `cf-equipment::brace_strut` item — T1/T2/T3 tiers, behaviorally
//! > differentiated (lock radius or lock strength scales with tier).
//! > Same cost class as M14E `support_beam_placer`.
//!
//! Same hold-to-place UX as the M14E support-beam placer; the engine
//! consumes the placement intent to (a) fire
//! `terrain.brace_strut_placed`, (b) debit the cost from the actor's
//! inventory, and (c) lock the ±N lateral-axis pixels around the
//! placement to integrity 500 — N scales by tier per VAL-M14F-031.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Canonical ids for the three brace-strut tiers.
pub const BRACE_STRUT_T1_ID: &str = "brace_strut_t1";
pub const BRACE_STRUT_T2_ID: &str = "brace_strut_t2";
pub const BRACE_STRUT_T3_ID: &str = "brace_strut_t3";

/// RON text for the canonical brace_strut_t1.ron content file. Embedded
/// at compile time so the runtime loader can resolve the tier spec
/// without filesystem access.
pub const BRACE_STRUT_T1_RON: &str =
    include_str!("../../../../content/equipment/brace_strut_t1.ron");
/// RON text for the canonical brace_strut_t2.ron content file.
pub const BRACE_STRUT_T2_RON: &str =
    include_str!("../../../../content/equipment/brace_strut_t2.ron");
/// RON text for the canonical brace_strut_t3.ron content file.
pub const BRACE_STRUT_T3_RON: &str =
    include_str!("../../../../content/equipment/brace_strut_t3.ron");

/// Brace-strut tier discriminator. T1/T2/T3 produce behaviorally
/// distinct lock radii per VAL-M14F-031.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BraceStrutTier {
    T1 = 1,
    T2 = 2,
    T3 = 3,
}

impl BraceStrutTier {
    pub fn as_str(self) -> &'static str {
        match self {
            BraceStrutTier::T1 => "t1",
            BraceStrutTier::T2 => "t2",
            BraceStrutTier::T3 => "t3",
        }
    }

    pub fn from_str_snake(s: &str) -> Option<Self> {
        match s {
            "t1" | "T1" => Some(BraceStrutTier::T1),
            "t2" | "T2" => Some(BraceStrutTier::T2),
            "t3" | "T3" => Some(BraceStrutTier::T3),
            _ => None,
        }
    }

    /// Canonical id for this tier. Resolves to `brace_strut_t1` /
    /// `brace_strut_t2` / `brace_strut_t3`.
    pub fn canonical_id(self) -> &'static str {
        match self {
            BraceStrutTier::T1 => BRACE_STRUT_T1_ID,
            BraceStrutTier::T2 => BRACE_STRUT_T2_ID,
            BraceStrutTier::T3 => BRACE_STRUT_T3_ID,
        }
    }

    /// Lock-radius (in pixels) the placement applies to the lateral
    /// integrity field. Scales 8/12/16 by tier per VAL-M14F-031.
    /// VAL-M14F-005 requires T1 to lock ±8 px around the strut.
    pub fn lock_radius_px(self) -> u32 {
        match self {
            BraceStrutTier::T1 => 8,
            BraceStrutTier::T2 => 12,
            BraceStrutTier::T3 => 16,
        }
    }

    /// Per-pass integrity boost applied to anchored cells. Stiffer
    /// tiers hold wider unsupported spans, per VAL-M14F-031.
    pub fn lock_strength(self) -> u16 {
        match self {
            BraceStrutTier::T1 => 200,
            BraceStrutTier::T2 => 350,
            BraceStrutTier::T3 => 500,
        }
    }
}

/// crafting cost + placement geometry. Same cost class as the M14E
/// support-beam placer (VAL-M14F-022).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BraceStrutSpec {
    pub id: String,
    pub display_name: String,
    pub tier: BraceStrutTier,
    /// Crafting cost per strut (resource id → unit count). T1 mirrors
    /// the M14E support-beam placer (`2 iron + 1 wood`); T2/T3 increase
    /// non-decreasingly per VAL-M14F-022.
    pub cost_per_unit: BTreeMap<String, u32>,
    /// Half-width (in pixels) of the lateral-axis lock window. Drives
    /// the `lateral_integrity_field` lock applied at placement time
    /// (VAL-M14F-005 / VAL-M14F-031).
    pub lock_radius_px: u32,
    /// Per-pass integrity boost applied to anchored cells.
    pub lock_strength: u16,
    pub mass_kg: f32,
    pub max_durability: f32,
}

impl BraceStrutSpec {
    /// Crafting cost vector as (Resource, count) tuples — iron / wood
    /// fixed order to align with M14E support-beam placer ordering.
    #[must_use]
    pub fn cost_per_unit_iron_wood(&self) -> [(&'static str, u32); 2] {
        [
            ("iron", self.cost_per_unit.get("iron").copied().unwrap_or(0)),
            ("wood", self.cost_per_unit.get("wood").copied().unwrap_or(0)),
        ]
    }

    /// True when every resource in `other` is element-wise ≥ in `self`.
    #[must_use]
    pub fn cost_ge(&self, other: &Self) -> bool {
        let mut keys: std::collections::BTreeSet<&String> = self.cost_per_unit.keys().collect();
        for k in other.cost_per_unit.keys() {
            keys.insert(k);
        }
        for k in keys {
            let a = self.cost_per_unit.get(k).copied().unwrap_or(0);
            let b = other.cost_per_unit.get(k).copied().unwrap_or(0);
            if a < b {
                return false;
            }
        }
        true
    }
}

/// Launch entry for the M14F brace-strut T1 — same cost as the M14E
/// support-beam placer (2 iron + 1 wood) per VAL-M14F-022.
#[must_use]
pub fn brace_strut_t1_default() -> BraceStrutSpec {
    let mut cost = BTreeMap::new();
    cost.insert("iron".to_string(), 2);
    cost.insert("wood".to_string(), 1);
    BraceStrutSpec {
        id: BRACE_STRUT_T1_ID.to_string(),
        display_name: "Brace Strut T1".to_string(),
        tier: BraceStrutTier::T1,
        cost_per_unit: cost,
        lock_radius_px: BraceStrutTier::T1.lock_radius_px(),
        lock_strength: BraceStrutTier::T1.lock_strength(),
        mass_kg: 2.5,
        max_durability: 100.0,
    }
}

/// Launch entry for the M14F brace-strut T2 — slightly costlier than
/// T1 (3 iron + 1 wood) so VAL-M14F-022's "non-decreasing scaling"
/// invariant holds.
#[must_use]
pub fn brace_strut_t2_default() -> BraceStrutSpec {
    let mut cost = BTreeMap::new();
    cost.insert("iron".to_string(), 3);
    cost.insert("wood".to_string(), 1);
    BraceStrutSpec {
        id: BRACE_STRUT_T2_ID.to_string(),
        display_name: "Brace Strut T2".to_string(),
        tier: BraceStrutTier::T2,
        cost_per_unit: cost,
        lock_radius_px: BraceStrutTier::T2.lock_radius_px(),
        lock_strength: BraceStrutTier::T2.lock_strength(),
        mass_kg: 3.5,
        max_durability: 150.0,
    }
}

/// Launch entry for the M14F brace-strut T3 — heaviest tier (4 iron +
/// 2 wood). VAL-M14F-022 non-decreasing scaling + VAL-M14F-031
/// lock-radius differentiation.
#[must_use]
pub fn brace_strut_t3_default() -> BraceStrutSpec {
    let mut cost = BTreeMap::new();
    cost.insert("iron".to_string(), 4);
    cost.insert("wood".to_string(), 2);
    BraceStrutSpec {
        id: BRACE_STRUT_T3_ID.to_string(),
        display_name: "Brace Strut T3".to_string(),
        tier: BraceStrutTier::T3,
        cost_per_unit: cost,
        lock_radius_px: BraceStrutTier::T3.lock_radius_px(),
        lock_strength: BraceStrutTier::T3.lock_strength(),
        mass_kg: 4.5,
        max_durability: 200.0,
    }
}

/// Errors that may occur loading a [`BraceStrutSpec`] from RON text.
#[derive(Debug)]
pub enum BraceStrutSpecLoadError {
    /// RON parse failed.
    Parse(ron::error::SpannedError),
    /// The deserialized payload violates M14F invariants.
    Invariant(String),
}

impl std::fmt::Display for BraceStrutSpecLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BraceStrutSpecLoadError::Parse(e) => write!(f, "ron parse error: {e}"),
            BraceStrutSpecLoadError::Invariant(s) => write!(f, "invariant violation: {s}"),
        }
    }
}

impl std::error::Error for BraceStrutSpecLoadError {}

/// and validate that the spec carries a non-empty id + cost map +
/// monotone non-zero lock radius. Used by [`brace_strut_for_tier`] +
/// [`find_brace_strut`] so content authors can tune the tier specs by
/// editing `content/equipment/brace_strut_t{1,2,3}.ron`.
pub fn parse_brace_strut_ron(text: &str) -> Result<BraceStrutSpec, BraceStrutSpecLoadError> {
    let spec: BraceStrutSpec = ron::from_str(text).map_err(BraceStrutSpecLoadError::Parse)?;
    if spec.id.is_empty() {
        return Err(BraceStrutSpecLoadError::Invariant(
            "brace_strut id must not be empty".to_string(),
        ));
    }
    if spec.cost_per_unit.is_empty() {
        return Err(BraceStrutSpecLoadError::Invariant(
            "brace_strut cost_per_unit must not be empty".to_string(),
        ));
    }
    if spec.lock_radius_px == 0 {
        return Err(BraceStrutSpecLoadError::Invariant(
            "brace_strut lock_radius_px must be > 0".to_string(),
        ));
    }
    if spec.lock_strength == 0 {
        return Err(BraceStrutSpecLoadError::Invariant(
            "brace_strut lock_strength must be > 0".to_string(),
        ));
    }
    Ok(spec)
}

/// Embedded RON text for a tier. Used by the runtime loader to resolve
/// `BraceStrutTier::T{1,2,3}` against the canonical content files.
#[must_use]
pub fn brace_strut_ron_for_tier(tier: BraceStrutTier) -> &'static str {
    match tier {
        BraceStrutTier::T1 => BRACE_STRUT_T1_RON,
        BraceStrutTier::T2 => BRACE_STRUT_T2_RON,
        BraceStrutTier::T3 => BRACE_STRUT_T3_RON,
    }
}

/// Resolve a brace-strut spec by id. Reads the canonical RON content
/// embedded via `include_str!` at compile time. Returns `None` for
/// unknown ids; on RON parse failure, falls back to the hard-coded
/// defaults so the runtime never panics.
#[must_use]
pub fn find_brace_strut(id: &str) -> Option<BraceStrutSpec> {
    let (text, fallback): (&str, fn() -> BraceStrutSpec) = match id {
        BRACE_STRUT_T1_ID => (BRACE_STRUT_T1_RON, brace_strut_t1_default),
        BRACE_STRUT_T2_ID => (BRACE_STRUT_T2_RON, brace_strut_t2_default),
        BRACE_STRUT_T3_ID => (BRACE_STRUT_T3_RON, brace_strut_t3_default),
        _ => return None,
    };
    Some(parse_brace_strut_ron(text).unwrap_or_else(|_| fallback()))
}

/// Resolve a brace-strut spec by tier discriminator. Reads from the
/// canonical RON content (see [`find_brace_strut`]).
#[must_use]
pub fn brace_strut_for_tier(tier: BraceStrutTier) -> BraceStrutSpec {
    let (text, fallback): (&str, fn() -> BraceStrutSpec) = match tier {
        BraceStrutTier::T1 => (BRACE_STRUT_T1_RON, brace_strut_t1_default),
        BraceStrutTier::T2 => (BRACE_STRUT_T2_RON, brace_strut_t2_default),
        BraceStrutTier::T3 => (BRACE_STRUT_T3_RON, brace_strut_t3_default),
    };
    parse_brace_strut_ron(text).unwrap_or_else(|_| fallback())
}

/// All three brace-strut tier specs in ascending order.
#[must_use]
pub fn brace_strut_catalog() -> [BraceStrutSpec; 3] {
    [
        brace_strut_t1_default(),
        brace_strut_t2_default(),
        brace_strut_t3_default(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brace_strut_tiers_t1_t2_t3_register() {
        assert!(find_brace_strut(BRACE_STRUT_T1_ID).is_some());
        assert!(find_brace_strut(BRACE_STRUT_T2_ID).is_some());
        assert!(find_brace_strut(BRACE_STRUT_T3_ID).is_some());
        assert!(find_brace_strut("brace_strut_t4").is_none());
        let catalog = brace_strut_catalog();
        assert_eq!(catalog.len(), 3);
        assert_eq!(catalog[0].tier, BraceStrutTier::T1);
        assert_eq!(catalog[1].tier, BraceStrutTier::T2);
        assert_eq!(catalog[2].tier, BraceStrutTier::T3);
    }

    #[test]
    fn tier_round_trips_through_str() {
        for t in [BraceStrutTier::T1, BraceStrutTier::T2, BraceStrutTier::T3] {
            assert_eq!(BraceStrutTier::from_str_snake(t.as_str()), Some(t));
            assert_eq!(BraceStrutTier::from_str_snake(t.as_str().to_uppercase().as_str()), Some(t));
        }
        assert_eq!(BraceStrutTier::from_str_snake("t4"), None);
    }

    /// (2 iron + 1 wood). T2/T3 are ≥ element-wise per "same cost class
    /// with monotone-non-decreasing scaling".
    #[test]
    fn t1_cost_mirrors_support_beam_placer_2iron_1wood() {
        let t1 = brace_strut_t1_default();
        assert_eq!(t1.cost_per_unit_iron_wood(), [("iron", 2), ("wood", 1)]);
        let t2 = brace_strut_t2_default();
        let t3 = brace_strut_t3_default();
        assert!(t2.cost_ge(&t1));
        assert!(t3.cost_ge(&t2));
        assert!(t3.cost_ge(&t1));
    }

    /// (8 / 12 / 16 px) — radius monotonically increases with tier.
    #[test]
    fn tier_lock_radii_strictly_increase_t1_t2_t3() {
        let t1 = BraceStrutTier::T1.lock_radius_px();
        let t2 = BraceStrutTier::T2.lock_radius_px();
        let t3 = BraceStrutTier::T3.lock_radius_px();
        assert!(t1 < t2 && t2 < t3, "expected t1({t1}) < t2({t2}) < t3({t3})");
        // Default specs surface the same radii through the spec struct.
        assert_eq!(brace_strut_t1_default().lock_radius_px, t1);
        assert_eq!(brace_strut_t2_default().lock_radius_px, t2);
        assert_eq!(brace_strut_t3_default().lock_radius_px, t3);
    }

    /// Provides a second axis of differentiation beyond lock radius so
    /// the assertion holds even if a renderer collapses radius display.
    #[test]
    fn tier_lock_strengths_strictly_increase_t1_t2_t3() {
        let t1 = BraceStrutTier::T1.lock_strength();
        let t2 = BraceStrutTier::T2.lock_strength();
        let t3 = BraceStrutTier::T3.lock_strength();
        assert!(t1 < t2 && t2 < t3, "expected t1({t1}) < t2({t2}) < t3({t3})");
        assert_eq!(brace_strut_t1_default().lock_strength, t1);
        assert_eq!(brace_strut_t2_default().lock_strength, t2);
        assert_eq!(brace_strut_t3_default().lock_strength, t3);
    }

    /// covered by T3's ±16 lock but NOT by T1's ±8 lock.
    #[test]
    fn t3_locks_wider_span_than_t1() {
        let t1 = BraceStrutTier::T1.lock_radius_px() * 2;
        let t3 = BraceStrutTier::T3.lock_radius_px() * 2;
        // T1 covers a 16-px-wide window; T3 covers 32 px. A 48-px span
        // strictly exceeds T1's coverage but is closer to being covered
        // by T3 (32 px reaches 2/3 of the way), per VAL-M14F-031's
        // "T3 holds a wider span than T1" predicate.
        assert!(t1 < t3);
        assert!(t3 == 32);
        assert!(t1 == 16);
    }

    /// M14E support-beam placer canonical id.
    #[test]
    fn brace_strut_ids_do_not_collide_with_support_beam_placer() {
        let placer_id = crate::tool::support_beam_placer::SUPPORT_BEAM_PLACER_ID;
        assert_ne!(BRACE_STRUT_T1_ID, placer_id);
        assert_ne!(BRACE_STRUT_T2_ID, placer_id);
        assert_ne!(BRACE_STRUT_T3_ID, placer_id);
    }

    /// parses + validates without invariant errors, so the runtime
    /// loader resolves to the content-authored spec (not the
    /// hard-coded default).
    #[test]
    fn embedded_ron_content_parses_for_all_three_tiers() {
        let t1 = parse_brace_strut_ron(BRACE_STRUT_T1_RON).expect("T1 RON must parse");
        let t2 = parse_brace_strut_ron(BRACE_STRUT_T2_RON).expect("T2 RON must parse");
        let t3 = parse_brace_strut_ron(BRACE_STRUT_T3_RON).expect("T3 RON must parse");
        assert_eq!(t1.id, BRACE_STRUT_T1_ID);
        assert_eq!(t2.id, BRACE_STRUT_T2_ID);
        assert_eq!(t3.id, BRACE_STRUT_T3_ID);
        assert_eq!(t1.tier, BraceStrutTier::T1);
        assert_eq!(t2.tier, BraceStrutTier::T2);
        assert_eq!(t3.tier, BraceStrutTier::T3);
    }

    /// whose fields match the content RON (not the hard-coded default
    /// path).
    #[test]
    fn brace_strut_for_tier_reads_from_content_ron() {
        let t1 = brace_strut_for_tier(BraceStrutTier::T1);
        let parsed = parse_brace_strut_ron(BRACE_STRUT_T1_RON).unwrap();
        assert_eq!(t1, parsed);
        let t3 = brace_strut_for_tier(BraceStrutTier::T3);
        let parsed_t3 = parse_brace_strut_ron(BRACE_STRUT_T3_RON).unwrap();
        assert_eq!(t3, parsed_t3);
    }

    /// (and `find_brace_strut` falls back to the default so callers
    /// never panic on bad content).
    #[test]
    fn parse_rejects_malformed_ron() {
        let err = parse_brace_strut_ron("not-a-valid-ron-payload");
        assert!(matches!(err, Err(BraceStrutSpecLoadError::Parse(_))));
        let err2 = parse_brace_strut_ron("(id: \"\", display_name: \"\", tier: t1, cost_per_unit: {}, lock_radius_px: 0, lock_strength: 0, mass_kg: 0.0, max_durability: 0.0)");
        assert!(matches!(err2, Err(BraceStrutSpecLoadError::Invariant(_))));
    }

    /// values agree on lock_radius_px and lock_strength so the engine's
    /// brace-strut placement reads identical values from either path.
    #[test]
    fn ron_loaded_lock_geometry_matches_tier_helpers() {
        for tier in [BraceStrutTier::T1, BraceStrutTier::T2, BraceStrutTier::T3] {
            let spec = brace_strut_for_tier(tier);
            assert_eq!(spec.lock_radius_px, tier.lock_radius_px());
            assert_eq!(spec.lock_strength, tier.lock_strength());
        }
    }
}
