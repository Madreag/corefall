//! M6C: Combat Dagger — fast + light.

use super::{MeleeKind, MeleePreset, DAGGER_COMBAT_ID};

#[must_use]
pub fn dagger_combat() -> MeleePreset {
    MeleePreset {
        id: DAGGER_COMBAT_ID.to_string(),
        display_name: "Combat Dagger".to_string(),
        kind: MeleeKind::Dagger,
        damage: 15.0,
        knockdown_chance: 0.02,
        bleed_chance: 0.35,
        reach: 9.0,
        animation_seconds: 0.18,
        damage_kind: "piercing".to_string(),
        mass_kg: 0.2,
        requires_host_weapon: false,
        non_lethal_jolt: false,
        can_mine_terrain: false,
        structural_breach: false,
    }
}
