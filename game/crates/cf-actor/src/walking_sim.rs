//! **M14A** § "Walking sim per-tick driver".
//!
//! Single public entry: [`walk_sim_tick`] — takes an `ActorState` + tick
//! context and advances every M14A subsystem in lockstep:
//!
//! 1. Update `MoveState` from velocity + intents (Walk vs Stand vs Crouch).
//! 2. Tick attitude spring (STABLE/UNSTABLE/DYING + prone state machine).
//! 3. Tick per-leg `WalkAngle` slope adapter.
//! 4. Tick arm sway + held device sway + head aim.
//! 5. Advance limb paths and emit `stride_frame` on completion.
//! 6. Tick jetpack if equipped (and apply thrust to actor velocity).
//! 7. Apply atmosphere overlay walk-speed + jet thrust pressure scaling.
//! 8. Recompute total mass if dirty.
//! 9. Tick quick-action bar cooldowns + radial phase.
//!
//! Returns [`WalkSimEvents`] so the engine layer can emit replay events.

use serde::{Deserialize, Serialize};

use crate::{
    arm_sway::{tick_arm_sway, ArmSwayContext, BG_ARM_FLAIL_SCALAR, FG_ARM_FLAIL_SCALAR},
    attitude::{
        attitude_spring_tick_dying, attitude_spring_tick_stable, attitude_spring_tick_unstable,
        tick_prone_state_machine, tick_walk_angle, AttitudeStatus, RotAngleTargets, SpringContext,
        MAX_CROUCH_ROTATION, MAX_WALKPATH_CROUCH_SHIFT,
    },
    limb_path::PathSide,
    mass_aggregator,
    move_state::{MoveState, ProneState},
    sim_overlay::compute_overlay,
    ActorState, AtmosphereSample, Status,
};

/// Inputs passed to [`walk_sim_tick`].
#[derive(Debug, Clone, Copy)]
pub struct WalkSimContext {
    /// Current tick number (monotonic).
    pub tick: u64,
    /// Wall-clock-stable ms since last tick.
    pub dt_ms: u32,
    /// `true` while the actor's horizontal movement intent is non-zero.
    pub move_input_active: bool,
    /// Movement intent sign (+1 right, -1 left, 0 idle).
    pub move_x: f32,
    /// `true` while the player holds the crouch toggle.
    pub crouch_input_active: bool,
    /// `true` while the jet/jump intent is active (sustain).
    pub jet_hold: bool,
    /// `true` on the tick the jet button was pressed (edge).
    pub jet_press_edge: bool,
    /// Atmosphere sample at the actor's position.
    pub atmosphere: AtmosphereSample,
    /// Reduce-motion accessibility flag (drives QAB radial time-slow).
    pub reduce_motion: bool,
}

/// Events emitted by one walk-sim tick.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct WalkSimEvents {
    pub stride_fired: bool,
    pub stride_side: u8, // 0 = fg, 1 = bg
    pub move_state_changed: Option<MoveState>,
    pub prone_state_changed: Option<ProneState>,
    pub jet_fired: bool,
    pub jet_exhausted: bool,
    pub jet_relit: bool,
    pub jet_thrust_n: f32,
    pub mass_changed: bool,
    pub knockdown: bool,
    pub foot_slip: bool,
    pub hypoxia_severity: u8,
    pub combustion_ready: bool,
    pub wind_force_applied: [f32; 2],
    pub atmosphere_warning_severity: u8,
}

/// Advance the actor's walking sim by one tick.
pub fn walk_sim_tick(actor: &mut ActorState, ctx: WalkSimContext) -> WalkSimEvents {
    let mut events = WalkSimEvents::default();

    // ----- 1) MoveState derivation -----
    let prev_state = actor.move_state;
    let on_ground = actor.on_ground;
    let velocity_mag = (actor.velocity.x * actor.velocity.x + actor.velocity.y * actor.velocity.y).sqrt();
    let new_state = if !on_ground {
        if actor.jet_active {
            MoveState::Jump
        } else {
            MoveState::Jump
        }
    } else if actor.limb_loss.both_legs_lost {
        MoveState::ArmCrawl
    } else if actor.prone_active {
        MoveState::Crawl
    } else if ctx.crouch_input_active {
        MoveState::Crouch
    } else if velocity_mag > actor.walk_threshold {
        MoveState::Walk
    } else {
        MoveState::Stand
    };
    actor.move_state = new_state;
    if prev_state != new_state {
        events.move_state_changed = Some(new_state);
    }

    // ----- 2) Attitude status derivation -----
    let attitude_status = match actor.status {
        Status::Stable | Status::Unstable => {
            // Above π = UNSTABLE; otherwise STABLE.
            if actor.attitude.rot.abs() > std::f32::consts::PI {
                AttitudeStatus::Unstable
            } else {
                AttitudeStatus::Stable
            }
        }
        Status::Downed | Status::Dying => AttitudeStatus::Dying,
        Status::Dead => AttitudeStatus::Dead,
        Status::Inactive => AttitudeStatus::Stable,
        // INERT robot slumps like a downed actor (limp, non-responsive).
        Status::Inert => AttitudeStatus::Dying,
    };

    let spring_ctx = SpringContext {
        move_state: actor.move_state,
        rot_angle_targets: RotAngleTargets::default(),
        aim_angle: actor.aim.y.atan2(actor.aim.x.abs().max(1e-3)),
        h_flipped: matches!(actor.facing, crate::FacingDirection::Left),
        health: actor.hp,
        max_health: actor.hp_max,
        velocity_x: actor.velocity.x,
        walk_path_offset: actor.walk_path_offset,
        max_crouch_rotation: MAX_CROUCH_ROTATION,
        max_walkpath_crouch_shift: MAX_WALKPATH_CROUCH_SHIFT,
    };

    // ----- 3) Per-status attitude tick -----
    match attitude_status {
        AttitudeStatus::Stable => attitude_spring_tick_stable(&mut actor.attitude, &spring_ctx),
        AttitudeStatus::Unstable => attitude_spring_tick_unstable(&mut actor.attitude, &spring_ctx),
        AttitudeStatus::Dying => {
            let _done = attitude_spring_tick_dying(&mut actor.attitude, &spring_ctx, ctx.dt_ms);
        }
        AttitudeStatus::Dead => {}
    }

    // ----- 4) Prone state machine -----
    let prev_prone = actor.prone_state;
    actor.prone_state = tick_prone_state_machine(
        &mut actor.attitude,
        actor.prone_state,
        ctx.dt_ms,
        ctx.crouch_input_active,
        ctx.move_input_active,
    );
    if prev_prone != actor.prone_state {
        events.prone_state_changed = Some(actor.prone_state);
    }

    // ----- 5) WalkAngle slope (M14A consumers pass real cf-terrain sample;
    //          this default keeps the slope flat) -----
    tick_walk_angle(&mut actor.walk_angle, 0.0, 0.0, ctx.dt_ms as f32 / 1000.0);

    // ----- 6) Crouch lean (PARITY-29) -----
    let desired_offset = if ctx.crouch_input_active {
        MAX_WALKPATH_CROUCH_SHIFT
    } else {
        0.0
    };
    actor.walk_path_offset.y = lerp(actor.walk_path_offset.y, -desired_offset, 0.3);

    // ----- 7) Arm sway -----
    let stride_phase = if matches!(actor.move_state, MoveState::Walk) {
        ((ctx.tick as f32 * ctx.dt_ms as f32) / 800.0).fract()
    } else {
        0.0
    };
    let holds_device = actor.inventory.rifle_slot().is_some();
    let two_hand = holds_device; // M14A baseline: any rifle = 2-handed.
    let arm_ctx = ArmSwayContext {
        body_rot: actor.attitude.rot,
        aim_angle: spring_ctx.aim_angle,
        sharp_aim_factor: actor.sharp_aim_progress.clamp(0.0, 1.0),
        two_hand_weapon: two_hand,
        holds_device,
        status_stable: matches!(actor.status, Status::Stable | Status::Unstable),
        fg_flail_scalar: FG_ARM_FLAIL_SCALAR,
        bg_flail_scalar: BG_ARM_FLAIL_SCALAR,
        stride_progress: stride_phase,
    };
    tick_arm_sway(&mut actor.arm_sway, &arm_ctx);

    // ----- 8) Stride alternation (PARITY-01 / PARITY-21) -----
    actor.stride_frame = false; // consumed end-of-tick
    if matches!(actor.move_state, MoveState::Walk) && on_ground {
        actor.stride_timer_ms = actor.stride_timer_ms.saturating_add(ctx.dt_ms);
        // Stride period ~ 350 ms scaled by mass_factor and overlay.
        let mass_factor = mass_aggregator::mass_factor(actor);
        let overlay = compute_overlay(ctx.atmosphere);
        let speed_scale = mass_factor * overlay.atmosphere_speed_mult;
        let stride_period_ms = (350.0 / speed_scale.max(0.1)).clamp(120.0, 1500.0) as u32;
        if actor.stride_timer_ms >= stride_period_ms {
            actor.stride_timer_ms = 0;
            let side_fg = !actor.last_stride_side_fg;
            let side_code = if side_fg { 0 } else { 1 };
            actor.emit_stride(ctx.tick, side_fg);
            events.stride_fired = true;
            events.stride_side = side_code;
            // Advance the corresponding limb path.
            let path_side = if side_fg { PathSide::Fg } else { PathSide::Bg };
            if let Some(path) = actor.limb_paths.get_mut(MoveState::Walk, path_side) {
                path.report_progress(1.0, ctx.dt_ms);
            }
        }
    } else if matches!(actor.move_state, MoveState::Stand) {
        actor.stride_timer_ms = 0;
    }

    // ----- 9) Jetpack tick -----
    if let Some(jet) = actor.jetpack.as_mut() {
        let was_emitting = jet.is_emitting;
        let total_mass = actor.total_mass_cached.max(80.0);
        let outcome = cf_equipment::jetpack_tick(
            jet,
            ctx.jet_hold,
            ctx.jet_press_edge,
            matches!(actor.status, Status::Inactive),
            total_mass,
            80.0,
            spring_ctx.aim_angle,
            spring_ctx.h_flipped,
            ctx.atmosphere.pressure_kpa,
            ctx.dt_ms,
        );
        match outcome.event {
            cf_equipment::JetpackEvent::Fired => events.jet_fired = true,
            cf_equipment::JetpackEvent::Exhausted => events.jet_exhausted = true,
            cf_equipment::JetpackEvent::Relit => events.jet_relit = true,
            _ => {}
        }
        events.jet_thrust_n = outcome.thrust_n;
        if outcome.thrust_n > 0.0 {
            // Apply thrust to velocity (F=ma → Δv = F·dt/m).
            let dv_x = outcome.thrust_vec[0] / total_mass.max(1.0) * (ctx.dt_ms as f32 / 1000.0);
            let dv_y = outcome.thrust_vec[1] / total_mass.max(1.0) * (ctx.dt_ms as f32 / 1000.0);
            actor.velocity.x += dv_x;
            actor.velocity.y += dv_y;
        }
        // Jetpack fuel burning changes mass.
        if was_emitting || outcome.is_emitting_after {
            actor.mark_mass_dirty();
        }
    }

    // ----- 10) Wind lateral force from atmosphere -----
    if ctx.atmosphere.wind != [0.0, 0.0] {
        let total_mass = actor.total_mass_cached.max(1.0);
        let wind_force = crate::atmosphere_contact::wind_force_for_actor(&ctx.atmosphere, actor.half_extents.x);
        let dt_secs = ctx.dt_ms as f32 / 1000.0;
        actor.velocity.x += wind_force[0] / total_mass * dt_secs;
        actor.velocity.y += wind_force[1] / total_mass * dt_secs;
        events.wind_force_applied = wind_force;
    }

    // ----- 11) Atmosphere overlay surfaces -----
    actor.atmosphere_sample = ctx.atmosphere;
    let overlay = compute_overlay(ctx.atmosphere);
    events.hypoxia_severity = overlay.hypoxia_severity;
    events.atmosphere_warning_severity = overlay.hypoxia_severity;
    events.combustion_ready = overlay.combustion_ready;

    // ----- 12) Mass cache recompute when dirty -----
    if actor.total_mass_dirty {
        let prev_total = actor.total_mass_cached;
        actor.total_mass_cached = mass_aggregator::total_mass(actor);
        actor.total_mass_dirty = false;
        if (prev_total - actor.total_mass_cached).abs() > 0.1 {
            events.mass_changed = true;
        }
    }

    // ----- 13) Quick-action bar tick -----
    actor.quick_action_bar.tick(ctx.dt_ms);
    actor.quick_action_bar.radial.reduce_motion = ctx.reduce_motion;

    events
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ActorId, Inventory, Vec2};

    fn make_actor() -> ActorState {
        ActorState::player(
            ActorId(1),
            "blue",
            Vec2::ZERO,
            100.0,
            Inventory::with_rifle("rifle_m1_default"),
        )
    }

    fn ctx(tick: u64) -> WalkSimContext {
        WalkSimContext {
            tick,
            dt_ms: 16,
            move_input_active: false,
            move_x: 0.0,
            crouch_input_active: false,
            jet_hold: false,
            jet_press_edge: false,
            atmosphere: AtmosphereSample::default(),
            reduce_motion: false,
        }
    }

    #[test]
    fn walking_actor_emits_strides() {
        let mut a = make_actor();
        a.on_ground = true;
        a.velocity = Vec2::new(5.0, 0.0);
        let mut strides = 0;
        for i in 0..120 {
            let mut c = ctx(i);
            c.move_input_active = true;
            c.move_x = 1.0;
            let ev = walk_sim_tick(&mut a, c);
            if ev.stride_fired {
                strides += 1;
            }
        }
        assert!(strides >= 4, "expected several strides, got {}", strides);
    }

    #[test]
    fn strides_alternate_sides() {
        let mut a = make_actor();
        a.on_ground = true;
        a.velocity = Vec2::new(5.0, 0.0);
        let mut last_side = 2_u8;
        let mut alternations = 0;
        for i in 0..240 {
            let mut c = ctx(i);
            c.move_input_active = true;
            c.move_x = 1.0;
            let ev = walk_sim_tick(&mut a, c);
            if ev.stride_fired {
                if last_side != 2 && ev.stride_side != last_side {
                    alternations += 1;
                }
                last_side = ev.stride_side;
            }
        }
        assert!(alternations >= 3, "feet should alternate; got {}", alternations);
    }

    #[test]
    fn jet_thrust_applies_when_holding() {
        let mut a = make_actor();
        a.equip_jetpack(cf_equipment::Jetpack::standard_powered_armor());
        a.on_ground = false;
        let before_vy = a.velocity.y;
        for i in 0..10 {
            let mut c = ctx(i);
            c.jet_hold = true;
            c.jet_press_edge = i == 0;
            walk_sim_tick(&mut a, c);
        }
        assert!(a.velocity.y > before_vy, "vy should increase from thrust");
    }

    #[test]
    fn move_state_transitions_to_walk_when_velocity_above_threshold() {
        let mut a = make_actor();
        a.on_ground = true;
        a.velocity = Vec2::new(5.0, 0.0);
        let mut c = ctx(0);
        c.move_input_active = true;
        let ev = walk_sim_tick(&mut a, c);
        assert_eq!(a.move_state, MoveState::Walk);
        assert_eq!(ev.move_state_changed, Some(MoveState::Walk));
    }

    #[test]
    fn crouch_input_transitions_to_crouch() {
        let mut a = make_actor();
        a.on_ground = true;
        let mut c = ctx(0);
        c.crouch_input_active = true;
        walk_sim_tick(&mut a, c);
        assert_eq!(a.move_state, MoveState::Crouch);
    }

    #[test]
    fn hypoxic_atmosphere_reports_warning() {
        let mut a = make_actor();
        a.on_ground = true;
        let mut c = ctx(0);
        c.atmosphere.o2_partial_kpa = 10.0;
        let ev = walk_sim_tick(&mut a, c);
        assert_eq!(ev.hypoxia_severity, 2);
    }

    #[test]
    fn mass_changed_event_fires_after_drop() {
        let mut a = make_actor();
        a.on_ground = true;
        a.equip_jetpack(cf_equipment::Jetpack::standard_powered_armor());
        // Initial tick caches the mass.
        walk_sim_tick(&mut a, ctx(0));
        a.drop_jetpack();
        let ev = walk_sim_tick(&mut a, ctx(1));
        assert!(ev.mass_changed);
    }
}
