//! Perf samples + render snapshots + intent epoch accessors.
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
    pub fn intent_epoch(&self) -> u64 {
        self.state.read().map(|s| s.intent_epoch).unwrap_or(0)
    }

    pub fn pending_runbundle(&self) -> bool {
        self.state.read().map(|s| s.pending_runbundle).unwrap_or(false)
    }

    pub fn clear_pending_runbundle(&self) {
        if let Ok(mut state) = self.state.write() {
            state.pending_runbundle = false;
        }
    }

    pub fn started_instant(&self) -> Instant {
        self.started_instant
    }

    pub(crate) fn perf_sample(&self) -> PerfSample {
        let state = self.state.read().expect("engine state poisoned");
        let mut samples = state.tick_durations_us.clone();
        let ticks_run = state.clock.tick().0;
        // M3 re-open (2026-05-13): roll up the terrain coalesce samples
        // collected by `flush_pending_dirty_batch`. Surfaces as
        // `summary.json.perf.terrain` per `specs/active/M3.md` § Re-opened gaps.
        let terrain_samples = state.perf_coalesce_samples.clone();
        let total_rects_in = state.perf_coalesce_rects_in_total;
        let total_rects_out = state.perf_coalesce_rects_out_total;
        drop(state);
        let wall_seconds = self.started_instant.elapsed().as_secs_f64();
        let avg_tick_ms = if samples.is_empty() {
            0.0
        } else {
            samples.iter().copied().sum::<u64>() as f64 / samples.len() as f64 / 1000.0
        };
        let p99_tick_ms = if samples.is_empty() {
            0.0
        } else {
            samples.sort_unstable();
            let idx = ((samples.len() as f64 * 0.99) as usize).min(samples.len() - 1);
            samples[idx] as f64 / 1000.0
        };
        let terrain = if terrain_samples.is_empty() {
            None
        } else {
            let batches_emitted = terrain_samples.len() as u64;
            let coalesce_cost_avg = terrain_samples.iter().map(|s| *s as f64).sum::<f64>() / batches_emitted as f64;
            let coalesce_cost_max = terrain_samples.iter().copied().max().unwrap_or(0);
            Some(cf_replay::TerrainPerfBlock {
                coalesce_cost_avg,
                coalesce_cost_max,
                total_rects_in,
                total_rects_out,
                batches_emitted,
            })
        };
        PerfSample {
            avg_frame_ms: avg_tick_ms,
            p99_frame_ms: p99_tick_ms,
            avg_tick_ms,
            p99_tick_ms,
            ticks_run,
            wall_seconds,
            tick_rate_hz: self.config.tick_rate_hz,
            terrain,
        }
    }

    /// Snapshot of the actor world for the Bevy bridge in `cf-app`. Decoupled from
    /// `EngineHandle::snapshot` (which serializes to JSON for the JSON-RPC envelope) so
    /// the bridge doesn't pay JSON serialization cost every frame.
    pub fn actor_render_snapshot(&self) -> ActorRenderSnapshot {
        let state = self.state.read().expect("engine state poisoned");
        let tick = state.clock.tick().0;
        let mut snapshot = ActorRenderSnapshot {
            tick,
            floor_y: 0.0,
            actors: Vec::new(),
            player_actor_id: None,
            player_rifle: None,
            breaches: Vec::new(),
            mission: None,
            extraction_zone: None,
            enemies: Vec::new(),
            reactor: None,
            timer: None,
        };
        for guard in state.reactive_guards.values() {
            let position = state
                .actor_state
                .as_ref()
                .and_then(|sim| sim.world.actors.get(&guard.actor).map(|a| [a.position.x, a.position.y]));
            snapshot.enemies.push(EnemyHudView {
                actor: guard.actor.0,
                state: guard.state.as_str().to_string(),
                last_tactic: guard.last_tactic.as_str().to_string(),
                intent_label: ai_intent_label(guard),
                position,
            });
        }
        if let Some(sim) = state.actor_state.as_ref() {
            snapshot.floor_y = sim.world.floor_y;
            snapshot.player_actor_id = sim.world.player.map(|id| id.0);
            for actor in sim.world.actors.values() {
                let rifle = sim.rifles.get(&actor.id);
                snapshot
                    .actors
                    .push(cf_actor::ActorObservation::from_actor_and_rifle(actor, rifle));
            }
            if let Some(player_id) = sim.world.player {
                let rifle_selected = sim
                    .world
                    .actors
                    .get(&player_id)
                    .is_some_and(|a| a.inventory.selected_item().is_rifle());
                if rifle_selected {
                    if let Some(rifle) = sim.rifles.get(&player_id) {
                        snapshot.player_rifle = Some(crate::engine::RifleHudView {
                            ammo: rifle.ammo_in_mag,
                            capacity: rifle.spec.mag_capacity,
                            fire_cooldown_ticks: rifle.fire_cooldown_ticks,
                            reload_remaining_ticks: rifle.reload_remaining_ticks,
                            reload_total_ticks: rifle.reload_ticks(),
                        });
                    }
                }
            }
        }
        if let Some(world) = state.breach_world.as_ref() {
            for s in world.iter() {
                snapshot.breaches.push(BreachRenderView {
                    id: s.id.clone(),
                    material: s.material.clone(),
                    bbox_min: s.bbox_min,
                    bbox_max: s.bbox_max,
                    hp: s.hp,
                    max_hp: s.max_hp,
                    broken: s.broken,
                    refusal_reason: s.refusal_reason.clone(),
                    dig_range: s.dig_range,
                });
            }
        }
        if let Some(mission) = state.mission.as_ref() {
            snapshot.mission = Some(MissionHudView {
                result: mission.result.as_str().to_string(),
                loss_reason: match &mission.result {
                    cf_mission::MissionResult::Lost { reason } => Some(reason.as_str().to_string()),
                    _ => None,
                },
                elapsed_ticks: mission.elapsed_ticks(tick),
                time_limit_ticks: mission.time_limit_ticks,
                ticks_remaining: mission.ticks_remaining(tick),
                active_objective: mission
                    .active_objective_index()
                    .map(|i| mission.objectives[i].id.clone()),
                last_event_label: mission.last_event_label.clone(),
                show_me_why_event_id: mission.show_me_why_event_id.clone(),
                show_replay_cta: mission.show_replay_cta,
            });
            // Surface the first `ReachZone` so cf-render-2d can draw the extraction zone.
            for obj in &mission.objectives {
                if let cf_mission::ObjectiveKind::ReachZone { min, max } = &obj.kind {
                    snapshot.extraction_zone = Some(ExtractionZoneView {
                        objective_id: obj.id.clone(),
                        min: *min,
                        max: *max,
                        completed: obj.status == cf_mission::ObjectiveStatus::Completed,
                    });
                    break;
                }
            }
            // Matches the `observe.mission.timer` color bands (green > 30s,
            // yellow 10-30s, red < 10s, none when expired or no timer).
            let total_ticks = mission.loss.time_limit_ticks;
            let mission_terminal = mission.result.is_terminal();
            let tick_rate = self.config.tick_rate_hz.max(1) as u64;
            if total_ticks > 0 {
                let remaining_ticks = total_ticks.saturating_sub(tick);
                let remaining_s = (remaining_ticks / tick_rate) as u32;
                let color_state = if mission_terminal {
                    "none".to_string()
                } else if remaining_s > 30 {
                    "green".to_string()
                } else if remaining_s >= 10 {
                    "yellow".to_string()
                } else {
                    "red".to_string()
                };
                snapshot.timer = Some(TimerHudView {
                    remaining_ticks,
                    total_ticks,
                    remaining_seconds: remaining_s,
                    color_state,
                    mission_terminal,
                });
            } else {
                snapshot.timer = Some(TimerHudView {
                    remaining_ticks: 0,
                    total_ticks: 0,
                    remaining_seconds: 0,
                    color_state: "none".to_string(),
                    mission_terminal,
                });
            }
        }
        // HUD snapshot. cf-app maps `pressure_state` to the sprite variant
        // + pressure-line tint; the armor pip layers drive the 3-armor-pip
        // band coloring under the HP bar.
        if let Some(world) = state.reactor_world.as_ref() {
            if let Some(reactor) = world.iter().next() {
                let layers: Vec<ReactorArmorLayerView> = reactor
                    .armor_layers
                    .iter()
                    .map(|l| ReactorArmorLayerView {
                        kind: l.kind.as_str().to_string(),
                        hp: l.hp,
                        max_hp: l.max_hp,
                        hp_percent: l.hp_percent(),
                        hardness: l.hardness,
                    })
                    .collect();
                snapshot.reactor = Some(ReactorHudView {
                    actor_id: reactor.id.clone(),
                    hp: reactor.hp,
                    max_hp: reactor.max_hp,
                    hp_percent: reactor.hp_percent(),
                    pressure_state: reactor.pressure_state.as_str().to_string(),
                    position: reactor.position,
                    mission_critical: reactor.mission_critical,
                    destroyed: reactor.is_destroyed(),
                    heat_signature_k: reactor.heat_signature_k,
                    armor_layers: layers,
                });
            }
        }
        snapshot
    }

    /// avoid contending with cfctl `observe.once` polls (which take read
    /// locks at 15 ms cadence): phase 1 reads overlay mode + dig preview +
    /// anchor under a read lock and detects whether the dirty set is
    /// empty; phase 2 only acquires the write lock when there's at least
    /// one dirty chunk to drain. Without this split, paced 60 Hz scripts
    /// with no active carves were starving cfctl polls because every Bevy
    /// frame was taking a write lock just to read the empty dirty set.
    pub fn terrain_render_snapshot(&self) -> TerrainRenderSnapshot {
        let needs_drain;
        let active;
        let anchor;
        let overlay_mode;
        let dig_preview;
        {
            let read = self.state.read().expect("engine state poisoned");
            overlay_mode = read.material_overlay_mode.clone();
            dig_preview = read.player_actor.and_then(|pid| {
                let actor = read.actor_state.as_ref()?.world.actors.get(&pid)?;
                let terrain = read.chunked_terrain.as_ref()?;
                const DIG_REACH: f32 = 22.0;
                const DIG_RADIUS: f32 = 12.0;
                let aim_x = actor.aim.x;
                let aim_y = actor.aim.y;
                let aim_len = ((aim_x * aim_x) + (aim_y * aim_y)).sqrt().max(0.001);
                let nx = aim_x / aim_len;
                let ny = aim_y / aim_len;
                let probe_x = actor.position.x + nx * DIG_REACH;
                let probe_y = actor.position.y + ny * DIG_REACH;
                let material_id = terrain.material_at_world(probe_x, probe_y);
                let valid = terrain.registry.is_diggable(material_id);
                Some(TerrainDigPreview {
                    position: [probe_x, probe_y],
                    radius: DIG_RADIUS,
                    valid,
                    material_id,
                })
            });
            active = read.chunked_terrain.is_some();
            anchor = read.chunked_terrain.as_ref().map(|t| t.anchor).unwrap_or([0.0, 0.0]);
            needs_drain = read
                .chunked_terrain
                .as_ref()
                .map(|t| t.dirty_chunk_count() > 0)
                .unwrap_or(false);
        }
        if !needs_drain {
            return TerrainRenderSnapshot {
                active,
                anchor,
                overlay_mode,
                dirty_updates: Vec::new(),
                dig_preview,
            };
        }
        let mut state = self.state.write().expect("engine state poisoned");
        let Some(terrain) = state.chunked_terrain.as_mut() else {
            return TerrainRenderSnapshot {
                active,
                anchor,
                overlay_mode,
                dirty_updates: Vec::new(),
                dig_preview,
            };
        };
        let dirty: Vec<cf_terrain::ChunkCoord> = terrain.dirty_chunks().collect();
        let mut updates = Vec::with_capacity(dirty.len());
        for coord in &dirty {
            let pixels = terrain.chunk_pixels(coord.cx, coord.cy);
            // M3 re-open (2026-05-13) fix #6: emit the per-chunk sub-rect
            // instead of the full 256×256 chunk so the renderer can re-upload
            // only the affected pixels. Falls back to the full chunk rect
            // when no sub-rect is available (chunk reclaimed, snapshot
            // restore, or first-time chunk allocation).
            let dirty_rect = terrain
                .take_chunk_dirty_rect(coord.cx, coord.cy)
                .map(|r| [r.min[0], r.min[1], r.max[0], r.max[1]])
                .unwrap_or([0, 0, cf_terrain::CHUNK_SIZE - 1, cf_terrain::CHUNK_SIZE - 1]);
            updates.push(TerrainChunkUpdate {
                cx: coord.cx,
                cy: coord.cy,
                dirty_rect,
                pixels,
            });
        }
        terrain.clear_dirty();
        TerrainRenderSnapshot {
            active,
            anchor,
            overlay_mode,
            dirty_updates: updates,
            dig_preview,
        }
    }

    /// uses this to limit debris spawn requests + report perf health.
    pub fn terrain_render_counters(&self) -> (u64, u64, u64) {
        let state = self.state.read().expect("engine state poisoned");
        (
            state.total_carve_events,
            state.total_debris_spawned,
            state.chunked_terrain.as_ref().map(|t| t.refusal_count).unwrap_or(0),
        )
    }

    pub(crate) fn reject_actor_command(
        &self,
        tick: Tick,
        sim_time_ms: f64,
        state: std::sync::RwLockWriteGuard<'_, EngineMutable>,
        method: &str,
    ) -> CommandResult {
        drop(state);
        self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_rejected",
            json!({
                "method": method,
                "reason": "act_player_unavailable_no_actor_world",
                "fix_hint": "load an M1+ scenario such as m1_actor_range that declares actors[]."
            }),
            None,
        );
        CommandResult::rejected("act_player_unavailable_no_actor_world", tick.0)
    }

}
