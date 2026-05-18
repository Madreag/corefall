//! **M12C**: Cinematic-owned camera state stack — replaces the gameplay
//! camera transform at the render layer during cinematic playback.
//!
//! Per spec § Notes for the implementer:
//!
//! > The cinematic camera REPLACES the gameplay camera at the render
//! > layer (`cf-render-2d::camera_takeover`); do NOT teleport the
//! > gameplay-owned camera transform — the camera stack is a separate
//! > optional resource that the renderer reads first when present.
//! > This keeps mid-cinematic save-state restorable.
//!
//! Per spec § Crates / modules touched:
//!
//! > `cf-render-2d::camera_takeover` (MODIFY): cinematic-owned camera
//! > state stack; reuses M12 juice camera_shake_amplitude.
//!
//! Architecture: this module hosts a Bevy `Resource` carrying the
//! current cinematic-owned camera state (translation + ortho-half-height
//! + shake offset). The cinematic kernel in `cf-cinematic` computes the
//! values per tick; cf-app mirrors them into this resource each frame.
//! The render system applies the offset to the active camera transform
//! WHEN the resource's `active` flag is true.

use bevy::prelude::*;

use serde::{Deserialize, Serialize};

/// **M12C** § "Cinematic-owned camera state stack". The renderer reads
/// this resource first when `active == true` and falls back to the
/// gameplay camera otherwise.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct CinematicCameraTakeover {
    /// True while a cinematic is playing (Started → Ended).
    pub active: bool,
    /// World-space translation to add to the camera transform.
    pub translation: Vec2,
    /// Orthographic half-height (zoom). `None` = use gameplay value.
    pub ortho_half_height: Option<f32>,
    /// Screen-space shake offset (px). Per spec § "Shake — perlin-noise
    /// additive offset" — reuses M12 juice camera_shake_amplitude
    /// curve via cf-cinematic.
    pub shake_px: Vec2,
    /// Active color-grade saturation multiplier (per-storyteller).
    pub color_grade_saturation: f32,
    /// Active color-grade value multiplier (per-storyteller).
    pub color_grade_value: f32,
    /// Active color-grade contrast multiplier (per-storyteller).
    pub color_grade_contrast: f32,
    /// True when the player has paused playback (the camera + ortho
    /// stay frozen at the last sampled values).
    pub paused: bool,
}

impl Default for CinematicCameraTakeover {
    fn default() -> Self {
        Self {
            active: false,
            translation: Vec2::ZERO,
            ortho_half_height: None,
            shake_px: Vec2::ZERO,
            color_grade_saturation: 1.0,
            color_grade_value: 1.0,
            color_grade_contrast: 1.0,
            paused: false,
        }
    }
}

impl CinematicCameraTakeover {
    /// Reset to the gameplay-camera path. Called on `cinematic.ended`.
    pub fn release(&mut self) {
        *self = Self::default();
    }

    /// Engage the takeover (initial state for a fresh cinematic).
    pub fn engage(&mut self) {
        self.active = true;
        self.paused = false;
        self.translation = Vec2::ZERO;
        self.shake_px = Vec2::ZERO;
        self.ortho_half_height = None;
        self.color_grade_saturation = 1.0;
        self.color_grade_value = 1.0;
        self.color_grade_contrast = 1.0;
    }

    /// Update from a `cf-cinematic::CinematicState` snapshot. Pure
    /// re-mirror — no policy.
    pub fn update_from_snapshot(&mut self, snap: &CinematicTakeoverSnapshot) {
        self.active = snap.active;
        self.translation = Vec2::new(snap.translation[0], snap.translation[1]);
        self.shake_px = Vec2::new(snap.shake_px[0], snap.shake_px[1]);
        self.ortho_half_height = if snap.ortho_half_height > 0.0 {
            Some(snap.ortho_half_height)
        } else {
            None
        };
        self.color_grade_saturation = snap.color_grade.saturation;
        self.color_grade_value = snap.color_grade.value;
        self.color_grade_contrast = snap.color_grade.contrast;
        self.paused = snap.paused;
    }

    /// Final composed offset (translation + shake-as-world-units). The
    /// shake is interpreted as world-units at the active camera scale
    /// when called by the renderer; the resource itself stores raw px.
    #[must_use]
    pub fn composed_world_offset(&self, world_per_px: f32) -> Vec2 {
        self.translation + self.shake_px * world_per_px
    }
}

/// Bevy-free intermediate snapshot the cf-app bridge fills from
/// `cf_cinematic::CinematicState` before mirroring into the
/// `CinematicCameraTakeover` resource. Decouples cf-render-2d from a
/// direct cf-cinematic dependency (which would pull the RON loader
/// into the render crate's build graph).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CinematicTakeoverSnapshot {
    /// True = takeover active.
    pub active: bool,
    /// Translation in world units.
    pub translation: [f32; 2],
    /// Shake offset in screen pixels.
    pub shake_px: [f32; 2],
    /// Orthographic half-height (`<= 0.0` = no override).
    pub ortho_half_height: f32,
    /// Per-storyteller color grade.
    pub color_grade: ColorGradeSnapshot,
    /// True = paused.
    pub paused: bool,
}

/// Bevy-free color grade snapshot. Mirrors
/// `cf_cinematic::storyteller_profile::ColorGradeBias`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ColorGradeSnapshot {
    /// Saturation multiplier.
    pub saturation: f32,
    /// Value multiplier.
    pub value: f32,
    /// Contrast multiplier.
    pub contrast: f32,
}

impl Default for ColorGradeSnapshot {
    fn default() -> Self {
        Self {
            saturation: 1.0,
            value: 1.0,
            contrast: 1.0,
        }
    }
}

impl Default for CinematicTakeoverSnapshot {
    fn default() -> Self {
        Self {
            active: false,
            translation: [0.0, 0.0],
            shake_px: [0.0, 0.0],
            ortho_half_height: 0.0,
            color_grade: ColorGradeSnapshot::default(),
            paused: false,
        }
    }
}

/// Plugin that registers the `CinematicCameraTakeover` resource. cf-app
/// adds this plugin alongside `JuicePlugin` + `ColorGradingPlugin`. The
/// renderer system that consumes the resource lives in cf-app since the
/// camera entity itself is owned at the binary integration layer.
#[derive(Default)]
pub struct CinematicCameraPlugin;

impl Plugin for CinematicCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CinematicCameraTakeover>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_inactive() {
        let s = CinematicCameraTakeover::default();
        assert!(!s.active);
        assert_eq!(s.translation, Vec2::ZERO);
        assert!(s.ortho_half_height.is_none());
        assert_eq!(s.shake_px, Vec2::ZERO);
        assert!(!s.paused);
    }

    #[test]
    fn engage_sets_active_flag_and_clears_offsets() {
        let mut s = CinematicCameraTakeover::default();
        s.translation = Vec2::new(99.0, 0.0);
        s.engage();
        assert!(s.active);
        assert_eq!(s.translation, Vec2::ZERO);
        assert!(s.ortho_half_height.is_none());
    }

    #[test]
    fn release_returns_to_default() {
        let mut s = CinematicCameraTakeover::default();
        s.engage();
        s.translation = Vec2::new(5.0, 3.0);
        s.release();
        assert!(!s.active);
        assert_eq!(s.translation, Vec2::ZERO);
    }

    #[test]
    fn update_from_snapshot_mirrors_every_field() {
        let mut s = CinematicCameraTakeover::default();
        let snap = CinematicTakeoverSnapshot {
            active: true,
            translation: [10.0, 5.0],
            shake_px: [3.0, -2.0],
            ortho_half_height: 12.0,
            color_grade: ColorGradeSnapshot {
                saturation: 0.85,
                value: 0.92,
                contrast: 1.0,
            },
            paused: false,
        };
        s.update_from_snapshot(&snap);
        assert!(s.active);
        assert_eq!(s.translation, Vec2::new(10.0, 5.0));
        assert_eq!(s.shake_px, Vec2::new(3.0, -2.0));
        assert_eq!(s.ortho_half_height, Some(12.0));
        assert!((s.color_grade_saturation - 0.85).abs() < 1e-3);
    }

    #[test]
    fn composed_world_offset_scales_shake_by_world_per_px() {
        let mut s = CinematicCameraTakeover::default();
        s.translation = Vec2::new(10.0, 0.0);
        s.shake_px = Vec2::new(8.0, 0.0);
        let offset = s.composed_world_offset(0.5);
        assert!((offset.x - (10.0 + 8.0 * 0.5)).abs() < 1e-3);
    }
}
