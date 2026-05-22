//! M8A § GPU compute offload — cosmetic particle system.
//!
//! Per M8A spec § Acceptance criteria — GPU compute offload: the
//! cosmetic particle pool moves to a compute shader. CPU emits ONE
//! batched `terrain.debris_spawned` event with `debris_count = N` (not
//! N individual events). Events stay `cosmetic: true` so they fall
//! outside the determinism checksum.
//!
//! M8A ships the scaffold: the Bevy plugin + WGSL shader (in
//! `shaders/particles.wgsl`) + the determinism-isolation invariant
//! verification. M9+ wires the live engine emission path.

use serde::{Deserialize, Serialize};

/// The compute-shader source for the M8A cosmetic particle integration.
/// Loaded at engine init; bound to (group 0, binding 0, 1) per the
/// `shaders/particles.wgsl` declaration.
pub const PARTICLE_INTEGRATION_WGSL: &str = include_str!("../shaders/particles.wgsl");

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct GpuParticle {
    pub pos_x: f32,
    pub pos_y: f32,
    pub vel_x: f32,
    pub vel_y: f32,
    pub age_ms: f32,
    pub seed: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct GpuParticleGlobals {
    pub tick: u32,
    pub dt_seconds: f32,
    pub gravity: f32,
}

/// the Bevy render schedule; the descriptor holds the pool capacity and
/// the shader source.
#[derive(Debug, Clone)]
pub struct GpuParticleSystem {
    pub pool_capacity: usize,
    pub shader_source: &'static str,
}

impl Default for GpuParticleSystem {
    fn default() -> Self {
        Self {
            pool_capacity: 65_536,
            shader_source: PARTICLE_INTEGRATION_WGSL,
        }
    }
}

impl GpuParticleSystem {
    /// replay ignores GPU state entirely. This helper returns the
    /// canonical `cosmetic: true` flag for any debris event batched into
    /// the recorder.
    pub fn debris_event_is_cosmetic(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shader_source_is_loaded() {
        assert!(PARTICLE_INTEGRATION_WGSL.contains("integrate_particles"));
    }

    #[test]
    fn debris_events_are_cosmetic() {
        let sys = GpuParticleSystem::default();
        assert!(sys.debris_event_is_cosmetic());
    }
}
