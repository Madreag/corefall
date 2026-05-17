//! M6C: Assault Rifle (Tier 2) — standard mid-tier.

use super::{WeaponClass, WeaponPreset, ASSAULT_RIFLE_T2_ID};
use crate::fire_modes::AdvancedFireMode;
use crate::{FireMode, RifleSpec};

#[must_use]
pub fn assault_rifle_t2() -> WeaponPreset {
    let firing = RifleSpec {
        preset_id: ASSAULT_RIFLE_T2_ID.to_string(),
        fire_interval_seconds: 0.095,
        mag_capacity: 30,
        reload_seconds: 1.7,
        recoil_impulse: 28.0,
        muzzle_forward_offset: 14.0,
        muzzle_vertical_offset: 5.0,
        projectile_speed: 1300.0,
        damage_per_hit: 14.0,
        projectile_lifetime_seconds: 1.5,
        recoil_decay_rate: 0.05,
        loudness: 1.1,
        inherits_firer_velocity: true,
        particle_count: 1,
        spread_radians: 0.015,
        tracer_round_to_total_ratio: 4,
        ai_fire_vel: 1300.0,
        ai_penetration: 0.5,
        ai_life_time: 1.5,
        ai_blast_radius: 0.0,
        fire_mode: FireMode::FullAuto,
    };
    WeaponPreset::new(
        ASSAULT_RIFLE_T2_ID,
        "Assault Rifle (T2)",
        WeaponClass::Rifle,
        firing,
        vec![
            AdvancedFireMode::Single,
            AdvancedFireMode::Burst3,
            AdvancedFireMode::Auto,
        ],
        3.4,
        300.0,
    )
}
