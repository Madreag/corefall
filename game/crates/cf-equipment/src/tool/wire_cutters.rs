//! M9C: wire_cutters — dedicated cut-wire tool.
//!
//! Spec § "Wire cutters is the dedicated tool (M30B T1 tool):
//! equipped + held [E] adjacent to wire for the cut-time-seconds;
//! emits `wire_cut` event with wire_id. Without cutters, an actor can
//! FORCE through (Speed -98% + 8 dmg/tick + likely affliction)."
//!
//! Per-wire cut times (spec table):
//! - `barbed_wire`: 3 s
//! - `razor_wire`: 4 s (cutter takes 1 HP damage)
//! - `electrified_fence` (depowered): 4 s
//! - `concertina_roll`: 4 s per coil section
//!
//! Per-actor cut state lives on the **actor** (`crossing: Option<wire_id>`)
//! per the spec § Notes; this module owns only the tool spec.
//!
//! VAL-M9C-054 + VAL-M9C-033 land here.

use serde::{Deserialize, Serialize};

/// Canonical id under which the wire cutters tool is registered in
/// the M9C tool catalog.
pub const WIRE_CUTTERS_ID: &str = "wire_cutters";

/// On-disk + in-code spec for the wire cutters tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WireCuttersSpec {
    pub id: String,
    pub display_name: String,
    /// Cut time in whole seconds for each wire kind. Keys match the
    /// `WireKind::as_str` values from `cf-fortification::wire`.
    pub cut_time_seconds: std::collections::BTreeMap<String, u32>,
    pub tier: u8,
    pub mass_kg: f32,
    pub max_durability: f32,
}

#[must_use]
pub fn wire_cutters_m9c_default() -> WireCuttersSpec {
    let mut cut = std::collections::BTreeMap::new();
    cut.insert("barbed_wire".to_string(), 3);
    cut.insert("razor_wire".to_string(), 4);
    cut.insert("electrified_fence".to_string(), 4);
    cut.insert("concertina_roll".to_string(), 4);
    WireCuttersSpec {
        id: WIRE_CUTTERS_ID.to_string(),
        display_name: "Wire Cutters".to_string(),
        cut_time_seconds: cut,
        tier: 1,
        mass_kg: 0.8,
        max_durability: 60.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_cutters_default_matches_spec_table() {
        let w = wire_cutters_m9c_default();
        assert_eq!(w.id, WIRE_CUTTERS_ID);
        assert_eq!(w.tier, 1);
        assert_eq!(w.cut_time_seconds.get("barbed_wire"), Some(&3));
        assert_eq!(w.cut_time_seconds.get("razor_wire"), Some(&4));
        assert_eq!(w.cut_time_seconds.get("electrified_fence"), Some(&4));
        assert_eq!(w.cut_time_seconds.get("concertina_roll"), Some(&4));
    }
}
