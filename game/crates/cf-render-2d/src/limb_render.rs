//! **M14A** § "cf-render-2d::limb_render" — sprite frame index keyed off
//! `stride_frame_index` + chassis rotation rendering + per-arm rotation.
//!
//! Pure data-transform helpers — the actual Bevy `Transform`/`Sprite`
//! mutations happen in `cf-app` which depends on these helpers.

use serde::{Deserialize, Serialize};

/// frames cycle through `walk_1..walk_6` per stride; idle frame is `walk_0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WalkSpriteFrame {
    /// Per-stride frame index (0-5).
    pub frame: u8,
    /// `true` on the tick a stride plant happened.
    pub stride_frame_just_fired: bool,
}

impl WalkSpriteFrame {
    /// Advance one tick. `stride_fired=true` advances the frame and stamps
    /// `stride_frame_just_fired`. Idle (no stride) returns idle frame.
    pub fn advance(&mut self, stride_fired: bool, walking: bool) {
        self.stride_frame_just_fired = stride_fired;
        if stride_fired {
            self.frame = (self.frame + 1) % 6;
        } else if !walking {
            self.frame = 0;
        }
    }
}

/// to a sprite transform z-rotation in radians. Returns angle in radians for
/// the renderer to feed to Bevy's `Transform::rotation_z`.
pub fn body_rotation_for_render(body_rot_rad: f32, h_flipped: bool) -> f32 {
    if h_flipped {
        -body_rot_rad
    } else {
        body_rot_rad
    }
}

/// the renderer.
pub fn fg_arm_rotation_for_render(fg_arm_rot_rad: f32, h_flipped: bool) -> f32 {
    if h_flipped {
        -fg_arm_rot_rad
    } else {
        fg_arm_rot_rad
    }
}

pub fn bg_arm_rotation_for_render(bg_arm_rot_rad: f32, h_flipped: bool) -> f32 {
    if h_flipped {
        -bg_arm_rot_rad
    } else {
        bg_arm_rot_rad
    }
}

pub fn head_rotation_for_render(head_rot_rad: f32, h_flipped: bool) -> f32 {
    if h_flipped {
        -head_rot_rad
    } else {
        head_rot_rad
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walk_frame_cycles_on_each_stride() {
        let mut f = WalkSpriteFrame::default();
        for _ in 0..6 {
            f.advance(true, true);
        }
        assert_eq!(f.frame, 0); // wrapped back to 0
    }

    #[test]
    fn walk_frame_returns_to_idle_when_not_walking() {
        let mut f = WalkSpriteFrame::default();
        f.frame = 3;
        f.advance(false, false);
        assert_eq!(f.frame, 0);
    }

    #[test]
    fn body_rotation_negates_for_flipped_facing() {
        assert_eq!(body_rotation_for_render(0.3, true), -0.3);
        assert_eq!(body_rotation_for_render(0.3, false), 0.3);
    }
}
