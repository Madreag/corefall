//! Inventory grid mutations + command recorders.
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
    pub(crate) fn record_command_accepted(&self, tick: Tick, sim_time_ms: f64, method: &str, extra: serde_json::Value) {
        let mut payload = json!({"method": method});
        if let Some(o) = extra.as_object() {
            if let Some(p) = payload.as_object_mut() {
                for (k, v) in o {
                    p.insert(k.clone(), v.clone());
                }
            }
        }
        self.recorder
            .record(tick, sim_time_ms, "control", "command_accepted", payload, None);
    }

    /// **M8 helper**: record a `control.command_rejected` envelope log.
    pub(crate) fn record_command_rejected(&self, tick: Tick, sim_time_ms: f64, method: &str, reason: &str) {
        self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_rejected",
            json!({"method": method, "reason": reason}),
            None,
        );
    }

    /// **M8 helper**: record a generic event with no parent reference.
    /// Per mission AGENTS.md § Audit-grep visibility convention, callers
    /// SHOULD prefer the inline `#[rustfmt::skip] let _ = self.recorder.
    /// record(...)` form so the audit grep finds the literal record call
    /// site. This helper stays as a fallback for cases where the inline
    /// form would clash with the surrounding control flow.
    #[allow(dead_code)]
    pub(crate) fn record_event(
        &self,
        tick: Tick,
        sim_time_ms: f64,
        category: &str,
        event_type: &str,
        payload: serde_json::Value,
    ) {
        self.recorder
            .record(tick, sim_time_ms, category, event_type, payload, None);
    }

    pub fn shutdown_requested(&self) -> bool {
        self.state.read().map(|s| s.shutdown_requested).unwrap_or(false)
    }

    /// **M6B**: ensure the actor has an inventory grid + encumbrance
    /// envelope; add a top-level placement of `item_id` × `count` to it
    /// (with `liters_filled` for liquid containers); return
    /// `(total_mass_kg, total_bulk_l, instance_id)`. The instance id is
    /// `0` when the actor is missing.
    pub(crate) fn add_to_inventory_grid_mut(
        &self,
        actor_id: ActorId,
        item_id: &str,
        count: u16,
        liters_filled: f32,
    ) -> (f32, f32, u64) {
        let mut state = match self.state.write() {
            Ok(s) => s,
            Err(_) => return (0.0, 0.0, 0),
        };
        let Some(actor) = state
            .actor_state
            .as_mut()
            .and_then(|sim| sim.world.actors.get_mut(&actor_id))
        else {
            return (0.0, 0.0, 0);
        };
        actor.inventory_grid_attach();
        let Some(grid) = actor.inventory_grid_mut() else {
            return (0.0, 0.0, 0);
        };
        let id = grid.add_top_level(item_id, count, liters_filled);
        let total_mass = grid.total_mass_kg();
        let total_bulk = grid.total_bulk_l();
        // Refresh the encumbrance envelope from the new totals.
        actor.recompute_inventory_encumbrance();
        (total_mass, total_bulk, id)
    }

    /// **M6B**: remove the most-recent top-level placement of `item_id`
    /// from the actor's inventory grid (FIFO order is arbitrary here —
    /// the drop flow doesn't pre-thread an instance id). Returns
    /// `(total_mass_kg, total_bulk_l, removed_instance_id)`; the
    /// instance id is `0` when no matching placement was found.
    pub(crate) fn remove_from_inventory_grid_mut(&self, actor_id: ActorId, item_id: &str) -> (f32, f32, u64) {
        let mut state = match self.state.write() {
            Ok(s) => s,
            Err(_) => return (0.0, 0.0, 0),
        };
        let Some(actor) = state
            .actor_state
            .as_mut()
            .and_then(|sim| sim.world.actors.get_mut(&actor_id))
        else {
            return (0.0, 0.0, 0);
        };
        if actor.inventory_grid.is_none() {
            return (0.0, 0.0, 0);
        }
        let removed_id = {
            let grid = actor.inventory_grid_mut().expect("grid present");
            let id_opt = grid
                .items
                .iter()
                .rev()
                .find(|p| p.item_id == item_id)
                .map(|p| p.instance_id);
            if let Some(id) = id_opt {
                grid.remove_top_level(id);
                id
            } else {
                0
            }
        };
        let grid = actor.inventory_grid().expect("grid present");
        let total_mass = grid.total_mass_kg();
        let total_bulk = grid.total_bulk_l();
        actor.recompute_inventory_encumbrance();
        (total_mass, total_bulk, removed_id)
    }

    /// **M6B**: after pickup / drop / liquid-fill changes the inventory,
    /// recompute the encumbrance envelope + emit
    /// `inventory.encumbrance_threshold_crossed` when the discrete band
    /// transitions. Also enforces the walk-speed-penalty side-effects
    /// (sprint is cancelled when band reaches Heavy).
    pub(crate) fn tick_m6b_encumbrance_after_change(&self, tick: Tick, sim_time_ms: f64, actor_id: ActorId) {
        let mut emit: Option<serde_json::Value> = None;
        if let Ok(mut state) = self.state.write() {
            let prev_band = state.m6b_last_encumbrance_band.get(&actor_id).copied();
            let Some(actor) = state
                .actor_state
                .as_mut()
                .and_then(|sim| sim.world.actors.get_mut(&actor_id))
            else {
                return;
            };
            actor.recompute_inventory_encumbrance();
            let Some(env) = actor.inventory_encumbrance else {
                return;
            };
            // Cancel sprint when actor enters the Heavy band.
            if env.encumbered() && actor.sprint_active {
                actor.sprint_active = false;
                actor.stamina.sprinting = false;
            }
            let new_band = env.band;
            let origin_id = actor.origin_id.clone();
            if Some(new_band) != prev_band {
                state.m6b_last_encumbrance_band.insert(actor_id, new_band);
                emit = Some(json!({
                    "actor": actor_id.0,
                    "from_band": prev_band.map(|b| b.as_str()).unwrap_or("none"),
                    "to_band": new_band.as_str(),
                    "total_carried_kg": env.total_carried_kg,
                    "max_carry_kg": env.max_carry_kg,
                    "carry_ratio": env.carry_ratio(),
                    "walk_speed_multiplier": env.walk_speed_multiplier,
                    "origin_id": origin_id,
                }));
            }
        }
        if let Some(payload) = emit {
            self.recorder.record(
                tick,
                sim_time_ms,
                "inventory",
                "encumbrance_threshold_crossed",
                payload,
                None,
            );
        }
    }

}
