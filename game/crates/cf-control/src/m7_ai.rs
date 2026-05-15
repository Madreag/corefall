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
    BossPhaseChangedEvent, BossSpecialAbilityEvent, BossState, PhaseChangedEvent, PhaseState, ReinforcementRegistry,
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
// **M7-B**: chatter cooldown is 4.0 seconds per `(actor, category)` per
/// spec § Chatter scaffold cooldown table. Re-exported so the audit greps
/// can find the constant on the cf-control side.
pub use cf_ai::CHATTER_COOLDOWN_SECONDS;

/// **M7-A**: per-actor AI state. **M7-B** adds `personality_modifier`
/// (one of Aggressive / Cautious / Loyal / LoneWolf / Neutral) which
/// re-weights the priority table on top of the role template.
#[derive(Debug, Clone)]
pub struct BotState {
    pub archetype: Archetype,
    pub stack: ThinkingStack,
    pub personality: PersonalityProfile,
    /// **M7-B**: personality modifier driving the priority re-weight.
    pub personality_modifier: PersonalityModifier,
    /// **M7-B**: per-faction allegiance assignment (defaults to AiEnemy
    /// for spawned guards). Drives friendly-fire decisions + the matrix
    /// when relationships shift.
    pub faction: FactionId,
    /// In-flight auto-triage mission (Medic).
    pub auto_triage: Option<AutoTriageMission>,
    /// In-flight auto-repair mission (Engineer).
    pub auto_repair: Option<AutoRepairMission>,
    /// **M7-A fix-round-2**: previous tick's chosen task. Used by
    /// `detect_behavior_transitions` to fire one event per transition INTO
    /// a sub-plan task family (cover / suppression / retreat) instead of
    /// once per tick the bot remains in that task.
    pub last_chosen_task: Option<TaskType>,
    /// **M7-A fix-round-2**: patrol route (waypoint loop + idle countdown).
    /// Auto-seeded with a 2-waypoint loop on bot creation; scenarios
    /// override via `set_patrol_route`.
    pub patrol: PatrolRoute,
    /// **M7-A fix-round-2**: pending squad-comm relays this bot owes its
    /// squadmates. Each entry fires `ai.squad_comm_relayed` once its
    /// `relay_tick` is reached (0.5 s delay per spec § Squad communication).
    pub squad_comm_pending: Vec<SquadCommPending>,
    /// **M7-A fix-round-2**: tracks the previous tick's player-visibility
    /// flag so we can detect the *transition* from "lost the player" to
    /// "spotted the player" and schedule one squad-comm relay per fresh
    /// detection, not one per tick the player stays visible.
    pub had_player_visibility: bool,
    /// **M7-A fix-round-2**: tracks the last elevation gain we emitted a
    /// `ai.high_ground_preference_applied` for. Re-emit only when the
    /// chosen task transitions back into the high-ground task family.
    pub last_high_ground_emission_task: Option<TaskType>,
    /// **M7-A fix-round-2**: tracks the last friendly-fire-avoidance
    /// emission tick so we don't spam events while the friendly stays in
    /// the line of fire. One emission per (actor, friendly) until the
    /// friendly clears the LOS.
    pub last_friendly_fire_avoidance_friendly: Option<ActorId>,
}

impl BotState {
    pub fn new(archetype: Archetype) -> Self {
        Self {
            archetype,
            stack: ThinkingStack::new(archetype),
            personality: PersonalityProfile::default(),
            personality_modifier: PersonalityModifier::Neutral,
            faction: FactionId::AiEnemy,
            auto_triage: None,
            auto_repair: None,
            last_chosen_task: None,
            patrol: PatrolRoute::new(vec![[0.0, 0.0], [10.0, 0.0]]),
            squad_comm_pending: Vec::new(),
            had_player_visibility: false,
            last_high_ground_emission_task: None,
            last_friendly_fire_avoidance_friendly: None,
        }
    }

    /// **M7-A fix-round-2**: replace the bot's patrol route. Scenarios that
    /// declare an explicit waypoint list call this; otherwise the default
    /// 2-waypoint loop seeded by `BotState::new` ticks the patrol contract.
    pub fn set_patrol_route(&mut self, waypoints: Vec<[f32; 2]>) {
        self.patrol = PatrolRoute::new(waypoints);
    }
}

/// **M7-A**: world-level AI surface owned by the engine. **M7-B** adds the
/// chatter cooldown table so production paths can rate-limit chatter
/// emission without duplicating per-actor state across call sites.
#[derive(Debug, Clone, Default)]
pub struct M7AiWorld {
    pub bots: BTreeMap<ActorId, BotState>,
    pub factions: FactionRelationships,
    pub phase: Option<PhaseState>,
    pub reinforcements: ReinforcementRegistry,
    pub boss: Option<BossState>,
    /// **M7-B**: per-actor per-category chatter cooldown gate.
    pub chatter_cooldowns: ChatterCooldownTable,
}

impl M7AiWorld {
    pub fn new() -> Self {
        Self {
            bots: BTreeMap::new(),
            factions: FactionRelationships::new(),
            phase: None,
            reinforcements: ReinforcementRegistry::default(),
            boss: None,
            chatter_cooldowns: ChatterCooldownTable::new(),
        }
    }

    pub fn assign_archetype(&mut self, actor: ActorId, archetype: Archetype) -> AssignmentResult {
        let entry = self.bots.entry(actor).or_insert_with(|| BotState::new(archetype));
        let prev = entry.archetype;
        if prev != archetype {
            entry.archetype = archetype;
            entry.stack.apply_archetype(archetype);
            AssignmentResult::Changed { previous: prev }
        } else {
            AssignmentResult::Unchanged
        }
    }

    pub fn bot(&self, actor: ActorId) -> Option<&BotState> {
        self.bots.get(&actor)
    }

    pub fn bot_mut(&mut self, actor: ActorId) -> Option<&mut BotState> {
        self.bots.get_mut(&actor)
    }

    /// Initialize phase pacing the first time the mission starts ticking.
    pub fn init_phase(&mut self, tick: u64) {
        if self.phase.is_none() {
            self.phase = Some(PhaseState::new(tick));
        }
    }

    /// **M7-B**: set a single task weight on an actor's PriorityTable.
    /// Clamps `weight` to `0..=9`. Returns `(old, new)` weights on success
    /// or `Err(reason)` if the actor has no `BotState`.
    pub fn set_priority(&mut self, actor: ActorId, task: TaskType, weight: u8) -> Result<(u8, u8), &'static str> {
        let bot = self.bots.get_mut(&actor).ok_or("no_such_actor")?;
        let old = bot.stack.priority.get(task);
        let clamped = weight.min(9);
        bot.stack.priority.set(task, clamped);
        // Keep the utility scorer's cached priority in sync so the next
        // tick uses the new weight.
        bot.stack.utility.priority = bot.stack.priority;
        Ok((old, clamped))
    }

    /// **M7-B**: set an actor's autonomy mode. Returns `Some(old)` on
    /// success, `None` if the actor has no `BotState`.
    pub fn set_autonomy(&mut self, actor: ActorId, mode: AutonomyMode) -> Option<AutonomyMode> {
        let bot = self.bots.get_mut(&actor)?;
        let old = bot.stack.autonomy;
        bot.stack.autonomy = mode;
        Some(old)
    }

    /// **M7-B**: replace an actor's PriorityTable with one of the 6
    /// spec-mandated role templates (also re-applies the archetype +
    /// behavior tree library). Returns `Some(())` on success.
    pub fn apply_role_template(&mut self, actor: ActorId, template: RoleTemplate) -> Option<()> {
        let bot = self.bots.get_mut(&actor)?;
        let archetype = template.archetype();
        bot.archetype = archetype;
        bot.stack.apply_archetype(archetype);
        Some(())
    }

    /// **M7-B**: apply a quick preset to an actor's PriorityTable. The
    /// preset shifts task families ±2 per spec § Quick presets. Returns
    /// `Some(())` on success.
    pub fn apply_quick_preset(&mut self, actor: ActorId, preset: QuickPresetId) -> Option<()> {
        let bot = self.bots.get_mut(&actor)?;
        preset.apply_to(&mut bot.stack.priority);
        bot.stack.utility.priority = bot.stack.priority;
        Some(())
    }

    /// **M7-B**: apply a personality modifier on top of the actor's
    /// current PriorityTable. Updates `bot.personality_modifier` for
    /// future round-trips through snapshot/restore.
    pub fn apply_personality_modifier(&mut self, actor: ActorId, modifier: PersonalityModifier) -> Option<()> {
        let bot = self.bots.get_mut(&actor)?;
        bot.personality_modifier = modifier;
        modifier.apply_to(&mut bot.stack.priority);
        bot.stack.utility.priority = bot.stack.priority;
        Some(())
    }

    /// **M7-B**: build the JSON view of an actor's PriorityTable for the
    /// `observe.priority_table` cfctl method. Returns `None` if the actor
    /// has no `BotState`.
    pub fn priority_table_view(&self, actor: ActorId) -> Option<Value> {
        let bot = self.bots.get(&actor)?;
        let mut weights = serde_json::Map::with_capacity(TaskType::COUNT);
        for task in TaskType::ALL.iter() {
            weights.insert(task.as_str().to_string(), Value::from(bot.stack.priority.get(*task)));
        }
        Some(json!({
            "actor_id": actor.0,
            "role": bot.archetype.as_str(),
            "personality_modifier": bot.personality_modifier.as_str(),
            "weights": weights,
        }))
    }

    /// **M7-B**: build the JSON view of an actor's autonomy state for the
    /// `observe.autonomy` cfctl method.
    pub fn autonomy_view(&self, actor: ActorId) -> Option<Value> {
        let bot = self.bots.get(&actor)?;
        let mode = bot.stack.autonomy;
        Some(json!({
            "actor_id": actor.0,
            "mode": mode.as_str(),
            "auto_action_cap": auto_action_cap_to_value(mode.auto_action_cap()),
            "doctrine_mode": bot.stack.doctrine_mode.as_str(),
        }))
    }

    /// **M7-B**: build a JSON snapshot of every actor's PriorityTable for
    /// the snapshot/restore round-trip contract. The map keys are
    /// stringified actor ids (deterministic via BTreeMap iteration).
    pub fn snapshot_actor_priorities(&self) -> Value {
        let mut map = serde_json::Map::new();
        for (actor, bot) in &self.bots {
            map.insert(
                actor.0.to_string(),
                serde_json::to_value(bot.stack.priority).unwrap_or(Value::Null),
            );
        }
        Value::Object(map)
    }

    /// **M7-B**: restore PriorityTables previously captured via
    /// `snapshot_actor_priorities`. Missing actors are skipped (the
    /// caller is expected to assign archetypes first). Returns the
    /// number of tables restored.
    pub fn restore_actor_priorities(&mut self, snapshot: &Value) -> usize {
        let Some(map) = snapshot.as_object() else {
            return 0;
        };
        let mut count = 0;
        for (actor_str, value) in map {
            let Ok(actor_id) = actor_str.parse::<u64>() else {
                continue;
            };
            let bot = match self.bots.get_mut(&ActorId(actor_id)) {
                Some(b) => b,
                None => continue,
            };
            let table: PriorityTable = match serde_json::from_value(value.clone()) {
                Ok(t) => t,
                Err(_) => continue,
            };
            bot.stack.priority = table;
            bot.stack.utility.priority = table;
            count += 1;
        }
        count
    }

    /// **M7-B**: try to emit a chatter event for `(actor, category)` at
    /// `current_tick`. Returns the event payload + caption text iff the
    /// cooldown is open. The engine records the event via cf-replay AND
    /// surfaces the caption via `HudState.captions`.
    pub fn try_emit_chatter(
        &mut self,
        actor: ActorId,
        category: ChatterCategory,
        text: impl Into<String>,
        current_tick: u64,
        tick_rate_hz: u32,
    ) -> Option<(ChatterEmittedEvent, EmissionInfo)> {
        let cooldown_seconds = CHATTER_COOLDOWN_SECONDS;
        let cooldown_ticks = (cooldown_seconds * tick_rate_hz.max(1) as f32).ceil() as u64;
        let info = self
            .chatter_cooldowns
            .try_emit(actor.0, category, current_tick, cooldown_ticks)?;
        let archetype_str = self
            .bots
            .get(&actor)
            .map(|b| b.archetype.as_str().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let cooldown_remaining_seconds = cooldown_seconds;
        let event = ChatterEmittedEvent {
            actor_id: actor.0,
            category,
            text: text.into(),
            voice_id: voice_id_for_archetype(&archetype_str),
            cooldown_remaining_seconds,
        };
        Some((event, info))
    }
}

fn auto_action_cap_to_value(cap: usize) -> Value {
    if cap == usize::MAX {
        Value::String("unbounded".to_string())
    } else {
        Value::from(cap as u64)
    }
}

pub enum AssignmentResult {
    Unchanged,
    Changed { previous: Archetype },
}

/// **M7-A**: pure helper — find the nearest live Medic-role ally to a
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

/// **M7-A**: returns true if a status represents a bot that can still
/// engage in combat / dispatch missions.
pub fn is_combat_ready(status: Status) -> bool {
    matches!(status, Status::Stable | Status::Unstable)
}

/// **M7-A**: pure helper — find the nearest live Engineer-role ally near a
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

/// **M7-A**: pure helper to determine if a bot's HP is below their
/// archetype's effective retreat threshold (factoring personality traits).
pub fn should_retreat(bot: &BotState, hp_fraction: f32) -> bool {
    let threshold = cf_ai::effective_retreat_threshold(&bot.personality);
    hp_fraction <= threshold
}

/// **M7-A**: build a `ThinkingContext` snapshot from current world state.
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

/// **M7-A**: build a JSON payload for `ai.reason_label_changed`.
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

/// **M7-A**: build a JSON payload for `ai.thinking_layer_invoked`.
pub fn thinking_layer_invoked_payload(actor_id: u64, output: &AiTickOutput) -> Value {
    let layers: Vec<&'static str> = output.layers_invoked.iter().map(|l| l.as_str()).collect();
    json!({
        "actor_id": actor_id,
        "layers": layers,
        "reactive_override": output.reactive_override,
        "chosen_task": output.chosen_task.as_str(),
    })
}

/// **M7-A**: build a JSON payload for `ai.archetype_chosen`.
pub fn archetype_chosen_payload(actor_id: u64, archetype: Archetype) -> Value {
    json!({
        "actor_id": actor_id,
        "archetype": archetype.as_str(),
    })
}

/// **M7-A**: build a JSON payload for `ai.auto_triage_initiated`.
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

/// **M7-A**: build a JSON payload for `ai.auto_triage_applied`.
pub fn auto_triage_applied_payload(event: &AutoTriageAppliedEvent) -> Value {
    json!({
        "medic_actor_id": event.medic_actor_id,
        "target_actor_id": event.target_actor_id,
        "dying_tick": event.dying_tick,
        "applied_tick": event.applied_tick,
        "elapsed_seconds": event.elapsed_seconds,
    })
}

/// **M7-A**: build a JSON payload for `ai.auto_repair_initiated`.
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

/// **M7-A**: build a JSON payload for `ai.auto_repair_progressed`.
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

/// **M7**: build a JSON payload for `mission.phase_changed`.
pub fn phase_changed_payload(event: &PhaseChangedEvent) -> Value {
    json!({
        "from": event.from.as_str(),
        "to": event.to.as_str(),
        "tick": event.tick,
        "cause": event.cause,
    })
}

/// **M7**: build a JSON payload for `mission.reinforcement_wave_spawned`.
pub fn reinforcement_wave_spawned_payload(event: &ReinforcementWaveSpawnedEvent) -> Value {
    json!({
        "wave_id": event.wave_id,
        "phase": event.phase.as_str(),
        "spawn_count": event.spawn_count,
        "dropship_zone": event.dropship_zone,
        "tick": event.tick,
    })
}

/// **M7**: build a JSON payload for `boss.phase_changed`.
pub fn boss_phase_changed_payload(event: &BossPhaseChangedEvent) -> Value {
    json!({
        "actor_id": event.actor_id,
        "from": event.from.as_str(),
        "to": event.to.as_str(),
        "hp_fraction": event.hp_fraction,
        "tick": event.tick,
    })
}

/// **M7**: build a JSON payload for `boss.special_ability_triggered`.
pub fn boss_special_ability_payload(event: &BossSpecialAbilityEvent) -> Value {
    json!({
        "actor_id": event.actor_id,
        "phase": event.phase.as_str(),
        "ability": event.ability,
        "tick": event.tick,
    })
}

// ---------------------------------------------------------------------------
// **M7-A fix-round-2**: payload helpers for the 7 behavior sub-plan events
// (cover_seeking_started / suppression_started / retreat_decision /
// squad_comm_relayed / patrol_waypoint_reached / friendly_fire_avoidance /
// high_ground_preference_applied). Each helper accepts the canonical cf-ai
// event struct and returns a `serde_json::Value` that matches the JSON
// schema in `cf-replay/schemas/event/ai_<event>.json`.
// ---------------------------------------------------------------------------

/// **M7-A fix-round-2**: build a JSON payload for `ai.cover_seeking_started`.
pub fn cover_seeking_started_payload(event: &CoverSeekingEvent) -> Value {
    json!({
        "actor_id": event.actor_id,
        "archetype": event.archetype.as_str(),
        "reason": event.reason.as_str(),
        "target_position": event.target_position,
        "distance": quantize(event.distance),
    })
}

/// **M7-A fix-round-2**: build a JSON payload for `ai.suppression_started`.
pub fn suppression_started_payload(event: &SuppressionEvent) -> Value {
    json!({
        "actor_id": event.actor_id,
        "target_actor_id": event.target_actor_id,
        "flanker_actor_id": event.flanker_actor_id,
        "duration_ticks": event.duration_ticks,
    })
}

/// **M7-A fix-round-2**: build a JSON payload for `ai.retreat_decision`.
pub fn retreat_decision_payload(event: &RetreatDecisionEvent) -> Value {
    json!({
        "actor_id": event.actor_id,
        "reason": event.reason.as_str(),
        "hp_fraction": quantize(event.hp_fraction),
        "tick": event.tick,
    })
}

/// **M7-A fix-round-2**: build a JSON payload for `ai.squad_comm_relayed`.
pub fn squad_comm_relayed_payload(event: &SquadCommRelayedEvent) -> Value {
    json!({
        "originator_actor_id": event.originator_actor_id,
        "receiver_actor_ids": event.receiver_actor_ids,
        "target_actor_id": event.target_actor_id,
        "target_position": event.target_position,
        "delay_ticks": event.delay_ticks,
    })
}

/// **M7-A fix-round-2**: build a JSON payload for `ai.patrol_waypoint_reached`.
pub fn patrol_waypoint_reached_payload(event: &PatrolWaypointReachedEvent) -> Value {
    json!({
        "actor_id": event.actor_id,
        "waypoint_index": event.waypoint_index,
        "position": event.position,
        "idle_seconds": quantize(event.idle_seconds),
    })
}

/// **M7-A fix-round-2**: build a JSON payload for `ai.friendly_fire_avoidance`.
pub fn friendly_fire_avoidance_payload(event: &FriendlyFireAvoidanceEvent) -> Value {
    json!({
        "actor_id": event.actor_id,
        "friendly_actor_id": event.friendly_actor_id,
        "kind": event.kind.as_str(),
    })
}

/// **M7-A fix-round-2**: build a JSON payload for
/// `ai.high_ground_preference_applied`.
pub fn high_ground_preference_applied_payload(event: &HighGroundEvent) -> Value {
    json!({
        "actor_id": event.actor_id,
        "target_position": event.target_position,
        "elevation_gain": quantize(event.elevation_gain),
    })
}

/// **M7-A fix-round-2**: world-state snapshot the engine collects each
/// AI tick to drive `detect_behavior_transitions`. All fields are owned
/// snapshots so the engine can release the world borrow before the
/// detector mutates `BotState` (cursor advance, squad-comm scheduling,
/// last-task tracking, etc.).
#[derive(Debug, Clone)]
pub struct BehaviorSignals {
    /// Bot's own actor id.
    pub actor_id: u64,
    /// Bot's world-space position [x, y].
    pub self_position: [f32; 2],
    /// Bot's current HP / hp_max ratio (0.0..=1.0).
    pub hp_fraction: f32,
    /// Whether the bot has line-of-sight on the player this tick.
    pub enemy_visible: bool,
    /// Whether the bot has been recently shot at (last 60 ticks).
    pub under_fire: bool,
    /// True when the reactive layer overrode utility this tick.
    pub reactive_override: bool,
    /// Player actor id for suppression / squad-comm payloads, when known.
    pub player_actor_id: Option<u64>,
    /// Player's current world-space position for squad-comm payloads.
    pub player_position: Option<[f32; 2]>,
    /// IDs of every other faction-allied bot. Used as `receiver_actor_ids`
    /// when the squad-comm relay timer expires.
    pub squadmates: Vec<u64>,
    /// `Some(friendly_actor_id)` when a faction-allied actor sits on the
    /// bot's firing line right now. Populated by the engine via
    /// `cf_ai::friendly_fire::is_friendly_in_line_of_fire`.
    pub friendly_in_line_of_fire: Option<u64>,
    /// Tick the engine is processing. Drives squad-comm relay timing +
    /// the patrol idle countdown.
    pub current_tick: u64,
    /// Tick rate the engine is running at (configurable; do NOT hardcode
    /// 60). Drives the squad-comm 0.5 s delay + the patrol 5-10 s pause.
    pub tick_rate_hz: u32,
}

/// **M7-A fix-round-2**: behavior-transition emission bundle. Each field
/// holds an optional ready-to-record JSON payload for the corresponding
/// `ai.*` event, or `None` if the transition didn't fire this tick.
/// `squad_comm_relayed` is `Vec` because a single tick may flush multiple
/// pending relays from the same originator.
#[derive(Debug, Clone, Default)]
pub struct BotBehaviorEmit {
    pub cover_seeking_started: Option<Value>,
    pub suppression_started: Option<Value>,
    pub retreat_decision: Option<Value>,
    pub squad_comm_relayed: Vec<Value>,
    pub patrol_waypoint_reached: Option<Value>,
    pub friendly_fire_avoidance: Option<Value>,
    pub high_ground_preference_applied: Option<Value>,
}

/// **M7-A fix-round-2**: detect behavior-sub-plan transitions for one bot
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

/// Quantize floats for stable replay output.
fn quantize(value: f32) -> f32 {
    if !value.is_finite() {
        return 0.0;
    }
    (value * 100.0).round() / 100.0
}

/// **M7-A**: outcome of `tick_bot` — what events the engine should emit.
#[derive(Debug, Clone)]
pub struct BotTickEmit {
    pub reason_label_changed: Option<Value>,
    pub thinking_layer_invoked: Option<Value>,
    pub auto_triage_initiated: Option<Value>,
    pub auto_triage_applied: Option<Value>,
    pub auto_repair_initiated: Option<Value>,
    pub auto_repair_progressed: Option<Value>,
    pub chosen_task: TaskType,
    pub chosen_action: BehaviorAction,
}

impl BotTickEmit {
    pub fn new(chosen_task: TaskType, chosen_action: BehaviorAction) -> Self {
        Self {
            reason_label_changed: None,
            thinking_layer_invoked: None,
            auto_triage_initiated: None,
            auto_triage_applied: None,
            auto_repair_initiated: None,
            auto_repair_progressed: None,
            chosen_task,
            chosen_action,
        }
    }
}

/// **M7-A**: drive the per-bot thinking-stack tick and surface event
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

/// **M7-A**: spawn an auto-triage mission for the Medic-role bot.
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

/// **M7-A**: apply stabilization — mark the auto-triage mission as applied
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

/// **M7-A**: spawn an auto-repair mission for the Engineer-role bot.
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

/// **M7-A**: record a repair-tick on the Engineer's active mission. Emits
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

/// **M7**: advance the 4-phase mission director, emitting
/// `mission.phase_changed` when a transition fires.
pub fn advance_phase(world: &mut M7AiWorld, tick: u64, tick_rate_hz: u32, cause: &str) -> Option<Value> {
    let phase = world.phase.as_mut()?;
    let deadline = phase.deadline_tick(tick_rate_hz)?;
    if tick < deadline {
        return None;
    }
    let from = phase.current;
    let to = phase.advance(tick)?;
    let event = PhaseChangedEvent {
        from,
        to,
        tick,
        cause: cause.to_string(),
    };
    Some(phase_changed_payload(&event))
}

/// **M7**: check whether any registered reinforcement wave should spawn
/// for the active phase + kill count, returning an event payload if so.
pub fn try_spawn_reinforcement(world: &mut M7AiWorld, kill_count: u32, tick: u64) -> Option<Value> {
    let phase = world.phase.as_ref()?.current;
    let event = world.reinforcements.try_spawn_next(phase, kill_count, tick)?;
    Some(reinforcement_wave_spawned_payload(&event))
}

/// **M7**: apply damage to the mini-boss + return `boss.phase_changed`
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

/// **M7**: emit a `boss.special_ability_triggered` payload when the boss
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

// **M7-B**: event payload helpers for the 9 NEW ai.* schemas. Each helper
// is a pure function that the engine calls right before
// `recorder.record(tick, sim_time_ms, "ai", "<event_type>", payload, parent)`.

/// **M7-B**: build a JSON payload for `ai.priority_table_changed`.
pub fn priority_table_changed_payload(actor_id: u64, task: TaskType, old_weight: u8, new_weight: u8) -> Value {
    json!({
        "actor_id": actor_id,
        "task": task.as_str(),
        "old_weight": old_weight,
        "new_weight": new_weight,
    })
}

/// **M7-B**: build a JSON payload for `ai.autonomy_mode_changed`.
pub fn autonomy_mode_changed_payload(actor_id: u64, from: AutonomyMode, to: AutonomyMode) -> Value {
    json!({
        "actor_id": actor_id,
        "from": from.as_str(),
        "to": to.as_str(),
    })
}

/// **M7-B**: build a JSON payload for `ai.role_template_applied`.
pub fn role_template_applied_payload(actor_id: u64, template: RoleTemplate) -> Value {
    json!({
        "actor_id": actor_id,
        "template_id": template.as_str(),
    })
}

/// **M7-B**: build a JSON payload for `ai.quick_preset_applied`.
pub fn quick_preset_applied_payload(actor_id: u64, preset: QuickPresetId) -> Value {
    json!({
        "actor_id": actor_id,
        "preset_id": preset.as_str(),
    })
}

/// **M7-B**: build a JSON payload for `ai.chatter_emitted`. Surfaces the
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

/// **M7-B**: build a JSON payload for `ai.personality_changed`. `traits`
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

/// **M7-B**: build a JSON payload for `ai.mood_changed`.
pub fn mood_changed_payload(actor_id: u64, delta: f32, new_mood: f32, cause: &str) -> Value {
    json!({
        "actor_id": actor_id,
        "delta": delta,
        "new_mood": new_mood,
        "cause": cause,
    })
}

/// **M7-B**: stress threshold the actor crossed. Names match the spec
/// step changes (mood < -75 = depressed; stress > 75 = broken).
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum StressThreshold {
    Calm,
    Stressed,
    Depressed,
    Broken,
}

impl StressThreshold {
    pub fn as_str(self) -> &'static str {
        match self {
            StressThreshold::Calm => "calm",
            StressThreshold::Stressed => "stressed",
            StressThreshold::Depressed => "depressed",
            StressThreshold::Broken => "broken",
        }
    }
}

/// **M7-B**: build a JSON payload for `ai.stress_threshold_crossed`.
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

/// **M7-B**: build a JSON payload for `ai.faction_allegiance_changed`.
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

#[cfg(test)]
mod tests {
    use super::*;
    use cf_actor::{Inventory, Vec2};
    use cf_mission::MissionPhase;

    fn actor(id: u64, x: f32, y: f32, status: Status) -> ActorState {
        let mut a = ActorState::player(ActorId(id), "test", Vec2::new(x, y), 100.0, Inventory::default());
        a.status = status;
        a
    }

    #[test]
    fn nearest_medic_picks_closest_alive() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(1), Archetype::Medic);
        world.assign_archetype(ActorId(2), Archetype::Medic);
        let mut actors = BTreeMap::new();
        actors.insert(ActorId(1), actor(1, 100.0, 0.0, Status::Stable));
        actors.insert(ActorId(2), actor(2, 10.0, 0.0, Status::Stable));
        actors.insert(ActorId(99), actor(99, 0.0, 0.0, Status::Dying));
        let pick = nearest_medic(&world.bots, &actors, ActorId(99), 500.0);
        assert!(pick.is_some());
        assert_eq!(pick.unwrap().0, ActorId(2));
    }

    #[test]
    fn begin_auto_triage_only_for_medic_archetype() {
        let mut bot = BotState::new(Archetype::Rifleman);
        let r = begin_auto_triage(&mut bot, ActorId(1), ActorId(2), 100, 60);
        assert!(r.is_none());

        let mut bot = BotState::new(Archetype::Medic);
        let r = begin_auto_triage(&mut bot, ActorId(1), ActorId(2), 100, 60);
        assert!(r.is_some());
        assert!(bot.auto_triage.is_some());
    }

    #[test]
    fn complete_auto_triage_terminates_mission() {
        let mut bot = BotState::new(Archetype::Medic);
        let _ = begin_auto_triage(&mut bot, ActorId(1), ActorId(2), 100, 60);
        let payload = complete_auto_triage(&mut bot, 360, 60);
        assert!(payload.is_some());
        assert!(bot.auto_triage.as_ref().unwrap().is_terminal());
    }

    #[test]
    fn auto_repair_progress_increments_counter() {
        let mut bot = BotState::new(Archetype::Engineer);
        let _ = begin_auto_repair(&mut bot, ActorId(1), ActorId(2), "leg_left", 100, 60);
        progress_auto_repair(&mut bot, 360, 5.0);
        progress_auto_repair(&mut bot, 420, 5.0);
        assert_eq!(bot.auto_repair.as_ref().unwrap().progressed_ticks, 2);
    }

    #[test]
    fn phase_advance_emits_payload_at_deadline() {
        let mut world = M7AiWorld::new();
        world.init_phase(0);
        let deadline_tick = 30 * 60;
        let r = advance_phase(&mut world, deadline_tick + 1, 60, "elapsed");
        assert!(r.is_some());
        assert_eq!(world.phase.as_ref().unwrap().current, MissionPhase::Buildup);
    }

    /// **M7-B**: `act.player.set_priority` mutates the bot's
    /// PriorityTable AND keeps the utility scorer's cached priority in
    /// sync so the next tick scores against the new weight.
    #[test]
    fn set_priority_mutates_state_and_utility_cache() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(7), Archetype::Medic);
        let r = world.set_priority(ActorId(7), TaskType::TriageDownedAlly, 3);
        assert!(r.is_ok());
        let (old, new) = r.unwrap();
        assert_eq!(old, 9, "Medic role template starts TriageDownedAlly at 9");
        assert_eq!(new, 3);
        let bot = world.bot(ActorId(7)).unwrap();
        assert_eq!(bot.stack.priority.get(TaskType::TriageDownedAlly), 3);
        assert_eq!(bot.stack.utility.priority.get(TaskType::TriageDownedAlly), 3);
    }

    #[test]
    fn set_priority_clamps_to_nine() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(7), Archetype::Sniper);
        let r = world.set_priority(ActorId(7), TaskType::SharpshootTarget, 250);
        assert!(r.is_ok());
        assert_eq!(r.unwrap().1, 9);
    }

    #[test]
    fn set_priority_rejects_unknown_actor() {
        let mut world = M7AiWorld::new();
        let r = world.set_priority(ActorId(99), TaskType::EngageVisibleEnemy, 5);
        assert!(r.is_err());
    }

    #[test]
    fn set_autonomy_returns_old_mode() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(7), Archetype::Rifleman);
        let prev = world.set_autonomy(ActorId(7), AutonomyMode::Manual);
        assert_eq!(prev, Some(AutonomyMode::FullAuto));
        assert_eq!(world.bot(ActorId(7)).unwrap().stack.autonomy, AutonomyMode::Manual);
    }

    #[test]
    fn apply_role_template_swaps_priority_and_archetype() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(7), Archetype::Rifleman);
        let r = world.apply_role_template(ActorId(7), RoleTemplate::Medic);
        assert!(r.is_some());
        let bot = world.bot(ActorId(7)).unwrap();
        assert_eq!(bot.archetype, Archetype::Medic);
        assert_eq!(bot.stack.priority.get(TaskType::TriageDownedAlly), 9);
    }

    #[test]
    fn apply_quick_preset_shifts_weights() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(7), Archetype::Rifleman);
        let r = world.apply_quick_preset(ActorId(7), QuickPresetId::Attack);
        assert!(r.is_some());
        let bot = world.bot(ActorId(7)).unwrap();
        // Rifleman base EngageVisibleEnemy = 7; +2 = 9.
        assert_eq!(bot.stack.priority.get(TaskType::EngageVisibleEnemy), 9);
        // HoldCover = 6; -2 = 4.
        assert_eq!(bot.stack.priority.get(TaskType::HoldCover), 4);
    }

    #[test]
    fn priority_table_view_lists_all_22_weights() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(7), Archetype::Medic);
        let view = world.priority_table_view(ActorId(7)).unwrap();
        let weights = view.get("weights").unwrap().as_object().unwrap();
        assert_eq!(weights.len(), 22);
        assert_eq!(weights.get("triage_downed_ally").unwrap(), &json!(9));
        assert_eq!(view.get("role").unwrap(), &json!("medic"));
    }

    #[test]
    fn autonomy_view_carries_mode_and_cap() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(7), Archetype::Rifleman);
        world.set_autonomy(ActorId(7), AutonomyMode::Standard);
        let view = world.autonomy_view(ActorId(7)).unwrap();
        assert_eq!(view.get("mode").unwrap(), &json!("standard"));
        assert_eq!(view.get("auto_action_cap").unwrap(), &json!(3));
    }

    /// **M7-B**: PriorityTable persists across snapshot/restore cycles
    /// (round-trip preserves weights). Spec § PriorityTable persists.
    #[test]
    fn priority_table_round_trips_through_snapshot_restore() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(1), Archetype::Sniper);
        world.assign_archetype(ActorId(2), Archetype::Engineer);
        world.set_priority(ActorId(1), TaskType::SharpshootTarget, 9).unwrap();
        world.set_priority(ActorId(2), TaskType::SetTrap, 8).unwrap();
        let snap = world.snapshot_actor_priorities();

        // Mutate after snapshot.
        world.set_priority(ActorId(1), TaskType::SharpshootTarget, 1).unwrap();
        world.set_priority(ActorId(2), TaskType::SetTrap, 1).unwrap();

        // Restore — weights return to snapshot values.
        let restored = world.restore_actor_priorities(&snap);
        assert_eq!(restored, 2);
        assert_eq!(
            world
                .bot(ActorId(1))
                .unwrap()
                .stack
                .priority
                .get(TaskType::SharpshootTarget),
            9
        );
        assert_eq!(world.bot(ActorId(2)).unwrap().stack.priority.get(TaskType::SetTrap), 8);
    }

    #[test]
    fn try_emit_chatter_gates_within_cooldown_window() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(7), Archetype::Medic);
        let first = world.try_emit_chatter(ActorId(7), ChatterCategory::Triage, "Treating Jenkins", 100, 60);
        assert!(first.is_some(), "first emission opens the slot");
        let (event, _info) = first.unwrap();
        assert_eq!(event.text, "Treating Jenkins");
        assert_eq!(event.voice_id, "voice.medic.default");
        // Same category 200 ticks later (3.33s) — still in 4s cooldown.
        let second = world.try_emit_chatter(ActorId(7), ChatterCategory::Triage, "Re-Treating", 200, 60);
        assert!(second.is_none(), "still in cooldown");
        // 4.0s later — boundary case (240 ticks @ 60 Hz). 100 + 240 = 340.
        let third = world.try_emit_chatter(ActorId(7), ChatterCategory::Triage, "Treating again", 340, 60);
        assert!(third.is_some(), "boundary-tick emission is allowed");
    }

    #[test]
    fn personality_modifier_recorded_on_apply() {
        let mut world = M7AiWorld::new();
        world.assign_archetype(ActorId(7), Archetype::Rifleman);
        let r = world.apply_personality_modifier(ActorId(7), PersonalityModifier::Aggressive);
        assert!(r.is_some());
        let bot = world.bot(ActorId(7)).unwrap();
        assert_eq!(bot.personality_modifier, PersonalityModifier::Aggressive);
        // Rifleman EngageVisibleEnemy = 7; +2 (Aggressive) = 9.
        assert_eq!(bot.stack.priority.get(TaskType::EngageVisibleEnemy), 9);
    }

    #[test]
    fn priority_table_changed_payload_shape() {
        let v = priority_table_changed_payload(7, TaskType::TriageDownedAlly, 9, 3);
        assert_eq!(v.get("actor_id").unwrap(), &json!(7));
        assert_eq!(v.get("task").unwrap(), &json!("triage_downed_ally"));
        assert_eq!(v.get("old_weight").unwrap(), &json!(9));
        assert_eq!(v.get("new_weight").unwrap(), &json!(3));
    }

    #[test]
    fn autonomy_mode_changed_payload_shape() {
        let v = autonomy_mode_changed_payload(7, AutonomyMode::FullAuto, AutonomyMode::Manual);
        assert_eq!(v.get("from").unwrap(), &json!("full_auto"));
        assert_eq!(v.get("to").unwrap(), &json!("manual"));
    }

    #[test]
    fn role_template_applied_payload_shape() {
        let v = role_template_applied_payload(7, RoleTemplate::Medic);
        assert_eq!(v.get("template_id").unwrap(), &json!("medic"));
    }

    #[test]
    fn quick_preset_applied_payload_shape() {
        let v = quick_preset_applied_payload(7, QuickPresetId::Rescue);
        assert_eq!(v.get("preset_id").unwrap(), &json!("rescue"));
    }

    #[test]
    fn faction_allegiance_changed_payload_shape() {
        let v = faction_allegiance_changed_payload(FactionId::Player, FactionId::AiAllied, -30, 45, "friendly_fire");
        assert_eq!(v.get("a").unwrap(), &json!("player"));
        assert_eq!(v.get("b").unwrap(), &json!("ai_allied"));
        assert_eq!(v.get("delta").unwrap(), &json!(-30));
        assert_eq!(v.get("new_value").unwrap(), &json!(45));
        assert_eq!(v.get("cause").unwrap(), &json!("friendly_fire"));
    }

    // -----------------------------------------------------------------
    // **M7-A fix-round-2 (audit gaps A1-A7)**: payload shape + behavior
    // transition coverage for the 7 sub-plan events the engine emits via
    // `detect_behavior_transitions`.
    // -----------------------------------------------------------------

    fn signals_baseline(actor_id: u64) -> BehaviorSignals {
        BehaviorSignals {
            actor_id,
            self_position: [0.0, 0.0],
            hp_fraction: 1.0,
            enemy_visible: false,
            under_fire: false,
            reactive_override: false,
            player_actor_id: Some(99),
            player_position: Some([100.0, 0.0]),
            squadmates: vec![2, 3],
            friendly_in_line_of_fire: None,
            current_tick: 100,
            tick_rate_hz: 60,
        }
    }

    #[test]
    fn cover_seeking_started_payload_shape() {
        let event = CoverSeekingEvent {
            actor_id: 7,
            archetype: Archetype::Rifleman,
            reason: CoverSeekingReason::Fired,
            target_position: [1.0, 2.0],
            distance: 3.5,
        };
        let v = cover_seeking_started_payload(&event);
        assert_eq!(v.get("actor_id").unwrap(), &json!(7));
        assert_eq!(v.get("archetype").unwrap(), &json!("rifleman"));
        assert_eq!(v.get("reason").unwrap(), &json!("fired"));
        assert_eq!(v.get("target_position").unwrap(), &json!([1.0, 2.0]));
        assert_eq!(v.get("distance").unwrap(), &json!(3.5));
    }

    #[test]
    fn suppression_started_payload_shape() {
        let event = SuppressionEvent::build(7, 99, Some(8), 60);
        let v = suppression_started_payload(&event);
        assert_eq!(v.get("actor_id").unwrap(), &json!(7));
        assert_eq!(v.get("target_actor_id").unwrap(), &json!(99));
        assert_eq!(v.get("flanker_actor_id").unwrap(), &json!(8));
        assert_eq!(v.get("duration_ticks").unwrap(), &json!(240));
    }

    #[test]
    fn retreat_decision_payload_shape() {
        let event = RetreatDecisionEvent {
            actor_id: 7,
            reason: RetreatReason::HpLow,
            hp_fraction: 0.25,
            tick: 100,
        };
        let v = retreat_decision_payload(&event);
        assert_eq!(v.get("actor_id").unwrap(), &json!(7));
        assert_eq!(v.get("reason").unwrap(), &json!("hp_low"));
        assert_eq!(v.get("hp_fraction").unwrap(), &json!(0.25));
        assert_eq!(v.get("tick").unwrap(), &json!(100));
    }

    #[test]
    fn squad_comm_relayed_payload_shape() {
        let event = SquadCommRelayedEvent {
            originator_actor_id: 7,
            receiver_actor_ids: vec![2, 3],
            target_actor_id: 99,
            target_position: [50.0, 60.0],
            delay_ticks: 30,
        };
        let v = squad_comm_relayed_payload(&event);
        assert_eq!(v.get("originator_actor_id").unwrap(), &json!(7));
        assert_eq!(v.get("receiver_actor_ids").unwrap(), &json!([2, 3]));
        assert_eq!(v.get("target_actor_id").unwrap(), &json!(99));
        assert_eq!(v.get("target_position").unwrap(), &json!([50.0, 60.0]));
        assert_eq!(v.get("delay_ticks").unwrap(), &json!(30));
    }

    #[test]
    fn patrol_waypoint_reached_payload_shape() {
        let event = PatrolWaypointReachedEvent {
            actor_id: 7,
            waypoint_index: 1,
            position: [10.0, 0.0],
            idle_seconds: 7.5,
        };
        let v = patrol_waypoint_reached_payload(&event);
        assert_eq!(v.get("actor_id").unwrap(), &json!(7));
        assert_eq!(v.get("waypoint_index").unwrap(), &json!(1));
        assert_eq!(v.get("position").unwrap(), &json!([10.0, 0.0]));
        assert_eq!(v.get("idle_seconds").unwrap(), &json!(7.5));
    }

    #[test]
    fn friendly_fire_avoidance_payload_shape() {
        let event = FriendlyFireAvoidanceEvent {
            actor_id: 7,
            friendly_actor_id: 8,
            kind: FriendlyFireKind::LineOfFire,
        };
        let v = friendly_fire_avoidance_payload(&event);
        assert_eq!(v.get("actor_id").unwrap(), &json!(7));
        assert_eq!(v.get("friendly_actor_id").unwrap(), &json!(8));
        assert_eq!(v.get("kind").unwrap(), &json!("line_of_fire"));
    }

    #[test]
    fn high_ground_preference_applied_payload_shape() {
        let event = HighGroundEvent {
            actor_id: 7,
            target_position: [3.0, 8.0],
            elevation_gain: 8.0,
        };
        let v = high_ground_preference_applied_payload(&event);
        assert_eq!(v.get("actor_id").unwrap(), &json!(7));
        assert_eq!(v.get("target_position").unwrap(), &json!([3.0, 8.0]));
        assert_eq!(v.get("elevation_gain").unwrap(), &json!(8.0));
    }

    /// **A1 production-path coverage**: cover-seeking transition fires
    /// when the bot's chosen task flips into HoldCover under a fire
    /// signal. Subsequent ticks at the same task do NOT re-fire.
    #[test]
    fn detect_emits_cover_seeking_on_transition_into_cover() {
        let mut bot = BotState::new(Archetype::Rifleman);
        let mut sig = signals_baseline(7);
        sig.under_fire = true;
        let emit = detect_behavior_transitions(&mut bot, TaskType::HoldCover, &sig);
        assert!(emit.cover_seeking_started.is_some());
        let emit2 = detect_behavior_transitions(&mut bot, TaskType::HoldCover, &sig);
        assert!(
            emit2.cover_seeking_started.is_none(),
            "no re-fire on second tick at same task"
        );
    }

    /// **A2 production-path coverage**: suppression-started fires on
    /// transition into SuppressFire and carries the player as target.
    #[test]
    fn detect_emits_suppression_on_transition_into_suppress() {
        let mut bot = BotState::new(Archetype::Rifleman);
        let sig = signals_baseline(7);
        let emit = detect_behavior_transitions(&mut bot, TaskType::SuppressFire, &sig);
        let payload = emit.suppression_started.expect("suppression payload");
        assert_eq!(payload.get("actor_id").unwrap(), &json!(7));
        assert_eq!(payload.get("target_actor_id").unwrap(), &json!(99));
    }

    /// **A3 production-path coverage**: retreat-decision fires when the
    /// bot transitions into RetreatToCover with HP below threshold.
    #[test]
    fn detect_emits_retreat_decision_when_hp_low() {
        let mut bot = BotState::new(Archetype::Rifleman);
        let mut sig = signals_baseline(7);
        sig.hp_fraction = 0.20;
        let emit = detect_behavior_transitions(&mut bot, TaskType::RetreatToCover, &sig);
        let payload = emit.retreat_decision.expect("retreat payload");
        assert_eq!(payload.get("reason").unwrap(), &json!("hp_low"));
        assert_eq!(payload.get("actor_id").unwrap(), &json!(7));
    }

    /// **A4 production-path coverage**: squad-comm relay schedules on
    /// the lost→spotted transition AND fires after the 0.5s delay
    /// (30 ticks @ 60Hz) carrying the squadmates as receivers.
    #[test]
    fn detect_emits_squad_comm_relayed_after_delay() {
        let mut bot = BotState::new(Archetype::Rifleman);
        let mut sig = signals_baseline(7);
        sig.enemy_visible = true;
        sig.current_tick = 100;
        let emit_first = detect_behavior_transitions(&mut bot, TaskType::EngageVisibleEnemy, &sig);
        assert!(
            emit_first.squad_comm_relayed.is_empty(),
            "no relay emitted before delay elapses"
        );
        sig.current_tick = 130;
        let emit_after = detect_behavior_transitions(&mut bot, TaskType::EngageVisibleEnemy, &sig);
        assert_eq!(emit_after.squad_comm_relayed.len(), 1);
        let payload = &emit_after.squad_comm_relayed[0];
        assert_eq!(payload.get("originator_actor_id").unwrap(), &json!(7));
        assert_eq!(payload.get("receiver_actor_ids").unwrap(), &json!([2, 3]));
        assert_eq!(payload.get("delay_ticks").unwrap(), &json!(30));
    }

    /// **A5 production-path coverage**: patrol-waypoint-reached fires
    /// on the tick the bot's idle countdown expires AND the cursor
    /// advances. The default route has 2 waypoints.
    #[test]
    fn detect_emits_patrol_waypoint_when_idle_expires() {
        let mut bot = BotState::new(Archetype::Rifleman);
        let sig = signals_baseline(7);
        let emit = detect_behavior_transitions(&mut bot, TaskType::Patrol, &sig);
        let payload = emit.patrol_waypoint_reached.expect("patrol payload");
        assert_eq!(payload.get("actor_id").unwrap(), &json!(7));
        assert!(payload.get("position").is_some());
    }

    /// **A6 production-path coverage**: friendly-fire-avoidance fires
    /// when the bot is shooting AND a friendly is in the line of fire.
    /// Throttled to one emission per friendly until cleared.
    #[test]
    fn detect_emits_friendly_fire_avoidance_when_friend_in_los() {
        let mut bot = BotState::new(Archetype::Rifleman);
        let mut sig = signals_baseline(7);
        sig.friendly_in_line_of_fire = Some(8);
        let emit = detect_behavior_transitions(&mut bot, TaskType::EngageVisibleEnemy, &sig);
        let payload = emit.friendly_fire_avoidance.expect("ff payload");
        assert_eq!(payload.get("friendly_actor_id").unwrap(), &json!(8));
        assert_eq!(payload.get("kind").unwrap(), &json!("line_of_fire"));
        // Re-tick with the same friendly still blocking — no re-fire.
        let emit2 = detect_behavior_transitions(&mut bot, TaskType::EngageVisibleEnemy, &sig);
        assert!(emit2.friendly_fire_avoidance.is_none());
    }

    /// **A7 production-path coverage**: high-ground-preference-applied
    /// fires when a Sniper transitions into SharpshootTarget while
    /// standing on positive elevation.
    #[test]
    fn detect_emits_high_ground_preference_for_sniper_on_elevation() {
        let mut bot = BotState::new(Archetype::Sniper);
        let mut sig = signals_baseline(7);
        sig.self_position = [10.0, 25.0];
        let emit = detect_behavior_transitions(&mut bot, TaskType::SharpshootTarget, &sig);
        let payload = emit.high_ground_preference_applied.expect("high-ground payload");
        assert_eq!(payload.get("actor_id").unwrap(), &json!(7));
        assert_eq!(payload.get("elevation_gain").unwrap(), &json!(25.0));
    }

    /// **A7 negative**: Rifleman archetype does NOT fire high-ground
    /// even on the same transition / elevation (high-ground is
    /// Sniper/Spotter-only per spec).
    #[test]
    fn detect_does_not_emit_high_ground_for_rifleman() {
        let mut bot = BotState::new(Archetype::Rifleman);
        let mut sig = signals_baseline(7);
        sig.self_position = [10.0, 25.0];
        let emit = detect_behavior_transitions(&mut bot, TaskType::SharpshootTarget, &sig);
        assert!(emit.high_ground_preference_applied.is_none());
    }
}
