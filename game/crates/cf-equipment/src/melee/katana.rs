//! M6C: Katana — long blade + high damage.

use super::{MeleeKind, MeleePreset, KATANA_ID};

#[must_use]
pub fn katana() -> MeleePreset {
    MeleePreset {
        id: KATANA_ID.to_string(),
        display_name: "Katana".to_string(),
        kind: MeleeKind::Katana,
        damage: 55.0,
        knockdown_chance: 0.10,
        bleed_chance: 0.55,
        reach: 28.0,
        animation_seconds: 0.65,
        damage_kind: "slash".to_string(),
        mass_kg: 1.1,
        requires_host_weapon: false,
        non_lethal_jolt: false,
        can_mine_terrain: false,
        structural_breach: false,
    }
}
