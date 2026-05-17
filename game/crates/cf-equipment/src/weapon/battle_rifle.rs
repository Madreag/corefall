//! M6C: Battle Rifle (7.62 NATO) — full-auto + heavier round.

use super::{WeaponClass, WeaponPreset, BATTLE_RIFLE_762_ID};
use crate::fire_modes::AdvancedFireMode;
use crate::{FireMode, RifleSpec};

#[must_use]
pub fn battle_rifle_762() -> WeaponPreset {
    let firing = RifleSpec {
        preset_id: BATTLE_RIFLE_762_ID.to_string(),
        fire_interval_seconds: 0.13,
        mag_capacity: 20,
        reload_seconds: 2.0,
        recoil_impulse: 42.0,
        muzzle_forward_offset: 16.0,
        muzzle_vertical_offset: 5.0,
        projectile_speed: 1500.0,
        damage_per_hit: 26.0,
        projectile_lifetime_seconds: 1.8,
        recoil_decay_rate: 0.06,
        loudness: 1.3,
        inherits_firer_velocity: true,
        particle_count: 1,
        spread_radians: 0.012,
        tracer_round_to_total_ratio: 4,
        ai_fire_vel: 1500.0,
        ai_penetration: 0.9,
        ai_life_time: 1.8,
        ai_blast_radius: 0.0,
        fire_mode: FireMode::FullAuto,
    };
    WeaponPreset::new(
        BATTLE_RIFLE_762_ID,
        "7.62 Battle Rifle",
        WeaponClass::BattleRifle,
        firing,
        vec![
            AdvancedFireMode::Single,
            AdvancedFireMode::Burst3,
            AdvancedFireMode::Auto,
        ],
        4.5,
        500.0,
    )
}
