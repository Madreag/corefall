//! M9B: entrenching tool — T0 trench-dig tool.
//!
//! Spec §"Notes for the implementer": "entrenching_tool is a new T0 tool
//! (cheap, slow): 5 dirt + 1 wood; digs shallow_scrape in 5s, standard in
//! 12s. Higher-tier pickaxes from M30B dig faster but use stamina."
//!
//! This module owns the data record + the launch catalog entry. The cfctl
//! handler `act.player.dig_trench_segment` (m9b-3) consumes
//! [`EntrenchingToolSpec`] to gate per-variant dig-time.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Canonical id under which the entrenching tool is registered in the
/// M9B tool catalog (see [`crate::tool::m9b_entrenching_tools`]).
pub const ENTRENCHING_TOOL_ID: &str = "entrenching_tool";

/// Tier-0 (entry-level) entrenching tool. Holds the material cost to
/// craft + the dig-time per trench-segment variant.
///
/// The `dig_time_seconds` map is keyed by [`cf_trench::SegmentVariant::as_str`]
/// values (`"shallow_scrape"`, `"standard"`, etc.) so cfctl handlers can
/// look up per-variant timings without hardcoding the enum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntrenchingToolSpec {
    pub id: String,
    pub display_name: String,
    /// Crafting cost (resource id → unit count). Spec: 5 dirt + 1 wood.
    pub material_cost: BTreeMap<String, u32>,
    /// Dig-time in whole in-game seconds, per trench segment variant id.
    /// `"shallow_scrape": 5`, `"standard": 12`.
    pub dig_time_seconds: BTreeMap<String, u32>,
    pub mass_kg: f32,
    pub max_durability: f32,
    /// Mining tier — entrenching tool is T0 (entry-level). Pickaxes from
    /// M30B occupy T1+; the cfctl dispatcher in m9b-3 picks the lowest-tier
    /// tool capable of the requested variant.
    pub tier: u8,
}

impl EntrenchingToolSpec {
    /// Lookup the spec's declared dig-time for the supplied trench
    /// segment variant id. Returns `None` when the variant is not
    /// registered in `dig_time_seconds` (callers fall back to `shallow_scrape`).
    #[must_use]
    pub fn dig_time_for_variant(&self, variant_id: &str) -> Option<u32> {
        self.dig_time_seconds.get(variant_id).copied()
    }
}

/// Launch entry for the M9B entrenching-tool catalog. Cost + dig-times
/// match the spec §"Notes for the implementer" verbatim.
#[must_use]
pub fn entrenching_tool_m9b_default() -> EntrenchingToolSpec {
    let mut cost = BTreeMap::new();
    cost.insert("dirt".to_string(), 5);
    cost.insert("wood".to_string(), 1);

    let mut dig = BTreeMap::new();
    dig.insert("shallow_scrape".to_string(), 5);
    dig.insert("standard".to_string(), 12);

    EntrenchingToolSpec {
        id: ENTRENCHING_TOOL_ID.to_string(),
        display_name: "Entrenching Tool".to_string(),
        material_cost: cost,
        dig_time_seconds: dig,
        mass_kg: 1.2,
        max_durability: 60.0,
        tier: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entrenching_tool_default_matches_spec_cost_and_timings() {
        let t = entrenching_tool_m9b_default();
        assert_eq!(t.id, ENTRENCHING_TOOL_ID);
        assert_eq!(t.tier, 0);
        assert_eq!(t.material_cost.get("dirt"), Some(&5));
        assert_eq!(t.material_cost.get("wood"), Some(&1));
        assert_eq!(t.material_cost.len(), 2);
        assert_eq!(t.dig_time_for_variant("shallow_scrape"), Some(5));
        assert_eq!(t.dig_time_for_variant("standard"), Some(12));
        assert_eq!(t.dig_time_for_variant("deep"), None);
    }
}
