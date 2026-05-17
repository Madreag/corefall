//! M6C: Tripwire mine — trigger on cross.

use super::{GrenadeKind, GrenadePreset, TRIPWIRE_MINE_ID};

#[must_use]
pub fn tripwire_mine() -> GrenadePreset {
    GrenadePreset {
        id: TRIPWIRE_MINE_ID.to_string(),
        display_name: "Tripwire Mine".to_string(),
        kind: GrenadeKind::TripwireMine,
        fuse_seconds: 0.0,
        radius: 70.0,
        damage_at_center: 110.0,
        adhesive: false,
        spawns_hazard: false,
        vision_disrupt: false,
        mass_kg: 1.0,
        spawn_material_id: String::new(),
        trigger_radius_tiles: 6,
        air_burst: false,
        craftable_t0: true,
        remote_detonated: false,
    }
}
