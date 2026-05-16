//! M6: Beacon — place persistent map marker (visible to squad).

use super::{ToolKind, ToolPreset, BEACON_M6_DEFAULT_ID};

#[must_use]
pub fn beacon_m6_default() -> ToolPreset {
    ToolPreset {
        id: BEACON_M6_DEFAULT_ID.to_string(),
        display_name: "Map Beacon".to_string(),
        kind: ToolKind::Beacon,
        wear_per_use: 25.0,
        max_durability: 100.0,
        radius: 0.0,
        spawns_material_id: String::new(),
        heat_generating: false,
        reveals_enemies: false,
        persistent_marker: true,
        mass_kg: 0.8,
    }
}
