//! M6C: Molotov cocktail — fire spread (M15 fire material).

use super::{GrenadeKind, GrenadePreset, MOLOTOV_COCKTAIL_ID};

#[must_use]
pub fn molotov_cocktail() -> GrenadePreset {
    GrenadePreset {
        id: MOLOTOV_COCKTAIL_ID.to_string(),
        display_name: "Molotov Cocktail".to_string(),
        kind: GrenadeKind::Molotov,
        fuse_seconds: 0.0,
        radius: 80.0,
        damage_at_center: 15.0,
        adhesive: false,
        spawns_hazard: true,
        vision_disrupt: false,
        mass_kg: 0.6,
        spawn_material_id: "fire".to_string(),
        trigger_radius_tiles: 0,
        air_burst: false,
        craftable_t0: true,
        remote_detonated: false,
    }
}
