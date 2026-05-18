//! M6C: Heavy Machine Gun (.50 cal) — crew-served; vehicle-mountable.

use super::{WeaponClass, WeaponPreset, HEAVY_MACHINE_GUN_50CAL_ID};
use crate::fire_modes::AdvancedFireMode;
use crate::{FireMode, RifleSpec, RoundKind};

#[must_use]
pub fn heavy_machine_gun_50cal() -> WeaponPreset {
    let firing = RifleSpec {
        preset_id: HEAVY_MACHINE_GUN_50CAL_ID.to_string(),
        fire_interval_seconds: 0.11,
        mag_capacity: 100,
        reload_seconds: 6.5,
        recoil_impulse: 90.0,
        muzzle_forward_offset: 26.0,
        muzzle_vertical_offset: 6.0,
        projectile_speed: 2200.0,
        damage_per_hit: 60.0,
        projectile_lifetime_seconds: 2.5,
        recoil_decay_rate: 0.07,
        loudness: 1.6,
        inherits_firer_velocity: false,
        particle_count: 1,
        spread_radians: 0.025,
        tracer_round_to_total_ratio: 3,
        ai_fire_vel: 2200.0,
        ai_penetration: 1.5,
        ai_life_time: 2.5,
        ai_blast_radius: 0.0,
        fire_mode: FireMode::FullAuto,
        primary_round: RoundKind::Regular,
    };
    let mut p = WeaponPreset::new(
        HEAVY_MACHINE_GUN_50CAL_ID,
        ".50 cal Heavy Machine Gun",
        WeaponClass::Hmg,
        firing,
        vec![AdvancedFireMode::Auto],
        38.0,
        1600.0,
    );
    // M6C spec literal: "crew-served; vehicle-mountable".
    p.crew_required = 2;
    p.bipod_compatible = true;
    p.vehicle_mountable = true;
    p
}
