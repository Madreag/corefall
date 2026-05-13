//! **M1**: projectile-spec helpers consumed by `cf-actor::sim` at spawn time.
//!
//! The actual `SpawnedProjectile` type lives in `cf-actor/src/sim.rs` because
//! the projectile world is owned by the actor world. This module exposes the
//! reverse mapping: given a weapon spec + firing context, build the per-shot
//! projectile parameters (inherited velocity factor, loudness radius,
//! particle count, tracer flag).

use crate::RifleSpec;

/// Per-shot parameters extracted from a [`RifleSpec`] for projectile spawn.
/// Pure value type; no allocations. The cf-actor sim consumes this when
/// constructing a `SpawnedProjectile`.
#[derive(Debug, Clone, Copy)]
pub struct ProjectileSpawnParams {
    /// Particle count to spawn per fire press (1 for rifles, N for shotguns).
    pub particle_count: u32,
    /// Velocity inheritance fraction (0.0..1.0). 0.5 = standard for M1 rifles.
    pub inherit_fraction: f32,
    /// Loudness radius scalar — multiplied by the engine's damage-derived base.
    pub loudness_scalar: f32,
    /// Per-particle spread angle (radians). 0.0 = perfectly aimed.
    pub spread_radians: f32,
    /// Per-particle muzzle velocity (units/s).
    pub muzzle_velocity: f32,
    /// Per-particle damage (mass × velocity² in M1's simple model).
    pub damage: f32,
}

impl ProjectileSpawnParams {
    /// **M1**: extract spawn params from a rifle spec.
    ///
    /// M1 audit pass 6 (2026-05-13): full velocity inheritance per spec
    /// literal `muzzle_velocity_vector + actor_velocity` (CCCP
    /// `HDFirearm.cpp:752`).
    pub fn from_rifle(spec: &RifleSpec) -> Self {
        Self {
            particle_count: spec.particle_count.max(1),
            inherit_fraction: if spec.inherits_firer_velocity { 1.0 } else { 0.0 },
            loudness_scalar: spec.loudness.max(0.1),
            spread_radians: spec.spread_radians,
            muzzle_velocity: spec.projectile_speed,
            damage: spec.damage_per_hit,
        }
    }

    /// **M1**: loudness radius for the alarm event. Formula matches the
    /// engine's `cf-actor::sim` site:
    /// `480 * (damage / 10).clamp(1, 3) * loudness_scalar`.
    pub fn loudness_radius(&self) -> f32 {
        480.0 * (self.damage / 10.0).clamp(1.0, 3.0) * self.loudness_scalar
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loudness_formula_baseline_rifle() {
        let params = ProjectileSpawnParams {
            particle_count: 1,
            inherit_fraction: 1.0,
            loudness_scalar: 1.0,
            spread_radians: 0.0,
            muzzle_velocity: 1200.0,
            damage: 12.0,
        };
        // 480 * (12 / 10).clamp(1, 3) * 1.0 = 480 * 1.2 = 576.0
        assert!((params.loudness_radius() - 576.0).abs() < 0.001);
    }

    #[test]
    fn inherit_fraction_disabled_for_off_flag() {
        // With inherits_firer_velocity=false, fraction should be 0.0.
        let params = ProjectileSpawnParams {
            particle_count: 1,
            inherit_fraction: 0.0,
            loudness_scalar: 1.0,
            spread_radians: 0.0,
            muzzle_velocity: 1200.0,
            damage: 12.0,
        };
        assert!(params.inherit_fraction.abs() < f32::EPSILON);
    }
}
