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
    server::{async_trait, CommandResult, ControlCommand, EngineHandle, SettingsPatch},
    state::{ObserveFrame, ObserveSettings, RunStatus},
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
        cfg
    }

    pub fn config_hash_input(&self) -> String {
        format!(
            "milestone={}|scenario={}|seed={}|ticks={}|hz={}|region={:?}|mode={}|control_api={}|debug={}|settings={:?}|expected_tests={:?}",
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
    /// every `cadence_ticks` ticks (M0 default = 60).
    pub fn drive_tick(&self) -> Option<Tick> {
        let start = Instant::now();
        let mut state = self.state.write().expect("engine state poisoned");
        let advanced = state.clock.advance();
        let mut checksum_payload: Option<(Tick, f64, String)> = None;
        let mut tick_sample_payload: Option<(Tick, f64, TickSampleStats)> = None;
        if let Some(tick) = advanced {
            state.rng.next_u64();
            let cadence = ChecksumConfig::m0_default().cadence_ticks;
            if cadence > 0 && tick.0 % cadence == 0 {
                let cs = sim_state_v1(tick, &state.rng);
                let sim_time_ms = state.clock.sim_time_ms();
                checksum_payload = Some((tick, sim_time_ms, cs.to_hex()));
                // M0.2-F4: emit a tick_sample summarizing the last `cadence` ticks.
                let stats = TickSampleStats::from_recent(&state.tick_durations_us, cadence as usize);
                tick_sample_payload = Some((tick, sim_time_ms, stats));
            }
        }
        let elapsed_us = start.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
        state.tick_durations_us.push(elapsed_us);
        let new_tick = state.clock.tick().0;
        drop(state);
        // Publish the latest tick so the panic reporter records `system.panic` at the
        // current tick (preserves events.jsonl monotonic ordering).
        self.current_tick.store(new_tick, std::sync::atomic::Ordering::Relaxed);
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
        advanced
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
        let cs = sim_state_v1(tick, &state.rng);
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
            duration_target_sec: f64::from(self.config.duration_ticks as u32) / f64::from(self.config.tick_rate_hz),
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

fn m0_notes_addendum() -> String {
    "## DR-002 v1 schema lock\n\n\
- Event envelope: `{schema_version, run_id, tick, sim_time_ms, event_id, category, event_type, payload, parent_event_id?, dropped_count?}`.\n\
- M0 categories: `system`, `control`, `determinism`. `snapshot` opens at M3.\n\
- Checksum: `algorithm=blake3`, `scope=sim_state_v1` (M0 covers `tick_counter || rng_state_bytes`; M2/M3 append actor/inventory/terrain bytes without bumping the suffix; layout-breaking bumps go to `_v2`).\n\
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
                state.clock = SimClock::new(SimConfig {
                    tick_rate_hz: self.config.tick_rate_hz,
                });
                state.rng = Rng::from_seed(self.config.seed);
                state.tick_durations_us.clear();
                let tick = state.clock.tick();
                drop(state);
                self.recorder.record(
                    tick,
                    0.0,
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
            ControlCommand::ActPlayerMove { x, y } => {
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
                        "fix_hint": "M0 has no player actor; M1 wires act.player.move to ControlIntent."
                    }),
                    None,
                );
                CommandResult::rejected("act_player_move_not_available_in_m0", tick.0)
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
        let result = engine.dispatch(ControlCommand::ActPlayerMove { x: 1.0, y: 0.0 }).await;
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
}
