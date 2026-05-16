//! M6: Rifle bash — blunt damage + 30% knockdown.

use super::{MeleeKind, MeleePreset, RIFLE_BASH_M6_DEFAULT_ID};

#[must_use]
pub fn rifle_bash_m6_default() -> MeleePreset {
    MeleePreset {
        id: RIFLE_BASH_M6_DEFAULT_ID.to_string(),
        display_name: "Rifle Bash".to_string(),
        kind: MeleeKind::RifleBash,
        damage: 15.0,
        knockdown_chance: 0.3,
        bleed_chance: 0.0,
        reach: 18.0,
        animation_seconds: 0.5,
        damage_kind: "blunt".to_string(),
        mass_kg: 0.0,
    }
}
