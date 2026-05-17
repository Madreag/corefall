//! M6C: Stun Baton — non-lethal + electric jolt.

use super::{MeleeKind, MeleePreset, STUN_BATON_ID};

#[must_use]
pub fn stun_baton() -> MeleePreset {
    MeleePreset {
        id: STUN_BATON_ID.to_string(),
        display_name: "Stun Baton".to_string(),
        kind: MeleeKind::StunBaton,
        damage: 6.0,
        knockdown_chance: 0.85,
        bleed_chance: 0.0,
        reach: 22.0,
        animation_seconds: 0.4,
        damage_kind: "electric".to_string(),
        mass_kg: 1.0,
        requires_host_weapon: false,
        non_lethal_jolt: true,
        can_mine_terrain: false,
        structural_breach: false,
    }
}
