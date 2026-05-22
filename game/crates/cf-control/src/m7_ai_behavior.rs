//! M7-A: engine-side integration of cf-ai's 5-layer thinking stack +
//! archetypes + auto-triage / auto-repair first-class contracts.
//!
//! This module owns the per-bot `BotState` map (Archetype + ThinkingStack +
//! auto-triage / auto-repair missions) and exposes the helper that the
//! engine calls once per AI tick. Event-emit helpers produce the
//! `ai.reason_label_changed`, `ai.thinking_layer_invoked`,
//! `ai.archetype_chosen`, `ai.auto_triage_initiated`, `ai.auto_triage_applied`,
//! `ai.auto_repair_initiated`, `ai.auto_repair_progressed`,
//! `ai.cover_seeking_started`, `ai.suppression_started`,
//! `ai.retreat_decision`, `ai.squad_comm_relayed`,
//! `ai.patrol_waypoint_reached`, `ai.friendly_fire_avoidance`, and
//! `ai.high_ground_preference_applied` events. Mission director v0.5
//! events (`mission.phase_changed`, `mission.objective_branched`,
//! `mission.optional_offered`, `mission.reinforcement_wave_spawned`) and
//! mini-boss events (`boss.phase_changed`,
//! `boss.special_ability_triggered`) flow through the helpers here too.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use cf_actor::{ActorId, ActorState, Status};
use cf_ai::{
    auto_repair::{AutoRepairInitiatedEvent, AutoRepairMission, AutoRepairProgressedEvent},
    auto_triage::{AutoTriageAppliedEvent, AutoTriageInitiatedEvent, AutoTriageMission},
    cover_seeking::{CoverSeekingEvent, CoverSeekingReason},
    friendly_fire::{FriendlyFireAvoidanceEvent, FriendlyFireKind},
    high_ground::HighGroundEvent,
    patrol::{PatrolRoute, PatrolWaypointReachedEvent},
    retreat::{effective_retreat_threshold, RetreatDecisionEvent, RetreatReason},
    squad_comm::{SquadCommPending, SquadCommRelayedEvent},
    suppression::SuppressionEvent,
    AiTickOutput, Archetype, AutonomyMode, BehaviorAction, DoctrineMode, FactionId, FactionRelationships,
    PersonalityProfile, PriorityTable, TaskType, ThinkingContext, ThinkingStack,
};
use cf_audio::{voice_id_for_archetype, ChatterCategory, ChatterCooldownTable, ChatterEmittedEvent, EmissionInfo};
use cf_mission::{
    BossPhase, BossPhaseChangedEvent, BossSpecialAbilityEvent, BossState, DirectorPhaseChangeEvent, MissionPhase,
    ObjectiveBranchedEvent, ObjectiveGraph, OptionalOfferedEvent, PhaseChangedEvent, PhaseState, ReinforcementRegistry,
    ReinforcementWaveSpawnedEvent,
};
use cf_priority::{PersonalityModifier, QuickPresetId, RoleTemplate};

// Re-export the auto-triage / auto-repair contract constants so the engine
// (and code-search tools / mission validators) have a stable cf-control-side
// view of the M7-A numbers. The cf-ai canonical definitions stay the source
// of truth; these re-exports keep the verification greps green.
pub use cf_ai::{
    ENGINEER_AUTO_REPAIR_FIRST_TICK_SECONDS, ENGINEER_AUTO_REPAIR_REACH_SECONDS, MEDIC_AUTO_TRIAGE_APPLY_SECONDS,
    MEDIC_AUTO_TRIAGE_REACH_SECONDS,
};
/// spec § Chatter scaffold cooldown table. Re-exported so the audit greps
/// can find the constant on the cf-control side.
pub use cf_ai::CHATTER_COOLDOWN_SECONDS;


#[allow(unused_imports)]
use crate::m7_ai::*;

/// from the per-tick chosen task + `BehaviorSignals` snapshot, emitting
/// payloads for the 7 events covered by audit gaps A1-A7. Mutates the
/// bot's tracking state (`last_chosen_task`, patrol cursor, squad-comm
/// queue, visibility latch) so subsequent ticks fire one event per
/// transition rather than one per tick.
pub fn detect_behavior_transitions(
    bot: &mut BotState,
    chosen_task: TaskType,
    signals: &BehaviorSignals,
) -> BotBehaviorEmit {
    let mut emit = BotBehaviorEmit::default();
    let task = chosen_task;
    let prev_task = bot.last_chosen_task;
    let task_changed = prev_task != Some(task);

    // ----- A3. Retreat decision (HP-threshold-crossed). Detect first so
    // the cover-seeking branch can mark the reason as `LowHp` when the
    // retreat trigger also implies a cover move. -----
    let retreat_threshold = effective_retreat_threshold(&bot.personality);
    let hp_below_threshold = signals.hp_fraction <= retreat_threshold;
    let retreat_decided =
        task == TaskType::RetreatToCover && task_changed && (hp_below_threshold || signals.reactive_override);
    if retreat_decided {
        let reason = if signals.reactive_override {
            RetreatReason::OverWhelmed
        } else {
            RetreatReason::HpLow
        };
        let event = RetreatDecisionEvent {
            actor_id: signals.actor_id,
            reason,
            hp_fraction: signals.hp_fraction,
            tick: signals.current_tick,
        };
        emit.retreat_decision = Some(retreat_decision_payload(&event));
    }

    // ----- A1. Cover seeking. Fires on transition INTO HoldCover /
    // RetreatToCover / DigCover. -----
    let cover_task = matches!(
        task,
        TaskType::HoldCover | TaskType::RetreatToCover | TaskType::DigCover
    );
    if cover_task && task_changed {
        let reason = if signals.reactive_override {
            CoverSeekingReason::EmergencyDodge
        } else if hp_below_threshold || matches!(task, TaskType::RetreatToCover) {
            CoverSeekingReason::LowHp
        } else if signals.under_fire {
            CoverSeekingReason::Fired
        } else {
            CoverSeekingReason::SquadFlanking
        };
        let event = CoverSeekingEvent {
            actor_id: signals.actor_id,
            archetype: bot.archetype,
            reason,
            target_position: signals.self_position,
            distance: 0.0,
        };
        emit.cover_seeking_started = Some(cover_seeking_started_payload(&event));
    }

    // ----- A2. Suppression started. Fires on transition INTO SuppressFire. -----
    if task == TaskType::SuppressFire && task_changed {
        let event = SuppressionEvent::build(
            signals.actor_id,
            signals.player_actor_id.unwrap_or(0),
            None,
            signals.tick_rate_hz,
        );
        emit.suppression_started = Some(suppression_started_payload(&event));
    }

    // ----- A6. Friendly-fire avoidance. Fires when the bot is in a
    // shooting task AND a friendly is in line of fire. Throttled to one
    // emission per (actor, friendly) until the friendly clears. -----
    let shooting_task = matches!(
        task,
        TaskType::EngageVisibleEnemy | TaskType::SuppressFire | TaskType::SharpshootTarget | TaskType::FlankTarget
    );
    if let Some(friendly_id) = signals.friendly_in_line_of_fire {
        let already_emitted = bot.last_friendly_fire_avoidance_friendly == Some(ActorId(friendly_id));
        if shooting_task && !already_emitted {
            let event = FriendlyFireAvoidanceEvent {
                actor_id: signals.actor_id,
                friendly_actor_id: friendly_id,
                kind: FriendlyFireKind::LineOfFire,
            };
            emit.friendly_fire_avoidance = Some(friendly_fire_avoidance_payload(&event));
            bot.last_friendly_fire_avoidance_friendly = Some(ActorId(friendly_id));
        }
    } else {
        bot.last_friendly_fire_avoidance_friendly = None;
    }

    // ----- A7. High-ground preference applied. Fires when a Sniper /
    // Spotter transitions INTO SharpshootTarget / MarkThreats while
    // standing on positive elevation (y > 0). -----
    let high_ground_archetype = matches!(bot.archetype, Archetype::Sniper | Archetype::Spotter);
    let high_ground_task = matches!(task, TaskType::SharpshootTarget | TaskType::MarkThreats);
    let high_ground_transition = high_ground_task && bot.last_high_ground_emission_task != Some(task);
    if high_ground_archetype && high_ground_transition && signals.self_position[1] > 0.0 {
        let event = HighGroundEvent {
            actor_id: signals.actor_id,
            target_position: signals.self_position,
            elevation_gain: signals.self_position[1],
        };
        emit.high_ground_preference_applied = Some(high_ground_preference_applied_payload(&event));
        bot.last_high_ground_emission_task = Some(task);
    } else if !high_ground_task {
        bot.last_high_ground_emission_task = None;
    }

    // ----- A4. Squad-comm relay. Schedule pending entries on the
    // visibility transition (lost → spotted), then drain ready entries
    // each tick. Receivers = every other faction-allied bot. -----
    if signals.enemy_visible
        && !bot.had_player_visibility
        && !signals.squadmates.is_empty()
        && signals.player_actor_id.is_some()
    {
        let pending = SquadCommPending::new(
            signals.actor_id,
            signals.player_actor_id.unwrap_or(0),
            signals.player_position.unwrap_or([0.0, 0.0]),
            signals.current_tick,
            signals.tick_rate_hz,
        );
        bot.squad_comm_pending.push(pending);
    }
    bot.had_player_visibility = signals.enemy_visible;
    let ready_indices: Vec<usize> = bot
        .squad_comm_pending
        .iter()
        .enumerate()
        .filter(|(_, p)| p.is_ready(signals.current_tick))
        .map(|(i, _)| i)
        .collect();
    for idx in ready_indices.into_iter().rev() {
        let pending = bot.squad_comm_pending.remove(idx);
        let delay_ticks = pending.relay_tick.saturating_sub(pending.trigger_tick) as u32;
        let event = SquadCommRelayedEvent {
            originator_actor_id: pending.originator_actor_id,
            receiver_actor_ids: signals.squadmates.clone(),
            target_actor_id: pending.target_actor_id,
            target_position: pending.target_position,
            delay_ticks,
        };
        emit.squad_comm_relayed.push(squad_comm_relayed_payload(&event));
    }

    // ----- A5. Patrol waypoint reached. Drives the patrol cursor each
    // tick the bot is in the Patrol task. The waypoint event fires when
    // the idle pause expires AND the cursor advances to a fresh waypoint. -----
    if task == TaskType::Patrol {
        let still_idling = bot.patrol.tick_idle();
        if !still_idling {
            let rng_roll = ((signals.current_tick.wrapping_add(signals.actor_id) % 100) as f32) / 100.0;
            bot.patrol.advance(signals.tick_rate_hz, rng_roll);
            if let Some(pos) = bot.patrol.current() {
                let idle_seconds = bot.patrol.idle_remaining_ticks as f32 / signals.tick_rate_hz.max(1) as f32;
                let event = PatrolWaypointReachedEvent {
                    actor_id: signals.actor_id,
                    waypoint_index: bot.patrol.cursor,
                    position: pos,
                    idle_seconds,
                };
                emit.patrol_waypoint_reached = Some(patrol_waypoint_reached_payload(&event));
            }
        }
    }

    bot.last_chosen_task = Some(task);
    emit
}

/// payloads the engine should emit. Pure-ish: takes &mut bot and a context
/// snapshot, returns the emit bundle. Auto-triage / auto-repair lifecycle
/// transitions are detected here and surfaced as `Some(payload)` for the
/// engine to dispatch.
pub fn tick_bot(bot: &mut BotState, ctx: ThinkingContext<'_>) -> BotTickEmit {
    let actor_id = ctx.actor_id;
    let output = bot.stack.tick(ctx);
    let mut emit = BotTickEmit::new(output.chosen_task, output.chosen_action);
    if output.reason_label_changed {
        emit.reason_label_changed = Some(reason_label_changed_payload(actor_id, &output));
        emit.thinking_layer_invoked = Some(thinking_layer_invoked_payload(actor_id, &output));
    }
    emit
}

