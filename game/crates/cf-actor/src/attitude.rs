//! **M14A** § "Rotational balancing spring — the CC 'I'm standing up' magic".
//!
//! Algorithmic port of CCCP `AHuman.cpp:2655-2725`. The Rust is original; the
//! constants are the calibration the player feels (`SPRING_DAMPING_BASE=0.98`,
//! `WALK_ROT_TARGET=0.15`, etc.). See `specs/active/M14A.md` § Constants.
//!
//! Three sub-springs that pick which one runs from [`AttitudeStatus`]:
//!   - STABLE: hold the per-stance lean target with health-modulated damping.
//!   - UNSTABLE: weak spring biased toward fall direction.
//!   - DYING: strong spring sweeps rotation toward sideways over 125 ms.
//!   - PRONE: 0.65 spring holds rotation near +π/2.
//!   - GOPRONE: 0.4 spring pulls rotation toward flat over 333 ms.

use serde::{Deserialize, Serialize};

use crate::move_state::{MoveState, ProneState};

pub const STAND_ROT_TARGET: f32 = 0.0;
pub const WALK_ROT_TARGET: f32 = 0.15;
pub const CROUCH_ROT_TARGET: f32 = 0.30;
pub const JUMP_ROT_TARGET: f32 = 0.45;
pub const SPRING_STRENGTH: f32 = 0.5;
pub const SPRING_DAMPING_BASE: f32 = 0.98;
pub const SPRING_DAMPING_HEALTH_COEF: f32 = 0.06;
pub const UNSTABLE_SPRING_K: f32 = 0.05;
pub const DYING_SPRING_K_SCALAR: f32 = 0.5;
pub const DYING_DURATION_MS: u32 = 125;
pub const STABLE_RECOVER_MS: u32 = 1500;
pub const PRONE_TRANSITION_MS: u32 = 333;
pub const PRONE_GOSPRING_K: f32 = 0.4;
pub const PRONE_HOLD_SPRING_K: f32 = 0.65;
pub const PRONE_DAMP_FACTOR: f32 = 0.85;
pub const MAX_WALKPATH_CROUCH_SHIFT: f32 = 6.0;
pub const MAX_CROUCH_ROTATION: f32 = 0.45;

/// Coarse status for the attitude spring — separate from the actor status
/// machine because the attitude system uses CCCP's `Actor.Status` enum that
/// only has the rotation-relevant 4 states.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttitudeStatus {
    Stable = 0,
    Unstable = 1,
    Dying = 2,
    Dead = 3,
}

impl Default for AttitudeStatus {
    fn default() -> Self {
        AttitudeStatus::Stable
    }
}

/// Per-stance lean targets array. Index by [`MoveState::target_index`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RotAngleTargets(pub [f32; MoveState::COUNT]);

impl Default for RotAngleTargets {
    fn default() -> Self {
        let mut targets = [0.0; MoveState::COUNT];
        targets[MoveState::NoMove.target_index()] = STAND_ROT_TARGET;
        targets[MoveState::Stand.target_index()] = STAND_ROT_TARGET;
        targets[MoveState::Walk.target_index()] = WALK_ROT_TARGET;
        targets[MoveState::Crouch.target_index()] = CROUCH_ROT_TARGET;
        targets[MoveState::Crawl.target_index()] = std::f32::consts::FRAC_PI_2;
        targets[MoveState::ArmCrawl.target_index()] = std::f32::consts::FRAC_PI_2;
        targets[MoveState::Climb.target_index()] = STAND_ROT_TARGET;
        targets[MoveState::Jump.target_index()] = JUMP_ROT_TARGET;
        targets[MoveState::Dislodge.target_index()] = WALK_ROT_TARGET;
        targets[MoveState::Hover.target_index()] = STAND_ROT_TARGET;
        Self(targets)
    }
}

impl RotAngleTargets {
    pub fn get(&self, state: MoveState) -> f32 {
        self.0[state.target_index()]
    }

    pub fn set(&mut self, state: MoveState, value: f32) {
        self.0[state.target_index()] = value;
    }
}

/// Per-actor attitude state — what the spring needs to read + write each tick.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct AttitudeState {
    /// Current chassis rotation in radians (0 = upright).
    pub rot: f32,
    /// Current angular velocity in radians/sec.
    pub angular_vel: f32,
    /// Cached target the last spring tick aimed at — surfaced to HUD via
    /// `observe.actor.attitude.rot_target`.
    pub rot_target: f32,
    /// Stable recovery timer in ms (UNSTABLE → STABLE when this elapses).
    pub stable_recover_timer_ms: u32,
    /// Dying timer in ms (DYING → DEAD when this exceeds DYING_DURATION_MS).
    pub dying_timer_ms: u32,
    /// Prone transition timer in ms.
    pub prone_timer_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct WalkAngleState {
    /// Foreground leg slope in radians (-PI/2..+PI/2).
    pub fg: f32,
    /// Background leg slope in radians.
    pub bg: f32,
}

/// Walk path offset — relative offset to apply to limb paths (crouch + lean).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct WalkPathOffset {
    pub x: f32,
    pub y: f32,
}

/// Context for one spring tick.
#[derive(Debug, Clone, Copy)]
pub struct SpringContext {
    /// Current movement state (drives lean-target lookup).
    pub move_state: MoveState,
    /// Per-stance lean targets.
    pub rot_angle_targets: RotAngleTargets,
    /// Aim angle in radians (positive = aiming up).
    pub aim_angle: f32,
    /// `true` when chassis is facing left (CCCP `m_HFlipped`).
    pub h_flipped: bool,
    /// Current HP for damping ramp.
    pub health: f32,
    /// Max HP for normalization.
    pub max_health: f32,
    /// Horizontal velocity (for UNSTABLE fall direction).
    pub velocity_x: f32,
    /// Walk path offset (drives crouch lean).
    pub walk_path_offset: WalkPathOffset,
    /// Max crouch rotation.
    pub max_crouch_rotation: f32,
    /// Max walk-path crouch shift.
    pub max_walkpath_crouch_shift: f32,
}

impl SpringContext {
    /// CCCP `GetFlipFactor()` — flip the lean sign based on facing.
    pub fn facing_factor(&self) -> f32 {
        if self.h_flipped {
            -1.0
        } else {
            1.0
        }
    }
}

/// CCCP `LERP(x1, x2, y1, y2, x)` — linear map x∈[x1, x2] → y∈[y1, y2].
fn lerp(x1: f32, x2: f32, y1: f32, y2: f32, x: f32) -> f32 {
    if (x2 - x1).abs() < 1e-9 {
        return y1;
    }
    let t = ((x - x1) / (x2 - x1)).clamp(0.0, 1.0);
    y1 + (y2 - y1) * t
}

pub fn attitude_spring_tick_stable(state: &mut AttitudeState, ctx: &SpringContext) {
    let target = ctx.rot_angle_targets.get(ctx.move_state) * ctx.facing_factor();
    let aim_dampen = if ctx.aim_angle > 0.0 {
        1.0 - ctx.aim_angle / std::f32::consts::FRAC_PI_2
    } else {
        1.0
    };
    let mut rot_target = target * aim_dampen;

    let crouch_adjust = if ctx.h_flipped {
        ctx.max_crouch_rotation
    } else {
        -ctx.max_crouch_rotation
    };
    rot_target += lerp(
        0.0,
        ctx.max_walkpath_crouch_shift,
        0.0,
        crouch_adjust,
        -ctx.walk_path_offset.y,
    );

    state.rot_target = rot_target;
    let rot_diff = state.rot - rot_target;
    let max_health = ctx.max_health.max(1.0);
    let health_ratio = (ctx.health / max_health).clamp(0.0, 1.0);
    let damping = SPRING_DAMPING_BASE - SPRING_DAMPING_HEALTH_COEF * (1.0 - health_ratio);
    state.angular_vel = state.angular_vel * damping - rot_diff * SPRING_STRENGTH;
    state.rot += state.angular_vel * (1.0 / 60.0);
}

pub fn attitude_spring_tick_unstable(state: &mut AttitudeState, ctx: &SpringContext) {
    let half_pi = std::f32::consts::FRAC_PI_2;
    let rot_target = if ctx.velocity_x.abs() > 1.0 {
        if ctx.velocity_x > 0.0 {
            -half_pi
        } else {
            half_pi
        }
    } else if state.rot > 0.0 {
        half_pi
    } else {
        -half_pi
    };
    state.rot_target = rot_target;
    let rot_diff = rot_target - state.rot;
    if rot_diff.abs() > 0.1 && rot_diff.abs() < std::f32::consts::PI {
        state.angular_vel += rot_diff * UNSTABLE_SPRING_K;
    }
    state.rot += state.angular_vel * (1.0 / 60.0);
}

///
/// Returns `true` when the dying dwell exceeds 125 ms — caller should
/// transition status to DEAD.
pub fn attitude_spring_tick_dying(state: &mut AttitudeState, ctx: &SpringContext, dt_ms: u32) -> bool {
    let half_pi = std::f32::consts::FRAC_PI_2;
    let rot_target = if ctx.velocity_x - (state.rot + state.angular_vel) > 0.0 {
        -half_pi
    } else {
        half_pi
    };
    state.rot_target = rot_target;
    state.dying_timer_ms = state.dying_timer_ms.saturating_add(dt_ms);
    if state.dying_timer_ms < DYING_DURATION_MS {
        let rot_diff = rot_target - state.rot;
        if rot_diff.abs() > 0.1 && rot_diff.abs() < std::f32::consts::PI {
            let vel_scalar = DYING_SPRING_K_SCALAR;
            state.angular_vel += rot_diff * vel_scalar;
        }
        state.rot += state.angular_vel * (1.0 / 60.0);
        false
    } else {
        true
    }
}

///
/// Returns the (possibly new) prone state after this tick.
pub fn tick_prone_state_machine(
    state: &mut AttitudeState,
    prone: ProneState,
    dt_ms: u32,
    crouch_active: bool,
    movement_input: bool,
) -> ProneState {
    let half_pi = std::f32::consts::FRAC_PI_2;
    match prone {
        ProneState::NotProne => {
            if crouch_active && movement_input {
                state.prone_timer_ms = 0;
                ProneState::GoProne
            } else {
                ProneState::NotProne
            }
        }
        ProneState::GoProne => {
            state.prone_timer_ms = state.prone_timer_ms.saturating_add(dt_ms);
            let target = if state.rot > 0.0 { half_pi } else { -half_pi };
            state.rot_target = target;
            let rot_diff = target - state.rot;
            state.angular_vel += rot_diff * PRONE_GOSPRING_K;
            state.rot += state.angular_vel * (1.0 / 60.0);
            if state.prone_timer_ms >= PRONE_TRANSITION_MS {
                ProneState::Prone
            } else {
                ProneState::GoProne
            }
        }
        ProneState::Prone => {
            let target = if state.rot > 0.0 { half_pi } else { -half_pi };
            state.rot_target = target;
            let rot_diff = target - state.rot;
            state.angular_vel = state.angular_vel * PRONE_DAMP_FACTOR - rot_diff * PRONE_HOLD_SPRING_K;
            state.rot += state.angular_vel * (1.0 / 60.0);
            if !crouch_active && !movement_input {
                state.prone_timer_ms = 0;
                ProneState::NotProne
            } else {
                ProneState::Prone
            }
        }
    }
}

///
/// Per-leg WalkAngle lerps to sampled slope (provided by caller from
/// cf-terrain `cast_strength_ray`) clamped to ±40°.
#[allow(clippy::similar_names)]
pub fn tick_walk_angle(state: &mut WalkAngleState, sampled_fg: f32, sampled_bg: f32, dt_secs: f32) {
    let clamp = (std::f32::consts::PI / 180.0) * 40.0;
    let fg_target = sampled_fg.clamp(-clamp, clamp);
    let bg_target = sampled_bg.clamp(-clamp, clamp);
    let smoothing = 4.0; // WALK_ANGLE_SMOOTHING_PER_SEC
    state.fg += (fg_target - state.fg) * (smoothing * dt_secs).min(1.0);
    state.bg += (bg_target - state.bg) * (smoothing * dt_secs).min(1.0);
}

/// `angular_vel += (offset × impulse_perpendicular) / MOI`.
///
/// Returns the change in angular velocity.
pub fn angular_impulse_from_offcenter_hit(
    hit_offset: [f32; 2],
    impulse: [f32; 2],
    moment_of_inertia: f32,
) -> f32 {
    let moi = moment_of_inertia.max(1e-3);
    // 2D cross product = x1*y2 - y1*x2
    let cross = hit_offset[0] * impulse[1] - hit_offset[1] * impulse[0];
    cross / moi
}

pub fn evaluate_knockdown(incoming_impulse_magnitude: f32, actor_mass_kg: f32) -> KnockdownOutcome {
    let stagger_threshold = actor_mass_kg.max(0.001) * 5.0;
    if incoming_impulse_magnitude > stagger_threshold {
        KnockdownOutcome::Knockdown
    } else if incoming_impulse_magnitude > stagger_threshold * 0.5 {
        KnockdownOutcome::Stagger
    } else {
        KnockdownOutcome::None
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnockdownOutcome {
    None = 0,
    Stagger = 1,
    Knockdown = 2,
}

impl KnockdownOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            KnockdownOutcome::None => "none",
            KnockdownOutcome::Stagger => "stagger",
            KnockdownOutcome::Knockdown => "knockdown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_for(state: MoveState) -> SpringContext {
        SpringContext {
            move_state: state,
            rot_angle_targets: RotAngleTargets::default(),
            aim_angle: 0.0,
            h_flipped: false,
            health: 100.0,
            max_health: 100.0,
            velocity_x: 0.0,
            walk_path_offset: WalkPathOffset::default(),
            max_crouch_rotation: MAX_CROUCH_ROTATION,
            max_walkpath_crouch_shift: MAX_WALKPATH_CROUCH_SHIFT,
        }
    }

    #[test]
    fn rot_targets_default_match_constants() {
        let t = RotAngleTargets::default();
        assert!((t.get(MoveState::Stand) - STAND_ROT_TARGET).abs() < 1e-6);
        assert!((t.get(MoveState::Walk) - WALK_ROT_TARGET).abs() < 1e-6);
        assert!((t.get(MoveState::Crouch) - CROUCH_ROT_TARGET).abs() < 1e-6);
        assert!((t.get(MoveState::Jump) - JUMP_ROT_TARGET).abs() < 1e-6);
    }

    #[test]
    fn stable_spring_converges_toward_target() {
        let mut state = AttitudeState {
            rot: 0.3,
            ..Default::default()
        };
        let ctx = ctx_for(MoveState::Stand);
        for _ in 0..120 {
            attitude_spring_tick_stable(&mut state, &ctx);
        }
        assert!(state.rot.abs() < 0.05, "rot did not converge: {}", state.rot);
    }

    #[test]
    fn walk_state_targets_positive_lean() {
        let mut state = AttitudeState::default();
        let ctx = ctx_for(MoveState::Walk);
        attitude_spring_tick_stable(&mut state, &ctx);
        assert!((state.rot_target - WALK_ROT_TARGET).abs() < 1e-6);
    }

    #[test]
    fn aiming_up_reduces_lean_target() {
        let mut state = AttitudeState::default();
        let mut ctx = ctx_for(MoveState::Walk);
        ctx.aim_angle = std::f32::consts::FRAC_PI_4;
        attitude_spring_tick_stable(&mut state, &ctx);
        // aim at pi/4 → dampen by 1 - 0.5 = 0.5
        assert!((state.rot_target - WALK_ROT_TARGET * 0.5).abs() < 1e-3);
    }

    #[test]
    fn unstable_falls_in_velocity_direction() {
        let mut state = AttitudeState::default();
        let mut ctx = ctx_for(MoveState::Walk);
        ctx.velocity_x = 5.0;
        attitude_spring_tick_unstable(&mut state, &ctx);
        // Moving right → rotation target is -PI/2 (falls forward / right).
        assert!(state.rot_target < 0.0);
    }

    #[test]
    fn dying_completes_within_125ms() {
        let mut state = AttitudeState::default();
        let ctx = ctx_for(MoveState::Stand);
        // 30 ticks * 4ms = 120ms — should still be dying.
        let mut completed = false;
        for _ in 0..30 {
            if attitude_spring_tick_dying(&mut state, &ctx, 4) {
                completed = true;
                break;
            }
        }
        assert!(!completed);
        // One more 5 ms push tips us past 125 ms.
        let done = attitude_spring_tick_dying(&mut state, &ctx, 10);
        assert!(done);
    }

    #[test]
    fn knockdown_threshold_scales_with_mass() {
        // 80 kg infantry: stagger threshold 400 N·s
        assert_eq!(evaluate_knockdown(150.0, 80.0), KnockdownOutcome::None);
        assert_eq!(evaluate_knockdown(250.0, 80.0), KnockdownOutcome::Stagger);
        assert_eq!(evaluate_knockdown(500.0, 80.0), KnockdownOutcome::Knockdown);
        // 380 kg heavy trooper: stagger threshold 1900 N·s
        assert_eq!(evaluate_knockdown(200.0, 380.0), KnockdownOutcome::None);
        assert_eq!(evaluate_knockdown(2000.0, 380.0), KnockdownOutcome::Knockdown);
    }

    #[test]
    fn prone_state_machine_runs_through_333ms_transition() {
        let mut state = AttitudeState::default();
        let mut prone = ProneState::NotProne;
        // Crouch + movement triggers GoProne.
        prone = tick_prone_state_machine(&mut state, prone, 16, true, true);
        assert_eq!(prone, ProneState::GoProne);
        // 21 * 16 = 336 ms — exceeds 333 ms threshold.
        for _ in 0..21 {
            prone = tick_prone_state_machine(&mut state, prone, 16, true, true);
        }
        assert_eq!(prone, ProneState::Prone);
        // Release inputs → returns to NotProne.
        prone = tick_prone_state_machine(&mut state, prone, 16, false, false);
        assert_eq!(prone, ProneState::NotProne);
    }

    #[test]
    fn off_center_hit_produces_angular_velocity() {
        // Hit offset [+4, +12] with impulse [-100, 0] → cross product < 0 → spin one way.
        let dv = angular_impulse_from_offcenter_hit([4.0, 12.0], [-100.0, 0.0], 1000.0);
        assert!(dv > 0.0); // 4*0 - 12*(-100) = 1200 / 1000 = 1.2
        assert!((dv - 1.2).abs() < 1e-6);
    }
}
