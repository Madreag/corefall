//! M6: SMG preset (close-range, 40-round mag, Single/Burst3/Auto, suppressor-friendly).

use super::{WeaponClass, WeaponPreset, SMG_M6_DEFAULT_ID};
use crate::fire_modes::AdvancedFireMode;
use crate::{FireMode, RifleSpec, RoundKind};

#[must_use]
pub fn smg_m6_default() -> WeaponPreset {
    let firing = RifleSpec {
        preset_id: SMG_M6_DEFAULT_ID.to_string(),
        fire_interval_seconds: 0.08,
        mag_capacity: 40,
        reload_seconds: 1.3,
        recoil_impulse: 18.0,
        muzzle_forward_offset: 10.0,
        muzzle_vertical_offset: 4.0,
        projectile_speed: 1100.0,
        damage_per_hit: 8.0,
        projectile_lifetime_seconds: 1.0,
        recoil_decay_rate: 0.05,
        loudness: 1.0,
        inherits_firer_velocity: true,
        particle_count: 1,
        spread_radians: 0.03,
        tracer_round_to_total_ratio: 5,
        ai_fire_vel: 1100.0,
        ai_penetration: 0.0,
        ai_life_time: 1.0,
        ai_blast_radius: 0.0,
        fire_mode: FireMode::FullAuto,
        primary_round: RoundKind::Regular,
    };
    WeaponPreset::new(
        SMG_M6_DEFAULT_ID,
        "Submachine Gun",
        WeaponClass::Smg,
        firing,
        vec![
            AdvancedFireMode::Single,
            AdvancedFireMode::Burst3,
            AdvancedFireMode::Auto,
        ],
        3.0,
        160.0,
    )
}
