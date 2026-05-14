//! M6: Welder — slow-cut metal_nohook.

use super::{ToolKind, ToolPreset, WELDER_M6_DEFAULT_ID};

#[must_use]
pub fn welder_m6_default() -> ToolPreset {
    ToolPreset {
        id: WELDER_M6_DEFAULT_ID.to_string(),
        display_name: "Plasma Welder".to_string(),
        kind: ToolKind::Welder,
        wear_per_use: 2.0,
        max_durability: 100.0,
        radius: 4.0,
        spawns_material_id: String::new(),
        heat_generating: true,
        reveals_enemies: false,
        persistent_marker: false,
        mass_kg: 3.5,
    }
}
