//! tick_m7b_squad + emit_m4b_snapshot_for_tick.
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
    pub(crate) fn tick_m7b_squad(&self, tick: Tick, sim_time_ms: f64) {
        let squad_id = crate::m7b_squad::PLAYER_SQUAD_ID;
        let tick_rate = self.config.tick_rate_hz;
        // 2s reslot cadence — only fires when the squad has an active
        // command (is moving) and the cadence elapsed.
        let reslot = {
            let mut state = match self.state.write() {
                Ok(s) => s,
                Err(_) => return,
            };
            let commander_actor_id = state.player_actor.map(|a| a.0);
            let commander_pos = state
                .player_actor
                .and_then(|pid| state.actor_state.as_ref().and_then(|sim| sim.world.actors.get(&pid)))
                .map(|a| [a.position.x, a.position.y])
                .unwrap_or([0.0, 0.0]);
            state
                .m7b_squad
                .tick_periodic_reslot(squad_id, commander_pos, 0.0, commander_actor_id, tick.0, tick_rate)
        };
        if let Some(out) = reslot {
            self.recorder
                .record(tick, sim_time_ms, "squad", "formation_set", out.formation_payload, None);
            for p in out.assignment_payloads {
                self.recorder
                    .record(tick, sim_time_ms, "squad", "formation_slot_assigned", p, None);
            }
        }

        // Slot-broken detection — emit one event per member whose world
        // position has wandered past `SLOT_BROKEN_THRESHOLD_UNITS` from
        // its assigned slot anchor. The next 2s reslot tick reassigns.
        let slot_broken = {
            let state = match self.state.read() {
                Ok(s) => s,
                Err(_) => return,
            };
            let mut positions: std::collections::BTreeMap<u64, [f32; 2]> = std::collections::BTreeMap::new();
            if let Some(sim) = state.actor_state.as_ref() {
                for (id, actor) in &sim.world.actors {
                    positions.insert(id.0, [actor.position.x, actor.position.y]);
                }
            }
            let cadence_ticks = (cf_ai::squad_state::SLOT_RESLOT_CADENCE_SECONDS * tick_rate.max(1) as f32) as u64;
            let next_solve = state
                .m7b_squad
                .squad(squad_id)
                .map(|s| s.last_solve_tick.saturating_add(cadence_ticks))
                .unwrap_or(tick.0);
            state
                .m7b_squad
                .detect_and_report_broken_slots(squad_id, &positions, next_solve)
        };
        for p in slot_broken {
            self.recorder
                .record(tick, sim_time_ms, "squad", "formation_slot_broken", p, None);
        }
    }

    /// **M4B § "Delta baseline cadence is enforced"** — per-tick snapshot
    /// emitter. Reads the current world state via [`Self::snapshot_world_save`]
    /// and produces either a baseline event (when `tick % cadence == 0`) or
    /// a delta event chained back to the most recent baseline.
    pub(crate) fn emit_m4b_snapshot_for_tick(&self, tick: Tick) {
        let cadence = self.config.delta_baseline_cadence_ticks;
        if cadence == 0 {
            return;
        }
        let sim_time_ms = self.state.read().map(|s| s.clock.sim_time_ms()).unwrap_or(0.0);
        let world = self.snapshot_world_save();
        let state_value = match serde_json::to_value(&world) {
            Ok(v) => v,
            Err(err) => {
                tracing::warn!(target: "cf::ctl::m4b", ?err, "snapshot serialization failed");
                return;
            }
        };
        let is_baseline = tick.0.is_multiple_of(cadence);
        if is_baseline {
            // Emit a baseline. Always emit, even at tick 0, so the chain
            // is anchored from the very first cadence boundary.
            match cf_replay::snapshot_baseline::emit_baseline(
                &self.recorder,
                tick,
                sim_time_ms,
                tick.0,
                state_value.clone(),
                cadence,
            ) {
                Ok(event_id) => {
                    let mut state = self.state.write().expect("engine state poisoned");
                    state.m4b_last_baseline_event_id = Some(event_id);
                    state.m4b_last_baseline_tick = Some(tick.0);
                    state.m4b_previous_snapshot = Some(state_value);
                }
                Err(err) => {
                    tracing::warn!(target: "cf::ctl::m4b", ?err, "baseline emission failed");
                }
            }
        } else {
            // Emit a delta. Skip when there's no baseline yet (shouldn't
            // happen — tick 0 always fires a baseline — but be defensive).
            let (baseline_event_id, baseline_tick, previous) = {
                let state = self.state.read().expect("engine state poisoned");
                (
                    state.m4b_last_baseline_event_id.clone(),
                    state.m4b_last_baseline_tick.unwrap_or(0),
                    state.m4b_previous_snapshot.clone(),
                )
            };
            let (Some(baseline_event_id), Some(previous)) = (baseline_event_id, previous) else {
                return;
            };
            let ops = cf_save::delta::diff(&previous, &state_value);
            let ops_value = match serde_json::to_value(&ops) {
                Ok(v) => v,
                Err(err) => {
                    tracing::warn!(target: "cf::ctl::m4b", ?err, "delta ops serialization failed");
                    return;
                }
            };
            if let Err(err) = cf_replay::snapshot_delta::emit_delta(
                &self.recorder,
                tick,
                sim_time_ms,
                tick.0,
                baseline_event_id,
                baseline_tick,
                ops_value,
            ) {
                tracing::warn!(target: "cf::ctl::m4b", ?err, "delta emission failed");
                return;
            }
            // Update the previous-snapshot cursor so the next delta diffs
            // against THIS tick's state, not the baseline's state.
            let mut state = self.state.write().expect("engine state poisoned");
            state.m4b_previous_snapshot = Some(state_value);
        }
    }

}
