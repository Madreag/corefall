//! M8 cfctl dispatch + observe surface implementation.
//!
//! Owns the mutation + event-emit logic for every M8 player UX cfctl
//! method (camera / photo / replay / killcam / debug / HUD layout +
//! smart commandable AI player UX surfaces — Tab tactical overlay,
//! Plan Composer, Q-hold context wheel, single-key panic, MMB tag,
//! 'Why?' key) so engine.rs doesn't grow further.
//!
//! The engine module declares every M8 EngineMutable field; this module
//! provides the per-method dispatcher that mutates that state + emits
//! the matching cf-replay event family + the `control.command_accepted`
//! envelope log.

use std::sync::RwLockWriteGuard;

use serde_json::{json, Value};

use cf_actor::{ActorId, IntentSource};
use cf_camera::CameraMode;
use cf_killcam::{start_slow_mo_kill_cam, KillcamPhase};
use cf_replay_scrub::WINDOW_SECONDS;
use cf_sim_core::Tick;
use cf_squad_ui::{
    context_wheel_for, ContextOrderKind, PanicCommand, PanicKind, PieMenuReason, PieMenuSelectError, PieMenuSlice,
    PieMenuTarget, Plan, PlanComposeError, PlanStepKind, ReticleTarget, DEFAULT_TAG_TTL_TICKS, MAX_PLAN_STEPS,
    PIE_MENU_SLICES_LEN, WHEEL_SLOTS_LEN,
};

use crate::engine::{EngineMutable, M0Engine};
use crate::server::CommandResult;

impl M0Engine {
    pub(crate) fn dispatch_camera_set_mode(
        &self,
        mode: String,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let parsed = match CameraMode::from_str(&mode) {
            Some(m) => m,
            None => {
                drop(state);
                self.record_command_rejected(tick, sim_time_ms, "act.camera.set_mode", "unknown_camera_mode");
                return CommandResult::rejected("unknown_camera_mode", tick.0);
            }
        };
        let mut state = state;
        let from = state.camera_state.mode;
        let fov_override = state.settings.scope_zoom_fov;
        let cursor_anchor = state.camera_state.position;
        let max_distance = state.camera_state.free_look_max_distance;
        match parsed {
            CameraMode::Follow => cf_camera::exit_scope(&mut state.camera_state),
            CameraMode::Scope => cf_camera::enter_scope(&mut state.camera_state, Some(fov_override)),
            CameraMode::FreeLook => cf_camera::enter_free_look(&mut state.camera_state, cursor_anchor, max_distance),
        }
        let to = state.camera_state.mode;
        let fov = state.camera_state.fov_degrees;
        drop(state);
        self.record_command_accepted(
            tick,
            sim_time_ms,
            "act.camera.set_mode",
            json!({"mode": parsed.as_str()}),
        );
        let payload = json!({"from": from.as_str(), "to": to.as_str(), "fov_degrees": fov});
        #[rustfmt::skip]
        let _ = self.recorder.record(tick, sim_time_ms, "camera", "mode_changed", payload, None);
        CommandResult::accepted(tick.0)
    }

    pub(crate) fn dispatch_camera_hit_stop(
        &self,
        duration_ms: u32,
        trigger: String,
        actor_id: Option<u64>,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let mut state = state;
        if !state.settings.hit_stop_enabled {
            drop(state);
            self.record_command_rejected(tick, sim_time_ms, "act.camera.hit_stop", "hit_stop_disabled");
            return CommandResult::rejected("hit_stop_disabled", tick.0);
        }
        cf_camera::trigger_hit_stop(&mut state.camera_state, duration_ms);
        let applied = state.camera_state.hit_stop_remaining_ms;
        drop(state);
        let accepted_payload = json!({"duration_ms": applied, "trigger": trigger.clone(), "actor_id": actor_id});
        self.record_command_accepted(tick, sim_time_ms, "act.camera.hit_stop", accepted_payload);
        let payload = json!({"duration_ms": applied, "trigger": trigger, "actor_id": actor_id});
        #[rustfmt::skip]
        let _ = self.recorder.record(tick, sim_time_ms, "camera", "hit_stop", payload, None);
        CommandResult::accepted(tick.0)
    }

    pub(crate) fn dispatch_camera_scope_zoom(
        &self,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let mut state = state;
        let from = state.camera_state.mode;
        let fov = state.settings.scope_zoom_fov;
        cf_camera::enter_scope(&mut state.camera_state, Some(fov));
        let to = state.camera_state.mode;
        let new_fov = state.camera_state.fov_degrees;
        drop(state);
        self.record_command_accepted(
            tick,
            sim_time_ms,
            "act.camera.scope_zoom",
            json!({"fov_degrees": new_fov}),
        );
        let payload = json!({"from": from.as_str(), "to": to.as_str(), "fov_degrees": new_fov});
        #[rustfmt::skip]
        let _ = self.recorder.record(tick, sim_time_ms, "camera", "mode_changed", payload, None);
        CommandResult::accepted(tick.0)
    }

    pub(crate) fn dispatch_camera_free_look_toggle(
        &self,
        active: bool,
        cursor: Option<(f32, f32)>,
        max_distance: f32,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let mut state = state;
        let from = state.camera_state.mode;
        if active {
            let cursor = cursor.unwrap_or(state.camera_state.position);
            cf_camera::enter_free_look(&mut state.camera_state, cursor, max_distance);
        } else {
            cf_camera::exit_free_look(&mut state.camera_state);
        }
        let to = state.camera_state.mode;
        let fov = state.camera_state.fov_degrees;
        drop(state);
        self.record_command_accepted(
            tick,
            sim_time_ms,
            "act.camera.free_look_toggle",
            json!({"active": active, "max_distance": max_distance, "cursor": cursor.map(|(x, y)| json!({"x": x, "y": y}))}),
        );
        let payload = json!({"from": from.as_str(), "to": to.as_str(), "fov_degrees": fov});
        #[rustfmt::skip]
        let _ = self.recorder.record(tick, sim_time_ms, "camera", "mode_changed", payload, None);
        CommandResult::accepted(tick.0)
    }

    pub(crate) fn dispatch_photo_enter(
        &self,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let mut state = state;
        let entered = state.photo_mode.enter();
        let filter = state.photo_mode.filter;
        drop(state);
        if !entered {
            self.record_command_rejected(tick, sim_time_ms, "act.photo.enter", "already_active");
            return CommandResult::rejected("already_active", tick.0);
        }
        self.record_command_accepted(tick, sim_time_ms, "act.photo.enter", json!({"filter": filter.as_str()}));
        let payload = json!({"filter": filter.as_str()});
        #[rustfmt::skip]
        let _ = self.recorder.record(tick, sim_time_ms, "photo_mode", "entered", payload, None);
        CommandResult::accepted(tick.0)
    }

    pub(crate) fn dispatch_photo_exit(
        &self,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let mut state = state;
        let exited = state.photo_mode.exit();
        let count = state.photo_mode.shot_count;
        drop(state);
        if !exited {
            self.record_command_rejected(tick, sim_time_ms, "act.photo.exit", "not_active");
            return CommandResult::rejected("not_active", tick.0);
        }
        self.record_command_accepted(tick, sim_time_ms, "act.photo.exit", json!({"shot_count": count}));
        let payload = json!({"shot_count": count});
        #[rustfmt::skip]
        let _ = self.recorder.record(tick, sim_time_ms, "photo_mode", "exited", payload, None);
        CommandResult::accepted(tick.0)
    }

    pub(crate) fn dispatch_photo_cycle_filter(
        &self,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let mut state = state;
        if !state.photo_mode.active {
            drop(state);
            self.record_command_rejected(tick, sim_time_ms, "act.photo.cycle_filter", "not_active");
            return CommandResult::rejected("not_active", tick.0);
        }
        let from = state.photo_mode.filter;
        let to = state.photo_mode.cycle_filter();
        drop(state);
        self.record_command_accepted(
            tick,
            sim_time_ms,
            "act.photo.cycle_filter",
            json!({"from": from.as_str(), "to": to.as_str()}),
        );
        let payload = json!({"from": from.as_str(), "to": to.as_str()});
        #[rustfmt::skip]
        let _ = self.recorder.record(tick, sim_time_ms, "photo_mode", "filter_changed", payload, None);
        CommandResult::accepted(tick.0)
    }

    pub(crate) fn dispatch_photo_shoot(
        &self,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let mut state = state;
        if !state.photo_mode.active {
            drop(state);
            self.record_command_rejected(tick, sim_time_ms, "act.photo.shoot", "not_active");
            return CommandResult::rejected("not_active", tick.0);
        }
        let filter = state.photo_mode.filter;
        let shot_index = state.photo_mode.record_shot();
        drop(state);
        self.record_command_accepted(
            tick,
            sim_time_ms,
            "act.photo.shoot",
            json!({"shot_index": shot_index, "filter": filter.as_str()}),
        );
        let payload = json!({"shot_index": shot_index, "filter": filter.as_str()});
        #[rustfmt::skip]
        let _ = self.recorder.record(tick, sim_time_ms, "photo_mode", "shot_taken", payload, None);
        CommandResult::accepted(tick.0)
    }

    pub(crate) fn dispatch_replay_scrub(
        &self,
        delta_seconds: f32,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let mut state = state;
        if !state.replay_scrub.open {
            state.replay_scrub.open();
        }
        let new_offset = state.replay_scrub.scrub(delta_seconds);
        let window = state.replay_scrub.window_seconds;
        drop(state);
        self.record_command_accepted(
            tick,
            sim_time_ms,
            "act.replay.scrub",
            json!({"delta_seconds": delta_seconds, "current_offset_seconds": new_offset, "window_seconds": window}),
        );
        let payload =
            json!({"delta_seconds": delta_seconds, "current_offset_seconds": new_offset, "window_seconds": window});
        #[rustfmt::skip]
        let _ = self.recorder.record(tick, sim_time_ms, "replay", "scrub_offset_changed", payload, None);
        CommandResult::accepted(tick.0)
    }

    pub(crate) fn dispatch_replay_bookmark(
        &self,
        label: String,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        if label.is_empty() {
            drop(state);
            self.record_command_rejected(tick, sim_time_ms, "act.replay.bookmark", "label_required");
            return CommandResult::rejected("label_required", tick.0);
        }
        let mut state = state;
        let total = state.replay_scrub.add_bookmark(tick.0, label.clone());
        let _ = WINDOW_SECONDS;
        drop(state);
        self.record_command_accepted(
            tick,
            sim_time_ms,
            "act.replay.bookmark",
            json!({"label": label.clone(), "total": total}),
        );
        let payload = json!({"tick": tick.0, "label": label});
        #[rustfmt::skip]
        let _ = self.recorder.record(tick, sim_time_ms, "replay", "bookmark_added", payload, None);
        CommandResult::accepted(tick.0)
    }

    pub(crate) fn dispatch_debug_toggle_overlay(
        &self,
        overlay_id: String,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let parsed = match cf_debug::DebugOverlay::from_str(&overlay_id) {
            Some(o) => o,
            None => {
                drop(state);
                self.record_command_rejected(tick, sim_time_ms, "act.debug.toggle_overlay", "unknown_overlay");
                return CommandResult::rejected("unknown_overlay", tick.0);
            }
        };
        let mut state = state;
        let enabled = state.debug_state.toggle(parsed);
        if enabled {
            state.settings.debug_overlays.insert(parsed.as_str().to_string());
        } else {
            state.settings.debug_overlays.remove(parsed.as_str());
        }
        drop(state);
        self.record_command_accepted(
            tick,
            sim_time_ms,
            "act.debug.toggle_overlay",
            json!({"overlay": parsed.as_str(), "enabled": enabled}),
        );
        let payload = json!({"overlay": parsed.as_str(), "enabled": enabled});
        #[rustfmt::skip]
        let _ = self.recorder.record(tick, sim_time_ms, "ux", "debug_overlay_toggled", payload, None);
        CommandResult::accepted(tick.0)
    }

    pub(crate) fn dispatch_ui_set_hud_layout(
        &self,
        node: String,
        x: f32,
        y: f32,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        drop(state);
        if node.is_empty() {
            self.record_command_rejected(tick, sim_time_ms, "act.ui.set_hud_layout", "node_required");
            return CommandResult::rejected("node_required", tick.0);
        }
        self.record_command_accepted(
            tick,
            sim_time_ms,
            "act.ui.set_hud_layout",
            json!({"node": node.clone(), "x": x, "y": y}),
        );
        let payload = json!({"node": node, "x": x, "y": y});
        #[rustfmt::skip]
        let _ = self.recorder.record(tick, sim_time_ms, "ux", "hud_layout_changed", payload, None);
        CommandResult::accepted(tick.0)
    }

    pub(crate) fn dispatch_ui_save_preset(
        &self,
        name: String,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        drop(state);
        self.record_command_accepted(tick, sim_time_ms, "act.ui.save_preset", json!({"name": name.clone()}));
        let payload = json!({"name": name});
        #[rustfmt::skip]
        let _ = self.recorder.record(tick, sim_time_ms, "ux", "preset_saved", payload, None);
        CommandResult::accepted(tick.0)
    }

    pub(crate) fn dispatch_toggle_tactical_overlay(
        &self,
        multiplayer: bool,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let mut state = state;
        let opened = state.tactical_overlay.toggle(multiplayer);
        let speed = state.tactical_overlay.sim_speed_pct;
        let focused = state.tactical_overlay.focused_actor_id;
        let count = state.tactical_overlay.open_count;
        drop(state);
        self.record_command_accepted(
            tick,
            sim_time_ms,
            "act.player.toggle_tactical_overlay",
            json!({"open": opened, "sim_speed_pct": speed, "focused_actor_id": focused, "open_count": count}),
        );
        let payload = json!({"open": opened, "sim_speed_pct": speed, "focused_actor_id": focused, "open_count": count});
        #[rustfmt::skip]
        let _ = self.recorder.record(tick, sim_time_ms, "ux", "tactical_overlay_toggled", payload, None);
        CommandResult::accepted(tick.0)
    }

    pub(crate) fn dispatch_compose_plan(
        &self,
        actor_id: u64,
        steps: Vec<String>,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        if steps.len() > MAX_PLAN_STEPS {
            drop(state);
            self.record_command_rejected(tick, sim_time_ms, "act.player.compose_plan", "plan_full");
            return CommandResult::rejected("plan_full", tick.0);
        }
        let kinds: Vec<PlanStepKind> = steps
            .iter()
            .map(|s| match s.as_str() {
                "flank_left" => PlanStepKind::FlankLeft,
                "flank_right" => PlanStepKind::FlankRight,
                "breach_door" => PlanStepKind::BreachDoor,
                "overwatch" => PlanStepKind::Overwatch,
                "throw_flash" => PlanStepKind::ThrowFlash,
                "stack_left" => PlanStepKind::StackLeft,
                "stack_right" => PlanStepKind::StackRight,
                "wait_for_go" => PlanStepKind::WaitForGo,
                "hold_east_corner" => PlanStepKind::HoldEastCorner,
                other => PlanStepKind::Custom(other.to_string()),
            })
            .collect();
        let mut state = state;
        let mut plan = Plan::empty(ActorId(actor_id).0);
        for k in kinds {
            if let Err(PlanComposeError::PlanFull) = plan.add_step(k, None) {
                drop(state);
                self.record_command_rejected(tick, sim_time_ms, "act.player.compose_plan", "plan_full");
                return CommandResult::rejected("plan_full", tick.0);
            }
        }
        let label_steps: Vec<String> = plan.steps.iter().map(|s| s.kind.label().to_string()).collect();
        let count = plan.steps.len();
        state.plans.insert(ActorId(actor_id), plan);
        drop(state);
        self.record_command_accepted(
            tick,
            sim_time_ms,
            "act.player.compose_plan",
            json!({"actor_id": actor_id, "step_count": count, "steps": label_steps.clone()}),
        );
        let composed_payload = json!({"actor_id": actor_id, "step_count": count, "steps": label_steps});
        #[rustfmt::skip]
        let _ = self.recorder.record(tick, sim_time_ms, "ai", "plan_composed", composed_payload, None);
        // M8 basic stub: empty step lists clear the bot's plan (abort);
        // non-empty step lists immediately execute the queued plan. M33+
        // adds a separate `act.player.execute_plan` cfctl method gated by
        // a "GO" key. The stub form ensures the schema-obligated
        // `ai.plan_aborted` and `ai.plan_executed` event families emit
        // from a real production path.
        if count == 0 {
            let aborted_payload = json!({"plan_count": 1, "actor_ids": [actor_id]});
            #[rustfmt::skip]
            let _ = self.recorder.record(tick, sim_time_ms, "ai", "plan_aborted", aborted_payload, None);
        } else {
            let executed_payload = json!({"plan_count": 1, "actor_ids": [actor_id]});
            #[rustfmt::skip]
            let _ = self.recorder.record(tick, sim_time_ms, "ai", "plan_executed", executed_payload, None);
        }
        CommandResult::accepted(tick.0)
    }

    pub(crate) fn dispatch_context_wheel_select(
        &self,
        actor_id: u64,
        slot: u8,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        if (slot as usize) >= WHEEL_SLOTS_LEN {
            drop(state);
            self.record_command_rejected(tick, sim_time_ms, "act.player.context_wheel_select", "invalid_slot");
            return CommandResult::rejected("invalid_slot", tick.0);
        }
        let wheel = context_wheel_for(ReticleTarget::None);
        let order: ContextOrderKind = wheel.slots[slot as usize].order;
        drop(state);
        self.record_command_accepted(
            tick,
            sim_time_ms,
            "act.player.context_wheel_select",
            json!({"actor_id": actor_id, "slot": slot, "order": order.as_str()}),
        );
        let opened_payload = json!({"target_kind": "none", "target_id": serde_json::Value::Null});
        #[rustfmt::skip]
        let _ = self.recorder.record(tick, sim_time_ms, "ai", "context_wheel_opened", opened_payload, None);
        let selected_payload = json!({"actor_id": actor_id, "slot": slot, "order": order.as_str()});
        #[rustfmt::skip]
        let _ = self.recorder.record(tick, sim_time_ms, "ai", "context_wheel_selected", selected_payload, None);
        CommandResult::accepted(tick.0)
    }

    pub(crate) fn dispatch_panic_call(
        &self,
        kind: String,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let parsed = match PanicKind::from_str(&kind) {
            Some(k) => k,
            None => {
                drop(state);
                self.record_command_rejected(tick, sim_time_ms, "act.player.panic_call", "unknown_panic_kind");
                return CommandResult::rejected("unknown_panic_kind", tick.0);
            }
        };
        let issuer = state.player_actor.unwrap_or_default().0;
        let cmd: PanicCommand = PanicCommand::no_responder(parsed, issuer);
        drop(state);
        self.record_command_accepted(
            tick,
            sim_time_ms,
            "act.player.panic_call",
            json!({
                "kind": parsed.as_str(),
                "issuer_actor_id": issuer,
                "responder_found": cmd.responder_found,
                "responder_actor_id": cmd.responder_actor_id,
            }),
        );
        let payload = json!({
            "kind": parsed.as_str(),
            "issuer_actor_id": issuer,
            "responder_found": cmd.responder_found,
            "responder_actor_id": cmd.responder_actor_id,
        });
        #[rustfmt::skip]
        let _ = self.recorder.record(tick, sim_time_ms, "ai", "panic_call_emitted", payload, None);
        CommandResult::accepted(tick.0)
    }

    pub(crate) fn dispatch_tag_target(
        &self,
        target_id: u64,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let mut state = state;
        let issuer = state.player_actor.unwrap_or_default().0;
        let info = state
            .tag_state
            .add_tag(target_id, tick.0, DEFAULT_TAG_TTL_TICKS, issuer)
            .clone();
        drop(state);
        self.record_command_accepted(
            tick,
            sim_time_ms,
            "act.player.tag_target",
            json!({"target_id": target_id, "weight_bonus": info.weight_bonus, "expires_at_tick": info.expires_at_tick}),
        );
        let payload = json!({
            "target_id": target_id,
            "issuer_actor_id": issuer,
            "weight_bonus": info.weight_bonus,
            "expires_at_tick": info.expires_at_tick,
            "ttl_ticks": DEFAULT_TAG_TTL_TICKS,
        });
        #[rustfmt::skip]
        let _ = self.recorder.record(tick, sim_time_ms, "ai", "target_tagged", payload, None);
        CommandResult::accepted(tick.0)
    }

    pub(crate) fn dispatch_query_why(
        &self,
        actor_id: u64,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let bot_id = ActorId(actor_id);
        let (label, recent_count): (Option<String>, usize) = match state.m7_ai_world.bots.get(&bot_id) {
            Some(bot) => {
                let label = bot.stack.reason_labels.latest().map(|l| l.format());
                let cnt = bot.stack.reason_labels.len();
                (label, cnt)
            }
            None => (None, 0),
        };
        drop(state);
        self.record_command_accepted(
            tick,
            sim_time_ms,
            "act.player.query_why",
            json!({"actor_id": actor_id, "reason_label": label.clone(), "recent_count": recent_count}),
        );
        let payload = json!({"actor_id": actor_id, "reason_label": label, "recent_count": recent_count});
        #[rustfmt::skip]
        let _ = self.recorder.record(tick, sim_time_ms, "ai", "reason_query_returned", payload, None);
        CommandResult::accepted(tick.0)
    }

    /// **M8**: open the T-key 8-slice pie menu with target context per
    /// spec § Pie menu. Slows sim to 20% in single-player; 100% in
    /// multiplayer. Emits `ux.pie_menu_opened`.
    pub(crate) fn dispatch_pie_menu_open(
        &self,
        target_kind: String,
        target_id: Option<u64>,
        multiplayer: bool,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let target = match PieMenuTarget::from_str(&target_kind, target_id) {
            Some(t) => t,
            None => {
                drop(state);
                self.record_command_rejected(tick, sim_time_ms, "act.player.pie_menu_open", "unknown_target_kind");
                return CommandResult::rejected("unknown_target_kind", tick.0);
            }
        };
        let mut state = state;
        let opened = state.pie_menu.open(target.clone(), multiplayer, tick.0);
        if !opened {
            let kind = state.pie_menu.target.kind_str();
            drop(state);
            self.record_command_rejected(tick, sim_time_ms, "act.player.pie_menu_open", "already_open");
            let _ = kind;
            return CommandResult::rejected("already_open", tick.0);
        }
        let slowdown = state.pie_menu.slowdown_factor_pct;
        let open_count = state.pie_menu.open_count;
        drop(state);
        self.record_command_accepted(
            tick,
            sim_time_ms,
            "act.player.pie_menu_open",
            json!({
                "target_kind": target.kind_str(),
                "target_id": target.target_id(),
                "slowdown_factor_pct": slowdown,
                "multiplayer": multiplayer,
                "open_count": open_count,
            }),
        );
        let payload = json!({
            "target_kind": target.kind_str(),
            "target_id": target.target_id(),
            "slowdown_factor_pct": slowdown,
            "multiplayer": multiplayer,
            "open_count": open_count,
            "open_tick": tick.0,
        });
        #[rustfmt::skip]
        let _ = self.recorder.record(tick, sim_time_ms, "ux", "pie_menu_opened", payload, None);
        CommandResult::accepted(tick.0)
    }

    /// **M8**: select a slot on the open pie menu. Valid slot + no
    /// reason → `ux.pie_menu_slice_chosen`. Valid slot + supplied
    /// `reason` → `ux.pie_menu_slice_rejected { slice, reason }`.
    pub(crate) fn dispatch_pie_menu_select(
        &self,
        slot: u8,
        reason: Option<String>,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        if (slot as usize) >= PIE_MENU_SLICES_LEN {
            drop(state);
            self.record_command_rejected(tick, sim_time_ms, "act.player.pie_menu_select", "invalid_slot");
            return CommandResult::rejected("invalid_slot", tick.0);
        }
        let parsed_reason: Option<PieMenuReason> = match reason.as_deref() {
            Some(r) => match PieMenuReason::from_str(r) {
                Some(p) => Some(p),
                None => {
                    drop(state);
                    self.record_command_rejected(tick, sim_time_ms, "act.player.pie_menu_select", "unknown_reason");
                    return CommandResult::rejected("unknown_reason", tick.0);
                }
            },
            None => None,
        };
        let mut state = state;
        if !state.pie_menu.open {
            drop(state);
            self.record_command_rejected(tick, sim_time_ms, "act.player.pie_menu_select", "menu_not_open");
            return CommandResult::rejected("menu_not_open", tick.0);
        }
        let target_kind: &'static str = state.pie_menu.target.kind_str();
        let target_id_opt: Option<u64> = state.pie_menu.target.target_id();
        let outcome = state.pie_menu.select(slot, parsed_reason);
        drop(state);
        match outcome {
            Ok(slice) => {
                self.record_command_accepted(
                    tick,
                    sim_time_ms,
                    "act.player.pie_menu_select",
                    json!({
                        "slot": slot,
                        "slice": slice.as_str(),
                        "target_kind": target_kind,
                        "target_id": target_id_opt,
                        "outcome": "chosen",
                    }),
                );
                let payload = json!({
                    "slot": slot,
                    "slice": slice.as_str(),
                    "target_kind": target_kind,
                    "target_id": target_id_opt,
                });
                #[rustfmt::skip]
                let _ = self.recorder.record(tick, sim_time_ms, "ux", "pie_menu_slice_chosen", payload, None);
                CommandResult::accepted(tick.0)
            }
            Err(PieMenuSelectError::Rejected { slice, reason }) => {
                self.record_command_accepted(
                    tick,
                    sim_time_ms,
                    "act.player.pie_menu_select",
                    json!({
                        "slot": slot,
                        "slice": slice.as_str(),
                        "reason": reason.as_str(),
                        "target_kind": target_kind,
                        "target_id": target_id_opt,
                        "outcome": "rejected",
                    }),
                );
                let payload = json!({
                    "slot": slot,
                    "slice": slice.as_str(),
                    "reason": reason.as_str(),
                    "target_kind": target_kind,
                    "target_id": target_id_opt,
                });
                #[rustfmt::skip]
                let _ = self.recorder.record(tick, sim_time_ms, "ux", "pie_menu_slice_rejected", payload, None);
                CommandResult::accepted(tick.0)
            }
            Err(PieMenuSelectError::InvalidSlot(_)) => {
                self.record_command_rejected(tick, sim_time_ms, "act.player.pie_menu_select", "invalid_slot");
                CommandResult::rejected("invalid_slot", tick.0)
            }
            Err(PieMenuSelectError::NotOpen) => {
                self.record_command_rejected(tick, sim_time_ms, "act.player.pie_menu_select", "menu_not_open");
                CommandResult::rejected("menu_not_open", tick.0)
            }
        }
    }

    /// **M8**: close the pie menu (idempotent). Emits
    /// `ux.pie_menu_closed` with the open duration in ticks. The
    /// `slice_chosen` field is `null` when closed without a selection.
    pub(crate) fn dispatch_pie_menu_close(
        &self,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let mut state = state;
        if !state.pie_menu.open {
            drop(state);
            self.record_command_rejected(tick, sim_time_ms, "act.player.pie_menu_close", "menu_not_open");
            return CommandResult::rejected("menu_not_open", tick.0);
        }
        let target_kind = state.pie_menu.target.kind_str();
        let target_id_opt = state.pie_menu.target.target_id();
        let opened_at = state.pie_menu.open_tick.unwrap_or(tick.0);
        let last_slice: Option<PieMenuSlice> = state.pie_menu.slice_under_cursor.and_then(PieMenuSlice::from_slot);
        let open_duration_ticks: u64 = tick.0.saturating_sub(opened_at);
        let was_open = state.pie_menu.close();
        drop(state);
        let slice_str: Option<&'static str> = last_slice.map(PieMenuSlice::as_str);
        self.record_command_accepted(
            tick,
            sim_time_ms,
            "act.player.pie_menu_close",
            json!({
                "target_kind": target_kind,
                "target_id": target_id_opt,
                "open_duration_ticks": open_duration_ticks,
                "slice_chosen": slice_str,
                "was_open": was_open,
            }),
        );
        let payload = json!({
            "target_kind": target_kind,
            "target_id": target_id_opt,
            "open_duration_ticks": open_duration_ticks,
            "slice_chosen": slice_str,
        });
        #[rustfmt::skip]
        let _ = self.recorder.record(tick, sim_time_ms, "ux", "pie_menu_closed", payload, None);
        CommandResult::accepted(tick.0)
    }

    /// **M8 helper**: trigger the slow-mo cinematic kill cam (boss final
    /// blow) from any engine site that detects the boss-down condition.
    /// Honors `Settings.cinematic_kills`. Emits
    /// `slow_mo.kill_cam_triggered` on success.
    pub(crate) fn trigger_slow_mo_kill_cam(
        &self,
        killer: u64,
        victim: u64,
        tick: Tick,
        sim_time_ms: f64,
        state: &mut EngineMutable,
    ) -> bool {
        let enabled = state.settings.cinematic_kills;
        let res = start_slow_mo_kill_cam(&mut state.killcam, killer, victim, enabled);
        match res {
            cf_killcam::recorder::KillcamTrigger::Started => {
                let payload = json!({
                    "killer_actor_id": killer,
                    "victim_actor_id": victim,
                    "duration_ms": cf_killcam::SLOW_MO_KILL_CAM_DURATION_MS,
                });
                #[rustfmt::skip]
                let _ = self.recorder.record(tick, sim_time_ms, "slow_mo", "kill_cam_triggered", payload, None);
                true
            }
            cf_killcam::recorder::KillcamTrigger::Skipped | cf_killcam::recorder::KillcamTrigger::AlreadyActive => {
                false
            }
        }
    }

    /// **M8 helper**: trigger a regular killcam on player death.
    /// Honors `Settings.killcam_enabled`. Emits `killcam.played` on
    /// success or `killcam.skipped` on opt-out.
    pub(crate) fn trigger_killcam_on_death(
        &self,
        killer: Option<u64>,
        victim: u64,
        tick: Tick,
        sim_time_ms: f64,
        state: &mut EngineMutable,
    ) -> bool {
        let enabled = state.settings.killcam_enabled;
        if killer.is_none() {
            let payload = json!({"victim_actor_id": victim, "reason": "no_killer"});
            #[rustfmt::skip]
            let _ = self.recorder.record(tick, sim_time_ms, "killcam", "skipped", payload, None);
            return false;
        }
        if !enabled {
            let payload = json!({"victim_actor_id": victim, "reason": "disabled_in_settings"});
            #[rustfmt::skip]
            let _ = self.recorder.record(tick, sim_time_ms, "killcam", "skipped", payload, None);
            return false;
        }
        let killer_id = killer.unwrap();
        let res = cf_killcam::start(&mut state.killcam, killer_id, victim, enabled);
        match res {
            cf_killcam::recorder::KillcamTrigger::Started => {
                let payload = json!({
                    "killer_actor_id": killer_id,
                    "victim_actor_id": victim,
                    "duration_ms": cf_killcam::KILLCAM_DURATION_MS,
                    "slow_mo_kill_cam": false,
                });
                #[rustfmt::skip]
                let _ = self.recorder.record(tick, sim_time_ms, "killcam", "played", payload, None);
                true
            }
            cf_killcam::recorder::KillcamTrigger::AlreadyActive => {
                let payload = json!({"victim_actor_id": victim, "reason": "already_active"});
                #[rustfmt::skip]
                let _ = self.recorder.record(tick, sim_time_ms, "killcam", "skipped", payload, None);
                false
            }
            cf_killcam::recorder::KillcamTrigger::Skipped => {
                let payload = json!({"victim_actor_id": victim, "reason": "disabled_in_settings"});
                #[rustfmt::skip]
                let _ = self.recorder.record(tick, sim_time_ms, "killcam", "skipped", payload, None);
                false
            }
        }
    }

    /// Per-frame state advance for the M8 surfaces — camera hit-stop
    /// decay, killcam playback progression (Recording → Playing → Done
    /// → Idle), and MMB tag TTL expiry. `engine.rs::drive_tick` invokes
    /// this once per tick via [`M0Engine::tick_m8`]; tests can call it
    /// directly with a synthesised `EngineMutable` and explicit
    /// `current_tick_value` so the surface is exercisable without a
    /// full engine bootstrap.
    pub(crate) fn tick_m8_state(&self, dt_ms: u32, current_tick_value: u64, state: &mut EngineMutable) {
        cf_camera::tick_hit_stop(&mut state.camera_state, dt_ms);
        let phase = cf_killcam::tick(&mut state.killcam, dt_ms);
        if phase == KillcamPhase::Done {
            state.killcam.reset();
        }
        let _ = state.tag_state.expire_old(current_tick_value);
    }

    /// **M8**: helper for cfctl `observe.camera`.
    pub(crate) fn snapshot_camera_state(&self) -> Value {
        let s = self.state.read().expect("engine state poisoned");
        let cam = &s.camera_state;
        json!({
            "schema_version": crate::SCHEMA_VERSION,
            "mode": cam.mode.as_str(),
            "position": { "x": cam.position.0, "y": cam.position.1 },
            "target": { "x": cam.target.0, "y": cam.target.1 },
            "lookahead_offset": { "x": cam.lookahead_offset.0, "y": cam.lookahead_offset.1 },
            "hit_stop_remaining_ms": cam.hit_stop_remaining_ms,
            "fov_degrees": cam.fov_degrees,
            "free_look_max_distance": cam.free_look_max_distance,
            "deadzone_radius": cam.deadzone_radius,
        })
    }

    /// **M8**: helper for cfctl `observe.localization.current_language`.
    pub(crate) fn snapshot_localization_language(&self) -> Value {
        let s = self.state.read().expect("engine state poisoned");
        json!({
            "schema_version": crate::SCHEMA_VERSION,
            "language": s.settings.language.clone(),
            "key_count": s.localization.len(),
        })
    }

    /// **M8**: helper for cfctl `observe.debug.overlays`.
    pub(crate) fn snapshot_debug_overlays(&self) -> Value {
        let s = self.state.read().expect("engine state poisoned");
        let enabled: Vec<&str> = s.debug_state.enabled_ids();
        let available: Vec<&str> = cf_debug::DebugOverlay::ALL.iter().map(|o| o.as_str()).collect();
        json!({
            "schema_version": crate::SCHEMA_VERSION,
            "enabled": enabled,
            "available": available,
            "render_allowed": cf_debug::DebugOverlayState::render_allowed(false, s.settings.debug_enabled),
        })
    }

    /// **M8**: helper for cfctl `observe.tactical_overlay`.
    pub(crate) fn snapshot_tactical_overlay(&self) -> Value {
        let s = self.state.read().expect("engine state poisoned");
        json!({
            "schema_version": crate::SCHEMA_VERSION,
            "open": s.tactical_overlay.open,
            "sim_speed_pct": s.tactical_overlay.sim_speed_pct,
            "focused_actor_id": s.tactical_overlay.focused_actor_id,
            "open_count": s.tactical_overlay.open_count,
        })
    }

    /// **M8**: helper for cfctl `observe.tags`.
    pub(crate) fn snapshot_tags(&self) -> Value {
        let s = self.state.read().expect("engine state poisoned");
        let tagged: Vec<Value> = s
            .tag_state
            .tagged
            .iter()
            .map(|(target_id, info)| {
                json!({
                    "target_id": target_id,
                    "tagged_at_tick": info.tagged_at_tick,
                    "expires_at_tick": info.expires_at_tick,
                    "weight_bonus": info.weight_bonus,
                    "issuer_actor_id": info.issuer_actor_id,
                })
            })
            .collect();
        json!({
            "schema_version": crate::SCHEMA_VERSION,
            "tagged": tagged,
        })
    }
}
