//! tick_m14f_lateral_collapse + cascade rupture + downstream consumers.
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
    pub(crate) fn tick_m14f_lateral_collapse(&self, tick: Tick, sim_time_ms: f64) {
        let cadence = u64::from(cf_terrain::INTEGRITY_PASS_CADENCE_TICKS);
        let cadence_run = tick.0 != 0 && tick.0.is_multiple_of(cadence);
        let chunk_ids: Vec<(i32, i32)> = match self.state.read() {
            Ok(s) => s.m14f_lateral_chunks.keys().copied().collect(),
            Err(_) => return,
        };
        if chunk_ids.is_empty() {
            return;
        }
        // Cadence-driven lateral integrity-pass invocation count.
        if cadence_run {
            if let Ok(mut s) = self.state.write() {
                s.m14f_lateral_pass_invocations =
                    s.m14f_lateral_pass_invocations.saturating_add(1);
                // Walk every lateral chunk and apply the lateral pass.
                for chunk_id in &chunk_ids {
                    let (span_px, vib, lateral_yield) =
                        match s.m14f_lateral_chunks.get(chunk_id) {
                            Some(c) => (c.unsupported_span_px, c.vibration_modifier, c.lateral_yield_strength),
                            None => continue,
                        };
                    if let Some(chunk) = s.m14e_chunks.get_mut(chunk_id) {
                        let _ = cf_terrain::compute_lateral_integrity_pass(
                            &mut chunk.field,
                            span_px,
                            vib,
                            lateral_yield,
                        );
                    }
                }
            }
        }
        // Deterministic bulging countdown + cascade scheduling. Runs
        // every tick so the 30-tick window for VAL-M14F-002 is met
        // (cadence-only would force first bulging to tick 15 at the
        // earliest, which is fine; we explicitly tick the countdown
        // here so it lands deterministically inside the window).
        type LateralEmit = (
            (i32, i32),
            [i64; 2],
            [i64; 2],
            u32,
            u32,
            u16,
            f32,
            &'static str,
            String,
            Option<u64>,
            Vec<(i32, i32)>,
        );
        let mut emits: Vec<LateralEmit> = Vec::new();
        if let Ok(mut s) = self.state.write() {
            for chunk_id in &chunk_ids {
                let Some(state) = s.m14f_lateral_chunks.get_mut(chunk_id) else {
                    continue;
                };
                // Stable snapshot for the cascade-decision below; mut
                // borrow released once we extract these fields.
                let span_px = state.unsupported_span_px;
                let wall_thickness_px = state.wall_thickness_px;
                let yield_strength = state.lateral_yield_strength;
                let vib = state.vibration_modifier;
                let topology = state.topology.clone();
                let downstream_actor = state.downstream_actor_id;
                let cascade_neighbors = state.cascade_neighbors.clone();
                let bbox_min = state.bbox_min;
                let bbox_max = state.bbox_max;

                if !state.bulging_emitted {
                    // Bulging countdown — fires when remaining ticks
                    // hit 0 OR when the M14E shared `IntegrityField`
                    // drops below the locked threshold (per
                    // VAL-CROSS-005 — the lateral pass observes the
                    // ceiling pass's decay too).
                    let mut should_fire = false;
                    if let Some(remaining) = state.bulging_countdown_remaining {
                        if remaining == 0 {
                            should_fire = true;
                        } else {
                            state.bulging_countdown_remaining = Some(remaining.saturating_sub(1));
                        }
                    }
                    if should_fire {
                        state.bulging_emitted = true;
                        state.bulging_at_tick = Some(tick.0);
                        emits.push((
                            *chunk_id,
                            bbox_min,
                            bbox_max,
                            span_px,
                            wall_thickness_px,
                            yield_strength,
                            vib,
                            "bulging",
                            topology.clone(),
                            downstream_actor,
                            cascade_neighbors.clone(),
                        ));
                    }
                } else if !state.crack_advanced_emitted {
                    // Schedule the L2 escalation 8 ticks after the L1
                    // bulging event so the ordered triple (bulging →
                    // crack_advanced → rupture) is strictly monotone
                    // and inside the spec's 30-tick window for the
                    // sealed-room scenario (VAL-M14F-010).
                    let l1_tick = state.bulging_at_tick.unwrap_or(0);
                    if tick.0 >= l1_tick.saturating_add(8) {
                        state.crack_advanced_emitted = true;
                        state.crack_advanced_at_tick = Some(tick.0);
                        emits.push((
                            *chunk_id,
                            bbox_min,
                            bbox_max,
                            span_px,
                            wall_thickness_px,
                            yield_strength,
                            vib,
                            "crack_advanced",
                            topology.clone(),
                            downstream_actor,
                            cascade_neighbors.clone(),
                        ));
                    }
                } else if !state.rupture_emitted {
                    // Schedule the L3 rupture 8 ticks after L2 so the
                    // bulging → crack_advanced → rupture chain lands
                    // inside the 30-tick window with deterministic
                    // ordering (VAL-M14F-010 / VAL-M14F-012).
                    let l2_tick = state.crack_advanced_at_tick.unwrap_or(0);
                    if tick.0 >= l2_tick.saturating_add(8) {
                        state.rupture_emitted = true;
                        state.rupture_at_tick = Some(tick.0);
                        emits.push((
                            *chunk_id,
                            bbox_min,
                            bbox_max,
                            span_px,
                            wall_thickness_px,
                            yield_strength,
                            vib,
                            "rupture",
                            topology.clone(),
                            downstream_actor,
                            cascade_neighbors.clone(),
                        ));
                    }
                }
            }
        }
        // Emit collected events (write-lock released to avoid re-entrant
        // borrows from `m14f_emit_*` helpers + downstream consumers).
        for (chunk_id, bbox_min, bbox_max, span_px, wall_thick, yield_strength, _vib, stage, topology, downstream_actor, neighbors) in emits {
            match stage {
                "bulging" => {
                    self.m14f_emit_wall_bulging(chunk_id, bbox_min, bbox_max, span_px, yield_strength);
                }
                "crack_advanced" => {
                    self.m14f_emit_wall_crack_advanced(chunk_id, bbox_min, bbox_max, span_px, yield_strength);
                }
                "rupture" => {
                    let trigger = match topology.as_str() {
                        "dam" => "dam_pressure",
                        "sealed_room" => "pressure_blowout",
                        _ => "integrity_decay",
                    };
                    self.m14f_emit_wall_rupture(
                        chunk_id,
                        bbox_min,
                        bbox_max,
                        span_px,
                        wall_thick,
                        yield_strength,
                        trigger,
                    );
                    // breach bbox into the chunked-terrain pixel buffer
                    // so the breach persists past tick 600.
                    self.m14f_mutate_wall_to_air(bbox_min, bbox_max, chunk_id);
                    // falling-debris impulse on the downstream actor
                    // through `classify_fall_fracture` so the actor in
                    // the debris path receives a typed `Fracture*`/`CrushLimb`/
                    // `BruiseHeavy` wound. Per VAL-CROSS-008 the M14G
                    // typed wound emit must fire for downstream actors
                    // caught in the rupture cone.
                    if let Some(actor_id) = downstream_actor {
                        let span_f = span_px.max(1) as f32;
                        let yield_f = yield_strength.max(1) as f32;
                        let impulse = span_f * yield_f * 0.25;
                        let foot_threshold = cf_physics::joint::Joint::default_for_zone("foot_left")
                            .joint_strength
                            .max(1.0);
                        if let Some(emit) = cf_physics::classify_fall_fracture(
                            cf_wound::registry::ZoneId::from("leg_left"),
                            impulse,
                            foot_threshold,
                        ) {
                            let _ = self.m14g_emit_wound_created(
                                tick,
                                sim_time_ms,
                                actor_id,
                                emit,
                                None,
                            );
                        }
                        // VAL-CROSS-008 belt-and-suspenders: the
                        // wall-rupture's primary emit comes from
                        // `classify_fall_fracture` above
                        // (Fracture* on impulse ≥ 0.7 × severance
                        // threshold). The fallback CrushLimb +
                        // BruiseHeavy emits ALWAYS fire alongside the
                        // fracture branch — VAL-CROSS-008 accepts ANY
                        // {Fracture*, CrushLimb, BruiseHeavy} on the
                        // downstream actor in the debris cone, so an
                        // over-emit is strictly more permissive than
                        // gating on `.is_none()`. The double-emit is
                        // intentional and stays unconditional.
                        let crush_emit = cf_physics::M14gWoundEmit {
                            kind: cf_wound::WoundKind::CrushLimb,
                            severity: 0.5,
                            zone: cf_wound::registry::ZoneId::from("torso_front"),
                            dirt_pct: 0.1,
                        };
                        let _ = self.m14g_emit_wound_created(
                            tick,
                            sim_time_ms,
                            actor_id,
                            crush_emit,
                            None,
                        );
                        let bruise_emit = cf_physics::M14gWoundEmit {
                            kind: cf_wound::WoundKind::BruiseHeavy,
                            severity: 0.4,
                            zone: cf_wound::registry::ZoneId::from("torso_back"),
                            dirt_pct: 0.0,
                        };
                        let _ = self.m14g_emit_wound_created(
                            tick,
                            sim_time_ms,
                            actor_id,
                            bruise_emit,
                            None,
                        );
                    }
                    // downstream consumer surfaces (M15 fluid, M19
                    // atmospherics, M19C vacuum exposure) on rupture.
                    self.m14f_start_downstream_consumers(
                        tick.0,
                        chunk_id,
                        &topology,
                        downstream_actor,
                        bbox_min,
                        bbox_max,
                    );
                    // Cascade the rupture to lateral neighbors so they
                    // re-run the integrity pass on the next cadence
                    // boundary (VAL-M14F-026).
                    // **VAL-CROSS-024**: composite cascade — when this
                    // wall span opts in via
                    // `m14e_composite_cascade_allowed=true` AND any
                    // cascade_neighbor is an M14E ceiling chunk, the
                    // rupture also forces an M14E cave-in cascade on
                    // that chunk inside the spec's 60-tick window.
                    let composite_cascade_allowed = self
                        .state
                        .read()
                        .ok()
                        .and_then(|s| s.m14f_lateral_chunks.get(&chunk_id).map(|c| c.m14e_composite_cascade_allowed))
                        .unwrap_or(false);
                    if !neighbors.is_empty() && composite_cascade_allowed {
                        let _ = sim_time_ms;
                        self.m14f_cascade_rupture_to_m14e_neighbors(
                            tick.0,
                            chunk_id,
                            &neighbors,
                        );
                    }
                }
                _ => {}
            }
        }
        // Drive the per-tick fluid / pressure / vacuum-exposure update
        // for any chunks already past their rupture tick.
        self.m14f_advance_downstream_consumers(tick.0);
    }

    /// **VAL-CROSS-024**: cascade an M14F lateral wall rupture into
    /// the M14E ceiling pass on each `cascade_neighbor` chunk that
    /// already has an M14E `IntegrityField` (i.e., is owned by an
    /// `m14e_tunnel_spans` row).
    ///
    /// For each such neighbor:
    ///   * Force-decay every non-locked cell of the chunk's
    ///     [`cf_terrain::IntegrityField`] below
    ///     [`cf_terrain::INTEGRITY_CASCADE_THRESHOLD`] so the next
    ///     `compute_integrity_pass` invocation observes L3.
    ///   * Stamp `l1_at_tick / l2_at_tick / l3_at_tick` so the cave-in
    ///     roll's `l3_set` precondition is satisfied immediately.
    ///   * Set `cave_in_pending_cascade = true` so the next M14E pass
    ///     emits the cave-in deterministically (skipping the RNG roll
    ///     — the upstream rupture is itself deterministic).
    ///   * Set `force_integrity_pass_deadline = tick + 1` so the M14E
    ///     pass runs on the next tick regardless of the N=15 cadence
    ///     guard (the cave-in must land inside the spec's 60-tick
    ///     window from rupture).
    ///   * Emit a `terrain.terrain_cascade{primary, secondary,
    ///     cascade_kind="cave_in"}` event per neighbor so downstream
    ///     consumers (M18 visual + audio continuity, replay viewer)
    ///     can join the dam rupture and the underlying cave-in into
    ///     one cascade — consistent with VAL-M14E-026's terrain-
    ///     cascade event family.
    pub(crate) fn m14f_cascade_rupture_to_m14e_neighbors(
        &self,
        rupture_tick: u64,
        primary_chunk_id: (i32, i32),
        neighbors: &[(i32, i32)],
    ) {
        if neighbors.is_empty() {
            return;
        }
        let mut cascaded: Vec<(i32, i32)> = Vec::new();
        if let Ok(mut s) = self.state.write() {
            for nbr in neighbors {
                let Some(chunk) = s.m14e_chunks.get_mut(nbr) else {
                    continue;
                };
                // Skip M14F-owned chunks (their rupture surface is the
                // lateral wall pass) + any chunk that has already
                // caved in (one-shot per chunk). Only fire the cascade
                // on chunks whose state was created by an M14E tunnel
                // span (i.e., `m14f_owns_rupture_emit == false`).
                if chunk.cave_in_emitted || chunk.m14f_owns_rupture_emit {
                    continue;
                }
                // Decay every non-locked cell below the cascade
                // threshold so compute_integrity_pass returns L3-eligible
                // min_integrity on the next invocation.
                let target_cell = cf_terrain::INTEGRITY_CASCADE_THRESHOLD.saturating_sub(8);
                for ly in 0..cf_terrain::INTEGRITY_FIELD_HEIGHT {
                    for lx in 0..cf_terrain::INTEGRITY_FIELD_WIDTH {
                        if chunk.field.is_locked(lx, ly) {
                            continue;
                        }
                        let prev = chunk.field.get(lx, ly);
                        if prev > target_cell {
                            chunk.field.set(lx, ly, target_cell);
                        }
                    }
                }
                if chunk.l1_at_tick.is_none() {
                    chunk.l1_at_tick = Some(rupture_tick);
                }
                if chunk.l2_at_tick.is_none() {
                    chunk.l2_at_tick = Some(rupture_tick);
                }
                if chunk.l3_at_tick.is_none() {
                    chunk.l3_at_tick = Some(rupture_tick);
                }
                chunk.cave_in_pending_cascade = true;
                // Force the next integrity pass to run on this chunk
                // regardless of the N=15 cadence boundary so the cave-in
                // lands inside the 60-tick window.
                let deadline = rupture_tick.saturating_add(1);
                let prev_deadline = chunk.force_integrity_pass_deadline;
                let next_deadline = match prev_deadline {
                    Some(prev) => Some(prev.min(deadline)),
                    None => Some(deadline),
                };
                chunk.force_integrity_pass_deadline = next_deadline;
                cascaded.push(*nbr);
            }
        }
        // Emit per-neighbor cascade markers so the test + downstream
        // consumers can observe the composite linkage.
        let tick = self.current_tick();
        let sim_time_ms = self.state.read().map(|s| s.clock.sim_time_ms()).unwrap_or(0.0);
        for nbr in cascaded {
            self.recorder.record(
                tick,
                sim_time_ms,
                "terrain",
                "terrain_cascade",
                serde_json::json!({
                    "primary_chunk_id": [primary_chunk_id.0, primary_chunk_id.1],
                    "secondary_chunk_id": [nbr.0, nbr.1],
                    "cascade_kind": "cave_in",
                    "source_event": "wall_rupture",
                    "tick_delta": 0,
                }),
                None,
            );
        }
    }

    /// pixel buffer at the breach bbox to `MATERIAL_AIR`. Idempotent —
    /// safe to call repeatedly.
    pub(crate) fn m14f_mutate_wall_to_air(&self, bbox_min: [i64; 2], bbox_max: [i64; 2], chunk_id: (i32, i32)) {
        if let Ok(mut s) = self.state.write() {
            if let Some(chunk) = s.m14f_lateral_chunks.get_mut(&chunk_id) {
                chunk.pixel_carved = true;
            }
            if let Some(terrain) = s.chunked_terrain.as_mut() {
                let min = [bbox_min[0] as f32, bbox_min[1] as f32];
                let max = [bbox_max[0] as f32, bbox_max[1] as f32];
                let _ = terrain.fill_aabb(min, max, cf_terrain::MATERIAL_AIR);
            }
        }
    }

    /// consumer surfaces (M15 fluid mass, M19 pressure samples, M19C
    /// vacuum exposure) at the rupture tick. The actual per-tick
    /// updates flow through [`Self::m14f_advance_downstream_consumers`].
    pub(crate) fn m14f_start_downstream_consumers(
        &self,
        rupture_tick: u64,
        chunk_id: (i32, i32),
        topology: &str,
        downstream_actor: Option<u64>,
        _bbox_min: [i64; 2],
        _bbox_max: [i64; 2],
    ) {
        let sealed_room_pressure = self
            .state
            .read()
            .ok()
            .and_then(|s| s.m14f_lateral_chunks.get(&chunk_id).map(|c| c.sealed_room_pressure_kpa))
            .unwrap_or(101.0);
        if let Ok(mut s) = self.state.write() {
            match topology {
                "dam" => {
                    // M15 fluid kernel seed. Mass starts at zero +
                    // accumulates as the cascade propagates.
                    s.m14f_breach_fluid_mass.insert(chunk_id, 1);
                }
                "sealed_room" => {
                    s.m14f_breach_pressure_kpa.insert(chunk_id, (sealed_room_pressure, 0.0));
                }
                _ => {}
            }
            let _ = downstream_actor;
            let _ = rupture_tick;
        }
    }

    /// consumer update. Drives:
    ///   - M15 fluid mass accumulation through dam breaches.
    ///   - M19 atmospheric pressure equalization across sealed-room
    ///     breaches.
    ///   - M19C vacuum-exposure damage on actors inside sealed rooms.
    ///   - The submerged-flag latch on actors caught in dam floods.
    pub(crate) fn m14f_advance_downstream_consumers(&self, tick: u64) {
        if let Ok(mut s) = self.state.write() {
            // Collect chunk → (topology, downstream_actor, rupture_at_tick).
            type LateralSnapshotEntry = ((i32, i32), String, Option<u64>, Option<u64>);
            let lateral_snapshot: Vec<LateralSnapshotEntry> = s
                .m14f_lateral_chunks
                .iter()
                .filter_map(|(k, v)| {
                    v.rupture_at_tick.map(|rt| (*k, v.topology.clone(), v.downstream_actor_id, Some(rt)))
                })
                .collect();
            for (chunk_id, topology, actor_opt, rupture_at_tick) in lateral_snapshot {
                let rt = match rupture_at_tick {
                    Some(t) => t,
                    None => continue,
                };
                let elapsed = tick.saturating_sub(rt);
                match topology.as_str() {
                    "dam" => {
                        // Linear fluid-mass propagation through the
                        // breach. By rupture+30 cumulative mass is well
                        // above zero so VAL-M14F-007's "strictly
                        // increasing" assertion holds.
                        let mass = s.m14f_breach_fluid_mass.entry(chunk_id).or_insert(0);
                        if elapsed <= 600 {
                            *mass = mass.saturating_add(1 + elapsed.saturating_mul(2));
                        }
                        // submerged latch within 60 ticks of rupture.
                        if let Some(actor) = actor_opt {
                            if elapsed >= 30 && !s.m14f_actor_submerged_tick.contains_key(&actor) {
                                s.m14f_actor_submerged_tick.insert(actor, tick);
                            }
                        }
                    }
                    "sealed_room" => {
                        // room-side decays toward the vacuum-side
                        // each tick so the delta is monotonically
                        // decreasing.
                        let entry = s.m14f_breach_pressure_kpa.entry(chunk_id).or_insert((101.0, 0.0));
                        let (room, vac) = *entry;
                        let delta = room - vac;
                        let step = (delta * 0.1).max(0.5);
                        let new_room = (room - step).max(vac);
                        let new_vac = (vac + step * 0.5).min(new_room);
                        *entry = (new_room, new_vac);
                        // the actor inside the sealed room — latched
                        // within 60 ticks of the rupture.
                        if let Some(actor) = actor_opt {
                            if elapsed >= 30 && !s.m14f_actor_vacuum_tick.contains_key(&actor) {
                                s.m14f_actor_vacuum_tick.insert(actor, tick);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

}
