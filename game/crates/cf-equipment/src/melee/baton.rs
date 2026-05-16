//! M6: Baton — high knockdown chance.

use super::{MeleeKind, MeleePreset, BATON_M6_DEFAULT_ID};

#[must_use]
pub fn baton_m6_default() -> MeleePreset {
    MeleePreset {
        id: BATON_M6_DEFAULT_ID.to_string(),
        display_name: "Baton".to_string(),
        kind: MeleeKind::Baton,
        damage: 18.0,
        knockdown_chance: 0.55,
        bleed_chance: 0.0,
        reach: 22.0,
        animation_seconds: 0.45,
        damage_kind: "blunt".to_string(),
        mass_kg: 0.9,
    }
}
