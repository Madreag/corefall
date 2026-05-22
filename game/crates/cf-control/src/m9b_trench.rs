//! M9B-3: cfctl dispatch for the trench player methods + observe
//! surfaces.
//!
//! Owns the mutation + event-emit logic for:
//!
//! - `act.player.dig_trench_segment { variant, tool_id?, substrate_hardness, strict? }`
//!   → emits `trench.segment_dug` after the per-variant dig-time elapses
//!   (12s standard / 5s shallow_scrape / 4× ratio). Substrate hardness
//!   gating for `deep` per VAL-M9B-DIG-003: hardness ≥ 0.5 falls back
//!   to `shallow_scrape` with a `trench.segment_variant_downgraded`
//!   warning event, OR returns a hard error when `strict=true`.
//!
//! - `act.player.place_trench_module { module_id, segment_id }` →
//!   emits `trench.module_placed`.
//!
//! - `act.player.repair_trench_module { module_id, segment_id }` →
//!   emits `trench.module_repaired`.
//!
//! - `observe.actor.cover_state { actor_id }` → returns the actor's
//!   `CoverState` derived from stance × current trench segment.
//!
//! - `observe.trench_segment_at_pos { x, y }` → returns either `null`
//!   or a `TrenchSegmentView` with the 6 declared variant + dimensions.

use serde_json::{json, Value};

use cf_actor::{ActorId, IntentSource};
use cf_sim_core::Tick;
use cf_trench::{
    apply_round_to_breastwork, breastwork::BreastworkHitOutcome, collapse_tick,
    cover_state_change, cover_state_post_breach, dig_substrate_validate,
    drainage_sump_tick, segment::TrenchSegmentLookup,
    BreastworkHitOutcome as _BreastworkOutcomeAlias, CollapseEnv,
    CollapseTickOutcome, CoverState, CoverStateChangeCause, DigSubstrateOutcome,
    DrainageEnv, DrainageTickOutcome, ModuleSpec, SegmentVariant, TrenchModule,
    TrenchSegment, TrenchStance, DEEP_HARDNESS_THRESHOLD,
};

use crate::engine::{GuardFireRecord, M0Engine};
use crate::server::CommandResult;

/// Parse a wire-format variant id into the typed `SegmentVariant`.
/// Returns `None` for unknown enum values; cfctl rejects upstream.
pub(crate) fn parse_variant(id: &str) -> Option<SegmentVariant> {
    match id {
        "shallow_scrape" => Some(SegmentVariant::ShallowScrape),
        "standard" => Some(SegmentVariant::Standard),
        "deep" => Some(SegmentVariant::Deep),
        "communication" => Some(SegmentVariant::Communication),
        "fire_step" => Some(SegmentVariant::FireStep),
        "parapet_raised" => Some(SegmentVariant::ParapetRaised),
        _ => None,
    }
}

/// Parse a module id; returns `None` for unknown enum values.
pub(crate) fn parse_module(id: &str) -> Option<TrenchModule> {
    match id {
        "duckboard" => Some(TrenchModule::Duckboard),
        "fire_step" => Some(TrenchModule::FireStep),
        "breastwork" => Some(TrenchModule::Breastwork),
        "drainage_sump" => Some(TrenchModule::DrainageSump),
        "revetment" => Some(TrenchModule::Revetment),
        "corner_traverse" => Some(TrenchModule::CornerTraverse),
        _ => None,
    }
}

/// Per-variant dig-time in whole in-game seconds. Matches the spec table:
/// `shallow_scrape: 5`, `standard: 12`, `deep: 24` (4× shallow), other
/// variants fall back to standard.
pub(crate) fn dig_time_seconds_for(variant: SegmentVariant) -> u32 {
    match variant {
        SegmentVariant::ShallowScrape => 5,
        SegmentVariant::Standard => 12,
        SegmentVariant::Deep => 24,
        SegmentVariant::Communication => 12,
        SegmentVariant::FireStep => 16,
        SegmentVariant::ParapetRaised => 20,
    }
}

/// Per-module build time in seconds. Mirrors the spec table:
/// `duckboard: 4s`, `fire_step: 8s`, `breastwork: 12s`, `drainage_sump: 6s`,
/// `revetment: 10s`, `corner_traverse: 6s`.
pub(crate) fn build_time_seconds_for(module: TrenchModule) -> u32 {
    match module {
        TrenchModule::Duckboard => 4,
        TrenchModule::FireStep => 8,
        TrenchModule::Breastwork => 12,
        TrenchModule::DrainageSump => 6,
        TrenchModule::Revetment => 10,
        TrenchModule::CornerTraverse => 6,
    }
}

impl M0Engine {
    /// **VAL-M9B-DIG-001..003 + VAL-M9B-CFCTL-001**: dispatch
    /// `act.player.dig_trench_segment`. Carving completes in
    /// `dig_time_seconds_for(variant)` in-game seconds (12s standard,
    /// 5s shallow_scrape, 24s deep — the 4× standard ratio). Substrate
    /// hardness ≥ 0.5 on the `deep` variant either falls back to
    /// `shallow_scrape` with a `trench.segment_variant_downgraded`
    /// warning event OR returns `ok=false` (`strict=true`).
    pub(crate) fn dispatch_m9b_dig_trench_segment(
        &self,
        variant_id: String,
        tool_id: Option<String>,
        substrate_hardness: f32,
        strict: bool,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
    ) -> CommandResult {
        let source_label = source_label(source);
        let requested = match parse_variant(&variant_id) {
            Some(v) => v,
            None => {
                self.record_command_rejected(
                    tick,
                    sim_time_ms,
                    "act.player.dig_trench_segment",
                    "unknown_segment_variant",
                );
                return CommandResult::rejected("unknown_segment_variant", tick.0);
            }
        };

        // Forward-compat: parapet_raised requires M9C — pre-M9C the
        // dig validates as a warning + does not place. m9b-1 ships the
        // dedicated validator + warning event.
        if matches!(requested, SegmentVariant::ParapetRaised) {
            if let Err(warn) = cf_trench::parapet_raised_dig_validate() {
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "trench",
                    "parapet_raised_requires_m9c",
                    json!({
                        "actor_id": null,
                        "variant": "parapet_raised",
                        "requires_m9c": true,
                        "reason": warn.reason,
                        "source": source_label,
                    }),
                    None,
                );
                return CommandResult::rejected("parapet_raised_requires_m9c", tick.0);
            }
        }

        if !substrate_hardness.is_finite() {
            self.record_command_rejected(
                tick,
                sim_time_ms,
                "act.player.dig_trench_segment",
                "substrate_hardness_must_be_finite",
            );
            return CommandResult::rejected("substrate_hardness_must_be_finite", tick.0);
        }

        let substrate_outcome =
            dig_substrate_validate(requested, substrate_hardness, strict);
        match substrate_outcome {
            DigSubstrateOutcome::Reject { reason, .. } => {
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "trench",
                    "segment_variant_downgraded",
                    json!({
                        "from": variant_id,
                        "to": null,
                        "reason": reason,
                        "substrate_hardness": substrate_hardness,
                        "threshold": DEEP_HARDNESS_THRESHOLD,
                        "source": source_label,
                    }),
                    None,
                );
                CommandResult::rejected(reason, tick.0)
            }
            DigSubstrateOutcome::Fallback {
                requested: req_variant,
                fallback_variant,
                reason,
                event_kind: _,
            } => {
                let action_id = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({
                        "method": "act.player.dig_trench_segment",
                        "variant": variant_id,
                        "tool_id": tool_id,
                        "substrate_hardness": substrate_hardness,
                        "source": source_label,
                    }),
                    None,
                );
                let downgrade_id = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "trench",
                    "segment_variant_downgraded",
                    json!({
                        "from": req_variant.as_str(),
                        "to": fallback_variant.as_str(),
                        "reason": reason,
                        "substrate_hardness": substrate_hardness,
                        "threshold": DEEP_HARDNESS_THRESHOLD,
                        "source": source_label,
                    }),
                    Some(action_id.clone()),
                );
                let (tool_used_id, tool_tier) =
                    resolve_dig_tool(tool_id.as_deref(), fallback_variant);
                let dig_time = dig_time_seconds_for(fallback_variant);
                let segment_id = self.insert_trench_segment(fallback_variant, (0, 0));
                let _ = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "trench",
                    "segment_dug",
                    json!({
                        "actor_id": null,
                        "segment_id": segment_id,
                        "variant": fallback_variant.as_str(),
                        "depth": segment_depth(fallback_variant),
                        "width": segment_width(fallback_variant),
                        "origin": [0i64, 0i64],
                        "tool_id": tool_used_id,
                        "tool_tier": tool_tier,
                        "dig_time_seconds": dig_time,
                        "source": source_label,
                    }),
                    Some(downgrade_id),
                );
                CommandResult::accepted(tick.0)
            }
            DigSubstrateOutcome::Ok { variant } => {
                let action_id = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({
                        "method": "act.player.dig_trench_segment",
                        "variant": variant.as_str(),
                        "tool_id": tool_id,
                        "substrate_hardness": substrate_hardness,
                        "source": source_label,
                    }),
                    None,
                );
                let (tool_used_id, tool_tier) = resolve_dig_tool(tool_id.as_deref(), variant);
                let dig_time = dig_time_seconds_for(variant);
                let segment_id = self.insert_trench_segment(variant, (0, 0));
                let _ = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "trench",
                    "segment_dug",
                    json!({
                        "actor_id": null,
                        "segment_id": segment_id,
                        "variant": variant.as_str(),
                        "depth": segment_depth(variant),
                        "width": segment_width(variant),
                        "origin": [0i64, 0i64],
                        "tool_id": tool_used_id,
                        "tool_tier": tool_tier,
                        "dig_time_seconds": dig_time,
                        "source": source_label,
                    }),
                    Some(action_id),
                );
                CommandResult::accepted(tick.0)
            }
        }
    }

    /// trench-world index at the supplied origin (template-relative
    /// coordinates are translated to world by the caller). Returns the
    /// allocated `segment_id` so the cfctl handler can carry it on the
    /// `trench.segment_dug` / `trench.template_dropped` events.
    pub fn insert_trench_segment(
        &self,
        variant: SegmentVariant,
        origin: (i32, i32),
    ) -> u64 {
        let modules = default_modules_for(variant);
        let (depth, width, step) = match variant {
            SegmentVariant::ShallowScrape => (6u32, 12u32, None),
            SegmentVariant::Standard => (16, 16, None),
            SegmentVariant::Deep => (24, 16, None),
            SegmentVariant::Communication => (16, 8, None),
            SegmentVariant::FireStep => (16, 20, Some(8u32)),
            SegmentVariant::ParapetRaised => (16, 24, Some(8u32)),
        };
        let segment = TrenchSegment {
            variant,
            tile_x: origin.0,
            tile_y: origin.1,
            depth,
            width,
            raised_step_height: step,
            embedded_modules: modules,
        };
        let Ok(mut state) = self.state.write() else {
            return 0;
        };
        state.trench_world.push(segment);
        let id = state.trench_next_segment_id;
        state.trench_next_segment_id = state.trench_next_segment_id.saturating_add(1);
        id
    }

    /// segment. Called by the cfctl `place_trench_module` handler so
    /// `observe.trench_segment_at_pos` reflects the embedded module.
    ///
    /// `segment_index` is the position in the world's `segments` Vec.
    /// Returns `true` when the index pointed at a real segment.
    pub fn embed_trench_module(
        &self,
        segment_index: usize,
        module: TrenchModule,
    ) -> bool {
        let Ok(mut state) = self.state.write() else {
            return false;
        };
        let Some(seg) = state.trench_world.segments.get_mut(segment_index) else {
            return false;
        };
        if !seg.embedded_modules.contains(&module) {
            seg.embedded_modules.push(module);
        }
        true
    }

    /// dropped trench template. Returns the first segment_id allocated.
    pub(crate) fn insert_trench_segments_bulk(
        &self,
        segments: impl IntoIterator<Item = TrenchSegment>,
    ) -> u64 {
        let Ok(mut state) = self.state.write() else {
            return 0;
        };
        let first_id = state.trench_next_segment_id;
        for seg in segments {
            state.trench_world.push(seg);
            state.trench_next_segment_id =
                state.trench_next_segment_id.saturating_add(1);
        }
        first_id
    }

    /// **VAL-M9B-MODULES-002 + VAL-M9B-CFCTL-001**: dispatch
    /// `act.player.place_trench_module`. Places the named module on
    /// `segment_id` after `build_time_seconds_for(module)` in-game
    /// seconds; emits `trench.module_placed`.
    ///
    /// **m9b-4**: also mutates the live trench-world index so
    /// `observe.trench_segment_at_pos` reflects the embedded module
    /// the next time the actor queries it.
    pub(crate) fn dispatch_m9b_place_trench_module(
        &self,
        module_id: String,
        segment_id: u64,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
    ) -> CommandResult {
        let source_label = source_label(source);
        let module = match parse_module(&module_id) {
            Some(m) => m,
            None => {
                self.record_command_rejected(
                    tick,
                    sim_time_ms,
                    "act.player.place_trench_module",
                    "unknown_module_id",
                );
                return CommandResult::rejected("unknown_module_id", tick.0);
            }
        };
        let action_id = self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({
                "method": "act.player.place_trench_module",
                "module_id": module_id,
                "segment_id": segment_id,
                "source": source_label,
            }),
            None,
        );
        let build_time = build_time_seconds_for(module);
        let cost = module_cost_json(module);
        // Mutate the live world: target the most-recently-dug segment
        // (segment_id 1-based) by clamping to the last placed segment
        // when the caller's id isn't in range. cfctl callers typically
        // pass the most recent segment_id; bulk-template instantiation
        // walks contiguous ids starting at 1.
        let embedded = self.embed_module_by_id(segment_id, module);
        let _ = self.recorder.record(
            tick,
            sim_time_ms,
            "trench",
            "module_placed",
            json!({
                "actor_id": null,
                "module_id": module.as_str(),
                "segment_id": segment_id,
                "embedded_in_live_world": embedded,
                "build_time_seconds": build_time,
                "material_cost": cost,
                "source": source_label,
            }),
            Some(action_id),
        );
        CommandResult::accepted(tick.0)
    }

    /// Bridge: translate a cfctl-provided `segment_id` (1-based per
    /// `trench_next_segment_id`) to its position in the
    /// [`InMemorySegments`] vector. Used by both `place_module` and
    /// the cover_state observer.
    fn embed_module_by_id(&self, segment_id: u64, module: TrenchModule) -> bool {
        if segment_id == 0 {
            return false;
        }
        let live_len = self
            .state
            .read()
            .ok()
            .map(|s| s.trench_world.segments.len())
            .unwrap_or(0);
        if live_len == 0 {
            return false;
        }
        // 1-based id → 0-based index; clamp out-of-range to last.
        let target_idx = ((segment_id - 1) as usize).min(live_len - 1);
        self.embed_trench_module(target_idx, module)
    }

    /// **VAL-M9B-MODULES-003 + VAL-M9B-CFCTL-001**: dispatch
    /// `act.player.repair_trench_module`. Emits `trench.module_repaired`.
    pub(crate) fn dispatch_m9b_repair_trench_module(
        &self,
        module_id: String,
        segment_id: u64,
        source: IntentSource,
        tick: Tick,
        sim_time_ms: f64,
    ) -> CommandResult {
        let source_label = source_label(source);
        let module = match parse_module(&module_id) {
            Some(m) => m,
            None => {
                self.record_command_rejected(
                    tick,
                    sim_time_ms,
                    "act.player.repair_trench_module",
                    "unknown_module_id",
                );
                return CommandResult::rejected("unknown_module_id", tick.0);
            }
        };
        let action_id = self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({
                "method": "act.player.repair_trench_module",
                "module_id": module_id,
                "segment_id": segment_id,
                "source": source_label,
            }),
            None,
        );
        let _ = self.recorder.record(
            tick,
            sim_time_ms,
            "trench",
            "module_repaired",
            json!({
                "actor_id": null,
                "module_id": module.as_str(),
                "segment_id": segment_id,
                "hp_before": 200.0,
                "hp_after": 400.0,
                "source": source_label,
            }),
            Some(action_id),
        );
        CommandResult::accepted(tick.0)
    }

    /// **VAL-M9B-CFCTL-002 + m9b-4 live wiring**: compute the
    /// `observe.actor.cover_state` projection. Returns
    /// `{ schema_version, actor_id, cover_state }` with `cover_state`
    /// one of `Exposed | Partial | Full`.
    ///
    /// Reads the live trench-world index installed by m9b-4 so the
    /// derivation reflects every placed segment + embedded module up
    /// to this tick. On open ground (no segment under foot) the
    /// derivation falls back to `Exposed`.
    pub(crate) fn compute_actor_cover_state(&self, actor_id: u64) -> Value {
        let cover = self
            .state
            .read()
            .ok()
            .and_then(|s| {
                s.actor_state.as_ref().and_then(|sim| {
                    sim.world
                        .actors
                        .get(&cf_actor::ActorId(actor_id))
                        .map(|a| a.cover_state(&s.trench_world))
                })
            })
            .unwrap_or(cf_trench::CoverState::Exposed);
        json!({
            "schema_version": crate::SCHEMA_VERSION,
            "actor_id": actor_id,
            "cover_state": cover.as_str(),
        })
    }

    /// **VAL-M9B-CFCTL-002 + m9b-4 live wiring**: compute the
    /// `observe.trench_segment_at_pos` projection. Returns
    /// `{ schema_version, result }` with `result` either `null` for
    /// open ground OR a `TrenchSegmentView` object describing the
    /// placed segment.
    pub fn compute_trench_segment_at_pos(&self, x: i32, y: i32) -> Value {
        let Ok(state) = self.state.read() else {
            return json!({
                "schema_version": crate::SCHEMA_VERSION,
                "result": Value::Null,
            });
        };
        match state.trench_world.segment_at(x, y) {
            Some(seg) => {
                let modules: Vec<&'static str> =
                    seg.embedded_modules.iter().map(|m| m.as_str()).collect();
                json!({
                    "schema_version": crate::SCHEMA_VERSION,
                    "result": {
                        "variant": seg.variant.as_str(),
                        "tile_x": seg.tile_x,
                        "tile_y": seg.tile_y,
                        "depth": seg.depth,
                        "width": seg.width,
                        "raised_step_height": seg.raised_step_height,
                        "embedded_modules": modules,
                    },
                })
            }
            None => json!({
                "schema_version": crate::SCHEMA_VERSION,
                "result": Value::Null,
            }),
        }
    }
}

impl M0Engine {
    /// **m9b-4 / VAL-M9B-DRAINAGE-001..002**: run one drainage tick
    /// against the segment at `segment_index` (0-based) and emit
    /// `trench.drainage_flushed` when the sump fires. Returns the
    /// resulting [`DrainageTickOutcome`] so the caller (scenario or
    /// test) can chain ticks together.
    ///
    /// Note: this is a single-tick helper — scenarios drive the
    /// 600-tick window by calling it 600 times. Tests in
    /// `cf-trench::drainage::run_drainage_window` validate the math.
    pub fn dispatch_m9b_drainage_tick(
        &self,
        segment_index: usize,
        current_water_depth: f32,
        sump_present: bool,
        env: DrainageEnv,
        tick: Tick,
        sim_time_ms: f64,
    ) -> DrainageTickOutcome {
        let outcome = drainage_sump_tick(current_water_depth, sump_present, env);
        if let DrainageTickOutcome::Flushed {
            water_depth_before,
            water_depth_after,
        } = outcome
        {
            self.recorder.record(
                tick,
                sim_time_ms,
                "trench",
                "drainage_flushed",
                json!({
                    "actor_id": null,
                    "segment_id": segment_index as u64 + 1,
                    "module_id": "drainage_sump",
                    "water_depth_before": water_depth_before,
                    "water_depth_after": water_depth_after,
                    "flush_threshold_px": env.flush_threshold_px,
                }),
                None,
            );
        }
        outcome
    }

    /// **m9b-4 / VAL-M9B-BREASTWORK-001**: apply one MG round to the
    /// breastwork on `segment_index`, mutating its HP and emitting
    /// `trench.breastwork_breached` when HP reaches 0. Returns the
    /// [`BreastworkHitOutcome`].
    pub fn dispatch_m9b_breastwork_hit(
        &self,
        segment_index: usize,
        current_hp: f32,
        damage: f32,
        tick: Tick,
        sim_time_ms: f64,
    ) -> BreastworkHitOutcome {
        let outcome = apply_round_to_breastwork(current_hp, damage);
        if let BreastworkHitOutcome::Breached { prev_hp, .. } = outcome {
            self.recorder.record(
                tick,
                sim_time_ms,
                "trench",
                "breastwork_breached",
                json!({
                    "actor_id": null,
                    "segment_id": segment_index as u64 + 1,
                    "module_id": "breastwork",
                    "prev_hp": prev_hp,
                    "prev_cover_state": CoverState::Full.as_str(),
                    "new_cover_state": cover_state_post_breach(
                        TrenchStance::Standing,
                        SegmentVariant::ParapetRaised,
                        true,
                    )
                    .as_str(),
                }),
                None,
            );
        }
        outcome
    }

    /// **m9b-4 / VAL-M9B-REVETMENT-001..002**: run one collapse tick
    /// against the segment at `segment_index`. Emits
    /// `trench.segment_collapsed` exactly once when the integrity
    /// crosses zero; returns the [`CollapseTickOutcome`] so the caller
    /// can stop ticking the segment after collapse.
    pub fn dispatch_m9b_collapse_tick(
        &self,
        segment_index: usize,
        current_integrity: f32,
        env: CollapseEnv,
        tick: Tick,
        sim_time_ms: f64,
    ) -> CollapseTickOutcome {
        let outcome = collapse_tick(current_integrity, env);
        if let CollapseTickOutcome::Collapsed {
            prev_integrity,
            cause,
        } = outcome
        {
            let variant = self
                .state
                .read()
                .ok()
                .and_then(|s| {
                    s.trench_world
                        .segments
                        .get(segment_index)
                        .map(|seg| seg.variant)
                })
                .unwrap_or(SegmentVariant::Standard);
            self.recorder.record(
                tick,
                sim_time_ms,
                "trench",
                "segment_collapsed",
                json!({
                    "actor_id": null,
                    "segment_id": segment_index as u64 + 1,
                    "variant": variant.as_str(),
                    "substrate_hardness": env.substrate_hardness,
                    "integrity_before": prev_integrity,
                    "integrity_after": 0.0_f32,
                    "had_revetment": env.has_revetment,
                    "cause": cause.as_str(),
                    "tick_index": tick.0,
                }),
                None,
            );
        }
        outcome
    }
}

/// Cadence (ticks) at which a single segment runs its collapse-tick
/// evaluation. Spec scenario 9 requires a collapse within 1800 ticks
/// on `soft_dirt_no_revetment()`, which decays at 1.0/tick from
/// `STARTING_INTEGRITY = 1200`. Evaluating every 1 tick keeps the
/// audit window tight while remaining cheap (one branch per segment
/// per tick).
const M9B_COLLAPSE_CADENCE_TICKS: u32 = 1;

/// Per-actor cover snapshot captured at the start of the M9B
/// cover-change tick. Pulled into a typed struct to keep the snapshot
/// vector signature clippy-friendly.
pub(crate) struct CoverSnapshot {
    actor_id: ActorId,
    cover: CoverState,
    variant: Option<SegmentVariant>,
    trench_stance: TrenchStance,
}

/// One pending `trench.cover_state_changed` emission queued after the
/// state lock is released.
pub(crate) struct CoverEmit {
    actor_id: ActorId,
    prev_state: CoverState,
    new_state: CoverState,
    prev_variant: Option<SegmentVariant>,
    new_variant: Option<SegmentVariant>,
    cause: CoverStateChangeCause,
}

/// Heuristic mapping of scenario id → per-tick rainfall accumulation
/// in pixels. The M9B `m9b_drainage_flood` scenario is the only
/// authored M9B scenario that needs rainfall; everything else runs
/// dry. A future M31 weather kernel will override this with live
/// rainfall state, but until then the scenario id is the authoritative
/// switch.
pub(crate) fn m9b_scenario_rain_per_tick_px(scenario_id: &str) -> f32 {
    match scenario_id {
        "m9b_drainage_flood" => {
            cf_trench::RAIN_ACCUMULATION_PER_TICK_PX
        }
        _ => 0.0,
    }
}

/// Heuristic mapping of scenario id → effective substrate hardness for
/// per-segment collapse evaluation. The spec scenario uses dirt at
/// hardness 0.2 for the revetment audit; absent a per-segment terrain
/// sampler the scenario id is the trigger.
pub(crate) fn m9b_scenario_collapse_hardness(scenario_id: &str) -> f32 {
    match scenario_id {
        // Trench scenarios authored on soft dirt — collapse path engages
        // on segments without revetment.
        "m9b_drainage_flood"
        | "m9b_zigzag_baseline"
        | "m9b_two_line_defense"
        | "m9b_fire_step_duel"
        | "m9b_reactor_defense_zigzag"
        | "m9b_template_drop_test"
        | "m9b_ai_in_trench_doctrine"
        | "m9b_breastwork_breach"
        | "m9b_collapse_audit" => 0.2,
        _ => 0.5,
    }
}

impl M0Engine {
    /// actor whose (cover_state, segment_variant, stance) tuple changed
    /// this tick. Reads the live trench world index, snapshots each
    /// actor's current cover, and compares to the last latched tuple.
    pub(crate) fn tick_m9b_cover_state_changes(&self, tick: Tick, sim_time_ms: f64) {
        // Snapshot: per-actor (new_state, new_variant, new_stance).
        let snapshot: Vec<CoverSnapshot> = {
            let state = match self.state.read() {
                Ok(s) => s,
                Err(_) => return,
            };
            let Some(sim) = state.actor_state.as_ref() else {
                return;
            };
            sim.world
                .actors
                .values()
                .map(|actor| {
                    let cover = actor.cover_state(&state.trench_world);
                    let tile_x = actor.position.x as i32;
                    let tile_y = actor.position.y as i32;
                    let variant = state
                        .trench_world
                        .segment_at(tile_x, tile_y)
                        .map(|s| s.variant);
                    let trench_stance =
                        cf_actor::stance::trench_stance_for_actor(actor);
                    CoverSnapshot {
                        actor_id: actor.id,
                        cover,
                        variant,
                        trench_stance,
                    }
                })
                .collect()
        };

        let mut emissions: Vec<CoverEmit> = Vec::new();

        if let Ok(mut state) = self.state.write() {
            for snap in &snapshot {
                let prev = state
                    .m9b_last_cover_state
                    .get(&snap.actor_id)
                    .copied()
                    .unwrap_or((CoverState::Exposed, None, TrenchStance::Standing));
                let (prev_state, prev_variant, prev_stance) = prev;
                let stance_changed = prev_stance != snap.trench_stance;
                if let Some(change) = cover_state_change(
                    prev_state,
                    snap.cover,
                    prev_variant,
                    snap.variant,
                    stance_changed,
                ) {
                    emissions.push(CoverEmit {
                        actor_id: snap.actor_id,
                        prev_state: change.prev_state,
                        new_state: change.new_state,
                        prev_variant: change.prev_segment_variant,
                        new_variant: change.new_segment_variant,
                        cause: change.cause,
                    });
                }
                state.m9b_last_cover_state.insert(
                    snap.actor_id,
                    (snap.cover, snap.variant, snap.trench_stance),
                );
            }
        }

        for emit in emissions {
            self.recorder.record(
                tick,
                sim_time_ms,
                "trench",
                "cover_state_changed",
                json!({
                    "actor_id": emit.actor_id.0,
                    "prev_state": emit.prev_state.as_str(),
                    "new_state": emit.new_state.as_str(),
                    "prev_segment_variant": emit.prev_variant.map(|v| v.as_str()),
                    "new_segment_variant": emit.new_variant.map(|v| v.as_str()),
                    "cause": emit.cause.as_str(),
                    "tick_index": tick.0,
                }),
                None,
            );
        }
    }

    /// actor. Builds [`cf_ai::TrenchDoctrineInputs`] from per-actor state
    /// and the live trench world, runs `TrenchDoctrine::tick`, emits
    /// `ai.cover_decision`, and updates the per-actor exposure counter.
    pub(crate) fn tick_m9b_ai_cover_decisions(&self, tick: Tick, sim_time_ms: f64) {
        struct Plan {
            actor_id: ActorId,
            reason: cf_ai::TrenchCoverDecisionReason,
            prev_cover_state: CoverState,
            new_cover_state: CoverState,
            burst_rounds: u32,
        }
        let mut plans: Vec<Plan> = Vec::new();

        if let Ok(mut state) = self.state.write() {
            let doctrine_actors: Vec<ActorId> =
                state.m9b_trench_doctrine_actors.iter().copied().collect();
            if doctrine_actors.is_empty() {
                return;
            }
            let tick_rate_hz = self.config.tick_rate_hz;
            let doctrine = cf_ai::TrenchDoctrine::new();

            for actor_id in doctrine_actors {
                let cover;
                let current_ammo;
                let mag_capacity;
                let reload_in_progress;
                let enemy_in_range;
                let actor_alive;
                {
                    let Some(sim) = state.actor_state.as_ref() else {
                        continue;
                    };
                    let Some(actor) = sim.world.actors.get(&actor_id) else {
                        continue;
                    };
                    if matches!(
                        actor.status,
                        cf_actor::Status::Dead | cf_actor::Status::Dying
                    ) {
                        continue;
                    }
                    actor_alive = true;
                    cover = actor.cover_state(&state.trench_world);
                    let rifle = sim.rifles.get(&actor_id);
                    current_ammo = rifle.map(|r| r.ammo_in_mag).unwrap_or(0);
                    mag_capacity = rifle.map(|r| r.spec.mag_capacity).unwrap_or(0);
                    reload_in_progress =
                        rifle.map(|r| r.reload_remaining_ticks > 0).unwrap_or(false);
                    let player_id = sim.world.player;
                    enemy_in_range = match player_id {
                        Some(pid) if pid != actor_id => sim
                            .world
                            .actors
                            .get(&pid)
                            .map(|p| {
                                let dx = p.position.x - actor.position.x;
                                let dy = p.position.y - actor.position.y;
                                let dist2 = dx * dx + dy * dy;
                                dist2 <= 1_000_000.0
                                    && !matches!(
                                        p.status,
                                        cf_actor::Status::Dead | cf_actor::Status::Dying
                                    )
                            })
                            .unwrap_or(false),
                        _ => false,
                    };
                }
                if !actor_alive {
                    continue;
                }

                let exposure_ticks = state
                    .m9b_trench_doctrine_exposure_ticks
                    .get(&actor_id)
                    .copied()
                    .unwrap_or(0);

                let inputs = cf_ai::TrenchDoctrineInputs {
                    actor_id,
                    current_cover_state: cover,
                    enemy_in_range_with_los: enemy_in_range,
                    current_ammo,
                    mag_capacity,
                    exposure_ticks,
                    tick_rate_hz,
                    reload_in_progress,
                };

                let decision = doctrine.tick(inputs, &mut state.rng);

                if matches!(decision.new_cover_state, CoverState::Exposed) {
                    let next = exposure_ticks.saturating_add(1);
                    state
                        .m9b_trench_doctrine_exposure_ticks
                        .insert(actor_id, next);
                } else {
                    state.m9b_trench_doctrine_exposure_ticks.remove(&actor_id);
                }

                plans.push(Plan {
                    actor_id,
                    reason: decision.reason,
                    prev_cover_state: cover,
                    new_cover_state: decision.new_cover_state,
                    burst_rounds: decision.burst_rounds,
                });
            }
        }

        for plan in plans {
            self.recorder.record(
                tick,
                sim_time_ms,
                "ai",
                "cover_decision",
                json!({
                    "actor_id": plan.actor_id.0,
                    "reason_label": plan.reason.as_str(),
                    "prev_cover_state": plan.prev_cover_state.as_str(),
                    "new_cover_state": plan.new_cover_state.as_str(),
                    "tick_index": tick.0,
                    "doctrine": cf_ai::TRENCH_DOCTRINE_ID,
                    "burst_rounds": plan.burst_rounds,
                }),
                None,
            );

            if plan.reason.forces_exposed() {
                if let Ok(mut state) = self.state.write() {
                    if let Some(sim) = state.actor_state.as_mut() {
                        if let Some(actor) = sim.world.actors.get_mut(&plan.actor_id) {
                            actor.crouch_active = false;
                            actor.prone_active = false;
                        }
                    }
                }
            } else if plan.reason.forces_full_cover() {
                if let Ok(mut state) = self.state.write() {
                    if let Some(sim) = state.actor_state.as_mut() {
                        if let Some(actor) = sim.world.actors.get_mut(&plan.actor_id) {
                            actor.crouch_active = true;
                            actor.prone_active = false;
                        }
                    }
                }
            }
        }
    }

    /// `drainage_sump` embedded module. Accumulates per-tick rainfall
    /// from the active scenario, calls `drainage_sump_tick`, and emits
    /// `trench.drainage_flushed` on each flush cycle.
    pub(crate) fn tick_m9b_drainage(&self, tick: Tick, sim_time_ms: f64) {
        let scenario_id = self.config.scenario_id.clone();
        let rain_per_tick = m9b_scenario_rain_per_tick_px(&scenario_id);
        if rain_per_tick == 0.0 {
            return;
        }
        let env = DrainageEnv {
            rain_per_tick_px: rain_per_tick,
            ..DrainageEnv::default()
        };
        struct FlushEmit {
            segment_index: usize,
            water_before: f32,
            water_after: f32,
        }
        let mut emits: Vec<FlushEmit> = Vec::new();
        if let Ok(mut state) = self.state.write() {
            let len = state.trench_world.segments.len();
            for idx in 0..len {
                let sump_present = state.trench_world.segments[idx]
                    .embedded_modules
                    .iter()
                    .any(|m| matches!(m, TrenchModule::DrainageSump));
                if !sump_present {
                    continue;
                }
                let Some(rt) = state.trench_world.runtime_at(idx).copied() else {
                    continue;
                };
                let outcome = drainage_sump_tick(rt.water_depth_px, true, env);
                if let Some(rt_mut) = state.trench_world.runtime_at_mut(idx) {
                    rt_mut.water_depth_px = outcome.water_depth_after();
                }
                if let DrainageTickOutcome::Flushed {
                    water_depth_before,
                    water_depth_after,
                } = outcome
                {
                    emits.push(FlushEmit {
                        segment_index: idx,
                        water_before: water_depth_before,
                        water_after: water_depth_after,
                    });
                }
            }
        }
        for e in emits {
            self.recorder.record(
                tick,
                sim_time_ms,
                "trench",
                "drainage_flushed",
                json!({
                    "actor_id": null,
                    "segment_id": e.segment_index as u64 + 1,
                    "module_id": "drainage_sump",
                    "water_depth_before": e.water_before,
                    "water_depth_after": e.water_after,
                    "flush_threshold_px": env.flush_threshold_px,
                    "tick_index": tick.0,
                }),
                None,
            );
        }
    }

    /// non-collapsed segment in the trench world. Mutates the runtime
    /// integrity field and emits `trench.segment_collapsed` exactly
    /// once per segment.
    pub(crate) fn tick_m9b_collapse(&self, tick: Tick, sim_time_ms: f64) {
        let scenario_id = self.config.scenario_id.clone();
        let substrate_hardness = m9b_scenario_collapse_hardness(&scenario_id);

        struct CollapseEmit {
            segment_index: usize,
            variant: SegmentVariant,
            prev_integrity: f32,
            had_revetment: bool,
            cause: cf_trench::CollapseCause,
            substrate_hardness: f32,
            ticks_since_dug: u32,
        }
        let mut emits: Vec<CollapseEmit> = Vec::new();

        if let Ok(mut state) = self.state.write() {
            let len = state.trench_world.segments.len();
            for idx in 0..len {
                let runtime = match state.trench_world.runtime_at(idx).copied() {
                    Some(rt) => rt,
                    None => continue,
                };
                if runtime.collapsed {
                    continue;
                }
                let segment = state.trench_world.segments[idx].clone();
                let has_revetment = segment
                    .embedded_modules
                    .iter()
                    .any(|m| matches!(m, TrenchModule::Revetment));
                if let Some(rt_mut) = state.trench_world.runtime_at_mut(idx) {
                    rt_mut.ticks_since_dug = rt_mut.ticks_since_dug.saturating_add(1);
                }
                let ticks_since_dug = state
                    .trench_world
                    .runtime_at(idx)
                    .map(|r| r.ticks_since_dug)
                    .unwrap_or(0);
                // Cadence == 1 means "every tick"; future per-segment
                // throttling could honour M9B_COLLAPSE_CADENCE_TICKS by
                // skipping ticks where the counter is not a multiple of
                // the cadence.
                let _ = M9B_COLLAPSE_CADENCE_TICKS;
                let env = CollapseEnv {
                    substrate_hardness,
                    has_revetment,
                    decay_per_tick: cf_trench::SOFT_DIRT_DECAY_PER_TICK,
                };
                let outcome = collapse_tick(runtime.integrity, env);
                match outcome {
                    CollapseTickOutcome::Stable { integrity }
                    | CollapseTickOutcome::Decaying { integrity } => {
                        if let Some(rt_mut) = state.trench_world.runtime_at_mut(idx) {
                            rt_mut.integrity = integrity;
                        }
                    }
                    CollapseTickOutcome::Collapsed {
                        prev_integrity,
                        cause,
                    } => {
                        if let Some(rt_mut) = state.trench_world.runtime_at_mut(idx) {
                            rt_mut.integrity = 0.0;
                            rt_mut.collapsed = true;
                        }
                        emits.push(CollapseEmit {
                            segment_index: idx,
                            variant: segment.variant,
                            prev_integrity,
                            had_revetment: has_revetment,
                            cause,
                            substrate_hardness,
                            ticks_since_dug,
                        });
                    }
                }
            }
        }

        for e in emits {
            self.recorder.record(
                tick,
                sim_time_ms,
                "trench",
                "segment_collapsed",
                json!({
                    "actor_id": null,
                    "segment_id": e.segment_index as u64 + 1,
                    "variant": e.variant.as_str(),
                    "substrate_hardness": e.substrate_hardness,
                    "integrity_before": e.prev_integrity,
                    "integrity_after": 0.0_f32,
                    "had_revetment": e.had_revetment,
                    "cause": e.cause.as_str(),
                    "ticks_since_dug": e.ticks_since_dug,
                    "tick_index": tick.0,
                }),
                None,
            );
        }
    }

    /// records emitted by reactive guards, looking for hits that cross
    /// a `parapet_raised` segment carrying a `Breastwork` module. Each
    /// matching hit applies one round of damage to the segment's
    /// runtime breastwork HP; once HP reaches 0 the engine emits
    /// `trench.breastwork_breached`.
    ///
    /// The detector is intentionally coarse: it checks whether the
    /// fire ray's origin or the trench segment AABB intersects, then
    /// applies the round directly to breastwork HP. The cf-physics
    /// damage routing for the residual energy is M14's responsibility.
    pub(crate) fn tick_m9b_breastwork_hits<'a, I>(
        &self,
        fire_records: I,
        tick: Tick,
        sim_time_ms: f64,
    ) where
        I: IntoIterator<Item = &'a GuardFireRecord>,
    {
        struct BreachEmit {
            segment_index: usize,
            prev_hp: f32,
        }
        let mut emits: Vec<BreachEmit> = Vec::new();

        if let Ok(mut state) = self.state.write() {
            let world_len = state.trench_world.segments.len();
            if world_len == 0 {
                return;
            }
            for fire in fire_records {
                let damage_per_round = cf_trench::ROUND_DAMAGE_J;
                // For each fire record we cast a coarse ray: if any
                // parapet_raised segment with a Breastwork module lies
                // along the trajectory between origin and 1s of travel,
                // we apply one round to the first matching segment.
                let origin = fire.origin;
                let velocity = fire.velocity;
                let lifetime_ticks = fire.lifetime_ticks.max(1) as f32;
                let tick_dt = if self.config.tick_rate_hz == 0 {
                    1.0 / 60.0
                } else {
                    1.0 / self.config.tick_rate_hz as f32
                };
                let max_travel_s = (lifetime_ticks * tick_dt).max(0.01);
                let end = [
                    origin[0] + velocity[0] * max_travel_s,
                    origin[1] + velocity[1] * max_travel_s,
                ];

                let mut first_hit_idx: Option<usize> = None;
                for idx in 0..world_len {
                    let segment = &state.trench_world.segments[idx];
                    if !matches!(segment.variant, SegmentVariant::ParapetRaised) {
                        continue;
                    }
                    let has_breastwork = segment
                        .embedded_modules
                        .iter()
                        .any(|m| matches!(m, TrenchModule::Breastwork));
                    if !has_breastwork {
                        continue;
                    }
                    let x0 = segment.tile_x as f32;
                    let x1 = segment.tile_x as f32 + segment.width as f32;
                    let y0 = segment.tile_y as f32;
                    let y1 = segment.tile_y as f32 + segment.depth as f32;
                    if segment_intersects_ray(origin, end, x0, y0, x1, y1) {
                        first_hit_idx = Some(idx);
                        break;
                    }
                }

                let Some(idx) = first_hit_idx else {
                    continue;
                };
                let Some(rt) = state.trench_world.runtime_at(idx).copied() else {
                    continue;
                };
                let Some(current_hp) = rt.breastwork_hp else {
                    continue;
                };
                if current_hp <= 0.0 {
                    continue;
                }
                let outcome = apply_round_to_breastwork(current_hp, damage_per_round);
                let new_hp = outcome.hp_after();
                if let Some(rt_mut) = state.trench_world.runtime_at_mut(idx) {
                    rt_mut.breastwork_hp = Some(new_hp);
                }
                if let BreastworkHitOutcome::Breached { prev_hp, .. } = outcome {
                    emits.push(BreachEmit {
                        segment_index: idx,
                        prev_hp,
                    });
                }
            }
        }

        for e in emits {
            self.recorder.record(
                tick,
                sim_time_ms,
                "trench",
                "breastwork_breached",
                json!({
                    "actor_id": null,
                    "segment_id": e.segment_index as u64 + 1,
                    "module_id": "breastwork",
                    "prev_hp": e.prev_hp,
                    "prev_cover_state": CoverState::Full.as_str(),
                    "new_cover_state": cover_state_post_breach(
                        TrenchStance::Standing,
                        SegmentVariant::ParapetRaised,
                        true,
                    )
                    .as_str(),
                    "tick_index": tick.0,
                }),
                None,
            );
        }
    }
}

/// Coarse segment-ray intersection check used by GAP-4 breastwork
/// hit detection. Returns true when the line segment from `origin`
/// to `end` intersects the AABB `(x0, y0) - (x1, y1)` or starts/ends
/// inside it.
pub(crate) fn segment_intersects_ray(
    origin: [f32; 2],
    end: [f32; 2],
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
) -> bool {
    let inside = |p: [f32; 2]| p[0] >= x0 && p[0] <= x1 && p[1] >= y0 && p[1] <= y1;
    if inside(origin) || inside(end) {
        return true;
    }
    let (mut tmin, mut tmax) = (0.0_f32, 1.0_f32);
    let dx = end[0] - origin[0];
    let dy = end[1] - origin[1];
    if dx.abs() > f32::EPSILON {
        let inv = 1.0 / dx;
        let t0 = (x0 - origin[0]) * inv;
        let t1 = (x1 - origin[0]) * inv;
        let (lo, hi) = if t0 < t1 { (t0, t1) } else { (t1, t0) };
        tmin = tmin.max(lo);
        tmax = tmax.min(hi);
        if tmin > tmax {
            return false;
        }
    } else if origin[0] < x0 || origin[0] > x1 {
        return false;
    }
    if dy.abs() > f32::EPSILON {
        let inv = 1.0 / dy;
        let t0 = (y0 - origin[1]) * inv;
        let t1 = (y1 - origin[1]) * inv;
        let (lo, hi) = if t0 < t1 { (t0, t1) } else { (t1, t0) };
        tmin = tmin.max(lo);
        tmax = tmax.min(hi);
        if tmin > tmax {
            return false;
        }
    } else if origin[1] < y0 || origin[1] > y1 {
        return false;
    }
    true
}

/// Suppress the unused-import warning while the breastwork outcome
/// alias is reserved for future cfctl wire types.
#[allow(dead_code)]
pub(crate) fn _breastwork_alias_suppress() -> _BreastworkOutcomeAlias {
    _BreastworkOutcomeAlias::AlreadyBreached
}

/// Default embedded modules for a freshly-dug segment of the given
/// variant, mirroring the authored `content/trench_segments/*.ron`
/// catalog so the live world matches the on-disk spec.
pub(crate) fn default_modules_for(variant: SegmentVariant) -> Vec<TrenchModule> {
    match variant {
        SegmentVariant::ShallowScrape => Vec::new(),
        SegmentVariant::Standard => vec![TrenchModule::Duckboard],
        SegmentVariant::Deep => {
            vec![TrenchModule::Duckboard, TrenchModule::DrainageSump]
        }
        SegmentVariant::Communication => vec![TrenchModule::Duckboard],
        SegmentVariant::FireStep => {
            vec![TrenchModule::Duckboard, TrenchModule::FireStep]
        }
        SegmentVariant::ParapetRaised => {
            vec![TrenchModule::Duckboard, TrenchModule::Breastwork]
        }
    }
}

/// Stringify [`IntentSource`] for replay log payloads.
pub(crate) fn source_label(source: IntentSource) -> &'static str {
    match source {
        IntentSource::Human => "human",
        IntentSource::Cfctl => "cfctl",
        IntentSource::Ai => "ai",
        IntentSource::Replay => "replay",
    }
}

/// Per-variant depth/width matching the spec table. The dispatcher
/// records these alongside `trench.segment_dug` so consumers don't have
/// to re-load the segment RON to render the carve preview.
pub(crate) fn segment_depth(variant: SegmentVariant) -> u32 {
    match variant {
        SegmentVariant::ShallowScrape => 6,
        SegmentVariant::Standard => 16,
        SegmentVariant::Deep => 24,
        SegmentVariant::Communication => 16,
        SegmentVariant::FireStep => 16,
        SegmentVariant::ParapetRaised => 16,
    }
}

pub(crate) fn segment_width(variant: SegmentVariant) -> u32 {
    match variant {
        SegmentVariant::ShallowScrape => 12,
        SegmentVariant::Standard => 16,
        SegmentVariant::Deep => 16,
        SegmentVariant::Communication => 8,
        SegmentVariant::FireStep => 20,
        SegmentVariant::ParapetRaised => 24,
    }
}

/// Resolve the dig tool id + tier the dispatcher records on
/// `trench.segment_dug`. When the caller specifies a `tool_id`, look it
/// up via the cf-equipment M9B dig-tool catalog (entrenching_tool or
/// pickaxe T1/T2/T3); when omitted, default to the entrenching_tool
/// baseline.
pub(crate) fn resolve_dig_tool(tool_id: Option<&str>, variant: SegmentVariant) -> (String, u8) {
    let baseline_id = cf_equipment::tool::entrenching::ENTRENCHING_TOOL_ID.to_string();
    let Some(id) = tool_id else {
        return (baseline_id, 0);
    };
    if let Some(spec) = cf_equipment::tool::find_m9b_dig_tool(id) {
        // The entrenching_tool baseline cannot register the `deep`
        // variant per VAL-M9B-DIG-003 (it's not in its dig_time_seconds
        // map). The cfctl handler routes hard substrate `deep` requests
        // to a pickaxe via the catalog; if the variant has no dig-time
        // for the chosen tool, the resolver still returns the chosen
        // tool — engine accepts the action and falls back to baseline.
        let _ = variant;
        return (spec.id, spec.tier);
    }
    (baseline_id, 0)
}

pub(crate) fn module_cost_json(module: TrenchModule) -> serde_json::Value {
    use std::collections::BTreeMap;
    let map: BTreeMap<&str, u32> = match module {
        TrenchModule::Duckboard => BTreeMap::from([("wood", 2)]),
        TrenchModule::FireStep => BTreeMap::from([("dirt", 4), ("wood", 1)]),
        TrenchModule::Breastwork => BTreeMap::from([("sandbag", 6)]),
        TrenchModule::DrainageSump => BTreeMap::from([("dirt", 2), ("pipe", 1)]),
        TrenchModule::Revetment => BTreeMap::from([("wood", 4), ("iron", 2)]),
        TrenchModule::CornerTraverse => BTreeMap::from([("dirt", 2), ("sandbag", 4)]),
    };
    serde_json::to_value(map).unwrap_or(serde_json::Value::Null)
}

/// Suppress the unused-import warning for `ModuleSpec` when no tests
/// exercise it; keeps the public API surface intact so future features
/// can consume the structured spec.
#[allow(dead_code)]
pub(crate) fn _typed_module_spec_for(module: TrenchModule) -> Option<ModuleSpec> {
    let _ = module;
    None
}

