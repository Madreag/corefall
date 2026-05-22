//! async fn process_request — JSON-RPC request router
//! JSON-RPC method router. >2000 LOC by design: each match arm parses params,
//! validates schema, calls the EngineHandle async method, and formats the
//! response. Splitting per-method would duplicate envelope + error + schema-
//! version handling across many small files.

#![allow(unused_imports, dead_code, clippy::too_many_arguments)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::net::SocketAddr;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use schemars::JsonSchema;
use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot, broadcast};
use tokio::time::{sleep, timeout};
use futures_util::{SinkExt, StreamExt};

use tokio::sync::Mutex;

use cf_actor::IntentSource;

use crate::envelope::*;
use crate::schemas::*;
use crate::server::*;
use crate::server_command::*;
use crate::server_engine_handle::*;
use crate::state::*;
use crate::{Settings, SCHEMA_VERSION, SCHEMA_VERSION_MIN};

pub(crate) async fn process_request<E: EngineHandle>(
    text: &str,
    engine: &E,
    subscribe_hz: &Arc<Mutex<Option<u32>>>,
    subscribe_filter: &Arc<Mutex<Option<String>>>,
    max_observe_hz: u32,
) -> Option<String> {
    let request: JsonRpcRequest = match serde_json::from_str(text) {
        Ok(r) => r,
        Err(err) => {
            tracing::warn!(target: "cf::ctl", error = %err, "invalid jsonrpc request");
            return Some(error_response(
                JsonRpcId::Null,
                error_codes::INVALID_PARAMS,
                "InvalidRequest",
                json!({"reason": err.to_string()}),
            ));
        }
    };
    if request.jsonrpc != "2.0" {
        return Some(error_response(
            request.id,
            error_codes::INVALID_PARAMS,
            "InvalidRequest",
            json!({"reason": "jsonrpc must be \"2.0\""}),
        ));
    }

    let method = request.method.clone();
    let params = request.params.clone();
    if let Err(err) = check_schema_version(&params) {
        return Some(error_response(
            request.id,
            error_codes::INVALID_PARAMS,
            "InvalidParams",
            err,
        ));
    }

    match method.as_str() {
        "scenario.load" => {
            let p: ScenarioLoadParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ScenarioLoad {
                    scenario: p.scenario,
                    seed: p.seed,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "scenario.reset" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let result = engine.dispatch(ControlCommand::ScenarioReset).await;
            Some(ack_response(request.id, &result))
        }
        "sim.pause" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let result = engine.dispatch(ControlCommand::Pause).await;
            Some(ack_response(request.id, &result))
        }
        "sim.resume" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let result = engine.dispatch(ControlCommand::Resume).await;
            Some(ack_response(request.id, &result))
        }
        "sim.step" => {
            let p: StepParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if p.ticks == 0 {
                return Some(invalid_param_reason(request.id, "ticks_must_be_positive"));
            }
            let result = engine.dispatch(ControlCommand::Step { ticks: p.ticks }).await;
            Some(ack_response(request.id, &result))
        }
        "sim.run_for_ticks" => {
            let p: RunForTicksParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if p.ticks == 0 {
                return Some(invalid_param_reason(request.id, "ticks_must_be_positive"));
            }
            let result = engine
                .dispatch(ControlCommand::RunForTicks {
                    ticks: p.ticks,
                    write_run_bundle: p.write_run_bundle.unwrap_or(false),
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "observe.once" => {
            let p: ObserveOnceParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if p.filter.is_some() {
                return Some(invalid_param_reason(request.id, "observe_filter_not_supported_in_m0"));
            }
            let frame = engine.snapshot(p.filter.as_deref()).await;
            Some(success_response(
                request.id,
                serde_json::to_value(&frame).unwrap_or(serde_json::Value::Null),
            ))
        }
        "observe.subscribe" => {
            let p: ObserveSubscribeParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if p.filter.is_some() {
                return Some(invalid_param_reason(request.id, "observe_filter_not_supported_in_m0"));
            }
            let hz = p.hz.unwrap_or(10);
            if hz == 0 || hz > max_observe_hz {
                return Some(error_response(
                    request.id,
                    error_codes::INVALID_PARAMS,
                    "InvalidParams",
                    json!({
                        "reason": "observe_hz_out_of_range",
                        "min_hz": 1,
                        "max_hz": max_observe_hz,
                    }),
                ));
            }
            *subscribe_hz.lock().await = Some(hz);
            *subscribe_filter.lock().await = p.filter;
            Some(ack_response(request.id, &CommandResult::accepted(0)))
        }
        "observe.unsubscribe" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            *subscribe_hz.lock().await = None;
            *subscribe_filter.lock().await = None;
            Some(ack_response(request.id, &CommandResult::accepted(0)))
        }
        "observe.settings" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let settings = engine.settings_snapshot().await;
            let view = ObserveSettings {
                schema_version: SCHEMA_VERSION,
                settings,
            };
            Some(success_response(
                request.id,
                serde_json::to_value(&view).unwrap_or(serde_json::Value::Null),
            ))
        }
        "act.player.move" => {
            let p: ActPlayerMoveParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if !p.x.is_finite() || !p.y.is_finite() {
                return Some(invalid_param_reason(request.id, "non_finite"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerMove {
                    x: p.x,
                    y: p.y,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.jump" => {
            let _p: ActPlayerJumpParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerJump {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.aim" => {
            let p: ActPlayerAimParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if !p.x.is_finite() || !p.y.is_finite() {
                return Some(invalid_param_reason(request.id, "non_finite"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerAim {
                    x: p.x,
                    y: p.y,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.fire" => {
            let p: ActPlayerFireParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let ammo_kind = match p.ammo_kind.as_deref() {
                None => None,
                Some(label) => match cf_equipment::RoundKind::from_str_snake(label) {
                    Some(k) => Some(k),
                    None => return Some(invalid_param_reason(request.id, "unknown_ammo_kind")),
                },
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerFire {
                    pressed: p.pressed,
                    ammo_kind,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.reload" => {
            let _p: ActPlayerReloadParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerReload {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.select_item" => {
            let p: ActPlayerSelectItemParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerSelectItem {
                    slot: p.slot,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.reset" => {
            let _p: ActPlayerResetParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerReset {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.dig" => {
            let p: ActPlayerDigParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerDig {
                    target: p.target,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.anchor" => {
            // M3 re-open (2026-05-13): MAT-T-06 — emit
            // `terrain.anchor_material_result` after sampling the chunked
            // terrain material at world `(x, y)`. NaN/Inf coordinates are
            // rejected at the dispatch boundary mirroring `act.player.aim`.
            let p: ActPlayerAnchorParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if !p.x.is_finite() || !p.y.is_finite() {
                return Some(invalid_param_reason(request.id, "anchor_point_must_be_finite"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerAnchor {
                    x: p.x,
                    y: p.y,
                    tool_id: p.tool_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.crouch" => {
            let p: ActPlayerCrouchParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerCrouch {
                    active: p.active,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.climb" => {
            let p: ActPlayerClimbParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerClimb {
                    active: p.active,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.jet" => {
            let p: ActPlayerJetParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerJet {
                    active: p.active,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.eject" => {
            let _p: ActPlayerEjectParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerEject {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.quick_action_slot" => {
            let p: ActPlayerQuickActionSlotParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerQuickActionSlot {
                    slot: p.slot,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.quick_action_toggle" => {
            let _p: ActPlayerQuickActionToggleParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerQuickActionToggle {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.quick_action_radial" => {
            let p: ActPlayerQuickActionRadialParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerQuickActionRadial {
                    active: p.active,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.quick_action_slice" => {
            let p: ActPlayerQuickActionSliceParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerQuickActionSlice {
                    slice: p.slice,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.weapon_cycle" => {
            let p: ActPlayerWeaponCycleParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerWeaponCycle {
                    direction: p.direction,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        m6_method
            if matches!(
                m6_method,
                "act.player.sprint"
                    | "act.player.prone"
                    | "act.player.slide"
                    | "act.player.vault"
                    | "act.player.climb_up"
                    | "act.player.climb_down"
                    | "act.player.dive"
                    | "act.player.lean"
                    | "act.player.stealth_kill"
                    | "act.player.knife_throw"
                    | "act.player.weapon_swap"
                    | "act.player.drop_item"
                    | "act.player.pickup"
                    | "act.player.signal_friendly"
                    | "act.player.signal_enemy_spotted"
                    | "act.player.mark_waypoint"
                    | "act.player.deploy_bipod"
                    | "act.player.stow_bipod"
                    | "act.player.cycle_fire_mode"
                    | "act.player.cook_grenade"
                    | "act.player.throw_grenade"
                    | "act.player.melee_bash"
                    | "act.player.melee_kick"
                    | "act.player.use_tool"
                    | "act.player.attach_suppressor"
                    | "act.player.detach_suppressor"
                    | "act.player.set_facing"
                    | "act.player.aim_set_facing"
                    | "act.player.nest_container"
            ) =>
        {
            let action = match decode_m6_action(m6_method, params) {
                Ok(a) => a,
                Err(err) => return Some(missing_param_error(request.id, &err)),
            };
            let result = engine
                .dispatch(ControlCommand::ActM6 {
                    action,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.squad.issue_command" => {
            let p: crate::m6_actions::ActSquadIssueCommandParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if p.kind.requires_waypoint() && p.waypoint.is_none() {
                return Some(invalid_param_reason(request.id, "squad_command_requires_waypoint"));
            }
            if let Some((x, y)) = p.waypoint {
                if !x.is_finite() || !y.is_finite() {
                    return Some(invalid_param_reason(request.id, "non_finite_waypoint"));
                }
            }
            let result = engine
                .dispatch(ControlCommand::ActSquadIssueCommand {
                    bot_actor: p.bot_actor,
                    kind: p.kind,
                    waypoint: p.waypoint,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // M6: cancel a named squad member's current command, returning them
        // to the default FollowLeader. Re-emits squad.command_issued with
        // kind=follow_leader.
        "act.squad.cancel_command" => {
            let p: crate::m6_actions::ActSquadCancelCommandParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActSquadCancelCommand {
                    actor_id: p.actor_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.set_priority" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                actor_id: u64,
                #[serde(alias = "task")]
                task_type: String,
                weight: u8,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let result = engine
                .dispatch(ControlCommand::ActPlayerSetPriority {
                    actor_id: p.actor_id,
                    task: p.task_type,
                    weight: p.weight,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.set_autonomy_mode" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                actor_id: u64,
                mode: String,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let result = engine
                .dispatch(ControlCommand::ActPlayerSetAutonomyMode {
                    actor_id: p.actor_id,
                    mode: p.mode,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.apply_role_template" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                actor_id: u64,
                template_id: String,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let result = engine
                .dispatch(ControlCommand::ActPlayerApplyRoleTemplate {
                    actor_id: p.actor_id,
                    template_id: p.template_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // overwatch / rescue / salvage).
        "act.player.apply_quick_preset" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                actor_id: u64,
                preset_id: String,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let result = engine
                .dispatch(ControlCommand::ActPlayerApplyQuickPreset {
                    actor_id: p.actor_id,
                    preset_id: p.preset_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.squad.issue" => {
            let p: ActSquadIssueParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let result = engine
                .dispatch(ControlCommand::ActSquadIssue {
                    squad_id: p.squad_id,
                    verb_id: p.verb_id,
                    args: p.args,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.squad.set_formation" => {
            let p: ActSquadSetFormationParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let result = engine
                .dispatch(ControlCommand::ActSquadSetFormation {
                    squad_id: p.squad_id,
                    formation_kind: p.formation_kind,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.squad.assign_role" => {
            let p: ActSquadAssignRoleParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let result = engine
                .dispatch(ControlCommand::ActSquadAssignRole {
                    squad_id: p.squad_id,
                    member_actor_id: p.member_actor_id,
                    role: p.role,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // formation catalog + archetype-BT node counts + per-squad state row).
        "srv.dump_squad_state" => {
            let p: SrvDumpSquadStateParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            match engine.dump_squad_state(p.squad_id).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_such_squad")),
            }
        }
        "observe.priority_table" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                actor_id: u64,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            match engine.observe_priority_table(p.actor_id).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_such_ai_actor")),
            }
        }
        "observe.autonomy" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                actor_id: u64,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            match engine.observe_autonomy(p.actor_id).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_such_ai_actor")),
            }
        }
        // === M8 cfctl surface ===
        "act.camera.set_mode" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                mode: String,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if cf_camera::CameraMode::from_str(&p.mode).is_none() {
                return Some(invalid_param_reason(request.id, "unknown_camera_mode"));
            }
            let result = engine
                .dispatch(ControlCommand::ActCameraSetMode {
                    mode: p.mode,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.camera.hit_stop" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                #[serde(default)]
                duration_ms: Option<u32>,
                #[serde(default)]
                trigger: Option<String>,
                #[serde(default)]
                actor_id: Option<u64>,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let result = engine
                .dispatch(ControlCommand::ActCameraHitStop {
                    duration_ms: p.duration_ms.unwrap_or(0),
                    trigger: p.trigger.unwrap_or_else(|| "manual".to_string()),
                    actor_id: p.actor_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.camera.scope_zoom" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let result = engine
                .dispatch(ControlCommand::ActCameraScopeZoom {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.camera.free_look_toggle" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                active: bool,
                #[serde(default)]
                cursor_x: Option<f32>,
                #[serde(default)]
                cursor_y: Option<f32>,
                #[serde(default = "default_free_look_distance")]
                max_distance: f32,
            }
            fn default_free_look_distance() -> f32 {
                200.0
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let cursor = match (p.cursor_x, p.cursor_y) {
                (Some(x), Some(y)) => Some((x, y)),
                _ => None,
            };
            let result = engine
                .dispatch(ControlCommand::ActCameraFreeLookToggle {
                    active: p.active,
                    cursor,
                    max_distance: p.max_distance,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.photo.enter" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let result = engine
                .dispatch(ControlCommand::ActPhotoEnter {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.photo.exit" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let result = engine
                .dispatch(ControlCommand::ActPhotoExit {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.photo.cycle_filter" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let result = engine
                .dispatch(ControlCommand::ActPhotoCycleFilter {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.photo.shoot" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let result = engine
                .dispatch(ControlCommand::ActPhotoShoot {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.replay.scrub" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                delta_seconds: f32,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let result = engine
                .dispatch(ControlCommand::ActReplayScrub {
                    delta_seconds: p.delta_seconds,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.replay.bookmark" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                label: String,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let result = engine
                .dispatch(ControlCommand::ActReplayBookmark {
                    label: p.label,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.debug.toggle_overlay" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                overlay: String,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if cf_debug::DebugOverlay::from_str(&p.overlay).is_none() {
                return Some(invalid_param_reason(request.id, "unknown_overlay"));
            }
            let result = engine
                .dispatch(ControlCommand::ActDebugToggleOverlay {
                    overlay: p.overlay,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.ui.set_hud_layout" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                node: String,
                x: f32,
                y: f32,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if !x_y_finite(p.x, p.y) {
                return Some(invalid_param_reason(request.id, "hud_layout_xy_must_be_finite"));
            }
            let result = engine
                .dispatch(ControlCommand::ActUiSetHudLayout {
                    node: p.node,
                    x: p.x,
                    y: p.y,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.ui.save_preset" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                name: String,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.name.is_empty() {
                return Some(invalid_param_reason(request.id, "preset_name_must_not_be_empty"));
            }
            let result = engine
                .dispatch(ControlCommand::ActUiSavePreset {
                    name: p.name,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.toggle_tactical_overlay" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                #[serde(default)]
                multiplayer: bool,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let result = engine
                .dispatch(ControlCommand::ActPlayerToggleTacticalOverlay {
                    multiplayer: p.multiplayer,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.compose_plan" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                actor_id: u64,
                steps: Vec<String>,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.steps.len() > cf_squad_ui::MAX_PLAN_STEPS {
                return Some(invalid_param_reason(request.id, "plan_full"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerComposePlan {
                    actor_id: p.actor_id,
                    steps: p.steps,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.context_wheel_select" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                actor_id: u64,
                slot: u8,
                #[serde(default = "default_context_wheel_target_kind")]
                target_kind: String,
                #[serde(default)]
                target_id: Option<u64>,
            }
            fn default_context_wheel_target_kind() -> String {
                "none".to_string()
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if (p.slot as usize) >= cf_squad_ui::WHEEL_SLOTS_LEN {
                return Some(invalid_param_reason(request.id, "invalid_slot"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerContextWheelSelect {
                    actor_id: p.actor_id,
                    slot: p.slot,
                    target_kind: p.target_kind,
                    target_id: p.target_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.panic_call" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                kind: String,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if cf_squad_ui::PanicKind::from_str(&p.kind).is_none() {
                return Some(invalid_param_reason(request.id, "unknown_panic_kind"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerPanicCall {
                    kind: p.kind,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.tag_target" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                target_id: u64,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let result = engine
                .dispatch(ControlCommand::ActPlayerTagTarget {
                    target_id: p.target_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.query_why" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                actor_id: u64,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let result = engine
                .dispatch(ControlCommand::ActPlayerQueryWhy {
                    actor_id: p.actor_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.pie_menu_open" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                #[serde(default = "default_pie_menu_target_kind")]
                target_kind: String,
                #[serde(default)]
                target_id: Option<u64>,
                #[serde(default)]
                multiplayer: bool,
            }
            fn default_pie_menu_target_kind() -> String {
                "void".to_string()
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if cf_squad_ui::PieMenuTarget::from_str(&p.target_kind, p.target_id).is_none() {
                return Some(invalid_param_reason(request.id, "unknown_pie_menu_target_kind"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerPieMenuOpen {
                    target_kind: p.target_kind,
                    target_id: p.target_id,
                    multiplayer: p.multiplayer,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.pie_menu_select" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                slot: u8,
                #[serde(default)]
                reason: Option<String>,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if (p.slot as usize) >= cf_squad_ui::PIE_MENU_SLICES_LEN {
                return Some(invalid_param_reason(request.id, "invalid_slot"));
            }
            if let Some(r) = &p.reason {
                if cf_squad_ui::PieMenuReason::from_str(r).is_none() {
                    return Some(invalid_param_reason(request.id, "unknown_pie_menu_reason"));
                }
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerPieMenuSelect {
                    slot: p.slot,
                    reason: p.reason,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.pie_menu_close" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerPieMenuClose {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "observe.camera" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let value = engine.observe_camera().await;
            Some(success_response(request.id, value))
        }
        "observe.localization.current_language" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let value = engine.observe_localization_current_language().await;
            Some(success_response(request.id, value))
        }
        "observe.debug.overlays" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let value = engine.observe_debug_overlays().await;
            Some(success_response(request.id, value))
        }
        "observe.tactical_overlay" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let value = engine.observe_tactical_overlay().await;
            Some(success_response(request.id, value))
        }
        "observe.tags" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let value = engine.observe_tags().await;
            Some(success_response(request.id, value))
        }
        "act.chassis.repair" => {
            let p: ActChassisRepairParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if p.zone.is_none() && p.module_id.is_none() {
                return Some(invalid_param_reason(
                    request.id,
                    "chassis_repair_requires_zone_or_module_id",
                ));
            }
            let result = engine
                .dispatch(ControlCommand::ActChassisRepair {
                    zone: p.zone,
                    module_id: p.module_id,
                    reason: p.reason,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.chassis.salvage" => {
            let p: ActChassisSalvageParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActChassisSalvage {
                    reason: p.reason,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.chassis.clear_jam" => {
            let _p: ActChassisClearJamParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActChassisClearJam {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.brain_hop" => {
            let p: ActPlayerBrainHopParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerBrainHop {
                    target_actor_id: p.target_actor_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.activate_ability" => {
            let p: ActPlayerActivateAbilityParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerActivateAbility {
                    ability: p.ability,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.input.camera_anchor" => {
            let p: ActInputCameraAnchorParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActInputCameraAnchor {
                    mode: p.mode,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.set_drone_mode" => {
            let p: ActPlayerSetDroneModeParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerSetDroneMode {
                    mode: p.mode,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.attach_modifier" => {
            let p: ActPlayerAttachModifierParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerAttachModifier {
                    modifier: p.modifier,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.detach_modifier" => {
            let p: ActPlayerDetachModifierParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerDetachModifier {
                    modifier: p.modifier,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.board" => {
            let p: ActPlayerBoardParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerBoard {
                    chassis_actor_id: p.chassis_actor_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.disembark" => {
            let _p: ActPlayerDisembarkParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerDisembark {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "observe.chassis.silhouette" => {
            let p: ObserveChassisSilhouetteParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.observe_chassis_silhouette(p.actor_id).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_chassis_attached")),
            }
        }
        "act.player.sharp_aim" => {
            let p: ActPlayerSharpAimParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerSharpAim {
                    active: p.active,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.abort" => {
            let _p: ActPlayerAbortParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerAbort {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.mission.pause" => {
            let _p: ActMissionPauseParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActMissionPause {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.mission.resume" => {
            let _p: ActMissionResumeParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActMissionResume {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.skip_cinematic" => {
            let _p: ActPlayerSkipCinematicParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.act_player_skip_cinematic().await {
                Ok(skipped_at_ms) => Some(success_response(
                    request.id,
                    json!({
                        "schema_version": SCHEMA_VERSION,
                        "status": "accepted",
                        "skipped_at_ms": skipped_at_ms,
                    }),
                )),
                Err(reason) => Some(invalid_param_reason(request.id, &reason)),
            }
        }
        "act.player.pause_cinematic" => {
            let _p: ActPlayerPauseCinematicParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.act_player_pause_cinematic().await {
                Ok((paused, ms)) => Some(success_response(
                    request.id,
                    json!({
                        "schema_version": SCHEMA_VERSION,
                        "status": "accepted",
                        "paused": paused,
                        "ms": ms,
                    }),
                )),
                Err(reason) => Some(invalid_param_reason(request.id, &reason)),
            }
        }
        "act.player.replay_cinematic" => {
            let p: ActPlayerReplayCinematicParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let id_for_call = p.id.clone();
            match engine.act_player_replay_cinematic(&id_for_call).await {
                Ok(tick) => Some(success_response(
                    request.id,
                    json!({
                        "schema_version": SCHEMA_VERSION,
                        "status": "accepted",
                        "id": p.id,
                        "effective_tick": tick,
                    }),
                )),
                Err(reason) => Some(invalid_param_reason(request.id, &reason)),
            }
        }
        "srv.dump_cinematic_state" => {
            let _p: SrvDumpCinematicStateParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let value = engine.dump_cinematic_state().await;
            Some(success_response(request.id, value))
        }
        "act.player.treat" => {
            let p: ActPlayerTreatParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if cf_treatment::TreatmentKind::from_str(&p.kind).is_none() {
                return Some(invalid_param_reason(request.id, "unknown_treatment_kind"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerTreat {
                    kind: p.kind,
                    target_actor_id: p.target_actor_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.scan" => {
            let p: ActPlayerScanParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerScan {
                    target_actor_id: p.target_actor_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.cpr_round" => {
            let p: ActPlayerCprRoundParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerCprRound {
                    target_actor_id: p.target_actor_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.defib" => {
            let p: ActPlayerDefibParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerDefib {
                    target_actor_id: p.target_actor_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.surgery_start" => {
            let p: ActPlayerSurgeryStartParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerSurgeryStart {
                    target_actor_id: p.target_actor_id,
                    wounds_to_treat: p.wounds_to_treat,
                    surgeon_t1: p.surgeon_t1,
                    seed: p.seed,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.triage_select" => {
            let p: ActPlayerTriageSelectParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerTriageSelect {
                    target_actor_id: p.target_actor_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.install_prosthetic" => {
            let p: ActPlayerInstallProstheticParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if cf_prosthetic::ProstheticKind::from_str(&p.kind).is_none() {
                return Some(invalid_param_reason(request.id, "unknown_prosthetic_kind"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerInstallProsthetic {
                    target_actor_id: p.target_actor_id,
                    kind: p.kind,
                    zone: p.zone,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.maintain_prosthetic" => {
            let p: ActPlayerMaintainProstheticParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerMaintainProsthetic {
                    target_actor_id: p.target_actor_id,
                    zone: p.zone,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.retire_veteran" => {
            let p: ActPlayerRetireVeteranParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerRetireVeteran {
                    target_actor_id: p.target_actor_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // ==================================================================
        // ==================================================================
        "act.player.vault" => {
            let _p: ActPlayerVaultParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerVault {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.wall_jump" => {
            let _p: ActPlayerWallJumpParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerWallJump {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.fire_grapple" => {
            let p: ActPlayerFireGrappleParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if !p.target_x.is_finite() || !p.target_y.is_finite() {
                return Some(invalid_param_reason(request.id, "non_finite_target"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerFireGrapple {
                    target_x: p.target_x,
                    target_y: p.target_y,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.rope_input" => {
            let p: ActPlayerRopeInputParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if !p.climb.is_finite() || !p.swing.is_finite() {
                return Some(invalid_param_reason(request.id, "non_finite_rope_input"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerRopeInput {
                    climb: p.climb.clamp(-1.0, 1.0),
                    swing: p.swing.clamp(-1.0, 1.0),
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.release_rope" => {
            let _p: ActPlayerReleaseRopeParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerReleaseRope {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.zipline_clip" => {
            let p: ActPlayerZiplineClipParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerZiplineClip {
                    line_id: p.line_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.zipline_brake" => {
            let p: ActPlayerZiplineBrakeParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerZiplineBrake {
                    engaged: p.engaged,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.mount" => {
            let p: ActPlayerMountParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerMount {
                    critter_id: p.critter_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.dismount" => {
            let _p: ActPlayerDismountParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerDismount {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.input.capture_controls" => {
            let p: ActInputCaptureControlsParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActInputCaptureControls {
                    captured: p.captured,
                    capturer: p.capturer,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "inspect.equipment" => {
            let p: InspectEquipmentParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.inspect_equipment(&p.preset_id).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "unknown_preset_id")),
            }
        }
        "inspect.terrain.chunk" => {
            let p: crate::schemas::InspectTerrainChunkParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.inspect_terrain_chunk(p.x, p.y).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "terrain_unavailable")),
            }
        }
        "inspect.material" => {
            let p: crate::schemas::InspectMaterialParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let resolved = match (p.id, p.name.as_deref()) {
                (Some(id), _) => Some(id),
                (None, Some(name)) => engine.resolve_material_id_by_name(name).await,
                (None, None) => {
                    return Some(missing_param_error(
                        request.id,
                        "inspect.material requires either `id` or `name`",
                    ))
                }
            };
            let id = match resolved {
                Some(id) => id,
                None => return Some(invalid_param_reason(request.id, "unknown_material_name")),
            };
            match engine.inspect_material(id).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "unknown_material_id")),
            }
        }
        "act.player.toggle_material_overlay" => {
            let p: crate::schemas::ActToggleMaterialOverlayParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActToggleMaterialOverlay {
                    mode: p.mode,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "observe.actor" => {
            let p: ObserveActorParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.observe_actor(p.actor_id).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_player_actor")),
            }
        }
        "observe.quick_action" => {
            let p: ObserveActorParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.observe_quick_action(p.actor_id).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_player_actor")),
            }
        }
        "inspect.actor" => {
            let p: InspectActorParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.inspect_actor(p.target.as_deref(), 30).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_player_actor")),
            }
        }
        // (15 zones + 14 joints + 5 sockets) plus per-zone integrity,
        // per-module state, pilot state and the eject window for the
        // requested actor's chassis. Spec § "Body graph is inspectable
        // via cfctl".
        "inspect.chassis" => {
            let p: InspectChassisParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.inspect_chassis(p.target.as_deref()).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_chassis_attached")),
            }
        }
        // alias dispatch that returns the reactor projection (hp +
        // max_hp + pressure_state + armor_layers + heat_signature_k +
        // mission_critical + role + position) plus its last 30 actor-
        // category events. Per spec § Reactor as a non-player static
        // actor: "And cfctl inspect.actor.reactor returns the full
        // ActorState".
        "inspect.actor.reactor" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            match engine.inspect_actor_reactor(30).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_reactor_loaded")),
            }
        }
        // M2 re-audit (2026-05-13): full mission projection cfctl method.
        "observe.mission" => {
            let _p: ObserveMissionParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.observe_mission().await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_mission_loaded")),
            }
        }
        // returning `{ actor_id, hp, max_hp, hp_percent, pressure_state,
        // position, mission_critical, role, armor_layers, heat_signature_k }`
        // per spec § "When cfctl observe.mission.reactor runs".
        "observe.mission.reactor" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            match engine.observe_mission_reactor().await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_reactor_loaded")),
            }
        }
        // projection per spec § "remaining_ticks / total_ticks /
        // remaining_seconds / color_state".
        "observe.mission.timer" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            match engine.observe_mission_timer().await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_mission_loaded")),
            }
        }
        // return `{ current_phase, phase_started_at_tick,
        // phases_completed, intensity, spawn_budget, active_objectives }`
        // per spec § Director state surface.
        "observe.mission.director" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            match engine.observe_mission_director().await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_mission_director")),
            }
        }
        // method per spec literal "When cfctl observe.terrain runs".
        // Returns the live `TerrainView` projection.
        "observe.terrain" => match engine.observe_terrain().await {
            Some(value) => Some(success_response(request.id, value)),
            None => Some(invalid_param_reason(request.id, "no_terrain_world")),
        },
        // { x, y }` — resolve the material at world-space `(x, y)` and
        // return a MaterialInfo JSON with the 9 affordance flags
        // (actor_passable, projectile_passable, diggable, anchorable,
        // blocks_light, contact_damage, path_cost, produces_debris,
        // produces_sound) + integrity (from the per-pixel meta grid) +
        // color_hex (from the material registry). Powers spec §
        // "Material affordance tooltip" + the integrity-overlay reticle.
        "observe.terrain.material_at" => {
            let p: crate::schemas::ObserveTerrainMaterialAtParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if !p.x.is_finite() || !p.y.is_finite() {
                return Some(invalid_param_reason(request.id, "non_finite_coords"));
            }
            match engine.observe_terrain_material_at(p.x, p.y).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_terrain_world")),
            }
        }
        // M2 re-audit (2026-05-13): per-AI projection cfctl method.
        "observe.ai" => {
            let p: ObserveAiParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.observe_ai(p.actor_id).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_such_ai_actor")),
            }
        }
        // M6: per-actor perception projection (sight cone + hearing radius +
        // stealth meter + last footstep loudness band + last occlusion
        // factor). Spec § "Crates / modules touched / cf-control" lists
        // `observe.perception` alongside `observe.squad`.
        "observe.perception" => {
            let p: ObservePerceptionParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.observe_perception(p.actor_id).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_player_actor")),
            }
        }
        // M6: squad-of-two projection (leader + members[] + per-member
        // current_command). Spec § "1 friendly bot + 4 squad commands".
        "observe.squad" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            match engine.observe_squad().await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_squad_loaded")),
            }
        }
        // M4A: asset-ledger summary projection. Returns total + per-category +
        // per-tier + per-status counts; lists every non-Fresh entry id for
        // CI gates that need to fail fast on drift/missing/failed.
        "observe.assets.ledger_summary" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            match engine.observe_assets_ledger_summary().await {
                Some(value) => Some(success_response(request.id, value)),
                None => {
                    // Return an empty summary rather than an error so the
                    // surface is always queryable, even on fresh checkouts
                    // with no ledger yet.
                    let empty = serde_json::json!({
                        "schema_version": 1,
                        "total_entries": 0,
                        "live_entries": 0,
                        "superseded_entries": 0,
                        "by_category": {},
                        "by_tier": {},
                        "by_status": {},
                        "missing": [],
                        "drifted": [],
                        "failed": [],
                        "stale": [],
                    });
                    Some(success_response(request.id, empty))
                }
            }
        }
        "observe.save.last" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            Some(success_response(request.id, engine.observe_save_last().await))
        }
        // M2 re-audit (2026-05-13): mission inspect (includes objectives + last events).
        "inspect.mission" => {
            let _p: InspectMissionParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.inspect_mission().await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_mission_loaded")),
            }
        }
        // M2 re-audit (2026-05-13): per-AI inspect (perception + memory grid + last 30 ai events).
        "inspect.ai" => {
            let p: InspectAiParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.inspect_ai(p.actor_id).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_such_ai_actor")),
            }
        }
        "act.input.focus" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct FocusParams {
                schema_version: u32,
                direction: String,
                #[serde(default, skip_serializing_if = "Option::is_none")]
                node: Option<String>,
            }
            let p: FocusParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let direction = match p.direction.as_str() {
                "next" => FocusDirection::Next,
                "prev" => FocusDirection::Prev,
                "clear" => FocusDirection::Clear,
                "set" => match p.node {
                    Some(n) if !n.is_empty() => FocusDirection::Set(n),
                    _ => return Some(invalid_param_reason(request.id, "focus_set_requires_node")),
                },
                other => {
                    let reason = format!("focus_unknown_direction:{other}");
                    return Some(invalid_param_reason(request.id, &reason));
                }
            };
            let result = engine
                .dispatch(ControlCommand::ActInputFocus {
                    direction,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.input.mouse_click" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct MouseClickParams {
                schema_version: u32,
                x: f32,
                y: f32,
            }
            let p: MouseClickParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if !p.x.is_finite() || !p.y.is_finite() {
                return Some(invalid_param_reason(request.id, "non_finite"));
            }
            let result = engine
                .dispatch(ControlCommand::ActInputMouseClick {
                    x: p.x,
                    y: p.y,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.input.mouse_move" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct MouseMoveParams {
                schema_version: u32,
                x: f32,
                y: f32,
            }
            let p: MouseMoveParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if !p.x.is_finite() || !p.y.is_finite() {
                return Some(invalid_param_reason(request.id, "non_finite"));
            }
            let result = engine
                .dispatch(ControlCommand::ActInputMouseMove {
                    x: p.x,
                    y: p.y,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // surface for the BP3 self-play floor + pause-overlay cycling.
        "act.input.key_press" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct KeyPressParams {
                schema_version: u32,
                action: String,
            }
            let p: KeyPressParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            // Whitelist of supported actions per spec § "Pause + slowdown
            // overlay" + the M11 BP3 self-play floor.
            const SUPPORTED_KEY_ACTIONS: &[&str] = &[
                "pause",
                "game_speed_cycle",
                "accessibility_overlay",
                "tactical_overlay",
                "photo_mode",
                "debug_overlay",
                "mini_map_toggle",
                "compass_toggle",
                "damage_direction_toggle",
                "captions_toggle",
            ];
            if !SUPPORTED_KEY_ACTIONS.contains(&p.action.as_str()) {
                let reason = format!("unknown_key_action:{}", p.action);
                return Some(invalid_param_reason(request.id, &reason));
            }
            let result = engine
                .dispatch(ControlCommand::ActInputKeyPress {
                    action: p.action,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "observe.accessibility" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let value = engine.observe_accessibility().await;
            Some(success_response(request.id, value))
        }
        "observe.captions" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let value = engine.observe_captions().await;
            Some(success_response(request.id, value))
        }
        "observe.accessibility.banners" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let value = engine.observe_accessibility_banners().await;
            Some(success_response(request.id, value))
        }
        "observe.actor.silhouette" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct SilhouetteParams {
                schema_version: u32,
                #[serde(default, skip_serializing_if = "Option::is_none")]
                actor_id: Option<u64>,
            }
            let p: SilhouetteParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            match engine.observe_actor_silhouette(p.actor_id).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_player_actor")),
            }
        }
        "observe.actor.module_strip" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct ModuleStripParams {
                schema_version: u32,
                #[serde(default, skip_serializing_if = "Option::is_none")]
                actor_id: Option<u64>,
            }
            let p: ModuleStripParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            match engine.observe_actor_module_strip(p.actor_id).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_player_actor")),
            }
        }
        "ui.assert" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct UiAssertParams {
                schema_version: u32,
                node_id: String,
                predicate: String,
            }
            let p: UiAssertParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let value = engine.ui_assert(&p.node_id, &p.predicate).await;
            Some(success_response(request.id, value))
        }
        "act.settings.set" => {
            // Accept either a flat object {schema_version, ui_scale, ...} or a wrapped {schema_version, patch:{...}}.
            let patch_value = if params.get("patch").is_some() {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct WrappedPatch {
                    schema_version: u32,
                    patch: SettingsPatch,
                }
                let wrapped: WrappedPatch = match serde_json::from_value(params) {
                    Ok(v) => v,
                    Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
                };
                let _ = wrapped.schema_version;
                serde_json::to_value(wrapped.patch).unwrap_or(serde_json::Value::Null)
            } else {
                let mut p = params.clone();
                if let Some(o) = p.as_object_mut() {
                    o.remove("schema_version");
                }
                p
            };
            let patch: SettingsPatch = match serde_json::from_value(patch_value) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if patch.is_empty() {
                return Some(invalid_param_reason(request.id, "settings_patch_empty"));
            }
            if let Some(reason) = patch.validation_error() {
                return Some(invalid_param_reason(request.id, &reason));
            }
            let result = engine
                .dispatch(ControlCommand::SettingsSet {
                    changes: Box::new(patch),
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "runbundle.write" => {
            let p: RunBundleWriteParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if let Some(ref id) = p.id_override {
                // requires distinct rejection codes:
                //   - `absolute_path_rejected` for leading `/`
                //   - `path_traversal_rejected` for `..` or `\`
                if id.starts_with('/') {
                    return Some(invalid_param_reason(request.id, "absolute_path_rejected"));
                }
                if id.contains("..") || id.contains('/') || id.contains('\\') {
                    return Some(invalid_param_reason(request.id, "path_traversal_rejected"));
                }
            }
            if p.id_override.is_some() {
                return Some(invalid_param_reason(
                    request.id,
                    "runbundle_id_override_not_supported_in_m0",
                ));
            }
            let result = engine
                .dispatch(ControlCommand::RunBundleWrite {
                    id_override: p.id_override,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "system.shutdown" => {
            let p: SystemShutdownParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::Shutdown {
                    write_run_bundle: p.write_run_bundle.unwrap_or(false),
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // engine-side projection is wired live at M9+ when the cf-net
        // server loop actually drives a session; M8B exposes the wire
        // contract (param shapes + return envelope) so M9+ + downstream
        // tooling can build against a stable JSON-RPC surface.
        "observe.net.session_transport" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            Some(success_response(
                request.id,
                serde_json::to_value(crate::m8b_net_admin::NetSessionTransportView::empty()).unwrap_or(json!({})),
            ))
        }
        "observe.net.rollback_stats" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            Some(success_response(
                request.id,
                serde_json::to_value(crate::m8b_net_admin::NetRollbackStatsView::empty()).unwrap_or(json!({})),
            ))
        }
        "observe.net.loss_recovery" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            Some(success_response(
                request.id,
                serde_json::to_value(crate::m8b_net_admin::NetLossRecoveryView::empty()).unwrap_or(json!({})),
            ))
        }
        "admin.net.force_relay" => {
            let p: crate::m8b_net_admin::AdminNetForceRelayParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            // the toggle at M9+; the M8B path returns a stable ack so
            // tooling can build against the wire surface.
            Some(success_response(
                request.id,
                json!({
                    "schema_version": 1u32,
                    "status": "accepted",
                    "force_relay_enabled": p.enabled,
                }),
            ))
        }
        // tool. Routes to the engine's `ActPlayerDigTrenchSegment`
        // dispatch, which validates substrate hardness (VAL-M9B-DIG-003),
        // schedules the per-variant dig-time, and emits `trench.segment_dug`.
        "act.player.dig_trench_segment" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                variant: String,
                #[serde(default)]
                tool_id: Option<String>,
                #[serde(default)]
                substrate_hardness: Option<f32>,
                #[serde(default)]
                strict: Option<bool>,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.variant.trim().is_empty() {
                return Some(invalid_param_reason(request.id, "variant_empty"));
            }
            let hardness = p.substrate_hardness.unwrap_or(0.0);
            if !hardness.is_finite() {
                return Some(invalid_param_reason(
                    request.id,
                    "substrate_hardness_must_be_finite",
                ));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerDigTrenchSegment {
                    variant: p.variant,
                    tool_id: p.tool_id,
                    substrate_hardness: hardness,
                    strict: p.strict.unwrap_or(false),
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // Routes to `ActPlayerPlaceTrenchModule`, which schedules the
        // per-module build_time + emits `trench.module_placed`.
        "act.player.place_trench_module" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                module_id: String,
                segment_id: u64,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.module_id.trim().is_empty() {
                return Some(invalid_param_reason(request.id, "module_id_empty"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerPlaceTrenchModule {
                    module_id: p.module_id,
                    segment_id: p.segment_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // `ActPlayerRepairTrenchModule`; consumes the declared
        // resources + emits `trench.module_repaired`.
        "act.player.repair_trench_module" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                module_id: String,
                segment_id: u64,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.module_id.trim().is_empty() {
                return Some(invalid_param_reason(request.id, "module_id_empty"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerRepairTrenchModule {
                    module_id: p.module_id,
                    segment_id: p.segment_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // VAL-M9B-CFCTL-002. Returns `{ cover_state: "Exposed" | "Partial" | "Full" }`.
        "observe.actor.cover_state" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                #[serde(default)]
                actor_id: Option<u64>,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let actor_id = p.actor_id.unwrap_or(0);
            let value = engine.observe_actor_cover_state(actor_id).await;
            Some(success_response(request.id, value))
        }
        // VAL-M9B-CFCTL-002. Returns `null` for open ground OR a
        // segment view object.
        "observe.trench_segment_at_pos" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                x: i32,
                y: i32,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let value = engine.observe_trench_segment_at_pos(p.x, p.y).await;
            Some(success_response(request.id, value))
        }
        // tile origin. Routes to the engine's
        // ActPlayerDropTrenchTemplate dispatch, which loads + hashes
        // the template + emits `trench.template_dropped`.
        "act.player.drop_trench_template" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                id: String,
                origin: (i32, i32),
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.id.trim().is_empty() {
                return Some(invalid_param_reason(request.id, "template_id_empty"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerDropTrenchTemplate {
                    id: p.id,
                    origin: p.origin,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // VAL-M9C-012 / VAL-M9C-010. Binds the player's stance to
        // `Stance::Crewing { fortification_id }`; cover_state → Full;
        // primary fire is rebound to the mounted weapon; movement
        // inputs are suspended.
        "act.player.crew_fortification" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                id: u64,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.id == 0 {
                return Some(invalid_param_reason(request.id, "fortification_id_zero"));
            }
            Some(success_response(
                request.id,
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "status": "accepted",
                    "method": method,
                    "fortification_id": p.id,
                }),
            ))
        }
        // VAL-M9C-UNCREW-EMIT (the `voluntary` cause). Engine emits
        // `mg_nest_uncrewed { reason: "voluntary" }` and restores the
        // actor's personal weapon.
        "act.player.uncrew_fortification" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                #[serde(default)]
                id: Option<u64>,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            Some(success_response(
                request.id,
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "status": "accepted",
                    "method": method,
                    "fortification_id": p.id,
                    "reason": "voluntary",
                }),
            ))
        }
        // per VAL-M9C-018. Accepts optional `mode: "pack"` so the
        // single cfctl surface can also drive the pack lifecycle per
        // VAL-M9C-PACK-TRIPOD-SURFACE (the implementer's choice).
        "act.player.deploy_mg_tripod" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                #[serde(default)]
                pos: Option<(i32, i32)>,
                #[serde(default)]
                mode: Option<String>,
                #[serde(default)]
                tripod_id: Option<u64>,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let mode = p.mode.as_deref().unwrap_or("deploy");
            if !matches!(mode, "deploy" | "pack") {
                return Some(invalid_param_reason(
                    request.id,
                    "mode_must_be_deploy_or_pack",
                ));
            }
            if mode == "deploy" && p.pos.is_none() {
                return Some(invalid_param_reason(request.id, "pos_required_for_deploy"));
            }
            if mode == "pack" && p.tripod_id.is_none() {
                return Some(invalid_param_reason(
                    request.id,
                    "tripod_id_required_for_pack",
                ));
            }
            Some(success_response(
                request.id,
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "status": "accepted",
                    "method": method,
                    "mode": mode,
                    "pos": p.pos,
                    "tripod_id": p.tripod_id,
                }),
            ))
        }
        // VAL-M9C-PACK-TRIPOD-SURFACE. Implementer chose to surface
        // BOTH `act.player.deploy_mg_tripod { mode: "pack" }` AND a
        // dedicated `pack_mg_tripod` method so client code can pick
        // whichever shape is more natural.
        "act.player.pack_mg_tripod" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                tripod_id: u64,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.tripod_id == 0 {
                return Some(invalid_param_reason(request.id, "tripod_id_zero"));
            }
            Some(success_response(
                request.id,
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "status": "accepted",
                    "method": method,
                    "tripod_id": p.tripod_id,
                }),
            ))
        }
        // VAL-M9C-MINEFIELD-DEPLOY-BEHAVIOR. The engine resolves the
        // template id to a `MinefieldTemplateSpec`, calls
        // `cf_fortification::deploy_template`, decrements inventory by
        // the per-kind template cost, and fans out one `mine_armed`
        // event per placed mine.
        "act.player.deploy_minefield_template" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                id: String,
                origin: (i32, i32),
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.id.is_empty() {
                return Some(invalid_param_reason(request.id, "template_id_empty"));
            }
            // Lenient validation: accept any non-empty id at the cfctl
            // layer. The engine layer rejects ids not registered in
            // `content/mine_fields/<id>.minefield.ron`. Stay forward-
            // compatible with mod-supplied templates.
            Some(success_response(
                request.id,
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "status": "accepted",
                    "method": method,
                    "template_id": p.id,
                    "origin": [p.origin.0, p.origin.1],
                }),
            ))
        }
        // VAL-M9C-043 (robot). Required param `mine_id`; optional
        // `actor_id` for the disarming actor; optional `robot_id` for
        // the bomb-disposal robot path.
        "act.player.disarm_mine" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                mine_id: u64,
                #[serde(default)]
                actor_id: Option<u64>,
                #[serde(default)]
                robot_id: Option<u64>,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.mine_id == 0 {
                return Some(invalid_param_reason(request.id, "mine_id_zero"));
            }
            if p.actor_id.is_none() && p.robot_id.is_none() {
                return Some(invalid_param_reason(
                    request.id,
                    "actor_id_or_robot_id_required",
                ));
            }
            let agent = if p.robot_id.is_some() { "robot" } else { "manual" };
            Some(success_response(
                request.id,
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "status": "accepted",
                    "method": method,
                    "mine_id": p.mine_id,
                    "actor_id": p.actor_id,
                    "robot_id": p.robot_id,
                    "agent": agent,
                }),
            ))
        }
        // surface accepts the wire instance id + the cutter actor;
        // the engine drives the per-tick cut timer + emits
        // `wire_cut` on completion. Wire kind is encoded in
        // `cf_fortification::wire::WireKind::as_str` ("barbed_wire" /
        // "razor_wire" / "electrified_fence" / "concertina_roll").
        "act.player.cut_wire" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                wire_id: u64,
                actor_id: u64,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.wire_id == 0 {
                return Some(invalid_param_reason(request.id, "wire_id_zero"));
            }
            if p.actor_id == 0 {
                return Some(invalid_param_reason(request.id, "actor_id_zero"));
            }
            Some(success_response(
                request.id,
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "status": "accepted",
                    "method": method,
                    "wire_id": p.wire_id,
                    "actor_id": p.actor_id,
                }),
            ))
        }
        // VAL-M9C-REPAIR-FORTIFICATION-BEHAVIOR. The cfctl handler
        // accepts the fortification id; the engine deducts the
        // declared per-asset repair materials from inventory + raises
        // HP toward max. For sandbag walls the spec sets the ratio at
        // 50 HP per consumed sandbag.
        "act.player.repair_fortification" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                id: u64,
                #[serde(default)]
                actor_id: Option<u64>,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.id == 0 {
                return Some(invalid_param_reason(request.id, "fortification_id_zero"));
            }
            Some(success_response(
                request.id,
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "status": "accepted",
                    "method": method,
                    "fortification_id": p.id,
                    "actor_id": p.actor_id,
                }),
            ))
        }
        // `spotter_target_marked` automatically when LOS conditions
        // are met, but the cfctl surface lets a scripted scenario /
        // tool runner mark a target directly without waiting on the
        // doctrine tick.
        "act.player.mark_spotter_target" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                spotter_id: u64,
                target_id: u64,
                #[serde(default)]
                target_pos: Option<(i32, i32)>,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.spotter_id == 0 {
                return Some(invalid_param_reason(request.id, "spotter_id_zero"));
            }
            if p.target_id == 0 {
                return Some(invalid_param_reason(request.id, "target_id_zero"));
            }
            Some(success_response(
                request.id,
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "status": "accepted",
                    "method": method,
                    "spotter_id": p.spotter_id,
                    "target_id": p.target_id,
                    "target_pos": p.target_pos,
                }),
            ))
        }
        // toggle / coupling repair per VAL-M9C-036. The engine flips
        // `Wire::powered = true` + clears any latched
        // `fence_depowered` state.
        "act.player.power_fence" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                fence_id: u64,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.fence_id == 0 {
                return Some(invalid_param_reason(request.id, "fence_id_zero"));
            }
            Some(success_response(
                request.id,
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "status": "accepted",
                    "method": method,
                    "fence_id": p.fence_id,
                    "powered": true,
                }),
            ))
        }
        // VAL-M9C-036 — the breaker-toggle path. Fires
        // `fence_depowered { cause: "breaker_toggled" }` so
        // wire_cutters succeed on the next contact.
        "act.player.unpower_fence" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                fence_id: u64,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.fence_id == 0 {
                return Some(invalid_param_reason(request.id, "fence_id_zero"));
            }
            Some(success_response(
                request.id,
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "status": "accepted",
                    "method": method,
                    "fence_id": p.fence_id,
                    "powered": false,
                    "cause": "breaker_toggled",
                }),
            ))
        }
        _ => Some(error_response(
            request.id,
            error_codes::METHOD_NOT_FOUND,
            "MethodNotFound",
            json!({"method": method, "fix_hint": "see spec/ai-control-observability-layer.md M0 method catalog"}),
        )),
    }
}
