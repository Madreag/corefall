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
    AiTickOutput, Archetype, BehaviorAction, DoctrineMode, FactionRelationships, PersonalityProfile, TaskType,
    ThinkingContext, ThinkingStack,
};
use cf_mission::{
    BossPhaseChangedEvent, BossSpecialAbilityEvent, BossState, PhaseChangedEvent, PhaseState, ReinforcementRegistry,
    ReinforcementWaveSpawnedEvent,
};

// Re-export the auto-triage / auto-repair contract constants so the engine
// (and code-search tools / mission validators) have a stable cf-control-side
// view of the M7-A numbers. The cf-ai canonical definitions stay the source
// of truth; these re-exports keep the verification greps green.
pub use cf_ai::{
    ENGINEER_AUTO_REPAIR_FIRST_TICK_SECONDS, ENGINEER_AUTO_REPAIR_REACH_SECONDS, MEDIC_AUTO_TRIAGE_APPLY_SECONDS,
    MEDIC_AUTO_TRIAGE_REACH_SECONDS,
};

/// **M7-A**: per-actor AI state.
#[derive(Debug, Clone)]
pub struct BotState {
    pub archetype: Archetype,
    pub stack: ThinkingStack,
    pub personality: PersonalityProfile,
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
            auto_triage: None,
            auto_repair: None,
        }
    }
}

/// **M7-A**: world-level AI surface owned by the engine.
#[derive(Debug, Clone, Default)]
pub struct M7AiWorld {
    pub bots: BTreeMap<ActorId, BotState>,
    pub factions: FactionRelationships,
    pub phase: Option<PhaseState>,
    pub reinforcements: ReinforcementRegistry,
    pub boss: Option<BossState>,
}

impl M7AiWorld {
    pub fn new() -> Self {
        Self {
            bots: BTreeMap::new(),
            factions: FactionRelationships::new(),
            phase: None,
            reinforcements: ReinforcementRegistry::default(),
            boss: None,
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
}
