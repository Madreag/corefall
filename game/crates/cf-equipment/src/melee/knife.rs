//! M6: Knife stab — piercing + 25% bleed.

use super::{MeleeKind, MeleePreset, KNIFE_M6_DEFAULT_ID};

#[must_use]
pub fn knife_m6_default() -> MeleePreset {
    MeleePreset {
        id: KNIFE_M6_DEFAULT_ID.to_string(),
        display_name: "Combat Knife".to_string(),
        kind: MeleeKind::Knife,
        damage: 22.0,
        knockdown_chance: 0.05,
        bleed_chance: 0.25,
        reach: 12.0,
        animation_seconds: 0.3,
        damage_kind: "piercing".to_string(),
        mass_kg: 0.3,
    }
}
