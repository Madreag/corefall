//! **M14A** § "Held device sway + arm-swing during walk".
//!
//! Algorithmic port of CCCP `AHuman.cpp:2475-2535`.

use serde::{Deserialize, Serialize};

/// Default scalars from CCCP `AHuman.cpp:91-95`.
pub const FG_ARM_FLAIL_SCALAR: f32 = 0.0;
pub const BG_ARM_FLAIL_SCALAR: f32 = 0.7;
pub const ARM_SWING_RATE: f32 = 1.0;
pub const DEVICE_ARM_SWAY_RATE: f32 = 0.5;
pub const LOOK_TO_AIM_RATIO: f32 = 0.7;
pub const HEAD_SMOOTHING: f32 = 0.15;

/// Per-actor arm sway runtime state.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ArmSwayState {
    /// FG arm rotation in radians.
    pub fg_arm_rot: f32,
    /// BG arm rotation in radians.
    pub bg_arm_rot: f32,
    /// Head rotation in radians (lerps toward aim or body).
    pub head_rot: f32,
    /// `true` when BG arm reaches to support the FG 2-handed device.
    pub bg_supporting_fg: bool,
    /// Effective device-sway rate (0 = no held device; 0.5 = held; 1.0 = empty).
    pub effective_sway_rate: f32,
    /// Stride-phase scalar (0..1) used for arm swing oscillation.
    pub stride_phase: f32,
}

/// Inputs the sway tick needs.
#[derive(Debug, Clone, Copy)]
pub struct ArmSwayContext {
    /// Current actor body rotation in radians.
    pub body_rot: f32,
    /// Current aim angle in radians.
    pub aim_angle: f32,
    /// `true` while sharp aim is fully engaged (FG arm stiffens further).
    pub sharp_aim_factor: f32,
    /// `true` when FG arm holds a 2-handed weapon.
    pub two_hand_weapon: bool,
    /// `true` when actor holds any device (rifle / pistol / tool).
    pub holds_device: bool,
    /// `true` when actor's status is STABLE — drives head tracking.
    pub status_stable: bool,
    /// FG arm flail scalar (per-chassis spec, default 0.0).
    pub fg_flail_scalar: f32,
    /// BG arm flail scalar (per-chassis spec, default 0.7).
    pub bg_flail_scalar: f32,
    /// Stride progress (drives empty-arm swing).
    pub stride_progress: f32,
}

pub fn fg_arm_rotation(ctx: &ArmSwayContext) -> f32 {
    let body_contrib = ctx.fg_flail_scalar
        * ctx.body_rot.sin().abs()
        * ctx.body_rot
        * (1.0 - ctx.sharp_aim_factor);
    ctx.aim_angle + body_contrib
}

pub fn bg_arm_rotation(ctx: &ArmSwayContext, bg_supporting_fg: bool) -> f32 {
    if bg_supporting_fg {
        // 2-handed support: BG arm reaches forward toward weapon. Sway × 0.5
        // per PARITY-11.
        ctx.aim_angle + 0.05 * ctx.body_rot
    } else {
        let body_contrib = ctx.bg_flail_scalar * ctx.body_rot.sin().abs() * ctx.body_rot;
        ctx.aim_angle + body_contrib
    }
}

/// `AHuman.cpp:2461-2466`.
pub fn head_rotation_target(ctx: &ArmSwayContext) -> f32 {
    let abs_rot = ctx.body_rot.abs();
    let limit = std::f32::consts::FRAC_PI_2 + std::f32::consts::FRAC_PI_4;
    if ctx.status_stable && abs_rot < limit {
        ctx.aim_angle * LOOK_TO_AIM_RATIO + ctx.body_rot * (0.9 - LOOK_TO_AIM_RATIO)
    } else {
        ctx.body_rot * 0.6 * std::f32::consts::FRAC_PI_4
    }
}

pub fn empty_arm_swing(stride_progress: f32) -> f32 {
    // Empty arm swing: sin wave at ArmSwingRate=1.0, phase-shifted PI.
    (stride_progress * std::f32::consts::TAU + std::f32::consts::PI).sin() * ARM_SWING_RATE * 0.15
}

/// Per-tick update — drive `ArmSwayState` from `ArmSwayContext`.
pub fn tick_arm_sway(state: &mut ArmSwayState, ctx: &ArmSwayContext) {
    let bg_supporting = ctx.holds_device && ctx.two_hand_weapon;
    state.bg_supporting_fg = bg_supporting;
    state.effective_sway_rate = if ctx.holds_device {
        DEVICE_ARM_SWAY_RATE
    } else {
        ARM_SWING_RATE
    };
    state.stride_phase = ctx.stride_progress.fract();

    // FG arm sticks to aim with minimal sway (FG_ARM_FLAIL_SCALAR=0.0 by
    // default). Add stride swing when no device is held.
    let fg_base = fg_arm_rotation(ctx);
    let fg_swing = if !ctx.holds_device {
        empty_arm_swing(state.stride_phase)
    } else {
        empty_arm_swing(state.stride_phase) * DEVICE_ARM_SWAY_RATE
    };
    state.fg_arm_rot = fg_base + fg_swing;

    // BG arm: counterweight or supports.
    let bg_base = bg_arm_rotation(ctx, bg_supporting);
    let bg_swing = if !ctx.holds_device {
        -empty_arm_swing(state.stride_phase)
    } else {
        -empty_arm_swing(state.stride_phase) * DEVICE_ARM_SWAY_RATE
    };
    state.bg_arm_rot = bg_base + bg_swing;

    // Head smoothing (lerp toward target).
    let target = head_rotation_target(ctx);
    state.head_rot += (target - state.head_rot) * HEAD_SMOOTHING;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> ArmSwayContext {
        ArmSwayContext {
            body_rot: 0.0,
            aim_angle: 0.0,
            sharp_aim_factor: 0.0,
            two_hand_weapon: false,
            holds_device: false,
            status_stable: true,
            fg_flail_scalar: FG_ARM_FLAIL_SCALAR,
            bg_flail_scalar: BG_ARM_FLAIL_SCALAR,
            stride_progress: 0.0,
        }
    }

    #[test]
    fn fg_arm_stiff_to_aim() {
        let mut c = ctx();
        c.aim_angle = 0.3;
        c.body_rot = 0.15;
        // FG_ARM_FLAIL_SCALAR=0.0 → arm tracks aim exactly.
        assert!((fg_arm_rotation(&c) - 0.3).abs() < 1e-6);
    }

    #[test]
    fn bg_arm_counterweight_with_body() {
        let mut c = ctx();
        c.body_rot = 0.5;
        let bg = bg_arm_rotation(&c, false);
        // BG arm has 0.7 flail; body_rot affects rotation.
        assert!(bg.abs() > 0.0);
    }

    #[test]
    fn bg_arm_supports_two_hand_weapon() {
        let mut c = ctx();
        c.holds_device = true;
        c.two_hand_weapon = true;
        let bg = bg_arm_rotation(&c, true);
        // Supporting: tracks aim with minimal body coupling.
        assert!(bg.abs() < 0.1);
    }

    #[test]
    fn head_tracks_aim_when_stable() {
        let mut c = ctx();
        c.aim_angle = std::f32::consts::FRAC_PI_4;
        let target = head_rotation_target(&c);
        // 0.7 * pi/4 ≈ 0.55
        assert!((target - 0.7 * std::f32::consts::FRAC_PI_4).abs() < 1e-3);
    }

    #[test]
    fn head_dangles_when_unstable() {
        let mut c = ctx();
        c.status_stable = false;
        c.aim_angle = std::f32::consts::FRAC_PI_4;
        c.body_rot = 0.4;
        let target = head_rotation_target(&c);
        // When unstable, head tracks body, not aim. Result should be
        // very different from the aim-tracked value.
        let stable_target = 0.7 * std::f32::consts::FRAC_PI_4;
        assert!((target - stable_target).abs() > 0.1);
    }

    #[test]
    fn tick_arm_sway_updates_supporting_flag() {
        let mut s = ArmSwayState::default();
        let mut c = ctx();
        c.holds_device = true;
        c.two_hand_weapon = true;
        tick_arm_sway(&mut s, &c);
        assert!(s.bg_supporting_fg);
        assert!((s.effective_sway_rate - DEVICE_ARM_SWAY_RATE).abs() < 1e-6);
    }
}
