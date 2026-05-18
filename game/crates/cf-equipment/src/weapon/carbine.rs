//! M6C: Compact Carbine — short barrel + carbine.

use super::{WeaponClass, WeaponPreset, CARBINE_COMPACT_ID};
use crate::fire_modes::AdvancedFireMode;
use crate::{FireMode, RifleSpec, RoundKind};

#[must_use]
pub fn carbine_compact() -> WeaponPreset {
    let firing = RifleSpec {
        preset_id: CARBINE_COMPACT_ID.to_string(),
        fire_interval_seconds: 0.085,
        mag_capacity: 30,
        reload_seconds: 1.4,
        recoil_impulse: 22.0,
        muzzle_forward_offset: 11.0,
        muzzle_vertical_offset: 4.0,
        projectile_speed: 1250.0,
        damage_per_hit: 11.0,
        projectile_lifetime_seconds: 1.2,
        recoil_decay_rate: 0.05,
        loudness: 1.0,
        inherits_firer_velocity: true,
        particle_count: 1,
        spread_radians: 0.02,
        tracer_round_to_total_ratio: 5,
        ai_fire_vel: 1250.0,
        ai_penetration: 0.3,
        ai_life_time: 1.2,
        ai_blast_radius: 0.0,
        fire_mode: FireMode::FullAuto,
        primary_round: RoundKind::Regular,
    };
    WeaponPreset::new(
        CARBINE_COMPACT_ID,
        "Compact Carbine",
        WeaponClass::Carbine,
        firing,
        vec![
            AdvancedFireMode::Single,
            AdvancedFireMode::Burst3,
            AdvancedFireMode::Auto,
        ],
        2.9,
        220.0,
    )
}
