//! M6C: Acid grenade — chemical splash (M15 acid material spawn).

use super::{GrenadeKind, GrenadePreset, ACID_GRENADE_ID};

#[must_use]
pub fn acid_grenade() -> GrenadePreset {
    GrenadePreset {
        id: ACID_GRENADE_ID.to_string(),
        display_name: "Acid Grenade".to_string(),
        kind: GrenadeKind::Acid,
        fuse_seconds: 3.0,
        radius: 60.0,
        damage_at_center: 20.0,
        adhesive: false,
        spawns_hazard: true,
        vision_disrupt: false,
        mass_kg: 0.5,
        spawn_material_id: "acid".to_string(),
        trigger_radius_tiles: 0,
        air_burst: false,
        craftable_t0: false,
        remote_detonated: false,
    }
}
