//! M8 — Scope reticle HUD widget (sniper ADS overlay).

use bevy::prelude::*;

/// Scope reticle widget Bevy resource.
#[derive(Resource, Debug, Clone, Default)]
pub struct ScopeReticleState {
    /// Whether the scope reticle is currently rendered.
    pub active: bool,
    /// Active FOV in degrees (mirrors cf-camera's CameraState.fov_degrees).
    pub fov_degrees: f32,
    /// Bloom multiplier (cf-camera's SCOPE_RETICLE_BLOOM_MULT when active).
    pub bloom_multiplier: f32,
}

impl ScopeReticleState {
    /// Enable the reticle with the supplied FOV + bloom.
    pub fn enable(&mut self, fov_degrees: f32, bloom_multiplier: f32) {
        self.active = true;
        self.fov_degrees = fov_degrees;
        self.bloom_multiplier = bloom_multiplier;
    }

    /// Disable the reticle.
    pub fn disable(&mut self) {
        self.active = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enable_then_disable() {
        let mut s = ScopeReticleState::default();
        s.enable(30.0, 0.3);
        assert!(s.active);
        assert!((s.fov_degrees - 30.0).abs() < f32::EPSILON);
        s.disable();
        assert!(!s.active);
    }
}
