//! M6: Grenade Launcher preset (arcing explosive rounds).

use super::{WeaponClass, WeaponPreset, GRENADE_LAUNCHER_M6_DEFAULT_ID};
use crate::fire_modes::AdvancedFireMode;
use crate::{FireMode, RifleSpec, RoundKind};

#[must_use]
pub fn grenade_launcher_m6_default() -> WeaponPreset {
    let firing = RifleSpec {
        preset_id: GRENADE_LAUNCHER_M6_DEFAULT_ID.to_string(),
        fire_interval_seconds: 1.5,
        mag_capacity: 4,
        reload_seconds: 3.5,
        recoil_impulse: 95.0,
        muzzle_forward_offset: 16.0,
        muzzle_vertical_offset: 8.0,
        projectile_speed: 480.0,
        damage_per_hit: 60.0,
        projectile_lifetime_seconds: 4.0,
        recoil_decay_rate: 0.08,
        loudness: 1.4,
        inherits_firer_velocity: false,
        particle_count: 1,
        spread_radians: 0.0,
        tracer_round_to_total_ratio: 0,
        ai_fire_vel: 480.0,
        ai_penetration: 0.0,
        ai_life_time: 4.0,
        ai_blast_radius: 60.0,
        fire_mode: FireMode::Semi,
        primary_round: RoundKind::HighExplosive,
    };
    WeaponPreset::new(
        GRENADE_LAUNCHER_M6_DEFAULT_ID,
        "Grenade Launcher",
        WeaponClass::GrenadeLauncher,
        firing,
        vec![AdvancedFireMode::Single, AdvancedFireMode::Arc],
        7.0,
        200.0,
    )
}
