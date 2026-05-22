//! Dirty-region flush + ai_path_reaction emit.
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
    pub(crate) fn flush_pending_dirty_batch(&self, tick: Tick, sim_time_ms: f64) {
        /// Hard coalescing budget per `terrain-material-slice-a`.
        const DIRTY_RECT_BUDGET: usize = 25;
        /// Number of consecutive ticks at `unupdated_areas > 0` before the
        /// engine emits a `terrain.forced_refresh_requested` signal for M22
        /// pathfinder. Tuned conservatively; 60 ticks @ 60 Hz = 1 second.
        const FORCED_REFRESH_THRESHOLD_TICKS: u32 = 60;

        let pending: Vec<PendingDirtyRect> = match self.state.write() {
            Ok(mut s) => std::mem::take(&mut s.pending_dirty_rects),
            Err(_) => return,
        };
        if pending.is_empty() {
            // No carves this tick — reset the sustained counter so transient
            // pressure doesn't accumulate into a stale forced-refresh signal.
            if let Ok(mut s) = self.state.write() {
                s.sustained_unupdated_ticks = 0;
            }
            return;
        }

        let rects_in = pending.len();

        // Collect unique source event ids in stable insertion order. A
        // BTreeSet preserves determinism if we keyed by string; we want
        // first-emit order so use a Vec with linear dedup.
        let mut source_event_ids: Vec<String> = Vec::with_capacity(rects_in);
        for entry in &pending {
            if !source_event_ids.contains(&entry.source_event_id) {
                source_event_ids.push(entry.source_event_id.clone());
            }
        }

        // Greedy coalesce: merge any rects whose AABBs overlap or touch on
        // an edge. Two-pass — first dedupe exact chunk hits (cx,cy match),
        // then if count > budget, AABB-union adjacent rects until count
        // ≤ budget. This is deterministic because we sort by (cx, cy) first.
        let mut merged: Vec<MergedDirtyRect> = pending
            .into_iter()
            .map(|e| MergedDirtyRect {
                cx: e.cx,
                cy: e.cy,
                min: e.min,
                max: e.max,
            })
            .collect();
        merged.sort_by(|a, b| (a.cx, a.cy, a.min[0], a.min[1]).cmp(&(b.cx, b.cy, b.min[0], b.min[1])));
        // Dedupe exact chunk matches (a single tick may dispatch multiple
        // carves into the same chunk; we only need one rect per chunk).
        merged.dedup_by(|a, b| a.cx == b.cx && a.cy == b.cy);

        // If we still exceed the budget, perform AABB unions on adjacent
        // pairs (in sorted order). This is intentionally simple and
        // deterministic — it always merges the lexicographically earliest
        // overlapping pair until count ≤ budget. Worst case: 60 chunks
        // collapse to a single super-rect.
        while merged.len() > DIRTY_RECT_BUDGET {
            let mut i = 0;
            let mut merged_any = false;
            while i + 1 < merged.len() {
                let (left, right) = merged.split_at_mut(i + 1);
                let a = &mut left[i];
                let b = &right[0];
                if rects_touch_or_overlap(a.min, a.max, b.min, b.max) {
                    a.min[0] = a.min[0].min(b.min[0]);
                    a.min[1] = a.min[1].min(b.min[1]);
                    a.max[0] = a.max[0].max(b.max[0]);
                    a.max[1] = a.max[1].max(b.max[1]);
                    merged.remove(i + 1);
                    merged_any = true;
                } else {
                    i += 1;
                }
            }
            if !merged_any {
                // Nothing further to merge — coalescing has saturated. Force
                // a global super-rect by unioning everything to fit the
                // budget exactly at 1 rect.
                if let Some((first, rest)) = merged.split_first_mut() {
                    for other in rest.iter() {
                        first.min[0] = first.min[0].min(other.min[0]);
                        first.min[1] = first.min[1].min(other.min[1]);
                        first.max[0] = first.max[0].max(other.max[0]);
                        first.max[1] = first.max[1].max(other.max[1]);
                    }
                }
                merged.truncate(1);
                break;
            }
        }

        let rects_out = merged.len();
        let unupdated_areas = rects_in.saturating_sub(rects_out) as u32;

        let out_rects_json: Vec<serde_json::Value> = merged
            .iter()
            .map(|m| {
                serde_json::json!({
                    "cx": m.cx,
                    "cy": m.cy,
                    "min": m.min,
                    "max": m.max,
                })
            })
            .collect();

        // Sample for `summary.json.perf.terrain` — keep last 1024 samples
        // (enough for a full-mission cost histogram without unbounded growth).
        if let Ok(mut s) = self.state.write() {
            const PERF_SAMPLE_CAP: usize = 1024;
            s.perf_coalesce_samples.push(rects_in as u32);
            if s.perf_coalesce_samples.len() > PERF_SAMPLE_CAP {
                s.perf_coalesce_samples.remove(0);
            }
            s.perf_coalesce_rects_in_total = s.perf_coalesce_rects_in_total.saturating_add(rects_in as u64);
            s.perf_coalesce_rects_out_total = s.perf_coalesce_rects_out_total.saturating_add(rects_out as u64);
            if unupdated_areas > 0 {
                s.sustained_unupdated_ticks = s.sustained_unupdated_ticks.saturating_add(1);
            } else {
                s.sustained_unupdated_ticks = 0;
            }
        }

        // The parent_event_id of the batch is the first contributing
        // source event (typically `tool_action_started.<id>` chain). Replay
        // viewers walk source_event_ids[] for the full causal fan-in.
        let parent_event_id = source_event_ids.first().cloned();
        // the path-invalidation version BEFORE emitting the batch, so the
        // subsequent terrain.path_invalidated event carries the right
        // version_old/_new. Only fires when out_rects[] is non-empty.
        let path_bbox =
            out_rects_json
                .iter()
                .filter_map(|v| v.as_object())
                .fold(None::<([f32; 2], [f32; 2])>, |acc, r| {
                    let min_v = r.get("min")?;
                    let max_v = r.get("max")?;
                    let min = min_v.as_array()?;
                    let max = max_v.as_array()?;
                    let mn = [min.first()?.as_f64()? as f32, min.get(1)?.as_f64()? as f32];
                    let mx = [max.first()?.as_f64()? as f32, max.get(1)?.as_f64()? as f32];
                    Some(match acc {
                        Some(a) => (
                            [a.0[0].min(mn[0]), a.0[1].min(mn[1])],
                            [a.1[0].max(mx[0]), a.1[1].max(mx[1])],
                        ),
                        None => (mn, mx),
                    })
                });
        let (version_old, version_new) = if let Ok(mut s) = self.state.write() {
            let old = s.path_invalidation_version;
            if path_bbox.is_some() {
                s.path_invalidation_version = s.path_invalidation_version.saturating_add(1);
            }
            (old, s.path_invalidation_version)
        } else {
            (0, 0)
        };
        let dirty_batch_id = self.recorder.record(
            tick,
            sim_time_ms,
            "terrain",
            "terrain_dirty_region_batch",
            serde_json::json!({
                "source_event_ids": source_event_ids,
                "in_rects": rects_in,
                "out_rects": out_rects_json,
                "unupdated_areas": unupdated_areas,
                "coalesce_cost": {
                    "rects_in": rects_in,
                    "rects_out": rects_out,
                },
            }),
            parent_event_id.clone(),
        );
        // `terrain.chunk_mutated` semantic event per merged dirty chunk
        // with the post-state blake3 checksum. Per M8A § "Semantic
        // terrain event protocol": "Every terrain mutation emits a
        // semantic event (not a bitmap delta)" + "every terrain change
        // emits `terrain.chunk_mutated` with `post_state_checksum`".
        // This closes the cf-net authoritative-server reconciliation
        // invariant.
        if let Ok(s) = self.state.read() {
            if let Some(terrain) = s.chunked_terrain.as_ref() {
                for m in &merged {
                    let checksum = terrain
                        .chunk_checksum(m.cx as i32, m.cy as i32)
                        .unwrap_or_else(|| String::new());
                    let _ = self.recorder.record(
                        tick,
                        sim_time_ms,
                        "terrain",
                        "chunk_mutated",
                        serde_json::json!({
                            "chunk_coords": [m.cx, m.cy],
                            "bbox": {"min": m.min, "max": m.max},
                            "post_state_checksum": checksum,
                            "cause": "terrain_batch",
                            "source_event_id": dirty_batch_id.clone(),
                        }),
                        Some(dirty_batch_id.clone()),
                    );
                }
            }
        }
        // M22+ pathfinder consumers. Placeholder event per spec ledger.
        if let Some((bbox_min, bbox_max)) = path_bbox {
            let terrain_path_id = self.recorder.record(
                tick,
                sim_time_ms,
                "terrain",
                "path_invalidated",
                serde_json::json!({
                    "bbox": { "min": bbox_min, "max": bbox_max },
                    "version_old": version_old,
                    "version_new": version_new,
                    "affected_teams": serde_json::Value::Array(Vec::new()),
                }),
                parent_event_id,
            );
            // when the dirty-region bbox intersects a guard's planned
            // pursuit line (guard → target), the AI's path is
            // invalidated. Emit `ai.path_invalidated` (category=ai) per
            // affected guard, followed by `ai.recovery_action` whose
            // action comes from `cf_ai::path_reaction::pick_recovery_action`
            // (reroute / fire_over_obstacle / give_up_and_fire_from_here).
            self.emit_ai_path_reaction(tick, sim_time_ms, bbox_min, bbox_max, terrain_path_id);
        }

        // Emit forced-refresh signal if sustained pressure exceeds threshold.
        let sustained = self.state.read().map(|s| s.sustained_unupdated_ticks).unwrap_or(0);
        if sustained >= FORCED_REFRESH_THRESHOLD_TICKS {
            self.recorder.record(
                tick,
                sim_time_ms,
                "terrain",
                "forced_refresh_requested",
                serde_json::json!({
                    "reason": "sustained_unupdated_areas",
                    "sustained_ticks": sustained,
                    "threshold_ticks": FORCED_REFRESH_THRESHOLD_TICKS,
                }),
                None,
            );
            // Reset counter so we don't spam — wait for another threshold
            // before re-emitting.
            if let Ok(mut s) = self.state.write() {
                s.sustained_unupdated_ticks = 0;
            }
        }
    }

    /// `ai.path_invalidated` (category=ai) + `ai.recovery_action` per
    /// guard whose planned pursuit line crosses the freshly-carved bbox.
    /// The recovery action is computed by
    /// `cf_ai::path_reaction::pick_recovery_action` from the fraction of
    /// the path that intersects the dirty bbox, whether the guard has
    /// line-of-sight to the target, and the remaining path length.
    pub(crate) fn emit_ai_path_reaction(
        &self,
        tick: Tick,
        sim_time_ms: f64,
        bbox_min: [f32; 2],
        bbox_max: [f32; 2],
        parent_event_id: String,
    ) {
        let state = match self.state.read() {
            Ok(s) => s,
            Err(_) => return,
        };
        let guards: Vec<(ActorId, [f32; 2], Option<[f32; 2]>)> = state
            .reactive_guards
            .keys()
            .filter_map(|gid| {
                state
                    .actor_state
                    .as_ref()
                    .and_then(|sim| sim.world.actors.get(gid))
                    .map(|a| (*gid, [a.position.x, a.position.y]))
            })
            .map(|(gid, gpos)| {
                let last_seen = state.reactive_guards.get(&gid).and_then(|g| g.last_player_position);
                (gid, gpos, last_seen)
            })
            .collect();
        let player_pos = state.player_actor.and_then(|pid| {
            state
                .actor_state
                .as_ref()
                .and_then(|sim| sim.world.actors.get(&pid))
                .filter(|a| !a.status.is_dead())
                .map(|a| [a.position.x, a.position.y])
        });
        let reactor_positions: Vec<[f32; 2]> = state
            .reactor_world
            .as_ref()
            .map(|w| w.iter().filter(|r| !r.is_destroyed()).map(|r| r.position).collect())
            .unwrap_or_default();
        let terrain = state.chunked_terrain.as_ref().cloned();
        drop(state);

        if guards.is_empty() {
            return;
        }

        let los_clear = |from: [f32; 2], to: [f32; 2]| -> bool {
            let Some(t) = terrain.as_ref() else { return true };
            let dx = to[0] - from[0];
            let dy = to[1] - from[1];
            const LOS_STEPS: u32 = 16;
            let steps = LOS_STEPS as f32;
            for i in 1..LOS_STEPS {
                let f = i as f32 / steps;
                let sx = from[0] + dx * f;
                let sy = from[1] + dy * f;
                let mat = t.material_at_world(sx, sy);
                if let Some(aff) = t.registry.affordance(mat) {
                    if aff.blocks_line_of_sight {
                        return false;
                    }
                }
            }
            true
        };

        for (gid, gpos, last_seen) in guards {
            let nearest_reactor = reactor_positions.iter().min_by(|a, b| {
                let da = (a[0] - gpos[0]).powi(2) + (a[1] - gpos[1]).powi(2);
                let db = (b[0] - gpos[0]).powi(2) + (b[1] - gpos[1]).powi(2);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            });
            let target = player_pos.or(last_seen).or(nearest_reactor.copied());
            let Some(target_xy) = target else { continue };

            const PATH_SAMPLES: u32 = 32;
            let dx = target_xy[0] - gpos[0];
            let dy = target_xy[1] - gpos[1];
            let total_len = (dx * dx + dy * dy).sqrt();
            if total_len < 1.0 {
                continue;
            }
            let mut dirty_hits = 0u32;
            for i in 0..PATH_SAMPLES {
                let f = (i as f32 + 0.5) / PATH_SAMPLES as f32;
                let sx = gpos[0] + dx * f;
                let sy = gpos[1] + dy * f;
                if sx >= bbox_min[0] && sx <= bbox_max[0] && sy >= bbox_min[1] && sy <= bbox_max[1] {
                    dirty_hits += 1;
                }
            }
            if dirty_hits == 0 {
                continue;
            }
            let fraction_of_path_dirty = dirty_hits as f32 / PATH_SAMPLES as f32;
            let has_los_to_target = los_clear(gpos, target_xy);

            let old_path_json = serde_json::json!([[gpos[0], gpos[1]], [target_xy[0], target_xy[1]],]);
            let path_invalidated_id = self.recorder.record(
                tick,
                sim_time_ms,
                "ai",
                "path_invalidated",
                serde_json::json!({
                    "actor": gid.0,
                    "actor_id": gid.0,
                    "bbox": { "min": bbox_min, "max": bbox_max },
                    "old_path": old_path_json,
                    "reason": "terrain_dirty",
                    "fraction_of_path_dirty": fraction_of_path_dirty,
                }),
                Some(parent_event_id.clone()),
            );
            let action =
                cf_ai::path_reaction::pick_recovery_action(fraction_of_path_dirty, has_los_to_target, total_len);
            let reason = match action {
                cf_ai::path_reaction::RecoveryAction::Reroute => "terrain_dirty_reroute",
                cf_ai::path_reaction::RecoveryAction::FireOverObstacle => "terrain_dirty_fire_over",
                cf_ai::path_reaction::RecoveryAction::GiveUpAndFireFromHere => "terrain_dirty_give_up",
            };
            self.recorder.record(
                tick,
                sim_time_ms,
                "ai",
                "recovery_action",
                serde_json::json!({
                    "actor": gid.0,
                    "actor_id": gid.0,
                    "action": action.as_str(),
                    "reason": reason,
                }),
                Some(path_invalidated_id),
            );
        }
    }

}
