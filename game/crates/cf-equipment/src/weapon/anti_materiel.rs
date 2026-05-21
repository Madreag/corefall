//! M6C: 12.7mm Anti-Materiel Rifle — anti-vehicle precision (HEAT/APFSDS
//! compatible per M14C).

use super::{WeaponClass, WeaponPreset, ANTI_MATERIEL_RIFLE_127_ID};
use crate::fire_modes::AdvancedFireMode;
use crate::{FireMode, RifleSpec, RoundKind};

#[must_use]
pub fn anti_materiel_rifle_127() -> WeaponPreset {
    let firing = RifleSpec {
        preset_id: ANTI_MATERIEL_RIFLE_127_ID.to_string(),
        fire_interval_seconds: 2.2,
        mag_capacity: 5,
        reload_seconds: 4.0,
        recoil_impulse: 160.0,
        muzzle_forward_offset: 28.0,
        muzzle_vertical_offset: 9.0,
        projectile_speed: 2900.0,
        damage_per_hit: 220.0,
        projectile_lifetime_seconds: 3.0,
        recoil_decay_rate: 0.1,
        loudness: 1.8,
        inherits_firer_velocity: false,
        particle_count: 1,
        spread_radians: 0.0,
        tracer_round_to_total_ratio: 1,
        ai_fire_vel: 2900.0,
        ai_penetration: 3.0,
        ai_life_time: 3.0,
        ai_blast_radius: 0.0,
        fire_mode: FireMode::Semi,
        primary_round: RoundKind::Regular,
        bullet_mass_kg: 0.85,
        bullet_sharpness: 0.97,
};
    WeaponPreset::new(
        ANTI_MATERIEL_RIFLE_127_ID,
        "12.7mm Anti-Materiel Rifle",
        WeaponClass::AntiMateriel,
        firing,
        vec![AdvancedFireMode::Single],
        14.0,
        2000.0,
    )
}
