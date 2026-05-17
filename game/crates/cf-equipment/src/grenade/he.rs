//! M6C: HE grenade — high explosive (M14C consumer; large radius).

use super::{GrenadeKind, GrenadePreset, HE_GRENADE_ID};

#[must_use]
pub fn he_grenade() -> GrenadePreset {
    GrenadePreset {
        id: HE_GRENADE_ID.to_string(),
        display_name: "HE Grenade".to_string(),
        kind: GrenadeKind::HighExplosive,
        fuse_seconds: 4.0,
        radius: 96.0,
        damage_at_center: 160.0,
        adhesive: false,
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
