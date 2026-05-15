//! Smooth-follow camera tick. Lerp + lookahead + deadzone per spec § 200ms ease.

use crate::{CameraState, LERP_FACTOR_MS};

/// Advance the smooth-follow target one frame. The 200ms ease constant
/// `LERP_FACTOR_MS` defines the time-to-90%-target; the per-frame alpha
/// derives from `dt_ms / LERP_FACTOR_MS`. Lookahead is applied along the
/// player's velocity vector before lerping; deadzone short-circuits the
/// lerp when the camera is already within `deadzone_radius` of target.
pub fn tick_smooth_follow(state: &mut CameraState, target_pos: (f32, f32), velocity: (f32, f32), dt_ms: u32) {
    let lookahead = lookahead_for(velocity, state.deadzone_radius);
    state.lookahead_offset = lookahead;
    let aim_target = (target_pos.0 + lookahead.0, target_pos.1 + lookahead.1);
    state.target = aim_target;

    let dx = aim_target.0 - state.position.0;
    let dy = aim_target.1 - state.position.1;
    let dist_sq = dx * dx + dy * dy;
    if dist_sq <= state.deadzone_radius * state.deadzone_radius {
        return;
    }

    let alpha = clamp01(dt_ms as f32 / LERP_FACTOR_MS as f32);
    state.position.0 += dx * alpha;
    state.position.1 += dy * alpha;
}

fn lookahead_for(velocity: (f32, f32), max: f32) -> (f32, f32) {
    let mag = (velocity.0 * velocity.0 + velocity.1 * velocity.1).sqrt();
    if mag < f32::EPSILON {
        return (0.0, 0.0);
    }
    let scale = (mag.min(max * 4.0) / 4.0).min(crate::DEFAULT_LOOKAHEAD_RANGE);
    (velocity.0 / mag * scale, velocity.1 / mag * scale)
}

fn clamp01(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookahead_extends_along_velocity() {
        let mut state = CameraState::at((0.0, 0.0));
        tick_smooth_follow(&mut state, (100.0, 0.0), (200.0, 0.0), 16);
        assert!(state.lookahead_offset.0 > 0.0);
        assert!((state.lookahead_offset.1).abs() < f32::EPSILON);
    }

    #[test]
    fn deadzone_skips_lerp() {
        let mut state = CameraState::at((0.0, 0.0));
        state.deadzone_radius = 10.0;
        let pos_before = state.position;
        tick_smooth_follow(&mut state, (4.0, 4.0), (0.0, 0.0), 16);
        assert_eq!(state.position, pos_before);
    }

    #[test]
    fn lerp_at_full_factor_advances_to_target() {
        let mut state = CameraState::at((0.0, 0.0));
        tick_smooth_follow(&mut state, (100.0, 0.0), (0.0, 0.0), LERP_FACTOR_MS);
        assert!((state.position.0 - 100.0).abs() < f32::EPSILON);
    }
}
