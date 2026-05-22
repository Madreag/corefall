//! M0 inline engine: drives the fixed-tick sim, emits the lock-approved event
//! categories (`system`, `control`, `determinism`), writes a run bundle, and
//! exposes an `EngineHandle` so the WebSocket server can drive the same engine.

use std::{
    collections::{BTreeMap, VecDeque},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::Instant,
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

/// Build a per-actor `RifleState` map from the world for the configured `tick_rate_hz`.
/// Each actor whose currently-selected slot OR any other slot holds a rifle gets a
/// state entry. We key on inventory contents (not on selection) so swapping slots at
/// runtime never strands an existing rifle's ammo / cooldown.
pub(crate) use crate::engine_build::{
    build_atmos_cell, build_gravity_override, build_m14d_projectile_snapshot,
    build_rifles_for_world, build_strat_cell, build_wind_source, gas_from_label,
    m9_concussion_band_for_dose, next_unit_draw, registry_color_hex_for,
};

pub use crate::engine_save::{
    ActorRenderSnapshot, BreachRenderView, EnemyHudView, ExtractionZoneView, HudCachesSnapshot,
    MissionHudView, ReactorArmorLayerView, ReactorHudView, RifleHudView, TerrainChunkUpdate,
    TerrainDigPreview, TerrainRenderSnapshot, TimerHudView,
};
pub(crate) use crate::engine_save::ActorWorldSnapshot;
pub(crate) use crate::engine_helpers::{
    ai_intent_label, apply_settings_patch, build_checksum_bytes, build_mission_view,
    build_module_strip_view, build_test_records, chassis_pilot_banner, chassis_stage_banner,
    discover_run_artifacts, effective_sim_speed_pct, hud_focusable_nodes, milestone_order_index,
    next_actions_for_milestone, notes_addendum_for_milestone, parse_body_zone,
    prototype_slice_for_milestone, push_banner, push_banner_dedup, push_caption,
    resolve_hud_node_at, status_change_cause, DigEvent, GuardFireRecord, M14SaveExtension,
    ToolValidityUpdate, M14_SAVE_EXTENSION_KEY,
};

/// gravity + wind force corrections without holding the read lock.
#[derive(Debug, Clone, Copy)]
pub(crate) struct M14bActorSnapshot {
    pub(crate) actor_id: ActorId,
    pub(crate) pos: [f32; 2],
    pub(crate) mass: f32,
    pub(crate) half_extent_y: f32,
    pub(crate) on_ground: bool,
    pub(crate) velocity: [f32; 2],
}

pub(crate) use crate::engine_m15::{
    build_heat_field_from_atmosphere, infer_ambient_world_from_scenario_id,
    inject_thermal_sources_and_diffuse, u16_slice_to_bytes,
};

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

pub(crate) fn env_platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

pub(crate) fn rustc_version_string() -> String {
    option_env!("CFAPP_RUSTC_VERSION").unwrap_or("unknown").to_string()
}

pub(crate) fn bevy_version_string() -> String {
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
pub(crate) const TICK_DURATIONS_HISTORY_CAP: usize = 4096;

/// Periodic per-tick performance stats emitted as `system.tick_sample`. Keeps M0 evidence
/// of per-tick cost in the run bundle without waiting for the M3 perf overlay.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TickSampleStats {
    /// How many ticks of history this sample summarized.
    pub(crate) window_ticks: u64,
    pub(crate) avg_tick_ms: f64,
    pub(crate) max_tick_ms: f64,
    pub(crate) p99_tick_ms: f64,
    /// Actual number of stored samples used (may be less than `window_ticks` early in a run).
    pub(crate) samples_observed: u64,
}

impl TickSampleStats {
pub(crate)     fn from_recent(samples_us: &[u64], window: usize) -> Self {
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
    pub(crate) config: M0EngineConfig,
    pub(crate) state: RwLock<EngineMutable>,
    pub(crate) recorder: Arc<Recorder>,
    /// Lock-free snapshot of the engine's current tick. Updated by `drive_tick` so that
    /// the panic reporter (which fires from a panicking thread, possibly while another
    /// thread holds `state`) can record `system.panic` at the right tick without
    /// blocking on the `RwLock`. Also drives `sim_time_ms` for the panic event.
    pub(crate) current_tick: Arc<std::sync::atomic::AtomicU64>,
    pub(crate) started_at: DateTime<Utc>,
    pub(crate) started_instant: Instant,
    pub(crate) run_bundle_dir: PathBuf,
    /// (no-op + tracing). cf-app or cf-tools-replay-viewer install their own
    /// implementation via `set_audio_plugin` to play real sound.
    pub(crate) audio_plugin: std::sync::Mutex<Box<dyn cf_audio::AudioPlugin>>,
    /// snapshot of the last quicksave / quickload / migrate operation.
    /// Updated by the cf-app F5/F9 path + `cfctl save quicksave/quickload`.
    pub(crate) last_save_cache: Arc<crate::m4b_save::LastSaveCache>,
}

pub(crate) struct EngineMutable {
    pub(crate) clock: SimClock,
    pub(crate) rng: Rng,
    pub(crate) settings: Settings,
    pub(crate) pending_runbundle: bool,
    pub(crate) shutdown_requested: bool,
    pub(crate) tick_durations_us: Vec<u64>,
    /// M1: pending player intent for the next tick. The dispatch handlers update fields
    /// here; `drive_tick` consumes the intent, applies it, then clears the edge-triggered
    /// fields. Continuous fields (`move_x`, `aim`) persist tick-to-tick.
    pub(crate) pending_intent: ControlIntent,
    /// M1: actor world + rifles + projectiles. `None` for M0 scenarios.
    pub(crate) actor_state: Option<ActorSimState>,
    /// Cached player actor id from the actor world for fast access.
    pub(crate) player_actor: Option<ActorId>,
    /// Monotonic counter incremented whenever `pending_intent` is externally
    /// reset (e.g. `scenario.reset` zeroes it). Edge-detecting input bridges
    /// (`cf-app::ingest_player_input`) watch this to know when their cached
    /// "last sent" trackers are stale and must redispatch held keys, even if
    /// the keyboard state itself has not changed.
    pub(crate) intent_epoch: u64,
    /// M1.5: breach world (soft-breach strips). `None` when scenario has no breaches.
    pub(crate) breach_world: Option<cf_terrain::BreachWorld>,
    /// M1.5: pending dig request consumed at the start of the next tick.
    /// `Some` only when an `act.player.dig` arrived since the last tick.
    pub(crate) pending_dig: Option<PendingDig>,
    /// M1.5: per-actor reactive-guard controllers, keyed by actor id.
    pub(crate) reactive_guards: BTreeMap<ActorId, cf_ai::ReactiveGuard>,
    /// M1.5: mission state machine. `None` when the scenario is sandbox-only.
    pub(crate) mission: Option<cf_mission::MissionState>,
    /// M1.5: monotonic id counter for guard projectiles. We share the actor
    /// projectile pool but allocate ids from a separate range so guard shots
    /// don't alias the player's projectile_id space across resets.
    pub(crate) next_guard_projectile_id: u64,
    /// M2: chunked pixel terrain. `None` for scenarios that have not opted
    /// into chunked terrain. Coexists with `breach_world`.
    pub(crate) chunked_terrain: Option<cf_terrain::ChunkedTerrain>,
    /// M2.5: reactor world (damageable static actors). `None` when no reactor
    /// is declared.
    pub(crate) reactor_world: Option<cf_mission::ReactorWorld>,
    /// M4A: HUD banner queue. Latest entries are pushed to the back; FIFO
    /// drain caps the queue at `M4A_BANNER_BUFFER`. The HUD draws the highest
    /// `severity` (critical > warning > info) entries first per priority +
    /// raised_at_tick FIFO. Replay events are NOT re-derived from the queue;
    /// they live in `events.jsonl`.
    pub(crate) hud_banners: VecDeque<crate::state::HudBannerView>,
    /// captured (used as the diff base for the next delta).
    pub(crate) m4b_previous_snapshot: Option<serde_json::Value>,
    /// Stamped onto each subsequent `snapshot.delta_emitted` so the
    /// reconstructor can chain them back.
    pub(crate) m4b_last_baseline_event_id: Option<String>,
    pub(crate) m4b_last_baseline_tick: Option<u64>,
    /// M4A: captions queue (audio-bound events surfaced as text). Drains FIFO
    /// at `M4A_CAPTION_BUFFER`. The HUD draws the most recent N entries when
    /// `Settings.captions == true`.
    pub(crate) hud_captions: VecDeque<crate::state::CaptionView>,
    /// M4A: tool-validity tracker (last carve / last refusal). Updated per
    /// tick by the dig pipeline.
    pub(crate) hud_tool_validity: crate::state::ToolValidityView,
    /// M4A: previous tick's per-actor status, used to detect state changes
    /// that should raise a banner without scanning the full event log.
    pub(crate) hud_last_status: BTreeMap<ActorId, cf_actor::Status>,
    /// M4A: previous tick's mission result, used to detect mission_resolved
    /// transitions for banner emission.
    pub(crate) hud_last_mission_result: Option<String>,
    /// overlay holds input; the CONTROLS CAPTURED HUD zone renders and all
    /// `act.player.*` dispatches reject with reason `controls_captured`.
    pub(crate) controls_captured_by: Option<String>,
    /// `act.player.abort` succeeds. `record_run_finished` reads this to
    /// emit `system.run_finished.outcome="abort"` per M4 § Expected
    /// outcome + system events (previously hardcoded clean/panic only).
    pub(crate) run_aborted: bool,
    /// ticks so when a projectile hits N ticks after spawn, the
    /// `combat.projectile_hit` event can parent to its originating
    /// `combat.projectile_spawned` event (closing the cause chain back to
    /// `equipment.weapon_fired` -> `input.intent_received`). Entries are
    /// pruned when the projectile reaches `combat.projectile_hit` or
    /// `combat.projectile_expired` to keep the map bounded.
    pub(crate) projectile_spawn_event_ids: BTreeMap<u64, String>,
    /// `combat.projectile_spawned` time (from
    /// `cf_actor::sim::SpawnedProjectile::round_kind`) and read by the
    /// `emit_m14_penetration_ray` helper to route HEAT / APFSDS impacts
    /// to the M14C producers (`heat_impact_producer` /
    /// `apfsds_impact_producer`) rather than the M14 baseline traversal.
    /// Pruned alongside `projectile_spawn_event_ids` after the projectile
    /// is resolved.
    pub(crate) projectile_round_kinds: BTreeMap<u64, cf_equipment::RoundKind>,
    /// next ReactiveGuard tick treats the damaged actor as a perception
    /// trigger. No consumer at M1; M1.5 ai layer reads it.
    #[allow(dead_code)]
    pub(crate) force_ai_update_this_tick: bool,
    /// actor step. The current tick's AI loop consumes these so guard
    /// hearing reacts ≤1 tick after the player's `equipment.alarm_registered`
    /// fires. Cleared after each AI loop.
    pub(crate) pending_alarms: Vec<cf_ai::AlarmInput>,
    /// step; promoted to `pending_alarms` at end-of-tick so they're
    /// available to the next tick's AI loop. Two-stage so AI never reads
    /// half-collected alarms mid-tick.
    pub(crate) pending_alarms_staging: Vec<cf_ai::AlarmInput>,
    /// M4A: HUD focus state (DR-012 ACC-A-04). The cf-app keyboard layer +
    /// cfctl `act.input.focus` advance/retreat focus through the canonical
    /// `HUD_FOCUSABLE_NODES` list; observe.accessibility surfaces it.
    pub(crate) hud_focus_index: Option<usize>,
    pub(crate) hud_focus_cycle: u64,
    /// (de-duplicated; each threshold fires exactly once per
    /// `TIMER_WARNING_THRESHOLDS_S`). Cleared on scenario reset.
    pub(crate) m9_timer_warnings_emitted: BTreeMap<u32, bool>,
    /// concussion band machine. Applied by combat hits + explosions.
    /// Decay/recovery happens via `m9_tick_concussion_recovery`.
    pub(crate) m9_concussion_dose: BTreeMap<ActorId, f32>,
    /// exactly once per transition.
    pub(crate) m9_concussion_band: BTreeMap<ActorId, &'static str>,
    /// every dose application; ticks down to zero before recovery starts.
    pub(crate) m9_concussion_recovery_lockout_ticks: BTreeMap<ActorId, u32>,
    /// raise stage-change banners without scanning the event log).
    pub(crate) hud_last_chassis_stage: Option<cf_chassis::ChassisStage>,
    pub(crate) hud_last_pilot_state: Option<cf_chassis::PilotState>,
    /// Used as the `show_me_why_event_id` anchor on
    /// `mission.mission_resolved` when result=lost (DR-023 onboarding
    /// handoff — M3B viewer rewinds to this tick).
    pub(crate) last_player_input_event_id: Option<String>,
    /// `actor.actor_status_changed` event id for the player actor. Used as
    /// `parent_event_id` on `mission.mission_resolved` when the loss path
    /// is `PlayerDead` so M10's cause-chain walker can hop
    /// `mission_resolved → actor_status_changed(player DYING) → projectile_hit → ...`.
    /// None until the first player status_changed fires.
    pub(crate) last_player_status_event_id: Option<String>,
    /// layer. One of "off" | "integrity" | "pathability" | "mobility" |
    /// "hazard" | "build_repair". Default "off".
    pub(crate) material_overlay_mode: String,
    /// Surfaced via `observe.terrain.total_debris_spawned`.
    pub(crate) total_debris_spawned: u64,
    /// `chunked_terrain.carve_count` (which counts terrain-state carves —
    /// `total_carve_events` counts every emitted carve event including
    /// strip + chunked).
    pub(crate) total_carve_events: u64,
    /// per-tick hazard damage event to one per actor.
    pub(crate) hazard_last_contact_tick: BTreeMap<ActorId, u64>,
    /// event, used as parent for the first batch of `mission.objective_started`
    /// emissions per spec line 558 ("every event carries parent_event_id").
    pub(crate) mission_started_event_id: Option<String>,
    /// event id keyed by objective id. Used as parent for
    /// `mission.objective_updated`, `mission.objective_completed`,
    /// `mission.objective_failed` so the cause chain walks back to the
    /// origination event.
    pub(crate) mission_objective_started_event_ids: BTreeMap<String, String>,
    /// id, used as `parent_event_id` for snapshot re-emits at objective
    /// transitions (per spec literal "every event in {... snapshot_*} has
    /// parent_event_id"). Updated whenever any mission.* event fires.
    pub(crate) last_mission_event_id: Option<String>,
    /// event id. Used as parent for `ai.tactic_chosen` events emitted when
    /// no fresh perception_signal fired this tick.
    pub(crate) last_ai_state_changed_by_actor: BTreeMap<ActorId, String>,
    /// Used as a fallback root parent when no other cause exists (per spec
    /// "the cause chain ... walks back to an `input.intent_received` or
    /// `system.run_started` root").
    pub(crate) run_started_event_id: Option<String>,
    /// the engine only emits a `system.critical_drop` event for the delta
    /// (not the full cumulative total) each tick.
    pub(crate) last_reported_dropped_gameplay: u64,
    /// event id, used as `parent_event_id` on the subsequent
    /// `equipment.weapon_reload_completed` so M10 viewers can walk the
    /// reload chain cleanly. Entry is inserted on reload_started and removed
    /// on reload_completed (so a cancelled reload doesn't strand a stale id).
    pub(crate) reload_started_event_id_by_actor: BTreeMap<ActorId, String>,
    /// Carve events push their dirty rects + source event ids here; the engine
    /// flushes ONE `terrain.terrain_dirty_region_batch` per tick at end of
    /// `drive_tick` with the merged rect list + all contributing source ids.
    /// See `specs/active/M3.md` § Re-opened gaps.
    pub(crate) pending_dirty_rects: Vec<PendingDirtyRect>,
    /// used to trigger `terrain.forced_refresh_requested` after sustained
    /// load. Reset on any tick with `unupdated_areas == 0`.
    pub(crate) sustained_unupdated_ticks: u32,
    /// counter. Bumped every time `flush_pending_dirty_batch` produces a
    /// non-empty `out_rects[]`. Carried on `terrain.path_invalidated`
    /// events so M22+ pathfinder consumers can detect cache invalidation.
    pub(crate) path_invalidation_version: u64,
    /// was emitted). Surfaced via `summary.json.perf.terrain` at run close.
    pub(crate) perf_coalesce_samples: Vec<u32>,
    pub(crate) perf_coalesce_rects_in_total: u64,
    pub(crate) perf_coalesce_rects_out_total: u64,
    /// default — populated by scenarios that declare a friendly bot. See
    /// `cf_squad::Squad` for the canonical shape.
    pub(crate) squad: cf_squad::Squad,
    /// `act.player.weapon_swap` and ticks here until completion, when the
    /// engine emits `equipment.weapon_swap_completed` and removes the entry.
    pub(crate) weapon_swap_state: BTreeMap<ActorId, cf_equipment::WeaponSwap>,
    /// throttling. Stamina is only re-emitted when the value moves by more
    /// than `M6_STAMINA_EMIT_DELTA` to keep replay volume bounded.
    pub(crate) m6_last_stamina_emit: BTreeMap<ActorId, f32>,
    /// only re-emitted when the band (Hidden / Risky / Spotted) changes.
    pub(crate) m6_last_stealth_band: BTreeMap<ActorId, u8>,
    /// 1 = above). Toggling emits an `inventory.weight_changed` event.
    pub(crate) m6_last_weight_bucket: BTreeMap<ActorId, bool>,
    /// (`None` / `Light` / `Moderate` / `Heavy`). Transitions emit
    /// `inventory.encumbrance_threshold_crossed`.
    pub(crate) m6b_last_encumbrance_band: BTreeMap<ActorId, cf_equipment::EncumbranceBand>,
    /// emitted `perception.footstep_emitted`). Prevents replay spam.
    pub(crate) m6_footstep_cooldown: BTreeMap<ActorId, u32>,
    /// `act.player.throw_grenade`. The tick scheduler advances each one
    /// under gravity + collision and emits
    /// `equipment.grenade_detonated` at fuse=0.
    pub(crate) grenade_projectiles: Vec<GrenadeProjectile>,
    /// `act.player.knife_throw`. The tick scheduler advances each one
    /// under physics and emits `combat.knife_throw_landed` on collision.
    pub(crate) knife_projectiles: Vec<cf_equipment::KnifeProjectile>,
    /// engine to emit `actor.facing_changed` only on flips (not every tick).
    pub(crate) m6_last_facing: BTreeMap<ActorId, cf_actor::FacingDirection>,
    /// entry is (owner_id, position). Surfaced via observe.squad for the
    /// HUD; consumed by future M7 mission director when waypoints route
    /// AI.
    pub(crate) m6_beacons: Vec<(ActorId, cf_actor::Vec2)>,
    /// `act.player.drop_item`, consumed by `act.player.pickup`. Each item
    /// carries the actor that dropped it, the item id (rifle preset or
    /// material id), the position, and the slot the dropping inventory
    /// originally held it in.
    pub(crate) m6_dropped_items: Vec<DroppedItem>,
    pub(crate) m6_next_dropped_item_id: u64,
    /// release whose `charge_fraction < SNIPER_MISFIRE_BELOW` annotates the
    /// `equipment.weapon_fired` event with `misfire=true`. Drained each tick
    /// after the recorder reads it.
    pub(crate) m6_charge_misfires: BTreeMap<ActorId, ChargeFireInfo>,
    /// (Archetype + 5-layer ThinkingStack + auto-triage/auto-repair
    /// missions), faction registry, 4-phase mission director, reinforcement
    /// registry, mini-boss state. Co-resident with M2 `reactive_guards`:
    /// the M2 guard FSM still drives projectile / fire behavior; M7-A
    /// adds the reason-label + role-template surface on top.
    pub(crate) m7_ai_world: crate::m7_ai::M7AiWorld,
    /// `SquadState` (current verb + formation + role assignments +
    /// breach-chain progress + bounding-step state) + verb registry +
    /// doctrine-compatibility matrix. Lives on the squad NOT on the held
    /// actor so brain-hop preserves doctrine.
    pub(crate) m7b_squad: crate::m7b_squad::M7BSquadWorld,
    pub(crate) camera_state: cf_camera::CameraState,
    pub(crate) photo_mode: cf_photo::PhotoModeState,
    pub(crate) replay_scrub: cf_replay_scrub::ReplayScrubState,
    /// 1.5s slow-mo cinematic variant.
    pub(crate) killcam: cf_killcam::KillcamState,
    /// currently rendered).
    pub(crate) debug_state: cf_debug::DebugOverlayState,
    /// focused actor + open count).
    pub(crate) tactical_overlay: cf_squad_ui::TacticalOverlayState,
    pub(crate) plans: BTreeMap<ActorId, cf_squad_ui::Plan>,
    /// utility weight bonus.
    pub(crate) tag_state: cf_squad_ui::TagState,
    /// the player's own 8 actor actions (Pickup / Drop / SwitchWeapon /
    /// ThrowGrenade / MeleeBash / DeployBipod / SignalSquad / UseMedkit)
    /// with target context + 6 disabled-slice reason labels + sim
    /// slowdown gate (single-player 20%, multiplayer 100%).
    pub(crate) pie_menu: cf_squad_ui::PieMenuState,
    /// `content/localization/en.json` baseline. Re-loaded if `Settings.
    /// language` changes (only `en` ships at M8).
    pub(crate) localization: cf_localization::LocalizationTable,
    /// accumulator. Each [`M0Engine::drive_tick`] call adds the
    /// effective sim-speed percentage (0..=100) to this counter; the
    /// sim advances only when the counter reaches 100, then 100 is
    /// subtracted. Integer arithmetic so the spec's "all events
    /// deterministic (replay-compatible)" + "use tick counter modulo
    /// arithmetic, not floating-point time" requirements hold. The
    /// effective percentage is the most-restrictive of
    /// `settings.game_speed_assist.speed_pct()` and the pie menu's
    /// `slowdown_factor_pct` (per the round-3 fix description:
    /// "the pie menu can stack with game_speed_assist; whichever is
    /// more restrictive wins").
    pub(crate) game_speed_accumulator: u16,
    /// hosting a networked multiplayer session (M36+). `game_speed_
    /// assist` is single-player-only per spec, so the per-tick
    /// scheduler treats this flag as a kill-switch: multiplayer
    /// always runs at 100% regardless of the Settings value. M8
    /// ships with the flag pinned `false` (no multiplayer scenarios
    /// exist yet); the persistent setter is reserved for the M36+
    /// scenario loader.
    pub(crate) multiplayer_session: bool,
    /// `act.player.dig_trench_segment`, `act.player.place_trench_module`,
    /// and `act.player.drop_trench_template`; consumed by
    /// `compute_actor_cover_state` + `compute_trench_segment_at_pos`
    /// so the observe surfaces project real per-segment state instead
    /// of always-empty placeholders.
    pub(crate) trench_world: cf_trench::segment::InMemorySegments,
    /// Replay events reference segments via this id so the cause
    /// chain stays linear across dig → place_module → repair_module
    /// → breach → collapse.
    pub(crate) trench_next_segment_id: u64,
    /// variant. Used by `tick_m9b_cover_state_changes` to detect
    /// per-tick transitions and emit `trench.cover_state_changed`.
    /// The stored tuple is `(prev_cover_state, prev_segment_variant,
    /// prev_trench_stance)` so the engine can attribute the transition
    /// to either `segment_boundary` (segment changed) or
    /// `stance_change` (stance changed within the same segment).
    pub(crate) m9b_last_cover_state: BTreeMap<
        ActorId,
        (
            cf_trench::CoverState,
            Option<cf_trench::SegmentVariant>,
            cf_trench::TrenchStance,
        ),
    >,
    /// `AI-TRENCH-A-01` doctrine. Increments while the actor remains
    /// in `CoverState::Exposed`; resets on any other cover state. The
    /// doctrine reads this to enforce the spec's "no AI remains
    /// Exposed continuously > 1.5 seconds" invariant.
    pub(crate) m9b_trench_doctrine_exposure_ticks: BTreeMap<ActorId, u32>,
    /// through the trench doctrine each tick. Currently populated by
    /// the scenario loader when a reactive_guard entry carries
    /// `doctrine: Some("AI-TRENCH-A-01")` in its scenario RON; the
    /// scenario `m9b_ai_in_trench_doctrine` opts in its three
    /// defenders.
    pub(crate) m9b_trench_doctrine_actors: std::collections::BTreeSet<ActorId>,
    /// cinematic is playing (opening / between-mission / ending);
    /// `None` when the gameplay camera + input are in normal control.
    /// cfctl `act.player.skip_cinematic`, `act.player.pause_cinematic`,
    /// `act.player.replay_cinematic`, and `srv.dump_cinematic_state`
    /// operate on this slot.
    pub(crate) cinematic_kernel: Option<cf_cinematic::CinematicKernel>,
    /// watched (or skipped past the 3-second confirm window). Lives
    /// here at M12C; M41 save format will persist it to `save.cinematic_seen_set`.
    pub(crate) cinematic_seen_set: cf_cinematic::SeenSet,
    /// when a cinematic kernel boots; releases at `cinematic.ended`.
    pub(crate) cinematic_mixer: cf_audio::CinematicMixer,
    /// (mirrors the cinematic kernel's composed offset). cf-app's
    /// bridge polls this via `engine.cinematic_takeover_snapshot()`.
    pub(crate) cinematic_takeover: cf_cinematic::CinematicTakeoverSnapshot,
    /// spec § Between-mission cinematic. Drained per between-mission
    /// engage; the M25 hook will fold real rival-alive state into this
    /// roll when it ships.
    pub(crate) cinematic_rival_taunt_roll: u8,
    /// manifest's `gravity_overrides[]` array; consumed by per-tick
    /// `cf_physics::apply_overrides` calls.
    pub(crate) m14b_gravity_overrides: Vec<cf_physics::GravityOverride>,
    pub(crate) m14b_wind_sources: Vec<cf_atmos::WindSource>,
    /// the wind force kernel + observe.frame.cells projection.
    pub(crate) m14b_atmos_cells: Vec<cf_atmos::AtmosCell>,
    /// every 4th tick per the spec.
    pub(crate) m14b_strat_cells: Vec<cf_atmos::StratCell>,
    /// `scripted_steps` array. `drive_tick` injects matching intents into
    /// `pending_intent` before the actor sim runs so headless cfctl drives
    /// of `m14c_heat_vs_era.ron` / `m14c_apfsds_vs_heavy.ron` actually fire
    /// the HEAT / APFSDS round at a deterministic tick (rather than no-op).
    pub(crate) m14c_scripted_steps: Vec<crate::scenario::ScenarioScriptStep>,
    /// `cf_physics::run_projectile_pair_pass` between the actor-collision
    /// pass and the terrain pass. Authored at scenario load + advanced
    /// each tick. Empty by default (pre-M14D scenarios behave identically).
    pub(crate) m14d_projectile_pair_pool: Vec<cf_physics::ProjectileSnapshot>,
    /// counter. Incremented once per call to
    /// [`M0Engine::tick_m14d_projectile_pair`]. Exposed via the
    /// schedule-trace accessor for the `pass_called_once_per_tick` test.
    pub(crate) m14d_pair_pass_invocations: u64,
    /// pass timing + candidate counts surfaced to perf tests.
    pub(crate) m14d_last_pair_pass_trace: cf_physics::ProjectilePairPassTrace,
    /// Default false — killcam excludes `collision.projectile_pair_contact`
    /// events unless the player opts in.
    pub(crate) m14d_replay_intercepts: bool,
    /// laser `owner_actor_id`. Engaged on every
    /// `collision.projectile_pair_contact{outcome="aps_intercept"}`
    /// event and decayed by [`cf_equipment::Cram::tick`] each
    /// projectile-pair pass. Empty by default; an entry materialises
    /// the first time a given owner fires an intercept.
    pub(crate) m14d_cram_cooldowns: BTreeMap<u64, cf_equipment::Cram>,
    /// each pass entry ("actor_collision_start", "projectile_pair_start",
    /// "terrain_start", ...) for the most recent N ticks so the engine
    /// integration test can assert ordering. Capped at 120 entries.
    pub(crate) m14d_schedule_trace: std::collections::VecDeque<&'static str>,
    /// per-tick step to emit `gravity.override_activated` only on entry +
    /// `gravity.override_deactivated` only on exit. The inner BTreeSet
    /// holds the override ids currently active for the actor; the outer
    /// map is keyed by actor id.
    pub(crate) m14b_active_overrides: BTreeMap<ActorId, std::collections::BTreeSet<u32>>,
    /// transient apertures (pipe ruptures) spawned via
    /// [`Self::inject_pipe_rupture`]. Each tick the value decrements; on
    /// reaching zero the WindSource + its synthetic atmosphere cells are
    /// removed.
    pub(crate) m14b_transient_wind_ttl: BTreeMap<u32, u32>,
    /// transient wind sources (pipe ruptures). Used to clean up the
    /// atmosphere cell list when the parent WindSource expires.
    pub(crate) m14b_transient_cells: Vec<u32>,
    /// scenario manifest's `m14e_tunnel_spans[]` array. Indexed by chunk
    /// coordinate; each entry tracks the current integrity field + the
    /// cached span_px + anchored flag the per-tick pass consumes.
    pub(crate) m14e_chunks: BTreeMap<(i32, i32), M14eChunkState>,
    /// once per N-tick boundary). Exposed via the schedule-trace
    /// accessor for the `compute_integrity_pass_runs_every_15_ticks`
    /// VAL-M14E-019 test.
    pub(crate) m14e_pass_invocations: u64,
    /// from `scenario.seed + m14e_cave_in_seed_offset`; advances on every
    /// cave-in roll regardless of outcome so the draw sequence is stable
    /// across same-seed runs.
    pub(crate) m14e_rng_state: u64,
    /// debris impulse routes through `cf_physics::cave_in_fall_impulse_chain`
    /// and forces the actor into KnockedDown.
    pub(crate) m14e_actor_knockdown: BTreeMap<u64, bool>,
    /// `terrain.cave_in_triggered`. Drives the 15-tick cascade window
    /// per VAL-M14E-018.
    pub(crate) m14e_last_cave_in_tick: BTreeMap<(i32, i32), u64>,
    /// events emitted (used for replay summary + cross-tick assertions).
    pub(crate) m14e_total_cave_ins: u32,
    pub(crate) m14e_total_beams_placed: u32,
    pub(crate) m14e_total_beams_destroyed: u32,
    /// falling-debris cones the per-tick collapse-check pass produces.
    /// `cf-app` drains this every frame; headless runs let it grow up to a
    /// soft cap (see `drain_*`).
    pub(crate) m14e_tunnel_collapse_queue: cf_render_2d::tunnel_collapse::TunnelCollapseQueue,
    /// (the engine surfaces these via `emit_audio_cue` already; we still
    /// keep a counter for cross-tick test assertions).
    pub(crate) m14e_tunnel_creak_count: u32,
    pub(crate) m14e_cave_in_thunder_count: u32,
    /// Used by the support-beam placer's inventory-debit path so VAL-M14E-009
    /// can assert the post-placement delta.
    pub(crate) m14e_actor_resources: BTreeMap<u64, BTreeMap<String, i64>>,
    /// "VIBRATION ACCUMULATING" HUD banner per VAL-M14E-015).
    pub(crate) m14e_plasma_cutter_active: BTreeMap<u64, bool>,
    /// coord. Authored by `m14f_lateral_wall_spans[]` in the scenario
    /// manifest. The per-chunk `IntegrityField` is borrowed from the
    /// shared `m14e_chunks` map per VAL-CROSS-005 — this map only
    /// tracks the lateral-axis metadata + per-tier emission flags.
    pub(crate) m14f_lateral_chunks: BTreeMap<(i32, i32), M14fLateralChunkState>,
    /// count. Equal to `floor(T / 15)` after T ticks.
    pub(crate) m14f_lateral_pass_invocations: u64,
    /// pass. Equal to the number of engine ticks since boot (one
    /// invocation per tick).
    pub(crate) m14g_wound_aging_invocations: u64,
    /// baked defaults on first use so cf-control does not need to read
    /// content files at engine boot.
    pub(crate) m14g_wound_registry: Option<cf_wound::WoundSpecRegistry>,
    /// by the scenario manifest. The engine ticks the dwell counter
    /// per `(actor_id, zone)` every tick.
    pub(crate) m14g_thermal_zones: Vec<crate::scenario::ScenarioThermalZone>,
    /// thermal pass.
    pub(crate) m14g_thermal_dwell_ticks: BTreeMap<(u64, String), u64>,
    /// per `(actor_id, zone)` so the producer fires escalation events
    /// only when the degree actually changes.
    pub(crate) m14g_thermal_emitted_kind: BTreeMap<(u64, String), cf_wound::WoundKind>,
    /// the scenario manifest. Each entry fires one `wound.created`
    /// event on its `fire_tick`.
    pub(crate) m14g_material_contacts: Vec<crate::scenario::ScenarioMaterialContact>,
    /// so the engine never emits the same contact twice.
    pub(crate) m14g_material_contacts_fired: std::collections::BTreeSet<usize>,
    /// per-actor `LongTermState` for downstream M41 consumers (roster
    /// UI, narrative tab).
    pub(crate) m14i_veteran_roster: cf_veteran::VeteranRoster,
    /// when `act.player.retire_veteran` fires. M48 storyteller consumes
    /// the canonical `narrative.veteran_retired` event ids registered
    /// here.
    pub(crate) m14i_retirement_narratives:
        cf_storyteller::retirement_event::RetirementNarrativeRegistry,
    /// which the actor was first registered as submerged / damp after
    /// a dam rupture.
    pub(crate) m14f_actor_submerged_tick: BTreeMap<u64, u64>,
    /// at which the actor was first registered as exposed to vacuum
    /// after a sealed-room rupture.
    pub(crate) m14f_actor_vacuum_tick: BTreeMap<u64, u64>,
    /// through the breach bbox per dam chunk. Increments each tick
    /// after rupture until the volume depletes.
    pub(crate) m14f_breach_fluid_mass: BTreeMap<(i32, i32), u64>,
    /// side) per sealed-room chunk. Updated each tick after rupture
    /// so the delta monotonically decreases toward equilibrium.
    pub(crate) m14f_breach_pressure_kpa: BTreeMap<(i32, i32), (f32, f32)>,
    /// cable in the scene, keyed by [`cf_physics::RopeId`]. The engine
    /// advances each rope each tick via `cf_physics::rope::Rope::step`.
    pub(crate) m14j_ropes: BTreeMap<cf_physics::RopeId, cf_physics::Rope>,
    /// zip-kit deploy.
    pub(crate) m14j_next_rope_id: u64,
    /// deployed zip-line (not a grapple rope). The slide engine consults
    /// this to apply gravity-along-cable + brake-deceleration.
    pub(crate) m14j_zipline_ropes: std::collections::BTreeSet<cf_physics::RopeId>,
    /// along the cable (m/s, positive = toward low end).
    pub(crate) m14j_zipline_speed_by_rider: BTreeMap<u64, f32>,
    /// the M15+M15B chemistry. When `chunked_terrain` is present, the
    /// engine calls `cf_material::kernel_step` each tick to drive
    /// per-pixel reactions, phase transitions, and CA movement. State
    /// lives here so the Margolus stepper parity persists across ticks.
    pub(crate) material_kernel: cf_material::MaterialKernel,
    /// `content/materials/reaction_registry.json` at engine init, or
    /// falls back to the hardcoded `default_reaction_registry` when
    /// the file isn't present.
    pub(crate) reaction_registry: cf_material::ReactionRegistry,
    /// `content/materials/phase_registry.json` at engine init.
    pub(crate) phase_registry: cf_material::PhaseRegistry,
    /// (293.15 K Earth baseline) until M19 atmospherics wires per-cell
    /// thermal sources. The kernel uses this for reaction temperature
    /// gating + phase-transition threshold crossing.
    pub(crate) heat_field: cf_terrain::HeatField,
    /// crossings for phase transitions. `None` on the first tick.
    pub(crate) prev_heat_field: Option<cf_terrain::HeatField>,
    /// saturation, fires nucleation + precipitation events as steam
    /// pixels climb above the altitude/temperature gates.
    pub(crate) precipitation_cycle: cf_material::PrecipitationCycle,
    /// `content/materials/precipitation_config.json` at engine init.
    pub(crate) precipitation_config: cf_material::PrecipitationConfig,
}

/// `EngineState.m14f_lateral_chunks` keyed by chunk coord. Tracks the
/// per-tier emission edges + the topology hook (mineshaft / dam /
/// sealed_room) the lateral pass consumes on rupture.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct M14fLateralChunkState {
    #[allow(dead_code)]
    pub span_id: String,
    pub bbox_min: [i64; 2],
    pub bbox_max: [i64; 2],
    pub unsupported_span_px: u32,
    pub wall_thickness_px: u32,
    pub lateral_yield_strength: u16,
    pub vibration_modifier: f32,
    pub cascade_neighbors: Vec<(i32, i32)>,
    pub downstream_actor_id: Option<u64>,
    pub topology: String,
    pub sealed_room_pressure_kpa: f32,
    /// Per VAL-M14F-002: deterministic bulging countdown — fires after
    /// `bulging_countdown_ticks` engine ticks have elapsed once the
    /// unsupported span exceeds the floor. Set at scenario init so
    /// 24-px spans fire bulging within 30 ticks reliably.
    pub bulging_countdown_remaining: Option<u32>,
    pub bulging_emitted: bool,
    pub bulging_at_tick: Option<u64>,
    pub crack_advanced_emitted: bool,
    pub crack_advanced_at_tick: Option<u64>,
    pub rupture_emitted: bool,
    pub rupture_at_tick: Option<u64>,
    pub pixel_carved: bool,
    /// **VAL-CROSS-024**: composite-cascade opt-in from the scenario
    /// manifest. When `true`, a `terrain.wall_rupture` on this chunk
    /// also cascades M14E cave-in on every `cascade_neighbors` chunk
    /// whose state is owned by an M14E tunnel span (i.e.,
    /// `m14f_owns_rupture_emit == false`). The default `false` keeps
    /// standalone mineshafts / dams / sealed-rooms isolated from the
    /// M14E ceiling cave-in surface.
    pub m14e_composite_cascade_allowed: bool,
}

/// `EngineState.m14e_chunks` keyed by chunk coord.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct M14eChunkState {
    pub field: cf_terrain::IntegrityField,
    #[allow(dead_code)]
    pub span_id: String,
    pub bbox_min: [i64; 2],
    pub bbox_max: [i64; 2],
    pub unsupported_span_px: u32,
    pub ceiling_thickness_px: u32,
    pub vibration_modifier: f32,
    pub anchored: bool,
    pub cascade_neighbors: Vec<(i32, i32)>,
    pub damage_actor_id: Option<u64>,
    pub structural_integrity_low_emitted: bool,
    pub cave_in_emitted: bool,
    /// **VAL-CROSS-024**: explicit per-chunk flag — `true` means an
    /// M14F lateral wall span owns the rupture emit on this chunk, so
    /// the M14E ceiling cave-in roll is suppressed. Distinct from
    /// `cave_in_emitted` (which is the one-shot M14E rupture latch).
    /// Setting this flag explicitly at scenario init (rather than
    /// pre-asserting `cave_in_emitted = true` as the prior code did)
    /// lets the composite dam-above-tunnel topology express both an
    /// M14F lateral rupture AND an M14E ceiling cave-in cascade on
    /// distinct chunks without aliasing the two suppression semantics.
    pub m14f_owns_rupture_emit: bool,
    /// **VAL-CROSS-024**: when set by the M14F lateral-pass cascade
    /// (wall_rupture → tunnel cave-in), the next M14E cave-in roll on
    /// this chunk fires deterministically — skipping the
    /// [`cf_terrain::cave_in_roll`] RNG check — because the cascade
    /// from the M14F rupture is itself deterministic. Cleared once
    /// the resulting cave-in fires.
    pub cave_in_pending_cascade: bool,
    pub l1_at_tick: Option<u64>,
    pub l2_at_tick: Option<u64>,
    pub l3_at_tick: Option<u64>,
    /// Per VAL-M14E-013 cadence fidelity: when set to `Some(deadline_tick)`,
    /// the per-tick collapse-check pass MUST recompute integrity on this
    /// chunk no later than `deadline_tick` (typically demolish_tick + 5).
    /// Honored even when the cadence guard would otherwise skip the chunk.
    pub force_integrity_pass_deadline: Option<u64>,
    /// Tracks which crack levels (L1/L2/L3) have already been enqueued as
    /// render decals so duplicate primitives aren't pushed across passes.
    pub crack_decal_l1_enqueued: bool,
    pub crack_decal_l2_enqueued: bool,
    pub crack_decal_l3_enqueued: bool,
    /// Caching of the most-recent emission's HUD banner so a second
    /// emit-pass doesn't re-stack the banner. Reset on demolish so a
    /// follow-up cascade can re-fire the banner.
    pub structural_warning_banner_emitted: bool,
}

/// hook to `emit_actor_events`. `charge_fraction` is the latched accumulator
/// at trigger release; `misfire` is true iff the fraction was below
/// [`cf_equipment::SNIPER_MISFIRE_BELOW`].
#[derive(Debug, Clone, Copy)]
pub(crate) struct ChargeFireInfo {
    pub charge_fraction: f32,
    pub misfire: bool,
}

/// `act.player.drop_item`; despawned by `act.player.pickup`.
#[derive(Debug, Clone)]
pub(crate) struct DroppedItem {
    pub id: u64,
    pub item_id: String,
    pub position: cf_actor::Vec2,
    pub weight_kg: f32,
    #[allow(dead_code)]
    pub dropped_by: ActorId,
    #[allow(dead_code)]
    pub original_slot: u8,
}

/// `grenade_projectiles` vector and advanced each tick under gravity +
/// collision. On fuse=0 the engine emits `equipment.grenade_detonated`
/// and applies type-specific effects (Frag radius damage, Smoke hazard
/// tile, Flash afflictions, Stick adhesive).
#[derive(Debug, Clone)]
pub(crate) struct GrenadeProjectile {
    pub id: u64,
    pub owner: ActorId,
    pub kind: cf_equipment::GrenadeKind,
    pub position: cf_actor::Vec2,
    pub velocity: cf_actor::Vec2,
    pub fuse_remaining: f32,
    pub radius: f32,
    pub damage_at_center: f32,
    pub adhesive: bool,
    pub spawns_hazard: bool,
    pub vision_disrupt: bool,
    pub stuck: bool,
}

/// emission phase so the engine can spawn a thrown grenade after the
/// write-guard is released.
#[derive(Debug, Clone)]
pub(crate) struct PendingGrenadeSpawn {
    pub(crate) owner: ActorId,
    pub(crate) kind: cf_equipment::GrenadeKind,
    pub(crate) origin: cf_actor::Vec2,
    pub(crate) velocity: cf_actor::Vec2,
    pub(crate) fuse_remaining: f32,
    pub(crate) radius: f32,
    pub(crate) damage_at_center: f32,
    pub(crate) adhesive: bool,
    pub(crate) spawns_hazard: bool,
    pub(crate) vision_disrupt: bool,
}

/// from the cfctl dispatch site so the engine can scan for hit actors +
/// roll knockdown + emit the hit event in a separate, post-dispatch phase.
#[derive(Debug, Clone)]
pub(crate) struct PendingMeleeResolve {
    pub(crate) attacker: ActorId,
    pub(crate) kind: cf_equipment::MeleeKind,
    pub(crate) facing_sign: f32,
    pub(crate) actor_position: cf_actor::Vec2,
}

/// from the cfctl dispatch site so the engine can spawn the
/// [`cf_equipment::KnifeProjectile`] after releasing the write-guard.
#[derive(Debug, Clone)]
pub(crate) struct PendingKnifeSpawn {
    pub(crate) owner: ActorId,
    pub(crate) origin: cf_actor::Vec2,
    pub(crate) aim: cf_actor::Vec2,
    pub(crate) base_damage: f32,
}

/// attempt from the cfctl dispatch site so the engine can find the target
/// (behind + within reach) + apply instant-kill damage + emit
/// `combat.stealth_kill_executed` after releasing the write-guard.
#[derive(Debug, Clone)]
pub(crate) struct StealthKillAttempt {
    pub(crate) attacker: ActorId,
    pub(crate) attacker_pos: cf_actor::Vec2,
    pub(crate) attacker_facing_x: f32,
}

/// released. The dispatcher captures the tool kind + origin/aim; the
/// post-dispatch resolver applies the side-effect (terrain carve/fill, reveal
/// hostile actors, drop beacon, etc).
#[derive(Debug, Clone, Copy)]
pub(crate) enum ToolEffectKind {
    Digger,
    Repair,
    Foam,
    Concrete,
    Welder,
    Drill,
    MultiTool,
    Beacon,
    SensorPulse,
}

#[derive(Debug, Clone)]
pub(crate) struct ToolEffect {
    pub(crate) kind: ToolEffectKind,
    pub(crate) origin: cf_actor::Vec2,
    pub(crate) aim: cf_actor::Vec2,
    pub(crate) actor_id: ActorId,
}

/// post-dispatch phase. Distinct from the dispatch's
/// [`PendingMeleeResolve`] so the resolver can do the actor scan + the
/// emitter can do only the recorder write.
#[derive(Debug, Clone)]
pub(crate) struct MeleeHitEmit {
    pub(crate) attacker: u64,
    pub(crate) target: u64,
    pub(crate) kind: cf_equipment::MeleeKind,
    pub(crate) damage: f32,
    pub(crate) hp_before: f32,
    pub(crate) hp_after: f32,
    pub(crate) knockdown_chance: f32,
    pub(crate) knockdown_rolled: f32,
    pub(crate) knockdown_triggered: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingDig {
    pub(crate) target: Option<String>,
    pub(crate) source: IntentSource,
}

/// during the tick. The engine flushes a coalesced batch at end-of-tick.
/// See `specs/active/M3.md` § Re-opened gaps, scenarios 2-4.
#[derive(Debug, Clone)]
pub(crate) struct PendingDirtyRect {
    pub source_event_id: String,
    pub cx: i32,
    pub cy: i32,
    pub min: [i64; 2],
    pub max: [i64; 2],
}

/// pass. Holds the merged AABB after union operations.
#[derive(Debug, Clone)]
pub(crate) struct MergedDirtyRect {
    pub(crate) cx: i32,
    pub(crate) cy: i32,
    pub(crate) min: [i64; 2],
    pub(crate) max: [i64; 2],
}

/// edge or interior. Used by the greedy coalesce pass. Adjacent chunks
/// (e.g. (0,0) at [0,0..256] + (1,0) at [256,0..512]) satisfy
/// `a.max[0] == b.min[0]` so the inclusive `>=`/`<=` comparison captures
/// shared-edge unions.
pub(crate) fn rects_touch_or_overlap(a_min: [i64; 2], a_max: [i64; 2], b_min: [i64; 2], b_max: [i64; 2]) -> bool {
    let x_overlap = a_min[0] <= b_max[0] && b_min[0] <= a_max[0];
    let y_overlap = a_min[1] <= b_max[1] && b_min[1] <= a_max[1];
    x_overlap && y_overlap
}

/// placement identified by `instance_id`. Depth is **1-indexed** to match
/// the spec § Acceptance criteria scenario ("chest (level 1) containing a
/// crate (level 2)") and `cf_actor::inventory::find_container_depth`:
///
/// - top-level item in `grid.items` = depth 1
/// - item nested in a top-level container = depth 2
/// - item nested two levels deep = depth 3 (only valid for non-container
///   children; container children are gated by `try_nest_depth`)
///
/// Returns 0 when the id is not found.
pub(crate) fn container_depth_of(grid: &cf_actor::InventoryGrid, instance_id: u64) -> u8 {
pub(crate)     fn walk(items: &[cf_actor::PlacedItem], target_id: u64, depth: u8) -> Option<u8> {
        for item in items {
            if item.instance_id == target_id {
                return Some(depth);
            }
            if let Some(inner) = &item.container {
                if let Some(d) = walk(&inner.items, target_id, depth.saturating_add(1)) {
                    return Some(d);
                }
            }
        }
        None
    }
    walk(&grid.items, instance_id, 1).unwrap_or(0)
}

pub(crate) fn observed_run_status(state: &EngineMutable) -> RunStatus {
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
            controls_captured_by: s.controls_captured_by.clone(),
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
                "scenario_id": self.config.scenario_id,
                "seed": self.config.seed,
                "tick_rate_hz": self.config.tick_rate_hz,
                "run_mode": self.config.run_mode,
                "control_api": self.config.control_api_enabled,
                "protocol_version": crate::SCHEMA_VERSION,
                // manifest_hash + build_id**. The manifest hash mirrors
                // `run_manifest.json.config_hash` (blake3 of effective
                // settings + scenario id) so an offline reviewer can pin
                // the bundle to its config without parsing the manifest.
                // `build_id` is the cf-app git commit short hash from
                // `cf_replay::BuildInfo.commit_sha`.
                "manifest_hash": self.config.config_hash,
                "build_id": self.config.commit_sha,
                "settings": settings_value,
            }),
            None,
        );
        // have no other cause (e.g. ai.tactic_chosen with no fresh
        // perception signal) can chain to it as a root.
        if let Ok(mut s) = self.state.write() {
            s.run_started_event_id = Some(started_id.clone());
        }
        self.emit_initial_snapshots(tick, sim_time_ms, Some(&started_id));
        self.emit_category_baseline(tick, sim_time_ms, &started_id);
        // baseline as part of run_started so the cadence is anchored from
        // the very first tick. drive_tick() advances starts at tick 1, so
        // tick 0 itself never goes through emit_m4b_snapshot_for_tick.
        self.emit_m4b_snapshot_for_tick(tick);
        // start so the baseline's `first_event_type` for the ux category
        // is reachable. Banners are cosmetic per the determinism-island
        // contract so this is flagged cosmetic.
        self.recorder.record_cosmetic(
            tick,
            sim_time_ms,
            "ux",
            "banner_raised",
            json!({
                "banner_id": "run_started_banner",
                "scenario_id": self.config.scenario_id,
                "severity": "info",
                "message": format!("scenario {} started", self.config.scenario_id),
            }),
            Some(started_id.clone()),
        );
        // M1 Seam S4: pre-emit `mission.mission_started` whenever a
        // MissionState is attached. M1.5 will populate richer payloads;
        // M1 emits a thin no-op so the M3B viewer + replay verifier can
        // expect the event type without conditionally rendering on
        // milestone.
        let has_mission = self.state.read().ok().map(|s| s.mission.is_some()).unwrap_or(false);
        if has_mission {
            let scenario_id = self.config.scenario_id.clone();
            // M2 re-audit (2026-05-13): lifecycle Loaded → InProgress.
            if let Ok(mut state) = self.state.write() {
                if let Some(mission) = state.mission.as_mut() {
                    mission.lifecycle = cf_mission::MissionLifecycle::InProgress;
                    if mission.id.is_empty() {
                        mission.id = scenario_id.clone();
                    }
                }
            }
            {
                // M2 re-audit pass 3 (2026-05-13): per spec line "mission.mission_started
                // fires once with mission_id, seed, scenario_id". Previous payload was
                // missing mission_id + seed. mission_id mirrors scenario_id at M2
                // (one mission per scenario load; M13+ adds explicit mission_id when
                // multiple missions per scenario exist).
                let mission_started_id = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "mission",
                    "mission_started",
                    json!({
                        "mission_id": scenario_id,
                        "scenario_id": scenario_id,
                        "scenario": scenario_id,
                        "seed": self.config.seed,
                        "tick": tick.0,
                    }),
                    Some(started_id.clone()),
                );
                // M2 re-audit (2026-05-13): stash the mission_started event
                // id so subsequent objective_started events can chain to it.
                // M4: also track as last_mission_event_id for snapshot
                // re-emit parent.
                if let Ok(mut s) = self.state.write() {
                    s.mission_started_event_id = Some(mission_started_id.clone());
                    s.last_mission_event_id = Some(mission_started_id);
                }
            }
        }
        // M1 Seam S2: when the scenario manifest sets tutorial_safety=true,
        // mark the controllable actor INACTIVE so the engine refuses lethal
        // damage transitions until the tutorial controller (M1.5+) flips it.
        if self.config.tutorial_safety {
            if let Ok(mut state) = self.state.write() {
                if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(player) = sim.world.player_actor_mut() {
                        let _ = player.set_inactive(true);
                    }
                }
            }
        }
        self.spawn_debug_panic_if_requested();
    }
}

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

