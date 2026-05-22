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

/// value into one of the four bands. Boundaries: ≥75 = Broken; ≥50 =
/// Depressed; ≥25 = Stressed; otherwise Calm.
pub fn stress_band_for(stress: f32) -> StressThreshold {
    if stress >= 75.0 {
        StressThreshold::Broken
    } else if stress >= 50.0 {
        StressThreshold::Depressed
    } else if stress >= 25.0 {
        StressThreshold::Stressed
    } else {
        StressThreshold::Calm
    }
}

/// actor for friendly-fire / kill-observer routing. Tracked bots carry
/// their faction directly; the player actor (identified by
/// `player_actor`) is always [`FactionId::Player`]. Returns `None` for
/// untracked, non-player actors (e.g. props).
pub fn faction_for_actor(world: &M7AiWorld, actor: ActorId, player_actor: Option<ActorId>) -> Option<FactionId> {
    if let Some(bot) = world.bots.get(&actor) {
        return Some(bot.faction);
    }
    if player_actor == Some(actor) {
        return Some(FactionId::Player);
    }
    None
}

/// and return the matching `ai.mood_changed` payload. The clamp to
/// `[-100, +100]` lives on [`PersonalityProfile::adjust_mood`]. Returns
/// `None` when the actor is not a tracked bot (e.g. the player or a
/// non-AI prop).
pub fn adjust_actor_mood(world: &mut M7AiWorld, actor: ActorId, delta: f32, cause: &str) -> Option<Value> {
    let bot = world.bots.get_mut(&actor)?;
    bot.personality.adjust_mood(delta);
    let new_mood = bot.personality.mood;
    Some(mood_changed_payload(actor.0, delta, new_mood, cause))
}

/// driven by sustained combat. Appends `current_tick` to the bot's
/// sliding window, trims entries older than
/// `SUSTAINED_COMBAT_WINDOW_SECONDS`, and (when the window just crossed
/// [`SUSTAINED_COMBAT_SHOT_COUNT`] and the sustained-combat latch is
/// open) pumps stress by [`STRESS_BAND_STEP`]. Returns a ready-to-record
/// `ai.stress_threshold_crossed` payload iff the pump moved the bot
/// into a higher band; otherwise `None`. The latch resets the next time
/// the window drops back below the shot threshold.
pub fn record_shot_for_stress(
    world: &mut M7AiWorld,
    actor: ActorId,
    current_tick: u64,
    tick_rate_hz: u32,
) -> Option<Value> {
    let bot = world.bots.get_mut(&actor)?;
    let rate = tick_rate_hz.max(1) as f32;
    let window_ticks = (SUSTAINED_COMBAT_WINDOW_SECONDS * rate).round() as u64;
    bot.recent_shot_ticks.push(current_tick);
    let cutoff = current_tick.saturating_sub(window_ticks);
    bot.recent_shot_ticks.retain(|t| *t >= cutoff);
    if bot.recent_shot_ticks.len() < SUSTAINED_COMBAT_SHOT_COUNT {
        bot.sustained_combat_latched = false;
        return None;
    }
    if bot.sustained_combat_latched {
        return None;
    }
    bot.sustained_combat_latched = true;
    let old_band = bot.last_stress_band;
    bot.personality.adjust_stress(STRESS_BAND_STEP);
    let new_band = stress_band_for(bot.personality.stress);
    if new_band == old_band {
        return None;
    }
    bot.last_stress_band = new_band;
    let stress_value = bot.personality.stress;
    Some(stress_threshold_crossed_payload(actor.0, new_band, true, stress_value))
}

/// delta to the world matrix and return a ready-to-record
/// `ai.faction_allegiance_changed` payload. Adjust is symmetric (per
/// [`FactionRelationships::adjust`]). Self-pairs are never adjusted
/// (allegiance(a, a) is the constant `+100`); the helper returns `None`
/// when `a == b`. `actual_delta` reflects post-clamp movement (so a
/// matrix already pinned at `+100` / `-100` reports `0`).
pub fn adjust_faction_relationships(
    world: &mut M7AiWorld,
    a: FactionId,
    b: FactionId,
    delta: i16,
    cause: &str,
) -> Option<Value> {
    if a == b {
        return None;
    }
    let old_value = world.factions.get(a, b);
    world.factions.adjust(a, b, delta);
    let new_value = world.factions.get(a, b);
    let actual_delta = new_value.saturating_sub(old_value);
    if actual_delta == 0 {
        return None;
    }
    Some(faction_allegiance_changed_payload(a, b, actual_delta, new_value, cause))
}

/// engine uses to decide whether a (shooter, target) hit counts as
/// friendly fire. Returns true when the two factions are the same, OR
/// when the current matrix entry between them is strictly positive
/// (i.e. they are allied). Self-pair `(a, a)` returns true (a bot
/// shooting another bot in its own faction is friendly fire).
pub fn is_friendly_fire(world: &M7AiWorld, shooter: FactionId, target: FactionId) -> bool {
    if shooter == target {
        return true;
    }
    world.factions.get(shooter, target) > 0
}

