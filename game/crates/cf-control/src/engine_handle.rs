//! EngineHandle trait impl for M0Engine.
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

#[async_trait]
impl EngineHandle for M0Engine {
    /// snapshot the engine has tracked since startup. Updated by F5/F9 +
    /// cfctl save subcommands via [`crate::m4b_save::LastSaveCache`].
    async fn observe_save_last(&self) -> serde_json::Value {
        serde_json::to_value(self.last_save_cache.snapshot()).unwrap_or(serde_json::Value::Null)
    }

    /// Public accessor used by cf-app + cfctl save subcommands to dispatch
    /// `system.save_completed` / `system.save_loaded` / `system.save_migrated`
    /// events with full context.
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
                    let mut chassis_view = a.chassis_view().as_ref().map(crate::state::ChassisView::from);
                    if let Some(cv) = chassis_view.as_mut() {
                        // actor's status. Settings.reduced_motion is read
                        // from the engine state at snapshot time.
                        cv.ragdoll_state = match a.status {
                            cf_actor::Status::Dying => {
                                if state.settings.reduced_motion {
                                    "static_collapse".to_string()
                                } else {
                                    "activating".to_string()
                                }
                            }
                            cf_actor::Status::Dead => "active".to_string(),
                            _ => "animated".to_string(),
                        };
                    }
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
                        // M1 re-audit (2026-05-13): mirror cf_actor::ActorObservation.inventory
                        inventory: a.inventory.items.iter().map(|i| i.label().to_string()).collect(),
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
                        stability: a.stability,
                        stability_recovery_rate: a.stability_recovery_rate,
                        sharp_aim_progress: a.sharp_aim_progress,
                        recoil_accumulator: a.recoil_accumulator,
                        knockdown_ticks_remaining: a.knockdown_ticks_remaining,
                        dying_dwell_ticks_remaining: a.dying_dwell_ticks_remaining,
                        mission_critical: a.mission_critical,
                        bloom_factor: a.bloom_factor,
                        mass_kg: a.mass_kg,
                        // against the live engine trench-world index. On
                        // open ground the value is "Exposed" per
                        // VAL-M9B-COVERMATRIX-001; standing in a placed
                        // segment reflects the (stance × variant) table.
                        cover_state: a.cover_state(&state.trench_world).as_str().to_string(),
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
            .map(|g| {
                let position = state
                    .actor_state
                    .as_ref()
                    .and_then(|sim| sim.world.actors.get(&g.actor).map(|a| [a.position.x, a.position.y]));
                let intent_label = ai_intent_label(g);
                crate::state::EnemyView {
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
                    position,
                    intent_label,
                }
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
            chunk_count: t.allocated_chunk_count() as u32,
            material_counts: t.material_counts(),
            material_distribution: t
                .material_counts()
                .into_iter()
                .filter_map(|(name, count)| cf_terrain::material_id_from_name(&name).map(|id| (id, count)))
                .collect(),
            current_overlay_mode: state.material_overlay_mode.clone(),
            total_carve_events: state.total_carve_events,
            total_debris_spawned: state.total_debris_spawned,
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
        // atmosphere projection (pressure + temperature + per-gas
        // mole fractions). Empty for scenarios that don't declare
        // `atmosphere_cells` in the manifest.
        let cells: Vec<crate::state::AtmosphericCellView> = state
            .m14b_atmos_cells
            .iter()
            .map(|c| {
                let strat = state.m14b_strat_cells.iter().find(|s| s.cell_id == c.id);
                let gases: Vec<crate::state::AtmosphericCellGasView> = strat
                    .map(|s| {
                        s.fractions
                            .iter()
                            .map(|(g, f)| crate::state::AtmosphericCellGasView {
                                gas: g.label().to_string(),
                                fraction: *f,
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let column_id = strat.map(|s| s.column_id).unwrap_or(c.id);
                crate::state::AtmosphericCellView {
                    id: c.id,
                    column_id,
                    min: c.min,
                    max: c.max,
                    pressure_kpa: c.pressure_kpa,
                    temp_k: c.temp_k,
                    gases,
                }
            })
            .collect();
        let gravity_vectors: Vec<crate::state::ActorGravityView> = state
            .actor_state
            .as_ref()
            .map(|sim| {
                let base = cf_physics::GravityField::Uniform(sim.world.gravity);
                sim.world
                    .actors
                    .values()
                    .map(|a| {
                        let pos = [a.position.x, a.position.y];
                        let result = cf_physics::apply_overrides(
                            base.sample(pos),
                            pos,
                            Some(a.id.0),
                            &state.m14b_gravity_overrides,
                        );
                        crate::state::ActorGravityView {
                            actor_id: a.id.0,
                            magnitude: result.gravity.magnitude,
                            direction: result.gravity.direction,
                            active_override_ids: result.active_ids,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        // project the live rope world + mount pairings.
        let ropes_view: Vec<crate::state::RopeView> = state
            .m14j_ropes
            .iter()
            .map(|(rid, r)| {
                let start = r.nodes.first().map(|n| n.position).unwrap_or([0.0, 0.0]);
                let end = r.nodes.last().map(|n| n.position).unwrap_or([0.0, 0.0]);
                crate::state::RopeView {
                    id: rid.raw(),
                    start,
                    end,
                    segment_count: r.segment_count,
                    segment_length_m: r.segment_length_m,
                    total_length_m: r.total_length_m(),
                    taut: r.taut,
                    embedded: r.embedded,
                    is_zipline: state.m14j_zipline_ropes.contains(rid),
                }
            })
            .collect();
        let ziplines_view: Vec<crate::state::ZiplineView> = state
            .m14j_zipline_ropes
            .iter()
            .filter_map(|rid| {
                let rope = state.m14j_ropes.get(rid)?;
                let start = rope.nodes.first().map(|n| n.position).unwrap_or([0.0, 0.0]);
                let end = rope.nodes.last().map(|n| n.position).unwrap_or([0.0, 0.0]);
                let (high_end, low_end) = if start[1] > end[1] { (start, end) } else { (end, start) };
                let dx = end[0] - start[0];
                let dy = end[1] - start[1];
                let span = (dx * dx + dy * dy).sqrt();
                let height_delta = (start[1] - end[1]).abs();
                let rider_count = state
                    .actor_state
                    .as_ref()
                    .map(|sim| {
                        sim.world
                            .actors
                            .values()
                            .filter(|a| a.zipline_attached == Some(*rid))
                            .count() as u32
                    })
                    .unwrap_or(0);
                Some(crate::state::ZiplineView {
                    id: rid.raw(),
                    high_end,
                    low_end,
                    span_m: span,
                    height_delta_m: height_delta,
                    max_speed_m_s: cf_equipment::ZIPLINE_MAX_SPEED_M_PER_S,
                    brake_decel_m_s2: cf_equipment::ZIPLINE_BRAKE_DECEL_M_PER_S2,
                    rider_count,
                })
            })
            .collect();
        let mount_links_view: Vec<crate::state::MountLinkView> = state
            .actor_state
            .as_ref()
            .map(|sim| {
                sim.world
                    .actors
                    .iter()
                    .filter_map(|(rider_id, a)| {
                        let m = a.mount?;
                        Some(crate::state::MountLinkView {
                            rider_id: rider_id.0,
                            critter_id: m.critter_id.0,
                            combined_mass_kg: m.combined_mass_kg,
                            mount_speed_retained: cf_actor::MOUNT_TOP_SPEED_RETAINED,
                            ride_direction: m.ride_direction,
                            firing_during_motion: m.firing_during_motion,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let frame = ObserveFrame {
            schema_version: SCHEMA_VERSION,
            run_id: self.recorder.run_id().to_string(),
            tick: state.clock.tick().0,
            sim_time_ms: state.clock.sim_time_ms(),
            run_status: observed_run_status(&state),
            scenario: self.config.scenario_id.clone(),
            events_since: self.recorder.snapshot_events().len() as u64,
            // stream so cf-e2e's events.<cat>.<type>.{count,first,last}
            // expectation grammar can drill into it. Heavy runs (≥18000
            // ticks) produce O(50K) events; the snapshot allocs a Vec
            // O(events) once per observe.once. Acceptable for M1 because
            // cf-e2e calls observe.once at most once per script.
            events: self
                .recorder
                .snapshot_events()
                .into_iter()
                .map(|e| {
                    json!({
                        "tick": e.tick,
                        "sim_time_ms": e.sim_time_ms,
                        "event_id": e.event_id,
                        "category": e.category,
                        "event_type": e.event_type,
                        "payload": e.payload,
                        "parent_event_id": e.parent_event_id,
                    })
                })
                .collect(),
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
            controls_capture: crate::state::ControlsCaptureView {
                captured: state.controls_captured_by.is_some(),
                capturer: state.controls_captured_by.clone(),
            },
            trench_segment_at_pos: None,
            cells,
            gravity_vectors,
            ropes: ropes_view,
            ziplines: ziplines_view,
            mount_links: mount_links_view,
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

    async fn inspect_equipment(&self, preset_id: &str) -> Option<serde_json::Value> {
        let spec = cf_equipment::rifle_preset(preset_id)?;
        serde_json::to_value(spec).ok()
    }

    async fn observe_actor(&self, actor_id: Option<u64>) -> Option<serde_json::Value> {
        let state = self.state.read().ok()?;
        let sim = state.actor_state.as_ref()?;
        let target_id = actor_id.unwrap_or_else(|| sim.world.player.map(|id| id.0).unwrap_or(0));
        let actor = sim.world.actors.get(&ActorId(target_id))?;
        let rifle = sim.rifles.get(&ActorId(target_id));
        let observation = cf_actor::ActorObservation::from_actor_and_rifle(actor, rifle);
        let mut payload = serde_json::to_value(observation).ok()?;
        // sim state + atmosphere snapshot.
        let mass_breakdown = cf_actor::mass_breakdown(actor);
        let mass_total = mass_breakdown.total();
        let mass_factor = if mass_total > 0.0 {
            (80.0_f32 / mass_total).clamp(0.25, 1.2)
        } else {
            1.0
        };
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("total_mass_kg".to_string(), json!(mass_total));
            obj.insert("mass_factor_walk".to_string(), json!(mass_factor));
            obj.insert("mass_factor_jump".to_string(), json!(mass_factor));
            obj.insert("chassis_mass_kg".to_string(), json!(mass_breakdown.chassis_kg));
            obj.insert("limb_mass_kg".to_string(), json!(mass_breakdown.limb_kg));
            obj.insert("held_devices_mass_kg".to_string(), json!(mass_breakdown.held_kg));
            obj.insert("inventory_weight_kg".to_string(), json!(mass_breakdown.inventory_kg));
            obj.insert(
                "jetpack_dry_mass_kg".to_string(),
                json!(actor.jetpack.as_ref().map_or(0.0, |j| j.dry_mass_kg)),
            );
            obj.insert(
                "jetpack_fuel_mass_kg".to_string(),
                json!(mass_breakdown.jetpack_fuel_kg),
            );
            obj.insert("wound_mass_kg".to_string(), json!(mass_breakdown.wound_kg));
            // Walking sim state.
            obj.insert("move_state".to_string(), json!(actor.move_state.as_str()));
            obj.insert("prone_state".to_string(), json!(actor.prone_state.as_str()));
            obj.insert("upper_body_state".to_string(), json!(actor.upper_body_state.as_str()));
            obj.insert(
                "attitude".to_string(),
                json!({
                    "rot": actor.attitude.rot,
                    "angular_vel": actor.attitude.angular_vel,
                    "rot_target": actor.attitude.rot_target,
                }),
            );
            obj.insert(
                "walk_angle".to_string(),
                json!({"fg": actor.walk_angle.fg, "bg": actor.walk_angle.bg}),
            );
            obj.insert(
                "walk_path_offset".to_string(),
                json!({"x": actor.walk_path_offset.x, "y": actor.walk_path_offset.y}),
            );
            obj.insert(
                "arm_sway".to_string(),
                json!({
                    "fg_arm_rot": actor.arm_sway.fg_arm_rot,
                    "bg_arm_rot": actor.arm_sway.bg_arm_rot,
                    "head_rot": actor.arm_sway.head_rot,
                    "bg_supporting_fg": actor.arm_sway.bg_supporting_fg,
                }),
            );
            obj.insert("stride_frame".to_string(), json!(actor.stride_frame));
            obj.insert("stride_timer_ms".to_string(), json!(actor.stride_timer_ms));
            obj.insert("last_stride_side_fg".to_string(), json!(actor.last_stride_side_fg));
            // Jetpack surface.
            if let Some(jet) = actor.jetpack.as_ref() {
                obj.insert(
                    "jetpack".to_string(),
                    json!({
                        "id": jet.id,
                        "type": jet.jetpack_type.as_str(),
                        "jet_time_left_ms": jet.jet_time_left_ms,
                        "jet_time_total_ms": jet.jet_time_total_ms,
                        "fuel_ratio": jet.fuel_ratio(),
                        "is_emitting": jet.is_emitting,
                        "throttle": jet.throttle,
                        "emit_angle": jet.emit_angle,
                    }),
                );
            }
            // Atmosphere overlay surface.
            obj.insert(
                "atmosphere".to_string(),
                json!({
                    "pressure_kpa": actor.atmosphere_sample.pressure_kpa,
                    "temp_k": actor.atmosphere_sample.temp_k,
                    "o2_partial_kpa": actor.atmosphere_sample.o2_partial_kpa,
                    "pollutant_partial_kpa": actor.atmosphere_sample.pollutant_partial_kpa,
                    "volatiles_partial_kpa": actor.atmosphere_sample.volatiles_partial_kpa,
                    "smoke_pct": actor.atmosphere_sample.smoke_pct,
                    "wind": actor.atmosphere_sample.wind,
                    "local_gravity_m_s2": actor.atmosphere_sample.local_gravity_m_s2,
                    "hypoxia_severity": actor.atmosphere_sample.hypoxia_severity(),
                }),
            );
        }
        Some(payload)
    }

    async fn observe_quick_action(&self, actor_id: Option<u64>) -> Option<serde_json::Value> {
        let state = self.state.read().ok()?;
        let sim = state.actor_state.as_ref()?;
        let target_id = actor_id.unwrap_or_else(|| sim.world.player.map(|id| id.0).unwrap_or(0));
        let actor = sim.world.actors.get(&ActorId(target_id))?;
        let bar = &actor.quick_action_bar;
        let bar_slots: Vec<serde_json::Value> = bar
            .slots
            .iter()
            .enumerate()
            .map(|(i, s)| {
                json!({
                    "slot": i + 1,
                    "kind": s.kind.as_str(),
                    "item_id": s.item_id,
                    "cooldown_ticks_remaining": s.cooldown_ticks_remaining,
                    "cooldown_total_ticks": s.cooldown_total_ticks,
                    "ammo": s.ammo,
                    "ammo_max": s.ammo_max,
                    "disabled_by_hazard": s.disabled_by_hazard,
                    "ready": s.ready(),
                })
            })
            .collect();
        Some(json!({
            "schema_version": SCHEMA_VERSION,
            "bar_slots": bar_slots,
            "last_used_slot": bar.last_used_slot + 1,
            "radial_open": matches!(bar.radial.phase, cf_actor::quick_action::RadialPhase::Open | cf_actor::quick_action::RadialPhase::Opening),
            "radial_phase": bar.radial.phase.as_str(),
            "radial_open_started_tick": bar.radial.opened_at_tick,
            "sim_time_multiplier": bar.radial.sim_time_multiplier,
        }))
    }

    /// `None` when no mission is loaded (e.g. m0_blank scenario).
    ///
    /// (with spec-literal field names — `status`, `timer_total_ticks`,
    /// `timer_ticks_remaining`, `current_objective_id`,
    /// `completed_objectives`, `failed_objectives`) instead of the raw
    /// `MissionState` struct so observe.mission carries the canonical
    /// surface.
    async fn observe_mission(&self) -> Option<serde_json::Value> {
        let state = self.state.read().ok()?;
        let mission = state.mission.as_ref()?;
        let current_tick = state.clock.tick().0;
        let view = cf_mission::MissionView::from_state(mission, current_tick);
        serde_json::to_value(view).ok()
    }

    /// reactor in the reactor world (M9 ships single-reactor scenarios; M25+
    /// command-core may surface multiple). `None` when no reactor is loaded.
    async fn observe_mission_reactor(&self) -> Option<serde_json::Value> {
        let state = self.state.read().ok()?;
        let world = state.reactor_world.as_ref()?;
        let reactor = world.iter().next()?;
        let layers: Vec<serde_json::Value> = reactor
            .armor_layers
            .iter()
            .map(|l| {
                json!({
                    "kind": l.kind.as_str(),
                    "hp": l.hp,
                    "max_hp": l.max_hp,
                    "hardness": l.hardness,
                    "hp_percent": l.hp_percent(),
                })
            })
            .collect();
        Some(json!({
            "schema_version": SCHEMA_VERSION,
            "actor_id": reactor.id.clone(),
            "kind": "Reactor",
            "hp": reactor.hp,
            "max_hp": reactor.max_hp,
            "hp_percent": reactor.hp_percent(),
            "pressure_state": reactor.pressure_state.as_str(),
            "position": reactor.position,
            "mission_critical": reactor.mission_critical,
            "role": reactor.role.clone(),
            "armor_layers": layers,
            "heat_signature_k": reactor.heat_signature_k,
            "destroyed": reactor.is_destroyed(),
        }))
    }

    /// timer projection. `color_state` follows the HUD rule from the spec:
    /// green > 30s, yellow 10-30s, red < 10s, "none" once expired/no timer.
    async fn observe_mission_timer(&self) -> Option<serde_json::Value> {
        let state = self.state.read().ok()?;
        let mission = state.mission.as_ref()?;
        let total_ticks = mission.loss.time_limit_ticks;
        if total_ticks == 0 {
            return Some(json!({
                "schema_version": SCHEMA_VERSION,
                "remaining_ticks": 0u64,
                "total_ticks": 0u64,
                "remaining_seconds": 0u64,
                "color_state": "none",
            }));
        }
        let tick_now = state.clock.tick().0;
        let remaining_ticks = total_ticks.saturating_sub(tick_now);
        let tick_rate = self.config.tick_rate_hz.max(1) as u64;
        let remaining_s = remaining_ticks / tick_rate;
        let color_state = if remaining_s > 30 {
            "green"
        } else if remaining_s >= 10 {
            "yellow"
        } else {
            "red"
        };
        Some(json!({
            "schema_version": SCHEMA_VERSION,
            "remaining_ticks": remaining_ticks,
            "total_ticks": total_ticks,
            "remaining_seconds": remaining_s,
            "color_state": color_state,
        }))
    }

    /// returns the M9 director projection per spec § Director state
    /// surface. Spawn-budget and intensity ladder to M25+; at M9 they
    /// are stable scalars (0.0 and 0 respectively) so the surface is
    /// already round-trippable.
    async fn observe_mission_director(&self) -> Option<serde_json::Value> {
        let state = self.state.read().ok()?;
        let phase = state.m7_ai_world.phase.as_ref()?;
        let active_objectives: Vec<String> = state
            .mission
            .as_ref()
            .map(|m| {
                m.objectives
                    .iter()
                    .filter(|o| o.status == cf_mission::ObjectiveStatus::Active)
                    .map(|o| o.id.clone())
                    .collect()
            })
            .unwrap_or_default();
        let phases_completed: Vec<String> = phase.phases_completed.iter().map(|p| p.as_str().to_string()).collect();
        let tick_rate = self.config.tick_rate_hz.max(1);
        let deadline_tick = phase.deadline_tick(tick_rate);
        Some(json!({
            "schema_version": SCHEMA_VERSION,
            "current_phase": phase.current.as_str(),
            "phase_started_at_tick": phase.entered_tick,
            "phases_completed": phases_completed,
            "intensity": 0.0_f32,
            "spawn_budget": 0_u32,
            "active_objectives": active_objectives,
            "deadline_tick": deadline_tick,
            "phase_sequence": phase
                .phase_sequence
                .iter()
                .map(|p| p.as_str().to_string())
                .collect::<Vec<_>>(),
        }))
    }

    /// returns the reactor projection plus the last `last_n_events`
    /// actor-category events. The reactor's id is a string (e.g.
    /// `"core_reactor"`), unlike inspect.actor which keys on u64.
    /// Returns `None` when no reactor world is loaded.
    async fn inspect_actor_reactor(&self, last_n_events: usize) -> Option<serde_json::Value> {
        let view = self.observe_mission_reactor().await?;
        let reactor_id = view.get("actor_id").and_then(|v| v.as_str()).map(|s| s.to_string());
        let events = self.recorder.snapshot_events();
        let mut filtered: Vec<serde_json::Value> = events
            .iter()
            .filter(|e| e.category == "actor" || e.category == "armor" || e.category == "mission")
            .filter(|e| match &reactor_id {
                None => true,
                Some(rid) => e
                    .payload
                    .get("reactor_id")
                    .and_then(|v| v.as_str())
                    .map(|p| p == rid.as_str())
                    .or_else(|| {
                        e.payload
                            .get("reactor")
                            .and_then(|v| v.as_str())
                            .map(|p| p == rid.as_str())
                    })
                    .or_else(|| {
                        e.payload
                            .get("actor")
                            .and_then(|v| v.as_str())
                            .map(|p| p == rid.as_str())
                    })
                    .unwrap_or(false),
            })
            .rev()
            .take(last_n_events)
            .map(|e| serde_json::to_value(e).unwrap_or(serde_json::Value::Null))
            .collect();
        filtered.reverse();
        Some(serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "actor": view,
            "events": filtered,
            "events_count": filtered.len(),
        }))
    }

    /// method per spec literal. Returns the live `TerrainView` projection.
    async fn observe_terrain(&self) -> Option<serde_json::Value> {
        let state = self.state.read().ok()?;
        let t = state.chunked_terrain.as_ref()?;
        let view = crate::state::TerrainView {
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
            chunk_count: t.allocated_chunk_count() as u32,
            material_counts: t.material_counts(),
            material_distribution: t
                .material_counts()
                .into_iter()
                .filter_map(|(name, count)| cf_terrain::material_id_from_name(&name).map(|id| (id, count)))
                .collect(),
            current_overlay_mode: state.material_overlay_mode.clone(),
            total_carve_events: state.total_carve_events,
            total_debris_spawned: state.total_debris_spawned,
        };
        serde_json::to_value(view).ok()
    }

    /// Returns guard state + perception summary + current target + reason.
    ///
    /// guard actor's `hp` + `hp_max` from the actor world so the
    /// difficulty preset round-trip ("guard's hp=120") can be verified
    /// without a separate observe.actor call.
    async fn observe_ai(&self, actor_id: u64) -> Option<serde_json::Value> {
        let state = self.state.read().ok()?;
        let guard = state.reactive_guards.get(&ActorId(actor_id))?;
        let mut v = serde_json::to_value(guard).ok()?;
        if let Some(world) = state.actor_state.as_ref() {
            if let Some(actor) = world.world.actors.get(&ActorId(actor_id)) {
                if let Some(obj) = v.as_object_mut() {
                    obj.insert("hp".into(), json!(actor.hp));
                    obj.insert("hp_max".into(), json!(actor.hp_max));
                }
            }
        }
        Some(v)
    }

    /// + stealth_meter + last footstep loudness band + last occlusion
    /// factor + spotted flag. `actor_id=None` resolves to the player.
    /// Returns `None` when no actor world is loaded.
    async fn observe_perception(&self, actor_id: Option<u64>) -> Option<serde_json::Value> {
        let state = self.state.read().ok()?;
        let sim = state.actor_state.as_ref()?;
        let target_id = actor_id.unwrap_or_else(|| sim.world.player.map(|id| id.0).unwrap_or(0));
        let actor = sim.world.actors.get(&ActorId(target_id))?;
        let events = self.recorder.snapshot_events();
        let last_footstep_band = events
            .iter()
            .rev()
            .find(|e| {
                e.category == "perception"
                    && e.event_type == "footstep_emitted"
                    && e.payload
                        .get("actor")
                        .and_then(|v| v.as_u64())
                        .map(|id| id == target_id)
                        .unwrap_or(false)
            })
            .and_then(|e| e.payload.get("band").and_then(|v| v.as_str()).map(str::to_string))
            .unwrap_or_else(|| cf_perception::LoudnessBand::Inaudible.as_str().to_string());
        let last_occlusion = events
            .iter()
            .rev()
            .find(|e| {
                e.category == "perception"
                    && e.event_type == "occlusion_applied"
                    && e.payload
                        .get("receiver")
                        .and_then(|v| v.as_u64())
                        .map(|id| id == target_id)
                        .unwrap_or(false)
            })
            .and_then(|e| e.payload.get("occlusion_factor").and_then(|v| v.as_f64()))
            .unwrap_or(1.0) as f32;
        Some(json!({
            "schema_version": 1,
            "actor_id": target_id,
            "sight_cone_degrees": 110.0,
            "hearing_radius": cf_perception::ALARM_RADIUS_BASE,
            "stealth_meter": actor.stealth_meter,
            "spotted": actor.stealth_meter >= 0.5,
            "last_footstep_loudness_band": last_footstep_band,
            "last_occlusion_factor": last_occlusion,
        }))
    }

    /// per-member current_command + hp + waypoint. Returns `None` when no
    /// actor world is loaded.
    async fn observe_squad(&self) -> Option<serde_json::Value> {
        let state = self.state.read().ok()?;
        let _sim = state.actor_state.as_ref()?;
        let squad = state.squad.clone();
        let members: Vec<serde_json::Value> = squad
            .iter()
            .map(|m| {
                json!({
                    "actor_id": m.actor.0,
                    "role": m.role.as_str(),
                    "display_name": m.display_name,
                    "current_command": m.current_command.kind.as_str(),
                    "waypoint": m.waypoint.map(|p| json!({"x": p.x, "y": p.y})),
                    "hp": m.hp,
                    "hp_max": m.hp_max,
                })
            })
            .collect();
        Some(json!({
            "schema_version": 1,
            "leader_id": squad.leader.as_ref().map(|l| l.actor.0),
            "member_count": squad.member_count(),
            "members": members,
        }))
    }

    /// + role + personality modifier. Returns `None` if the actor has no
    /// `BotState`.
    async fn observe_priority_table(&self, actor_id: u64) -> Option<serde_json::Value> {
        let state = self.state.read().ok()?;
        state.m7_ai_world.priority_table_view(cf_actor::ActorId(actor_id))
    }

    /// doctrine_mode. Returns `None` if the actor has no `BotState`.
    async fn observe_autonomy(&self, actor_id: u64) -> Option<serde_json::Value> {
        let state = self.state.read().ok()?;
        state.m7_ai_world.autonomy_view(cf_actor::ActorId(actor_id))
    }

    async fn dump_squad_state(&self, squad_id: u64) -> Option<serde_json::Value> {
        let state = self.state.read().ok()?;
        Some(state.m7b_squad.dump_state_view(squad_id))
    }

    /// `CinematicState` projection. Returns `None` when no cinematic is
    /// active (callers fall back to a 'no cinematic' sentinel).
    async fn dump_cinematic_state(&self) -> serde_json::Value {
        let state = self.state.read().expect("engine state poisoned");
        if let Some(kernel) = state.cinematic_kernel.as_ref() {
            let snapshot = kernel.state();
            json!({
                "schema_version": snapshot.schema_version,
                "cinematic_id": snapshot.cinematic_id,
                "source": snapshot.source.map(|s| s.as_str()),
                "phase": match snapshot.phase {
                    cf_cinematic::PlaybackPhase::PendingStart => "pending_start",
                    cf_cinematic::PlaybackPhase::Playing => "playing",
                    cf_cinematic::PlaybackPhase::Paused => "paused",
                    cf_cinematic::PlaybackPhase::Ended => "ended",
                },
                "playhead_ms": snapshot.playhead_ms,
                "duration_ms": snapshot.duration_ms,
                "replay": snapshot.replay,
                "paused": snapshot.paused,
                "sandbox_suppressed": snapshot.sandbox_suppressed,
                "active_word_index": snapshot.active_word_index,
                "briefing_card_lines": snapshot.briefing_card_lines.clone(),
                "camera_translation": [snapshot.camera_translation[0], snapshot.camera_translation[1]],
                "camera_shake_px": [snapshot.camera_shake_px[0], snapshot.camera_shake_px[1]],
                "camera_ortho_half_height": snapshot.camera_ortho_half_height,
                "active": kernel.is_active(),
                "blocks_gameplay_input": kernel.blocks_gameplay_input(),
                "seen_set_count": state.cinematic_seen_set.len(),
            })
        } else {
            json!({
                "schema_version": 1,
                "cinematic_id": null,
                "source": null,
                "phase": "ended",
                "playhead_ms": 0,
                "duration_ms": 0,
                "replay": false,
                "paused": false,
                "sandbox_suppressed": false,
                "active_word_index": null,
                "briefing_card_lines": [],
                "camera_translation": [0.0, 0.0],
                "camera_shake_px": [0.0, 0.0],
                "camera_ortho_half_height": 0.0,
                "active": false,
                "blocks_gameplay_input": false,
                "seen_set_count": state.cinematic_seen_set.len(),
            })
        }
    }

    /// currently-playing cinematic. Returns `Ok(())` on success or an
    /// error reason when the skip is rejected.
    async fn act_player_skip_cinematic(&self) -> Result<u32, String> {
        let mut state = self.state.write().expect("engine state poisoned");
        let tick = state.clock.tick();
        let sim_time_ms = state.clock.sim_time_ms();
        let kernel = match state.cinematic_kernel.as_mut() {
            Some(k) => k,
            None => return Err("no_cinematic_active".to_string()),
        };
        let request = kernel.request_skip();
        let (skipped_ms, events) = match request {
            Some((sk, end)) => {
                // Extract skipped_at_ms via pattern match.
                let mut ms_out = 0u32;
                if let cf_cinematic::CinematicEvent::Skipped { skipped_at_ms, .. } = &sk {
                    ms_out = *skipped_at_ms;
                }
                (ms_out, vec![sk, end])
            }
            None => return Err("skip_blocked_within_confirm_window".to_string()),
        };
        // Mirror seen-set from kernel back into engine-level field.
        state.cinematic_seen_set = kernel.seen().clone();
        let parent = state.run_started_event_id.clone();
        drop(state);
        for ev in events {
            self.emit_cinematic_event(tick, sim_time_ms, &ev, parent.as_deref());
        }
        Ok(skipped_ms)
    }

    /// Returns the (paused, ms) tuple after the toggle.
    async fn act_player_pause_cinematic(&self) -> Result<(bool, u32), String> {
        let mut state = self.state.write().expect("engine state poisoned");
        let tick = state.clock.tick();
        let sim_time_ms = state.clock.sim_time_ms();
        let kernel = match state.cinematic_kernel.as_mut() {
            Some(k) => k,
            None => return Err("no_cinematic_active".to_string()),
        };
        let event = if kernel.state().phase == cf_cinematic::PlaybackPhase::Paused {
            kernel.request_resume()
        } else {
            kernel.request_pause()
        };
        let Some(ev) = event else {
            return Err("invalid_pause_state".to_string());
        };
        let (paused, ms) = match &ev {
            cf_cinematic::CinematicEvent::Paused { ms, .. } => (true, *ms),
            cf_cinematic::CinematicEvent::Resumed { ms, .. } => (false, *ms),
            _ => (false, 0),
        };
        let parent = state.run_started_event_id.clone();
        drop(state);
        self.emit_cinematic_event(tick, sim_time_ms, &ev, parent.as_deref());
        Ok((paused, ms))
    }

    /// cinematic from `Codex → Cinematics`. The dispatcher loads the
    /// script from `content/cinematics/**/<id>.cinematic.ron`, narration
    /// track (if any), and the active storyteller profile, then engages
    /// the kernel with `replay=true` so save state is NOT mutated.
    async fn act_player_replay_cinematic(&self, id: &str) -> Result<u64, String> {
        let state_read = self.state.read().expect("engine state poisoned");
        let seen = state_read.cinematic_seen_set.clone();
        if !seen.contains(id) {
            return Err("cinematic_locked".to_string());
        }
        drop(state_read);
        // Locate the script in opening / between / ending directories.
        let candidates = [
            ("opening", format!("game/content/cinematics/opening/{id}.cinematic.ron")),
            ("between", format!("game/content/cinematics/between/{id}.cinematic.ron")),
            ("ending", format!("game/content/cinematics/ending/{id}.cinematic.ron")),
        ];
        let mut script_bytes: Option<Vec<u8>> = None;
        for (_label, path) in &candidates {
            if let Ok(bytes) = std::fs::read(path) {
                script_bytes = Some(bytes);
                break;
            }
        }
        let bytes = script_bytes.ok_or_else(|| format!("script_not_found:{id}"))?;
        let script = cf_cinematic::CinematicScript::from_ron(&bytes).map_err(|e| format!("script_parse_error:{e}"))?;
        let profile = cf_cinematic::builtin_profile(
            script
                .storyteller
                .unwrap_or(cf_cinematic::StorytellerId::CassandraClassic),
        );
        let narration = if let Some(track_id) = &script.narration_track_id {
            let track_path = format!("game/content/audio/voice/cinematic/{track_id}.narration_track.json");
            std::fs::read(&track_path)
                .ok()
                .and_then(|b| cf_cinematic::NarrationTrack::from_json(&b).ok())
                .unwrap_or_default()
        } else {
            cf_cinematic::NarrationTrack::default()
        };
        let seed = self.config.seed
            ^ u64::from_le_bytes({
                let mut buf = [0u8; 8];
                let id_hash = blake3::hash(id.as_bytes());
                buf.copy_from_slice(&id_hash.as_bytes()[..8]);
                buf
            });
        let mut state = self.state.write().expect("engine state poisoned");
        let tick = state.clock.tick();
        state.cinematic_kernel = Some(cf_cinematic::CinematicKernel::new(
            script, profile, narration, seed, seen, true,
        ));
        Ok(tick.0)
    }

    async fn observe_camera(&self) -> serde_json::Value {
        self.snapshot_camera_state()
    }

    async fn observe_localization_current_language(&self) -> serde_json::Value {
        self.snapshot_localization_language()
    }

    async fn observe_debug_overlays(&self) -> serde_json::Value {
        self.snapshot_debug_overlays()
    }

    async fn observe_tactical_overlay(&self) -> serde_json::Value {
        self.snapshot_tactical_overlay()
    }

    async fn observe_tags(&self) -> serde_json::Value {
        self.snapshot_tags()
    }

    async fn observe_accessibility(&self) -> serde_json::Value {
        let settings = self.current_settings();
        let s = self.state.read().expect("engine state poisoned");
        let focused_node = s.hud_focus_index.map(|i| HUD_FOCUSABLE_NODES[i].to_string());
        let banners: Vec<serde_json::Value> = s
            .hud_banners
            .iter()
            .map(|b| {
                json!({
                    "banner_id": b.id,
                    "severity": b.severity,
                    "label": b.label,
                    "raised_at_tick": b.raised_at_tick,
                    "accessibility_id": b.accessibility_id,
                })
            })
            .collect();
        let captions: Vec<serde_json::Value> = s
            .hud_captions
            .iter()
            .map(|c| {
                json!({
                    "id": c.id,
                    "label": c.label,
                    "raised_at_tick": c.raised_at_tick,
                    "accessibility_id": c.accessibility_id,
                })
            })
            .collect();
        let nodes: Vec<&'static str> = HUD_FOCUSABLE_NODES.to_vec();
        let settings_value = serde_json::to_value(&settings).unwrap_or(serde_json::Value::Null);
        json!({
            "schema_version": SCHEMA_VERSION,
            "settings": settings_value,
            "focusable_nodes": nodes,
            "focused_node": focused_node,
            "banners": banners,
            "captions": captions,
            "ui_scale": settings.ui_scale,
            "high_contrast": settings.high_contrast,
            "contrast_mode": settings.contrast_mode.as_str(),
            "captions_enabled": settings.captions,
            "caption_mode": settings.caption_mode.as_str(),
            "caption_categories": settings.caption_categories.iter().collect::<Vec<_>>(),
            "input_profile": settings.input_profile.as_str(),
            "hold_behavior": settings.hold_behavior.as_str(),
            "screen_shake_scale": settings.screen_shake_scale,
            "camera_motion": settings.camera_motion.as_str(),
            "objective_help": settings.objective_help.as_str(),
            "debug_explainer_level": settings.debug_explainer_level.as_str(),
        })
    }

    async fn observe_captions(&self) -> serde_json::Value {
        let s = self.state.read().expect("engine state poisoned");
        let queue: Vec<serde_json::Value> = s
            .hud_captions
            .iter()
            .map(|c| {
                json!({
                    "id": c.id,
                    "label": c.label,
                    "raised_at_tick": c.raised_at_tick,
                    "accessibility_id": c.accessibility_id,
                })
            })
            .collect();
        json!({ "schema_version": SCHEMA_VERSION, "queue": queue })
    }

    async fn observe_accessibility_banners(&self) -> serde_json::Value {
        let s = self.state.read().expect("engine state poisoned");
        let banners: Vec<serde_json::Value> = s
            .hud_banners
            .iter()
            .map(|b| {
                json!({
                    "banner_id": b.id,
                    "severity": b.severity,
                    "label": b.label,
                    "raised_at_tick": b.raised_at_tick,
                    "accessibility_id": b.accessibility_id,
                    "expires_at_tick": b.expires_at_tick,
                })
            })
            .collect();
        json!({ "schema_version": SCHEMA_VERSION, "banners": banners })
    }

    async fn observe_actor_silhouette(&self, actor_id: Option<u64>) -> Option<serde_json::Value> {
        let state = self.state.read().ok()?;
        let sim = state.actor_state.as_ref()?;
        let target_id = actor_id.unwrap_or_else(|| sim.world.player.map(|id| id.0).unwrap_or(0));
        let actor = sim.world.actors.get(&ActorId(target_id))?;
        let silhouette = actor.body_silhouette();
        Some(json!({
            "schema_version": SCHEMA_VERSION,
            "actor_id": target_id,
            "head_hp_pct": silhouette.head_hp_pct,
            "torso_hp_pct": silhouette.torso_hp_pct,
            "arm_left_hp_pct": silhouette.arm_left_hp_pct,
            "arm_right_hp_pct": silhouette.arm_right_hp_pct,
            "leg_left_hp_pct": silhouette.leg_left_hp_pct,
            "leg_right_hp_pct": silhouette.leg_right_hp_pct,
            "placeholder": silhouette.placeholder,
        }))
    }

    async fn observe_actor_module_strip(&self, actor_id: Option<u64>) -> Option<serde_json::Value> {
        let state = self.state.read().ok()?;
        let sim = state.actor_state.as_ref()?;
        let target_id = actor_id.unwrap_or_else(|| sim.world.player.map(|id| id.0).unwrap_or(0));
        let actor = sim.world.actors.get(&ActorId(target_id))?;
        let rifle = sim.rifles.get(&ActorId(target_id));
        let module_strip = match actor.chassis_module_strip() {
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
            None => build_module_strip_view(rifle, actor.inventory.selected_item().is_rifle()),
        };
        Some(json!({
            "schema_version": SCHEMA_VERSION,
            "actor_id": target_id,
            "modules": module_strip.modules,
            "placeholder": module_strip.placeholder,
        }))
    }

    /// Predicate forms supported:
    ///   - `severity=<critical|warning|info>` (matches against `hud.banners.*`)
    ///   - `text~=<substring>` (case-insensitive contains)
    ///   - `present=true|false` (existence check)
    async fn ui_assert(&self, node_id: &str, predicate: &str) -> serde_json::Value {
        let s = self.state.read().expect("engine state poisoned");
        let observed_text = match node_id {
            "hud.banners" => {
                // First (highest-severity) banner's label.
                s.hud_banners.iter().map(|b| b.label.clone()).next().unwrap_or_default()
            }
            "hud.captions" => s
                .hud_captions
                .iter()
                .map(|c| c.label.clone())
                .next()
                .unwrap_or_default(),
            "hud.last_event" => s
                .hud_captions
                .iter()
                .last()
                .map(|c| c.label.clone())
                .unwrap_or_default(),
            _ => String::new(),
        };
        let severity = s
            .hud_banners
            .iter()
            .map(|b| b.severity.clone())
            .next()
            .unwrap_or_default();
        let pass = if let Some(rest) = predicate.strip_prefix("severity=") {
            severity == rest
        } else if let Some(rest) = predicate.strip_prefix("text~=") {
            observed_text.to_lowercase().contains(&rest.to_lowercase())
        } else if let Some(rest) = predicate.strip_prefix("present=") {
            let want = rest == "true";
            let present = !observed_text.is_empty();
            present == want
        } else {
            false
        };
        json!({
            "schema_version": SCHEMA_VERSION,
            "node_id": node_id,
            "predicate": predicate,
            "pass": pass,
            "observed": {
                "text": observed_text,
                "severity": severity,
            },
        })
    }

    /// actor against the engine's trench-segment world. Pre-segment
    /// placement the world is empty + cover defaults to `Exposed`. The
    /// m9b_trench dispatcher module owns the live wiring.
    async fn observe_actor_cover_state(&self, actor_id: u64) -> serde_json::Value {
        self.compute_actor_cover_state(actor_id)
    }

    /// supplied tile, or `null` when the tile is open ground.
    async fn observe_trench_segment_at_pos(&self, x: i32, y: i32) -> serde_json::Value {
        self.compute_trench_segment_at_pos(x, y)
    }

    /// 30 mission-category events.
    async fn inspect_mission(&self) -> Option<serde_json::Value> {
        let mission = self.observe_mission().await?;
        let events = self.recorder.snapshot_events();
        let mut filtered: Vec<serde_json::Value> = events
            .iter()
            .filter(|e| e.category == "mission")
            .rev()
            .take(30)
            .map(|e| serde_json::to_value(e).unwrap_or(serde_json::Value::Null))
            .collect();
        filtered.reverse();
        Some(serde_json::json!({
            "mission": mission,
            "events": filtered,
        }))
    }

    /// `ai.*` events filtered to `actor_id`.
    async fn inspect_ai(&self, actor_id: u64) -> Option<serde_json::Value> {
        let view = self.observe_ai(actor_id).await?;
        let events = self.recorder.snapshot_events();
        let mut filtered: Vec<serde_json::Value> = events
            .iter()
            .filter(|e| e.category == "ai")
            .filter(|e| {
                e.payload
                    .get("actor_id")
                    .and_then(|v| v.as_u64())
                    .map(|id| id == actor_id)
                    .unwrap_or(false)
            })
            .rev()
            .take(30)
            .map(|e| serde_json::to_value(e).unwrap_or(serde_json::Value::Null))
            .collect();
        filtered.reverse();
        Some(serde_json::json!({
            "ai": view,
            "events": filtered,
        }))
    }

    async fn inspect_actor(&self, target: Option<&str>, last_n_events: usize) -> Option<serde_json::Value> {
        let actor_id_opt: Option<u64> = match target {
            None | Some("player") | Some("") => None,
            Some(t) => t.parse::<u64>().ok(),
        };
        let view = self.observe_actor(actor_id_opt).await?;
        // Pull last N actor-category events for the target.
        let id_for_filter = view.get("id").and_then(|v| v.as_u64());
        let events = self.recorder.snapshot_events();
        let mut filtered: Vec<serde_json::Value> = events
            .iter()
            .filter(|e| e.category == "actor")
            .filter(|e| {
                id_for_filter
                    .and_then(|id| e.payload.get("actor").and_then(|v| v.as_u64()).map(|p| p == id))
                    .unwrap_or(true)
            })
            .rev()
            .take(last_n_events)
            .map(|e| serde_json::to_value(e).unwrap_or(serde_json::Value::Null))
            .collect();
        filtered.reverse();
        let merged = serde_json::json!({
            "actor": view,
            "events": filtered,
            "events_count": filtered.len(),
        });
        Some(merged)
    }

    /// silhouette projection: per-zone HP percentages + pilot weight class
    /// scale factor. Surfaces the chassis half of the dual-layer HUD
    /// silhouette so consumers can pair with `observe.actor.silhouette`
    /// (the pilot side).
    async fn observe_chassis_silhouette(&self, actor_id: Option<u64>) -> Option<serde_json::Value> {
        let state = self.state.read().ok()?;
        let sim = state.actor_state.as_ref()?;
        let target_id = actor_id.unwrap_or_else(|| sim.world.player.map(|id| id.0).unwrap_or(0));
        let actor = sim.world.actors.get(&cf_actor::ActorId(target_id))?;
        let chassis = actor.chassis.as_ref()?;
        let zones: Vec<serde_json::Value> = chassis
            .zones
            .iter()
            .map(|z| {
                json!({
                    "zone": z.zone.as_str(),
                    "hp_pct": z.zone_integrity(),
                    "external_integrity": z.external_integrity(),
                    "internal_integrity": z.internal_integrity(),
                    "core_integrity": z.core_integrity(),
                    "destroyed": z.destroyed,
                })
            })
            .collect();
        Some(json!({
            "schema_version": SCHEMA_VERSION,
            "actor_id": target_id,
            "spec_id": chassis.spec_id,
            "kind": chassis.kind.as_str(),
            "stage": chassis.stage.as_str(),
            "pilot_silhouette_scale": chassis.kind.pilot_silhouette_scale(),
            "placeholder": false,
            "destroyed_zones": chassis
                .destroyed_zones()
                .iter()
                .map(|z| z.as_str().to_string())
                .collect::<Vec<_>>(),
            "zones": zones,
        }))
    }

    /// integrity + per-module state + pilot state + eject window for the
    /// requested actor's chassis. Returns `None` when no chassis is attached.
    async fn inspect_chassis(&self, target: Option<&str>) -> Option<serde_json::Value> {
        let state = self.state.read().ok()?;
        let sim = state.actor_state.as_ref()?;
        let target_id_opt: Option<u64> = match target {
            None | Some("player") | Some("") => None,
            Some(t) => t.parse::<u64>().ok(),
        };
        let target_id = target_id_opt.unwrap_or_else(|| sim.world.player.map(|id| id.0).unwrap_or(0));
        let actor = sim.world.actors.get(&cf_actor::ActorId(target_id))?;
        let chassis = actor.chassis.as_ref()?;

        // Build per-zone integrity payload.
        let zones: Vec<serde_json::Value> = chassis
            .zones
            .iter()
            .map(|z| {
                let layers: Vec<serde_json::Value> = z
                    .layers
                    .iter()
                    .map(|l| {
                        json!({
                            "kind": l.kind.as_str(),
                            "hp": l.hp,
                            "hp_max": l.hp_max,
                            "hardness": l.hardness,
                            "integrity": l.integrity(),
                            "breached": l.is_breached(),
                        })
                    })
                    .collect();
                json!({
                    "zone": z.zone.as_str(),
                    "layers": layers,
                    "external_integrity": z.external_integrity(),
                    "internal_integrity": z.internal_integrity(),
                    "core_integrity": z.core_integrity(),
                    "wound_hp": z.wound_hp,
                    "wound_hp_max": z.wound_hp_max,
                    "wound_integrity": z.wound_integrity(),
                    "zone_integrity": z.zone_integrity(),
                    "destroyed": z.destroyed,
                })
            })
            .collect();
        // Joints — exactly 14 for the humanoid body graph.
        let joints: Vec<serde_json::Value> = chassis
            .body_graph
            .joints
            .iter()
            .map(|j| {
                json!({
                    "id": j.id,
                    "parent": j.parent.as_str(),
                    "child": j.child.as_str(),
                    "intact": j.intact,
                })
            })
            .collect();
        // Equipment sockets — 5 for the humanoid body graph.
        let sockets: Vec<serde_json::Value> = chassis
            .body_graph
            .sockets
            .iter()
            .map(|s| {
                json!({
                    "id": s.id,
                    "zone": s.zone.as_str(),
                    "occupied": s.occupied,
                    "mounted_role": s.mounted_role,
                })
            })
            .collect();
        // Modules — id, kind, state, bound zone, hp.
        let modules: Vec<serde_json::Value> = chassis
            .modules
            .iter()
            .map(|m| {
                json!({
                    "id": m.id,
                    "kind": m.kind.as_str(),
                    "state": m.state.as_str(),
                    "bound_zone": m.bound_zone.as_str(),
                    "hp": m.hp,
                    "hp_max": m.hp_max,
                    "integrity": m.integrity(),
                    "last_reason": m.last_reason,
                })
            })
            .collect();
        // Movement-contribution table from the body graph (which capabilities are
        // lost when each zone is destroyed). Useful for AI doctrine + HUD.
        let movement_contributions: Vec<serde_json::Value> = chassis
            .body_graph
            .movement_contributions
            .iter()
            .map(|c| {
                json!({
                    "zone": c.zone.as_str(),
                    "move_speed_factor_when_destroyed": c.move_speed_factor_when_destroyed,
                    "jump_impulse_factor_when_destroyed": c.jump_impulse_factor_when_destroyed,
                    "disables_rifle_when_destroyed": c.disables_rifle_when_destroyed,
                    "forces_crawl_when_destroyed": c.forces_crawl_when_destroyed,
                    "drops_gear_when_destroyed": c.drops_gear_when_destroyed,
                    "disables_jet_when_destroyed": c.disables_jet_when_destroyed,
                })
            })
            .collect();
        let salvaged_module_ids: Vec<String> = chassis.salvaged_modules.iter().map(|m| m.id.clone()).collect();
        let destroyed_zones: Vec<String> = chassis
            .destroyed_zones()
            .iter()
            .map(|z| z.as_str().to_string())
            .collect();
        Some(json!({
            "schema_version": SCHEMA_VERSION,
            "actor_id": target_id,
            "spec_id": chassis.spec_id,
            "kind": chassis.kind.as_str(),
            "stage": chassis.stage.as_str(),
            "pilot_state": chassis.pilot_state.as_str(),
            "tutorial_safety": chassis.tutorial_safety,
            "mass_kg": chassis.mass_kg,
            "weapon_jammed": chassis.weapon_jammed,
            "tick_rate_hz": chassis.tick_rate_hz,
            "integrity": chassis.integrity(),
            "eject_ticks_remaining": chassis.eject_window.ticks_remaining,
            "eject_ticks_total": chassis.eject_window.ticks_total,
            "eject_triggered_at_tick": chassis.eject_window.triggered_at_tick,
            "body_graph": {
                "zone_count": chassis.body_graph.zones.len(),
                "joint_count": chassis.body_graph.joints.len(),
                "socket_count": chassis.body_graph.sockets.len(),
                "zones": chassis.body_graph.zones.iter().map(|z| z.as_str()).collect::<Vec<_>>(),
                "joints": joints,
                "sockets": sockets,
                "movement_contributions": movement_contributions,
            },
            "zones": zones,
            "destroyed_zones": destroyed_zones,
            "modules": modules,
            "salvaged_module_ids": salvaged_module_ids,
            "last_stage_reason": chassis.last_stage_reason,
        }))
    }

    async fn inspect_terrain_chunk(&self, cx: i32, cy: i32) -> Option<serde_json::Value> {
        let state = self.state.read().ok()?;
        let terrain = state.chunked_terrain.as_ref()?;
        let pixels = terrain.chunk_pixels(cx, cy);
        let checksum = terrain.chunk_checksum(cx, cy);
        // RLE-encode pixels for compact transport. Format: pairs of [material_id, run_length].
        let mut rle: Vec<serde_json::Value> = Vec::new();
        let mut iter = pixels.iter().peekable();
        while let Some(&first) = iter.next() {
            let mut run: u32 = 1;
            while let Some(&&n) = iter.peek() {
                if n == first {
                    iter.next();
                    run += 1;
                } else {
                    break;
                }
            }
            rle.push(serde_json::json!([first, run]));
        }
        let cs = cf_terrain::CHUNK_SIZE as i64;
        let origin = [cx as i64 * cs, cy as i64 * cs];
        // M3 re-audit pass 4 (2026-05-13): spec requires the response to
        // include `dirty_rect` AND the chunk's stored `last_modified_tick`
        // (not the engine's current tick).
        let dirty_rect = terrain.chunk_dirty_rect(cx, cy).map(|r| {
            serde_json::json!({
                "min": [r.min[0], r.min[1]],
                "max": [r.max[0], r.max[1]],
            })
        });
        let last_modified_tick = terrain.chunk_last_modified_tick(cx, cy);
        // `material_grid` (RLE-encoded) and `chunk_checksum`. The legacy
        // `material_grid_rle` + `checksum` aliases are kept alongside for
        // backwards-compat with any in-flight tooling.
        Some(serde_json::json!({
            "chunk_pos": { "cx": cx, "cy": cy },
            "chunk_size_pixels": cf_terrain::CHUNK_SIZE,
            "pixel_origin": origin,
            "material_grid": rle.clone(),
            "material_grid_rle": rle,
            "chunk_checksum": checksum.clone(),
            "checksum": checksum,
            "last_modified_tick": last_modified_tick,
            "dirty_rect": dirty_rect,
        }))
    }

    async fn inspect_material(&self, id: u16) -> Option<serde_json::Value> {
        let aff = cf_terrain::material_affordance(id)?;
        // Try to load the JSON registry to surface the full MaterialDef
        // (with future-compat fields). If load fails we fall back to the
        // runtime affordance projection.
        if let Some(path) = cf_material::MaterialRegistry::locate_default() {
            if let Ok((registry, _)) = cf_material::load_registry_from_file(&path) {
                if let Some(def) = registry.find_by_id(id) {
                    if let Ok(value) = serde_json::to_value(def) {
                        return Some(value);
                    }
                }
            }
        }
        Some(serde_json::json!({
            "id": aff.id,
            "name": aff.name,
            "display_name": aff.name,
            "hardness": aff.hardness,
            "diggable": aff.diggable,
            "anchorable": aff.anchorable,
            "hazard": aff.hazard,
            "damage_per_tick": aff.damage_per_tick,
            "path_cost": aff.path_cost,
            "density": aff.density,
            "drillable": aff.drillable,
            "blastable": aff.blastable,
            "beam_cuttable": aff.beam_cuttable,
            "projectile_passable": aff.projectile_passable,
            "actor_passable": aff.actor_passable,
            "blocks_line_of_sight": aff.blocks_line_of_sight,
            "stickiness": aff.stickiness,
            "restitution": aff.restitution,
            "friction": aff.friction,
            "spawn_material": aff.spawn_material.map(cf_terrain::material_name_from_id),
            "spawn_material_id": aff.spawn_material,
            "refusal_reason": aff.refusal_reason,
        }))
    }

    /// { x, y }` — resolve the material at world-space `(x, y)` against
    /// the live chunked terrain and return a `MaterialInfo` JSON with the
    /// 9 affordance flags (actor_passable, projectile_passable, diggable,
    /// anchorable, blocks_light, contact_damage, path_cost,
    /// produces_debris, produces_sound) + integrity (read from the
    /// per-pixel meta grid via `ChunkedTerrain::pixel_integrity`) + the
    /// material's `color_hex` (resolved from the on-disk material
    /// registry, with a derived fallback when the registry isn't present).
    /// Powers spec § "Material affordance tooltip" + the integrity-overlay
    /// reticle. Returns `None` when no chunked terrain is loaded.
    async fn observe_terrain_material_at(&self, x: f32, y: f32) -> Option<serde_json::Value> {
        let state = self.state.read().ok()?;
        let terrain = state.chunked_terrain.as_ref()?;
        let anchor = terrain.anchor;
        let width_px = terrain.width_px;
        let height_px = terrain.height_px;
        let material_id = terrain.material_at_world(x, y);
        let local_x = x - anchor[0];
        let local_y = y - anchor[1];
        let px = local_x.floor() as i64;
        let py = local_y.floor() as i64;
        let in_bounds = px >= 0 && py >= 0 && (px as u64) < width_px as u64 && (py as u64) < height_px as u64;
        let integrity = terrain.pixel_integrity(px, py);
        let band = cf_terrain::IntegrityBand::from_integrity(integrity);
        drop(state);
        let aff = cf_terrain::material_affordance(material_id)?;
        let produces_debris = aff.spawn_material.is_some();
        let produces_sound = aff.solid;
        let color_hex = registry_color_hex_for(material_id).unwrap_or_else(|| {
            let [r, g, b, _] = aff.overlay_rgba;
            format!("{:02X}{:02X}{:02X}", r, g, b)
        });
        Some(serde_json::json!({
            "schema_version": 1,
            "x": x,
            "y": y,
            "pixel": [px, py],
            "in_bounds": in_bounds,
            "material_id": material_id,
            "material_name": aff.name,
            "integrity": integrity,
            "band": band.as_str(),
            "color_hex": color_hex,
            "affordances": {
                "actor_passable": aff.actor_passable,
                "projectile_passable": aff.projectile_passable,
                "diggable": aff.diggable,
                "anchorable": aff.anchorable,
                "blocks_light": aff.blocks_line_of_sight,
                "contact_damage": aff.hazard,
                "path_cost": aff.path_cost,
                "produces_debris": produces_debris,
                "produces_sound": produces_sound,
            },
            "hardness": aff.hardness,
        }))
    }

    async fn dispatch(&self, command: ControlCommand) -> CommandResult {
        self.dispatch_command(command).await
    }
}

/// Locate `content/trench_templates/<id>.trench.ron` by searching the
/// canonical paths the engine may be launched from (workspace root,
/// `game/`, or one level above). Returns a typed error string for the
/// engine handler to surface in the `control.command_rejected` event.
pub(crate) fn load_trench_template(id: &str) -> Result<cf_content::TrenchTemplate, String> {
    use std::path::{Path, PathBuf};
    let filename = format!("{id}.trench.ron");
    let candidates: [PathBuf; 4] = [
        PathBuf::from("content/trench_templates").join(&filename),
        PathBuf::from("../content/trench_templates").join(&filename),
        PathBuf::from("game/content/trench_templates").join(&filename),
        PathBuf::from("../game/content/trench_templates").join(&filename),
    ];
    let mut tried: Vec<String> = Vec::new();
    for path in &candidates {
        let p: &Path = path.as_ref();
        if p.exists() {
            let text = std::fs::read_to_string(p).map_err(|e| format!("read {}: {e}", p.display()))?;
            return cf_content::TrenchTemplate::from_ron_str(&text).map_err(|e| format!("parse {}: {e}", p.display()));
        }
        tried.push(p.display().to_string());
    }
    Err(format!(
        "template id `{id}` not found under content/trench_templates/; tried [{}]",
        tried.join(", ")
    ))
}

/// Returns the set of M9C fortification ids that are currently
/// shipped + ready to instantiate. Pre-M9C every id is missing →
/// optional placeholders degrade to warnings (per spec § Notes for
/// the implementer / VAL-M9B-TEMPLATE-004); post-M9C the 23 canonical
/// kinds enumerated by [`cf_fortification::FortificationKind::ALL`]
/// are all present so the same templates promote to real placed
/// fortifications (VAL-CROSS-001 / VAL-CROSS-NEW-014).
pub(crate) fn resolved_fortifications_for_build() -> std::collections::HashSet<String> {
    cf_fortification::FortificationKind::ALL
        .iter()
        .map(|k| k.as_str().to_string())
        .collect()
}

