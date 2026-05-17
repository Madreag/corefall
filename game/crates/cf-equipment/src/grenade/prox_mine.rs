//! M6C-7: Proximity mine — auto-trigger on hostile approach (4-tile radius).

use super::{GrenadeKind, GrenadePreset, PROXIMITY_MINE_ID};
use super::mines::PROXIMITY_TRIGGER_RADIUS_TILES;

#[must_use]
pub fn proximity_mine() -> GrenadePreset {
    GrenadePreset {
        id: PROXIMITY_MINE_ID.to_string(),
        display_name: "Proximity Mine".to_string(),
        kind: GrenadeKind::ProximityMine,
        fuse_seconds: 0.5,
        radius: 64.0,
        damage_at_center: 130.0,
        adhesive: false,
        spawns_hazard: false,
        vision_disrupt: false,
        mass_kg: 1.4,
        spawn_material_id: String::new(),
        trigger_radius_tiles: PROXIMITY_TRIGGER_RADIUS_TILES,
        air_burst: false,
        craftable_t0: false,
        remote_detonated: false,
    }
}
