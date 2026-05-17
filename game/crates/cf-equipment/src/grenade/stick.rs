//! M6: Stick grenade — adheres to actor/surface, 4 s fuse.

use super::{GrenadeKind, GrenadePreset, STICK_M6_DEFAULT_ID};

#[must_use]
pub fn stick_m6_default() -> GrenadePreset {
    GrenadePreset {
        id: STICK_M6_DEFAULT_ID.to_string(),
        display_name: "Sticky Grenade".to_string(),
        kind: GrenadeKind::Stick,
        fuse_seconds: 4.0,
        radius: 36.0,
        damage_at_center: 75.0,
        adhesive: true,
        spawns_hazard: false,
        vision_disrupt: false,
        mass_kg: 0.5,
        spawn_material_id: String::new(),
        trigger_radius_tiles: 0,
        air_burst: false,
        craftable_t0: false,
        remote_detonated: false,
    }
}
