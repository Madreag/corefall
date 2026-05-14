//! M6: Shotgun preset (8-pellet pump-action, close range).

use super::{WeaponClass, WeaponPreset, SHOTGUN_M6_DEFAULT_ID};
use crate::fire_modes::AdvancedFireMode;
use crate::{FireMode, RifleSpec};

#[must_use]
pub fn shotgun_m6_default() -> WeaponPreset {
    let firing = RifleSpec {
        preset_id: SHOTGUN_M6_DEFAULT_ID.to_string(),
        fire_interval_seconds: 0.85,
        mag_capacity: 8,
        reload_seconds: 2.5,
        recoil_impulse: 60.0,
        muzzle_forward_offset: 14.0,
        muzzle_vertical_offset: 4.0,
        projectile_speed: 900.0,
        damage_per_hit: 8.0,
        projectile_lifetime_seconds: 0.6,
        recoil_decay_rate: 0.07,
        loudness: 1.3,
        inherits_firer_velocity: true,
        particle_count: 8,
        spread_radians: 0.15,
        tracer_round_to_total_ratio: 0,
        ai_fire_vel: 900.0,
        ai_penetration: 0.0,
        ai_life_time: 0.6,
        ai_blast_radius: 0.0,
        fire_mode: FireMode::Semi,
    };
    WeaponPreset::new(
        SHOTGUN_M6_DEFAULT_ID,
        "Combat Shotgun",
        WeaponClass::Shotgun,
        firing,
        vec![AdvancedFireMode::Single, AdvancedFireMode::Pump],
        4.0,
        80.0,
    )
}
