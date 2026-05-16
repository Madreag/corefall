//! M9 — Reactor spark particle emitter (bullet-impact cosmetic VFX).
//!
//! Spec § Bullet-impact sparks on reactor — every projectile hit on the
//! reactor's AABB spawns a brief spark burst at the impact point. Sparks
//! are COSMETIC (the recorder fires the event with cosmetic=true so
//! replay determinism is not affected by particle RNG drift). Capped at
//! `SPARK_CAP_PER_HIT` per event so a high-RPM stream of hits doesn't
//! flood the renderer.

use bevy::prelude::Resource;

/// Maximum spark particles per impact, per M9 spec § Sim numbers:
/// `Spark VFX cap per hit = 12 particles`.
pub const SPARK_CAP_PER_HIT: u32 = 12;

/// One spark particle's render-time state. The renderer consumes the
/// list and advances `age_ms` per frame; particles expire when age >=
/// `lifetime_ms`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SparkParticle {
    pub origin: [f32; 2],
    pub velocity: [f32; 2],
    pub lifetime_ms: u32,
    pub age_ms: u32,
}

impl SparkParticle {
    pub fn is_expired(&self) -> bool {
        self.age_ms >= self.lifetime_ms
    }

    pub fn advance(&mut self, dt_ms: u32) {
        self.age_ms = self.age_ms.saturating_add(dt_ms);
    }
}

/// Spark emitter state. Drains expired particles automatically.
///
/// cf-app's recorder-event pump pushes a burst on every reactor
/// `combat.projectile_hit` event (target_kind="reactor"); the renderer
/// ticks `tick(dt_ms)` per frame to advance + retire particles.
#[derive(Resource, Debug, Clone, Default)]
pub struct SparkEmitterState {
    pub particles: Vec<SparkParticle>,
}

impl SparkEmitterState {
    /// Spawn up to `SPARK_CAP_PER_HIT` particles at the impact point. The
    /// caller fills the velocity field with whatever cosmetic spread the
    /// renderer chooses (jittered radially from impact_point, etc).
    pub fn spawn_burst(&mut self, origin: [f32; 2], count: u32, lifetime_ms: u32) -> u32 {
        let to_spawn = count.min(SPARK_CAP_PER_HIT);
        for _ in 0..to_spawn {
            self.particles.push(SparkParticle {
                origin,
                velocity: [0.0, 0.0],
                lifetime_ms,
                age_ms: 0,
            });
        }
        to_spawn
    }

    pub fn tick(&mut self, dt_ms: u32) {
        for p in self.particles.iter_mut() {
            p.advance(dt_ms);
        }
        self.particles.retain(|p| !p.is_expired());
    }

    pub fn live_count(&self) -> usize {
        self.particles.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_caps_at_per_hit_limit() {
        let mut s = SparkEmitterState::default();
        let actual = s.spawn_burst([0.0, 0.0], 50, 200);
        assert_eq!(actual, SPARK_CAP_PER_HIT);
        assert_eq!(s.live_count() as u32, SPARK_CAP_PER_HIT);
    }

    #[test]
    fn tick_expires_particles() {
        let mut s = SparkEmitterState::default();
        s.spawn_burst([0.0, 0.0], 5, 100);
        s.tick(50);
        assert_eq!(s.live_count(), 5);
        s.tick(60);
        assert_eq!(s.live_count(), 0);
    }
}
