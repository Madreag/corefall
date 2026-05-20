//! **M14J** § "cf-actor::sim MODIFY — Stance transition table extended;
//! per-stance limb path swap".
//!
//! Per-tick advanced-mobility integration pass. Called by the engine right
//! after `walk_sim_tick` so every M14A actor gets:
//!  - Auto-vault detect + trigger (chest-high obstacles ≤1.2 m).
//!  - Wall-detect grace window (wall-jump available within 250 ms of contact).
//!  - Stance auto-reverts: `Vault` → `Walking`, `WallJump` → `Airborne`.
//!  - Wall-jump chain reset on ground contact.
//!  - Per-stance limb path swap for `Climbing` / `SwimSurface` / `SwimSubmerged`.
//!  - Swim breath countdown when submerged.
//!  - Race-aware swim-stamina drain consumption.
//!  - Per-stride `swim.stroke` event metadata when in a swim stance.
//!  - Helmet seal + dive-suit drowning suppression (spec dependency M19C).
//!
//! Pure / deterministic: every helper takes `&mut ActorState + dt_ms + tick`
//! and never reads a clock or `thread_rng`. The caller (engine) provides the
//! cached `parkour_signal.vault_candidate` / `wall_candidate` populated from
//! the M14 swept-volume query.

use serde::{Deserialize, Serialize};

use crate::{
    limb_path::{LimbPath, PathSide},
    mount::mounted_aim_spread,
    move_state::{MoveState, SwimKind},
    parkour::{apply_vault, VaultCandidate, VAULT_DURATION_MS},
    ActorState,
};

/// **M14J** § per-tick stride period (ms) for the swim limb path. Spec § "stroke
/// rate consumes M16 swim-stamina" — 4-stroke cycle ≈ 800 ms baseline.
pub const SWIM_STROKE_PERIOD_MS: u32 = 800;

/// **M14J** § per-tick swim breath drain (seconds of breath per simulation
/// second) for a submerged actor. Spec § "swim-stamina drains at race-aware
/// rate (Human 1.0×, Aqueous 0.5×, Robotic = sinks)".
pub const SWIM_BREATH_DRAIN_SECONDS_PER_SEC: f32 = 1.0;

/// **M14J** § ladder/cable/vine vertical climb speed (m/s). Spec § "the
/// actor advances at 1 m/s vertical".
pub const CLIMB_VERTICAL_SPEED_M_PER_S: f32 = 1.0;

/// **M14J** § maximum breath reservoir (seconds) — baseline for a Human
/// origin. Race-aware scaling lives on `ActorState::swim_drain_multiplier`
/// (M17 origin reaction table); when the actor is at surface this is the
/// cap that breath recovers toward. Spec § "breath_held_s reaching 0 →
/// drowning" + "drains at race-aware rate". Audit finding #5a/#8 fix.
pub const SWIM_MAX_BREATH_SECONDS: f32 = 30.0;

/// **M14J § swim stamina per stroke** — fraction of stamina drained per
/// stroke cycle, scaled by `swim_drain_multiplier`. Spec § "stroke rate
/// consumes M16 swim-stamina". Audit finding #5a/#8 fix.
pub const SWIM_STAMINA_PER_STROKE: f32 = 0.05;

/// **M14J § climb path tick scaling** — fraction of a stride cycle advanced
/// per ms on the Climb limb path while `climb_active`. Spec § "climbs a
/// ladder rung-by-rung". Audit finding #5a/#8 fix.
pub const CLIMB_PATH_PROGRESS_DIVISOR_MS: f32 = 350.0;

/// **M14J** § "stamina_remaining" payload value: the `actor.stamina.current`
/// surface scaled to a unit interval. Surfaced via [`M14jTickEvents`] for the
/// `swim.stroke` event payload.
fn stamina_unit(actor: &ActorState) -> f32 {
    let cur = actor.stamina.current;
    let max = actor.stamina.max.max(1.0);
    (cur / max).clamp(0.0, 1.0)
}

/// **M14J** § per-tick events emitted by the M14J integration pass. The
/// engine consumes these and turns them into recorder envelopes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct M14jTickEvents {
    /// `Some(obstacle_height_m)` on the tick auto-vault committed. Engine
    /// emits `actor.vaulted` with the obstacle_height payload.
    pub vault_triggered_height_m: Option<f32>,
    /// `true` on the tick the vault cinematic finished + actor advanced
    /// over the obstacle. Position is already updated by [`tick_m14j_actor`].
    pub vault_completed: bool,
    /// `true` on the tick the wall-jump cinematic returned to airborne.
    pub wall_jump_completed: bool,
    /// `Some(swim_kind)` on the tick a swim stroke cycle just completed.
    /// Engine emits `swim.stroke` with the kind + stamina_remaining +
    /// drain_multiplier payload.
    pub swim_stroke_emitted: Option<SwimKind>,
    /// `Some(depth_m)` on the tick the actor drowned (breath reached 0
    /// while submerged + no helmet/dive-suit seal). Engine emits
    /// `actor.drowned` + appends `AfflictionKind::Drowning`.
    pub actor_drowned_depth_m: Option<f32>,
    /// True when the actor's wall-jump chain reset this tick (e.g. landed
    /// on ground).
    pub wall_jump_chain_reset: bool,
    /// **M14J** § `swim_drain_multiplier` snapshot for the `swim.stroke`
    /// payload (so consumers see the live multiplier per-stroke).
    pub swim_drain_multiplier_snapshot: f32,
    /// **M14J** § `stamina_remaining` snapshot (unit interval) for the
    /// `swim.stroke` payload.
    pub stamina_remaining_snapshot: f32,
}

/// **M14J § "actor.swim_kind = Surface → SwimSurface stance"**. Routes the
/// active limb path based on the M14J state. Returns the canonical
/// swim limb path keyed by `SwimKind`, falling back to the M14A Walk path
/// for legacy registries that haven't registered the M14J swim paths yet.
fn active_swim_path(actor: &mut ActorState) -> Option<&mut LimbPath> {
    if actor.swim_kind == SwimKind::None {
        return None;
    }
    // Prefer the M14J swim registry slot.
    if actor.limb_paths.get_swim(actor.swim_kind).is_some() {
        return actor.limb_paths.get_swim_mut(actor.swim_kind);
    }
    let fallback_state = match actor.swim_kind {
        SwimKind::SurfaceBreast | SwimKind::SurfaceFreestyle => MoveState::Walk,
        SwimKind::Dive => MoveState::Jump,
        SwimKind::Tread => MoveState::Stand,
        SwimKind::None => return None,
    };
    actor.limb_paths.get_mut(fallback_state, PathSide::Fg)
}

/// **M14J** § "Auto-vault clears chest-high crate at full run" + spec § "Per-
/// stance limb path swap" + spec § "Swim refinement supersedes M16
/// placeholder".
///
/// Advances every M14J per-actor surface one tick:
///  - Decrements parkour timers + handles vault/wall-jump auto-reverts.
///  - On ground contact: resets wall-jump chain.
///  - When walking + grounded + a `vault_candidate` is cached AND no cinematic
///    is active: auto-triggers Vault (sets the 200 ms timer + emits the
///    obstacle_height for the engine to record).
///  - When a vault cinematic finishes (timer just hit 0): advances the actor
///    position over the obstacle while preserving horizontal velocity.
///  - When climbing (Stance::Climbing intent active): clamps vertical
///    velocity to ±CLIMB_VERTICAL_SPEED_M_PER_S and zeroes horizontal drift
///    so the actor advances rung-by-rung.
///  - Advances the active swim limb path when `swim_kind != None`; on
///    cycle completion records the stroke kind in events so the engine
///    can fire `swim.stroke`.
///  - When submerged + no helmet/dive-suit seal: drains breath; when breath
///    hits 0 + still submerged → records the drown depth so the engine
///    fires `actor.drowned` + appends the `Drowning` affliction.
///
/// `dt_ms` is the per-tick wall-clock dt; `tick` is the monotonic engine tick.
pub fn tick_m14j_actor(actor: &mut ActorState, dt_ms: u32, tick: u64) -> M14jTickEvents {
    let mut events = M14jTickEvents::default();
    let prev_vault_ms = actor.parkour_signal.vault_ticks_remaining_ms;
    let prev_wall_jump_ms = actor.wall_jump_ticks_remaining_ms;

    // ----- 1) Tick parkour timers + wall-jump cinematic shadow timer -----
    actor.parkour_signal.tick(dt_ms);
    actor.wall_jump_ticks_remaining_ms = actor.wall_jump_ticks_remaining_ms.saturating_sub(dt_ms);

    // ----- 2) Ground contact resets wall-jump chain + clears grace -----
    if actor.on_ground {
        let had_chain = actor.parkour_signal.chained_wall_jumps_since_ground > 0
            || actor.parkour_signal.wall_contact_grace_remaining_ms > 0;
        actor.parkour_signal.note_ground_contact();
        if had_chain {
            events.wall_jump_chain_reset = true;
        }
    }

    // ----- 3) Vault completion: apply position move + preserve velocity -----
    // MUST run BEFORE auto-trigger so a finished vault doesn't immediately
    // re-trigger off the still-cached candidate.
    if prev_vault_ms > 0 && actor.parkour_signal.vault_ticks_remaining_ms == 0 {
        if let Some(cand) = actor.parkour_signal.vault_candidate.take() {
            let facing_sign = actor.facing.sign();
            let new_pos = apply_vault([actor.position.x, actor.position.y], &cand, facing_sign);
            actor.position.x = new_pos[0];
            actor.position.y = new_pos[1];
            // Horizontal velocity is preserved (not modified by apply_vault).
        }
        events.vault_completed = true;
    }

    // ----- 4) Wall-jump completion: returns to Airborne -----
    if prev_wall_jump_ms > 0 && actor.wall_jump_ticks_remaining_ms == 0 {
        events.wall_jump_completed = true;
        actor.parkour_signal.wall_jump_ticks_remaining_ms = 0;
    }

    // ----- 5) Auto-vault commit when cached candidate + walking + grounded -----
    let walking = matches!(actor.move_state, MoveState::Walk);
    let in_cinematic = actor.parkour_signal.vault_ticks_remaining_ms > 0
        || actor.wall_jump_ticks_remaining_ms > 0;
    if walking && actor.on_ground && !in_cinematic && !events.vault_completed {
        if let Some(cand) = actor.parkour_signal.vault_candidate {
            actor.parkour_signal.vault_ticks_remaining_ms = VAULT_DURATION_MS;
            events.vault_triggered_height_m = Some(cand.height_m);
        }
    }

    // ----- 6) Climbing intent: clamp vertical speed to 1 m/s, no jet drain -----
    if actor.climb_active {
        let vy = actor.velocity.y;
        let clamped = vy.clamp(-CLIMB_VERTICAL_SPEED_M_PER_S, CLIMB_VERTICAL_SPEED_M_PER_S);
        actor.velocity.y = clamped;
        // Horizontal drift damped to zero on a ladder rung.
        actor.velocity.x *= 0.5;
        // Climbing path advance — one tick worth of progress.
        if let Some(path) = actor.limb_paths.get_mut(MoveState::Climb, PathSide::Fg) {
            path.report_progress(dt_ms as f32 / CLIMB_PATH_PROGRESS_DIVISOR_MS, dt_ms);
        }
    }

    // ----- 7) Swim limb path advance + stroke event emission -----
    if actor.swim_kind != SwimKind::None {
        actor.stride_timer_ms = actor.stride_timer_ms.saturating_add(dt_ms);
        if actor.stride_timer_ms >= SWIM_STROKE_PERIOD_MS {
            actor.stride_timer_ms = 0;
            // Advance the swim limb path one cycle.
            if let Some(path) = active_swim_path(actor) {
                path.report_progress(1.0, dt_ms);
            }
            events.swim_stroke_emitted = Some(actor.swim_kind);
            events.swim_drain_multiplier_snapshot = actor.swim_drain_multiplier;
            events.stamina_remaining_snapshot = stamina_unit(actor);
            // Drain swim stamina at race-aware rate.
            let drain = SWIM_STAMINA_PER_STROKE * actor.swim_drain_multiplier.max(0.0);
            actor.stamina.consume(drain);
            actor.last_stride_tick = tick;
        }
    }

    // ----- 8) Submerged breath drain + drowning trigger -----
    let dt_seconds = dt_ms as f32 / 1000.0;
    if actor.swim_kind.is_submerged() {
        // Helmet seal + dive-suit shell suppresses drowning per spec
        // § "helmet seal + M19C dive-suit shell removes the `drowning`
        // affliction the swim-stamina rule would otherwise produce".
        let helmet_seal = actor_has_helmet_seal(actor);
        let dive_suit = actor_has_dive_suit(actor);
        let drowning_suppressed = helmet_seal || dive_suit;
        if !drowning_suppressed && !actor.swim_disabled_sinks {
            actor.swim_breath_seconds =
                (actor.swim_breath_seconds - dt_seconds * SWIM_BREATH_DRAIN_SECONDS_PER_SEC).max(0.0);
            if actor.swim_breath_seconds <= 0.0 {
                // Depth is the y position relative to a presumed surface at y=0.
                let depth_m = (-actor.position.y).max(0.0);
                events.actor_drowned_depth_m = Some(depth_m);
            }
        } else if drowning_suppressed {
            // Helmet seal: breath does NOT drain underwater.
            // (Dive suit recharges as long as it has internal O2 supply; that
            // accounting happens in M19C's o2 module.)
        }
    } else if matches!(actor.swim_kind, SwimKind::SurfaceBreast | SwimKind::SurfaceFreestyle) {
        // At surface: recover breath at the same rate as it drains submerged,
        // capped at SWIM_MAX_BREATH_SECONDS. Caller (engine) is responsible
        // for ensuring `swim_kind == SurfaceBreast|SurfaceFreestyle` only
        // when the actor is actually over water — separation of concerns
        // keeps the actor tick pure (no terrain lookups).
        actor.swim_breath_seconds =
            (actor.swim_breath_seconds + dt_seconds * SWIM_BREATH_DRAIN_SECONDS_PER_SEC).min(SWIM_MAX_BREATH_SECONDS);
    }

    events
}

/// **M14J** § "helmet seal" — true when the actor wears a sealed helmet
/// (M19C PPE / body armor slot helmet). Defensive default: returns true
/// only when the body_armor slot has an active helmet seal.
#[must_use]
pub fn actor_has_helmet_seal(actor: &ActorState) -> bool {
    actor.body_armor.helmet_seal_active()
}

/// **M14J** § "M19C dive-suit shell" — true when the actor wears a dive
/// suit on the body armor slot.
#[must_use]
pub fn actor_has_dive_suit(actor: &ActorState) -> bool {
    actor.body_armor.dive_suit_equipped()
}

/// **M14J** § "Mounted rider fires one-handed weapon at gallop" — extra
/// firing aim spread when the actor is mounted on a moving critter. Returns
/// the bonus penalty in radians. Pure helper; engine sums with the base
/// `M14A` aim spread.
#[must_use]
pub fn mount_motion_aim_penalty(actor: &ActorState, critter_speed: f32) -> f32 {
    if actor.mount.is_some() {
        mounted_aim_spread(0.0, critter_speed) - 0.0
    } else {
        0.0
    }
}

/// **M14J** § "VaultCandidate from a chunked-terrain swept query". Convenience
/// helper for engine + test code: populates the actor's
/// `parkour_signal.vault_candidate` field when a candidate is detected,
/// clears it when the predicate returns no candidate.
pub fn populate_vault_candidate(
    actor: &mut ActorState,
    is_solid: impl Fn(f32, f32) -> bool,
) -> Option<VaultCandidate> {
    let candidate = crate::parkour::detect_vault(
        [actor.position.x, actor.position.y],
        [actor.half_extents.x, actor.half_extents.y],
        actor.facing,
        actor.velocity.x,
        is_solid,
    );
    actor.parkour_signal.vault_candidate = candidate;
    candidate
}

/// **M14J** § "Wall-detect within 250 ms of contact" — populate the actor's
/// `parkour_signal.wall_candidate` + reset/refresh the grace window. Returns
/// the live candidate so the caller can mirror it to the dispatch layer.
pub fn populate_wall_candidate(
    actor: &mut ActorState,
    is_solid: impl Fn(f32, f32) -> bool,
) -> Option<crate::parkour::WallCandidate> {
    let cand = crate::parkour::detect_wall(
        [actor.position.x, actor.position.y],
        [actor.half_extents.x, actor.half_extents.y],
        is_solid,
    );
    if cand.is_some() {
        actor.parkour_signal.wall_candidate = cand;
        actor.parkour_signal.wall_contact_grace_remaining_ms = crate::parkour::WALL_CONTACT_GRACE_MS;
    } else if actor.on_ground {
        actor.parkour_signal.wall_candidate = None;
    }
    cand
}

/// **M14J § "wall-jump-detect helper" — combined per-tick populate + tick
/// helper. Pure / deterministic. Returns the resolved `M14jTickEvents`.
pub fn tick_m14j_full(
    actor: &mut ActorState,
    dt_ms: u32,
    tick: u64,
    is_solid: Option<&dyn Fn(f32, f32) -> bool>,
) -> M14jTickEvents {
    if let Some(predicate) = is_solid {
        if actor.on_ground && matches!(actor.move_state, MoveState::Walk) {
            populate_vault_candidate(actor, predicate);
        }
        if !actor.on_ground {
            populate_wall_candidate(actor, predicate);
        }
    }
    tick_m14j_actor(actor, dt_ms, tick)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ActorId, FacingDirection, Inventory, Vec2};

    fn make_walker() -> ActorState {
        let mut a = ActorState::player(
            ActorId(1),
            "blue",
            Vec2::ZERO,
            100.0,
            Inventory::with_rifle("rifle_m1_default"),
        );
        a.on_ground = true;
        a.move_state = MoveState::Walk;
        a.facing = FacingDirection::Right;
        a.velocity = Vec2::new(5.0, 0.0);
        a
    }

    #[test]
    fn auto_vault_triggers_when_candidate_cached() {
        let mut a = make_walker();
        a.parkour_signal.vault_candidate = Some(crate::parkour::VaultCandidate {
            near_x: a.position.x + a.half_extents.x + 0.5,
            top_y: a.position.y + 1.0,
            height_m: 1.0,
        });
        let ev = tick_m14j_actor(&mut a, 16, 1);
        assert_eq!(ev.vault_triggered_height_m, Some(1.0));
        assert_eq!(a.parkour_signal.vault_ticks_remaining_ms, VAULT_DURATION_MS);
    }

    #[test]
    fn vault_completion_translates_actor() {
        let mut a = make_walker();
        let initial_x = a.position.x;
        a.parkour_signal.vault_candidate = Some(crate::parkour::VaultCandidate {
            near_x: a.position.x + a.half_extents.x + 0.5,
            top_y: a.position.y + 0.8,
            height_m: 0.8,
        });
        let initial_vx = a.velocity.x;
        let _ev = tick_m14j_actor(&mut a, 16, 1);
        // Drive ticks until vault timer reaches 0 (≥200ms).
        for tick in 2..40u64 {
            tick_m14j_actor(&mut a, 16, tick);
        }
        assert!(a.position.x > initial_x, "actor must advance over obstacle");
        assert!((a.velocity.x - initial_vx).abs() < 0.01, "horizontal velocity preserved");
    }

    #[test]
    fn wall_jump_chain_resets_on_ground() {
        let mut a = make_walker();
        a.parkour_signal.chained_wall_jumps_since_ground = 2;
        a.parkour_signal.wall_contact_grace_remaining_ms = 100;
        a.on_ground = true;
        let ev = tick_m14j_actor(&mut a, 16, 1);
        assert!(ev.wall_jump_chain_reset);
        assert_eq!(a.parkour_signal.chained_wall_jumps_since_ground, 0);
    }

    #[test]
    fn swim_breath_drains_when_submerged() {
        let mut a = make_walker();
        a.swim_kind = SwimKind::Dive;
        a.swim_breath_seconds = 1.0;
        for tick in 0..120u64 {
            tick_m14j_actor(&mut a, 16, tick);
        }
        assert!(a.swim_breath_seconds < 0.1, "breath should drain over time, got {}", a.swim_breath_seconds);
    }

    #[test]
    fn swim_stroke_emitted_per_cycle() {
        let mut a = make_walker();
        a.swim_kind = SwimKind::SurfaceBreast;
        a.stride_timer_ms = 0;
        let mut strokes = 0u32;
        for tick in 0..200u64 {
            let ev = tick_m14j_actor(&mut a, 16, tick);
            if ev.swim_stroke_emitted.is_some() {
                strokes += 1;
            }
        }
        assert!(strokes >= 3, "expected several swim strokes; got {}", strokes);
    }

    #[test]
    fn submerged_drown_event_fires_when_breath_zero() {
        let mut a = make_walker();
        a.swim_kind = SwimKind::Dive;
        a.swim_breath_seconds = 0.0;
        a.position.y = -3.0;
        let ev = tick_m14j_actor(&mut a, 16, 1);
        assert_eq!(ev.actor_drowned_depth_m, Some(3.0));
    }

    #[test]
    fn climb_clamps_vertical_speed() {
        let mut a = make_walker();
        a.climb_active = true;
        a.velocity.y = 50.0;
        tick_m14j_actor(&mut a, 16, 1);
        assert!(a.velocity.y <= CLIMB_VERTICAL_SPEED_M_PER_S);
        assert!(a.velocity.y >= -CLIMB_VERTICAL_SPEED_M_PER_S);
    }

    #[test]
    fn auto_vault_skipped_when_in_cinematic() {
        let mut a = make_walker();
        a.parkour_signal.vault_ticks_remaining_ms = VAULT_DURATION_MS / 2;
        a.parkour_signal.vault_candidate = Some(crate::parkour::VaultCandidate {
            near_x: 0.0,
            top_y: 0.0,
            height_m: 0.5,
        });
        let ev = tick_m14j_actor(&mut a, 16, 1);
        assert!(ev.vault_triggered_height_m.is_none());
    }
}
