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
pub mod concrete;
pub mod dig_pickaxe;
pub mod drill;
pub mod entrenching;
pub mod foam;
pub mod multi_tool;
pub mod repair;
pub mod sensor_pulse;
pub mod welder;

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

/// **M9B**: T0 entrenching-tool catalog. The cf-equipment tool surface
/// keeps the M6 `ToolPreset` schema (no cost/dig-time fields) untouched
/// and exposes M9B-specific dig tools via this parallel catalog so the
/// cfctl handler `act.player.dig_trench_segment` (m9b-3) can look up
/// material cost + per-variant dig-time without churning M6 callers.
#[must_use]
pub fn m9b_entrenching_tools() -> Vec<entrenching::EntrenchingToolSpec> {
    vec![entrenching::entrenching_tool_m9b_default()]
}

/// **M9B**: lookup an entrenching-tool spec by its catalog id. Returns
/// `None` for unknown ids so cfctl can surface a structured error
/// rather than panic.
#[must_use]
pub fn find_entrenching_tool(id: &str) -> Option<entrenching::EntrenchingToolSpec> {
    m9b_entrenching_tools().into_iter().find(|t| t.id == id)
}

/// **M9B**: union catalog of every dig-tool registered for M9B trench
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

/// **M9B**: lookup any registered dig-tool spec (entrenching_tool or
/// pickaxe T1/T2/T3) by catalog id.
#[must_use]
pub fn find_m9b_dig_tool(id: &str) -> Option<entrenching::EntrenchingToolSpec> {
    if let Some(spec) = find_entrenching_tool(id) {
        return Some(spec);
    }
    dig_pickaxe::find_pickaxe_dig(id).map(|p| dig_pickaxe::as_dig_tool_spec(&p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn at_least_seven_tools() {
        let v = m6_tool_presets();
        assert!(v.len() >= 7);
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

    /// VAL-M9B-PICKAXE-001: pickaxe-tier dig times are strictly faster
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

    /// VAL-M9B-PICKAXE-001: pickaxes register a `deep` dig time the
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

    /// VAL-M9B-PICKAXE-001: every pickaxe declares non-zero stamina cost.
    #[test]
    fn pickaxes_have_stamina_cost() {
        for p in dig_pickaxe::m9b_pickaxe_dig_tools() {
            assert!(p.stamina_cost > 0, "{} stamina cost must be > 0", p.id);
        }
    }
}
