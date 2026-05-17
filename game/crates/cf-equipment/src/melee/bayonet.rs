//! M6C: Bayonet — rifle attachment.

use super::{MeleeKind, MeleePreset, BAYONET_ID};

#[must_use]
pub fn bayonet() -> MeleePreset {
    MeleePreset {
        id: BAYONET_ID.to_string(),
        display_name: "Bayonet".to_string(),
        kind: MeleeKind::Bayonet,
        damage: 24.0,
        knockdown_chance: 0.05,
        bleed_chance: 0.35,
        reach: 14.0,
        animation_seconds: 0.35,
        damage_kind: "piercing".to_string(),
        mass_kg: 0.4,
        requires_host_weapon: true,
        non_lethal_jolt: false,
        can_mine_terrain: false,
        structural_breach: false,
    }
}
