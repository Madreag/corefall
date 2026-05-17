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
}
