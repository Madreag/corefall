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
        // **M1 Gap C**: collect weapon_fired event_id per actor so subsequent
        // projectile_spawned events parent to the closer fire event rather
        // than the input.intent_received root. Built during the actor-outcomes
        // loop below and consumed by the spawn loop.
        let mut weapon_fired_event_by_actor: BTreeMap<u64, String> = BTreeMap::new();
        // input.intent_received reflects what was actually consumed (after status gating).
        let player_outcome = report.actor_outcomes.iter().find(|o| o.actor == intent.actor).cloned();
        // M1 audit pass 5 (2026-05-13): spec literal lists 9 player actions
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
        // **M1.5**: track latest input event_id for the mission_resolved
        // "show_me_why" replay-handoff anchor (DR-023).
        if let Ok(mut s) = self.state.write() {
            s.last_player_input_event_id = Some(intent_event_id.clone());
        }

        for outcome in &report.actor_outcomes {
            // **M1.5 G8**: the dedicated dying-dwell-elapsed path below emits
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
                // **M13** § "Brain hopping" — when the brain actor takes
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
                // M1 audit pass 6 (2026-05-13): emit BodyHit audio cue when
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
                // **M14** § "Falling damage → leg joint impulse → potential
                // severance" — walk BOTH foot → shin → leg chains via
                // `cf_physics::fall_impulse_chain`. When a joint detaches or
                // gibs from the landing impulse, emit the M14 attachable
                // events + impulse propagation cascade.
                //
                // **M14 audit pass 4 (Finding 2)**:
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
                // **M6**: per spec, fire is locked during weapon swap. Emit
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
                // **M6**: when the M6 fire-mode post-step pass latched a Charge
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
                // **M6**: emit `equipment.magazine_changed` for the pop +
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
                // **M12B** § Per-tick spatial-resolve pass. Emits 4 cosmetic
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
                    // M2 audit pass 5 (2026-05-13): capture the
                    // `equipment.alarm_registered` event id and stage it
                    // alongside the AlarmInput so the next-tick AI loop
                    // can thread it through `PerceptionSignal.alarm_event_id`,
                    // which the engine emits as `ai.perception_signal.parent_event_id`.
                    // M1 audit pass 7 (2026-05-13): spec literal payload
                    // includes `source_id` (the equipment preset id) and
                    // `pos` (= muzzle position). Keep existing aliases for
                    // back-compat.
                    // **M6**: the `loudness_radius` was already multiplied
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
                    // **M1.5 G2**: stage the alarm for next tick's AI loop
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
                // **M4 § Cosmetic event types**: camera punch / shake is
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
                // **M14** § "Ragdoll-on-death" — DYING entry transitions
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
                // M2 audit pass 5 (2026-05-13): capture the entered_dying
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
                                // M1 audit pass 6 (2026-05-13): spec literal
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
                        // **M1 R2 / Gap G1**: spawn a `LooseItem` in the sim
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
                // **M14** § "Ragdoll-on-death" — DEAD transition promotes
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
        // **M14**: track projectile-spawn origins this tick so the swept-
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
                // **M14C** § stash the round kind so a HEAT / APFSDS hit
                // resolved N ticks later still routes to the M14C
                // producer pipeline.
                s.projectile_round_kinds.insert(spawn.id, spawn.round_kind);
            }
        }
        // **M14 audit pass 3 (Findings 3 + 4)**: build the swept-collision
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
        for hit in &report.hits {
            // M1 Gap C2: parent the hit to its originating projectile_spawned
            // event rather than the input.intent_received root, so a M3B
            // viewer can walk hit -> spawn -> weapon_fired -> intent in one chain.
            // Spawns persist on `EngineMutable::projectile_spawn_event_ids`
            // because hits commonly fire ticks after the spawn.
            //
            // **M14** fix: do NOT prune the spawn entry inside the hit loop —
            // a swept-collision shot can produce multiple hits for the same
            // projectile this tick, and every hit's `parent_event_id` must
            // resolve to the same spawn. Pruning happens after the hits loop
            // via `projectiles_resolved_this_tick`.
            let hit_parent = self
                .state
                .read()
                .ok()
                .and_then(|s| s.projectile_spawn_event_ids.get(&hit.projectile_id).cloned())
                .unwrap_or_else(|| intent_event_id.clone());
            let projectile_hit_event_id = self.recorder.record(
                tick,
                sim_time_ms,
                "combat",
                "projectile_hit",
                json!({
                    "projectile_id": hit.projectile_id,
                    "shooter": hit.shooter.0,
                    "target": hit.target.0,
                    "hit_position": [hit.hit_position.x, hit.hit_position.y],
                    "damage": hit.damage,
                    "zone": hit.zone,
                }),
                Some(hit_parent),
            );
            // M14 boundary: cf_trench::damage_route_for(target_cover_state,
            // body_zone) → DamageRoute decides whether this hit checks
            // the parapet material first (ParapetFirst) or routes directly
            // to the actor zone (ActorDirect). M9B owns the pure deriver
            // + the per-segment breastwork HP gate; M14 owns the energy
            // attenuation pass that consumes DamageRoute. Wire the call
            // here when M14's damage pipeline lands.
            // **M14** § "Full swept-collision pipeline" — emit
            // `combat.swept_collision` for every projectile-vs-actor hit
            // with the priority index/total computed from the
            // closer-first ordering built above. The event chains to
            // `combat.projectile_hit` so the cause walker can hop
            // swept_collision → projectile_hit → projectile_spawned →
            // weapon_fired → input.intent_received.
            let (priority_index, priority_total) = swept_priority
                .get(&(hit.projectile_id, hit.target.0))
                .copied()
                .unwrap_or((0u32, 1u32));
            let swept_dist = swept_distance
                .get(&(hit.projectile_id, hit.target.0))
                .copied()
                .unwrap_or(0.0);
            let swept_t = swept_entry_t
                .get(&(hit.projectile_id, hit.target.0))
                .copied()
                .unwrap_or(0.0)
                .clamp(0.0, 1.0);
            let ray_origin = swept_origin
                .get(&hit.projectile_id)
                .copied()
                .unwrap_or([hit.hit_position.x, hit.hit_position.y]);
            let ray_direction = swept_direction.get(&hit.projectile_id).copied().unwrap_or([1.0, 0.0]);
            let (stance_label, facing_label) = self
                .state
                .read()
                .ok()
                .and_then(|s| s.actor_state.as_ref().map(|sim| sim.world.actors.clone()))
                .and_then(|actors| {
                    actors.get(&hit.target).map(|a| {
                        let inputs = cf_actor::StanceInputs {
                            velocity: a.velocity,
                            on_ground: a.on_ground,
                            status: a.status,
                            crouch_active: a.crouch_active,
                            climb_active: a.climb_active,
                            jet_active: a.jet_active,
                            knockdown_ticks_remaining: a.knockdown_ticks_remaining,
                            dying_ticks_remaining: a.dying_dwell_ticks_remaining,
                            ..cf_actor::StanceInputs::default()
                        };
                        let stance = cf_actor::derive_stance(inputs);
                        let facing_label = a.facing.as_str();
                        (stance.as_str().to_string(), facing_label.to_string())
                    })
                })
                .unwrap_or_else(|| ("idle".to_string(), "right".to_string()));
            self.recorder.record(
                tick,
                sim_time_ms,
                "combat",
                "swept_collision",
                json!({
                    "projectile_id": hit.projectile_id,
                    "shooter_id": hit.shooter.0,
                    "target_id": hit.target.0,
                    "priority_index": priority_index,
                    "priority_total": priority_total,
                    "entry_point": [hit.hit_position.x, hit.hit_position.y],
                    "ray_origin": ray_origin,
                    "ray_direction": ray_direction,
                    "entry_t": swept_t,
                    "distance_traveled": swept_dist,
                    "zone": hit.zone,
                    "stance": stance_label,
                    "facing": facing_label,
                    "damage": hit.damage,
                    "energy_remaining": (hit.damage - 1.0).max(0.0),
                    "source_event_id": projectile_hit_event_id.clone(),
                }),
                Some(projectile_hit_event_id.clone()),
            );
            // M12B: route body-hit through the spatial-resolve helper so
            // the 4 cosmetic events fire for the victim's position.
            self.emit_audio_cue_for_actor(
                cf_audio::AudioCue::BodyHit {
                    zone: hit.zone.clone(),
                    caption: format!("body hit ({})", hit.zone),
                },
                tick,
                sim_time_ms,
                hit.target,
            );
            if hit.previous_status != hit.new_status {
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "actor",
                    "actor_status_changed",
                    json!({
                        "actor": hit.target.0,
                        "previous_status": hit.previous_status.as_str(),
                        "new_status": hit.new_status.as_str(),
                        "cause": "projectile_hit",
                        "projectile_event": projectile_hit_event_id,
                    }),
                    Some(projectile_hit_event_id.clone()),
                );
            }
            // Gap C3: when this hit lands the killing blow (target transitions
            // through DYING / DEAD), latch the projectile_hit event id onto
            // the victim's actor state so the dwell-elapsed DEAD event AND
            // the next-tick inventory_dropped event AND the DYING latch all
            // resolve back to this projectile_hit (and from there to
            // weapon_fired -> input.intent_received).
            if matches!(hit.new_status, cf_actor::Status::Dying | cf_actor::Status::Dead) {
                if let Ok(mut s) = self.state.write() {
                    if let Some(sim) = s.actor_state.as_mut() {
                        if let Some(target) = sim.world.actors.get_mut(&hit.target) {
                            target.last_lethal_cause_event_id = Some(projectile_hit_event_id.clone());
                        }
                    }
                }
            }
            // **M8** (Cluster C fix): the projectile-hit lethal edge is the
            // production wiring point for `killcam.played` / `killcam.skipped`.
            // When the projectile transitions the player from a live status
            // into DYING / DEAD, fire `M0Engine::trigger_killcam_on_death`
            // with the killer's actor id so `Settings.killcam_enabled` gates
            // the 3 s killcam playback (or emits `killcam.skipped` when
            // disabled). See `specs/active/M8.md` § "Killcam on player death".
            let is_lethal_transition = matches!(hit.new_status, cf_actor::Status::Dying | cf_actor::Status::Dead)
                && !matches!(hit.previous_status, cf_actor::Status::Dying | cf_actor::Status::Dead);
            if is_lethal_transition {
                let victim_is_player = self.state.read().ok().and_then(|s| s.player_actor) == Some(hit.target);
                if victim_is_player {
                    if let Ok(mut s) = self.state.write() {
                        self.trigger_killcam_on_death(Some(hit.shooter.0), hit.target.0, tick, sim_time_ms, &mut s);
                    }
                }
            }
            // **M14G § VAL-M14G-027 / VAL-CROSS-001 / VAL-M14G-011**:
            // emit a typed `wound.created` for the projectile hit using
            // the cf-physics `classify_gunshot` producer. The legacy
            // `combat.wound_added` placeholder is gone — per VAL-M14G-027
            // the producer must dispatch a `WoundKind`-typed event rather
            // than the legacy generic emit. We still publish an internal
            // parent id so the M9 organ-cascade chain can link back to
            // the typed wound.
            let entry_zone = cf_wound::registry::ZoneId::from(hit.zone.as_str());
            let exited_backstop = hit
                .chassis_outcome
                .as_ref()
                .map(|o| !o.layers_breached.is_empty())
                .unwrap_or(false);
            let exit_zone = match hit.zone.as_str() {
                "torso" | "torso_front" => cf_wound::registry::ZoneId::from("torso_back"),
                "torso_back" => cf_wound::registry::ZoneId::from("torso_front"),
                "chest" | "chest_front" => cf_wound::registry::ZoneId::from("chest_back"),
                other => cf_wound::registry::ZoneId::from(other),
            };
            let severity_in = (hit.damage / 100.0).clamp(0.05, 1.0);
            let severity_out = (severity_in * 0.85).clamp(0.05, 1.0);
            let gunshot = cf_physics::classify_gunshot(
                entry_zone,
                exit_zone,
                severity_in,
                severity_out,
                exited_backstop,
            );
            let mut wound_event_id: Option<String> = None;
            for emit in gunshot {
                let id = self.m14g_emit_wound_created(
                    tick,
                    sim_time_ms,
                    hit.target.0,
                    emit,
                    Some(projectile_hit_event_id.clone()),
                );
                if wound_event_id.is_none() {
                    wound_event_id = id;
                }
            }
            let wound_event_id = wound_event_id.unwrap_or_else(|| projectile_hit_event_id.clone());
            // **M9** § internal.* + concussion.* — fire the deep-damage
            // events from the production hit path. Spec § "Internal organ
            // damage / Internal circuit damage / Concussion bands" requires
            // schemas WITH emission sites. Schemas live in cf-replay/schemas/
            // (M5-locked); producers ladder up here at M9.
            //
            // Routing rule at M9:
            //   - human/organic actor → internal.organ_damaged (per-organ HP
            //     delta keyed off hit.zone)
            //   - robot/mechanical actor → internal.circuit_damaged (per-
            //     circuit HP delta)
            //
            // M14+ refines the per-zone organ graph + per-circuit topology;
            // M9 emits the M5-shaped payload with scalar from/to using the
            // hit damage as proxy for the organ/circuit HP delta.
            //
            // The actor's "is_robot" detection currently has no explicit
            // flag, so M9 uses the convention "robot teams" (team starts
            // with 'r') to surface the producer for both pathways without
            // touching the M1 actor type. The audit verifies M9 SHIPS both
            // emission sites; M14 will fix the routing.
            let target_kind_is_robot = self
                .state
                .read()
                .ok()
                .and_then(|s| s.actor_state.as_ref().map(|sim| sim.world.actors.clone()))
                .and_then(|actors| actors.get(&hit.target).map(|a| a.team.clone()))
                .map(|team| team.eq_ignore_ascii_case("red_robot") || team.eq_ignore_ascii_case("robot"))
                .unwrap_or(false);
            // **M14** § "Per-organ internal damage routing" — replace the
            // M9 organ_id/circuit_id stubs with cf_internal's weighted
            // selection. Hit zone proximity drives the candidate pool;
            // the engine's seeded RNG picks deterministically.
            let graph_kind = if target_kind_is_robot {
                cf_internal::InternalGraphKind::Robot
            } else {
                cf_internal::InternalGraphKind::Humanoid
            };
            let rng_roll = if let Ok(mut s) = self.state.write() {
                (s.rng.next_u64() as f64 / u64::MAX as f64) as f32
            } else {
                0.5
            };
            let decision = cf_internal::route_internal_damage(graph_kind, hit.zone.as_str(), hit.damage, rng_roll);
            // M14 fallback when decision is None (below heavy threshold) —
            // keep emitting the legacy M9 organ/circuit shape so existing
            // consumers (M10 cause-chain walker) still observe per-hit
            // routing. The legacy zone→organ table is M14-aware (organ
            // names match the schema enum) so output is valid.
            let (organ_id, circuit_id, applied_internal_dmg, route_via_m14) = match decision {
                Some(d) => match d.graph_kind {
                    cf_internal::InternalGraphKind::Humanoid => (d.target_id, "cpu", d.applied_damage, true),
                    cf_internal::InternalGraphKind::Robot => ("heart", d.target_id, d.applied_damage, true),
                },
                None => {
                    let o = match hit.zone.as_str() {
                        "head" => "brain",
                        "torso" => "heart",
                        "arm_left" | "left_arm" | "forearm_left" | "hand_left" => "lungs_left",
                        "arm_right" | "right_arm" | "forearm_right" | "hand_right" => "lungs_right",
                        "leg_left" | "left_leg" | "shin_left" | "foot_left" => "kidneys_left",
                        "leg_right" | "right_leg" | "shin_right" | "foot_right" => "kidneys_right",
                        _ => "heart",
                    };
                    let c = match hit.zone.as_str() {
                        "head" => "sensor_array",
                        "torso" => "cpu",
                        _ => "power_core",
                    };
                    (o, c, hit.damage, false)
                }
            };
            let from_hp = 100.0_f32;
            let to_hp = (from_hp - applied_internal_dmg).max(0.0);
            if target_kind_is_robot {
                let circuit_event_id = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "internal",
                    "circuit_damaged",
                    json!({
                        "actor_id": hit.target.0,
                        "circuit_id": circuit_id,
                        "circuit_kind": cf_internal::circuit_kind(circuit_id),
                        "from_hp": from_hp,
                        "to_hp": to_hp,
                        "cause": "kinetic_pierce",
                        "source_hit_event_id": projectile_hit_event_id.clone(),
                        "route_via_m14": route_via_m14,
                    }),
                    Some(wound_event_id.clone()),
                );
                if to_hp <= 0.0 {
                    let circuit_destroyed_id = self.recorder.record(
                        tick,
                        sim_time_ms,
                        "internal",
                        "circuit_destroyed",
                        json!({
                            "actor_id": hit.target.0,
                            "circuit_id": circuit_id,
                            "circuit_kind": cf_internal::circuit_kind(circuit_id),
                            "cause": "kinetic_pierce",
                            "source_hit_event_id": projectile_hit_event_id.clone(),
                        }),
                        Some(circuit_event_id),
                    );
                    // **M14** § "Per-circuit failure cascade applies afflictions".
                    let (affliction_kind, severity) = match circuit_id {
                        "power_core" => ("power_failure", 1.0),
                        "cpu" => ("control_lost", 1.0),
                        "sensor_array" => ("blindness", 0.8),
                        "motor_controller_left_arm" => ("arm_motor_failure_left", 0.8),
                        "motor_controller_right_arm" => ("arm_motor_failure_right", 0.8),
                        "motor_controller_left_leg" => ("leg_motor_failure_left", 0.8),
                        "motor_controller_right_leg" => ("leg_motor_failure_right", 0.8),
                        "hydraulic_pump" => ("hydraulic_failure", 0.7),
                        "coolant_pump" => ("overheating", 0.6),
                        "oil_reservoir" | "fuel_tank" => ("fluid_leak", 0.6),
                        "comm_relay" => ("comm_lost", 0.4),
                        _ => ("circuit_shock", 0.5),
                    };
                    let _ = self.recorder.record(
                        tick,
                        sim_time_ms,
                        "internal",
                        "circuit_failure_cascade",
                        json!({
                            "actor_id": hit.target.0,
                            "circuit_id": circuit_id,
                            "circuit_kind": cf_internal::circuit_kind(circuit_id),
                            "affliction_kind": affliction_kind,
                            "severity": severity,
                            "source_event_id": circuit_destroyed_id.clone(),
                        }),
                        Some(circuit_destroyed_id.clone()),
                    );
                }
            } else {
                let organ_event_id = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "internal",
                    "organ_damaged",
                    json!({
                        "actor_id": hit.target.0,
                        "organ_id": organ_id,
                        "organ_kind": cf_internal::organ_kind(organ_id),
                        "from_hp": from_hp,
                        "to_hp": to_hp,
                        "cause": "kinetic_pierce",
                        "source_hit_event_id": projectile_hit_event_id.clone(),
                        "route_via_m14": route_via_m14,
                    }),
                    Some(wound_event_id.clone()),
                );
                if to_hp <= 0.0 {
                    let organ_destroyed_id = self.recorder.record(
                        tick,
                        sim_time_ms,
                        "internal",
                        "organ_destroyed",
                        json!({
                            "actor_id": hit.target.0,
                            "organ_id": organ_id,
                            "organ_kind": cf_internal::organ_kind(organ_id),
                            "cause": "kinetic_pierce",
                            "source_hit_event_id": projectile_hit_event_id.clone(),
                        }),
                        Some(organ_event_id),
                    );
                    // **M14** § "Per-organ failure cascade applies afflictions".
                    // Each destroyed organ triggers a specific affliction
                    // per CCCP organ-failure-cascade table.
                    let (affliction_kind, severity) = match organ_id {
                        "brain" => ("brain_failure", 1.0),
                        "heart" => ("cardiac_arrest", 1.0),
                        "lungs_left" | "lungs_right" => ("respiratory_failure", 0.8),
                        "liver" => ("toxicosis", 0.6),
                        "kidneys_left" | "kidneys_right" => ("renal_failure", 0.7),
                        "spine" => ("paralysis", 0.9),
                        "stomach" | "intestines" | "pancreas" => ("internal_bleed", 0.6),
                        "eyes_left" | "eyes_right" => ("blindness", 0.5),
                        "ears_left" | "ears_right" => ("deafness", 0.4),
                        _ => ("organ_shock", 0.5),
                    };
                    let _ = self.recorder.record(
                        tick,
                        sim_time_ms,
                        "internal",
                        "organ_failure_cascade",
                        json!({
                            "actor_id": hit.target.0,
                            "organ_id": organ_id,
                            "organ_kind": cf_internal::organ_kind(organ_id),
                            "affliction_kind": affliction_kind,
                            "severity": severity,
                            "source_event_id": organ_destroyed_id.clone(),
                        }),
                        Some(organ_destroyed_id.clone()),
                    );
                }
            }
            // **M9** § Concussion bands — accumulate dose per hit and emit
            // band crossings (Clear → Mild → Moderate → Severe → KO_Imminent
            // → KO) per concussion.band_changed schema. KO threshold (=100)
            // emits ko_threshold_crossed. Dose scales with damage (cap at
            // 100). Only fires for organic actors; robots are exempt.
            if !target_kind_is_robot {
                let new_dose: f32 = {
                    let prev = self
                        .state
                        .read()
                        .ok()
                        .and_then(|s| s.m9_concussion_dose.get(&hit.target).copied())
                        .unwrap_or(0.0);
                    let dose = (prev + hit.damage * 0.6).clamp(0.0, 100.0);
                    if let Ok(mut s) = self.state.write() {
                        s.m9_concussion_dose.insert(hit.target, dose);
                        s.m9_concussion_recovery_lockout_ticks
                            .insert(hit.target, self.config.tick_rate_hz.max(1));
                    }
                    dose
                };
                let new_band = m9_concussion_band_for_dose(new_dose);
                let prev_band = self
                    .state
                    .read()
                    .ok()
                    .and_then(|s| s.m9_concussion_band.get(&hit.target).copied())
                    .unwrap_or("Clear");
                let dose_event_id = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "concussion",
                    "dose_changed",
                    json!({
                        "actor_id": hit.target.0,
                        "from_dose": (new_dose - hit.damage * 0.6).clamp(0.0, 100.0),
                        "to_dose": new_dose,
                        "source_event_id": projectile_hit_event_id.clone(),
                        "origin_id": "Human",
                    }),
                    Some(wound_event_id.clone()),
                );
                if prev_band != new_band {
                    if let Ok(mut s) = self.state.write() {
                        s.m9_concussion_band.insert(hit.target, new_band);
                    }
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "concussion",
                        "band_changed",
                        json!({
                            "actor_id": hit.target.0,
                            "from_band": prev_band,
                            "to_band": new_band,
                            "dose": new_dose,
                        }),
                        Some(dose_event_id.clone()),
                    );
                }
                if (new_dose - 100.0).abs() < f32::EPSILON {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "concussion",
                        "ko_threshold_crossed",
                        json!({
                            "actor_id": hit.target.0,
                            "ko_duration_s": 5.5_f32,
                        }),
                        Some(dose_event_id),
                    );
                }
            }
            // M1: hit-stop request (DR-055 placeholder). Triggers when damage
            // exceeds a critical threshold so the renderer can briefly freeze
            // the frame. Full hit-stop renderer effect lands at M5+ when the
            // damage grammar carries crit info.
            const CRITICAL_DAMAGE_THRESHOLD: f32 = 20.0;
            if hit.damage > CRITICAL_DAMAGE_THRESHOLD {
                // **M4 § Cosmetic event types**: hit-stop is visual juice
                // per determinism-island-contract.md.
                self.recorder.record_cosmetic(
                    tick,
                    sim_time_ms,
                    "ux",
                    "hit_stop_requested",
                    json!({
                        "actor": hit.target.0,
                        "shooter": hit.shooter.0,
                        "damage": hit.damage,
                        "duration_ms": 80,
                    }),
                    Some(projectile_hit_event_id.clone()),
                );
            }
            // **M5**: emit chassis-grade events from the hit outcome.
            if let Some(outcome) = &hit.chassis_outcome {
                self.emit_chassis_events(
                    tick,
                    sim_time_ms,
                    hit.target,
                    outcome,
                    Some(projectile_hit_event_id.clone()),
                );
                // **M14** § "Limb detachment via joint impulse" — when a
                // zone is destroyed by this hit, route the impulse through
                // a `cf_physics::Joint`. If `joint_impulse > joint_strength`
                // emit `attachable.detached`; if `>= gib_impulse_limit`
                // emit `attachable.gib_threshold_crossed` + the spawn data
                // `body.gib_created`. Each child attachable cascades per
                // `cf_actor::default_cascade_chain`.
                if outcome.zone_destroyed {
                    let zone_label = outcome
                        .zone
                        .map(|z| z.as_str().to_string())
                        .unwrap_or_else(|| hit.zone.clone());
                    // Hit damage proxies impulse magnitude for M14
                    // (real impulse routing arrives at M15 when ammo specs
                    // carry kinetic energy). Scale 50× so heavy hits cross
                    // typical joint thresholds.
                    let joint_impulse_magnitude = (hit.damage * 50.0).max(0.0);
                    let joint = cf_physics::Joint::default_for_zone(zone_label.as_str());
                    let eval = cf_physics::evaluate_joint(joint, joint_impulse_magnitude);
                    let origin_kind_label = self
                        .state
                        .read()
                        .ok()
                        .and_then(|s| s.actor_state.as_ref().map(|sim| sim.world.actors.clone()))
                        .and_then(|actors| {
                            actors.get(&hit.target).map(|a| {
                                if target_kind_is_robot {
                                    "robot".to_string()
                                } else {
                                    a.origin_id.clone()
                                }
                            })
                        })
                        .unwrap_or_else(|| "human".to_string());
                    let origin_kind = cf_actor::GibOriginKind::from_str_lossy(origin_kind_label.as_str());
                    if eval.gib {
                        // Emit gib_threshold_crossed + body.gib_created
                        // + per-child cascade events.
                        let gib_threshold_id = self.recorder.record(
                            tick,
                            sim_time_ms,
                            "attachable",
                            "gib_threshold_crossed",
                            json!({
                                "actor_id": hit.target.0,
                                "attachable_id": zone_label,
                                "parent_zone": zone_label,
                                "joint_impulse": eval.impulse_in,
                                "gib_impulse_limit": joint.gib_impulse_limit,
                                "source_event_id": projectile_hit_event_id.clone(),
                                "cause": "kinetic_pierce",
                            }),
                            Some(projectile_hit_event_id.clone()),
                        );
                        let gib_spawn = cf_actor::GibSpawn::default_for_origin(origin_kind);
                        let gib_created_id = self.recorder.record(
                            tick,
                            sim_time_ms,
                            "body",
                            "gib_created",
                            json!({
                                "actor_id": hit.target.0,
                                "parent_zone": zone_label,
                                "gib_particle": gib_spawn.particle,
                                "count": gib_spawn.count,
                                "spread_radians": gib_spawn.spread_radians,
                                "min_velocity": gib_spawn.min_velocity,
                                "max_velocity": gib_spawn.max_velocity,
                                "life_variation": gib_spawn.life_variation,
                                "spread_mode": match gib_spawn.spread_mode {
                                    cf_actor::SpreadMode::SpreadRandom => "SpreadRandom",
                                    cf_actor::SpreadMode::SpreadEven => "SpreadEven",
                                    cf_actor::SpreadMode::SpreadSpiral => "SpreadSpiral",
                                },
                                "inherits_velocity": gib_spawn.inherits_velocity,
                                "inherits_angular_velocity": gib_spawn.inherits_angular_velocity,
                                "ignores_team_hits": gib_spawn.ignores_team_hits,
                                "origin_kind": origin_kind.as_str(),
                                "spawn_position": [hit.hit_position.x, hit.hit_position.y],
                                "source_event_id": gib_threshold_id.clone(),
                            }),
                            Some(gib_threshold_id.clone()),
                        );
                        // Cascade — every child attachable gibs in turn.
                        for child in cf_actor::default_cascade_chain(zone_label.as_str()) {
                            self.recorder.record(
                                tick,
                                sim_time_ms,
                                "body",
                                "gib_cascade_triggered",
                                json!({
                                    "actor_id": hit.target.0,
                                    "parent_zone": zone_label,
                                    "child_zone": *child,
                                    "cascade_depth": 1u32,
                                    "source_event_id": gib_created_id.clone(),
                                    "reason": "parent_gibbed",
                                }),
                                Some(gib_created_id.clone()),
                            );
                        }
                    } else if eval.detach {
                        // Cleanly detached — toss the limb forward at the
                        // residual impulse. detach_velocity ∝ impulse_out
                        // / mass; M14 uses a scaled forward toss.
                        let detach_position = [hit.hit_position.x, hit.hit_position.y];
                        let detach_velocity = [
                            ray_direction[0] * (eval.impulse_out / 80.0).clamp(0.0, 500.0),
                            ray_direction[1] * (eval.impulse_out / 80.0).clamp(0.0, 500.0),
                        ];
                        let detached_event_id = self.recorder.record(
                            tick,
                            sim_time_ms,
                            "attachable",
                            "detached",
                            json!({
                                "actor_id": hit.target.0,
                                "attachable_id": zone_label,
                                "parent_zone": zone_label,
                                "joint_impulse": eval.impulse_in,
                                "joint_strength": joint.joint_strength,
                                "gib_impulse_limit": joint.gib_impulse_limit,
                                "detach_position": detach_position,
                                "detach_velocity": detach_velocity,
                                "damage_multiplier": joint.damage_multiplier,
                                "source_event_id": projectile_hit_event_id.clone(),
                                "cause": "kinetic_pierce",
                            }),
                            Some(projectile_hit_event_id.clone()),
                        );
                        // **M14I**: register the severed limb so the
                        // post-survival pass can promote it to phantom_limb.
                        // Chains phantom_limb.acquired to attachable.detached
                        // so the cause-chain walker traces back to the
                        // projectile hit.
                        self.m14i_record_phantom_limb_with_parent(
                            hit.target.0,
                            zone_label.as_str(),
                            tick,
                            sim_time_ms,
                            Some(detached_event_id),
                        );
                    }
                    // Emit `physics.impulse_propagated` for the joint that
                    // routed this hit — closes the impulse chain back to
                    // the projectile_hit event so the M10 cause walker
                    // resolves swept_collision → impulse_propagated.
                    let _ = self.recorder.record(
                        tick,
                        sim_time_ms,
                        "physics",
                        "impulse_propagated",
                        json!({
                            "actor_id": hit.target.0,
                            "from_zone": zone_label,
                            "to_zone": "parent",
                            "impulse_in": eval.impulse_in,
                            "impulse_absorbed": eval.impulse_absorbed,
                            "impulse_out": eval.impulse_out,
                            "joint_strength": joint.joint_strength,
                            "damage_multiplier": joint.damage_multiplier,
                            "kind": "kinetic",
                            "source_event_id": projectile_hit_event_id.clone(),
                        }),
                        Some(projectile_hit_event_id.clone()),
                    );
                }
                // **M13** § "Limb loss functional consequences" — head/torso
                // destruction is INSTANT DEATH per the CCCP decapitation rule.
                // Forcing `actor.hp = 0` here lets the existing status-change
                // pipeline emit `actor_status_changed → Dead` + brain death
                // events on the next tick.
                if outcome.lethal {
                    if let Ok(mut s) = self.state.write() {
                        if let Some(sim) = s.actor_state.as_mut() {
                            if let Some(target) = sim.world.actors.get_mut(&hit.target) {
                                target.hp = 0.0;
                            }
                        }
                    }
                }
                // **M14** § "Full penetration ray flow" — when armor was
                // breached, traverse the chassis interior modules in ray-
                // order via `cf_physics::traverse_ray`. Emits
                // `armor.penetration_ray_traversed` + per-module damage
                // events + spalling fragments.
                if !outcome.layers_breached.is_empty() {
                    self.emit_m14_penetration_ray(
                        tick,
                        sim_time_ms,
                        hit.target,
                        hit,
                        outcome,
                        ray_direction,
                        Some(projectile_hit_event_id.clone()),
                    );
                }
                // **M14C** § HEAT / APFSDS producer wiring — when the
                // projectile is a tank-grade round, route the impact
                // through `heat_impact_producer` / `apfsds_impact_producer`
                // and emit `armor.era_pre_detonated` (strict ordering)
                // BEFORE `armor.heat_jet_traversed` / `armor.apfsds_long_rod_through`.
                // For HEAT/APFSDS we do NOT require armor breach (a HEAT
                // jet bypasses spaced armor per VAL-M14C-021, and APFSDS
                // can over-penetrate unarmored infantry per VAL-M14C-016);
                // the producer's own gates (5° cone, ERA reduction,
                // standoff curve) decide whether to emit per impact.
                let round_kind_for_hit = self
                    .state
                    .read()
                    .ok()
                    .and_then(|s| s.projectile_round_kinds.get(&hit.projectile_id).copied())
                    .unwrap_or(cf_equipment::RoundKind::Regular);
                if matches!(
                    round_kind_for_hit,
                    cf_equipment::RoundKind::Heat | cf_equipment::RoundKind::Apfsds
                ) {
                    self.emit_m14c_armor_events(
                        tick,
                        sim_time_ms,
                        hit.target,
                        hit,
                        round_kind_for_hit,
                        Some(projectile_hit_event_id.clone()),
                    );
                }
            }
            // **M8** (Cluster E fix): auto-trigger a 100 ms hit-stop pulse on
            // an AP-round hit per spec § "Hit-stop on impact — Given melee
            // hit OR AP round hit". At M8 baseline, an AP-round hit is
            // detected via the closest available signal in the M6-M8
            // codebase: either the M5 chassis armor was actually breached
            // (`chassis_outcome.layers_breached` non-empty — literal AP
            // behavior) OR the projectile's damage cleared the
            // CRITICAL_DAMAGE_THRESHOLD (high-energy round indistinguishable
            // from AP at M8; M13/M14 fills explicit AP-round tiers per
            // `combat_projectile_hit_mo.json::ap_round_tier`). Honors
            // `Settings.hit_stop_enabled` and emits `camera.hit_stop` with
            // `trigger="ap_round_hit"`.
            let pierced_armor = hit
                .chassis_outcome
                .as_ref()
                .map(|c| !c.layers_breached.is_empty())
                .unwrap_or(false);
            let is_ap_round_hit = pierced_armor || hit.damage > CRITICAL_DAMAGE_THRESHOLD;
            let mut ap_hit_stop_payload: Option<serde_json::Value> = None;
            if is_ap_round_hit {
                if let Ok(mut s) = self.state.write() {
                    if s.settings.hit_stop_enabled {
                        cf_camera::trigger_hit_stop(&mut s.camera_state, 100);
                        let applied = s.camera_state.hit_stop_remaining_ms;
                        ap_hit_stop_payload = Some(
                            json!({"duration_ms": applied, "trigger": "ap_round_hit", "actor_id": Some(hit.target.0)}),
                        );
                    }
                }
            }
            if let Some(payload) = ap_hit_stop_payload {
                #[rustfmt::skip]
                let _ = self.recorder.record(tick, sim_time_ms, "camera", "hit_stop", payload, None);
            }
        }
        // **M14** fix: prune the spawn_event_id map AFTER the hits loop so
        // multi-actor swept-collision hits all parent to the same spawn.
        // The pre-M14 code pruned per-hit which dropped the parent ref for
        // every subsequent hit of the same projectile (the priority queue
        // depended on this fix).
        {
            let mut resolved_ids: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
            for hit in &report.hits {
                resolved_ids.insert(hit.projectile_id);
            }
            if !resolved_ids.is_empty() {
                if let Ok(mut s) = self.state.write() {
                    for id in &resolved_ids {
                        s.projectile_spawn_event_ids.remove(id);
                        // **M14C** § drop the projectile's round-kind entry
                        // alongside the spawn-id entry; the projectile is
                        // resolved this tick so future ticks can't read
                        // it again.
                        s.projectile_round_kinds.remove(id);
                    }
                }
            }
        }
        for expired in &report.expired_projectiles {
            let parent = self
                .state
                .read()
                .ok()
                .and_then(|s| s.projectile_spawn_event_ids.get(&expired.id).cloned())
                .unwrap_or_else(|| intent_event_id.clone());
            // **M14** § "Bullet sharpness decay over distance" — when the
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

        // **M1 R2 / Gap G1**: emit `actor.inventory_settled` for every loose
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
