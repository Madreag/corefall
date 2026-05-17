//! M6: Flash grenade — deafen + blind afflictions.

use super::{GrenadeKind, GrenadePreset, FLASH_M6_DEFAULT_ID};

#[must_use]
pub fn flash_m6_default() -> GrenadePreset {
    GrenadePreset {
        id: FLASH_M6_DEFAULT_ID.to_string(),
        display_name: "Flash Grenade".to_string(),
        kind: GrenadeKind::Flash,
        fuse_seconds: 1.5,
        radius: 80.0,
        damage_at_center: 4.0,
        adhesive: false,
        spawns_hazard: false,
        vision_disrupt: true,
        mass_kg: 0.3,
        spawn_material_id: String::new(),
        trigger_radius_tiles: 0,
        air_burst: false,
        craftable_t0: false,
        remote_detonated: false,
    }
}
