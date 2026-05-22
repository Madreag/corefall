//! Dispatch handlers for cfctl commands.
//!
//! Extracted from engine.rs.

#![allow(unused_imports, dead_code, clippy::too_many_arguments)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use chrono::{DateTime, Utc};
use serde_json::{json, Value};

use cf_actor::sim::{step as actor_step, ActorSimState, ActorTickOutcome, StepDeps, StepReport};
use cf_actor::{ActorId, ActorWorld, ControlIntent, IntentSource, ItemSlot, Vec2};
use cf_replay::{
    diagnostics, ArtifactItem, BuildInfo, BundleInputs, CapabilitiesBlock, CaptureConfig,
    ChecksumConfig, PerfSample, Recorder, RunManifest, SceneInfo, SettingsBlock, TestRecord,
    CONTROL_SCHEMA_VERSION, EVENT_ENVELOPE_VERSION, MANIFEST_SCHEMA_VERSION, SCENARIO_SCHEMA_VERSION,
};
use cf_sim_core::{
    checksum::{sim_state_v1, CHECKSUM_ALGORITHM, CHECKSUM_SCOPE},
    ids::{iso_hyphen_safe, make_run_id},
    Rng, SimClock, SimConfig, Tick, WallClock,
};

use crate::engine::*;
use crate::scenario::Scenario;
use crate::server::{async_trait, CommandResult, ControlCommand, EngineHandle, SettingsPatch};
use crate::state::{ActorView, ObserveFrame, ObserveSettings, RunStatus};
use crate::{Settings, SCHEMA_VERSION};

impl M0Engine {

    pub(crate) fn dispatch_squad_command(
        &self,
        bot_actor: Option<u64>,
        kind: crate::m6_actions::SquadCommandKindOverWire,
        waypoint: Option<(f32, f32)>,
        source: cf_actor::IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: std::sync::RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        if !self.config.has_actor_world {
            return self.reject_actor_command(tick, sim_time_ms, state, "act.squad.issue_command");
        }
        let mut state = state;
        let issuer = state.player_actor.unwrap_or_default();
        // broadcast to all followers when `bot_actor` is None). This
        // mutates `state.squad` so the AI tick can consult
        // `member.current_command` and act accordingly.
        let waypoint_v2 = waypoint.map(|(x, y)| cf_actor::Vec2::new(x, y));
        let command_kind = match kind {
            crate::m6_actions::SquadCommandKindOverWire::FollowLeader => cf_squad::SquadCommandKind::FollowLeader,
            crate::m6_actions::SquadCommandKindOverWire::HoldPosition => cf_squad::SquadCommandKind::HoldPosition,
            crate::m6_actions::SquadCommandKindOverWire::DefendPoint => cf_squad::SquadCommandKind::DefendPoint,
            crate::m6_actions::SquadCommandKindOverWire::PushToWaypoint => cf_squad::SquadCommandKind::PushToWaypoint,
        };
        let command = cf_squad::SquadCommand {
            kind: command_kind,
            waypoint: waypoint_v2,
            issuer,
        };
        let applied = if let Some(target_id) = bot_actor {
            state.squad.issue_command(cf_actor::ActorId(target_id), command.clone())
        } else {
            state.squad.broadcast_to_followers(&command) > 0
        };
        drop(state);
        let payload = json!({
            "method": "act.squad.issue_command",
            "bot_actor": bot_actor,
            "kind": kind.as_str(),
            "waypoint": waypoint.map(|(x, y)| json!({"x": x, "y": y})),
            "applied": applied,
        });
        self.recorder
            .record(tick, sim_time_ms, "control", "command_accepted", payload.clone(), None);
        let squad_event = json!({
            "bot_actor": bot_actor,
            "kind": kind.as_str(),
            "waypoint": waypoint.map(|(x, y)| json!({"x": x, "y": y})),
        });
        self.recorder
            .record(tick, sim_time_ms, "squad", "command_issued", squad_event, None);
        CommandResult::accepted(tick.0)
    }

    /// to the default `FollowLeader` command. Re-emits
    /// `squad.command_issued` with `kind="follow_leader"` so the replay
    /// stream stays linear.
    pub(crate) fn dispatch_squad_cancel_command(
        &self,
        actor_id: u64,
        source: cf_actor::IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: std::sync::RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        if !self.config.has_actor_world {
            return self.reject_actor_command(tick, sim_time_ms, state, "act.squad.cancel_command");
        }
        let mut state = state;
        let target = cf_actor::ActorId(actor_id);
        let updated = state
            .squad
            .issue_command(target, cf_squad::SquadCommand::follow(cf_actor::ActorId::default()));
        drop(state);
        let payload = json!({
            "method": "act.squad.cancel_command",
            "actor_id": actor_id,
            "applied": updated,
        });
        self.recorder
            .record(tick, sim_time_ms, "control", "command_accepted", payload, None);
        let squad_event = json!({
            "bot_actor": actor_id,
            "kind": cf_squad::SquadCommandKind::FollowLeader.as_str(),
            "waypoint": serde_json::Value::Null,
            "cause": "cancel_command",
        });
        self.recorder
            .record(tick, sim_time_ms, "squad", "command_issued", squad_event, None);
        CommandResult::accepted(tick.0)
    }

    /// PriorityTable AND the utility scorer's cached priority so the
    /// next AI tick scores against the new weight. Emits
    /// `ai.priority_table_changed` on success.
    pub(crate) fn dispatch_set_priority(
        &self,
        actor_id: u64,
        task: String,
        weight: u8,
        source: cf_actor::IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: std::sync::RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let actor = cf_actor::ActorId(actor_id);
        let task_type = match cf_ai::TaskType::from_str(&task) {
            Some(t) => t,
            None => {
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({"method": "act.player.set_priority", "reason": "unknown_task"}),
                    None,
                );
                return CommandResult::rejected("unknown_task", tick.0);
            }
        };
        let mut state = state;
        let result = state.m7_ai_world.set_priority(actor, task_type, weight);
        let (old_weight, new_weight) = match result {
            Ok(pair) => pair,
            Err(reason) => {
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({"method": "act.player.set_priority", "reason": reason}),
                    None,
                );
                return CommandResult::rejected(reason, tick.0);
            }
        };
        drop(state);
        self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({
                "method": "act.player.set_priority",
                "actor_id": actor_id,
                "task": task_type.as_str(),
                "weight": new_weight,
            }),
            None,
        );
        let payload = crate::m7_ai::priority_table_changed_payload(actor_id, task_type, old_weight, new_weight);
        self.recorder
            .record(tick, sim_time_ms, "ai", "priority_table_changed", payload, None);
        CommandResult::accepted(tick.0)
    }

    /// `ai.autonomy_mode_changed` on success.
    pub(crate) fn dispatch_set_autonomy_mode(
        &self,
        actor_id: u64,
        mode: String,
        source: cf_actor::IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: std::sync::RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let actor = cf_actor::ActorId(actor_id);
        let new_mode = match cf_ai::AutonomyMode::from_str(&mode) {
            Some(m) => m,
            None => {
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({"method": "act.player.set_autonomy_mode", "reason": "unknown_mode"}),
                    None,
                );
                return CommandResult::rejected("unknown_mode", tick.0);
            }
        };
        let mut state = state;
        let prev = state.m7_ai_world.set_autonomy(actor, new_mode);
        let from = match prev {
            Some(p) => p,
            None => {
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({"method": "act.player.set_autonomy_mode", "reason": "no_such_actor"}),
                    None,
                );
                return CommandResult::rejected("no_such_actor", tick.0);
            }
        };
        drop(state);
        self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({
                "method": "act.player.set_autonomy_mode",
                "actor_id": actor_id,
                "mode": new_mode.as_str(),
            }),
            None,
        );
        let payload = crate::m7_ai::autonomy_mode_changed_payload(actor_id, from, new_mode);
        self.recorder
            .record(tick, sim_time_ms, "ai", "autonomy_mode_changed", payload, None);
        CommandResult::accepted(tick.0)
    }

    /// bot's PriorityTable with the chosen role template + emits
    /// `ai.role_template_applied` AND `ai.archetype_chosen`.
    pub(crate) fn dispatch_apply_role_template(
        &self,
        actor_id: u64,
        template_id: String,
        source: cf_actor::IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: std::sync::RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let actor = cf_actor::ActorId(actor_id);
        let template = match cf_priority::RoleTemplate::from_str(&template_id) {
            Some(t) => t,
            None => {
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({"method": "act.player.apply_role_template", "reason": "unknown_template_id"}),
                    None,
                );
                return CommandResult::rejected("unknown_template_id", tick.0);
            }
        };
        let mut state = state;
        match state.m7_ai_world.apply_role_template(actor, template) {
            Some(()) => {}
            None => {
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({"method": "act.player.apply_role_template", "reason": "no_such_actor"}),
                    None,
                );
                return CommandResult::rejected("no_such_actor", tick.0);
            }
        }
        let archetype = template.archetype();
        // Try to emit chatter (OrderAck) when role template applies.
        let chatter_emit = state
            .m7_ai_world
            .try_emit_chatter(
                actor,
                cf_audio::ChatterCategory::OrderAck,
                "Roger, switching role.",
                tick.0,
                self.config.tick_rate_hz,
            )
            .map(|(event, _)| event);
        drop(state);
        self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({
                "method": "act.player.apply_role_template",
                "actor_id": actor_id,
                "template_id": template.as_str(),
            }),
            None,
        );
        let payload = crate::m7_ai::role_template_applied_payload(actor_id, template);
        self.recorder
            .record(tick, sim_time_ms, "ai", "role_template_applied", payload, None);
        let archetype_payload = crate::m7_ai::archetype_chosen_payload(actor_id, archetype);
        self.recorder
            .record(tick, sim_time_ms, "ai", "archetype_chosen", archetype_payload, None);
        if let Some(event) = chatter_emit {
            let chatter_payload = crate::m7_ai::chatter_emitted_payload(&event);
            self.recorder
                .record(tick, sim_time_ms, "ai", "chatter_emitted", chatter_payload, None);
        }
        CommandResult::accepted(tick.0)
    }

    /// weights ±2 per spec § Quick presets. Emits
    /// `ai.quick_preset_applied`.
    pub(crate) fn dispatch_apply_quick_preset(
        &self,
        actor_id: u64,
        preset_id: String,
        source: cf_actor::IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: std::sync::RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let actor = cf_actor::ActorId(actor_id);
        let preset = match cf_priority::QuickPresetId::from_str(&preset_id) {
            Some(p) => p,
            None => {
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({"method": "act.player.apply_quick_preset", "reason": "unknown_preset_id"}),
                    None,
                );
                return CommandResult::rejected("unknown_preset_id", tick.0);
            }
        };
        let mut state = state;
        match state.m7_ai_world.apply_quick_preset(actor, preset) {
            Some(()) => {}
            None => {
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({"method": "act.player.apply_quick_preset", "reason": "no_such_actor"}),
                    None,
                );
                return CommandResult::rejected("no_such_actor", tick.0);
            }
        }
        drop(state);
        self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({
                "method": "act.player.apply_quick_preset",
                "actor_id": actor_id,
                "preset_id": preset.as_str(),
            }),
            None,
        );
        let payload = crate::m7_ai::quick_preset_applied_payload(actor_id, preset);
        self.recorder
            .record(tick, sim_time_ms, "ai", "quick_preset_applied", payload, None);
        CommandResult::accepted(tick.0)
    }

    /// doctrine check, mutate squad state, emit the appropriate event.
    /// Chain-aware verbs (`stack_door` / `breach_door` / `frag_out` /
    /// `advance` / `retreat_in_order`) also drive the matching squad
    /// state machine + emit the chain events.
    pub(crate) fn dispatch_m7b_squad_issue(
        &self,
        squad_id: u64,
        verb_id: String,
        args: Vec<serde_json::Value>,
        source: cf_actor::IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: std::sync::RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let mut parsed_args = Vec::with_capacity(args.len());
        for (i, value) in args.iter().enumerate() {
            match crate::m7b_squad::parse_verb_arg(value) {
                Ok(v) => parsed_args.push(v),
                Err(err) => {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({
                            "method": "act.squad.issue",
                            "reason": "invalid_arg",
                            "verb_id": verb_id,
                            "arg_index": i,
                            "detail": err,
                        }),
                        None,
                    );
                    return CommandResult::rejected("invalid_arg", tick.0);
                }
            }
        }
        let mut state = state;
        let issuer = state.player_actor.map(|a| a.0).unwrap_or(0);
        let outcome = state
            .m7b_squad
            .issue_verb(squad_id, &verb_id, parsed_args.clone(), issuer, tick.0);
        let event_category = "squad";
        let event_type = if outcome.is_accepted() {
            "command_issued"
        } else {
            "command_vetoed"
        };
        let payload = outcome.payload.clone();
        let was_accepted = outcome.is_accepted();

        // The squad state machines (breach chain + bounding) auto-drive
        // their events here so the per-verb issue produces the correct
        // chain-step / bounding-step payloads per spec.
        let mut chain_events: Vec<(&str, serde_json::Value)> = Vec::new();
        if was_accepted {
            let stack_actor_ids: Vec<u64> = state
                .squad
                .followers
                .iter()
                .map(|m| m.actor.0)
                .chain(state.squad.leader.iter().map(|l| l.actor.0))
                .take(4)
                .collect();
            match verb_id.as_str() {
                "stack_door" => {
                    // Args: door, side.
                    let door_id = parsed_args
                        .iter()
                        .find_map(|a| match a {
                            cf_ai::VerbArgValue::Door(d) => Some(*d),
                            _ => None,
                        })
                        .unwrap_or(0);
                    let side = parsed_args
                        .iter()
                        .find_map(|a| match a {
                            cf_ai::VerbArgValue::Side(s) => Some(s.clone()),
                            _ => None,
                        })
                        .unwrap_or_else(|| "left".to_string());
                    let started =
                        state
                            .m7b_squad
                            .start_breach_chain(squad_id, door_id, &side, stack_actor_ids.clone(), tick.0);
                    chain_events.push(("breach_chain_started", started));
                }
                "breach_door" | "frag_out" | "advance" => {
                    // Each subsequent chain verb advances the chain by
                    // one step. The engine emits the matching
                    // breach_chain_step / breach_chain_complete event.
                    let res = state
                        .m7b_squad
                        .advance_breach_chain_with_actors(squad_id, tick.0, &stack_actor_ids);
                    if let Some(p) = res.step_payload {
                        chain_events.push(("breach_chain_step", p));
                    }
                    if let Some(p) = res.complete_payload {
                        chain_events.push(("breach_chain_complete", p));
                    }
                }
                "retreat_in_order" => {
                    // cover, half move 30u, swap); emits
                    // squad.bounding_step". Start the bounding sequence
                    // at the rally arg (if supplied).
                    let rally = parsed_args
                        .iter()
                        .find_map(|a| match a {
                            cf_ai::VerbArgValue::Waypoint(w) => Some(*w),
                            _ => None,
                        })
                        .unwrap_or([0.0, 0.0]);
                    if let Some(squad) = state.m7b_squad.squad_mut(squad_id) {
                        squad.start_bounding(rally, tick.0);
                    }
                    if let Some(p) = state.m7b_squad.tick_bounding(squad_id, Some(rally)) {
                        chain_events.push(("bounding_step", p));
                    }
                }
                _ => {}
            }
        }
        drop(state);
        self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({"method": "act.squad.issue", "squad_id": squad_id, "verb_id": verb_id, "accepted": was_accepted}),
            None,
        );
        self.recorder
            .record(tick, sim_time_ms, event_category, event_type, payload, None);
        for (ev, payload) in chain_events {
            self.recorder.record(tick, sim_time_ms, "squad", ev, payload, None);
        }
        CommandResult::accepted(tick.0)
    }

    /// count, run slot solver, emit `squad.formation_set` +
    /// `squad.formation_slot_assigned` per slot.
    pub(crate) fn dispatch_m7b_squad_set_formation(
        &self,
        squad_id: u64,
        formation_kind: String,
        source: cf_actor::IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: std::sync::RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let kind = match cf_ai::FormationKind::from_str(&formation_kind) {
            Some(k) => k,
            None => {
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({"method": "act.squad.set_formation", "reason": "unknown_formation_kind", "formation_kind": formation_kind}),
                    None,
                );
                return CommandResult::rejected("unknown_formation_kind", tick.0);
            }
        };
        let mut state = state;
        let commander_actor_id = state.player_actor.map(|a| a.0);
        let commander_pos = state
            .player_actor
            .and_then(|pid| state.actor_state.as_ref().and_then(|sim| sim.world.actors.get(&pid)))
            .map(|a| [a.position.x, a.position.y])
            .unwrap_or([0.0, 0.0]);
        let outcome = state
            .m7b_squad
            .set_formation(squad_id, kind, commander_pos, 0.0, commander_actor_id, tick.0);
        let mut collapse_payload: Option<serde_json::Value> = None;
        if outcome.previous != outcome.new_kind {
            let squad = state.m7b_squad.squad(squad_id);
            let member_count = squad.map(|s| s.role_assignments.len()).unwrap_or(0);
            collapse_payload = Some(json!({
                "squad_id": squad_id,
                "previous_kind": outcome.previous.as_str(),
                "new_kind": outcome.new_kind.as_str(),
                "member_count": member_count,
                "trigger_actor_id": serde_json::Value::Null,
            }));
        }
        let formation_payload = outcome.formation_payload;
        let assignment_payloads = outcome.assignment_payloads;
        drop(state);
        self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({"method": "act.squad.set_formation", "squad_id": squad_id, "formation_kind": formation_kind}),
            None,
        );
        if let Some(p) = collapse_payload {
            self.recorder
                .record(tick, sim_time_ms, "squad", "formation_collapsed", p, None);
        }
        self.recorder
            .record(tick, sim_time_ms, "squad", "formation_set", formation_payload, None);
        for p in assignment_payloads {
            self.recorder
                .record(tick, sim_time_ms, "squad", "formation_slot_assigned", p, None);
        }
        CommandResult::accepted(tick.0)
    }

    /// + emit `squad.role_assigned`.
    pub(crate) fn dispatch_m7b_squad_assign_role(
        &self,
        squad_id: u64,
        member_actor_id: u64,
        role: String,
        source: cf_actor::IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: std::sync::RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let parsed = match cf_ai::SquadRoleHint::from_str(&role) {
            Some(r) => r,
            None => {
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({"method": "act.squad.assign_role", "reason": "unknown_role", "role": role}),
                    None,
                );
                return CommandResult::rejected("unknown_role", tick.0);
            }
        };
        let mut state = state;
        let outcome = state.m7b_squad.assign_role(squad_id, member_actor_id, parsed);
        let payload = outcome.payload;
        drop(state);
        self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({"method": "act.squad.assign_role", "squad_id": squad_id, "member_actor_id": member_actor_id, "role": role}),
            None,
        );
        self.recorder
            .record(tick, sim_time_ms, "squad", "role_assigned", payload, None);
        CommandResult::accepted(tick.0)
    }

    pub(crate) fn dispatch_m7b_dump_squad_state(
        &self,
        squad_id: u64,
        source: cf_actor::IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: std::sync::RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let view = state.m7b_squad.dump_state_view(squad_id);
        drop(state);
        self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({"method": "srv.dump_squad_state", "squad_id": squad_id}),
            None,
        );
        // Stash the view in the engine for the cfctl response. The
        // cfctl-side `dump_squad_state` helper reads it back. To avoid a
        // mailbox round-trip we cheat slightly: the response path uses
        // `M0Engine::dump_squad_state` which queries the world directly.
        let _ = view;
        CommandResult::accepted(tick.0)
    }

}

