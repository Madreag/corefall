//! M6: Concrete gun — spawn hardened concrete material.

use super::{ToolKind, ToolPreset, CONCRETE_M6_DEFAULT_ID};

#[must_use]
pub fn concrete_m6_default() -> ToolPreset {
    ToolPreset {
        id: CONCRETE_M6_DEFAULT_ID.to_string(),
        display_name: "Concrete Gun".to_string(),
        kind: ToolKind::Concrete,
        wear_per_use: 1.5,
        max_durability: 100.0,
        radius: 10.0,
        spawns_material_id: "concrete".to_string(),
        heat_generating: false,
        reveals_enemies: false,
        persistent_marker: false,
        mass_kg: 3.0,
    }
}
