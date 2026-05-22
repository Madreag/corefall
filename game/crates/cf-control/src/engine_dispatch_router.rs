//! Router for `EngineHandle::dispatch`. Body extracted from engine_handle.rs.

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
use crate::engine_handle::{load_trench_template, resolved_fortifications_for_build};
use crate::scenario::Scenario;
use crate::server::{async_trait, CommandResult, ControlCommand, EngineHandle, SettingsPatch};
use crate::state::{ActorView, ObserveFrame, ObserveSettings, RunStatus};
use crate::{Settings, SCHEMA_VERSION};

impl M0Engine {
    pub(crate) async fn dispatch_command(&self, command: ControlCommand) -> CommandResult {
        let mut state = self.state.write().expect("engine state poisoned");
        let tick = state.clock.tick();
        let sim_time_ms = state.clock.sim_time_ms();
        // **M12C** § "Player gameplay input is blocked (only skip / pause
        // accepted)" while a cinematic is playing. Mirror of the
        // `controls_captured_by` gate below — keyed off the cinematic
        // kernel rather than the overlay capture flag. Squad/camera/UI
        // commands flow through to preserve cfctl scripting hooks.
        if state
            .cinematic_kernel
            .as_ref()
            .is_some_and(|k| k.blocks_gameplay_input())
        {
            let method = match &command {
                ControlCommand::ActPlayerMove { .. } => Some("act.player.move"),
                ControlCommand::ActPlayerJump { .. } => Some("act.player.jump"),
                ControlCommand::ActPlayerAim { .. } => Some("act.player.aim"),
                ControlCommand::ActPlayerFire { .. } => Some("act.player.fire"),
                ControlCommand::ActPlayerReload { .. } => Some("act.player.reload"),
                ControlCommand::ActPlayerSelectItem { .. } => Some("act.player.select_item"),
                ControlCommand::ActPlayerReset { .. } => Some("act.player.reset"),
                ControlCommand::ActPlayerDig { .. } => Some("act.player.dig"),
                ControlCommand::ActPlayerAnchor { .. } => Some("act.player.anchor"),
                ControlCommand::ActPlayerCrouch { .. } => Some("act.player.crouch"),
                ControlCommand::ActPlayerClimb { .. } => Some("act.player.climb"),
                ControlCommand::ActPlayerJet { .. } => Some("act.player.jet"),
                ControlCommand::ActPlayerEject { .. } => Some("act.player.eject"),
                ControlCommand::ActPlayerQuickActionSlot { .. } => Some("act.player.quick_action_slot"),
                ControlCommand::ActPlayerQuickActionToggle { .. } => Some("act.player.quick_action_toggle"),
                ControlCommand::ActPlayerQuickActionRadial { .. } => Some("act.player.quick_action_radial"),
                ControlCommand::ActPlayerQuickActionSlice { .. } => Some("act.player.quick_action_slice"),
                ControlCommand::ActPlayerWeaponCycle { .. } => Some("act.player.weapon_cycle"),
                ControlCommand::ActPlayerSharpAim { .. } => Some("act.player.sharp_aim"),
                ControlCommand::ActM6 { action, .. } => Some(action.method_name()),
                ControlCommand::ActPlayerBrainHop { .. } => Some("act.player.brain_hop"),
                ControlCommand::ActPlayerActivateAbility { .. } => Some("act.player.activate_ability"),
                ControlCommand::ActPlayerAttachModifier { .. } => Some("act.player.attach_modifier"),
                ControlCommand::ActPlayerDetachModifier { .. } => Some("act.player.detach_modifier"),
                ControlCommand::ActPlayerBoard { .. } => Some("act.player.board"),
                ControlCommand::ActPlayerDisembark { .. } => Some("act.player.disembark"),
                ControlCommand::ActPlayerSetDroneMode { .. } => Some("act.player.set_drone_mode"),
                _ => None,
            };
            if let Some(method_name) = method {
                let cinematic_id = state
                    .cinematic_kernel
                    .as_ref()
                    .and_then(|k| k.state().cinematic_id.clone())
                    .unwrap_or_default();
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({
                        "method": method_name,
                        "reason": "cinematic_active",
                        "cinematic_id": cinematic_id,
                    }),
                    None,
                );
                return CommandResult::rejected("cinematic_active", tick.0);
            }
        }
        // Gap D2: while an overlay has captured controls, reject every
        // `act.player.*` command. Capture/release commands themselves still
        // flow through so the UI can release the capture.
        if let Some(capturer) = state.controls_captured_by.clone() {
            let method = match &command {
                ControlCommand::ActPlayerMove { .. } => Some("act.player.move"),
                ControlCommand::ActPlayerJump { .. } => Some("act.player.jump"),
                ControlCommand::ActPlayerAim { .. } => Some("act.player.aim"),
                ControlCommand::ActPlayerFire { .. } => Some("act.player.fire"),
                ControlCommand::ActPlayerReload { .. } => Some("act.player.reload"),
                ControlCommand::ActPlayerSelectItem { .. } => Some("act.player.select_item"),
                ControlCommand::ActPlayerReset { .. } => Some("act.player.reset"),
                ControlCommand::ActPlayerDig { .. } => Some("act.player.dig"),
                ControlCommand::ActPlayerAnchor { .. } => Some("act.player.anchor"),
                ControlCommand::ActPlayerCrouch { .. } => Some("act.player.crouch"),
                ControlCommand::ActPlayerClimb { .. } => Some("act.player.climb"),
                ControlCommand::ActPlayerJet { .. } => Some("act.player.jet"),
                ControlCommand::ActPlayerEject { .. } => Some("act.player.eject"),
                ControlCommand::ActPlayerQuickActionSlot { .. } => Some("act.player.quick_action_slot"),
                ControlCommand::ActPlayerQuickActionToggle { .. } => Some("act.player.quick_action_toggle"),
                ControlCommand::ActPlayerQuickActionRadial { .. } => Some("act.player.quick_action_radial"),
                ControlCommand::ActPlayerQuickActionSlice { .. } => Some("act.player.quick_action_slice"),
                ControlCommand::ActPlayerWeaponCycle { .. } => Some("act.player.weapon_cycle"),
                ControlCommand::ActPlayerSharpAim { .. } => Some("act.player.sharp_aim"),
                ControlCommand::ActPlayerAbort { .. } => Some("act.player.abort"),
                ControlCommand::ActM6 { action, .. } => Some(action.method_name()),
                ControlCommand::ActSquadIssueCommand { .. } => Some("act.squad.issue_command"),
                ControlCommand::ActSquadCancelCommand { .. } => Some("act.squad.cancel_command"),
                // **M7B**: squad-command grammar surface — observable in
                // capture mode so the player still hears the rejection.
                ControlCommand::ActSquadIssue { .. } => Some("act.squad.issue"),
                ControlCommand::ActSquadSetFormation { .. } => Some("act.squad.set_formation"),
                ControlCommand::ActSquadAssignRole { .. } => Some("act.squad.assign_role"),
                // **M13** chassis-grade methods rejected during input capture.
                ControlCommand::ActPlayerBrainHop { .. } => Some("act.player.brain_hop"),
                ControlCommand::ActPlayerActivateAbility { .. } => Some("act.player.activate_ability"),
                ControlCommand::ActInputCameraAnchor { .. } => Some("act.input.camera_anchor"),
                ControlCommand::ActPlayerSetDroneMode { .. } => Some("act.player.set_drone_mode"),
                ControlCommand::ActPlayerAttachModifier { .. } => Some("act.player.attach_modifier"),
                ControlCommand::ActPlayerDetachModifier { .. } => Some("act.player.detach_modifier"),
                ControlCommand::ActPlayerBoard { .. } => Some("act.player.board"),
                ControlCommand::ActPlayerDisembark { .. } => Some("act.player.disembark"),
                _ => None,
            };
            if let Some(method_name) = method {
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({
                        "method": method_name,
                        "reason": "controls_captured",
                        "capturer": capturer,
                    }),
                    None,
                );
                return CommandResult::rejected("controls_captured", tick.0);
            }
        }
        // **M13** § "Boarding / disembarking transitions" — "Input rejected
        // during transition". When the player's chassis is mid-transition,
        // every act.player.* (except board/disembark/eject) is rejected with
        // `chassis_in_transition`. This mirrors the M1 controls-capture gate
        // above but keyed off chassis state rather than overlay capture.
        // Uses the already-held `state` write lock to avoid deadlock — DO
        // NOT call self.state.read() here, this function owns the write lock.
        // **M14 audit pass 4 (Finding 1)**: also include the player-side
        // boarding timer in the input-lock gate. A foot soldier mid-board
        // has no chassis yet, so a chassis-only gate would let them
        // continue moving / firing during the 1500ms transition.
        let mid_transition = state
            .player_actor
            .and_then(|pid| state.actor_state.as_ref().and_then(|sim| sim.world.actors.get(&pid)))
            .map(|a| {
                a.boarding_ticks_remaining > 0
                    || a.pending_boarding_target.is_some()
                    || a.chassis.as_ref().map(|c| c.is_in_transition()).unwrap_or(false)
            })
            .unwrap_or(false);
        if mid_transition {
            let method = match &command {
                ControlCommand::ActPlayerMove { .. } => Some("act.player.move"),
                ControlCommand::ActPlayerJump { .. } => Some("act.player.jump"),
                ControlCommand::ActPlayerAim { .. } => Some("act.player.aim"),
                ControlCommand::ActPlayerFire { .. } => Some("act.player.fire"),
                ControlCommand::ActPlayerReload { .. } => Some("act.player.reload"),
                ControlCommand::ActPlayerSelectItem { .. } => Some("act.player.select_item"),
                ControlCommand::ActPlayerDig { .. } => Some("act.player.dig"),
                ControlCommand::ActPlayerAnchor { .. } => Some("act.player.anchor"),
                ControlCommand::ActPlayerCrouch { .. } => Some("act.player.crouch"),
                ControlCommand::ActPlayerClimb { .. } => Some("act.player.climb"),
                ControlCommand::ActPlayerJet { .. } => Some("act.player.jet"),
                ControlCommand::ActPlayerQuickActionSlot { .. } => Some("act.player.quick_action_slot"),
                ControlCommand::ActPlayerQuickActionToggle { .. } => Some("act.player.quick_action_toggle"),
                ControlCommand::ActPlayerQuickActionRadial { .. } => Some("act.player.quick_action_radial"),
                ControlCommand::ActPlayerQuickActionSlice { .. } => Some("act.player.quick_action_slice"),
                ControlCommand::ActPlayerWeaponCycle { .. } => Some("act.player.weapon_cycle"),
                ControlCommand::ActPlayerActivateAbility { .. } => Some("act.player.activate_ability"),
                ControlCommand::ActPlayerAttachModifier { .. } => Some("act.player.attach_modifier"),
                ControlCommand::ActPlayerDetachModifier { .. } => Some("act.player.detach_modifier"),
                ControlCommand::ActM6 { action, .. } => Some(action.method_name()),
                _ => None,
            };
            if let Some(method_name) = method {
                let player_actor_id = state.player_actor.map(|p| p.0).unwrap_or(0);
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({
                        "method": method_name,
                        "reason": "chassis_in_transition",
                        "actor": player_actor_id,
                    }),
                    None,
                );
                return CommandResult::rejected("chassis_in_transition", tick.0);
            }
        }
        match command {
            ControlCommand::ScenarioLoad { scenario, seed } => {
                // M0 cannot swap scenarios mid-run (no reload pipeline yet) and cannot
                // re-seed the engine (the RNG/clock are constructed from `config.seed` at
                // engine creation time and `scenario.reset` is the only way to reset them
                // — it uses the original seed). Both cases must be rejected, not faked.
                // (M0.2-F3.)
                if scenario != self.config.scenario_id {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({
                            "method": "scenario.load",
                            "reason": "scenario_swap_not_supported_in_m0",
                            "fix_hint": "M0 ships a single scenario per cf-app launch; relaunch with --scenario <id>. Hot-swap lands at M3."
                        }),
                        None,
                    );
                    CommandResult::rejected("scenario_swap_not_supported_in_m0", tick.0)
                } else if seed.is_some() && seed != Some(self.config.seed) {
                    let requested = seed.unwrap();
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({
                            "method": "scenario.load",
                            "reason": "seed_override_not_supported_in_m0",
                            "active_seed": self.config.seed,
                            "requested_seed": requested,
                            "fix_hint": "M0 cannot re-seed a live engine. Relaunch cf-app with --seed <n>, or use scenario.reset to reset to the original seed."
                        }),
                        None,
                    );
                    CommandResult::rejected("seed_override_not_supported_in_m0", tick.0)
                } else {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_accepted",
                        json!({"method": "scenario.load", "scenario": scenario, "seed": self.config.seed}),
                        None,
                    );
                    CommandResult::accepted(tick.0)
                }
            }
            ControlCommand::ScenarioReset => {
                // Reset the world state (RNG + actor world + pending intent) but do NOT
                // rewind the clock. Rewinding would violate `events.jsonl` monotonicity if
                // any events were recorded at higher ticks before the reset. The clock is a
                // monotonic timeline; `scenario.reset` is a content reload, not a time-warp.
                state.rng = Rng::from_seed(self.config.seed);
                state.tick_durations_us.clear();
                // Capture in-flight projectiles + the projectile-id counter from the old
                // sim state before we replace it. We emit a `combat.projectile_expired`
                // event for each discarded projectile so every `combat.projectile_spawned`
                // entry in the event log has a matched termination event, and we carry
                // the counter forward so post-reset projectile ids never alias pre-reset
                // ones — the event log is a single monotonic timeline that replay
                // analyzers correlate by `projectile_id`.
                let discarded_projectiles: Vec<(u64, ActorId, Vec2)> = state
                    .actor_state
                    .as_ref()
                    .map(|s| s.projectiles.iter().map(|p| (p.id, p.owner, p.position)).collect())
                    .unwrap_or_default();
                let next_projectile_id_carry = state.actor_state.as_ref().map(|s| s.next_projectile_id()).unwrap_or(0);
                // Preserve the pre-reset intent source so the next idle tick's
                // `input.intent_received` event still attributes to whoever was
                // driving (cfctl OR human at the keyboard) rather than spuriously
                // flipping to `cfctl` because the reset handler hardcoded a default.
                let preserved_source = state.pending_intent.source;
                if let Some(initial) = self.config.initial_actor_world.as_ref() {
                    let mut sim_state = ActorSimState::new(initial.world.clone());
                    sim_state.set_next_projectile_id(next_projectile_id_carry);
                    for (id, rifle) in build_rifles_for_world(&initial.world, self.config.tick_rate_hz) {
                        sim_state.ensure_rifle_for(id, rifle);
                    }
                    state.actor_state = Some(sim_state);
                    state.player_actor = initial.player;
                    state.pending_intent = ControlIntent::new(initial.player.unwrap_or(ActorId(0)), preserved_source);
                }
                state.intent_epoch = state.intent_epoch.wrapping_add(1);
                state.pending_dig = None;
                state.projectile_spawn_event_ids.clear();
                state.controls_captured_by = None;
                // M1.5: rewind breach world.
                if let (Some(world), Some(initial)) =
                    (state.breach_world.as_mut(), self.config.initial_breach_world.as_ref())
                {
                    *world = initial.world.clone();
                }
                // M1.5: rewind every reactive guard to its initial config so AI
                // memory + ammo + cooldowns reset cleanly.
                for guard in &self.config.initial_guards {
                    if let Some(g) = state.reactive_guards.get_mut(&guard.actor) {
                        *g = cf_ai::ReactiveGuard::new(guard.actor, guard.params);
                    }
                }
                // M1.5: rewind the mission state machine. Started-at-tick stays at
                // the live engine tick so the timer measures from reset.
                if let Some(mission) = state.mission.as_mut() {
                    mission.reset(tick.0);
                }
                // M2: rewind chunked terrain to the manifest's authored stamps.
                if let Some(initial_terrain) = self.config.initial_chunked_terrain.as_ref() {
                    state.chunked_terrain = Some(initial_terrain.clone());
                }
                // M2.5: rewind reactor world to manifest defaults (full hp,
                // not destroyed).
                if let Some(reactor_world) = state.reactor_world.as_mut() {
                    reactor_world.reset();
                }
                drop(state);
                for (projectile_id, owner, last_position) in &discarded_projectiles {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "combat",
                        "projectile_expired",
                        json!({
                            "id": projectile_id,
                            "owner": owner.0,
                            "last_position": [last_position.x, last_position.y],
                            "cause": "scenario_reset",
                        }),
                        None,
                    );
                }
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "scenario.reset"}),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::Pause => {
                state.clock.pause();
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "sim.pause"}),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::Resume => {
                state.clock.resume();
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "sim.resume"}),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::Step { ticks } => {
                if ticks == 0 {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "sim.step", "reason": "ticks_must_be_positive"}),
                        None,
                    );
                    return CommandResult::rejected("ticks_must_be_positive", tick.0);
                }
                state.clock.step(ticks);
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "sim.step", "ticks": ticks}),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::RunForTicks {
                ticks,
                write_run_bundle,
            } => {
                if ticks == 0 {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "sim.run_for_ticks", "reason": "ticks_must_be_positive"}),
                        None,
                    );
                    return CommandResult::rejected("ticks_must_be_positive", tick.0);
                }
                state.clock.step(ticks);
                if write_run_bundle {
                    state.pending_runbundle = true;
                }
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "sim.run_for_ticks", "ticks": ticks, "write_run_bundle": write_run_bundle}),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActPlayerMove { x, y, source } => {
                if !self.config.has_actor_world {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({
                            "method": "act.player.move",
                            "reason": "act_player_move_not_available_in_m0",
                            "x": x,
                            "y": y,
                            "fix_hint": "M0 has no player actor; load an M1 scenario such as m1_actor_range to enable act.player.*."
                        }),
                        None,
                    );
                    return CommandResult::rejected("act_player_move_not_available_in_m0", tick.0);
                }
                // Defense-in-depth: the JSON-RPC server rejects NaN/Inf at the wire
                // layer, but the engine dispatch is also reachable from cf-app's keyboard
                // bridge (and any future bridge / direct-dispatch caller). Reject here
                // too so a non-finite axis cannot leak into pending_intent and NaN-poison
                // the muzzle / projectile path.
                if !x.is_finite() || !y.is_finite() {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({
                            "method": "act.player.move",
                            "reason": "non_finite",
                            "x": x,
                            "y": y,
                        }),
                        None,
                    );
                    return CommandResult::rejected("non_finite", tick.0);
                }
                let player = state.player_actor;
                if let Some(player_id) = player {
                    state.pending_intent.actor = player_id;
                    state.pending_intent.source = source;
                    let clamped = x.clamp(-1.0, 1.0);
                    if (clamped - x).abs() > f32::EPSILON {
                        // M1 re-audit (2026-05-13): spec line for "Magnitude
                        // clamp on movement intent" — "And emits a debug
                        // log with the clamp; not a hard reject."
                        tracing::debug!(
                            target: "cf::control::move_clamp",
                            requested = x,
                            clamped = clamped,
                            actor = player_id.0,
                            "act.player.move magnitude clamped to [-1.0, 1.0]"
                        );
                    }
                    state.pending_intent.move_x = clamped;
                    // y is reserved for future ladder/climb input.
                    let _ = y;
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_accepted",
                        json!({"method": "act.player.move", "x": x, "y": y, "actor": player_id.0}),
                        None,
                    );
                    CommandResult::accepted(tick.0)
                } else {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({
                            "method": "act.player.move",
                            "reason": "no_player_actor",
                            "fix_hint": "scenario manifest must declare exactly one actor with controllable=true."
                        }),
                        None,
                    );
                    CommandResult::rejected("no_player_actor", tick.0)
                }
            }
            ControlCommand::ActPlayerJump { source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.jump");
                }
                let player = state.player_actor;
                if let Some(player_id) = player {
                    state.pending_intent.actor = player_id;
                    state.pending_intent.source = source;
                    state.pending_intent.jump = true;
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_accepted",
                        json!({"method": "act.player.jump", "actor": player_id.0}),
                        None,
                    );
                    CommandResult::accepted(tick.0)
                } else {
                    self.reject_actor_command(tick, sim_time_ms, state, "act.player.jump")
                }
            }
            ControlCommand::ActPlayerAim { x, y, source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.aim");
                }
                // Defense-in-depth (mirrors act.player.move): non-finite aim must NEVER
                // reach pending_intent. cf_actor::sim::step normalizes the aim, but
                // `Vec2::normalize_or_x` only short-circuits on a tiny vector — a NaN/Inf
                // input survives normalization and propagates into the muzzle origin,
                // projectile velocity, and recoil sign.
                if !x.is_finite() || !y.is_finite() {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({
                            "method": "act.player.aim",
                            "reason": "non_finite",
                            "x": x,
                            "y": y,
                        }),
                        None,
                    );
                    return CommandResult::rejected("non_finite", tick.0);
                }
                let player = state.player_actor;
                if let Some(player_id) = player {
                    state.pending_intent.actor = player_id;
                    state.pending_intent.source = source;
                    state.pending_intent.aim = Vec2::new(x, y);
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_accepted",
                        json!({"method": "act.player.aim", "actor": player_id.0, "x": x, "y": y}),
                        None,
                    );
                    CommandResult::accepted(tick.0)
                } else {
                    self.reject_actor_command(tick, sim_time_ms, state, "act.player.aim")
                }
            }
            ControlCommand::ActPlayerFire {
                pressed,
                ammo_kind,
                source,
            } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.fire");
                }
                let player = state.player_actor;
                if let Some(player_id) = player {
                    state.pending_intent.actor = player_id;
                    state.pending_intent.source = source;
                    // `pressed: true` raises the edge for one tick (cleared by
                    // clear_edges) and sets the sticky held flag; `pressed:
                    // false` releases the held flag. Semi-mode rifles latch
                    // after one shot; FullAuto rifles auto-repeat at cadence
                    // while held.
                    if pressed {
                        state.pending_intent.fire = true;
                        state.pending_intent.fire_held = true;
                    } else {
                        state.pending_intent.fire_held = false;
                    }
                    // **M14C** § propagate the per-shot ammo-kind override
                    // from cfctl `act.player.fire { ammo_kind: ... }` into
                    // the actor's pending intent so cf-actor::sim picks the
                    // correct `RoundKind` when the magazine pops next. The
                    // edge clears via `ControlIntent::clear_edges` so it
                    // never bleeds into a follow-up tick. Without this
                    // propagation the cfctl drive of
                    // `m14c_heat_vs_era.ron` /
                    // `m14c_apfsds_vs_heavy.ron` emits zero
                    // `armor.heat_jet_traversed` /
                    // `armor.apfsds_long_rod_through` events at runtime
                    // (the M14C scrutiny gap).
                    if pressed {
                        state.pending_intent.ammo_kind = ammo_kind;
                    }
                    drop(state);
                    let ammo_kind_str: Option<&'static str> = ammo_kind.map(cf_equipment::RoundKind::as_str);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_accepted",
                        json!({
                            "method": "act.player.fire",
                            "actor": player_id.0,
                            "pressed": pressed,
                            "ammo_kind": ammo_kind_str,
                        }),
                        None,
                    );
                    CommandResult::accepted(tick.0)
                } else {
                    self.reject_actor_command(tick, sim_time_ms, state, "act.player.fire")
                }
            }
            ControlCommand::ActPlayerReload { source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.reload");
                }
                let player = state.player_actor;
                if let Some(player_id) = player {
                    state.pending_intent.actor = player_id;
                    state.pending_intent.source = source;
                    state.pending_intent.reload = true;
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_accepted",
                        json!({"method": "act.player.reload", "actor": player_id.0}),
                        None,
                    );
                    CommandResult::accepted(tick.0)
                } else {
                    self.reject_actor_command(tick, sim_time_ms, state, "act.player.reload")
                }
            }
            ControlCommand::ActPlayerSelectItem { slot, source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.select_item");
                }
                let player = state.player_actor;
                if let Some(player_id) = player {
                    state.pending_intent.actor = player_id;
                    state.pending_intent.source = source;
                    state.pending_intent.selected_item = Some(ItemSlot(slot));
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_accepted",
                        json!({"method": "act.player.select_item", "actor": player_id.0, "slot": slot}),
                        None,
                    );
                    CommandResult::accepted(tick.0)
                } else {
                    self.reject_actor_command(tick, sim_time_ms, state, "act.player.select_item")
                }
            }
            ControlCommand::ActPlayerReset { source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.reset");
                }
                let player = state.player_actor;
                if let Some(player_id) = player {
                    state.pending_intent.actor = player_id;
                    state.pending_intent.source = source;
                    state.pending_intent.reset = true;
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_accepted",
                        json!({"method": "act.player.reset", "actor": player_id.0}),
                        None,
                    );
                    CommandResult::accepted(tick.0)
                } else {
                    self.reject_actor_command(tick, sim_time_ms, state, "act.player.reset")
                }
            }
            ControlCommand::ActPlayerSharpAim { active, source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.sharp_aim");
                }
                let player = state.player_actor;
                if let Some(player_id) = player {
                    state.pending_intent.actor = player_id;
                    state.pending_intent.source = source;
                    state.pending_intent.sharp_aim = active;
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_accepted",
                        json!({"method": "act.player.sharp_aim", "actor": player_id.0, "active": active}),
                        None,
                    );
                    CommandResult::accepted(tick.0)
                } else {
                    self.reject_actor_command(tick, sim_time_ms, state, "act.player.sharp_aim")
                }
            }
            ControlCommand::ActPlayerAbort { source } => {
                // **M1.5 G9**: player-initiated forfeit. Marks the mission
                // (if any) as Aborted and emits mission.mission_resolved.
                // Idempotent: a second abort while the mission is already
                // terminal is rejected with `mission_already_terminal`.
                let _ = source;
                if let Some(mission) = state.mission.as_mut() {
                    if mission.result.is_terminal() {
                        drop(state);
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "control",
                            "command_rejected",
                            json!({
                                "method": "act.player.abort",
                                "reason": "mission_already_terminal",
                            }),
                            None,
                        );
                        return CommandResult::rejected("mission_already_terminal", tick.0);
                    }
                    mission.result = cf_mission::MissionResult::Aborted;
                    mission.last_event_tick = tick.0;
                    mission.last_event_label = "mission_resolved".to_string();
                    mission.last_transition_tick = tick.0;
                    // M2 re-audit (2026-05-13): lifecycle → Resolved on abort.
                    mission.lifecycle = cf_mission::MissionLifecycle::Resolved;
                    // M2 re-audit (2026-05-13): route through the typed enum's
                    // as_str() — never a raw string literal. Per spec pitfall:
                    // "String-literal loss reasons: DR-002 stable-vocabulary
                    // contract. Use the typed enum's `as_str()`."
                    mission.loss_reason_label = Some(cf_mission::LossReason::Aborted.as_str().to_string());
                    // **M14 audit pass 2 (GAP-M4-02)**: latch the run-aborted
                    // flag so record_run_finished emits outcome="abort".
                    state.run_aborted = true;
                    drop(state);
                    let accepted_id = self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_accepted",
                        json!({"method": "act.player.abort"}),
                        None,
                    );
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "mission",
                        "mission_resolved",
                        json!({
                            "result": "aborted",
                            // M2 audit pass 7 (2026-05-13): route through
                            // the typed enum's as_str() (DR-002 stable
                            // vocabulary contract) — never a raw literal.
                            "loss_reason": cf_mission::LossReason::Aborted.as_str(),
                            "cause": "player_aborted",
                        }),
                        Some(accepted_id),
                    );
                    return CommandResult::accepted(tick.0);
                }
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({
                        "method": "act.player.abort",
                        "reason": "no_mission_in_scenario",
                    }),
                    None,
                );
                CommandResult::rejected("no_mission_in_scenario", tick.0)
            }
            ControlCommand::ActMissionPause { source } => {
                // **M1.5**: tutorial-modal pause. Suspends mission objective
                // progress + timer; emits mission.objective_paused. No-ops
                // when no mission, already paused, or mission is terminal.
                let _ = source;
                let Some(mission) = state.mission.as_mut() else {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.mission.pause", "reason": "no_mission_in_scenario"}),
                        None,
                    );
                    return CommandResult::rejected("no_mission_in_scenario", tick.0);
                };
                if mission.result.is_terminal() {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.mission.pause", "reason": "mission_already_terminal"}),
                        None,
                    );
                    return CommandResult::rejected("mission_already_terminal", tick.0);
                }
                if mission.paused {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.mission.pause", "reason": "already_paused"}),
                        None,
                    );
                    return CommandResult::rejected("already_paused", tick.0);
                }
                let active = mission.pause(tick.0);
                drop(state);
                let accepted_id = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "act.mission.pause"}),
                    None,
                );
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "mission",
                    "objective_paused",
                    json!({"objective": active}),
                    Some(accepted_id),
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActMissionResume { source } => {
                // **M1.5**: lift the pause. No-op if not paused.
                let _ = source;
                let Some(mission) = state.mission.as_mut() else {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.mission.resume", "reason": "no_mission_in_scenario"}),
                        None,
                    );
                    return CommandResult::rejected("no_mission_in_scenario", tick.0);
                };
                if !mission.paused {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.mission.resume", "reason": "not_paused"}),
                        None,
                    );
                    return CommandResult::rejected("not_paused", tick.0);
                }
                let active = mission.resume(tick.0);
                drop(state);
                let accepted_id = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "act.mission.resume"}),
                    None,
                );
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "mission",
                    "objective_resumed",
                    json!({"objective": active}),
                    Some(accepted_id),
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActInputCaptureControls {
                captured,
                capturer,
                source,
            } => {
                let _ = source;
                let prev = state.controls_captured_by.clone();
                let new = if captured {
                    Some(capturer.clone().unwrap_or_else(|| "unknown".to_string()))
                } else {
                    None
                };
                state.controls_captured_by = new.clone();
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({
                        "method": "act.input.capture_controls",
                        "captured": captured,
                        "capturer": capturer,
                    }),
                    None,
                );
                // Emit ux.controls_captured / ux.controls_released on transition.
                match (prev.as_deref(), new.as_deref()) {
                    (None, Some(c)) => {
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "ux",
                            "controls_captured",
                            json!({"capturer": c}),
                            None,
                        );
                    }
                    (Some(_), None) => {
                        self.recorder
                            .record(tick, sim_time_ms, "ux", "controls_released", json!({}), None);
                    }
                    _ => {}
                }
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActToggleMaterialOverlay { mode, source } => {
                let _ = source;
                let prev = state.material_overlay_mode.clone();
                let next = match mode.as_deref() {
                    Some("off" | "integrity" | "pathability" | "mobility" | "hazard" | "build_repair") => {
                        mode.unwrap_or_default()
                    }
                    Some(other) => {
                        drop(state);
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "control",
                            "command_rejected",
                            json!({
                                "method": "act.player.toggle_material_overlay",
                                "reason": "unknown_overlay_mode",
                                "mode": other,
                            }),
                            None,
                        );
                        return CommandResult::rejected("unknown_overlay_mode", tick.0);
                    }
                    None => match prev.as_str() {
                        "off" => "integrity".to_string(),
                        "integrity" => "pathability".to_string(),
                        "pathability" => "mobility".to_string(),
                        "mobility" => "hazard".to_string(),
                        "hazard" => "build_repair".to_string(),
                        _ => "off".to_string(),
                    },
                };
                state.material_overlay_mode = next.clone();
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({
                        "method": "act.player.toggle_material_overlay",
                        "mode": next.clone(),
                    }),
                    None,
                );
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "ux",
                    "overlay_mode_changed",
                    json!({"from": prev, "to": next}),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActPlayerDig { target, source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.dig");
                }
                if state.breach_world.is_none() && state.chunked_terrain.is_none() {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({
                            "method": "act.player.dig",
                            "reason": "no_terrain_world",
                            "fix_hint": "scenario manifest must declare either breaches[] (M1.5) or terrain (M2 chunked)."
                        }),
                        None,
                    );
                    return CommandResult::rejected("no_terrain_world", tick.0);
                }
                if state.player_actor.is_none() {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.dig");
                }
                // M1 audit pass 5 (2026-05-13): spec literal — "during
                // knockdown ALL input is rejected: move, aim, fire, reload,
                // jump, dig, select_item are no-ops". The sim-side
                // accepts_input gate covers move/aim/jump/fire/reload/select_item
                // but dig is routed through pending_dig at the dispatch
                // boundary. Add the knockdown gate here so dig is a no-op
                // with a labeled rejection.
                let player_knocked_down = state
                    .player_actor
                    .and_then(|pid| state.actor_state.as_ref().map(|w| (pid, w)))
                    .and_then(|(pid, w)| w.world.actors.get(&pid))
                    .map(|a| a.knockdown_ticks_remaining > 0)
                    .unwrap_or(false);
                if player_knocked_down {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({
                            "method": "act.player.dig",
                            "reason": "knockdown",
                        }),
                        None,
                    );
                    return CommandResult::rejected("knockdown", tick.0);
                }
                state.pending_dig = Some(PendingDig {
                    target: target.clone(),
                    source,
                });
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "act.player.dig", "target": target}),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActPlayerAnchor { x, y, tool_id, source } => {
                // M3 re-open (2026-05-13): MAT-T-06 — sample the chunked
                // terrain material at (x, y) and emit
                // `terrain.anchor_material_result`. Refuses when the chunked
                // terrain is not loaded (no surface to anchor against) and
                // when the sampled material's `anchorable` affordance is
                // false. Spec ref: `specs/active/M3.md` § Re-opened gaps.
                let actor_id = state.player_actor.map(|a| a.0);
                let tool_label = tool_id.clone().unwrap_or_else(|| "anchor_tool".to_string());
                let source_label = match source {
                    IntentSource::Human => "human",
                    IntentSource::Cfctl => "cfctl",
                    IntentSource::Ai => "ai",
                    IntentSource::Replay => "replay",
                };
                let terrain_ref = state.chunked_terrain.as_ref();
                if terrain_ref.is_none() {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({
                            "method": "act.player.anchor",
                            "reason": "no_chunked_terrain",
                            "fix_hint": "scenario manifest must declare a chunked terrain (M2+)."
                        }),
                        None,
                    );
                    return CommandResult::rejected("no_chunked_terrain", tick.0);
                }
                let terrain = terrain_ref.expect("chunked terrain is_some");
                // Sample the material at the target world point. Out-of-bounds
                // reads return the chunk's default material (`air`), which is
                // non-anchorable.
                let material_id = terrain.material_at_world(x as f32, y as f32);
                let affordance = cf_terrain::material_affordance(material_id);
                let mat_name = affordance.map(|a| a.name).unwrap_or("unknown");
                let anchorable = affordance.map(|a| a.anchorable).unwrap_or(false);
                drop(state);

                // Emit a control.command_accepted parent so the anchor result
                // can chain back through the full event ladder.
                let action_id = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({
                        "method": "act.player.anchor",
                        "tool_id": tool_label,
                        "source": source_label,
                        "point": [x, y],
                    }),
                    None,
                );

                // M3 audit pass 5 (2026-05-13): refuse reason is the stable
                // spec vocabulary `material_not_anchorable`; the specific
                // material is exposed on the `material` payload field.
                let (result, reason) = if anchorable {
                    ("accepted", None)
                } else {
                    ("refused", Some("material_not_anchorable".to_string()))
                };
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "terrain",
                    "anchor_material_result",
                    json!({
                        "actor_id": actor_id,
                        "tool_id": tool_label,
                        "material_id": material_id,
                        "material": mat_name,
                        "point": [x, y],
                        "result": result,
                        "reason": reason,
                    }),
                    Some(action_id),
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActInputFocus { direction, source } => {
                let _ = source;
                let n = HUD_FOCUSABLE_NODES.len();
                let prev_idx = state.hud_focus_index;
                let new_idx: Option<usize> = match &direction {
                    crate::server::FocusDirection::Next => Some(match prev_idx {
                        Some(i) => (i + 1) % n,
                        None => 0,
                    }),
                    crate::server::FocusDirection::Prev => Some(match prev_idx {
                        Some(i) => (i + n - 1) % n,
                        None => n - 1,
                    }),
                    crate::server::FocusDirection::Set(node) => {
                        match HUD_FOCUSABLE_NODES.iter().position(|x| *x == node) {
                            Some(i) => Some(i),
                            None => {
                                drop(state);
                                self.recorder.record(
                                    tick,
                                    sim_time_ms,
                                    "control",
                                    "command_rejected",
                                    json!({
                                        "method": "act.input.focus",
                                        "reason": "focus_unknown_node",
                                        "node": node,
                                    }),
                                    None,
                                );
                                return CommandResult::rejected("focus_unknown_node", tick.0);
                            }
                        }
                    }
                    crate::server::FocusDirection::Clear => None,
                };
                state.hud_focus_index = new_idx;
                state.hud_focus_cycle = state.hud_focus_cycle.saturating_add(1);
                let new_node: Option<String> = new_idx.map(|i| HUD_FOCUSABLE_NODES[i].to_string());
                let direction_str = match &direction {
                    crate::server::FocusDirection::Next => "next",
                    crate::server::FocusDirection::Prev => "prev",
                    crate::server::FocusDirection::Set(_) => "set",
                    crate::server::FocusDirection::Clear => "clear",
                };
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({
                        "method": "act.input.focus",
                        "direction": direction_str,
                        "node": new_node,
                    }),
                    None,
                );
                // **M11 § DR-012 closure**: emit `ux.focus_moved` paired
                // with the control event so the replay viewer can render
                // the focus traversal as a first-class HUD event.
                let from_node = prev_idx.map(|i| HUD_FOCUSABLE_NODES[i].to_string()).unwrap_or_default();
                let to_node = new_node.clone().unwrap_or_default();
                let _ = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "ux",
                    "focus_moved",
                    json!({
                        "from": from_node,
                        "to": to_node,
                        "direction": direction_str,
                    }),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActInputMouseClick { x, y, source } => {
                let _ = source;
                drop(state);
                if !x.is_finite() || !y.is_finite() {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.input.mouse_click", "reason": "non_finite"}),
                        None,
                    );
                    return CommandResult::rejected("non_finite", tick.0);
                }
                let target = resolve_hud_node_at(x, y).unwrap_or_default();
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "act.input.mouse_click", "x": x, "y": y, "target_node_id": target}),
                    None,
                );
                let _ = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "ux",
                    "mouse_clicked",
                    json!({"x": x, "y": y, "target_node_id": target}),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActInputMouseMove { x, y, source } => {
                let _ = source;
                drop(state);
                if !x.is_finite() || !y.is_finite() {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.input.mouse_move", "reason": "non_finite"}),
                        None,
                    );
                    return CommandResult::rejected("non_finite", tick.0);
                }
                let hover = resolve_hud_node_at(x, y).unwrap_or_default();
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "act.input.mouse_move", "x": x, "y": y, "hover_node_id": hover}),
                    None,
                );
                let _ = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "ux",
                    "mouse_moved",
                    json!({"x": x, "y": y, "hover_node_id": hover}),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            // **M11 audit pass (GAP-M11-01 HIGH fix)**: keyed action press
            // dispatch for the BP3 self-play floor + pause-overlay cycling.
            // Translates a UI-bound action into the corresponding settings
            // mutation or overlay toggle. Emits `control.command_accepted`
            // + the relevant follow-on event (`ux.game_speed_assist_changed`,
            // `ux.debug_overlay_toggled`, etc.).
            ControlCommand::ActInputKeyPress { action, source } => {
                let _ = source;
                let action_str = action.clone();
                let follow_on_event: Option<(String, String, serde_json::Value)>;
                match action.as_str() {
                    "pause" => {
                        // Toggle game_speed_assist between Off and FullPause.
                        let next = match state.settings.game_speed_assist {
                            crate::settings::GameSpeedAssist::FullPause => crate::settings::GameSpeedAssist::Off,
                            _ => crate::settings::GameSpeedAssist::FullPause,
                        };
                        state.settings.game_speed_assist = next;
                        follow_on_event = Some((
                            "ux".to_string(),
                            "game_speed_assist_changed".to_string(),
                            json!({"to": next.as_str(), "via": "act.input.key_press"}),
                        ));
                    }
                    "game_speed_cycle" => {
                        // Cycle Off → Slowdown75 → Slowdown25 → FullPause → Off.
                        let next = match state.settings.game_speed_assist {
                            crate::settings::GameSpeedAssist::Off => crate::settings::GameSpeedAssist::Slowdown75,
                            crate::settings::GameSpeedAssist::Slowdown75 => {
                                crate::settings::GameSpeedAssist::Slowdown25
                            }
                            crate::settings::GameSpeedAssist::Slowdown25 => crate::settings::GameSpeedAssist::FullPause,
                            crate::settings::GameSpeedAssist::FullPause => crate::settings::GameSpeedAssist::Off,
                        };
                        state.settings.game_speed_assist = next;
                        follow_on_event = Some((
                            "ux".to_string(),
                            "game_speed_assist_changed".to_string(),
                            json!({"to": next.as_str(), "via": "act.input.key_press"}),
                        ));
                    }
                    "accessibility_overlay"
                    | "tactical_overlay"
                    | "photo_mode"
                    | "debug_overlay"
                    | "mini_map_toggle"
                    | "compass_toggle"
                    | "damage_direction_toggle"
                    | "captions_toggle" => {
                        // Map each toggle action to its corresponding
                        // settings field; emit the matching ux event.
                        let event_type = match action.as_str() {
                            "accessibility_overlay" => "accessibility_overlay_toggled",
                            "tactical_overlay" => "tactical_overlay_toggled",
                            "photo_mode" => "photo_mode_toggled",
                            "debug_overlay" => "debug_overlay_toggled",
                            "mini_map_toggle" => "mini_map_toggled",
                            "compass_toggle" => "compass_toggled",
                            "damage_direction_toggle" => "damage_direction_toggled",
                            "captions_toggle" => "captions_toggled",
                            _ => "overlay_toggled",
                        };
                        match action.as_str() {
                            "mini_map_toggle" => state.settings.mini_map_enabled = !state.settings.mini_map_enabled,
                            "compass_toggle" => state.settings.compass_enabled = !state.settings.compass_enabled,
                            "damage_direction_toggle" => {
                                state.settings.damage_direction_enabled = !state.settings.damage_direction_enabled
                            }
                            "captions_toggle" => state.settings.captions = !state.settings.captions,
                            _ => {}
                        }
                        follow_on_event = Some((
                            "ux".to_string(),
                            event_type.to_string(),
                            json!({"via": "act.input.key_press"}),
                        ));
                    }
                    _ => {
                        // Should never reach: server-side whitelist gates this.
                        drop(state);
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "control",
                            "command_rejected",
                            json!({"method": "act.input.key_press", "reason": "unknown_key_action", "action": action_str}),
                            None,
                        );
                        return CommandResult::rejected("unknown_key_action", tick.0);
                    }
                }
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "act.input.key_press", "action": action_str}),
                    None,
                );
                if let Some((cat, etype, payload)) = follow_on_event {
                    let _ = self.recorder.record(tick, sim_time_ms, &cat, &etype, payload, None);
                }
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActPlayerCrouch { active, source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.crouch");
                }
                let player_id = state.player_actor.expect("player actor present");
                let _ = source;
                if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        actor.crouch_active = active;
                    }
                }
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "act.player.crouch", "actor": player_id.0, "active": active}),
                    None,
                );
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "actor",
                    "animation_event",
                    json!({
                        "actor": player_id.0,
                        "kind": if active { "crouch_started" } else { "crouch_ended" },
                    }),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActPlayerClimb { active, source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.climb");
                }
                let player_id = state.player_actor.expect("player actor present");
                let _ = source;
                if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        actor.climb_active = active;
                    }
                }
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "act.player.climb", "actor": player_id.0, "active": active}),
                    None,
                );
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "actor",
                    "animation_event",
                    json!({
                        "actor": player_id.0,
                        "kind": if active { "climb_started" } else { "climb_ended" },
                    }),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActPlayerJet { active, source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.jet");
                }
                let player_id = state.player_actor.expect("player actor present");
                let _ = source;
                let mut jet_ok = false;
                let mut reject_reason: Option<String> = None;
                if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        if active {
                            let module_ok = actor
                                .chassis
                                .as_ref()
                                .and_then(|c| c.module_by_kind(cf_chassis::ModuleKind::Jet))
                                .map(|m| {
                                    matches!(
                                        m.state,
                                        cf_chassis::ModuleStateKind::Nominal | cf_chassis::ModuleStateKind::Degraded
                                    )
                                })
                                .unwrap_or(true); // no chassis = treat as no jet, but allow toggle
                            if module_ok {
                                actor.jet_active = true;
                                jet_ok = true;
                            } else {
                                reject_reason = Some("jet_module_unavailable".to_string());
                            }
                        } else {
                            actor.jet_active = false;
                            jet_ok = true;
                        }
                    }
                }
                drop(state);
                if let Some(reason) = reject_reason {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.player.jet", "reason": reason.clone(), "actor": player_id.0}),
                        None,
                    );
                    return CommandResult::rejected(reason, tick.0);
                }
                let _ = jet_ok;
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "act.player.jet", "actor": player_id.0, "active": active}),
                    None,
                );
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "actor",
                    "animation_event",
                    json!({
                        "actor": player_id.0,
                        "kind": if active { "jet_thrust_started" } else { "jet_thrust_ended" },
                    }),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActPlayerEject { source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.eject");
                }
                let player_id = state.player_actor.expect("player actor present");
                let _ = source;
                let mut emit: Option<(String, String, u32, bool)> = None;
                let mut reject_reason: Option<String> = None;
                if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        if let Some(chassis) = actor.chassis.as_mut() {
                            if let Some(accepted) = chassis.attempt_eject(tick.0) {
                                emit = Some((
                                    chassis.spec_id.clone(),
                                    chassis.pilot_state.as_str().to_string(),
                                    accepted.ticks_total,
                                    accepted.tutorial_extract,
                                ));
                            } else {
                                reject_reason = Some("pilot_not_in_chassis".to_string());
                            }
                        } else {
                            reject_reason = Some("no_chassis_attached".to_string());
                        }
                    }
                }
                drop(state);
                if let Some(reason) = reject_reason {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.player.eject", "reason": reason.clone(), "actor": player_id.0}),
                        None,
                    );
                    return CommandResult::rejected(reason, tick.0);
                }
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "act.player.eject", "actor": player_id.0}),
                    None,
                );
                if let Some((spec_id, pilot_state, ticks_total, tutorial_extract)) = emit {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "chassis",
                        "pilot_ejected",
                        json!({
                            "actor": player_id.0,
                            "spec_id": spec_id,
                            "pilot_state": pilot_state,
                            "eject_ticks_total": ticks_total,
                            "tutorial_extract": tutorial_extract,
                        }),
                        None,
                    );
                    if tutorial_extract {
                        // Tutorial extract jumps straight to extracted.
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "chassis",
                            "pilot_extracted",
                            json!({"actor": player_id.0, "via": "tutorial_safety"}),
                            None,
                        );
                    }
                }
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActPlayerQuickActionSlot { slot, source } => {
                let _ = source;
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.quick_action_slot");
                }
                let player_id = state.player_actor.expect("player actor present");
                let slot_idx = slot.saturating_sub(1);
                let outcome = if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        actor.quick_action_bar.try_invoke_slot(slot_idx)
                    } else {
                        cf_actor::quick_action::InvokeOutcome::Rejected("actor_missing")
                    }
                } else {
                    cf_actor::quick_action::InvokeOutcome::Rejected("no_actor_world")
                };
                drop(state);
                match outcome {
                    cf_actor::quick_action::InvokeOutcome::Accepted => {
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "quick_action",
                            "slot_invoked",
                            json!({"actor": player_id.0, "slot": slot}),
                            None,
                        );
                        CommandResult::accepted(tick.0)
                    }
                    cf_actor::quick_action::InvokeOutcome::Rejected(reason) => {
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "control",
                            "command_rejected",
                            json!({"method": "act.player.quick_action_slot", "reason": reason, "slot": slot}),
                            None,
                        );
                        CommandResult::rejected(reason.to_string(), tick.0)
                    }
                }
            }
            ControlCommand::ActPlayerQuickActionToggle { source } => {
                let _ = source;
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.quick_action_toggle");
                }
                let player_id = state.player_actor.expect("player actor present");
                let outcome = if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        actor.quick_action_bar.try_invoke_toggle()
                    } else {
                        cf_actor::quick_action::InvokeOutcome::Rejected("actor_missing")
                    }
                } else {
                    cf_actor::quick_action::InvokeOutcome::Rejected("no_actor_world")
                };
                drop(state);
                match outcome {
                    cf_actor::quick_action::InvokeOutcome::Accepted => {
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "quick_action",
                            "slot_invoked_via_toggle",
                            json!({"actor": player_id.0}),
                            None,
                        );
                        CommandResult::accepted(tick.0)
                    }
                    cf_actor::quick_action::InvokeOutcome::Rejected(reason) => {
                        CommandResult::rejected(reason.to_string(), tick.0)
                    }
                }
            }
            ControlCommand::ActPlayerQuickActionRadial { active, source } => {
                let _ = source;
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.quick_action_radial");
                }
                let player_id = state.player_actor.expect("player actor present");
                if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        if active {
                            actor.quick_action_bar.open_radial(tick.0, false);
                            self.recorder.record(
                                tick,
                                sim_time_ms,
                                "quick_action",
                                "radial_opened",
                                json!({"actor": player_id.0, "tick": tick.0}),
                                None,
                            );
                        } else {
                            let invoked = actor.quick_action_bar.close_radial(None);
                            self.recorder.record(
                                tick,
                                sim_time_ms,
                                "quick_action",
                                "radial_closed",
                                json!({"actor": player_id.0, "cancelled": invoked.is_none()}),
                                None,
                            );
                        }
                    }
                }
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActPlayerQuickActionSlice { slice, source } => {
                let _ = source;
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.quick_action_slice");
                }
                let player_id = state.player_actor.expect("player actor present");
                let outcome = if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        let slice_idx = slice.saturating_sub(1);
                        let invoked = actor.quick_action_bar.close_radial(Some(slice_idx));
                        if invoked.is_some() {
                            cf_actor::quick_action::InvokeOutcome::Accepted
                        } else {
                            cf_actor::quick_action::InvokeOutcome::Rejected("slice_rejected")
                        }
                    } else {
                        cf_actor::quick_action::InvokeOutcome::Rejected("actor_missing")
                    }
                } else {
                    cf_actor::quick_action::InvokeOutcome::Rejected("no_actor_world")
                };
                drop(state);
                match outcome {
                    cf_actor::quick_action::InvokeOutcome::Accepted => {
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "quick_action",
                            "slice_selected",
                            json!({"actor": player_id.0, "slice": slice}),
                            None,
                        );
                        CommandResult::accepted(tick.0)
                    }
                    cf_actor::quick_action::InvokeOutcome::Rejected(reason) => {
                        CommandResult::rejected(reason.to_string(), tick.0)
                    }
                }
            }
            ControlCommand::ActPlayerWeaponCycle { direction, source } => {
                let _ = source;
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.weapon_cycle");
                }
                let player_id = state.player_actor.expect("player actor present");
                let cycled = if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        let slot = actor.last_used_quick_slot;
                        actor.quick_action_bar.cycle_within_slot(slot, direction as i32)
                    } else {
                        None
                    }
                } else {
                    None
                };
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "quick_action",
                    "weapon_cycle",
                    json!({"actor": player_id.0, "direction": direction, "current": cycled}),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActChassisRepair {
                zone,
                module_id,
                reason,
                source,
            } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.chassis.repair");
                }
                let player_id = state.player_actor.expect("player actor present");
                let _ = source;
                let mut zone_result: Option<cf_chassis::RepairOutcome> = None;
                let mut module_result: Option<cf_chassis::ModuleTransition> = None;
                let mut reject_reason: Option<String> = None;
                // **M5**: `act.chassis.repair` is idempotent — a repair on an already-Nominal
                // module/zone returns None (no transition) but the COMMAND succeeds. Only an
                // unknown zone string or an unknown module id rejects; calling repair on a
                // healthy chassis is a no-op accept.
                if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        if let Some(chassis) = actor.chassis.as_mut() {
                            if let Some(zone_str) = &zone {
                                if let Some(zone_kind) = parse_body_zone(zone_str) {
                                    zone_result = chassis.repair_zone(zone_kind, &reason);
                                } else {
                                    reject_reason = Some(format!("chassis_repair_unknown_zone:{zone_str}"));
                                }
                            }
                            if reject_reason.is_none() {
                                if let Some(mid) = &module_id {
                                    // Validate the module id exists on the chassis BEFORE repairing.
                                    // If the module is already Nominal, repair_module returns None
                                    // but the command should still accept (idempotent no-op).
                                    if chassis.module(mid).is_none() {
                                        reject_reason = Some(format!("chassis_repair_unknown_module:{mid}"));
                                    } else {
                                        module_result = chassis.repair_module(mid, &reason);
                                    }
                                }
                            }
                        } else {
                            reject_reason = Some("no_chassis_attached".to_string());
                        }
                    }
                }
                drop(state);
                if let Some(r) = reject_reason {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.chassis.repair", "reason": r.clone(), "actor": player_id.0}),
                        None,
                    );
                    return CommandResult::rejected(r, tick.0);
                }
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({
                        "method": "act.chassis.repair",
                        "actor": player_id.0,
                        "zone": zone,
                        "module_id": module_id,
                        "reason": reason,
                    }),
                    None,
                );
                if let Some(out) = zone_result {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "chassis",
                        "repaired",
                        json!({
                            "actor": player_id.0,
                            "zone": out.zone.as_str(),
                            "was_destroyed": out.was_destroyed,
                            "modules_restored": out.modules_restored,
                            "prev_stage": out.prev_stage.as_str(),
                            "new_stage": out.new_stage.as_str(),
                            "reason": out.reason,
                        }),
                        None,
                    );
                }
                if let Some(t) = module_result {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "chassis",
                        "module_state_changed",
                        json!({
                            "actor": player_id.0,
                            "module_id": t.id,
                            "state": t.state.as_str(),
                            "reason": t.reason,
                        }),
                        None,
                    );
                }
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActChassisSalvage { reason, source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.chassis.salvage");
                }
                let player_id = state.player_actor.expect("player actor present");
                let _ = source;
                let mut salvage_out: Option<cf_chassis::SalvageOutcome> = None;
                let mut reject_reason: Option<String> = None;
                if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        if let Some(chassis) = actor.chassis.as_mut() {
                            salvage_out = chassis.salvage(&reason);
                            if salvage_out.is_none() {
                                reject_reason = Some("chassis_not_wreck_or_disabled".to_string());
                            }
                        } else {
                            reject_reason = Some("no_chassis_attached".to_string());
                        }
                    }
                }
                drop(state);
                if let Some(r) = reject_reason {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.chassis.salvage", "reason": r.clone(), "actor": player_id.0}),
                        None,
                    );
                    return CommandResult::rejected(r, tick.0);
                }
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "act.chassis.salvage", "actor": player_id.0, "reason": reason}),
                    None,
                );
                if let Some(out) = salvage_out {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "chassis",
                        "salvaged",
                        json!({
                            "actor": player_id.0,
                            "salvaged_module_ids": out.salvaged_module_ids,
                            "reason": out.reason,
                        }),
                        None,
                    );
                }
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActChassisClearJam { source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.chassis.clear_jam");
                }
                let player_id = state.player_actor.expect("player actor present");
                let _ = source;
                let mut cleared = false;
                if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        if let Some(chassis) = actor.chassis.as_mut() {
                            cleared = chassis.clear_jam();
                        }
                    }
                }
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "act.chassis.clear_jam", "actor": player_id.0, "cleared": cleared}),
                    None,
                );
                if cleared {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "chassis",
                        "weapon_cleared",
                        json!({"actor": player_id.0, "via": "manual"}),
                        None,
                    );
                }
                CommandResult::accepted(tick.0)
            }
            // **M13** § "Brain hopping / multi-actor control".
            ControlCommand::ActPlayerBrainHop {
                target_actor_id,
                source,
            } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.brain_hop");
                }
                let _ = source;
                let prior = state.player_actor;
                let target = cf_actor::ActorId(target_actor_id);
                let mut reject: Option<&'static str> = None;
                let mut prev_team: Option<String> = None;
                let mut next_team: Option<String> = None;
                if let Some(sim) = state.actor_state.as_mut() {
                    if !sim.world.actors.contains_key(&target) {
                        reject = Some("brain_hop_unknown_target");
                    } else if prior == Some(target) {
                        reject = Some("brain_hop_same_actor");
                    } else if prior.is_none() {
                        // **M14 audit pass 4 (Finding 9)**: surface the
                        // "no prior actor to hop from" case with its own
                        // reason instead of misreporting as not_friendly
                        // (which the team-match branch would otherwise
                        // emit when prior_team is None vs target_team
                        // Some).
                        reject = Some("brain_hop_no_prior_actor");
                    } else {
                        // Teams must match (transfer to friendly only).
                        let prior_team = prior.and_then(|p| sim.world.actors.get(&p).map(|a| a.team.clone()));
                        let target_team = sim.world.actors.get(&target).map(|a| a.team.clone());
                        if prior_team != target_team {
                            reject = Some("brain_hop_not_friendly");
                        } else {
                            prev_team = prior_team;
                            next_team = target_team;
                        }
                    }
                    if reject.is_none() {
                        // Clear brain flag on prior + set on target.
                        if let Some(p) = prior {
                            if let Some(a) = sim.world.actors.get_mut(&p) {
                                a.clear_brain();
                                a.controllable = false;
                            }
                        }
                        if let Some(a) = sim.world.actors.get_mut(&target) {
                            a.mark_brain(tick.0);
                            a.controllable = true;
                        }
                        sim.world.player = Some(target);
                    }
                }
                if reject.is_none() {
                    state.player_actor = Some(target);
                }
                let prior_id = prior.map(|p| p.0).unwrap_or(0);
                let _ = (prev_team, next_team);
                drop(state);
                if let Some(reason) = reject {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.player.brain_hop", "reason": reason, "target": target_actor_id}),
                        None,
                    );
                    return CommandResult::rejected(reason, tick.0);
                }
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "actor",
                    "brain_hop_initiated",
                    json!({"from_actor": prior_id, "target_actor": target_actor_id}),
                    None,
                );
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "actor",
                    "brain_hop_completed",
                    json!({"from_actor": prior_id, "target_actor": target_actor_id}),
                    None,
                );
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "ux",
                    "actor_switched",
                    json!({"from_actor": prior_id, "target_actor": target_actor_id, "transition_ms": 200}),
                    None,
                );
                // **M7B** § "Cortex-Command-style commander hopping" —
                // emit `squad.brain_hop` so consumers can join the hop
                // with the squad's preserved doctrine + formation row.
                // The hop never mutates `m7b_squad` state — that's the
                // whole point of the spec's "doctrine survives the hop"
                // invariant.
                let squad_hop_payload = {
                    let mut squad_lock = self.state.write().ok();
                    if let Some(s) = squad_lock.as_mut() {
                        Some(s.m7b_squad.brain_hop_payload(
                            crate::m7b_squad::PLAYER_SQUAD_ID,
                            prior_id,
                            target_actor_id,
                            tick.0,
                            true,
                        ))
                    } else {
                        None
                    }
                };
                if let Some(p) = squad_hop_payload {
                    self.recorder.record(tick, sim_time_ms, "squad", "brain_hop", p, None);
                }
                // **M13** § "Brain hopping" — caption "Switched to <actor name>".
                if let Ok(mut s) = self.state.write() {
                    let caption_label = format!("Switched to actor {target_actor_id}");
                    s.hud_captions.push_back(crate::state::CaptionView {
                        id: format!("brain_hop.{target_actor_id}"),
                        label: caption_label,
                        raised_at_tick: tick.0,
                        accessibility_id: format!("hud.caption.brain_hop.{target_actor_id}"),
                    });
                }
                CommandResult::accepted(tick.0)
            }
            // **M13** § "Chassis ability slots".
            ControlCommand::ActPlayerActivateAbility { ability, source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.activate_ability");
                }
                let _ = source;
                let player_id = state.player_actor.expect("player actor present");
                let parsed = cf_chassis::ChassisAbility::parse(&ability);
                let Some(parsed) = parsed else {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.player.activate_ability", "reason": "unknown_ability", "ability": ability}),
                        None,
                    );
                    return CommandResult::rejected("unknown_ability", tick.0);
                };
                let mut outcome: Result<cf_chassis::AbilitySlotState, cf_chassis::AbilityRejectReason> =
                    Err(cf_chassis::AbilityRejectReason::NotEquipped);
                if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        if let Some(chassis) = actor.chassis.as_mut() {
                            outcome = chassis.activate_ability(parsed);
                        } else {
                            drop(state);
                            self.recorder.record(
                                tick,
                                sim_time_ms,
                                "control",
                                "command_rejected",
                                json!({"method": "act.player.activate_ability", "reason": "no_chassis_attached"}),
                                None,
                            );
                            return CommandResult::rejected("no_chassis_attached", tick.0);
                        }
                    }
                }
                drop(state);
                match outcome {
                    Ok(slot) => {
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "ability",
                            "activated",
                            json!({
                                "actor": player_id.0,
                                "ability": parsed.as_str(),
                                "effect_ticks_total": slot.effect_total_ticks,
                                "cooldown_ticks_total": slot.cooldown_total_ticks,
                            }),
                            None,
                        );
                        CommandResult::accepted(tick.0)
                    }
                    Err(reason) => {
                        let r = reason.as_str();
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "control",
                            "command_rejected",
                            json!({"method": "act.player.activate_ability", "reason": r, "ability": parsed.as_str()}),
                            None,
                        );
                        CommandResult::rejected(r, tick.0)
                    }
                }
            }
            // **M13** § "Cockpit camera anchor".
            ControlCommand::ActInputCameraAnchor { mode, source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.input.camera_anchor");
                }
                let _ = source;
                let player_id = state.player_actor.expect("player actor present");
                let Some(parsed) = cf_chassis::CameraAnchor::parse(&mode) else {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.input.camera_anchor", "reason": "unknown_camera_anchor", "mode": mode}),
                        None,
                    );
                    return CommandResult::rejected("unknown_camera_anchor", tick.0);
                };
                let mut prev_anchor: Option<cf_chassis::CameraAnchor> = None;
                let mut reject_reason: Option<&'static str> = None;
                if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        if let Some(chassis) = actor.chassis.as_mut() {
                            match chassis.set_camera_anchor(parsed) {
                                Ok(prev) => prev_anchor = Some(prev),
                                Err(r) => reject_reason = Some(r),
                            }
                        } else {
                            reject_reason = Some("no_chassis_attached");
                        }
                    }
                }
                drop(state);
                if let Some(r) = reject_reason {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.input.camera_anchor", "reason": r, "mode": mode}),
                        None,
                    );
                    return CommandResult::rejected(r, tick.0);
                }
                let prev = prev_anchor.unwrap_or(cf_chassis::CameraAnchor::Default);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "camera",
                    "anchor_changed",
                    json!({
                        "actor": player_id.0,
                        "from": prev.as_str(),
                        "to": parsed.as_str(),
                    }),
                    None,
                );
                if parsed == cf_chassis::CameraAnchor::Cockpit {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "chassis",
                        "cockpit_entered",
                        json!({"actor": player_id.0}),
                        None,
                    );
                } else if prev == cf_chassis::CameraAnchor::Cockpit {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "chassis",
                        "cockpit_exited",
                        json!({"actor": player_id.0}),
                        None,
                    );
                }
                CommandResult::accepted(tick.0)
            }
            // **M13** § "Drone allies — 4 modes".
            ControlCommand::ActPlayerSetDroneMode { mode, source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.set_drone_mode");
                }
                let _ = source;
                let player_id = state.player_actor.expect("player actor present");
                let Some(parsed) = cf_chassis::DroneMode::parse(&mode) else {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.player.set_drone_mode", "reason": "unknown_drone_mode", "mode": mode}),
                        None,
                    );
                    return CommandResult::rejected("unknown_drone_mode", tick.0);
                };
                let mut prev_mode: Option<cf_chassis::DroneMode> = None;
                let mut applied = false;
                let mut spawned = false;
                if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        if actor.drone_ally.is_none() {
                            actor.drone_ally = Some(cf_chassis::DroneAllyState::default());
                            spawned = true;
                        }
                        if let Some(drone) = actor.drone_ally.as_mut() {
                            prev_mode = Some(drone.mode);
                            drone.mode = parsed;
                            applied = true;
                        }
                    }
                }
                drop(state);
                if !applied {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.player.set_drone_mode", "reason": "no_drone_state"}),
                        None,
                    );
                    return CommandResult::rejected("no_drone_state", tick.0);
                }
                if spawned {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "drone",
                        "spawned",
                        json!({"actor": player_id.0, "mode": parsed.as_str()}),
                        None,
                    );
                }
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "drone",
                    "mode_changed",
                    json!({
                        "actor": player_id.0,
                        "from": prev_mode.map(|m| m.as_str().to_string()),
                        "to": parsed.as_str(),
                    }),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            // **M13** § "Weapon modifier slots".
            ControlCommand::ActPlayerAttachModifier { modifier, source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.attach_modifier");
                }
                let _ = source;
                let player_id = state.player_actor.expect("player actor present");
                let Some(parsed) = cf_chassis::WeaponModifier::parse(&modifier) else {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.player.attach_modifier", "reason": "unknown_modifier", "modifier": modifier}),
                        None,
                    );
                    return CommandResult::rejected("unknown_modifier", tick.0);
                };
                let mut outcome: Result<bool, &'static str> = Err("no_chassis_attached");
                let mut now_combined = false;
                if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        if let Some(chassis) = actor.chassis.as_mut() {
                            outcome = chassis.attach_weapon_modifier(parsed);
                            now_combined = chassis.weapon_modifiers.is_combined();
                        }
                    }
                }
                drop(state);
                match outcome {
                    Ok(true) => {
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "equipment",
                            "modifier_attached",
                            json!({"actor": player_id.0, "modifier": parsed.as_str()}),
                            None,
                        );
                        if now_combined {
                            self.recorder.record(
                                tick,
                                sim_time_ms,
                                "equipment",
                                "weapon_modifier_combined",
                                json!({"actor": player_id.0}),
                                None,
                            );
                        }
                        CommandResult::accepted(tick.0)
                    }
                    Ok(false) => CommandResult::accepted(tick.0),
                    Err(r) => {
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "control",
                            "command_rejected",
                            json!({"method": "act.player.attach_modifier", "reason": r, "modifier": parsed.as_str()}),
                            None,
                        );
                        CommandResult::rejected(r, tick.0)
                    }
                }
            }
            ControlCommand::ActPlayerDetachModifier { modifier, source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.detach_modifier");
                }
                let _ = source;
                let player_id = state.player_actor.expect("player actor present");
                let Some(parsed) = cf_chassis::WeaponModifier::parse(&modifier) else {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.player.detach_modifier", "reason": "unknown_modifier", "modifier": modifier}),
                        None,
                    );
                    return CommandResult::rejected("unknown_modifier", tick.0);
                };
                let mut detached = false;
                let mut has_chassis = false;
                if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        if let Some(chassis) = actor.chassis.as_mut() {
                            has_chassis = true;
                            detached = chassis.detach_weapon_modifier(parsed);
                        }
                    }
                }
                drop(state);
                // **M14 audit pass 3 (Finding 2)**: previous implementation
                // always returned accepted even when nothing detached. Now:
                // reject with reason `no_chassis` when the player has no
                // chassis; reject with `modifier_not_attached` when the
                // chassis has no instance of `parsed` to remove.
                if !has_chassis {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.player.detach_modifier", "reason": "no_chassis", "modifier": parsed.as_str()}),
                        None,
                    );
                    return CommandResult::rejected("no_chassis", tick.0);
                }
                if !detached {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.player.detach_modifier", "reason": "modifier_not_attached", "modifier": parsed.as_str()}),
                        None,
                    );
                    return CommandResult::rejected("modifier_not_attached", tick.0);
                }
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "equipment",
                    "modifier_detached",
                    json!({"actor": player_id.0, "modifier": parsed.as_str()}),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            // **M13** § "Boarding / disembarking transitions".
            ControlCommand::ActPlayerBoard {
                chassis_actor_id,
                source,
            } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.board");
                }
                let _ = source;
                let player_id = state.player_actor.expect("player actor present");
                // **M14 audit pass 4 (Finding 1)**: the boarding timer +
                // pending-target now live on the PLAYER actor, not on the
                // target chassis. This makes input lock, HUD banner, and
                // concurrent-board rejection trivially correct: a single
                // `boarding_ticks_remaining > 0` check on the player
                // gates everything. On completion the actor transfer is
                // performed in `tick_chassis_eject_for_all`.
                let target_id = ActorId(chassis_actor_id);
                let validation: Result<(), &'static str> = (|| {
                    let sim = state.actor_state.as_ref().ok_or("no_actor_world")?;
                    let player = sim.world.actors.get(&player_id).ok_or("player_actor_missing")?;
                    if player.boarding_ticks_remaining > 0 || player.pending_boarding_target.is_some() {
                        return Err("player_already_boarding");
                    }
                    if player.chassis.is_some() {
                        return Err("player_already_chassis_bound");
                    }
                    let target_actor = sim.world.actors.get(&target_id).ok_or("target_actor_not_found")?;
                    if target_id == player_id {
                        return Err("cannot_board_self");
                    }
                    let target_chassis = target_actor.chassis.as_ref().ok_or("target_actor_has_no_chassis")?;
                    if target_chassis.is_in_transition() {
                        return Err("target_chassis_busy");
                    }
                    if target_actor.controllable {
                        return Err("target_chassis_already_piloted");
                    }
                    Ok(())
                })();
                if let Err(reason) = validation {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.player.board", "reason": reason, "chassis_actor_id": chassis_actor_id}),
                        None,
                    );
                    return CommandResult::rejected(reason, tick.0);
                }
                // **M14 audit pass 4 (Finding 1)**: latch the timer on the
                // player + also mark the target chassis as in-transition
                // so a second player attempting to board the same chassis
                // is rejected (target_chassis_busy).
                let transition_ticks = {
                    let mut latched: u32 = 90; // fallback: 1.5s @ 60Hz
                    if let Some(sim) = state.actor_state.as_ref() {
                        if let Some(target_actor) = sim.world.actors.get(&target_id) {
                            if let Some(chassis) = target_actor.chassis.as_ref() {
                                latched = chassis.transition_ticks_total.max(1);
                            }
                        }
                    }
                    latched
                };
                if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(target_actor) = sim.world.actors.get_mut(&target_id) {
                        if let Some(chassis) = target_actor.chassis.as_mut() {
                            chassis.begin_boarding();
                        }
                    }
                    if let Some(player) = sim.world.actors.get_mut(&player_id) {
                        player.boarding_ticks_remaining = transition_ticks;
                        player.pending_boarding_target = Some(chassis_actor_id);
                    }
                }
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "actor",
                    "boarding",
                    json!({"actor": player_id.0, "chassis_actor_id": chassis_actor_id, "duration_ms": 1500}),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActPlayerDisembark { source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.disembark");
                }
                let _ = source;
                let player_id = state.player_actor.expect("player actor present");
                let mut started = false;
                if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        if let Some(chassis) = actor.chassis.as_mut() {
                            started = chassis.begin_disembarking();
                        }
                    }
                }
                drop(state);
                if !started {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.player.disembark", "reason": "transition_in_progress_or_no_chassis"}),
                        None,
                    );
                    return CommandResult::rejected("transition_in_progress_or_no_chassis", tick.0);
                }
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "actor",
                    "disembarking",
                    json!({"actor": player_id.0, "duration_ms": 1500}),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::SettingsSet { changes } => {
                if changes.is_empty() {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.settings.set", "reason": "settings_patch_empty"}),
                        None,
                    );
                    return CommandResult::rejected("settings_patch_empty", tick.0);
                }
                if let Some(reason) = changes.validation_error() {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.settings.set", "reason": reason.clone()}),
                        None,
                    );
                    return CommandResult::rejected(reason, tick.0);
                }
                let prev_settings = state.settings.clone();
                let changed = apply_settings_patch(&mut state.settings, &changes);
                // **M1.5 G6**: when ai_difficulty changed, re-apply the
                // preset to every live ReactiveGuard so the new params take
                // effect on the next AI tick.
                //
                // M2 audit pass 7 (2026-05-13): also propagate the preset's
                // `hp` into every reactive guard's actor state so the spec
                // literal "guard's hp=120" round-trip holds.
                if changed.iter().any(|f| f == "ai_difficulty") {
                    let preset = cf_ai::DifficultyPreset::builtin(&state.settings.ai_difficulty);
                    if let Some(preset) = preset {
                        let tick_rate_hz = self.config.tick_rate_hz;
                        let guard_ids: Vec<ActorId> = state.reactive_guards.keys().copied().collect();
                        for guard in state.reactive_guards.values_mut() {
                            preset.apply_to(&mut guard.params, tick_rate_hz);
                        }
                        // M2 audit pass 7 (2026-05-13): also write preset.hp
                        // into each reactive guard's actor state so the
                        // round-trip "guard's hp=120" holds. Borrow guard_ids
                        // before the actor_state mutable borrow to avoid an
                        // overlapping mutable borrow on `state`.
                        if let Some(world) = state.actor_state.as_mut() {
                            for gid in &guard_ids {
                                if let Some(actor) = world.world.actors.get_mut(gid) {
                                    actor.hp = preset.hp;
                                    actor.hp_max = preset.hp;
                                }
                            }
                        }
                    }
                }
                // M1 audit pass 7 (2026-05-13): propagate `gravity` setting
                // into the live actor world so subsequent ticks use the new
                // value (settings.gravity is the magnitude; world.gravity
                // is signed-negative).
                if changed.iter().any(|f| f == "gravity") {
                    let gravity_signed = -state.settings.gravity;
                    if let Some(world) = state.actor_state.as_mut() {
                        world.world.gravity = gravity_signed;
                    }
                }
                let new_settings = state.settings.clone();
                drop(state);
                let value = serde_json::to_value(&new_settings).unwrap_or(serde_json::Value::Null);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "settings_changed",
                    json!({"method": "act.settings.set", "fields_changed": changed, "settings": value.clone()}),
                    None,
                );
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "settings_observed",
                    json!({"settings": value}),
                    None,
                );
                // M1 Gap G1: emit one accessibility.settings_changed event per
                // changed a11y-relevant field. Backward-compat: the
                // control.settings_changed envelope above stays unchanged.
                const A11Y_FIELDS: &[&str] = &[
                    "ui_scale",
                    "high_contrast",
                    "captions",
                    "reduced_motion",
                    "reduced_shake",
                    "reduced_flash",
                    "reduce_camera_shake_pct",
                    "hold_to_confirm",
                    "key_remap_enabled",
                    "key_bindings",
                    // M11 ACC-A surface fields.
                    "contrast_mode",
                    "caption_mode",
                    "caption_background_opacity",
                    "caption_categories",
                    "input_profile",
                    "remap_groups",
                    "hold_behavior",
                    "screen_shake_scale",
                    "camera_motion",
                    "objective_help",
                    "debug_explainer_level",
                ];
                let prev_value = serde_json::to_value(&prev_settings).unwrap_or(serde_json::Value::Null);
                for field in &changed {
                    if !A11Y_FIELDS.contains(&field.as_str()) {
                        continue;
                    }
                    let from = prev_value.get(field).cloned().unwrap_or(serde_json::Value::Null);
                    let to = value.get(field).cloned().unwrap_or(serde_json::Value::Null);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "accessibility",
                        "settings_changed",
                        json!({
                            "field": field,
                            "from": from,
                            "to": to,
                        }),
                        None,
                    );
                }
                // **M11 § DR-012 closure**: when `ui_scale` changed, emit a
                // dedicated `accessibility.ui_scale_applied` event so the
                // replay viewer + cf-app's UiScale binding agree on when the
                // HUD reflowed.
                if changed.iter().any(|f| f == "ui_scale") {
                    let _ = self.recorder.record(
                        tick,
                        sim_time_ms,
                        "accessibility",
                        "ui_scale_applied",
                        json!({ "ui_scale": new_settings.ui_scale }),
                        None,
                    );
                }
                // **M8 game_speed_assist consumer (Round-3 fix):** when the
                // game_speed_assist value transitioned (Off ↔ Slowdown75 ↔
                // Slowdown25 ↔ FullPause), surface the change as a dedicated
                // `ux.game_speed_assist_changed` event so replay tooling can
                // mark where the sim-tick scheduler will start skipping ticks.
                // The schema lives at
                // `cf-replay/schemas/event/ux_game_speed_assist_changed.json`.
                if changed.iter().any(|f| f == "game_speed_assist") {
                    let from_assist = prev_settings.game_speed_assist;
                    let to_assist = new_settings.game_speed_assist;
                    let payload = json!({
                        "from": from_assist.as_str(),
                        "to": to_assist.as_str(),
                        "speed_pct": to_assist.speed_pct(),
                    });
                    let _ = self
                        .recorder
                        .record(tick, sim_time_ms, "ux", "game_speed_assist_changed", payload, None);
                }
                CommandResult::accepted(tick.0)
            }
            ControlCommand::RunBundleWrite { id_override } => {
                if let Some(id_override) = id_override {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({
                            "method": "runbundle.write",
                            "reason": "runbundle_id_override_not_supported_in_m0",
                            "id_override": id_override,
                            "fix_hint": "M0 run ids are deterministic from milestone/time/seed/scenario; explicit bundle id override lands with later tooling if still needed."
                        }),
                        None,
                    );
                    return CommandResult::rejected("runbundle_id_override_not_supported_in_m0", tick.0);
                }
                state.pending_runbundle = true;
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "runbundle.write", "id_override": id_override}),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::Shutdown { write_run_bundle } => {
                state.shutdown_requested = true;
                state.pending_runbundle = state.pending_runbundle || write_run_bundle;
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "system.shutdown", "write_run_bundle": write_run_bundle}),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActM6 { action, source } => {
                self.dispatch_m6_action(action, source, tick, sim_time_ms, state)
            }
            ControlCommand::ActSquadIssueCommand {
                bot_actor,
                kind,
                waypoint,
                source,
            } => self.dispatch_squad_command(bot_actor, kind, waypoint, source, tick, sim_time_ms, state),
            ControlCommand::ActSquadCancelCommand { actor_id, source } => {
                self.dispatch_squad_cancel_command(actor_id, source, tick, sim_time_ms, state)
            }
            ControlCommand::ActPlayerSetPriority {
                actor_id,
                task,
                weight,
                source,
            } => self.dispatch_set_priority(actor_id, task, weight, source, tick, sim_time_ms, state),
            ControlCommand::ActPlayerSetAutonomyMode { actor_id, mode, source } => {
                self.dispatch_set_autonomy_mode(actor_id, mode, source, tick, sim_time_ms, state)
            }
            ControlCommand::ActPlayerApplyRoleTemplate {
                actor_id,
                template_id,
                source,
            } => self.dispatch_apply_role_template(actor_id, template_id, source, tick, sim_time_ms, state),
            ControlCommand::ActPlayerApplyQuickPreset {
                actor_id,
                preset_id,
                source,
            } => self.dispatch_apply_quick_preset(actor_id, preset_id, source, tick, sim_time_ms, state),
            // === M7B squad-command grammar ===
            ControlCommand::ActSquadIssue {
                squad_id,
                verb_id,
                args,
                source,
            } => self.dispatch_m7b_squad_issue(squad_id, verb_id, args, source, tick, sim_time_ms, state),
            ControlCommand::ActSquadSetFormation {
                squad_id,
                formation_kind,
                source,
            } => self.dispatch_m7b_squad_set_formation(squad_id, formation_kind, source, tick, sim_time_ms, state),
            ControlCommand::ActSquadAssignRole {
                squad_id,
                member_actor_id,
                role,
                source,
            } => self.dispatch_m7b_squad_assign_role(squad_id, member_actor_id, role, source, tick, sim_time_ms, state),
            ControlCommand::SrvDumpSquadState { squad_id, source } => {
                self.dispatch_m7b_dump_squad_state(squad_id, source, tick, sim_time_ms, state)
            }
            // === M8 cfctl surface ===
            ControlCommand::ActCameraSetMode { mode, source } => {
                self.dispatch_camera_set_mode(mode, source, tick, sim_time_ms, state)
            }
            ControlCommand::ActCameraHitStop {
                duration_ms,
                trigger,
                actor_id,
                source,
            } => self.dispatch_camera_hit_stop(duration_ms, trigger, actor_id, source, tick, sim_time_ms, state),
            ControlCommand::ActCameraScopeZoom { source } => {
                self.dispatch_camera_scope_zoom(source, tick, sim_time_ms, state)
            }
            ControlCommand::ActCameraFreeLookToggle {
                active,
                cursor,
                max_distance,
                source,
            } => self.dispatch_camera_free_look_toggle(active, cursor, max_distance, source, tick, sim_time_ms, state),
            ControlCommand::ActPhotoEnter { source } => self.dispatch_photo_enter(source, tick, sim_time_ms, state),
            ControlCommand::ActPhotoExit { source } => self.dispatch_photo_exit(source, tick, sim_time_ms, state),
            ControlCommand::ActPhotoCycleFilter { source } => {
                self.dispatch_photo_cycle_filter(source, tick, sim_time_ms, state)
            }
            ControlCommand::ActPhotoShoot { source } => self.dispatch_photo_shoot(source, tick, sim_time_ms, state),
            ControlCommand::ActReplayScrub { delta_seconds, source } => {
                self.dispatch_replay_scrub(delta_seconds, source, tick, sim_time_ms, state)
            }
            ControlCommand::ActReplayBookmark { label, source } => {
                self.dispatch_replay_bookmark(label, source, tick, sim_time_ms, state)
            }
            ControlCommand::ActDebugToggleOverlay { overlay, source } => {
                self.dispatch_debug_toggle_overlay(overlay, source, tick, sim_time_ms, state)
            }
            ControlCommand::ActUiSetHudLayout { node, x, y, source } => {
                self.dispatch_ui_set_hud_layout(node, x, y, source, tick, sim_time_ms, state)
            }
            ControlCommand::ActUiSavePreset { name, source } => {
                self.dispatch_ui_save_preset(name, source, tick, sim_time_ms, state)
            }
            ControlCommand::ActPlayerToggleTacticalOverlay { multiplayer, source } => {
                self.dispatch_toggle_tactical_overlay(multiplayer, source, tick, sim_time_ms, state)
            }
            ControlCommand::ActPlayerComposePlan {
                actor_id,
                steps,
                source,
            } => self.dispatch_compose_plan(actor_id, steps, source, tick, sim_time_ms, state),
            ControlCommand::ActPlayerContextWheelSelect {
                actor_id,
                slot,
                target_kind,
                target_id,
                source,
            } => self.dispatch_context_wheel_select(
                actor_id,
                slot,
                target_kind,
                target_id,
                source,
                tick,
                sim_time_ms,
                state,
            ),
            ControlCommand::ActPlayerPanicCall { kind, source } => {
                self.dispatch_panic_call(kind, source, tick, sim_time_ms, state)
            }
            ControlCommand::ActPlayerTagTarget { target_id, source } => {
                self.dispatch_tag_target(target_id, source, tick, sim_time_ms, state)
            }
            ControlCommand::ActPlayerQueryWhy { actor_id, source } => {
                self.dispatch_query_why(actor_id, source, tick, sim_time_ms, state)
            }
            ControlCommand::ActPlayerPieMenuOpen {
                target_kind,
                target_id,
                multiplayer,
                source,
            } => self.dispatch_pie_menu_open(target_kind, target_id, multiplayer, source, tick, sim_time_ms, state),
            ControlCommand::ActPlayerPieMenuSelect { slot, reason, source } => {
                self.dispatch_pie_menu_select(slot, reason, source, tick, sim_time_ms, state)
            }
            ControlCommand::ActPlayerPieMenuClose { source } => {
                self.dispatch_pie_menu_close(source, tick, sim_time_ms, state)
            }
            ControlCommand::ActPlayerDigTrenchSegment {
                variant,
                tool_id,
                substrate_hardness,
                strict,
                source,
            } => {
                drop(state);
                self.dispatch_m9b_dig_trench_segment(
                    variant,
                    tool_id,
                    substrate_hardness,
                    strict,
                    source,
                    tick,
                    sim_time_ms,
                )
            }
            ControlCommand::ActPlayerPlaceTrenchModule {
                module_id,
                segment_id,
                source,
            } => {
                drop(state);
                self.dispatch_m9b_place_trench_module(module_id, segment_id, source, tick, sim_time_ms)
            }
            ControlCommand::ActPlayerRepairTrenchModule {
                module_id,
                segment_id,
                source,
            } => {
                drop(state);
                self.dispatch_m9b_repair_trench_module(module_id, segment_id, source, tick, sim_time_ms)
            }
            ControlCommand::ActPlayerDropTrenchTemplate { id, origin, source } => {
                let source_label = match source {
                    IntentSource::Human => "human",
                    IntentSource::Cfctl => "cfctl",
                    IntentSource::Ai => "ai",
                    IntentSource::Replay => "replay",
                };
                drop(state);
                let action_id = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({
                        "method": "act.player.drop_trench_template",
                        "id": id,
                        "origin": [origin.0, origin.1],
                        "source": source_label,
                    }),
                    None,
                );
                let template = match load_trench_template(&id) {
                    Ok(t) => t,
                    Err(err) => {
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "control",
                            "command_rejected",
                            json!({
                                "method": "act.player.drop_trench_template",
                                "reason": "trench_template_load_failed",
                                "id": id,
                                "detail": err,
                            }),
                            Some(action_id),
                        );
                        return CommandResult::rejected("trench_template_load_failed", tick.0);
                    }
                };
                let resolved: std::collections::HashSet<String> = resolved_fortifications_for_build();
                let inst_request = cf_content::TrenchTemplateInstantiation {
                    template: &template,
                    origin,
                    resolved_fortifications: resolved,
                    instance_id_base: tick.0.saturating_mul(1024),
                };
                let inst = template.instantiate(&inst_request);
                let placed_json: Vec<serde_json::Value> = inst
                    .placed_fortifications
                    .iter()
                    .map(|p| {
                        json!({
                            "fortification_id": p.fortification_id,
                            "world_pos": [p.world_pos.0, p.world_pos.1],
                            "instance_id": p.instance_id,
                        })
                    })
                    .collect();
                let missing_json: Vec<serde_json::Value> = inst
                    .missing_fortifications
                    .iter()
                    .map(|m| {
                        json!({
                            "fortification_id": m.fortification_id,
                            "world_pos": [m.world_pos.0, m.world_pos.1],
                        })
                    })
                    .collect();
                // **m9b-4**: also push the template's runtime segments
                // into the live trench-world index so subsequent
                // observe.trench_segment_at_pos calls find them.
                let segment_count = inst.segments.len();
                let first_segment_id = self.insert_trench_segments_bulk(inst.trench_segments.clone());
                let dropped_id = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "trench",
                    "template_dropped",
                    json!({
                        "template_id": inst.id,
                        "template_sha256": inst.template_sha256,
                        "origin": [inst.origin.0, inst.origin.1],
                        "segment_count": segment_count,
                        "first_segment_id": first_segment_id,
                        "placed_fortifications": placed_json,
                        "missing_fortifications": missing_json,
                    }),
                    Some(action_id),
                );
                for warn in &inst.missing_fortifications {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "trench",
                        "template_missing_fortification",
                        json!({
                            "template_id": inst.id,
                            "fortification_id": warn.fortification_id,
                            "world_pos": [warn.world_pos.0, warn.world_pos.1],
                            "reason": "m9c_not_shipped",
                        }),
                        Some(dropped_id.clone()),
                    );
                }
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActPlayerTreat {
                kind,
                target_actor_id,
                source,
            } => {
                drop(state);
                self.dispatch_m14h_treat(kind, target_actor_id, source, tick, sim_time_ms)
            }
            ControlCommand::ActPlayerScan {
                target_actor_id,
                source,
            } => {
                drop(state);
                self.dispatch_m14h_scan(target_actor_id, source, tick, sim_time_ms)
            }
            ControlCommand::ActPlayerCprRound {
                target_actor_id,
                source,
            } => {
                drop(state);
                self.dispatch_m14h_cpr_round(target_actor_id, source, tick, sim_time_ms)
            }
            ControlCommand::ActPlayerDefib {
                target_actor_id,
                source,
            } => {
                drop(state);
                self.dispatch_m14h_defib(target_actor_id, source, tick, sim_time_ms)
            }
            ControlCommand::ActPlayerSurgeryStart {
                target_actor_id,
                wounds_to_treat,
                surgeon_t1,
                seed,
                source,
            } => {
                drop(state);
                self.dispatch_m14h_surgery_start(
                    target_actor_id,
                    wounds_to_treat,
                    surgeon_t1,
                    seed,
                    source,
                    tick,
                    sim_time_ms,
                )
            }
            ControlCommand::ActPlayerTriageSelect {
                target_actor_id,
                source,
            } => {
                drop(state);
                self.dispatch_m14h_triage_select(target_actor_id, source, tick, sim_time_ms)
            }
            ControlCommand::ActPlayerInstallProsthetic {
                target_actor_id,
                kind,
                zone,
                source,
            } => {
                drop(state);
                self.dispatch_m14i_install_prosthetic(
                    target_actor_id,
                    kind,
                    zone,
                    source,
                    tick,
                    sim_time_ms,
                )
            }
            ControlCommand::ActPlayerMaintainProsthetic {
                target_actor_id,
                zone,
                source,
            } => {
                drop(state);
                self.dispatch_m14i_maintain_prosthetic(
                    target_actor_id,
                    zone,
                    source,
                    tick,
                    sim_time_ms,
                )
            }
            ControlCommand::ActPlayerRetireVeteran {
                target_actor_id,
                source,
            } => {
                drop(state);
                self.dispatch_m14i_retire_veteran(target_actor_id, source, tick, sim_time_ms)
            }
            ControlCommand::ActPlayerVault { source } => {
                drop(state);
                let _ = source;
                self.dispatch_m14j_vault(tick, sim_time_ms)
            }
            ControlCommand::ActPlayerWallJump { source } => {
                drop(state);
                let _ = source;
                self.dispatch_m14j_wall_jump(tick, sim_time_ms)
            }
            ControlCommand::ActPlayerFireGrapple {
                target_x,
                target_y,
                source,
            } => {
                drop(state);
                let _ = source;
                self.dispatch_m14j_fire_grapple(target_x, target_y, tick, sim_time_ms)
            }
            ControlCommand::ActPlayerRopeInput { climb, swing, source } => {
                drop(state);
                let _ = source;
                self.dispatch_m14j_rope_input(climb, swing, tick, sim_time_ms)
            }
            ControlCommand::ActPlayerReleaseRope { source } => {
                drop(state);
                let _ = source;
                self.dispatch_m14j_release_rope(tick, sim_time_ms)
            }
            ControlCommand::ActPlayerZiplineClip { line_id, source } => {
                drop(state);
                let _ = source;
                self.dispatch_m14j_zipline_clip(line_id, tick, sim_time_ms)
            }
            ControlCommand::ActPlayerZiplineBrake { engaged, source } => {
                drop(state);
                let _ = source;
                self.dispatch_m14j_zipline_brake(engaged, tick, sim_time_ms)
            }
            ControlCommand::ActPlayerMount { critter_id, source } => {
                drop(state);
                let _ = source;
                self.dispatch_m14j_mount(critter_id, tick, sim_time_ms)
            }
            ControlCommand::ActPlayerDismount { source } => {
                drop(state);
                let _ = source;
                self.dispatch_m14j_dismount(tick, sim_time_ms)
            }
        }
    }
}
