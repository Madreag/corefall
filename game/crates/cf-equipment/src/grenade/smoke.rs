//! M6: Smoke grenade — spawns smoke hazard tiles on detonation.

use super::{GrenadeKind, GrenadePreset, SMOKE_M6_DEFAULT_ID};

#[must_use]
pub fn smoke_m6_default() -> GrenadePreset {
    GrenadePreset {
        id: SMOKE_M6_DEFAULT_ID.to_string(),
        display_name: "Smoke Grenade".to_string(),
        kind: GrenadeKind::Smoke,
        fuse_seconds: 5.0,
        radius: 64.0,
        damage_at_center: 0.0,
        adhesive: false,
        spawns_hazard: true,
        vision_disrupt: false,
        mass_kg: 0.35,
    }
}
