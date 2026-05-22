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

use cf_actor::{ActorState, Status, Vec2};
use cf_sim_core::Rng;

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
use reactive_guard_params::seconds_to_ticks;
pub use difficulty::DifficultyPreset;
pub use guard_state::{GuardState, GuardStateTransition, Tactic};
pub use perception::PerceptionRecord;
pub use reactive_guard::{ReactiveGuard, ReactiveGuardView};
pub use tick_io::{
    AlarmInput, EnemyTickReport, FireRecord, GuardTickInputs, MissedShotReason, PerceptionSignal,
    StuckRecoveryRecord, TacticRecord, TargetAcquiredRecord, TargetLostRecord,
};

/// One reactive-guard tick. Returns a structured report the engine turns into
/// recorder events; the engine is responsible for spawning the projectile and
/// applying damage when `fire.is_some()` AND `!fire.will_miss`.
#[must_use]
pub fn step(guard: &mut ReactiveGuard, inputs: GuardTickInputs<'_>, rng: &mut Rng) -> EnemyTickReport {
    let mut report = EnemyTickReport::default();

    // 1) Death check. A dead guard does nothing. A DYING guard ticks down
    //    its dwell and then transitions to Dead; while in DYING the guard
    //    cannot fire / move / re-acquire.
    //
    // exposes the full death ladder (Engaged → Dying → Dead) for the
    // replay viewer to walk.
    // M2 re-audit (2026-05-13): cause vocabulary per spec — killed_by_<id>
    // when the engine recorded a damage source; falls back to "killed_by_unknown"
    // when no source is available.
    let killed_by_cause = match inputs.last_damage_source {
        Some(id) => format!("killed_by_{id}"),
        None => "killed_by_unknown".to_string(),
    };
    if inputs.self_actor.status == Status::Dead || guard.state == GuardState::Dead {
        if guard.state != GuardState::Dead {
            let prev = guard.state;
            guard.state = GuardState::Dead;
            report.state_changes.push(GuardStateTransition {
                previous: prev,
                next: GuardState::Dead,
                cause: "dying_dwell_elapsed".to_string(),
            });
        }
        return report;
    }
    if inputs.self_actor.status == Status::Dying || guard.state == GuardState::Dying {
        if guard.state != GuardState::Dying {
            let prev = guard.state;
            guard.state = GuardState::Dying;
            guard.dying_dwell_remaining_ticks = guard.params.dying_dwell_ticks(inputs.tick_rate_hz);
            report.state_changes.push(GuardStateTransition {
                previous: prev,
                next: GuardState::Dying,
                cause: killed_by_cause.clone(),
            });
            return report;
        }
        if guard.dying_dwell_remaining_ticks > 0 {
            guard.dying_dwell_remaining_ticks -= 1;
            if guard.dying_dwell_remaining_ticks == 0 {
                let prev = guard.state;
                guard.state = GuardState::Dead;
                report.state_changes.push(GuardStateTransition {
                    previous: prev,
                    next: GuardState::Dead,
                    cause: "dying_dwell_elapsed".to_string(),
                });
            }
        }
        return report;
    }
    // HP=0 with status not yet DYING (e.g. tutorial_safety policy demoted
    // the kill into the body machine but we observed it pre-promotion):
    // synthesise the transition AI-side so AI surface stays ahead of the
    // body's DYING gate.
    if inputs.self_actor.hp <= 0.0 && guard.state != GuardState::Dying {
        let prev = guard.state;
        guard.state = GuardState::Dying;
        guard.dying_dwell_remaining_ticks = guard.params.dying_dwell_ticks(inputs.tick_rate_hz);
        report.state_changes.push(GuardStateTransition {
            previous: prev,
            next: GuardState::Dying,
            cause: killed_by_cause,
        });
        return report;
    }

    // Clear per-tick latches.
    guard.heard_alarm_this_tick = None;

    // closest source so guards with multiple simultaneous alarms produce a
    // deterministic single perception_signal.
    if guard.params.hearing_radius > 0.0 && !inputs.alarms.is_empty() {
        let self_pos = inputs.self_actor.position;
        let mut closest: Option<(f32, &AlarmInput)> = None;
        for alarm in inputs.alarms {
            let dx = alarm.source_position[0] - self_pos.x;
            let dy = alarm.source_position[1] - self_pos.y;
            let dist = (dx * dx + dy * dy).sqrt();
            // The alarm's loudness_radius is the source's outer envelope.
            // The guard's hearing_radius is the listener's inner envelope.
            // Hearing fires when dist ≤ MIN(alarm.loudness_radius, guard.hearing_radius).
            let effective_radius = alarm.loudness_radius.min(guard.params.hearing_radius);
            if dist <= effective_radius && closest.as_ref().is_none_or(|(d, _)| dist < *d) {
                closest = Some((dist, alarm));
            }
        }
        if let Some((dist, alarm)) = closest {
            // Hearing confidence decays linearly with distance: full at the
            // source, zero at the guard's hearing_radius.
            let confidence = if guard.params.hearing_radius > 0.0 {
                (1.0 - dist / guard.params.hearing_radius).clamp(0.0, 1.0)
            } else {
                0.0
            };
            guard.heard_alarm_this_tick = Some(alarm.source_position);
            guard.last_player_position = Some(alarm.source_position);
            guard.memory_last_refresh_tick = Some(inputs.tick);
            guard.alert_dwell_remaining_ticks = guard.params.alert_dwell_ticks(inputs.tick_rate_hz);
            report.perception_signals.push(PerceptionSignal {
                kind: "hearing",
                source_actor: Some(alarm.source_actor),
                source_position: Some(alarm.source_position),
                confidence,
                tick: inputs.tick,
                // alarm event id so the engine emit can use it as
                // `parent_event_id` (M10 chain).
                alarm_event_id: alarm.alarm_event_id.clone(),
            });
            // Hearing-without-LOS transitions Idle → Alert with reason
            // `"heard_shot"` (AI-H-01 contract). Guards already in Alert
            // / Engaged stay in their current state; the alarm refreshes
            // the alert_dwell timer above.
            if guard.state == GuardState::Idle {
                guard.state = GuardState::Alert;
                report.state_changes.push(GuardStateTransition {
                    previous: GuardState::Idle,
                    next: GuardState::Alert,
                    cause: "heard_shot".to_string(),
                });
            }
        }
    }

    // 2) Tick down cooldowns. Capture pre-decrement values for `alert_dwell_remaining_ticks`
    //    and `burst_pause_remaining_ticks` so that the state-machine + tactic checks below
    //    compare against the value the previous tick LEFT (not the value AFTER decrementing
    //    on this tick). Without this, `alert_dwell_seconds * tick_rate_hz = D` produces a
    //    D-1 effective dwell because the SET-tick's value is decremented before any check
    //    on the following tick. Same fix for burst_pause so the firing/scoring gates honor
    //    the configured pause duration end-to-end.
    let prev_alert_dwell_remaining_ticks = guard.alert_dwell_remaining_ticks;
    let prev_burst_pause_remaining_ticks = guard.burst_pause_remaining_ticks;
    decrement(&mut guard.fire_cooldown_ticks, 1);
    decrement(&mut guard.aim_settle_remaining_ticks, 1);
    decrement(&mut guard.burst_pause_remaining_ticks, 1);
    decrement(&mut guard.alert_dwell_remaining_ticks, 1);

    // 3) Reload progress.
    if guard.reload_remaining_ticks > 0 {
        guard.reload_remaining_ticks -= 1;
        if guard.reload_remaining_ticks == 0 {
            guard.ammo_in_mag = guard.params.mag_capacity;
            guard.burst_shots_fired = 0;
            report.reload_completed = true;
        }
    }

    // 4) Perception. The guard sees the player when:
    //    - Player exists and is alive.
    //    - Distance ≤ sight_radius.
    //    - Angle from the guard's facing direction ≤ sight_cone / 2.
    let perception = compute_perception(guard, &inputs);
    report.perception.clone_from(&perception);

    // `ai.perception_signal.count` / `last.payload.kind=sight`. Sight signals
    // fire every tick the guard sees the player; sight_lost fires once on
    // the transition tick.
    let player_visible_now = perception.as_ref().is_some_and(|p| p.player_seen);
    let player_was_visible = guard
        .last_player_seen_tick
        .is_some_and(|t| t == inputs.tick.saturating_sub(1));
    if let Some(p) = &perception {
        if p.player_seen {
            report.perception_signals.push(PerceptionSignal {
                kind: "sight",
                source_actor: inputs.player.map(|pl| pl.id.0),
                source_position: p.last_seen_position,
                confidence: 1.0,
                tick: inputs.tick,
                alarm_event_id: None,
            });
        } else if player_was_visible {
            report.perception_signals.push(PerceptionSignal {
                kind: "sight_lost",
                source_actor: inputs.player.map(|pl| pl.id.0),
                source_position: p.last_seen_position,
                confidence: 0.0,
                tick: inputs.tick,
                alarm_event_id: None,
            });
        }
    }

    // for `memory_decay_ticks` AND there's no fresh perception this tick,
    // purge the memory and emit a `memory_decayed` signal.
    if guard.params.memory_decay_ticks > 0 && !player_visible_now && guard.heard_alarm_this_tick.is_none() {
        if let Some(last_refresh) = guard.memory_last_refresh_tick {
            let age = inputs.tick.saturating_sub(last_refresh);
            if age >= u64::from(guard.params.memory_decay_ticks) && guard.last_player_position.is_some() {
                let pos = guard.last_player_position.take();
                guard.memory_last_refresh_tick = None;
                report.perception_signals.push(PerceptionSignal {
                    kind: "memory_decayed",
                    source_actor: inputs.player.map(|pl| pl.id.0),
                    source_position: pos,
                    confidence: 0.0,
                    tick: inputs.tick,
                    alarm_event_id: None,
                });
            }
        }
    }

    // 5) State machine. Transitions are reason-labelled so the recorder cause
    //    chain stays semantically valid.
    //
    // gate so a sighting at low hp keeps the guard in Retreating (it can
    // still engage from Retreating, but the state surface reflects the
    // wound). Recover at recover_hp_pct (hysteresis vs retreat_hp_pct).
    let hp_pct = if guard.max_hp > 0.0 {
        inputs.self_actor.hp / guard.max_hp
    } else {
        1.0
    };
    let should_retreat = hp_pct < guard.params.retreat_hp_pct;
    if should_retreat && guard.state != GuardState::Retreating {
        if matches!(guard.state, GuardState::Engaged | GuardState::Alert | GuardState::Idle) {
            let prev = guard.state;
            guard.state = GuardState::Retreating;
            report.state_changes.push(GuardStateTransition {
                previous: prev,
                next: GuardState::Retreating,
                cause: "low_hp".to_string(),
            });
        }
    } else if !should_retreat && hp_pct >= guard.params.recover_hp_pct && guard.state == GuardState::Retreating {
        let prev = guard.state;
        guard.state = if player_visible_now {
            GuardState::Engaged
        } else {
            GuardState::Alert
        };
        report.state_changes.push(GuardStateTransition {
            previous: prev,
            next: guard.state,
            cause: "hp_recovered".to_string(),
        });
    }
    if let Some(p) = &perception {
        if p.player_seen {
            guard.last_player_seen_tick = Some(inputs.tick);
            guard.last_player_position = p.last_seen_position;
            guard.memory_last_refresh_tick = Some(inputs.tick);
            guard.alert_dwell_remaining_ticks = guard.params.alert_dwell_ticks(inputs.tick_rate_hz);
            // sighting (Idle → Alert), not on every per-tick refresh while
            // already Alert/Engaged — otherwise the countdown never reaches
            // zero and Alert → Engaged never fires. Arming moves into the
            // `prev == GuardState::Idle` branch below.
            let prev = guard.state;
            // Retreating (do NOT auto-promote to Engaged). The hp gate above
            // already promoted back to Engaged when hp recovered.
            if guard.state != GuardState::Retreating {
                // Idle → Alert (cause="saw_player_in_cone") on first
                // sighting, THEN Alert → Engaged (cause="target_acquired")
                // after the aim-settle window elapses. Previously the code
                // jumped Idle → Engaged in a single tick.
                if prev == GuardState::Idle {
                    // Idle → Alert: arm aim_settle so Alert → Engaged
                    // promotion gates on it.
                    guard.state = GuardState::Alert;
                    guard.aim_settle_remaining_ticks = guard.params.aim_settle_ticks(inputs.tick_rate_hz);
                    report.state_changes.push(GuardStateTransition {
                        previous: GuardState::Idle,
                        next: GuardState::Alert,
                        cause: "saw_player_in_cone".to_string(),
                    });
                } else if prev == GuardState::Alert && guard.aim_settle_remaining_ticks == 0 {
                    // Alert → Engaged when aim_settle is done.
                    guard.state = GuardState::Engaged;
                    report.state_changes.push(GuardStateTransition {
                        previous: GuardState::Alert,
                        next: GuardState::Engaged,
                        cause: "target_acquired".to_string(),
                    });
                    if let Some(player) = inputs.player {
                        report.target_acquired = Some(TargetAcquiredRecord {
                            target_actor: player.id.0,
                            via: "sight",
                        });
                    }
                } else if prev == GuardState::Retreating || prev == GuardState::Engaged {
                    // Retreating already handled by the early gate; falling
                    // through here means stay-Engaged (no transition).
                    guard.state = GuardState::Engaged;
                }
            }
        } else if prev_alert_dwell_remaining_ticks > 0 {
            let prev = guard.state;
            if guard.state == GuardState::Engaged {
                guard.state = GuardState::Alert;
                if prev != GuardState::Alert {
                    report.state_changes.push(GuardStateTransition {
                        previous: prev,
                        next: GuardState::Alert,
                        cause: "target_lost".to_string(),
                    });
                    if let Some(player) = inputs.player {
                        report.target_lost = Some(TargetLostRecord {
                            target_actor: player.id.0,
                            reason: "los_blocked",
                        });
                    }
                }
            }
        } else if guard.state != GuardState::Idle && guard.state != GuardState::Retreating {
            let prev = guard.state;
            guard.state = GuardState::Idle;
            report.state_changes.push(GuardStateTransition {
                previous: prev,
                next: GuardState::Idle,
                cause: "alert_expired".to_string(),
            });
        }
    }

    // Engaged AND can't see the player, increment stuck_ticks. Reset when
    // the player is visible OR when the guard fires successfully OR when
    // memory decays. When stuck_ticks crosses 60 (1 second @60Hz) the
    // engine emits ai.stuck_state_changed + ai.recovery_action and the
    // counter resets. Recovery action is `wait_then_search` at M1.5;
    // M2+ adds `dig_through` when chunked-terrain pathing lands.
    if matches!(
        guard.state,
        GuardState::Alert | GuardState::Engaged | GuardState::Retreating
    ) && !player_visible_now
    {
        guard.stuck_ticks = guard.stuck_ticks.saturating_add(1);
        if guard.stuck_ticks >= 60 && !guard.stuck_recovery_latched {
            guard.stuck_recovery_latched = true;
            report.stuck_recovery = Some(StuckRecoveryRecord {
                stuck_ticks: guard.stuck_ticks,
                blocker: "no_path",
                action: "wait_then_search",
                reason: "los_blocked_too_long",
            });
            // Reset so a second stuck window can fire later in the run.
            guard.stuck_ticks = 0;
        }
    } else {
        guard.stuck_ticks = 0;
        guard.stuck_recovery_latched = false;
    }

    // 6) Aim tracking. When a player is currently visible, aim straight at them.
    //    When alerted but not visible, aim at the last seen position.
    update_aim(guard, &perception, inputs.self_actor.position);

    // 7) Utility scoring → tactic choice.
    let player_visible = perception.as_ref().is_some_and(|p| p.player_seen);
    let player_distance = perception.as_ref().and_then(|p| p.distance);
    let scores = score_tactics(guard, player_visible, player_distance, prev_burst_pause_remaining_ticks);
    let (tactic, reason) = pick_tactic(guard, &scores, player_visible);
    guard.last_tactic = tactic;
    report.tactic_chosen = Some(TacticRecord {
        tactic,
        reason,
        score_attack: scores.attack,
        score_reload: scores.reload,
        score_hold: scores.hold,
        score_search: scores.search,
    });

    // 8) Apply tactic.
    match tactic {
        Tactic::Reload => {
            if guard.reload_remaining_ticks == 0 && guard.ammo_in_mag < guard.params.mag_capacity {
                guard.reload_remaining_ticks = guard.params.reload_ticks(inputs.tick_rate_hz);
                guard.fire_cooldown_ticks = 0;
                guard.burst_pause_remaining_ticks = 0;
                guard.burst_shots_fired = 0;
                report.reload_started = true;
                // fires with state=Engaged, reason='reloading' (state
                // unchanged; reason updates)" when entering reload mid-fight.
                // The transition is a same-state self-loop so M10 walkers can
                // tag the moment without inferring it from event ordering.
                if guard.state == GuardState::Engaged {
                    report.state_changes.push(GuardStateTransition {
                        previous: GuardState::Engaged,
                        next: GuardState::Engaged,
                        cause: "reloading".to_string(),
                    });
                }
            }
        }
        Tactic::Attack => {
            if let Some(fire) = try_fire(
                guard,
                inputs.self_actor,
                &perception,
                rng,
                inputs.tick_rate_hz,
                prev_burst_pause_remaining_ticks,
            ) {
                // reason label so the cause-chain viewer can render an icon
                // set rather than string-typing. The reason is bucketed
                // from the same miss_roll the rng produced, so identical
                // seeds produce identical reasons across runs.
                if fire.will_miss {
                    report.missed_shot_reason = Some(classify_miss_reason(fire.miss_roll));
                }
                report.fire = Some(fire);
            } else if guard.ammo_in_mag == 0 && guard.reload_remaining_ticks == 0 {
                report.dry_fire = true;
                guard.reload_remaining_ticks = guard.params.reload_ticks(inputs.tick_rate_hz);
                report.reload_started = true;
            }
        }
        Tactic::Hold | Tactic::Search | Tactic::AimSettle => {
            // AimSettle/Hold/Search produce no fire + no reload work this
            // tick; the per-tick aim_settle countdown runs unconditionally
            // earlier in the step body.
        }
    }

    report
}

fn decrement(value: &mut u32, by: u32) {
    if *value >= by {
        *value -= by;
    } else {
        *value = 0;
    }
}

/// Same seed → same reason. Order picked so low rolls (close to threshold)
/// favour recoil_deviation (the "your finger slipped" miss); higher rolls
/// shift toward target_moved / occlusion / lucky_dodge (the "they did
/// something" misses).
fn classify_miss_reason(miss_roll: f32) -> MissedShotReason {
    let r = miss_roll.clamp(0.0, 0.9999);
    if r < 0.25 {
        MissedShotReason::RecoilDeviation
    } else if r < 0.50 {
        MissedShotReason::TargetMoved
    } else if r < 0.75 {
        MissedShotReason::Occlusion
    } else {
        MissedShotReason::LuckyDodge
    }
}

fn compute_perception(guard: &ReactiveGuard, inputs: &GuardTickInputs<'_>) -> Option<PerceptionRecord> {
    let player = inputs.player?;
    if player.status.is_dead() {
        return Some(PerceptionRecord {
            player_seen: false,
            distance: None,
            angle_degrees: None,
            last_seen_position: guard.last_player_position,
            state: guard.state,
        });
    }
    let dx = player.position.x - inputs.self_actor.position.x;
    let dy = player.position.y - inputs.self_actor.position.y;
    let distance = ((dx * dx) + (dy * dy)).sqrt();
    if distance > guard.params.sight_radius {
        return Some(PerceptionRecord {
            player_seen: false,
            distance: Some(distance),
            angle_degrees: None,
            last_seen_position: guard.last_player_position,
            state: guard.state,
        });
    }
    let facing = if inputs.self_actor.aim != Vec2::ZERO {
        inputs.self_actor.aim.normalize_or_x()
    } else {
        Vec2::new(-1.0, 0.0)
    };
    let to_player = if distance > 1e-3 {
        Vec2::new(dx / distance, dy / distance)
    } else {
        return Some(PerceptionRecord {
            player_seen: true,
            distance: Some(distance),
            angle_degrees: Some(0.0),
            last_seen_position: Some([player.position.x, player.position.y]),
            state: guard.state,
        });
    };
    let dot = (facing.x * to_player.x + facing.y * to_player.y).clamp(-1.0, 1.0);
    let angle_rad = dot.acos();
    let angle_deg = angle_rad * 180.0 / std::f32::consts::PI;
    let half_cone = (guard.params.sight_cone_degrees / 2.0).max(0.0);
    let visible = angle_deg <= half_cone;
    Some(PerceptionRecord {
        player_seen: visible,
        distance: Some(distance),
        angle_degrees: Some(angle_deg),
        last_seen_position: if visible {
            Some([player.position.x, player.position.y])
        } else {
            guard.last_player_position
        },
        state: guard.state,
    })
}

fn update_aim(guard: &mut ReactiveGuard, perception: &Option<PerceptionRecord>, self_pos: Vec2) {
    let target = match perception {
        Some(p) if p.player_seen => p.last_seen_position,
        _ => guard.last_player_position,
    };
    if let Some([tx, ty]) = target {
        let dx = tx - self_pos.x;
        let dy = ty - self_pos.y;
        let len = ((dx * dx) + (dy * dy)).sqrt();
        if len > 1e-3 {
            guard.aim = [dx / len, dy / len];
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct TacticScores {
    attack: f32,
    reload: f32,
    hold: f32,
    search: f32,
}

fn score_tactics(
    guard: &ReactiveGuard,
    player_visible: bool,
    player_distance: Option<f32>,
    prev_burst_pause_remaining_ticks: u32,
) -> TacticScores {
    let mut scores = TacticScores::default();
    let ammo_ratio = if guard.params.mag_capacity == 0 {
        0.0
    } else {
        guard.ammo_in_mag as f32 / guard.params.mag_capacity as f32
    };
    let reloading = guard.reload_remaining_ticks > 0;

    // Reload: high when low on ammo and not reloading; impossible while reloading.
    if reloading {
        scores.reload = -1.0;
    } else if ammo_ratio <= 0.0 {
        scores.reload = 1.0;
    } else if ammo_ratio < 0.34 {
        scores.reload = 0.6;
    } else {
        scores.reload = 0.05;
    }

    // Attack: requires visibility + ammo + cooldown clear; weighted by distance.
    if player_visible && guard.ammo_in_mag > 0 && guard.fire_cooldown_ticks == 0 && !reloading {
        let distance_pull = match player_distance {
            Some(d) => {
                let normalized = (1.0 - (d / guard.params.sight_radius)).clamp(0.0, 1.0);
                0.4 + 0.6 * normalized
            }
            None => 0.6,
        };
        let burst_penalty = if prev_burst_pause_remaining_ticks > 0 {
            -0.5
        } else {
            0.0
        };
        let aim_penalty = if guard.aim_settle_remaining_ticks > 0 {
            -0.25
        } else {
            0.0
        };
        scores.attack = (distance_pull + burst_penalty + aim_penalty).clamp(-1.0, 1.0);
    }

    // Hold: baseline non-zero so a guard with no tactic doesn't sit at score 0.0.
    scores.hold = 0.1;

    // Search: small positive when alerted-without-sight.
    if guard.state == GuardState::Alert && !player_visible {
        scores.search = 0.3;
    }

    scores
}

fn pick_tactic(guard: &ReactiveGuard, scores: &TacticScores, player_visible: bool) -> (Tactic, &'static str) {
    if guard.reload_remaining_ticks > 0 {
        return (Tactic::Reload, "reload_in_progress");
    }
    // active AND the player is visible, the chosen tactic is `aim_settle`
    // with reason `initial_acquisition`. This gates BEFORE the magazine /
    // attack / search ladder.
    if guard.aim_settle_remaining_ticks > 0 && player_visible {
        return (Tactic::AimSettle, "initial_acquisition");
    }
    if guard.ammo_in_mag == 0 {
        return (Tactic::Reload, "magazine_empty");
    }
    let mut best = (Tactic::Hold, scores.hold, "hold_default");
    if scores.attack > best.1 {
        best = (Tactic::Attack, scores.attack, "attack_target");
    }
    if scores.reload > best.1 {
        best = (Tactic::Reload, scores.reload, "low_ammo");
    }
    if scores.search > best.1 {
        best = (Tactic::Search, scores.search, "search_alerted");
    }
    (best.0, best.2)
}

fn try_fire(
    guard: &mut ReactiveGuard,
    self_actor: &ActorState,
    perception: &Option<PerceptionRecord>,
    rng: &mut Rng,
    tick_rate_hz: u32,
    prev_burst_pause_remaining_ticks: u32,
) -> Option<FireRecord> {
    if guard.aim_settle_remaining_ticks > 0 {
        return None;
    }
    if guard.fire_cooldown_ticks > 0 {
        return None;
    }
    if prev_burst_pause_remaining_ticks > 0 {
        return None;
    }
    if guard.ammo_in_mag == 0 {
        return None;
    }
    let player_visible = perception.as_ref().is_some_and(|p| p.player_seen);
    if !player_visible {
        return None;
    }
    let aim_unit = Vec2::new(guard.aim[0], guard.aim[1]).normalize_or_x();
    let muzzle = [
        self_actor.position.x + aim_unit.x * guard.params.muzzle_forward_offset,
        self_actor.position.y + guard.params.muzzle_vertical_offset + aim_unit.y * guard.params.muzzle_forward_offset,
    ];
    // Miss roll: deterministic from the engine RNG so replays match. We pull one
    // u64 and project its high 53 bits onto [0, 1). `u64::MAX as f64` would round
    // up to 2^64 (f64 has only 52 mantissa bits), so the largest u64 values would
    // produce exactly 1.0 and let `miss_chance == 1.0` ("always miss") still hit.
    let raw = rng.next_u64();
    let unit_roll = ((raw >> 11) as f64 / ((1u64 << 53) as f64)) as f32;
    let miss_threshold = guard.params.miss_chance.clamp(0.0, 1.0);
    // f32's ~24-bit mantissa cannot represent values strictly between (1 - 2^-24)
    // and 1.0, so `unit_roll` can still round up to 1.0 even from the 53-bit
    // source. Treat `miss_chance >= 1.0` as a guaranteed miss to honor the
    // documented `[0, 1]` contract.
    let will_miss = miss_threshold >= 1.0 || unit_roll < miss_threshold;
    let velocity = if will_miss {
        // Drift the projectile a fixed angular amount — enough to miss a 16-wide
        // actor at the maximum sight radius. The drift sign alternates by burst
        // shot index so misses are visually varied.
        let drift: f32 = 0.18
            * if guard.burst_shots_fired.is_multiple_of(2) {
                1.0
            } else {
                -1.0
            };
        let cos = drift.cos();
        let sin = drift.sin();
        let dx = aim_unit.x * cos - aim_unit.y * sin;
        let dy = aim_unit.x * sin + aim_unit.y * cos;
        [dx * guard.params.projectile_speed, dy * guard.params.projectile_speed]
    } else {
        [
            aim_unit.x * guard.params.projectile_speed,
            aim_unit.y * guard.params.projectile_speed,
        ]
    };
    guard.ammo_in_mag = guard.ammo_in_mag.saturating_sub(1);
    guard.burst_shots_fired += 1;
    guard.fire_cooldown_ticks = seconds_to_ticks(0.20, tick_rate_hz);
    if guard.burst_shots_fired >= guard.params.burst_shots {
        guard.burst_pause_remaining_ticks = guard.params.burst_pause_ticks(tick_rate_hz);
        guard.burst_shots_fired = 0;
    }
    let lifetime_ticks = guard.params.projectile_lifetime_ticks(tick_rate_hz);
    Some(FireRecord {
        muzzle_origin: muzzle,
        velocity,
        aim: [aim_unit.x, aim_unit.y],
        damage: guard.params.damage_per_hit,
        miss_roll: unit_roll,
        miss_threshold,
        will_miss,
        lifetime_ticks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_actor::{Inventory, InventoryItem, ItemSlot};

    fn guard_actor() -> ActorState {
        let inv = Inventory {
            items: vec![InventoryItem::Empty; 4],
            selected: ItemSlot(0),
        };
        let mut a = ActorState::player(ActorId(2), "red", Vec2::new(900.0, 32.0), 80.0, inv);
        a.controllable = false;
        a.aim = Vec2::new(-1.0, 0.0);
        a
    }

    fn player_actor(x: f32, y: f32) -> ActorState {
        ActorState::player(ActorId(1), "blue", Vec2::new(x, y), 100.0, Inventory::default())
    }

    fn rng() -> Rng {
        Rng::from_seed(13)
    }

    fn tick_inputs<'a>(tick: u64, guard_a: &'a ActorState, player: Option<&'a ActorState>) -> GuardTickInputs<'a> {
        GuardTickInputs {
            tick,
            tick_rate_hz: 60,
            self_actor: guard_a,
            player,
            alarms: &[],
            last_damage_source: player.map(|p| p.id.0),
        }
    }

    fn tick_inputs_with_alarms<'a>(
        tick: u64,
        guard_a: &'a ActorState,
        player: Option<&'a ActorState>,
        alarms: &'a [AlarmInput],
    ) -> GuardTickInputs<'a> {
        GuardTickInputs {
            tick,
            tick_rate_hz: 60,
            self_actor: guard_a,
            player,
            alarms,
            last_damage_source: player.map(|p| p.id.0),
        }
    }

    #[test]
    fn idle_when_player_not_present() {
        let mut guard = ReactiveGuard::new(ActorId(2), ReactiveGuardParams::default());
        let actor = guard_actor();
        let mut rng = rng();
        let report = step(&mut guard, tick_inputs(1, &actor, None), &mut rng);
        assert_eq!(guard.state, GuardState::Idle);
        assert!(report.fire.is_none());
        assert!(report.tactic_chosen.is_some());
    }

    #[test]
    fn engages_when_player_in_cone() {
        let mut guard = ReactiveGuard::new(ActorId(2), ReactiveGuardParams::default());
        let actor = guard_actor();
        let player = player_actor(700.0, 32.0);
        let mut rng = rng();
        // Idle → Alert (cause="saw_player_in_cone") AND arms aim_settle.
        let report = step(&mut guard, tick_inputs(1, &actor, Some(&player)), &mut rng);
        assert_eq!(guard.state, GuardState::Alert);
        assert!(!report.state_changes.is_empty());
        let perception = report.perception.unwrap();
        assert!(perception.player_seen);
        assert!(perception.distance.unwrap() > 0.0);
        // After aim_settle ticks elapse with the player still in cone,
        // Alert → Engaged (cause="target_acquired").
        let settle_ticks = guard.aim_settle_remaining_ticks;
        for t in 2..=(2 + settle_ticks as u64) {
            let _ = step(&mut guard, tick_inputs(t, &actor, Some(&player)), &mut rng);
        }
        assert_eq!(guard.state, GuardState::Engaged);
    }

    #[test]
    fn does_not_fire_during_aim_settle() {
        let mut guard = ReactiveGuard::new(ActorId(2), ReactiveGuardParams::default());
        let actor = guard_actor();
        let player = player_actor(700.0, 32.0);
        let mut rng = rng();
        // Tick 1 starts aim settle.
        let report = step(&mut guard, tick_inputs(1, &actor, Some(&player)), &mut rng);
        assert!(report.fire.is_none());
        assert!(guard.aim_settle_remaining_ticks > 0);
    }

    #[test]
    fn fires_after_aim_settles() {
        let mut params = ReactiveGuardParams::default();
        params.miss_chance = 0.0;
        params.aim_settle_seconds = 0.05;
        let mut guard = ReactiveGuard::new(ActorId(2), params);
        let actor = guard_actor();
        let player = player_actor(700.0, 32.0);
        let mut rng = Rng::from_seed(7);
        let mut shots = 0;
        for tick in 1..=120 {
            let report = step(&mut guard, tick_inputs(tick, &actor, Some(&player)), &mut rng);
            if report.fire.is_some() {
                shots += 1;
            }
        }
        assert!(shots > 0, "guard must fire at least once after aim settle");
    }

    #[test]
    fn out_of_cone_does_not_engage() {
        let mut guard = ReactiveGuard::new(ActorId(2), ReactiveGuardParams::default());
        let mut actor = guard_actor();
        actor.aim = Vec2::new(1.0, 0.0); // Face right.
        let player = player_actor(0.0, 32.0); // Player far to the left.
        let mut rng = rng();
        let report = step(&mut guard, tick_inputs(1, &actor, Some(&player)), &mut rng);
        let perception = report.perception.unwrap();
        assert!(!perception.player_seen);
        assert_ne!(guard.state, GuardState::Engaged);
    }

    #[test]
    fn dead_actor_locks_state_to_dead() {
        let mut guard = ReactiveGuard::new(ActorId(2), ReactiveGuardParams::default());
        let mut actor = guard_actor();
        actor.hp = 0.0;
        actor.status = Status::Dead;
        let mut rng = rng();
        let report = step(&mut guard, tick_inputs(1, &actor, None), &mut rng);
        assert_eq!(guard.state, GuardState::Dead);
        assert!(!report.state_changes.is_empty());
    }

    #[test]
    fn deterministic_under_same_seed() {
        fn play_500_ticks(seed: u64) -> Vec<bool> {
            let mut params = ReactiveGuardParams::default();
            params.aim_settle_seconds = 0.05;
            let mut guard = ReactiveGuard::new(ActorId(2), params);
            let actor = guard_actor();
            let player = player_actor(700.0, 32.0);
            let mut rng = Rng::from_seed(seed);
            let mut fires = Vec::new();
            for tick in 1..=500 {
                let report = step(&mut guard, tick_inputs(tick, &actor, Some(&player)), &mut rng);
                fires.push(report.fire.is_some());
            }
            fires
        }
        let a = play_500_ticks(13);
        let b = play_500_ticks(13);
        assert_eq!(a, b, "same seed must produce identical fire pattern");
    }

    #[test]
    fn out_of_ammo_triggers_reload() {
        let mut params = ReactiveGuardParams::default();
        params.aim_settle_seconds = 0.05;
        params.miss_chance = 0.0;
        params.mag_capacity = 2;
        params.burst_shots = 2;
        params.burst_pause_seconds = 0.05;
        let mut guard = ReactiveGuard::new(ActorId(2), params);
        let actor = guard_actor();
        let player = player_actor(700.0, 32.0);
        let mut rng = rng();
        let mut reload_started = false;
        for tick in 1..=300 {
            let report = step(&mut guard, tick_inputs(tick, &actor, Some(&player)), &mut rng);
            if report.reload_started {
                reload_started = true;
                break;
            }
        }
        assert!(reload_started);
    }

    #[test]
    fn reset_returns_full_mag_and_idle() {
        let mut guard = ReactiveGuard::new(ActorId(2), ReactiveGuardParams::default());
        guard.ammo_in_mag = 0;
        guard.state = GuardState::Engaged;
        guard.reload_remaining_ticks = 30;
        guard.reset();
        assert_eq!(guard.state, GuardState::Idle);
        assert_eq!(guard.ammo_in_mag, ReactiveGuardParams::default().mag_capacity);
        assert_eq!(guard.reload_remaining_ticks, 0);
    }

    /// Regression: prior to this fix, `alert_dwell_remaining_ticks` was decremented
    /// at the top of `step()` BEFORE the state-machine check, so configuring
    /// `alert_dwell_seconds * tick_rate_hz = D` produced D-1 ticks of Alert
    /// dwell instead of D. Bugbot ID cf33d096-95e2-4104-bfe8-c9127c660223.
    #[test]
    fn alert_dwell_lasts_full_configured_duration_after_player_lost() {
        let mut params = ReactiveGuardParams::default();
        params.alert_dwell_seconds = 0.05; // 0.05 * 60 = 3 ticks of Alert.
        let mut guard = ReactiveGuard::new(ActorId(2), params);
        let actor = guard_actor();
        let player_visible = player_actor(700.0, 32.0);
        // Out-of-cone player so perception still runs (player_seen=false).
        // Sight radius default is 700 in cf-ai params; place far behind the guard.
        let player_lost = player_actor(2000.0, 32.0);
        let mut rng = rng();

        // Alert (not Engaged) so the alert_dwell test starts from Alert
        // with the dwell pre-armed. The semantics carry through unchanged
        // for the loss path.
        let _ = step(&mut guard, tick_inputs(1, &actor, Some(&player_visible)), &mut rng);
        assert_eq!(guard.state, GuardState::Alert);
        assert_eq!(guard.alert_dwell_remaining_ticks, 3);

        // Tick 2: player out-of-sight -> dwell decrements to 2, prev=3 > 0 keeps Alert.
        let _ = step(&mut guard, tick_inputs(2, &actor, Some(&player_lost)), &mut rng);
        assert_eq!(guard.state, GuardState::Alert);

        // Tick 3: dwell decrements to 1, prev=2 > 0 keeps Alert.
        let _ = step(&mut guard, tick_inputs(3, &actor, Some(&player_lost)), &mut rng);
        assert_eq!(guard.state, GuardState::Alert);

        // Tick 4: dwell decrements to 0, prev=1 > 0 keeps Alert (third tick of dwell).
        let _ = step(&mut guard, tick_inputs(4, &actor, Some(&player_lost)), &mut rng);
        assert_eq!(guard.state, GuardState::Alert);

        // Tick 5: dwell stays at 0, prev=0 fails the > 0 check -> transitions to Idle.
        let _ = step(&mut guard, tick_inputs(5, &actor, Some(&player_lost)), &mut rng);
        assert_eq!(guard.state, GuardState::Idle);
    }

    /// Regression: same off-by-one decrement-before-check pattern affected
    /// `burst_pause_remaining_ticks` so a configured pause of D ticks gated
    /// firing for only D-1 ticks. Bugbot ID cf33d096-95e2-4104-bfe8-c9127c660223.
    ///
    /// `try_fire` always sets `fire_cooldown_ticks` to `seconds_to_ticks(0.20, 60) = 12`
    /// after a successful shot. We use `burst_pause_seconds = 0.30` (18 ticks)
    /// so the pause duration is strictly longer than the fire cooldown — the
    /// last 6 blocked ticks are isolated to burst_pause alone, which is what
    /// this test exercises.
    #[test]
    fn burst_pause_blocks_fire_for_full_configured_duration() {
        let mut params = ReactiveGuardParams::default();
        params.aim_settle_seconds = 0.0;
        params.miss_chance = 0.0;
        params.mag_capacity = 10;
        params.burst_shots = 1;
        params.burst_pause_seconds = 0.30; // 18 ticks of pause; > 12-tick fire cooldown.
        let mut guard = ReactiveGuard::new(ActorId(2), params);
        let actor = guard_actor();
        let player = player_actor(700.0, 32.0);
        let mut rng = Rng::from_seed(7);

        // Tick 1: aim_settle = 0 means instant settle, guard fires immediately and
        // burst_pause SETS to 18.
        let r1 = step(&mut guard, tick_inputs(1, &actor, Some(&player)), &mut rng);
        assert!(r1.fire.is_some(), "tick 1: zero aim_settle, must fire");
        assert_eq!(guard.burst_pause_remaining_ticks, 18);

        // Ticks 2-19: burst_pause must block for the full 18-tick duration.
        // (Ticks 2-13 are also blocked by fire_cooldown=12; ticks 14-19 are
        // blocked by burst_pause alone since fire_cooldown has cleared by then.)
        for tick in 2..=19 {
            let r = step(&mut guard, tick_inputs(tick, &actor, Some(&player)), &mut rng);
            assert!(
                r.fire.is_none(),
                "tick {tick}: burst_pause should block fire for the full 18-tick configured duration"
            );
        }

        // Tick 20: pause expired (prev=0); fire_cooldown also clear; guard fires again.
        let r20 = step(&mut guard, tick_inputs(20, &actor, Some(&player)), &mut rng);
        assert!(
            r20.fire.is_some(),
            "tick 20: pause + cooldown expired, fire should resume"
        );
    }

    /// and transitions Idle → Alert with reason="heard_shot" + perception
    /// signal kind="hearing".
    #[test]
    fn ai_h_01_sentry_hears_threat_without_los() {
        let mut params = ReactiveGuardParams::default();
        params.hearing_radius = 480.0;
        let mut guard = ReactiveGuard::new(ActorId(2), params);
        let actor = guard_actor();
        let mut rng = rng();
        // Player NOT in sight cone (this fixture has guard facing left and
        // the player isn't passed) — pure hearing path.
        let alarms = [AlarmInput {
            source_actor: 1,
            source_position: [actor.position.x + 200.0, actor.position.y],
            loudness_radius: 480.0,
            alarm_event_id: None,
        }];
        let report = step(&mut guard, tick_inputs_with_alarms(1, &actor, None, &alarms), &mut rng);
        assert_eq!(guard.state, GuardState::Alert);
        let transitioned = report
            .state_changes
            .first()
            .cloned()
            .expect("state must change on heard_shot");
        assert_eq!(transitioned.previous, GuardState::Idle);
        assert_eq!(transitioned.next, GuardState::Alert);
        assert_eq!(transitioned.cause, "heard_shot");
        let hearing = report
            .perception_signals
            .iter()
            .find(|s| s.kind == "hearing")
            .expect("hearing perception_signal must fire");
        assert_eq!(hearing.source_actor, Some(1));
        assert!(hearing.confidence > 0.0 && hearing.confidence <= 1.0);
    }

    /// identical seeds produce identical reasons.
    #[test]
    fn classify_miss_reason_buckets_are_stable() {
        assert_eq!(classify_miss_reason(0.0), MissedShotReason::RecoilDeviation);
        assert_eq!(classify_miss_reason(0.24), MissedShotReason::RecoilDeviation);
        assert_eq!(classify_miss_reason(0.26), MissedShotReason::TargetMoved);
        assert_eq!(classify_miss_reason(0.49), MissedShotReason::TargetMoved);
        assert_eq!(classify_miss_reason(0.51), MissedShotReason::Occlusion);
        assert_eq!(classify_miss_reason(0.74), MissedShotReason::Occlusion);
        assert_eq!(classify_miss_reason(0.76), MissedShotReason::LuckyDodge);
        assert_eq!(classify_miss_reason(0.99), MissedShotReason::LuckyDodge);
    }

    #[test]
    fn low_hp_transitions_to_retreating() {
        let mut params = ReactiveGuardParams::default();
        params.retreat_hp_pct = 0.5;
        let mut guard = ReactiveGuard::new(ActorId(2), params);
        guard.max_hp = 100.0;
        let mut actor = guard_actor();
        actor.hp = 40.0; // 40% < 50% retreat gate
        let player = player_actor(80.0, 32.0); // visible to start
        let mut rng = rng();
        let report = step(&mut guard, tick_inputs(1, &actor, Some(&player)), &mut rng);
        assert_eq!(guard.state, GuardState::Retreating);
        let transitioned = report.state_changes.first().cloned().expect("hp gate must transition");
        assert_eq!(transitioned.cause, "low_hp");
        assert_eq!(transitioned.next, GuardState::Retreating);
    }
}
