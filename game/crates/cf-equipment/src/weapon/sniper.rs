//! M6: Sniper preset (long range, charge fire, 5-round mag).

use super::{WeaponClass, WeaponPreset, SNIPER_M6_DEFAULT_ID};
use crate::fire_modes::AdvancedFireMode;
use crate::{FireMode, RifleSpec, RoundKind};

#[must_use]
pub fn sniper_m6_default() -> WeaponPreset {
    let firing = RifleSpec {
        preset_id: SNIPER_M6_DEFAULT_ID.to_string(),
        fire_interval_seconds: 1.4,
        mag_capacity: 5,
        reload_seconds: 3.0,
        recoil_impulse: 85.0,
        muzzle_forward_offset: 18.0,
        muzzle_vertical_offset: 6.0,
        projectile_speed: 2200.0,
        damage_per_hit: 70.0,
        projectile_lifetime_seconds: 2.0,
        recoil_decay_rate: 0.1,
        loudness: 1.5,
        inherits_firer_velocity: false,
        particle_count: 1,
        spread_radians: 0.0,
        tracer_round_to_total_ratio: 1,
        ai_fire_vel: 2200.0,
        ai_penetration: 1.5,
        ai_life_time: 2.0,
        ai_blast_radius: 0.0,
        fire_mode: FireMode::Semi,
        primary_round: RoundKind::Regular,
        bullet_mass_kg: 0.0098,
        bullet_sharpness: 0.92,
};
    WeaponPreset::new(
        SNIPER_M6_DEFAULT_ID,
        "Long Rifle",
        WeaponClass::Sniper,
        firing,
        vec![AdvancedFireMode::Single, AdvancedFireMode::Charge],
        6.5,
        800.0,
    )
}
