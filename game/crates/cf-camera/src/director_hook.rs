//! M10B § cf-camera::director_hook — per-tick pose override entry
//! point for offline export.
//!
//! Spec § Crates / modules touched:
//!
//! > `cf-camera` — Camera director hook: per-tick pose override from
//! > `*.camera.ron` instead of live input.
//!
//! The live camera state machine ([`crate::tick_smooth_follow`]) is
//! input-driven (player WASD / cursor / hit-stop pulses). M10B's
//! offline export replays a bundle through the M4B delta chain and
//! must produce deterministic camera poses with NO live input —
//! `cf-replay-export::camera_director` resolves the per-tick pose
//! from a `*.camera.ron` script + Catmull-Rom interpolation, then
//! pushes it through [`apply_director_pose`] to override the camera
//! state for the corresponding render frame.
//!
//! Contract:
//!
//! - The hook is a pure function: same `(state, pose)` input produces
//!   the same output `state`.
//! - No RNG, no platform conditionals.
//! - The smooth-follow lerp + lookahead computation is bypassed (the
//!   pose is the authoritative target).
//! - The hit-stop pulse timer continues to decay independently — the
//!   director only owns position + zoom + rotation, not the time-step
//!   freeze.

use serde::{Deserialize, Serialize};

use crate::{CameraMode, CameraState};

/// Pose vector layout — identical to the script-side `Pose` in
/// `cf-replay-export::camera_script`. The hook takes a borrowed
/// reference so the export pipeline can iterate per-tick without
/// allocating.
pub type DirectorPose = [f32; 6];

/// Component count. Mirrors `cf_replay_export::camera_script::POSE_COMPONENTS`.
pub const DIRECTOR_POSE_COMPONENTS: usize = 6;

/// Marker tag identifying which director kind produced this pose.
/// Carried alongside the pose so the camera state can record which
/// authored track is currently active (useful for editor UI + audit
/// logs).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectorCameraKind {
    /// Free-floating cursor-driven camera (cursor anchor in
    /// world-space, no actor following).
    FreeCam,
    /// Smooth-follow on the player actor.
    FollowPlayer,
    /// Objective-anchored camera (tracks the active mission
    /// objective's world position).
    ObjectiveCam,
    /// War-Thunder-style kill-cam (anchored on the killing event).
    KillCam,
}

impl DirectorCameraKind {
    /// Canonical snake_case identifier surfaced to cfctl + replay
    /// bundles.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            DirectorCameraKind::FreeCam => "free_cam",
            DirectorCameraKind::FollowPlayer => "follow_player",
            DirectorCameraKind::ObjectiveCam => "objective_cam",
            DirectorCameraKind::KillCam => "kill_cam",
        }
    }

    /// Map a director kind to the live camera state's [`CameraMode`].
    /// `free_cam` / `objective_cam` / `kill_cam` overlay the
    /// [`CameraMode::FreeLook`] mode (cursor-driven anchor); the
    /// `follow_player` kind maps to [`CameraMode::Follow`].
    #[must_use]
    pub const fn to_camera_mode(self) -> CameraMode {
        match self {
            DirectorCameraKind::FollowPlayer => CameraMode::Follow,
            DirectorCameraKind::FreeCam | DirectorCameraKind::ObjectiveCam | DirectorCameraKind::KillCam => {
                CameraMode::FreeLook
            }
        }
    }
}

/// Override the camera state with the director-supplied pose. The
/// state's `mode` switches to the corresponding [`CameraMode`] and the
/// position / target / lookahead / FOV fields are written verbatim
/// from `pose`.
///
/// Pose component layout — `(x, y, zoom, rotation, lookahead_x,
/// lookahead_y)`. The `zoom` component maps to the camera state's FOV
/// via [`zoom_to_fov_degrees`]; `rotation` is unused by the live
/// camera (no per-frame rotation system ships in M8) but is preserved
/// for future use by the offline rasterizer.
pub fn apply_director_pose(state: &mut CameraState, pose: &DirectorPose, kind: DirectorCameraKind) {
    state.mode = kind.to_camera_mode();
    state.position = (pose[0], pose[1]);
    state.target = (pose[0], pose[1]);
    state.lookahead_offset = (pose[4], pose[5]);
    state.fov_degrees = zoom_to_fov_degrees(pose[2]);
    state.free_look_cursor = (pose[0] + pose[4], pose[1] + pose[5]);
}

/// Convert the director pose's `zoom` component to the live camera
/// state's FOV in degrees.
///
/// `zoom = 1.0` → [`crate::FOLLOW_FOV_DEGREES`] (75°).
/// `zoom > 1.0` → narrower FOV (closer zoom).
/// `zoom < 1.0` → wider FOV (farther zoom).
/// `zoom ≤ 0.0` → falls back to the follow FOV (defensive guard
/// against scripts authored with non-positive zooms).
#[must_use]
pub fn zoom_to_fov_degrees(zoom: f32) -> f32 {
    if zoom <= 0.0 || !zoom.is_finite() {
        return crate::FOLLOW_FOV_DEGREES;
    }
    crate::FOLLOW_FOV_DEGREES / zoom
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_director_pose_sets_position_and_target() {
        let mut state = CameraState::at((0.0, 0.0));
        let pose: DirectorPose = [50.0, 100.0, 1.0, 0.0, 10.0, 20.0];
        apply_director_pose(&mut state, &pose, DirectorCameraKind::FollowPlayer);
        assert_eq!(state.position, (50.0, 100.0));
        assert_eq!(state.target, (50.0, 100.0));
        assert_eq!(state.lookahead_offset, (10.0, 20.0));
        assert_eq!(state.mode, CameraMode::Follow);
    }

    #[test]
    fn apply_director_pose_maps_zoom_to_fov() {
        let mut state = CameraState::at((0.0, 0.0));
        // zoom = 1.0 → baseline FOV
        let pose: DirectorPose = [0.0, 0.0, 1.0, 0.0, 0.0, 0.0];
        apply_director_pose(&mut state, &pose, DirectorCameraKind::FreeCam);
        assert_eq!(state.fov_degrees, crate::FOLLOW_FOV_DEGREES);
        // zoom = 2.0 → half FOV (closer zoom)
        let pose2: DirectorPose = [0.0, 0.0, 2.0, 0.0, 0.0, 0.0];
        apply_director_pose(&mut state, &pose2, DirectorCameraKind::FreeCam);
        assert_eq!(state.fov_degrees, crate::FOLLOW_FOV_DEGREES / 2.0);
    }

    #[test]
    fn apply_director_pose_handles_non_finite_zoom_defensively() {
        let mut state = CameraState::at((0.0, 0.0));
        let pose: DirectorPose = [0.0, 0.0, f32::NAN, 0.0, 0.0, 0.0];
        apply_director_pose(&mut state, &pose, DirectorCameraKind::FreeCam);
        assert_eq!(state.fov_degrees, crate::FOLLOW_FOV_DEGREES);
    }

    #[test]
    fn director_kind_round_trip_str() {
        for k in [
            DirectorCameraKind::FreeCam,
            DirectorCameraKind::FollowPlayer,
            DirectorCameraKind::ObjectiveCam,
            DirectorCameraKind::KillCam,
        ] {
            assert!(!k.as_str().is_empty());
        }
    }

    #[test]
    fn director_kind_maps_to_camera_mode() {
        assert_eq!(DirectorCameraKind::FollowPlayer.to_camera_mode(), CameraMode::Follow);
        assert_eq!(DirectorCameraKind::FreeCam.to_camera_mode(), CameraMode::FreeLook);
        assert_eq!(DirectorCameraKind::ObjectiveCam.to_camera_mode(), CameraMode::FreeLook);
        assert_eq!(DirectorCameraKind::KillCam.to_camera_mode(), CameraMode::FreeLook);
    }

    #[test]
    fn apply_director_pose_is_pure_no_rng() {
        let mut a = CameraState::at((0.0, 0.0));
        let mut b = CameraState::at((0.0, 0.0));
        let pose: DirectorPose = [123.0, -45.0, 1.5, 0.0, 0.0, 0.0];
        apply_director_pose(&mut a, &pose, DirectorCameraKind::FreeCam);
        apply_director_pose(&mut b, &pose, DirectorCameraKind::FreeCam);
        assert_eq!(a, b);
    }
}
