//! Scope ADS — sniper zoom + reticle precision boost per spec.

use crate::{CameraMode, CameraState, FOLLOW_FOV_DEGREES, SCOPE_FOV_DEGREES};

/// Reticle bloom multiplier while in Scope mode (per spec § "reticle
/// precision boost ×0.3 bloom").
pub const SCOPE_RETICLE_BLOOM_MULT: f32 = 0.3;

/// Enter scope mode. Sets FOV to the configured `scope_zoom_fov` (defaulting
/// to `SCOPE_FOV_DEGREES`). The accessibility setting `scope_zoom_fov` may
/// override the default through cf-control's Settings; the helper accepts
/// the override as `fov_degrees`.
pub fn enter_scope(state: &mut CameraState, fov_degrees: Option<f32>) {
    state.mode = CameraMode::Scope;
    state.fov_degrees = fov_degrees.unwrap_or(SCOPE_FOV_DEGREES).clamp(5.0, 90.0);
}

/// Exit scope mode and restore Follow FOV.
pub fn exit_scope(state: &mut CameraState) {
    state.mode = CameraMode::Follow;
    state.fov_degrees = FOLLOW_FOV_DEGREES;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enter_scope_sets_default_fov() {
        let mut s = CameraState::default();
        enter_scope(&mut s, None);
        assert_eq!(s.mode, CameraMode::Scope);
        assert_eq!(s.fov_degrees, SCOPE_FOV_DEGREES);
    }

    #[test]
    fn enter_scope_honors_accessibility_override() {
        let mut s = CameraState::default();
        enter_scope(&mut s, Some(20.0));
        assert_eq!(s.fov_degrees, 20.0);
    }

    #[test]
    fn enter_scope_clamps_extreme_overrides() {
        let mut s = CameraState::default();
        enter_scope(&mut s, Some(1000.0));
        assert_eq!(s.fov_degrees, 90.0);
    }

    #[test]
    fn exit_scope_restores_follow() {
        let mut s = CameraState::default();
        enter_scope(&mut s, None);
        exit_scope(&mut s);
        assert_eq!(s.mode, CameraMode::Follow);
        assert_eq!(s.fov_degrees, FOLLOW_FOV_DEGREES);
    }
}
