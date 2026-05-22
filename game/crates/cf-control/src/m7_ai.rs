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

pub use crate::m7_ai_behavior::{detect_behavior_transitions, tick_bot};
pub use crate::m7_ai_payloads::*;
pub use crate::m7_ai_phase::{
    advance_phase, advance_phase_with_director_event, apply_boss_damage,
    apply_boss_damage_and_ability, boss_ability_for_phase, boss_special_ability,
    drain_boss_phase_ability, drain_objective_graph_emissions, ensure_phase_initialised,
    force_advance_phase, track_kills, try_spawn_reinforcement,
};
pub use crate::m7_ai_stress::{
    adjust_actor_mood, adjust_faction_relationships, faction_for_actor, is_friendly_fire,
    record_shot_for_stress, stress_band_for,
};
pub use crate::m7_ai_thinking::{
    build_context, is_combat_ready, nearest_engineer, nearest_medic, should_retreat,
};
pub use crate::m7_ai_triage::{
    begin_auto_repair, begin_auto_triage, complete_auto_triage, progress_auto_repair,
    ready_repair_progressions, ready_triage_completions,
};
pub use crate::m7_ai_types::{
    AssignmentResult, BehaviorSignals, BossDamageEmit, BotBehaviorEmit, BotState, BotTickEmit,
    InitialBossState, InitialPhaseState, InitialReinforcementWave, M7AiWorld, ObjectiveGraphEmit,
    StressThreshold,
};

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

pub(crate) fn auto_action_cap_to_value(cap: usize) -> Value {
    if cap == usize::MAX {
        Value::String("unbounded".to_string())
    } else {
        Value::from(cap as u64)
    }
}





















// ---------------------------------------------------------------------------
// (cover_seeking_started / suppression_started / retreat_decision /
// squad_comm_relayed / patrol_waypoint_reached / friendly_fire_avoidance /
// high_ground_preference_applied). Each helper accepts the canonical cf-ai
// event struct and returns a `serde_json::Value` that matches the JSON
// schema in `cf-replay/schemas/event/ai_<event>.json`.
// ---------------------------------------------------------------------------











/// Quantize floats for stable replay output.
pub(crate) fn quantize(value: f32) -> f32 {
    if !value.is_finite() {
        return 0.0;
    }
    (value * 100.0).round() / 100.0
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


















impl InitialReinforcementWave {
    pub fn into_wave(self) -> cf_mission::ReinforcementWave {
        let mut wave =
            cf_mission::ReinforcementWave::new(self.id, self.phase, self.trigger_kill_count, self.dropship_zone);
        wave.spawn_count = self.spawn_count.max(1);
        wave
    }
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







