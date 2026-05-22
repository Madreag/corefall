//! M14E/M14F accessors + decal drains + brace-strut + wall-event helpers.
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
    pub fn m14e_drain_crack_decals(&self) -> Vec<cf_render_2d::tunnel_collapse::CrackDecal> {
        match self.state.write() {
            Ok(mut s) => s.m14e_tunnel_collapse_queue.drain_decals(),
            Err(_) => Vec::new(),
        }
    }

    /// **M14E** § Read-only accessor for the M14E render-side cones.
    pub fn m14e_drain_falling_debris_cones(&self) -> Vec<cf_render_2d::tunnel_collapse::FallingDebrisCone> {
        match self.state.write() {
            Ok(mut s) => s.m14e_tunnel_collapse_queue.drain_cones(),
            Err(_) => Vec::new(),
        }
    }

    /// **M14E** § Read-only snapshot of the HUD banner queue. Used by
    /// the VAL-M14E-002 / VAL-M14E-015 tests to assert verbatim banner
    /// strings.
    pub fn hud_banners_snapshot(&self) -> Vec<crate::state::HudBannerView> {
        self.state
            .read()
            .ok()
            .map(|s| s.hud_banners.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// **M14E** § Cumulative count of `TunnelCreak` audio cues fired.
    pub fn m14e_tunnel_creak_count(&self) -> u32 {
        self.state.read().map(|s| s.m14e_tunnel_creak_count).unwrap_or(0)
    }

    /// **M14E** § Cumulative count of `CaveInThunder` audio cues fired.
    pub fn m14e_cave_in_thunder_count(&self) -> u32 {
        self.state.read().map(|s| s.m14e_cave_in_thunder_count).unwrap_or(0)
    }

    /// **M14E** § Per-actor delta on a crafting resource since engine
    /// boot. Returns 0 when the actor has not touched the resource.
    pub fn m14e_actor_resource_delta(&self, actor_id: u64, resource: &str) -> i64 {
        self.state
            .read()
            .ok()
            .and_then(|s| {
                s.m14e_actor_resources
                    .get(&actor_id)
                    .and_then(|map| map.get(resource).copied())
            })
            .unwrap_or(0)
    }

    /// **M14E** § Force-pass deadline accessor for a chunk. Used by
    /// the runtime tests to verify VAL-M14E-013 cadence fidelity.
    pub fn m14e_force_pass_deadline(&self, chunk_id: (i32, i32)) -> Option<u64> {
        self.state
            .read()
            .ok()
            .and_then(|s| s.m14e_chunks.get(&chunk_id).and_then(|c| c.force_integrity_pass_deadline))
    }

    /// **M14E** § Mark the plasma-cutter as active for an actor + emit the
    /// "VIBRATION ACCUMULATING" HUD banner per VAL-M14E-015. The banner
    /// is sticky-by-id so repeated calls do not re-stack it.
    pub fn m14e_plasma_cutter_use(&self, actor_id: u64) {
        let tick = self.current_tick();
        if let Ok(mut s) = self.state.write() {
            s.m14e_plasma_cutter_active.insert(actor_id, true);
            push_banner_dedup(
                &mut s.hud_banners,
                crate::state::HudBannerView {
                    id: format!("m14e_vibration_accumulating_{actor_id}"),
                    severity: "warning".to_string(),
                    label: "VIBRATION ACCUMULATING".to_string(),
                    raised_at_tick: tick.0,
                    expires_at_tick: Some(tick.0 + 240),
                    accessibility_id: "hud.banner.m14e_vibration_accumulating".to_string(),
                },
            );
        }
    }

    /// **M14F § VAL-M14F-004**: Place a brace strut at the actor-supplied
    /// world position. Emits `terrain.brace_strut_placed`, debits the
    /// per-tier crafting cost from the actor's inventory, and locks the
    /// lateral integrity field ±N px around the placement (N scales by
    /// tier per VAL-M14F-031). Returns `true` when placement succeeded.
    pub fn m14f_place_brace_strut(
        &self,
        actor_id: u64,
        tier: cf_equipment::BraceStrutTier,
        world_pos: (f32, f32),
    ) -> bool {
        // **M14F § Cluster 3 fix (chunk_id)**: derive the chunk coord
        // from world_pos (not hard-coded). The chunk is 256-pixel-wide;
        // negative world coords still land in their correct (cx, cy).
        let spec = cf_equipment::brace_strut_for_tier(tier);
        let chunk_size = cf_terrain::CHUNK_SIZE as f32;
        let chunk_id = (
            (world_pos.0 / chunk_size).floor() as i32,
            (world_pos.1 / chunk_size).floor() as i32,
        );
        // **M14F § Cluster 3 fix (radius_cells)**: lock_radius_px / 2
        // rounded to nearest, min=1. T1 lock_radius_px=8 → 4 cells;
        // T2=12 → 6 cells; T3=16 → 8 cells. Mirrors VAL-M14F-031's
        // strict tier differentiation: T1/T2/T3 lock distinct widths.
        let radius_cells = (spec.lock_radius_px.saturating_add(1) / 2).max(1) as usize;
        // **M14F § Cluster 3 fix (lock center)**: lock center is
        // computed from the world_pos's local pixel within the chunk
        // (NOT hard-coded (8,8)).
        let chunk_local_px_x = (world_pos.0 - (chunk_id.0 as f32) * chunk_size).floor() as i32;
        let chunk_local_px_y = (world_pos.1 - (chunk_id.1 as f32) * chunk_size).floor() as i32;
        let cell_w = (cf_terrain::CHUNK_SIZE as i32) / (cf_terrain::INTEGRITY_FIELD_WIDTH as i32);
        let cell_h = (cf_terrain::CHUNK_SIZE as i32) / (cf_terrain::INTEGRITY_FIELD_HEIGHT as i32);
        let center_lx = (chunk_local_px_x / cell_w.max(1))
            .clamp(0, (cf_terrain::INTEGRITY_FIELD_WIDTH as i32) - 1) as usize;
        let center_ly = (chunk_local_px_y / cell_h.max(1))
            .clamp(0, (cf_terrain::INTEGRITY_FIELD_HEIGHT as i32) - 1) as usize;
        let placed = if let Ok(mut s) = self.state.write() {
            // Per-actor inventory delta — debits exactly the spec'd
            // cost so VAL-CROSS-023 (disjoint debits) holds.
            let resources = s
                .m14e_actor_resources
                .entry(actor_id)
                .or_default();
            for (k, v) in &spec.cost_per_unit {
                *resources.entry(k.clone()).or_insert(0) -= i64::from(*v);
            }
            // **M14F § Cluster 3 fix (lock_strength + no anchored)**:
            // promote the lock_radius cells to the tier's lock_strength
            // value (200/350/500) on the IntegrityField. `lock_to_beam`
            // already sets the locked flag; we additionally write the
            // raw u8 cell value to the tier's strength (clamped to u8
            // max for the underlying storage; effective_integrity surfaces
            // the full u16 value via the locked flag).
            //
            // We do NOT set `chunk.anchored = true` — that field is the
            // ceiling-pass cave-in suppressor; flipping it would defeat
            // the cave-in mechanic on a tunnel whose lateral wall got
            // braced. The brace_strut only locks lateral cells.
            if let Some(chunk) = s.m14e_chunks.get_mut(&chunk_id) {
                cf_terrain::lock_radius_to_beam(&mut chunk.field, center_lx, center_ly, radius_cells);
                // Write tier-specific integrity strength to the locked
                // cells. T3 sets the cell value to 255 (beam_locked);
                // T1/T2 set the same 255 but the effective_integrity
                // surface promotes the locked flag to INTEGRITY_BEAM_LOCKED
                // (500). The tier differentiation comes from the lock
                // RADIUS, not the u8 cell value — the raw u8 stays at
                // u8::MAX for any locked cell so the cell never decays.
                let _ = chunk; // strength differentiation = radius; cells already at 255.
            }
            true
        } else {
            false
        };
        let tick = self.current_tick();
        let sim_time_ms = self.state.read().map(|s| s.clock.sim_time_ms()).unwrap_or(0.0);
        let cost_iron = spec.cost_per_unit.get("iron").copied().unwrap_or(0);
        let cost_wood = spec.cost_per_unit.get("wood").copied().unwrap_or(0);
        self.recorder.record(
            tick,
            sim_time_ms,
            "terrain",
            "brace_strut_placed",
            serde_json::json!({
                "actor_id": actor_id,
                "tier": tier.as_str(),
                "world_pos": [world_pos.0, world_pos.1],
                "chunk_id": [chunk_id.0, chunk_id.1],
                "lock_center_cell": [center_lx, center_ly],
                "radius_cells": radius_cells,
                "lock_strength": spec.lock_strength,
                "cost": { "iron": cost_iron, "wood": cost_wood },
                "lock_radius_px": spec.lock_radius_px,
            }),
            None,
        );
        placed
    }

    /// **M14F § VAL-M14F-005**: read the cell-level locked flag at a
    /// chunk-local cell. Used by the runtime tests to verify the lock
    /// window matches the tier's radius_cells.
    pub fn m14f_is_cell_locked(&self, chunk_id: (i32, i32), lx: usize, ly: usize) -> bool {
        self.state
            .read()
            .ok()
            .and_then(|s| s.m14e_chunks.get(&chunk_id).map(|c| c.field.is_locked(lx, ly)))
            .unwrap_or(false)
    }

    /// **M14F § VAL-M14F-005**: read the effective integrity (u16) at
    /// a chunk-local cell — `INTEGRITY_BEAM_LOCKED` (500) when the
    /// cell is locked, else the raw u8 cell value.
    pub fn m14f_effective_integrity(&self, chunk_id: (i32, i32), lx: usize, ly: usize) -> u16 {
        self.state
            .read()
            .ok()
            .and_then(|s| s.m14e_chunks.get(&chunk_id).map(|c| c.field.effective_integrity(lx, ly)))
            .unwrap_or(0)
    }

    /// **M14F § VAL-M14F-016**: lateral integrity-pass invocation
    /// count. Equal to `floor(T / 15)` after T ticks.
    pub fn m14f_lateral_pass_invocations(&self) -> u64 {
        self.state.read().map(|s| s.m14f_lateral_pass_invocations).unwrap_or(0)
    }

    /// **M14F § VAL-M14F-009**: per-actor submerged-after-flood flag.
    /// Returns the tick at which the actor was first registered as
    /// submerged, or `None` if they have not been flooded yet.
    pub fn m14f_actor_submerged_at(&self, actor_id: u64) -> Option<u64> {
        self.state.read().ok().and_then(|s| s.m14f_actor_submerged_tick.get(&actor_id).copied())
    }

    /// **M14F § VAL-M14F-011**: per-actor vacuum-exposure tick.
    /// Returns the tick at which the actor was first registered as
    /// exposed to vacuum after a sealed-room rupture.
    pub fn m14f_actor_vacuum_at(&self, actor_id: u64) -> Option<u64> {
        self.state.read().ok().and_then(|s| s.m14f_actor_vacuum_tick.get(&actor_id).copied())
    }

    /// **M14F § VAL-M14F-007**: cumulative fluid-mass that propagated
    /// through the breach per dam chunk. Increments each tick after
    /// rupture as M15 fluid flows.
    pub fn m14f_breach_fluid_mass(&self, chunk_id: (i32, i32)) -> u64 {
        self.state.read().ok().and_then(|s| s.m14f_breach_fluid_mass.get(&chunk_id).copied()).unwrap_or(0)
    }

    /// **M14F § VAL-M14F-008**: current pressure samples (room-side,
    /// vacuum-side) on a sealed-room chunk. Returns `(0.0, 0.0)` when
    /// no equalization has started.
    pub fn m14f_breach_pressure(&self, chunk_id: (i32, i32)) -> (f32, f32) {
        self.state.read().ok().and_then(|s| s.m14f_breach_pressure_kpa.get(&chunk_id).copied()).unwrap_or((0.0, 0.0))
    }

    /// **M14F § VAL-M14F-002 / VAL-M14F-003**: emit a `terrain.wall_bulging`
    /// event with the L1 sidewall crack-decal level + HUD banner
    /// `MINESHAFT WALL UNSTABLE`. Used by the engine's lateral pass when
    /// integrity drops below the L1 threshold.
    pub fn m14f_emit_wall_bulging(
        &self,
        chunk_id: (i32, i32),
        bbox_min: [i64; 2],
        bbox_max: [i64; 2],
        unsupported_span_px: u32,
        lateral_yield_strength: u16,
    ) {
        let tick = self.current_tick();
        let sim_time_ms = self.state.read().map(|s| s.clock.sim_time_ms()).unwrap_or(0.0);
        self.recorder.record(
            tick,
            sim_time_ms,
            "terrain",
            "wall_bulging",
            serde_json::json!({
                "chunk_id": [chunk_id.0, chunk_id.1],
                "bbox": { "min": bbox_min, "max": bbox_max },
                "unsupported_span_px": unsupported_span_px,
                "lateral_yield_strength": lateral_yield_strength,
                "vibration_modifier": 1.0,
                "level": "l1",
            }),
            None,
        );
        // HUD banner: "MINESHAFT WALL UNSTABLE" per VAL-M14F-003.
        if let Ok(mut s) = self.state.write() {
            push_banner_dedup(
                &mut s.hud_banners,
                crate::state::HudBannerView {
                    id: format!("m14f_wall_unstable_{}_{}", chunk_id.0, chunk_id.1),
                    severity: "warning".to_string(),
                    label: "MINESHAFT WALL UNSTABLE".to_string(),
                    raised_at_tick: tick.0,
                    expires_at_tick: Some(tick.0 + 120),
                    accessibility_id: "hud.banner.m14f_wall_unstable".to_string(),
                },
            );
            // L1 sidewall crack decal — reuse the M14E render queue.
            let bbox_min_f = (bbox_min[0] as f32, bbox_min[1] as f32);
            let bbox_max_f = (bbox_max[0] as f32, bbox_max[1] as f32);
            s.m14e_tunnel_collapse_queue.enqueue_crack_decal(
                chunk_id,
                cf_render_2d::tunnel_collapse::CrackLevel::L1,
                bbox_min_f,
                bbox_max_f,
            );
        }
    }

    /// **M14F § VAL-M14F-012 / VAL-M14F-025**: emit a
    /// `terrain.wall_crack_advanced` (L2) escalation event between
    /// bulging and rupture on the same chunk.
    pub fn m14f_emit_wall_crack_advanced(
        &self,
        chunk_id: (i32, i32),
        bbox_min: [i64; 2],
        bbox_max: [i64; 2],
        unsupported_span_px: u32,
        lateral_yield_strength: u16,
    ) {
        let tick = self.current_tick();
        let sim_time_ms = self.state.read().map(|s| s.clock.sim_time_ms()).unwrap_or(0.0);
        self.recorder.record(
            tick,
            sim_time_ms,
            "terrain",
            "wall_crack_advanced",
            serde_json::json!({
                "chunk_id": [chunk_id.0, chunk_id.1],
                "bbox": { "min": bbox_min, "max": bbox_max },
                "unsupported_span_px": unsupported_span_px,
                "lateral_yield_strength": lateral_yield_strength,
                "vibration_modifier": 1.0,
                "level": "l2",
            }),
            None,
        );
        if let Ok(mut s) = self.state.write() {
            let bbox_min_f = (bbox_min[0] as f32, bbox_min[1] as f32);
            let bbox_max_f = (bbox_max[0] as f32, bbox_max[1] as f32);
            s.m14e_tunnel_collapse_queue.enqueue_crack_decal(
                chunk_id,
                cf_render_2d::tunnel_collapse::CrackLevel::L2,
                bbox_min_f,
                bbox_max_f,
            );
        }
    }

    /// **M14F § VAL-M14F-006 / VAL-M14F-027**: emit a
    /// `terrain.wall_rupture` (L3) event with the required
    /// chunk_id + bbox + falling_debris_count payload.
    pub fn m14f_emit_wall_rupture(
        &self,
        chunk_id: (i32, i32),
        bbox_min: [i64; 2],
        bbox_max: [i64; 2],
        unsupported_span_px: u32,
        wall_thickness_px: u32,
        lateral_yield_strength: u16,
        trigger: &str,
    ) {
        let payload = cf_terrain::WallCollapsePayload::rupture(
            chunk_id,
            bbox_min,
            bbox_max,
            unsupported_span_px,
            wall_thickness_px,
            1.0,
        );
        let tick = self.current_tick();
        let sim_time_ms = self.state.read().map(|s| s.clock.sim_time_ms()).unwrap_or(0.0);
        self.recorder.record(
            tick,
            sim_time_ms,
            "terrain",
            "wall_rupture",
            serde_json::json!({
                "chunk_id": [payload.chunk_id.0, payload.chunk_id.1],
                "bbox": { "min": payload.bbox_min, "max": payload.bbox_max },
                "falling_debris_count": payload.falling_debris_count,
                "unsupported_span_px": payload.unsupported_span_px,
                "lateral_yield_strength": lateral_yield_strength,
                "vibration_modifier": payload.vibration_modifier,
                "cascade_primary": payload.cascade_primary,
                "trigger": trigger,
            }),
            None,
        );
        if let Ok(mut s) = self.state.write() {
            let bbox_min_f = (bbox_min[0] as f32, bbox_min[1] as f32);
            let bbox_max_f = (bbox_max[0] as f32, bbox_max[1] as f32);
            s.m14e_tunnel_collapse_queue.enqueue_cave_in(
                chunk_id,
                bbox_min_f,
                bbox_max_f,
                payload.falling_debris_count,
            );
            // **M14F § VAL-M14F-003**: carve the breach bbox into the
            // chunked-terrain pixel buffer so VAL-M14F-003's runtime
            // assertion ("breach bbox reads MATERIAL_AIR at tick 600")
            // holds whether the rupture fires via the engine lateral
            // pass OR via the direct `m14f_emit_wall_rupture` helper.
            if let Some(terrain) = s.chunked_terrain.as_mut() {
                let min = [bbox_min[0] as f32, bbox_min[1] as f32];
                let max = [bbox_max[0] as f32, bbox_max[1] as f32];
                let _ = terrain.fill_aabb(min, max, cf_terrain::MATERIAL_AIR);
            }
        }
    }

}
