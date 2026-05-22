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
    /// **M1 / Seam S2**: scenario tutorial_safety flag. When true the
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
    /// **M6**: initial squad roster built from `ScenarioActor.squad_role`.
    /// Each entry pairs (actor_id, SquadRole, display_name). Empty for
    /// scenarios with no squad declarations.
    pub initial_squad_members: Vec<InitialSquadMember>,
    /// M3A: configurable checksum cadence. 0 = disabled. Default from
    /// `ChecksumConfig::m0_default().cadence_ticks` (60).
    pub checksum_cadence_ticks: u64,
    /// **M1.5 G6**: optional difficulty preset id from the scenario
    /// manifest. Applied to each reactive guard at engine construction.
    /// `None` keeps each guard's params as authored.
    pub difficulty_preset: Option<String>,
    /// **M4 § Expected outcome contract**: when `Some(...)`, overrides the
    /// inferred default in `build_manifest`. `None` lets the engine derive
    /// the outcome (Panic if `debug_inject_panic_at_tick` is set, else
    /// Clean).
    pub expected_outcome_override: Option<cf_replay::ExpectedOutcome>,
    /// **M7 director v0.5 (audit gap A12)**: optional 4-phase pacing
    /// state seeded from the scenario manifest. `None` opts out — the
    /// engine still ticks `advance_phase` but it returns None until
    /// `init_phase` is called.
    pub initial_phase_state: Option<cf_mission::PhaseState>,
    /// **M7 director v0.5 (audit gap A15)**: opt-in reinforcement wave
    /// declarations the engine flattens into `M7AiWorld.reinforcements`.
    pub initial_reinforcement_waves: Vec<cf_mission::ReinforcementWave>,
    /// **M7 director v0.5 (audit gap A16)**: optional mini-boss state
    /// seeded into `M7AiWorld.boss`. `None` opts out — `apply_boss_damage`
    /// returns None and no `boss.*` events fire.
    pub initial_boss_state: Option<cf_mission::BossState>,
    /// **M7 director v0.5 (audit gap A13/A14)**: optional v0.5 objective
    /// graph seeded into `M7AiWorld.objective_graph`. `None` opts out —
    /// the M2 single-vec objective list keeps working unchanged.
    pub initial_objective_graph: Option<cf_mission::ObjectiveGraph>,
    /// **M4B § "Delta baseline cadence is enforced"** — ticks between
    /// `snapshot.baseline_emitted` events. Default 600 (10 s @ 60 Hz);
    /// 0 disables snapshot emission entirely.
    pub delta_baseline_cadence_ticks: u64,
    /// **M4B § "Tamper-evident competitive replays"** — when true, the
    /// recorder runs in chain mode (per-event BLAKE3 keyed hash + final
    /// anchor in `RunManifest.ledger_chain_anchor`). Default false.
    pub ledger_chain_enabled: bool,
    /// **M14B** § gravity field overrides authored by the scenario manifest.
    /// Empty by default; producer-side `cf_physics::apply_overrides` reads
    /// this list each tick.
    pub initial_gravity_overrides: Vec<cf_physics::GravityOverride>,
    /// **M14B** § wind apertures authored by the scenario manifest.
    pub initial_wind_sources: Vec<cf_atmos::WindSource>,
    /// **M14B** § authored atmosphere cells (pressure + temperature). Drives
    /// the wind force kernel + stratification.
    pub initial_atmosphere_cells: Vec<cf_atmos::AtmosCell>,
    /// **M14B** § per-column gas composition for stratification (parallel
    /// to `initial_atmosphere_cells` by `cell_id`). Empty = pure-air default.
    pub initial_stratification_cells: Vec<cf_atmos::StratCell>,
    /// **M14C** § scripted director steps from the scenario manifest.
    /// `drive_tick` reads this each tick and injects matching intents into
    /// `pending_intent` before the actor sim runs. Empty by default.
    pub initial_scripted_steps: Vec<crate::scenario::ScenarioScriptStep>,
    /// **M14D** § initial projectile-pair pool authored by the scenario
    /// manifest's `m14d_projectile_pool[]` field. Drives the per-tick
    /// projectile-pair CCD pass (`cf_physics::run_projectile_pair_pass`).
    /// Empty by default — pre-M14D scenarios behave identically.
    pub initial_m14d_projectile_pool: Vec<cf_physics::ProjectileSnapshot>,
    /// **M14D § VAL-M14D-019** initial per-player `replay_intercepts`
    /// setting. Default false — killcam excludes
    /// `collision.projectile_pair_contact` events unless the player
    /// opts in via this setting.
    pub initial_replay_intercepts: bool,
    /// **M14E** § initial tunnel-span fixtures from the scenario manifest.
    /// Empty by default; M14E scenarios populate one or more spans for
    /// the per-tick collapse-check pass.
    pub initial_m14e_tunnel_spans: Vec<crate::scenario::ScenarioTunnelSpan>,
    /// **M14E** § seed offset for the cave-in roll RNG. Added to the
    /// engine's `seed` to derive the cave-in RNG state.
    pub initial_m14e_cave_in_seed_offset: u64,
    /// **M14F § VAL-M14F-016**: initial lateral-wall fixtures from the
    /// scenario manifest. Empty by default; M14F scenarios populate
    /// one or more rows so the lateral integrity pass + bulging →
    /// crack_advanced → rupture cascade fires against a known sidewall
    /// topology. Shares the same per-chunk `IntegrityField` buffer as
    /// the ceiling pass per VAL-CROSS-005.
    pub initial_m14f_lateral_wall_spans: Vec<crate::scenario::LateralWallSpan>,
    /// **M14G § VAL-M14G-013/014/030**: initial thermal-contact zones
    /// from the scenario manifest. The engine ticks each zone's dwell
    /// counter every tick and runs
    /// [`cf_environment::classify_tile_thermal`] to emit typed burn /
    /// frostbite wounds.
    pub initial_m14g_thermal_zones: Vec<crate::scenario::ScenarioThermalZone>,
    /// **M14G § VAL-M14G-029**: initial material-contact entries from
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
    /// **M9B audit GAP-2**: optional doctrine id that opts the actor
    /// into a per-tick AI cover-decision pipeline. Currently only
    /// `"AI-TRENCH-A-01"` is recognised (the M9B trench garrison
    /// doctrine); unknown values are ignored.
    #[allow(dead_code)]
    pub doctrine: Option<String>,
}

/// **M6**: initial squad member built from a scenario actor entry.
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

/// **M14B** § per-actor snapshot consumed by `tick_m14b` to compute
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



/// **M15B § AmbientWorld inference** — derive the precipitation cycle's
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
            // **M4B § "Delta baseline cadence is enforced"** — default 600
            // ticks (10 s @ 60 Hz) per spec.
            delta_baseline_cadence_ticks: cf_save::delta::DEFAULT_BASELINE_CADENCE_TICKS,
            // **M4B § "Tamper-evident competitive replays"** — chain mode is
            // OFF by default for dev runs. Tournament mode opts in via cf-app
            // / cfctl `--ledger-chain` / `--tournament-mode`.
            ledger_chain_enabled: false,
            // **M14B** § empty by default; scenarios opt in by declaring
            // gravity_overrides / wind_sources / atmosphere_cells in the
            // manifest.
            initial_gravity_overrides: Vec::new(),
            initial_wind_sources: Vec::new(),
            initial_atmosphere_cells: Vec::new(),
            initial_stratification_cells: Vec::new(),
            initial_scripted_steps: Vec::new(),
            // **M14D** § empty by default; scenarios opt in by declaring
            // `m14d_projectile_pool[]` / `m14d_replay_intercepts` in the
            // manifest.
            initial_m14d_projectile_pool: Vec::new(),
            initial_replay_intercepts: false,
            // **M14E** § empty by default; scenarios opt in by declaring
            // `m14e_tunnel_spans[]` and (optionally) `m14e_cave_in_seed_offset`.
            initial_m14e_tunnel_spans: Vec::new(),
            initial_m14e_cave_in_seed_offset: 0,
            // **M14F § VAL-M14F-016**: empty by default; the M14F
            // scenarios declare `m14f_lateral_wall_spans[]`.
            initial_m14f_lateral_wall_spans: Vec::new(),
            // **M14G § VAL-M14G-013/014/029/030**: empty by default;
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
        // **M1.5 G6**: forward the scenario's difficulty preset id into the
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
        // **M6**: build initial squad roster from `ScenarioActor.squad_role`.
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
        // **M7 director v0.5 (audit gaps A12-A17)**: propagate the v0.5
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
        // **M14B** § gravity field + wind force + atmosphere cells. Build
        // cf-physics + cf-atmos types from the scenario manifest's
        // `gravity_overrides[]` / `wind_sources[]` / `atmosphere_cells[]`
        // arrays. Empty arrays = pass-through (no overrides).
        cfg.initial_gravity_overrides = scenario.gravity_overrides.iter().map(build_gravity_override).collect();
        cfg.initial_wind_sources = scenario.wind_sources.iter().map(build_wind_source).collect();
        cfg.initial_atmosphere_cells = scenario.atmosphere_cells.iter().map(build_atmos_cell).collect();
        cfg.initial_stratification_cells = scenario.atmosphere_cells.iter().map(build_strat_cell).collect();
        // **M14C** § propagate the scenario's scripted director steps.
        cfg.initial_scripted_steps = scenario.scripted_steps.clone();
        // **M14D** § propagate the scenario's projectile-pair pool +
        // replay_intercepts setting.
        cfg.initial_m14d_projectile_pool = scenario
            .m14d_projectile_pool
            .iter()
            .map(build_m14d_projectile_snapshot)
            .collect();
        cfg.initial_replay_intercepts = scenario.m14d_replay_intercepts;
        // **M14E** § propagate the scenario's tunnel spans + cave-in seed
        // offset so the per-tick collapse-check pass seeds correctly.
        cfg.initial_m14e_tunnel_spans = scenario.m14e_tunnel_spans.clone();
        cfg.initial_m14e_cave_in_seed_offset = scenario.m14e_cave_in_seed_offset;
        cfg.initial_m14f_lateral_wall_spans = scenario.m14f_lateral_wall_spans.clone();
        // **M14G § VAL-M14G-013/014/029/030**: propagate the scenario's
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
    /// **M1 R2**: pluggable audio backend. M1 default is `NullAudioPlugin`
    /// (no-op + tracing). cf-app or cf-tools-replay-viewer install their own
    /// implementation via `set_audio_plugin` to play real sound.
    pub(crate) audio_plugin: std::sync::Mutex<Box<dyn cf_audio::AudioPlugin>>,
    /// **M4B § "observe.save.last returns last save metadata"** — shared
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
    /// **M4B § "Delta baseline cadence is enforced"** — last snapshot we
    /// captured (used as the diff base for the next delta).
    pub(crate) m4b_previous_snapshot: Option<serde_json::Value>,
    /// **M4B**: event_id of the most recent `snapshot.baseline_emitted`.
    /// Stamped onto each subsequent `snapshot.delta_emitted` so the
    /// reconstructor can chain them back.
    pub(crate) m4b_last_baseline_event_id: Option<String>,
    /// **M4B**: tick at which the most recent baseline was emitted.
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
    /// **M1 / Gap D**: controls-captured state. `Some(capturer)` while an
    /// overlay holds input; the CONTROLS CAPTURED HUD zone renders and all
    /// `act.player.*` dispatches reject with reason `controls_captured`.
    pub(crate) controls_captured_by: Option<String>,
    /// **M14 audit pass 2 (GAP-M4-02 HIGH fix)**: latched true when
    /// `act.player.abort` succeeds. `record_run_finished` reads this to
    /// emit `system.run_finished.outcome="abort"` per M4 § Expected
    /// outcome + system events (previously hardcoded clean/panic only).
    pub(crate) run_aborted: bool,
    /// **M1 / Gap C2**: projectile_id -> spawn_event_id map persisted across
    /// ticks so when a projectile hits N ticks after spawn, the
    /// `combat.projectile_hit` event can parent to its originating
    /// `combat.projectile_spawned` event (closing the cause chain back to
    /// `equipment.weapon_fired` -> `input.intent_received`). Entries are
    /// pruned when the projectile reaches `combat.projectile_hit` or
    /// `combat.projectile_expired` to keep the map bounded.
    pub(crate) projectile_spawn_event_ids: BTreeMap<u64, String>,
    /// **M14C** § per-projectile round-kind discriminator. Populated at
    /// `combat.projectile_spawned` time (from
    /// `cf_actor::sim::SpawnedProjectile::round_kind`) and read by the
    /// `emit_m14_penetration_ray` helper to route HEAT / APFSDS impacts
    /// to the M14C producers (`heat_impact_producer` /
    /// `apfsds_impact_producer`) rather than the M14 baseline traversal.
    /// Pruned alongside `projectile_spawn_event_ids` after the projectile
    /// is resolved.
    pub(crate) projectile_round_kinds: BTreeMap<u64, cf_equipment::RoundKind>,
    /// **M1.5 forward-hook (Seam S1)**: latched by damage events so the
    /// next ReactiveGuard tick treats the damaged actor as a perception
    /// trigger. No consumer at M1; M1.5 ai layer reads it.
    #[allow(dead_code)]
    pub(crate) force_ai_update_this_tick: bool,
    /// **M1.5 G2 (hearing)**: alarms collected during the previous tick's
    /// actor step. The current tick's AI loop consumes these so guard
    /// hearing reacts ≤1 tick after the player's `equipment.alarm_registered`
    /// fires. Cleared after each AI loop.
    pub(crate) pending_alarms: Vec<cf_ai::AlarmInput>,
    /// **M1.5 G2 (hearing) staging**: alarms produced by THIS tick's actor
    /// step; promoted to `pending_alarms` at end-of-tick so they're
    /// available to the next tick's AI loop. Two-stage so AI never reads
    /// half-collected alarms mid-tick.
    pub(crate) pending_alarms_staging: Vec<cf_ai::AlarmInput>,
    /// M4A: HUD focus state (DR-012 ACC-A-04). The cf-app keyboard layer +
    /// cfctl `act.input.focus` advance/retreat focus through the canonical
    /// `HUD_FOCUSABLE_NODES` list; observe.accessibility surfaces it.
    pub(crate) hud_focus_index: Option<usize>,
    pub(crate) hud_focus_cycle: u64,
    /// **M9**: timer-warning thresholds already emitted this mission run
    /// (de-duplicated; each threshold fires exactly once per
    /// `TIMER_WARNING_THRESHOLDS_S`). Cleared on scenario reset.
    pub(crate) m9_timer_warnings_emitted: BTreeMap<u32, bool>,
    /// **M9**: per-actor concussion dose accumulator (0..=100) for the
    /// concussion band machine. Applied by combat hits + explosions.
    /// Decay/recovery happens via `m9_tick_concussion_recovery`.
    pub(crate) m9_concussion_dose: BTreeMap<ActorId, f32>,
    /// **M9**: per-actor last-seen concussion band so band crossings emit
    /// exactly once per transition.
    pub(crate) m9_concussion_band: BTreeMap<ActorId, &'static str>,
    /// **M9**: per-actor concussion recovery countdown (ticks). Reset on
    /// every dose application; ticks down to zero before recovery starts.
    pub(crate) m9_concussion_recovery_lockout_ticks: BTreeMap<ActorId, u32>,
    /// **M5**: previous tick's chassis stage on the player actor (used to
    /// raise stage-change banners without scanning the event log).
    pub(crate) hud_last_chassis_stage: Option<cf_chassis::ChassisStage>,
    /// **M5**: previous tick's pilot state.
    pub(crate) hud_last_pilot_state: Option<cf_chassis::PilotState>,
    /// **M1.5**: latest `input.intent_received` event_id from the player.
    /// Used as the `show_me_why_event_id` anchor on
    /// `mission.mission_resolved` when result=lost (DR-023 onboarding
    /// handoff — M3B viewer rewinds to this tick).
    pub(crate) last_player_input_event_id: Option<String>,
    /// **M2 re-audit pass 4 (2026-05-13)**: most-recent
    /// `actor.actor_status_changed` event id for the player actor. Used as
    /// `parent_event_id` on `mission.mission_resolved` when the loss path
    /// is `PlayerDead` so M10's cause-chain walker can hop
    /// `mission_resolved → actor_status_changed(player DYING) → projectile_hit → ...`.
    /// None until the first player status_changed fires.
    pub(crate) last_player_status_event_id: Option<String>,
    /// **M2**: current material-overlay mode for the HUD legend + render
    /// layer. One of "off" | "integrity" | "pathability" | "mobility" |
    /// "hazard" | "build_repair". Default "off".
    pub(crate) material_overlay_mode: String,
    /// **M2**: total debris pixels spawned (cumulative across the run).
    /// Surfaced via `observe.terrain.total_debris_spawned`.
    pub(crate) total_debris_spawned: u64,
    /// **M2**: total carve events emitted (cumulative). Distinct from
    /// `chunked_terrain.carve_count` (which counts terrain-state carves —
    /// `total_carve_events` counts every emitted carve event including
    /// strip + chunked).
    pub(crate) total_carve_events: u64,
    /// **M2**: last hazard contact tick per actor — used to debounce the
    /// per-tick hazard damage event to one per actor.
    pub(crate) hazard_last_contact_tick: BTreeMap<ActorId, u64>,
    /// **M2 re-audit (2026-05-13)**: id of the latest `mission.mission_started`
    /// event, used as parent for the first batch of `mission.objective_started`
    /// emissions per spec line 558 ("every event carries parent_event_id").
    pub(crate) mission_started_event_id: Option<String>,
    /// **M2 re-audit (2026-05-13)**: per-objective `mission.objective_started`
    /// event id keyed by objective id. Used as parent for
    /// `mission.objective_updated`, `mission.objective_completed`,
    /// `mission.objective_failed` so the cause chain walks back to the
    /// origination event.
    pub(crate) mission_objective_started_event_ids: BTreeMap<String, String>,
    /// **M4 § Parent-event-id cause chains**: most-recent `mission.*` event
    /// id, used as `parent_event_id` for snapshot re-emits at objective
    /// transitions (per spec literal "every event in {... snapshot_*} has
    /// parent_event_id"). Updated whenever any mission.* event fires.
    pub(crate) last_mission_event_id: Option<String>,
    /// **M4 § ai cause chains**: per-actor most-recent `ai.state_changed`
    /// event id. Used as parent for `ai.tactic_chosen` events emitted when
    /// no fresh perception_signal fired this tick.
    pub(crate) last_ai_state_changed_by_actor: BTreeMap<ActorId, String>,
    /// **M4 § system events**: most-recent `system.run_started` event id.
    /// Used as a fallback root parent when no other cause exists (per spec
    /// "the cause chain ... walks back to an `input.intent_received` or
    /// `system.run_started` root").
    pub(crate) run_started_event_id: Option<String>,
    /// **M4 § system.critical_drop**: last reported gameplay drop count so
    /// the engine only emits a `system.critical_drop` event for the delta
    /// (not the full cumulative total) each tick.
    pub(crate) last_reported_dropped_gameplay: u64,
    /// **M1 re-audit pass 4 (2026-05-13)**: per-actor `equipment.weapon_reload_started`
    /// event id, used as `parent_event_id` on the subsequent
    /// `equipment.weapon_reload_completed` so M10 viewers can walk the
    /// reload chain cleanly. Entry is inserted on reload_started and removed
    /// on reload_completed (so a cancelled reload doesn't strand a stale id).
    pub(crate) reload_started_event_id_by_actor: BTreeMap<ActorId, String>,
    /// **M3 re-open (2026-05-13)**: per-tick coalesced dirty-region accumulator.
    /// Carve events push their dirty rects + source event ids here; the engine
    /// flushes ONE `terrain.terrain_dirty_region_batch` per tick at end of
    /// `drive_tick` with the merged rect list + all contributing source ids.
    /// See `specs/active/M3.md` § Re-opened gaps.
    pub(crate) pending_dirty_rects: Vec<PendingDirtyRect>,
    /// **M3 re-open**: rolling counter of ticks where `unupdated_areas > 0`,
    /// used to trigger `terrain.forced_refresh_requested` after sustained
    /// load. Reset on any tick with `unupdated_areas == 0`.
    pub(crate) sustained_unupdated_ticks: u32,
    /// **M3 audit pass 7 (2026-05-13)**: monotonic path-invalidation version
    /// counter. Bumped every time `flush_pending_dirty_batch` produces a
    /// non-empty `out_rects[]`. Carried on `terrain.path_invalidated`
    /// events so M22+ pathfinder consumers can detect cache invalidation.
    pub(crate) path_invalidation_version: u64,
    /// **M3 re-open**: cumulative coalesce cost samples (ticks where a batch
    /// was emitted). Surfaced via `summary.json.perf.terrain` at run close.
    pub(crate) perf_coalesce_samples: Vec<u32>,
    pub(crate) perf_coalesce_rects_in_total: u64,
    pub(crate) perf_coalesce_rects_out_total: u64,
    /// **M6**: squad-of-two state surfaced by `observe.squad`. Empty by
    /// default — populated by scenarios that declare a friendly bot. See
    /// `cf_squad::Squad` for the canonical shape.
    pub(crate) squad: cf_squad::Squad,
    /// **M6**: per-actor in-flight weapon swap state. A swap starts on
    /// `act.player.weapon_swap` and ticks here until completion, when the
    /// engine emits `equipment.weapon_swap_completed` and removes the entry.
    pub(crate) weapon_swap_state: BTreeMap<ActorId, cf_equipment::WeaponSwap>,
    /// **M6**: last-emitted stamina value per actor for change-detection
    /// throttling. Stamina is only re-emitted when the value moves by more
    /// than `M6_STAMINA_EMIT_DELTA` to keep replay volume bounded.
    pub(crate) m6_last_stamina_emit: BTreeMap<ActorId, f32>,
    /// **M6**: last-emitted stealth-meter value per actor. Stealth meter is
    /// only re-emitted when the band (Hidden / Risky / Spotted) changes.
    pub(crate) m6_last_stealth_band: BTreeMap<ActorId, u8>,
    /// **M6**: last-emitted weight-bucket per actor (0 = under threshold,
    /// 1 = above). Toggling emits an `inventory.weight_changed` event.
    pub(crate) m6_last_weight_bucket: BTreeMap<ActorId, bool>,
    /// **M6B**: last-emitted discrete encumbrance band per actor
    /// (`None` / `Light` / `Moderate` / `Heavy`). Transitions emit
    /// `inventory.encumbrance_threshold_crossed`.
    pub(crate) m6b_last_encumbrance_band: BTreeMap<ActorId, cf_equipment::EncumbranceBand>,
    /// **M6**: per-actor footstep cadence accumulator (ticks since last
    /// emitted `perception.footstep_emitted`). Prevents replay spam.
    pub(crate) m6_footstep_cooldown: BTreeMap<ActorId, u32>,
    /// **M6**: in-flight grenade projectiles thrown via
    /// `act.player.throw_grenade`. The tick scheduler advances each one
    /// under gravity + collision and emits
    /// `equipment.grenade_detonated` at fuse=0.
    pub(crate) grenade_projectiles: Vec<GrenadeProjectile>,
    /// **M6**: in-flight knife projectiles thrown via
    /// `act.player.knife_throw`. The tick scheduler advances each one
    /// under physics and emits `combat.knife_throw_landed` on collision.
    pub(crate) knife_projectiles: Vec<cf_equipment::KnifeProjectile>,
    /// **M6**: latched-per-actor previous `FacingDirection`, used by the
    /// engine to emit `actor.facing_changed` only on flips (not every tick).
    pub(crate) m6_last_facing: BTreeMap<ActorId, cf_actor::FacingDirection>,
    /// **M6**: persistent map markers dropped via the Beacon tool. Each
    /// entry is (owner_id, position). Surfaced via observe.squad for the
    /// HUD; consumed by future M7 mission director when waypoints route
    /// AI.
    pub(crate) m6_beacons: Vec<(ActorId, cf_actor::Vec2)>,
    /// **M6**: physically-dropped items in the world. Created by
    /// `act.player.drop_item`, consumed by `act.player.pickup`. Each item
    /// carries the actor that dropped it, the item id (rifle preset or
    /// material id), the position, and the slot the dropping inventory
    /// originally held it in.
    pub(crate) m6_dropped_items: Vec<DroppedItem>,
    /// **M6**: monotonic id counter for dropped items.
    pub(crate) m6_next_dropped_item_id: u64,
    /// **M6**: per-actor latch consumed by `emit_actor_events` so a Charge-mode
    /// release whose `charge_fraction < SNIPER_MISFIRE_BELOW` annotates the
    /// `equipment.weapon_fired` event with `misfire=true`. Drained each tick
    /// after the recorder reads it.
    pub(crate) m6_charge_misfires: BTreeMap<ActorId, ChargeFireInfo>,
    /// **M7-A**: smart commandable AI surface — per-actor BotState
    /// (Archetype + 5-layer ThinkingStack + auto-triage/auto-repair
    /// missions), faction registry, 4-phase mission director, reinforcement
    /// registry, mini-boss state. Co-resident with M2 `reactive_guards`:
    /// the M2 guard FSM still drives projectile / fire behavior; M7-A
    /// adds the reason-label + role-template surface on top.
    pub(crate) m7_ai_world: crate::m7_ai::M7AiWorld,
    /// **M7B**: deep squad-command grammar surface — per-squad
    /// `SquadState` (current verb + formation + role assignments +
    /// breach-chain progress + bounding-step state) + verb registry +
    /// doctrine-compatibility matrix. Lives on the squad NOT on the held
    /// actor so brain-hop preserves doctrine.
    pub(crate) m7b_squad: crate::m7b_squad::M7BSquadWorld,
    /// **M8**: smooth-follow + hit-stop + scope + free-look camera state.
    pub(crate) camera_state: cf_camera::CameraState,
    /// **M8**: photo mode (basic stub) state machine + filter + free camera.
    pub(crate) photo_mode: cf_photo::PhotoModeState,
    /// **M8**: replay-scrubber 30s window + bookmarks.
    pub(crate) replay_scrub: cf_replay_scrub::ReplayScrubState,
    /// **M8**: killcam state machine (Idle / Recording / Playing / Done) +
    /// 1.5s slow-mo cinematic variant.
    pub(crate) killcam: cf_killcam::KillcamState,
    /// **M8**: cf-debug overlay registry (which of the 7 overlays are
    /// currently rendered).
    pub(crate) debug_state: cf_debug::DebugOverlayState,
    /// **M8**: Tab tactical overlay state (open + sim-speed cap +
    /// focused actor + open count).
    pub(crate) tactical_overlay: cf_squad_ui::TacticalOverlayState,
    /// **M8**: per-bot tactical plan queue (Plan Composer).
    pub(crate) plans: BTreeMap<ActorId, cf_squad_ui::Plan>,
    /// **M8**: MMB tag state — tagged target ids + per-tag TTL + +0.5
    /// utility weight bonus.
    pub(crate) tag_state: cf_squad_ui::TagState,
    /// **M8**: T-key pie menu state — 8-slice radial action wheel for
    /// the player's own 8 actor actions (Pickup / Drop / SwitchWeapon /
    /// ThrowGrenade / MeleeBash / DeployBipod / SignalSquad / UseMedkit)
    /// with target context + 6 disabled-slice reason labels + sim
    /// slowdown gate (single-player 20%, multiplayer 100%).
    pub(crate) pie_menu: cf_squad_ui::PieMenuState,
    /// **M8**: en localization table loaded once from the bundled
    /// `content/localization/en.json` baseline. Re-loaded if `Settings.
    /// language` changes (only `en` ships at M8).
    pub(crate) localization: cf_localization::LocalizationTable,
    /// **M8 game_speed_assist consumer**: deterministic tick-skip
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
    /// **M8 game_speed_assist consumer**: true when the engine is
    /// hosting a networked multiplayer session (M36+). `game_speed_
    /// assist` is single-player-only per spec, so the per-tick
    /// scheduler treats this flag as a kill-switch: multiplayer
    /// always runs at 100% regardless of the Settings value. M8
    /// ships with the flag pinned `false` (no multiplayer scenarios
    /// exist yet); the persistent setter is reserved for the M36+
    /// scenario loader.
    pub(crate) multiplayer_session: bool,
    /// **M9B**: live trench-segment placement index. Mutated by
    /// `act.player.dig_trench_segment`, `act.player.place_trench_module`,
    /// and `act.player.drop_trench_template`; consumed by
    /// `compute_actor_cover_state` + `compute_trench_segment_at_pos`
    /// so the observe surfaces project real per-segment state instead
    /// of always-empty placeholders.
    pub(crate) trench_world: cf_trench::segment::InMemorySegments,
    /// **M9B**: monotonic id counter for placed trench segments.
    /// Replay events reference segments via this id so the cause
    /// chain stays linear across dig → place_module → repair_module
    /// → breach → collapse.
    pub(crate) trench_next_segment_id: u64,
    /// **M9B audit GAP-1**: per-actor latched cover state + segment
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
    /// **M9B audit GAP-2**: per-actor exposure tick counter for the
    /// `AI-TRENCH-A-01` doctrine. Increments while the actor remains
    /// in `CoverState::Exposed`; resets on any other cover state. The
    /// doctrine reads this to enforce the spec's "no AI remains
    /// Exposed continuously > 1.5 seconds" invariant.
    pub(crate) m9b_trench_doctrine_exposure_ticks: BTreeMap<ActorId, u32>,
    /// **M9B audit GAP-2**: opt-in set of actor ids the engine drives
    /// through the trench doctrine each tick. Currently populated by
    /// the scenario loader when a reactive_guard entry carries
    /// `doctrine: Some("AI-TRENCH-A-01")` in its scenario RON; the
    /// scenario `m9b_ai_in_trench_doctrine` opts in its three
    /// defenders.
    pub(crate) m9b_trench_doctrine_actors: std::collections::BTreeSet<ActorId>,
    /// **M12C**: in-engine cinematic playback kernel. `Some` while a
    /// cinematic is playing (opening / between-mission / ending);
    /// `None` when the gameplay camera + input are in normal control.
    /// cfctl `act.player.skip_cinematic`, `act.player.pause_cinematic`,
    /// `act.player.replay_cinematic`, and `srv.dump_cinematic_state`
    /// operate on this slot.
    pub(crate) cinematic_kernel: Option<cf_cinematic::CinematicKernel>,
    /// **M12C**: persisted seen-set of cinematics the player has
    /// watched (or skipped past the 3-second confirm window). Lives
    /// here at M12C; M41 save format will persist it to `save.cinematic_seen_set`.
    pub(crate) cinematic_seen_set: cf_cinematic::SeenSet,
    /// **M12C**: LUFS-aware narration/music/SFX duck mixer. Engaged
    /// when a cinematic kernel boots; releases at `cinematic.ended`.
    /// Per spec § "Cinematic mixer ducks music under narration".
    pub(crate) cinematic_mixer: cf_audio::CinematicMixer,
    /// **M12C**: snapshot of the renderer-side camera takeover state
    /// (mirrors the cinematic kernel's composed offset). cf-app's
    /// bridge polls this via `engine.cinematic_takeover_snapshot()`.
    pub(crate) cinematic_takeover: cf_cinematic::CinematicTakeoverSnapshot,
    /// **M12C**: `cinematic.rival_taunt` 40% deterministic gate per
    /// spec § Between-mission cinematic. Drained per between-mission
    /// engage; the M25 hook will fold real rival-alive state into this
    /// roll when it ships.
    pub(crate) cinematic_rival_taunt_roll: u8,
    /// **M14B** § gravity field producer state. Authored by the scenario
    /// manifest's `gravity_overrides[]` array; consumed by per-tick
    /// `cf_physics::apply_overrides` calls.
    pub(crate) m14b_gravity_overrides: Vec<cf_physics::GravityOverride>,
    /// **M14B** § wind force producer apertures.
    pub(crate) m14b_wind_sources: Vec<cf_atmos::WindSource>,
    /// **M14B** § authored atmosphere cells (pressure + temperature). Drives
    /// the wind force kernel + observe.frame.cells projection.
    pub(crate) m14b_atmos_cells: Vec<cf_atmos::AtmosCell>,
    /// **M14B** § gas-stratification cells (per-column composition). Runs
    /// every 4th tick per the spec.
    pub(crate) m14b_strat_cells: Vec<cf_atmos::StratCell>,
    /// **M14C** § scripted director steps loaded from the scenario manifest's
    /// `scripted_steps` array. `drive_tick` injects matching intents into
    /// `pending_intent` before the actor sim runs so headless cfctl drives
    /// of `m14c_heat_vs_era.ron` / `m14c_apfsds_vs_heavy.ron` actually fire
    /// the HEAT / APFSDS round at a deterministic tick (rather than no-op).
    pub(crate) m14c_scripted_steps: Vec<crate::scenario::ScenarioScriptStep>,
    /// **M14D** § projectile-pair pool consumed by
    /// `cf_physics::run_projectile_pair_pass` between the actor-collision
    /// pass and the terrain pass. Authored at scenario load + advanced
    /// each tick. Empty by default (pre-M14D scenarios behave identically).
    pub(crate) m14d_projectile_pair_pool: Vec<cf_physics::ProjectileSnapshot>,
    /// **M14D § VAL-M14D-020** per-tick projectile-pair pass invocation
    /// counter. Incremented once per call to
    /// [`M0Engine::tick_m14d_projectile_pair`]. Exposed via the
    /// schedule-trace accessor for the `pass_called_once_per_tick` test.
    pub(crate) m14d_pair_pass_invocations: u64,
    /// **M14D § VAL-M14D-008/009/010** rolling trace of the per-tick pair
    /// pass timing + candidate counts surfaced to perf tests.
    pub(crate) m14d_last_pair_pass_trace: cf_physics::ProjectilePairPassTrace,
    /// **M14D § VAL-M14D-019** per-player `replay_intercepts` setting.
    /// Default false — killcam excludes `collision.projectile_pair_contact`
    /// events unless the player opts in.
    pub(crate) m14d_replay_intercepts: bool,
    /// **M14D § VAL-M14D-006** C-RAM cooldown latches keyed by APS
    /// laser `owner_actor_id`. Engaged on every
    /// `collision.projectile_pair_contact{outcome="aps_intercept"}`
    /// event and decayed by [`cf_equipment::Cram::tick`] each
    /// projectile-pair pass. Empty by default; an entry materialises
    /// the first time a given owner fires an intercept.
    pub(crate) m14d_cram_cooldowns: BTreeMap<u64, cf_equipment::Cram>,
    /// **M14D § VAL-M14D-020** schedule-trace ordered window. Records
    /// each pass entry ("actor_collision_start", "projectile_pair_start",
    /// "terrain_start", ...) for the most recent N ticks so the engine
    /// integration test can assert ordering. Capped at 120 entries.
    pub(crate) m14d_schedule_trace: std::collections::VecDeque<&'static str>,
    /// **M14B** § per-(actor, override) activation latch. Used by the
    /// per-tick step to emit `gravity.override_activated` only on entry +
    /// `gravity.override_deactivated` only on exit. The inner BTreeSet
    /// holds the override ids currently active for the actor; the outer
    /// map is keyed by actor id.
    pub(crate) m14b_active_overrides: BTreeMap<ActorId, std::collections::BTreeSet<u32>>,
    /// **M14B** § per-WindSource remaining TTL in ticks. Used for
    /// transient apertures (pipe ruptures) spawned via
    /// [`Self::inject_pipe_rupture`]. Each tick the value decrements; on
    /// reaching zero the WindSource + its synthetic atmosphere cells are
    /// removed.
    pub(crate) m14b_transient_wind_ttl: BTreeMap<u32, u32>,
    /// **M14B** § synthetic atmosphere-cell ids that were spawned by
    /// transient wind sources (pipe ruptures). Used to clean up the
    /// atmosphere cell list when the parent WindSource expires.
    pub(crate) m14b_transient_cells: Vec<u32>,
    /// **M14E** § per-chunk integrity-field state authored by the
    /// scenario manifest's `m14e_tunnel_spans[]` array. Indexed by chunk
    /// coordinate; each entry tracks the current integrity field + the
    /// cached span_px + anchored flag the per-tick pass consumes.
    pub(crate) m14e_chunks: BTreeMap<(i32, i32), M14eChunkState>,
    /// **M14E** § integrity-pass invocation count (incremented exactly
    /// once per N-tick boundary). Exposed via the schedule-trace
    /// accessor for the `compute_integrity_pass_runs_every_15_ticks`
    /// VAL-M14E-019 test.
    pub(crate) m14e_pass_invocations: u64,
    /// **M14E** § deterministic RNG cursor for the cave-in roll. Seeded
    /// from `scenario.seed + m14e_cave_in_seed_offset`; advances on every
    /// cave-in roll regardless of outcome so the draw sequence is stable
    /// across same-seed runs.
    pub(crate) m14e_rng_state: u64,
    /// **M14E** § knockdown latch keyed by actor id. Set when a cave-in
    /// debris impulse routes through `cf_physics::cave_in_fall_impulse_chain`
    /// and forces the actor into KnockedDown.
    pub(crate) m14e_actor_knockdown: BTreeMap<u64, bool>,
    /// **M14E** § last-tick at which a chunk fired
    /// `terrain.cave_in_triggered`. Drives the 15-tick cascade window
    /// per VAL-M14E-018.
    pub(crate) m14e_last_cave_in_tick: BTreeMap<(i32, i32), u64>,
    /// **M14E** § total cumulative number of `terrain.cave_in_triggered`
    /// events emitted (used for replay summary + cross-tick assertions).
    pub(crate) m14e_total_cave_ins: u32,
    /// **M14E** § cumulative `terrain.support_beam_placed` event count.
    pub(crate) m14e_total_beams_placed: u32,
    /// **M14E** § cumulative `terrain.support_beam_destroyed` event count.
    pub(crate) m14e_total_beams_destroyed: u32,
    /// **M14E** § render-side queue mirroring the L1/L2/L3 crack decals +
    /// falling-debris cones the per-tick collapse-check pass produces.
    /// `cf-app` drains this every frame; headless runs let it grow up to a
    /// soft cap (see `drain_*`).
    pub(crate) m14e_tunnel_collapse_queue: cf_render_2d::tunnel_collapse::TunnelCollapseQueue,
    /// **M14E** § total `cf-audio::AudioCue::TunnelCreak` cues enqueued
    /// (the engine surfaces these via `emit_audio_cue` already; we still
    /// keep a counter for cross-tick test assertions).
    pub(crate) m14e_tunnel_creak_count: u32,
    /// **M14E** § total `cf-audio::AudioCue::CaveInThunder` cues enqueued.
    pub(crate) m14e_cave_in_thunder_count: u32,
    /// **M14E** § per-actor crafting resource ledger (iron, wood, etc.).
    /// Used by the support-beam placer's inventory-debit path so VAL-M14E-009
    /// can assert the post-placement delta.
    pub(crate) m14e_actor_resources: BTreeMap<u64, BTreeMap<String, i64>>,
    /// **M14E** § per-actor plasma-cutter use flag (drives the
    /// "VIBRATION ACCUMULATING" HUD banner per VAL-M14E-015).
    pub(crate) m14e_plasma_cutter_active: BTreeMap<u64, bool>,
    /// **M14F** § per-chunk lateral-wall runtime state keyed by chunk
    /// coord. Authored by `m14f_lateral_wall_spans[]` in the scenario
    /// manifest. The per-chunk `IntegrityField` is borrowed from the
    /// shared `m14e_chunks` map per VAL-CROSS-005 — this map only
    /// tracks the lateral-axis metadata + per-tier emission flags.
    pub(crate) m14f_lateral_chunks: BTreeMap<(i32, i32), M14fLateralChunkState>,
    /// **M14F § VAL-M14F-016**: lateral integrity pass invocation
    /// count. Equal to `floor(T / 15)` after T ticks.
    pub(crate) m14f_lateral_pass_invocations: u64,
    /// **M14G § VAL-M14G-046**: total invocations of the wound-aging
    /// pass. Equal to the number of engine ticks since boot (one
    /// invocation per tick).
    pub(crate) m14g_wound_aging_invocations: u64,
    /// **M14G**: per-engine WoundSpec registry, populated lazily from the
    /// baked defaults on first use so cf-control does not need to read
    /// content files at engine boot.
    pub(crate) m14g_wound_registry: Option<cf_wound::WoundSpecRegistry>,
    /// **M14G § VAL-M14G-013/014/030**: thermal-contact zones authored
    /// by the scenario manifest. The engine ticks the dwell counter
    /// per `(actor_id, zone)` every tick.
    pub(crate) m14g_thermal_zones: Vec<crate::scenario::ScenarioThermalZone>,
    /// **M14G**: per-`(actor_id, zone)` dwell-tick counter for the
    /// thermal pass.
    pub(crate) m14g_thermal_dwell_ticks: BTreeMap<(u64, String), u64>,
    /// **M14G**: latch the most-recently emitted burn/frostbite degree
    /// per `(actor_id, zone)` so the producer fires escalation events
    /// only when the degree actually changes.
    pub(crate) m14g_thermal_emitted_kind: BTreeMap<(u64, String), cf_wound::WoundKind>,
    /// **M14G § VAL-M14G-029**: material-contact entries authored by
    /// the scenario manifest. Each entry fires one `wound.created`
    /// event on its `fire_tick`.
    pub(crate) m14g_material_contacts: Vec<crate::scenario::ScenarioMaterialContact>,
    /// **M14G**: indices of `m14g_material_contacts` already fired,
    /// so the engine never emits the same contact twice.
    pub(crate) m14g_material_contacts_fired: std::collections::BTreeSet<usize>,
    /// **M14I**: persistent veteran roster (cf-veteran). Snapshots the
    /// per-actor `LongTermState` for downstream M41 consumers (roster
    /// UI, narrative tab).
    pub(crate) m14i_veteran_roster: cf_veteran::VeteranRoster,
    /// **M14I**: registry of pending retirement narratives. Populated
    /// when `act.player.retire_veteran` fires. M48 storyteller consumes
    /// the canonical `narrative.veteran_retired` event ids registered
    /// here.
    pub(crate) m14i_retirement_narratives:
        cf_storyteller::retirement_event::RetirementNarrativeRegistry,
    /// **M14F § VAL-M14F-009**: per-actor flood-contact flag. Tick at
    /// which the actor was first registered as submerged / damp after
    /// a dam rupture.
    pub(crate) m14f_actor_submerged_tick: BTreeMap<u64, u64>,
    /// **M14F § VAL-M14F-011**: per-actor vacuum-exposure tick. Tick
    /// at which the actor was first registered as exposed to vacuum
    /// after a sealed-room rupture.
    pub(crate) m14f_actor_vacuum_tick: BTreeMap<u64, u64>,
    /// **M14F § VAL-M14F-007**: cumulative fluid-mass that propagated
    /// through the breach bbox per dam chunk. Increments each tick
    /// after rupture until the volume depletes.
    pub(crate) m14f_breach_fluid_mass: BTreeMap<(i32, i32), u64>,
    /// **M14F § VAL-M14F-008**: pressure samples (room-side, vacuum-
    /// side) per sealed-room chunk. Updated each tick after rupture
    /// so the delta monotonically decreases toward equilibrium.
    pub(crate) m14f_breach_pressure_kpa: BTreeMap<(i32, i32), (f32, f32)>,
    /// **M14J § verlet-rope world** — every embedded grapple line + zip-line
    /// cable in the scene, keyed by [`cf_physics::RopeId`]. The engine
    /// advances each rope each tick via `cf_physics::rope::Rope::step`.
    pub(crate) m14j_ropes: BTreeMap<cf_physics::RopeId, cf_physics::Rope>,
    /// **M14J § rope-id allocator**. Bumped on every grapple embed +
    /// zip-kit deploy.
    pub(crate) m14j_next_rope_id: u64,
    /// **M14J § zipline kind tag** — true entry means this rope is a
    /// deployed zip-line (not a grapple rope). The slide engine consults
    /// this to apply gravity-along-cable + brake-deceleration.
    pub(crate) m14j_zipline_ropes: std::collections::BTreeSet<cf_physics::RopeId>,
    /// **M14J § zipline rider slide speed** — per-rider current speed
    /// along the cable (m/s, positive = toward low end).
    pub(crate) m14j_zipline_speed_by_rider: BTreeMap<u64, f32>,
    /// **M15 § active material kernel** — per-tick orchestrator for
    /// the M15+M15B chemistry. When `chunked_terrain` is present, the
    /// engine calls `cf_material::kernel_step` each tick to drive
    /// per-pixel reactions, phase transitions, and CA movement. State
    /// lives here so the Margolus stepper parity persists across ticks.
    pub(crate) material_kernel: cf_material::MaterialKernel,
    /// **M15 § reaction registry**. Loaded from
    /// `content/materials/reaction_registry.json` at engine init, or
    /// falls back to the hardcoded `default_reaction_registry` when
    /// the file isn't present.
    pub(crate) reaction_registry: cf_material::ReactionRegistry,
    /// **M15 § phase-transition registry**. Loaded from
    /// `content/materials/phase_registry.json` at engine init.
    pub(crate) phase_registry: cf_material::PhaseRegistry,
    /// **M15 § per-cell heat field**. Stub-initialized at ambient
    /// (293.15 K Earth baseline) until M19 atmospherics wires per-cell
    /// thermal sources. The kernel uses this for reaction temperature
    /// gating + phase-transition threshold crossing.
    pub(crate) heat_field: cf_terrain::HeatField,
    /// **M15 § previous-tick heat snapshot**. Used to detect threshold
    /// crossings for phase transitions. `None` on the first tick.
    pub(crate) prev_heat_field: Option<cf_terrain::HeatField>,
    /// **M15B § precipitation cycle**. Tracks per-cell cloud
    /// saturation, fires nucleation + precipitation events as steam
    /// pixels climb above the altitude/temperature gates.
    pub(crate) precipitation_cycle: cf_material::PrecipitationCycle,
    /// **M15B § precipitation tuning config**. Loaded from
    /// `content/materials/precipitation_config.json` at engine init.
    pub(crate) precipitation_config: cf_material::PrecipitationConfig,
}

/// **M14F** § Per-chunk lateral-wall runtime state. Lives on
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

/// **M14E** § Per-chunk integrity-field runtime state. Lives on
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

/// **M6**: per-actor charge-fire annotation shipped from the M6 post-step
/// hook to `emit_actor_events`. `charge_fraction` is the latched accumulator
/// at trigger release; `misfire` is true iff the fraction was below
/// [`cf_equipment::SNIPER_MISFIRE_BELOW`].
#[derive(Debug, Clone, Copy)]
pub(crate) struct ChargeFireInfo {
    pub charge_fraction: f32,
    pub misfire: bool,
}

/// **M6**: a physical item entity in the world. Spawned by
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

/// **M6**: one in-flight grenade projectile. Owned by the engine's
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

/// **M6**: scratch struct passed from cfctl dispatch to the post-dispatch
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

/// **M6**: scratch struct that captures the parameters of a melee strike
/// from the cfctl dispatch site so the engine can scan for hit actors +
/// roll knockdown + emit the hit event in a separate, post-dispatch phase.
#[derive(Debug, Clone)]
pub(crate) struct PendingMeleeResolve {
    pub(crate) attacker: ActorId,
    pub(crate) kind: cf_equipment::MeleeKind,
    pub(crate) facing_sign: f32,
    pub(crate) actor_position: cf_actor::Vec2,
}

/// **M6**: scratch struct that captures the parameters of a knife throw
/// from the cfctl dispatch site so the engine can spawn the
/// [`cf_equipment::KnifeProjectile`] after releasing the write-guard.
#[derive(Debug, Clone)]
pub(crate) struct PendingKnifeSpawn {
    pub(crate) owner: ActorId,
    pub(crate) origin: cf_actor::Vec2,
    pub(crate) aim: cf_actor::Vec2,
    pub(crate) base_damage: f32,
}

/// **M6**: scratch struct that captures the parameters of a stealth-kill
/// attempt from the cfctl dispatch site so the engine can find the target
/// (behind + within reach) + apply instant-kill damage + emit
/// `combat.stealth_kill_executed` after releasing the write-guard.
#[derive(Debug, Clone)]
pub(crate) struct StealthKillAttempt {
    pub(crate) attacker: ActorId,
    pub(crate) attacker_pos: cf_actor::Vec2,
    pub(crate) attacker_facing_x: f32,
}

/// **M6**: per-tool effect kind dispatched after the actor write-guard is
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

/// **M6**: resolved melee hit data, captured for emission in the
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

/// **M3 re-open (2026-05-13)**: a single dirty-region entry pushed by a carve
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

/// **M3 re-open (2026-05-13)**: helper struct for the end-of-tick coalesce
/// pass. Holds the merged AABB after union operations.
#[derive(Debug, Clone)]
pub(crate) struct MergedDirtyRect {
    pub(crate) cx: i32,
    pub(crate) cy: i32,
    pub(crate) min: [i64; 2],
    pub(crate) max: [i64; 2],
}

/// **M3 re-open**: two rects "touch or overlap" if they share at least one
/// edge or interior. Used by the greedy coalesce pass. Adjacent chunks
/// (e.g. (0,0) at [0,0..256] + (1,0) at [256,0..512]) satisfy
/// `a.max[0] == b.min[0]` so the inclusive `>=`/`<=` comparison captures
/// shared-edge unions.
pub(crate) fn rects_touch_or_overlap(a_min: [i64; 2], a_max: [i64; 2], b_min: [i64; 2], b_max: [i64; 2]) -> bool {
    let x_overlap = a_min[0] <= b_max[0] && b_min[0] <= a_max[0];
    let y_overlap = a_min[1] <= b_max[1] && b_min[1] <= a_max[1];
    x_overlap && y_overlap
}

/// **M6B**: walk the inventory grid tree to find the nesting depth of the
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
        let mut m9b_trench_doctrine_actors = std::collections::BTreeSet::<ActorId>::new();
        // **M1.5 G6**: when the scenario carries a difficulty_preset id,
        // overlay the preset onto each guard's params at spawn time so the
        // preset's miss_chance / aim_settle / hearing_radius / etc. are
        // already active by the first AI tick. The preset gracefully
        // falls back to the per-guard params from the scenario manifest
        // when the id is unknown.
        let preset = config
            .difficulty_preset
            .as_deref()
            .and_then(cf_ai::DifficultyPreset::builtin);
        for guard in &config.initial_guards {
            let mut params = guard.params;
            if let Some(p) = &preset {
                p.apply_to(&mut params, config.tick_rate_hz);
            }
            if guard
                .doctrine
                .as_deref()
                .map(|d| d == cf_ai::trench_doctrine::DOCTRINE_ID)
                .unwrap_or(false)
            {
                m9b_trench_doctrine_actors.insert(guard.actor);
            }
            let mut rg = cf_ai::ReactiveGuard::new(guard.actor, params);
            // Latch max_hp from the actor world the engine just built so
            // the retreat-hp gate has a real denominator.
            if let Some(sim) = &actor_state {
                if let Some(actor) = sim.world.actors.get(&guard.actor) {
                    rg.max_hp = actor.hp.max(1.0);
                }
            }
            reactive_guards.insert(guard.actor, rg);
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
        // **M6**: instantiate the squad-of-two from the scenario manifest.
        // One leader + N followers; the engine emits `squad.member_added`
        // for each member at run start (see `emit_initial_snapshots`).
        let mut squad = cf_squad::Squad::default();
        for member in &config.initial_squad_members {
            let m = cf_squad::SquadMember::new(member.actor, member.role, member.display_name.clone(), member.hp_max);
            match member.role {
                cf_squad::SquadRole::Leader => {
                    let _ = squad.add_leader(m);
                }
                cf_squad::SquadRole::Follower => {
                    let _ = squad.add_follower(m);
                }
            }
        }

        // **M7-A**: seed the M7-A AI world with a `BotState` for every
        // reactive guard the scenario declared so the 5-layer thinking
        // stack ticks alongside the M2 FSM from tick 0. Built before the
        // EngineMutable move below so the borrow checker is happy.
        // **M7 director v0.5 (audit gaps A12-A17)**: seed phase / waves /
        // boss / graph from the scenario manifest fields plumbed via
        // `M0EngineConfig::initial_*`. None means the scenario opts
        // out of v0.5 (the M2 single-vec objective list stays the
        // authoritative mission shape).
        let m7_ai_world_seed = {
            let mut world = crate::m7_ai::M7AiWorld::new();
            for actor_id in reactive_guards.keys() {
                world.assign_archetype(*actor_id, cf_ai::Archetype::Rifleman);
            }
            if let Some(phase) = config.initial_phase_state.clone() {
                world.phase = Some(phase);
            } else if !config.initial_reactors.is_empty() {
                // **M9** (audit fix gap 3): when the scenario carries a
                // reactor world but no explicit phase_state, default-init
                // the 7-phase reactor-defense pacer so guards spawn at
                // tick ~300 + cfctl `observe.mission.director` has a real
                // PhaseState to project. Scenarios that want M7 4-phase
                // pacing instead still set `phase_state` explicitly.
                world.phase = Some(cf_mission::PhaseState::new_m9_reactor_defense(0));
            }
            for wave in config.initial_reinforcement_waves.clone() {
                world.reinforcements.push(wave);
            }
            if let Some(boss) = config.initial_boss_state.clone() {
                world.boss = Some(boss);
            }
            if let Some(graph) = config.initial_objective_graph.clone() {
                world.objective_graph = Some(graph);
            }
            world
        };

        diagnostics::set_panic_reporter({
            let recorder = recorder.clone();
            let tick_snap = current_tick.clone();
            move |msg| {
                let t = tick_snap.load(std::sync::atomic::Ordering::Relaxed);
                report_panic_to_recorder(&recorder, t, t as f64 * tick_dt_ms, msg);
            }
        });

        // **M14B**: snapshot the producer-side authored state before moving
        // `config` into the engine struct. The engine's mutable side owns
        // its own copy so per-tick mutations (e.g. `DamagedGrav` wave-front
        // growth, stratification deltas) don't bleed back into the config.
        let m14b_gravity_overrides = config.initial_gravity_overrides.clone();
        let m14b_wind_sources = config.initial_wind_sources.clone();
        let m14b_atmos_cells = config.initial_atmosphere_cells.clone();
        let m14b_strat_cells = config.initial_stratification_cells.clone();
        let m14c_scripted_steps = config.initial_scripted_steps.clone();
        let m14d_projectile_pair_pool = config.initial_m14d_projectile_pool.clone();
        let m14d_replay_intercepts = config.initial_replay_intercepts;
        let m14e_initial_tunnel_spans = config.initial_m14e_tunnel_spans.clone();
        let m14e_cave_in_seed_offset = config.initial_m14e_cave_in_seed_offset;
        let m14e_initial_rng_state = config.seed.wrapping_add(m14e_cave_in_seed_offset);
        let m14f_initial_lateral_wall_spans = config.initial_m14f_lateral_wall_spans.clone();
        let mut m14e_chunks: BTreeMap<(i32, i32), M14eChunkState> = BTreeMap::new();
        for span in &m14e_initial_tunnel_spans {
            let mut field = cf_terrain::IntegrityField::pristine();
            if span.anchored {
                let center_lx = cf_terrain::INTEGRITY_FIELD_WIDTH / 2;
                let center_ly = cf_terrain::INTEGRITY_FIELD_HEIGHT / 2;
                cf_terrain::lock_radius_to_beam(&mut field, center_lx, center_ly, 1);
            }
            m14e_chunks.insert(
                span.chunk_id,
                M14eChunkState {
                    field,
                    span_id: span.id.clone(),
                    bbox_min: [span.bbox_min.0, span.bbox_min.1],
                    bbox_max: [span.bbox_max.0, span.bbox_max.1],
                    unsupported_span_px: span.unsupported_span_px,
                    ceiling_thickness_px: span.ceiling_thickness_px,
                    vibration_modifier: span.vibration_modifier,
                    anchored: span.anchored,
                    cascade_neighbors: span.cascade_neighbors.clone(),
                    damage_actor_id: span.damage_actor_id,
                    structural_integrity_low_emitted: false,
                    cave_in_emitted: false,
                    m14f_owns_rupture_emit: false,
                    cave_in_pending_cascade: false,
                    l1_at_tick: None,
                    l2_at_tick: None,
                    l3_at_tick: None,
                    force_integrity_pass_deadline: None,
                    crack_decal_l1_enqueued: false,
                    crack_decal_l2_enqueued: false,
                    crack_decal_l3_enqueued: false,
                    structural_warning_banner_emitted: false,
                },
            );
        }
        // **M14F § VAL-M14F-002 / VAL-M14F-016**: build the per-chunk
        // lateral-wall state from the scenario's
        // `m14f_lateral_wall_spans[]` rows. Each row gets a fresh
        // pristine `IntegrityField` in the shared `m14e_chunks` map
        // (per VAL-CROSS-005) when no ceiling-span already covers that
        // chunk. The bulging countdown is deterministic: any span
        // strictly above the 16-px floor counts down toward a guaranteed
        // bulging event well inside the spec's 30-tick window
        // (VAL-M14F-002).
        let mut m14f_lateral_chunks: BTreeMap<(i32, i32), M14fLateralChunkState> = BTreeMap::new();
        for span in &m14f_initial_lateral_wall_spans {
            // Make sure the chunk has an entry in the shared map so the
            // lateral pass can borrow `chunk.field`. We re-use the M14E
            // chunk-state surface (single-buffer invariant).
            // **VAL-CROSS-024**: explicit-flag suppression. On chunks
            // created exclusively by M14F lateral wall init (no prior
            // M14E ceiling span) `m14f_owns_rupture_emit = true` keeps
            // the M14E cave-in roll off — the lateral pass owns the
            // rupture surface there. Chunks that already have an M14E
            // tunnel span keep their M14E-init value (false) so a
            // composite "ceiling + wall on the same chunk_id" topology
            // emits both events.  Setting
            // `m14e_composite_cascade_allowed=true` does NOT flip this
            // flag — it only opts the rupture into cascading M14E
            // cave-in on its `cascade_neighbors`, see
            // [`Self::m14f_cascade_rupture_to_m14e_neighbors`].
            m14e_chunks.entry(span.chunk_id).or_insert_with(|| M14eChunkState {
                field: cf_terrain::IntegrityField::pristine(),
                span_id: span.id.clone(),
                bbox_min: [span.bbox_min.0, span.bbox_min.1],
                bbox_max: [span.bbox_max.0, span.bbox_max.1],
                unsupported_span_px: span.unsupported_span_px,
                ceiling_thickness_px: span.wall_thickness_px,
                vibration_modifier: span.vibration_modifier,
                anchored: false,
                cascade_neighbors: span.cascade_neighbors.clone(),
                damage_actor_id: span.downstream_actor_id,
                structural_integrity_low_emitted: false,
                cave_in_emitted: false,
                m14f_owns_rupture_emit: true,
                cave_in_pending_cascade: false,
                l1_at_tick: None,
                l2_at_tick: None,
                l3_at_tick: None,
                force_integrity_pass_deadline: None,
                crack_decal_l1_enqueued: false,
                crack_decal_l2_enqueued: false,
                crack_decal_l3_enqueued: false,
                structural_warning_banner_emitted: false,
            });
            // Deterministic bulging countdown — fires inside the spec's
            // 30-tick window per VAL-M14F-002 for any span strictly
            // above the lateral-stable floor (12 px).
            let countdown_ticks = if span.unsupported_span_px > cf_terrain::WALL_LATERAL_STABLE_SPAN_PX {
                let over = span.unsupported_span_px - cf_terrain::WALL_LATERAL_STABLE_SPAN_PX;
                let yield_factor = if span.lateral_yield_strength == 0 {
                    1.0_f32
                } else {
                    (50.0_f32 / (span.lateral_yield_strength as f32)).clamp(0.1, 2.0)
                };
                let vib = span.vibration_modifier.max(0.25);
                let base = (24.0_f32 / (over as f32).max(1.0)) * (1.0_f32 / vib) / yield_factor;
                Some(base.clamp(1.0, 25.0) as u32)
            } else {
                None
            };
            m14f_lateral_chunks.insert(
                span.chunk_id,
                M14fLateralChunkState {
                    span_id: span.id.clone(),
                    bbox_min: [span.bbox_min.0, span.bbox_min.1],
                    bbox_max: [span.bbox_max.0, span.bbox_max.1],
                    unsupported_span_px: span.unsupported_span_px,
                    wall_thickness_px: span.wall_thickness_px,
                    lateral_yield_strength: span.lateral_yield_strength,
                    vibration_modifier: span.vibration_modifier,
                    cascade_neighbors: span.cascade_neighbors.clone(),
                    downstream_actor_id: span.downstream_actor_id,
                    topology: span.topology.clone(),
                    sealed_room_pressure_kpa: span.sealed_room_pressure_kpa,
                    bulging_countdown_remaining: countdown_ticks,
                    bulging_emitted: false,
                    bulging_at_tick: None,
                    crack_advanced_emitted: false,
                    crack_advanced_at_tick: None,
                    rupture_emitted: false,
                    rupture_at_tick: None,
                    pixel_carved: false,
                    m14e_composite_cascade_allowed: span.m14e_composite_cascade_allowed,
                },
            );
        }
        let m14g_thermal_zones_init = config.initial_m14g_thermal_zones.clone();
        let m14g_material_contacts_init = config.initial_m14g_material_contacts.clone();
        // **M15 § Active material kernel** — compute the per-tick
        // state inputs from `config` BEFORE the Self construction
        // moves `config` into `Self.config`.
        let m15_initial_heat = build_heat_field_from_atmosphere(&config.initial_atmosphere_cells);
        let m15_ambient_world = infer_ambient_world_from_scenario_id(&config.scenario_id);
        let engine = Self {
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
                // **M14 audit pass 2 (GAP-M4-02)**: track whether the run
                // was aborted via act.player.abort so record_run_finished
                // emits outcome="abort" per M4 spec.
                run_aborted: false,
                hud_last_mission_result: None,
                controls_captured_by: None,
                force_ai_update_this_tick: false,
                pending_alarms: Vec::new(),
                pending_alarms_staging: Vec::new(),
                projectile_spawn_event_ids: BTreeMap::new(),
                projectile_round_kinds: BTreeMap::new(),
                hud_focus_index: None,
                hud_focus_cycle: 0,
                m9_timer_warnings_emitted: BTreeMap::new(),
                m9_concussion_dose: BTreeMap::new(),
                m9_concussion_band: BTreeMap::new(),
                m9_concussion_recovery_lockout_ticks: BTreeMap::new(),
                hud_last_chassis_stage: None,
                hud_last_pilot_state: None,
                last_player_input_event_id: None,
                last_player_status_event_id: None,
                material_overlay_mode: "off".to_string(),
                total_debris_spawned: 0,
                total_carve_events: 0,
                hazard_last_contact_tick: BTreeMap::new(),
                mission_started_event_id: None,
                mission_objective_started_event_ids: BTreeMap::new(),
                last_mission_event_id: None,
                last_ai_state_changed_by_actor: BTreeMap::new(),
                run_started_event_id: None,
                last_reported_dropped_gameplay: 0,
                reload_started_event_id_by_actor: BTreeMap::new(),
                pending_dirty_rects: Vec::new(),
                sustained_unupdated_ticks: 0,
                path_invalidation_version: 0,
                perf_coalesce_samples: Vec::new(),
                perf_coalesce_rects_in_total: 0,
                perf_coalesce_rects_out_total: 0,
                squad,
                weapon_swap_state: BTreeMap::new(),
                m6_last_stamina_emit: BTreeMap::new(),
                m6_last_stealth_band: BTreeMap::new(),
                m6_last_weight_bucket: BTreeMap::new(),
                m6b_last_encumbrance_band: BTreeMap::new(),
                m6_footstep_cooldown: BTreeMap::new(),
                grenade_projectiles: Vec::new(),
                knife_projectiles: Vec::new(),
                m6_last_facing: BTreeMap::new(),
                m6_beacons: Vec::new(),
                m6_dropped_items: Vec::new(),
                m6_next_dropped_item_id: 1,
                m6_charge_misfires: BTreeMap::new(),
                m7_ai_world: m7_ai_world_seed,
                m7b_squad: crate::m7b_squad::M7BSquadWorld::new(),
                camera_state: cf_camera::CameraState::default(),
                photo_mode: cf_photo::PhotoModeState::default(),
                replay_scrub: cf_replay_scrub::ReplayScrubState::default(),
                killcam: cf_killcam::KillcamState::default(),
                debug_state: cf_debug::DebugOverlayState::default(),
                tactical_overlay: cf_squad_ui::TacticalOverlayState::default(),
                plans: BTreeMap::new(),
                tag_state: cf_squad_ui::TagState::default(),
                pie_menu: cf_squad_ui::PieMenuState::closed(),
                localization: cf_localization::LocalizationTable::english_baseline()
                    .unwrap_or_else(|_| cf_localization::LocalizationTable::new("en")),
                game_speed_accumulator: 0,
                multiplayer_session: false,
                // **M4B § "Delta baseline cadence is enforced"** —
                // snapshot emitter state is empty until the first
                // baseline fires at tick 0 (or `delta_baseline_cadence_ticks
                // == 0` disables emission entirely).
                m4b_previous_snapshot: None,
                m4b_last_baseline_event_id: None,
                m4b_last_baseline_tick: None,
                // **M9B**: empty trench-world index. Mutated as the
                // player digs segments + places modules; observe
                // surfaces project the live state.
                trench_world: cf_trench::segment::InMemorySegments::new(),
                trench_next_segment_id: 1,
                m9b_last_cover_state: BTreeMap::new(),
                m9b_trench_doctrine_exposure_ticks: BTreeMap::new(),
                m9b_trench_doctrine_actors,
                cinematic_kernel: None,
                cinematic_seen_set: cf_cinematic::SeenSet::default(),
                cinematic_mixer: cf_audio::CinematicMixer::new(),
                cinematic_takeover: cf_cinematic::CinematicTakeoverSnapshot::default(),
                cinematic_rival_taunt_roll: 0,
                m14b_gravity_overrides,
                m14b_wind_sources,
                m14b_atmos_cells,
                m14b_strat_cells,
                m14b_active_overrides: BTreeMap::new(),
                m14b_transient_wind_ttl: BTreeMap::new(),
                m14b_transient_cells: Vec::new(),
                m14c_scripted_steps,
                m14d_projectile_pair_pool,
                m14d_pair_pass_invocations: 0,
                m14d_last_pair_pass_trace: cf_physics::ProjectilePairPassTrace::default(),
                m14d_replay_intercepts,
                m14d_cram_cooldowns: BTreeMap::new(),
                m14d_schedule_trace: std::collections::VecDeque::with_capacity(120),
                m14e_chunks,
                m14e_pass_invocations: 0,
                m14e_rng_state: m14e_initial_rng_state,
                m14e_actor_knockdown: BTreeMap::new(),
                m14e_last_cave_in_tick: BTreeMap::new(),
                m14e_total_cave_ins: 0,
                m14e_total_beams_placed: 0,
                m14e_total_beams_destroyed: 0,
                m14e_tunnel_collapse_queue: cf_render_2d::tunnel_collapse::TunnelCollapseQueue::new(),
                m14e_tunnel_creak_count: 0,
                m14e_cave_in_thunder_count: 0,
                m14e_actor_resources: BTreeMap::new(),
                m14e_plasma_cutter_active: BTreeMap::new(),
                m14f_lateral_chunks,
                m14f_lateral_pass_invocations: 0,
                m14i_veteran_roster: cf_veteran::VeteranRoster::new(),
                m14i_retirement_narratives:
                    cf_storyteller::retirement_event::RetirementNarrativeRegistry::new(),
                m14g_wound_aging_invocations: 0,
                m14g_wound_registry: None,
                m14g_thermal_zones: m14g_thermal_zones_init,
                m14g_thermal_dwell_ticks: BTreeMap::new(),
                m14g_thermal_emitted_kind: BTreeMap::new(),
                m14g_material_contacts: m14g_material_contacts_init,
                m14g_material_contacts_fired: std::collections::BTreeSet::new(),
                m14f_actor_submerged_tick: BTreeMap::new(),
                m14f_actor_vacuum_tick: BTreeMap::new(),
                m14f_breach_fluid_mass: BTreeMap::new(),
                m14f_breach_pressure_kpa: BTreeMap::new(),
                m14j_ropes: BTreeMap::new(),
                m14j_next_rope_id: 1,
                m14j_zipline_ropes: std::collections::BTreeSet::new(),
                m14j_zipline_speed_by_rider: BTreeMap::new(),
                // **M15 § Active material kernel** wiring. Loaders fall
                // back to hardcoded defaults when the content JSON
                // files aren't present (e.g., headless replay-verifier
                // without content/ on the path).
                material_kernel: cf_material::MaterialKernel::new().with_parallel(true),
                reaction_registry: cf_material::ReactionRegistry::load_default_or_hardcoded(),
                phase_registry: cf_material::PhaseRegistry::load_default_or_hardcoded(),
                heat_field: m15_initial_heat,
                prev_heat_field: None,
                precipitation_cycle: cf_material::PrecipitationCycle::new(m15_ambient_world),
                precipitation_config: cf_material::PrecipitationConfig::load_default_or_baseline(),
            }),
            recorder,
            current_tick,
            started_at,
            started_instant,
            run_bundle_dir,
            audio_plugin: std::sync::Mutex::new(Box::new(cf_audio::NullAudioPlugin)),
            last_save_cache: Arc::new(crate::m4b_save::LastSaveCache::new()),
        };
        // **M4B § "Tournament-mode chain anchor"** — enable chain mode on
        // the recorder when the config opts in. Must happen AFTER the
        // recorder is constructed but BEFORE any tick fires so the very
        // first event in the bundle gets a chain hash.
        if engine.config.ledger_chain_enabled {
            engine.recorder.enable_chain_mode(engine.config.seed);
        }
        engine
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
                // **M4 § system.run_started carries protocol_version +
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
        // **M4**: stash the run_started event id so downstream events that
        // have no other cause (e.g. ai.tactic_chosen with no fresh
        // perception signal) can chain to it as a root.
        if let Ok(mut s) = self.state.write() {
            s.run_started_event_id = Some(started_id.clone());
        }
        self.emit_initial_snapshots(tick, sim_time_ms, Some(&started_id));
        self.emit_category_baseline(tick, sim_time_ms, &started_id);
        // **M4B § "Delta baseline cadence is enforced"** — emit the tick-0
        // baseline as part of run_started so the cadence is anchored from
        // the very first tick. drive_tick() advances starts at tick 1, so
        // tick 0 itself never goes through emit_m4b_snapshot_for_tick.
        self.emit_m4b_snapshot_for_tick(tick);
        // **M4 § ux first_event_type**: emit one `ux.banner_raised` at run
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
    fn m8_effective_sim_speed_pct_default_is_off_no_pie_menu() {
        let settings = Settings::default();
        let pie = cf_squad_ui::PieMenuState::closed();
        assert_eq!(effective_sim_speed_pct(&settings, &pie, false), 100);
    }

    #[test]
    fn m8_effective_sim_speed_pct_slowdown75_alone() {
        let mut settings = Settings::default();
        settings.game_speed_assist = crate::settings::GameSpeedAssist::Slowdown75;
        let pie = cf_squad_ui::PieMenuState::closed();
        assert_eq!(effective_sim_speed_pct(&settings, &pie, false), 75);
    }

    #[test]
    fn m8_effective_sim_speed_pct_slowdown25_alone() {
        let mut settings = Settings::default();
        settings.game_speed_assist = crate::settings::GameSpeedAssist::Slowdown25;
        let pie = cf_squad_ui::PieMenuState::closed();
        assert_eq!(effective_sim_speed_pct(&settings, &pie, false), 25);
    }

    #[test]
    fn m8_effective_sim_speed_pct_full_pause_alone() {
        let mut settings = Settings::default();
        settings.game_speed_assist = crate::settings::GameSpeedAssist::FullPause;
        let pie = cf_squad_ui::PieMenuState::closed();
        assert_eq!(effective_sim_speed_pct(&settings, &pie, false), 0);
    }

    #[test]
    fn m8_effective_sim_speed_pct_pie_menu_open_stacks_with_assist_most_restrictive_wins() {
        let mut settings = Settings::default();
        settings.game_speed_assist = crate::settings::GameSpeedAssist::Slowdown75;
        let mut pie = cf_squad_ui::PieMenuState::closed();
        pie.open(cf_squad_ui::PieMenuTarget::Void, false, 1);
        assert_eq!(pie.slowdown_factor_pct, cf_squad_ui::SINGLE_PLAYER_SLOWDOWN_PCT);
        assert_eq!(
            effective_sim_speed_pct(&settings, &pie, false),
            cf_squad_ui::SINGLE_PLAYER_SLOWDOWN_PCT,
            "pie menu's 20% slowdown is more restrictive than game_speed_assist's 75%",
        );
    }

    #[test]
    fn m8_effective_sim_speed_pct_multiplayer_ignores_assist_but_honors_pie_menu() {
        let mut settings = Settings::default();
        settings.game_speed_assist = crate::settings::GameSpeedAssist::FullPause;
        let pie = cf_squad_ui::PieMenuState::closed();
        assert_eq!(
            effective_sim_speed_pct(&settings, &pie, true),
            100,
            "multiplayer must ignore game_speed_assist (single-player only)",
        );
        let mut mp_pie = cf_squad_ui::PieMenuState::closed();
        mp_pie.open(cf_squad_ui::PieMenuTarget::Void, true, 1);
        assert_eq!(mp_pie.slowdown_factor_pct, 100);
        assert_eq!(effective_sim_speed_pct(&settings, &mp_pie, true), 100);
    }

    #[test]
    fn m8_speed_pct_75_skips_one_in_four_ticks_via_accumulator() {
        let mut acc: u16 = 0;
        let pct: u16 = 75;
        let mut advances = 0;
        let mut skips = 0;
        for _ in 0..4 {
            acc = acc.saturating_add(pct);
            if acc >= 100 {
                acc -= 100;
                advances += 1;
            } else {
                skips += 1;
            }
        }
        assert_eq!(advances, 3, "Slowdown75: 3 advances per 4 wall ticks");
        assert_eq!(skips, 1, "Slowdown75: 1 skip per 4 wall ticks");
    }

    #[test]
    fn m8_speed_pct_25_skips_three_in_four_ticks_via_accumulator() {
        let mut acc: u16 = 0;
        let pct: u16 = 25;
        let mut advances = 0;
        let mut skips = 0;
        for _ in 0..4 {
            acc = acc.saturating_add(pct);
            if acc >= 100 {
                acc -= 100;
                advances += 1;
            } else {
                skips += 1;
            }
        }
        assert_eq!(advances, 1, "Slowdown25: 1 advance per 4 wall ticks");
        assert_eq!(skips, 3, "Slowdown25: 3 skips per 4 wall ticks");
    }

    #[test]
    fn m8_speed_pct_20_pie_menu_skips_four_in_five_ticks_via_accumulator() {
        let mut acc: u16 = 0;
        let pct: u16 = u16::from(cf_squad_ui::SINGLE_PLAYER_SLOWDOWN_PCT);
        let mut advances = 0;
        for _ in 0..5 {
            acc = acc.saturating_add(pct);
            if acc >= 100 {
                acc -= 100;
                advances += 1;
            }
        }
        assert_eq!(advances, 1, "Pie menu 20%: 1 advance per 5 wall ticks");
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
    fn m8_drive_tick_full_pause_returns_none_without_advancing_clock() {
        let scenario_path = write_test_scenario();
        let mut cfg = load_test_scenario_and_config(scenario_path);
        cfg.settings.game_speed_assist = crate::settings::GameSpeedAssist::FullPause;
        cfg.run_mode = "test-game-speed-full-pause".to_string();
        let engine = M0Engine::new(cfg);
        let start = engine.current_tick();
        for _ in 0..32 {
            assert!(
                engine.drive_tick().is_none(),
                "FullPause must always return None from drive_tick",
            );
        }
        assert_eq!(engine.current_tick(), start, "FullPause must not advance the clock",);
    }

    #[test]
    fn m8_drive_tick_slowdown75_advances_three_in_four_ticks() {
        let scenario_path = write_test_scenario();
        let mut cfg = load_test_scenario_and_config(scenario_path);
        cfg.settings.game_speed_assist = crate::settings::GameSpeedAssist::Slowdown75;
        cfg.run_mode = "test-game-speed-slowdown75".to_string();
        let engine = M0Engine::new(cfg);
        let mut advances = 0;
        let mut skips = 0;
        for _ in 0..400 {
            if engine.drive_tick().is_some() {
                advances += 1;
            } else {
                skips += 1;
            }
        }
        assert_eq!(advances, 300, "Slowdown75: 3 in 4 ticks advance (=300 of 400)");
        assert_eq!(skips, 100, "Slowdown75: 1 in 4 ticks skipped (=100 of 400)");
    }

    #[test]
    fn m8_drive_tick_slowdown25_advances_one_in_four_ticks() {
        let scenario_path = write_test_scenario();
        let mut cfg = load_test_scenario_and_config(scenario_path);
        cfg.settings.game_speed_assist = crate::settings::GameSpeedAssist::Slowdown25;
        cfg.run_mode = "test-game-speed-slowdown25".to_string();
        let engine = M0Engine::new(cfg);
        let mut advances = 0;
        let mut skips = 0;
        for _ in 0..400 {
            if engine.drive_tick().is_some() {
                advances += 1;
            } else {
                skips += 1;
            }
        }
        assert_eq!(advances, 100, "Slowdown25: 1 in 4 ticks advance (=100 of 400)");
        assert_eq!(skips, 300, "Slowdown25: 3 in 4 ticks skipped (=300 of 400)");
    }

    #[test]
    fn m8_drive_tick_off_advances_every_tick() {
        let scenario_path = write_test_scenario();
        let mut cfg = load_test_scenario_and_config(scenario_path);
        cfg.settings.game_speed_assist = crate::settings::GameSpeedAssist::Off;
        cfg.run_mode = "test-game-speed-off".to_string();
        let engine = M0Engine::new(cfg);
        for _ in 0..64 {
            assert!(
                engine.drive_tick().is_some(),
                "game_speed_assist=Off must always advance",
            );
        }
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
                changes: Box::new(SettingsPatch {
                    ui_scale: Some(2.0),
                    high_contrast: Some(true),
                    captions: Some(false),
                    ..SettingsPatch::default()
                }),
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
                changes: Box::new(SettingsPatch {
                    ui_scale: Some(0.01),
                    ..SettingsPatch::default()
                }),
            })
            .await;
        let low_settings = engine.settings_snapshot().await;
        assert!((low_settings.ui_scale - crate::settings::UI_SCALE_MIN).abs() < f32::EPSILON);
        let low_frame = engine.snapshot(None).await;
        assert!((low_frame.accessibility.ui_scale_applied - crate::settings::UI_SCALE_MIN).abs() < f32::EPSILON);

        let _ = engine
            .dispatch(ControlCommand::SettingsSet {
                changes: Box::new(SettingsPatch {
                    ui_scale: Some(99.0),
                    ..SettingsPatch::default()
                }),
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
                ammo_kind: None,
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
                ammo_kind: None,
                source: IntentSource::Human,
            })
            .await;
        assert_eq!(press.status, crate::state::ControlEnvelopeStatus::Accepted);

        let release = engine
            .dispatch(ControlCommand::ActPlayerFire {
                pressed: false,
                ammo_kind: None,
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
                ammo_kind: None,
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
        // Actor should not accept input. CCCP Actor.cpp:1229 — HP=0 enters
        // DYING (the death animation dwell window). Either DYING or DEAD
        // refuses input.
        let frame = engine.snapshot(None).await;
        let player = frame
            .actors
            .iter()
            .find(|a| Some(a.id) == frame.player_actor_id)
            .unwrap();
        assert!(
            player.status == "dying" || player.status == "dead",
            "expected dying or dead, got {}",
            player.status
        );
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
                ammo_kind: None,
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
            assert_eq!(result.reason.as_deref(), Some("non_finite"));
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
            assert_eq!(result.reason.as_deref(), Some("non_finite"));
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
                    ammo_kind: None,
                    source: IntentSource::Cfctl,
                })
                .await;
            // Drive enough ticks for the fired projectile to reach the dummy at x=900
            // before the next shot (player at x=200, projectile speed 1200 unit/s ≈ 20
            // unit/tick at 60 Hz → 35 ticks to cross 700 units).
            for _ in 0..fire_interval_ticks.max(35) {
                engine.drive_tick();
            }
            // Release the trigger so the Semi rifle latch clears and the next
            // pressed:true can fire (M1 default rifle is Semi).
            let _ = engine
                .dispatch(ControlCommand::ActPlayerFire {
                    pressed: false,
                    ammo_kind: None,
                    source: IntentSource::Cfctl,
                })
                .await;
            engine.drive_tick();
        }
        let events = engine.recorder().snapshot_events();
        // CCCP Actor.cpp:1229 — HP=0 enters DYING (not DEAD); the DEAD
        // transition fires later when the dwell elapses. Accept either as
        // proof the projectile_hit cause-chain reached the terminal status.
        let kill_event = events.iter().find(|e| {
            e.category == "actor"
                && e.event_type == "actor_status_changed"
                && (e.payload["new_status"] == "dying" || e.payload["new_status"] == "dead")
                && e.payload["cause"] == "projectile_hit"
        });
        assert!(
            kill_event.is_some(),
            "expected a projectile_hit-caused dying/dead status transition; got events: {:?}",
            events
                .iter()
                .filter(|e| e.event_type == "actor_status_changed")
                .map(|e| e.payload.clone())
                .collect::<Vec<_>>()
        );
    }

    /// **Enhancement D2**: in-process cross-run determinism. Drive the engine
    /// twice with the same seed + same script and assert the final
    /// determinism checksum hex strings match byte-for-byte.
    #[tokio::test]
    async fn cross_run_determinism_same_seed_same_final_checksum() {
        async fn drive_run() -> Option<String> {
            let path = write_m1_scenario();
            let config = load_m1_test_config(path);
            let engine = M0Engine::new(config);
            engine.record_run_started();
            // Settle to ground.
            for _ in 0..6 {
                engine.drive_tick();
            }
            let _ = engine
                .dispatch(ControlCommand::ActPlayerAim {
                    x: 1.0,
                    y: 0.0,
                    source: IntentSource::Cfctl,
                })
                .await;
            // Fire/release a handful of shots to exercise the cause chain.
            for _ in 0..3 {
                let _ = engine
                    .dispatch(ControlCommand::ActPlayerFire {
                        pressed: true,
                        ammo_kind: None,
                        source: IntentSource::Cfctl,
                    })
                    .await;
                for _ in 0..12 {
                    engine.drive_tick();
                }
                let _ = engine
                    .dispatch(ControlCommand::ActPlayerFire {
                        pressed: false,
                        ammo_kind: None,
                        source: IntentSource::Cfctl,
                    })
                    .await;
                engine.drive_tick();
            }
            for _ in 0..120 {
                engine.drive_tick();
            }
            engine.recorder().final_checksum_hex()
        }
        let cs_a = drive_run().await.expect("run a produced a checksum");
        let cs_b = drive_run().await.expect("run b produced a checksum");
        assert_eq!(
            cs_a, cs_b,
            "cross-run determinism: same seed + same script must produce byte-identical final sim checksum"
        );
    }

    /// **Gap C4**: walk parent_event_id from `actor.inventory_dropped` back to
    /// the root `input.intent_received`. Every link must resolve to a real
    /// recorded event id (no `ParentMissingFromBundle`). The expected chain:
    ///   inventory_dropped -> status_changed(DYING) -> projectile_hit
    ///     -> projectile_spawned -> weapon_fired -> input.intent_received
    #[tokio::test]
    async fn cause_chain_walks_from_inventory_dropped_to_intent() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();
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
        let fire_interval_ticks = cf_equipment::rifle_preset(cf_equipment::RIFLE_M1_DEFAULT_ID)
            .unwrap()
            .fire_interval_ticks(60) as usize;
        // Kill the dummy (100 HP / 12 dmg => 9 hits + buffer).
        for _ in 0..12 {
            let _ = engine
                .dispatch(ControlCommand::ActPlayerFire {
                    pressed: true,
                    ammo_kind: None,
                    source: IntentSource::Cfctl,
                })
                .await;
            for _ in 0..fire_interval_ticks.max(35) {
                engine.drive_tick();
            }
            let _ = engine
                .dispatch(ControlCommand::ActPlayerFire {
                    pressed: false,
                    ammo_kind: None,
                    source: IntentSource::Cfctl,
                })
                .await;
            engine.drive_tick();
        }
        // Let the DYING dwell elapse so inventory_dropped + DEAD chain emit.
        for _ in 0..120 {
            engine.drive_tick();
        }
        let events = engine.recorder().snapshot_events();
        // Build id -> event lookup for the walk.
        let by_id: std::collections::BTreeMap<String, &cf_replay::Event> =
            events.iter().map(|e| (e.event_id.clone(), e)).collect();
        // Find the inventory_dropped for the dummy (actor_id 2).
        let drop_event = events.iter().find(|e| {
            e.category == "actor"
                && e.event_type == "inventory_dropped"
                && e.payload.get("actor").and_then(|v| v.as_u64()) == Some(2)
        });
        // The dummy carries no rifle in m1_actor_range (its inventory.rifle: None),
        // so the inventory_dropped event may not fire (label="empty"). In that
        // case the chain test still has value via status_changed(DYING).
        let chain_root = drop_event.or_else(|| {
            events.iter().find(|e| {
                e.category == "actor"
                    && e.event_type == "actor_status_changed"
                    && e.payload.get("new_status").and_then(|v| v.as_str()) == Some("dying")
                    && e.payload.get("actor").and_then(|v| v.as_u64()) == Some(2)
            })
        });
        let chain_root = chain_root.expect("must find inventory_dropped OR status_changed(DYING) for actor 2");
        // Walk the parent_event_id chain.
        let mut chain_types: Vec<String> = Vec::new();
        let mut current = chain_root;
        chain_types.push(format!("{}.{}", current.category, current.event_type));
        let mut walked = 0;
        while let Some(parent_id) = current.parent_event_id.clone() {
            walked += 1;
            assert!(walked < 50, "chain walk runaway (events={:?})", chain_types);
            let parent = by_id
                .get(&parent_id)
                .unwrap_or_else(|| panic!("ParentMissingFromBundle: parent_id={parent_id} not in run"));
            chain_types.push(format!("{}.{}", parent.category, parent.event_type));
            current = parent;
        }
        // The walk must terminate at an input.intent_received root.
        let terminal = chain_types.last().expect("chain must have at least one link").clone();
        assert!(
            terminal == "input.intent_received",
            "cause chain must terminate at input.intent_received; got chain: {:?}",
            chain_types
        );
        // The chain must include projectile_hit and weapon_fired links.
        assert!(
            chain_types.iter().any(|s| s == "combat.projectile_hit"),
            "chain missing combat.projectile_hit: {chain_types:?}",
        );
        assert!(
            chain_types.iter().any(|s| s == "equipment.weapon_fired"),
            "chain missing equipment.weapon_fired: {chain_types:?}",
        );
    }

    // --- M3 re-open (2026-05-13): coalesce-logic regression tests ---

    #[test]
    fn rects_touch_or_overlap_detects_shared_edge() {
        // Two CHUNK_SIZE × CHUNK_SIZE rects sitting edge-to-edge along x.
        // Chunk (0,0) occupies [0,0..256] and chunk (1,0) occupies [256,0..512].
        // The shared edge at x=256 means the AABBs touch.
        let a_min = [0i64, 0i64];
        let a_max = [256i64, 256i64];
        let b_min = [256i64, 0i64];
        let b_max = [512i64, 256i64];
        assert!(rects_touch_or_overlap(a_min, a_max, b_min, b_max));
    }

    #[test]
    fn rects_touch_or_overlap_detects_diagonal_neighbor() {
        // Corner-touching rects (diagonal). a.max == b.min for both axes.
        // The greedy coalescer treats this as touching so the union covers
        // both chunks in one pass.
        let a_min = [0i64, 0i64];
        let a_max = [256i64, 256i64];
        let b_min = [256i64, 256i64];
        let b_max = [512i64, 512i64];
        assert!(rects_touch_or_overlap(a_min, a_max, b_min, b_max));
    }

    #[test]
    fn rects_touch_or_overlap_rejects_disjoint() {
        // A gap of 10 pixels between rects → no overlap → coalesce keeps
        // them as separate batch entries.
        let a_min = [0i64, 0i64];
        let a_max = [256i64, 256i64];
        let b_min = [266i64, 0i64];
        let b_max = [522i64, 256i64];
        assert!(!rects_touch_or_overlap(a_min, a_max, b_min, b_max));
    }

    #[test]
    fn rects_touch_or_overlap_detects_interior_overlap() {
        // A rect fully contained inside another.
        let a_min = [0i64, 0i64];
        let a_max = [256i64, 256i64];
        let b_min = [100i64, 100i64];
        let b_max = [120i64, 120i64];
        assert!(rects_touch_or_overlap(a_min, a_max, b_min, b_max));
    }

    #[tokio::test]
    async fn m6_sprint_drains_stamina_and_auto_cancels() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();

        let result = engine
            .dispatch(ControlCommand::ActM6 {
                action: crate::m6_actions::M6Action::Sprint { active: true },
                source: IntentSource::Cfctl,
            })
            .await;
        assert_eq!(result.status, crate::state::ControlEnvelopeStatus::Accepted);

        for _ in 0..(5 * 60 + 2) {
            engine.drive_tick();
        }

        let events = engine.recorder().snapshot_events();
        assert!(
            events
                .iter()
                .any(|e| e.category == "actor" && e.event_type == "stamina_changed"),
            "actor.stamina_changed must be emitted as stamina drains"
        );
        let state = engine.state.read().expect("engine state poisoned");
        let sim = state.actor_state.as_ref().expect("actor sim present");
        let actor = sim.world.actors.get(&ActorId(1)).expect("player actor present");
        assert!(
            actor.stamina.current <= 0.01,
            "after 5s sprint stamina must drain to ~0: {}",
            actor.stamina.current
        );
        assert!(!actor.sprint_active, "sprint must auto-cancel at zero stamina");
    }

    #[tokio::test]
    async fn m6_cinematic_slide_transitions_back_to_crouch() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();

        engine
            .dispatch(ControlCommand::ActM6 {
                action: crate::m6_actions::M6Action::Sprint { active: true },
                source: IntentSource::Cfctl,
            })
            .await;
        engine
            .dispatch(ControlCommand::ActM6 {
                action: crate::m6_actions::M6Action::Slide,
                source: IntentSource::Cfctl,
            })
            .await;

        for _ in 0..40 {
            engine.drive_tick();
        }

        let events = engine.recorder().snapshot_events();
        let stance_changed = events
            .iter()
            .find(|e| {
                e.category == "actor"
                    && e.event_type == "stance_changed"
                    && e.payload
                        .get("cause")
                        .and_then(|v| v.as_str())
                        .map(|s| s == "cinematic_complete")
                        .unwrap_or(false)
            })
            .expect("actor.stance_changed must fire when slide finishes");
        let to_stance = stance_changed
            .payload
            .get("to_stance")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert_eq!(to_stance, "crouching", "slide must transition to crouch");

        let state = engine.state.read().expect("engine state poisoned");
        let sim = state.actor_state.as_ref().expect("actor sim present");
        let actor = sim.world.actors.get(&ActorId(1)).expect("player actor present");
        assert_eq!(actor.cinematic_ticks_remaining, 0);
        assert!(actor.cinematic_kind.is_none());
    }

    #[tokio::test]
    async fn m6_lean_angle_approaches_target_over_time() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();

        engine
            .dispatch(ControlCommand::ActM6 {
                action: crate::m6_actions::M6Action::Lean { direction: 1.0 },
                source: IntentSource::Cfctl,
            })
            .await;

        for _ in 0..120 {
            engine.drive_tick();
        }

        let state = engine.state.read().expect("engine state poisoned");
        let sim = state.actor_state.as_ref().expect("actor sim present");
        let actor = sim.world.actors.get(&ActorId(1)).expect("player actor present");
        assert!(
            actor.lean_state.angle_degrees >= 40.0,
            "lean angle must approach +45° (got {})",
            actor.lean_state.angle_degrees
        );
    }

    #[tokio::test]
    async fn m6_weapon_swap_completes_after_300ms() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();

        engine
            .dispatch(ControlCommand::ActM6 {
                action: crate::m6_actions::M6Action::WeaponSwap { slot: 1 },
                source: IntentSource::Cfctl,
            })
            .await;

        for _ in 0..30 {
            engine.drive_tick();
        }

        let events = engine.recorder().snapshot_events();
        assert!(
            events
                .iter()
                .any(|e| e.category == "equipment" && e.event_type == "weapon_swap_started"),
            "weapon_swap_started must fire when swap is requested"
        );
        assert!(
            events
                .iter()
                .any(|e| e.category == "equipment" && e.event_type == "weapon_swap_completed"),
            "weapon_swap_completed must fire after 300ms tick path: {:?}",
            events
                .iter()
                .map(|e| (e.category.clone(), e.event_type.clone()))
                .collect::<Vec<_>>()
        );
    }

    /// **M6B § Acceptance: Container nesting depth-limited**.
    /// Full engine round trip: a chest at depth-1 holding a crate at
    /// depth-2; attempting to nest a third container into the crate
    /// rejects with the spec-locked `max_depth_exceeded` reason and
    /// emits `actor.action_rejected` (no `inventory.container_nested`
    /// fires for the rejection).
    #[tokio::test]
    async fn m6b_nest_container_engine_rejects_max_depth() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();

        // Seed the player actor's grid with a chest (top-level) +
        // crate (nested into chest at depth 2).
        let chest_id;
        let crate_id;
        {
            let mut state = engine.state.write().unwrap();
            let player_id = state.player_actor.unwrap();
            let actor = state
                .actor_state
                .as_mut()
                .unwrap()
                .world
                .actors
                .get_mut(&player_id)
                .unwrap();
            actor.inventory_grid_attach();
            let grid = actor.inventory_grid_mut().unwrap();
            chest_id = grid.add_top_level("chest", 1, 0.0);
            crate_id = grid.try_nest_container(chest_id, "crate").unwrap();
        }
        engine.drive_tick();

        // Step 1: nest another container (crate) into the crate. This
        // would land at depth 3 = MAX_CONTAINER_NEST_DEPTH+1; the
        // dispatch returns Rejected with the locked reason.
        let result_rejected = engine
            .dispatch(ControlCommand::ActM6 {
                action: crate::m6_actions::M6Action::NestContainer {
                    parent_instance_id: crate_id,
                    child_item_id: "crate".to_string(),
                },
                source: IntentSource::Cfctl,
            })
            .await;
        assert_eq!(result_rejected.status, crate::state::ControlEnvelopeStatus::Rejected);
        assert_eq!(
            result_rejected.reason.as_deref(),
            Some(cf_equipment::MAX_DEPTH_EXCEEDED)
        );

        // Step 2: nest a medkit into the crate. Non-container child at
        // depth 3 is allowed (depth cap only constrains containers).
        let result_accepted = engine
            .dispatch(ControlCommand::ActM6 {
                action: crate::m6_actions::M6Action::NestContainer {
                    parent_instance_id: crate_id,
                    child_item_id: "medkit".to_string(),
                },
                source: IntentSource::Cfctl,
            })
            .await;
        assert_eq!(result_accepted.status, crate::state::ControlEnvelopeStatus::Accepted);

        engine.drive_tick();

        let events = engine.recorder().snapshot_events();
        // Rejection emits actor.action_rejected with the locked reason.
        let rejected = events
            .iter()
            .find(|e| {
                e.category == "actor"
                    && e.event_type == "action_rejected"
                    && e.payload
                        .get("reason")
                        .and_then(|r| r.as_str())
                        .map(|s| s == cf_equipment::MAX_DEPTH_EXCEEDED)
                        .unwrap_or(false)
            })
            .expect(
                "expected actor.action_rejected with reason 'max_depth_exceeded'; \
                 saw events: see test output",
            );
        assert_eq!(
            rejected.payload.get("action").and_then(|v| v.as_str()),
            Some("act.player.nest_container")
        );

        // Success path emits inventory.container_nested with depth=3.
        let nested = events
            .iter()
            .find(|e| e.category == "inventory" && e.event_type == "container_nested")
            .expect("expected inventory.container_nested for successful medkit nest");
        let depth = nested.payload.get("depth").and_then(|v| v.as_u64()).unwrap_or(0);
        assert_eq!(depth, 3, "medkit nested at depth 3 (inside crate)");
        assert_eq!(
            nested.payload.get("child_item_id").and_then(|v| v.as_str()),
            Some("medkit")
        );
        assert_eq!(
            nested.payload.get("child_is_container").and_then(|v| v.as_bool()),
            Some(false)
        );
    }

    /// **M6B § Acceptance: Encumbrance band transition fires the
    /// `inventory.encumbrance_threshold_crossed` event**.
    #[tokio::test]
    async fn m6b_encumbrance_band_transition_fires_event() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();

        // Seed the player with 15 rifles → Heavy band (52.5 / 50 ratio).
        {
            let mut state = engine.state.write().unwrap();
            let player_id = state.player_actor.unwrap();
            let actor = state
                .actor_state
                .as_mut()
                .unwrap()
                .world
                .actors
                .get_mut(&player_id)
                .unwrap();
            actor.inventory_grid_attach();
            let grid = actor.inventory_grid_mut().unwrap();
            for _ in 0..15 {
                grid.add_top_level("rifle_m1", 1, 0.0);
            }
        }
        // The tick recomputes encumbrance + detects band change.
        engine.drive_tick();
        engine.drive_tick();

        let events = engine.recorder().snapshot_events();
        let band_crossed = events
            .iter()
            .find(|e| e.category == "inventory" && e.event_type == "encumbrance_threshold_crossed")
            .expect("encumbrance_threshold_crossed must fire when band changes");
        assert_eq!(
            band_crossed.payload.get("to_band").and_then(|v| v.as_str()),
            Some("heavy")
        );
        let walk_mult = band_crossed
            .payload
            .get("walk_speed_multiplier")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        assert!((walk_mult - 0.5).abs() < 0.01, "walk_speed_multiplier must be ~0.5");
    }

    /// **M6B § Acceptance: Item picked up via the engine adds canonical mass
    /// to the inventory grid AND emits `equipment.item_picked_up_with_mass`**.
    #[tokio::test]
    async fn m6b_pickup_emits_mass_aware_event_and_updates_grid() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();

        // Spawn a dropped rifle near the player.
        let player_pos = {
            let state = engine.state.read().unwrap();
            let player_id = state.player_actor.unwrap();
            state
                .actor_state
                .as_ref()
                .unwrap()
                .world
                .actors
                .get(&player_id)
                .unwrap()
                .position
        };
        // Drop the held rifle so it lands in the world.
        engine
            .dispatch(ControlCommand::ActM6 {
                action: crate::m6_actions::M6Action::DropItem { slot: Some(0) },
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        // Push the dropped item next to the player so pickup is in range.
        {
            let mut state = engine.state.write().unwrap();
            for item in state.m6_dropped_items.iter_mut() {
                item.position = player_pos;
            }
        }
        engine.drive_tick();
        // Now pick it up.
        engine
            .dispatch(ControlCommand::ActM6 {
                action: crate::m6_actions::M6Action::Pickup,
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        engine.drive_tick();

        let events = engine.recorder().snapshot_events();
        // Both the legacy event AND the mass-aware sibling MUST fire.
        let legacy = events
            .iter()
            .filter(|e| e.category == "equipment" && e.event_type == "item_picked_up")
            .count();
        let mass_aware = events
            .iter()
            .filter(|e| e.category == "equipment" && e.event_type == "item_picked_up_with_mass")
            .count();
        assert!(legacy >= 1, "legacy equipment.item_picked_up must still fire");
        assert!(
            mass_aware >= 1,
            "M6B equipment.item_picked_up_with_mass must fire alongside legacy event"
        );
        // The mass_aware event carries canonical mass + dimensions from
        // the ItemSpec registry (mass=3.5, dims=2×4 per rifle_m1_default
        // → falls back to legacy weight when not in registry; rifle_m1_default
        // IS in the registry so we expect 3.5).
        let mass_event = events
            .iter()
            .find(|e| e.category == "equipment" && e.event_type == "item_picked_up_with_mass")
            .unwrap();
        let mass_kg = mass_event
            .payload
            .get("mass_kg")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        assert!(
            (mass_kg - 3.5).abs() < 0.01,
            "mass_kg from registry must be 3.5 (got {mass_kg})"
        );
        let total = mass_event
            .payload
            .get("inventory_total_mass_kg")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        assert!(total > 0.0, "inventory_total_mass_kg must be > 0 after pickup");
    }
}
