//! M6C: Pipe bomb — improvised; craftable.

use super::{GrenadeKind, GrenadePreset, PIPE_BOMB_ID};

#[must_use]
pub fn pipe_bomb() -> GrenadePreset {
    GrenadePreset {
        id: PIPE_BOMB_ID.to_string(),
        display_name: "Pipe Bomb".to_string(),
        kind: GrenadeKind::PipeBomb,
        fuse_seconds: 6.0,
        radius: 56.0,
        damage_at_center: 95.0,
        adhesive: false,
        spawns_hazard: false,
        vision_disrupt: false,
        mass_kg: 0.8,
        spawn_material_id: String::new(),
        trigger_radius_tiles: 0,
        air_burst: false,
        craftable_t0: true,
        remote_detonated: false,
    }
}
