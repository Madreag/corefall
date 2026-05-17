//! cf-camera — M8 camera + game-feel surface (smooth follow + hit-stop +
//! scope + free-look).
//!
//! Owns the per-frame camera state machine and produces deterministic state
//! transitions consumable by cf-app's render layer and cf-control's
//! `observe.camera`. The crate keeps zero Bevy/render coupling so unit
//! tests stay headless and the engine snapshot surface is portable.
//!
//! Public surface:
//!
//! - [`CameraMode`] — Follow / Scope / FreeLook discriminator.
//! - [`CameraState`] — position + lookahead offset + hit-stop residual ms +
//!   scope FOV + free-look anchor.
//! - [`tick_smooth_follow`] — lerp + lookahead per spec § 200ms ease.
//! - [`trigger_hit_stop`] / [`tick_hit_stop`] — 50-200ms freeze per spec.
//! - [`enter_scope`] / [`exit_scope`] — 30° FOV per spec.
//! - [`enter_free_look`] / [`exit_free_look`] — RMB-toggled cursor follow.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod director_hook;
pub mod follow;
pub mod free_look;
pub mod hit_stop;
pub mod scope;

use serde::{Deserialize, Serialize};

/// 200ms smooth-follow lerp constant per M8 spec § "lerp + lookahead +
/// deadzone (200ms ease)".
pub const LERP_FACTOR_MS: u32 = 200;
/// Default scope FOV in degrees (sniper ADS) per M8 spec § "Scope zoom
/// (sniper ADS — 30° FOV)".
pub const SCOPE_FOV_DEGREES: f32 = 30.0;
/// Default Follow-mode FOV in degrees.
pub const FOLLOW_FOV_DEGREES: f32 = 75.0;
/// Default hit-stop pulse duration in ms (mid-band of 50-200ms).
pub const DEFAULT_HIT_STOP_MS: u32 = 100;
/// Maximum hit-stop pulse duration in ms (per spec — 50..200ms band).
pub const MAX_HIT_STOP_MS: u32 = 200;
/// Minimum hit-stop pulse duration in ms.
pub const MIN_HIT_STOP_MS: u32 = 50;
/// Default smooth-follow deadzone radius in world units. Camera does not
/// re-target while target is within this radius of the current position.
pub const DEFAULT_DEADZONE_RADIUS: f32 = 8.0;
/// Default lookahead magnitude in world units (added to target position
/// along velocity direction).
pub const DEFAULT_LOOKAHEAD_RANGE: f32 = 24.0;

/// Camera mode discriminator. Spec § Camera + game feel: smooth follow,
/// scope ADS, and free-look (RMB toggle) are the three top-level modes.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CameraMode {
    /// Default smooth-follow on the player actor.
    #[default]
    Follow,
    /// Sniper scope ADS at `SCOPE_FOV_DEGREES`.
    Scope,
    /// Cursor-driven free-look (RMB toggle) within `max_distance` of the
    /// player actor.
    FreeLook,
}

impl CameraMode {
    /// Canonical snake_case identifier surfaced to cfctl + replay bundles.
    pub fn as_str(self) -> &'static str {
        match self {
            CameraMode::Follow => "follow",
            CameraMode::Scope => "scope",
            CameraMode::FreeLook => "free_look",
        }
    }

    /// Parse from the cfctl wire form.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<CameraMode> {
        Some(match value {
            "follow" => CameraMode::Follow,
            "scope" => CameraMode::Scope,
            "free_look" => CameraMode::FreeLook,
            _ => return None,
        })
    }
}

/// Per-frame camera state. Owned by cf-control's engine and snapshotted
/// into the run bundle envelope so replay can reconstruct camera mode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraState {
    /// Active camera mode.
    pub mode: CameraMode,
    /// World-space camera position (the rendered viewport center).
    pub position: (f32, f32),
    /// World-space target the camera is converging to.
    pub target: (f32, f32),
    /// Lookahead vector (added to `target` based on player velocity).
    pub lookahead_offset: (f32, f32),
    /// Remaining hit-stop pulse in milliseconds. While > 0, the sim should
    /// pause its time-step (rendering continues).
    pub hit_stop_remaining_ms: u32,
    /// Active FOV in degrees.
    pub fov_degrees: f32,
    /// Maximum free-look distance from the player actor in world units.
    pub free_look_max_distance: f32,
    /// World-space cursor anchor while in FreeLook mode.
    pub free_look_cursor: (f32, f32),
    /// Smooth-follow deadzone radius in world units.
    pub deadzone_radius: f32,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            mode: CameraMode::Follow,
            position: (0.0, 0.0),
            target: (0.0, 0.0),
            lookahead_offset: (0.0, 0.0),
            hit_stop_remaining_ms: 0,
            fov_degrees: FOLLOW_FOV_DEGREES,
            free_look_max_distance: 200.0,
            free_look_cursor: (0.0, 0.0),
            deadzone_radius: DEFAULT_DEADZONE_RADIUS,
        }
    }
}

impl CameraState {
    /// Construct a fresh follow-mode state at the supplied world position.
    pub fn at(position: (f32, f32)) -> Self {
        Self {
            position,
            target: position,
            ..Self::default()
        }
    }

    /// Returns true while the hit-stop pulse should pause sim time-step.
    pub fn is_hit_stop_active(&self) -> bool {
        self.hit_stop_remaining_ms > 0
    }
}

pub use director_hook::{
    apply_director_pose, zoom_to_fov_degrees, DirectorCameraKind, DirectorPose, DIRECTOR_POSE_COMPONENTS,
};
pub use follow::tick_smooth_follow;
pub use free_look::{enter_free_look, exit_free_look};
pub use hit_stop::{tick_hit_stop, trigger_hit_stop};
pub use scope::{enter_scope, exit_scope};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_mode_round_trip_str() {
        for m in [CameraMode::Follow, CameraMode::Scope, CameraMode::FreeLook] {
            assert_eq!(CameraMode::from_str(m.as_str()), Some(m));
        }
    }

    #[test]
    fn camera_state_at_initialises_follow_at_position() {
        let s = CameraState::at((10.0, 20.0));
        assert_eq!(s.mode, CameraMode::Follow);
        assert_eq!(s.position, (10.0, 20.0));
        assert_eq!(s.target, (10.0, 20.0));
        assert_eq!(s.fov_degrees, FOLLOW_FOV_DEGREES);
    }
}
