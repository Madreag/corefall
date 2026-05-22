//! apply_tool_effect dispatcher.
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
    pub(crate) fn apply_tool_effect(&self, effect: ToolEffect, tick: Tick, sim_time_ms: f64) {
        use cf_terrain::{material_id_from_name, MATERIAL_REPAIR_FILL};
        match effect.kind {
            ToolEffectKind::Digger => {
                let target = [
                    effect.origin.x + effect.aim.x * 16.0,
                    effect.origin.y + effect.aim.y * 16.0,
                ];
                if let Ok(mut s) = self.state.write() {
                    if let Some(terrain) = s.chunked_terrain.as_mut() {
                        let _ = terrain.try_carve(target, 6.0);
                    }
                }
            }
            ToolEffectKind::Repair => {
                let target = [
                    effect.origin.x + effect.aim.x * 12.0,
                    effect.origin.y + effect.aim.y * 12.0,
                ];
                if let Ok(mut s) = self.state.write() {
                    if let Some(terrain) = s.chunked_terrain.as_mut() {
                        let _ = terrain.try_fill_or_repair(target, 12.0, MATERIAL_REPAIR_FILL);
                    }
                }
            }
            ToolEffectKind::Foam => {
                let mat = material_id_from_name("loose_fill").unwrap_or(0);
                let target = [
                    effect.origin.x + effect.aim.x * 8.0,
                    effect.origin.y + effect.aim.y * 8.0,
                ];
                if let Ok(mut s) = self.state.write() {
                    if let Some(terrain) = s.chunked_terrain.as_mut() {
                        let _ = terrain.try_fill_or_repair(target, 8.0, mat);
                    }
                }
            }
            ToolEffectKind::Concrete => {
                let mat = material_id_from_name("concrete").unwrap_or(0);
                let target = [
                    effect.origin.x + effect.aim.x * 10.0,
                    effect.origin.y + effect.aim.y * 10.0,
                ];
                if let Ok(mut s) = self.state.write() {
                    if let Some(terrain) = s.chunked_terrain.as_mut() {
                        let _ = terrain.try_fill_or_repair(target, 10.0, mat);
                    }
                }
            }
            ToolEffectKind::Welder => {
                let target = [
                    effect.origin.x + effect.aim.x * 4.0,
                    effect.origin.y + effect.aim.y * 4.0,
                ];
                if let Ok(mut s) = self.state.write() {
                    if let Some(terrain) = s.chunked_terrain.as_mut() {
                        let _ = terrain.try_carve(target, 4.0);
                    }
                    if let Some(actor) = s
                        .actor_state
                        .as_mut()
                        .and_then(|sim| sim.world.actors.get_mut(&effect.actor_id))
                    {
                        actor.drill_heat = (actor.drill_heat + cf_equipment::DRILL_HEAT_PER_USE * 0.5).min(1.0);
                    }
                }
            }
            ToolEffectKind::Drill => {
                let target = [
                    effect.origin.x + effect.aim.x * 6.0,
                    effect.origin.y + effect.aim.y * 6.0,
                ];
                let mut overheated = false;
                let mut heat_now = 0.0_f32;
                if let Ok(mut s) = self.state.write() {
                    if let Some(terrain) = s.chunked_terrain.as_mut() {
                        // Drill carves twice as wide as the regular digger.
                        let _ = terrain.try_carve(target, 12.0);
                    }
                    if let Some(actor) = s
                        .actor_state
                        .as_mut()
                        .and_then(|sim| sim.world.actors.get_mut(&effect.actor_id))
                    {
                        actor.drill_heat = (actor.drill_heat + cf_equipment::DRILL_HEAT_PER_USE).min(1.0);
                        heat_now = actor.drill_heat;
                        if actor.drill_heat >= cf_equipment::DRILL_JAM_HEAT_THRESHOLD {
                            overheated = true;
                        }
                    }
                }
                if overheated {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "equipment",
                        "drill_overheated",
                        json!({
                            "actor": effect.actor_id.0,
                            "heat": heat_now,
                            "threshold": cf_equipment::DRILL_JAM_HEAT_THRESHOLD,
                        }),
                        None,
                    );
                }
            }
            ToolEffectKind::MultiTool => {
                let probe_pos = [
                    effect.origin.x + effect.aim.x * 18.0,
                    effect.origin.y + effect.aim.y * 18.0,
                ];
                let material_name = if let Ok(s) = self.state.read() {
                    s.chunked_terrain.as_ref().map(|t| {
                        let mid = t.material_at_world(probe_pos[0], probe_pos[1]);
                        cf_terrain::material_name_from_id(mid).to_string()
                    })
                } else {
                    None
                };
                let affordances: Vec<&'static str> = match material_name.as_deref() {
                    Some("dirt") | Some("loose_fill") => vec!["dig", "fill", "anchor"],
                    Some("concrete") | Some("concrete_soft") => vec!["dig", "anchor"],
                    Some("metal_nohook") => vec!["weld_cut"],
                    Some("anchor") => vec!["anchor"],
                    Some("repair_fill") => vec!["repair"],
                    Some("hazard") => vec!["hazard"],
                    _ => vec!["probe"],
                };
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "equipment",
                    "tool_used",
                    json!({
                        "actor": effect.actor_id.0,
                        "tool": "multi_tool",
                        "probe_position": probe_pos,
                        "probed_material": material_name.unwrap_or_default(),
                        "affordances": affordances,
                    }),
                    None,
                );
            }
            ToolEffectKind::Beacon => {
                if let Ok(mut s) = self.state.write() {
                    s.m6_beacons.push((effect.actor_id, effect.origin));
                }
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "equipment",
                    "beacon_dropped",
                    json!({
                        "actor": effect.actor_id.0,
                        "owner_id": effect.actor_id.0,
                        "position": [effect.origin.x, effect.origin.y],
                    }),
                    None,
                );
            }
            ToolEffectKind::SensorPulse => {
                let reveal_until_tick = tick.0.saturating_add(
                    (cf_equipment::SENSOR_PULSE_REVEAL_SECONDS * self.config.tick_rate_hz as f32) as u64,
                );
                let mut revealed: Vec<u64> = Vec::new();
                if let Ok(mut s) = self.state.write() {
                    let origin = effect.origin;
                    let actor_ids: Vec<ActorId> = s
                        .actor_state
                        .as_ref()
                        .map(|sim| sim.world.actors.keys().copied().collect())
                        .unwrap_or_default();
                    for aid in actor_ids {
                        if aid == effect.actor_id {
                            continue;
                        }
                        if let Some(target) = s.actor_state.as_mut().and_then(|sim| sim.world.actors.get_mut(&aid)) {
                            let dx = target.position.x - origin.x;
                            let dy = target.position.y - origin.y;
                            let dist = (dx * dx + dy * dy).sqrt();
                            if dist <= cf_equipment::SENSOR_PULSE_REVEAL_RADIUS {
                                target.reveal_until_tick = reveal_until_tick;
                                revealed.push(aid.0);
                            }
                        }
                    }
                }
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "equipment",
                    "sensor_pulse_fired",
                    json!({
                        "actor": effect.actor_id.0,
                        "origin": [effect.origin.x, effect.origin.y],
                        "radius": cf_equipment::SENSOR_PULSE_REVEAL_RADIUS,
                        "reveal_until_tick": reveal_until_tick,
                        "reveal_seconds": cf_equipment::SENSOR_PULSE_REVEAL_SECONDS,
                        "revealed_actors": revealed,
                    }),
                    None,
                );
            }
        }
    }

}
