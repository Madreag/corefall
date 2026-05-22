//! Per-tick `step()` driver — advance the mission state machine by one tick.
//! Split out of `lib.rs` for the 2k-LOC ceiling. Public API is re-exported
//! at the crate root.

use cf_actor::{ActorId, Status};

use crate::loss::LossReason;
use crate::objective_types::{ObjectiveKind, ObjectiveStatus};
use crate::result::MissionResult;
use crate::state::MissionState;
use crate::tick::{MissionTickInputs, MissionTickReport, ObjectiveProgressUpdate};

/// vocabulary so the run-bundle viewer can render a progress bar.
const PROGRESS_QUARTILES: [f32; 4] = [0.25, 0.5, 0.75, 1.0];

/// Drive the mission state machine for one tick. Idempotent once the result is
/// terminal (returns an empty report after Won/Lost).
#[must_use]
pub fn step(state: &mut MissionState, inputs: MissionTickInputs<'_>) -> MissionTickReport {
    let mut report = MissionTickReport::default();
    if state.result.is_terminal() {
        return report;
    }
    // AND timer accounting. The caller is responsible for calling
    // `MissionState::resume` to lift the gate.
    if state.paused {
        return report;
    }

    // 0) BP2 fix: if no objective is currently Active (e.g. on the FIRST tick
    //    after `MissionState::new()` or `reset()`), activate the first pending
    //    objective AND push it to `report.objective_started` so the engine
    //    emits a `mission.objective_started` event. Without this guard the
    //    first objective transitioned Pending → Active silently inside new()
    //    and the started event was lost.
    if !state.objectives.iter().any(|o| o.status == ObjectiveStatus::Active) {
        if let Some(first) = state
            .objectives
            .iter_mut()
            .find(|o| o.status == ObjectiveStatus::Pending)
        {
            first.status = ObjectiveStatus::Active;
            report.objective_started.push(first.id.clone());
            state.last_event_tick = inputs.tick;
            state.last_event_label = format!("objective_started:{}", first.id);
        }
    }

    // 1) Loss conditions take precedence over objective progress so a fail-state
    //    that lands on the same tick as an objective completion is still recorded
    //    as a loss. This matches the M7 mission-director failure ordering.
    if state.loss.player_dead {
        let player_dead = match inputs.player {
            Some(p) => p.status.is_dead() || p.hp <= 0.0,
            None => false,
        };
        if player_dead {
            state.result = MissionResult::Lost {
                reason: LossReason::PlayerDead,
            };
            state.last_event_tick = inputs.tick;
            state.last_event_label = "mission_lost_player_dead".to_string();
            report.final_result = Some(state.result.clone());
            return report;
        }
    }
    // which actor is currently being puppeted. Scans every actor flagged
    // `is_brain == true` and checks for `Dead` status / hp ≤ 0. This runs
    // BEFORE reactor_destroyed because brain death is mission-critical
    // even when a defend_reactor objective is in flight.
    let brain_dead = inputs
        .actors
        .values()
        .any(|a| a.is_brain && (a.status.is_dead() || a.hp <= 0.0));
    if brain_dead {
        state.result = MissionResult::Lost {
            reason: LossReason::BrainDestroyed,
        };
        state.last_event_tick = inputs.tick;
        state.last_event_label = "mission_lost_brain_destroyed".to_string();
        report.final_result = Some(state.result.clone());
        return report;
    }
    // Reactor destruction loses immediately for any active `defend_reactor`
    // objective. M2.5's micro reactor defense needs `mission.loss_reason =
    // reactor_destroyed` to be the visible failure label. The check runs BEFORE
    // the timer expiry check so a reactor destroyed exactly at the timer
    // boundary still records as `reactor_destroyed`.
    //
    // The first pass scans for the failing index using an immutable borrow so
    // we can write `state.last_event_label = format!(...)` afterwards without
    // aliasing `state.objectives`. The second pass mutates the matched
    // objective's `status` to `Failed` so `MissionView::from_state` reports
    // `failed` in the observe envelope (Devin review BUG_pr-review-job
    // -8dddb0ae78c7456997c4d2dc7aade217_0001).
    // objectives. If a defend_reactor is queued behind another objective
    // (Pending status) and its reactor is destroyed in the meantime, the
    // mission MUST resolve as Lost { ReactorDestroyed } immediately rather
    // than wait for the objective to become Active. Completed/Failed rows
    // are skipped because they're terminal states.
    let reactor_destroyed_match: Option<(usize, String, String)> =
        state.objectives.iter().enumerate().find_map(|(idx, obj)| {
            if matches!(obj.status, ObjectiveStatus::Completed | ObjectiveStatus::Failed) {
                return None;
            }
            if let ObjectiveKind::DefendReactor { target } = &obj.kind {
                if inputs.reactors_destroyed.get(target).copied().unwrap_or(false) {
                    return Some((idx, obj.id.clone(), target.clone()));
                }
            }
            None
        });
    if let Some((idx, obj_id, target)) = reactor_destroyed_match {
        state.objectives[idx].status = ObjectiveStatus::Failed;
        state.result = MissionResult::Lost {
            reason: LossReason::ReactorDestroyed,
        };
        state.last_event_tick = inputs.tick;
        state.last_event_label = format!("mission_lost_reactor_destroyed:{target}");
        report.objective_failed.push(obj_id);
        report.final_result = Some(state.result.clone());
        return report;
    }

    // When `loss_on_destroyed` is true (schema default) and the defended
    // actor is destroyed before the deadline, immediately resolve the
    // mission as Lost. Reactors map to `LossReason::ReactorDestroyed`;
    // numeric actor ids map to `LossReason::PlayerDied` for M14 baseline
    // (M25+ command-core will introduce ActorDestroyed).
    let defend_actor_destroyed_match: Option<(usize, String, String, bool)> =
        state.objectives.iter().enumerate().find_map(|(idx, obj)| {
            if matches!(obj.status, ObjectiveStatus::Completed | ObjectiveStatus::Failed) {
                return None;
            }
            if let ObjectiveKind::DefendActor {
                actor_id,
                loss_on_destroyed,
                tutorial_safety,
                ..
            } = &obj.kind
            {
                if !*loss_on_destroyed || *tutorial_safety {
                    return None;
                }
                let destroyed_reactor = inputs
                    .reactors_destroyed
                    .get(actor_id)
                    .copied()
                    .unwrap_or(false);
                let destroyed_actor = actor_id.parse::<u64>().ok().is_some_and(|n| {
                    inputs
                        .actors
                        .get(&ActorId(n))
                        .map(|a| a.status == Status::Dead)
                        .unwrap_or(false)
                });
                if destroyed_reactor || destroyed_actor {
                    return Some((idx, obj.id.clone(), actor_id.clone(), destroyed_reactor));
                }
            }
            None
        });
    if let Some((idx, obj_id, target, was_reactor)) = defend_actor_destroyed_match {
        state.objectives[idx].status = ObjectiveStatus::Failed;
        state.result = MissionResult::Lost {
            reason: if was_reactor {
                LossReason::ReactorDestroyed
            } else {
                LossReason::PlayerDead
            },
        };
        state.last_event_tick = inputs.tick;
        state.last_event_label = format!("mission_lost_defend_actor_destroyed:{target}");
        report.objective_failed.push(obj_id);
        report.final_result = Some(state.result.clone());
        return report;
    }

    let timer_expired = state.time_limit_ticks > 0 && state.elapsed_ticks(inputs.tick) >= state.time_limit_ticks;
    if timer_expired {
        // Special case: an active `defend_reactor` objective WINS when the
        // timer expires (the player held the reactor through the wave). We
        // detect this by looking for an active defend_reactor objective whose
        // reactor is still alive.
        let defend_active_alive = state.objectives.iter().any(|obj| {
            matches!(obj.status, ObjectiveStatus::Active)
                && match &obj.kind {
                    ObjectiveKind::DefendReactor { target } => {
                        !inputs.reactors_destroyed.get(target).copied().unwrap_or(false)
                    }
                    _ => false,
                }
        });
        if defend_active_alive {
            // Mark every active defend_reactor objective complete and check win
            // condition below.
            for obj in &mut state.objectives {
                if obj.status != ObjectiveStatus::Active {
                    continue;
                }
                if let ObjectiveKind::DefendReactor { target } = &obj.kind {
                    if !inputs.reactors_destroyed.get(target).copied().unwrap_or(false) {
                        obj.status = ObjectiveStatus::Completed;
                        report.objective_completed.push(obj.id.clone());
                        state.last_event_tick = inputs.tick;
                        state.last_event_label = format!("objective_completed:{}", obj.id);
                    }
                }
            }
            // Devin BUG_pr-review-job 0001 (flag): if the timer is expired AND
            // a defend_reactor was just completed by surviving the timer, the
            // mission MUST resolve on this tick rather than fall through. On
            // the NEXT tick the timer would still be expired, the
            // defend_reactor would no longer be active (we just completed
            // it), and `defend_active_alive` would be false, sending the
            // mission to TimerExpired loss instead of Won. The latent bug
            // only matters for hypothetical mixed-objective scenarios
            // (DefendReactor + ReachZone, etc.), but resolving here is the
            // robust fix.
            //
            // If every required objective is now complete, win immediately.
            // Otherwise, the player completed defend_reactor at the timer
            // boundary but still owes other required objectives — that's a
            // TimerExpired loss because the rest of the mission is not done.
            if state.outstanding_required() == 0 && state.failed_required() == 0 {
                state.result = MissionResult::Won;
                state.last_event_tick = inputs.tick;
                state.last_event_label = "mission_won".to_string();
                report.final_result = Some(state.result.clone());
            } else {
                state.result = MissionResult::Lost {
                    reason: LossReason::TimerExpired,
                };
                state.last_event_tick = inputs.tick;
                state.last_event_label = "mission_lost_timer".to_string();
                report.final_result = Some(state.result.clone());
            }
            return report;
        }
        // fires with objective_id='reach_extraction', reason='timer_expired'"
        // when the timer expires while any Active objective is incomplete.
        // Iterate every Active objective and flip it to Failed so the engine
        // emits per-objective `mission.objective_failed` before
        // `mission.mission_resolved`.
        for obj in &mut state.objectives {
            if obj.status == ObjectiveStatus::Active {
                obj.status = ObjectiveStatus::Failed;
                report.objective_failed.push(obj.id.clone());
            }
        }
        state.result = MissionResult::Lost {
            reason: LossReason::TimerExpired,
        };
        state.last_event_tick = inputs.tick;
        state.last_event_label = "mission_lost_timer".to_string();
        report.final_result = Some(state.result.clone());
        return report;
    }

    // 1b) **M1.5**: emit `mission.objective_updated` events when the active
    // `BreachBarrier` objective crosses the 25/50/75/100% carve milestones.
    // The 100% milestone fires on the same tick as `objective_completed`
    // so the cause chain shows: dig_request -> objective_updated{progress:1.0}
    // -> objective_completed.
    for obj in &mut state.objectives {
        if obj.status != ObjectiveStatus::Active {
            continue;
        }
        let progress = match &obj.kind {
            ObjectiveKind::BreachBarrier { target } => inputs.breaches_progress.get(target).copied().unwrap_or(0.0),
            _ => continue,
        };
        while (obj.progress_milestone_index as usize) < PROGRESS_QUARTILES.len() {
            let next = PROGRESS_QUARTILES[obj.progress_milestone_index as usize];
            if progress + 1e-6 >= next {
                obj.progress_milestone_index += 1;
                report.objective_updated.push(ObjectiveProgressUpdate {
                    objective_id: obj.id.clone(),
                    progress: next,
                });
                state.last_event_tick = inputs.tick;
                state.last_event_label = format!("objective_updated:{}:{:.2}", obj.id, next);
            } else {
                break;
            }
        }
    }

    // 2) Progress objectives in declaration order. We only advance one row at a
    //    time so the player always has a single Active objective for the HUD.
    let mut started_index: Option<usize> = None;
    for (i, obj) in state.objectives.iter_mut().enumerate() {
        if obj.status != ObjectiveStatus::Active {
            continue;
        }
        let completed = match &obj.kind {
            ObjectiveKind::BreachBarrier { target } => inputs.breaches_broken.get(target).copied().unwrap_or(false),
            ObjectiveKind::NeutralizeActor { target } => inputs
                .actors
                .get(&ActorId(*target))
                .is_some_and(|a| a.status == Status::Dead),
            ObjectiveKind::ReachZone { min, max } => match inputs.player {
                Some(p) => point_in_aabb(p.position.x, p.position.y, *min, *max),
                None => false,
            },
            ObjectiveKind::DefendReactor { .. } => {
                // DefendReactor only completes via the timer-expired branch
                // above; passive ticks never auto-complete it.
                false
            }
            // current_tick >= until_tick (or mission time_limit_ticks when
            // until_tick is None) AND the defended actor is still alive.
            // Loss-on-destroy is handled in the fail-sensor pre-pass.
            //
            // schema; resolve via reactors_destroyed map first (M9
            // command-core / reactor case), then fall back to parsing as
            // a u64 actor id (M25+ turret / chassis-module case).
            ObjectiveKind::DefendActor {
                actor_id,
                until_tick,
                ..
            } => {
                let deadline = until_tick.unwrap_or(state.time_limit_ticks);
                if inputs.tick < deadline {
                    false
                } else if inputs.reactors_destroyed.contains_key(actor_id) {
                    !inputs
                        .reactors_destroyed
                        .get(actor_id)
                        .copied()
                        .unwrap_or(false)
                } else if let Ok(numeric) = actor_id.parse::<u64>() {
                    inputs.actors.get(&ActorId(numeric)).is_some_and(|a| {
                        a.status != Status::Dead && a.status != Status::Dying
                    })
                } else {
                    false
                }
            }
            // M2 re-audit (2026-05-13): SurviveTimer completes when the
            // window has elapsed AND the player is still alive. The fail
            // branch (player dies) is handled by the player-dead loss
            // check earlier in step(); a SurviveTimer that hasn't elapsed
            // simply stays Active until then.
            ObjectiveKind::SurviveTimer { survive_ticks } => {
                let elapsed = inputs.tick.saturating_sub(state.started_at_tick);
                elapsed >= *survive_ticks
                    && inputs
                        .player
                        .is_some_and(|p| p.status != Status::Dead && p.status != Status::Dying)
            }
            // M2 re-audit (2026-05-13): EscortActor completes when the
            // escortee enters the destination AABB AND is still alive.
            ObjectiveKind::EscortActor {
                target,
                destination_min,
                destination_max,
            } => inputs.actors.get(&ActorId(*target)).is_some_and(|a| {
                a.status != Status::Dead
                    && point_in_aabb(a.position.x, a.position.y, *destination_min, *destination_max)
            }),
        };
        if completed {
            obj.status = ObjectiveStatus::Completed;
            report.objective_completed.push(obj.id.clone());
            state.last_event_tick = inputs.tick;
            state.last_event_label = format!("objective_completed:{}", obj.id);
            started_index = Some(i + 1);
            break;
        }
    }
    // Activate the next pending required objective, if any.
    if let Some(start_from) = started_index {
        for (j, obj) in state.objectives.iter_mut().enumerate().skip(start_from) {
            if obj.status == ObjectiveStatus::Pending {
                obj.status = ObjectiveStatus::Active;
                report.objective_started.push(obj.id.clone());
                state.last_event_tick = inputs.tick;
                state.last_event_label = format!("objective_started:{}", obj.id);
                break;
            }
            // Skip any already-completed/failed rows (e.g. optional rows resolved earlier).
            if !obj.status.is_terminal() {
                break;
            }
            let _ = j;
        }
    }

    // 3) Win condition: every required objective reached `Completed` and zero
    //    required failures.
    if state.outstanding_required() == 0 && state.failed_required() == 0 {
        state.result = MissionResult::Won;
        state.last_event_tick = inputs.tick;
        state.last_event_label = "mission_won".to_string();
        report.final_result = Some(state.result.clone());
    }

    // Track transition timing for analytics (W1 item 866).
    if report.final_result.is_some()
        || !report.objective_started.is_empty()
        || !report.objective_completed.is_empty()
        || !report.objective_failed.is_empty()
        || !report.objective_updated.is_empty()
    {
        state.last_transition_tick = inputs.tick;
    }
    if let Some(MissionResult::Lost { ref reason }) = report.final_result {
        state.loss_reason_label = Some(reason.as_str().to_string());
    }

    report
}

pub(crate) fn point_in_aabb(x: f32, y: f32, min: [f32; 2], max: [f32; 2]) -> bool {
    x >= min[0] && x <= max[0] && y >= min[1] && y <= max[1]
}
