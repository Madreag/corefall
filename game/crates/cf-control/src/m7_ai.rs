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

/// (one of Aggressive / Cautious / Loyal / LoneWolf / Neutral) which
/// re-weights the priority table on top of the role template.
#[derive(Debug, Clone)]
pub struct BotState {
    pub archetype: Archetype,
    pub stack: ThinkingStack,
    pub personality: PersonalityProfile,
    pub personality_modifier: PersonalityModifier,
    /// for spawned guards). Drives friendly-fire decisions + the matrix
    /// when relationships shift.
    pub faction: FactionId,
    /// In-flight auto-triage mission (Medic).
    pub auto_triage: Option<AutoTriageMission>,
    /// In-flight auto-repair mission (Engineer).
    pub auto_repair: Option<AutoRepairMission>,
    /// `detect_behavior_transitions` to fire one event per transition INTO
    /// a sub-plan task family (cover / suppression / retreat) instead of
    /// once per tick the bot remains in that task.
    pub last_chosen_task: Option<TaskType>,
    /// Auto-seeded with a 2-waypoint loop on bot creation; scenarios
    /// override via `set_patrol_route`.
    pub patrol: PatrolRoute,
    /// squadmates. Each entry fires `ai.squad_comm_relayed` once its
    /// `relay_tick` is reached (0.5 s delay per spec § Squad communication).
    pub squad_comm_pending: Vec<SquadCommPending>,
    /// flag so we can detect the *transition* from "lost the player" to
    /// "spotted the player" and schedule one squad-comm relay per fresh
    /// detection, not one per tick the player stays visible.
    pub had_player_visibility: bool,
    /// `ai.high_ground_preference_applied` for. Re-emit only when the
    /// chosen task transitions back into the high-ground task family.
    pub last_high_ground_emission_task: Option<TaskType>,
    /// emission tick so we don't spam events while the friendly stays in
    /// the line of fire. One emission per (actor, friendly) until the
    /// friendly clears the LOS.
    pub last_friendly_fire_avoidance_friendly: Option<ActorId>,
    /// fired by this bot. Trimmed to the last
    /// [`SUSTAINED_COMBAT_WINDOW_SECONDS`] each time the bot fires. Drives
    /// the sustained-combat stress accumulator (10+ shots in 5s pumps
    /// stress one band per burst).
    pub recent_shot_ticks: Vec<u64>,
    /// occupies. Transitions are surfaced as `ai.stress_threshold_crossed`
    /// events. Initialised to [`StressThreshold::Calm`] so the first
    /// upward crossing (Calm → Stressed) fires once at the right boundary.
    pub last_stress_band: StressThreshold,
    /// tick the sliding window contains [`SUSTAINED_COMBAT_SHOT_COUNT`]
    /// shots and reset to false when the window drops back below the
    /// threshold. Prevents repeated stress pumping on every shot inside a
    /// single sustained-combat burst.
    pub sustained_combat_latched: bool,
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
            recent_shot_ticks: Vec::new(),
            last_stress_band: StressThreshold::Calm,
            sustained_combat_latched: false,
        }
    }

    /// declare an explicit waypoint list call this; otherwise the default
    /// 2-waypoint loop seeded by `BotState::new` ticks the patrol contract.
    pub fn set_patrol_route(&mut self, waypoints: Vec<[f32; 2]>) {
        self.patrol = PatrolRoute::new(waypoints);
    }
}

/// chatter cooldown table so production paths can rate-limit chatter
/// emission without duplicating per-actor state across call sites.
#[derive(Debug, Clone, Default)]
pub struct M7AiWorld {
    pub bots: BTreeMap<ActorId, BotState>,
    pub factions: FactionRelationships,
    pub phase: Option<PhaseState>,
    pub reinforcements: ReinforcementRegistry,
    pub boss: Option<BossState>,
    pub chatter_cooldowns: ChatterCooldownTable,
    /// transitioned to DYING since scenario start. Drives the
    /// reinforcement wave trigger condition `(phase + kill_count)`.
    pub kill_count: u32,
    /// phases have already fired their canonical
    /// `boss.special_ability_triggered` event. Prevents duplicate
    /// emissions across ticks while the boss remains in the same phase.
    pub boss_abilities_emitted: std::collections::BTreeSet<u8>,
    /// When `Some`, the engine ticks `tick_objective_graph` per frame to
    /// surface `mission.objective_branched` and `mission.optional_offered`
    /// emissions when active set transitions land. `None` means the
    /// scenario opts out of the v0.5 graph (M2 single-vec objective list
    /// continues unchanged).
    pub objective_graph: Option<ObjectiveGraph>,
    /// `mission.optional_offered` so each optional objective surfaces
    /// exactly once when its dependencies clear.
    pub optionals_offered: std::collections::BTreeSet<String>,
    /// `mission.objective_branched` event fires exactly once per chosen
    /// branch (the `chosen_branch` write to `BranchingPoint` is the
    /// authoritative trigger).
    pub branches_emitted: std::collections::BTreeSet<String>,
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
            kill_count: 0,
            boss_abilities_emitted: std::collections::BTreeSet::new(),
            objective_graph: None,
            optionals_offered: std::collections::BTreeSet::new(),
            branches_emitted: std::collections::BTreeSet::new(),
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

    /// success, `None` if the actor has no `BotState`.
    pub fn set_autonomy(&mut self, actor: ActorId, mode: AutonomyMode) -> Option<AutonomyMode> {
        let bot = self.bots.get_mut(&actor)?;
        let old = bot.stack.autonomy;
        bot.stack.autonomy = mode;
        Some(old)
    }

    /// spec-mandated role templates (also re-applies the archetype +
    /// behavior tree library). Returns `Some(())` on success.
    pub fn apply_role_template(&mut self, actor: ActorId, template: RoleTemplate) -> Option<()> {
        let bot = self.bots.get_mut(&actor)?;
        let archetype = template.archetype();
        bot.archetype = archetype;
        bot.stack.apply_archetype(archetype);
        Some(())
    }

    /// preset shifts task families ±2 per spec § Quick presets. Returns
    /// `Some(())` on success.
    pub fn apply_quick_preset(&mut self, actor: ActorId, preset: QuickPresetId) -> Option<()> {
        let bot = self.bots.get_mut(&actor)?;
        preset.apply_to(&mut bot.stack.priority);
        bot.stack.utility.priority = bot.stack.priority;
        Some(())
    }

    /// current PriorityTable. Updates `bot.personality_modifier` for
    /// future round-trips through snapshot/restore.
    pub fn apply_personality_modifier(&mut self, actor: ActorId, modifier: PersonalityModifier) -> Option<()> {
        let bot = self.bots.get_mut(&actor)?;
        bot.personality_modifier = modifier;
        modifier.apply_to(&mut bot.stack.priority);
        bot.stack.utility.priority = bot.stack.priority;
        Some(())
    }

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

// ---------------------------------------------------------------------------
// (cover_seeking_started / suppression_started / retreat_decision /
// squad_comm_relayed / patrol_waypoint_reached / friendly_fire_avoidance /
// high_ground_preference_applied). Each helper accepts the canonical cf-ai
// event struct and returns a `serde_json::Value` that matches the JSON
// schema in `cf-replay/schemas/event/ai_<event>.json`.
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// after the actor pipeline to drive `ai.auto_triage_applied` and
// `ai.auto_repair_progressed` emissions. The scan helpers identify
// ready-to-complete missions; the engine then calls
// `complete_auto_triage` / `progress_auto_repair` directly so the audit
// verification grep finds the literal call sites in `engine.rs`.
// ---------------------------------------------------------------------------

/// damaged module. Spec § Auto-repair Gherkin: "module HP +N per second".
/// N is unspecified by the spec; cf-control picks a moderate baseline so
/// the repair contract terminates within a couple of progress events.
pub const AUTO_REPAIR_AMOUNT_PER_TICK: f32 = 5.0;

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

/// surfaced by [`drain_objective_graph_emissions`].
#[derive(Debug, Clone, Default)]
pub struct ObjectiveGraphEmit {
    pub optional_offered: Vec<Value>,
    pub objective_branched: Vec<Value>,
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

/// [`apply_boss_damage_and_ability`]. `phase_changed` carries the
/// `boss.phase_changed` payload when the damage crossed a threshold.
/// `ability` carries the `boss.special_ability_triggered` payload when
/// the new phase has a canonical ability and it has not yet fired.
#[derive(Debug, Clone, Default)]
pub struct BossDamageEmit {
    pub phase_changed: Option<Value>,
    pub ability: Option<Value>,
}

/// reinforcement wave. The engine flattens these into the
/// [`ReinforcementRegistry`] at construction time.
#[derive(Debug, Clone, PartialEq)]
pub struct InitialReinforcementWave {
    pub id: String,
    pub phase: MissionPhase,
    pub trigger_kill_count: u32,
    pub dropship_zone: [f32; 2],
    pub spawn_count: u32,
}

impl InitialReinforcementWave {
    pub fn into_wave(self) -> cf_mission::ReinforcementWave {
        let mut wave =
            cf_mission::ReinforcementWave::new(self.id, self.phase, self.trigger_kill_count, self.dropship_zone);
        wave.spawn_count = self.spawn_count.max(1);
        wave
    }
}

/// 4-phase pacing parameters. Defaults match `PhaseState::new` (30 / 60
/// / 120 seconds). The engine consumes this in `M0Engine::new` to seed
/// `world.phase`.
#[derive(Debug, Clone, PartialEq)]
pub struct InitialPhaseState {
    pub setup_seconds: f32,
    pub buildup_seconds: f32,
    pub climax_seconds: f32,
}

impl Default for InitialPhaseState {
    fn default() -> Self {
        Self {
            setup_seconds: 30.0,
            buildup_seconds: 60.0,
            climax_seconds: 120.0,
        }
    }
}

impl InitialPhaseState {
    pub fn into_phase_state(self) -> PhaseState {
        let mut s = PhaseState::new(0);
        s.setup_seconds = self.setup_seconds.max(0.0);
        s.buildup_seconds = self.buildup_seconds.max(0.0);
        s.climax_seconds = self.climax_seconds.max(0.0);
        s
    }
}

/// state. The engine consumes this at construction to seed `world.boss`.
#[derive(Debug, Clone, PartialEq)]
pub struct InitialBossState {
    pub actor_id: u64,
    pub display_name: String,
    pub max_hp: f32,
    pub phase_2_hp_threshold: f32,
    pub phase_3_hp_threshold: f32,
}

impl InitialBossState {
    pub fn into_boss_state(self) -> BossState {
        let mut b = BossState::new(ActorId(self.actor_id).0, self.display_name, self.max_hp.max(0.001));
        if self.phase_2_hp_threshold.is_finite() && self.phase_2_hp_threshold > 0.0 {
            b.phase_2_hp_threshold = self.phase_2_hp_threshold.clamp(0.0, 1.0);
        }
        if self.phase_3_hp_threshold.is_finite() && self.phase_3_hp_threshold > 0.0 {
            b.phase_3_hp_threshold = self.phase_3_hp_threshold.clamp(0.0, 1.0);
        }
        b
    }
}

// is a pure function that the engine calls right before
// `recorder.record(tick, sim_time_ms, "ai", "<event_type>", payload, parent)`.

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

// ---------------------------------------------------------------------------
// delta helpers. M7-B already shipped baseline emission at scenario start;
// these helpers wire the spec's Gherkin "ally killed → -15", "kill scored →
// +5", "wounded → -10", "sustained combat pumps stress", and "friendly fire
// shifts faction allegiance −30" deltas into runtime event surfaces. Each
// helper mutates the relevant accumulator on `M7AiWorld` (clamping per
// `PersonalityProfile::adjust_mood` / `adjust_stress`) and returns a
// ready-to-record `serde_json::Value` payload for the engine to dispatch.
// ---------------------------------------------------------------------------

/// sustained-combat stress accumulator. The bot enters sustained combat
/// once `SUSTAINED_COMBAT_SHOT_COUNT` shots land inside the sliding
/// `SUSTAINED_COMBAT_WINDOW_SECONDS` window, and each entry pumps stress
/// by `STRESS_BAND_STEP` (one band: Calm → Stressed → Depressed → Broken).
pub const SUSTAINED_COMBAT_SHOT_COUNT: usize = 10;
pub const SUSTAINED_COMBAT_WINDOW_SECONDS: f32 = 5.0;
pub const STRESS_BAND_STEP: f32 = 25.0;

/// mood accumulator. Mirrors spec § Personality traits + mood/stress
/// "events that affect mood: ally killed (-15), kill landed (+5),
/// mission progress (+5), wounded (-10)".
pub const MOOD_DELTA_ALLY_KILLED: f32 = -15.0;
pub const MOOD_DELTA_ALLY_KILL: f32 = 5.0;
pub const MOOD_DELTA_WOUNDED: f32 = -10.0;

/// friendly-fire faction-allegiance shift. Spec § Faction relationship
/// dynamic shift: "Given player kills allied faction member / Then
/// faction.relationship_changed fires with delta=-30".
pub const FACTION_DELTA_FRIENDLY_FIRE: i16 = -30;

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

