//! M6: Repair tool — restore terrain integrity + repair equipment.

use super::{ToolKind, ToolPreset, REPAIR_M6_DEFAULT_ID};

#[must_use]
pub fn repair_m6_default() -> ToolPreset {
    ToolPreset {
        id: REPAIR_M6_DEFAULT_ID.to_string(),
        display_name: "Repair Tool".to_string(),
        kind: ToolKind::Repair,
        wear_per_use: 0.5,
        max_durability: 100.0,
        radius: 12.0,
        spawns_material_id: String::new(),
        heat_generating: false,
        reveals_enemies: false,
        persistent_marker: false,
        mass_kg: 1.8,
    }
}
