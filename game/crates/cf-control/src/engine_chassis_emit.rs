//! Chassis events + eject tick.
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
    pub(crate) fn emit_chassis_events(
        &self,
        tick: Tick,
        sim_time_ms: f64,
        actor: ActorId,
        outcome: &cf_chassis::ZoneDamageOutcome,
        parent: Option<String>,
    ) {
        let zone = outcome.zone.map(|z| z.as_str().to_string()).unwrap_or_default();
        // **M13** § "Hit reactions per body part" — emit the per-zone reaction
        // event when a zone takes damage AND apply the reaction window on the
        // actor.
        if let Some(zone_kind) = outcome.zone {
            let reaction = cf_chassis::HitReaction::for_zone(zone_kind);
            let mut duration_ticks: u32 = 0;
            if let Ok(mut state) = self.state.write() {
                let tick_rate = self.config.tick_rate_hz;
                if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(target) = sim.world.actors.get_mut(&actor) {
                        let applied = target.apply_hit_reaction(zone_kind, tick_rate);
                        duration_ticks = applied.duration_ticks(tick_rate);
                    }
                }
            }
            self.recorder.record(
                tick,
                sim_time_ms,
                "combat",
                "hit_reaction_played",
                json!({
                    "actor": actor.0,
                    "zone": zone,
                    "kind": reaction.kind,
                    "duration_seconds": reaction.duration_seconds,
                    "duration_ticks": duration_ticks,
                    "concussion_dose": reaction.concussion_dose,
                    "drop_chance": reaction.drop_chance,
                    "speed_factor": reaction.speed_factor,
                }),
                parent.clone(),
            );
        }
        // **M13** § "Limb loss functional consequences" — emit per-zone
        // severance event when the zone is destroyed.
        if outcome.zone_destroyed {
            if let Some(zone_kind) = outcome.zone {
                let event_type = match zone_kind {
                    cf_chassis::BodyZone::Head => "head_destroyed",
                    cf_chassis::BodyZone::Torso => "torso_destroyed",
                    cf_chassis::BodyZone::ArmLeft | cf_chassis::BodyZone::ArmRight => "arm_severed",
                    cf_chassis::BodyZone::ForearmLeft | cf_chassis::BodyZone::ForearmRight => "forearm_severed",
                    cf_chassis::BodyZone::HandLeft | cf_chassis::BodyZone::HandRight => "hand_severed",
                    cf_chassis::BodyZone::LegLeft | cf_chassis::BodyZone::LegRight => "leg_severed",
                    cf_chassis::BodyZone::ShinLeft | cf_chassis::BodyZone::ShinRight => "shin_severed",
                    cf_chassis::BodyZone::FootLeft | cf_chassis::BodyZone::FootRight => "foot_severed",
                    cf_chassis::BodyZone::Backpack => "backpack_severed",
                    _ => "zone_destroyed",
                };
                let side = match zone_kind {
                    cf_chassis::BodyZone::ArmLeft
                    | cf_chassis::BodyZone::ForearmLeft
                    | cf_chassis::BodyZone::HandLeft
                    | cf_chassis::BodyZone::LegLeft
                    | cf_chassis::BodyZone::ShinLeft
                    | cf_chassis::BodyZone::FootLeft => "left",
                    cf_chassis::BodyZone::ArmRight
                    | cf_chassis::BodyZone::ForearmRight
                    | cf_chassis::BodyZone::HandRight
                    | cf_chassis::BodyZone::LegRight
                    | cf_chassis::BodyZone::ShinRight
                    | cf_chassis::BodyZone::FootRight => "right",
                    _ => "n/a",
                };
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "actor",
                    event_type,
                    json!({
                        "actor": actor.0,
                        "zone": zone,
                        "side": side,
                        "cause": outcome.cause,
                    }),
                    parent.clone(),
                );
            }
        }
        for ld in &outcome.layer_damage {
            self.recorder.record(
                tick,
                sim_time_ms,
                "chassis",
                "armor_layer_damaged",
                json!({
                    "actor": actor.0,
                    "zone": zone,
                    "layer": ld.layer.as_str(),
                    "damage": ld.damage,
                    "hp_after": ld.hp_after,
                    "breached": ld.breached,
                    "cause": outcome.cause,
                }),
                parent.clone(),
            );
        }
        for glance in &outcome.glances {
            self.recorder.record(
                tick,
                sim_time_ms,
                "chassis",
                "armor_layer_glanced",
                json!({
                    "actor": actor.0,
                    "zone": zone,
                    "layer": glance.layer.as_str(),
                    "absorbed": glance.absorbed,
                    "cause": outcome.cause,
                }),
                parent.clone(),
            );
        }
        if outcome.zone_destroyed {
            self.recorder.record(
                tick,
                sim_time_ms,
                "chassis",
                "armor_zone_destroyed",
                json!({
                    "actor": actor.0,
                    "zone": zone,
                    "cause": outcome.cause,
                }),
                parent.clone(),
            );
        }
        for j in &outcome.joints_severed {
            self.recorder.record(
                tick,
                sim_time_ms,
                "chassis",
                "joint_severed",
                json!({
                    "actor": actor.0,
                    "joint": j,
                    "cause": outcome.cause,
                }),
                parent.clone(),
            );
        }
        for mt in &outcome.module_transitions {
            self.recorder.record(
                tick,
                sim_time_ms,
                "chassis",
                "module_state_changed",
                json!({
                    "actor": actor.0,
                    "module_id": mt.id,
                    "state": mt.state.as_str(),
                    "reason": mt.reason,
                }),
                parent.clone(),
            );
            // **M13** § "Critical chassis modules with full mechanics" — when
            // a module transition crosses into Warning / Failed AND the
            // module has a failure_cascade, drive the corresponding cascade
            // events (`module.ammo_rack_cooking` / `_detonated`,
            // `module.engine_fire`, `module.reactor_pressure_advanced`,
            // `module.optics_impaired`, `module.mobility_reduced`,
            // `module.pilot_direct_hit`). Also surface the
            // `chassis.module_critical` signal for M7 Engineer auto-repair
            // consumers.
            if matches!(
                mt.state,
                cf_chassis::ModuleStateKind::Warning | cf_chassis::ModuleStateKind::Failed
            ) {
                let (cascade, kind_str) = {
                    let s = self.state.read().ok();
                    let module_info = s.as_ref().and_then(|st| {
                        st.actor_state
                            .as_ref()
                            .and_then(|sim| sim.world.actors.get(&actor))
                            .and_then(|a| a.chassis.as_ref())
                            .and_then(|c| c.module(&mt.id))
                            .map(|m| (m.failure_cascade, m.kind, m.ammo_quantity_remaining))
                    });
                    if let Some((cascade, kind, _ammo)) = module_info {
                        (cascade, kind.as_str().to_string())
                    } else {
                        (cf_chassis::FailureCascade::None, String::new())
                    }
                };
                // M7 chassis_module_critical signal — fires when ANY ally
                // chassis module crosses into Warning/Failed.
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "chassis",
                    "module_critical",
                    json!({
                        "actor": actor.0,
                        "module_id": mt.id,
                        "module_kind": kind_str,
                        "state": mt.state.as_str(),
                        "reason": mt.reason,
                    }),
                    parent.clone(),
                );
                // Per-cascade event emission. The actual state mutation
                // (cookoff counters, pressure tier, etc.) happens in the
                // chassis-side critical-module helper invoked below.
                match cascade {
                    cf_chassis::FailureCascade::AmmoCookoff => {
                        let event_type = if mt.state == cf_chassis::ModuleStateKind::Failed {
                            "ammo_rack_detonated"
                        } else {
                            "ammo_rack_cooking"
                        };
                        // Drive the state mutation on the chassis side.
                        let cascade_outcome = if let Ok(mut st) = self.state.write() {
                            st.actor_state.as_mut().and_then(|sim| {
                                sim.world.actors.get_mut(&actor).and_then(|a| {
                                    a.chassis.as_mut().and_then(|c| {
                                        c.apply_critical_module_damage(&mt.id, 0.0, "module_state_advanced")
                                    })
                                })
                            })
                        } else {
                            None
                        };
                        let rounds = cascade_outcome
                            .and_then(|o| {
                                o.cascade_events.iter().find_map(|e| match e {
                                    cf_chassis::CriticalModuleEvent::AmmoCooking { rounds_cooked } => {
                                        Some(*rounds_cooked)
                                    }
                                    cf_chassis::CriticalModuleEvent::AmmoDetonated { rounds_detonated } => {
                                        Some(*rounds_detonated)
                                    }
                                    _ => None,
                                })
                            })
                            .unwrap_or(0);
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "module",
                            event_type,
                            json!({
                                "actor": actor.0,
                                "module_id": mt.id,
                                "rounds": rounds,
                            }),
                            parent.clone(),
                        );
                    }
                    cf_chassis::FailureCascade::EngineFire => {
                        // **M14 audit pass 3 (Finding 5)**: drive the
                        // engine oil/coolant state mutations via the
                        // chassis-side critical helper. Previously the
                        // engine emitted `engine_oil_leak` / `engine_fire`
                        // events without ever calling the helper, so the
                        // chassis state (oil_level, coolant_level) never
                        // actually mutated.
                        let _ = if let Ok(mut st) = self.state.write() {
                            st.actor_state.as_mut().and_then(|sim| {
                                sim.world.actors.get_mut(&actor).and_then(|a| {
                                    a.chassis.as_mut().and_then(|c| {
                                        c.apply_critical_module_damage(&mt.id, 0.0, "module_state_advanced")
                                    })
                                })
                            })
                        } else {
                            None
                        };
                        let event_type = if mt.state == cf_chassis::ModuleStateKind::Failed {
                            "engine_fire"
                        } else {
                            "engine_oil_leak"
                        };
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "module",
                            event_type,
                            json!({
                                "actor": actor.0,
                                "module_id": mt.id,
                            }),
                            parent.clone(),
                        );
                    }
                    cf_chassis::FailureCascade::ReactorOverpressure => {
                        // **M14 audit pass 3 (Finding 5)**: drive the
                        // reactor pressure_state advancement via the
                        // critical helper. The previous code emitted
                        // `reactor_pressure_advanced` without mutating
                        // pressure_state on the chassis module.
                        let _ = if let Ok(mut st) = self.state.write() {
                            st.actor_state.as_mut().and_then(|sim| {
                                sim.world.actors.get_mut(&actor).and_then(|a| {
                                    a.chassis.as_mut().and_then(|c| {
                                        c.apply_critical_module_damage(&mt.id, 0.0, "module_state_advanced")
                                    })
                                })
                            })
                        } else {
                            None
                        };
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "module",
                            "reactor_pressure_advanced",
                            json!({
                                "actor": actor.0,
                                "module_id": mt.id,
                                "state": mt.state.as_str(),
                            }),
                            parent.clone(),
                        );
                    }
                    cf_chassis::FailureCascade::SightImpairment => {
                        // **M14 audit pass 3 (Finding 5)**.
                        let _ = if let Ok(mut st) = self.state.write() {
                            st.actor_state.as_mut().and_then(|sim| {
                                sim.world.actors.get_mut(&actor).and_then(|a| {
                                    a.chassis.as_mut().and_then(|c| {
                                        c.apply_critical_module_damage(&mt.id, 0.0, "module_state_advanced")
                                    })
                                })
                            })
                        } else {
                            None
                        };
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "module",
                            "optics_impaired",
                            json!({
                                "actor": actor.0,
                                "module_id": mt.id,
                                "blind": mt.state == cf_chassis::ModuleStateKind::Failed,
                            }),
                            parent.clone(),
                        );
                    }
                    cf_chassis::FailureCascade::MobilityLoss => {
                        // **M14 audit pass 3 (Finding 5)**.
                        let _ = if let Ok(mut st) = self.state.write() {
                            st.actor_state.as_mut().and_then(|sim| {
                                sim.world.actors.get_mut(&actor).and_then(|a| {
                                    a.chassis.as_mut().and_then(|c| {
                                        c.apply_critical_module_damage(&mt.id, 0.0, "module_state_advanced")
                                    })
                                })
                            })
                        } else {
                            None
                        };
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "module",
                            "mobility_reduced",
                            json!({
                                "actor": actor.0,
                                "module_id": mt.id,
                                "immobile": mt.state == cf_chassis::ModuleStateKind::Failed,
                            }),
                            parent.clone(),
                        );
                    }
                    cf_chassis::FailureCascade::PilotDirectDamage => {
                        // **M14 audit pass 3 (Finding 5)**: critical
                        // helper promotes pilot_state to Injured per
                        // CCCP cockpit penetration rules. Previously the
                        // engine emitted `pilot_direct_hit` without
                        // touching pilot_state on the chassis.
                        let _ = if let Ok(mut st) = self.state.write() {
                            st.actor_state.as_mut().and_then(|sim| {
                                sim.world.actors.get_mut(&actor).and_then(|a| {
                                    a.chassis.as_mut().and_then(|c| {
                                        c.apply_critical_module_damage(&mt.id, 0.0, "module_state_advanced")
                                    })
                                })
                            })
                        } else {
                            None
                        };
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "module",
                            "pilot_direct_hit",
                            json!({
                                "actor": actor.0,
                                "module_id": mt.id,
                            }),
                            parent.clone(),
                        );
                    }
                    cf_chassis::FailureCascade::None => {}
                }
            }
        }
        // Recompute stage + emit transition event if advanced.
        if let Ok(mut state) = self.state.write() {
            if let Some(sim) = state.actor_state.as_mut() {
                if let Some(target_actor) = sim.world.actors.get_mut(&actor) {
                    if let Some(chassis) = target_actor.chassis.as_mut() {
                        let prev = chassis.stage;
                        if let Some(next) = chassis.recompute_stage() {
                            if next != prev {
                                let kind = chassis.kind.as_str().to_string();
                                let spec_id = chassis.spec_id.clone();
                                drop(state);
                                self.recorder.record(
                                    tick,
                                    sim_time_ms,
                                    "chassis",
                                    "stage_changed",
                                    json!({
                                        "actor": actor.0,
                                        "spec_id": spec_id,
                                        "kind": kind,
                                        "previous_stage": prev.as_str(),
                                        "new_stage": next.as_str(),
                                        "cause": outcome.cause,
                                    }),
                                    parent,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// **M5**: tick the chassis eject sequence on every actor. Emits
    /// `chassis.pilot_ejected` / `chassis.pilot_bailed_too_late` /
    /// `chassis.pilot_lost` based on the tick result, plus the matching
    /// stage transition.
    pub(crate) fn tick_chassis_eject_for_all(&self, tick: Tick, sim_time_ms: f64) {
        let mut emits: Vec<(ActorId, &'static str, String)> = Vec::new();
        // **M13** § "Chassis ability slots" / "Boarding transitions" /
        // "Drone allies — fuel" / "Hit reactions per body part" — per-tick
        // updates collected here so we emit events in deterministic order.
        let mut ability_events: Vec<(ActorId, &'static str, cf_chassis::ChassisAbility)> = Vec::new();
        let mut transition_events: Vec<(ActorId, cf_chassis::TransitionCompleted)> = Vec::new();
        let mut drone_fuel_low: Vec<ActorId> = Vec::new();
        let mut hit_reaction_ended: Vec<ActorId> = Vec::new();
        // **M13** § "Pilot eject is a real actor split: the pilot becomes a
        // new ActorId with its own ActorState (no chassis attached). The
        // chassis becomes a wreck (interactable for salvage)." Collect the
        // actor IDs whose pilot ejected this tick so we can perform the
        // split AFTER the borrow ends.
        let mut pilots_to_split: Vec<ActorId> = Vec::new();
        if let Ok(mut state) = self.state.write() {
            if let Some(sim) = state.actor_state.as_mut() {
                let ids: Vec<ActorId> = sim.world.actors.keys().copied().collect();
                for id in ids {
                    let Some(actor) = sim.world.actors.get_mut(&id) else {
                        continue;
                    };
                    // **M13** drone fuel drain (only when chassis = Drone).
                    let drone_tick_rate = actor.chassis.as_ref().map(|c| c.tick_rate_hz).unwrap_or(60);
                    if let Some(drone) = actor.drone_ally.as_mut() {
                        if drone.tick_fuel(drone_tick_rate) {
                            drone_fuel_low.push(id);
                        }
                    }
                    // **M13** hit-reaction timer.
                    if actor.tick_hit_reaction() {
                        hit_reaction_ended.push(id);
                    }
                    let Some(chassis) = actor.chassis.as_mut() else {
                        continue;
                    };
                    // **M13** ability cooldowns + transitions.
                    let ability_outcome = chassis.tick_abilities();
                    for ab in ability_outcome.effects_ended {
                        ability_events.push((id, "effect_ended", ab));
                    }
                    for ab in ability_outcome.cooldowns_expired {
                        ability_events.push((id, "cooldown_expired", ab));
                    }
                    if let Some(t) = chassis.tick_transitions() {
                        transition_events.push((id, t));
                    }
                    if let Some(progress) = chassis.tick_eject() {
                        let stage_after = chassis.stage.as_str().to_string();
                        match progress {
                            cf_chassis::EjectProgress::Ejected => {
                                emits.push((id, "pilot_state_changed", stage_after.clone()));
                                emits.push((id, "pilot_separated", stage_after));
                                pilots_to_split.push(id);
                            }
                            cf_chassis::EjectProgress::BailedTooLate => {
                                emits.push((id, "pilot_bailed_too_late", stage_after));
                            }
                        }
                    }
                }
            }
        }
        // **M13** § "Pilot eject is a real actor split" — perform the split
        // for each ejected pilot. The original ActorId stays at its position
        // as the chassis wreck (not controllable, stage=Wreck). A new ActorId
        // spawns as foot infantry (controllable, no chassis). The player
        // pointer updates to the new pilot. We emit `chassis.actor_split`
        // with both ids so consumers can chain.
        let mut split_events: Vec<(u64, u64)> = Vec::new();
        for wreck_actor_id in pilots_to_split {
            if let Ok(mut state) = self.state.write() {
                let new_pilot_id = if let Some(sim) = state.actor_state.as_mut() {
                    // Resolve the wreck actor's spawn info.
                    let Some(wreck) = sim.world.actors.get(&wreck_actor_id) else {
                        continue;
                    };
                    let team = wreck.team.clone();
                    let position = wreck.position;
                    let was_player = sim.world.player == Some(wreck_actor_id);
                    let was_brain = wreck.is_brain;
                    // Next ActorId = max+1. Stable + deterministic.
                    let next_id = sim
                        .world
                        .actors
                        .keys()
                        .map(|k| k.0)
                        .max()
                        .map(|max| max + 1)
                        .unwrap_or(2);
                    let pilot_id = cf_actor::ActorId(next_id);
                    // Build a foot-infantry pilot actor at the chassis position.
                    let pilot_inventory = cf_actor::Inventory::with_rifle(cf_equipment::RIFLE_M1_DEFAULT_ID);
                    let mut pilot = cf_actor::ActorState::player(pilot_id, &team, position, 100.0, pilot_inventory);
                    pilot.controllable = was_player;
                    if was_brain {
                        pilot.mark_brain(tick.0);
                    }
                    // Mark the wreck as non-controllable + flip the chassis
                    // stage to Wreck so the salvage flow is immediately valid.
                    if let Some(wreck_mut) = sim.world.actors.get_mut(&wreck_actor_id) {
                        wreck_mut.controllable = false;
                        wreck_mut.clear_brain();
                        if let Some(chassis) = wreck_mut.chassis.as_mut() {
                            chassis.stage = cf_chassis::ChassisStage::Wreck;
                            chassis.last_stage_reason = "pilot_ejected_actor_split".to_string();
                        }
                    }
                    sim.world.insert(pilot);
                    if was_player {
                        sim.world.player = Some(pilot_id);
                    }
                    Some(pilot_id.0)
                } else {
                    None
                };
                if let Some(pilot_id_u64) = new_pilot_id {
                    if state.player_actor == Some(wreck_actor_id) {
                        state.player_actor = Some(cf_actor::ActorId(pilot_id_u64));
                    }
                    split_events.push((wreck_actor_id.0, pilot_id_u64));
                }
            }
        }
        for (id, kind, stage) in emits {
            self.recorder.record(
                tick,
                sim_time_ms,
                "chassis",
                kind,
                json!({"actor": id.0, "stage": stage}),
                None,
            );
        }
        // **M13** § "Pilot eject is a real actor split" — emit
        // `chassis.actor_split` so consumers can chain wreck ↔ pilot ids.
        for (wreck_actor, pilot_actor) in split_events {
            self.recorder.record(
                tick,
                sim_time_ms,
                "chassis",
                "actor_split",
                json!({
                    "wreck_actor_id": wreck_actor,
                    "pilot_actor_id": pilot_actor,
                }),
                None,
            );
        }
        for (id, kind, ab) in ability_events {
            self.recorder.record(
                tick,
                sim_time_ms,
                "ability",
                kind,
                json!({"actor": id.0, "ability": ab.as_str()}),
                None,
            );
        }
        // **M14 audit pass 4 (Finding 1)**: drive the PLAYER-side boarding
        // timer + perform the pilot-into-chassis transfer when it expires.
        // This must run BEFORE the transition_events loop so we can drop
        // the target-side `Boarded` echo (the chassis-side `begin_boarding`
        // mirrors the same timer purely for concurrent-board rejection; we
        // emit ONE canonical `actor.boarded` event keyed on the player id
        // matching the original `actor.boarding` event).
        let mut boarding_completed: Vec<(ActorId, ActorId)> = Vec::new();
        if let Ok(mut state) = self.state.write() {
            if let Some(sim) = state.actor_state.as_mut() {
                let actor_ids: Vec<ActorId> = sim.world.actors.keys().copied().collect();
                for id in actor_ids {
                    if let Some(a) = sim.world.actors.get_mut(&id) {
                        if a.boarding_ticks_remaining > 0 {
                            a.boarding_ticks_remaining -= 1;
                            if a.boarding_ticks_remaining == 0 {
                                if let Some(t) = a.pending_boarding_target.take() {
                                    boarding_completed.push((id, ActorId(t)));
                                }
                            }
                        }
                    }
                }
            }
        }
        let boarded_target_ids: std::collections::HashSet<ActorId> =
            boarding_completed.iter().map(|(_, t)| *t).collect();
        transition_events.retain(|(id, side)| match side {
            cf_chassis::TransitionCompleted::Boarded => !boarded_target_ids.contains(id),
            _ => true,
        });
        let mut boarded_emit: Vec<(u64, u64)> = Vec::new();
        let mut boarding_aborted: Vec<(u64, u64, &'static str)> = Vec::new();
        for (player_id, target_id) in boarding_completed {
            if let Ok(mut state) = self.state.write() {
                let (was_brain, player_alive) = state
                    .actor_state
                    .as_ref()
                    .and_then(|sim| {
                        sim.world
                            .actors
                            .get(&player_id)
                            .map(|p| (p.is_brain, p.status != cf_actor::Status::Dead))
                    })
                    .unwrap_or((false, false));
                let target_present = state
                    .actor_state
                    .as_ref()
                    .and_then(|sim| sim.world.actors.get(&target_id))
                    .map(|t| t.chassis.is_some() && t.status != cf_actor::Status::Dead)
                    .unwrap_or(false);
                // **M14 audit pass 4 (Finding 1)**: cancel the transfer if
                // either side became invalid during the 1500ms transition
                // (player died, target chassis destroyed, etc.). Without
                // this guard a dead player could be merged into a missing
                // target, leaving state.player_actor pointing at nothing.
                if !player_alive || !target_present {
                    boarding_aborted.push((
                        player_id.0,
                        target_id.0,
                        if !player_alive {
                            "player_died"
                        } else {
                            "target_unavailable"
                        },
                    ));
                    continue;
                }
                if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(target) = sim.world.actors.get_mut(&target_id) {
                        target.controllable = true;
                        if was_brain {
                            target.mark_brain(tick.0);
                        }
                    }
                    sim.world.actors.remove(&player_id);
                    sim.world.player = Some(target_id);
                }
                state.player_actor = Some(target_id);
                boarded_emit.push((player_id.0, target_id.0));
            }
        }
        for (id, side) in transition_events {
            let event_type = match side {
                cf_chassis::TransitionCompleted::Boarded => "boarded",
                cf_chassis::TransitionCompleted::Disembarked => "disembarked",
            };
            self.recorder
                .record(tick, sim_time_ms, "actor", event_type, json!({"actor": id.0}), None);
        }
        // **M14 audit pass 4 (Finding 1)**: canonical actor.boarded — pair
        // matches the actor.boarding event so consumers can chain via
        // payload.actor.
        for (player_u64, target_u64) in boarded_emit {
            self.recorder.record(
                tick,
                sim_time_ms,
                "actor",
                "boarded",
                json!({"actor": player_u64, "chassis_actor_id": target_u64}),
                None,
            );
        }
        // **M14 audit pass 4 (Finding 1)**: boarding aborted (player died
        // or target was destroyed during the transition).
        for (player_u64, target_u64, reason) in boarding_aborted {
            self.recorder.record(
                tick,
                sim_time_ms,
                "actor",
                "boarding_aborted",
                json!({
                    "actor": player_u64,
                    "chassis_actor_id": target_u64,
                    "reason": reason,
                }),
                None,
            );
        }
        for id in drone_fuel_low {
            self.recorder
                .record(tick, sim_time_ms, "drone", "fuel_low", json!({"actor": id.0}), None);
        }
        for id in hit_reaction_ended {
            self.recorder.record(
                tick,
                sim_time_ms,
                "combat",
                "hit_reaction_ended",
                json!({"actor": id.0}),
                None,
            );
        }
    }

}
