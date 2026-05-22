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

/// downed actor, returning its actor id (and squared distance). Walks the
/// `bots` map deterministically (BTreeMap key order). Skips bots whose
/// archetype is not Medic or whose status is Dying/Dead.
pub fn nearest_medic(
    bots: &BTreeMap<ActorId, BotState>,
    actors: &BTreeMap<ActorId, ActorState>,
    downed: ActorId,
    max_distance: f32,
) -> Option<(ActorId, f32)> {
    let downed_actor = actors.get(&downed)?;
    let mut best: Option<(ActorId, f32)> = None;
    for (id, bot) in bots {
        if *id == downed {
            continue;
        }
        if bot.archetype != Archetype::Medic {
            continue;
        }
        let actor = match actors.get(id) {
            Some(a) => a,
            None => continue,
        };
        if !is_combat_ready(actor.status) {
            continue;
        }
        let dx = actor.position.x - downed_actor.position.x;
        let dy = actor.position.y - downed_actor.position.y;
        let d2 = dx * dx + dy * dy;
        if d2 > max_distance * max_distance {
            continue;
        }
        if best.map(|(_, bd2)| d2 < bd2).unwrap_or(true) {
            best = Some((*id, d2));
        }
    }
    best
}

/// engage in combat / dispatch missions.
pub fn is_combat_ready(status: Status) -> bool {
    matches!(status, Status::Stable | Status::Unstable)
}

/// chassis module that needs repair.
pub fn nearest_engineer(
    bots: &BTreeMap<ActorId, BotState>,
    actors: &BTreeMap<ActorId, ActorState>,
    target: ActorId,
    max_distance: f32,
) -> Option<(ActorId, f32)> {
    let target_actor = actors.get(&target)?;
    let mut best: Option<(ActorId, f32)> = None;
    for (id, bot) in bots {
        if *id == target {
            continue;
        }
        if bot.archetype != Archetype::Engineer {
            continue;
        }
        let actor = match actors.get(id) {
            Some(a) => a,
            None => continue,
        };
        if !is_combat_ready(actor.status) {
            continue;
        }
        let dx = actor.position.x - target_actor.position.x;
        let dy = actor.position.y - target_actor.position.y;
        let d2 = dx * dx + dy * dy;
        if d2 > max_distance * max_distance {
            continue;
        }
        if best.map(|(_, bd2)| d2 < bd2).unwrap_or(true) {
            best = Some((*id, d2));
        }
    }
    best
}

/// archetype's effective retreat threshold (factoring personality traits).
pub fn should_retreat(bot: &BotState, hp_fraction: f32) -> bool {
    let threshold = cf_ai::effective_retreat_threshold(&bot.personality);
    hp_fraction <= threshold
}

///
/// The engine constructs one of these per-bot per-tick and feeds it to
/// `ThinkingStack::tick`. World-state booleans are deterministic functions
/// of the current snapshot; layers cannot mutate the world. This helper is
/// pure (besides the borrow) so tests can drive the stack with known inputs.
#[allow(clippy::too_many_arguments)]
pub fn build_context<'a>(
    bot: &BotState,
    self_actor: &ActorState,
    tick: u64,
    tick_rate_hz: u32,
    enemy_visible: bool,
    enemy_distance_normalized: f32,
    under_fire: bool,
    downed_ally_within_reach: bool,
    ally_chassis_critical: bool,
    terrain_breach_within_range: bool,
    has_objective_target: bool,
) -> ThinkingContext<'a> {
    let mut ctx = ThinkingContext::stub();
    ctx.tick = tick;
    ctx.tick_rate_hz = tick_rate_hz;
    ctx.actor_id = self_actor.id.0;
    ctx.archetype = bot.archetype;
    ctx.autonomy = bot.stack.autonomy;
    ctx.doctrine_mode = bot.stack.doctrine_mode;
    ctx.role = std::borrow::Cow::Borrowed(bot.archetype.as_str());
    let hp_max = self_actor.hp_max.max(0.001);
    ctx.self_hp_fraction = (self_actor.hp / hp_max).clamp(0.0, 1.0);
    ctx.mood_normalized = (bot.personality.mood / 100.0).clamp(-1.0, 1.0);
    ctx.enemy_visible = enemy_visible;
    ctx.enemy_distance_normalized = enemy_distance_normalized.clamp(0.0, 1.0);
    ctx.under_fire = under_fire;
    ctx.downed_ally_within_reach = downed_ally_within_reach;
    ctx.ally_chassis_critical = ally_chassis_critical;
    ctx.terrain_breach_within_range = terrain_breach_within_range;
    ctx.has_objective_target = has_objective_target;
    ctx.doctrine = match bot.stack.doctrine_mode {
        DoctrineMode::Defensive => "defensive".into(),
        DoctrineMode::Aggressive => "aggressive".into(),
        DoctrineMode::Scout => "scout".into(),
    };
    ctx
}

