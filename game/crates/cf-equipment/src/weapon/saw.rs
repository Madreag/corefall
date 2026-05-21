//! M6C: Squad Automatic Weapon (SAW) — bipod + sustained suppress.

use super::{WeaponClass, WeaponPreset, SQUAD_AUTOMATIC_SAW_ID};
use crate::fire_modes::AdvancedFireMode;
use crate::{FireMode, RifleSpec, RoundKind};

#[must_use]
pub fn squad_automatic_saw() -> WeaponPreset {
    let firing = RifleSpec {
        preset_id: SQUAD_AUTOMATIC_SAW_ID.to_string(),
        fire_interval_seconds: 0.075,
        mag_capacity: 100,
        reload_seconds: 6.0,
        recoil_impulse: 26.0,
        muzzle_forward_offset: 18.0,
        muzzle_vertical_offset: 5.0,
        projectile_speed: 1300.0,
        damage_per_hit: 15.0,
        projectile_lifetime_seconds: 1.5,
        recoil_decay_rate: 0.05,
        loudness: 1.2,
        inherits_firer_velocity: false,
        particle_count: 1,
        spread_radians: 0.03,
        tracer_round_to_total_ratio: 4,
        ai_fire_vel: 1300.0,
        ai_penetration: 0.5,
        ai_life_time: 1.5,
        ai_blast_radius: 0.0,
        fire_mode: FireMode::FullAuto,
        primary_round: RoundKind::Regular,
        bullet_mass_kg: 0.005,
        bullet_sharpness: 0.85,
};
    let mut p = WeaponPreset::new(
        SQUAD_AUTOMATIC_SAW_ID,
        "Squad Automatic Weapon",
        WeaponClass::Saw,
        firing,
        vec![AdvancedFireMode::Auto],
        7.5,
        700.0,
    );
    // M6C spec literal: "bipod + sustained suppress".
    p.bipod_compatible = true;
    p
}
