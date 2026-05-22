//! M6: tools (7 launch tools extending M1's digger).
//!
//! Per spec § "7 tools":
//! - Digger (M1; preserved)
//! - Repair tool — restore terrain integrity + repair equipment
//! - Foam gun — spawn loose_fill material
//! - Concrete gun — spawn hardened concrete
//! - Welder — cut metal_nohook
//! - Drill — 2x faster than digger; overheats
//! - Multi-tool — probe material affordances
//! - Beacon — drop persistent map marker
//! - Sensor pulse — reveal enemies / hazards in radius

pub mod beacon;
pub mod brace_strut;
pub mod concrete;
pub mod dig_pickaxe;
pub mod drill;
pub mod engineering_tool;
pub mod entrenching;
pub mod foam;
pub mod minesweeper;
pub mod multi_tool;
pub mod repair;
pub mod sensor_pulse;
pub mod support_beam_placer;
pub mod welder;
pub mod wire_cutters;

use serde::{Deserialize, Serialize};

pub const REPAIR_M6_DEFAULT_ID: &str = "tool_repair_m6";
pub const FOAM_M6_DEFAULT_ID: &str = "tool_foam_m6";
pub const CONCRETE_M6_DEFAULT_ID: &str = "tool_concrete_m6";
pub const WELDER_M6_DEFAULT_ID: &str = "tool_welder_m6";
pub const DRILL_M6_DEFAULT_ID: &str = "tool_drill_m6";
pub const MULTI_TOOL_M6_DEFAULT_ID: &str = "tool_multi_tool_m6";
pub const BEACON_M6_DEFAULT_ID: &str = "tool_beacon_m6";
pub const SENSOR_PULSE_M6_DEFAULT_ID: &str = "tool_sensor_pulse_m6";

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    /// M1 baseline — kept for compat / reference.
    Digger = 0,
    Repair = 1,
    Foam = 2,
    Concrete = 3,
    Welder = 4,
    Drill = 5,
    MultiTool = 6,
    Beacon = 7,
    SensorPulse = 8,
}

impl ToolKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ToolKind::Digger => "digger",
            ToolKind::Repair => "repair",
            ToolKind::Foam => "foam",
            ToolKind::Concrete => "concrete",
            ToolKind::Welder => "welder",
            ToolKind::Drill => "drill",
            ToolKind::MultiTool => "multi_tool",
            ToolKind::Beacon => "beacon",
            ToolKind::SensorPulse => "sensor_pulse",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolPreset {
    pub id: String,
    pub display_name: String,
    pub kind: ToolKind,
    /// Per-use wear (0..1 of full durability per activation).
    pub wear_per_use: f32,
    /// Max durability (defaults to 100).
    pub max_durability: f32,
    /// Effective radius in world units for spread tools.
    pub radius: f32,
    /// Spawn material id for spray tools ("loose_fill", "concrete", "").
    pub spawns_material_id: String,
    /// True if generates heat (Drill).
    pub heat_generating: bool,
    /// True if reveals enemies in radius (SensorPulse).
    pub reveals_enemies: bool,
    /// True if places persistent marker (Beacon).
    pub persistent_marker: bool,
    pub mass_kg: f32,
}

#[must_use]
pub fn m6_tool_presets() -> Vec<ToolPreset> {
    vec![
        repair::repair_m6_default(),
        foam::foam_m6_default(),
        concrete::concrete_m6_default(),
        welder::welder_m6_default(),
        drill::drill_m6_default(),
        multi_tool::multi_tool_m6_default(),
        beacon::beacon_m6_default(),
        sensor_pulse::sensor_pulse_m6_default(),
    ]
}

/// keeps the M6 `ToolPreset` schema (no cost/dig-time fields) untouched
/// and exposes M9B-specific dig tools via this parallel catalog so the
/// cfctl handler `act.player.dig_trench_segment` (m9b-3) can look up
/// material cost + per-variant dig-time without churning M6 callers.
#[must_use]
pub fn m9b_entrenching_tools() -> Vec<entrenching::EntrenchingToolSpec> {
    vec![entrenching::entrenching_tool_m9b_default()]
}

/// `None` for unknown ids so cfctl can surface a structured error
/// rather than panic.
#[must_use]
pub fn find_entrenching_tool(id: &str) -> Option<entrenching::EntrenchingToolSpec> {
    m9b_entrenching_tools().into_iter().find(|t| t.id == id)
}

/// carving — entrenching_tool (T0) plus the three M30B-tier pickaxes
/// (T1 / T2 / T3). Returned as a sorted-by-`tier` list so the cfctl
/// dispatcher picks the lowest dig time among the player's equipped
/// tools deterministically. Each entry is projected onto the canonical
/// [`entrenching::EntrenchingToolSpec`] shape; the pickaxe-specific
/// stamina cost is queried separately via [`dig_pickaxe::find_pickaxe_dig`].
#[must_use]
pub fn m9b_dig_tools_all() -> Vec<entrenching::EntrenchingToolSpec> {
    let mut all: Vec<entrenching::EntrenchingToolSpec> = m9b_entrenching_tools();
    for p in dig_pickaxe::m9b_pickaxe_dig_tools() {
        all.push(dig_pickaxe::as_dig_tool_spec(&p));
    }
    all.sort_by_key(|t| t.tier);
    all
}

/// pickaxe T1/T2/T3) by catalog id.
#[must_use]
pub fn find_m9b_dig_tool(id: &str) -> Option<entrenching::EntrenchingToolSpec> {
    if let Some(spec) = find_entrenching_tool(id) {
        return Some(spec);
    }
    dig_pickaxe::find_pickaxe_dig(id).map(|p| dig_pickaxe::as_dig_tool_spec(&p))
}

/// § "Crates / modules touched": `minesweeper`, `wire_cutters`,
/// `engineering_tool`. The entrenching_tool is shared with M9B and is
/// catalogued separately via [`m9b_entrenching_tools`].
#[derive(Debug, Clone, PartialEq)]
pub struct M9cToolCatalog {
    pub minesweeper: minesweeper::MinesweeperToolSpec,
    pub wire_cutters: wire_cutters::WireCuttersSpec,
    pub engineering_tool: engineering_tool::EngineeringToolSpec,
}

/// Default M9C tool catalog used by the cfctl handlers + AI
/// engineer-doctrine consumer.
#[must_use]
pub fn m9c_tool_catalog() -> M9cToolCatalog {
    M9cToolCatalog {
        minesweeper: minesweeper::minesweeper_m9c_default(),
        wire_cutters: wire_cutters::wire_cutters_m9c_default(),
        engineering_tool: engineering_tool::engineering_tool_m9c_default(),
    }
}

/// True when the supplied tool id maps to one of the M9C fortification
/// tools (minesweeper / wire_cutters / engineering_tool).
#[must_use]
pub fn is_m9c_tool(id: &str) -> bool {
    matches!(
        id,
        minesweeper::MINESWEEPER_ID
            | wire_cutters::WIRE_CUTTERS_ID
            | engineering_tool::ENGINEERING_TOOL_ID
    )
}

/// `support_beam` material at the placement target. Returns the
/// single registered placer instance.
#[must_use]
pub fn m14e_support_beam_placer() -> support_beam_placer::SupportBeamPlacerSpec {
    support_beam_placer::support_beam_placer_m14e_default()
}

/// unknown ids so cfctl handlers can emit a structured error.
#[must_use]
pub fn find_support_beam_placer(id: &str) -> Option<support_beam_placer::SupportBeamPlacerSpec> {
    if id == support_beam_placer::SUPPORT_BEAM_PLACER_ID {
        Some(support_beam_placer::support_beam_placer_m14e_default())
    } else {
        None
    }
}

/// brace-strut tier items (T1 / T2 / T3).
#[must_use]
pub fn is_brace_strut_tool(id: &str) -> bool {
    matches!(
        id,
        brace_strut::BRACE_STRUT_T1_ID | brace_strut::BRACE_STRUT_T2_ID | brace_strut::BRACE_STRUT_T3_ID
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn at_least_seven_tools() {
        let v = m6_tool_presets();
        assert!(v.len() >= 7);
    }

    /// entrenching_tool are all registered + reachable from the
    /// equipment catalog. (entrenching_tool ships in M9B.)
    #[test]
    fn tools_m9c_registered() {
        let cat = m9c_tool_catalog();
        assert_eq!(cat.minesweeper.id, minesweeper::MINESWEEPER_ID);
        assert_eq!(cat.wire_cutters.id, wire_cutters::WIRE_CUTTERS_ID);
        assert_eq!(cat.engineering_tool.id, engineering_tool::ENGINEERING_TOOL_ID);
        assert!(is_m9c_tool("minesweeper"));
        assert!(is_m9c_tool("wire_cutters"));
        assert!(is_m9c_tool("engineering_tool"));
        assert!(!is_m9c_tool("entrenching_tool"));
        // VAL-M9B parity: entrenching_tool ships in M9B; covered by
        // the existing `entrenching_tool_registered` test.
        assert!(find_entrenching_tool(entrenching::ENTRENCHING_TOOL_ID).is_some());
    }

    #[test]
    fn drill_heat_generating() {
        let v = m6_tool_presets();
        let d = v.iter().find(|t| t.kind == ToolKind::Drill).unwrap();
        assert!(d.heat_generating);
    }

    #[test]
    fn sensor_pulse_reveals() {
        let v = m6_tool_presets();
        let s = v.iter().find(|t| t.kind == ToolKind::SensorPulse).unwrap();
        assert!(s.reveals_enemies);
    }

    #[test]
    fn beacon_persistent() {
        let v = m6_tool_presets();
        let b = v.iter().find(|t| t.kind == ToolKind::Beacon).unwrap();
        assert!(b.persistent_marker);
    }

    /// than the entrenching_tool baseline (T3 < T2 < T1 < entrenching).
    #[test]
    fn pickaxe_dig_time_scales_with_tier_in_catalog() {
        let all = m9b_dig_tools_all();
        // Sorted by tier ascending: entrenching(0), pickaxe T1, T2, T3.
        let by_id: std::collections::BTreeMap<&str, &entrenching::EntrenchingToolSpec> =
            all.iter().map(|t| (t.id.as_str(), t)).collect();
        let baseline = by_id
            .get(entrenching::ENTRENCHING_TOOL_ID)
            .expect("entrenching_tool present")
            .dig_time_for_variant("standard")
            .expect("baseline standard dig time");
        let t1 = by_id
            .get(dig_pickaxe::PICKAXE_DIG_T1_ID)
            .unwrap()
            .dig_time_for_variant("standard")
            .unwrap();
        let t2 = by_id
            .get(dig_pickaxe::PICKAXE_DIG_T2_ID)
            .unwrap()
            .dig_time_for_variant("standard")
            .unwrap();
        let t3 = by_id
            .get(dig_pickaxe::PICKAXE_DIG_T3_ID)
            .unwrap()
            .dig_time_for_variant("standard")
            .unwrap();
        assert!(
            t3 < t2 && t2 < t1 && t1 < baseline,
            "expected T3({t3}) < T2({t2}) < T1({t1}) < entrenching({baseline})"
        );
    }

    /// entrenching_tool does not — the pickaxe is the only tool that
    /// can attempt the `deep` variant.
    #[test]
    fn pickaxes_register_deep_variant() {
        for tier_id in [
            dig_pickaxe::PICKAXE_DIG_T1_ID,
            dig_pickaxe::PICKAXE_DIG_T2_ID,
            dig_pickaxe::PICKAXE_DIG_T3_ID,
        ] {
            let spec = find_m9b_dig_tool(tier_id).expect("pickaxe registered");
            assert!(
                spec.dig_time_for_variant("deep").is_some(),
                "{tier_id} must register `deep` dig time"
            );
        }
        // The T0 entrenching_tool intentionally does NOT register `deep` —
        // the cfctl handler falls back to shallow_scrape on hard substrate
        // for the entrenching_tool path (VAL-M9B-DIG-003).
        let baseline = find_m9b_dig_tool(entrenching::ENTRENCHING_TOOL_ID).unwrap();
        assert!(baseline.dig_time_for_variant("deep").is_none());
    }

    #[test]
    fn pickaxes_have_stamina_cost() {
        for p in dig_pickaxe::m9b_pickaxe_dig_tools() {
            assert!(p.stamina_cost > 0, "{} stamina cost must be > 0", p.id);
        }
    }

    /// canonical per-beam cost `2 iron + 1 wood`.
    #[test]
    fn support_beam_placer_registered_at_t1_with_iron_wood_cost() {
        let placer = m14e_support_beam_placer();
        assert_eq!(placer.id, support_beam_placer::SUPPORT_BEAM_PLACER_ID);
        assert_eq!(placer.tier, 1);
        let cost = placer.cost_per_beam_iron_wood();
        assert_eq!(cost, [("iron", 2), ("wood", 1)]);
    }

    /// slot id from the M14F brace_strut catalog (no collision).
    #[test]
    fn support_beam_placer_lookup_finds_canonical_id() {
        assert!(find_support_beam_placer("support_beam_placer").is_some());
        assert!(find_support_beam_placer("unknown_tool").is_none());
        // Distinct slot id check: support_beam_placer must NOT collide
        // with any registered M9C tool.
        assert!(!is_m9c_tool("support_beam_placer"));
    }

    /// VAL-M14F-017 + VAL-CROSS-023: brace-strut T1/T2/T3 register at
    /// distinct ids from the support-beam placer.
    #[test]
    fn brace_strut_tiers_register_distinct_from_support_beam_placer() {
        assert!(brace_strut::find_brace_strut(brace_strut::BRACE_STRUT_T1_ID).is_some());
        assert!(brace_strut::find_brace_strut(brace_strut::BRACE_STRUT_T2_ID).is_some());
        assert!(brace_strut::find_brace_strut(brace_strut::BRACE_STRUT_T3_ID).is_some());

        let placer_id = support_beam_placer::SUPPORT_BEAM_PLACER_ID;
        assert_ne!(placer_id, brace_strut::BRACE_STRUT_T1_ID);
        assert_ne!(placer_id, brace_strut::BRACE_STRUT_T2_ID);
        assert_ne!(placer_id, brace_strut::BRACE_STRUT_T3_ID);

        // The brace-strut catalog must not collide with the M14E placer
        // material id (= 8 = support_beam). We surface tier IDs as
        // distinct strings — the engine slots them in unique inventory
        // entries.
        assert!(is_brace_strut_tool(brace_strut::BRACE_STRUT_T1_ID));
        assert!(is_brace_strut_tool(brace_strut::BRACE_STRUT_T2_ID));
        assert!(is_brace_strut_tool(brace_strut::BRACE_STRUT_T3_ID));
        assert!(!is_brace_strut_tool(placer_id));
        assert!(!is_brace_strut_tool("unknown_tool"));
    }

    /// (2 iron + 1 wood per VAL-M14F-022) and does NOT debit a
    /// `support_beam_placer` resource.
    #[test]
    fn brace_strut_t1_cost_matches_support_beam_placer_cost_class() {
        let placer = m14e_support_beam_placer();
        let t1 = brace_strut::brace_strut_t1_default();
        let placer_cost = placer.cost_per_beam_iron_wood();
        let t1_cost = t1.cost_per_unit_iron_wood();
        assert_eq!(placer_cost, t1_cost);
    }
}
