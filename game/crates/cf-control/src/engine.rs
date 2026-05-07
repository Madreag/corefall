//! M0 inline engine: drives the fixed-tick sim, emits the lock-approved event
//! categories (`system`, `control`, `determinism`), writes a run bundle, and
//! exposes an `EngineHandle` so the WebSocket server can drive the same engine.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::Instant,
};

use chrono::{DateTime, Utc};
use serde_json::json;

use cf_actor::{
    sim::{step as actor_step, ActorSimState, ActorTickOutcome, StepDeps, StepReport},
    ActorId, ActorWorld, ControlIntent, IntentSource, ItemSlot, Vec2,
};
use cf_replay::{
    diagnostics, BuildInfo, BundleInputs, CapabilitiesBlock, CaptureConfig, ChecksumConfig, PerfSample, Recorder,
    RunManifest, SceneInfo, SettingsBlock, TestRecord, CONTROL_SCHEMA_VERSION, EVENT_ENVELOPE_VERSION,
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
    /// Region dimensions copied from the scenario manifest (for run-bundle metadata).
    pub region_width: f32,
    pub region_height: f32,
    pub config_hash: String,
    pub commit_sha: String,
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
        let mut world = ActorWorld::new(scenario.floor_y, scenario.gravity);
        for actor in &scenario.actors {
            let state = actor.build_state();
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
            region_width: 0.0,
            region_height: 0.0,
            config_hash: String::new(),
            commit_sha: env!("CARGO_PKG_VERSION").to_string(),
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
        }
    }

    /// Build a config from a scenario manifest. Pulls `seed`, `duration_ticks`, `expected_tests`,
    /// and `region` straight out of the loaded `Scenario`. The CLI may still override individual
    /// fields after this call.
    pub fn for_loaded_scenario(scenario: &crate::scenario::Scenario, scenario_path: PathBuf) -> Self {
        let mut cfg = Self::for_test_scenario_only(&scenario.id, scenario_path);
        cfg.seed = scenario.seed;
        cfg.duration_ticks = scenario.duration_ticks.unwrap_or(0);
        cfg.expected_tests = if scenario.expected_tests.is_empty() {
            vec!["M0-SMOKE-01".to_string()]
        } else {
            scenario.expected_tests.clone()
        };
        cfg.region_width = scenario.region.width;
        cfg.region_height = scenario.region.height;
        if scenario.has_actor_world() {
            cfg.has_actor_world = true;
            cfg.initial_actor_world = Some(InitialActorWorld::from_scenario(scenario));
            // Bump the milestone hint when the scenario actually carries an actor world.
            // Per-actor RifleState is built lazily in M0Engine::new with the configured
            // tick_rate_hz so 60 Hz vs 120 Hz produce identical real-time RPS / reload.
            cfg.milestone = "m1".to_string();
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
            (self.region_width, self.region_height),
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

const BEVY_VERSION_FALLBACK: &str = "0.14";

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
            let mut sim_state = ActorSimState::new(initial.world.clone());
            for (id, rifle) in build_rifles_for_world(&initial.world, config.tick_rate_hz) {
                sim_state.ensure_rifle_for(id, rifle);
            }
            (Some(sim_state), initial.player)
        } else {
            (None, None)
        };
        let pending_intent = ControlIntent::new(player_actor.unwrap_or(ActorId(0)), IntentSource::Cfctl);

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

    pub fn record_run_started(&self) {
        let state = self.state.read().expect("engine state poisoned");
        let tick = state.clock.tick();
        let sim_time_ms = state.clock.sim_time_ms();
        let settings_value = serde_json::to_value(&state.settings).unwrap_or(serde_json::Value::Null);
        drop(state);
        self.recorder.record(
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
                "settings": settings_value,
            }),
            None,
        );
        self.spawn_debug_panic_if_requested();
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
        if let Some(tick) = advanced {
            state.rng.next_u64();
            // M1: step the actor world if present. The pending intent is consumed and
            // its edge-triggered fields cleared so the next tick starts fresh.
            if state.actor_state.is_some() {
                let intent = state.pending_intent.clone();
                state.pending_intent.clear_edges();
                let region_min_x = 0.0_f32;
                let region_max_x = self.config.region_width.max(0.0);
                let region_max_y = self.config.region_height.max(0.0);
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
                let actor_state = state.actor_state.as_mut().expect("actor state present");
                let report = actor_step(
                    actor_state,
                    &mut intents,
                    StepDeps {
                        tick_dt,
                        region_min_x,
                        region_max_x,
                        region_max_y,
                        auto_reload_when_empty: auto_reload,
                    },
                );
                step_report = Some((tick, state.clock.sim_time_ms(), intent, report));
            }
            let cadence = ChecksumConfig::m0_default().cadence_ticks;
            if cadence > 0 && tick.0 % cadence == 0 {
                let actor_bytes = state
                    .actor_state
                    .as_ref()
                    .map(|s| s.checksum_bytes())
                    .unwrap_or_default();
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
                    "cadence_ticks": ChecksumConfig::m0_default().cadence_ticks,
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
        advanced
    }

    fn emit_actor_events(&self, tick: Tick, sim_time_ms: f64, intent: &ControlIntent, report: &StepReport) {
        // input.intent_received reflects what was actually consumed (after status gating).
        let player_outcome = report.actor_outcomes.iter().find(|o| o.actor == intent.actor).cloned();
        let player_view = json!({
            "actor": intent.actor.0,
            "source": match intent.source {
                IntentSource::Human => "human",
                IntentSource::Cfctl => "cfctl",
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
            let projectile_event_id = format!("projectile:{}", hit.projectile_id);
            self.recorder.record(
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
                        "projectile_event": projectile_event_id,
                    }),
                    Some(intent_event_id.clone()),
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
        let actor_bytes = state
            .actor_state
            .as_ref()
            .map(|s| s.checksum_bytes())
            .unwrap_or_default();
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
                "cadence_ticks": ChecksumConfig::m0_default().cadence_ticks,
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
        };
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
        let inputs = BundleInputs {
            recorder: &self.recorder,
            manifest,
            started_at: self.started_at,
            ended_at,
            exit_code,
            result: if exit_code == 0 {
                "pass".to_string()
            } else {
                "fail".to_string()
            },
            blockers: vec![],
            next_actions: vec!["Proceed to M1 task cards in spec/native-implementation-backlog.".to_string()],
            tests: vec![TestRecord {
                id: "M0-SMOKE-01".to_string(),
                result: if exit_code == 0 {
                    "pass".to_string()
                } else {
                    "fail".to_string()
                },
                evidence_event_ids: self.first_and_last_event_ids(),
                notes: Some("Fixed-tick smoke + run bundle parity per M0 done-criteria.".to_string()),
            }],
            artifacts: vec![],
            assumptions_tested: self.config.assumptions_tested.clone(),
            good: vec![
                "Fixed-tick scheduler stable; cfctl/control envelope serializes per DR-002 v1 lock.".to_string(),
            ],
            bad: vec![],
            meh: vec![],
            evidence_links: vec![
                "events.jsonl".to_string(),
                "summary.json".to_string(),
                "run_manifest.json".to_string(),
            ],
            notes_extra: m0_notes_addendum(),
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
            prototype_slice: "M0".to_string(),
            run_mode: self.config.run_mode.clone(),
            milestone: self.config.milestone.clone(),
            build: BuildInfo {
                commit_sha: self.config.commit_sha.clone(),
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
            material_schema_version: "n/a-m0".to_string(),
            config_hash: self.config.config_hash.clone(),
            assumptions_tested: self.config.assumptions_tested.clone(),
            linked_specs: self.config.linked_specs.clone(),
            expected_tests: self.config.expected_tests.clone(),
            capture_config: CaptureConfig::default(),
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
            },
            checksum: ChecksumConfig::m0_default(),
            tick_rate_hz: self.config.tick_rate_hz,
        }
    }
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

/// Cause label for `actor.actor_status_changed` events emitted from `step_one_actor`.
///
/// In M1 the only mutator inside `step_one_actor` that touches `actor.status` is
/// `actor.reset()` (called when the player issues `act.player.reset`). Damage-driven
/// transitions land in the projectile-hit loop with cause `projectile_hit`. The
/// `intent` branch is reserved for future intent-driven status changes (e.g. M5
/// chassis ejection, M5.6 hazard contact) and is not currently reachable.
fn status_change_cause(outcome: &ActorTickOutcome) -> &'static str {
    if outcome.reset {
        "reset"
    } else {
        "intent"
    }
}

fn m0_notes_addendum() -> String {
    "## DR-002 v1 schema lock\n\n\
- Event envelope: `{schema_version, run_id, tick, sim_time_ms, event_id, category, event_type, payload, parent_event_id?, dropped_count?}`.\n\
- M0 categories: `system`, `control`, `determinism`. `snapshot` opens at M3.\n\
- Checksum: `algorithm=blake3`, `scope=sim_state_v1` (M0 covers `tick_counter || rng_state_bytes`; M1 appends actor/inventory/projectile bytes via `cf_actor::sim::ActorSimState::checksum_bytes()`; M2/M3 will append terrain bytes; all without bumping the suffix; layout-breaking bumps go to `_v2`).\n\
- Manifest extensions: `checksum.{algorithm,scope,cadence_ticks}`, `settings:{...}` block.\n\
- Summary extensions: `final_sim_checksum`, `checksum_event_count`, `first_tick`, `last_tick`.\n\
- M3 picks up replay verification (`first_divergence` event), the `snapshot` category, and full headless replay parity.\n\
\n## DR-012 floor lock\n\n\
- Six accessibility flags wired into `cf-control::Settings` and `run_manifest.json.settings`.\n\
- Settings can be live-updated via `act.settings.set` and re-read via `observe.settings`.\n\
- Localization deferred to M4 — the discipline rule (no baked English-only player-facing strings) applies.\n"
        .to_string()
}

fn apply_settings_patch(settings: &mut Settings, patch: &SettingsPatch) -> Vec<String> {
    let mut changed = Vec::new();
    if let Some(v) = patch.ui_scale {
        if (settings.ui_scale - v).abs() > f32::EPSILON {
            settings.ui_scale = v;
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
        };
        let tick = state.clock.tick();
        let sim_time_ms = state.clock.sim_time_ms();
        drop(state);
        self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "observation_sent",
            json!({"frame_run_id": frame.run_id, "tick": frame.tick}),
            None,
        );
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
                if let Some(initial) = self.config.initial_actor_world.as_ref() {
                    let mut sim_state = ActorSimState::new(initial.world.clone());
                    for (id, rifle) in build_rifles_for_world(&initial.world, self.config.tick_rate_hz) {
                        sim_state.ensure_rifle_for(id, rifle);
                    }
                    state.actor_state = Some(sim_state);
                    state.player_actor = initial.player;
                    state.pending_intent =
                        ControlIntent::new(initial.player.unwrap_or(ActorId(0)), IntentSource::Cfctl);
                }
                state.intent_epoch = state.intent_epoch.wrapping_add(1);
                drop(state);
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
