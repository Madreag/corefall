//! M6C: Bouncing Betty — anti-personnel + air-burst.

use super::{GrenadeKind, GrenadePreset, BOUNCING_BETTY_ID};

#[must_use]
pub fn bouncing_betty() -> GrenadePreset {
    GrenadePreset {
        id: BOUNCING_BETTY_ID.to_string(),
        display_name: "Bouncing Betty".to_string(),
        kind: GrenadeKind::BouncingBetty,
        fuse_seconds: 0.3,
        radius: 90.0,
        damage_at_center: 180.0,
        adhesive: false,
        spawns_hazard: false,
        vision_disrupt: false,
        mass_kg: 1.8,
        spawn_material_id: String::new(),
        trigger_radius_tiles: 3,
        air_burst: true,
        craftable_t0: false,
        remote_detonated: false,
    }
}
