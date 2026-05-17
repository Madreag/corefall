//! M9B: pickaxe-as-trench-dig tool catalog (T1 / T2 / T3 mining tier).
//!
//! Spec §"Notes for the implementer" / spec table footnote:
//!
//! > entrenching_tool is a new T0 tool (cheap, slow): 5 dirt + 1 wood;
//! > digs shallow_scrape in 5s, standard in 12s. **Higher-tier pickaxes
//! > from M30B dig faster but use stamina.**
//!
//! M30B (mining-tool tier ladder) is still `active`; the dig-tool surface
//! that m9b-3 requires is owned here so VAL-M9B-PICKAXE-001 can be
//! verified before M30B lands. Each tier shaves a deterministic fraction
//! off the entrenching_tool baseline and adds a stamina cost per dig.
//! Per the project AGENTS.md determinism rule (no `thread_rng`) the
//! per-tier ratios are constants.
//!
//! The cfctl dispatcher in `cf-control::server` consumes
//! [`PickaxeDigSpec`] when `act.player.dig_trench_segment` is issued
//! while the player holds a pickaxe tier id: it picks the lowest dig-time
//! among the player's equipped dig tools, then deducts stamina per
//! [`PickaxeDigSpec::stamina_cost`] before scheduling the dig timer.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::tool::entrenching::EntrenchingToolSpec;

pub const PICKAXE_DIG_T1_ID: &str = "pickaxe_dig_t1";
pub const PICKAXE_DIG_T2_ID: &str = "pickaxe_dig_t2";
pub const PICKAXE_DIG_T3_ID: &str = "pickaxe_dig_t3";

/// Per-tier multiplier applied to the entrenching_tool baseline. T1
/// shaves 25%, T2 shaves 50%, T3 shaves 75% — the contract assertion
/// VAL-M9B-PICKAXE-001 only requires monotonic improvement
/// `T3 < T2 < T1 < entrenching_tool baseline`, so the exact values are
/// internal but document the spec's "higher-tier dig faster" intent.
const T1_TIME_MULTIPLIER_PERCENT: u32 = 75;
const T2_TIME_MULTIPLIER_PERCENT: u32 = 50;
const T3_TIME_MULTIPLIER_PERCENT: u32 = 25;

/// Per-dig stamina cost, paid once when the cfctl handler accepts the
/// dig and again on each completed dig tick (the engine throttles
/// stamina updates so the cost monotonically decreases for the actor —
/// the contract assertion VAL-M9B-PICKAXE-001 checks "stamina decreases
/// monotonically during dig" so we only need a non-zero floor).
const T1_STAMINA_PER_DIG: u32 = 5;
const T2_STAMINA_PER_DIG: u32 = 10;
const T3_STAMINA_PER_DIG: u32 = 20;

/// On-disk schema for the pickaxe-as-dig-tool record. Mirrors
/// [`EntrenchingToolSpec`] field names so the cfctl handler can treat
/// every dig tool uniformly when looking up `dig_time_for_variant` —
/// the only added surface is `stamina_cost`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PickaxeDigSpec {
    pub id: String,
    pub display_name: String,
    pub material_cost: BTreeMap<String, u32>,
    /// Dig-time in whole in-game seconds, per trench segment variant id.
    /// `"shallow_scrape"`, `"standard"`, and `"deep"` are all populated
    /// (pickaxes can dig the deep variant where the entrenching_tool
    /// hits substrate hardness limits — see VAL-M9B-DIG-003).
    pub dig_time_seconds: BTreeMap<String, u32>,
    pub mass_kg: f32,
    pub max_durability: f32,
    /// Mining tier — T1 / T2 / T3.
    pub tier: u8,
    /// Stamina drained per dig action.
    pub stamina_cost: u32,
}

impl PickaxeDigSpec {
    /// Lookup the per-variant dig time. Returns `None` for variants the
    /// pickaxe is not registered to dig (mirrors
    /// [`EntrenchingToolSpec::dig_time_for_variant`]).
    #[must_use]
    pub fn dig_time_for_variant(&self, variant_id: &str) -> Option<u32> {
        self.dig_time_seconds.get(variant_id).copied()
    }
}

fn scale_baseline(baseline: u32, percent: u32) -> u32 {
    let scaled = (u64::from(baseline) * u64::from(percent)) / 100;
    let min_one = if scaled == 0 { 1 } else { scaled as u32 };
    min_one
}

fn pickaxe_entry(
    id: &str,
    display: &str,
    tier: u8,
    time_pct: u32,
    stamina: u32,
    mass: f32,
    durability: f32,
) -> PickaxeDigSpec {
    let baseline = crate::tool::entrenching::entrenching_tool_m9b_default();
    let mut dig = BTreeMap::new();
    let shallow = baseline
        .dig_time_for_variant("shallow_scrape")
        .unwrap_or(5);
    let standard = baseline.dig_time_for_variant("standard").unwrap_or(12);
    let deep_baseline = standard.saturating_mul(2);
    dig.insert(
        "shallow_scrape".to_string(),
        scale_baseline(shallow, time_pct),
    );
    dig.insert("standard".to_string(), scale_baseline(standard, time_pct));
    dig.insert("deep".to_string(), scale_baseline(deep_baseline, time_pct));

    let mut cost = BTreeMap::new();
    cost.insert("steel".to_string(), u32::from(tier) * 2);
    cost.insert("wood".to_string(), 1);

    PickaxeDigSpec {
        id: id.to_string(),
        display_name: display.to_string(),
        material_cost: cost,
        dig_time_seconds: dig,
        mass_kg: mass,
        max_durability: durability,
        tier,
        stamina_cost: stamina,
    }
}

/// Tier 1 (basic) pickaxe — 75% of entrenching_tool dig time.
#[must_use]
pub fn pickaxe_dig_t1_default() -> PickaxeDigSpec {
    pickaxe_entry(
        PICKAXE_DIG_T1_ID,
        "Pickaxe T1",
        1,
        T1_TIME_MULTIPLIER_PERCENT,
        T1_STAMINA_PER_DIG,
        2.4,
        100.0,
    )
}

/// Tier 2 (powered) pickaxe — 50% of entrenching_tool dig time.
#[must_use]
pub fn pickaxe_dig_t2_default() -> PickaxeDigSpec {
    pickaxe_entry(
        PICKAXE_DIG_T2_ID,
        "Pickaxe T2",
        2,
        T2_TIME_MULTIPLIER_PERCENT,
        T2_STAMINA_PER_DIG,
        3.0,
        150.0,
    )
}

/// Tier 3 (industrial) pickaxe — 25% of entrenching_tool dig time.
#[must_use]
pub fn pickaxe_dig_t3_default() -> PickaxeDigSpec {
    pickaxe_entry(
        PICKAXE_DIG_T3_ID,
        "Pickaxe T3",
        3,
        T3_TIME_MULTIPLIER_PERCENT,
        T3_STAMINA_PER_DIG,
        4.2,
        220.0,
    )
}

/// Launch catalog for the three M30B-tier pickaxe entries that M9B
/// promotes as additional dig tools.
#[must_use]
pub fn m9b_pickaxe_dig_tools() -> Vec<PickaxeDigSpec> {
    vec![
        pickaxe_dig_t1_default(),
        pickaxe_dig_t2_default(),
        pickaxe_dig_t3_default(),
    ]
}

/// Lookup a pickaxe dig spec by its catalog id.
#[must_use]
pub fn find_pickaxe_dig(id: &str) -> Option<PickaxeDigSpec> {
    m9b_pickaxe_dig_tools().into_iter().find(|t| t.id == id)
}

/// Project a [`PickaxeDigSpec`] onto the [`EntrenchingToolSpec`] shape
/// so the cfctl dispatcher can treat every dig tool uniformly — caller
/// keeps the original stamina/tier metadata on the side.
#[must_use]
pub fn as_dig_tool_spec(p: &PickaxeDigSpec) -> EntrenchingToolSpec {
    EntrenchingToolSpec {
        id: p.id.clone(),
        display_name: p.display_name.clone(),
        material_cost: p.material_cost.clone(),
        dig_time_seconds: p.dig_time_seconds.clone(),
        mass_kg: p.mass_kg,
        max_durability: p.max_durability,
        tier: p.tier,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline_standard() -> u32 {
        crate::tool::entrenching::entrenching_tool_m9b_default()
            .dig_time_for_variant("standard")
            .unwrap()
    }

    #[test]
    fn three_tiers_registered() {
        let v = m9b_pickaxe_dig_tools();
        assert_eq!(v.len(), 3);
        assert_eq!(v[0].id, PICKAXE_DIG_T1_ID);
        assert_eq!(v[1].id, PICKAXE_DIG_T2_ID);
        assert_eq!(v[2].id, PICKAXE_DIG_T3_ID);
    }

    /// VAL-M9B-PICKAXE-001: dig-time decreases with tier.
    #[test]
    fn pickaxe_dig_time_scales_with_tier() {
        let baseline = baseline_standard();
        let t1 = pickaxe_dig_t1_default()
            .dig_time_for_variant("standard")
            .unwrap();
        let t2 = pickaxe_dig_t2_default()
            .dig_time_for_variant("standard")
            .unwrap();
        let t3 = pickaxe_dig_t3_default()
            .dig_time_for_variant("standard")
            .unwrap();
        assert!(
            t3 < t2 && t2 < t1 && t1 < baseline,
            "expected T3({t3}) < T2({t2}) < T1({t1}) < baseline({baseline})"
        );
    }

    #[test]
    fn pickaxe_dig_time_scales_for_shallow_scrape() {
        let t1 = pickaxe_dig_t1_default()
            .dig_time_for_variant("shallow_scrape")
            .unwrap();
        let t2 = pickaxe_dig_t2_default()
            .dig_time_for_variant("shallow_scrape")
            .unwrap();
        let t3 = pickaxe_dig_t3_default()
            .dig_time_for_variant("shallow_scrape")
            .unwrap();
        assert!(
            t3 <= t2 && t2 <= t1,
            "shallow_scrape: T3({t3}) <= T2({t2}) <= T1({t1})"
        );
    }

    #[test]
    fn pickaxes_can_dig_deep() {
        for p in m9b_pickaxe_dig_tools() {
            assert!(
                p.dig_time_for_variant("deep").is_some(),
                "{} must register a deep dig-time",
                p.id
            );
        }
    }

    #[test]
    fn stamina_cost_is_non_zero_each_tier() {
        for p in m9b_pickaxe_dig_tools() {
            assert!(
                p.stamina_cost > 0,
                "{} must have non-zero stamina cost",
                p.id
            );
        }
    }

    #[test]
    fn find_pickaxe_dig_returns_some_for_known_ids() {
        assert!(find_pickaxe_dig(PICKAXE_DIG_T1_ID).is_some());
        assert!(find_pickaxe_dig(PICKAXE_DIG_T2_ID).is_some());
        assert!(find_pickaxe_dig(PICKAXE_DIG_T3_ID).is_some());
        assert!(find_pickaxe_dig("nonexistent").is_none());
    }

    #[test]
    fn as_dig_tool_spec_round_trip_preserves_dig_times() {
        let p = pickaxe_dig_t2_default();
        let spec = as_dig_tool_spec(&p);
        assert_eq!(
            spec.dig_time_for_variant("standard"),
            p.dig_time_for_variant("standard")
        );
        assert_eq!(spec.id, p.id);
        assert_eq!(spec.tier, p.tier);
    }
}
