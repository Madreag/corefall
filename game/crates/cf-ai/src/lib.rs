//! M1.5 reactive enemy controller.
//!
//! M1.5 ships ONE enemy archetype: the `ReactiveGuard`. It exists to give the
//! micro-breach scenario a reason to exist (pressure + counter-attack) without
//! pre-empting the M6 AI core. The DR-008 LEAN (hybrid jobs + utility scoring +
//! scripted hooks) is honoured by this implementation as follows:
//!
//! - **Job (intent layer)**: the guard runs a tiny scripted state machine —
//!   `Idle → Alert → Engaged → Retreating → Dying → Dead` — based on whether the
//!   player is inside its sight cone (and its own hp). M6 will replace the
//!   script with the full job board.
//! - **Tactic (utility scoring)**: per tick the guard scores three tactics
//!   (`Reload`, `Attack`, `Hold`) and picks the highest. Scores are deterministic
//!   functions of the tick, distance, ammo, and cooldowns. M6 will widen the
//!   tactic library; the score-then-pick contract stays the same.
//! - **Custom (scripted hooks)**: aim settle, miss roll, and burst pacing are
//!   scripted in this file. Mods will eventually slot in via the M5/M8 modding
//!   data path; M1.5 keeps everything in code.
//!
//! Every recorder-relevant decision is exposed via [`EnemyTickReport`]; the
//! engine turns it into the `ai.*` / `equipment.weapon_*` / `combat.projectile_*`
//! events the run-bundle schema requires for M1.5.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::struct_excessive_bools,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::float_cmp,
    clippy::if_not_else,
    clippy::field_reassign_with_default,
    clippy::needless_pass_by_value,
    clippy::ref_option,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::large_enum_variant,
    // M7-A additions — the 5-layer thinking stack + archetype + memory
    // surface trips a few additional pedantic lints whose intent doesn't
    // apply here (e.g. `from_str` is an inherent constructor distinct
    // from the FromStr trait; the M7-A enums use the inherent form by spec).
    clippy::should_implement_trait,
    clippy::explicit_iter_loop,
    clippy::needless_lifetimes,
    clippy::redundant_closure,
    clippy::match_same_arms,
    clippy::similar_names,
    clippy::map_unwrap_or,
    clippy::assigning_clones,
    clippy::needless_range_loop,
    clippy::manual_clamp,
    clippy::unused_self,
    clippy::struct_field_names,
    clippy::elidable_lifetime_names,
    clippy::uninlined_format_args
)]

// M2 spec "## Files" wiring: re-export the canonical types via thin
// modules so consumers that import per the spec paths
// (`cf_ai::perception::*` / `cf_ai::guard_state::*` / `cf_ai::difficulty::*`)
// compile cleanly.
pub mod components;
pub mod critter_mount_doctrine;
pub mod difficulty;
pub mod guard_state;
pub mod perception;
pub mod reactive_guard;
pub mod reactive_guard_params;
pub mod step;
pub mod systems;
pub mod tick_io;

// 5-layer thinking stack (Reactive / Utility / BehaviorTree / HTN / LLM
// prior) is composable + testable in isolation; the engine drives the
// stack via `ThinkingStack::tick` once per AI tick per bot. M7-B will
// refactor the PriorityTable + autonomy/role cfctl surface into the
// dedicated `cf-priority` crate.
pub mod archetype;
pub mod auto_repair;
pub mod auto_triage;
pub mod medic_doctrine;
pub mod autonomy;
pub mod behavior_tree;
pub mod bot_memory;
pub mod cf_mind;
pub mod constants;
pub mod cover_seeking;
pub mod faction;
pub mod friendly_fire;
pub mod high_ground;
pub mod htn;
pub mod llm_prior;
pub mod m17_doctrine;
pub mod patrol;
pub mod personality;
pub mod priority;
pub mod reactive;
pub mod reason_label;
pub mod retreat;
pub mod squad_comm;
pub mod suppression;
pub mod task;
pub mod thinking_stack;
pub mod utility;

// over (reactor, player) candidate set + recovery action picker for
// terrain dirty region intersections.
pub mod path_reaction;
pub mod target_selection;

// actors deployed in cf-trench fire_step segments. Emits
// `ai.cover_decision` events with reason_label in
// `{step_up_for_shot, step_down_to_reload, hold_full_cover, reload_safe}`.
pub mod trench_doctrine;

// detected within 24 tiles; uncrew + retreat when nest HP < 200;
// auto-swap depleted ammo box. Spec § "Notes for the implementer".
pub mod mg_doctrine;

// observation post emits `spotter_target_marked` (TTL 3s) for targets
// visible only to the spotter; squad MGs / snipers consume the mark
// for +50% acquisition. Mark cap = 1 per target. Spec § "Spotter role".
pub mod observer_doctrine;

// wire forward of the perimeter, repair breaches, disarm enemy
// minefields in the squad's path. Spec § "Notes for the implementer".
pub mod engineer_doctrine;

// decisions. Detour > 30 tiles around AT ditch unless suspension HP
// > 70% (plow); always detour around dragon's teeth.
pub mod anti_tank_doctrine;

// + per-archetype BT. Spec § "the squad obeys real grammar — formation
// orders, combat verbs, breach-stack discipline — and the player can take
// the wheel of any one of them without the rest forgetting the plan."
pub mod archetype_bt;
pub mod commander_hop;
pub mod formation;
pub mod squad_command_grammar;
pub mod squad_state;

pub use archetype::Archetype;
pub use auto_repair::{AutoRepairInitiatedEvent, AutoRepairMission, AutoRepairProgressedEvent, AutoRepairState};
pub use critter_mount_doctrine::{
    select_gait_for_free_critter, select_gait_for_ride_input, CritterGait, CritterMountGoal,
};
pub use auto_triage::{AutoTriageAppliedEvent, AutoTriageInitiatedEvent, AutoTriageMission, AutoTriageState};
pub use autonomy::{AutonomyMode, DoctrineMode};
pub use behavior_tree::{BehaviorAction, BehaviorTreeLayer, BtNode};
pub use bot_memory::{
    AllyMemoryRecord, BotMemory, PerceptionCell, RecentEvent, RecentEventKind, RecentEventRing, ThreatMemoryRecord,
    ThreatWeaponClass, ALLY_MEMORY_CAPACITY, PERCEPTION_GRID_CELLS, PERCEPTION_GRID_DIM, RECENT_EVENTS_RING_DEPTH,
    THREAT_MEMORY_CAPACITY,
};
pub use cf_mind::{Doctrine, LlmMind, MindContext, NullLlmMind};
pub use constants::{
    seconds_to_ticks_for, CHATTER_COOLDOWN_SECONDS, ENGINEER_AUTO_REPAIR_FIRST_TICK_SECONDS,
    ENGINEER_AUTO_REPAIR_REACH_SECONDS, MEDIC_AUTO_TRIAGE_APPLY_SECONDS, MEDIC_AUTO_TRIAGE_REACH_SECONDS,
    PATROL_IDLE_MAX_SECONDS, PATROL_IDLE_MIN_SECONDS, SQUAD_COMM_RELAY_DELAY_SECONDS,
};
pub use cover_seeking::{CoverSeekingEvent, CoverSeekingReason};
pub use faction::{FactionId, FactionRelationships, RelationshipChangedEvent};
pub use friendly_fire::{FriendlyFireAvoidanceEvent, FriendlyFireKind};
pub use high_ground::HighGroundEvent;
pub use htn::{HtnGoal, HtnLayer, HtnRootGoal};
pub use llm_prior::{DoctrinePrior, LlmPriorLayer};
pub use m17_doctrine::{
    battery_retreat, evaluate_m17_doctrine, refuses_vacuum, shed_utility_equipment, thermal_retreat,
    M17DoctrineInputs, M17DoctrineReason, BATTERY_RETREAT_FRACTION, POWER_SHED_FRACTION,
    THERMAL_RETREAT_FRACTION, VACUUM_MIN_OXYGEN_SECONDS,
};
pub use patrol::{PatrolRoute, PatrolWaypointReachedEvent};
pub use personality::{MoodChangedEvent, PersonalityProfile, PersonalityTrait};
pub use priority::{PriorityTable, QuickPreset};
pub use reactive::{ReactiveDecision, ReactiveLayer};
pub use reason_label::{ReasonLabel, ReasonLabelRing, REASON_LABEL_RING_DEPTH};
pub use retreat::{effective_retreat_threshold, RetreatDecisionEvent, RetreatReason};
pub use squad_comm::{SquadCommPending, SquadCommRelayedEvent};
pub use suppression::SuppressionEvent;
pub use task::TaskType;
pub use trench_doctrine::{
    CoverDecision as TrenchCoverDecision, CoverDecisionReason as TrenchCoverDecisionReason,
    TrenchDoctrine, TrenchDoctrineConfig, TrenchDoctrineInputs,
    DOCTRINE_ID as TRENCH_DOCTRINE_ID, MAX_EXPOSURE_SECONDS as TRENCH_MAX_EXPOSURE_SECONDS,
};
pub use mg_doctrine::{
    assign_crews as assign_mg_crews, decide as mg_doctrine_decide, MgDoctrineDecision,
    MgDoctrineInputs, MgNestObservation, DOCTRINE_ID as MG_DOCTRINE_ID,
};
pub use observer_doctrine::{
    apply_decision as apply_observer_decision, decide as observer_doctrine_decide,
    mark_expired as observer_mark_expired, ttl_ticks_for as observer_ttl_ticks_for,
    ObserverDoctrineDecision, ObserverDoctrineInputs,
    DOCTRINE_ID as OBSERVER_DOCTRINE_ID,
    SPOTTER_MARK_LOS_LOSS_TTL_SECONDS, SPOTTER_MARK_TTL_SECONDS,
    SPOTTER_TARGET_MARK_ACQUISITION_BONUS as OBSERVER_TARGET_MARK_ACQUISITION_BONUS,
};
pub use engineer_doctrine::{
    decide as engineer_doctrine_decide, BreachedFortification, EngineerDoctrineDecision,
    EngineerDoctrineInputs, EnemyMineObservation, PerimeterSite,
    DOCTRINE_ID as ENGINEER_DOCTRINE_ID, ENGINEER_DOCTRINE_BREACH_HP_THRESHOLD,
    ENGINEER_DOCTRINE_LAY_MINE_FORWARD_TILES,
};
pub use anti_tank_doctrine::{
    decide as anti_tank_doctrine_decide, AntiTankDoctrineDecision, AntiTankDoctrineInputs,
    DetourReason, ObservedObstacle, ANTI_TANK_DOCTRINE_DETOUR_THRESHOLD_TILES,
    ANTI_TANK_DOCTRINE_PLOW_SUSPENSION_THRESHOLD_PERCENT,
    DOCTRINE_ID as ANTI_TANK_DOCTRINE_ID,
};
pub use thinking_stack::{
    format_task_camel, AiTickOutput, Layer, LayerKind, LayerOutput, ThinkingContext, ThinkingStack,
};
pub use utility::{base_utility, situational_bonus, ScoredTask, UtilityLayer};

// per-archetype BT re-exports.
pub use archetype_bt::{bt_for as archetype_bt_for, node_ids_for as archetype_bt_nodes, ArchetypeBtKind};
pub use commander_hop::{build_los_radial, finalize_hop, CommanderHopState, HopError, HopResult, LosRadialCandidate};
pub use formation::{
    rotate_local_to_world, world_anchor_for_slot, FormationDef, FormationKind, FormationSlot,
    SlotAssignment, SlotSolver, SquadRoleHint,
};
pub use squad_command_grammar::{
    builtin_registry as squad_verb_registry, parse_verb_invocation, try_issue as squad_try_issue,
    verb_family_label, CommandIssue, DoctrineCompatMatrix, ParsedVerb, SquadCommand as SquadGrammarCommand,
    VerbArgKind, VerbArgSpec, VerbArgValue, VerbDef, VerbFamily, VerbRegistry, VetoReason,
};
pub use squad_state::{
    BoundingEvent, BoundingPhase, BoundingState, BreachChainState, BreachChainStep, RoleAssignmentResult,
    SlotBrokenReport, SquadId, SquadState, SLOT_BROKEN_THRESHOLD_UNITS, SLOT_RESLOT_CADENCE_SECONDS,
};
pub use reactive_guard_params::ReactiveGuardParams;
pub use difficulty::DifficultyPreset;
pub use guard_state::{GuardState, GuardStateTransition, Tactic};
pub use perception::PerceptionRecord;
pub use reactive_guard::{ReactiveGuard, ReactiveGuardView};
pub use step::step;
pub use tick_io::{
    AlarmInput, EnemyTickReport, FireRecord, GuardTickInputs, MissedShotReason, PerceptionSignal,
    StuckRecoveryRecord, TacticRecord, TargetAcquiredRecord, TargetLostRecord,
};
