//! M6C: C4 charge — manual detonation + remote.

use super::{GrenadeKind, GrenadePreset, C4_CHARGE_ID};

#[must_use]
pub fn c4_charge() -> GrenadePreset {
    GrenadePreset {
        id: C4_CHARGE_ID.to_string(),
        display_name: "C4 Charge".to_string(),
        kind: GrenadeKind::C4Charge,
        fuse_seconds: 0.0,
        radius: 110.0,
        damage_at_center: 320.0,
        adhesive: true,
        spawns_hazard: false,
        vision_disrupt: false,
        mass_kg: 1.5,
        spawn_material_id: String::new(),
        trigger_radius_tiles: 0,
        air_burst: false,
        craftable_t0: false,
        remote_detonated: true,
    }
}
