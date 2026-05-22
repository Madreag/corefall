//! M0 inline engine: drives the fixed-tick sim, emits the lock-approved event
//! categories (`system`, `control`, `determinism`), writes a run bundle, and
//! exposes an `EngineHandle` so the WebSocket server can drive the same engine.

#![allow(unused_imports, dead_code)]

use std::{
    collections::{BTreeMap, VecDeque},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::Instant,
};

use crate::engine::{bevy_version_string, env_platform, rustc_version_string};
use crate::engine_build::{
    build_atmos_cell, build_gravity_override, build_m14d_projectile_snapshot, build_rifles_for_world,
    build_strat_cell, build_wind_source, gas_from_label, m9_concussion_band_for_dose, next_unit_draw,
    registry_color_hex_for,
};

pub(crate) const M4A_BANNER_BUFFER: usize = 8;
pub(crate) const M4A_CAPTION_BUFFER: usize = 8;
pub(crate) const M4A_STATUS_BANNER_EXPIRY_TICKS: u64 = 180;
pub(crate) const M4A_CAPTION_EXPIRY_TICKS: u64 = 120;

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
    /// engine flags `ActorState::set_inactive(true)` for every controllable
    /// actor on scene-load until the manifest's tutorial-controller flips
    /// it off (M1.5 owns the tutorial controller; M1 just plumbs the flag).
    pub tutorial_safety: bool,
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
    /// Each entry pairs (actor_id, SquadRole, display_name). Empty for
    /// scenarios with no squad declarations.
    pub initial_squad_members: Vec<InitialSquadMember>,
    /// M3A: configurable checksum cadence. 0 = disabled. Default from
    /// `ChecksumConfig::m0_default().cadence_ticks` (60).
    pub checksum_cadence_ticks: u64,
    /// manifest. Applied to each reactive guard at engine construction.
    /// `None` keeps each guard's params as authored.
    pub difficulty_preset: Option<String>,
    /// inferred default in `build_manifest`. `None` lets the engine derive
    /// the outcome (Panic if `debug_inject_panic_at_tick` is set, else
    /// Clean).
    pub expected_outcome_override: Option<cf_replay::ExpectedOutcome>,
    /// state seeded from the scenario manifest. `None` opts out — the
    /// engine still ticks `advance_phase` but it returns None until
    /// `init_phase` is called.
    pub initial_phase_state: Option<cf_mission::PhaseState>,
    /// declarations the engine flattens into `M7AiWorld.reinforcements`.
    pub initial_reinforcement_waves: Vec<cf_mission::ReinforcementWave>,
    /// seeded into `M7AiWorld.boss`. `None` opts out — `apply_boss_damage`
    /// returns None and no `boss.*` events fire.
    pub initial_boss_state: Option<cf_mission::BossState>,
    /// graph seeded into `M7AiWorld.objective_graph`. `None` opts out —
    /// the M2 single-vec objective list keeps working unchanged.
    pub initial_objective_graph: Option<cf_mission::ObjectiveGraph>,
    /// `snapshot.baseline_emitted` events. Default 600 (10 s @ 60 Hz);
    /// 0 disables snapshot emission entirely.
    pub delta_baseline_cadence_ticks: u64,
    /// recorder runs in chain mode (per-event BLAKE3 keyed hash + final
    /// anchor in `RunManifest.ledger_chain_anchor`). Default false.
    pub ledger_chain_enabled: bool,
    /// Empty by default; producer-side `cf_physics::apply_overrides` reads
    /// this list each tick.
    pub initial_gravity_overrides: Vec<cf_physics::GravityOverride>,
    pub initial_wind_sources: Vec<cf_atmos::WindSource>,
    /// the wind force kernel + stratification.
    pub initial_atmosphere_cells: Vec<cf_atmos::AtmosCell>,
    /// to `initial_atmosphere_cells` by `cell_id`). Empty = pure-air default.
    pub initial_stratification_cells: Vec<cf_atmos::StratCell>,
    /// `drive_tick` reads this each tick and injects matching intents into
    /// `pending_intent` before the actor sim runs. Empty by default.
    pub initial_scripted_steps: Vec<crate::scenario::ScenarioScriptStep>,
    /// manifest's `m14d_projectile_pool[]` field. Drives the per-tick
    /// projectile-pair CCD pass (`cf_physics::run_projectile_pair_pass`).
    /// Empty by default — pre-M14D scenarios behave identically.
    pub initial_m14d_projectile_pool: Vec<cf_physics::ProjectileSnapshot>,
    /// setting. Default false — killcam excludes
    /// `collision.projectile_pair_contact` events unless the player
    /// opts in via this setting.
    pub initial_replay_intercepts: bool,
    /// Empty by default; M14E scenarios populate one or more spans for
    /// the per-tick collapse-check pass.
    pub initial_m14e_tunnel_spans: Vec<crate::scenario::ScenarioTunnelSpan>,
    /// engine's `seed` to derive the cave-in RNG state.
    pub initial_m14e_cave_in_seed_offset: u64,
    /// scenario manifest. Empty by default; M14F scenarios populate
    /// one or more rows so the lateral integrity pass + bulging →
    /// crack_advanced → rupture cascade fires against a known sidewall
    /// topology. Shares the same per-chunk `IntegrityField` buffer as
    /// the ceiling pass per VAL-CROSS-005.
    pub initial_m14f_lateral_wall_spans: Vec<crate::scenario::LateralWallSpan>,
    /// from the scenario manifest. The engine ticks each zone's dwell
    /// counter every tick and runs
    /// [`cf_environment::classify_tile_thermal`] to emit typed burn /
    /// frostbite wounds.
    pub initial_m14g_thermal_zones: Vec<crate::scenario::ScenarioThermalZone>,
    /// the scenario manifest. Each entry fires one
    /// [`cf_material::classify_reaction`] call at its `fire_tick`.
    pub initial_m14g_material_contacts: Vec<crate::scenario::ScenarioMaterialContact>,
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
    /// into a per-tick AI cover-decision pipeline. Currently only
    /// `"AI-TRENCH-A-01"` is recognised (the M9B trench garrison
    /// doctrine); unknown values are ignored.
    #[allow(dead_code)]
    pub doctrine: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InitialSquadMember {
    pub actor: ActorId,
    pub role: cf_squad::SquadRole,
    pub display_name: String,
    pub hp_max: f32,
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
            tutorial_safety: false,
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
            initial_squad_members: Vec::new(),
            checksum_cadence_ticks: ChecksumConfig::m0_default().cadence_ticks,
            difficulty_preset: None,
            expected_outcome_override: None,
            initial_phase_state: None,
            initial_reinforcement_waves: Vec::new(),
            initial_boss_state: None,
            initial_objective_graph: None,
            // ticks (10 s @ 60 Hz) per spec.
            delta_baseline_cadence_ticks: cf_save::delta::DEFAULT_BASELINE_CADENCE_TICKS,
            // OFF by default for dev runs. Tournament mode opts in via cf-app
            // / cfctl `--ledger-chain` / `--tournament-mode`.
            ledger_chain_enabled: false,
            // gravity_overrides / wind_sources / atmosphere_cells in the
            // manifest.
            initial_gravity_overrides: Vec::new(),
            initial_wind_sources: Vec::new(),
            initial_atmosphere_cells: Vec::new(),
            initial_stratification_cells: Vec::new(),
            initial_scripted_steps: Vec::new(),
            // `m14d_projectile_pool[]` / `m14d_replay_intercepts` in the
            // manifest.
            initial_m14d_projectile_pool: Vec::new(),
            initial_replay_intercepts: false,
            // `m14e_tunnel_spans[]` and (optionally) `m14e_cave_in_seed_offset`.
            initial_m14e_tunnel_spans: Vec::new(),
            initial_m14e_cave_in_seed_offset: 0,
            // scenarios declare `m14f_lateral_wall_spans[]`.
            initial_m14f_lateral_wall_spans: Vec::new(),
            // scenarios opt in by declaring `m14g_thermal_zones[]`
            // and `m14g_material_contacts[]`.
            initial_m14g_thermal_zones: Vec::new(),
            initial_m14g_material_contacts: Vec::new(),
        }
    }

    /// Build a config from a scenario manifest. Pulls `seed`, `duration_ticks`, `expected_tests`,
    /// and `region` straight out of the loaded `Scenario`. The CLI may still override individual
    /// fields after this call.
    pub fn for_loaded_scenario(scenario: &crate::scenario::Scenario, scenario_path: PathBuf) -> Self {
        let mut cfg = Self::for_test_scenario_only(&scenario.id, scenario_path.clone());
        cfg.seed = scenario.seed;
        cfg.duration_ticks = scenario.duration_ticks.unwrap_or(0);
        // M1 Seam S2: forward scenario.tutorial_safety into the engine config.
        cfg.tutorial_safety = scenario.tutorial_safety;
        // engine config so the engine can apply the preset at spawn time.
        cfg.difficulty_preset = scenario.difficulty_preset.clone();
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
                    doctrine: enemy.doctrine.clone(),
                });
            }
        }
        for actor in &scenario.actors {
            if let Some(role_str) = actor.squad_role.as_deref() {
                let role = match role_str.to_ascii_lowercase().as_str() {
                    "leader" => Some(cf_squad::SquadRole::Leader),
                    "follower" => Some(cf_squad::SquadRole::Follower),
                    _ => None,
                };
                if let Some(role) = role {
                    let display_name = actor
                        .squad_archetype
                        .clone()
                        .unwrap_or_else(|| format!("Actor {}", actor.id));
                    cfg.initial_squad_members.push(InitialSquadMember {
                        actor: ActorId(actor.id),
                        role,
                        display_name,
                        hp_max: actor.hp,
                    });
                }
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
        // mission director fields from the scenario manifest into the
        // engine config so `M0Engine::new` can seed `M7AiWorld` with
        // phase pacing + reinforcement waves + boss state + objective
        // graph at construction time. Each field is optional so the
        // M2 single-vec objective list keeps working unchanged.
        if let Some(phase) = &scenario.phase_state {
            cfg.initial_phase_state = Some(phase.build_phase_state());
        }
        cfg.initial_reinforcement_waves = scenario.reinforcement_waves.iter().map(|w| w.build_wave()).collect();
        if let Some(boss) = &scenario.boss_state {
            cfg.initial_boss_state = Some(boss.build_boss_state());
        }
        if let Some(graph) = &scenario.objective_graph {
            cfg.initial_objective_graph = Some(graph.build_graph());
        }
        // cf-physics + cf-atmos types from the scenario manifest's
        // `gravity_overrides[]` / `wind_sources[]` / `atmosphere_cells[]`
        // arrays. Empty arrays = pass-through (no overrides).
        cfg.initial_gravity_overrides = scenario.gravity_overrides.iter().map(build_gravity_override).collect();
        cfg.initial_wind_sources = scenario.wind_sources.iter().map(build_wind_source).collect();
        cfg.initial_atmosphere_cells = scenario.atmosphere_cells.iter().map(build_atmos_cell).collect();
        cfg.initial_stratification_cells = scenario.atmosphere_cells.iter().map(build_strat_cell).collect();
        cfg.initial_scripted_steps = scenario.scripted_steps.clone();
        // replay_intercepts setting.
        cfg.initial_m14d_projectile_pool = scenario
            .m14d_projectile_pool
            .iter()
            .map(build_m14d_projectile_snapshot)
            .collect();
        cfg.initial_replay_intercepts = scenario.m14d_replay_intercepts;
        // offset so the per-tick collapse-check pass seeds correctly.
        cfg.initial_m14e_tunnel_spans = scenario.m14e_tunnel_spans.clone();
        cfg.initial_m14e_cave_in_seed_offset = scenario.m14e_cave_in_seed_offset;
        cfg.initial_m14f_lateral_wall_spans = scenario.m14f_lateral_wall_spans.clone();
        // thermal-contact + material-contact fixtures so the engine's
        // per-tick environmental + chemistry passes have something to
        // chew on.
        cfg.initial_m14g_thermal_zones = scenario.m14g_thermal_zones.clone();
        cfg.initial_m14g_material_contacts = scenario.m14g_material_contacts.clone();
        // Promote the milestone tag when the scenario uses M14B producers.
        if !scenario.gravity_overrides.is_empty()
            || !scenario.wind_sources.is_empty()
            || !scenario.atmosphere_cells.is_empty()
        {
            cfg.milestone = "m14b".to_string();
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

