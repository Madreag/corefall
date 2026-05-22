//! Periodic snapshot emitters.
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
    pub(crate) fn emit_periodic_snapshot_actor(&self, tick: Tick, sim_time_ms: f64, parent_event_id: Option<String>) {
        let actor_state = self
            .state
            .read()
            .expect("engine state poisoned")
            .actor_state
            .as_ref()
            .cloned();
        let Some(sim) = actor_state else { return };
        for actor in sim.world.actors.values() {
            let inventory_summary: Vec<serde_json::Value> = actor
                .inventory
                .items
                .iter()
                .enumerate()
                .map(|(i, it)| {
                    json!({
                        "slot": i,
                        "label": it.label(),
                        "kind": it.kind_label(),
                    })
                })
                .collect();
            let body_silhouette = json!({
                "placeholder": true,
                "milestone_ready": "M13",
            });
            self.recorder.record(
                tick,
                sim_time_ms,
                "snapshot",
                "snapshot_actor",
                json!({
                    "actor": actor.id.0,
                    "actor_id": actor.id.0,
                    "team": actor.team,
                    "controllable": actor.controllable,
                    "position": [actor.position.x, actor.position.y],
                    "pos": [actor.position.x, actor.position.y],
                    "velocity": [actor.velocity.x, actor.velocity.y],
                    "aim": [actor.aim.x, actor.aim.y],
                    "status": actor.status.as_str(),
                    "stance": actor.stance().as_str(),
                    "hp": actor.hp,
                    "hp_max": actor.hp_max,
                    "max_hp": actor.hp_max,
                    "selected_slot": actor.inventory.selected.0,
                    "kind": "actor",
                    "stability": actor.stability,
                    "stability_recovery_rate": actor.stability_recovery_rate,
                    "sharp_aim_progress": actor.sharp_aim_progress,
                    "recoil_accumulator": actor.recoil_accumulator,
                    "knockdown_ticks_remaining": actor.knockdown_ticks_remaining,
                    "mission_critical": actor.mission_critical,
                    "bloom_factor": actor.bloom_factor,
                    "dying_dwell_ticks_remaining": actor.dying_dwell_ticks_remaining,
                    "mass_kg": actor.mass_kg,
                    "mass": actor.mass_kg,
                    "inventory_summary": inventory_summary,
                    "body_silhouette": body_silhouette,
                    "cadence_source": "periodic_15_ticks",
                }),
                parent_event_id.clone(),
            );
        }
    }

    /// the scene-start payload built in `emit_initial_snapshots` so M10
    /// timeline + M11 reactor-strip widgets keep seeing the M9-enriched
    /// fields (pressure_state, armor_layers, heat_signature_k,
    /// mission_critical, role) at the configured cadence. Spec § cf-actor
    /// "actor.snapshot includes reactor's hp + per-layer hp +
    /// pressure_state + heat_signature_k + position."
    pub(crate) fn emit_periodic_snapshot_reactor(&self, tick: Tick, sim_time_ms: f64, parent_event_id: Option<String>) {
        let reactor_world = self
            .state
            .read()
            .expect("engine state poisoned")
            .reactor_world
            .as_ref()
            .cloned();
        let Some(reactors) = reactor_world else { return };
        for r in reactors.iter() {
            let armor_layers: Vec<serde_json::Value> = r
                .armor_layers
                .iter()
                .map(|l| {
                    json!({
                        "kind": l.kind.as_str(),
                        "hp": l.hp,
                        "max_hp": l.max_hp,
                        "hardness": l.hardness,
                    })
                })
                .collect();
            self.recorder.record(
                tick,
                sim_time_ms,
                "snapshot",
                "snapshot_actor",
                json!({
                    "actor": r.id.clone(),
                    "kind": "reactor",
                    "position": r.position,
                    "half_extents": r.half_extents,
                    "hp": r.hp,
                    "hp_max": r.max_hp,
                    "max_hp": r.max_hp,
                    "hp_percent": r.hp_percent(),
                    "destroyed": r.is_destroyed(),
                    "pressure_state": r.pressure_state.as_str(),
                    "armor_layers": armor_layers,
                    "heat_signature_k": r.heat_signature_k,
                    "mission_critical": r.mission_critical,
                    "role": r.role.clone(),
                    "cadence_source": "periodic_60_ticks",
                }),
                parent_event_id.clone(),
            );
        }
    }

    /// Same payload as the scene-start version.
    pub(crate) fn emit_periodic_snapshot_terrain_summary(&self, tick: Tick, sim_time_ms: f64, parent_event_id: Option<String>) {
        let chunked_terrain = self
            .state
            .read()
            .expect("engine state poisoned")
            .chunked_terrain
            .as_ref()
            .cloned();
        let Some(terrain) = chunked_terrain else { return };
        let snapshot = terrain.snapshot();
        let (total_debris_spawned, total_carve_events) = self
            .state
            .read()
            .ok()
            .map(|s| (s.total_debris_spawned, s.total_carve_events))
            .unwrap_or((0u64, 0u64));
        let integrity_distribution = json!({
            "Pristine": snapshot.material_counts.values().copied().sum::<u64>(),
            "Scratched": 0u64,
            "Cracked": 0u64,
            "Critical": 0u64,
            "Destroyed": snapshot.carve_count,
        });
        let hazard_tile_count: u64 = snapshot
            .material_counts
            .iter()
            .filter(|(name, _)| name.as_str() == "hazard")
            .map(|(_, count)| *count)
            .sum();
        let total_pixels: u64 = snapshot.material_counts.values().copied().sum();
        let average_integrity = if total_pixels > 0 {
            1.0 - (snapshot.carve_count as f64 / total_pixels as f64)
        } else {
            1.0
        };
        self.recorder.record(
            tick,
            sim_time_ms,
            "snapshot",
            "snapshot_terrain_summary",
            json!({
                "tick": tick.0,
                "width_px": snapshot.width_px,
                "height_px": snapshot.height_px,
                "default_material": snapshot.default_material,
                "carve_count": snapshot.carve_count,
                "total_carve_events": total_carve_events,
                "refusal_count": snapshot.refusal_count,
                "material_counts": snapshot.material_counts,
                "allocated_chunks": snapshot.chunks.len(),
                "total_chunks": snapshot.chunks.len(),
                "dirty_chunk_count": snapshot.chunks.len(),
                "total_debris_spawned": total_debris_spawned,
                "integrity_distribution": integrity_distribution,
                "hazard_tile_count": hazard_tile_count,
                "average_integrity": average_integrity,
                "cadence_source": "periodic_1_second",
            }),
            parent_event_id,
        );
    }

    /// **DEBUG-ONLY**: spawn a worker thread that panics at the requested tick if
    /// `config.debug_inject_panic_at_tick` is set. The global panic hook (installed by
    /// `cf_replay::diagnostics::init`) routes the panic into the engine's reporter,
    /// which records `system.panic` + bumps `by_severity.error`. Used to capture M0-008
    /// evidence in real run bundles via `cf-app --debug-inject-panic-at-tick <n>`.
    pub(crate) fn spawn_debug_panic_if_requested(&self) {
        let target_tick = match self.config.debug_inject_panic_at_tick {
            Some(t) => t,
            None => return,
        };
        let tick_dt_ms = 1000.0 / f64::from(self.config.tick_rate_hz.max(1));
        let started = self.started_instant;
        std::thread::spawn(move || {
            let target_ms = (target_tick as f64) * tick_dt_ms;
            loop {
                let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
                if elapsed_ms >= target_ms {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            // The global panic hook (installed by `cf_replay::diagnostics::init`) routes
            // this panic into the engine's reporter, which records `system.panic` at the
            // engine's current tick and bumps `by_severity.error`.
            panic!("DEBUG_INJECTED_PANIC at tick~{target_tick} (cf-app --debug-inject-panic-at-tick {target_tick})");
        });
    }

    pub fn record_setting_snapshot(&self) {
        let state = self.state.read().expect("engine state poisoned");
        let tick = state.clock.tick();
        let sim_time_ms = state.clock.sim_time_ms();
        let settings_value = serde_json::to_value(&state.settings).unwrap_or(serde_json::Value::Null);
        drop(state);
        self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "settings_observed",
            json!({"settings": settings_value}),
            None,
        );
    }

}
