//! Free helper fns + types extracted from engine.rs.
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

pub(crate) fn build_checksum_bytes(state: &EngineMutable) -> Vec<u8> {
    let mut out = state
        .actor_state
        .as_ref()
        .map(|s| s.checksum_bytes())
        .unwrap_or_default();
    if let Some(world) = state.breach_world.as_ref() {
        out.extend_from_slice(&world.checksum_bytes());
    }
    out.extend_from_slice(&(state.reactive_guards.len() as u64).to_le_bytes());
    for g in state.reactive_guards.values() {
        out.extend_from_slice(&g.checksum_bytes());
    }
    if let Some(mission) = state.mission.as_ref() {
        // `mission_state (current_phase, timer_remaining_ticks,
        // objective_states[])`. Previously only objective status was
        // hashed, so two missions with identical objective statuses but
        // different lifecycle / timer state would collide. Append the
        // current lifecycle, timer fields, and pause state so the
        // checksum captures the full mission state.
        out.push(mission.lifecycle as u8);
        out.extend_from_slice(&mission.time_limit_ticks.to_le_bytes());
        out.push(if mission.paused { 1u8 } else { 0u8 });
        out.extend_from_slice(&mission.last_transition_tick.to_le_bytes());
        out.extend_from_slice(&(mission.objectives.len() as u64).to_le_bytes());
        for obj in &mission.objectives {
            out.push(obj.status as u8);
        }
    }
    if let Some(terrain) = state.chunked_terrain.as_ref() {
        out.extend_from_slice(&terrain.checksum_bytes());
    }
    if let Some(reactors) = state.reactor_world.as_ref() {
        out.extend_from_slice(&reactors.checksum_bytes());
    }
    // stratification producer state so any non-determinism in the
    // DamagedGrav wave-front growth, transient wind apertures, or
    // stratification deltas surfaces as a checksum drift instead of
    // silently going undetected.
    out.extend_from_slice(&(state.m14b_gravity_overrides.len() as u64).to_le_bytes());
    for ovr in &state.m14b_gravity_overrides {
        out.extend_from_slice(&ovr.id().to_le_bytes());
        if let cf_physics::GravityOverride::DamagedGrav { wave_front_radius, .. } = ovr {
            out.extend_from_slice(&wave_front_radius.to_le_bytes());
        }
    }
    out.extend_from_slice(&(state.m14b_wind_sources.len() as u64).to_le_bytes());
    for ws in &state.m14b_wind_sources {
        out.extend_from_slice(&ws.id.to_le_bytes());
        out.extend_from_slice(&ws.aperture_area_m2.to_le_bytes());
    }
    out.extend_from_slice(&(state.m14b_atmos_cells.len() as u64).to_le_bytes());
    for c in &state.m14b_atmos_cells {
        out.extend_from_slice(&c.id.to_le_bytes());
        out.extend_from_slice(&c.pressure_kpa.to_le_bytes());
        out.extend_from_slice(&c.temp_k.to_le_bytes());
    }
    out.extend_from_slice(&(state.m14b_strat_cells.len() as u64).to_le_bytes());
    for s in &state.m14b_strat_cells {
        out.extend_from_slice(&s.cell_id.to_le_bytes());
        out.extend_from_slice(&(s.fractions.len() as u32).to_le_bytes());
        for (gas, frac) in &s.fractions {
            out.extend_from_slice(gas.label().as_bytes());
            out.extend_from_slice(&frac.to_le_bytes());
        }
    }
    out.extend_from_slice(&(state.m14b_transient_wind_ttl.len() as u64).to_le_bytes());
    for (id, ttl) in &state.m14b_transient_wind_ttl {
        out.extend_from_slice(&id.to_le_bytes());
        out.extend_from_slice(&ttl.to_le_bytes());
    }
    // ships with this mission so save → load → save round-trips byte-
    // identically. Append-only — never reorder existing fields.
    out.extend_from_slice(&(state.m14e_chunks.len() as u64).to_le_bytes());
    for id in state.m14e_chunks.keys() {
        out.extend_from_slice(&id.0.to_le_bytes());
        out.extend_from_slice(&id.1.to_le_bytes());
    }
    out.extend_from_slice(&(state.m14e_actor_resources.len() as u64).to_le_bytes());
    for (actor_id, deltas) in &state.m14e_actor_resources {
        out.extend_from_slice(&actor_id.to_le_bytes());
        out.extend_from_slice(&(deltas.len() as u64).to_le_bytes());
        for (k, v) in deltas {
            out.extend_from_slice(k.as_bytes());
            out.push(0);
            out.extend_from_slice(&v.to_le_bytes());
        }
    }
    out.extend_from_slice(&(state.m14f_lateral_chunks.len() as u64).to_le_bytes());
    for id in state.m14f_lateral_chunks.keys() {
        out.extend_from_slice(&id.0.to_le_bytes());
        out.extend_from_slice(&id.1.to_le_bytes());
    }
    out.extend_from_slice(&state.m14f_lateral_pass_invocations.to_le_bytes());
    out.extend_from_slice(&(state.m14f_actor_submerged_tick.len() as u64).to_le_bytes());
    for (k, v) in &state.m14f_actor_submerged_tick {
        out.extend_from_slice(&k.to_le_bytes());
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.extend_from_slice(&(state.m14f_actor_vacuum_tick.len() as u64).to_le_bytes());
    for (k, v) in &state.m14f_actor_vacuum_tick {
        out.extend_from_slice(&k.to_le_bytes());
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.extend_from_slice(&(state.m14f_breach_fluid_mass.len() as u64).to_le_bytes());
    for (id, m) in &state.m14f_breach_fluid_mass {
        out.extend_from_slice(&id.0.to_le_bytes());
        out.extend_from_slice(&id.1.to_le_bytes());
        out.extend_from_slice(&m.to_le_bytes());
    }
    out.extend_from_slice(&(state.m14f_breach_pressure_kpa.len() as u64).to_le_bytes());
    for (id, (room, vac)) in &state.m14f_breach_pressure_kpa {
        out.extend_from_slice(&id.0.to_le_bytes());
        out.extend_from_slice(&id.1.to_le_bytes());
        out.extend_from_slice(&room.to_le_bytes());
        out.extend_from_slice(&vac.to_le_bytes());
    }
    // M14G — wound aging pass invocation counter (already covered by per-actor
    // wound list inside `actor_state.checksum_bytes`, but include the counter
    // here so independent cadence drift is observable on the engine level).
    out.extend_from_slice(&state.m14g_wound_aging_invocations.to_le_bytes());
    // M14G — thermal pass dwell counters + latched degrees. Hash both so
    // save → load → save round-trips byte-identically (VAL-CROSS-029).
    out.extend_from_slice(&(state.m14g_thermal_dwell_ticks.len() as u64).to_le_bytes());
    for ((actor_id, zone), dwell) in &state.m14g_thermal_dwell_ticks {
        out.extend_from_slice(&actor_id.to_le_bytes());
        out.extend_from_slice(zone.as_bytes());
        out.push(0);
        out.extend_from_slice(&dwell.to_le_bytes());
    }
    out.extend_from_slice(&(state.m14g_thermal_emitted_kind.len() as u64).to_le_bytes());
    for ((actor_id, zone), kind) in &state.m14g_thermal_emitted_kind {
        out.extend_from_slice(&actor_id.to_le_bytes());
        out.extend_from_slice(zone.as_bytes());
        out.push(0);
        out.push(*kind as u8);
    }
    out.extend_from_slice(&(state.m14g_material_contacts_fired.len() as u64).to_le_bytes());
    for idx in &state.m14g_material_contacts_fired {
        out.extend_from_slice(&(*idx as u64).to_le_bytes());
    }
    out
}

