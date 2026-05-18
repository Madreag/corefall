//! M6C: Sniper Rifle (Tier 2) — long range + high damage.

use super::{WeaponClass, WeaponPreset, SNIPER_RIFLE_T2_ID};
use crate::fire_modes::AdvancedFireMode;
use crate::{FireMode, RifleSpec, RoundKind};

#[must_use]
pub fn sniper_rifle_t2() -> WeaponPreset {
    let firing = RifleSpec {
        preset_id: SNIPER_RIFLE_T2_ID.to_string(),
        fire_interval_seconds: 1.7,
        mag_capacity: 5,
        reload_seconds: 3.5,
        recoil_impulse: 110.0,
        muzzle_forward_offset: 22.0,
        muzzle_vertical_offset: 8.0,
        projectile_speed: 2700.0,
        damage_per_hit: 110.0,
        projectile_lifetime_seconds: 2.5,
        recoil_decay_rate: 0.09,
        loudness: 1.6,
        inherits_firer_velocity: false,
        particle_count: 1,
        spread_radians: 0.0,
        tracer_round_to_total_ratio: 1,
        ai_fire_vel: 2700.0,
        ai_penetration: 2.0,
        ai_life_time: 2.5,
        ai_blast_radius: 0.0,
        fire_mode: FireMode::Semi,
        primary_round: RoundKind::Regular,
    };
    WeaponPreset::new(
        SNIPER_RIFLE_T2_ID,
        "Sniper Rifle (T2)",
        WeaponClass::Sniper,
        firing,
        vec![AdvancedFireMode::Single, AdvancedFireMode::Charge],
        7.5,
        1200.0,
    )
}
