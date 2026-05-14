//! M6: Foam gun — spawn loose_fill material in air space.

use super::{ToolKind, ToolPreset, FOAM_M6_DEFAULT_ID};

#[must_use]
pub fn foam_m6_default() -> ToolPreset {
    ToolPreset {
        id: FOAM_M6_DEFAULT_ID.to_string(),
        display_name: "Foam Gun".to_string(),
        kind: ToolKind::Foam,
        wear_per_use: 1.0,
        max_durability: 100.0,
        radius: 8.0,
        spawns_material_id: "loose_fill".to_string(),
        heat_generating: false,
        reveals_enemies: false,
        persistent_marker: false,
        mass_kg: 2.5,
    }
}
