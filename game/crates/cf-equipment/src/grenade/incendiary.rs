//! M6C: Incendiary grenade — fire + smoke.

use super::{GrenadeKind, GrenadePreset, INCENDIARY_GRENADE_ID};

#[must_use]
pub fn incendiary_grenade() -> GrenadePreset {
    GrenadePreset {
        id: INCENDIARY_GRENADE_ID.to_string(),
        display_name: "Incendiary Grenade".to_string(),
        kind: GrenadeKind::Incendiary,
        fuse_seconds: 2.0,
        radius: 80.0,
        damage_at_center: 35.0,
        adhesive: false,
        spawns_hazard: true,
        vision_disrupt: false,
        mass_kg: 0.45,
        spawn_material_id: "fire".to_string(),
        trigger_radius_tiles: 0,
        air_burst: false,
        craftable_t0: false,
        remote_detonated: false,
    }
}
