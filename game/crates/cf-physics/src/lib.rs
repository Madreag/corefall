//! M1: minimal 2D physics helpers.
//!
//! Stateless functions used by the `cf-control` engine each tick to step the actor
//! world. Real broadphase/narrowphase/CCD lands in M5.5 (DR-033 / T-PHYS); for M1 we
//! only need:
//!
//! - Gravity (vertical acceleration, capped at terminal velocity).
//! - Ground collision against a flat floor (M2 chunked terrain replaces this without
//!   changing the public function signatures).
//! - Recoil impulse application from a fired weapon.
//!
//! All functions are pure (they take and return values; they never call wall-clock or
//! `rand::thread_rng`). The engine's seeded RNG is wired in by callers when randomness
//! is needed.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::missing_const_for_fn
)]

use serde::{Deserialize, Serialize};

/// Inputs to [`step_kinematics`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StepInputs {
    pub position_y: f32,
    pub velocity_y: f32,
    pub gravity: f32,
    pub tick_dt: f32,
    /// World-space y of the floor (M2 will replace this with a per-pixel height query).
    pub floor_y: f32,
    /// Half-extent in the y axis (used to keep the actor's bottom on the floor).
    pub half_extent_y: f32,
    /// Maximum downward velocity allowed (terminal velocity). Negative.
    pub terminal_velocity_y: f32,
}

/// Output of [`step_kinematics`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StepOutputs {
    pub position_y: f32,
    pub velocity_y: f32,
    pub on_ground: bool,
    /// Vertical impulse absorbed by the floor (0.0 when not landing). Positive when
    /// the actor was falling and hit ground; consumers may emit a `body.landed` event
    /// when above a threshold (M5 wires this into chassis damage).
    pub landed_impulse: f32,
}

/// Apply gravity for `tick_dt` seconds, integrate velocity and position, and clamp the
/// actor against `floor_y`. Returns the new state plus a `landed_impulse` value when the
/// actor first contacts the floor this tick.
#[must_use]
pub fn step_kinematics(inputs: StepInputs) -> StepOutputs {
    // Ground-contact tolerance is 1e-3 (1 mm at the canonical scale where
    // 1 unit = 1 m, world spans ~10-1000 m). At f32 precision and that
    // scale, 1e-3 sits well above quantization noise (~1.2e-7 relative)
    // and well below sub-tick fall distance (~9.8 mm/tick at 60 Hz under
    // Earth gravity), so a "just-landed" actor is reliably detected as
    // on-ground without false positives during free-fall. When BP4-BP5
    // expand world scale beyond ~1 km, this constant should become
    // scale-relative (issue #19 follow-up) — for now the tested 60 Hz +
    // 120 Hz determinism contract holds at this scale on every CI
    // platform (Linux x86_64, Windows x86_64, macOS aarch64).
    let was_on_ground =
        (inputs.position_y - (inputs.floor_y + inputs.half_extent_y)).abs() < 1e-3 && inputs.velocity_y <= 0.0;
    let mut velocity_y = inputs.velocity_y + inputs.gravity * inputs.tick_dt;
    if velocity_y < inputs.terminal_velocity_y {
        velocity_y = inputs.terminal_velocity_y;
    }
    let mut position_y = inputs.position_y + velocity_y * inputs.tick_dt;
    let floor_top = inputs.floor_y + inputs.half_extent_y;
    let mut on_ground = false;
    let mut landed_impulse = 0.0;
    if position_y <= floor_top {
        let pre_clamp_v = velocity_y;
        position_y = floor_top;
        velocity_y = 0.0;
        on_ground = true;
        if !was_on_ground {
            landed_impulse = -pre_clamp_v.min(0.0);
        }
    }
    StepOutputs {
        position_y,
        velocity_y,
        on_ground,
        landed_impulse,
    }
}

/// Inputs for [`apply_jump`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct JumpInputs {
    pub velocity_y: f32,
    pub on_ground: bool,
    pub jump_impulse: f32,
}

/// If the actor is on the ground, set `velocity_y = jump_impulse` (positive = up).
/// Returns `(new_velocity_y, accepted)`.
#[must_use]
pub fn apply_jump(inputs: JumpInputs) -> (f32, bool) {
    if !inputs.on_ground {
        return (inputs.velocity_y, false);
    }
    (inputs.jump_impulse, true)
}

/// Inputs for [`apply_horizontal_motion`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HorizontalInputs {
    pub position_x: f32,
    pub velocity_x: f32,
    pub move_x: f32,
    pub max_speed: f32,
    pub ground_acceleration: f32,
    pub air_acceleration: f32,
    pub ground_friction: f32,
    pub on_ground: bool,
    pub tick_dt: f32,
    /// World-space horizontal bounds the actor must stay inside (inclusive). Mirrors
    /// the scenario region and replaces the M2 chunked terrain's solid-pixel walls.
    pub min_x: f32,
    pub max_x: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HorizontalOutputs {
    pub position_x: f32,
    pub velocity_x: f32,
}

/// Step horizontal motion: apply movement input, friction, and clamp to the region
/// bounds. The bounds clamp is the M1 stand-in for chunked terrain solid pixels.
#[must_use]
pub fn apply_horizontal_motion(inputs: HorizontalInputs) -> HorizontalOutputs {
    let target_speed = inputs.move_x.clamp(-1.0, 1.0) * inputs.max_speed;
    let acceleration = if inputs.on_ground {
        inputs.ground_acceleration
    } else {
        inputs.air_acceleration
    };
    let mut velocity_x = if (target_speed - inputs.velocity_x).abs() <= acceleration * inputs.tick_dt {
        target_speed
    } else {
        let dir = (target_speed - inputs.velocity_x).signum();
        inputs.velocity_x + dir * acceleration * inputs.tick_dt
    };
    if inputs.on_ground && inputs.move_x.abs() < 1e-3 {
        let friction_step = inputs.ground_friction * inputs.tick_dt;
        if velocity_x.abs() <= friction_step {
            velocity_x = 0.0;
        } else {
            velocity_x -= friction_step * velocity_x.signum();
        }
    }
    let mut position_x = inputs.position_x + velocity_x * inputs.tick_dt;
    if position_x < inputs.min_x {
        position_x = inputs.min_x;
        velocity_x = velocity_x.max(0.0);
    }
    if position_x > inputs.max_x {
        position_x = inputs.max_x;
        velocity_x = velocity_x.min(0.0);
    }
    HorizontalOutputs { position_x, velocity_x }
}

/// Apply a recoil impulse along the negation of the firer's aim direction, scaled by
/// `recoil_impulse`. `aim_x` should be the x-component of the (already-normalized) aim
/// vector; the recoil is projected through that x-component so vertical aim produces no
/// horizontal kick (instead of an arbitrary leftward jolt) and diagonal aim only kicks
/// by the horizontal projection. Vertical recoil isn't modeled in M1 (the rifle preset
/// only kicks horizontally). Returns the new horizontal velocity.
#[must_use]
pub fn apply_recoil(velocity_x: f32, aim_x: f32, recoil_impulse: f32) -> f32 {
    velocity_x - aim_x * recoil_impulse
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_step() -> StepInputs {
        StepInputs {
            position_y: 100.0,
            velocity_y: 0.0,
            gravity: -980.0,
            tick_dt: 1.0 / 60.0,
            floor_y: 0.0,
            half_extent_y: 16.0,
            terminal_velocity_y: -2000.0,
        }
    }

    #[test]
    fn gravity_accelerates_actor_downward() {
        let s = step_kinematics(default_step());
        assert!(s.velocity_y < 0.0, "gravity must accelerate downward");
        assert!(!s.on_ground);
    }

    #[test]
    fn floor_clamps_position_and_zeroes_velocity() {
        let mut inputs = default_step();
        inputs.position_y = 17.0;
        inputs.velocity_y = -2000.0;
        let s = step_kinematics(inputs);
        assert!(s.on_ground);
        assert!((s.position_y - 16.0).abs() < f32::EPSILON);
        assert!((s.velocity_y).abs() < f32::EPSILON);
        assert!(s.landed_impulse > 0.0);
    }

    #[test]
    fn terminal_velocity_caps_fall() {
        let mut inputs = default_step();
        inputs.position_y = 5000.0;
        inputs.velocity_y = -1900.0;
        // Even after one tick of gravity the velocity must NOT exceed terminal.
        let s = step_kinematics(inputs);
        assert!(s.velocity_y >= inputs.terminal_velocity_y - 1e-3);
    }

    #[test]
    fn jump_only_when_grounded() {
        let (v, ok) = apply_jump(JumpInputs {
            velocity_y: 0.0,
            on_ground: true,
            jump_impulse: 420.0,
        });
        assert!(ok);
        assert!((v - 420.0).abs() < f32::EPSILON);

        let (v_air, ok_air) = apply_jump(JumpInputs {
            velocity_y: -100.0,
            on_ground: false,
            jump_impulse: 420.0,
        });
        assert!(!ok_air);
        assert!((v_air - -100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn horizontal_motion_clamped_to_bounds() {
        let inputs = HorizontalInputs {
            position_x: -100.0,
            velocity_x: -500.0,
            move_x: -1.0,
            max_speed: 200.0,
            ground_acceleration: 1000.0,
            air_acceleration: 400.0,
            ground_friction: 800.0,
            on_ground: true,
            tick_dt: 1.0 / 60.0,
            min_x: 0.0,
            max_x: 1280.0,
        };
        let o = apply_horizontal_motion(inputs);
        assert!((o.position_x - 0.0).abs() < f32::EPSILON);
        assert!(o.velocity_x >= 0.0);
    }

    #[test]
    fn ground_friction_zeroes_idle_actor() {
        let inputs = HorizontalInputs {
            position_x: 100.0,
            velocity_x: 5.0,
            move_x: 0.0,
            max_speed: 200.0,
            ground_acceleration: 1000.0,
            air_acceleration: 400.0,
            ground_friction: 1000.0,
            on_ground: true,
            tick_dt: 1.0 / 60.0,
            min_x: 0.0,
            max_x: 1280.0,
        };
        let o = apply_horizontal_motion(inputs);
        assert!((o.velocity_x).abs() < f32::EPSILON);
    }

    #[test]
    fn recoil_pushes_against_aim() {
        let v = apply_recoil(0.0, 1.0, 50.0);
        assert!((v - -50.0).abs() < f32::EPSILON);
        let v = apply_recoil(0.0, -1.0, 50.0);
        assert!((v - 50.0).abs() < f32::EPSILON);
        // No NaN when aim_x is zero.
        let v = apply_recoil(0.0, 0.0, 25.0);
        assert!(v.is_finite());
    }
}
