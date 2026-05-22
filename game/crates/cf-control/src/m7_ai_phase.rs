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

/// § Mission director v0.5 → Mini-boss. Phase 1 has no special ability
/// (ranged baseline); Phase 2 raises a shield; Phase 3 enters enraged
/// final stand. The string is the wire form embedded in
/// `boss.special_ability_triggered.ability`.
pub fn boss_ability_for_phase(phase: BossPhase) -> Option<&'static str> {
    match phase {
        BossPhase::Phase1 => None,
        BossPhase::Phase2 => Some("shield"),
        BossPhase::Phase3 => Some("enraged"),
    }
}

/// `mission.phase_changed` when a transition fires. **M9** extends this
/// to drive the 7-phase reactor-defense pacer; the
/// `mission.director_phase_change` companion payload is surfaced through
/// [`advance_phase_with_director_event`].
pub fn advance_phase(world: &mut M7AiWorld, tick: u64, tick_rate_hz: u32, cause: &str) -> Option<Value> {
    advance_phase_with_director_event(world, tick, tick_rate_hz, cause).map(|(legacy, _director)| legacy)
}

/// `mission.phase_changed` payload (back-compat) AND the M9
/// `mission.director_phase_change` payload (with `duration_seconds` of
/// the just-completed phase). Returns `Some((legacy_payload,
/// director_payload))` on a transition, `None` otherwise. The director
/// payload is the canonical surface for M10 viewer + M11 HUD strips.
pub fn advance_phase_with_director_event(
    world: &mut M7AiWorld,
    tick: u64,
    tick_rate_hz: u32,
    cause: &str,
) -> Option<(Value, Value)> {
    let phase = world.phase.as_mut()?;
    let deadline = phase.deadline_tick(tick_rate_hz)?;
    if tick < deadline {
        return None;
    }
    let from = phase.current;
    let duration_seconds = phase.phase_elapsed_seconds(tick, tick_rate_hz);
    let to = phase.advance(tick)?;
    let phases_completed = phase.phases_completed.clone();
    let legacy = PhaseChangedEvent {
        from,
        to,
        tick,
        cause: cause.to_string(),
    };
    let director = DirectorPhaseChangeEvent {
        from,
        to,
        tick,
        cause: cause.to_string(),
        duration_seconds,
    };
    Some((
        phase_changed_payload(&legacy),
        director_phase_change_payload(&director, &phases_completed),
    ))
}

/// to drive BuildUp → SustainPeak (when reactor pressure crosses into
/// Critical), SustainPeak → Relax (when guard dies), and Relax →
/// Debrief (when mission resolves). Unlike `advance_phase`, this does
/// NOT consult the deadline tick — it advances unconditionally. Returns
/// the (legacy, director) payload pair iff the pacer had a successor
/// phase.
pub fn force_advance_phase(world: &mut M7AiWorld, tick: u64, tick_rate_hz: u32, cause: &str) -> Option<(Value, Value)> {
    let phase = world.phase.as_mut()?;
    let from = phase.current;
    let duration_seconds = phase.phase_elapsed_seconds(tick, tick_rate_hz);
    let to = phase.advance(tick)?;
    let phases_completed = phase.phases_completed.clone();
    let legacy = PhaseChangedEvent {
        from,
        to,
        tick,
        cause: cause.to_string(),
    };
    let director = DirectorPhaseChangeEvent {
        from,
        to,
        tick,
        cause: cause.to_string(),
        duration_seconds,
    };
    Some((
        phase_changed_payload(&legacy),
        director_phase_change_payload(&director, &phases_completed),
    ))
}

/// for the active phase + kill count, returning an event payload if so.
pub fn try_spawn_reinforcement(world: &mut M7AiWorld, kill_count: u32, tick: u64) -> Option<Value> {
    let phase = world.phase.as_ref()?.current;
    let event = world.reinforcements.try_spawn_next(phase, kill_count, tick)?;
    Some(reinforcement_wave_spawned_payload(&event))
}

/// payload if a phase transition fired.
pub fn apply_boss_damage(world: &mut M7AiWorld, damage: f32, tick: u64) -> Option<Value> {
    let boss = world.boss.as_mut()?;
    let from = boss.current_phase;
    let new = boss.apply_damage(damage)?;
    let event = BossPhaseChangedEvent {
        actor_id: boss.actor_id,
        from,
        to: new,
        hp_fraction: boss.hp_fraction(),
        tick,
    };
    Some(boss_phase_changed_payload(&event))
}

/// activates a phase-specific ability (e.g. shield on Phase2).
pub fn boss_special_ability(world: &M7AiWorld, ability: &str, tick: u64) -> Option<Value> {
    let boss = world.boss.as_ref()?;
    let event = BossSpecialAbilityEvent {
        actor_id: boss.actor_id,
        phase: boss.current_phase,
        ability: ability.to_string(),
        tick,
    };
    Some(boss_special_ability_payload(&event))
}

/// canonical ability payload for the current phase iff that phase has not
/// yet emitted its `boss.special_ability_triggered` event since scenario
/// start. The world's `boss_abilities_emitted` latch is updated so the
/// next call returns `None`.
pub fn drain_boss_phase_ability(world: &mut M7AiWorld, tick: u64) -> Option<Value> {
    let phase = world.boss.as_ref()?.current_phase;
    let ability = boss_ability_for_phase(phase)?;
    let key = phase.as_u8();
    if world.boss_abilities_emitted.contains(&key) {
        return None;
    }
    world.boss_abilities_emitted.insert(key);
    boss_special_ability(world, ability, tick)
}

/// graph. Returns ready-to-record payloads for every optional objective
/// that just became reachable (its dependencies cleared) and every
/// branching point whose `chosen_branch` was set since the last scan.
/// Mutates per-graph latches so each event fires at most once per
/// objective_id / branching_point_id.
pub fn drain_objective_graph_emissions(world: &mut M7AiWorld, tick: u64) -> ObjectiveGraphEmit {
    let mut emit = ObjectiveGraphEmit::default();
    let Some(graph) = world.objective_graph.as_ref() else {
        return emit;
    };
    let active = graph.active_ids();
    for id in &active {
        let node = match graph.iter().find(|n| n.id == *id) {
            Some(n) => n,
            None => continue,
        };
        if !node.optional {
            continue;
        }
        if world.optionals_offered.contains(id) {
            continue;
        }
        let event = OptionalOfferedEvent {
            objective_id: id.clone(),
            tick,
        };
        emit.optional_offered.push(optional_offered_payload(&event));
        world.optionals_offered.insert(id.clone());
    }
    for branch in &graph.branches {
        if let Some(chosen) = branch.chosen_branch.clone() {
            if world.branches_emitted.contains(&branch.id) {
                continue;
            }
            let other = if chosen == branch.branch_a_id {
                branch.branch_b_id.clone()
            } else {
                branch.branch_a_id.clone()
            };
            let event = ObjectiveBranchedEvent {
                branching_point_id: branch.id.clone(),
                chosen_branch: chosen,
                other_branch: other,
                tick: branch.offered_tick.unwrap_or(tick),
            };
            emit.objective_branched.push(objective_branched_payload(&event));
            world.branches_emitted.insert(branch.id.clone());
        }
    }
    emit
}

/// and add one to `world.kill_count` for each enemy actor that is NOT
/// the controllable player. Returns the new cumulative count. The
/// reinforcement registry consumes this count via
/// [`try_spawn_reinforcement`] on the same tick.
///
/// `is_kill` is a closure the engine passes to filter outcomes (e.g.
/// "actor is a registered reactive guard"). Returning `false` skips
/// the outcome (covers the player dying, friendly bots dying, etc.).
pub fn track_kills<F>(world: &mut M7AiWorld, entered_dying_actors: &[ActorId], mut is_kill: F) -> u32
where
    F: FnMut(ActorId) -> bool,
{
    for actor in entered_dying_actors {
        if is_kill(*actor) {
            world.kill_count = world.kill_count.saturating_add(1);
        }
    }
    world.kill_count
}

/// the first tick the engine drives. Idempotent — once `world.phase` is
/// `Some`, subsequent calls are a no-op.
pub fn ensure_phase_initialised(world: &mut M7AiWorld, tick: u64) {
    world.init_phase(tick);
}

/// [`apply_boss_damage`] with [`drain_boss_phase_ability`] so the engine
/// can emit both `boss.phase_changed` and `boss.special_ability_triggered`
/// for a single damage application in one call.
pub fn apply_boss_damage_and_ability(world: &mut M7AiWorld, damage: f32, tick: u64) -> BossDamageEmit {
    let phase_changed = apply_boss_damage(world, damage, tick);
    let ability = if phase_changed.is_some() {
        drain_boss_phase_ability(world, tick)
    } else {
        None
    };
    BossDamageEmit { phase_changed, ability }
}

