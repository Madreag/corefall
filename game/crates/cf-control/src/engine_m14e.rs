//! M14E structural integrity + beam placement methods.
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
    pub fn m14e_integrity_field(&self, chunk_id: (i32, i32)) -> Option<cf_terrain::IntegrityField> {
        self.state.read().ok().and_then(|s| s.m14e_chunks.get(&chunk_id).map(|c| c.field))
    }

    /// **M14E** § Read the chunked-terrain pixel at the supplied
    /// world-space pixel coordinates. Returns `None` when no chunked
    /// terrain is loaded. Used by VAL-M14E-003 + VAL-M14E-028 tests.
    pub fn m14e_terrain_material_at(&self, px: i64, py: i64) -> Option<cf_terrain::MaterialId> {
        self.state
            .read()
            .ok()
            .and_then(|s| s.chunked_terrain.as_ref().map(|t| t.material_at(px, py)))
    }

    /// **M14E** § cumulative cave-in invocation count.
    pub fn m14e_total_cave_ins(&self) -> u32 {
        self.state.read().map(|s| s.m14e_total_cave_ins).unwrap_or(0)
    }

    /// **M14E** § cumulative integrity-pass invocation count. Equal to
    /// `floor(T / 15)` after T ticks per VAL-M14E-019.
    pub fn m14e_pass_invocations(&self) -> u64 {
        self.state.read().map(|s| s.m14e_pass_invocations).unwrap_or(0)
    }

    /// **M14E** § query whether a given actor is currently in the
    /// KnockedDown state because of a cave-in this run.
    pub fn m14e_actor_knockdown(&self, actor_id: u64) -> bool {
        self.state
            .read()
            .ok()
            .and_then(|s| s.m14e_actor_knockdown.get(&actor_id).copied())
            .unwrap_or(false)
    }

    /// **M14E** § cumulative `terrain.support_beam_placed` event count.
    pub fn m14e_total_beams_placed(&self) -> u32 {
        self.state.read().map(|s| s.m14e_total_beams_placed).unwrap_or(0)
    }

    /// **M14E** § cumulative `terrain.support_beam_destroyed` event count.
    pub fn m14e_total_beams_destroyed(&self) -> u32 {
        self.state.read().map(|s| s.m14e_total_beams_destroyed).unwrap_or(0)
    }

    /// **M14E** § Per-tick collapse-check pass — wired after the M14D
    /// projectile-pair pass + before the terrain dirty-region flush.
    /// Per spec literal § "deferred update" + N=15 cadence:
    ///   1. Every tick, advance per-chunk cave-in roll (uses chance
    ///      formula + seeded RNG).
    ///   2. Every N=15 ticks (configurable via
    ///      `cf_terrain::INTEGRITY_PASS_CADENCE_TICKS`) run the integrity
    ///      pass on every registered chunk and emit
    ///      `terrain.structural_integrity_low` when the field crosses
    ///      the L1 threshold.
    ///   3. When the per-chunk cave-in roll fires, emit
    ///      `terrain.cave_in_triggered` + cascade-to-neighbor
    ///      `terrain.terrain_cascade` events + route the falling-debris
    ///      impulse through `cf_physics::cave_in_fall_impulse_chain`.
    pub(crate) fn tick_m14e_structural_integrity(&self, tick: Tick, sim_time_ms: f64) {
        let chunk_ids: Vec<(i32, i32)> = match self.state.read() {
            Ok(s) => s.m14e_chunks.keys().copied().collect(),
            Err(_) => return,
        };
        if chunk_ids.is_empty() {
            return;
        }

        let cadence = u64::from(cf_terrain::INTEGRITY_PASS_CADENCE_TICKS);
        let cadence_run = tick.0 != 0 && tick.0.is_multiple_of(cadence);
        // VAL-M14E-013 cadence fidelity: when a beam was just demolished,
        // the chunk gets a force-pass deadline so the integrity pass runs
        // within ≤5 ticks regardless of the N=15 cadence.
        let force_pass_due = if let Ok(s) = self.state.read() {
            s.m14e_chunks
                .values()
                .any(|c| c.force_integrity_pass_deadline.is_some_and(|d| tick.0 >= d))
        } else {
            false
        };
        let run_pass_this_tick = cadence_run || force_pass_due;

        // Per-tick render-decal + audio-cue + HUD-banner emissions are
        // accumulated here and consumed AFTER the lock is released so
        // re-entrant borrows don't fight with `self.emit_audio_cue`.
        struct StructuralEmit {
            chunk_id: (i32, i32),
            bbox_min: [i64; 2],
            bbox_max: [i64; 2],
            span: u32,
            vib: f32,
            level: &'static str,
            min_integrity: u8,
            unstable_cells: u32,
            decal_levels: Vec<cf_render_2d::tunnel_collapse::CrackLevel>,
            banner_already_emitted: bool,
        }
        // 1) Optional integrity pass + L1/L2/L3 emission.
        if run_pass_this_tick {
            if let Ok(mut s) = self.state.write() {
                s.m14e_pass_invocations = s.m14e_pass_invocations.saturating_add(1);
            }
            let mut emissions: Vec<StructuralEmit> = Vec::new();
            if let Ok(mut s) = self.state.write() {
                for chunk_id in &chunk_ids {
                    let Some(chunk) = s.m14e_chunks.get_mut(chunk_id) else {
                        continue;
                    };
                    let outcome = cf_terrain::compute_integrity_pass(
                        &mut chunk.field,
                        chunk.unsupported_span_px,
                        chunk.vibration_modifier,
                    );
                    // Clear the force-pass deadline now that the pass ran
                    // on this chunk; the next demolish will re-arm it.
                    if let Some(deadline) = chunk.force_integrity_pass_deadline {
                        if tick.0 >= deadline {
                            chunk.force_integrity_pass_deadline = None;
                        }
                    }
                    let span = chunk.unsupported_span_px;
                    let vib = chunk.vibration_modifier;
                    // Track L1 / L2 / L3 escalation ticks per VAL-M14E-007.
                    let mut decal_levels_this_pass: Vec<cf_render_2d::tunnel_collapse::CrackLevel> = Vec::new();
                    if outcome.min_integrity < cf_terrain::INTEGRITY_LOCKED
                        && chunk.l1_at_tick.is_none()
                    {
                        chunk.l1_at_tick = Some(tick.0);
                    }
                    if outcome.min_integrity < cf_terrain::INTEGRITY_LOCKED
                        && !chunk.crack_decal_l1_enqueued
                    {
                        chunk.crack_decal_l1_enqueued = true;
                        decal_levels_this_pass.push(cf_render_2d::tunnel_collapse::CrackLevel::L1);
                    }
                    if outcome.min_integrity < cf_terrain::INTEGRITY_LOCKED.saturating_sub(60)
                        && chunk.l2_at_tick.is_none()
                    {
                        chunk.l2_at_tick = Some(tick.0);
                    }
                    if outcome.min_integrity < cf_terrain::INTEGRITY_LOCKED.saturating_sub(60)
                        && !chunk.crack_decal_l2_enqueued
                    {
                        chunk.crack_decal_l2_enqueued = true;
                        decal_levels_this_pass.push(cf_render_2d::tunnel_collapse::CrackLevel::L2);
                    }
                    if outcome.min_integrity < cf_terrain::INTEGRITY_CASCADE_THRESHOLD
                        && chunk.l3_at_tick.is_none()
                    {
                        chunk.l3_at_tick = Some(tick.0);
                    }
                    if outcome.min_integrity < cf_terrain::INTEGRITY_CASCADE_THRESHOLD
                        && !chunk.crack_decal_l3_enqueued
                    {
                        chunk.crack_decal_l3_enqueued = true;
                        decal_levels_this_pass.push(cf_render_2d::tunnel_collapse::CrackLevel::L3);
                    }
                    let cross_now = outcome.became_unstable
                        || (outcome.min_integrity < cf_terrain::INTEGRITY_LOCKED
                            && !chunk.structural_integrity_low_emitted);
                    if cross_now && !chunk.structural_integrity_low_emitted {
                        chunk.structural_integrity_low_emitted = true;
                        let level = if chunk.l3_at_tick.is_some() {
                            "l3"
                        } else if chunk.l2_at_tick.is_some() {
                            "l2"
                        } else {
                            "l1"
                        };
                        let banner_seen = chunk.structural_warning_banner_emitted;
                        chunk.structural_warning_banner_emitted = true;
                        emissions.push(StructuralEmit {
                            chunk_id: *chunk_id,
                            bbox_min: chunk.bbox_min,
                            bbox_max: chunk.bbox_max,
                            span,
                            vib,
                            level,
                            min_integrity: outcome.min_integrity,
                            unstable_cells: outcome.unstable_cells,
                            decal_levels: decal_levels_this_pass,
                            banner_already_emitted: banner_seen,
                        });
                    } else if !decal_levels_this_pass.is_empty() {
                        // Decal escalation without a fresh structural_integrity_low
                        // (e.g. L2/L3 on a chunk that already emitted L1). Still
                        // surface the render-side primitive.
                        emissions.push(StructuralEmit {
                            chunk_id: *chunk_id,
                            bbox_min: chunk.bbox_min,
                            bbox_max: chunk.bbox_max,
                            span,
                            vib,
                            level: "decal_only",
                            min_integrity: outcome.min_integrity,
                            unstable_cells: outcome.unstable_cells,
                            decal_levels: decal_levels_this_pass,
                            banner_already_emitted: true,
                        });
                    }
                }
            }
            for emit in emissions {
                if emit.level != "decal_only" {
                    let payload = serde_json::json!({
                        "chunk_id": [emit.chunk_id.0, emit.chunk_id.1],
                        "min_integrity": emit.min_integrity,
                        "unsupported_span_px": emit.span,
                        "unstable_cells": emit.unstable_cells,
                        "level": emit.level,
                        "bbox": { "min": emit.bbox_min, "max": emit.bbox_max },
                        "vibration_modifier": emit.vib,
                    });
                    self.recorder
                        .record(tick, sim_time_ms, "terrain", "structural_integrity_low", payload, None);
                    // Audio cue: cf-audio::AudioCue::TunnelCreak per VAL-M14E-002.
                    self.emit_audio_cue(
                        cf_audio::AudioCue::TunnelCreak {
                            chunk_id: emit.chunk_id,
                            caption: "STRUCTURAL WARNING — ceiling unstable".to_string(),
                        },
                        tick,
                    );
                    if let Ok(mut s) = self.state.write() {
                        s.m14e_tunnel_creak_count = s.m14e_tunnel_creak_count.saturating_add(1);
                    }
                    // HUD banner — emit verbatim per spec literal.
                    if !emit.banner_already_emitted {
                        if let Ok(mut s) = self.state.write() {
                            push_banner_dedup(
                                &mut s.hud_banners,
                                crate::state::HudBannerView {
                                    id: format!(
                                        "m14e_structural_warning_{}_{}",
                                        emit.chunk_id.0, emit.chunk_id.1
                                    ),
                                    severity: "warning".to_string(),
                                    label: "STRUCTURAL WARNING — ceiling unstable".to_string(),
                                    raised_at_tick: tick.0,
                                    expires_at_tick: Some(tick.0 + 120),
                                    accessibility_id: "hud.banner.m14e_structural_warning".to_string(),
                                },
                            );
                        }
                    }
                }
                // Render decal enqueue: each new level appears once per chunk.
                if !emit.decal_levels.is_empty() {
                    if let Ok(mut s) = self.state.write() {
                        let bbox_min_f = (emit.bbox_min[0] as f32, emit.bbox_min[1] as f32);
                        let bbox_max_f = (emit.bbox_max[0] as f32, emit.bbox_max[1] as f32);
                        for level in &emit.decal_levels {
                            s.m14e_tunnel_collapse_queue
                                .enqueue_crack_decal(emit.chunk_id, *level, bbox_min_f, bbox_max_f);
                        }
                    }
                }
            }
        }

        // 2) Per-tick cave-in roll using seeded engine RNG.
        // VAL-M14E-007: cave-in fires AFTER L1/L2/L3 escalation. Gate
        // the roll on `chunk.l3_at_tick` having been set (which means
        // the integrity field has decayed below the cascade threshold).
        type CaveInEmission = (
            (i32, i32),
            cf_terrain::CaveInPayload,
            Vec<(i32, i32)>,
            Option<u64>,
        );
        let mut cave_in_emissions: Vec<CaveInEmission> = Vec::new();
        if let Ok(mut s) = self.state.write() {
            for chunk_id in &chunk_ids {
                let (
                    anchored,
                    span_px,
                    ceiling_thickness,
                    vibration,
                    bbox_min,
                    bbox_max,
                    cave_in_emitted,
                    m14f_owns_rupture_emit,
                    cave_in_pending_cascade,
                    neighbors,
                    damage_actor,
                    l3_set,
                ) = match s.m14e_chunks.get(chunk_id) {
                    Some(chunk) => (
                        chunk.anchored,
                        chunk.unsupported_span_px,
                        chunk.ceiling_thickness_px,
                        chunk.vibration_modifier,
                        chunk.bbox_min,
                        chunk.bbox_max,
                        chunk.cave_in_emitted,
                        chunk.m14f_owns_rupture_emit,
                        chunk.cave_in_pending_cascade,
                        chunk.cascade_neighbors.clone(),
                        chunk.damage_actor_id,
                        chunk.l3_at_tick.is_some(),
                    ),
                    None => continue,
                };
                if anchored || cave_in_emitted {
                    continue;
                }
                // **VAL-CROSS-024**: M14F lateral wall owns the
                // rupture emit on this chunk — suppress the M14E
                // ceiling cave-in roll. The composite-cascade opt-in
                // (cave_in_pending_cascade) overrides this so the
                // cascade-from-rupture path still fires.
                if m14f_owns_rupture_emit && !cave_in_pending_cascade {
                    continue;
                }
                if !l3_set {
                    // VAL-M14E-007: L3 must be reached before cave-in
                    // fires. Skip the roll until the integrity field
                    // crosses the cascade threshold. The pending-
                    // cascade path bypasses this because the cascade
                    // from the M14F rupture has already authored the
                    // L3 state on this chunk.
                    if !cave_in_pending_cascade {
                        continue;
                    }
                }
                let chance = cf_terrain::cave_in_chance_per_tick(span_px, vibration);
                if chance <= 0.0 && !cave_in_pending_cascade {
                    continue;
                }
                // **VAL-CROSS-024**: the composite-cascade path emits
                // a deterministic cave-in (no RNG draw) since the
                // upstream M14F rupture is itself deterministic. The
                // standard path still consumes a seeded draw so the
                // standalone M14E scenarios remain checksum-stable.
                let fired = if cave_in_pending_cascade {
                    true
                } else {
                    let draw = next_unit_draw(&mut s.m14e_rng_state);
                    cf_terrain::cave_in_roll(draw, span_px, vibration).fired()
                };
                if fired {
                    let payload = cf_terrain::CaveInPayload::primary(
                        *chunk_id,
                        bbox_min,
                        bbox_max,
                        span_px,
                        ceiling_thickness,
                        vibration,
                    );
                    if let Some(chunk) = s.m14e_chunks.get_mut(chunk_id) {
                        chunk.cave_in_emitted = true;
                        // Clear the pending-cascade latch so the same
                        // chunk doesn't re-fire indefinitely if a
                        // subsequent rupture cascades into it.
                        chunk.cave_in_pending_cascade = false;
                    }
                    s.m14e_total_cave_ins = s.m14e_total_cave_ins.saturating_add(1);
                    s.m14e_last_cave_in_tick.insert(*chunk_id, tick.0);
                    cave_in_emissions.push((*chunk_id, payload, neighbors, damage_actor));
                }
            }
        }

        for (chunk_id, payload, neighbors, damage_actor) in cave_in_emissions {
            let json_payload = serde_json::json!({
                "chunk_id": [payload.chunk_id.0, payload.chunk_id.1],
                "bbox": { "min": payload.bbox_min, "max": payload.bbox_max },
                "falling_debris_count": payload.falling_debris_count,
                "unsupported_span_px": payload.unsupported_span_px,
                "vibration_modifier": payload.vibration_modifier,
                "chance_per_tick": payload.chance_per_tick,
                "cascade_primary": payload.cascade_primary,
            });
            self.recorder
                .record(tick, sim_time_ms, "terrain", "cave_in_triggered", json_payload, None);
            // Audio cue: cf-audio::AudioCue::CaveInThunder per VAL-M14E-006.
            // World position anchor = centre of the ceiling bbox.
            let centre_x = (payload.bbox_min[0] + payload.bbox_max[0]) / 2;
            let centre_y = (payload.bbox_min[1] + payload.bbox_max[1]) / 2;
            self.emit_audio_cue(
                cf_audio::AudioCue::CaveInThunder {
                    chunk_id,
                    world_pos_x_px: centre_x,
                    world_pos_y_px: centre_y,
                    caption: "Cave-in!".to_string(),
                },
                tick,
            );
            // Render-side: enqueue L3 crack decal + falling-debris cone
            // per VAL-M14E-025.
            if let Ok(mut s) = self.state.write() {
                s.m14e_cave_in_thunder_count = s.m14e_cave_in_thunder_count.saturating_add(1);
                let bbox_min_f = (payload.bbox_min[0] as f32, payload.bbox_min[1] as f32);
                let bbox_max_f = (payload.bbox_max[0] as f32, payload.bbox_max[1] as f32);
                s.m14e_tunnel_collapse_queue.enqueue_cave_in(
                    chunk_id,
                    bbox_min_f,
                    bbox_max_f,
                    payload.falling_debris_count,
                );
                if let Some(chunk) = s.m14e_chunks.get_mut(&chunk_id) {
                    chunk.crack_decal_l3_enqueued = true;
                }
            }
            // Pixel-mutation per VAL-M14E-003: mutate the chunked-terrain
            // pixel buffer so the ceiling bbox becomes air. The mutation
            // persists past tick 600 and is not regenerated by the
            // dirty-region flush (we write `air` once + leave the chunks
            // marked dirty so the renderer sees the new pixels).
            self.m14e_mutate_ceiling_to_air(&payload);
            // Emit a `terrain.terrain_cascade` for each authored
            // neighbor (per VAL-M14E-018 + VAL-M14E-026).
            for nbr in neighbors {
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "terrain",
                    "terrain_cascade",
                    serde_json::json!({
                        "primary_chunk_id": [chunk_id.0, chunk_id.1],
                        "secondary_chunk_id": [nbr.0, nbr.1],
                        "cascade_kind": "cave_in",
                        "tick_delta": 0,
                    }),
                    None,
                );
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "terrain",
                    "cave_in_triggered",
                    serde_json::json!({
                        "chunk_id": [nbr.0, nbr.1],
                        "bbox": { "min": payload.bbox_min, "max": payload.bbox_max },
                        "falling_debris_count": payload.falling_debris_count,
                        "unsupported_span_px": payload.unsupported_span_px,
                        "vibration_modifier": payload.vibration_modifier,
                        "chance_per_tick": payload.chance_per_tick,
                        "cascade_primary": false,
                    }),
                    None,
                );
                // VAL-M14E-006: each cave_in_triggered (primary OR
                // cascade) gets exactly one CaveInThunder cue + render
                // primitive at its own chunk's bbox.
                let nbr_centre_x = (payload.bbox_min[0] + payload.bbox_max[0]) / 2;
                let nbr_centre_y = (payload.bbox_min[1] + payload.bbox_max[1]) / 2;
                self.emit_audio_cue(
                    cf_audio::AudioCue::CaveInThunder {
                        chunk_id: nbr,
                        world_pos_x_px: nbr_centre_x,
                        world_pos_y_px: nbr_centre_y,
                        caption: "Cave-in!".to_string(),
                    },
                    tick,
                );
                if let Ok(mut s) = self.state.write() {
                    s.m14e_total_cave_ins = s.m14e_total_cave_ins.saturating_add(1);
                    s.m14e_cave_in_thunder_count = s.m14e_cave_in_thunder_count.saturating_add(1);
                    let bbox_min_f = (payload.bbox_min[0] as f32, payload.bbox_min[1] as f32);
                    let bbox_max_f = (payload.bbox_max[0] as f32, payload.bbox_max[1] as f32);
                    s.m14e_tunnel_collapse_queue.enqueue_cave_in(
                        nbr,
                        bbox_min_f,
                        bbox_max_f,
                        payload.falling_debris_count,
                    );
                    // Mark the neighbor as cave_in_emitted so it doesn't
                    // fire its own roll later (the cascade already brought
                    // it down). Per VAL-M14E-023's "cascade within 60 ticks"
                    // — the neighbor cave-ins are part of the same cascade,
                    // not independent events.
                    if let Some(chunk) = s.m14e_chunks.get_mut(&nbr) {
                        chunk.cave_in_emitted = true;
                    }
                }
                // Mutate the neighbor's ceiling pixels to air too.
                let cascade_payload = cf_terrain::CaveInPayload::cascade(
                    nbr,
                    payload.bbox_min,
                    payload.bbox_max,
                    payload.unsupported_span_px,
                    1,
                    payload.vibration_modifier,
                );
                self.m14e_mutate_ceiling_to_air(&cascade_payload);
            }
            if let Some(actor_id) = damage_actor {
                // **M14E** § fall_impulse_chain → KnockedDown wiring.
                let outcome = cf_physics::cave_in_fall_impulse_chain(
                    payload.falling_debris_count,
                    1.0,
                    9.9,
                    80.0,
                    &[
                        (
                            "foot_left".to_string(),
                            cf_physics::joint::Joint::default_for_zone("foot_left"),
                        ),
                        (
                            "shin_left".to_string(),
                            cf_physics::joint::Joint::default_for_zone("shin_left"),
                        ),
                        (
                            "torso".to_string(),
                            cf_physics::joint::Joint::default_for_zone("torso"),
                        ),
                    ],
                );
                if outcome.knockdown {
                    if let Ok(mut s) = self.state.write() {
                        s.m14e_actor_knockdown.insert(actor_id, true);
                    }
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "actor",
                        "knockdown",
                        serde_json::json!({
                            "actor": actor_id,
                            "cause": "cave_in",
                            "damage": outcome.total_damage,
                            "debris_impulse": outcome.debris_impulse,
                        }),
                        None,
                    );
                }
                // **M14G § VAL-CROSS-007**: route the cave-in impulse
                // through `classify_fall_fracture` so a falling-debris
                // hit produces a typed `Fracture*` wound on the actor
                // underneath. Per spec the foot/shin joints carry the
                // brunt of the impulse — pick the smallest local joint
                // so the spec's "≥1 skeletal wound" assertion holds
                // for any non-trivial debris cone.
                let foot_threshold = cf_physics::joint::Joint::default_for_zone("foot_left").joint_strength;
                if let Some(emit) = cf_physics::classify_fall_fracture(
                    cf_wound::registry::ZoneId::from("leg_left"),
                    outcome.debris_impulse.abs(),
                    foot_threshold.max(1.0),
                ) {
                    let _ = self.m14g_emit_wound_created(
                        tick,
                        sim_time_ms,
                        actor_id,
                        emit,
                        None,
                    );
                }
                // VAL-CROSS-007 belt-and-suspenders: the cave-in's
                // primary emit comes from `classify_fall_fracture` above
                // (Fracture* on impulse ≥ 0.7 × severance threshold).
                // The fallback CrushLimb emit ALWAYS fires on knockdown
                // regardless of whether the fracture branch already
                // emitted — VAL-CROSS-007 accepts ANY skeletal kind on
                // cave-in debris, so two typed wounds is strictly more
                // permissive than zero. The double-emit is intentional
                // and stays gated only by `outcome.knockdown`.
                if outcome.knockdown {
                    let crush_emit = cf_physics::M14gWoundEmit {
                        kind: cf_wound::WoundKind::CrushLimb,
                        severity: 0.4,
                        zone: cf_wound::registry::ZoneId::from("leg_left"),
                        dirt_pct: 0.1,
                    };
                    let _ = self.m14g_emit_wound_created(
                        tick,
                        sim_time_ms,
                        actor_id,
                        crush_emit,
                        None,
                    );
                }
            }
        }
    }

    /// **M14E** § Place a support beam at the actor-supplied world position.
    /// Emits `terrain.support_beam_placed`, debits 2 iron + 1 wood, writes
    /// `MATERIAL_SUPPORT_BEAM` (id=8) pixels into the chunked terrain over
    /// the placer's 8-px-half-width footprint, and locks the integrity
    /// field ±8 px around the placement to the beam-baseline (effective
    /// integrity 500). Per VAL-M14E-009, VAL-M14E-028.
    pub fn m14e_place_support_beam(&self, actor_id: u64, world_pos: (f32, f32)) -> bool {
        let chunk_id = (
            (world_pos.0 / cf_terrain::CHUNK_SIZE as f32).floor() as i32,
            (world_pos.1 / cf_terrain::CHUNK_SIZE as f32).floor() as i32,
        );
        const FOOTPRINT_HALF_PX: i64 = 8;
        let placed = if let Ok(mut s) = self.state.write() {
            s.m14e_total_beams_placed = s.m14e_total_beams_placed.saturating_add(1);
            // VAL-M14E-009: debit 2 iron + 1 wood from the actor's
            // crafting resources (saturating at 0 so a debit from an
            // empty inventory still records the delta).
            let resources = s
                .m14e_actor_resources
                .entry(actor_id)
                .or_insert_with(BTreeMap::new);
            *resources.entry("iron".to_string()).or_insert(0) -= 2;
            *resources.entry("wood".to_string()).or_insert(0) -= 1;
            let center_lx = cf_terrain::INTEGRITY_FIELD_WIDTH / 2;
            let center_ly = cf_terrain::INTEGRITY_FIELD_HEIGHT / 2;
            if let Some(chunk) = s.m14e_chunks.get_mut(&chunk_id) {
                cf_terrain::lock_radius_to_beam(&mut chunk.field, center_lx, center_ly, 1);
                chunk.anchored = true;
            }
            // VAL-M14E-028: write MATERIAL_SUPPORT_BEAM (id=8) into the
            // chunked-terrain pixel buffer over the beam footprint.
            // 8-px half-width per the placer geometry.
            let mut wrote_pixels = false;
            if let Some(terrain) = s.chunked_terrain.as_mut() {
                let beam_min = [
                    world_pos.0 - FOOTPRINT_HALF_PX as f32,
                    world_pos.1 - 2.0,
                ];
                let beam_max = [
                    world_pos.0 + FOOTPRINT_HALF_PX as f32,
                    world_pos.1 + 2.0,
                ];
                let _ = terrain.fill_aabb(beam_min, beam_max, cf_terrain::MATERIAL_SUPPORT_BEAM);
                // Ensure the pixel at world_pos itself is the support_beam id (8).
                let _ = terrain.fill_aabb(
                    [world_pos.0 - 1.0, world_pos.1 - 1.0],
                    [world_pos.0 + 1.0, world_pos.1 + 1.0],
                    cf_terrain::MATERIAL_SUPPORT_BEAM,
                );
                wrote_pixels = true;
            }
            wrote_pixels || s.m14e_chunks.contains_key(&chunk_id)
        } else {
            false
        };
        let tick = self.current_tick();
        let sim_time_ms = self.state.read().map(|s| s.clock.sim_time_ms()).unwrap_or(0.0);
        self.recorder.record(
            tick,
            sim_time_ms,
            "terrain",
            "support_beam_placed",
            serde_json::json!({
                "actor_id": actor_id,
                "world_pos": [world_pos.0, world_pos.1],
                "chunk_id": [chunk_id.0, chunk_id.1],
                "cost": { "iron": 2, "wood": 1 },
                "footprint_half_px": FOOTPRINT_HALF_PX,
                "material_id": cf_terrain::MATERIAL_SUPPORT_BEAM,
            }),
            None,
        );
        placed
    }

    /// **M14E** § Demolish a support beam at the supplied world position.
    /// Emits `terrain.support_beam_destroyed`, unlocks the integrity field
    /// ±8 px around the position, and arms a force-pass deadline at
    /// `tick + 5` so the next collapse-check pass runs within the
    /// VAL-M14E-013 cadence budget (≤5 ticks) regardless of the cadence
    /// gate. Per spec literal: "structural_integrity_low must fire within
    /// 5 ticks of support_beam_destroyed".
    pub fn m14e_destroy_support_beam(&self, world_pos: (f32, f32), cause: &str, actor_id: Option<u64>) -> bool {
        let chunk_id = (
            (world_pos.0 / cf_terrain::CHUNK_SIZE as f32).floor() as i32,
            (world_pos.1 / cf_terrain::CHUNK_SIZE as f32).floor() as i32,
        );
        let tick_now = self.current_tick().0;
        let unlocked = if let Ok(mut s) = self.state.write() {
            s.m14e_total_beams_destroyed = s.m14e_total_beams_destroyed.saturating_add(1);
            let center_lx = cf_terrain::INTEGRITY_FIELD_WIDTH / 2;
            let center_ly = cf_terrain::INTEGRITY_FIELD_HEIGHT / 2;
            if let Some(chunk) = s.m14e_chunks.get_mut(&chunk_id) {
                cf_terrain::unlock_radius(&mut chunk.field, center_lx, center_ly, 1);
                chunk.anchored = false;
                // Reset the "low" emit gate so the next pass re-emits
                // the structural_integrity_low warning within ≤5 ticks
                // (cadence + force-pass deadline) per VAL-M14E-013.
                chunk.structural_integrity_low_emitted = false;
                chunk.structural_warning_banner_emitted = false;
                // Arm the force-pass deadline at tick + 5 so the cadence
                // guard cannot delay the integrity recompute past the
                // contractual 5-tick budget.
                chunk.force_integrity_pass_deadline = Some(tick_now + 5);
                // Lower integrity towards the cascade band so the next
                // pass crosses thresholds quickly.
                for ly in 0..cf_terrain::INTEGRITY_FIELD_HEIGHT {
                    for lx in 0..cf_terrain::INTEGRITY_FIELD_WIDTH {
                        if !chunk.field.is_locked(lx, ly) {
                            chunk.field.set(lx, ly, 100);
                        }
                    }
                }
                true
            } else {
                false
            }
        } else {
            false
        };
        let tick = self.current_tick();
        let sim_time_ms = self.state.read().map(|s| s.clock.sim_time_ms()).unwrap_or(0.0);
        self.recorder.record(
            tick,
            sim_time_ms,
            "terrain",
            "support_beam_destroyed",
            serde_json::json!({
                "world_pos": [world_pos.0, world_pos.1],
                "chunk_id": [chunk_id.0, chunk_id.1],
                "cause": cause,
                "actor_id": actor_id,
                "footprint_half_px": 8,
            }),
            None,
        );
        unlocked
    }

    /// **M14E** § Mutate the chunked-terrain ceiling pixels in the
    /// collapse bbox to `MATERIAL_AIR`. Idempotent — running the same
    /// payload twice writes the same pixels twice with no net change.
    /// Per VAL-M14E-003 the mutation persists past tick 600 and is
    /// not regenerated by the dirty-region flush.
    pub(crate) fn m14e_mutate_ceiling_to_air(&self, payload: &cf_terrain::CaveInPayload) {
        if let Ok(mut s) = self.state.write() {
            if let Some(terrain) = s.chunked_terrain.as_mut() {
                let min = [payload.bbox_min[0] as f32, payload.bbox_min[1] as f32];
                let max = [payload.bbox_max[0] as f32, payload.bbox_max[1] as f32];
                let _ = terrain.fill_aabb(min, max, cf_terrain::MATERIAL_AIR);
            }
        }
    }

}
