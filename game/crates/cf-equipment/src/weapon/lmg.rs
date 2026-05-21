//! M6C: Belt-fed Light Machine Gun — sustained fire.

use super::{WeaponClass, WeaponPreset, LMG_BELT_FED_ID};
use crate::fire_modes::AdvancedFireMode;
use crate::{FireMode, RifleSpec, RoundKind};

#[must_use]
pub fn lmg_belt_fed() -> WeaponPreset {
    let firing = RifleSpec {
        preset_id: LMG_BELT_FED_ID.to_string(),
        fire_interval_seconds: 0.085,
        mag_capacity: 200,
        reload_seconds: 7.5,
        recoil_impulse: 30.0,
        muzzle_forward_offset: 18.0,
        muzzle_vertical_offset: 5.0,
        projectile_speed: 1400.0,
        damage_per_hit: 18.0,
        projectile_lifetime_seconds: 1.7,
        recoil_decay_rate: 0.05,
        loudness: 1.3,
        inherits_firer_velocity: false,
        particle_count: 1,
        spread_radians: 0.03,
        tracer_round_to_total_ratio: 4,
        ai_fire_vel: 1400.0,
        ai_penetration: 0.6,
        ai_life_time: 1.7,
        ai_blast_radius: 0.0,
        fire_mode: FireMode::FullAuto,
        primary_round: RoundKind::Regular,
        bullet_mass_kg: 0.0098,
        bullet_sharpness: 0.85,
};
    WeaponPreset::new(
        LMG_BELT_FED_ID,
        "Belt-fed LMG",
        WeaponClass::Lmg,
        firing,
        vec![AdvancedFireMode::Auto],
        9.5,
        750.0,
    )
}
