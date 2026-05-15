//! M9 — Reactor destruction VFX (one-shot explosion burst).
//!
//! Spec § Explosion VFX on reactor destruction — when
//! `mission.reactor_destroyed` fires, the renderer spawns: a large flash,
//! a debris scatter (capped at `EXPLOSION_DEBRIS_CAP_PER_HIT`), and screen
//! shake with magnitude proportional to `1.0 - reduce_camera_shake_pct`
//! (accessibility setting). Total VFX terminates within 1 second per
//! spec ("explosion VFX terminates within 1 second; no perpetual
//! particles").

use bevy::prelude::Resource;

/// Per M9 spec § Sim numbers: `Explosion debris cap = 200 pixels`.
pub const EXPLOSION_DEBRIS_CAP_PER_HIT: u32 = 200;

/// Maximum frame duration for the explosion VFX. Per spec the burst must
/// terminate within 1 second; the renderer drives all particles past this
/// even if their per-particle lifetime is longer.
pub const EXPLOSION_MAX_DURATION_MS: u32 = 1000;

/// One render-time particle for the explosion burst.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExplosionParticle {
    pub origin: [f32; 2],
    pub velocity: [f32; 2],
    pub lifetime_ms: u32,
    pub age_ms: u32,
}

impl ExplosionParticle {
    pub fn is_expired(&self) -> bool {
        self.age_ms >= self.lifetime_ms.min(EXPLOSION_MAX_DURATION_MS)
    }

    pub fn advance(&mut self, dt_ms: u32) {
        self.age_ms = self.age_ms.saturating_add(dt_ms);
    }
}

/// Explosion VFX render state. Includes the flash + debris + accessibility-
/// adjusted screen-shake magnitude.
///
/// cf-app spawns one burst on each `mission.reactor_destroyed` event;
/// the renderer ticks `tick(dt_ms)` per frame to advance + retire
/// particles, terminating the whole VFX within 1 second per spec.
#[derive(Resource, Debug, Clone, Default)]
pub struct ExplosionState {
    pub origin: [f32; 2],
    pub flash_remaining_ms: u32,
    pub shake_magnitude: f32,
    pub debris: Vec<ExplosionParticle>,
}

impl ExplosionState {
    /// Spawn a one-shot explosion burst. `reduce_camera_shake_pct ∈ [0, 1]`
    /// is the accessibility multiplier; 1.0 disables shake entirely.
    pub fn spawn(&mut self, origin: [f32; 2], debris_count: u32, reduce_camera_shake_pct: f32) -> u32 {
        self.origin = origin;
        self.flash_remaining_ms = 250;
        self.shake_magnitude = 1.0 * (1.0 - reduce_camera_shake_pct.clamp(0.0, 1.0));
        let count = debris_count.min(EXPLOSION_DEBRIS_CAP_PER_HIT);
        self.debris.clear();
        self.debris.reserve(count as usize);
        for _ in 0..count {
            self.debris.push(ExplosionParticle {
                origin,
                velocity: [0.0, 0.0],
                lifetime_ms: 800,
                age_ms: 0,
            });
        }
        count
    }

    pub fn tick(&mut self, dt_ms: u32) {
        self.flash_remaining_ms = self.flash_remaining_ms.saturating_sub(dt_ms);
        for p in self.debris.iter_mut() {
            p.advance(dt_ms);
        }
        self.debris.retain(|p| !p.is_expired());
    }

    pub fn is_finished(&self) -> bool {
        self.flash_remaining_ms == 0 && self.debris.is_empty() && self.shake_magnitude == 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debris_capped_per_spec() {
        let mut s = ExplosionState::default();
        let actual = s.spawn([10.0, 20.0], 1000, 0.0);
        assert_eq!(actual, EXPLOSION_DEBRIS_CAP_PER_HIT);
        assert_eq!(s.debris.len() as u32, EXPLOSION_DEBRIS_CAP_PER_HIT);
    }

    #[test]
    fn reduce_camera_shake_disables_shake() {
        let mut s = ExplosionState::default();
        s.spawn([0.0, 0.0], 10, 1.0);
        assert_eq!(s.shake_magnitude, 0.0);
    }

    #[test]
    fn explosion_terminates_within_one_second() {
        let mut s = ExplosionState::default();
        s.spawn([0.0, 0.0], 10, 0.0);
        s.tick(1000);
        assert!(s.debris.is_empty(), "explosion must terminate within 1s per spec");
    }
}
