//! M6: Frag grenade.

use super::{GrenadeKind, GrenadePreset, FRAG_M6_DEFAULT_ID};

#[must_use]
pub fn frag_m6_default() -> GrenadePreset {
    GrenadePreset {
        id: FRAG_M6_DEFAULT_ID.to_string(),
        display_name: "Frag Grenade".to_string(),
        kind: GrenadeKind::Frag,
        fuse_seconds: 5.0,
        radius: 48.0,
        damage_at_center: 90.0,
        adhesive: false,
        spawns_hazard: false,
        vision_disrupt: false,
        mass_kg: 0.4,
        spawn_material_id: String::new(),
        trigger_radius_tiles: 0,
        air_burst: false,
        craftable_t0: false,
        remote_detonated: false,
    }
}
