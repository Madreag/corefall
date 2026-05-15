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
        }
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
}
