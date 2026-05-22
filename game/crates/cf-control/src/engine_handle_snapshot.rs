//! engine_handle::snapshot_impl — extracted from engine_handle.rs.

#![allow(unused_imports, dead_code, clippy::too_many_arguments)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use serde_json::{json, Value};

use cf_actor::{ActorId, ControlIntent, Vec2};
use cf_replay::{ArtifactItem, BuildInfo, BundleInputs, CapabilitiesBlock, CaptureConfig, ChecksumConfig, PerfSample, Recorder, RunManifest, SceneInfo, SettingsBlock, TestRecord};

use crate::engine::*;
use crate::server::{async_trait, CommandResult, ControlCommand, EngineHandle, SettingsPatch};
use crate::state::{ActorView, ObserveFrame, ObserveSettings, RunStatus};
use crate::{Settings, SCHEMA_VERSION};

impl M0Engine {
    pub(crate) fn snapshot_impl(&self, _filter: Option<&str>) -> ObserveFrame {
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
}
