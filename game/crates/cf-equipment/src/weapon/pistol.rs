//! M6: Pistol preset (sidearm; faster swap; 12-round mag).

use super::{WeaponClass, WeaponPreset, PISTOL_M6_DEFAULT_ID};
use crate::fire_modes::AdvancedFireMode;
use crate::{FireMode, RifleSpec, RoundKind};

#[must_use]
pub fn pistol_m6_default() -> WeaponPreset {
    let firing = RifleSpec {
        preset_id: PISTOL_M6_DEFAULT_ID.to_string(),
        fire_interval_seconds: 0.18,
        mag_capacity: 12,
        reload_seconds: 1.1,
        recoil_impulse: 12.0,
        muzzle_forward_offset: 8.0,
        muzzle_vertical_offset: 3.0,
        projectile_speed: 950.0,
        damage_per_hit: 14.0,
        projectile_lifetime_seconds: 1.0,
        recoil_decay_rate: 0.05,
        loudness: 0.9,
        inherits_firer_velocity: true,
        particle_count: 1,
        spread_radians: 0.01,
        tracer_round_to_total_ratio: 0,
        ai_fire_vel: 950.0,
        ai_penetration: 0.0,
        ai_life_time: 1.0,
        ai_blast_radius: 0.0,
        fire_mode: FireMode::Semi,
        primary_round: RoundKind::Regular,
        bullet_mass_kg: 0.0082,
        bullet_sharpness: 0.75,
};
    WeaponPreset::new(
        PISTOL_M6_DEFAULT_ID,
        "Service Pistol",
        WeaponClass::Pistol,
        firing,
        vec![AdvancedFireMode::Single],
        1.0,
        120.0,
    )
}
