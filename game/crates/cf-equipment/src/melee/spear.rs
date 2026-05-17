//! M6C: Spear — reach + thrust.

use super::{MeleeKind, MeleePreset, SPEAR_ID};

#[must_use]
pub fn spear() -> MeleePreset {
    MeleePreset {
        id: SPEAR_ID.to_string(),
        display_name: "Spear".to_string(),
        kind: MeleeKind::Spear,
        damage: 38.0,
        knockdown_chance: 0.18,
        bleed_chance: 0.30,
        reach: 36.0,
        animation_seconds: 0.55,
        damage_kind: "piercing".to_string(),
        mass_kg: 1.6,
        requires_host_weapon: false,
        non_lethal_jolt: false,
        can_mine_terrain: false,
        structural_breach: false,
    }
}
