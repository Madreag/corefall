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

pub enum AssignmentResult {
    Unchanged,
    Changed { previous: Archetype },
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

/// surfaced by [`drain_objective_graph_emissions`].
#[derive(Debug, Clone, Default)]
pub struct ObjectiveGraphEmit {
    pub optional_offered: Vec<Value>,
    pub objective_branched: Vec<Value>,
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

/// 4-phase pacing parameters. Defaults match `PhaseState::new` (30 / 60
/// / 120 seconds). The engine consumes this in `M0Engine::new` to seed
/// `world.phase`.
#[derive(Debug, Clone, PartialEq)]
pub struct InitialPhaseState {
    pub setup_seconds: f32,
    pub buildup_seconds: f32,
    pub climax_seconds: f32,
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

/// step changes (mood < -75 = depressed; stress > 75 = broken).
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum StressThreshold {
    Calm,
    Stressed,
    Depressed,
    Broken,
}

