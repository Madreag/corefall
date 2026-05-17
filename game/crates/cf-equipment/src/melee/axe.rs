//! M6C: Axe/Hatchet — split wood + melee.

use super::{MeleeKind, MeleePreset, AXE_HATCHET_ID};

#[must_use]
pub fn axe_hatchet() -> MeleePreset {
    MeleePreset {
        id: AXE_HATCHET_ID.to_string(),
        display_name: "Felling Axe".to_string(),
        kind: MeleeKind::Axe,
        damage: 42.0,
        knockdown_chance: 0.20,
        bleed_chance: 0.45,
        reach: 22.0,
        animation_seconds: 0.85,
        damage_kind: "slash".to_string(),
        mass_kg: 1.8,
        requires_host_weapon: false,
        non_lethal_jolt: false,
        can_mine_terrain: true,
        structural_breach: false,
    }
}
