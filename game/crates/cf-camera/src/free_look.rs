//! Free-look mode — RMB-toggle cursor follow within max distance.

use crate::{CameraMode, CameraState};

/// Enter free-look mode. The cursor anchor + max-distance clamp is
/// captured in state so cf-control's tick can compute the free-look
/// position each frame.
pub fn enter_free_look(state: &mut CameraState, cursor: (f32, f32), max_distance: f32) {
    state.mode = CameraMode::FreeLook;
    state.free_look_cursor = clamp_cursor(state.position, cursor, max_distance);
    state.free_look_max_distance = max_distance.max(0.0);
}

/// Exit free-look mode and restore Follow.
pub fn exit_free_look(state: &mut CameraState) {
    state.mode = CameraMode::Follow;
    state.free_look_cursor = (0.0, 0.0);
}

/// Re-clamp the free-look cursor to within `max_distance` of `anchor`.
pub fn clamp_cursor(anchor: (f32, f32), cursor: (f32, f32), max_distance: f32) -> (f32, f32) {
    let dx = cursor.0 - anchor.0;
    let dy = cursor.1 - anchor.1;
    let mag = (dx * dx + dy * dy).sqrt();
    if mag <= max_distance || mag <= f32::EPSILON {
        return cursor;
    }
    let scale = max_distance / mag;
    (anchor.0 + dx * scale, anchor.1 + dy * scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enter_sets_free_look_mode() {
        let mut s = CameraState::default();
        enter_free_look(&mut s, (50.0, 0.0), 100.0);
        assert_eq!(s.mode, CameraMode::FreeLook);
        assert_eq!(s.free_look_cursor, (50.0, 0.0));
        assert_eq!(s.free_look_max_distance, 100.0);
    }

    #[test]
    fn enter_clamps_cursor_beyond_max() {
        let mut s = CameraState::at((0.0, 0.0));
        enter_free_look(&mut s, (300.0, 0.0), 100.0);
        assert!((s.free_look_cursor.0 - 100.0).abs() < f32::EPSILON);
        assert!((s.free_look_cursor.1 - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn exit_restores_follow() {
        let mut s = CameraState::default();
        enter_free_look(&mut s, (10.0, 0.0), 50.0);
        exit_free_look(&mut s);
        assert_eq!(s.mode, CameraMode::Follow);
        assert_eq!(s.free_look_cursor, (0.0, 0.0));
    }
}
