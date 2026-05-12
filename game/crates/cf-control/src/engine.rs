//! M0 inline engine: drives the fixed-tick sim, emits the lock-approved event
//! categories (`system`, `control`, `determinism`), writes a run bundle, and
//! exposes an `EngineHandle` so the WebSocket server can drive the same engine.

use std::{
    collections::{BTreeMap, VecDeque},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::Instant,
};

/// M4A HUD banner queue cap. Beyond this many entries the FIFO drains; the
/// HUD only draws the highest-priority N each tick.
const M4A_BANNER_BUFFER: usize = 8;
/// M4A captions queue cap. Mirrors banner buffer.
const M4A_CAPTION_BUFFER: usize = 8;
/// M4A banner expiry (ticks). Status banners auto-clear after ~3 seconds at
/// 60 Hz so the HUD stays readable; mission banners stay until end-of-run.
const M4A_STATUS_BANNER_EXPIRY_TICKS: u64 = 180;
/// M4A captions expiry (ticks). Captions auto-clear after ~2 seconds at 60 Hz.
const M4A_CAPTION_EXPIRY_TICKS: u64 = 120;

/// **M4A canonical focusable HUD node list** (DR-012 ACC-A-04). This is the
/// single source of truth: cf-control's `observe.accessibility.focusable_nodes`,
/// cf-app's keyboard focus traversal, cf-e2e's `--verify-focus`, the live-WS
/// acceptance tests, and any future cfctl `ui assert` tooling all read from
/// this constant. Changing the list (adding / removing / renaming a node)
/// MUST update every consumer in the same pass; the cf-e2e + live-WS tests
/// fail-closed when a node is missing from the observed surface.
pub const HUD_FOCUSABLE_NODES: &[&str] = &[
    "hud.status_strip",
    "hud.silhouette",
    "hud.module_strip",
    "hud.stance",
    "hud.objective",
    "hud.mission",
    "hud.enemy",
    "hud.breach",
    "hud.tool",
    "hud.captions",
    "hud.banners",
    "hud.last_event",
];

use chrono::{DateTime, Utc};
use serde_json::json;

use cf_actor::{
    sim::{step as actor_step, ActorSimState, ActorTickOutcome, StepDeps, StepReport},
    ActorId, ActorWorld, ControlIntent, IntentSource, ItemSlot, Vec2,
};
use cf_replay::{
    diagnostics, ArtifactItem, BuildInfo, BundleInputs, CapabilitiesBlock, CaptureConfig, ChecksumConfig, PerfSample,
    Recorder, RunManifest, SceneInfo, SettingsBlock, TestRecord, CONTROL_SCHEMA_VERSION, EVENT_ENVELOPE_VERSION,
    MANIFEST_SCHEMA_VERSION, SCENARIO_SCHEMA_VERSION,
};
use cf_sim_core::{
    checksum::{sim_state_v1, CHECKSUM_ALGORITHM, CHECKSUM_SCOPE},
    ids::{iso_hyphen_safe, make_run_id},
    Rng, SimClock, SimConfig, Tick, WallClock,
};

use crate::{
    scenario::Scenario,
    server::{async_trait, CommandResult, ControlCommand, EngineHandle, SettingsPatch},
    state::{ActorView, ObserveFrame, ObserveSettings, RunStatus},
    Settings, SCHEMA_VERSION,
};

#[derive(Debug, Clone)]
pub struct M0EngineConfig {
    pub milestone: String,
    pub scenario_id: String,
    pub scenario_path: PathBuf,
    pub seed: u64,
    pub duration_ticks: u64,
    pub run_mode: String,
    pub run_bundle_root: PathBuf,
    pub write_run_bundle: bool,
    pub settings: Settings,
    pub control_api_enabled: bool,
    pub debug_capabilities: Vec<String>,
    pub tick_rate_hz: u32,
    /// **M4A**: source-truthful `capture_config` flag. cf-app sets this when
    /// `--capture-grid` is on so the run manifest reports
    /// `capture_config.{events:true, screenshots:true, captures:true}` instead
    /// of the historical `screenshots:false / captures:false` lie. Default:
    /// false (matches the headless / no-capture-grid path).
    pub capture_grid_enabled: bool,
    /// Region dimensions copied from the scenario manifest (for run-bundle metadata).
    pub region_width: f32,
    pub region_height: f32,
    /// Region bottom-left anchor copied from the scenario manifest. Used together with
    /// `region_width`/`region_height` to derive the world-space X/Y bounds that the sim
    /// step uses for actor clamping and projectile out-of-bounds expiry. Defaults to
    /// `(0.0, 0.0)` for M0/M1 scenarios that anchor at the world origin.
    pub region_anchor_x: f32,
    pub region_anchor_y: f32,
    pub config_hash: String,
    pub commit_sha: String,
    /// True when the bundle was produced from a dirty checkout. `commit_sha`
    /// alone is not enough for BP closure evidence because many audit-fix
    /// iterations can share the same HEAD while running different code.
    pub worktree_dirty: bool,
    /// Fingerprint of tracked diffs + untracked file contents when
    /// `worktree_dirty` is true. Closure tooling uses this to reject stale
    /// same-commit dirty bundles.
    pub worktree_fingerprint: Option<String>,
    /// Dirty paths recorded for reviewer/debug visibility. The fingerprint is
    /// authoritative; this list is the human-readable trail.
    pub worktree_dirty_files: Vec<String>,
    pub rust_version: String,
    pub bevy_version: String,
    pub platform: String,
    pub linked_specs: Vec<String>,
    pub assumptions_tested: Vec<String>,
    pub expected_tests: Vec<String>,
    /// True = pace ticks against wall-clock at `tick_rate_hz`. False = run
    /// as fast as possible (used by short tests / E2E that don't need real
    /// wall-time pacing).
    pub paced: bool,
    /// **DEBUG-ONLY**: if `Some(tick)`, the engine spawns a sub-thread on the first call
    /// to `record_run_started` that sleeps until `tick * tick_dt` then panics. Used to
    /// produce a real run bundle containing a `system.panic` event for M0-008 evidence.
    /// Production runs leave this `None`. Mirrored by `cf-app --debug-inject-panic-at-tick`.
    pub debug_inject_panic_at_tick: Option<u64>,
    /// M1: initial actor world built from the scenario manifest. `None` for M0-style
    /// scenarios (`m0_blank`) where the engine ticks an empty sim with no actors.
    pub initial_actor_world: Option<InitialActorWorld>,
    /// True when the scenario manifest declared at least one typed `actors[]` entry.
    /// Used by the engine to decide whether `act.player.*` commands should be applied
    /// or rejected as `act_player_unavailable_no_actor_world`.
    pub has_actor_world: bool,
    /// M1.5: initial mission/breach/AI state built from the scenario manifest.
    /// `None` for sandbox scenarios (m0_blank, m1_actor_range).
    pub initial_breach_world: Option<InitialBreachWorld>,
    /// M1.5: initial reactive-guard configurations keyed by actor id.
    pub initial_guards: Vec<InitialGuard>,
    /// M1.5: scenario objectives (in declaration order) to feed `cf-mission`.
    pub initial_objectives: Vec<cf_mission::Objective>,
    /// M1.5: loss conditions for the mission (timer + player-dead check).
    pub mission_loss: Option<cf_mission::LossConditions>,
    /// M2: optional chunked pixel terrain authored by the scenario manifest.
    pub initial_chunked_terrain: Option<cf_terrain::ChunkedTerrain>,
    /// M2.5: optional ordered list of reactor world entries.
    pub initial_reactors: Vec<cf_mission::Reactor>,
    /// M3A: configurable checksum cadence. 0 = disabled. Default from
    /// `ChecksumConfig::m0_default().cadence_ticks` (60).
    pub checksum_cadence_ticks: u64,
}

/// M1.5: initial breach world snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct InitialBreachWorld {
    pub world: cf_terrain::BreachWorld,
}

/// M1.5: initial reactive-guard config (one per enemy actor).
#[derive(Debug, Clone, PartialEq)]
pub struct InitialGuard {
    pub actor: ActorId,
    pub params: cf_ai::ReactiveGuardParams,
}

/// Snapshot of the initial actor world. Held in the engine config so `scenario.reset`
/// can rebuild the world without reloading the manifest from disk. Per-actor
/// `RifleState` is built at engine init via [`build_rifles_for_world`] so the
/// configured `tick_rate_hz` is honoured (60 Hz vs 120 Hz produce different tick
/// budgets but the same real-time RPS / reload duration).
#[derive(Debug, Clone, PartialEq)]
pub struct InitialActorWorld {
    pub world: ActorWorld,
    pub player: Option<ActorId>,
}

impl InitialActorWorld {
    pub fn from_scenario(scenario: &Scenario) -> Self {
        Self::from_scenario_with_tick_rate(scenario, 60)
    }

    /// **M5**: build initial actor world with chassis attachment using the
    /// engine's configured tick_rate_hz so chassis eject windows are real-time
    /// stable.
    pub fn from_scenario_with_tick_rate(scenario: &Scenario, tick_rate_hz: u32) -> Self {
        let mut world = ActorWorld::new(scenario.floor_y, scenario.gravity);
        for actor in &scenario.actors {
            let state = actor.build_state_with_tick_rate(tick_rate_hz);
            world.insert(state);
        }
        let player = world.player;
        Self { world, player }
    }
}

/// Build a per-actor `RifleState` map from the world for the configured `tick_rate_hz`.
/// Each actor whose currently-selected slot OR any other slot holds a rifle gets a
/// state entry. We key on inventory contents (not on selection) so swapping slots at
/// runtime never strands an existing rifle's ammo / cooldown.
fn build_rifles_for_world(world: &ActorWorld, tick_rate_hz: u32) -> BTreeMap<ActorId, cf_equipment::RifleState> {
    let mut rifles = BTreeMap::new();
    for actor in world.actors.values() {
        for item in &actor.inventory.items {
            if let cf_actor::InventoryItem::Rifle { preset } = item {
                if let Some(spec) = cf_equipment::rifle_preset(preset) {
                    rifles.insert(actor.id, cf_equipment::RifleState::new(spec, tick_rate_hz));
                    break;
                }
            }
        }
    }
    rifles
}

impl M0EngineConfig {
    /// **TEST-ONLY** bare-bones default. Production code MUST NOT call this — it bypasses
    /// the scenario manifest (no real `seed`/`duration_ticks`/`expected_tests`/`region`)
    /// and ships zero-defaults that will leak into the run bundle.
    ///
    /// Production callers MUST go through [`crate::runtime::build_engine_config`] (which in
    /// turn calls [`Self::for_loaded_scenario`]) so every field reflects the actual scenario
    /// manifest + CLI overrides + real build metadata.
    ///
    /// The `for_test_scenario_only` name is intentionally awkward to make accidental
    /// production use stand out in code review and grep.
    #[doc(hidden)]
    pub fn for_test_scenario_only(scenario_id: &str, scenario_path: PathBuf) -> Self {
        Self {
            milestone: "m0".to_string(),
            scenario_id: scenario_id.to_string(),
            scenario_path,
            seed: 0,
            duration_ticks: 0,
            run_mode: "headless-smoke".to_string(),
            run_bundle_root: PathBuf::from("prototype_runs/native"),
            write_run_bundle: false,
            settings: Settings::default(),
            control_api_enabled: false,
            debug_capabilities: Vec::new(),
            tick_rate_hz: 60,
            capture_grid_enabled: false,
            region_width: 0.0,
            region_height: 0.0,
            region_anchor_x: 0.0,
            region_anchor_y: 0.0,
            config_hash: String::new(),
            commit_sha: env!("CARGO_PKG_VERSION").to_string(),
            worktree_dirty: false,
            worktree_fingerprint: None,
            worktree_dirty_files: Vec::new(),
            rust_version: rustc_version_string(),
            bevy_version: bevy_version_string(),
            platform: env_platform(),
            linked_specs: vec![
                "spec/prototype-roadmap".to_string(),
                "spec/native-implementation-backlog".to_string(),
                "spec/ai-control-observability-layer".to_string(),
            ],
            assumptions_tested: vec![
                "Workspace builds and ticks the fixed sim for the configured duration.".to_string(),
                "Run bundle conforms to the run-bundle checker (DR-002 v1 lock).".to_string(),
                "Settings + capabilities are observable through cf-control.".to_string(),
            ],
            expected_tests: Vec::new(),
            paced: false,
            debug_inject_panic_at_tick: None,
            initial_actor_world: None,
            has_actor_world: false,
            initial_breach_world: None,
            initial_guards: Vec::new(),
            initial_objectives: Vec::new(),
            mission_loss: None,
            initial_chunked_terrain: None,
            initial_reactors: Vec::new(),
            checksum_cadence_ticks: ChecksumConfig::m0_default().cadence_ticks,
        }
    }

    /// Build a config from a scenario manifest. Pulls `seed`, `duration_ticks`, `expected_tests`,
    /// and `region` straight out of the loaded `Scenario`. The CLI may still override individual
    /// fields after this call.
    pub fn for_loaded_scenario(scenario: &crate::scenario::Scenario, scenario_path: PathBuf) -> Self {
        let mut cfg = Self::for_test_scenario_only(&scenario.id, scenario_path.clone());
        cfg.seed = scenario.seed;
        cfg.duration_ticks = scenario.duration_ticks.unwrap_or(0);
        cfg.expected_tests = if scenario.expected_tests.is_empty() {
            vec!["M0-SMOKE-01".to_string()]
        } else {
            scenario.expected_tests.clone()
        };
        cfg.region_width = scenario.region.width;
        cfg.region_height = scenario.region.height;
        cfg.region_anchor_x = scenario.region.anchor.0;
        cfg.region_anchor_y = scenario.region.anchor.1;
        if scenario.has_actor_world() {
            cfg.has_actor_world = true;
            cfg.initial_actor_world = Some(InitialActorWorld::from_scenario(scenario));
            // Bump the milestone hint when the scenario actually carries an actor world.
            // Per-actor RifleState is built lazily in M0Engine::new with the configured
            // tick_rate_hz so 60 Hz vs 120 Hz produce identical real-time RPS / reload.
            cfg.milestone = "m1".to_string();
        }
        // M1.5: breach world.
        if !scenario.breaches.is_empty() {
            let strips: Vec<_> = scenario.breaches.iter().map(|b| b.build_strip()).collect();
            cfg.initial_breach_world = Some(InitialBreachWorld {
                world: cf_terrain::BreachWorld::new(strips),
            });
        }
        // M1.5: reactive guards.
        for actor in &scenario.actors {
            if let Some(enemy) = &actor.enemy {
                cfg.initial_guards.push(InitialGuard {
                    actor: ActorId(actor.id),
                    params: enemy.build_params(),
                });
            }
        }
        // M1.5: objectives + mission loss conditions.
        if scenario.has_mission() {
            cfg.initial_objectives = scenario
                .objectives
                .iter()
                .cloned()
                .map(|o| o.into_objective())
                .collect();
            cfg.mission_loss = Some(
                scenario
                    .mission
                    .as_ref()
                    .map(|m| m.loss_conditions())
                    .unwrap_or_default(),
            );
            cfg.milestone = "m1.5".to_string();
        }
        // M2: chunked terrain.
        if let Some(t) = &scenario.terrain {
            // We unwrap on the manifest-validated build because validate() has
            // already rejected unknown materials before this point. If the
            // build still fails (e.g. width 0), we leave terrain empty and
            // record a tracing warning so the milestone bug surfaces.
            let scenario_path_str = scenario_path.display().to_string();
            match t.build_terrain(&scenario_path_str) {
                Ok(terrain) => {
                    cfg.initial_chunked_terrain = Some(terrain);
                    cfg.milestone = "m2".to_string();
                }
                Err(err) => {
                    tracing::warn!(
                        target: "cf::control",
                        error = %err,
                        scenario_id = %scenario.id,
                        "scenario chunked terrain failed to build; engine will run without terrain"
                    );
                }
            }
        }
        // M2.5: reactor world.
        if scenario.has_reactors() {
            cfg.initial_reactors = scenario.reactors.iter().map(|r| r.build_reactor()).collect();
            cfg.milestone = "m2.5".to_string();
        }
        // **M5**: any actor with a chassis attached bumps the milestone tag to m5
        // so run bundles produced by chassis scenarios route into the m5_* slot
        // under prototype_runs/native/.
        if scenario.actors.iter().any(|a| a.chassis.is_some()) {
            cfg.milestone = "m5".to_string();
        }
        // M4A: explicit milestone override wins over scenario-shape derivation
        // so a scenario can reuse the M1.5 / M2 / M2.5 world while tagging the
        // run bundle for the actual milestone being proven.
        if let Some(override_str) = scenario.milestone_override.as_deref() {
            let trimmed = override_str.trim();
            if !trimmed.is_empty() {
                cfg.milestone = trimmed.to_lowercase();
            }
        }
        cfg
    }

    pub fn config_hash_input(&self) -> String {
        format!(
            "milestone={}|scenario={}|seed={}|ticks={}|hz={}|region={:?}|mode={}|control_api={}|debug={}|settings={:?}|expected_tests={:?}|has_actor_world={}",
            self.milestone,
            self.scenario_id,
            self.seed,
            self.duration_ticks,
            self.tick_rate_hz,
            (
                self.region_anchor_x,
                self.region_anchor_y,
                self.region_width,
                self.region_height,
            ),
            self.run_mode,
            self.control_api_enabled,
            self.debug_capabilities.join(","),
            self.settings,
            self.expected_tests,
            self.has_actor_world,
        )
    }

    pub fn fill_config_hash(&mut self) {
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.config_hash_input().as_bytes());
        let h = hasher.finalize();
        self.config_hash = hex::encode(&h.as_bytes()[..16]);
    }
}

fn env_platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

fn rustc_version_string() -> String {
    option_env!("CFAPP_RUSTC_VERSION").unwrap_or("unknown").to_string()
}

fn bevy_version_string() -> String {
    option_env!("CFAPP_BEVY_VERSION")
        .unwrap_or(BEVY_VERSION_FALLBACK)
        .to_string()
}

const BEVY_VERSION_FALLBACK: &str = "0.18.1";

/// Record a `system.panic` event into a recorder + bump the `error` severity counter.
/// `tick` / `sim_time_ms` should be the engine's current values so the event slots into
/// `events.jsonl` in monotonic-tick order. Pulled out of the engine's panic-reporter
/// closure so the M0-008 unit test can drive the same code path without depending on the
/// global `PANIC_REPORTER` slot (which test parallelism races).
pub fn report_panic_to_recorder(recorder: &Arc<Recorder>, tick: u64, sim_time_ms: f64, msg: &str) {
    recorder.record_severity("error");
    recorder.record(
        Tick(tick),
        sim_time_ms,
        "system",
        "panic",
        json!({"message": msg}),
        None,
    );
}

/// Upper bound on the number of per-tick duration samples retained in
/// `EngineMutable::tick_durations_us`. Only the last `cadence_ticks` entries are ever
/// read by `TickSampleStats::from_recent`, so anything above this cap is dead weight.
/// Set well above the default 60 Hz cadence to leave headroom for higher tick rates
/// and larger checksum cadences without trimming on every tick.
const TICK_DURATIONS_HISTORY_CAP: usize = 4096;

/// Periodic per-tick performance stats emitted as `system.tick_sample`. Keeps M0 evidence
/// of per-tick cost in the run bundle without waiting for the M3 perf overlay.
#[derive(Debug, Clone, Copy)]
struct TickSampleStats {
    /// How many ticks of history this sample summarized.
    window_ticks: u64,
    avg_tick_ms: f64,
    max_tick_ms: f64,
    p99_tick_ms: f64,
    /// Actual number of stored samples used (may be less than `window_ticks` early in a run).
    samples_observed: u64,
}

impl TickSampleStats {
    fn from_recent(samples_us: &[u64], window: usize) -> Self {
        let take = samples_us.len().min(window);
        let slice = &samples_us[samples_us.len() - take..];
        if slice.is_empty() {
            return Self {
                window_ticks: window as u64,
                avg_tick_ms: 0.0,
                max_tick_ms: 0.0,
                p99_tick_ms: 0.0,
                samples_observed: 0,
            };
        }
        let mut sorted: Vec<u64> = slice.to_vec();
        sorted.sort_unstable();
        let avg_us = slice.iter().copied().sum::<u64>() as f64 / slice.len() as f64;
        let max_us = *sorted.last().unwrap() as f64;
        let p99_idx = ((slice.len() as f64 * 0.99) as usize).min(slice.len() - 1);
        let p99_us = sorted[p99_idx] as f64;
        Self {
            window_ticks: window as u64,
            avg_tick_ms: avg_us / 1000.0,
            max_tick_ms: max_us / 1000.0,
            p99_tick_ms: p99_us / 1000.0,
            samples_observed: slice.len() as u64,
        }
    }
}

#[derive(Debug, Clone)]
pub struct M0EngineOutcome {
    pub run_id: String,
    pub bundle_dir: Option<PathBuf>,
    pub final_checksum_hex: Option<String>,
    pub ticks_run: u64,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub wall_seconds: f64,
}

pub struct M0Engine {
    config: M0EngineConfig,
    state: RwLock<EngineMutable>,
    recorder: Arc<Recorder>,
    /// Lock-free snapshot of the engine's current tick. Updated by `drive_tick` so that
    /// the panic reporter (which fires from a panicking thread, possibly while another
    /// thread holds `state`) can record `system.panic` at the right tick without
    /// blocking on the `RwLock`. Also drives `sim_time_ms` for the panic event.
    current_tick: Arc<std::sync::atomic::AtomicU64>,
    started_at: DateTime<Utc>,
    started_instant: Instant,
    run_bundle_dir: PathBuf,
}

struct EngineMutable {
    clock: SimClock,
    rng: Rng,
    settings: Settings,
    pending_runbundle: bool,
    shutdown_requested: bool,
    tick_durations_us: Vec<u64>,
    /// M1: pending player intent for the next tick. The dispatch handlers update fields
    /// here; `drive_tick` consumes the intent, applies it, then clears the edge-triggered
    /// fields. Continuous fields (`move_x`, `aim`) persist tick-to-tick.
    pending_intent: ControlIntent,
    /// M1: actor world + rifles + projectiles. `None` for M0 scenarios.
    actor_state: Option<ActorSimState>,
    /// Cached player actor id from the actor world for fast access.
    player_actor: Option<ActorId>,
    /// Monotonic counter incremented whenever `pending_intent` is externally
    /// reset (e.g. `scenario.reset` zeroes it). Edge-detecting input bridges
    /// (`cf-app::ingest_player_input`) watch this to know when their cached
    /// "last sent" trackers are stale and must redispatch held keys, even if
    /// the keyboard state itself has not changed.
    intent_epoch: u64,
    /// M1.5: breach world (soft-breach strips). `None` when scenario has no breaches.
    breach_world: Option<cf_terrain::BreachWorld>,
    /// M1.5: pending dig request consumed at the start of the next tick.
    /// `Some` only when an `act.player.dig` arrived since the last tick.
    pending_dig: Option<PendingDig>,
    /// M1.5: per-actor reactive-guard controllers, keyed by actor id.
    reactive_guards: BTreeMap<ActorId, cf_ai::ReactiveGuard>,
    /// M1.5: mission state machine. `None` when the scenario is sandbox-only.
    mission: Option<cf_mission::MissionState>,
    /// M1.5: monotonic id counter for guard projectiles. We share the actor
    /// projectile pool but allocate ids from a separate range so guard shots
    /// don't alias the player's projectile_id space across resets.
    next_guard_projectile_id: u64,
    /// M2: chunked pixel terrain. `None` for scenarios that have not opted
    /// into chunked terrain. Coexists with `breach_world`.
    chunked_terrain: Option<cf_terrain::ChunkedTerrain>,
    /// M2.5: reactor world (damageable static actors). `None` when no reactor
    /// is declared.
    reactor_world: Option<cf_mission::ReactorWorld>,
    /// M4A: HUD banner queue. Latest entries are pushed to the back; FIFO
    /// drain caps the queue at `M4A_BANNER_BUFFER`. The HUD draws the highest
    /// `severity` (critical > warning > info) entries first per priority +
    /// raised_at_tick FIFO. Replay events are NOT re-derived from the queue;
    /// they live in `events.jsonl`.
    hud_banners: VecDeque<crate::state::HudBannerView>,
    /// M4A: captions queue (audio-bound events surfaced as text). Drains FIFO
    /// at `M4A_CAPTION_BUFFER`. The HUD draws the most recent N entries when
    /// `Settings.captions == true`.
    hud_captions: VecDeque<crate::state::CaptionView>,
    /// M4A: tool-validity tracker (last carve / last refusal). Updated per
    /// tick by the dig pipeline.
    hud_tool_validity: crate::state::ToolValidityView,
    /// M4A: previous tick's per-actor status, used to detect state changes
    /// that should raise a banner without scanning the full event log.
    hud_last_status: BTreeMap<ActorId, cf_actor::Status>,
    /// M4A: previous tick's mission result, used to detect mission_resolved
    /// transitions for banner emission.
    hud_last_mission_result: Option<String>,
    /// M4A: HUD focus state (DR-012 ACC-A-04). The cf-app keyboard layer +
    /// cfctl `act.input.focus` advance/retreat focus through the canonical
    /// `HUD_FOCUSABLE_NODES` list; observe.accessibility surfaces it.
    hud_focus_index: Option<usize>,
    hud_focus_cycle: u64,
    /// **M5**: previous tick's chassis stage on the player actor (used to
    /// raise stage-change banners without scanning the event log).
    hud_last_chassis_stage: Option<cf_chassis::ChassisStage>,
    /// **M5**: previous tick's pilot state.
    hud_last_pilot_state: Option<cf_chassis::PilotState>,
}

/// Pending dig request set by `act.player.dig` and consumed at the start of the
/// next tick.
#[derive(Debug, Clone)]
struct PendingDig {
    target: Option<String>,
    source: IntentSource,
}

fn observed_run_status(state: &EngineMutable) -> RunStatus {
    if state.shutdown_requested {
        return RunStatus::Ended;
    }
    match state.clock.mode() {
        cf_sim_core::SimMode::Running => RunStatus::Running,
        cf_sim_core::SimMode::Paused => RunStatus::Paused,
        cf_sim_core::SimMode::Stepping(_) => RunStatus::Stepping,
    }
}

impl M0Engine {
    pub fn new(mut config: M0EngineConfig) -> Self {
        if config.config_hash.is_empty() {
            config.fill_config_hash();
        }
        let started_at = WallClock.now_utc();
        let started_instant = Instant::now();
        let started_iso = iso_hyphen_safe(started_at);
        let run_id = make_run_id(&config.milestone, &started_iso, config.seed, &config.scenario_id);
        let run_bundle_dir = config.run_bundle_root.join(&run_id);
        let recorder = Arc::new(Recorder::new(run_id.clone()));
        let clock = SimClock::new(SimConfig {
            tick_rate_hz: config.tick_rate_hz,
        });
        let rng = Rng::from_seed(config.seed);
        let initial_settings = config.settings.clone();
        let current_tick = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let tick_dt_ms = 1000.0 / f64::from(config.tick_rate_hz.max(1));
        let (actor_state, player_actor) = if let Some(initial) = &config.initial_actor_world {
            let mut world = initial.world.clone();
            // **M5**: ensure chassis eject windows are sized for the engine's
            // configured tick_rate_hz so 60 Hz vs 120 Hz produce identical
            // real-time eject windows. The InitialActorWorld is built at
            // 60 Hz default; we adjust each chassis's ticks_total here when
            // the engine ticks at a different rate.
            if config.tick_rate_hz != 60 {
                let scale = config.tick_rate_hz as f32 / 60.0;
                for actor in world.actors.values_mut() {
                    if let Some(chassis) = actor.chassis.as_mut() {
                        chassis.tick_rate_hz = config.tick_rate_hz;
                        let new_ticks = ((chassis.eject_window.ticks_total as f32) * scale).round() as u32;
                        chassis.eject_window.ticks_total = new_ticks.max(1);
                    }
                }
            }
            let mut sim_state = ActorSimState::new(world.clone());
            for (id, rifle) in build_rifles_for_world(&world, config.tick_rate_hz) {
                sim_state.ensure_rifle_for(id, rifle);
            }
            (Some(sim_state), initial.player)
        } else {
            (None, None)
        };
        let pending_intent = ControlIntent::new(player_actor.unwrap_or(ActorId(0)), IntentSource::Cfctl);

        // M1.5: breach world + mission + reactive guards.
        let breach_world = config.initial_breach_world.as_ref().map(|b| b.world.clone());
        // M2: chunked terrain (cloned from the immutable manifest snapshot).
        let chunked_terrain = config.initial_chunked_terrain.clone();
        // M2.5: reactor world.
        let reactor_world = if config.initial_reactors.is_empty() {
            None
        } else {
            Some(cf_mission::ReactorWorld::new(config.initial_reactors.clone()))
        };
        let mut reactive_guards = BTreeMap::new();
        for guard in &config.initial_guards {
            reactive_guards.insert(guard.actor, cf_ai::ReactiveGuard::new(guard.actor, guard.params));
        }
        let mission = if config.initial_objectives.is_empty() && config.mission_loss.is_none() {
            None
        } else {
            Some(cf_mission::MissionState::new(
                config.initial_objectives.clone(),
                0,
                config.mission_loss.unwrap_or_default(),
            ))
        };

        diagnostics::set_panic_reporter({
            let recorder = recorder.clone();
            let tick_snap = current_tick.clone();
            move |msg| {
                let t = tick_snap.load(std::sync::atomic::Ordering::Relaxed);
                report_panic_to_recorder(&recorder, t, t as f64 * tick_dt_ms, msg);
            }
        });

        Self {
            config,
            state: RwLock::new(EngineMutable {
                clock,
                rng,
                settings: initial_settings,
                pending_runbundle: false,
                shutdown_requested: false,
                tick_durations_us: Vec::with_capacity(1024),
                pending_intent,
                actor_state,
                player_actor,
                intent_epoch: 0,
                breach_world,
                pending_dig: None,
                reactive_guards,
                mission,
                next_guard_projectile_id: 1_000_000,
                chunked_terrain,
                reactor_world,
                hud_banners: VecDeque::new(),
                hud_captions: VecDeque::new(),
                hud_tool_validity: crate::state::ToolValidityView::default(),
                hud_last_status: BTreeMap::new(),
                hud_last_mission_result: None,
                hud_focus_index: None,
                hud_focus_cycle: 0,
                hud_last_chassis_stage: None,
                hud_last_pilot_state: None,
            }),
            recorder,
            current_tick,
            started_at,
            started_instant,
            run_bundle_dir,
        }
    }

    pub fn run_id(&self) -> &str {
        self.recorder.run_id()
    }

    pub fn recorder(&self) -> Arc<Recorder> {
        self.recorder.clone()
    }

    pub fn run_bundle_dir(&self) -> &Path {
        &self.run_bundle_dir
    }

    pub fn config(&self) -> &M0EngineConfig {
        &self.config
    }

    /// M4A: live settings accessor for cf-app's HUD + UiScale bridge. Reflects
    /// any `act.settings.set` patches applied since startup, NOT the config
    /// snapshot in `M0EngineConfig.settings`.
    pub fn current_settings(&self) -> Settings {
        self.state.read().map(|s| s.settings.clone()).unwrap_or_default()
    }

    /// M4A: snapshot of the current HUD-state caches (banners, captions,
    /// tool_validity). cf-app reads this per frame to populate HudState
    /// without locking the WebSocket observe path. The accessor always
    /// returns owned clones so cf-app can keep them across frames.
    pub fn hud_caches_snapshot(&self) -> HudCachesSnapshot {
        let s = self.state.read().expect("engine state poisoned");
        HudCachesSnapshot {
            banners: s.hud_banners.iter().cloned().collect(),
            captions: s.hud_captions.iter().cloned().collect(),
            tool_validity: s.hud_tool_validity.clone(),
            focused_node: s.hud_focus_index.map(|i| HUD_FOCUSABLE_NODES[i].to_string()),
            focus_cycle: s.hud_focus_cycle,
        }
    }

    pub fn record_run_started(&self) {
        let state = self.state.read().expect("engine state poisoned");
        let tick = state.clock.tick();
        let sim_time_ms = state.clock.sim_time_ms();
        let settings_value = serde_json::to_value(&state.settings).unwrap_or(serde_json::Value::Null);
        drop(state);
        let started_id = self.recorder.record(
            tick,
            sim_time_ms,
            "system",
            "run_started",
            json!({
                "scenario": self.config.scenario_id,
                "seed": self.config.seed,
                "tick_rate_hz": self.config.tick_rate_hz,
                "run_mode": self.config.run_mode,
                "control_api": self.config.control_api_enabled,
                "protocol_version": crate::SCHEMA_VERSION,
                "settings": settings_value,
            }),
            None,
        );
        self.emit_initial_snapshots(tick, sim_time_ms, &started_id);
        self.emit_category_baseline(tick, sim_time_ms, &started_id);
        self.spawn_debug_panic_if_requested();
    }

    /// M3A item 91: emit a `system.category_baseline` event listing every event
    /// category the engine is aware of. Categories without active producers at
    /// this BP are marked `status: "registered"` so the run bundle proves the
    /// taxonomy is declared even before producers ship.
    fn emit_category_baseline(&self, tick: Tick, sim_time_ms: f64, parent_event_id: &str) {
        let categories = vec![
            ("input", "active"),
            ("control", "active"),
            ("actor", "active"),
            ("equipment", "active"),
            ("combat", "active"),
            ("terrain", "active"),
            ("mission", "active"),
            ("ai", "active"),
            ("snapshot", "active"),
            ("determinism", "active"),
            ("system", "active"),
            ("chassis", "active"),
            ("capture", "active"),
            ("mind", "registered"),
            ("collision", "registered"),
            ("server", "registered"),
            ("anti_cheat", "registered"),
            ("mmo", "registered"),
            ("material", "registered"),
            ("reaction", "registered"),
            ("atmospherics", "registered"),
            ("affliction", "registered"),
            ("body", "registered"),
            ("logistics", "registered"),
            ("ux", "registered"),
            ("accessibility", "registered"),
            ("performance", "registered"),
        ];
        self.recorder.record(
            tick,
            sim_time_ms,
            "system",
            "category_baseline",
            json!({
                "categories": categories.iter()
                    .map(|(name, status)| json!({"name": name, "status": status}))
                    .collect::<Vec<_>>(),
                "total": categories.len(),
                "active": categories.iter().filter(|(_, s)| *s == "active").count(),
            }),
            Some(parent_event_id.to_string()),
        );
    }

    /// M3A-002: emit `snapshot.snapshot_actor`, `snapshot.snapshot_inventory`,
    /// and `snapshot.snapshot_terrain_chunk` events at scenario start so the
    /// cf-headless replay verifier (and any future M3B viewer) can reconstruct
    /// the world without re-loading the manifest from disk. Snapshots are
    /// emitted again on every objective change inside `drive_tick`.
    fn emit_initial_snapshots(&self, tick: Tick, sim_time_ms: f64, parent_event_id: &str) {
        let state = self.state.read().expect("engine state poisoned");
        let actor_state = state.actor_state.as_ref().cloned();
        let chunked_terrain = state.chunked_terrain.as_ref().cloned();
        let reactor_world = state.reactor_world.as_ref().cloned();
        drop(state);
        if let Some(sim) = actor_state {
            for actor in sim.world.actors.values() {
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "snapshot",
                    "snapshot_actor",
                    json!({
                        "actor": actor.id.0,
                        "team": actor.team,
                        "controllable": actor.controllable,
                        "position": [actor.position.x, actor.position.y],
                        "velocity": [actor.velocity.x, actor.velocity.y],
                        "aim": [actor.aim.x, actor.aim.y],
                        "status": actor.status.as_str(),
                        "hp": actor.hp,
                        "hp_max": actor.hp_max,
                        "selected_slot": actor.inventory.selected.0,
                        "kind": "actor",
                    }),
                    Some(parent_event_id.to_string()),
                );
                let rifle_ammo = sim
                    .rifles
                    .get(&actor.id)
                    .map(|r| json!({"ammo_in_mag": r.ammo_in_mag, "mag_capacity": r.spec.mag_capacity, "reloading": r.is_reloading()}))
                    .unwrap_or(json!(null));
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "snapshot",
                    "snapshot_inventory",
                    json!({
                        "actor": actor.id.0,
                        "selected_slot": actor.inventory.selected.0,
                        "items": actor.inventory.items.iter().map(|i| i.label()).collect::<Vec<_>>(),
                        "rifle_state": rifle_ammo,
                    }),
                    Some(parent_event_id.to_string()),
                );
            }
        }
        if let Some(reactors) = reactor_world {
            for r in reactors.iter() {
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
                        "destroyed": r.is_destroyed(),
                    }),
                    Some(parent_event_id.to_string()),
                );
            }
        }
        if let Some(terrain) = chunked_terrain {
            let snapshot = terrain.snapshot();
            for chunk in &snapshot.chunks {
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "snapshot",
                    "snapshot_terrain_chunk",
                    json!({
                        "cx": chunk.coord.cx,
                        "cy": chunk.coord.cy,
                        "default_material": snapshot.default_material,
                        "schema": snapshot.schema,
                        "pixels_len": chunk.pixels.len(),
                        "pixels_blake3": hex::encode(&blake3::hash(&chunk.pixels).as_bytes()[..16]),
                    }),
                    Some(parent_event_id.to_string()),
                );
            }
            self.recorder.record(
                tick,
                sim_time_ms,
                "snapshot",
                "snapshot_terrain_summary",
                json!({
                    "width_px": snapshot.width_px,
                    "height_px": snapshot.height_px,
                    "default_material": snapshot.default_material,
                    "carve_count": snapshot.carve_count,
                    "refusal_count": snapshot.refusal_count,
                    "material_counts": snapshot.material_counts,
                    "allocated_chunks": snapshot.chunks.len(),
                }),
                Some(parent_event_id.to_string()),
            );
        }
    }

    /// **DEBUG-ONLY**: spawn a worker thread that panics at the requested tick if
    /// `config.debug_inject_panic_at_tick` is set. The global panic hook (installed by
    /// `cf_replay::diagnostics::init`) routes the panic into the engine's reporter,
    /// which records `system.panic` + bumps `by_severity.error`. Used to capture M0-008
    /// evidence in real run bundles via `cf-app --debug-inject-panic-at-tick <n>`.
    fn spawn_debug_panic_if_requested(&self) {
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

    /// Drive a single tick. Emits a `determinism.sim_checksum` and a `system.tick_sample`
    /// every `cadence_ticks` ticks (M0 default = 60). When the engine carries an
    /// [`ActorSimState`], drives the M1 actor pipeline and emits the resulting `input.*`
    /// / `actor.*` / `equipment.*` / `combat.*` / `body.*` events.
    pub fn drive_tick(&self) -> Option<Tick> {
        let start = Instant::now();
        let mut state = self.state.write().expect("engine state poisoned");
        let advanced = state.clock.advance();
        let mut checksum_payload: Option<(Tick, f64, String)> = None;
        let mut tick_sample_payload: Option<(Tick, f64, TickSampleStats)> = None;
        let mut step_report: Option<(Tick, f64, ControlIntent, StepReport)> = None;
        let mut snapshot_payload: Option<(Tick, f64, ActorWorldSnapshot)> = None;
        // M1.5: bundle returned from `cf-terrain::try_dig` plus the dig source.
        // Stored locally so events can be emitted after the state guard is dropped.
        let mut dig_payload: Option<(Tick, f64, DigEvent)> = None;
        let mut ai_payloads: Vec<(Tick, f64, ActorId, cf_ai::EnemyTickReport)> = Vec::new();
        let mut guard_fire_records: Vec<GuardFireRecord> = Vec::new();
        let mut mission_payload: Option<(Tick, f64, cf_mission::MissionTickReport)> = None;
        if let Some(tick) = advanced {
            state.rng.next_u64();

            // BP2 dig path. Chunked terrain takes priority when loaded; legacy
            // breach strips drive M1.5 backward compatibility. The dig first
            // probes chunked terrain in front of the player; if that produces a
            // result (Carved / Refused / NoOp) we consume the dig there. If
            // chunked terrain is NOT loaded but a breach world is, we fall back
            // to the M1.5 strip path.
            if state.pending_dig.is_some() && (state.chunked_terrain.is_some() || state.breach_world.is_some()) {
                let pending = state.pending_dig.take().expect("pending dig is_some");
                let player_pos_aim = state.player_actor.and_then(|pid| {
                    state
                        .actor_state
                        .as_ref()
                        .and_then(|sim| sim.world.actors.get(&pid))
                        .map(|a| ((a.position.x, a.position.y), (a.aim.x, a.aim.y)))
                });
                if let Some(((px, py), (ax, ay))) = player_pos_aim {
                    if let Some(terrain) = state.chunked_terrain.as_mut() {
                        // Tool reach + radius: 22-pixel reach along aim, 12-px
                        // carve radius. The radius is tuned so consecutive
                        // digs while the player walks (~3-4 px/tick) overlap
                        // and form a continuous tunnel without leaving micro-
                        // gaps that would block projectile-vs-terrain checks.
                        // M2 design intent (tight bites that require many
                        // digs) is preserved because each dig still only
                        // clears ~450 pixels out of a typical ~12,800-pixel
                        // shield mound.
                        const DIG_REACH: f32 = 22.0;
                        const DIG_RADIUS: f32 = 12.0;
                        let aim_len = (ax * ax + ay * ay).sqrt().max(0.001);
                        let nx = ax / aim_len;
                        let ny = ay / aim_len;
                        let target_x = px + nx * DIG_REACH;
                        let target_y = py + ny * DIG_REACH;
                        let outcome = terrain.try_carve([target_x, target_y], DIG_RADIUS);
                        dig_payload = Some((
                            tick,
                            state.clock.sim_time_ms(),
                            DigEvent::Chunked {
                                outcome,
                                source: pending.source,
                                origin: [px, py],
                                aim: [nx, ny],
                                target: [target_x, target_y],
                            },
                        ));
                    } else if let Some(world) = state.breach_world.as_mut() {
                        let outcome = cf_terrain::try_dig(
                            world,
                            cf_terrain::DigRequest {
                                origin: [px, py],
                                aim: [ax, ay],
                                explicit_target: pending.target.clone(),
                            },
                        );
                        dig_payload = Some((
                            tick,
                            state.clock.sim_time_ms(),
                            DigEvent::Strip {
                                outcome,
                                source: pending.source,
                                origin: [px, py],
                            },
                        ));
                    }
                }
            }

            // M1: step the actor world if present. The pending intent is consumed and
            // its edge-triggered fields cleared so the next tick starts fresh. M1.5
            // augments this by running each reactive guard's controller and feeding its
            // generated intent into the same actor-step pipeline.
            if state.actor_state.is_some() {
                let intent = state.pending_intent.clone();
                state.pending_intent.clear_edges();
                let region_min_x = self.config.region_anchor_x;
                let region_max_x = self.config.region_anchor_x + self.config.region_width.max(0.0);
                let region_max_y = self.config.region_anchor_y + self.config.region_height.max(0.0);
                let tick_dt = SimConfig {
                    tick_rate_hz: self.config.tick_rate_hz,
                }
                .tick_dt()
                .as_secs_f32();
                let auto_reload = false;
                let player = state.player_actor;
                let mut intents = BTreeMap::new();
                if let Some(player_id) = player {
                    intents.insert(player_id, intent.clone());
                }

                // M1.5: tick reactive guards. We collect their fire records and apply
                // them to the actor world AFTER the player step so we don't aliasing
                // borrow the actor world mutably twice. The temporary `take()` of
                // each guard releases the BTreeMap borrow so we can mutate state.rng.
                let sim_time_ms = state.clock.sim_time_ms();
                let guard_ids: Vec<ActorId> = state.reactive_guards.keys().copied().collect();
                for guard_id in guard_ids {
                    let (self_actor, player_actor) = {
                        let sim = match state.actor_state.as_ref() {
                            Some(s) => s,
                            None => break,
                        };
                        (
                            sim.world.actors.get(&guard_id).cloned(),
                            player.and_then(|pid| sim.world.actors.get(&pid).cloned()),
                        )
                    };
                    let self_actor = match self_actor {
                        Some(a) => a,
                        None => continue,
                    };
                    let player_ref = player_actor.as_ref();
                    let mut guard = state
                        .reactive_guards
                        .remove(&guard_id)
                        .expect("guard exists by construction");
                    let report = cf_ai::step(
                        &mut guard,
                        cf_ai::GuardTickInputs {
                            tick: tick.0,
                            tick_rate_hz: self.config.tick_rate_hz,
                            self_actor: &self_actor,
                            player: player_ref,
                        },
                        &mut state.rng,
                    );
                    state.reactive_guards.insert(guard_id, guard);
                    if let Some(fire) = &report.fire {
                        guard_fire_records.push(GuardFireRecord {
                            shooter: guard_id,
                            origin: fire.muzzle_origin,
                            velocity: fire.velocity,
                            damage: fire.damage,
                            lifetime_ticks: fire.lifetime_ticks,
                            will_miss: fire.will_miss,
                        });
                    }
                    ai_payloads.push((tick, sim_time_ms, guard_id, report));
                }

                let actor_state_mut = state.actor_state.as_mut().expect("actor state present");
                let report = actor_step(
                    actor_state_mut,
                    &mut intents,
                    StepDeps {
                        tick_dt,
                        region_min_x,
                        region_max_x,
                        region_max_y,
                        auto_reload_when_empty: auto_reload,
                    },
                );

                // M1.5: spawn guard projectiles into the same projectile pool the
                // actor step uses so cf-actor's swept hit detection runs against
                // them on subsequent ticks. We allocate ids from the dedicated
                // guard range to avoid colliding with player projectile ids.
                if !guard_fire_records.is_empty() {
                    for fire in &guard_fire_records {
                        let id = state.next_guard_projectile_id;
                        state.next_guard_projectile_id = state.next_guard_projectile_id.wrapping_add(1);
                        let actor_state_mut = state.actor_state.as_mut().expect("actor state present");
                        actor_state_mut.projectiles.push(cf_actor::sim::Projectile {
                            id,
                            owner: fire.shooter,
                            origin: cf_actor::Vec2::new(fire.origin[0], fire.origin[1]),
                            position: cf_actor::Vec2::new(fire.origin[0], fire.origin[1]),
                            velocity: cf_actor::Vec2::new(fire.velocity[0], fire.velocity[1]),
                            damage: fire.damage,
                            remaining_ticks: fire.lifetime_ticks,
                        });
                    }
                }

                step_report = Some((tick, state.clock.sim_time_ms(), intent, report));
            }

            // M2: projectile-vs-chunked-terrain collision. Solid terrain
            // pixels stop projectiles cold; the projectile expires with cause
            // `terrain_hit`. This is what makes M2.5 micro_reactor_defense
            // strategic: dirt mounds between the guard and the reactor block
            // bullets, and the player's dig action exposes the reactor.
            let mut terrain_kills: Vec<(u64, ActorId, [f32; 2])> = Vec::new();
            if state.chunked_terrain.is_some() && state.actor_state.is_some() {
                let EngineMutable {
                    actor_state,
                    chunked_terrain,
                    ..
                } = &mut *state;
                let terrain = chunked_terrain.as_ref().expect("chunked terrain present");
                if let Some(actor_state_mut) = actor_state.as_mut() {
                    actor_state_mut.projectiles.retain(|proj| {
                        // Treat each projectile as a point. The pixel-cell
                        // containing the centre defines the collision test;
                        // padding the AABB to ±0.5 was too aggressive and
                        // blocked projectiles flying through carved tunnels.
                        //
                        // Use `material_at_world` so the terrain anchor offset
                        // is honored. Calling `material_at` with raw world
                        // floats would falsely shift the lookup by `anchor`
                        // for any scenario that authors a non-(0, 0) anchor
                        // (DR-007 Bugbot finding 864084a2).
                        let mat = terrain.material_at_world(proj.position.x, proj.position.y);
                        if terrain.registry.is_solid(mat) {
                            terrain_kills.push((proj.id, proj.owner, [proj.position.x, proj.position.y]));
                            false
                        } else {
                            true
                        }
                    });
                }
            }
            if !terrain_kills.is_empty() {
                let sim_time_ms = state.clock.sim_time_ms();
                for (projectile_id, owner, pos) in terrain_kills {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "combat",
                        "projectile_expired",
                        json!({
                            "id": projectile_id,
                            "owner": owner.0,
                            "last_position": pos,
                            "cause": "terrain_hit",
                        }),
                        None,
                    );
                }
            }

            // M2.5: route projectile hits onto reactor AABBs. We walk every
            // live projectile after the actor step and damage the first
            // reactor whose AABB contains the projectile position. Hits emit
            // `combat.projectile_hit` (target=reactor) + `actor.actor_status_changed`
            // (target=reactor) when the reactor reaches zero hp.
            //
            // Per-hit state (`hp_after`, `hp_max`, `destroyed_after`) is
            // captured AT THE MOMENT THE HIT IS PROCESSED, not later. The
            // earlier "read final reactor state in the emit loop" approach
            // was Bugbot 2ce56d7e: when two projectiles hit the same reactor
            // in one tick, the first hit's event would falsely report the
            // post-second-hit hp + destroyed flag, producing duplicate
            // destruction events.
            struct ReactorHit {
                rid: String,
                damage_applied: f32,
                position: [f32; 2],
                projectile_id: u64,
                hp_after: f32,
                hp_max: f32,
                destroyed_after: bool,
                /// True only on the hit that flipped the reactor to
                /// destroyed (so we emit `actor_status_changed` exactly
                /// once per reactor).
                triggered_destruction: bool,
            }
            let mut reactor_hits: Vec<ReactorHit> = Vec::new();
            if state.reactor_world.is_some() && state.actor_state.is_some() {
                let EngineMutable {
                    actor_state,
                    reactor_world,
                    ..
                } = &mut *state;
                let reactors = reactor_world.as_mut().expect("reactor world present");
                if let Some(actor_state_mut) = actor_state.as_mut() {
                    actor_state_mut.projectiles.retain(|proj| {
                        let mut consumed = false;
                        for r in reactors.iter_mut() {
                            if r.is_destroyed() {
                                continue;
                            }
                            if r.aabb_contains(proj.position.x, proj.position.y) {
                                let prev_hp = r.hp;
                                let prev_destroyed = r.is_destroyed();
                                r.apply_damage(proj.damage);
                                let actual = (prev_hp - r.hp).max(0.0);
                                let now_destroyed = r.is_destroyed();
                                reactor_hits.push(ReactorHit {
                                    rid: r.id.clone(),
                                    damage_applied: actual,
                                    position: [proj.position.x, proj.position.y],
                                    projectile_id: proj.id,
                                    hp_after: r.hp,
                                    hp_max: r.max_hp,
                                    destroyed_after: now_destroyed,
                                    triggered_destruction: now_destroyed && !prev_destroyed,
                                });
                                consumed = true;
                                break;
                            }
                        }
                        !consumed
                    });
                }
            }
            if !reactor_hits.is_empty() {
                let sim_time_ms = state.clock.sim_time_ms();
                for hit in reactor_hits {
                    let hit_id = self.recorder.record(
                        tick,
                        sim_time_ms,
                        "combat",
                        "projectile_hit",
                        json!({
                            "target_kind": "reactor",
                            "target": hit.rid.clone(),
                            "position": hit.position,
                            "damage": hit.damage_applied,
                            "projectile_id": hit.projectile_id,
                        }),
                        None,
                    );
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "actor",
                        "reactor_damaged",
                        json!({
                            "reactor": hit.rid.clone(),
                            "hp": hit.hp_after,
                            "hp_max": hit.hp_max,
                            "destroyed": hit.destroyed_after,
                            "damage_applied": hit.damage_applied,
                        }),
                        Some(hit_id.clone()),
                    );
                    // Emit `actor_status_changed` ONLY on the hit that
                    // flipped the reactor to destroyed. Subsequent same-
                    // tick hits on the same reactor have
                    // `destroyed_after == true` but `triggered_destruction
                    // == false`, so they don't duplicate the transition
                    // event.
                    if hit.triggered_destruction {
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "actor",
                            "actor_status_changed",
                            json!({
                                "actor_kind": "reactor",
                                "actor": hit.rid,
                                "previous_status": "active",
                                "new_status": "destroyed",
                                "cause": "projectile_hit",
                            }),
                            Some(hit_id),
                        );
                    }
                }
            }

            // M1.5: tick the mission state machine after the actor world settles.
            // This runs even when the scenario has no actor world so a breach-only
            // or timer-only scenario still ticks its loss timer and objectives.
            if state.mission.is_some() {
                let sim_time_ms = state.clock.sim_time_ms();
                // Snapshot inputs so we can drop the actor borrow before we mutate
                // the mission slot. The actor world clones cheaply (BTreeMap is
                // O(n)); 16-actor scenarios are well within budget. When no actor
                // world is loaded we feed the mission an empty actor map.
                let breaches_broken = state.breach_world.as_ref().map(|w| w.broken_map()).unwrap_or_default();
                let player_id = state.player_actor;
                let (actors_clone, player_clone) = match state.actor_state.as_ref() {
                    Some(actor_state_ref) => {
                        let actors = actor_state_ref.world.actors.clone();
                        let player_clone = player_id.and_then(|pid| actors.get(&pid).cloned());
                        (actors, player_clone)
                    }
                    None => (BTreeMap::new(), None),
                };
                let reactors_destroyed = state
                    .reactor_world
                    .as_ref()
                    .map(|w| w.destroyed_map())
                    .unwrap_or_default();
                let mission = state.mission.as_mut().expect("mission present");
                let inputs = cf_mission::MissionTickInputs {
                    tick: tick.0,
                    player: player_clone.as_ref(),
                    actors: &actors_clone,
                    breaches_broken: &breaches_broken,
                    reactors_destroyed: &reactors_destroyed,
                };
                let report = cf_mission::step(mission, inputs);
                if !report.objective_completed.is_empty()
                    || !report.objective_started.is_empty()
                    || !report.objective_failed.is_empty()
                    || report.final_result.is_some()
                {
                    mission_payload = Some((tick, sim_time_ms, report));
                }
            }
            let cadence = self.config.checksum_cadence_ticks;
            if cadence > 0 && tick.0 % cadence == 0 {
                let actor_bytes = build_checksum_bytes(&state);
                let cs = sim_state_v1(tick, &state.rng, &actor_bytes);
                let sim_time_ms = state.clock.sim_time_ms();
                checksum_payload = Some((tick, sim_time_ms, cs.to_hex()));
                // M0.2-F4: emit a tick_sample summarizing the last `cadence` ticks.
                let stats = TickSampleStats::from_recent(&state.tick_durations_us, cadence as usize);
                tick_sample_payload = Some((tick, sim_time_ms, stats));
                if let Some(actor_state) = state.actor_state.as_ref() {
                    snapshot_payload = Some((tick, sim_time_ms, ActorWorldSnapshot::from(actor_state)));
                }
            }
        }
        let elapsed_us = start.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
        state.tick_durations_us.push(elapsed_us);
        // `TickSampleStats::from_recent` only ever reads the last `cadence_ticks` entries
        // (default 60). Cap the buffer well above that so long-running sessions without a
        // `scenario.reset` don't accumulate millions of dead entries (~1.7 MB/hr at 60 Hz).
        // Drain in batches so the trim cost amortises to O(1) per tick.
        if state.tick_durations_us.len() > TICK_DURATIONS_HISTORY_CAP * 2 {
            let drop = state.tick_durations_us.len() - TICK_DURATIONS_HISTORY_CAP;
            state.tick_durations_us.drain(..drop);
        }
        let new_tick = state.clock.tick().0;
        drop(state);
        // Publish the latest tick so the panic reporter records `system.panic` at the
        // current tick (preserves events.jsonl monotonic ordering).
        self.current_tick.store(new_tick, std::sync::atomic::Ordering::Relaxed);

        // Emit M1 events from the actor step.
        if let Some((tick, sim_time_ms, intent, report)) = step_report {
            self.emit_actor_events(tick, sim_time_ms, &intent, &report);
        }

        if let Some((tick, sim_time_ms, hex)) = checksum_payload {
            self.recorder.record(
                tick,
                sim_time_ms,
                "determinism",
                "sim_checksum",
                json!({
                    "checksum_hex": hex,
                    "algorithm": CHECKSUM_ALGORITHM,
                    "scope": CHECKSUM_SCOPE,
                    "cadence_ticks": self.config.checksum_cadence_ticks,
                    "tick_rate_hz": self.config.tick_rate_hz,
                    "seed": self.config.seed,
                }),
                None,
            );
        }
        if let Some((tick, sim_time_ms, stats)) = tick_sample_payload {
            self.recorder.record(
                tick,
                sim_time_ms,
                "system",
                "tick_sample",
                json!({
                    "tick_rate_hz": self.config.tick_rate_hz,
                    "window_ticks": stats.window_ticks,
                    "avg_tick_ms": stats.avg_tick_ms,
                    "max_tick_ms": stats.max_tick_ms,
                    "p99_tick_ms": stats.p99_tick_ms,
                    "samples_observed": stats.samples_observed,
                }),
                None,
            );
        }
        if let Some((tick, sim_time_ms, snapshot)) = snapshot_payload {
            self.recorder.record(
                tick,
                sim_time_ms,
                "actor",
                "actor_snapshot",
                json!({
                    "actors": snapshot.actors,
                    "player_actor_id": snapshot.player_actor_id,
                }),
                None,
            );
        }

        // M1.5 / M2: emit terrain dig events (always emit `tool_action_started`,
        // then `terrain_carved` or `tool_refused` based on outcome). The
        // `source: chunked|strip` field lets replay viewers tell M1.5 strip
        // digs from M2 chunked-terrain digs.
        let mut dig_validity_update: Option<(u64, ToolValidityUpdate)> = None;
        if let Some((tick, sim_time_ms, evt)) = dig_payload {
            let dig_source = match evt.source() {
                IntentSource::Human => "human",
                IntentSource::Cfctl => "cfctl",
                IntentSource::Ai => "ai",
                IntentSource::Replay => "replay",
            };
            let mode = match &evt {
                DigEvent::Strip { .. } => "strip",
                DigEvent::Chunked { .. } => "chunked",
            };
            let action_id = self.recorder.record(
                tick,
                sim_time_ms,
                "terrain",
                "tool_action_started",
                json!({
                    "tool": "digger",
                    "mode": mode,
                    "source": dig_source,
                    "origin": evt.origin(),
                    "explicit_target": evt.outcome_target_string(),
                }),
                None,
            );
            match evt {
                DigEvent::Strip { outcome, .. } => match outcome {
                    cf_terrain::DigOutcome::Carved {
                        strip_id,
                        material,
                        bbox_min,
                        bbox_max,
                        damage_applied,
                        hp_remaining,
                        broken,
                    } => {
                        dig_validity_update = Some((tick.0, ToolValidityUpdate::Carve));
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "terrain",
                            "terrain_carved",
                            json!({
                                "tick": tick.0,
                                "mode": "strip",
                                "bbox": { "min": bbox_min, "max": bbox_max },
                                "material_before": material.clone(),
                                "material_after": if broken { "air" } else { &material },
                                "count": 1u32,
                                "strip_id": strip_id,
                                "damage_applied": damage_applied,
                                "hp_remaining": hp_remaining,
                                "broken": broken,
                            }),
                            Some(action_id.clone()),
                        );
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "terrain",
                            "terrain_breach_stub",
                            json!({
                                "strip_id": "stub",
                                "tick": tick.0,
                                "broken": broken,
                            }),
                            Some(action_id),
                        );
                    }
                    cf_terrain::DigOutcome::Refused {
                        reason,
                        strip_id,
                        material,
                        bbox_min,
                        bbox_max,
                    } => {
                        dig_validity_update = Some((
                            tick.0,
                            ToolValidityUpdate::Refuse {
                                reason: reason.clone(),
                                target: strip_id.clone(),
                            },
                        ));
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "terrain",
                            "tool_refused",
                            json!({
                                "reason": reason,
                                "mode": "strip",
                                "strip_id": strip_id,
                                "material": material,
                                "bbox_min": bbox_min,
                                "bbox_max": bbox_max,
                            }),
                            Some(action_id),
                        );
                    }
                },
                DigEvent::Chunked {
                    outcome, aim, target, ..
                } => match outcome {
                    cf_terrain::ChunkedCarveOutcome::Carved(stats) => {
                        dig_validity_update = Some((tick.0, ToolValidityUpdate::Carve));
                        let mat_name = cf_terrain::material_affordance(stats.dominant_material)
                            .map(|m| m.name)
                            .unwrap_or("unknown");
                        let dirty: Vec<serde_json::Value> = stats
                            .dirty_chunks
                            .iter()
                            .map(|c| json!({"cx": c.cx, "cy": c.cy}))
                            .collect();
                        let chunk_carved_id = self.recorder.record(
                            tick,
                            sim_time_ms,
                            "terrain",
                            "terrain_carved",
                            json!({
                                "tick": tick.0,
                                "mode": "chunked",
                                "bbox": { "min": stats.bbox_min, "max": stats.bbox_max },
                                "material": mat_name,
                                "dominant_material_id": stats.dominant_material,
                                "count": stats.count,
                                "aim": aim,
                                "target": target,
                                "dirty_chunks": dirty,
                            }),
                            Some(action_id.clone()),
                        );
                        // M2 also emits a `material.chunk_dirtied` event per
                        // dirty chunk so the M5.6 active material kernel can
                        // pick up the same vocabulary later.
                        for chunk in &stats.dirty_chunks {
                            self.recorder.record(
                                tick,
                                sim_time_ms,
                                "material",
                                "chunk_dirtied",
                                json!({
                                    "cx": chunk.cx,
                                    "cy": chunk.cy,
                                    "cause": "dig",
                                }),
                                Some(chunk_carved_id.clone()),
                            );
                        }
                    }
                    cf_terrain::ChunkedCarveOutcome::Refused(refusal) => {
                        let mat_name = cf_terrain::material_affordance(refusal.material)
                            .map(|m| m.name)
                            .unwrap_or("unknown");
                        dig_validity_update = Some((
                            tick.0,
                            ToolValidityUpdate::Refuse {
                                reason: refusal.reason.to_string(),
                                target: Some(format!("chunked:{mat_name}")),
                            },
                        ));
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "terrain",
                            "tool_refused",
                            json!({
                                "reason": refusal.reason,
                                "mode": "chunked",
                                "material": mat_name,
                                "material_id": refusal.material,
                                "probe_at": refusal.probe_at,
                            }),
                            Some(action_id),
                        );
                    }
                    cf_terrain::ChunkedCarveOutcome::NoOp(noop) => {
                        dig_validity_update = Some((
                            tick.0,
                            ToolValidityUpdate::Refuse {
                                reason: "out_of_range".to_string(),
                                target: None,
                            },
                        ));
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "terrain",
                            "tool_refused",
                            json!({
                                "reason": "out_of_range",
                                "mode": "chunked",
                                "probe_at": noop.probe_at,
                            }),
                            Some(action_id),
                        );
                    }
                },
            }
        }
        // M4A: persist tool-validity update for the HUD + observe consumers.
        if let Some((update_tick, update)) = dig_validity_update {
            let mut state = self.state.write().expect("engine state poisoned");
            match update {
                ToolValidityUpdate::Carve => {
                    state.hud_tool_validity.last_carve_tick = Some(update_tick);
                    state.hud_tool_validity.valid = true;
                }
                ToolValidityUpdate::Refuse { reason, target } => {
                    state.hud_tool_validity.last_refusal_tick = Some(update_tick);
                    state.hud_tool_validity.last_refusal_reason = Some(reason);
                    state.hud_tool_validity.last_refusal_target = target;
                    state.hud_tool_validity.valid = false;
                }
            }
        }

        // M1.5: emit AI events for each guard.
        for (tick, sim_time_ms, guard_id, report) in &ai_payloads {
            self.emit_guard_events(*tick, *sim_time_ms, *guard_id, report);
        }

        // M1.5: emit mission events.
        if let Some((tick, sim_time_ms, report)) = mission_payload {
            for id in &report.objective_started {
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "mission",
                    "objective_started",
                    json!({"objective": id}),
                    None,
                );
            }
            for id in &report.objective_completed {
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "mission",
                    "objective_completed",
                    json!({"objective": id}),
                    None,
                );
                // **M5**: when an objective completes AND the player has
                // ejected (Ejected pilot reached the extraction zone),
                // promote the chassis pilot_state to Extracted so further
                // damage is fully suppressed.
                if let Ok(mut s) = self.state.write() {
                    let player_id = s.player_actor;
                    if let Some(pid) = player_id {
                        if let Some(sim) = s.actor_state.as_mut() {
                            if let Some(actor) = sim.world.actors.get_mut(&pid) {
                                if let Some(chassis) = actor.chassis.as_mut() {
                                    if chassis.mark_pilot_extracted() {
                                        let actor_id = pid.0;
                                        drop(s);
                                        self.recorder.record(
                                            tick,
                                            sim_time_ms,
                                            "chassis",
                                            "pilot_extracted",
                                            json!({"actor": actor_id, "via": "reach_zone"}),
                                            None,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
            for id in &report.objective_failed {
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "mission",
                    "objective_failed",
                    json!({"objective": id}),
                    None,
                );
            }
            if let Some(result) = report.final_result {
                let payload = match result {
                    cf_mission::MissionResult::Won => json!({"result": "won"}),
                    cf_mission::MissionResult::Lost { reason } => {
                        json!({"result": "lost", "reason": reason.as_str()})
                    }
                    cf_mission::MissionResult::Active => json!({"result": "active"}),
                    cf_mission::MissionResult::Aborted => json!({"result": "aborted"}),
                };
                self.recorder
                    .record(tick, sim_time_ms, "mission", "mission_resolved", payload, None);
            }
            // W1 item 770: re-emit snapshots on any objective state change so
            // the replay verifier and viewer can reconstruct mid-mission state.
            self.emit_initial_snapshots(tick, sim_time_ms, "objective_change");
        }

        // **M5**: tick the chassis eject sequence for every actor + emit progress events.
        if let Some(t) = advanced {
            let sim_time_ms = self.state.read().map(|s| s.clock.sim_time_ms()).unwrap_or(0.0);
            self.tick_chassis_eject_for_all(t, sim_time_ms);
        }

        // M4A: refresh HUD banners + captions + tool_validity caches AFTER all events
        // have been emitted for this tick. The cache reads world state directly so it
        // does not have to scan the event log on every observe().
        if let Some(t) = advanced {
            let mut state = self.state.write().expect("engine state poisoned");
            self.refresh_hud_caches(&mut state, t);
            self.refresh_hud_chassis_banners(&mut state, t);
        }

        advanced
    }

    /// **M5**: raise HUD banners for chassis stage transitions (armor cracked,
    /// weapon jammed, eject window, pilot lost) and refresh the per-player
    /// stage cache. Mirrors `refresh_hud_caches` but reads chassis state.
    fn refresh_hud_chassis_banners(&self, state: &mut EngineMutable, tick: Tick) {
        let now_tick = tick.0;
        let player_id = state.player_actor;
        let Some(sim) = state.actor_state.as_ref() else { return };
        let Some(pid) = player_id else { return };
        let Some(actor) = sim.world.actors.get(&pid) else {
            return;
        };
        let Some(chassis) = actor.chassis.as_ref() else { return };
        let prev_stage = state.hud_last_chassis_stage;
        let prev_pilot = state.hud_last_pilot_state;
        let cur_stage = chassis.stage;
        let cur_pilot = chassis.pilot_state;
        // Stage transition banner.
        if Some(cur_stage) != prev_stage {
            if let Some(banner) = chassis_stage_banner(cur_stage, now_tick) {
                push_banner(&mut state.hud_banners, banner);
            }
        }
        // Pilot eject banner (during the active eject window).
        if matches!(cur_pilot, cf_chassis::PilotState::Ejecting) {
            push_banner_dedup(
                &mut state.hud_banners,
                crate::state::HudBannerView {
                    id: "eject_active".to_string(),
                    severity: "critical".to_string(),
                    label: format!("EJECTING — {} TICKS", chassis.eject_window.ticks_remaining),
                    raised_at_tick: now_tick,
                    expires_at_tick: Some(now_tick + 30),
                    accessibility_id: "hud.banner.eject_active".to_string(),
                },
            );
        }
        if Some(cur_pilot) != prev_pilot {
            if let Some(banner) = chassis_pilot_banner(cur_pilot, now_tick) {
                push_banner(&mut state.hud_banners, banner);
            }
        }
        // Weapon jam banner.
        if chassis.weapon_jammed {
            push_banner_dedup(
                &mut state.hud_banners,
                crate::state::HudBannerView {
                    id: "weapon_jammed".to_string(),
                    severity: "warning".to_string(),
                    label: "WEAPON JAMMED — CLEAR".to_string(),
                    raised_at_tick: now_tick,
                    expires_at_tick: Some(now_tick + M4A_STATUS_BANNER_EXPIRY_TICKS),
                    accessibility_id: "hud.banner.weapon_jammed".to_string(),
                },
            );
        }
        state.hud_last_chassis_stage = Some(cur_stage);
        state.hud_last_pilot_state = Some(cur_pilot);
    }

    /// M4A HUD cache refresh. Called once per drive_tick after every category's
    /// events have been emitted. Updates `hud_banners`, `hud_captions`, and the
    /// `hud_last_*` diffing cursors. The HUD + `cfctl observe` reads the cache
    /// directly during `snapshot()`.
    fn refresh_hud_caches(&self, state: &mut EngineMutable, tick: Tick) {
        // Drain expired banners + captions.
        let now_tick = tick.0;
        state.hud_banners.retain(|b| match b.expires_at_tick {
            Some(exp) => now_tick < exp,
            None => true,
        });
        state
            .hud_captions
            .retain(|c| now_tick.saturating_sub(c.raised_at_tick) < M4A_CAPTION_EXPIRY_TICKS);

        // Status-change banners. The previous tick's status is cached in
        // `hud_last_status`; raise a banner whenever the player's status
        // worsens (Stable -> Unstable / Unstable -> Downed / any -> Dead).
        if let Some(sim) = state.actor_state.as_ref() {
            // Snapshot the status diff out of the borrow so we can push to the
            // banner queue (which lives on the same `state` borrow).
            let mut player_dead = false;
            let mut player_downed = false;
            let mut player_unstable = false;
            let mut diffs: Vec<(ActorId, cf_actor::Status)> = Vec::new();
            for (id, actor) in &sim.world.actors {
                let prev = state.hud_last_status.get(id).copied();
                let cur = actor.status;
                if prev.is_some() && prev != Some(cur) {
                    diffs.push((*id, cur));
                    if Some(*id) == sim.world.player {
                        match cur {
                            cf_actor::Status::Dead => player_dead = true,
                            cf_actor::Status::Downed => player_downed = true,
                            cf_actor::Status::Unstable => player_unstable = true,
                            cf_actor::Status::Stable => {}
                        }
                    }
                }
            }
            if player_dead {
                push_banner(
                    &mut state.hud_banners,
                    crate::state::HudBannerView {
                        id: "eject_now".to_string(),
                        severity: "critical".to_string(),
                        label: "EJECT NOW".to_string(),
                        raised_at_tick: now_tick,
                        expires_at_tick: None,
                        accessibility_id: "hud.banner.eject_now".to_string(),
                    },
                );
            } else if player_downed {
                push_banner(
                    &mut state.hud_banners,
                    crate::state::HudBannerView {
                        id: "armor_cracked".to_string(),
                        severity: "critical".to_string(),
                        label: "ARMOR CRACKED".to_string(),
                        raised_at_tick: now_tick,
                        expires_at_tick: Some(now_tick + M4A_STATUS_BANNER_EXPIRY_TICKS),
                        accessibility_id: "hud.banner.armor_cracked".to_string(),
                    },
                );
            } else if player_unstable {
                push_banner(
                    &mut state.hud_banners,
                    crate::state::HudBannerView {
                        id: "hp_low".to_string(),
                        severity: "warning".to_string(),
                        label: "HP LOW".to_string(),
                        raised_at_tick: now_tick,
                        expires_at_tick: Some(now_tick + M4A_STATUS_BANNER_EXPIRY_TICKS),
                        accessibility_id: "hud.banner.hp_low".to_string(),
                    },
                );
            }
            // Per-tick caption emission for status changes (audio-bound at BP6+;
            // the captions surface lands at M4A so the contract is testable).
            for (id, st) in diffs {
                let label = format!("actor {} → {}", id.0, st.as_str());
                push_caption(
                    &mut state.hud_captions,
                    crate::state::CaptionView {
                        id: format!("status_changed.{}", id.0),
                        label,
                        raised_at_tick: now_tick,
                        accessibility_id: format!("hud.caption.status_changed.{}", id.0),
                    },
                );
            }

            // AMMO OUT banner: triggered when the selected rifle hits 0/cap with no reload in progress.
            if let Some(player_id) = sim.world.player {
                if let Some(rifle) = sim.rifles.get(&player_id) {
                    if rifle.spec.mag_capacity > 0 && rifle.ammo_in_mag == 0 && rifle.reload_remaining_ticks == 0 {
                        push_banner_dedup(
                            &mut state.hud_banners,
                            crate::state::HudBannerView {
                                id: "ammo_out".to_string(),
                                severity: "warning".to_string(),
                                label: "AMMO OUT — RELOAD".to_string(),
                                raised_at_tick: now_tick,
                                expires_at_tick: Some(now_tick + M4A_STATUS_BANNER_EXPIRY_TICKS),
                                accessibility_id: "hud.banner.ammo_out".to_string(),
                            },
                        );
                    }
                }
            }
        }

        // Mission resolution banner.
        let cur_mission_result = state.mission.as_ref().map(|m| match m.result {
            cf_mission::MissionResult::Won => "won".to_string(),
            cf_mission::MissionResult::Lost { .. } => "lost".to_string(),
            cf_mission::MissionResult::Active => "active".to_string(),
            cf_mission::MissionResult::Aborted => "aborted".to_string(),
        });
        if state.hud_last_mission_result != cur_mission_result {
            if let Some(result) = cur_mission_result.as_deref() {
                if result == "won" {
                    push_banner(
                        &mut state.hud_banners,
                        crate::state::HudBannerView {
                            id: "mission_won".to_string(),
                            severity: "info".to_string(),
                            label: "MISSION WON".to_string(),
                            raised_at_tick: now_tick,
                            expires_at_tick: None,
                            accessibility_id: "hud.banner.mission_won".to_string(),
                        },
                    );
                } else if result == "lost" {
                    push_banner(
                        &mut state.hud_banners,
                        crate::state::HudBannerView {
                            id: "mission_failed".to_string(),
                            severity: "critical".to_string(),
                            label: "MISSION FAILED".to_string(),
                            raised_at_tick: now_tick,
                            expires_at_tick: None,
                            accessibility_id: "hud.banner.mission_failed".to_string(),
                        },
                    );
                }
            }
            state.hud_last_mission_result = cur_mission_result;
        }

        // Refresh hud_last_status for next tick.
        state.hud_last_status.clear();
        if let Some(sim) = state.actor_state.as_ref() {
            for (id, actor) in &sim.world.actors {
                state.hud_last_status.insert(*id, actor.status);
            }
        }
    }

    /// Translate a `cf_ai::EnemyTickReport` into recorder events.
    fn emit_guard_events(&self, tick: Tick, sim_time_ms: f64, guard_id: ActorId, report: &cf_ai::EnemyTickReport) {
        // Always emit ai.perception (even when player_seen=false) so replay
        // viewers can step through the guard's awareness.
        if let Some(p) = &report.perception {
            self.recorder.record(
                tick,
                sim_time_ms,
                "ai",
                "ai_perception",
                json!({
                    "actor": guard_id.0,
                    "player_seen": p.player_seen,
                    "distance": p.distance,
                    "angle_degrees": p.angle_degrees,
                    "last_seen_position": p.last_seen_position,
                    "state": p.state.as_str(),
                }),
                None,
            );
        }
        if let Some(t) = &report.tactic_chosen {
            self.recorder.record(
                tick,
                sim_time_ms,
                "ai",
                "tactic_chosen",
                json!({
                    "actor": guard_id.0,
                    "tactic": t.tactic.as_str(),
                    "reason": t.reason,
                    "score_attack": t.score_attack,
                    "score_reload": t.score_reload,
                    "score_hold": t.score_hold,
                    "score_search": t.score_search,
                }),
                None,
            );
        }
        if let Some(s) = &report.state_changed {
            self.recorder.record(
                tick,
                sim_time_ms,
                "ai",
                "state_changed",
                json!({
                    "actor": guard_id.0,
                    "previous": s.previous.as_str(),
                    "next": s.next.as_str(),
                    "cause": s.cause,
                }),
                None,
            );
        }
        if report.reload_started {
            self.recorder.record(
                tick,
                sim_time_ms,
                "equipment",
                "weapon_reload_started",
                json!({"actor": guard_id.0}),
                None,
            );
        }
        if report.reload_completed {
            self.recorder.record(
                tick,
                sim_time_ms,
                "equipment",
                "weapon_reloaded",
                json!({"actor": guard_id.0}),
                None,
            );
        }
        if report.dry_fire {
            self.recorder.record(
                tick,
                sim_time_ms,
                "equipment",
                "weapon_dry_fire",
                json!({"actor": guard_id.0}),
                None,
            );
        }
        if let Some(fire) = &report.fire {
            self.recorder.record(
                tick,
                sim_time_ms,
                "equipment",
                "weapon_fired",
                json!({
                    "actor": guard_id.0,
                    "muzzle_origin": fire.muzzle_origin,
                    "miss_threshold": fire.miss_threshold,
                    "miss_roll": fire.miss_roll,
                    "will_miss": fire.will_miss,
                }),
                None,
            );
            self.recorder.record(
                tick,
                sim_time_ms,
                "combat",
                "projectile_spawned",
                json!({
                    "owner": guard_id.0,
                    "origin": fire.muzzle_origin,
                    "velocity": fire.velocity,
                    "damage": fire.damage,
                    "lifetime_ticks": fire.lifetime_ticks,
                    "will_miss": fire.will_miss,
                }),
                None,
            );
        }
    }

    fn emit_actor_events(&self, tick: Tick, sim_time_ms: f64, intent: &ControlIntent, report: &StepReport) {
        // input.intent_received reflects what was actually consumed (after status gating).
        let player_outcome = report.actor_outcomes.iter().find(|o| o.actor == intent.actor).cloned();
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
            "applied_move_x": player_outcome.as_ref().map(|o| o.move_x).unwrap_or(0.0),
            "jump_accepted": player_outcome.as_ref().map(|o| o.jump_accepted).unwrap_or(false),
        });
        // Always emit input.intent_received once per tick, even when idle, so replay
        // tooling can confirm input flow.
        let intent_event_id = self
            .recorder
            .record(tick, sim_time_ms, "input", "intent_received", player_view, None);

        for outcome in &report.actor_outcomes {
            if outcome.previous_status != outcome.new_status {
                self.recorder.record(
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
                self.recorder.record(
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
            }
            if outcome.reload_started {
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "equipment",
                    "weapon_reload_started",
                    json!({"actor": outcome.actor.0}),
                    Some(intent_event_id.clone()),
                );
            }
            if outcome.reload_completed {
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "equipment",
                    "weapon_reloaded",
                    json!({"actor": outcome.actor.0}),
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
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "equipment",
                    "weapon_fired",
                    json!({
                        "actor": outcome.actor.0,
                        "muzzle_origin": [muzzle.x, muzzle.y],
                        "recoil_impulse": outcome.recoil_applied,
                    }),
                    Some(intent_event_id.clone()),
                );
            }
        }
        for spawn in &report.spawned_projectiles {
            self.recorder.record(
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
                }),
                Some(intent_event_id.clone()),
            );
        }
        for hit in &report.hits {
            // Capture the real event_id of the projectile_hit so the follow-up
            // actor_status_changed can both reference it via the `projectile_event`
            // payload field AND parent-chain to it (a stronger cause-chain link than
            // the same-tick input.intent_received). The recorder makes this id
            // available; the previous synthetic "projectile:N" string was a label
            // that pointed to no real event.
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
                Some(intent_event_id.clone()),
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
            // **M5**: emit chassis-grade events from the hit outcome.
            if let Some(outcome) = &hit.chassis_outcome {
                self.emit_chassis_events(
                    tick,
                    sim_time_ms,
                    hit.target,
                    outcome,
                    Some(projectile_hit_event_id.clone()),
                );
            }
        }
        for expired in &report.expired_projectiles {
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
                Some(intent_event_id.clone()),
            );
        }
    }

    /// **M5**: emit chassis-related events from a [`cf_chassis::ZoneDamageOutcome`].
    /// Also recomputes the chassis stage and emits `chassis.stage_changed` when it
    /// advances. `parent` is the parent-link id for the event (usually the
    /// `combat.projectile_hit` event id).
    fn emit_chassis_events(
        &self,
        tick: Tick,
        sim_time_ms: f64,
        actor: ActorId,
        outcome: &cf_chassis::ZoneDamageOutcome,
        parent: Option<String>,
    ) {
        let zone = outcome.zone.map(|z| z.as_str().to_string()).unwrap_or_default();
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
    fn tick_chassis_eject_for_all(&self, tick: Tick, sim_time_ms: f64) {
        let mut emits: Vec<(ActorId, &'static str, String)> = Vec::new();
        if let Ok(mut state) = self.state.write() {
            if let Some(sim) = state.actor_state.as_mut() {
                let ids: Vec<ActorId> = sim.world.actors.keys().copied().collect();
                for id in ids {
                    let Some(actor) = sim.world.actors.get_mut(&id) else {
                        continue;
                    };
                    let Some(chassis) = actor.chassis.as_mut() else {
                        continue;
                    };
                    if let Some(progress) = chassis.tick_eject() {
                        let stage_after = chassis.stage.as_str().to_string();
                        match progress {
                            cf_chassis::EjectProgress::Ejected => {
                                emits.push((id, "pilot_state_changed", stage_after.clone()));
                                emits.push((id, "pilot_separated", stage_after));
                            }
                            cf_chassis::EjectProgress::BailedTooLate => {
                                emits.push((id, "pilot_bailed_too_late", stage_after));
                            }
                        }
                    }
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
    }

    pub fn record_run_finished(&self, exit_code: i32) {
        // Always emit one final `determinism.sim_checksum` so every bundle has at least one
        // checksum and `summary.json.final_sim_checksum` is never null on a valid run.
        // (Acceptance fix M2 from the M0 review.)
        self.emit_final_checksum();
        let state = self.state.read().expect("engine state poisoned");
        let tick = state.clock.tick();
        let sim_time_ms = state.clock.sim_time_ms();
        drop(state);
        self.recorder.record(
            tick,
            sim_time_ms,
            "system",
            "run_finished",
            json!({"exit_code": exit_code}),
            None,
        );
    }

    /// Emit one `determinism.sim_checksum` event at the current tick, regardless of cadence.
    /// Idempotent within a tick (we still always emit; the recorder will give it a unique seq).
    pub fn emit_final_checksum(&self) {
        let state = self.state.read().expect("engine state poisoned");
        let tick = state.clock.tick();
        let sim_time_ms = state.clock.sim_time_ms();
        let actor_bytes = build_checksum_bytes(&state);
        let cs = sim_state_v1(tick, &state.rng, &actor_bytes);
        drop(state);
        self.recorder.record(
            tick,
            sim_time_ms,
            "determinism",
            "sim_checksum",
            json!({
                "checksum_hex": cs.to_hex(),
                "algorithm": CHECKSUM_ALGORITHM,
                "scope": CHECKSUM_SCOPE,
                "cadence_ticks": self.config.checksum_cadence_ticks,
                "tick_rate_hz": self.config.tick_rate_hz,
                "seed": self.config.seed,
                "kind": "final",
            }),
            None,
        );
    }

    pub fn current_tick(&self) -> Tick {
        self.state.read().expect("engine state poisoned").clock.tick()
    }

    pub fn shutdown_requested(&self) -> bool {
        self.state.read().map(|s| s.shutdown_requested).unwrap_or(false)
    }

    /// Monotonic counter that increments whenever `pending_intent` is
    /// externally reset (currently only `scenario.reset`). Input bridges that
    /// edge-trigger dispatch on keyboard-state change watch this so that
    /// holding a key across a reset still produces a fresh dispatch on the
    /// next frame.
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

    fn perf_sample(&self) -> PerfSample {
        let state = self.state.read().expect("engine state poisoned");
        let mut samples = state.tick_durations_us.clone();
        let ticks_run = state.clock.tick().0;
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
        PerfSample {
            avg_frame_ms: avg_tick_ms,
            p99_frame_ms: p99_tick_ms,
            avg_tick_ms,
            p99_tick_ms,
            ticks_run,
            wall_seconds,
            tick_rate_hz: self.config.tick_rate_hz,
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
        };
        for guard in state.reactive_guards.values() {
            snapshot.enemies.push(EnemyHudView {
                actor: guard.actor.0,
                state: guard.state.as_str().to_string(),
                last_tactic: guard.last_tactic.as_str().to_string(),
            });
        }
        if let Some(sim) = state.actor_state.as_ref() {
            snapshot.floor_y = sim.world.floor_y;
            snapshot.player_actor_id = sim.world.player.map(|id| id.0);
            for actor in sim.world.actors.values() {
                snapshot.actors.push(cf_actor::ActorObservation::from(actor));
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
                loss_reason: match mission.result {
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
        }
        snapshot
    }

    fn reject_actor_command(
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

    pub fn write_run_bundle(&self, ended_at: DateTime<Utc>, exit_code: i32) -> Result<PathBuf, cf_replay::BundleError> {
        // M2 (extended): every bundle written from the engine — including mid-run
        // `runbundle.write` that fires before `record_run_finished` — must contain at
        // least one `determinism.sim_checksum` event so `summary.json.final_sim_checksum`
        // is never null on a valid bundle.
        self.emit_final_checksum();
        let manifest = self.build_manifest();
        let perf = self.perf_sample();
        let result = if exit_code == 0 { "pass" } else { "fail" };
        let evidence_ids = self.first_and_last_event_ids();
        let tests = build_test_records(
            &self.config.expected_tests,
            &self.config.milestone,
            result,
            &evidence_ids,
        );
        let (artifacts, capture_evidence_link) = discover_run_artifacts(&self.run_bundle_dir);
        let mut evidence_links = vec![
            "events.jsonl".to_string(),
            "summary.json".to_string(),
            "run_manifest.json".to_string(),
        ];
        if let Some(link) = capture_evidence_link {
            evidence_links.push(link);
        }
        let inputs = BundleInputs {
            recorder: &self.recorder,
            manifest,
            started_at: self.started_at,
            ended_at,
            exit_code,
            result: result.to_string(),
            blockers: vec![],
            next_actions: next_actions_for_milestone(&self.config.milestone),
            tests,
            artifacts,
            assumptions_tested: self.config.assumptions_tested.clone(),
            good: vec![],
            bad: vec![],
            meh: vec![],
            evidence_links,
            notes_extra: notes_addendum_for_milestone(&self.config.milestone),
            perf: Some(perf),
        };
        cf_replay::write_run_bundle(&self.run_bundle_dir, inputs)?;
        Ok(self.run_bundle_dir.clone())
    }

    fn first_and_last_event_ids(&self) -> Vec<String> {
        let events = self.recorder.snapshot_events();
        match (events.first(), events.last()) {
            (Some(first), Some(last)) if first.event_id != last.event_id => {
                vec![first.event_id.clone(), last.event_id.clone()]
            }
            (Some(only), _) => vec![only.event_id.clone()],
            _ => vec![],
        }
    }

    fn build_manifest(&self) -> RunManifest {
        let mut schemas = BTreeMap::new();
        schemas.insert("control".to_string(), CONTROL_SCHEMA_VERSION);
        schemas.insert("scenario".to_string(), SCENARIO_SCHEMA_VERSION);
        schemas.insert("events".to_string(), EVENT_ENVELOPE_VERSION);

        let live_settings = self.state.read().map(|s| s.settings.clone()).unwrap_or_default();

        RunManifest {
            schema_version: MANIFEST_SCHEMA_VERSION.to_string(),
            run_id: self.recorder.run_id().to_string(),
            prototype_slice: prototype_slice_for_milestone(&self.config.milestone),
            run_mode: self.config.run_mode.clone(),
            milestone: self.config.milestone.clone(),
            build: BuildInfo {
                commit_sha: self.config.commit_sha.clone(),
                worktree_dirty: self.config.worktree_dirty,
                worktree_fingerprint: self.config.worktree_fingerprint.clone(),
                worktree_dirty_files: self.config.worktree_dirty_files.clone(),
                rust_version: self.config.rust_version.clone(),
                bevy_version: self.config.bevy_version.clone(),
                platform: self.config.platform.clone(),
            },
            scene: SceneInfo {
                id: self.config.scenario_id.clone(),
                display_name: self.config.scenario_id.clone(),
                source_path: self.config.scenario_path.display().to_string(),
            },
            seed: self.config.seed,
            started_at_utc: self.started_at.to_rfc3339(),
            duration_target_sec: self.config.duration_ticks as f64 / f64::from(self.config.tick_rate_hz),
            material_schema_version: if self.config.initial_chunked_terrain.is_some() {
                cf_terrain::MATERIAL_SCHEMA_VERSION.to_string()
            } else {
                "n/a-m0".to_string()
            },
            config_hash: self.config.config_hash.clone(),
            assumptions_tested: self.config.assumptions_tested.clone(),
            linked_specs: self.config.linked_specs.clone(),
            expected_tests: self.config.expected_tests.clone(),
            capture_config: if self.config.capture_grid_enabled {
                CaptureConfig {
                    events: true,
                    screenshots: true,
                    captures: true,
                }
            } else {
                CaptureConfig::default()
            },
            schemas,
            capabilities: CapabilitiesBlock {
                debug: self.config.debug_capabilities.iter().any(|c| c == "debug"),
                control_api: self.config.control_api_enabled,
                save_load: false,
                debug_capabilities: self.config.debug_capabilities.clone(),
            },
            settings: SettingsBlock {
                ui_scale: live_settings.ui_scale,
                high_contrast: live_settings.high_contrast,
                captions: live_settings.captions,
                reduced_motion: live_settings.reduced_motion,
                reduced_shake: live_settings.reduced_shake,
                reduced_flash: live_settings.reduced_flash,
                hold_to_confirm: live_settings.hold_to_confirm,
                hold_threshold_ms: live_settings.hold_threshold_ms,
                key_remap_enabled: live_settings.key_remap_enabled,
                key_bindings: live_settings.key_bindings.clone(),
            },
            checksum: ChecksumConfig::m0_default(),
            tick_rate_hz: self.config.tick_rate_hz,
            // M3A-005: declare lifecycle outcome. cf-app + cfctl + cf-e2e
            // drive runs that exit cleanly via `system.run_finished`. The
            // panic-injection debug path (`cf-app --debug-inject-panic-at-tick`)
            // overrides this to `Panic` so the bundle's expected_outcome
            // matches the produced events.
            expected_outcome: if self.config.debug_inject_panic_at_tick.is_some() {
                cf_replay::ExpectedOutcome::Panic
            } else {
                cf_replay::ExpectedOutcome::Clean
            },
        }
    }
}

/// M4A HUD-cache snapshot for cf-app. Mirrors `ObserveFrame.banners /
/// captions / tool_validity / accessibility.focused_node / focus_cycle` so
/// cf-app does not have to call the async `snapshot()` path every frame.
#[derive(Debug, Clone, Default)]
pub struct HudCachesSnapshot {
    pub banners: Vec<crate::state::HudBannerView>,
    pub captions: Vec<crate::state::CaptionView>,
    pub tool_validity: crate::state::ToolValidityView,
    pub focused_node: Option<String>,
    pub focus_cycle: u64,
}

/// Snapshot of the actor world for cf-app's Bevy bridge. Cheap to clone; reuses
/// `cf-actor::ActorObservation` which is the public actor projection.
#[derive(Debug, Clone, Default)]
pub struct ActorRenderSnapshot {
    pub tick: u64,
    pub floor_y: f32,
    pub actors: Vec<cf_actor::ActorObservation>,
    pub player_actor_id: Option<u64>,
    pub player_rifle: Option<RifleHudView>,
    /// M1.5: breach strips for the renderer.
    pub breaches: Vec<BreachRenderView>,
    /// M1.5: mission HUD bundle. `None` when scenario has no mission.
    pub mission: Option<MissionHudView>,
    /// M1.5: extraction zone derived from the first `ReachZone` objective.
    pub extraction_zone: Option<ExtractionZoneView>,
    /// M1.5: per-enemy state + tactic projection so the HUD doesn't fabricate values.
    pub enemies: Vec<EnemyHudView>,
}

/// M1.5: HUD-side projection of one reactive guard.
#[derive(Debug, Clone)]
pub struct EnemyHudView {
    pub actor: u64,
    pub state: String,
    pub last_tactic: String,
}

/// M1.5: render-side projection of a breach strip.
#[derive(Debug, Clone)]
pub struct BreachRenderView {
    pub id: String,
    pub material: String,
    pub bbox_min: [f32; 2],
    pub bbox_max: [f32; 2],
    pub hp: f32,
    pub max_hp: f32,
    pub broken: bool,
    pub refusal_reason: Option<String>,
    /// Maximum distance from the player's centre to the nearest point on the
    /// strip's AABB for the dig to be considered "in range". Mirrors
    /// [`cf_terrain::BreachStrip::dig_range`] so HUD/render consumers can
    /// compute an in-range check that matches the engine's dig contract.
    pub dig_range: f32,
}

/// M1.5: HUD-side projection of mission state.
#[derive(Debug, Clone)]
pub struct MissionHudView {
    pub result: String,
    pub loss_reason: Option<String>,
    pub elapsed_ticks: u64,
    pub time_limit_ticks: u64,
    pub ticks_remaining: Option<u64>,
    pub active_objective: Option<String>,
    pub last_event_label: String,
}

/// M1.5: extraction zone for the renderer.
#[derive(Debug, Clone)]
pub struct ExtractionZoneView {
    pub objective_id: String,
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub completed: bool,
}

/// Rifle ammo / cooldown / reload bundle for the HUD bridge. Mirrors `cf-ui::HudRifle`
/// without depending on cf-ui.
#[derive(Debug, Clone, Default)]
pub struct RifleHudView {
    pub ammo: u32,
    pub capacity: u32,
    pub fire_cooldown_ticks: u32,
    pub reload_remaining_ticks: u32,
    pub reload_total_ticks: u32,
}

/// Compact projection of the actor world emitted as the payload of `actor.actor_snapshot`
/// events at the configured cadence (60 ticks by default).
struct ActorWorldSnapshot {
    actors: Vec<serde_json::Value>,
    player_actor_id: Option<u64>,
}

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
enum DigEvent {
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
    fn outcome_target_string(&self) -> Option<String> {
        match self {
            DigEvent::Strip { outcome, .. } => match outcome {
                cf_terrain::DigOutcome::Carved { strip_id, .. } => Some(strip_id.clone()),
                cf_terrain::DigOutcome::Refused { strip_id, .. } => strip_id.clone(),
            },
            DigEvent::Chunked { .. } => None,
        }
    }

    fn source(&self) -> IntentSource {
        match self {
            DigEvent::Strip { source, .. } | DigEvent::Chunked { source, .. } => *source,
        }
    }

    fn origin(&self) -> [f32; 2] {
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
struct GuardFireRecord {
    shooter: ActorId,
    origin: [f32; 2],
    velocity: [f32; 2],
    damage: f32,
    lifetime_ticks: u32,
    will_miss: bool,
}

/// Build the JsonSchema-friendly mission view used by the observe envelope.
/// M4A: build the module strip placeholder for a single actor. Until M5 lands
/// real chassis modules, the strip carries one weapon_mount slot derived from
/// the actor's selected rifle, plus three `not_present` placeholder slots
/// (jet, shield, sensor) so HUD + accessibility consumers can rely on stable
/// ids before the real implementation lands.
fn build_module_strip_view(
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
fn hud_focusable_nodes() -> Vec<String> {
    HUD_FOCUSABLE_NODES.iter().map(|s| (*s).to_string()).collect()
}

fn build_mission_view(state: &cf_mission::MissionState, current_tick: u64) -> crate::state::MissionView {
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
    }
}

/// Build the checksum bytes covering every M1.5 + BP2 sub-state. Layout is
/// append-only relative to M1 so the `sim_state_v1` suffix stays valid:
/// `(M0 prefix) || (M1 actor bytes) || (M1.5 breach + guards + mission) ||
/// (M2 chunked terrain) || (M2.5 reactor world)`.
/// **M5**: parse a body zone name (`head`, `torso`, ...) into a `cf_chassis::BodyZone`.
fn parse_body_zone(s: &str) -> Option<cf_chassis::BodyZone> {
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

fn build_checksum_bytes(state: &EngineMutable) -> Vec<u8> {
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
    out
}

/// M4A: outcome of one dig used to update the HUD tool-validity cache.
enum ToolValidityUpdate {
    Carve,
    Refuse { reason: String, target: Option<String> },
}

/// M4A: push a banner to the HUD queue, capping at `M4A_BANNER_BUFFER`.
fn push_banner(queue: &mut VecDeque<crate::state::HudBannerView>, banner: crate::state::HudBannerView) {
    queue.push_back(banner);
    while queue.len() > M4A_BANNER_BUFFER {
        queue.pop_front();
    }
}

/// M4A: push a banner only if no banner with the same `id` is already in the
/// queue. Used for "sticky" banners (e.g., AMMO OUT) that should not flicker
/// when conditions persist tick-to-tick.
fn push_banner_dedup(queue: &mut VecDeque<crate::state::HudBannerView>, banner: crate::state::HudBannerView) {
    if queue.iter().any(|b| b.id == banner.id) {
        return;
    }
    push_banner(queue, banner);
}

/// M4A: push a caption to the HUD queue, capping at `M4A_CAPTION_BUFFER`.
fn push_caption(queue: &mut VecDeque<crate::state::CaptionView>, caption: crate::state::CaptionView) {
    queue.push_back(caption);
    while queue.len() > M4A_CAPTION_BUFFER {
        queue.pop_front();
    }
}

/// **M5**: build a HUD banner for a chassis stage transition.
fn chassis_stage_banner(stage: cf_chassis::ChassisStage, now_tick: u64) -> Option<crate::state::HudBannerView> {
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
fn chassis_pilot_banner(state: cf_chassis::PilotState, now_tick: u64) -> Option<crate::state::HudBannerView> {
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
fn status_change_cause(outcome: &ActorTickOutcome) -> &'static str {
    debug_assert!(
        outcome.reset,
        "status_change_cause called for an outcome with no known cause; M1 only emits step_one_actor status changes via actor.reset(). Extend ActorTickOutcome with an explicit cause discriminant before adding new mutators."
    );
    // Defensive fallback for release builds: if a future milestone introduces
    // another status-mutating path inside `step_one_actor` without extending
    // `ActorTickOutcome` with an explicit cause discriminant, mislabeling the
    // change as `reset` would silently corrupt replay/cause-chain analysis.
    // Surfacing `unknown` makes the contract gap visible in the run bundle so
    // it can be caught and fixed rather than masquerading as a reset.
    if outcome.reset {
        "reset"
    } else {
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

fn milestone_order_index(milestone: &str) -> u32 {
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

fn prototype_slice_for_milestone(milestone: &str) -> String {
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
fn next_actions_for_milestone(milestone: &str) -> Vec<String> {
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
fn notes_addendum_for_milestone(milestone: &str) -> String {
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
fn build_test_records(
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
fn discover_run_artifacts(run_bundle_dir: &Path) -> (Vec<ArtifactItem>, Option<String>) {
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

fn apply_settings_patch(settings: &mut Settings, patch: &SettingsPatch) -> Vec<String> {
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
    changed
}

#[async_trait]
impl EngineHandle for M0Engine {
    async fn snapshot(&self, _filter: Option<&str>) -> ObserveFrame {
        let state = self.state.read().expect("engine state poisoned");
        let actors = if let Some(sim) = state.actor_state.as_ref() {
            sim.world
                .actors
                .values()
                .map(|a| {
                    // Gate rifle fields on the actor's currently-selected slot, mirroring
                    // `actor_render_snapshot` (which the cf-app HUD reads). When a non-rifle
                    // slot is selected the wire shows null/None for ammo/capacity/cooldowns
                    // so external observers (cfctl, replay viewers, AI agents) match what
                    // the player sees in the HUD ("NO RIFLE"). The rifle keeps its physical
                    // state in `sim.rifles` regardless of selection — this view is filtered.
                    let rifle = if a.inventory.selected_item().is_rifle() {
                        sim.rifles.get(&a.id)
                    } else {
                        None
                    };
                    let silhouette = a.body_silhouette();
                    // **M5**: when a chassis is attached, the module strip comes
                    // straight from the chassis (placeholder=false). Without a
                    // chassis we fall back to the M4A weapon-mount derivation.
                    let module_strip = match a.chassis_module_strip() {
                        Some(strip) => crate::state::ModuleStripView {
                            modules: strip
                                .modules
                                .iter()
                                .map(|m| crate::state::ModuleStateView {
                                    id: m.id.clone(),
                                    label: m.label.clone(),
                                    state: m.state.clone(),
                                    kind: m.kind.clone(),
                                })
                                .collect(),
                            placeholder: strip.placeholder,
                        },
                        None => build_module_strip_view(rifle, a.inventory.selected_item().is_rifle()),
                    };
                    let chassis_view = a.chassis_view().as_ref().map(crate::state::ChassisView::from);
                    ActorView {
                        id: a.id.0,
                        team: a.team.clone(),
                        controllable: a.controllable,
                        position: [a.position.x, a.position.y],
                        velocity: [a.velocity.x, a.velocity.y],
                        aim: [a.aim.x, a.aim.y],
                        on_ground: a.on_ground,
                        status: a.status.as_str().to_string(),
                        hp: a.hp,
                        hp_max: a.hp_max,
                        selected_slot: a.inventory.selected.0,
                        selected_item: a.inventory.selected_item().label().to_string(),
                        rifle_ammo: rifle.map(|r| r.ammo_in_mag),
                        rifle_capacity: rifle.map(|r| r.spec.mag_capacity),
                        rifle_fire_cooldown_ticks: rifle.map(|r| r.fire_cooldown_ticks),
                        rifle_reload_remaining_ticks: rifle.map(|r| r.reload_remaining_ticks),
                        rifle_reload_total_ticks: rifle.map(|r| r.reload_ticks()),
                        stance: a.stance().as_str().to_string(),
                        body_silhouette: crate::state::BodySilhouetteView {
                            head_hp_pct: silhouette.head_hp_pct,
                            torso_hp_pct: silhouette.torso_hp_pct,
                            arm_left_hp_pct: silhouette.arm_left_hp_pct,
                            arm_right_hp_pct: silhouette.arm_right_hp_pct,
                            leg_left_hp_pct: silhouette.leg_left_hp_pct,
                            leg_right_hp_pct: silhouette.leg_right_hp_pct,
                            placeholder: silhouette.placeholder,
                        },
                        module_strip,
                        chassis: chassis_view,
                        origin_id: a.origin_id.clone(),
                        crouch_active: a.crouch_active,
                        climb_active: a.climb_active,
                        jet_active: a.jet_active,
                    }
                })
                .collect()
        } else {
            Vec::new()
        };
        let player_actor_id = state
            .actor_state
            .as_ref()
            .and_then(|sim| sim.world.player.map(|id| id.0));
        let current_tick_value = state.clock.tick().0;
        let mission = state
            .mission
            .as_ref()
            .map(|m| build_mission_view(m, current_tick_value));
        let breaches = state
            .breach_world
            .as_ref()
            .map(|w| {
                w.iter()
                    .map(|s| crate::state::BreachView {
                        id: s.id.clone(),
                        material: s.material.clone(),
                        bbox_min: s.bbox_min,
                        bbox_max: s.bbox_max,
                        hp: s.hp,
                        max_hp: s.max_hp,
                        broken: s.broken,
                        refusal_reason: s.refusal_reason.clone(),
                        dig_range: s.dig_range,
                    })
                    .collect()
            })
            .unwrap_or_default();
        let enemies: Vec<crate::state::EnemyView> = state
            .reactive_guards
            .values()
            .map(|g| crate::state::EnemyView {
                actor: g.actor.0,
                state: g.state.as_str().to_string(),
                last_tactic: g.last_tactic.as_str().to_string(),
                ammo: g.ammo_in_mag,
                mag_capacity: g.params.mag_capacity,
                fire_cooldown_ticks: g.fire_cooldown_ticks,
                reload_remaining_ticks: g.reload_remaining_ticks,
                aim_settle_remaining_ticks: g.aim_settle_remaining_ticks,
                alert_dwell_remaining_ticks: g.alert_dwell_remaining_ticks,
                aim: g.aim,
            })
            .collect();
        let terrain = state.chunked_terrain.as_ref().map(|t| crate::state::TerrainView {
            width_px: t.width_px,
            height_px: t.height_px,
            anchor: t.anchor,
            default_material: cf_terrain::material_affordance(t.default_material)
                .map(|m| m.name.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            carve_count: t.carve_count,
            refusal_count: t.refusal_count,
            dirty_chunk_count: t.dirty_chunk_count() as u32,
            allocated_chunk_count: t.allocated_chunk_count() as u32,
            material_counts: t.material_counts(),
        });
        let reactors: Vec<crate::state::ReactorView> = state
            .reactor_world
            .as_ref()
            .map(|w| {
                w.iter()
                    .map(|r| crate::state::ReactorView {
                        id: r.id.clone(),
                        position: r.position,
                        half_extents: r.half_extents,
                        hp: r.hp,
                        max_hp: r.max_hp,
                        destroyed: r.is_destroyed(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        // M4A surfaces: banner queue, caption queue, tool-validity, accessibility.
        let banners: Vec<crate::state::HudBannerView> = state.hud_banners.iter().cloned().collect();
        let captions_raw: Vec<crate::state::CaptionView> = state.hud_captions.iter().cloned().collect();
        let captions: Vec<crate::state::CaptionView> = if state.settings.captions {
            captions_raw
        } else {
            // When captions are disabled, the HUD does not render them, but
            // `cfctl observe` still surfaces a structurally-empty queue so AI
            // agents and accessibility tooling can verify the contract holds.
            Vec::new()
        };
        let tool_validity = if state.chunked_terrain.is_some() || state.breach_world.is_some() {
            Some(state.hud_tool_validity.clone())
        } else {
            None
        };
        let accessibility = crate::state::AccessibilityView {
            ui_scale_applied: state.settings.ui_scale,
            high_contrast_applied: state.settings.high_contrast,
            captions_visible: state.settings.captions,
            reduced_motion_applied: state.settings.reduced_motion,
            reduced_shake_applied: state.settings.reduced_shake,
            reduced_flash_applied: state.settings.reduced_flash,
            hold_to_confirm_applied: state.settings.hold_to_confirm,
            hold_threshold_ms: state.settings.hold_threshold_ms,
            key_remap_enabled: state.settings.key_remap_enabled,
            key_bindings: state.settings.key_bindings.clone(),
            focusable_nodes: hud_focusable_nodes(),
            focused_node: state.hud_focus_index.map(|i| HUD_FOCUSABLE_NODES[i].to_string()),
            focus_cycle: state.hud_focus_cycle,
        };
        let frame = ObserveFrame {
            schema_version: SCHEMA_VERSION,
            run_id: self.recorder.run_id().to_string(),
            tick: state.clock.tick().0,
            sim_time_ms: state.clock.sim_time_ms(),
            run_status: observed_run_status(&state),
            scenario: self.config.scenario_id.clone(),
            events_since: self.recorder.snapshot_events().len() as u64,
            events: vec![],
            settings: ObserveSettings {
                schema_version: SCHEMA_VERSION,
                settings: state.settings.clone(),
            },
            actors,
            player_actor_id,
            mission,
            breaches,
            enemies,
            terrain,
            reactors,
            banners,
            captions,
            tool_validity,
            accessibility,
        };
        let tick = state.clock.tick();
        let sim_time_ms = state.clock.sim_time_ms();
        // Record observation_sent BEFORE dropping the lock so that drive_tick (which
        // takes the write lock) cannot insert higher-tick events between this read and
        // the record call. M1.5 emits ~3 events per tick from drive_tick (input/AI/
        // mission), so any race here produces non-monotonic events.jsonl ordering.
        self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "observation_sent",
            json!({"frame_run_id": frame.run_id, "tick": frame.tick}),
            None,
        );
        drop(state);
        frame
    }

    async fn settings_snapshot(&self) -> Settings {
        self.state.read().map(|s| s.settings.clone()).unwrap_or_default()
    }

    async fn dispatch(&self, command: ControlCommand) -> CommandResult {
        let mut state = self.state.write().expect("engine state poisoned");
        let tick = state.clock.tick();
        let sim_time_ms = state.clock.sim_time_ms();
        match command {
            ControlCommand::ScenarioLoad { scenario, seed } => {
                // M0 cannot swap scenarios mid-run (no reload pipeline yet) and cannot
                // re-seed the engine (the RNG/clock are constructed from `config.seed` at
                // engine creation time and `scenario.reset` is the only way to reset them
                // — it uses the original seed). Both cases must be rejected, not faked.
                // (M0.2-F3.)
                if scenario != self.config.scenario_id {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({
                            "method": "scenario.load",
                            "reason": "scenario_swap_not_supported_in_m0",
                            "fix_hint": "M0 ships a single scenario per cf-app launch; relaunch with --scenario <id>. Hot-swap lands at M3."
                        }),
                        None,
                    );
                    CommandResult::rejected("scenario_swap_not_supported_in_m0", tick.0)
                } else if seed.is_some() && seed != Some(self.config.seed) {
                    let requested = seed.unwrap();
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({
                            "method": "scenario.load",
                            "reason": "seed_override_not_supported_in_m0",
                            "active_seed": self.config.seed,
                            "requested_seed": requested,
                            "fix_hint": "M0 cannot re-seed a live engine. Relaunch cf-app with --seed <n>, or use scenario.reset to reset to the original seed."
                        }),
                        None,
                    );
                    CommandResult::rejected("seed_override_not_supported_in_m0", tick.0)
                } else {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_accepted",
                        json!({"method": "scenario.load", "scenario": scenario, "seed": self.config.seed}),
                        None,
                    );
                    CommandResult::accepted(tick.0)
                }
            }
            ControlCommand::ScenarioReset => {
                // Reset the world state (RNG + actor world + pending intent) but do NOT
                // rewind the clock. Rewinding would violate `events.jsonl` monotonicity if
                // any events were recorded at higher ticks before the reset. The clock is a
                // monotonic timeline; `scenario.reset` is a content reload, not a time-warp.
                state.rng = Rng::from_seed(self.config.seed);
                state.tick_durations_us.clear();
                // Capture in-flight projectiles + the projectile-id counter from the old
                // sim state before we replace it. We emit a `combat.projectile_expired`
                // event for each discarded projectile so every `combat.projectile_spawned`
                // entry in the event log has a matched termination event, and we carry
                // the counter forward so post-reset projectile ids never alias pre-reset
                // ones — the event log is a single monotonic timeline that replay
                // analyzers correlate by `projectile_id`.
                let discarded_projectiles: Vec<(u64, ActorId, Vec2)> = state
                    .actor_state
                    .as_ref()
                    .map(|s| s.projectiles.iter().map(|p| (p.id, p.owner, p.position)).collect())
                    .unwrap_or_default();
                let next_projectile_id_carry = state.actor_state.as_ref().map(|s| s.next_projectile_id()).unwrap_or(0);
                // Preserve the pre-reset intent source so the next idle tick's
                // `input.intent_received` event still attributes to whoever was
                // driving (cfctl OR human at the keyboard) rather than spuriously
                // flipping to `cfctl` because the reset handler hardcoded a default.
                let preserved_source = state.pending_intent.source;
                if let Some(initial) = self.config.initial_actor_world.as_ref() {
                    let mut sim_state = ActorSimState::new(initial.world.clone());
                    sim_state.set_next_projectile_id(next_projectile_id_carry);
                    for (id, rifle) in build_rifles_for_world(&initial.world, self.config.tick_rate_hz) {
                        sim_state.ensure_rifle_for(id, rifle);
                    }
                    state.actor_state = Some(sim_state);
                    state.player_actor = initial.player;
                    state.pending_intent = ControlIntent::new(initial.player.unwrap_or(ActorId(0)), preserved_source);
                }
                state.intent_epoch = state.intent_epoch.wrapping_add(1);
                state.pending_dig = None;
                // M1.5: rewind breach world.
                if let (Some(world), Some(initial)) =
                    (state.breach_world.as_mut(), self.config.initial_breach_world.as_ref())
                {
                    *world = initial.world.clone();
                }
                // M1.5: rewind every reactive guard to its initial config so AI
                // memory + ammo + cooldowns reset cleanly.
                for guard in &self.config.initial_guards {
                    if let Some(g) = state.reactive_guards.get_mut(&guard.actor) {
                        *g = cf_ai::ReactiveGuard::new(guard.actor, guard.params);
                    }
                }
                // M1.5: rewind the mission state machine. Started-at-tick stays at
                // the live engine tick so the timer measures from reset.
                if let Some(mission) = state.mission.as_mut() {
                    mission.reset(tick.0);
                }
                // M2: rewind chunked terrain to the manifest's authored stamps.
                if let Some(initial_terrain) = self.config.initial_chunked_terrain.as_ref() {
                    state.chunked_terrain = Some(initial_terrain.clone());
                }
                // M2.5: rewind reactor world to manifest defaults (full hp,
                // not destroyed).
                if let Some(reactor_world) = state.reactor_world.as_mut() {
                    reactor_world.reset();
                }
                drop(state);
                for (projectile_id, owner, last_position) in &discarded_projectiles {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "combat",
                        "projectile_expired",
                        json!({
                            "id": projectile_id,
                            "owner": owner.0,
                            "last_position": [last_position.x, last_position.y],
                            "cause": "scenario_reset",
                        }),
                        None,
                    );
                }
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "scenario.reset"}),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::Pause => {
                state.clock.pause();
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "sim.pause"}),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::Resume => {
                state.clock.resume();
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "sim.resume"}),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::Step { ticks } => {
                if ticks == 0 {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "sim.step", "reason": "ticks_must_be_positive"}),
                        None,
                    );
                    return CommandResult::rejected("ticks_must_be_positive", tick.0);
                }
                state.clock.step(ticks);
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "sim.step", "ticks": ticks}),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::RunForTicks {
                ticks,
                write_run_bundle,
            } => {
                if ticks == 0 {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "sim.run_for_ticks", "reason": "ticks_must_be_positive"}),
                        None,
                    );
                    return CommandResult::rejected("ticks_must_be_positive", tick.0);
                }
                state.clock.step(ticks);
                if write_run_bundle {
                    state.pending_runbundle = true;
                }
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "sim.run_for_ticks", "ticks": ticks, "write_run_bundle": write_run_bundle}),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActPlayerMove { x, y, source } => {
                if !self.config.has_actor_world {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({
                            "method": "act.player.move",
                            "reason": "act_player_move_not_available_in_m0",
                            "x": x,
                            "y": y,
                            "fix_hint": "M0 has no player actor; load an M1 scenario such as m1_actor_range to enable act.player.*."
                        }),
                        None,
                    );
                    return CommandResult::rejected("act_player_move_not_available_in_m0", tick.0);
                }
                // Defense-in-depth: the JSON-RPC server rejects NaN/Inf at the wire
                // layer, but the engine dispatch is also reachable from cf-app's keyboard
                // bridge (and any future bridge / direct-dispatch caller). Reject here
                // too so a non-finite axis cannot leak into pending_intent and NaN-poison
                // the muzzle / projectile path.
                if !x.is_finite() || !y.is_finite() {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({
                            "method": "act.player.move",
                            "reason": "axis_must_be_finite",
                            "x": x,
                            "y": y,
                        }),
                        None,
                    );
                    return CommandResult::rejected("axis_must_be_finite", tick.0);
                }
                let player = state.player_actor;
                if let Some(player_id) = player {
                    state.pending_intent.actor = player_id;
                    state.pending_intent.source = source;
                    state.pending_intent.move_x = x.clamp(-1.0, 1.0);
                    // y is reserved for future ladder/climb input.
                    let _ = y;
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_accepted",
                        json!({"method": "act.player.move", "x": x, "y": y, "actor": player_id.0}),
                        None,
                    );
                    CommandResult::accepted(tick.0)
                } else {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({
                            "method": "act.player.move",
                            "reason": "no_player_actor",
                            "fix_hint": "scenario manifest must declare exactly one actor with controllable=true."
                        }),
                        None,
                    );
                    CommandResult::rejected("no_player_actor", tick.0)
                }
            }
            ControlCommand::ActPlayerJump { source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.jump");
                }
                let player = state.player_actor;
                if let Some(player_id) = player {
                    state.pending_intent.actor = player_id;
                    state.pending_intent.source = source;
                    state.pending_intent.jump = true;
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_accepted",
                        json!({"method": "act.player.jump", "actor": player_id.0}),
                        None,
                    );
                    CommandResult::accepted(tick.0)
                } else {
                    self.reject_actor_command(tick, sim_time_ms, state, "act.player.jump")
                }
            }
            ControlCommand::ActPlayerAim { x, y, source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.aim");
                }
                // Defense-in-depth (mirrors act.player.move): non-finite aim must NEVER
                // reach pending_intent. cf_actor::sim::step normalizes the aim, but
                // `Vec2::normalize_or_x` only short-circuits on a tiny vector — a NaN/Inf
                // input survives normalization and propagates into the muzzle origin,
                // projectile velocity, and recoil sign.
                if !x.is_finite() || !y.is_finite() {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({
                            "method": "act.player.aim",
                            "reason": "aim_must_be_finite",
                            "x": x,
                            "y": y,
                        }),
                        None,
                    );
                    return CommandResult::rejected("aim_must_be_finite", tick.0);
                }
                let player = state.player_actor;
                if let Some(player_id) = player {
                    state.pending_intent.actor = player_id;
                    state.pending_intent.source = source;
                    state.pending_intent.aim = Vec2::new(x, y);
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_accepted",
                        json!({"method": "act.player.aim", "actor": player_id.0, "x": x, "y": y}),
                        None,
                    );
                    CommandResult::accepted(tick.0)
                } else {
                    self.reject_actor_command(tick, sim_time_ms, state, "act.player.aim")
                }
            }
            ControlCommand::ActPlayerFire { pressed, source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.fire");
                }
                let player = state.player_actor;
                if let Some(player_id) = player {
                    state.pending_intent.actor = player_id;
                    state.pending_intent.source = source;
                    // `pressed: false` is an explicit release (a no-op for M1's
                    // single-press rifle per the schema). Only a press raises the
                    // edge so a release sent in the same tick as a prior press
                    // does not erase the queued shot before `drive_tick` runs.
                    if pressed {
                        state.pending_intent.fire = true;
                    }
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_accepted",
                        json!({"method": "act.player.fire", "actor": player_id.0, "pressed": pressed}),
                        None,
                    );
                    CommandResult::accepted(tick.0)
                } else {
                    self.reject_actor_command(tick, sim_time_ms, state, "act.player.fire")
                }
            }
            ControlCommand::ActPlayerReload { source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.reload");
                }
                let player = state.player_actor;
                if let Some(player_id) = player {
                    state.pending_intent.actor = player_id;
                    state.pending_intent.source = source;
                    state.pending_intent.reload = true;
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_accepted",
                        json!({"method": "act.player.reload", "actor": player_id.0}),
                        None,
                    );
                    CommandResult::accepted(tick.0)
                } else {
                    self.reject_actor_command(tick, sim_time_ms, state, "act.player.reload")
                }
            }
            ControlCommand::ActPlayerSelectItem { slot, source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.select_item");
                }
                let player = state.player_actor;
                if let Some(player_id) = player {
                    state.pending_intent.actor = player_id;
                    state.pending_intent.source = source;
                    state.pending_intent.selected_item = Some(ItemSlot(slot));
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_accepted",
                        json!({"method": "act.player.select_item", "actor": player_id.0, "slot": slot}),
                        None,
                    );
                    CommandResult::accepted(tick.0)
                } else {
                    self.reject_actor_command(tick, sim_time_ms, state, "act.player.select_item")
                }
            }
            ControlCommand::ActPlayerReset { source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.reset");
                }
                let player = state.player_actor;
                if let Some(player_id) = player {
                    state.pending_intent.actor = player_id;
                    state.pending_intent.source = source;
                    state.pending_intent.reset = true;
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_accepted",
                        json!({"method": "act.player.reset", "actor": player_id.0}),
                        None,
                    );
                    CommandResult::accepted(tick.0)
                } else {
                    self.reject_actor_command(tick, sim_time_ms, state, "act.player.reset")
                }
            }
            ControlCommand::ActPlayerDig { target, source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.dig");
                }
                if state.breach_world.is_none() && state.chunked_terrain.is_none() {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({
                            "method": "act.player.dig",
                            "reason": "no_terrain_world",
                            "fix_hint": "scenario manifest must declare either breaches[] (M1.5) or terrain (M2 chunked)."
                        }),
                        None,
                    );
                    return CommandResult::rejected("no_terrain_world", tick.0);
                }
                if state.player_actor.is_none() {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.dig");
                }
                state.pending_dig = Some(PendingDig {
                    target: target.clone(),
                    source,
                });
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "act.player.dig", "target": target}),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActInputFocus { direction, source } => {
                let _ = source;
                let n = HUD_FOCUSABLE_NODES.len();
                let prev_idx = state.hud_focus_index;
                let new_idx: Option<usize> = match &direction {
                    crate::server::FocusDirection::Next => Some(match prev_idx {
                        Some(i) => (i + 1) % n,
                        None => 0,
                    }),
                    crate::server::FocusDirection::Prev => Some(match prev_idx {
                        Some(i) => (i + n - 1) % n,
                        None => n - 1,
                    }),
                    crate::server::FocusDirection::Set(node) => {
                        match HUD_FOCUSABLE_NODES.iter().position(|x| *x == node) {
                            Some(i) => Some(i),
                            None => {
                                drop(state);
                                self.recorder.record(
                                    tick,
                                    sim_time_ms,
                                    "control",
                                    "command_rejected",
                                    json!({
                                        "method": "act.input.focus",
                                        "reason": "focus_unknown_node",
                                        "node": node,
                                    }),
                                    None,
                                );
                                return CommandResult::rejected("focus_unknown_node", tick.0);
                            }
                        }
                    }
                    crate::server::FocusDirection::Clear => None,
                };
                state.hud_focus_index = new_idx;
                state.hud_focus_cycle = state.hud_focus_cycle.saturating_add(1);
                let new_node: Option<String> = new_idx.map(|i| HUD_FOCUSABLE_NODES[i].to_string());
                let direction_str = match &direction {
                    crate::server::FocusDirection::Next => "next",
                    crate::server::FocusDirection::Prev => "prev",
                    crate::server::FocusDirection::Set(_) => "set",
                    crate::server::FocusDirection::Clear => "clear",
                };
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({
                        "method": "act.input.focus",
                        "direction": direction_str,
                        "node": new_node,
                    }),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActPlayerCrouch { active, source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.crouch");
                }
                let player_id = state.player_actor.expect("player actor present");
                let _ = source;
                if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        actor.crouch_active = active;
                    }
                }
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "act.player.crouch", "actor": player_id.0, "active": active}),
                    None,
                );
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "actor",
                    "animation_event",
                    json!({
                        "actor": player_id.0,
                        "kind": if active { "crouch_started" } else { "crouch_ended" },
                    }),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActPlayerClimb { active, source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.climb");
                }
                let player_id = state.player_actor.expect("player actor present");
                let _ = source;
                if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        actor.climb_active = active;
                    }
                }
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "act.player.climb", "actor": player_id.0, "active": active}),
                    None,
                );
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "actor",
                    "animation_event",
                    json!({
                        "actor": player_id.0,
                        "kind": if active { "climb_started" } else { "climb_ended" },
                    }),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActPlayerJet { active, source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.jet");
                }
                let player_id = state.player_actor.expect("player actor present");
                let _ = source;
                let mut jet_ok = false;
                let mut reject_reason: Option<String> = None;
                if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        if active {
                            let module_ok = actor
                                .chassis
                                .as_ref()
                                .and_then(|c| c.module_by_kind(cf_chassis::ModuleKind::Jet))
                                .map(|m| {
                                    matches!(
                                        m.state,
                                        cf_chassis::ModuleStateKind::Nominal | cf_chassis::ModuleStateKind::Degraded
                                    )
                                })
                                .unwrap_or(true); // no chassis = treat as no jet, but allow toggle
                            if module_ok {
                                actor.jet_active = true;
                                jet_ok = true;
                            } else {
                                reject_reason = Some("jet_module_unavailable".to_string());
                            }
                        } else {
                            actor.jet_active = false;
                            jet_ok = true;
                        }
                    }
                }
                drop(state);
                if let Some(reason) = reject_reason {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.player.jet", "reason": reason.clone(), "actor": player_id.0}),
                        None,
                    );
                    return CommandResult::rejected(reason, tick.0);
                }
                let _ = jet_ok;
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "act.player.jet", "actor": player_id.0, "active": active}),
                    None,
                );
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "actor",
                    "animation_event",
                    json!({
                        "actor": player_id.0,
                        "kind": if active { "jet_thrust_started" } else { "jet_thrust_ended" },
                    }),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActPlayerEject { source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.player.eject");
                }
                let player_id = state.player_actor.expect("player actor present");
                let _ = source;
                let mut emit: Option<(String, String, u32, bool)> = None;
                let mut reject_reason: Option<String> = None;
                if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        if let Some(chassis) = actor.chassis.as_mut() {
                            if let Some(accepted) = chassis.attempt_eject(tick.0) {
                                emit = Some((
                                    chassis.spec_id.clone(),
                                    chassis.pilot_state.as_str().to_string(),
                                    accepted.ticks_total,
                                    accepted.tutorial_extract,
                                ));
                            } else {
                                reject_reason = Some("pilot_not_in_chassis".to_string());
                            }
                        } else {
                            reject_reason = Some("no_chassis_attached".to_string());
                        }
                    }
                }
                drop(state);
                if let Some(reason) = reject_reason {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.player.eject", "reason": reason.clone(), "actor": player_id.0}),
                        None,
                    );
                    return CommandResult::rejected(reason, tick.0);
                }
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "act.player.eject", "actor": player_id.0}),
                    None,
                );
                if let Some((spec_id, pilot_state, ticks_total, tutorial_extract)) = emit {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "chassis",
                        "pilot_ejected",
                        json!({
                            "actor": player_id.0,
                            "spec_id": spec_id,
                            "pilot_state": pilot_state,
                            "eject_ticks_total": ticks_total,
                            "tutorial_extract": tutorial_extract,
                        }),
                        None,
                    );
                    if tutorial_extract {
                        // Tutorial extract jumps straight to extracted.
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "chassis",
                            "pilot_extracted",
                            json!({"actor": player_id.0, "via": "tutorial_safety"}),
                            None,
                        );
                    }
                }
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActChassisRepair {
                zone,
                module_id,
                reason,
                source,
            } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.chassis.repair");
                }
                let player_id = state.player_actor.expect("player actor present");
                let _ = source;
                let mut zone_result: Option<cf_chassis::RepairOutcome> = None;
                let mut module_result: Option<cf_chassis::ModuleTransition> = None;
                let mut reject_reason: Option<String> = None;
                // **M5**: `act.chassis.repair` is idempotent — a repair on an already-Nominal
                // module/zone returns None (no transition) but the COMMAND succeeds. Only an
                // unknown zone string or an unknown module id rejects; calling repair on a
                // healthy chassis is a no-op accept.
                if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        if let Some(chassis) = actor.chassis.as_mut() {
                            if let Some(zone_str) = &zone {
                                if let Some(zone_kind) = parse_body_zone(zone_str) {
                                    zone_result = chassis.repair_zone(zone_kind, &reason);
                                } else {
                                    reject_reason = Some(format!("chassis_repair_unknown_zone:{zone_str}"));
                                }
                            }
                            if reject_reason.is_none() {
                                if let Some(mid) = &module_id {
                                    // Validate the module id exists on the chassis BEFORE repairing.
                                    // If the module is already Nominal, repair_module returns None
                                    // but the command should still accept (idempotent no-op).
                                    if chassis.module(mid).is_none() {
                                        reject_reason = Some(format!("chassis_repair_unknown_module:{mid}"));
                                    } else {
                                        module_result = chassis.repair_module(mid, &reason);
                                    }
                                }
                            }
                        } else {
                            reject_reason = Some("no_chassis_attached".to_string());
                        }
                    }
                }
                drop(state);
                if let Some(r) = reject_reason {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.chassis.repair", "reason": r.clone(), "actor": player_id.0}),
                        None,
                    );
                    return CommandResult::rejected(r, tick.0);
                }
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({
                        "method": "act.chassis.repair",
                        "actor": player_id.0,
                        "zone": zone,
                        "module_id": module_id,
                        "reason": reason,
                    }),
                    None,
                );
                if let Some(out) = zone_result {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "chassis",
                        "repaired",
                        json!({
                            "actor": player_id.0,
                            "zone": out.zone.as_str(),
                            "was_destroyed": out.was_destroyed,
                            "modules_restored": out.modules_restored,
                            "prev_stage": out.prev_stage.as_str(),
                            "new_stage": out.new_stage.as_str(),
                            "reason": out.reason,
                        }),
                        None,
                    );
                }
                if let Some(t) = module_result {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "chassis",
                        "module_state_changed",
                        json!({
                            "actor": player_id.0,
                            "module_id": t.id,
                            "state": t.state.as_str(),
                            "reason": t.reason,
                        }),
                        None,
                    );
                }
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActChassisSalvage { reason, source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.chassis.salvage");
                }
                let player_id = state.player_actor.expect("player actor present");
                let _ = source;
                let mut salvage_out: Option<cf_chassis::SalvageOutcome> = None;
                let mut reject_reason: Option<String> = None;
                if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        if let Some(chassis) = actor.chassis.as_mut() {
                            salvage_out = chassis.salvage(&reason);
                            if salvage_out.is_none() {
                                reject_reason = Some("chassis_not_wreck_or_disabled".to_string());
                            }
                        } else {
                            reject_reason = Some("no_chassis_attached".to_string());
                        }
                    }
                }
                drop(state);
                if let Some(r) = reject_reason {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.chassis.salvage", "reason": r.clone(), "actor": player_id.0}),
                        None,
                    );
                    return CommandResult::rejected(r, tick.0);
                }
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "act.chassis.salvage", "actor": player_id.0, "reason": reason}),
                    None,
                );
                if let Some(out) = salvage_out {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "chassis",
                        "salvaged",
                        json!({
                            "actor": player_id.0,
                            "salvaged_module_ids": out.salvaged_module_ids,
                            "reason": out.reason,
                        }),
                        None,
                    );
                }
                CommandResult::accepted(tick.0)
            }
            ControlCommand::ActChassisClearJam { source } => {
                if !self.config.has_actor_world {
                    return self.reject_actor_command(tick, sim_time_ms, state, "act.chassis.clear_jam");
                }
                let player_id = state.player_actor.expect("player actor present");
                let _ = source;
                let mut cleared = false;
                if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        if let Some(chassis) = actor.chassis.as_mut() {
                            cleared = chassis.clear_jam();
                        }
                    }
                }
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "act.chassis.clear_jam", "actor": player_id.0, "cleared": cleared}),
                    None,
                );
                if cleared {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "chassis",
                        "weapon_cleared",
                        json!({"actor": player_id.0, "via": "manual"}),
                        None,
                    );
                }
                CommandResult::accepted(tick.0)
            }
            ControlCommand::SettingsSet { changes } => {
                if changes.is_empty() {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.settings.set", "reason": "settings_patch_empty"}),
                        None,
                    );
                    return CommandResult::rejected("settings_patch_empty", tick.0);
                }
                if let Some(reason) = changes.validation_error() {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({"method": "act.settings.set", "reason": reason.clone()}),
                        None,
                    );
                    return CommandResult::rejected(reason, tick.0);
                }
                let changed = apply_settings_patch(&mut state.settings, &changes);
                let new_settings = state.settings.clone();
                drop(state);
                let value = serde_json::to_value(&new_settings).unwrap_or(serde_json::Value::Null);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "settings_changed",
                    json!({"method": "act.settings.set", "fields_changed": changed, "settings": value.clone()}),
                    None,
                );
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "settings_observed",
                    json!({"settings": value}),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::RunBundleWrite { id_override } => {
                if let Some(id_override) = id_override {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({
                            "method": "runbundle.write",
                            "reason": "runbundle_id_override_not_supported_in_m0",
                            "id_override": id_override,
                            "fix_hint": "M0 run ids are deterministic from milestone/time/seed/scenario; explicit bundle id override lands with later tooling if still needed."
                        }),
                        None,
                    );
                    return CommandResult::rejected("runbundle_id_override_not_supported_in_m0", tick.0);
                }
                state.pending_runbundle = true;
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "runbundle.write", "id_override": id_override}),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
            ControlCommand::Shutdown { write_run_bundle } => {
                state.shutdown_requested = true;
                state.pending_runbundle = state.pending_runbundle || write_run_bundle;
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "system.shutdown", "write_run_bundle": write_run_bundle}),
                    None,
                );
                CommandResult::accepted(tick.0)
            }
        }
    }
}

/// Drive an inline run for `duration_ticks`, paced at the configured tick rate
/// when `config.paced` is true. Used by both `cf-app --headless-smoke` and
/// `cfctl run --ticks`. Returns once the target tick count is reached.
pub fn run_m0_inline(config: M0EngineConfig) -> Result<M0EngineOutcome, cf_replay::BundleError> {
    let engine = M0Engine::new(config.clone());
    engine.record_run_started();
    engine.record_setting_snapshot();

    let target_ticks = config.duration_ticks;
    let tick_dt = SimConfig {
        tick_rate_hz: config.tick_rate_hz,
    }
    .tick_dt();
    let started = engine.started_instant();
    let mut next_tick_at = started + tick_dt;

    while engine.current_tick().0 < target_ticks {
        if engine.drive_tick().is_none() {
            break;
        }
        if config.paced {
            let now = Instant::now();
            if next_tick_at > now {
                std::thread::sleep(next_tick_at - now);
            }
            next_tick_at += tick_dt;
        }
    }

    engine.record_run_finished(0);
    let ended_at = WallClock.now_utc();
    let wall_seconds = engine.started_instant().elapsed().as_secs_f64();
    let bundle_dir = if config.write_run_bundle {
        Some(engine.write_run_bundle(ended_at, 0)?)
    } else {
        None
    };

    Ok(M0EngineOutcome {
        run_id: engine.run_id().to_string(),
        bundle_dir,
        final_checksum_hex: engine.recorder().final_checksum_hex(),
        ticks_run: engine.current_tick().0,
        started_at: engine.started_at,
        ended_at,
        wall_seconds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_SCENARIO_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    #[test]
    fn prototype_slice_for_milestone_uppercases_letter_suffix() {
        // Bugbot 3212491755 + Devin 3212416493 regression: letter-suffixed
        // milestones (m3a, m3b, m4a, m4b) must produce uppercase prototype
        // slice strings (M3A, M3B, M4A, M4B). Pre-fix, `format!("M{rest}")`
        // returned the lowercased rest from `to_lowercase()` and produced
        // `M3a` / `M3b` / etc.
        assert_eq!(prototype_slice_for_milestone("m3a"), "M3A");
        assert_eq!(prototype_slice_for_milestone("M3A"), "M3A");
        assert_eq!(prototype_slice_for_milestone("m3b"), "M3B");
        assert_eq!(prototype_slice_for_milestone("m4a"), "M4A");
        assert_eq!(prototype_slice_for_milestone("m4b"), "M4B");
    }

    #[test]
    fn prototype_slice_for_milestone_handles_numeric_and_dotted_milestones() {
        assert_eq!(prototype_slice_for_milestone("m0"), "M0");
        assert_eq!(prototype_slice_for_milestone("m1"), "M1");
        assert_eq!(prototype_slice_for_milestone("m1.5"), "M1.5");
        assert_eq!(prototype_slice_for_milestone("m2"), "M2");
        assert_eq!(prototype_slice_for_milestone("m2.5"), "M2.5");
        assert_eq!(prototype_slice_for_milestone("m5.5.5"), "M5.5.5");
    }

    #[test]
    fn prototype_slice_for_milestone_empty_input_falls_back_to_m0() {
        assert_eq!(prototype_slice_for_milestone(""), "M0");
        assert_eq!(prototype_slice_for_milestone("   "), "M0");
    }

    #[test]
    fn notes_addendum_categories_match_per_milestone_layering() {
        // Devin 3212580450 regression: notes_addendum_for_milestone must NOT
        // claim categories that haven't shipped yet at the named milestone.
        // M0 = system / control / determinism only; M1 adds actor / combat /
        // equipment / input; M1.5 adds ai / mission / terrain; M2 adds
        // material; M3A adds snapshot. Layer is append-only.
        let m0 = notes_addendum_for_milestone("m0");
        assert!(m0.contains("`system`"));
        assert!(m0.contains("`control`"));
        assert!(m0.contains("`determinism`"));
        assert!(!m0.contains("`actor`"), "M0 must NOT advertise actor category");
        assert!(!m0.contains("`material`"), "M0 must NOT advertise material category");
        assert!(!m0.contains("`snapshot`"), "M0 must NOT advertise snapshot category");

        let m1 = notes_addendum_for_milestone("m1");
        assert!(m1.contains("`actor`"));
        assert!(m1.contains("`combat`"));
        assert!(!m1.contains("`material`"), "M1 must NOT advertise material category");
        assert!(!m1.contains("`mission`"), "M1 must NOT advertise mission category");

        let m1_5 = notes_addendum_for_milestone("m1.5");
        assert!(m1_5.contains("`ai`"));
        assert!(m1_5.contains("`mission`"));
        assert!(m1_5.contains("`terrain`"));
        assert!(!m1_5.contains("`material`"), "M1.5 must NOT advertise material (M2+)");
        assert!(!m1_5.contains("`snapshot`"), "M1.5 must NOT advertise snapshot (M3A+)");

        let m2 = notes_addendum_for_milestone("m2");
        assert!(m2.contains("`material`"));
        assert!(!m2.contains("`snapshot`"), "M2 must NOT advertise snapshot (M3A+)");

        let m3a = notes_addendum_for_milestone("m3a");
        assert!(m3a.contains("`snapshot`"));
        assert!(m3a.contains("`material`"));
        assert!(m3a.contains("`mission`"));
    }

    #[test]
    fn notes_addendum_categories_layer_correctly_for_post_m5_10_milestones() {
        // Devin 3212593186 regression: the prior explicit-enumeration match
        // arms stopped at m5.10, so M6/M6.5/M7/M8/etc. silently fell through
        // to "categories shipped: system, control, determinism" only —
        // missing the entire append-only layer they should have inherited.
        // After the milestone_order_index refactor, M6+ correctly inherits
        // every prior category.
        for m in [
            "m6", "m6.5", "m6.6", "m7", "m7.5", "m7.7", "m8", "m8.5", "m8.6", "m9", "m9.5", "m10", "m11", "m12",
        ] {
            let body = notes_addendum_for_milestone(m);
            assert!(body.contains("`actor`"), "{m}: missing actor category");
            assert!(body.contains("`mission`"), "{m}: missing mission category");
            assert!(body.contains("`material`"), "{m}: missing material category");
            assert!(body.contains("`snapshot`"), "{m}: missing snapshot category");
        }
    }

    #[test]
    fn milestone_order_index_orders_canonical_roadmap() {
        assert!(milestone_order_index("m0") < milestone_order_index("m1"));
        assert!(milestone_order_index("m1") < milestone_order_index("m1.5"));
        assert!(milestone_order_index("m1.5") < milestone_order_index("m2"));
        assert!(milestone_order_index("m2") < milestone_order_index("m2.5"));
        assert!(milestone_order_index("m2.5") < milestone_order_index("m3a"));
        assert!(milestone_order_index("m3a") < milestone_order_index("m3b"));
        assert!(milestone_order_index("m3b") < milestone_order_index("m4a"));
        assert!(milestone_order_index("m4a") < milestone_order_index("m4b"));
        assert!(milestone_order_index("m4b") < milestone_order_index("m5"));
        assert!(milestone_order_index("m5") < milestone_order_index("m5.10"));
        assert!(milestone_order_index("m5.10") < milestone_order_index("m6"));
        assert!(milestone_order_index("m6") < milestone_order_index("m12"));
        // Unknown milestones map to MILESTONE_INDEX_UNKNOWN (after M12) so
        // future milestones default to the final-state universe rather than
        // accidentally falling back to M0's empty categories.
        assert!(milestone_order_index("future-milestone-x") > milestone_order_index("m12"));
    }

    #[test]
    fn notes_addendum_includes_dr007_for_every_m2_plus_milestone() {
        // Bugbot 3212607793 + Devin 3212623450 regression: DR-007 is
        // reference documentation for the material set shape. Every M2+
        // bundle has material events in events.jsonl + benefits from the
        // addendum, regardless of whether the milestone EXTENDS or just
        // RUNS ON TOP of chunked terrain. The prior explicit allowlist
        // (M2/M2.5/M3A/M5..M5.10 only) excluded M3B/M4A/M4B + every M6+
        // milestone — including M6.6 'AI Material Competence', M7.5 'Base
        // Atmospherics', M8.5 'Material Lab', M8.6 'Mining'. Switched to
        // `idx >= MILESTONE_INDEX_M2` to match the category-layering
        // pattern.
        for m in [
            "m2", "m2.5", "m3a", "m3b", "m4a", "m4b", "m5", "m5.5", "m5.5.5", "m5.6", "m5.7", "m5.8", "m5.9", "m5.9.5",
            "m5.10", "m6", "m6.5", "m6.6", "m7", "m7.5", "m7.7", "m8", "m8.5", "m8.6", "m9", "m9.5", "m10", "m11",
            "m12",
        ] {
            assert!(
                notes_addendum_for_milestone(m).contains("DR-007 launch material set"),
                "{m} should include DR-007 addendum (idx >= M2)"
            );
        }
        // M0 and M1 are PRE-material — they don't have material events yet,
        // so the addendum is correctly omitted.
        assert!(!notes_addendum_for_milestone("m0").contains("DR-007 launch material set"));
        assert!(!notes_addendum_for_milestone("m1").contains("DR-007 launch material set"));
        assert!(!notes_addendum_for_milestone("m1.5").contains("DR-007 launch material set"));
    }

    fn temp_run_root() -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("cf_engine_test_{}_{}", std::process::id(), uuid_like()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn uuid_like() -> String {
        let now = WallClock.now_utc();
        format!("{}", now.timestamp_nanos_opt().unwrap_or_default())
    }

    fn write_test_scenario() -> PathBuf {
        let mut p = std::env::temp_dir();
        let seq = TEST_SCENARIO_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        p.push(format!("m0_blank_{}_{}.ron", std::process::id(), seq));
        std::fs::write(
            &p,
            r#"(
  schema_version: 1,
  id: "m0_blank",
  display_name: "M0 Blank Scene",
  description: "Empty scene used for engine bootstrap and run-bundle smoke.",
  seed: 42,
  duration_ticks: Some(60),
  region: (anchor: (0.0, 0.0), width: 1280.0, height: 720.0),
  gravity: -980.0,
  teams: [],
  actors: [],
  objectives: [],
  director: None,
  capabilities: (
    debug: false,
    control_api: true,
    save_load: false,
  ),
  save_fields: [],
  expected_tests: ["M0-SMOKE-01"],
  notes: "",
)"#,
        )
        .unwrap();
        p
    }

    fn load_test_scenario_and_config(path: PathBuf) -> M0EngineConfig {
        let scenario = crate::scenario::Scenario::load_from_file(&path).unwrap();
        M0EngineConfig::for_loaded_scenario(&scenario, path)
    }

    #[test]
    fn run_m0_inline_writes_a_valid_bundle() {
        let root = temp_run_root();
        let scenario_path = write_test_scenario();
        let mut config = load_test_scenario_and_config(scenario_path);
        config.run_bundle_root = root.clone();
        config.write_run_bundle = true;
        config.run_mode = "test".to_string();

        let outcome = run_m0_inline(config).unwrap();
        let bundle = outcome.bundle_dir.unwrap();
        let manifest_text = std::fs::read_to_string(bundle.join("run_manifest.json")).unwrap();
        assert!(manifest_text.contains("prototype-run-manifest.v0.1"));
        assert!(manifest_text.contains("\"sim_state_v1\""));
        assert!(manifest_text.contains("\"tick_rate_hz\""));
        let summary: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(bundle.join("summary.json")).unwrap()).unwrap();
        assert!(summary.get("first_tick").is_some());
        assert!(summary.get("last_tick").is_some());
        assert!(summary["performance"]["tick_rate_hz"].is_number());
        // M2 fix: every bundle must have a non-null final checksum.
        assert!(
            summary["final_sim_checksum"].is_string(),
            "final_sim_checksum must not be null; got {}",
            summary["final_sim_checksum"]
        );
        assert!(
            summary["checksum_event_count"].as_u64().unwrap_or(0) >= 1,
            "every bundle must record at least one determinism.sim_checksum"
        );
        let notes = std::fs::read_to_string(bundle.join("notes.md")).unwrap();
        for h in [
            "## Assumptions Tested",
            "## Good",
            "## Bad",
            "## Meh",
            "## Evidence Links",
            "## Next Actions",
        ] {
            assert!(notes.contains(h));
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn run_manifest_records_active_key_bindings() {
        let root = temp_run_root();
        let scenario_path = write_test_scenario();
        let mut config = load_test_scenario_and_config(scenario_path);
        config.run_bundle_root = root.clone();
        config.write_run_bundle = true;
        config.run_mode = "test-remap-manifest".to_string();
        config.settings.key_remap_enabled = true;
        config.settings.key_bindings = std::collections::BTreeMap::from([
            ("aim_up".to_string(), "Numpad8".to_string()),
            ("fire".to_string(), "KeyF".to_string()),
        ]);

        let outcome = run_m0_inline(config).unwrap();
        let bundle = outcome.bundle_dir.unwrap();
        let manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(bundle.join("run_manifest.json")).unwrap()).unwrap();
        assert_eq!(manifest["settings"]["key_remap_enabled"], true);
        assert_eq!(manifest["settings"]["key_bindings"]["aim_up"], "Numpad8");
        assert_eq!(manifest["settings"]["key_bindings"]["fire"], "KeyF");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn for_loaded_scenario_pulls_seed_and_expected_tests_from_manifest() {
        let scenario_path = write_test_scenario();
        let scenario = crate::scenario::Scenario::load_from_file(&scenario_path).unwrap();
        let cfg = M0EngineConfig::for_loaded_scenario(&scenario, scenario_path);
        assert_eq!(cfg.seed, scenario.seed);
        assert_eq!(cfg.duration_ticks, scenario.duration_ticks.unwrap_or(0));
        assert_eq!(cfg.expected_tests, vec!["M0-SMOKE-01".to_string()]);
        assert!((cfg.region_width - 1280.0).abs() < f32::EPSILON);
        assert!((cfg.region_height - 720.0).abs() < f32::EPSILON);
    }

    #[test]
    fn mid_run_write_run_bundle_has_final_checksum() {
        // Repro for the M0.1 follow-up gap: a `runbundle.write` request that fires
        // BEFORE the run is finalized previously produced a bundle with
        // `final_sim_checksum=null` and `checksum_event_count=0`.
        let root = temp_run_root();
        let scenario_path = write_test_scenario();
        let mut config = load_test_scenario_and_config(scenario_path);
        config.duration_ticks = 6;
        config.run_bundle_root = root.clone();
        config.run_mode = "test-mid-run".to_string();
        let engine = M0Engine::new(config);
        engine.record_run_started();
        for _ in 0..6 {
            engine.drive_tick();
        }
        // Write the bundle WITHOUT calling record_run_finished, mimicking the live
        // `runbundle.write` server path.
        let bundle = engine.write_run_bundle(WallClock.now_utc(), 0).unwrap();
        let summary: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(bundle.join("summary.json")).unwrap()).unwrap();
        assert!(
            summary["final_sim_checksum"].is_string(),
            "mid-run runbundle.write must still emit a final checksum; got {}",
            summary["final_sim_checksum"]
        );
        assert!(
            summary["checksum_event_count"].as_u64().unwrap_or(0) >= 1,
            "mid-run bundle must record at least one determinism.sim_checksum"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn panic_in_sub_thread_emits_system_panic_event_and_increments_severity() {
        // M0.2-F5: M0-008 task card requires "panic test triggers a controlled panic in a
        // sub-thread and verifies the event is emitted; counter assertion."
        //
        // The engine wires `M0Engine::new` → `diagnostics::set_panic_reporter` → a closure
        // that calls `report_panic_to_recorder(&recorder, msg)`. This test:
        //   1. Spawns a sub-thread that genuinely calls `panic!`.
        //   2. `JoinHandle::join` catches the panic (returns Err with payload).
        //   3. Routes the captured payload through `report_panic_to_recorder`, which is
        //      the SAME function the global panic hook invokes — bypassing the global
        //      `PANIC_REPORTER` slot only because cargo test parallelism would race
        //      another test's `M0Engine::new` for the slot.
        //   4. Asserts the recorder now contains a `system.panic` event AND the
        //      `by_severity.error` counter advanced.
        let scenario_path = write_test_scenario();
        let config = load_test_scenario_and_config(scenario_path);
        let engine = M0Engine::new(config);
        let recorder = engine.recorder();
        let pre_error_count = recorder.counts().by_severity.get("error").copied().unwrap_or(0);
        let pre_panic_events = recorder
            .snapshot_events()
            .iter()
            .filter(|e| e.category == "system" && e.event_type == "panic")
            .count();

        // Real panic on a sub-thread, real catch via `join`.
        let handle = std::thread::spawn(|| -> () {
            panic!("controlled M0.2-F5 panic for test");
        });
        let join_err = handle.join().expect_err("the spawned thread MUST panic");
        let panic_msg: String = if let Some(s) = join_err.downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = join_err.downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic payload".to_string()
        };

        // Same code path the global panic hook drives (see `M0Engine::new`). Use tick=0
        // because we never advanced the engine.
        report_panic_to_recorder(&recorder, 0, 0.0, &panic_msg);

        let panics: Vec<_> = recorder
            .snapshot_events()
            .into_iter()
            .filter(|e| e.category == "system" && e.event_type == "panic")
            .collect();
        assert!(
            panics.len() > pre_panic_events,
            "system.panic must land in events.jsonl after a sub-thread panic; pre={pre_panic_events} post={}",
            panics.len()
        );
        let recorded_msg = panics.last().unwrap().payload["message"].as_str().unwrap_or("");
        assert!(
            recorded_msg.contains("controlled M0.2-F5 panic for test"),
            "system.panic payload must include the panic message; got `{recorded_msg}`"
        );
        let post_error_count = recorder.counts().by_severity.get("error").copied().unwrap_or(0);
        assert!(
            post_error_count > pre_error_count,
            "system.panic must increment summary.json.event_counts.by_severity.error; pre={pre_error_count} post={post_error_count}"
        );
    }

    #[tokio::test]
    async fn scenario_load_with_mismatched_seed_is_rejected() {
        // M0.2-F3: scenario.load with a seed that differs from the active engine seed
        // must be REJECTED, not silently accepted-and-ignored. M0 cannot re-seed a live
        // engine.
        let scenario_path = write_test_scenario();
        let mut config = load_test_scenario_and_config(scenario_path);
        config.seed = 42;
        let engine = M0Engine::new(config);
        let result = engine
            .dispatch(ControlCommand::ScenarioLoad {
                scenario: "m0_blank".to_string(),
                seed: Some(7),
            })
            .await;
        assert_eq!(
            result.status,
            crate::state::ControlEnvelopeStatus::Rejected,
            "scenario.load with mismatched seed must reject; got {:?}",
            result.status
        );
        assert_eq!(result.reason.as_deref(), Some("seed_override_not_supported_in_m0"));
        // The recorder must have a `command_rejected` event with the right reason.
        let events = engine.recorder().snapshot_events();
        let rejection = events
            .iter()
            .find(|e| {
                e.category == "control" && e.event_type == "command_rejected" && e.payload["method"] == "scenario.load"
            })
            .expect("rejection event must be recorded");
        assert_eq!(rejection.payload["reason"], "seed_override_not_supported_in_m0");
        assert_eq!(rejection.payload["active_seed"], 42);
        assert_eq!(rejection.payload["requested_seed"], 7);
    }

    #[tokio::test]
    async fn scenario_load_with_matching_seed_is_accepted() {
        // F3 follow-up: scenario.load with seed == active seed is a benign no-op and
        // should be accepted (this matches the cfctl client's "reconfirm" semantics).
        let scenario_path = write_test_scenario();
        let mut config = load_test_scenario_and_config(scenario_path);
        config.seed = 42;
        let engine = M0Engine::new(config);
        let result = engine
            .dispatch(ControlCommand::ScenarioLoad {
                scenario: "m0_blank".to_string(),
                seed: Some(42),
            })
            .await;
        assert_eq!(result.status, crate::state::ControlEnvelopeStatus::Accepted);
    }

    #[tokio::test]
    async fn scenario_load_unknown_scenario_is_rejected() {
        let scenario_path = write_test_scenario();
        let config = load_test_scenario_and_config(scenario_path);
        let engine = M0Engine::new(config);
        let result = engine
            .dispatch(ControlCommand::ScenarioLoad {
                scenario: "some_other_scenario".to_string(),
                seed: None,
            })
            .await;
        assert_eq!(result.status, crate::state::ControlEnvelopeStatus::Rejected);
        assert_eq!(result.reason.as_deref(), Some("scenario_swap_not_supported_in_m0"));
    }

    #[tokio::test]
    async fn step_zero_is_rejected_without_status_drift() {
        let scenario_path = write_test_scenario();
        let config = load_test_scenario_and_config(scenario_path);
        let engine = M0Engine::new(config);
        let before = engine.snapshot(None).await;
        let result = engine.dispatch(ControlCommand::Step { ticks: 0 }).await;
        let after = engine.snapshot(None).await;
        assert_eq!(result.status, crate::state::ControlEnvelopeStatus::Rejected);
        assert_eq!(result.reason.as_deref(), Some("ticks_must_be_positive"));
        assert_eq!(after.tick, before.tick, "step(0) must not advance the sim");
        assert_eq!(
            after.run_status, before.run_status,
            "step(0) must not leave observe.once reporting a fake Stepping state"
        );
    }

    #[tokio::test]
    async fn step_completion_observation_pauses_after_requested_ticks() {
        let scenario_path = write_test_scenario();
        let config = load_test_scenario_and_config(scenario_path);
        let engine = M0Engine::new(config);
        let result = engine.dispatch(ControlCommand::Step { ticks: 2 }).await;
        assert_eq!(result.status, crate::state::ControlEnvelopeStatus::Accepted);
        assert_eq!(engine.snapshot(None).await.run_status, RunStatus::Stepping);
        engine.drive_tick();
        assert_eq!(engine.snapshot(None).await.run_status, RunStatus::Stepping);
        engine.drive_tick();
        assert_eq!(
            engine.snapshot(None).await.run_status,
            RunStatus::Paused,
            "observe.once must reflect the SimClock after the requested step count completes"
        );
    }

    #[tokio::test]
    async fn run_for_zero_ticks_is_rejected_without_status_drift() {
        let scenario_path = write_test_scenario();
        let config = load_test_scenario_and_config(scenario_path);
        let engine = M0Engine::new(config);
        let before = engine.snapshot(None).await;
        let result = engine
            .dispatch(ControlCommand::RunForTicks {
                ticks: 0,
                write_run_bundle: true,
            })
            .await;
        let after = engine.snapshot(None).await;
        assert_eq!(result.status, crate::state::ControlEnvelopeStatus::Rejected);
        assert_eq!(result.reason.as_deref(), Some("ticks_must_be_positive"));
        assert_eq!(after.tick, before.tick);
        assert_eq!(after.run_status, before.run_status);
        assert!(
            !engine.pending_runbundle(),
            "rejected run_for_ticks(0) must not queue a run bundle"
        );
    }

    #[tokio::test]
    async fn act_player_move_rejects_until_m1_actor_exists() {
        let scenario_path = write_test_scenario();
        let config = load_test_scenario_and_config(scenario_path);
        let engine = M0Engine::new(config);
        let result = engine
            .dispatch(ControlCommand::ActPlayerMove {
                x: 1.0,
                y: 0.0,
                source: IntentSource::Cfctl,
            })
            .await;
        assert_eq!(result.status, crate::state::ControlEnvelopeStatus::Rejected);
        assert_eq!(result.reason.as_deref(), Some("act_player_move_not_available_in_m0"));
        let rejection = engine
            .recorder()
            .snapshot_events()
            .into_iter()
            .find(|event| event.category == "control" && event.event_type == "command_rejected")
            .expect("rejected act.player.move must record evidence");
        assert_eq!(rejection.payload["method"], "act.player.move");
        assert_eq!(rejection.payload["reason"], "act_player_move_not_available_in_m0");
    }

    #[tokio::test]
    async fn runbundle_id_override_is_rejected_until_supported() {
        let scenario_path = write_test_scenario();
        let config = load_test_scenario_and_config(scenario_path);
        let engine = M0Engine::new(config);
        let result = engine
            .dispatch(ControlCommand::RunBundleWrite {
                id_override: Some("manual-id".to_string()),
            })
            .await;
        assert_eq!(result.status, crate::state::ControlEnvelopeStatus::Rejected);
        assert_eq!(
            result.reason.as_deref(),
            Some("runbundle_id_override_not_supported_in_m0")
        );
        assert!(
            !engine.pending_runbundle(),
            "unsupported id_override must not queue a bundle write"
        );
    }

    #[test]
    fn tick_sample_event_emitted_at_cadence() {
        // M0.2-F4: every cadence_ticks (60 by default) the engine must emit a
        // `system.tick_sample` event with avg/max/p99 in ms and the configured tick rate.
        let scenario_path = write_test_scenario();
        let mut config = load_test_scenario_and_config(scenario_path);
        config.duration_ticks = 60;
        let engine = M0Engine::new(config);
        engine.record_run_started();
        for _ in 0..60 {
            engine.drive_tick();
        }
        let events = engine.recorder().snapshot_events();
        let samples: Vec<_> = events
            .iter()
            .filter(|e| e.category == "system" && e.event_type == "tick_sample")
            .collect();
        assert!(
            !samples.is_empty(),
            "system.tick_sample should fire at least once over 60 ticks @ cadence 60"
        );
        let payload = &samples[0].payload;
        assert_eq!(payload["tick_rate_hz"].as_u64(), Some(60));
        assert!(payload["avg_tick_ms"].is_number());
        assert!(payload["max_tick_ms"].is_number());
        assert!(payload["p99_tick_ms"].is_number());
        assert!(
            payload["samples_observed"].as_u64().unwrap_or(0) >= 1,
            "tick_sample must report at least one sample"
        );
    }

    #[test]
    fn very_short_run_still_has_final_checksum() {
        let root = temp_run_root();
        let scenario_path = write_test_scenario();
        let mut config = load_test_scenario_and_config(scenario_path);
        config.duration_ticks = 1; // shorter than cadence; pre-fix this produced final_sim_checksum=null.
        config.run_bundle_root = root.clone();
        config.write_run_bundle = true;
        config.run_mode = "test-tiny".to_string();
        let outcome = run_m0_inline(config).unwrap();
        assert!(
            outcome.final_checksum_hex.is_some(),
            "1-tick run must still emit a final checksum"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn run_m0_inline_records_tick_rate_120() {
        let root = temp_run_root();
        let scenario_path = write_test_scenario();
        let mut config = load_test_scenario_and_config(scenario_path);
        config.duration_ticks = 30;
        config.tick_rate_hz = 120;
        config.run_bundle_root = root.clone();
        config.write_run_bundle = true;
        config.run_mode = "test-120hz".to_string();
        let outcome = run_m0_inline(config).unwrap();
        let bundle = outcome.bundle_dir.unwrap();
        let manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(bundle.join("run_manifest.json")).unwrap()).unwrap();
        assert_eq!(manifest["tick_rate_hz"], 120);
        assert!((manifest["duration_target_sec"].as_f64().unwrap() - 0.25).abs() < 1e-6);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn run_m0_inline_paced_takes_real_time() {
        let root = temp_run_root();
        let scenario_path = write_test_scenario();
        let mut config = load_test_scenario_and_config(scenario_path);
        config.duration_ticks = 30;
        config.tick_rate_hz = 60;
        config.run_bundle_root = root.clone();
        config.write_run_bundle = false;
        config.run_mode = "test-paced".to_string();
        config.paced = true;
        let outcome = run_m0_inline(config).unwrap();
        // 30 ticks at 60 Hz = 0.5 s. Allow a small lower bound.
        assert!(
            outcome.wall_seconds >= 0.45,
            "paced run should be near 0.5 s wall, got {}",
            outcome.wall_seconds
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn settings_set_propagates_to_observe() {
        let scenario_path = write_test_scenario();
        let mut config = load_test_scenario_and_config(scenario_path);
        config.tick_rate_hz = 60;
        let engine = M0Engine::new(config);

        let s0 = engine.settings_snapshot().await;
        assert!((s0.ui_scale - 1.0).abs() < f32::EPSILON);

        let _ = engine
            .dispatch(ControlCommand::SettingsSet {
                changes: SettingsPatch {
                    ui_scale: Some(2.0),
                    high_contrast: Some(true),
                    captions: Some(false),
                    ..SettingsPatch::default()
                },
            })
            .await;
        let s1 = engine.settings_snapshot().await;
        assert!((s1.ui_scale - 2.0).abs() < f32::EPSILON);
        assert!(s1.high_contrast);
        assert!(!s1.captions);

        let frame = engine.snapshot(None).await;
        assert!((frame.settings.settings.ui_scale - 2.0).abs() < f32::EPSILON);
        assert!(frame.settings.settings.high_contrast);
    }

    #[tokio::test]
    async fn settings_set_clamps_ui_scale_before_observe() {
        let scenario_path = write_test_scenario();
        let config = load_test_scenario_and_config(scenario_path);
        let engine = M0Engine::new(config);

        let _ = engine
            .dispatch(ControlCommand::SettingsSet {
                changes: SettingsPatch {
                    ui_scale: Some(0.01),
                    ..SettingsPatch::default()
                },
            })
            .await;
        let low_settings = engine.settings_snapshot().await;
        assert!((low_settings.ui_scale - crate::settings::UI_SCALE_MIN).abs() < f32::EPSILON);
        let low_frame = engine.snapshot(None).await;
        assert!((low_frame.accessibility.ui_scale_applied - crate::settings::UI_SCALE_MIN).abs() < f32::EPSILON);

        let _ = engine
            .dispatch(ControlCommand::SettingsSet {
                changes: SettingsPatch {
                    ui_scale: Some(99.0),
                    ..SettingsPatch::default()
                },
            })
            .await;
        let high_settings = engine.settings_snapshot().await;
        assert!((high_settings.ui_scale - crate::settings::UI_SCALE_MAX).abs() < f32::EPSILON);
        let high_frame = engine.snapshot(None).await;
        assert!((high_frame.accessibility.ui_scale_applied - crate::settings::UI_SCALE_MAX).abs() < f32::EPSILON);
    }

    #[test]
    fn config_hash_is_stable_for_inputs() {
        let scenario_path = PathBuf::from("/tmp/scenario.ron");
        let mut a = M0EngineConfig::for_test_scenario_only("m0_blank", scenario_path.clone());
        let mut b = M0EngineConfig::for_test_scenario_only("m0_blank", scenario_path);
        a.fill_config_hash();
        b.fill_config_hash();
        assert_eq!(a.config_hash, b.config_hash);
        assert!(!a.config_hash.is_empty());
    }

    fn write_m1_scenario() -> PathBuf {
        let mut p = std::env::temp_dir();
        let seq = TEST_SCENARIO_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        p.push(format!("m1_actor_range_{}_{}.ron", std::process::id(), seq));
        std::fs::write(
            &p,
            r#"(
  schema_version: 1,
  id: "m1_actor_range",
  display_name: "M1 Actor Range",
  description: "M1 engine test fixture.",
  seed: 7,
  duration_ticks: Some(120),
  region: (anchor: (0.0, 0.0), width: 1280.0, height: 720.0),
  gravity: -980.0,
  floor_y: 16.0,
  teams: [],
  actors: [
    (id: 1, team: "blue", spawn: (200.0, 32.0), controllable: true, hp: 100.0,
      inventory: (rifle: Some("rifle_m1_default")), half_extents: Some((8.0, 16.0))),
    (id: 2, team: "red", spawn: (900.0, 32.0), controllable: false, hp: 100.0,
      inventory: (rifle: None)),
  ],
  objectives: [],
  director: None,
  capabilities: (debug: false, control_api: true, save_load: false),
  save_fields: [],
  expected_tests: ["M1-SMOKE-01"],
  notes: "",
)"#,
        )
        .unwrap();
        p
    }

    fn load_m1_test_config(path: PathBuf) -> M0EngineConfig {
        let scenario = crate::scenario::Scenario::load_from_file(&path).unwrap();
        M0EngineConfig::for_loaded_scenario(&scenario, path)
    }

    #[tokio::test]
    async fn m1_act_player_move_updates_pending_intent_and_emits_input_event() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();

        let result = engine
            .dispatch(ControlCommand::ActPlayerMove {
                x: 1.0,
                y: 0.0,
                source: IntentSource::Cfctl,
            })
            .await;
        assert_eq!(result.status, crate::state::ControlEnvelopeStatus::Accepted);
        engine.drive_tick();
        let events = engine.recorder().snapshot_events();
        let intent = events
            .iter()
            .find(|e| e.category == "input" && e.event_type == "intent_received")
            .expect("input.intent_received must be recorded");
        assert!((intent.payload["move_x"].as_f64().unwrap() - 1.0).abs() < 1e-3);
    }

    #[tokio::test]
    async fn m1_act_player_fire_spawns_projectile_event() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();

        let result = engine
            .dispatch(ControlCommand::ActPlayerFire {
                pressed: true,
                source: IntentSource::Cfctl,
            })
            .await;
        assert_eq!(result.status, crate::state::ControlEnvelopeStatus::Accepted);
        engine.drive_tick();
        let events = engine.recorder().snapshot_events();
        assert!(
            events
                .iter()
                .any(|e| e.category == "equipment" && e.event_type == "weapon_fired"),
            "weapon_fired must land in events: {:?}",
            events
                .iter()
                .map(|e| (e.category.clone(), e.event_type.clone()))
                .collect::<Vec<_>>()
        );
        assert!(
            events
                .iter()
                .any(|e| e.category == "combat" && e.event_type == "projectile_spawned"),
            "projectile_spawned must land in events"
        );
    }

    #[tokio::test]
    async fn m1_act_player_fire_release_preserves_queued_press() {
        // Regression proof for the cf-app keyboard bridge contract: key release sends
        // `pressed: false` so future hold-to-fire weapons can observe release edges.
        // M1's rifle is press-edge driven, so release must be accepted but must not
        // erase a still-unconsumed press before the next fixed tick drains the intent.
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();

        let press = engine
            .dispatch(ControlCommand::ActPlayerFire {
                pressed: true,
                source: IntentSource::Human,
            })
            .await;
        assert_eq!(press.status, crate::state::ControlEnvelopeStatus::Accepted);

        let release = engine
            .dispatch(ControlCommand::ActPlayerFire {
                pressed: false,
                source: IntentSource::Human,
            })
            .await;
        assert_eq!(
            release.status,
            crate::state::ControlEnvelopeStatus::Accepted,
            "explicit fire release must stay a valid command"
        );

        engine.drive_tick();
        let events = engine.recorder().snapshot_events();
        let intent = events
            .iter()
            .find(|e| e.category == "input" && e.event_type == "intent_received")
            .expect("input.intent_received must be recorded after press+release");
        assert_eq!(
            intent.payload.get("source").and_then(|v| v.as_str()),
            Some("human"),
            "same-tick press+release should retain the human source"
        );
        assert_eq!(
            intent.payload.get("fire").and_then(|v| v.as_bool()),
            Some(true),
            "release must not clobber the queued fire edge before drive_tick"
        );
        assert!(
            events
                .iter()
                .any(|e| e.category == "equipment" && e.event_type == "weapon_fired"),
            "queued press must still fire after same-tick release; events: {:?}",
            events
                .iter()
                .map(|e| (e.category.clone(), e.event_type.clone(), e.payload.clone()))
                .collect::<Vec<_>>()
        );
        assert!(
            events
                .iter()
                .any(|e| e.category == "combat" && e.event_type == "projectile_spawned"),
            "queued press must still spawn a projectile after same-tick release"
        );
    }

    #[tokio::test]
    async fn m1_act_player_aim_normalizes_and_records_event() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();

        let _ = engine
            .dispatch(ControlCommand::ActPlayerAim {
                x: 0.0,
                y: 1.0,
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        let frame = engine.snapshot(None).await;
        let player = frame
            .actors
            .iter()
            .find(|a| Some(a.id) == frame.player_actor_id)
            .unwrap();
        // Aim normalized to unit vector (0, 1).
        assert!((player.aim[1] - 1.0).abs() < 1e-3);
    }

    #[tokio::test]
    async fn m1_act_player_jump_rejected_in_air_recorded() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();

        // First jump from spawn (above ground) — actor is NOT on_ground until physics
        // drops it, so the first jump is refused. Tick a few times so the actor lands.
        for _ in 0..6 {
            engine.drive_tick();
        }
        // Now jump should succeed.
        let _ = engine
            .dispatch(ControlCommand::ActPlayerJump {
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        let events = engine.recorder().snapshot_events();
        let jumped = events
            .iter()
            .any(|e| e.category == "actor" && e.event_type == "actor_jumped");
        assert!(jumped, "actor_jumped should land after the actor settles on the floor");
    }

    #[tokio::test]
    async fn m1_act_player_reset_emits_actor_reset_event() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();

        let _ = engine
            .dispatch(ControlCommand::ActPlayerReset {
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        let events = engine.recorder().snapshot_events();
        assert!(events.iter().any(|e| e.event_type == "actor_reset"));
    }

    #[tokio::test]
    async fn m1_act_player_select_item_changes_slot_in_observation() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();
        let _ = engine
            .dispatch(ControlCommand::ActPlayerSelectItem {
                slot: 1,
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        let frame = engine.snapshot(None).await;
        let player = frame
            .actors
            .iter()
            .find(|a| Some(a.id) == frame.player_actor_id)
            .unwrap();
        assert_eq!(player.selected_slot, 1);
    }

    #[tokio::test]
    async fn m1_actor_render_snapshot_hides_rifle_when_non_rifle_slot_selected() {
        // M1-FIX-9 regression: actor_render_snapshot() must clear player_rifle when
        // the player's currently-selected slot is not a rifle, so the HUD shows
        // "NO RIFLE" instead of READY/COOLDOWN.
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();
        // Default selection (slot 0 = rifle) - HUD should show rifle.
        let snap_a = engine.actor_render_snapshot();
        assert!(snap_a.player_rifle.is_some(), "rifle slot selected -> HUD shows rifle");
        // Select an empty slot.
        let _ = engine
            .dispatch(ControlCommand::ActPlayerSelectItem {
                slot: 1,
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        let snap_b = engine.actor_render_snapshot();
        assert!(
            snap_b.player_rifle.is_none(),
            "non-rifle slot -> HUD hides rifle (NO RIFLE)"
        );
        // Switch back to slot 0.
        let _ = engine
            .dispatch(ControlCommand::ActPlayerSelectItem {
                slot: 0,
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        let snap_c = engine.actor_render_snapshot();
        assert!(snap_c.player_rifle.is_some(), "back to slot 0 -> HUD shows rifle again");
    }

    #[tokio::test]
    async fn m1_observe_actor_view_hides_rifle_state_when_non_rifle_slot_selected() {
        // Mirrors `m1_actor_render_snapshot_hides_rifle_when_non_rifle_slot_selected` for
        // the wire-format `ActorView` exposed via `observe.once` / `observe.subscribe`.
        // The cfctl/replay/AI consumers must see the same NO RIFLE state the player sees
        // in the HUD; otherwise external observers mis-attribute fire-press behavior.
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();
        // Default selection (slot 0 = rifle) - ActorView must show rifle fields.
        let frame_a = engine.snapshot(None).await;
        let player_a = frame_a
            .actors
            .iter()
            .find(|a| Some(a.id) == frame_a.player_actor_id)
            .unwrap();
        assert!(
            player_a.rifle_ammo.is_some(),
            "rifle slot selected -> rifle_ammo populated"
        );
        assert!(player_a.rifle_capacity.is_some());
        assert!(
            player_a.rifle_reload_total_ticks.is_some(),
            "rifle slot selected -> reload total is visible to cfctl/AI observers"
        );

        let _ = engine
            .dispatch(ControlCommand::ActPlayerFire {
                pressed: true,
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        let _ = engine
            .dispatch(ControlCommand::ActPlayerReload {
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        let frame_reload = engine.snapshot(None).await;
        let player_reload = frame_reload
            .actors
            .iter()
            .find(|a| Some(a.id) == frame_reload.player_actor_id)
            .unwrap();
        assert!(
            player_reload
                .rifle_reload_remaining_ticks
                .is_some_and(|ticks| ticks > 0),
            "reload command should expose remaining reload ticks"
        );
        assert_eq!(
            player_reload.rifle_reload_total_ticks,
            Some(90),
            "M1 rifle reload is 1.5s at the 60 Hz test default"
        );

        // Select an empty slot.
        let _ = engine
            .dispatch(ControlCommand::ActPlayerSelectItem {
                slot: 1,
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        let frame_b = engine.snapshot(None).await;
        let player_b = frame_b
            .actors
            .iter()
            .find(|a| Some(a.id) == frame_b.player_actor_id)
            .unwrap();
        assert!(
            player_b.rifle_ammo.is_none(),
            "non-rifle slot -> rifle_ammo must be None on the wire"
        );
        assert!(player_b.rifle_capacity.is_none());
        assert!(player_b.rifle_fire_cooldown_ticks.is_none());
        assert!(player_b.rifle_reload_remaining_ticks.is_none());
        assert!(player_b.rifle_reload_total_ticks.is_none());
        // Re-select rifle slot 0.
        let _ = engine
            .dispatch(ControlCommand::ActPlayerSelectItem {
                slot: 0,
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        let frame_c = engine.snapshot(None).await;
        let player_c = frame_c
            .actors
            .iter()
            .find(|a| Some(a.id) == frame_c.player_actor_id)
            .unwrap();
        assert!(
            player_c.rifle_ammo.is_some(),
            "back to slot 0 -> rifle_ammo populated again"
        );
        assert_eq!(player_c.rifle_reload_total_ticks, Some(90));
    }

    #[tokio::test]
    async fn m1_actor_snapshot_event_emitted_at_cadence() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();
        for _ in 0..60 {
            engine.drive_tick();
        }
        let events = engine.recorder().snapshot_events();
        assert!(events
            .iter()
            .any(|e| e.category == "actor" && e.event_type == "actor_snapshot"));
    }

    #[tokio::test]
    async fn m1_observe_includes_actor_view_with_rifle_state() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();
        let frame = engine.snapshot(None).await;
        assert!(!frame.actors.is_empty());
        let player = frame
            .actors
            .iter()
            .find(|a| Some(a.id) == frame.player_actor_id)
            .unwrap();
        assert_eq!(player.rifle_capacity, Some(30));
        assert_eq!(player.rifle_ammo, Some(30));
    }

    #[tokio::test]
    async fn m1_dead_player_rejects_movement_input() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();
        // Force player into Dead status by directly mutating world state via reset-then-damage.
        {
            let mut state = engine.state.write().unwrap();
            if let Some(sim) = state.actor_state.as_mut() {
                let player = sim.world.player_actor_mut().unwrap();
                let _ = player.apply_damage(1000.0);
            }
        }
        let _ = engine
            .dispatch(ControlCommand::ActPlayerMove {
                x: 1.0,
                y: 0.0,
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        // Actor should not have moved (status::Dead refuses input).
        let frame = engine.snapshot(None).await;
        let player = frame
            .actors
            .iter()
            .find(|a| Some(a.id) == frame.player_actor_id)
            .unwrap();
        assert_eq!(player.status, "dead");
    }

    #[tokio::test]
    async fn m1_scenario_reset_rebuilds_actor_world() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();
        // Move + fire to mutate state.
        let _ = engine
            .dispatch(ControlCommand::ActPlayerMove {
                x: 1.0,
                y: 0.0,
                source: IntentSource::Cfctl,
            })
            .await;
        let _ = engine
            .dispatch(ControlCommand::ActPlayerFire {
                pressed: true,
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        // Reset.
        let _ = engine.dispatch(ControlCommand::ScenarioReset).await;
        let frame = engine.snapshot(None).await;
        let player = frame
            .actors
            .iter()
            .find(|a| Some(a.id) == frame.player_actor_id)
            .unwrap();
        // After reset, the actor is at spawn (200, 32) with full ammo.
        assert!((player.position[0] - 200.0).abs() < 0.5);
        assert_eq!(player.rifle_ammo, Some(30));
    }

    #[tokio::test]
    async fn m1_scenario_reset_preserves_intent_source() {
        // Regression: ScenarioReset rebuilt pending_intent with a hardcoded
        // IntentSource::Cfctl regardless of who was previously controlling the actor.
        // Now we preserve the pre-reset source so the next idle tick's
        // input.intent_received correctly attributes (cfctl OR human) and the
        // replay event log doesn't contain spurious source flips on reset.
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();
        // Drive a Human-source aim so pending_intent.source = Human.
        let _ = engine
            .dispatch(ControlCommand::ActPlayerAim {
                x: 1.0,
                y: 0.0,
                source: IntentSource::Human,
            })
            .await;
        // Now reset — pre-fix this would clobber source back to Cfctl.
        let _ = engine.dispatch(ControlCommand::ScenarioReset).await;
        // Next tick should record input.intent_received with source = human.
        engine.drive_tick();
        let events = engine.recorder().snapshot_events();
        let intent_events: Vec<_> = events
            .iter()
            .filter(|e| e.category == "input" && e.event_type == "intent_received")
            .collect();
        let last_intent = intent_events.last().expect("at least one intent_received event");
        assert_eq!(
            last_intent.payload.get("source").and_then(|v| v.as_str()),
            Some("human"),
            "post-reset intent must preserve the Human source",
        );
    }

    #[tokio::test]
    async fn m1_act_player_aim_accepts_finite_at_engine_layer() {
        // Sanity: with finite values, engine dispatch accepts aim.
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();
        let result = engine
            .dispatch(ControlCommand::ActPlayerAim {
                x: 1.0,
                y: 0.0,
                source: IntentSource::Cfctl,
            })
            .await;
        assert_eq!(result.status, crate::state::ControlEnvelopeStatus::Accepted);
    }

    #[tokio::test]
    async fn m1_act_player_aim_rejects_nonfinite_at_engine_layer() {
        // Defense-in-depth: the JSON-RPC server layer rejects NaN/Inf before dispatch
        // (see live_ws_m1_act_player_aim_nan_rejected). The engine ALSO rejects at the
        // dispatch boundary so any future caller (cf-app keyboard bridge, future mouse
        // bridge, future gamepad bridge, future direct-dispatch script) cannot leak
        // NaN/Inf into pending_intent and NaN-poison the muzzle / projectile path.
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();
        for (x, y) in [
            (f32::NAN, 0.0),
            (0.0, f32::NAN),
            (f32::INFINITY, 0.0),
            (0.0, f32::NEG_INFINITY),
        ] {
            let result = engine
                .dispatch(ControlCommand::ActPlayerAim {
                    x,
                    y,
                    source: IntentSource::Cfctl,
                })
                .await;
            assert_eq!(
                result.status,
                crate::state::ControlEnvelopeStatus::Rejected,
                "aim ({x}, {y}) must reject"
            );
            assert_eq!(result.reason.as_deref(), Some("aim_must_be_finite"));
        }
    }

    #[tokio::test]
    async fn m1_act_player_move_rejects_nonfinite_at_engine_layer() {
        // Defense-in-depth mirror for act.player.move (cf-app's keyboard bridge produces
        // 0.0 / ±1.0 today, but a future mouse / gamepad / scripted bridge could send a
        // NaN/Inf move axis through engine.dispatch directly).
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();
        for (x, y) in [
            (f32::NAN, 0.0),
            (0.0, f32::NAN),
            (f32::INFINITY, 0.0),
            (0.0, f32::NEG_INFINITY),
        ] {
            let result = engine
                .dispatch(ControlCommand::ActPlayerMove {
                    x,
                    y,
                    source: IntentSource::Cfctl,
                })
                .await;
            assert_eq!(
                result.status,
                crate::state::ControlEnvelopeStatus::Rejected,
                "move ({x}, {y}) must reject"
            );
            assert_eq!(result.reason.as_deref(), Some("axis_must_be_finite"));
        }
    }

    #[tokio::test]
    async fn m1_kill_chain_records_actor_status_changed_with_projectile_hit_cause() {
        // M1-D04 end-to-end evidence via the dispatch path: drive the engine through
        // act.player.aim + act.player.fire enough times to kill the dummy, then assert
        // the recorder captured an actor.actor_status_changed event with cause
        // "projectile_hit". Engine + sim test `projectile_eventually_hits_dummy_and_can_kill_it`
        // already proves the underlying physics; this test adds the dispatch + event
        // emission proof.
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();
        // Settle to ground first.
        for _ in 0..10 {
            engine.drive_tick();
        }
        let _ = engine
            .dispatch(ControlCommand::ActPlayerAim {
                x: 1.0,
                y: 0.0,
                source: IntentSource::Cfctl,
            })
            .await;
        // Fire 9 shots (dummy has 100 HP, rifle 12 dmg/hit → 9 hits = 108 dmg). Each shot
        // requires the rifle's fire interval (6 ticks) to cool down between presses.
        let fire_interval_ticks = cf_equipment::rifle_preset(cf_equipment::RIFLE_M1_DEFAULT_ID)
            .unwrap()
            .fire_interval_ticks(60) as usize;
        for _ in 0..12 {
            let _ = engine
                .dispatch(ControlCommand::ActPlayerFire {
                    pressed: true,
                    source: IntentSource::Cfctl,
                })
                .await;
            // Drive enough ticks for the fired projectile to reach the dummy at x=900
            // before the next shot (player at x=200, projectile speed 1200 unit/s ≈ 20
            // unit/tick at 60 Hz → 35 ticks to cross 700 units).
            for _ in 0..fire_interval_ticks.max(35) {
                engine.drive_tick();
            }
        }
        let events = engine.recorder().snapshot_events();
        let kill_event = events.iter().find(|e| {
            e.category == "actor"
                && e.event_type == "actor_status_changed"
                && e.payload["new_status"] == "dead"
                && e.payload["cause"] == "projectile_hit"
        });
        assert!(
            kill_event.is_some(),
            "expected a projectile_hit-caused dead status transition; got events: {:?}",
            events
                .iter()
                .filter(|e| e.event_type == "actor_status_changed")
                .map(|e| e.payload.clone())
                .collect::<Vec<_>>()
        );
    }
}
