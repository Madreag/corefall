//! **M14E** § Support-beam placer tool.
//!
//! Spec §"Crates / modules touched":
//! > Add `support_beam_placer` tool (T1; cost 2 iron + 1 wood per beam).
//!
//! And §"Files":
//! > `game/crates/cf-equipment/src/tools.rs` (MODIFY: support_beam_placer tool)
//!
//! The placer writes the `support_beam` material (id=8) into the
//! terrain at the target location. The cf-control engine consumes this
//! tool to fire `terrain.support_beam_placed`, debit `2 iron + 1 wood`
//! from the actor's inventory, and lock the ±8 pixels around the beam
//! to the integrity-field beam-baseline (500).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Canonical id under which the support-beam placer is registered.
pub const SUPPORT_BEAM_PLACER_ID: &str = "support_beam_placer";

/// Tech-tier of the placer. Per spec literal T1.
pub const SUPPORT_BEAM_PLACER_TIER: u8 = 1;

/// cost (`2 iron + 1 wood`) plus the placement geometry. The
/// `placement_pixel_footprint` field is the half-extent (in pixels)
/// around the placer target that the `support_beam` material covers
/// when the tool fires.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SupportBeamPlacerSpec {
    pub id: String,
    pub display_name: String,
    /// Crafting cost per beam (resource id → unit count). Spec literal:
    /// `2 iron + 1 wood per beam`.
    pub cost_per_beam: BTreeMap<String, u32>,
    /// Tech-tier (T1).
    pub tier: u8,
    /// Half-width (in pixels) of the beam footprint that gets written
    /// to the terrain when the placer fires. Default 8 → the beam
    /// covers a 16-pixel span, matching the spec literal "locks the ±8
    /// pixels around the beam".
    pub placement_pixel_footprint_half: u32,
    /// Mass of the placer for inventory carry weight (kg).
    pub mass_kg: f32,
    /// Max durability of the placer (per-use wear is fixed at 0.05 of
    /// `max_durability` per placement; the engine accounts for wear
    /// when it consumes the tool).
    pub max_durability: f32,
}

impl SupportBeamPlacerSpec {
    /// Per-beam crafting cost as an ordered (resource, count) vec for
    /// stable iteration in tests + replay payloads.
    #[must_use]
    pub fn cost_per_beam_ordered(&self) -> Vec<(String, u32)> {
        // BTreeMap iteration is already ordered by key but we want the
        // spec's natural reading order: iron first, then wood.
        let mut out = Vec::with_capacity(self.cost_per_beam.len());
        if let Some(n) = self.cost_per_beam.get("iron") {
            out.push(("iron".to_string(), *n));
        }
        if let Some(n) = self.cost_per_beam.get("wood") {
            out.push(("wood".to_string(), *n));
        }
        for (k, v) in &self.cost_per_beam {
            if k != "iron" && k != "wood" {
                out.push((k.clone(), *v));
            }
        }
        out
    }

    /// Crafting cost vector as (Resource, count) tuples. Used by the
    /// VAL-M14E-012 assertion which calls for
    /// `cost_per_beam == [(Iron, 2), (Wood, 1)]`.
    #[must_use]
    pub fn cost_per_beam_iron_wood(&self) -> [(&'static str, u32); 2] {
        [
            ("iron", self.cost_per_beam.get("iron").copied().unwrap_or(0)),
            ("wood", self.cost_per_beam.get("wood").copied().unwrap_or(0)),
        ]
    }
}

/// Launch entry for the M14E support-beam placer. Per spec: T1,
/// 2 iron + 1 wood per beam, placement footprint covers ±8 pixels.
#[must_use]
pub fn support_beam_placer_m14e_default() -> SupportBeamPlacerSpec {
    let mut cost = BTreeMap::new();
    cost.insert("iron".to_string(), 2);
    cost.insert("wood".to_string(), 1);
    SupportBeamPlacerSpec {
        id: SUPPORT_BEAM_PLACER_ID.to_string(),
        display_name: "Support Beam Placer".to_string(),
        cost_per_beam: cost,
        tier: SUPPORT_BEAM_PLACER_TIER,
        placement_pixel_footprint_half: 8,
        mass_kg: 2.5,
        max_durability: 100.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// per-beam cost `[(Iron, 2), (Wood, 1)]`.
    #[test]
    fn support_beam_placer_default_matches_spec_cost_and_tier() {
        let t = support_beam_placer_m14e_default();
        assert_eq!(t.id, SUPPORT_BEAM_PLACER_ID);
        assert_eq!(t.tier, 1);
        assert_eq!(t.cost_per_beam.get("iron"), Some(&2));
        assert_eq!(t.cost_per_beam.get("wood"), Some(&1));
        assert_eq!(t.cost_per_beam.len(), 2);
        assert_eq!(t.cost_per_beam_iron_wood(), [("iron", 2), ("wood", 1)]);
        assert_eq!(t.placement_pixel_footprint_half, 8);
    }

    /// VAL-M14E-012 cost-ordering invariant: iron precedes wood.
    #[test]
    fn cost_per_beam_iron_wood_order_matches_spec() {
        let t = support_beam_placer_m14e_default();
        let ordered = t.cost_per_beam_ordered();
        assert_eq!(ordered.len(), 2);
        assert_eq!(ordered[0].0, "iron");
        assert_eq!(ordered[1].0, "wood");
    }
}
