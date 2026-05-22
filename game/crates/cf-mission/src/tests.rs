//! Integration tests for `MissionState` + `step()` + `Reactor` cascade.
//! Split out of `lib.rs` for the 2k-LOC ceiling.

use std::collections::BTreeMap;

use cf_actor::{ActorId, ActorState, Inventory, Status, Vec2};

use crate::loss::{LossConditions, LossReason};
use crate::objective_types::{Objective, ObjectiveKind, ObjectiveStatus};
use crate::reactor;
use crate::reactor_world::{Reactor, ReactorWorld};
use crate::result::MissionResult;
use crate::state::MissionState;
use crate::step_engine::step;
use crate::tick::MissionTickInputs;
use crate::view::MissionView;

fn build_state() -> MissionState {
    let objectives = vec![
        Objective {
            id: "breach".to_string(),
            kind: ObjectiveKind::BreachBarrier {
                target: "outer_wall".to_string(),
            },
            optional: false,
            status: ObjectiveStatus::Pending,
            progress_milestone_index: 0,
            progress: 0.0,
            fail_sensor: None,
        },
        Objective {
            id: "neutralize".to_string(),
            kind: ObjectiveKind::NeutralizeActor { target: 2 },
            optional: false,
            status: ObjectiveStatus::Pending,
            progress_milestone_index: 0,
            progress: 0.0,
            fail_sensor: None,
        },
        Objective {
            id: "extract".to_string(),
            kind: ObjectiveKind::ReachZone {
                min: [1180.0, 16.0],
                max: [1280.0, 64.0],
            },
            optional: false,
            status: ObjectiveStatus::Pending,
            progress_milestone_index: 0,
            progress: 0.0,
            fail_sensor: None,
        },
    ];
    MissionState::new(
        objectives,
        0,
        LossConditions {
            player_dead: true,
            time_limit_ticks: 60 * 90,
        },
    )
}

fn player_at(x: f32, y: f32) -> ActorState {
    ActorState::player(ActorId(1), "blue", Vec2::new(x, y), 100.0, Inventory::default())
}

fn mk_actors(player: ActorState, enemy_dead: bool) -> BTreeMap<ActorId, ActorState> {
    let mut m = BTreeMap::new();
    m.insert(player.id, player);
    let mut enemy = ActorState::player(ActorId(2), "red", Vec2::new(900.0, 32.0), 80.0, Inventory::default());
    if enemy_dead {
        enemy.hp = 0.0;
        enemy.status = Status::Dead;
    }
    m.insert(enemy.id, enemy);
    m
}

#[test]
fn first_objective_starts_pending_then_activates_on_first_step() {
    // BP2 fix: first objective is Pending after construction; the first
    // step() activates it AND emits objective_started so the engine emits
    // a `mission.objective_started` event for objective 0.
    let mut state = build_state();
    assert_eq!(state.objectives[0].status, ObjectiveStatus::Pending);
    assert_eq!(state.objectives[1].status, ObjectiveStatus::Pending);
    let actors = mk_actors(player_at(120.0, 32.0), false);
    let report = step(
        &mut state,
        MissionTickInputs {
            tick: 1,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &BTreeMap::new(),
            reactors_destroyed: &BTreeMap::new(),
            breaches_progress: &BTreeMap::new(),
        },
    );
    assert_eq!(report.objective_started, vec!["breach".to_string()]);
    assert_eq!(state.objectives[0].status, ObjectiveStatus::Active);
    assert_eq!(state.objectives[1].status, ObjectiveStatus::Pending);
}

#[test]
fn pause_suspends_step_and_timer_resume_restores() {
    let mut state = build_state();
    let actors = mk_actors(player_at(120.0, 32.0), false);
    // Tick 1: activate breach.
    let _ = step(
        &mut state,
        MissionTickInputs {
            tick: 1,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &BTreeMap::new(),
            reactors_destroyed: &BTreeMap::new(),
            breaches_progress: &BTreeMap::new(),
        },
    );
    // Tick 5: pause. elapsed_ticks at tick 5 = 5.
    let active = state.pause(5).expect("pause returns active id");
    assert_eq!(active, "breach");
    assert!(state.paused);
    assert_eq!(state.elapsed_ticks(5), 5);
    // Tick 50: still paused; elapsed should not advance past tick 5's value.
    let mut breaches = BTreeMap::new();
    breaches.insert("outer_wall".to_string(), true);
    let report = step(
        &mut state,
        MissionTickInputs {
            tick: 50,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &breaches,
            reactors_destroyed: &BTreeMap::new(),
            breaches_progress: &BTreeMap::new(),
        },
    );
    assert!(report.objective_completed.is_empty(), "paused step is a no-op");
    assert!(state.objectives[0].status == ObjectiveStatus::Active);
    // ticks_remaining at tick 50 while paused = original time_limit - 5.
    let remaining_paused = state.ticks_remaining(50).unwrap();
    assert_eq!(remaining_paused, state.time_limit_ticks - 5);
    // Resume at tick 50: paused for 45 ticks; subsequent timer reads reflect that credit.
    let resumed = state.resume(50).expect("resume returns active id");
    assert_eq!(resumed, "breach");
    assert!(!state.paused);
    assert_eq!(state.total_paused_ticks, 45);
    // Tick 100: elapsed = 100 - 0 - 45 = 55.
    assert_eq!(state.elapsed_ticks(100), 55);
}

#[test]
fn pause_resume_skip_when_terminal_or_double_called() {
    let mut state = build_state();
    let actors = mk_actors(player_at(120.0, 32.0), false);
    // Pause once.
    let _ = step(
        &mut state,
        MissionTickInputs {
            tick: 1,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &BTreeMap::new(),
            reactors_destroyed: &BTreeMap::new(),
            breaches_progress: &BTreeMap::new(),
        },
    );
    assert!(state.pause(2).is_some());
    // Second pause: None (already paused).
    assert!(state.pause(3).is_none());
    // Resume.
    assert!(state.resume(4).is_some());
    // Second resume: None (not paused).
    assert!(state.resume(5).is_none());
    // Terminal: pause refuses.
    state.result = MissionResult::Won;
    assert!(state.pause(6).is_none());
}

#[test]
fn breach_progress_milestones_emit_objective_updated() {
    // carve milestones for the active `BreachBarrier` objective.
    let mut state = build_state();
    let actors = mk_actors(player_at(120.0, 32.0), false);
    // Tick 1: activate the breach objective.
    let _ = step(
        &mut state,
        MissionTickInputs {
            tick: 1,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &BTreeMap::new(),
            reactors_destroyed: &BTreeMap::new(),
            breaches_progress: &BTreeMap::new(),
        },
    );
    // Tick 2: 30% progress -> 25% milestone fires once.
    let mut progress = BTreeMap::new();
    progress.insert("outer_wall".to_string(), 0.30_f32);
    let r2 = step(
        &mut state,
        MissionTickInputs {
            tick: 2,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &BTreeMap::new(),
            reactors_destroyed: &BTreeMap::new(),
            breaches_progress: &progress,
        },
    );
    assert_eq!(r2.objective_updated.len(), 1);
    assert_eq!(r2.objective_updated[0].objective_id, "breach");
    assert!((r2.objective_updated[0].progress - 0.25).abs() < 1e-3);
    // Tick 3: 60% progress -> 50% milestone fires (75% not yet).
    progress.insert("outer_wall".to_string(), 0.60_f32);
    let r3 = step(
        &mut state,
        MissionTickInputs {
            tick: 3,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &BTreeMap::new(),
            reactors_destroyed: &BTreeMap::new(),
            breaches_progress: &progress,
        },
    );
    assert_eq!(r3.objective_updated.len(), 1);
    assert!((r3.objective_updated[0].progress - 0.5).abs() < 1e-3);
    // Tick 4: 99% progress crosses 75% only.
    progress.insert("outer_wall".to_string(), 0.99_f32);
    let r4 = step(
        &mut state,
        MissionTickInputs {
            tick: 4,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &BTreeMap::new(),
            reactors_destroyed: &BTreeMap::new(),
            breaches_progress: &progress,
        },
    );
    assert_eq!(r4.objective_updated.len(), 1);
    assert!((r4.objective_updated[0].progress - 0.75).abs() < 1e-3);
    // Tick 5: 100% progress + broken=true -> 100% milestone fires AND
    // the objective completes on the same tick. The objective_updated
    // entry precedes the objective_completed one so the cause chain
    // reads dig -> objective_updated{1.0} -> objective_completed.
    let mut broken = BTreeMap::new();
    broken.insert("outer_wall".to_string(), true);
    progress.insert("outer_wall".to_string(), 1.0_f32);
    let r5 = step(
        &mut state,
        MissionTickInputs {
            tick: 5,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &broken,
            reactors_destroyed: &BTreeMap::new(),
            breaches_progress: &progress,
        },
    );
    assert_eq!(r5.objective_updated.len(), 1);
    assert!((r5.objective_updated[0].progress - 1.0).abs() < 1e-3);
    assert_eq!(r5.objective_completed, vec!["breach".to_string()]);
}

#[test]
fn breach_completion_advances_to_neutralize() {
    let mut state = build_state();
    let actors = mk_actors(player_at(120.0, 32.0), false);
    // BP2 fix: now that the first objective starts Pending, drive one
    // empty step() first so it activates + emits objective_started.
    // Then the breach-broken tick completes it + activates "neutralize".
    let _activation = step(
        &mut state,
        MissionTickInputs {
            tick: 1,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &BTreeMap::new(),
            reactors_destroyed: &BTreeMap::new(),
            breaches_progress: &BTreeMap::new(),
        },
    );
    assert_eq!(state.objectives[0].status, ObjectiveStatus::Active);
    let mut breaches = BTreeMap::new();
    breaches.insert("outer_wall".to_string(), true);
    let report = step(
        &mut state,
        MissionTickInputs {
            tick: 60,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &breaches,
            reactors_destroyed: &BTreeMap::new(),
            breaches_progress: &BTreeMap::new(),
        },
    );
    assert_eq!(report.objective_completed, vec!["breach".to_string()]);
    assert_eq!(report.objective_started, vec!["neutralize".to_string()]);
    assert_eq!(state.objectives[0].status, ObjectiveStatus::Completed);
    assert_eq!(state.objectives[1].status, ObjectiveStatus::Active);
}

#[test]
fn full_clear_wins_mission() {
    let mut state = build_state();
    let mut breaches = BTreeMap::new();
    breaches.insert("outer_wall".to_string(), true);
    let player = player_at(1200.0, 32.0);
    let actors = mk_actors(player.clone(), true);
    // Tick 1: breach completes.
    let _ = step(
        &mut state,
        MissionTickInputs {
            tick: 1,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &breaches,
            reactors_destroyed: &BTreeMap::new(),
            breaches_progress: &BTreeMap::new(),
        },
    );
    // Tick 2: neutralize completes.
    let _ = step(
        &mut state,
        MissionTickInputs {
            tick: 2,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &breaches,
            reactors_destroyed: &BTreeMap::new(),
            breaches_progress: &BTreeMap::new(),
        },
    );
    // Tick 3: extract completes.
    let report = step(
        &mut state,
        MissionTickInputs {
            tick: 3,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &breaches,
            reactors_destroyed: &BTreeMap::new(),
            breaches_progress: &BTreeMap::new(),
        },
    );
    assert_eq!(report.objective_completed, vec!["extract".to_string()]);
    assert_eq!(report.final_result, Some(MissionResult::Won));
    assert!(matches!(state.result, MissionResult::Won));
}

#[test]
fn player_dead_loses_immediately() {
    let mut state = build_state();
    let mut player = player_at(120.0, 32.0);
    player.hp = 0.0;
    player.status = Status::Dead;
    let actors = mk_actors(player, false);
    let report = step(
        &mut state,
        MissionTickInputs {
            tick: 30,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &BTreeMap::new(),
            reactors_destroyed: &BTreeMap::new(),
            breaches_progress: &BTreeMap::new(),
        },
    );
    assert!(matches!(
        state.result,
        MissionResult::Lost {
            reason: LossReason::PlayerDead
        }
    ));
    assert!(report.final_result.is_some());
}

/// the actor flagged `is_brain == true` is dead, regardless of which
/// actor is currently being puppeted by the player pointer.
#[test]
fn brain_dead_loses_immediately_via_loss_reason_brain_destroyed() {
    let mut state = build_state();
    let player = player_at(120.0, 32.0);
    let mut brain_actor = player_at(200.0, 32.0);
    brain_actor.id = ActorId(2);
    brain_actor.is_brain = true;
    brain_actor.status = Status::Dead;
    brain_actor.hp = 0.0;
    let mut actors = mk_actors(player, false);
    actors.insert(brain_actor.id, brain_actor);
    let report = step(
        &mut state,
        MissionTickInputs {
            tick: 30,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &BTreeMap::new(),
            reactors_destroyed: &BTreeMap::new(),
            breaches_progress: &BTreeMap::new(),
        },
    );
    assert!(matches!(
        state.result,
        MissionResult::Lost {
            reason: LossReason::BrainDestroyed
        }
    ));
    assert!(report.final_result.is_some());
    assert_eq!(state.last_event_label, "mission_lost_brain_destroyed");
}

#[test]
fn timer_expiry_loses_mission() {
    let mut state = build_state();
    let actors = mk_actors(player_at(120.0, 32.0), false);
    let report = step(
        &mut state,
        MissionTickInputs {
            tick: 60 * 90,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &BTreeMap::new(),
            reactors_destroyed: &BTreeMap::new(),
            breaches_progress: &BTreeMap::new(),
        },
    );
    assert!(matches!(
        state.result,
        MissionResult::Lost {
            reason: LossReason::TimerExpired
        }
    ));
    assert!(report.final_result.is_some());
}

#[test]
fn terminal_state_is_idempotent() {
    let mut state = build_state();
    state.result = MissionResult::Won;
    let actors = mk_actors(player_at(120.0, 32.0), false);
    let report = step(
        &mut state,
        MissionTickInputs {
            tick: 30,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &BTreeMap::new(),
            reactors_destroyed: &BTreeMap::new(),
            breaches_progress: &BTreeMap::new(),
        },
    );
    assert!(report.objective_completed.is_empty());
    assert!(report.final_result.is_none());
}

#[test]
fn reset_returns_to_pending_then_activates_on_first_step() {
    // BP2 fix: reset() leaves all objectives Pending; the next step()
    // activates the first one + emits objective_started.
    let mut state = build_state();
    state.objectives[0].status = ObjectiveStatus::Completed;
    state.objectives[1].status = ObjectiveStatus::Active;
    state.result = MissionResult::Won;
    state.reset(100);
    assert_eq!(state.objectives[0].status, ObjectiveStatus::Pending);
    assert_eq!(state.objectives[1].status, ObjectiveStatus::Pending);
    assert_eq!(state.started_at_tick, 100);
    assert!(matches!(state.result, MissionResult::InProgress));
    // Drive one step; first objective activates + objective_started fires.
    let actors = mk_actors(player_at(120.0, 32.0), false);
    let report = step(
        &mut state,
        MissionTickInputs {
            tick: 101,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &BTreeMap::new(),
            reactors_destroyed: &BTreeMap::new(),
            breaches_progress: &BTreeMap::new(),
        },
    );
    assert_eq!(report.objective_started, vec!["breach".to_string()]);
    assert_eq!(state.objectives[0].status, ObjectiveStatus::Active);
}

fn build_reactor_defense_state(time_limit_ticks: u64) -> MissionState {
    let objectives = vec![Objective {
        id: "defend_reactor".to_string(),
        kind: ObjectiveKind::DefendReactor {
            target: "core_reactor".to_string(),
        },
        optional: false,
        status: ObjectiveStatus::Pending,
        progress_milestone_index: 0,
        progress: 0.0,
        fail_sensor: None,
    }];
    MissionState::new(
        objectives,
        0,
        LossConditions {
            player_dead: true,
            time_limit_ticks,
        },
    )
}

#[test]
fn defend_reactor_loses_when_reactor_destroyed() {
    let mut state = build_reactor_defense_state(60 * 90);
    let actors = mk_actors(player_at(120.0, 32.0), false);
    let mut reactors = BTreeMap::new();
    reactors.insert("core_reactor".to_string(), true);
    let report = step(
        &mut state,
        MissionTickInputs {
            tick: 100,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &BTreeMap::new(),
            reactors_destroyed: &reactors,
            breaches_progress: &BTreeMap::new(),
        },
    );
    assert_eq!(
        state.result,
        MissionResult::Lost {
            reason: LossReason::ReactorDestroyed
        }
    );
    assert_eq!(report.objective_failed, vec!["defend_reactor".to_string()]);
    assert!(report.final_result.is_some());
    // Regression for Devin BUG_pr-review-job-8dddb0ae78c7456997c4d2dc7aade217_0001:
    // the failing objective's `status` field MUST flip to `Failed` so the
    // observe envelope reports it correctly. Pre-fix, the loop borrowed
    // `&state.objectives` so `obj.status` could not be mutated.
    assert_eq!(state.objectives[0].status, ObjectiveStatus::Failed);
    let view = MissionView::from_state(&state, 100);
    assert_eq!(view.objectives[0].status, "failed");
    assert_eq!(view.loss_reason.as_deref(), Some("reactor_destroyed"));
}

#[test]
fn pending_defend_reactor_loses_when_reactor_destroyed_before_objective_activates() {
    // behind an earlier objective (Pending status) MUST detect its
    // reactor being destroyed and resolve the mission as
    // `Lost { ReactorDestroyed }`. Pre-fix the destruction was
    // ignored until the objective became Active.
    let objectives = vec![
        Objective {
            id: "reach".to_string(),
            kind: ObjectiveKind::ReachZone {
                min: [1180.0, 16.0],
                max: [1280.0, 64.0],
            },
            optional: false,
            status: ObjectiveStatus::Pending,
            progress_milestone_index: 0,
            progress: 0.0,
            fail_sensor: None,
        },
        Objective {
            id: "defend".to_string(),
            kind: ObjectiveKind::DefendReactor {
                target: "core_reactor".to_string(),
            },
            optional: false,
            status: ObjectiveStatus::Pending,
            progress_milestone_index: 0,
            progress: 0.0,
            fail_sensor: None,
        },
    ];
    let mut state = MissionState::new(
        objectives,
        0,
        LossConditions {
            player_dead: true,
            time_limit_ticks: 60 * 60,
        },
    );
    // BP2 fix: after construction NO objective is Active yet — step()
    // activates the first one. `defend` is queued at index 1 = Pending.
    assert_eq!(state.objectives[0].status, ObjectiveStatus::Pending);
    assert_eq!(state.objectives[1].status, ObjectiveStatus::Pending);
    let actors = mk_actors(player_at(120.0, 32.0), false);
    let mut reactors = BTreeMap::new();
    reactors.insert("core_reactor".to_string(), true);
    let report = step(
        &mut state,
        MissionTickInputs {
            tick: 100,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &BTreeMap::new(),
            reactors_destroyed: &reactors,
            breaches_progress: &BTreeMap::new(),
        },
    );
    assert!(matches!(
        state.result,
        MissionResult::Lost {
            reason: LossReason::ReactorDestroyed
        }
    ));
    assert_eq!(report.objective_failed, vec!["defend".to_string()]);
    assert_eq!(state.objectives[1].status, ObjectiveStatus::Failed);
}

#[test]
fn defend_reactor_with_outstanding_objective_loses_at_timer_even_with_reactor_alive() {
    // Devin BUG_pr-review-job 0001 (flag) regression: a mixed-objective
    // scenario (DefendReactor + ReachZone where the player is NOT in
    // the zone at timer expiry) must resolve on the timer-expired tick
    // as TimerExpired loss, not silently stay Active.
    let objectives = vec![
        Objective {
            id: "defend".to_string(),
            kind: ObjectiveKind::DefendReactor {
                target: "core_reactor".to_string(),
            },
            optional: false,
            status: ObjectiveStatus::Pending,
            progress_milestone_index: 0,
            progress: 0.0,
            fail_sensor: None,
        },
        Objective {
            id: "reach".to_string(),
            kind: ObjectiveKind::ReachZone {
                min: [1180.0, 16.0],
                max: [1280.0, 64.0],
            },
            optional: false,
            status: ObjectiveStatus::Pending,
            progress_milestone_index: 0,
            progress: 0.0,
            fail_sensor: None,
        },
    ];
    let mut state = MissionState::new(
        objectives,
        0,
        LossConditions {
            player_dead: true,
            time_limit_ticks: 60 * 60,
        },
    );
    let actors = mk_actors(player_at(120.0, 32.0), false); // not in extract zone
    let mut reactors = BTreeMap::new();
    reactors.insert("core_reactor".to_string(), false);
    let report = step(
        &mut state,
        MissionTickInputs {
            tick: 60 * 60,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &BTreeMap::new(),
            reactors_destroyed: &reactors,
            breaches_progress: &BTreeMap::new(),
        },
    );
    // Defend was completed by surviving the timer.
    assert!(report.objective_completed.contains(&"defend".to_string()));
    // But the reach zone is still pending → mission must lose on timer.
    assert!(matches!(
        state.result,
        MissionResult::Lost {
            reason: LossReason::TimerExpired
        }
    ));
    assert!(report.final_result.is_some());
}

#[test]
fn timer_expires_on_first_step_with_reachzone_first_defendreactor_pending_yields_timer_loss() {
    // ReachZone listed first (so Phase 0 activates ReachZone, not the
    // DefendReactor) and the timer happens to be already expired on the
    // first step() call, the mission must resolve as `Lost { TimerExpired }`
    // — NOT silently win because the Pending DefendReactor's reactor is
    // still alive. The win path requires the DefendReactor to actually be
    // Active, and the reach-zone is still outstanding, so the only correct
    // resolution is TimerExpired loss.
    let objectives = vec![
        Objective {
            id: "reach".to_string(),
            kind: ObjectiveKind::ReachZone {
                min: [1180.0, 16.0],
                max: [1280.0, 64.0],
            },
            optional: false,
            status: ObjectiveStatus::Pending,
            progress_milestone_index: 0,
            progress: 0.0,
            fail_sensor: None,
        },
        Objective {
            id: "defend".to_string(),
            kind: ObjectiveKind::DefendReactor {
                target: "core_reactor".to_string(),
            },
            optional: false,
            status: ObjectiveStatus::Pending,
            progress_milestone_index: 0,
            progress: 0.0,
            fail_sensor: None,
        },
    ];
    let mut state = MissionState::new(
        objectives,
        0,
        LossConditions {
            player_dead: true,
            time_limit_ticks: 1, // timer expires on the first step()
        },
    );
    let actors = mk_actors(player_at(120.0, 32.0), false); // not in zone
    let mut reactors = BTreeMap::new();
    reactors.insert("core_reactor".to_string(), false); // reactor still alive
    let report = step(
        &mut state,
        MissionTickInputs {
            tick: 1,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &BTreeMap::new(),
            reactors_destroyed: &reactors,
            breaches_progress: &BTreeMap::new(),
        },
    );
    assert!(matches!(
        state.result,
        MissionResult::Lost {
            reason: LossReason::TimerExpired
        }
    ));
    // Phase 0 still ran first — ReachZone activated.
    assert_eq!(report.objective_started, vec!["reach".to_string()]);
    // are flipped to Failed on timer-expired loss so the engine emits
    // `mission.objective_failed { reason: "timer_expired" }` before
    // `mission.mission_resolved`. The ReachZone just got Activated this
    // tick, then fails immediately.
    assert_eq!(state.objectives[0].status, ObjectiveStatus::Failed);
    // DefendReactor never got activated — it stays Pending (the player
    // didn't even get a tick to start defending).
    assert_eq!(state.objectives[1].status, ObjectiveStatus::Pending);
    assert_eq!(report.objective_failed, vec!["reach".to_string()]);
}

#[test]
fn defend_reactor_wins_when_timer_expires_with_reactor_alive() {
    let mut state = build_reactor_defense_state(60 * 60);
    let actors = mk_actors(player_at(120.0, 32.0), false);
    let mut reactors = BTreeMap::new();
    reactors.insert("core_reactor".to_string(), false);
    let report = step(
        &mut state,
        MissionTickInputs {
            tick: 60 * 60,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &BTreeMap::new(),
            reactors_destroyed: &reactors,
            breaches_progress: &BTreeMap::new(),
        },
    );
    assert!(matches!(state.result, MissionResult::Won));
    assert_eq!(report.objective_completed, vec!["defend_reactor".to_string()]);
    assert!(report.final_result.is_some());
}

#[test]
fn reactor_apply_damage_two_partial_hits_then_kill_in_separate_calls() {
    // kill hit with the per-hit hp captured at each step. The cf-control
    // engine emits per-hit state captured at apply_damage time, not the
    // post-loop final state, so each event reflects the truthful hp.
    let mut r = Reactor {
        id: "r".to_string(),
        position: [0.0, 0.0],
        half_extents: [16.0, 16.0],
        hp: 100.0,
        max_hp: 100.0,
        destroyed: false,
        ..Default::default()
    };
    let prev_hp_1 = r.hp;
    let prev_destroyed_1 = r.is_destroyed();
    r.apply_damage(30.0);
    assert_eq!(r.hp, 70.0);
    assert!(!r.is_destroyed());
    assert!(
        !prev_destroyed_1 && !r.is_destroyed(),
        "first hit should not have flipped destroyed"
    );
    let _ = prev_hp_1;

    let prev_hp_2 = r.hp;
    let prev_destroyed_2 = r.is_destroyed();
    r.apply_damage(40.0);
    assert_eq!(r.hp, 30.0);
    assert!(!r.is_destroyed());
    assert!(
        !prev_destroyed_2 && !r.is_destroyed(),
        "second hit should not have flipped destroyed"
    );
    let _ = prev_hp_2;

    let prev_destroyed_3 = r.is_destroyed();
    r.apply_damage(40.0);
    assert_eq!(r.hp, 0.0);
    assert!(r.is_destroyed());
    assert!(
        !prev_destroyed_3 && r.is_destroyed(),
        "third hit should have flipped destroyed"
    );

    // Subsequent damage is a no-op (latched destroyed).
    let before = r.hp;
    r.apply_damage(50.0);
    assert_eq!(r.hp, before);
}

#[test]
fn reactor_object_apply_damage_drives_destruction() {
    let mut r = Reactor {
        id: "r".to_string(),
        position: [100.0, 32.0],
        half_extents: [16.0, 16.0],
        hp: 30.0,
        max_hp: 30.0,
        destroyed: false,
        ..Default::default()
    };
    r.apply_damage(10.0);
    assert!(!r.is_destroyed());
    r.apply_damage(20.0);
    assert!(r.is_destroyed());
    let before = r.hp;
    r.apply_damage(50.0);
    assert_eq!(r.hp, before);
}

#[test]
fn reactor_world_destroyed_map_round_trip() {
    let world = ReactorWorld::new(vec![Reactor {
        id: "alpha".to_string(),
        position: [0.0, 0.0],
        half_extents: [8.0, 8.0],
        hp: 50.0,
        max_hp: 50.0,
        destroyed: false,
        ..Default::default()
    }]);
    let map = world.destroyed_map();
    assert_eq!(map.get("alpha"), Some(&false));
}

#[test]
fn reactor_aabb_contains_inside_and_outside() {
    let r = Reactor {
        id: "r".to_string(),
        position: [100.0, 100.0],
        half_extents: [16.0, 16.0],
        hp: 50.0,
        max_hp: 50.0,
        destroyed: false,
        ..Default::default()
    };
    assert!(r.aabb_contains(100.0, 100.0));
    assert!(r.aabb_contains(116.0, 116.0));
    assert!(!r.aabb_contains(200.0, 100.0));
}

#[test]
fn mission_view_round_trip() {
    // BP2 fix: build_state() returns all objectives Pending; drive one
    // step() to activate the first one before asserting active_objective.
    let mut state = build_state();
    let actors = mk_actors(player_at(120.0, 32.0), false);
    let _ = step(
        &mut state,
        MissionTickInputs {
            tick: 1,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &BTreeMap::new(),
            reactors_destroyed: &BTreeMap::new(),
            breaches_progress: &BTreeMap::new(),
        },
    );
    let view = MissionView::from_state(&state, 30);
    assert_eq!(view.result, "in_progress");
    assert_eq!(view.active_objective.as_deref(), Some("breach"));
    assert_eq!(view.objectives.len(), 3);
    assert_eq!(view.objectives[0].kind, "breach_barrier");
    assert_eq!(view.objectives[1].kind, "neutralize_actor");
    assert_eq!(view.objectives[2].kind, "reach_zone");
}

#[test]
fn objective_failed_emitted_on_reactor_destroyed() {
    // Item 679: test objective_failed event path.
    //
    // **Hardening regression fix (M1 R2)**: the production code at
    // `reactor_destroyed_match` was hardened per Bugbot 3212230553 (Low) to
    // scan Active OR Pending defend_reactor objectives so a destroyed
    // reactor on a queued objective still produces an immediate loss.
    // That means the destruction now resolves on tick 1 (when the first-
    // pending guard activates the objective and the reactor-destroyed
    // check runs in the same tick). The original test discarded tick 1
    // and asserted on tick 2 (which returns an empty report because the
    // mission is already terminal). Updated to assert on tick 1's report
    // — same intent (objective_failed fires when reactor destroyed),
    // correct lifecycle.
    let objectives = vec![Objective {
        id: "defend".to_string(),
        kind: ObjectiveKind::DefendReactor {
            target: "core".to_string(),
        },
        optional: false,
        status: ObjectiveStatus::Pending,
        progress_milestone_index: 0,
        progress: 0.0,
        fail_sensor: None,
    }];
    let loss = LossConditions {
        player_dead: false,
        time_limit_ticks: 3600,
    };
    let mut state = MissionState::new(objectives, 0, loss);
    let reactors = ReactorWorld::new(vec![Reactor {
        id: "core".to_string(),
        position: [50.0, 50.0],
        half_extents: [10.0, 10.0],
        hp: 0.0,
        max_hp: 100.0,
        destroyed: true,
        ..Default::default()
    }]);
    let actors = mk_actors(player_at(100.0, 32.0), false);
    let report = step(
        &mut state,
        MissionTickInputs {
            tick: 1,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &BTreeMap::new(),
            reactors_destroyed: &reactors.destroyed_map(),
            breaches_progress: &BTreeMap::new(),
        },
    );
    assert!(
        !report.objective_failed.is_empty(),
        "objective_failed must be emitted when reactor is destroyed (tick 1 report = {report:?})"
    );
    assert_eq!(report.objective_failed[0], "defend");
    assert!(
        matches!(
            state.result,
            MissionResult::Lost {
                reason: LossReason::ReactorDestroyed
            }
        ),
        "state.result must be Lost(ReactorDestroyed), got {:?}",
        state.result
    );
    // Subsequent ticks are no-ops because the mission is terminal.
    let report_after = step(
        &mut state,
        MissionTickInputs {
            tick: 2,
            player: actors.get(&ActorId(1)),
            actors: &actors,
            breaches_broken: &BTreeMap::new(),
            reactors_destroyed: &reactors.destroyed_map(),
            breaches_progress: &BTreeMap::new(),
        },
    );
    assert!(report_after.objective_failed.is_empty(), "terminal step must be empty");
}

/// destruction by capping HP at 1 and forcing pressure_state=Critical.
#[test]
fn reactor_tutorial_safety_caps_hp_at_one() {
    let mut reactor = Reactor {
        id: "core_reactor".to_string(),
        position: [0.0, 0.0],
        half_extents: [16.0, 16.0],
        hp: 100.0,
        max_hp: 100.0,
        ..Default::default()
    };
    let report = reactor.apply_damage_cascade_with_safety(500.0, true);
    assert_eq!(reactor.hp, 1.0, "tutorial_safety caps reactor HP at 1.0");
    assert_eq!(
        reactor.pressure_state,
        reactor::PressureState::Critical,
        "tutorial_safety forces pressure_state=Critical"
    );
    assert!(!reactor.is_destroyed(), "tutorial_safety blocks destruction");
    assert!(
        !report.now_destroyed,
        "tutorial-safety damage report must not flag now_destroyed"
    );
    assert!(
        !report.triggered_destruction,
        "tutorial-safety damage report must not trigger destruction"
    );
}

/// cascade still destroys the reactor on lethal damage (back-compat
/// with the legacy `apply_damage_cascade` signature).
#[test]
fn reactor_without_tutorial_safety_still_destroys() {
    let mut reactor = Reactor {
        id: "core_reactor".to_string(),
        position: [0.0, 0.0],
        half_extents: [16.0, 16.0],
        hp: 100.0,
        max_hp: 100.0,
        ..Default::default()
    };
    let report = reactor.apply_damage_cascade_with_safety(500.0, false);
    assert_eq!(reactor.hp, 0.0);
    assert!(reactor.is_destroyed());
    assert_eq!(reactor.pressure_state, reactor::PressureState::Destroyed);
    assert!(report.now_destroyed);
    assert!(report.triggered_destruction);
}
