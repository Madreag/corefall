//! emit_guard_events — per-guard reactive AI events.
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
    pub(crate) fn emit_guard_events(&self, tick: Tick, sim_time_ms: f64, guard_id: ActorId, report: &cf_ai::EnemyTickReport) {
        // Always emit ai.ai_perception (even when player_seen=false) so replay
        // viewers can step through the guard's awareness.
        let mut last_perception_signal_id: Option<String> = None;
        if let Some(p) = &report.perception {
            self.recorder.record(
                tick,
                sim_time_ms,
                "ai",
                "ai_perception",
                json!({
                    "actor": guard_id.0,
                    "player_seen": p.player_seen,
                    "distance": p.distance,
                    "angle_degrees": p.angle_degrees,
                    "last_seen_position": p.last_seen_position,
                    "state": p.state.as_str(),
                }),
                None,
            );
        }
        // **M1.5 G2/G3**: emit one `ai.perception_signal` per fresh signal.
        for sig in &report.perception_signals {
            // M2 audit pass 5 (2026-05-13): hearing perception signals chain
            // back to the originating `equipment.alarm_registered` event so
            // M10 can walk `state_changed(heard_shot) → perception_signal(hearing)
            // → alarm_registered`. Other signal kinds have no upstream parent
            // event (sight is intrinsic, memory_decayed is timer-driven).
            let perception_parent = sig.alarm_event_id.clone();
            // M2 audit pass 7 (2026-05-13): payload includes spec-literal
            // aliases — `actor_id` (guard), `source_id` (player), `source_pos`
            // (=source_position), `line_of_sight` (for sight kinds). Legacy
            // `actor`/`source_actor`/`source_position` retained.
            let line_of_sight = match sig.kind {
                "sight" => Some("clear"),
                "sight_lost" => Some("blocked"),
                _ => None,
            };
            let id = self.recorder.record(
                tick,
                sim_time_ms,
                "ai",
                "perception_signal",
                json!({
                    "actor_id": guard_id.0,
                    "actor": guard_id.0,
                    "kind": sig.kind,
                    "source_id": sig.source_actor,
                    "source_actor": sig.source_actor,
                    "source_pos": sig.source_position,
                    "source_position": sig.source_position,
                    "last_known_pos": sig.source_position,
                    "line_of_sight": line_of_sight,
                    "confidence": sig.confidence,
                }),
                perception_parent,
            );
            last_perception_signal_id = Some(id);
        }
        // M2 re-audit pass 4 (2026-05-13): retain the ai.tactic_chosen
        // event id so the subsequent equipment.weapon_fired emit can
        // chain back to it (spec cause chain requires
        // weapon_fired → tactic_chosen → target_acquired → perception_signal).
        //
        // **M4 § Parent-event-id cause chains**: when no fresh perception
        // signal fired this tick, fall back (in priority order) to the
        // actor's most recent ai.state_changed event, then to
        // system.run_started as the root parent. This guarantees
        // tactic_chosen always carries a parent_event_id per spec.
        let mut tactic_chosen_event_id: Option<String> = None;
        if let Some(t) = &report.tactic_chosen {
            let tactic_parent = last_perception_signal_id.clone().or_else(|| {
                self.state.read().ok().and_then(|s| {
                    s.last_ai_state_changed_by_actor
                        .get(&guard_id)
                        .cloned()
                        .or_else(|| s.run_started_event_id.clone())
                })
            });
            let id = self.recorder.record(
                tick,
                sim_time_ms,
                "ai",
                "tactic_chosen",
                json!({
                    "actor": guard_id.0,
                    "tactic": t.tactic.as_str(),
                    "reason": t.reason,
                    "score_attack": t.score_attack,
                    "score_reload": t.score_reload,
                    "score_hold": t.score_hold,
                    "score_search": t.score_search,
                }),
                tactic_parent,
            );
            tactic_chosen_event_id = Some(id);
        }
        // M2 audit pass 5 (2026-05-13): emit one `ai.state_changed` event per
        // transition in spec order. A single tick can produce multiple
        // transitions (e.g. Idle → Alert via heard_shot, then Alert → Engaged
        // via target_acquired after aim_settle elapses on the same tick).
        for s in &report.state_changes {
            // M2 audit pass 7 (2026-05-13): spec literal payload uses
            // `from`/`to`/`reason` (matching the JSON schema). Emit both the
            // schema-required names AND the legacy `previous`/`next`/`cause`
            // alias so in-flight bundles continue to parse.
            let event_id = self.recorder.record(
                tick,
                sim_time_ms,
                "ai",
                "state_changed",
                json!({
                    "actor_id": guard_id.0,
                    "actor": guard_id.0,
                    "from": s.previous.as_str(),
                    "to": s.next.as_str(),
                    "reason": s.cause,
                    "previous": s.previous.as_str(),
                    "next": s.next.as_str(),
                    "cause": s.cause,
                }),
                last_perception_signal_id.clone(),
            );
            // **M4 § ai cause chains**: track most-recent state_changed
            // per actor so subsequent tactic_chosen events (without a
            // fresh perception signal) can chain to it.
            if let Ok(mut st) = self.state.write() {
                st.last_ai_state_changed_by_actor.insert(guard_id, event_id);
            }
        }
        // M2 audit pass 7 (2026-05-13): stash the most recent state-change
        // cause onto the guard so the --ai-debug label can render
        // "ALERT: heard shot" (reason) rather than the chosen tactic.
        if let Some(last) = report.state_changes.last() {
            if let Ok(mut s) = self.state.write() {
                if let Some(guard) = s.reactive_guards.get_mut(&guard_id) {
                    guard.last_state_change_cause = Some(last.cause.clone());
                }
            }
        }
        // **M1.5 G1**: target_acquired chains to the last perception_signal
        // so M3B can walk acquired → signal → alarm/sight.
        if let Some(t) = &report.target_acquired {
            let acquired_id = self.recorder.record(
                tick,
                sim_time_ms,
                "ai",
                "target_acquired",
                json!({
                    "actor": guard_id.0,
                    "target_actor": t.target_actor,
                    "via": t.via,
                }),
                last_perception_signal_id.clone(),
            );
            // **M9 § Reactive guard targeting + path reaction (DR-008
            // utility scoring)**: wire `cf_ai::target_selection::score_all`
            // into the production target-acquisition path. The scorer
            // ranks every candidate (player + reactor) by
            // `score = w_proximity * (1/distance) + w_los * has_los +
            // w_threat * is_player + w_value * is_high_value_static`.
            // Payload exposes the full candidates vec + chosen + reason
            // so M10's death-recap can render "player scored higher than
            // reactor" / "reactor scored higher than player".
            let scored_payload = self.compute_target_scored_payload(guard_id, t.target_actor);
            self.recorder.record(
                tick,
                sim_time_ms,
                "ai",
                "target_scored",
                scored_payload,
                Some(acquired_id),
            );
        }
        if let Some(t) = &report.target_lost {
            self.recorder.record(
                tick,
                sim_time_ms,
                "ai",
                "target_lost",
                json!({
                    "actor": guard_id.0,
                    "target_actor": t.target_actor,
                    "reason": t.reason,
                }),
                None,
            );
        }
        // **M14 audit pass 3 (GAP-M7-05 LOW fix)**: M7 spec § Personality
        // traits — paranoid bots occasionally fire `ai.target_acquired`
        // with no real target. Only fires when:
        //   - The bot has the Paranoid trait
        //   - No real target was acquired this tick
        //   - A seeded RNG roll is below the spec-baseline 2% probability
        // The synthetic event carries `reason="false_sighting"` so the
        // M10 cause-chain viewer can distinguish phantom from real
        // acquisitions.
        if report.target_acquired.is_none() {
            let has_paranoid = self
                .state
                .read()
                .ok()
                .map(|s| {
                    s.m7_ai_world
                        .bots
                        .get(&guard_id)
                        .map(|b| {
                            b.personality
                                .traits
                                .iter()
                                .any(|t| matches!(t, cf_ai::PersonalityTrait::Paranoid))
                        })
                        .unwrap_or(false)
                })
                .unwrap_or(false);
            if has_paranoid {
                let rng_roll = if let Ok(mut s) = self.state.write() {
                    (s.rng.next_u64() as f64 / u64::MAX as f64) as f32
                } else {
                    1.0
                };
                if rng_roll < 0.02 {
                    let _ = self.recorder.record(
                        tick,
                        sim_time_ms,
                        "ai",
                        "target_acquired",
                        json!({
                            "actor": guard_id.0,
                            "target_actor": 0u64,
                            "via": "false_sighting",
                            "reason": "paranoid_trait",
                            "synthetic": true,
                        }),
                        None,
                    );
                }
            }
        }
        // **M1.5 G4**: missed_shot_reason fires per miss to give the replay
        // viewer a stable vocabulary of why a guard's shot didn't connect.
        if let Some(reason) = &report.missed_shot_reason {
            self.recorder.record(
                tick,
                sim_time_ms,
                "ai",
                "missed_shot_reason",
                json!({
                    "actor": guard_id.0,
                    "reason": reason.as_str(),
                }),
                None,
            );
        }
        // **M1.5 G5**: stuck_state_changed + recovery_action.
        //
        // M2 audit pass 7 (2026-05-13): spec literal payload requires
        // `stuck_time_ticks` + `blocker_id` + `old_state` + `new_state`
        // (with values e.g. "engaged"→"engaged_stuck"). Keep legacy
        // `stuck_ticks` + `blocker` aliases for back-compat.
        if let Some(r) = &report.stuck_recovery {
            self.recorder.record(
                tick,
                sim_time_ms,
                "ai",
                "stuck_state_changed",
                json!({
                    "actor_id": guard_id.0,
                    "actor": guard_id.0,
                    "stuck_time_ticks": r.stuck_ticks,
                    "stuck_ticks": r.stuck_ticks,
                    "blocker_id": r.blocker,
                    "blocker": r.blocker,
                    "old_state": "engaged",
                    "new_state": "engaged_stuck",
                }),
                None,
            );
            self.recorder.record(
                tick,
                sim_time_ms,
                "ai",
                "recovery_action",
                json!({
                    "actor": guard_id.0,
                    "action": r.action,
                    "reason": r.reason,
                }),
                None,
            );
        }
        if report.reload_started {
            self.recorder.record(
                tick,
                sim_time_ms,
                "equipment",
                "weapon_reload_started",
                json!({"actor": guard_id.0}),
                None,
            );
        }
        if report.reload_completed {
            self.recorder.record(
                tick,
                sim_time_ms,
                "equipment",
                "weapon_reloaded",
                json!({"actor": guard_id.0}),
                None,
            );
        }
        if report.dry_fire {
            self.recorder.record(
                tick,
                sim_time_ms,
                "equipment",
                "weapon_dry_fire",
                json!({"actor": guard_id.0}),
                None,
            );
        }
        if let Some(fire) = &report.fire {
            // M2 re-audit pass 4 (2026-05-13): chain guard weapon_fired +
            // projectile_spawned to ai.tactic_chosen so the cause chain
            // walks back to the AI's decision.
            let weapon_fired_id = self.recorder.record(
                tick,
                sim_time_ms,
                "equipment",
                "weapon_fired",
                json!({
                    "actor": guard_id.0,
                    "muzzle_origin": fire.muzzle_origin,
                    "miss_threshold": fire.miss_threshold,
                    "miss_roll": fire.miss_roll,
                    "will_miss": fire.will_miss,
                }),
                tactic_chosen_event_id.clone(),
            );
            self.recorder.record(
                tick,
                sim_time_ms,
                "combat",
                "projectile_spawned",
                json!({
                    "owner": guard_id.0,
                    "origin": fire.muzzle_origin,
                    "velocity": fire.velocity,
                    "damage": fire.damage,
                    "lifetime_ticks": fire.lifetime_ticks,
                    "will_miss": fire.will_miss,
                }),
                Some(weapon_fired_id),
            );
        }
    }

}
