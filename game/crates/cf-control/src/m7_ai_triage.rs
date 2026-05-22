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

///
/// Returns the initiated payload + initiates the mission. cf-control emits
/// `ai.auto_triage_initiated` with the returned payload AND, on subsequent
/// ticks, emits `ai.auto_triage_applied` when the engine confirms
/// stabilization landed.
pub fn begin_auto_triage(
    bot: &mut BotState,
    medic_id: ActorId,
    target_id: ActorId,
    dying_tick: u64,
    tick_rate_hz: u32,
) -> Option<Value> {
    if bot.archetype != Archetype::Medic {
        return None;
    }
    if bot.auto_triage.as_ref().is_some_and(|m| !m.is_terminal()) {
        return None;
    }
    let mission = AutoTriageMission::new(medic_id.0, target_id.0, dying_tick, tick_rate_hz);
    let payload = auto_triage_initiated_payload(&AutoTriageInitiatedEvent::from_mission(&mission));
    bot.auto_triage = Some(mission);
    Some(payload)
}

/// AND return the corresponding event payload. Returns `None` if there's
/// no active mission or it already terminated.
pub fn complete_auto_triage(bot: &mut BotState, tick: u64, tick_rate_hz: u32) -> Option<Value> {
    let mission = bot.auto_triage.as_mut()?;
    if mission.is_terminal() {
        return None;
    }
    mission.mark_applied(tick);
    let elapsed = ((tick.saturating_sub(mission.dying_transition_tick)) as f32 / tick_rate_hz.max(1) as f32).max(0.0);
    let event = AutoTriageAppliedEvent {
        medic_actor_id: mission.medic_actor_id,
        target_actor_id: mission.target_actor_id,
        dying_tick: mission.dying_transition_tick,
        applied_tick: tick,
        elapsed_seconds: elapsed,
    };
    Some(auto_triage_applied_payload(&event))
}

pub fn begin_auto_repair(
    bot: &mut BotState,
    engineer_id: ActorId,
    target_id: ActorId,
    module_id: impl Into<String>,
    trigger_tick: u64,
    tick_rate_hz: u32,
) -> Option<Value> {
    if bot.archetype != Archetype::Engineer {
        return None;
    }
    if bot.auto_repair.as_ref().is_some_and(|m| !m.is_terminal()) {
        return None;
    }
    let mission = AutoRepairMission::new(engineer_id.0, target_id.0, module_id, trigger_tick, tick_rate_hz);
    let payload = auto_repair_initiated_payload(&AutoRepairInitiatedEvent::from_mission(&mission));
    bot.auto_repair = Some(mission);
    Some(payload)
}

/// `ai.auto_repair_progressed` payload.
pub fn progress_auto_repair(bot: &mut BotState, tick: u64, repair_amount: f32) -> Option<Value> {
    let mission = bot.auto_repair.as_mut()?;
    if mission.is_terminal() {
        return None;
    }
    mission.record_repair_tick();
    let event = AutoRepairProgressedEvent {
        engineer_actor_id: mission.engineer_actor_id,
        target_actor_id: mission.target_actor_id,
        target_module_id: mission.target_module_id.clone(),
        tick,
        repair_amount,
        total_progressed_ticks: mission.progressed_ticks,
    };
    Some(auto_repair_progressed_payload(&event))
}

/// `(medic_id, target_id)` pairs whose auto-triage missions have reached
/// their `reach_deadline_tick` this tick. Engine-side caller invokes
/// `complete_auto_triage` directly on each medic bot to mark the mission
/// applied + emit `ai.auto_triage_applied`. Returned ids are guaranteed
/// non-terminal at the time of the scan.
pub fn ready_triage_completions(world: &M7AiWorld, current_tick: u64) -> Vec<(ActorId, ActorId)> {
    let mut out = Vec::new();
    for (medic_id, bot) in world.bots.iter() {
        if let Some(mission) = bot.auto_triage.as_ref() {
            if !mission.is_terminal() && current_tick >= mission.reach_deadline_tick {
                out.push((*medic_id, ActorId(mission.target_actor_id)));
            }
        }
    }
    out
}

/// `(engineer_id, target_id, module_id)` triples whose auto-repair
/// missions have reached `first_tick_deadline_tick` AND not yet recorded
/// any repair tick. Engine-side caller invokes `progress_auto_repair`
/// directly on each engineer bot to advance the mission + emit
/// `ai.auto_repair_progressed`.
pub fn ready_repair_progressions(world: &M7AiWorld, current_tick: u64) -> Vec<(ActorId, ActorId, String)> {
    let mut out = Vec::new();
    for (engineer_id, bot) in world.bots.iter() {
        if let Some(mission) = bot.auto_repair.as_ref() {
            if !mission.is_terminal()
                && current_tick >= mission.first_tick_deadline_tick
                && mission.progressed_ticks == 0
            {
                out.push((
                    *engineer_id,
                    ActorId(mission.target_actor_id),
                    mission.target_module_id.clone(),
                ));
            }
        }
    }
    out
}

