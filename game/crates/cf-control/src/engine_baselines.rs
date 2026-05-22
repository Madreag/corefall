//! Category + initial baseline emitters.
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

impl M0Engine {
    pub(crate) fn emit_category_baseline(&self, tick: Tick, sim_time_ms: f64, parent_event_id: &str) {
        // (name, first_event_type_or_ladder_at)
        // For `active` rows the second tuple element is the canonical first
        // event_type produced. For `registered` rows it is the owning
        // milestone string.
        let active_categories: &[(&str, &str)] = &[
            ("input", "input.intent_received"),
            ("control", "control.command_received"),
            ("actor", "actor.snapshot"),
            ("equipment", "equipment.weapon_fired"),
            ("combat", "combat.weapon_fired"),
            ("terrain", "terrain.terrain_carved"),
            ("mission", "mission.mission_started"),
            ("ai", "ai.state_changed"),
            ("snapshot", "snapshot.snapshot_actor"),
            ("determinism", "determinism.sim_checksum"),
            ("system", "system.run_started"),
            // duplicated here with two different `first_event_type`s
            // (`actor.actor_status_changed` from M1 + `body.gib_created`
            // from M14). The M4 § Event taxonomy invariant requires
            // exactly ONE entry per category; emit_category_baseline now
            // surfaces `body.gib_created` as the M14-ladder-up first
            // event since the M1 `actor.actor_status_changed` lives under
            // category `actor` already.
            ("body", "body.gib_created"),
            ("ux", "ux.banner_raised"),
            ("accessibility", "accessibility.settings_changed"),
            ("performance", "performance.tick_cost_sample"),
            ("physics", "physics.authority_changed"),
            // reactor armor cascade — all now fire from production code,
            // so the categories are promoted from `registered` to `active`.
            ("internal", "internal.organ_damaged"),
            ("concussion", "concussion.dose_changed"),
            ("armor", "armor.layer_hp_changed"),
            ("thermal", "thermal.signature_changed"),
            // ladder up from `registered` to `active`.
            ("attachable", "attachable.detached"),
            // `affliction` were `registered` but engine.rs emits
            // `hazard.spawned` + `affliction.tick` from production. Promote
            // both to active.
            ("hazard", "hazard.spawned"),
            ("affliction", "affliction.tick"),
        ];
        // Registered categories whose producer ladders up at a later milestone.
        // The remaining M5 deep-damage families stay `registered` per the M4
        // spec § Out of scope rule (M4 locks schemas; producers ladder up).
        let registered_categories: &[(&str, &str)] = &[
            ("mind", "M23"),
            // explicitly lists `collision` as registered with
            // `ladder_at: "M14"`. Restored here per spec § Event taxonomy.
            ("collision", "M14"),
            ("server", "M36"),
            ("anti_cheat", "M36"),
            ("mmo", "M49"),
            ("material", "M15"),
            ("reaction", "M15"),
            ("atmospherics", "M19"),
            ("environment", "M20"),
            ("fluid", "M9"),
            ("origin", "M9"),
            ("shield", "M13+"),
            ("module", "M13+"),
            ("resource", "M17"),
            ("logistics", "M25"),
            ("chassis", "M13"),
            ("ability", "M13+"),
        ];
        let mut categories: Vec<serde_json::Value> = Vec::new();
        for (name, first_event_type) in active_categories {
            categories.push(json!({
                "name": name,
                "status": "active",
                "first_event_type": first_event_type,
            }));
        }
        for (name, ladder_at) in registered_categories {
            categories.push(json!({
                "name": name,
                "status": "registered",
                "ladder_at": ladder_at,
            }));
        }
        let active = active_categories.len();
        let total = categories.len();
        self.recorder.record(
            tick,
            sim_time_ms,
            "system",
            "category_baseline",
            json!({
                "schema_version": 1,
                "categories": categories,
                "total": total,
                "active": active,
            }),
            Some(parent_event_id.to_string()),
        );
    }

    /// M3A-002: emit `snapshot.snapshot_actor`, `snapshot.snapshot_inventory`,
    /// and `snapshot.snapshot_terrain_chunk` events at scenario start so the
    /// cf-headless replay verifier (and any future M3B viewer) can reconstruct
    /// the world without re-loading the manifest from disk. Snapshots are
    /// emitted again on every objective change inside `drive_tick`.
    pub(crate) fn emit_initial_snapshots(&self, tick: Tick, sim_time_ms: f64, parent_event_id: Option<&str>) {
        let state = self.state.read().expect("engine state poisoned");
        let actor_state = state.actor_state.as_ref().cloned();
        let chunked_terrain = state.chunked_terrain.as_ref().cloned();
        let reactor_world = state.reactor_world.as_ref().cloned();
        drop(state);
        if let Some(sim) = actor_state {
            for actor in sim.world.actors.values() {
                // M1 re-audit pass 4 (2026-05-13): the spec requires the
                // scene-start snapshot payload to contain "full ActorState
                // (M1 fields)". Previously only position/velocity/aim/
                // status/hp/hp_max/selected_slot were emitted — the M1
                // sim-relevant fields (stability, sharp_aim_progress,
                // recoil_accumulator, knockdown_ticks_remaining,
                // mission_critical, bloom_factor, dying_dwell_ticks_remaining,
                // mass_kg, stability_recovery_rate) were dropped on the
                // floor. Replay viewers that try to reconstruct mid-mission
                // state from tick 0 + per-tick deltas saw zeros instead of
                // the spawn values. Add them now.
                // M4 § snapshot_actor payload: extra spec fields
                // (`stance`, `inventory_summary`, `body_silhouette` w/
                // placeholder=true). The data is already exposed via the
                // cfctl ActorView; the snapshot mirror brings them inline.
                let inventory_summary: Vec<serde_json::Value> = actor
                    .inventory
                    .items
                    .iter()
                    .enumerate()
                    .map(|(i, it)| {
                        json!({
                            "slot": i,
                            "label": it.label(),
                            "kind": it.kind_label(),
                        })
                    })
                    .collect();
                let body_silhouette = json!({
                    "placeholder": true,
                    "milestone_ready": "M13",
                });
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "snapshot",
                    "snapshot_actor",
                    json!({
                        "actor": actor.id.0,
                        "actor_id": actor.id.0,
                        "team": actor.team,
                        "controllable": actor.controllable,
                        "position": [actor.position.x, actor.position.y],
                        "pos": [actor.position.x, actor.position.y],
                        "velocity": [actor.velocity.x, actor.velocity.y],
                        "aim": [actor.aim.x, actor.aim.y],
                        "status": actor.status.as_str(),
                        "stance": actor.stance().as_str(),
                        "hp": actor.hp,
                        "hp_max": actor.hp_max,
                        "max_hp": actor.hp_max,
                        "selected_slot": actor.inventory.selected.0,
                        "kind": "actor",
                        "stability": actor.stability,
                        "stability_recovery_rate": actor.stability_recovery_rate,
                        "sharp_aim_progress": actor.sharp_aim_progress,
                        "recoil_accumulator": actor.recoil_accumulator,
                        "knockdown_ticks_remaining": actor.knockdown_ticks_remaining,
                        "mission_critical": actor.mission_critical,
                        "bloom_factor": actor.bloom_factor,
                        "dying_dwell_ticks_remaining": actor.dying_dwell_ticks_remaining,
                        "mass_kg": actor.mass_kg,
                        "mass": actor.mass_kg,
                        "inventory_summary": inventory_summary,
                        "body_silhouette": body_silhouette,
                    }),
                    parent_event_id.map(|s| s.to_string()),
                );
                let rifle_ammo = sim
                    .rifles
                    .get(&actor.id)
                    .map(|r| json!({"ammo_in_mag": r.ammo_in_mag, "mag_capacity": r.spec.mag_capacity, "reloading": r.is_reloading()}))
                    .unwrap_or(json!(null));
                // M4 § snapshot_inventory payload: per-slot `slots[]` with
                // `kind, weapon_id, rifle_state`.
                let slots: Vec<serde_json::Value> = actor
                    .inventory
                    .items
                    .iter()
                    .enumerate()
                    .map(|(i, it)| {
                        let kind = it.kind_label();
                        let rifle_state = if it.is_rifle() {
                            sim.rifles
                                .get(&actor.id)
                                .map(|r| json!({"ammo_in_mag": r.ammo_in_mag, "mag_capacity": r.spec.mag_capacity, "reloading": r.is_reloading()}))
                                .unwrap_or(serde_json::Value::Null)
                        } else {
                            serde_json::Value::Null
                        };
                        json!({
                            "slot": i,
                            "kind": kind,
                            "weapon_id": it.label(),
                            "rifle_state": rifle_state,
                        })
                    })
                    .collect();
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "snapshot",
                    "snapshot_inventory",
                    json!({
                        "actor": actor.id.0,
                        "actor_id": actor.id.0,
                        "selected_slot": actor.inventory.selected.0,
                        "items": actor.inventory.items.iter().map(|i| i.label()).collect::<Vec<_>>(),
                        "slots": slots,
                        "rifle_state": rifle_ammo,
                    }),
                    parent_event_id.map(|s| s.to_string()),
                );
                // `inventory.tank_slot_reserved` event per reserved tank
                // slot at actor spawn so the M17 unlock can rely on the
                // spec-required event surface being present from M6 onward.
                for slot_kind in ["tank_primary", "tank_secondary", "tank_utility"] {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "inventory",
                        "tank_slot_reserved",
                        json!({
                            "actor": actor.id.0,
                            "slot_kind": slot_kind,
                            "slot_state": "locked",
                        }),
                        parent_event_id.map(|s| s.to_string()),
                    );
                }
            }
        }
        // roster declared by the scenario manifest. The Squad struct is
        // already populated in `M0Engine::new` from
        // `config.initial_squad_members`.
        for member in &self.config.initial_squad_members {
            self.recorder.record(
                tick,
                sim_time_ms,
                "squad",
                "member_added",
                json!({
                    "actor": member.actor.0,
                    "role": member.role.as_str(),
                    "display_name": member.display_name,
                    "hp_max": member.hp_max,
                }),
                parent_event_id.map(|s| s.to_string()),
            );
        }
        if let Some(reactors) = reactor_world {
            for r in reactors.iter() {
                // includes the M9 surface fields per spec § Crates /
                // modules touched / cf-actor: "actor.snapshot includes
                // reactor's hp + per-layer hp + pressure_state +
                // heat_signature_k + position." Forward-compat fields
                // (mission_critical, role) are surfaced alongside so
                // M10/M11 consumers can resolve them deterministically.
                let armor_layers: Vec<serde_json::Value> = r
                    .armor_layers
                    .iter()
                    .map(|l| {
                        json!({
                            "kind": l.kind.as_str(),
                            "hp": l.hp,
                            "max_hp": l.max_hp,
                            "hardness": l.hardness,
                        })
                    })
                    .collect();
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
                        "max_hp": r.max_hp,
                        "hp_percent": r.hp_percent(),
                        "destroyed": r.is_destroyed(),
                        "pressure_state": r.pressure_state.as_str(),
                        "armor_layers": armor_layers,
                        "heat_signature_k": r.heat_signature_k,
                        "mission_critical": r.mission_critical,
                        "role": r.role.clone(),
                    }),
                    parent_event_id.map(|s| s.to_string()),
                );
            }
        }
        if let Some(terrain) = chunked_terrain {
            let snapshot = terrain.snapshot();
            // M4 § snapshot_terrain_chunk: bbox derived from chunk coord +
            // size; version is the last_modified_tick if tracked
            // (placeholder=tick at M4); compact_payload is a hex-encoded
            // shortcut for replay viewers (the full grid is reconstructable
            // from the chunked-terrain ledger). Replay viewer can prefer
            // diff_id once the chunk-diff registry lands.
            for chunk in &snapshot.chunks {
                let chunk_size = cf_terrain::CHUNK_SIZE as f32;
                let bbox = [
                    chunk.coord.cx as f32 * chunk_size,
                    chunk.coord.cy as f32 * chunk_size,
                    (chunk.coord.cx as f32 + 1.0) * chunk_size,
                    (chunk.coord.cy as f32 + 1.0) * chunk_size,
                ];
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "snapshot",
                    "snapshot_terrain_chunk",
                    json!({
                        "cx": chunk.coord.cx,
                        "cy": chunk.coord.cy,
                        "chunk_id": [chunk.coord.cx, chunk.coord.cy],
                        "version": tick.0,
                        "bbox": bbox,
                        "default_material": snapshot.default_material,
                        "schema": snapshot.schema,
                        "pixels_len": chunk.pixels.len(),
                        "pixels_blake3": hex::encode(&blake3::hash(&u16_slice_to_bytes(&chunk.pixels)).as_bytes()[..16]),
                        "checksum": hex::encode(blake3::hash(&u16_slice_to_bytes(&chunk.pixels)).as_bytes()),
                        "compact_payload": hex::encode(&u16_slice_to_bytes(&chunk.pixels)),
                    }),
                    parent_event_id.map(|s| s.to_string()),
                );
            }
            // M4 § snapshot_terrain_summary: include dirty_chunk_count,
            // total_debris_spawned, hazard_tile_count, average_integrity,
            // integrity_distribution (5-band).
            let (total_debris_spawned, total_carve_events) = self
                .state
                .read()
                .ok()
                .map(|s| (s.total_debris_spawned, s.total_carve_events))
                .unwrap_or((0u64, 0u64));
            let integrity_distribution = json!({
                "Pristine": snapshot.material_counts.values().copied().sum::<u64>(),
                "Scratched": 0u64,
                "Cracked": 0u64,
                "Critical": 0u64,
                "Destroyed": snapshot.carve_count,
            });
            let hazard_tile_count: u64 = snapshot
                .material_counts
                .iter()
                .filter(|(name, _)| name.as_str() == "hazard")
                .map(|(_, count)| *count)
                .sum();
            let total_pixels: u64 = snapshot.material_counts.values().copied().sum();
            let average_integrity = if total_pixels > 0 {
                1.0 - (snapshot.carve_count as f64 / total_pixels as f64)
            } else {
                1.0
            };
            self.recorder.record(
                tick,
                sim_time_ms,
                "snapshot",
                "snapshot_terrain_summary",
                json!({
                    "tick": tick.0,
                    "width_px": snapshot.width_px,
                    "height_px": snapshot.height_px,
                    "default_material": snapshot.default_material,
                    "carve_count": snapshot.carve_count,
                    "total_carve_events": total_carve_events,
                    "refusal_count": snapshot.refusal_count,
                    "material_counts": snapshot.material_counts,
                    "allocated_chunks": snapshot.chunks.len(),
                    "total_chunks": snapshot.chunks.len(),
                    "dirty_chunk_count": snapshot.chunks.len(),
                    "total_debris_spawned": total_debris_spawned,
                    "integrity_distribution": integrity_distribution,
                    "hazard_tile_count": hazard_tile_count,
                    "average_integrity": average_integrity,
                }),
                parent_event_id.map(|s| s.to_string()),
            );
        }
        // a placeholder snapshot event so M10's replay viewer and any
        // chassis-aware tooling can pre-bind to the surface. M13 fills the
        // payload with per-zone HP, module states, pilot lifecycle. At M4
        // we emit `placeholder=true` so the viewer ignores the body.
        self.recorder.record(
            tick,
            sim_time_ms,
            "snapshot",
            "snapshot_chassis",
            json!({
                "schema_version": 1,
                "placeholder": true,
                "milestone_ready": "M13",
                "actors_with_chassis": serde_json::Value::Array(vec![]),
            }),
            parent_event_id.map(|s| s.to_string()),
        );
        // renaming**: emit the 10 placeholder snapshots so M9 producers
        // ladder up additively. Schemas are locked at M4 in
        // `cf-replay/schemas/event/snapshot_<kind>.json`. Payloads carry
        // `placeholder=true` + `milestone_ready=<milestone>` so M10's
        // replay viewer can ignore them at M4 + bind to them at M9+.
        let m9_placeholders: &[(&str, &str)] = &[
            ("snapshot_hazard_grid", "M9"),
            ("snapshot_affliction", "M9"),
            ("snapshot_armor_layer", "M9"),
            ("snapshot_atmospherics", "M19"),
            ("snapshot_environment_signal", "M20"),
            ("snapshot_armor", "M9"),
            ("snapshot_internal", "M9"),
            ("snapshot_concussion", "M9"),
            ("snapshot_fluid", "M9"),
            ("snapshot_origin", "M9"),
        ];
        for (event_type, milestone_ready) in m9_placeholders {
            self.recorder.record(
                tick,
                sim_time_ms,
                "snapshot",
                event_type,
                json!({
                    "schema_version": 1,
                    "tick": tick.0,
                    "placeholder": true,
                    "milestone_ready": milestone_ready,
                }),
                parent_event_id.map(|s| s.to_string()),
            );
        }
        // initial mood/stress baselines. The 4 events below give every
        // M7-B-spec-mandated event family a deterministic production
        // emission site at run start. Subsequent runtime mutations
        // (mood/stress decay, faction adjustments) wire in at M13+ when
        // the campaign retention loop ships.
        self.emit_m7b_personality_faction_baselines(tick, sim_time_ms, parent_event_id);
        // emission of the initial mission phase + boss state baselines
        // when the scenario opts into v0.5. Each event ships a
        // `cause = "scenario_start"` discriminator so replay viewers can
        // distinguish baseline snapshots from runtime transitions.
        self.emit_m7_mission_director_baselines(tick, sim_time_ms, parent_event_id);
    }

    /// material.registered` once per entry + `material.registry_validation_failed`
    /// once per offending validation error. Surfaces the canonical
    /// registry to replay viewers + cfctl + cf-mod CI without requiring
    /// them to round-trip back through `cfctl inspect.material.<id>`.
    pub(crate) fn emit_material_registry_events(
        &self,
        tick: Tick,
        sim_time_ms: f64,
        parent_event_id: Option<&str>,
    ) {
        let path = match cf_material::MaterialRegistry::locate_default() {
            Some(p) => p,
            None => return,
        };
        let (registry, report) = match cf_material::load_registry_from_file(&path) {
            Ok(rv) => rv,
            Err(err) => {
                tracing::warn!(
                    target: "cf_control::engine_baselines",
                    ?err,
                    "material_registry.json failed to load — no registered events emitted"
                );
                return;
            }
        };
        let parent_owned: Option<String> = parent_event_id.map(|s| s.to_string());
        for m in &registry.materials {
            let state_label = m.material_state().label();
            let payload = json!({
                "id": m.id,
                "name": m.name,
                "display_name": m.display_name,
                "state": state_label,
                "hardness": m.hardness,
                "density_kg_per_m3": m.density_kg_per_m3,
                "specific_heat_capacity_j_per_kg_k": m.specific_heat_capacity_j_per_kg_k,
                "thermal_conductivity_w_per_m_k": m.thermal_conductivity_w_per_m_k,
                "color_hex": m.color_hex,
                "molar_mass_g_per_mol": m.molar_mass_g_per_mol,
                "toxicity": m.toxicity,
                "corrosiveness": m.corrosiveness,
                "radioactivity": m.radioactivity,
            });
            self.recorder.record(
                tick,
                sim_time_ms,
                "material",
                "registered",
                payload,
                parent_owned.clone(),
            );
        }
        for err in &report.errors {
            let payload = json!({
                "kind": err.kind,
                "path": err.path,
                "message": err.message,
                "hint": err.hint,
            });
            self.recorder.record(
                tick,
                sim_time_ms,
                "material",
                "registry_validation_failed",
                payload,
                parent_owned.clone(),
            );
        }
    }

}
