//! M6C: Pressure mine — trigger on actor weight.

use super::{GrenadeKind, GrenadePreset, PRESSURE_MINE_ID};

#[must_use]
pub fn pressure_mine() -> GrenadePreset {
    GrenadePreset {
        id: PRESSURE_MINE_ID.to_string(),
        display_name: "Pressure Mine".to_string(),
        kind: GrenadeKind::PressureMine,
        fuse_seconds: 0.1,
        radius: 60.0,
        damage_at_center: 145.0,
        adhesive: false,
        spawns_hazard: false,
        vision_disrupt: false,
        mass_kg: 1.6,
        spawn_material_id: String::new(),
        trigger_radius_tiles: 1,
        air_burst: false,
        craftable_t0: false,
        remote_detonated: false,
    }
}
