//! M6C: Revolver (.357) — slow + heavy hit; manual reload.

use super::{WeaponClass, WeaponPreset, REVOLVER_357_ID};
use crate::fire_modes::AdvancedFireMode;
use crate::{FireMode, RifleSpec, RoundKind};

#[must_use]
pub fn revolver_357() -> WeaponPreset {
    let firing = RifleSpec {
        preset_id: REVOLVER_357_ID.to_string(),
        fire_interval_seconds: 0.55,
        mag_capacity: 6,
        reload_seconds: 4.0,
        recoil_impulse: 38.0,
        muzzle_forward_offset: 9.0,
        muzzle_vertical_offset: 3.0,
        projectile_speed: 1100.0,
        damage_per_hit: 38.0,
        projectile_lifetime_seconds: 1.2,
        recoil_decay_rate: 0.06,
        loudness: 1.2,
        inherits_firer_velocity: true,
        particle_count: 1,
        spread_radians: 0.0,
        tracer_round_to_total_ratio: 0,
        ai_fire_vel: 1100.0,
        ai_penetration: 0.4,
        ai_life_time: 1.2,
        ai_blast_radius: 0.0,
        fire_mode: FireMode::Semi,
        primary_round: RoundKind::Regular,
        bullet_mass_kg: 0.014,
        bullet_sharpness: 0.7,
};
    WeaponPreset::new(
        REVOLVER_357_ID,
        ".357 Revolver",
        WeaponClass::Revolver,
        firing,
        vec![AdvancedFireMode::Single],
        1.4,
        160.0,
    )
}
