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







/// [`M0Engine::snapshot_world_save`] stashes the M14C/D/E/F/G runtime
/// state in [`cf_save::WorldSave::mod_payload`]. Append-only — readers
/// that don't understand this key still round-trip the value verbatim
/// per the SaveBlob "Mod-extending fields survive migration" contract.
pub(crate) const M14_SAVE_EXTENSION_KEY: &str = "corefall.m14_state";

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
    /// rifle_preset, ammo_in_mag, reload_remaining_ticks,
    /// fire_cooldown_ticks)`. Restored alongside the SaveBlob's
    /// per-actor rifle fields so the loaded engine's `sim.rifles`
    /// matches the source state for VAL-CROSS-029 byte-equality.
    pub rifle_states: Vec<RifleStateSnapshot>,
}

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
pub(crate) const MILESTONE_INDEX_M0: u32 = 0;
pub(crate) const MILESTONE_INDEX_M1: u32 = 1;
pub(crate) const MILESTONE_INDEX_M1_5: u32 = 2;
pub(crate) const MILESTONE_INDEX_M2: u32 = 3;
pub(crate) const MILESTONE_INDEX_M3A: u32 = 5;
pub(crate) const MILESTONE_INDEX_UNKNOWN: u32 = 999;

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









// `M0Engine` (sync; called by cf-shell mission-load hooks + the cf-app
// per-frame loop). The async dispatch methods (`act_player_*` /
// `dump_cinematic_state`) live in the `EngineHandle` trait impl below.
