//! M6C: Sledgehammer — heavy 2-hand; blunt + structural breach.

use super::{MeleeKind, MeleePreset, SLEDGEHAMMER_ID};

#[must_use]
pub fn sledgehammer() -> MeleePreset {
    MeleePreset {
        id: SLEDGEHAMMER_ID.to_string(),
        display_name: "Sledgehammer".to_string(),
        kind: MeleeKind::Sledgehammer,
        damage: 65.0,
        knockdown_chance: 0.65,
        bleed_chance: 0.0,
        reach: 24.0,
        animation_seconds: 1.0,
        damage_kind: "blunt".to_string(),
        mass_kg: 5.0,
        requires_host_weapon: false,
        non_lethal_jolt: false,
        can_mine_terrain: false,
        structural_breach: true,
    }
}
