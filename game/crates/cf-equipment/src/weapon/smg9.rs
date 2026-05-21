//! M6C: 9mm Submachine Gun — rapid fire; close range; large magazine.

use super::{WeaponClass, WeaponPreset, SUBMACHINE_GUN_9MM_ID};
use crate::fire_modes::AdvancedFireMode;
use crate::{FireMode, RifleSpec, RoundKind};

#[must_use]
pub fn submachine_gun_9mm() -> WeaponPreset {
    let firing = RifleSpec {
        preset_id: SUBMACHINE_GUN_9MM_ID.to_string(),
        fire_interval_seconds: 0.066,
        mag_capacity: 50,
        reload_seconds: 1.4,
        recoil_impulse: 16.0,
        muzzle_forward_offset: 10.0,
        muzzle_vertical_offset: 4.0,
        projectile_speed: 1050.0,
        damage_per_hit: 8.5,
        projectile_lifetime_seconds: 0.9,
        recoil_decay_rate: 0.05,
        loudness: 1.0,
        inherits_firer_velocity: true,
        particle_count: 1,
        spread_radians: 0.04,
        tracer_round_to_total_ratio: 5,
        ai_fire_vel: 1050.0,
        ai_penetration: 0.0,
        ai_life_time: 0.9,
        ai_blast_radius: 0.0,
        fire_mode: FireMode::FullAuto,
        primary_round: RoundKind::Regular,
        bullet_mass_kg: 0.008,
        bullet_sharpness: 0.75,
};
    WeaponPreset::new(
        SUBMACHINE_GUN_9MM_ID,
        "9mm Submachine Gun",
        WeaponClass::Smg,
        firing,
        vec![
            AdvancedFireMode::Single,
            AdvancedFireMode::Burst3,
            AdvancedFireMode::Auto,
        ],
        2.8,
        140.0,
    )
}
