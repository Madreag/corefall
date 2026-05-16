//! M6: Multi-tool — probe material affordances; reveal HUD tooltip.

use super::{ToolKind, ToolPreset, MULTI_TOOL_M6_DEFAULT_ID};

#[must_use]
pub fn multi_tool_m6_default() -> ToolPreset {
    ToolPreset {
        id: MULTI_TOOL_M6_DEFAULT_ID.to_string(),
        display_name: "Multi-Tool".to_string(),
        kind: ToolKind::MultiTool,
        wear_per_use: 0.0,
        max_durability: 100.0,
        radius: 18.0,
        spawns_material_id: String::new(),
        heat_generating: false,
        reveals_enemies: false,
        persistent_marker: false,
        mass_kg: 0.5,
    }
}
