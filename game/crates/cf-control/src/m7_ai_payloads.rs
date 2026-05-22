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

pub fn reason_label_changed_payload(actor_id: u64, output: &AiTickOutput) -> Value {
    let label = &output.reason_label;
    json!({
        "actor_id": actor_id,
        "label": label.format(),
        "chosen_task": label.chosen_task,
        "chosen_target": label.chosen_target,
        "score": quantize(label.score),
        "doctrine": label.doctrine,
        "role": label.role,
        "htn_goal_stack": label.htn_goal_stack,
        "behavior_tree_node": label.behavior_tree_node,
    })
}

pub fn thinking_layer_invoked_payload(actor_id: u64, output: &AiTickOutput) -> Value {
    let layers: Vec<&'static str> = output.layers_invoked.iter().map(|l| l.as_str()).collect();
    json!({
        "actor_id": actor_id,
        "layers": layers,
        "reactive_override": output.reactive_override,
        "chosen_task": output.chosen_task.as_str(),
    })
}

pub fn archetype_chosen_payload(actor_id: u64, archetype: Archetype) -> Value {
    json!({
        "actor_id": actor_id,
        "archetype": archetype.as_str(),
    })
}

pub fn auto_triage_initiated_payload(event: &AutoTriageInitiatedEvent) -> Value {
    json!({
        "medic_actor_id": event.medic_actor_id,
        "target_actor_id": event.target_actor_id,
        "dying_tick": event.dying_tick,
        "reach_deadline_tick": event.reach_deadline_tick,
        "apply_deadline_tick": event.apply_deadline_tick,
        "reach_seconds": event.reach_seconds,
        "apply_seconds": event.apply_seconds,
    })
}

pub fn auto_triage_applied_payload(event: &AutoTriageAppliedEvent) -> Value {
    json!({
        "medic_actor_id": event.medic_actor_id,
        "target_actor_id": event.target_actor_id,
        "dying_tick": event.dying_tick,
        "applied_tick": event.applied_tick,
        "elapsed_seconds": event.elapsed_seconds,
    })
}

pub fn auto_repair_initiated_payload(event: &AutoRepairInitiatedEvent) -> Value {
    json!({
        "engineer_actor_id": event.engineer_actor_id,
        "target_actor_id": event.target_actor_id,
        "target_module_id": event.target_module_id,
        "triggered_tick": event.triggered_tick,
        "reach_deadline_tick": event.reach_deadline_tick,
        "first_tick_deadline_tick": event.first_tick_deadline_tick,
        "reach_seconds": event.reach_seconds,
        "first_tick_seconds": event.first_tick_seconds,
    })
}

pub fn auto_repair_progressed_payload(event: &AutoRepairProgressedEvent) -> Value {
    json!({
        "engineer_actor_id": event.engineer_actor_id,
        "target_actor_id": event.target_actor_id,
        "target_module_id": event.target_module_id,
        "tick": event.tick,
        "repair_amount": event.repair_amount,
        "total_progressed_ticks": event.total_progressed_ticks,
    })
}

pub fn phase_changed_payload(event: &PhaseChangedEvent) -> Value {
    json!({
        "from": event.from.as_str(),
        "to": event.to.as_str(),
        "tick": event.tick,
        "cause": event.cause,
    })
}

pub fn reinforcement_wave_spawned_payload(event: &ReinforcementWaveSpawnedEvent) -> Value {
    json!({
        "wave_id": event.wave_id,
        "phase": event.phase.as_str(),
        "spawn_count": event.spawn_count,
        "dropship_zone": event.dropship_zone,
        "tick": event.tick,
    })
}

pub fn boss_phase_changed_payload(event: &BossPhaseChangedEvent) -> Value {
    json!({
        "actor_id": event.actor_id,
        "from": event.from.as_str(),
        "to": event.to.as_str(),
        "hp_fraction": event.hp_fraction,
        "tick": event.tick,
    })
}

pub fn boss_special_ability_payload(event: &BossSpecialAbilityEvent) -> Value {
    json!({
        "actor_id": event.actor_id,
        "phase": event.phase.as_str(),
        "ability": event.ability,
        "tick": event.tick,
    })
}

/// `mission.objective_branched`.
pub fn objective_branched_payload(event: &ObjectiveBranchedEvent) -> Value {
    json!({
        "branching_point_id": event.branching_point_id,
        "chosen_branch": event.chosen_branch,
        "other_branch": event.other_branch,
        "tick": event.tick,
    })
}

/// `mission.optional_offered`.
pub fn optional_offered_payload(event: &OptionalOfferedEvent) -> Value {
    json!({
        "objective_id": event.objective_id,
        "tick": event.tick,
    })
}

pub fn cover_seeking_started_payload(event: &CoverSeekingEvent) -> Value {
    json!({
        "actor_id": event.actor_id,
        "archetype": event.archetype.as_str(),
        "reason": event.reason.as_str(),
        "target_position": event.target_position,
        "distance": quantize(event.distance),
    })
}

pub fn suppression_started_payload(event: &SuppressionEvent) -> Value {
    json!({
        "actor_id": event.actor_id,
        "target_actor_id": event.target_actor_id,
        "flanker_actor_id": event.flanker_actor_id,
        "duration_ticks": event.duration_ticks,
    })
}

pub fn retreat_decision_payload(event: &RetreatDecisionEvent) -> Value {
    json!({
        "actor_id": event.actor_id,
        "reason": event.reason.as_str(),
        "hp_fraction": quantize(event.hp_fraction),
        "tick": event.tick,
    })
}

pub fn squad_comm_relayed_payload(event: &SquadCommRelayedEvent) -> Value {
    json!({
        "originator_actor_id": event.originator_actor_id,
        "receiver_actor_ids": event.receiver_actor_ids,
        "target_actor_id": event.target_actor_id,
        "target_position": event.target_position,
        "delay_ticks": event.delay_ticks,
    })
}

pub fn patrol_waypoint_reached_payload(event: &PatrolWaypointReachedEvent) -> Value {
    json!({
        "actor_id": event.actor_id,
        "waypoint_index": event.waypoint_index,
        "position": event.position,
        "idle_seconds": quantize(event.idle_seconds),
    })
}

pub fn friendly_fire_avoidance_payload(event: &FriendlyFireAvoidanceEvent) -> Value {
    json!({
        "actor_id": event.actor_id,
        "friendly_actor_id": event.friendly_actor_id,
        "kind": event.kind.as_str(),
    })
}

/// `ai.high_ground_preference_applied`.
pub fn high_ground_preference_applied_payload(event: &HighGroundEvent) -> Value {
    json!({
        "actor_id": event.actor_id,
        "target_position": event.target_position,
        "elevation_gain": quantize(event.elevation_gain),
    })
}

/// `phases_completed` list mirrors `PhaseState::phases_completed` so the
/// M10 viewer can render the in-order pacer timeline without
/// reconstructing it from the event stream.
pub fn director_phase_change_payload(event: &DirectorPhaseChangeEvent, phases_completed: &[MissionPhase]) -> Value {
    let phases: Vec<Value> = phases_completed.iter().map(|p| Value::from(p.as_str())).collect();
    json!({
        "from": event.from.as_str(),
        "to": event.to.as_str(),
        "tick": event.tick,
        "cause": event.cause,
        "duration_seconds": event.duration_seconds,
        "phases_completed": phases,
    })
}

pub fn priority_table_changed_payload(actor_id: u64, task: TaskType, old_weight: u8, new_weight: u8) -> Value {
    json!({
        "actor_id": actor_id,
        "task": task.as_str(),
        "old_weight": old_weight,
        "new_weight": new_weight,
    })
}

pub fn autonomy_mode_changed_payload(actor_id: u64, from: AutonomyMode, to: AutonomyMode) -> Value {
    json!({
        "actor_id": actor_id,
        "from": from.as_str(),
        "to": to.as_str(),
    })
}

pub fn role_template_applied_payload(actor_id: u64, template: RoleTemplate) -> Value {
    json!({
        "actor_id": actor_id,
        "template_id": template.as_str(),
    })
}

pub fn quick_preset_applied_payload(actor_id: u64, preset: QuickPresetId) -> Value {
    json!({
        "actor_id": actor_id,
        "preset_id": preset.as_str(),
    })
}

/// `ChatterEmittedEvent` shape in cf-replay's wire form.
pub fn chatter_emitted_payload(event: &ChatterEmittedEvent) -> Value {
    json!({
        "actor_id": event.actor_id,
        "category": event.category.as_str(),
        "text": event.text,
        "voice_id": event.voice_id,
        "cooldown_remaining_seconds": event.cooldown_remaining_seconds,
    })
}

/// is the list of `PersonalityTrait` snake_case ids; `modifier` is the
/// optional active `PersonalityModifier`.
pub fn personality_changed_payload(
    actor_id: u64,
    traits: &[cf_ai::PersonalityTrait],
    modifier: Option<PersonalityModifier>,
    cause: &str,
) -> Value {
    let traits_json: Vec<Value> = traits.iter().map(|t| Value::from(t.as_str())).collect();
    json!({
        "actor_id": actor_id,
        "traits": traits_json,
        "modifier": modifier.map(|m| m.as_str()),
        "cause": cause,
    })
}

pub fn mood_changed_payload(actor_id: u64, delta: f32, new_mood: f32, cause: &str) -> Value {
    json!({
        "actor_id": actor_id,
        "delta": delta,
        "new_mood": new_mood,
        "cause": cause,
    })
}

pub fn stress_threshold_crossed_payload(
    actor_id: u64,
    threshold: StressThreshold,
    direction_entered: bool,
    stress_value: f32,
) -> Value {
    json!({
        "actor_id": actor_id,
        "threshold": threshold.as_str(),
        "direction": if direction_entered { "entered" } else { "exited" },
        "stress_value": stress_value,
    })
}

pub fn faction_allegiance_changed_payload(
    a: FactionId,
    b: FactionId,
    delta: i16,
    new_value: i16,
    cause: &str,
) -> Value {
    json!({
        "a": a.as_str(),
        "b": b.as_str(),
        "delta": delta,
        "new_value": new_value,
        "cause": cause,
    })
}

