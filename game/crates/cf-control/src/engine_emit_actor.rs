//! emit_actor_events — the per-tick actor event emitter.
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
    pub(crate) fn emit_actor_events(&self, tick: Tick, sim_time_ms: f64, intent: &ControlIntent, report: &StepReport) {
        // projectile_spawned events parent to the closer fire event rather
        // than the input.intent_received root. Built during the actor-outcomes
        // loop below and consumed by the spawn loop.
        let mut weapon_fired_event_by_actor: BTreeMap<u64, String> = BTreeMap::new();
        // input.intent_received reflects what was actually consumed (after status gating).
        let player_outcome = report.actor_outcomes.iter().find(|o| o.actor == intent.actor).cloned();
        // whose edge-trigger flag the payload must include
        // (move/aim/fire/reload/jump/dig/select_item/reset/sharp_aim). The
        // prior payload omitted `dig` and `sharp_aim`. `dig` is consumed
        // through the per-actor pending_dig queue (not a flag on
        // ControlIntent), so we surface its edge by checking whether a
        // pending dig is staged for the player this tick.
        let dig_pressed = self.state.read().ok().map(|s| s.pending_dig.is_some()).unwrap_or(false);
        let player_view = json!({
            "actor": intent.actor.0,
            "source": match intent.source {
                IntentSource::Human => "human",
                IntentSource::Cfctl => "cfctl",
                IntentSource::Ai => "ai",
                IntentSource::Replay => "replay",
            },
            "move_x": intent.move_x,
            "aim_x": intent.aim.x,
            "aim_y": intent.aim.y,
            "jump": intent.jump,
            "fire": intent.fire,
            "reload": intent.reload,
            "selected_item": intent.selected_item.map(|s| s.0),
            "reset": intent.reset,
            "sharp_aim": intent.sharp_aim,
            "dig": dig_pressed,
            "applied_move_x": player_outcome.as_ref().map(|o| o.move_x).unwrap_or(0.0),
            "jump_accepted": player_outcome.as_ref().map(|o| o.jump_accepted).unwrap_or(false),
        });
        // Always emit input.intent_received once per tick, even when idle, so replay
        // tooling can confirm input flow.
        let intent_event_id = self
            .recorder
            .record(tick, sim_time_ms, "input", "intent_received", player_view, None);
        // "show_me_why" replay-handoff anchor (DR-023).
        if let Ok(mut s) = self.state.write() {
            s.last_player_input_event_id = Some(intent_event_id.clone());
        }

        for outcome in &report.actor_outcomes {
            // its own actor_status_changed event with cause='dying_dwell_elapsed'
            // and the correct lethal-cause parent_event_id. Skip the generic
            // status-changed emission for that transition to avoid duplicate
            // events + a mis-causally-labelled 'reset'/'unknown' fallback.
            if outcome.previous_status != outcome.new_status && !outcome.dying_dwell_elapsed {
                let status_event_id = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "actor",
                    "actor_status_changed",
                    json!({
                        "actor": outcome.actor.0,
                        "previous_status": outcome.previous_status.as_str(),
                        "new_status": outcome.new_status.as_str(),
                        "cause": status_change_cause(outcome),
                    }),
                    Some(intent_event_id.clone()),
                );
                // M2 re-audit pass 4 (2026-05-13): stash the most-recent
                // player status_changed event id so `mission.mission_resolved`
                // on the PlayerDead loss path can chain to it.
                let is_player = self.state.read().ok().and_then(|s| s.player_actor) == Some(outcome.actor);
                if is_player {
                    if let Ok(mut s) = self.state.write() {
                        s.last_player_status_event_id = Some(status_event_id.clone());
                    }
                }
                // status-changing damage, emit `actor.brain_damaged`. On
                // death (status -> Dead), emit `actor.brain_destroyed` and
                // surface the LossReason::BrainDestroyed via
                // `mission_resolved` at the next tick.
                let brain_marker = self
                    .state
                    .read()
                    .ok()
                    .and_then(|s| {
                        s.actor_state
                            .as_ref()
                            .and_then(|sim| sim.world.actors.get(&outcome.actor).map(|a| a.is_brain))
                    })
                    .unwrap_or(false);
                if brain_marker {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "actor",
                        "brain_damaged",
                        json!({
                            "actor": outcome.actor.0,
                            "previous_status": outcome.previous_status.as_str(),
                            "new_status": outcome.new_status.as_str(),
                        }),
                        Some(status_event_id.clone()),
                    );
                    if outcome.new_status == cf_actor::Status::Dead {
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "actor",
                            "brain_destroyed",
                            json!({
                                "actor": outcome.actor.0,
                                "cause": status_change_cause(outcome),
                            }),
                            Some(status_event_id.clone()),
                        );
                    }
                }
                // a travel-impulse triggered the status change (per spec
                // "And a body-hit sound event is emitted").
                // M12B: route through emit_audio_cue_for_actor so the 4
                // cosmetic spatial-resolve events fire for the body-hit
                // cue's source position.
                if outcome.travel_impulse_damage {
                    self.emit_audio_cue_for_actor(
                        cf_audio::AudioCue::BodyHit {
                            zone: "torso".to_string(),
                            caption: format!("actor {} took travel impulse", outcome.actor.0),
                        },
                        tick,
                        sim_time_ms,
                        outcome.actor,
                    );
                }
            }
            if outcome.reset {
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "actor",
                    "actor_reset",
                    json!({"actor": outcome.actor.0}),
                    Some(intent_event_id.clone()),
                );
            }
            if let Some(slot) = outcome.selection_changed {
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "equipment",
                    "selected_item_changed",
                    json!({"actor": outcome.actor.0, "slot": slot.0}),
                    Some(intent_event_id.clone()),
                );
            }
            if outcome.jump_accepted {
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "actor",
                    "actor_jumped",
                    json!({"actor": outcome.actor.0}),
                    Some(intent_event_id.clone()),
                );
            }
            if outcome.landed_impulse > 0.5 {
                let landed_id = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "actor",
                    "actor_landed",
                    json!({
                        "actor": outcome.actor.0,
                        "impulse": outcome.landed_impulse,
                    }),
                    Some(intent_event_id.clone()),
                );
                // severance" — walk BOTH foot → shin → leg chains via
                // `cf_physics::fall_impulse_chain`. When a joint detaches or
                // gibs from the landing impulse, emit the M14 attachable
                // events + impulse propagation cascade.
                //
                //   (a) Unit fix — `outcome.landed_impulse` is the landing
                //       velocity magnitude in m/s (NOT impulse). Dividing
                //       by mass_kg here turned the input into velocity / kg
                //       which made the downstream `velocity × mass_per_foot`
                //       composition collapse to ~impulse/2 instead of
                //       velocity × mass_per_foot. Now: pass it directly.
                //   (b) Both legs — the impulse splits across two feet
                //       (left + right) when standing. Walk both chains
                //       independently so a fall can sever either leg.
                let mass_kg = self
                    .state
                    .read()
                    .ok()
                    .and_then(|s| s.actor_state.as_ref().map(|sim| sim.world.actors.clone()))
                    .and_then(|actors| actors.get(&outcome.actor).map(|a| a.mass_kg))
                    .unwrap_or(80.0);
                let landing_velocity = outcome.landed_impulse;
                let mass_per_foot = mass_kg.max(1.0) / 2.0;
                let leg_chains: [[&str; 3]; 2] = [
                    ["foot_left", "shin_left", "leg_left"],
                    ["foot_right", "shin_right", "leg_right"],
                ];
                for chain_zones in leg_chains.iter() {
                    let fall_joints: Vec<(String, cf_physics::Joint)> = chain_zones
                        .iter()
                        .map(|z| (z.to_string(), cf_physics::Joint::default_for_zone(z)))
                        .collect();
                    let fall_chain = cf_physics::fall_impulse_chain(landing_velocity, mass_per_foot, &fall_joints);
                    for (zone, eval) in &fall_chain {
                        let _ = self.recorder.record(
                            tick,
                            sim_time_ms,
                            "physics",
                            "impulse_propagated",
                            json!({
                                "actor_id": outcome.actor.0,
                                "from_zone": zone,
                                "to_zone": "parent",
                                "impulse_in": eval.impulse_in,
                                "impulse_absorbed": eval.impulse_absorbed,
                                "impulse_out": eval.impulse_out,
                                "joint_strength": cf_physics::Joint::default_for_zone(zone).joint_strength,
                                "damage_multiplier": cf_physics::Joint::default_for_zone(zone).damage_multiplier,
                                "kind": "fall",
                                "source_event_id": landed_id.clone(),
                            }),
                            Some(landed_id.clone()),
                        );
                        if eval.gib {
                            let _ = self.recorder.record(
                                tick,
                                sim_time_ms,
                                "attachable",
                                "gib_threshold_crossed",
                                json!({
                                    "actor_id": outcome.actor.0,
                                    "attachable_id": zone,
                                    "parent_zone": zone,
                                    "joint_impulse": eval.impulse_in,
                                    "gib_impulse_limit": cf_physics::Joint::default_for_zone(zone).gib_impulse_limit,
                                    "source_event_id": landed_id.clone(),
                                    "cause": "fall_damage",
                                }),
                                Some(landed_id.clone()),
                            );
                        } else if eval.detach {
                            let _ = self.recorder.record(
                                tick,
                                sim_time_ms,
                                "attachable",
                                "detached",
                                json!({
                                    "actor_id": outcome.actor.0,
                                    "attachable_id": zone,
                                    "parent_zone": zone,
                                    "joint_impulse": eval.impulse_in,
                                    "joint_strength": cf_physics::Joint::default_for_zone(zone).joint_strength,
                                    "gib_impulse_limit": cf_physics::Joint::default_for_zone(zone).gib_impulse_limit,
                                    "detach_position": [0.0_f32, 0.0_f32],
                                    "detach_velocity": [0.0_f32, 0.0_f32],
                                    "damage_multiplier": cf_physics::Joint::default_for_zone(zone).damage_multiplier,
                                    "source_event_id": landed_id.clone(),
                                    "cause": "fall_damage",
                                }),
                                Some(landed_id.clone()),
                            );
                        }
                    }
                }
            }
            if outcome.reload_started {
                // M1 re-audit pass 4 (2026-05-13): spec requires
                // `equipment.weapon_reload_started.weapon_id`,
                // `.magazine_id`, `.reload_duration_ticks`.
                let weapon_id = cf_equipment::RIFLE_M1_DEFAULT_ID.to_string();
                // Pre-reload magazine_index is the one being SWAPPED OUT; the
                // post-reload index lands on `weapon_reload_completed`. The
                // engine doesn't introspect the rifle directly here, so we
                // derive the outgoing magazine_id by subtracting one from the
                // post-reload counter on the completion event; the started
                // event uses a "pending" suffix.
                let magazine_id = format!("{weapon_id}:pending");
                let started_id = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "equipment",
                    "weapon_reload_started",
                    json!({
                        "actor": outcome.actor.0,
                        "weapon_id": weapon_id,
                        "magazine_id": magazine_id,
                        "reload_duration_ticks": outcome.reload_ticks_total,
                    }),
                    Some(intent_event_id.clone()),
                );
                if let Ok(mut s) = self.state.write() {
                    s.reload_started_event_id_by_actor.insert(outcome.actor, started_id);
                }
                // M12B: route reload-started through emit_audio_cue_for_actor
                // so the 4 cosmetic spatial-resolve events fire for the
                // reloading actor's position.
                self.emit_audio_cue_for_actor(
                    cf_audio::AudioCue::ReloadStarted {
                        equipment_id: cf_equipment::RIFLE_M1_DEFAULT_ID.to_string(),
                        caption: format!("actor {} reloading", outcome.actor.0),
                    },
                    tick,
                    sim_time_ms,
                    outcome.actor,
                );
            }
            if outcome.reload_completed {
                // M1 re-audit pass 4 (2026-05-13): spec requires the
                // completion event to be named `weapon_reload_completed`
                // AND carry `parent_event_id=<weapon_reload_started>`. We
                // keep `weapon_reloaded` emitted as well for backwards-
                // compat with any existing run bundles.
                let weapon_id = cf_equipment::RIFLE_M1_DEFAULT_ID.to_string();
                let magazine_id = format!("{}:{}", weapon_id, outcome.magazine_index_after_reload);
                let reload_started_parent = self
                    .state
                    .read()
                    .ok()
                    .and_then(|s| s.reload_started_event_id_by_actor.get(&outcome.actor).cloned())
                    .or_else(|| Some(intent_event_id.clone()));
                let payload = json!({
                    "actor": outcome.actor.0,
                    "weapon_id": weapon_id,
                    "magazine_id": magazine_id,
                });
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "equipment",
                    "weapon_reload_completed",
                    payload.clone(),
                    reload_started_parent.clone(),
                );
                // Legacy alias kept for run-bundle backwards-compat.
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "equipment",
                    "weapon_reloaded",
                    payload,
                    reload_started_parent,
                );
                if let Ok(mut s) = self.state.write() {
                    s.reload_started_event_id_by_actor.remove(&outcome.actor);
                }
                // M12B: route reload-completed through the spatial helper.
                self.emit_audio_cue_for_actor(
                    cf_audio::AudioCue::ReloadCompleted {
                        equipment_id: cf_equipment::RIFLE_M1_DEFAULT_ID.to_string(),
                        caption: format!("actor {} reload complete", outcome.actor.0),
                    },
                    tick,
                    sim_time_ms,
                    outcome.actor,
                );
            }
            if outcome.fire_denied_reloading {
                // M1 re-audit pass 4 (2026-05-13): spec requires
                // `control.command_rejected reason="reloading"` when fire
                // is pressed during reload. Surface the rejection so
                // replay viewers can show "REFUSED: reloading" in the
                // last-event ticker.
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({
                        "actor": outcome.actor.0,
                        "method": "act.player.fire",
                        "reason": "reloading",
                    }),
                    Some(intent_event_id.clone()),
                );
            }
            if outcome.fire_denied_by_swap {
                // `actor.action_rejected reason="swap_in_progress"` so the
                // HUD + replay surface the cause.
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "actor",
                    "action_rejected",
                    json!({
                        "actor": outcome.actor.0,
                        "action": "act.player.fire",
                        "reason": "swap_in_progress",
                    }),
                    Some(intent_event_id.clone()),
                );
            }
            if outcome.dry_fire {
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "equipment",
                    "weapon_dry_fire",
                    json!({"actor": outcome.actor.0}),
                    Some(intent_event_id.clone()),
                );
            }
            if outcome.fired {
                let muzzle = outcome.muzzle_origin.unwrap_or(Vec2::ZERO);
                // misfire, surface it on the `equipment.weapon_fired` payload
                // alongside the charge_fraction so replay consumers can render
                // the "MISFIRE" caption + accurate charge bar.
                let charge_info = self
                    .state
                    .read()
                    .ok()
                    .and_then(|s| s.m6_charge_misfires.get(&outcome.actor).copied());
                let mut payload = json!({
                    "actor": outcome.actor.0,
                    "muzzle_origin": [muzzle.x, muzzle.y],
                    "recoil_impulse": outcome.recoil_applied,
                    "loudness_radius": outcome.loudness_radius,
                    "bloom_factor": outcome.bloom_factor,
                    "bipod_deployed": outcome.bipod_deployed_at_fire,
                    "suppressor_attached": outcome.suppressor_attached_at_fire,
                });
                if let Some(info) = charge_info {
                    if let Some(obj) = payload.as_object_mut() {
                        obj.insert("misfire".to_string(), serde_json::Value::Bool(info.misfire));
                        obj.insert(
                            "charge_fraction".to_string(),
                            serde_json::Value::from(info.charge_fraction),
                        );
                    }
                }
                let weapon_fired_id = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "equipment",
                    "weapon_fired",
                    payload,
                    Some(intent_event_id.clone()),
                );
                weapon_fired_event_by_actor.insert(outcome.actor.0, weapon_fired_id.clone());
                // M17 — firing drains a power-survival origin's battery (the
                // per-shot action cost; spec § "Power drain: fire weapon = 0.1 kWh").
                if let Ok(mut s) = self.state.write() {
                    let cost = s.m17_tuning.power_action_cost_fire_kwh;
                    if let Some(w) = s.actor_state.as_mut() {
                        if let Some(a) = w.world.actors.get_mut(&outcome.actor) {
                            if a.origin().is_power_survival() && a.resources.power > 0.0 {
                                a.resources.power = (a.resources.power - cost).max(0.0);
                            }
                        }
                    }
                }
                // `equipment.shell_ejected` for the casing on each fire.
                if let Some(popped) = outcome.popped_round.as_ref() {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "equipment",
                        "magazine_changed",
                        json!({
                            "actor": outcome.actor.0,
                            "remaining": popped.remaining_in_mag,
                            "round_kind": popped.round_kind.as_str(),
                        }),
                        Some(weapon_fired_id.clone()),
                    );
                }
                if let Some(shell) = outcome.shell_ejection.as_ref() {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "equipment",
                        "shell_ejected",
                        json!({
                            "actor": outcome.actor.0,
                            "shell_id": shell.shell_id,
                            "kind": shell.kind.as_str(),
                            "origin": [shell.origin_x, shell.origin_y],
                            "velocity": [shell.velocity_x, shell.velocity_y],
                            "lifetime_seconds": shell.lifetime_seconds,
                        }),
                        Some(weapon_fired_id.clone()),
                    );
                }
                self.emit_audio_cue(
                    cf_audio::AudioCue::WeaponFired {
                        equipment_id: cf_equipment::RIFLE_M1_DEFAULT_ID.to_string(),
                        caption: format!("actor {} fires rifle", outcome.actor.0),
                    },
                    tick,
                );
                // replay events (audio.spatial_resolved / reverb_applied /
                // occluded / doppler_shifted) so the replay verifier sees
                // the same event stream across two engines with identical
                // seed. Pure math; no Bevy, no DSP.
                self.emit_m12b_spatial_resolve(
                    tick,
                    sim_time_ms,
                    "weapon_fired",
                    [muzzle.x, muzzle.y],
                    // Source velocity: shooter is roughly stationary at fire
                    // time; full velocity surfaces when projectile carries
                    // its own emission.
                    [0.0, 0.0],
                    cf_audio::Medium::Air,
                    &[],
                    cf_audio::ReverbProfile::open_outdoor(),
                    None,
                );
                // M1: acoustic noise alarm (CCCP HDFirearm.cpp:948 — registered
                // alarm event consumed by M1.5+ AI perception within the radius).
                if outcome.loudness_radius > 0.0 {
                    // `equipment.alarm_registered` event id and stage it
                    // alongside the AlarmInput so the next-tick AI loop
                    // can thread it through `PerceptionSignal.alarm_event_id`,
                    // which the engine emits as `ai.perception_signal.parent_event_id`.
                    // includes `source_id` (the equipment preset id) and
                    // `pos` (= muzzle position). Keep existing aliases for
                    // back-compat.
                    // by [`SUPPRESSOR_LOUDNESS_FACTOR`] inside
                    // `cf-actor::sim::fire_actor`. Surface the suppressed
                    // flag so replay consumers can render the "suppressed"
                    // badge without re-deriving the factor.
                    let alarm_event_id = self.recorder.record(
                        tick,
                        sim_time_ms,
                        "equipment",
                        "alarm_registered",
                        json!({
                            "actor": outcome.actor.0,
                            "source_id": cf_equipment::RIFLE_M1_DEFAULT_ID,
                            "pos": [muzzle.x, muzzle.y],
                            "muzzle_origin": [muzzle.x, muzzle.y],
                            "loudness_radius": outcome.loudness_radius,
                            "loudness": outcome.loudness_radius,
                            "suppressed": outcome.suppressor_attached_at_fire,
                            "cause": "weapon_fired",
                        }),
                        Some(weapon_fired_id.clone()),
                    );
                    // so guards inside the hearing_radius react ≤1 tick
                    // after the fire event.
                    if let Ok(mut s) = self.state.write() {
                        s.pending_alarms_staging.push(cf_ai::AlarmInput {
                            source_actor: outcome.actor.0,
                            source_position: [muzzle.x, muzzle.y],
                            loudness_radius: outcome.loudness_radius,
                            alarm_event_id: Some(alarm_event_id),
                        });
                    }
                }
                // M1: camera punch / hit-stop forward-hooks for DR-055 game feel.
                // The renderer reads these to apply screen shake and brief
                // freeze-frame on critical hits. The events fire at the surface
                // boundary; full juice lands at M5+.
                // visual juice (see determinism-island-contract.md). Flag
                // cosmetic so the determinism island excludes it AND the
                // recorder drops it first under backpressure.
                self.recorder.record_cosmetic(
                    tick,
                    sim_time_ms,
                    "ux",
                    "camera_punch_requested",
                    json!({
                        "actor": outcome.actor.0,
                        "magnitude": outcome.recoil_applied,
                    }),
                    Some(weapon_fired_id.clone()),
                );
            }
            // M1: sharp-aim invalidation surface (CCCP AHuman.cpp:1779).
            if let Some(reason) = outcome.sharp_aim_invalidation_reason.as_ref() {
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "actor",
                    "sharp_aim_invalidated",
                    json!({
                        "actor": outcome.actor.0,
                        "reason": reason,
                    }),
                    Some(intent_event_id.clone()),
                );
            }
            // M1: knockdown surface — physics authority handover (animation <-> ragdoll).
            if outcome.knockdown_started {
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "physics",
                    "authority_changed",
                    json!({
                        "actor": outcome.actor.0,
                        "from": "animation",
                        "to": "ragdoll",
                        "cause": "knockdown",
                    }),
                    Some(intent_event_id.clone()),
                );
            }
            if outcome.knockdown_recovered {
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "physics",
                    "authority_changed",
                    json!({
                        "actor": outcome.actor.0,
                        "from": "ragdoll",
                        "to": "animation",
                        "cause": "knockdown_recovered",
                    }),
                    Some(intent_event_id.clone()),
                );
            }
            // M1: DYING entry → inventory drop (CCCP Actor.cpp:1215).
            if outcome.entered_dying {
                // Gap C3: parent the DYING status change to the latched
                // lethal cause (projectile_hit) when available, else fall
                // back to intent_event_id.
                let dying_parent = outcome
                    .lethal_cause_event_id
                    .clone()
                    .unwrap_or_else(|| intent_event_id.clone());
                let dying_event_id = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "actor",
                    "actor_status_changed",
                    json!({
                        "actor": outcome.actor.0,
                        "previous_status": outcome.previous_status.as_str(),
                        "new_status": "dying",
                        "cause": "lethal_damage",
                    }),
                    Some(dying_parent),
                );
                // the actor's body to physical-debris authority. Emit
                // `physics.ragdoll_activated` carrying the reduced_motion
                // gating so the renderer can skip the animation while
                // the sim state still steps. Setting `reduced_motion_skip`
                // does not alter determinism; it's a cosmetic-renderer
                // signal only.
                let (rd_pos, rd_vel, rd_mass, reduced_motion) = self
                    .state
                    .read()
                    .ok()
                    .and_then(|s| {
                        let sim = s.actor_state.as_ref()?;
                        let actor = sim.world.actors.get(&outcome.actor)?;
                        Some((
                            [actor.position.x, actor.position.y],
                            [actor.velocity.x, actor.velocity.y],
                            actor.mass_kg,
                            s.settings.reduced_motion,
                        ))
                    })
                    .unwrap_or(([0.0, 0.0], [0.0, 0.0], 80.0, false));
                let _ = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "physics",
                    "ragdoll_activated",
                    json!({
                        "actor_id": outcome.actor.0,
                        "status": "dying",
                        "position": rd_pos,
                        "velocity": rd_vel,
                        "mass_kg": rd_mass,
                        "reduced_motion_skip": reduced_motion,
                        "source_event_id": dying_event_id.clone(),
                    }),
                    Some(dying_event_id.clone()),
                );
                // event id as the player's "last status changed" anchor.
                // `mission.mission_resolved` on the PlayerDead loss path
                // uses this so the cause chain walks
                // `mission_resolved → status_changed(dying) → projectile_hit
                // → weapon_fired → tactic_chosen → perception_signal`.
                // The generic status path's `last_player_status_event_id`
                // anchor (parent=intent) is too shallow for the spec
                // chain — we OVERWRITE with the dying event id since
                // entered_dying happens AFTER the generic emit each tick.
                let is_player = self.state.read().ok().and_then(|s| s.player_actor) == Some(outcome.actor);
                if is_player {
                    if let Ok(mut s) = self.state.write() {
                        s.last_player_status_event_id = Some(dying_event_id.clone());
                    }
                }
                if let (Some(pos), Some(vel), Some(label)) = (
                    outcome.inventory_drop_position,
                    outcome.inventory_drop_velocity,
                    outcome.inventory_drop_label.as_ref(),
                ) {
                    if label != "empty" {
                        let dropped_event_id = self.recorder.record(
                            tick,
                            sim_time_ms,
                            "actor",
                            "inventory_dropped",
                            json!({
                                "actor": outcome.actor.0,
                                // requires `item_id` (the equipment preset id
                                // like "rifle_m1_default"). Legacy `item_label`
                                // ("rifle") kept as an alias for backwards
                                // compat with any in-flight bundles.
                                "item_id": label,
                                "item_label": label,
                                "hand_position": [pos.x, pos.y],
                                "toss_velocity": [vel.x, vel.y],
                            }),
                            Some(dying_event_id.clone()),
                        );
                        // so subsequent ticks integrate gravity + emit
                        // `actor.inventory_settled` once it comes to rest.
                        // We acquire the state lock briefly here; the lock
                        // ordering matches `dispatch` (state write → recorder
                        // record) so cannot deadlock.
                        if let Ok(mut s) = self.state.write() {
                            if let Some(sim) = s.actor_state.as_mut() {
                                sim.spawn_loose_item(label.clone(), pos, vel, dropped_event_id);
                            }
                        }
                        // M12B: route inventory-drop through emit_audio_cue_for_actor
                        // so the 4 cosmetic spatial-resolve events fire at the
                        // dropping actor's position.
                        self.emit_audio_cue_for_actor(
                            cf_audio::AudioCue::InventoryDropped {
                                item_label: label.clone(),
                                caption: format!("{label} dropped"),
                            },
                            tick,
                            sim_time_ms,
                            outcome.actor,
                        );
                    }
                }
            }
            // M1: DYING dwell elapsed → DEAD (CCCP Actor.cpp:1229).
            if outcome.dying_dwell_elapsed {
                // Gap C3: chain to the latched lethal cause so the M3B viewer
                // can walk DEAD -> DYING -> wound_added -> projectile_hit
                // -> projectile_spawned -> weapon_fired -> input.intent_received.
                let dead_parent = outcome
                    .lethal_cause_event_id
                    .clone()
                    .unwrap_or_else(|| intent_event_id.clone());
                let dead_event_id = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "actor",
                    "actor_status_changed",
                    json!({
                        "actor": outcome.actor.0,
                        "previous_status": "dying",
                        "new_status": "dead",
                        "cause": "dying_dwell_elapsed",
                    }),
                    Some(dead_parent),
                );
                // the actor's ragdoll to "active". The `physics.authority_changed`
                // remains the legacy M1 stub; `physics.ragdoll_activated`
                // here is the M14 dedicated event so consumers can index
                // the ragdoll lifecycle without scanning the generic
                // authority stream.
                let (rd_pos, rd_vel, rd_mass, reduced_motion) = self
                    .state
                    .read()
                    .ok()
                    .and_then(|s| {
                        let sim = s.actor_state.as_ref()?;
                        let actor = sim.world.actors.get(&outcome.actor)?;
                        Some((
                            [actor.position.x, actor.position.y],
                            [actor.velocity.x, actor.velocity.y],
                            actor.mass_kg,
                            s.settings.reduced_motion,
                        ))
                    })
                    .unwrap_or(([0.0, 0.0], [0.0, 0.0], 80.0, false));
                let _ = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "physics",
                    "ragdoll_activated",
                    json!({
                        "actor_id": outcome.actor.0,
                        "status": "dead",
                        "position": rd_pos,
                        "velocity": rd_vel,
                        "mass_kg": rd_mass,
                        "reduced_motion_skip": reduced_motion,
                        "source_event_id": dead_event_id.clone(),
                    }),
                    Some(dead_event_id.clone()),
                );
                let _ = dead_event_id;
            }
        }
        // M1 (Gap C1/C2): each `combat.projectile_spawned` parents to its
        // owning `equipment.weapon_fired` event, captured from the actor-
        // outcomes loop via `weapon_fired_event_by_actor`. The closer
        // cause-chain link is what M3B walks when scrubbing the run bundle.
        // Spawn ids are persisted on `EngineMutable::projectile_spawn_event_ids`
        // so a projectile that hits N ticks later can still parent its hit
        // event to the originating spawn.
        // collision priority queue can compute `distance_traveled` for each
        // multi-actor hit using the originating shot's spawn position.
        let mut spawn_origins_this_tick: BTreeMap<u64, [f32; 2]> = BTreeMap::new();
        let mut spawn_velocities_this_tick: BTreeMap<u64, [f32; 2]> = BTreeMap::new();
        for spawn in &report.spawned_projectiles {
            let parent = weapon_fired_event_by_actor
                .get(&spawn.owner.0)
                .cloned()
                .unwrap_or_else(|| intent_event_id.clone());
            let id = self.recorder.record(
                tick,
                sim_time_ms,
                "combat",
                "projectile_spawned",
                json!({
                    "id": spawn.id,
                    "owner": spawn.owner.0,
                    "origin": [spawn.origin.x, spawn.origin.y],
                    "velocity": [spawn.velocity.x, spawn.velocity.y],
                    "damage": spawn.damage,
                    "is_tracer": spawn.is_tracer,
                    "particle_index": spawn.particle_index,
                    "particle_count": spawn.particle_count,
                }),
                Some(parent),
            );
            spawn_origins_this_tick.insert(spawn.id, [spawn.origin.x, spawn.origin.y]);
            spawn_velocities_this_tick.insert(spawn.id, [spawn.velocity.x, spawn.velocity.y]);
            if let Ok(mut s) = self.state.write() {
                s.projectile_spawn_event_ids.insert(spawn.id, id);
                // resolved N ticks later still routes to the M14C
                // producer pipeline.
                s.projectile_round_kinds.insert(spawn.id, spawn.round_kind);
            }
        }
        // priority queue per projectile from the new HitOutcome fields.
        // The cf-actor sim now collects ALL actors a projectile crosses
        // this tick (not just the closest) and stamps each HitOutcome
        // with the exact entry_t / ray_origin / ray_direction /
        // distance_traveled at the moment of intersection. That data is
        // accurate even for projectiles that spawned on prior ticks —
        // the engine no longer reconstructs lossy estimates.
        let mut swept_priority: BTreeMap<(u64, u64), (u32, u32)> = BTreeMap::new();
        let mut swept_origin: BTreeMap<u64, [f32; 2]> = BTreeMap::new();
        let mut swept_direction: BTreeMap<u64, [f32; 2]> = BTreeMap::new();
        let mut swept_distance: BTreeMap<(u64, u64), f32> = BTreeMap::new();
        let mut swept_entry_t: BTreeMap<(u64, u64), f32> = BTreeMap::new();
        {
            let mut per_projectile: BTreeMap<u64, Vec<&cf_actor::sim::HitOutcome>> = BTreeMap::new();
            for hit in &report.hits {
                per_projectile.entry(hit.projectile_id).or_default().push(hit);
            }
            for (pid, hits_for_proj) in per_projectile {
                let candidates: Vec<cf_physics::SweptHitCandidate> = hits_for_proj
                    .iter()
                    .map(|h| cf_physics::SweptHitCandidate {
                        target_id: h.target.0,
                        entry_t: h.entry_t,
                        distance_traveled: h.distance_traveled,
                        entry_point: [h.hit_position.x, h.hit_position.y],
                        ray_origin: [h.ray_origin.x, h.ray_origin.y],
                        ray_direction: [h.ray_direction.x, h.ray_direction.y],
                    })
                    .collect();
                if let Some(first) = hits_for_proj.first() {
                    swept_origin.insert(pid, [first.ray_origin.x, first.ray_origin.y]);
                    swept_direction.insert(pid, [first.ray_direction.x, first.ray_direction.y]);
                }
                let resolved = cf_physics::prioritize_swept_collisions(candidates);
                for r in resolved {
                    swept_priority.insert((pid, r.target_id), (r.priority_index, r.priority_total));
                    swept_distance.insert((pid, r.target_id), r.distance_traveled);
                    swept_entry_t.insert((pid, r.target_id), r.entry_t);
                }
            }
        }
        // Touch the spawn lookup maps so the compiler doesn't flag them
        // as unused now that we read the canonical metadata off
        // HitOutcome directly.
        let _ = &spawn_origins_this_tick;
        let _ = &spawn_velocities_this_tick;
        self.emit_combat_hits(tick, sim_time_ms, &intent_event_id, intent, report, &swept_priority, &swept_distance, &swept_entry_t, &swept_origin, &swept_direction);
        for expired in &report.expired_projectiles {
            let parent = self
                .state
                .read()
                .ok()
                .and_then(|s| s.projectile_spawn_event_ids.get(&expired.id).cloned())
                .unwrap_or_else(|| intent_event_id.clone());
            // projectile expires by reaching max_range (TTL elapsed), emit
            // `combat.bullet_sharpness_decay` carrying the distance + decay
            // band so replay viewers + AI agents can verify the round
            // attenuation contract. Distance is computed from the persisted
            // origin if available; otherwise we use the last position as
            // proxy (zero distance, expired=true).
            let origin = spawn_origins_this_tick.get(&expired.id).copied();
            let distance = origin
                .map(|o| {
                    let dx = expired.last_position.x - o[0];
                    let dy = expired.last_position.y - o[1];
                    (dx * dx + dy * dy).sqrt()
                })
                .unwrap_or(0.0);
            // M14 baseline: rifle effective 100, max 500. Real per-projectile
            // ranges arrive when ammo specs carry them (M15+).
            let effective_range = 100.0_f32;
            let max_range = 500.0_f32;
            let base_damage = 25.0_f32;
            let outcome = cf_physics::decay_damage(cf_physics::SharpnessInputs {
                distance_traveled: distance,
                effective_range,
                max_range,
                base_damage,
                base_sharpness: 0.8,
            });
            self.recorder.record(
                tick,
                sim_time_ms,
                "combat",
                "bullet_sharpness_decay",
                json!({
                    "projectile_id": expired.id,
                    "owner_id": expired.owner.0,
                    "distance_traveled": distance,
                    "effective_range": effective_range,
                    "base_damage": base_damage,
                    "decayed_damage": outcome.decayed_damage,
                    "sharpness_before": 0.8_f32,
                    "sharpness_after": outcome.decayed_sharpness,
                    "expired": outcome.expired,
                    "source_event_id": parent.clone(),
                }),
                Some(parent.clone()),
            );
            if let Ok(mut s) = self.state.write() {
                s.projectile_spawn_event_ids.remove(&expired.id);
                s.projectile_round_kinds.remove(&expired.id);
            }
            self.recorder.record(
                tick,
                sim_time_ms,
                "combat",
                "projectile_expired",
                json!({
                    "id": expired.id,
                    "owner": expired.owner.0,
                    "last_position": [expired.last_position.x, expired.last_position.y],
                }),
                Some(parent),
            );
        }

        // item that came to rest this tick. parent_event_id walks back to
        // the originating `actor.inventory_dropped` so cf-tools-replay-viewer
        // can render the full chain inventory_dropped → inventory_settled.
        for settled in &report.settled_loose_items {
            self.recorder.record(
                tick,
                sim_time_ms,
                "actor",
                "inventory_settled",
                json!({
                    "loose_item_id": settled.id,
                    "item_label": settled.item_label.clone(),
                    "rest_position": [settled.position.x, settled.position.y],
                }),
                Some(settled.source_event_id.clone()),
            );
            // M12B: inventory_settled is positional but the source is the
            // loose item (not an actor); call the spatial-resolve helper
            // directly with the item's rest position.
            self.emit_audio_cue(
                cf_audio::AudioCue::InventorySettled {
                    item_label: settled.item_label.clone(),
                    caption: format!("{} settled", settled.item_label),
                },
                tick,
            );
            self.emit_m12b_spatial_resolve(
                tick,
                sim_time_ms,
                &format!("inventory_settled.{}", settled.id),
                [settled.position.x, settled.position.y],
                [0.0, 0.0],
                cf_audio::Medium::Air,
                &[],
                cf_audio::ReverbProfile::open_outdoor(),
                None,
            );
        }
    }

}
