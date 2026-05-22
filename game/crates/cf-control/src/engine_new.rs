//! M0Engine::new constructor — extracted from engine.rs.

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

        // reactive guard the scenario declared so the 5-layer thinking
        // stack ticks alongside the M2 FSM from tick 0. Built before the
        // EngineMutable move below so the borrow checker is happy.
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
                // snapshot emitter state is empty until the first
                // baseline fires at tick 0 (or `delta_baseline_cadence_ticks
                // == 0` disables emission entirely).
                m4b_previous_snapshot: None,
                m4b_last_baseline_event_id: None,
                m4b_last_baseline_tick: None,
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
        // the recorder when the config opts in. Must happen AFTER the
        // recorder is constructed but BEFORE any tick fires so the very
        // first event in the bundle gets a chain hash.
        if engine.config.ledger_chain_enabled {
            engine.recorder.enable_chain_mode(engine.config.seed);
        }
        engine
    }
}
