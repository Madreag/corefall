//! M6: Hatchet — heavy chop, slash damage.

use super::{MeleeKind, MeleePreset, HATCHET_M6_DEFAULT_ID};

#[must_use]
pub fn hatchet_m6_default() -> MeleePreset {
    MeleePreset {
        id: HATCHET_M6_DEFAULT_ID.to_string(),
        display_name: "Hatchet".to_string(),
        kind: MeleeKind::Hatchet,
        damage: 32.0,
        knockdown_chance: 0.15,
        bleed_chance: 0.4,
        reach: 20.0,
        animation_seconds: 0.7,
        damage_kind: "slash".to_string(),
        mass_kg: 1.2,
        requires_host_weapon: false,
        non_lethal_jolt: false,
        can_mine_terrain: false,
        structural_breach: false,
    }
}
