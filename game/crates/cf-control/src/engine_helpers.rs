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

impl From<&ActorSimState> for ActorWorldSnapshot {
    fn from(sim: &ActorSimState) -> Self {
        let actors = sim
            .world
            .actors
            .values()
            .map(|a| {
                json!({
                    "id": a.id.0,
                    "team": a.team,
                    "controllable": a.controllable,
                    "position": [a.position.x, a.position.y],
                    "velocity": [a.velocity.x, a.velocity.y],
                    "aim": [a.aim.x, a.aim.y],
                    "on_ground": a.on_ground,
                    "status": a.status.as_str(),
                    "hp": a.hp,
                    "hp_max": a.hp_max,
                })
            })
            .collect();
        Self {
            actors,
            player_actor_id: sim.world.player.map(|id| id.0),
        }
    }
}

/// Dig outcome packed for cross-thread transport so events can be emitted
/// after the engine state guard is dropped. M1.5 ships [`DigEvent::Strip`]
/// (legacy `BreachStrip` path) and BP2 (M2) adds [`DigEvent::Chunked`] for
/// chunked-terrain digs. Engine prefers `Chunked` whenever the scenario opts
/// into chunked terrain.
#[derive(Debug, Clone)]
pub(crate) enum DigEvent {
    Strip {
        outcome: cf_terrain::DigOutcome,
        source: IntentSource,
        origin: [f32; 2],
    },
    Chunked {
        outcome: cf_terrain::ChunkedCarveOutcome,
        source: IntentSource,
        origin: [f32; 2],
        aim: [f32; 2],
        target: [f32; 2],
    },
}

impl DigEvent {
pub(crate)     fn outcome_target_string(&self) -> Option<String> {
        match self {
            DigEvent::Strip { outcome, .. } => match outcome {
                cf_terrain::DigOutcome::Carved { strip_id, .. } => Some(strip_id.clone()),
                cf_terrain::DigOutcome::Refused { strip_id, .. } => strip_id.clone(),
            },
            DigEvent::Chunked { .. } => None,
        }
    }

pub(crate)     fn source(&self) -> IntentSource {
        match self {
            DigEvent::Strip { source, .. } | DigEvent::Chunked { source, .. } => *source,
        }
    }

pub(crate)     fn origin(&self) -> [f32; 2] {
        match self {
            DigEvent::Strip { origin, .. } | DigEvent::Chunked { origin, .. } => *origin,
        }
    }
}

/// M1.5: bundle returned from a guard's [`cf_ai::FireRecord`] so we can spawn
/// projectiles into the actor pool after the guard step finishes. `will_miss`
/// is recorded for cause-chain visibility — the projectile velocity is already
/// drifted at AI step time, so the engine just propagates it.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct GuardFireRecord {
    pub(crate) shooter: ActorId,
    pub(crate) origin: [f32; 2],
    pub(crate) velocity: [f32; 2],
    pub(crate) damage: f32,
    pub(crate) lifetime_ticks: u32,
    pub(crate) will_miss: bool,
}

/// Build the JsonSchema-friendly mission view used by the observe envelope.
/// M4A: build the module strip placeholder for a single actor. Until M5 lands
/// real chassis modules, the strip carries one weapon_mount slot derived from
/// the actor's selected rifle, plus three `not_present` placeholder slots
/// (jet, shield, sensor) so HUD + accessibility consumers can rely on stable
/// ids before the real implementation lands.
pub(crate) fn build_module_strip_view(
    rifle: Option<&cf_equipment::RifleState>,
    has_rifle_selected: bool,
) -> crate::state::ModuleStripView {
    let weapon_state = match (rifle, has_rifle_selected) {
        (Some(r), true) => {
            let reloading = r.reload_remaining_ticks > 0;
            let empty = r.spec.mag_capacity > 0 && r.ammo_in_mag == 0;
            if reloading || empty {
                "warning"
            } else {
                "nominal"
            }
        }
        _ => "not_present",
    };
    let weapon_label = match (rifle, has_rifle_selected) {
        (Some(r), true) => {
            if r.reload_remaining_ticks > 0 {
                "RELOADING".to_string()
            } else if r.spec.mag_capacity > 0 && r.ammo_in_mag == 0 {
                "EMPTY".to_string()
            } else {
                format!("READY {}/{}", r.ammo_in_mag, r.spec.mag_capacity)
            }
        }
        _ => "—".to_string(),
    };
    let modules = vec![
        crate::state::ModuleStateView {
            id: "weapon_mount".to_string(),
            label: weapon_label,
            state: weapon_state.to_string(),
            kind: "weapon_mount".to_string(),
        },
        crate::state::ModuleStateView {
            id: "jet".to_string(),
            label: "JET N/A".to_string(),
            state: "not_present".to_string(),
            kind: "jet".to_string(),
        },
        crate::state::ModuleStateView {
            id: "shield".to_string(),
            label: "SHIELD N/A".to_string(),
            state: "not_present".to_string(),
            kind: "shield".to_string(),
        },
        crate::state::ModuleStateView {
            id: "sensor".to_string(),
            label: "SENSOR N/A".to_string(),
            state: "not_present".to_string(),
            kind: "sensor".to_string(),
        },
    ];
    crate::state::ModuleStripView {
        modules,
        placeholder: true,
    }
}

/// M4A: stable accessibility ids for every focusable HUD node, in z-order.
/// Single source: [`HUD_FOCUSABLE_NODES`]. Consumed by `cfctl ui` (M4B+),
/// `cf-e2e --verify-focus`, the live-WS acceptance tests, and cf-app's
/// keyboard focus traversal system.
pub(crate) fn hud_focusable_nodes() -> Vec<String> {
    HUD_FOCUSABLE_NODES.iter().map(|s| (*s).to_string()).collect()
}

/// **M1.5**: compose the AI-debug intent label rendered above the guard
/// sprite when `Settings.ai_debug == true`. Format mirrors the spec text
/// ("ALERT: heard_shot", "ENGAGED", "RELOADING", "STUCK: blocked"). The
/// label is also produced when ai_debug is disabled so the run bundle
/// is identical regardless of overlay state — cf-ui simply hides the
/// element when the flag is off.
pub(crate) fn ai_intent_label(guard: &cf_ai::ReactiveGuard) -> String {
    let state_label = match guard.state {
        cf_ai::GuardState::Idle => "IDLE",
        cf_ai::GuardState::Alert => "ALERT",
        cf_ai::GuardState::Engaged => "ENGAGED",
        cf_ai::GuardState::Retreating => "RETREATING",
        cf_ai::GuardState::Dying => "DYING",
        cf_ai::GuardState::Dead => "DEAD",
    };
    // M2 audit pass 7 (2026-05-13): spec literal — label shows
    // "{STATE}: {REASON}" (e.g. "ALERT: heard shot", "ENGAGING",
    // "RELOADING", "STUCK: blocked"). Reason is the most recent
    // state-change cause. Fall back to the chosen-tactic vocabulary when
    // no transition has fired yet (e.g. tick 0 with no perception).
    if guard.reload_remaining_ticks > 0 {
        return format!("{state_label}: RELOADING");
    }
    if guard.stuck_recovery_latched {
        return format!("{state_label}: STUCK: blocked");
    }
    if let Some(cause) = &guard.last_state_change_cause {
        // Render reason in human-readable form (snake_case → space).
        let pretty = cause.replace('_', " ");
        return format!("{state_label}: {pretty}");
    }
    match guard.last_tactic {
        cf_ai::Tactic::Attack => format!("{state_label}: ATTACK"),
        cf_ai::Tactic::Search => format!("{state_label}: SEARCH"),
        cf_ai::Tactic::Reload => format!("{state_label}: RELOAD"),
        cf_ai::Tactic::AimSettle => format!("{state_label}: AIM"),
        cf_ai::Tactic::Hold => state_label.to_string(),
    }
}

pub(crate) fn build_mission_view(state: &cf_mission::MissionState, current_tick: u64) -> crate::state::MissionView {
    let view = cf_mission::MissionView::from_state(state, current_tick);
    let objectives = view
        .objectives
        .into_iter()
        .map(|o| crate::state::ObjectiveView {
            id: o.id,
            kind: o.kind,
            status: o.status,
            optional: o.optional,
            target_actor: o.target_actor,
            target_breach: o.target_breach,
            target_reactor: o.target_reactor,
            zone_min: o.zone_min,
            zone_max: o.zone_max,
        })
        .collect();
    crate::state::MissionView {
        result: view.result,
        loss_reason: view.loss_reason,
        elapsed_ticks: view.elapsed_ticks,
        time_limit_ticks: view.time_limit_ticks,
        ticks_remaining: view.ticks_remaining,
        active_objective: view.active_objective,
        objectives,
        last_event_tick: view.last_event_tick,
        last_event_label: view.last_event_label,
        show_me_why_event_id: view.show_me_why_event_id,
        show_replay_cta: view.show_replay_cta,
    }
}

/// Build the checksum bytes covering every M1.5 + BP2 sub-state. Layout is
/// append-only relative to M1 so the `sim_state_v1` suffix stays valid:
/// `(M0 prefix) || (M1 actor bytes) || (M1.5 breach + guards + mission) ||
/// (M2 chunked terrain) || (M2.5 reactor world)`.
/// **M5**: parse a body zone name (`head`, `torso`, ...) into a `cf_chassis::BodyZone`.
pub(crate) fn parse_body_zone(s: &str) -> Option<cf_chassis::BodyZone> {
    match s {
        "head" => Some(cf_chassis::BodyZone::Head),
        "torso" => Some(cf_chassis::BodyZone::Torso),
        "arm_left" => Some(cf_chassis::BodyZone::ArmLeft),
        "arm_right" => Some(cf_chassis::BodyZone::ArmRight),
        "leg_left" => Some(cf_chassis::BodyZone::LegLeft),
        "leg_right" => Some(cf_chassis::BodyZone::LegRight),
        "backpack" => Some(cf_chassis::BodyZone::Backpack),
        _ => None,
    }
}

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
        // **M4 § Checksum scope sim_state_v1** — element #17 spec literal:
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
    // **M14B § Checksum**: include the gravity field + wind force + gas
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
    // **M14G § VAL-CROSS-029**: hash the M14E + M14F transient state that
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

/// **M14G § VAL-CROSS-029**: mod-payload key under which
/// [`M0Engine::snapshot_world_save`] stashes the M14C/D/E/F/G runtime
/// state in [`cf_save::WorldSave::mod_payload`]. Append-only — readers
/// that don't understand this key still round-trip the value verbatim
/// per the SaveBlob "Mod-extending fields survive migration" contract.
pub(crate) const M14_SAVE_EXTENSION_KEY: &str = "corefall.m14_state";

/// **M14G § VAL-CROSS-029**: serializable wrapper around the M14C/D/E/F/G
/// engine state that participates in the save/load round-trip.
/// `capture` reads from a read-locked [`EngineMutable`] reference;
/// `apply` writes back into a write-locked one. Both round-trip through
/// the [`M14_SAVE_EXTENSION_KEY`] entry in [`cf_save::WorldSave::mod_payload`].
///
/// `serde_json` only supports string-shaped map keys at the wire level,
/// so every M14 state surface that uses a tuple key (`(i32, i32)` for
/// chunk coordinates, `(u64, String)` for `(actor, zone)`) is captured
/// as a `Vec<(key, value)>` here and reassembled into its native
/// `BTreeMap<TupleKey, _>` shape inside [`Self::apply`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct M14SaveExtension {
    /// Per-actor [`cf_wound::ActorWoundList`] — captures the M14G
    /// `ActorWoundList` field on each [`cf_actor::Actor`].
    pub actor_wound_lists: Vec<(u64, cf_wound::ActorWoundList)>,
    /// M14D projectile-pair pool — captures the in-flight projectiles
    /// the per-tick CCD pass operates on.
    pub m14d_projectile_pool: Vec<cf_physics::ProjectileSnapshot>,
    /// M14E chunked integrity buffers (256-byte per-chunk
    /// `IntegrityField` + support_beam anchored flag + crack/cave-in
    /// state + cadence deadlines).
    pub m14e_chunks: Vec<((i32, i32), M14eChunkState)>,
    /// M14E cumulative beam/cave-in event counters.
    pub m14e_total_cave_ins: u32,
    pub m14e_total_beams_placed: u32,
    pub m14e_total_beams_destroyed: u32,
    /// M14E per-actor crafting-resource ledger (iron/wood debits).
    pub m14e_actor_resources: Vec<(u64, BTreeMap<String, i64>)>,
    /// M14E deterministic cave-in RNG cursor.
    pub m14e_rng_state: u64,
    /// M14E pass invocation counter (cadence schedule trace).
    pub m14e_pass_invocations: u64,
    /// M14E knockdown latch keyed by actor id.
    pub m14e_actor_knockdown: Vec<(u64, bool)>,
    /// M14E last-tick of a chunk's cave-in (drives 15-tick cascade
    /// window).
    pub m14e_last_cave_in_tick: Vec<((i32, i32), u64)>,
    /// M14E tunnel-creak audio cue counter.
    pub m14e_tunnel_creak_count: u32,
    /// M14E cave-in thunder audio cue counter.
    pub m14e_cave_in_thunder_count: u32,
    /// M14E plasma-cutter active flag per actor.
    pub m14e_plasma_cutter_active: Vec<(u64, bool)>,
    /// M14F lateral integrity buffers + brace_strut state per chunk.
    pub m14f_lateral_chunks: Vec<((i32, i32), M14fLateralChunkState)>,
    /// M14F lateral pass invocation count (cadence trace).
    pub m14f_lateral_pass_invocations: u64,
    /// M14F per-actor flood-contact tick.
    pub m14f_actor_submerged_tick: Vec<(u64, u64)>,
    /// M14F per-actor vacuum-exposure tick.
    pub m14f_actor_vacuum_tick: Vec<(u64, u64)>,
    /// M14F cumulative fluid mass passed through each dam-chunk breach.
    pub m14f_breach_fluid_mass: Vec<((i32, i32), u64)>,
    /// M14F (room_kpa, vacuum_kpa) pressure pair per sealed-room chunk.
    pub m14f_breach_pressure_kpa: Vec<((i32, i32), (f32, f32))>,
    /// M14G aging-pass invocation count.
    pub m14g_wound_aging_invocations: u64,
    /// M14G thermal dwell counter per (actor, zone).
    pub m14g_thermal_dwell_ticks: Vec<((u64, String), u64)>,
    /// M14G latched thermal degree per (actor, zone).
    pub m14g_thermal_emitted_kind: Vec<((u64, String), cf_wound::WoundKind)>,
    /// M14G material-contact one-shot fired set.
    pub m14g_material_contacts_fired: Vec<usize>,
    /// **M14G**: per-actor rifle state at save tick — `(actor_id,
    /// rifle_preset, ammo_in_mag, reload_remaining_ticks,
    /// fire_cooldown_ticks)`. Restored alongside the SaveBlob's
    /// per-actor rifle fields so the loaded engine's `sim.rifles`
    /// matches the source state for VAL-CROSS-029 byte-equality.
    pub rifle_states: Vec<RifleStateSnapshot>,
}

/// **M14G § VAL-CROSS-029**: serializable view of one `cf_equipment::RifleState`
/// stamp used by [`M14SaveExtension`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct RifleStateSnapshot {
    pub actor_id: u64,
    pub preset_id: String,
    pub ammo_in_mag: u32,
    pub reload_remaining_ticks: u32,
    pub fire_cooldown_ticks: u32,
}

impl M14SaveExtension {
    pub(crate) fn capture(
        state: &EngineMutable,
        actor_wound_lists: BTreeMap<u64, cf_wound::ActorWoundList>,
    ) -> Self {
        let rifle_states: Vec<RifleStateSnapshot> = state
            .actor_state
            .as_ref()
            .map(|sim| {
                sim.rifles
                    .iter()
                    .map(|(actor_id, r)| RifleStateSnapshot {
                        actor_id: actor_id.0,
                        preset_id: r.spec.preset_id.clone(),
                        ammo_in_mag: r.ammo_in_mag,
                        reload_remaining_ticks: r.reload_remaining_ticks,
                        fire_cooldown_ticks: r.fire_cooldown_ticks,
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            actor_wound_lists: actor_wound_lists.into_iter().collect(),
            m14d_projectile_pool: state.m14d_projectile_pair_pool.clone(),
            m14e_chunks: state.m14e_chunks.iter().map(|(k, v)| (*k, v.clone())).collect(),
            m14e_total_cave_ins: state.m14e_total_cave_ins,
            m14e_total_beams_placed: state.m14e_total_beams_placed,
            m14e_total_beams_destroyed: state.m14e_total_beams_destroyed,
            m14e_actor_resources: state
                .m14e_actor_resources
                .iter()
                .map(|(k, v)| (*k, v.clone()))
                .collect(),
            m14e_rng_state: state.m14e_rng_state,
            m14e_pass_invocations: state.m14e_pass_invocations,
            m14e_actor_knockdown: state
                .m14e_actor_knockdown
                .iter()
                .map(|(k, v)| (*k, *v))
                .collect(),
            m14e_last_cave_in_tick: state
                .m14e_last_cave_in_tick
                .iter()
                .map(|(k, v)| (*k, *v))
                .collect(),
            m14e_tunnel_creak_count: state.m14e_tunnel_creak_count,
            m14e_cave_in_thunder_count: state.m14e_cave_in_thunder_count,
            m14e_plasma_cutter_active: state
                .m14e_plasma_cutter_active
                .iter()
                .map(|(k, v)| (*k, *v))
                .collect(),
            m14f_lateral_chunks: state
                .m14f_lateral_chunks
                .iter()
                .map(|(k, v)| (*k, v.clone()))
                .collect(),
            m14f_lateral_pass_invocations: state.m14f_lateral_pass_invocations,
            m14f_actor_submerged_tick: state
                .m14f_actor_submerged_tick
                .iter()
                .map(|(k, v)| (*k, *v))
                .collect(),
            m14f_actor_vacuum_tick: state
                .m14f_actor_vacuum_tick
                .iter()
                .map(|(k, v)| (*k, *v))
                .collect(),
            m14f_breach_fluid_mass: state
                .m14f_breach_fluid_mass
                .iter()
                .map(|(k, v)| (*k, *v))
                .collect(),
            m14f_breach_pressure_kpa: state
                .m14f_breach_pressure_kpa
                .iter()
                .map(|(k, v)| (*k, *v))
                .collect(),
            m14g_wound_aging_invocations: state.m14g_wound_aging_invocations,
            m14g_thermal_dwell_ticks: state
                .m14g_thermal_dwell_ticks
                .iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect(),
            m14g_thermal_emitted_kind: state
                .m14g_thermal_emitted_kind
                .iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect(),
            m14g_material_contacts_fired: state
                .m14g_material_contacts_fired
                .iter()
                .copied()
                .collect(),
            rifle_states,
        }
    }

    pub(crate) fn apply(self, state: &mut EngineMutable) {
        if let Some(sim) = state.actor_state.as_mut() {
            for (actor_id, list) in self.actor_wound_lists {
                if let Some(actor) = sim.world.actors.get_mut(&cf_actor::ActorId(actor_id)) {
                    actor.m14g_wound_list = list;
                }
            }
            for rifle_snap in self.rifle_states {
                if let Some(r) = sim.rifles.get_mut(&cf_actor::ActorId(rifle_snap.actor_id)) {
                    if r.spec.preset_id == rifle_snap.preset_id {
                        r.ammo_in_mag = rifle_snap.ammo_in_mag;
                        r.reload_remaining_ticks = rifle_snap.reload_remaining_ticks;
                        r.fire_cooldown_ticks = rifle_snap.fire_cooldown_ticks;
                    }
                }
            }
        }
        state.m14d_projectile_pair_pool = self.m14d_projectile_pool;
        state.m14e_chunks = self.m14e_chunks.into_iter().collect();
        state.m14e_total_cave_ins = self.m14e_total_cave_ins;
        state.m14e_total_beams_placed = self.m14e_total_beams_placed;
        state.m14e_total_beams_destroyed = self.m14e_total_beams_destroyed;
        state.m14e_actor_resources = self.m14e_actor_resources.into_iter().collect();
        state.m14e_rng_state = self.m14e_rng_state;
        state.m14e_pass_invocations = self.m14e_pass_invocations;
        state.m14e_actor_knockdown = self.m14e_actor_knockdown.into_iter().collect();
        state.m14e_last_cave_in_tick = self.m14e_last_cave_in_tick.into_iter().collect();
        state.m14e_tunnel_creak_count = self.m14e_tunnel_creak_count;
        state.m14e_cave_in_thunder_count = self.m14e_cave_in_thunder_count;
        state.m14e_plasma_cutter_active = self.m14e_plasma_cutter_active.into_iter().collect();
        state.m14f_lateral_chunks = self.m14f_lateral_chunks.into_iter().collect();
        state.m14f_lateral_pass_invocations = self.m14f_lateral_pass_invocations;
        state.m14f_actor_submerged_tick = self.m14f_actor_submerged_tick.into_iter().collect();
        state.m14f_actor_vacuum_tick = self.m14f_actor_vacuum_tick.into_iter().collect();
        state.m14f_breach_fluid_mass = self.m14f_breach_fluid_mass.into_iter().collect();
        state.m14f_breach_pressure_kpa = self.m14f_breach_pressure_kpa.into_iter().collect();
        state.m14g_wound_aging_invocations = self.m14g_wound_aging_invocations;
        state.m14g_thermal_dwell_ticks = self.m14g_thermal_dwell_ticks.into_iter().collect();
        state.m14g_thermal_emitted_kind = self.m14g_thermal_emitted_kind.into_iter().collect();
        state.m14g_material_contacts_fired = self.m14g_material_contacts_fired.into_iter().collect();
    }
}

/// M4A: outcome of one dig used to update the HUD tool-validity cache.
pub(crate) enum ToolValidityUpdate {
    Carve,
    Refuse { reason: String, target: Option<String> },
}

/// **M11 / DR-012**: resolve which HUD focusable node lives under a logical
/// pointer position. cf-app owns the actual hit-box geometry; this server-
/// side helper provides a deterministic mapping based on a normalized
/// 0..1 layout grid so cfctl `act.input.mouse_click` can target any node
/// by name without a render loop in the loop.
///
/// The mapping is COARSE on purpose: cfctl ui_assert + replay-side
/// rendering use this stub to confirm that the input round-trips through
/// the engine; precise pointer-to-pixel mapping lives in cf-app when the
/// pointer actually traverses the HUD.
pub(crate) fn resolve_hud_node_at(x: f32, y: f32) -> Option<String> {
    // Negative coords mean "off-screen" — emit empty target.
    if x < 0.0 || y < 0.0 {
        return None;
    }
    // The 12 HUD_FOCUSABLE_NODES occupy a 1×N vertical band on the left
    // edge of the screen by default. The exact pixel geometry is owned by
    // cf-ui's spawn_status_strip; this is the engine-side coarse map.
    let bucket = (y / 24.0).floor() as usize;
    if bucket < HUD_FOCUSABLE_NODES.len() {
        Some(HUD_FOCUSABLE_NODES[bucket].to_string())
    } else {
        None
    }
}

pub(crate) fn push_banner(queue: &mut VecDeque<crate::state::HudBannerView>, banner: crate::state::HudBannerView) {
    queue.push_back(banner);
    while queue.len() > M4A_BANNER_BUFFER {
        queue.pop_front();
    }
}

pub(crate) fn push_banner_dedup(queue: &mut VecDeque<crate::state::HudBannerView>, banner: crate::state::HudBannerView) {
    if queue.iter().any(|b| b.id == banner.id) {
        return;
    }
    push_banner(queue, banner);
}

pub(crate) fn push_caption(queue: &mut VecDeque<crate::state::CaptionView>, caption: crate::state::CaptionView) {
    queue.push_back(caption);
    while queue.len() > M4A_CAPTION_BUFFER {
        queue.pop_front();
    }
}

/// **M5**: build a HUD banner for a chassis stage transition.
pub(crate) fn chassis_stage_banner(stage: cf_chassis::ChassisStage, now_tick: u64) -> Option<crate::state::HudBannerView> {
    let (id, severity, label) = match stage {
        cf_chassis::ChassisStage::Nominal => return None,
        cf_chassis::ChassisStage::Degraded => return None,
        cf_chassis::ChassisStage::ModuleWarning => ("chassis_module_warning", "warning", "MODULE WARNING"),
        cf_chassis::ChassisStage::ModuleFailed => ("chassis_module_failed", "warning", "MODULE FAILED"),
        cf_chassis::ChassisStage::WeaponJammed => return None, // handled separately
        cf_chassis::ChassisStage::ArmorCracked => ("chassis_armor_cracked", "critical", "ARMOR CRACKED"),
        cf_chassis::ChassisStage::Disabled => ("chassis_disabled", "critical", "CHASSIS DISABLED"),
        cf_chassis::ChassisStage::PilotInjured => ("chassis_pilot_injured", "critical", "PILOT INJURED"),
        cf_chassis::ChassisStage::Eject => ("chassis_eject_now", "critical", "EJECT NOW"),
        cf_chassis::ChassisStage::BailTooLate => ("chassis_bail_too_late", "critical", "BAILED TOO LATE"),
        cf_chassis::ChassisStage::Wreck => ("chassis_wreck", "critical", "CHASSIS WRECKED"),
        cf_chassis::ChassisStage::Gibbed => ("chassis_gibbed", "critical", "CHASSIS DESTROYED"),
    };
    Some(crate::state::HudBannerView {
        id: id.to_string(),
        severity: severity.to_string(),
        label: label.to_string(),
        raised_at_tick: now_tick,
        expires_at_tick: Some(now_tick + M4A_STATUS_BANNER_EXPIRY_TICKS * 2),
        accessibility_id: format!("hud.banner.{id}"),
    })
}

/// **M5**: build a HUD banner for a pilot-state transition (eject/extract/lost).
pub(crate) fn chassis_pilot_banner(state: cf_chassis::PilotState, now_tick: u64) -> Option<crate::state::HudBannerView> {
    let (id, severity, label) = match state {
        cf_chassis::PilotState::Bound => return None,
        cf_chassis::PilotState::Injured => ("pilot_injured", "warning", "PILOT INJURED"),
        cf_chassis::PilotState::Ejecting => ("pilot_ejecting", "critical", "EJECTING"),
        cf_chassis::PilotState::Ejected => ("pilot_ejected", "info", "PILOT EJECTED"),
        cf_chassis::PilotState::Extracted => ("pilot_extracted", "info", "PILOT EXTRACTED"),
        cf_chassis::PilotState::BailedTooLate => ("pilot_bailed_too_late", "critical", "BAILED TOO LATE"),
        cf_chassis::PilotState::Lost => ("pilot_lost", "critical", "PILOT LOST"),
    };
    Some(crate::state::HudBannerView {
        id: id.to_string(),
        severity: severity.to_string(),
        label: label.to_string(),
        raised_at_tick: now_tick,
        expires_at_tick: Some(now_tick + M4A_STATUS_BANNER_EXPIRY_TICKS * 2),
        accessibility_id: format!("hud.banner.{id}"),
    })
}

/// Cause label for `actor.actor_status_changed` events emitted from `step_one_actor`.
///
/// In M1 the only mutator inside `step_one_actor` that touches `actor.status` is
/// `actor.reset()` (called when the player issues `act.player.reset`). Damage-driven
/// transitions are emitted from a separate projectile-hit loop with cause
/// `projectile_hit`, never via this helper. Future milestones (M5 chassis ejection,
/// M5.6 hazard contact, etc.) MUST extend [`ActorTickOutcome`] with an explicit
/// cause discriminant rather than relying on a generic catch-all label here, so
/// the cause-chain stays semantically correct for replay analysis.
///
/// **M1 audit pass 6 (2026-05-13)**: recognize the `travel_impulse_damage`
/// flag (latched by `cf-actor::sim` when an UNSTABLE actor takes
/// travel-impulse damage per CCCP `Actor.cpp:1199`).
pub(crate) fn status_change_cause(outcome: &ActorTickOutcome) -> &'static str {
    if outcome.travel_impulse_damage {
        "travel_impulse"
    } else if outcome.reset {
        "reset"
    } else {
        // Defensive fallback: if a future milestone introduces another
        // status-mutating path inside `step_one_actor` without extending
        // `ActorTickOutcome` with an explicit cause discriminant,
        // surfacing `unknown` makes the contract gap visible in the run
        // bundle so it can be caught and fixed.
        "unknown"
    }
}

/// Map a normalized milestone hint (`m0`, `m1`, `m1.5`, `m2`, `m2.5`, ...)
/// to the upper-case `prototype_slice` label written into `run_manifest.json`.
/// Falls back to upper-casing the input so future milestones keep working
/// without an explicit branch here.
/// Canonical roadmap milestone ordering, used by every per-milestone helper
/// that needs "is this milestone >= Mx?". Each index is a position in the
/// canonical Build Points spine — M0=0, M1=1, M1.5=2, M2=3, M2.5=4, M3A=5,
/// M3B=6, M4A=7, M4B=8, M5=9, M5.5=10, M5.5.5=11, M5.6=12, M5.7=13, M5.8=14,
/// M5.9=15, M5.9.5=16, M5.10=17, M6=18, M6.5=19, M6.6=20, M7=21, M7.5=22,
/// M7.7=23, M8=24, M8.5=25, M8.6=26, M9=27, M9.5=28, M10=29, M11=30, M12=31.
/// Unknown milestones map to `MILESTONE_INDEX_UNKNOWN` (after M12) so they
/// default to the final-state universe (every category is included, every
/// addendum fires) — better to over-document a future milestone than
/// silently skip categories that have been shipping for years.
///
/// Append a row when a new milestone lands in the canonical roadmap. The
/// constants below (`MILESTONE_INDEX_M0`, `_M1`, `_M1_5`, `_M2`, `_M3A`) are
/// landmark gates the category-layering logic + DR-007 addendum check; only
/// add new constants here when a new event category or schema is introduced
/// (the current landmarks cover M0 baseline, M1 actor, M1.5 ai/mission/terrain,
/// M2 material, M3A snapshot; if M5.6 introduces a new category, add
/// `MILESTONE_INDEX_M5_6`).
const MILESTONE_INDEX_M0: u32 = 0;
const MILESTONE_INDEX_M1: u32 = 1;
const MILESTONE_INDEX_M1_5: u32 = 2;
const MILESTONE_INDEX_M2: u32 = 3;
const MILESTONE_INDEX_M3A: u32 = 5;
const MILESTONE_INDEX_UNKNOWN: u32 = 999;

/// BP4 + BP5 forward-compat event-category reservation.
///
/// The recorder accepts arbitrary category strings (see
/// `cf_replay::Recorder::record`); there is no central whitelist that rejects
/// unknown categories. This const documents the categories that BP4 + BP5
/// milestones will start emitting so:
///
/// 1. Tooling (replay viewer, run-bundle checker, summary aggregators) can
///    bake forward-compat handling now instead of being rewritten when each
///    milestone lands.
/// 2. The per-milestone `notes_addendum_for_milestone` category list below
///    has an authoritative reference for which categories are "reserved
///    (no emitters yet)" vs "shipped at this milestone".
/// 3. AI agents auditing the codebase can grep for the category name and
///    find the owning milestone without scanning the roadmap.
///
/// Each entry is `(category, owning_milestone, note)`. No category in this
/// list should be emitted by any code path until its owning milestone ships
/// the producing system. `chassis` is already emitted by M5 chassis-stage
/// hooks (see `emit_chassis_events`) — it's listed here for completeness so
/// the BP4/BP5 reservation table is canonical.
#[allow(dead_code)]
const RESERVED_EVENT_CATEGORIES: &[(&str, &str, &str)] = &[
    ("collision", "M5.5", "full-collision + body-pixel impact events"),
    ("reaction", "M5.6", "material reaction-table priority resolution"),
    ("affliction", "M5.7 + M5.8", "wound/affliction status grammar"),
    ("atmospherics", "M5.9", "hull/gap/pump/vent/oxygen/pressure/fire"),
    ("environment", "M5.10", "DR-040 EnvironmentSignal aggregator"),
    ("gravity", "M5.5 / M5.9 — DR-038", "per-actor + global gravity field"),
    ("ballistics", "M5.5 / M5.9 — DR-038", "projectile aerodynamics + drag"),
    ("mind", "M6.5", "AI mind/intent telemetry"),
    (
        "body_force_feedback",
        "M5",
        "cf-actor body_force_feedback hit-hook stub event type",
    ),
    (
        "chassis",
        "M5 (already shipped)",
        "chassis-stage transitions emitted via emit_chassis_events",
    ),
];

pub(crate) fn milestone_order_index(milestone: &str) -> u32 {
    match milestone.trim().to_lowercase().as_str() {
        "" | "m0" => MILESTONE_INDEX_M0,
        "m1" => MILESTONE_INDEX_M1,
        "m1.5" => MILESTONE_INDEX_M1_5,
        "m2" => MILESTONE_INDEX_M2,
        "m2.5" => 4,
        "m3a" => MILESTONE_INDEX_M3A,
        "m3b" => 6,
        "m4a" => 7,
        "m4b" => 8,
        "m5" => 9,
        "m5.5" => 10,
        "m5.5.5" => 11,
        "m5.6" => 12,
        "m5.7" => 13,
        "m5.8" => 14,
        "m5.9" => 15,
        "m5.9.5" => 16,
        "m5.10" => 17,
        "m6" => 18,
        "m6.5" => 19,
        "m6.6" => 20,
        "m7" => 21,
        "m7.5" => 22,
        "m7.7" => 23,
        "m8" => 24,
        "m8.5" => 25,
        "m8.6" => 26,
        "m9" => 27,
        "m9.5" => 28,
        "m10" => 29,
        "m11" => 30,
        "m12" => 31,
        _ => MILESTONE_INDEX_UNKNOWN,
    }
}

pub(crate) fn prototype_slice_for_milestone(milestone: &str) -> String {
    let normalized = milestone.trim().to_lowercase();
    if normalized.is_empty() {
        return "M0".to_string();
    }
    // Bugbot 3212491755 + Devin 3212416493 both caught: the prior
    // `format!("M{rest}")` produced lowercase letter suffixes (`m3a` → `M3a`)
    // because `rest` retained the lowercased form from `normalized`. Letter-
    // suffixed milestones (M3A/M3B/M4A/M4B) must produce uppercase suffixes
    // to match the canonical roadmap naming + the source-truthful evidence
    // contract in AGENTS.md (run_manifest.json.prototype_slice ↔ roadmap id).
    if let Some(rest) = normalized.strip_prefix('m') {
        return format!("M{}", rest.to_uppercase());
    }
    normalized.to_uppercase()
}

/// Per-milestone "what to do next" line written into `summary.json.next_actions`.
/// Stale "Proceed to M1 task cards" boilerplate masqueraded as M0 metadata in
/// every bundle through M2.5; the canonical roadmap (Build Points table) is the
/// source of truth and we pin the next milestone here so an offline reviewer
/// can read the bundle and immediately see what the implementer was supposed
/// to ship next.
pub(crate) fn next_actions_for_milestone(milestone: &str) -> Vec<String> {
    let normalized = milestone.trim().to_lowercase();
    let next = match normalized.as_str() {
        "" | "m0" => "Proceed to M1 task cards in spec/native-implementation-backlog.",
        "m1" => "Proceed to M1.5 (Micro Breach Fun Slice) per spec/prototype-roadmap.md#BP1.",
        "m1.5" => "Proceed to BP2 (M2 + M2.5 + M3A) per spec/prototype-roadmap.md#BP2.",
        "m2" => "Proceed to M2.5 (Micro Reactor Defense Fun Slice) per spec/prototype-roadmap.md#BP2.",
        "m2.5" => "Proceed to M3A (Event Recorder Core) per spec/prototype-roadmap.md#BP2.",
        "m3a" => "Proceed to BP3 (M3B + M4A + M5) per spec/prototype-roadmap.md#BP3.",
        "m3b" => "Proceed to M4A (Readability And ACC-A Floor) per spec/prototype-roadmap.md#BP3.",
        "m4a" => "Proceed to M5 (Equipment, Chassis, And Damage Grammar) per spec/prototype-roadmap.md#BP3.",
        "m5" => "Proceed to BP4 (M5.5 + M5.5.5 + M5.6 + M5.7 + M5.8) per spec/prototype-roadmap.md#BP4.",
        _ => "Proceed to the next assigned milestone per spec/prototype-roadmap.md.",
    };
    vec![next.to_string()]
}

/// Per-milestone notes-addendum prose written into `notes.md` after the
/// scenario-author rows (Good/Bad/Meh/Evidence). The historical
/// `m0_notes_addendum` baked the M0 staging story ("M2/M3 will append terrain
/// bytes; all without bumping the suffix") into every bundle, which became
/// flat-out wrong once M2 / M2.5 / M3A landed. This helper returns the
/// up-to-date DR-002 + DR-012 lock prose AND the milestone's own pinned
/// contract addendum (e.g. material schema for M2+, expected-outcome contract
/// for M3A+).
pub(crate) fn notes_addendum_for_milestone(milestone: &str) -> String {
    let normalized = milestone.trim().to_lowercase();
    // Devin 3212580450 caught the source-truthful evidence bug here: claiming
    // ALL 12 event categories ship at every milestone is wrong (M0 only ships
    // system / control / determinism; terrain / material / mission / ai are
    // M1.5+; snapshot is M3A+). Build the per-milestone category list so the
    // notes addendum reflects what actually fired in this run, not the union
    // across the whole roadmap. Layer is append-only: each milestone inherits
    // every prior category.
    //
    // Devin 3212593186 follow-up: refactor from explicit per-milestone match
    // arms (which silently broke for M3B / M4A / M4B / M6+ that weren't
    // enumerated) to an ordering-based comparison via `milestone_order_index`.
    // The order index is the canonical roadmap progression and any new
    // milestone is added in one place rather than scattered across 4 match
    // statements that each had to be kept in sync.
    let idx = milestone_order_index(&normalized);
    let mut categories: Vec<&'static str> = vec!["system", "control", "determinism"];
    if idx >= MILESTONE_INDEX_M1 {
        categories.extend(["actor", "combat", "equipment", "input"]);
    }
    if idx >= MILESTONE_INDEX_M1_5 {
        categories.extend(["ai", "mission", "terrain"]);
    }
    if idx >= MILESTONE_INDEX_M2 {
        categories.push("material");
    }
    if idx >= MILESTONE_INDEX_M3A {
        categories.push("snapshot");
    }
    let categories_inline = categories
        .iter()
        .map(|c| format!("`{c}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let mut s = String::new();
    s.push_str("## DR-002 schema lock\n\n");
    s.push_str("- Event envelope: `{schema_version, run_id, tick, sim_time_ms, event_id, category, event_type, payload, parent_event_id?, dropped_count?}`.\n");
    s.push_str(&format!(
        "- Categories shipped through this milestone: {categories_inline}. Future categories layer in additively without breaking v1 envelope readers.\n"
    ));
    s.push_str("- Checksum: `algorithm=blake3`, `scope=sim_state_v1`. Layout is append-only: M0 (`tick_counter || rng_state_bytes`) || M1 (actor / inventory / projectile bytes) || M1.5 (breach + guards + mission bytes) || M2 (chunked-terrain bytes) || M2.5 (reactor-world bytes). Layout-breaking bumps go to `_v2`.\n");
    s.push_str("- Manifest extensions: `checksum.{algorithm,scope,cadence_ticks}`, `settings:{...}` block, `expected_outcome:{clean|panic|abort}` (M3A).\n");
    s.push_str("- Summary extensions: `final_sim_checksum`, `checksum_event_count`, `first_tick`, `last_tick`, `artifacts.items[]` populated from `captures/` when present (M2+).\n");
    s.push_str("- M3A picks up headless replay verification: `cf-headless replay <bundle> --scenario-path <path>` reconstructs commands from `control.command_accepted` and asserts the cadence checksums tick-for-tick.\n");
    s.push_str("\n## DR-012 floor lock\n\n");
    s.push_str("- Six accessibility flags wired into `cf-control::Settings` and `run_manifest.json.settings`.\n");
    s.push_str("- Settings can be live-updated via `act.settings.set` and re-read via `observe.settings`.\n");
    s.push_str(
        "- Localization deferred to M4 — the discipline rule (no baked English-only player-facing strings) applies.\n",
    );
    // DR-007 launch material set is reference documentation for what the
    // material system shape is. Every M2+ bundle that has material events
    // in events.jsonl benefits from seeing it, including milestones that
    // RUN ON TOP OF chunked terrain (M3B replay viewer, M4A readability)
    // and milestones that EXTEND it (M5.5 collision + materials, M5.6
    // material kernel, M6.6 AI material competence, M7.5 base atmospherics,
    // M8.5 material lab, M8.6 mining + refining).
    //
    // Bugbot 3212607793 + Devin 3212623450 caught the prior explicit
    // allowlist that stopped at M5.10 — when M6.6 / M7.5 / M8.5 / M8.6
    // (all of which clearly extend or work with materials) ship, they
    // would have silently missed the addendum. The fix matches the
    // category-layering pattern: `idx >= MILESTONE_INDEX_M2` so every
    // milestone past M2 in roadmap order inherits the material reference.
    // Unknown milestones map to MILESTONE_INDEX_UNKNOWN (post-M12) so
    // future milestones default to including the addendum.
    if idx >= MILESTONE_INDEX_M2 {
        s.push_str("\n## DR-007 launch material set\n\n");
        s.push_str("- 8 launch materials (ids 0..7): `air`, `dirt`, `concrete`, `metal_nohook`, `hazard`, `loose_fill`, `repair_fill`, `anchor`. `material_schema_version=cf-terrain-launch-v1`.\n");
        s.push_str("- Per-material affordances cover solid/diggable/hardness/anchorable/hazard/path_cost/overlay_rgba/refusal_reason.\n");
    }
    s
}

/// Build the `summary.json.tests[]` entries from the scenario's
/// `expected_tests` manifest field. Each entry's `result` is exit-code-driven
/// (engine-wide pass/fail), `evidence_event_ids` is the run's first+last event
/// id pair, and `notes` is a stable per-milestone rationale. If the scenario
/// declares no expected tests we synthesize a single milestone-level smoke
/// row so the array is never empty.
pub(crate) fn build_test_records(
    expected_tests: &[String],
    milestone: &str,
    result: &str,
    evidence_event_ids: &[String],
) -> Vec<TestRecord> {
    let normalized = milestone.trim().to_lowercase();
    let notes = match normalized.as_str() {
        "" | "m0" => "M0 fixed-tick smoke + run-bundle parity per spec/native-implementation-backlog.",
        "m1" => "M1 actor controller round-trip (move + jump + aim + fire + reload + select_item).",
        "m1.5" => "M1.5 micro breach fun slice (dig outer wall, kill guard, reach extraction).",
        "m2" => "M2 chunked-terrain dig path (dirt fast / concrete slow / metal_nohook + anchor refused).",
        "m2.5" => {
            "M2.5 micro reactor defense fun slice (dirt-shield strategic choice; reactor protected or destroyed)."
        }
        "m3a" => "M3A event recorder core (snapshot.* + expected_outcome contract + cf-headless replay verifier).",
        _ => "Milestone-scope acceptance per spec/native-implementation-backlog.",
    };
    if expected_tests.is_empty() {
        let id = match normalized.as_str() {
            "" | "m0" => "M0-SMOKE-01",
            "m1" => "M1-SMOKE-01",
            "m1.5" => "M1.5-SMOKE-01",
            "m2" => "M2-SMOKE-01",
            "m2.5" => "M2.5-SMOKE-01",
            "m3a" => "M3A-SMOKE-01",
            _ => "MILESTONE-SMOKE-01",
        };
        return vec![TestRecord {
            id: id.to_string(),
            result: result.to_string(),
            evidence_event_ids: evidence_event_ids.to_vec(),
            notes: Some(notes.to_string()),
        }];
    }
    expected_tests
        .iter()
        .map(|id| TestRecord {
            id: id.clone(),
            result: result.to_string(),
            evidence_event_ids: evidence_event_ids.to_vec(),
            notes: Some(notes.to_string()),
        })
        .collect()
}

/// Discover capture artifacts on disk at run-bundle write time. Returns
/// `(artifacts, evidence_link)` where:
///
/// - `artifacts` lists the recordable items inside `<run>/captures/`:
///   `capture_manifest.json`, `summary_grid.png`, every `grid_NNN.png`, and
///   one `capture_frames` summary entry counting the frame_*.png files.
/// - `evidence_link` is `"captures/"` when any capture artifact is present so
///   `notes.md`'s evidence-link list reflects the on-disk shape.
///
/// `summary_grid.png` may not exist at write_run_bundle time (the cf-e2e
/// composer adds it AFTER cf-app exits); `capture_grid.py` patches
/// `summary.json.artifacts.items[]` post-hoc to add the grid PNGs in that
/// case. This helper covers the in-process path (frames + manifest) and is
/// idempotent with the post-hoc patcher.
pub(crate) fn discover_run_artifacts(run_bundle_dir: &Path) -> (Vec<ArtifactItem>, Option<String>) {
    let captures_dir = run_bundle_dir.join("captures");
    if !captures_dir.is_dir() {
        return (Vec::new(), None);
    }
    let mut items: Vec<ArtifactItem> = Vec::new();
    let manifest_path = captures_dir.join("capture_manifest.json");
    if manifest_path.is_file() {
        items.push(ArtifactItem {
            kind: "capture_manifest".to_string(),
            path: "captures/capture_manifest.json".to_string(),
        });
    }
    let summary_grid = captures_dir.join("summary_grid.png");
    if summary_grid.is_file() {
        items.push(ArtifactItem {
            kind: "summary_grid".to_string(),
            path: "captures/summary_grid.png".to_string(),
        });
        let summary_grid_json = captures_dir.join("summary_grid.json");
        if summary_grid_json.is_file() {
            items.push(ArtifactItem {
                kind: "summary_grid_json".to_string(),
                path: "captures/summary_grid.json".to_string(),
            });
        }
    }
    let mut grids: Vec<String> = Vec::new();
    let mut frames: u64 = 0;
    if let Ok(read_dir) = std::fs::read_dir(&captures_dir) {
        for entry in read_dir.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy().to_string();
            if name_str.starts_with("grid_") && name_str.ends_with(".png") {
                grids.push(name_str);
            } else if name_str.starts_with("frame_") && name_str.ends_with(".png") {
                frames += 1;
            }
        }
    }
    grids.sort();
    for g in grids {
        items.push(ArtifactItem {
            kind: "capture_grid".to_string(),
            path: format!("captures/{g}"),
        });
    }
    if frames > 0 {
        items.push(ArtifactItem {
            kind: "capture_frames".to_string(),
            path: format!("captures/ ({frames} frame_*.png)"),
        });
    }
    let link = if items.is_empty() {
        None
    } else {
        Some("captures/".to_string())
    };
    (items, link)
}

/// **M8 game_speed_assist + pie-menu sim-speed composition.** Returns the
/// effective sim-speed percentage (0..=100) the per-tick scheduler honors
/// for the current tick. Composes:
///
/// - `settings.game_speed_assist.speed_pct()` (Off=100 / Slowdown75=75 /
///   Slowdown25=25 / FullPause=0) — single-player only; multiplayer
///   sessions force this leg to 100 per spec.
/// - `pie_menu.slowdown_factor_pct` (100 when closed, 20 in single-player
///   open, 100 in multiplayer open).
///
/// Most-restrictive wins (per the fix description: "the pie menu can stack
/// with game_speed_assist; whichever is more restrictive wins"). Pure
/// function so determinism is easy to verify in unit tests.
pub(crate) fn effective_sim_speed_pct(
    settings: &Settings,
    pie_menu: &cf_squad_ui::PieMenuState,
    multiplayer_session: bool,
) -> u8 {
    let assist_pct = if multiplayer_session {
        100
    } else {
        settings.game_speed_assist.speed_pct()
    };
    assist_pct.min(pie_menu.slowdown_factor_pct)
}

pub(crate) fn apply_settings_patch(settings: &mut Settings, patch: &SettingsPatch) -> Vec<String> {
    let mut changed = Vec::new();
    if let Some(v) = patch.ui_scale {
        let clamped = v.clamp(crate::settings::UI_SCALE_MIN, crate::settings::UI_SCALE_MAX);
        if (settings.ui_scale - clamped).abs() > f32::EPSILON {
            settings.ui_scale = clamped;
            changed.push("ui_scale".to_string());
        }
    }
    if let Some(v) = patch.high_contrast {
        if settings.high_contrast != v {
            settings.high_contrast = v;
            changed.push("high_contrast".to_string());
        }
    }
    if let Some(v) = patch.captions {
        if settings.captions != v {
            settings.captions = v;
            changed.push("captions".to_string());
        }
    }
    if let Some(v) = patch.reduced_motion {
        if settings.reduced_motion != v {
            settings.reduced_motion = v;
            changed.push("reduced_motion".to_string());
        }
    }
    if let Some(v) = patch.reduced_shake {
        if settings.reduced_shake != v {
            settings.reduced_shake = v;
            changed.push("reduced_shake".to_string());
        }
    }
    if let Some(v) = patch.reduced_flash {
        if settings.reduced_flash != v {
            settings.reduced_flash = v;
            changed.push("reduced_flash".to_string());
        }
    }
    if let Some(v) = patch.hold_to_confirm {
        if settings.hold_to_confirm != v {
            settings.hold_to_confirm = v;
            changed.push("hold_to_confirm".to_string());
        }
    }
    if let Some(v) = patch.hold_threshold_ms {
        let clamped = v.clamp(50, 2000);
        if settings.hold_threshold_ms != clamped {
            settings.hold_threshold_ms = clamped;
            changed.push("hold_threshold_ms".to_string());
        }
    }
    if let Some(v) = patch.key_remap_enabled {
        if settings.key_remap_enabled != v {
            settings.key_remap_enabled = v;
            changed.push("key_remap_enabled".to_string());
        }
    }
    if let Some(ref new_bindings) = patch.key_bindings {
        if &settings.key_bindings != new_bindings {
            settings.key_bindings = new_bindings.clone();
            changed.push("key_bindings".to_string());
        }
    }
    if let Some(v) = patch.reduce_camera_shake_pct {
        let clamped = v.clamp(0.0, 1.0);
        if (settings.reduce_camera_shake_pct - clamped).abs() > f32::EPSILON {
            settings.reduce_camera_shake_pct = clamped;
            changed.push("reduce_camera_shake_pct".to_string());
        }
    }
    if let Some(v) = patch.tick_rate_hz {
        // M1: the engine's tick_rate_hz is fixed at construction (so deterministic
        // checksums are per-rate). The setting mirrors that value so cfctl
        // observe.settings round-trips it; we accept the patch but the engine
        // does NOT live-retick to the new rate. A future M5+ command will swap
        // tick rate via scenario reload.
        let v = v.max(1);
        if settings.tick_rate_hz != v {
            settings.tick_rate_hz = v;
            changed.push("tick_rate_hz".to_string());
        }
    }
    // M1 Gap F1-F2: feel cvars (already validated by SettingsPatch::validation_error).
    if let Some(v) = patch.accel {
        if (settings.accel - v).abs() > f32::EPSILON {
            settings.accel = v;
            changed.push("accel".to_string());
        }
    }
    if let Some(v) = patch.friction {
        if (settings.friction - v).abs() > f32::EPSILON {
            settings.friction = v;
            changed.push("friction".to_string());
        }
    }
    if let Some(v) = patch.gravity {
        if (settings.gravity - v).abs() > f32::EPSILON {
            settings.gravity = v;
            changed.push("gravity".to_string());
        }
    }
    if let Some(v) = patch.jump_force {
        if (settings.jump_force - v).abs() > f32::EPSILON {
            settings.jump_force = v;
            changed.push("jump_force".to_string());
        }
    }
    if let Some(v) = patch.recoil_decay_per_tick {
        if (settings.recoil_decay_per_tick - v).abs() > f32::EPSILON {
            settings.recoil_decay_per_tick = v;
            changed.push("recoil_decay_per_tick".to_string());
        }
    }
    if let Some(v) = patch.sharp_aim_build_ticks {
        if settings.sharp_aim_build_ticks != v {
            settings.sharp_aim_build_ticks = v;
            changed.push("sharp_aim_build_ticks".to_string());
        }
    }
    if let Some(v) = patch.walk_threshold {
        if (settings.walk_threshold - v).abs() > f32::EPSILON {
            settings.walk_threshold = v;
            changed.push("walk_threshold".to_string());
        }
    }
    if let Some(ref id) = patch.ai_difficulty {
        if settings.ai_difficulty != *id {
            settings.ai_difficulty = id.clone();
            changed.push("ai_difficulty".to_string());
        }
    }
    if let Some(v) = patch.ai_debug {
        if settings.ai_debug != v {
            settings.ai_debug = v;
            changed.push("ai_debug".to_string());
        }
    }
    // === M8 accessibility / camera / debug / locale extensions ===
    if let Some(ref s) = patch.game_speed_assist {
        if let Some(parsed) = crate::settings::GameSpeedAssist::from_str(s) {
            if settings.game_speed_assist != parsed {
                settings.game_speed_assist = parsed;
                changed.push("game_speed_assist".to_string());
            }
        }
    }
    if let Some(ref s) = patch.color_cue_mode {
        if let Some(parsed) = crate::settings::ColorCueMode::from_str(s) {
            if settings.color_cue_mode != parsed {
                settings.color_cue_mode = parsed;
                changed.push("color_cue_mode".to_string());
            }
        }
    }
    if let Some(ref s) = patch.aim_assist {
        if let Some(parsed) = crate::settings::AimAssist::from_str(s) {
            if settings.aim_assist != parsed {
                settings.aim_assist = parsed;
                changed.push("aim_assist".to_string());
            }
        }
    }
    if let Some(v) = patch.damage_numbers {
        if settings.damage_numbers != v {
            settings.damage_numbers = v;
            changed.push("damage_numbers".to_string());
        }
    }
    if let Some(v) = patch.killcam_enabled {
        if settings.killcam_enabled != v {
            settings.killcam_enabled = v;
            changed.push("killcam_enabled".to_string());
        }
    }
    if let Some(v) = patch.hit_stop_enabled {
        if settings.hit_stop_enabled != v {
            settings.hit_stop_enabled = v;
            changed.push("hit_stop_enabled".to_string());
        }
    }
    if let Some(v) = patch.cinematic_kills {
        if settings.cinematic_kills != v {
            settings.cinematic_kills = v;
            changed.push("cinematic_kills".to_string());
        }
    }
    if let Some(v) = patch.mini_map_enabled {
        if settings.mini_map_enabled != v {
            settings.mini_map_enabled = v;
            changed.push("mini_map_enabled".to_string());
        }
    }
    if let Some(v) = patch.compass_enabled {
        if settings.compass_enabled != v {
            settings.compass_enabled = v;
            changed.push("compass_enabled".to_string());
        }
    }
    if let Some(v) = patch.damage_direction_enabled {
        if settings.damage_direction_enabled != v {
            settings.damage_direction_enabled = v;
            changed.push("damage_direction_enabled".to_string());
        }
    }
    if let Some(v) = patch.mini_map_zoom {
        let clamped = v.clamp(0.25, 4.0);
        if (settings.mini_map_zoom - clamped).abs() > f32::EPSILON {
            settings.mini_map_zoom = clamped;
            changed.push("mini_map_zoom".to_string());
        }
    }
    if let Some(v) = patch.scope_zoom_fov {
        let clamped = v.clamp(5.0, 90.0);
        if (settings.scope_zoom_fov - clamped).abs() > f32::EPSILON {
            settings.scope_zoom_fov = clamped;
            changed.push("scope_zoom_fov".to_string());
        }
    }
    if let Some(v) = patch.text_scale {
        let clamped = v.clamp(crate::settings::UI_SCALE_MIN, crate::settings::UI_SCALE_MAX);
        if (settings.text_scale - clamped).abs() > f32::EPSILON {
            settings.text_scale = clamped;
            changed.push("text_scale".to_string());
        }
    }
    if let Some(ref s) = patch.ui_density {
        if let Some(parsed) = crate::settings::UiDensity::from_str(s) {
            if settings.ui_density != parsed {
                settings.ui_density = parsed;
                changed.push("ui_density".to_string());
            }
        }
    }
    if let Some(ref s) = patch.language {
        if !s.is_empty() && &settings.language != s {
            settings.language = s.clone();
            changed.push("language".to_string());
        }
    }
    if let Some(v) = patch.speedrun_mode {
        if settings.speedrun_mode != v {
            settings.speedrun_mode = v;
            changed.push("speedrun_mode".to_string());
        }
    }
    if let Some(v) = patch.permadeath {
        if settings.permadeath != v {
            settings.permadeath = v;
            changed.push("permadeath".to_string());
        }
    }
    if let Some(v) = patch.no_respawn {
        if settings.no_respawn != v {
            settings.no_respawn = v;
            changed.push("no_respawn".to_string());
        }
    }
    if let Some(v) = patch.fog_of_war_on {
        if settings.fog_of_war_on != v {
            settings.fog_of_war_on = v;
            changed.push("fog_of_war_on".to_string());
        }
    }
    if let Some(v) = patch.limited_ammo {
        if settings.limited_ammo != v {
            settings.limited_ammo = v;
            changed.push("limited_ammo".to_string());
        }
    }
    if let Some(v) = patch.time_limit {
        if settings.time_limit != v {
            settings.time_limit = v;
            changed.push("time_limit".to_string());
        }
    }
    if let Some(v) = patch.no_minimap {
        if settings.no_minimap != v {
            settings.no_minimap = v;
            changed.push("no_minimap".to_string());
        }
    }
    if let Some(v) = patch.hardcore_mode {
        if settings.hardcore_mode != v {
            settings.hardcore_mode = v;
            changed.push("hardcore_mode".to_string());
        }
    }
    if let Some(v) = patch.friendly_fire_on {
        if settings.friendly_fire_on != v {
            settings.friendly_fire_on = v;
            changed.push("friendly_fire_on".to_string());
        }
    }
    if let Some(v) = patch.debug_enabled {
        if settings.debug_enabled != v {
            settings.debug_enabled = v;
            changed.push("debug_enabled".to_string());
        }
    }
    // === M11 ACC-A floor (DR-003 + DR-012 closure) ===
    if let Some(ref s) = patch.contrast_mode {
        if let Some(parsed) = crate::settings::ContrastMode::from_str(s) {
            if settings.contrast_mode != parsed {
                settings.contrast_mode = parsed;
                // Mirror to the legacy bool so M8 surfaces keep working.
                let want_high_contrast = !matches!(parsed, crate::settings::ContrastMode::Standard);
                if settings.high_contrast != want_high_contrast {
                    settings.high_contrast = want_high_contrast;
                    changed.push("high_contrast".to_string());
                }
                changed.push("contrast_mode".to_string());
            }
        }
    }
    if let Some(ref s) = patch.caption_mode {
        if let Some(parsed) = crate::settings::CaptionMode::from_str(s) {
            if settings.caption_mode != parsed {
                settings.caption_mode = parsed;
                let want_captions = !matches!(parsed, crate::settings::CaptionMode::Off);
                if settings.captions != want_captions {
                    settings.captions = want_captions;
                    changed.push("captions".to_string());
                }
                changed.push("caption_mode".to_string());
            }
        }
    }
    if let Some(v) = patch.caption_background_opacity {
        let clamped = v.clamp(0.0, 1.0);
        if (settings.caption_background_opacity - clamped).abs() > f32::EPSILON {
            settings.caption_background_opacity = clamped;
            changed.push("caption_background_opacity".to_string());
        }
    }
    if let Some(ref cats) = patch.caption_categories {
        if &settings.caption_categories != cats {
            settings.caption_categories = cats.clone();
            changed.push("caption_categories".to_string());
        }
    }
    if let Some(ref s) = patch.input_profile {
        if let Some(parsed) = crate::settings::InputProfile::from_str(s) {
            if settings.input_profile != parsed {
                settings.input_profile = parsed;
                changed.push("input_profile".to_string());
            }
        }
    }
    if let Some(ref groups) = patch.remap_groups {
        if &settings.remap_groups != groups {
            settings.remap_groups = groups.clone();
            changed.push("remap_groups".to_string());
        }
    }
    if let Some(ref s) = patch.hold_behavior {
        if let Some(parsed) = crate::settings::HoldBehavior::from_str(s) {
            if settings.hold_behavior != parsed {
                settings.hold_behavior = parsed;
                changed.push("hold_behavior".to_string());
            }
        }
    }
    if let Some(v) = patch.screen_shake_scale {
        let clamped = v.clamp(0.0, 1.0);
        if (settings.screen_shake_scale - clamped).abs() > f32::EPSILON {
            settings.screen_shake_scale = clamped;
            // Mirror to the inverse-sense legacy field so cf-app's existing
            // camera-shake path keeps working: legacy = 1.0 - scale.
            let legacy = 1.0 - clamped;
            settings.reduce_camera_shake_pct = legacy;
            changed.push("screen_shake_scale".to_string());
        }
    }
    if let Some(ref s) = patch.camera_motion {
        if let Some(parsed) = crate::settings::CameraMotion::from_str(s) {
            if settings.camera_motion != parsed {
                settings.camera_motion = parsed;
                changed.push("camera_motion".to_string());
            }
        }
    }
    if let Some(ref s) = patch.objective_help {
        if let Some(parsed) = crate::settings::ObjectiveHelp::from_str(s) {
            if settings.objective_help != parsed {
                settings.objective_help = parsed;
                changed.push("objective_help".to_string());
            }
        }
    }
    if let Some(ref s) = patch.debug_explainer_level {
        if let Some(parsed) = crate::settings::DebugExplainerLevel::from_str(s) {
            if settings.debug_explainer_level != parsed {
                settings.debug_explainer_level = parsed;
                changed.push("debug_explainer_level".to_string());
            }
        }
    }
    // === M12 cinematic story beats ===
    if let Some(ref s) = patch.comic_style_overlay {
        if let Some(parsed) = crate::settings::ComicStyleOverlay::from_str(s) {
            if settings.comic_style_overlay != parsed {
                settings.comic_style_overlay = parsed;
                changed.push("comic_style_overlay".to_string());
            }
        }
    }
    if let Some(v) = patch.comic_death_recap {
        if settings.comic_death_recap != v {
            settings.comic_death_recap = v;
            changed.push("comic_death_recap".to_string());
        }
    }
    changed
}

// **M12C**: Cinematic kernel integration helpers — inherent methods on
// `M0Engine` (sync; called by cf-shell mission-load hooks + the cf-app
// per-frame loop). The async dispatch methods (`act_player_*` /
// `dump_cinematic_state`) live in the `EngineHandle` trait impl below.
