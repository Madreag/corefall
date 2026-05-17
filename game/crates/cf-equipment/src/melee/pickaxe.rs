//! M6C: Pickaxe (combat variant) — melee + mining (T1 hybrid).

use super::{MeleeKind, MeleePreset, PICKAXE_COMBAT_VARIANT_ID};

#[must_use]
pub fn pickaxe_combat_variant() -> MeleePreset {
    MeleePreset {
        id: PICKAXE_COMBAT_VARIANT_ID.to_string(),
        display_name: "Combat Pickaxe".to_string(),
        kind: MeleeKind::Pickaxe,
        damage: 36.0,
        knockdown_chance: 0.15,
        bleed_chance: 0.30,
        reach: 22.0,
        animation_seconds: 0.8,
        damage_kind: "piercing".to_string(),
        mass_kg: 2.4,
        requires_host_weapon: false,
        non_lethal_jolt: false,
        can_mine_terrain: true,
        structural_breach: false,
    }
}
