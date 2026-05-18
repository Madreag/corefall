//! M6C: Designated Marksman Rifle (7.62) — semi-auto precision.

use super::{WeaponClass, WeaponPreset, DMR_762_ID};
use crate::fire_modes::AdvancedFireMode;
use crate::{FireMode, RifleSpec, RoundKind};

#[must_use]
pub fn dmr_762() -> WeaponPreset {
    let firing = RifleSpec {
        preset_id: DMR_762_ID.to_string(),
        fire_interval_seconds: 0.45,
        mag_capacity: 20,
        reload_seconds: 2.2,
        recoil_impulse: 55.0,
        muzzle_forward_offset: 18.0,
        muzzle_vertical_offset: 6.0,
        projectile_speed: 1800.0,
        damage_per_hit: 48.0,
        projectile_lifetime_seconds: 2.0,
        recoil_decay_rate: 0.08,
        loudness: 1.4,
        inherits_firer_velocity: false,
        particle_count: 1,
        spread_radians: 0.003,
        tracer_round_to_total_ratio: 4,
        ai_fire_vel: 1800.0,
        ai_penetration: 1.2,
        ai_life_time: 2.0,
        ai_blast_radius: 0.0,
        fire_mode: FireMode::Semi,
        primary_round: RoundKind::Regular,
    };
    WeaponPreset::new(
        DMR_762_ID,
        "7.62 DMR",
        WeaponClass::Dmr,
        firing,
        vec![AdvancedFireMode::Single],
        5.2,
        700.0,
    )
}
