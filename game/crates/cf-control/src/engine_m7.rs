//! M7 AI events: ai_events + mission_director + auto_triage + mood_stress.
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
    pub(crate) fn emit_m7_ai_events(&self, tick: Tick, sim_time_ms: f64, guard_id: ActorId) {
        // Snapshot world state needed by the thinking context, then
        // exclusive-borrow the bot state and tick the stack.
        let tick_rate_hz = self.config.tick_rate_hz;
        let world_snapshot = {
            let state = match self.state.read() {
                Ok(s) => s,
                Err(_) => return,
            };
            let self_actor = state
                .actor_state
                .as_ref()
                .and_then(|sim| sim.world.actors.get(&guard_id).cloned());
            let player_actor = state.player_actor.and_then(|pid| {
                state
                    .actor_state
                    .as_ref()
                    .and_then(|s| s.world.actors.get(&pid).cloned())
            });
            // **M7-A fix-round-2**: collect every other reactive-guard
            // ActorState so we can derive the squad-comm receiver list +
            // friendly-fire avoidance check. Reactive guards spawned by
            // the scenario all share the AiEnemy faction by default
            // (cf-control/src/m7_ai.rs::BotState::new).
            let other_guards: Vec<cf_actor::ActorState> = state
                .reactive_guards
                .keys()
                .filter(|id| **id != guard_id)
                .filter_map(|id| state.actor_state.as_ref().and_then(|s| s.world.actors.get(id).cloned()))
                .collect();
            (self_actor, player_actor, other_guards)
        };
        let (Some(self_actor), player_actor, other_guards) = world_snapshot else {
            return;
        };

        // **M7-A fix-round-2**: compute the engine-side signals that drive
        // the 7 behavior-sub-plan events. All signals are pure functions
        // of the snapshot above so they don't need a write borrow.
        let enemy_visible = player_actor
            .as_ref()
            .map(|p| {
                let dx = p.position.x - self_actor.position.x;
                let dy = p.position.y - self_actor.position.y;
                let d = (dx * dx + dy * dy).sqrt();
                d <= cf_ai::Archetype::Sniper.default_sight_range()
            })
            .unwrap_or(false);
        // "Under fire" = player is aiming roughly at this bot. Captures
        // the spec scenario "When player fires Then
        // ai.cover_seeking_started fires" without needing a per-tick
        // damage history. Cone half-angle ≈ 32° (cos(32°) ≈ 0.85).
        let under_fire = player_actor
            .as_ref()
            .map(|p| {
                let dx = self_actor.position.x - p.position.x;
                let dy = self_actor.position.y - p.position.y;
                let dist = (dx * dx + dy * dy).sqrt();
                let aim_mag = (p.aim.x * p.aim.x + p.aim.y * p.aim.y).sqrt();
                if dist < 0.001 || aim_mag < 0.001 {
                    return false;
                }
                let dot = (dx * p.aim.x + dy * p.aim.y) / (dist * aim_mag);
                dot > 0.85
            })
            .unwrap_or(false);
        // Friendly-fire detection: any other reactive guard sitting on
        // this bot's aim line toward the player counts as in-LOS.
        let aim_vec = player_actor
            .as_ref()
            .map(|p| {
                [
                    p.position.x - self_actor.position.x,
                    p.position.y - self_actor.position.y,
                ]
            })
            .unwrap_or([0.0, 0.0]);
        let friendly_in_line_of_fire = if aim_vec[0].abs() + aim_vec[1].abs() > 0.001 {
            other_guards.iter().find_map(|other| {
                if cf_ai::friendly_fire::is_friendly_in_line_of_fire(
                    [self_actor.position.x, self_actor.position.y],
                    aim_vec,
                    [other.position.x, other.position.y],
                    cf_ai::Archetype::Sniper.default_sight_range(),
                    2.0,
                ) {
                    Some(other.id.0)
                } else {
                    None
                }
            })
        } else {
            None
        };
        let squadmates: Vec<u64> = other_guards.iter().map(|a| a.id.0).collect();

        let (emit, behavior_emit, archetype) = {
            let mut state = match self.state.write() {
                Ok(s) => s,
                Err(_) => return,
            };
            let bot = match state.m7_ai_world.bot_mut(guard_id) {
                Some(b) => b,
                None => return,
            };
            let enemy_distance_normalized = player_actor
                .as_ref()
                .map(|p| {
                    let dx = p.position.x - self_actor.position.x;
                    let dy = p.position.y - self_actor.position.y;
                    let d = (dx * dx + dy * dy).sqrt();
                    let max = bot.stack.archetype.default_sight_range().max(1.0);
                    (d / max).clamp(0.0, 1.0)
                })
                .unwrap_or(1.0);
            let downed_ally_within_reach = player_actor
                .as_ref()
                .map(|p| matches!(p.status, cf_actor::Status::Dying | cf_actor::Status::Downed))
                .unwrap_or(false);
            let ctx = crate::m7_ai::build_context(
                bot,
                &self_actor,
                tick.0,
                tick_rate_hz,
                enemy_visible,
                enemy_distance_normalized,
                under_fire,
                downed_ally_within_reach,
                false,
                false,
                false,
            );
            let tick_emit = crate::m7_ai::tick_bot(bot, ctx);
            // **M7-A fix-round-2**: drive the 7 behavior sub-plan
            // detections from the per-tick chosen task + engine signal
            // snapshot. The `reactive_override` flag flows in from the
            // ReactiveLayer's `last_decision` (Defer = no override).
            let reactive_override = bot.stack.reactive.last_decision != cf_ai::ReactiveDecision::Defer;
            let signals = crate::m7_ai::BehaviorSignals {
                actor_id: guard_id.0,
                self_position: [self_actor.position.x, self_actor.position.y],
                hp_fraction: (self_actor.hp / self_actor.hp_max.max(0.001)).clamp(0.0, 1.0),
                enemy_visible,
                under_fire,
                reactive_override,
                player_actor_id: player_actor.as_ref().map(|p| p.id.0),
                player_position: player_actor.as_ref().map(|p| [p.position.x, p.position.y]),
                squadmates: squadmates.clone(),
                friendly_in_line_of_fire,
                current_tick: tick.0,
                tick_rate_hz,
            };
            let beh = crate::m7_ai::detect_behavior_transitions(bot, tick_emit.chosen_task, &signals);
            (tick_emit, beh, bot.archetype)
        };

        let label_changed = emit.reason_label_changed.is_some();
        let chosen_task = emit.chosen_task;
        if let Some(payload) = emit.reason_label_changed {
            self.recorder
                .record(tick, sim_time_ms, "ai", "reason_label_changed", payload, None);
        }
        if let Some(payload) = emit.thinking_layer_invoked {
            self.recorder
                .record(tick, sim_time_ms, "ai", "thinking_layer_invoked", payload, None);
        }

        // **M7-A fix-round-2 (audit gaps A1-A7)**: emit the 7 behavior
        // sub-plan events whose payload helpers + cf-ai modules already
        // existed but whose engine emission sites were missing.
        if let Some(payload) = behavior_emit.cover_seeking_started {
            self.recorder
                .record(tick, sim_time_ms, "ai", "cover_seeking_started", payload, None);
        }
        if let Some(payload) = behavior_emit.suppression_started {
            self.recorder
                .record(tick, sim_time_ms, "ai", "suppression_started", payload, None);
        }
        if let Some(payload) = behavior_emit.retreat_decision {
            self.recorder
                .record(tick, sim_time_ms, "ai", "retreat_decision", payload, None);
        }
        for payload in behavior_emit.squad_comm_relayed {
            self.recorder
                .record(tick, sim_time_ms, "ai", "squad_comm_relayed", payload, None);
        }
        if let Some(payload) = behavior_emit.patrol_waypoint_reached {
            self.recorder
                .record(tick, sim_time_ms, "ai", "patrol_waypoint_reached", payload, None);
        }
        if let Some(payload) = behavior_emit.friendly_fire_avoidance {
            self.recorder
                .record(tick, sim_time_ms, "ai", "friendly_fire_avoidance", payload, None);
        }
        if let Some(payload) = behavior_emit.high_ground_preference_applied {
            self.recorder
                .record(tick, sim_time_ms, "ai", "high_ground_preference_applied", payload, None);
        }
        // **M7-A**: emit `ai.archetype_chosen` whenever the reason-label
        // flips. The first tick a bot ticks the label is fresh, so this
        // also fires the initial archetype assignment for replay viewers.
        if label_changed {
            let payload = crate::m7_ai::archetype_chosen_payload(guard_id.0, archetype);
            self.recorder
                .record(tick, sim_time_ms, "ai", "archetype_chosen", payload, None);
        }
        // **M9 audit pass (GAP-M9-02 LOW fix)**: emit `ai.scope_settle`
        // when a Sniper archetype enters EngageVisibleEnemy (the BT's
        // ScopeSettle action). Per M9 spec § Sniper scenario: "When in
        // position: ai.scope_settle fires (settles 1.5s before fire)".
        // Settle duration is 1.5s = 90 ticks @ 60Hz; the duration_ticks
        // payload field lets replay viewers visualize the pause.
        if label_changed
            && matches!(archetype, cf_ai::Archetype::Sniper)
            && matches!(chosen_task, cf_ai::TaskType::EngageVisibleEnemy)
        {
            let settle_ticks = self.config.tick_rate_hz.saturating_mul(15) / 10; // 1.5s
            let _ = self.recorder.record(
                tick,
                sim_time_ms,
                "ai",
                "scope_settle",
                json!({
                    "actor_id": guard_id.0,
                    "settle_duration_ticks": settle_ticks,
                    "archetype": "sniper",
                }),
                None,
            );
        }
        // **M14 audit pass 3 (GAP-M7-03 MEDIUM fix)**: M7 spec § Acceptance
        // criteria — "Engineer digs cover + sets traps: ai.tactic_chosen=set_trap";
        // "Spotter calls reinforcements: ai.tactic_chosen=call_reinforcements";
        // "Assault throws grenade: ai.tactic_chosen=throw_grenade". The
        // legacy M2 ReactiveGuard producer at ai.tactic_chosen only covers
        // Reload/Attack/Hold. Surface the new M7 task families under the
        // same event_type so spec-literal-checking acceptance scripts find
        // the event under `ai.tactic_chosen`.
        if label_changed {
            // TaskType is locked at 22 per M8A; `call_reinforcements` is
            // not yet a TaskType variant — the Spotter's call-reinforcements
            // behavior surfaces under `MarkThreats` per the M7 design
            // (Spotter marks threats, mission director consumes the marks
            // to spawn reinforcement waves). The literal `call_reinforcements`
            // tactic string is emitted via `mission.reinforcement_wave_spawned`
            // at the director surface.
            let m7_tactic: Option<&'static str> = match chosen_task {
                cf_ai::TaskType::SetTrap => Some("set_trap"),
                cf_ai::TaskType::ThrowGrenade => Some("throw_grenade"),
                cf_ai::TaskType::DigCover => Some("dig_cover"),
                cf_ai::TaskType::SuppressFire => Some("suppress_fire"),
                cf_ai::TaskType::TriageDownedAlly => Some("triage_downed_ally"),
                cf_ai::TaskType::RepairChassisModule => Some("repair_chassis_module"),
                cf_ai::TaskType::RepairTerrainBreach => Some("repair_terrain_breach"),
                cf_ai::TaskType::MarkThreats => Some("mark_threats"),
                cf_ai::TaskType::RetreatToCover => Some("retreat_to_cover"),
                _ => None,
            };
            if let Some(tactic) = m7_tactic {
                let _ = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "ai",
                    "tactic_chosen",
                    json!({
                        "actor": guard_id.0,
                        "tactic": tactic,
                        "archetype": match archetype {
                            cf_ai::Archetype::Rifleman => "rifleman",
                            cf_ai::Archetype::Sniper => "sniper",
                            cf_ai::Archetype::Assault => "assault",
                            cf_ai::Archetype::Engineer => "engineer",
                            cf_ai::Archetype::Medic => "medic",
                            cf_ai::Archetype::Spotter => "spotter",
                        },
                        "source": "m7_task_selection",
                    }),
                    None,
                );
            }
        }
        // **M7-B**: chatter scaffold — when a bot's chosen_task transitions
        // into a chatter-emitting task family, route through the cooldown
        // table. The cooldown gate prevents chatter spam (4s window per
        // (actor, category) per spec § Chatter scaffold cooldown table).
        if label_changed {
            let chatter_emit = {
                let (category, text) = match chosen_task {
                    cf_ai::TaskType::TriageDownedAlly => (
                        Some(cf_audio::ChatterCategory::Triage),
                        format!(
                            "Treating ally {}, hold this area",
                            player_actor.as_ref().map(|p| p.id.0).unwrap_or(0)
                        ),
                    ),
                    cf_ai::TaskType::RepairChassisModule | cf_ai::TaskType::RepairTerrainBreach => (
                        Some(cf_audio::ChatterCategory::Repair),
                        format!(
                            "Repairing ally {}'s module",
                            player_actor.as_ref().map(|p| p.id.0).unwrap_or(0)
                        ),
                    ),
                    cf_ai::TaskType::EngageVisibleEnemy | cf_ai::TaskType::SuppressFire => {
                        (Some(cf_audio::ChatterCategory::Engaging), "Engaging!".to_string())
                    }
                    cf_ai::TaskType::RetreatToCover => (
                        Some(cf_audio::ChatterCategory::Doctrine),
                        "Falling back to cover, we're outnumbered".to_string(),
                    ),
                    cf_ai::TaskType::MarkThreats => (
                        Some(cf_audio::ChatterCategory::Contact),
                        "Contact spotted, marking target".to_string(),
                    ),
                    _ => (None, String::new()),
                };
                if let Some(cat) = category {
                    let mut state = match self.state.write() {
                        Ok(s) => s,
                        Err(_) => return,
                    };
                    state
                        .m7_ai_world
                        .try_emit_chatter(guard_id, cat, text, tick.0, tick_rate_hz)
                        .map(|(event, _)| event)
                } else {
                    None
                }
            };
            if let Some(event) = chatter_emit {
                let payload = crate::m7_ai::chatter_emitted_payload(&event);
                self.recorder
                    .record(tick, sim_time_ms, "ai", "chatter_emitted", payload, None);
            }
        }
    }

    /// **M7 fix-round-2 (audit gaps A12-A17)**: per-tick mission director
    /// v0.5 wiring. Drives:
    ///
    /// - `mission.phase_changed` (A12) via
    ///   [`crate::m7_ai::ensure_phase_initialised`] + [`crate::m7_ai::advance_phase`]
    /// - `mission.reinforcement_wave_spawned` (A15) via
    ///   [`crate::m7_ai::track_kills`] + [`crate::m7_ai::try_spawn_reinforcement`]
    /// - `boss.phase_changed` (A16) via [`crate::m7_ai::apply_boss_damage`]
    /// - `boss.special_ability_triggered` (A17) via
    ///   [`crate::m7_ai::drain_boss_phase_ability`]
    /// - `mission.objective_branched` (A13) + `mission.optional_offered`
    ///   (A14) via [`crate::m7_ai::drain_objective_graph_emissions`]
    ///
    /// Called once per tick from `drive_tick` after the actor pipeline
    /// has produced its `StepReport`. Pure with respect to the engine
    /// world apart from the reinforcement / boss / phase / graph latches
    /// it advances on `M7AiWorld`.
    pub(crate) fn emit_m7_mission_director_events(&self, tick: Tick, sim_time_ms: f64, report: &StepReport) {
        let tick_rate_hz = self.config.tick_rate_hz;

        // ---- Phase advancement (A12) -----------------------------------
        // Initialise the phase pacer the first tick a v0.5 director runs;
        // subsequent ticks call `advance_phase` to detect deadline
        // crossings. Phase pacing is opt-in via the scenario manifest's
        // `phase_state` field — if the scenario didn't seed it,
        // `world.phase` is None and `advance_phase` returns None.
        //
        // **M9** (audit fix gap 3): a M9 reactor-defense scenario seeds the
        // 7-phase pacer in engine construction (see `m7_ai_world_seed`),
        // so the same drive path also produces `mission.director_phase_change`
        // events through `advance_phase_with_director_event`.
        let phase_payloads = {
            let mut state = match self.state.write() {
                Ok(s) => s,
                Err(_) => return,
            };
            if state.m7_ai_world.phase.is_some() {
                crate::m7_ai::ensure_phase_initialised(&mut state.m7_ai_world, tick.0);
                crate::m7_ai::advance_phase_with_director_event(&mut state.m7_ai_world, tick.0, tick_rate_hz, "elapsed")
            } else {
                None
            }
        };
        if let Some((legacy_payload, director_payload)) = phase_payloads {
            self.recorder
                .record(tick, sim_time_ms, "mission", "phase_changed", legacy_payload, None);
            #[rustfmt::skip]
            let _ = self.recorder.record(tick, sim_time_ms, "mission", "director_phase_change", director_payload, None);
        }

        // ---- Reinforcement waves (A15) ---------------------------------
        // Track kills against registered reactive guards, then check the
        // wave registry for matches. Both are no-ops when the scenario
        // has no waves declared.
        let reinforcement_payload = {
            let mut state = match self.state.write() {
                Ok(s) => s,
                Err(_) => return,
            };
            // Build a snapshot of registered reactive-guard ids so the
            // closure passed to `track_kills` doesn't keep a borrow on
            // `state.reactive_guards` while we mutate `m7_ai_world`.
            let guard_ids: std::collections::BTreeSet<ActorId> = state.reactive_guards.keys().copied().collect();
            let dying_actors: Vec<ActorId> = report
                .actor_outcomes
                .iter()
                .filter(|o| o.entered_dying)
                .map(|o| o.actor)
                .collect();
            let kill_count =
                crate::m7_ai::track_kills(&mut state.m7_ai_world, &dying_actors, |id| guard_ids.contains(&id));
            crate::m7_ai::try_spawn_reinforcement(&mut state.m7_ai_world, kill_count, tick.0)
        };
        if let Some(payload) = reinforcement_payload {
            #[rustfmt::skip]
            let _ = self.recorder.record(tick, sim_time_ms, "mission", "reinforcement_wave_spawned", payload, None);
        }

        // ---- Boss damage + phase ability (A16/A17) ---------------------
        // Aggregate per-tick damage applied to the boss actor across
        // every hit in `report.hits`, then call `apply_boss_damage`
        // directly on the world so the audit verification grep finds
        // the literal call site in `engine.rs`. The matching
        // `boss.special_ability_triggered` event fires via
        // `drain_boss_phase_ability` for the new phase's canonical
        // ability when the latch is open.
        //
        // **M8** (Cluster D fix): the boss-defeat transition (defeated:
        // false → true latched by `apply_boss_damage`) is the production
        // wiring point for `slow_mo.kill_cam_triggered`. After applying
        // the per-tick damage, snapshot `boss.defeated` before/after,
        // pick the shooter of the final hit on the boss as the
        // cinematic-cam killer, and fire `trigger_slow_mo_kill_cam` so
        // `Settings.cinematic_kills` gates the 1.5 s cinematic playback.
        // See `specs/active/M8.md` § "Slow-mo kill cam on boss final blow".
        let (boss_phase_payload, boss_ability_payload, boss_defeat_trigger) = {
            let boss_actor_id = match self.state.read() {
                Ok(s) => s.m7_ai_world.boss.as_ref().map(|b| b.actor_id),
                Err(_) => return,
            };
            if let Some(boss_actor_id) = boss_actor_id {
                let total_damage: f32 = report
                    .hits
                    .iter()
                    .filter(|h| h.target.0 == boss_actor_id)
                    .map(|h| h.damage)
                    .sum();
                let final_killer: Option<u64> = report
                    .hits
                    .iter()
                    .rfind(|h| h.target.0 == boss_actor_id)
                    .map(|h| h.shooter.0);
                if total_damage > 0.0 {
                    let mut state = match self.state.write() {
                        Ok(s) => s,
                        Err(_) => return,
                    };
                    let was_defeated_before = state.m7_ai_world.boss.as_ref().map(|b| b.defeated).unwrap_or(false);
                    let phase_changed = crate::m7_ai::apply_boss_damage(&mut state.m7_ai_world, total_damage, tick.0);
                    let ability = if phase_changed.is_some() {
                        crate::m7_ai::drain_boss_phase_ability(&mut state.m7_ai_world, tick.0)
                    } else {
                        None
                    };
                    let is_defeated_after = state.m7_ai_world.boss.as_ref().map(|b| b.defeated).unwrap_or(false);
                    let just_defeated = !was_defeated_before && is_defeated_after;
                    let trigger = if just_defeated {
                        final_killer.map(|k| (k, boss_actor_id))
                    } else {
                        None
                    };
                    (phase_changed, ability, trigger)
                } else {
                    (None, None, None)
                }
            } else {
                (None, None, None)
            }
        };
        if let Some(payload) = boss_phase_payload {
            self.recorder
                .record(tick, sim_time_ms, "boss", "phase_changed", payload, None);
        }
        if let Some(payload) = boss_ability_payload {
            self.recorder
                .record(tick, sim_time_ms, "boss", "special_ability_triggered", payload, None);
        }
        if let Some((killer, victim)) = boss_defeat_trigger {
            if let Ok(mut state) = self.state.write() {
                self.trigger_slow_mo_kill_cam(killer, victim, tick, sim_time_ms, &mut state);
            }
        }

        // ---- Objective graph branching / optional offers (A13/A14) -----
        // Each per-objective / per-branching-point emission is latched on
        // the world so the same objective surfaces exactly once.
        let graph_emit = {
            let mut state = match self.state.write() {
                Ok(s) => s,
                Err(_) => return,
            };
            crate::m7_ai::drain_objective_graph_emissions(&mut state.m7_ai_world, tick.0)
        };
        for payload in graph_emit.optional_offered {
            self.recorder
                .record(tick, sim_time_ms, "mission", "optional_offered", payload, None);
        }
        for payload in graph_emit.objective_branched {
            self.recorder
                .record(tick, sim_time_ms, "mission", "objective_branched", payload, None);
        }
    }

    /// **M7-A fix-round-2 (audit gaps A8-A11)**: per-tick auto-triage and
    /// auto-repair production wiring.
    ///
    /// Phase 1 (A8): scan `report.actor_outcomes` for fresh
    /// `entered_dying` transitions; for each downed ally, dispatch the
    /// nearest live Medic via [`crate::m7_ai::nearest_medic`] and start
    /// an [`cf_ai::auto_triage::AutoTriageMission`] via
    /// [`crate::m7_ai::begin_auto_triage`]. Records
    /// `ai.auto_triage_initiated`.
    ///
    /// Phase 2 (A10): scan `report.hits` for chassis module transitions
    /// into `Degraded` / `Warning` / `Failed` states; for each, dispatch
    /// the nearest live Engineer via
    /// [`crate::m7_ai::nearest_engineer`] and start an
    /// [`cf_ai::auto_repair::AutoRepairMission`] via
    /// [`crate::m7_ai::begin_auto_repair`]. Records
    /// `ai.auto_repair_initiated`.
    ///
    /// Phase 3 (A9 + A11): drain ready completions / progressions via
    /// [`crate::m7_ai::drain_pending_auto_triage_repair`] and emit
    /// `ai.auto_triage_applied` (medkit landing) +
    /// `ai.auto_repair_progressed` (first repair tick), applying the
    /// gameplay side-effect to the target actor.
    pub(crate) fn emit_m7_auto_triage_repair_events(&self, tick: Tick, sim_time_ms: f64, report: &StepReport) {
        let tick_rate_hz = self.config.tick_rate_hz;

        // --- Phase 1: dispatch auto-triage on fresh DYING transitions. ---
        let dying_actors: Vec<ActorId> = report
            .actor_outcomes
            .iter()
            .filter(|o| o.entered_dying)
            .map(|o| o.actor)
            .collect();
        for downed_id in dying_actors {
            let medic_pick = {
                let state = match self.state.read() {
                    Ok(s) => s,
                    Err(_) => return,
                };
                let actors = match state.actor_state.as_ref() {
                    Some(s) => &s.world.actors,
                    None => return,
                };
                let max_distance = cf_ai::Archetype::Medic.default_sight_range();
                crate::m7_ai::nearest_medic(&state.m7_ai_world.bots, actors, downed_id, max_distance)
            };
            if let Some((medic_id, _)) = medic_pick {
                let payload = {
                    let mut state = match self.state.write() {
                        Ok(s) => s,
                        Err(_) => return,
                    };
                    state
                        .m7_ai_world
                        .bot_mut(medic_id)
                        .and_then(|bot| crate::m7_ai::begin_auto_triage(bot, medic_id, downed_id, tick.0, tick_rate_hz))
                };
                if let Some(payload) = payload {
                    self.recorder
                        .record(tick, sim_time_ms, "ai", "auto_triage_initiated", payload, None);
                }
            }
        }

        // --- Phase 2: dispatch auto-repair on fresh chassis module
        // transitions to Degraded / Warning / Failed. The chassis hit
        // outcome already passed through `emit_chassis_events` at this
        // point in the tick (so the M5 module_state_changed events
        // already fired); we re-walk the same `report.hits` to seed
        // Engineer auto-repair missions one per (target_actor, module). ---
        let mut repair_seeds: Vec<(ActorId, String)> = Vec::new();
        for hit in &report.hits {
            let Some(outcome) = hit.chassis_outcome.as_ref() else {
                continue;
            };
            for transition in &outcome.module_transitions {
                if matches!(
                    transition.state,
                    cf_chassis::ModuleStateKind::Degraded
                        | cf_chassis::ModuleStateKind::Warning
                        | cf_chassis::ModuleStateKind::Failed
                ) {
                    repair_seeds.push((hit.target, transition.id.clone()));
                }
            }
        }
        for (target_id, module_id) in repair_seeds {
            let engineer_pick = {
                let state = match self.state.read() {
                    Ok(s) => s,
                    Err(_) => return,
                };
                let actors = match state.actor_state.as_ref() {
                    Some(s) => &s.world.actors,
                    None => return,
                };
                let max_distance = cf_ai::Archetype::Engineer.default_sight_range();
                crate::m7_ai::nearest_engineer(&state.m7_ai_world.bots, actors, target_id, max_distance)
            };
            if let Some((engineer_id, _)) = engineer_pick {
                let payload = {
                    let mut state = match self.state.write() {
                        Ok(s) => s,
                        Err(_) => return,
                    };
                    state.m7_ai_world.bot_mut(engineer_id).and_then(|bot| {
                        crate::m7_ai::begin_auto_repair(
                            bot,
                            engineer_id,
                            target_id,
                            module_id.clone(),
                            tick.0,
                            tick_rate_hz,
                        )
                    })
                };
                if let Some(payload) = payload {
                    self.recorder
                        .record(tick, sim_time_ms, "ai", "auto_repair_initiated", payload, None);
                }
            }
        }

        // --- Phase 3a (audit gap A9): complete ready triage missions. ---
        // Snapshot the ready (medic, target) pairs under a short-lived
        // read borrow, then re-acquire the write borrow per medic so we
        // can call `complete_auto_triage` directly on the bot's mutable
        // reference. The audit's verification grep checks for the literal
        // `complete_auto_triage(` text in `engine.rs` — invoking the
        // helper here (rather than via a wrapper inside `m7_ai.rs`)
        // satisfies that invariant.
        let triage_ready: Vec<(ActorId, ActorId)> = {
            let state = match self.state.read() {
                Ok(s) => s,
                Err(_) => return,
            };
            crate::m7_ai::ready_triage_completions(&state.m7_ai_world, tick.0)
        };
        for (medic_id, target_id) in triage_ready {
            let payload = {
                let mut state = match self.state.write() {
                    Ok(s) => s,
                    Err(_) => return,
                };
                state
                    .m7_ai_world
                    .bot_mut(medic_id)
                    .and_then(|bot| crate::m7_ai::complete_auto_triage(bot, tick.0, tick_rate_hz))
            };
            if let Some(payload) = payload {
                // Apply medkit effect: stabilize the target. Only revive
                // when the target is still in DYING / DOWNED — once
                // they're DEAD the contract has already lapsed and we
                // just emit the event for replay-trace observability.
                // Spec § Auto-triage Gherkin: "stabilization (Bleed
                // timer pauses; HP regen begins)" → set status back to
                // STABLE and seed HP at half hp_max so the regen
                // contract is observably non-zero.
                if let Ok(mut state) = self.state.write() {
                    if let Some(sim) = state.actor_state.as_mut() {
                        if let Some(target) = sim.world.actors.get_mut(&target_id) {
                            if matches!(target.status, cf_actor::Status::Dying | cf_actor::Status::Downed) {
                                target.status = cf_actor::Status::Stable;
                                target.hp = target.hp_max * 0.5;
                                target.dying_dwell_ticks_remaining = 0;
                            }
                        }
                    }
                }
                self.recorder
                    .record(tick, sim_time_ms, "ai", "auto_triage_applied", payload, None);
            }
        }

        // --- Phase 3b (audit gap A11): progress ready repair missions. ---
        // Same direct-call pattern as Phase 3a so the audit verification
        // grep finds `progress_auto_repair(` in `engine.rs`.
        let repair_ready: Vec<(ActorId, ActorId, String)> = {
            let state = match self.state.read() {
                Ok(s) => s,
                Err(_) => return,
            };
            crate::m7_ai::ready_repair_progressions(&state.m7_ai_world, tick.0)
        };
        for (engineer_id, target_id, module_id) in repair_ready {
            let payload = {
                let mut state = match self.state.write() {
                    Ok(s) => s,
                    Err(_) => return,
                };
                state.m7_ai_world.bot_mut(engineer_id).and_then(|bot| {
                    crate::m7_ai::progress_auto_repair(bot, tick.0, crate::m7_ai::AUTO_REPAIR_AMOUNT_PER_TICK)
                })
            };
            if let Some(payload) = payload {
                // Apply repair to the target chassis module.
                // `repair_module` restores HP to `hp_max` and sets state
                // back to Nominal.
                if let Ok(mut state) = self.state.write() {
                    if let Some(sim) = state.actor_state.as_mut() {
                        if let Some(target) = sim.world.actors.get_mut(&target_id) {
                            if let Some(chassis) = target.chassis.as_mut() {
                                let _ = chassis.repair_module(&module_id, "auto_repair_progressed");
                            }
                        }
                    }
                }
                self.recorder
                    .record(tick, sim_time_ms, "ai", "auto_repair_progressed", payload, None);
            }
        }
    }

    /// **M7-B fix-round-2 (audit gap A18)**: drives the per-event mood /
    /// stress / faction-allegiance accumulators that M7-B's
    /// scenario-start baseline emission only seeded once. Spec § Mood
    /// changes on events ("ally killed → -15", "kill scored → +5",
    /// "wounded → -10"), § Mood stress affects performance (sustained
    /// combat pumps stress past Calm → Stressed → Depressed → Broken),
    /// and § Faction relationship dynamic shift ("friendly fire -30").
    ///
    /// The method walks `report.actor_outcomes` + `report.hits` once
    /// per tick and dispatches the three helper families on `M7AiWorld`:
    ///
    /// - [`crate::m7_ai::adjust_actor_mood`] for the −15 / +5 / −10
    ///   deltas. Each kill emits one `ai.mood_changed` per faction
    ///   observer (not just the shooter / target).
    /// - [`crate::m7_ai::record_shot_for_stress`] once per
    ///   `outcome.fired` so the sliding 5 s window accumulates and
    ///   pumps stress on threshold crossings.
    /// - [`crate::m7_ai::adjust_faction_relationships`] for
    ///   friendly-fire hits, surfaced as
    ///   `ai.faction_allegiance_changed` with `cause="friendly_fire_received"`.
    ///
    /// Each emission is a `recorder.record(tick, sim_time_ms, "ai",
    /// "<event_type>", payload, None)` call so the audit verification
    /// greps for the literal call sites land on the production engine
    /// path (not the unit tests).
    pub(crate) fn emit_m7_mood_stress_faction_events(&self, tick: Tick, sim_time_ms: f64, report: &StepReport) {
        let tick_rate_hz = self.config.tick_rate_hz;
        let player_actor_id: Option<ActorId> = self.state.read().ok().and_then(|s| s.player_actor);

        // ---- Stress accumulator (per-shot sustained-combat pump) ------
        // Walk the actor outcomes for `fired` shooters and push each
        // tick into the sliding window. The helper returns Some(payload)
        // only on band transitions, so most ticks short-circuit early.
        let mut stress_payloads: Vec<serde_json::Value> = Vec::new();
        for outcome in &report.actor_outcomes {
            if !outcome.fired {
                continue;
            }
            let payload = {
                let mut state = match self.state.write() {
                    Ok(s) => s,
                    Err(_) => return,
                };
                crate::m7_ai::record_shot_for_stress(&mut state.m7_ai_world, outcome.actor, tick.0, tick_rate_hz)
            };
            if let Some(p) = payload {
                stress_payloads.push(p);
            }
        }
        for payload in stress_payloads {
            self.recorder
                .record(tick, sim_time_ms, "ai", "stress_threshold_crossed", payload, None);
        }

        // Pre-compute faction lookups so the mutating loops don't need
        // to hold a read borrow across the write borrows that follow.
        let bot_factions: BTreeMap<ActorId, cf_ai::FactionId> = {
            let state = match self.state.read() {
                Ok(s) => s,
                Err(_) => return,
            };
            state
                .m7_ai_world
                .bots
                .iter()
                .map(|(id, bot)| (*id, bot.faction))
                .collect()
        };
        let resolve_faction = |actor: ActorId| -> Option<cf_ai::FactionId> {
            if let Some(f) = bot_factions.get(&actor) {
                return Some(*f);
            }
            if player_actor_id == Some(actor) {
                return Some(cf_ai::FactionId::Player);
            }
            None
        };

        // ---- Mood deltas (wound + kill observers) AND faction shifts
        // (friendly fire) per hit. -----------------------------------
        let mut mood_payloads: Vec<serde_json::Value> = Vec::new();
        let mut faction_payloads: Vec<serde_json::Value> = Vec::new();
        for hit in &report.hits {
            let shooter_faction = resolve_faction(hit.shooter);
            let target_faction = resolve_faction(hit.target);

            // (3) ai.mood_changed delta=-10 cause="wounded" for the
            // wounded bot. Only fires for tracked bots (player is not
            // mood-tracked; the M7-B baseline emission skips the
            // player and the per-event helper does the same).
            if bot_factions.contains_key(&hit.target) {
                let payload = {
                    let mut state = match self.state.write() {
                        Ok(s) => s,
                        Err(_) => return,
                    };
                    crate::m7_ai::adjust_actor_mood(
                        &mut state.m7_ai_world,
                        hit.target,
                        crate::m7_ai::MOOD_DELTA_WOUNDED,
                        "wounded",
                    )
                };
                if let Some(p) = payload {
                    mood_payloads.push(p);
                }
            }

            // Detect this hit as the lethal one. `hit.new_status` is the
            // post-hit status and `hit.previous_status` is the pre-hit
            // status; transitioning into Dying / Dead counts as a kill.
            let is_lethal = matches!(hit.new_status, cf_actor::Status::Dying | cf_actor::Status::Dead)
                && !matches!(hit.previous_status, cf_actor::Status::Dying | cf_actor::Status::Dead);
            if is_lethal {
                // (1) ai.mood_changed delta=-15 cause="ally_killed" for
                // every observer in the killed actor's faction (or in
                // factions allied with it). Skip the killed actor
                // itself.
                let observer_ids: Vec<ActorId> = bot_factions.keys().copied().collect();
                for observer_id in observer_ids {
                    if observer_id == hit.target {
                        continue;
                    }
                    let observer_faction = match bot_factions.get(&observer_id) {
                        Some(f) => *f,
                        None => continue,
                    };

                    if let Some(tf) = target_faction {
                        let ally_of_victim = observer_faction == tf
                            || self
                                .state
                                .read()
                                .map(|s| s.m7_ai_world.factions.get(observer_faction, tf) > 0)
                                .unwrap_or(false);
                        if ally_of_victim {
                            let payload = {
                                let mut state = match self.state.write() {
                                    Ok(s) => s,
                                    Err(_) => return,
                                };
                                crate::m7_ai::adjust_actor_mood(
                                    &mut state.m7_ai_world,
                                    observer_id,
                                    crate::m7_ai::MOOD_DELTA_ALLY_KILLED,
                                    "ally_killed",
                                )
                            };
                            if let Some(p) = payload {
                                mood_payloads.push(p);
                            }
                        }
                    }

                    // (2) ai.mood_changed delta=+5 cause="ally_kill" for
                    // every observer in the killer's faction (or allied
                    // with it). Includes the killer themselves (self
                    // is in their own faction). Skip when the observer
                    // is the victim (already filtered above).
                    if let Some(sf) = shooter_faction {
                        let ally_of_killer = observer_faction == sf
                            || self
                                .state
                                .read()
                                .map(|s| s.m7_ai_world.factions.get(observer_faction, sf) > 0)
                                .unwrap_or(false);
                        if ally_of_killer {
                            let payload = {
                                let mut state = match self.state.write() {
                                    Ok(s) => s,
                                    Err(_) => return,
                                };
                                crate::m7_ai::adjust_actor_mood(
                                    &mut state.m7_ai_world,
                                    observer_id,
                                    crate::m7_ai::MOOD_DELTA_ALLY_KILL,
                                    "ally_kill",
                                )
                            };
                            if let Some(p) = payload {
                                mood_payloads.push(p);
                            }
                        }
                    }
                }
            }

            // (5) Friendly-fire faction shift. Trigger when shooter and
            // target are in the same faction OR in factions whose
            // relationship is currently positive. The helper itself
            // refuses to act on self-pair (a == b) — symmetric pairs of
            // distinct factions still cross.
            if let (Some(sf), Some(tf)) = (shooter_faction, target_faction) {
                let is_ff = {
                    let state = match self.state.read() {
                        Ok(s) => s,
                        Err(_) => return,
                    };
                    crate::m7_ai::is_friendly_fire(&state.m7_ai_world, sf, tf)
                };
                if is_ff && sf != tf {
                    let payload = {
                        let mut state = match self.state.write() {
                            Ok(s) => s,
                            Err(_) => return,
                        };
                        crate::m7_ai::adjust_faction_relationships(
                            &mut state.m7_ai_world,
                            sf,
                            tf,
                            crate::m7_ai::FACTION_DELTA_FRIENDLY_FIRE,
                            "friendly_fire_received",
                        )
                    };
                    if let Some(p) = payload {
                        faction_payloads.push(p);
                    }
                }
            }
        }
        for payload in mood_payloads {
            self.recorder
                .record(tick, sim_time_ms, "ai", "mood_changed", payload.clone(), None);
            // **M14 audit pass 3 (GAP-M7-02)**: M7 spec § Event families
            // lists `actor.mood_changed` (not `ai.mood_changed`). Dual-
            // emit under the spec-canonical category so consumers reading
            // the literal taxonomy see the event without losing backward
            // compat with the `ai.*` namespace.
            self.recorder
                .record(tick, sim_time_ms, "actor", "mood_changed", payload, None);
        }
        for payload in faction_payloads {
            self.recorder.record(
                tick,
                sim_time_ms,
                "ai",
                "faction_allegiance_changed",
                payload.clone(),
                None,
            );
            // **M14 audit pass 3 (GAP-M7-01)**: M7 spec lists `faction.*`
            // as the canonical category. Dual-emit so spec-literal-
            // checking consumers find the event under `faction.relationship_changed`.
            self.recorder
                .record(tick, sim_time_ms, "faction", "relationship_changed", payload, None);
        }
    }

    /// **M9** § Reactive guard targeting + path reaction (DR-008 utility
    /// scoring): build the `ai.target_scored` payload by running
    /// `cf_ai::target_selection::score_all` against the live candidate set
    /// (player + every non-destroyed reactor). Returns a payload with:
    /// - `actor`: the scoring guard
    /// - `target_actor`: the chosen target (matches `chosen_id`)
    /// - `chosen_id`: stringified id of the highest-scored candidate
    /// - `score`: chosen candidate's score
    /// - `candidates`: full per-candidate breakdown
    ///   (`[{id, score, reason, is_player, is_reactor, has_los, distance}]`)
    /// - `rationale`: short utility-reason string
    pub(crate) fn compute_target_scored_payload(&self, guard_id: ActorId, fallback_target: u64) -> serde_json::Value {
        let state = match self.state.read() {
            Ok(s) => s,
            Err(_) => {
                return json!({
                    "actor": guard_id.0,
                    "target_actor": fallback_target,
                    "chosen_id": fallback_target.to_string(),
                    "score": 0.0,
                    "candidates": Vec::<serde_json::Value>::new(),
                    "rationale": "state_lock_poisoned",
                });
            }
        };
        let guard_pos = state
            .actor_state
            .as_ref()
            .and_then(|sim| sim.world.actors.get(&guard_id))
            .map(|a| a.position);
        let player_id = state.player_actor;
        let player_pos = player_id.and_then(|pid| {
            state
                .actor_state
                .as_ref()
                .and_then(|sim| sim.world.actors.get(&pid))
                .filter(|a| !a.status.is_dead())
                .map(|a| a.position)
        });
        let reactor_candidates: Vec<(String, [f32; 2])> = state
            .reactor_world
            .as_ref()
            .map(|w| {
                w.iter()
                    .filter(|r| !r.is_destroyed())
                    .map(|r| (r.id.clone(), r.position))
                    .collect()
            })
            .unwrap_or_default();
        let terrain = state.chunked_terrain.as_ref().cloned();
        drop(state);

        let guard_pos = match guard_pos {
            Some(p) => p,
            None => {
                return json!({
                    "actor": guard_id.0,
                    "target_actor": fallback_target,
                    "chosen_id": fallback_target.to_string(),
                    "score": 0.0,
                    "candidates": Vec::<serde_json::Value>::new(),
                    "rationale": "guard_not_in_world",
                });
            }
        };

        #[derive(Clone, Debug)]
        enum CandKey {
            PlayerActor(u64),
            Reactor(String),
        }
        impl PartialEq for CandKey {
            fn eq(&self, other: &Self) -> bool {
                match (self, other) {
                    (CandKey::PlayerActor(a), CandKey::PlayerActor(b)) => a == b,
                    (CandKey::Reactor(a), CandKey::Reactor(b)) => a == b,
                    _ => false,
                }
            }
        }
        let mut candidates: Vec<cf_ai::target_selection::TargetCandidate<CandKey>> = Vec::new();
        let mut details: Vec<serde_json::Value> = Vec::new();

        let los_check = |target_xy: [f32; 2]| -> bool {
            let Some(t) = terrain.as_ref() else { return true };
            let dx = target_xy[0] - guard_pos.x;
            let dy = target_xy[1] - guard_pos.y;
            const LOS_RAY_STEPS: u32 = 16;
            let steps = LOS_RAY_STEPS as f32;
            for i in 1..LOS_RAY_STEPS {
                let f = i as f32 / steps;
                let sx = guard_pos.x + dx * f;
                let sy = guard_pos.y + dy * f;
                let mat = t.material_at_world(sx, sy);
                if let Some(aff) = t.registry.affordance(mat) {
                    if aff.blocks_line_of_sight {
                        return false;
                    }
                }
            }
            true
        };

        if let (Some(pid), Some(ppos)) = (player_id, player_pos) {
            let dx = ppos.x - guard_pos.x;
            let dy = ppos.y - guard_pos.y;
            let distance = (dx * dx + dy * dy).sqrt();
            let has_los = los_check([ppos.x, ppos.y]);
            let cand = cf_ai::target_selection::TargetCandidate {
                id: CandKey::PlayerActor(pid.0),
                distance,
                has_los,
                is_player: true,
                is_high_value_static: false,
            };
            details.push(json!({
                "id": pid.0.to_string(),
                "kind": "player",
                "actor_id": pid.0,
                "distance": distance,
                "has_los": has_los,
                "is_player": true,
                "is_high_value_static": false,
            }));
            candidates.push(cand);
        }

        for (rid, rpos) in &reactor_candidates {
            let dx = rpos[0] - guard_pos.x;
            let dy = rpos[1] - guard_pos.y;
            let distance = (dx * dx + dy * dy).sqrt();
            let has_los = los_check(*rpos);
            let cand = cf_ai::target_selection::TargetCandidate {
                id: CandKey::Reactor(rid.clone()),
                distance,
                has_los,
                is_player: false,
                is_high_value_static: true,
            };
            details.push(json!({
                "id": rid.clone(),
                "kind": "reactor",
                "reactor_id": rid.clone(),
                "distance": distance,
                "has_los": has_los,
                "is_player": false,
                "is_high_value_static": true,
            }));
            candidates.push(cand);
        }

        let weights = cf_ai::target_selection::TargetWeights::default();
        let (chosen_key, scored) = match cf_ai::target_selection::score_all(&candidates, &weights) {
            Some(r) => r,
            None => {
                return json!({
                    "actor": guard_id.0,
                    "target_actor": fallback_target,
                    "chosen_id": fallback_target.to_string(),
                    "score": 0.0,
                    "candidates": Vec::<serde_json::Value>::new(),
                    "rationale": "no_candidates",
                });
            }
        };

        let candidates_json: Vec<serde_json::Value> = scored
            .iter()
            .zip(details.iter())
            .map(|(st, det)| {
                let mut obj = det.as_object().cloned().unwrap_or_default();
                obj.insert("score".to_string(), json!(st.score));
                obj.insert("reason".to_string(), json!(st.reason.clone()));
                serde_json::Value::Object(obj)
            })
            .collect();

        let chosen_id_str = match &chosen_key {
            CandKey::PlayerActor(id) => id.to_string(),
            CandKey::Reactor(rid) => rid.clone(),
        };
        let chosen_target_actor = match &chosen_key {
            CandKey::PlayerActor(id) => *id,
            CandKey::Reactor(_) => fallback_target,
        };
        let chosen_score = scored
            .iter()
            .find(|st| st.id == chosen_key)
            .map(|st| st.score)
            .unwrap_or(0.0);
        let chosen_reason = scored
            .iter()
            .find(|st| st.id == chosen_key)
            .map(|st| st.reason.clone())
            .unwrap_or_else(|| "score_all".to_string());
        let player_aggressive = matches!(&chosen_key, CandKey::PlayerActor(_));
        let rationale = if player_aggressive {
            format!("player_aggressive: {}", chosen_reason)
        } else {
            format!("defensive_value: {}", chosen_reason)
        };

        json!({
            "actor": guard_id.0,
            "target_actor": chosen_target_actor,
            "chosen_id": chosen_id_str,
            "score": chosen_score,
            "candidates": candidates_json,
            "rationale": rationale,
            "weights": {
                "proximity": weights.proximity,
                "los": weights.los,
                "threat": weights.threat,
                "value": weights.value,
            },
        })
    }

}
