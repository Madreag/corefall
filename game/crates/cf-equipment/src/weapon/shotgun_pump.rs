//! M6C: Pump Shotgun — spread + close range devastating.

use super::{WeaponClass, WeaponPreset, SHOTGUN_PUMP_ID};
use crate::fire_modes::AdvancedFireMode;
use crate::{FireMode, RifleSpec, RoundKind};

#[must_use]
pub fn shotgun_pump() -> WeaponPreset {
    let firing = RifleSpec {
        preset_id: SHOTGUN_PUMP_ID.to_string(),
        fire_interval_seconds: 0.95,
        mag_capacity: 6,
        reload_seconds: 0.45,
        recoil_impulse: 75.0,
        muzzle_forward_offset: 16.0,
        muzzle_vertical_offset: 4.0,
        projectile_speed: 880.0,
        damage_per_hit: 10.0,
        projectile_lifetime_seconds: 0.5,
        recoil_decay_rate: 0.08,
        loudness: 1.4,
        inherits_firer_velocity: true,
        particle_count: 10,
        spread_radians: 0.18,
        tracer_round_to_total_ratio: 0,
        ai_fire_vel: 880.0,
        ai_penetration: 0.2,
        ai_life_time: 0.5,
        ai_blast_radius: 0.0,
        fire_mode: FireMode::Semi,
        primary_round: RoundKind::Pellet,
        bullet_mass_kg: 0.028,
        bullet_sharpness: 0.5,
};
    WeaponPreset::new(
        SHOTGUN_PUMP_ID,
        "Pump Shotgun",
        WeaponClass::Shotgun,
        firing,
        vec![AdvancedFireMode::Single, AdvancedFireMode::Pump],
        3.6,
        70.0,
    )
}
