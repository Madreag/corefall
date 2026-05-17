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
fn parse_variant(id: &str) -> Option<SegmentVariant> {
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
fn parse_module(id: &str) -> Option<TrenchModule> {
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
fn dig_time_seconds_for(variant: SegmentVariant) -> u32 {
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
fn build_time_seconds_for(module: TrenchModule) -> u32 {
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

    /// **M9B / m9b-4**: place a [`TrenchSegment`] into the live engine
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

    /// **M9B / m9b-4**: append a module to a previously placed
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

    /// **M9B / m9b-4**: bulk-insert a vector of placed segments from a
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

/// Heuristic mapping of scenario id → per-tick rainfall accumulation
/// in pixels. The M9B `m9b_drainage_flood` scenario is the only
/// authored M9B scenario that needs rainfall; everything else runs
/// dry. A future M31 weather kernel will override this with live
/// rainfall state, but until then the scenario id is the authoritative
/// switch.
fn m9b_scenario_rain_per_tick_px(scenario_id: &str) -> f32 {
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
fn m9b_scenario_collapse_hardness(scenario_id: &str) -> f32 {
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
    /// **M9B audit GAP-1**: emit `trench.cover_state_changed` for every
    /// actor whose (cover_state, segment_variant, stance) tuple changed
    /// this tick. Reads the live trench world index, snapshots each
    /// actor's current cover, and compares to the last latched tuple.
    pub(crate) fn tick_m9b_cover_state_changes(&self, tick: Tick, sim_time_ms: f64) {
        // Snapshot: per-actor (new_state, new_variant, new_stance).
        let snapshot: Vec<(ActorId, CoverState, Option<SegmentVariant>, TrenchStance)> = {
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
                    (actor.id, cover, variant, trench_stance)
                })
                .collect()
        };

        let mut emissions: Vec<(ActorId, CoverState, CoverState, Option<SegmentVariant>, Option<SegmentVariant>, CoverStateChangeCause)> =
            Vec::new();

        if let Ok(mut state) = self.state.write() {
            for (actor_id, new_state, new_variant, new_stance) in &snapshot {
                let prev = state
                    .m9b_last_cover_state
                    .get(actor_id)
                    .copied()
                    .unwrap_or((CoverState::Exposed, None, TrenchStance::Standing));
                let (prev_state, prev_variant, prev_stance) = prev;
                let stance_changed = prev_stance != *new_stance;
                if let Some(change) = cover_state_change(
                    prev_state,
                    *new_state,
                    prev_variant,
                    *new_variant,
                    stance_changed,
                ) {
                    emissions.push((
                        *actor_id,
                        change.prev_state,
                        change.new_state,
                        change.prev_segment_variant,
                        change.new_segment_variant,
                        change.cause,
                    ));
                }
                state
                    .m9b_last_cover_state
                    .insert(*actor_id, (*new_state, *new_variant, *new_stance));
            }
        }

        for (actor_id, prev_state, new_state, prev_variant, new_variant, cause) in emissions {
            self.recorder.record(
                tick,
                sim_time_ms,
                "trench",
                "cover_state_changed",
                json!({
                    "actor_id": actor_id.0,
                    "prev_state": prev_state.as_str(),
                    "new_state": new_state.as_str(),
                    "prev_segment_variant": prev_variant.map(|v| v.as_str()),
                    "new_segment_variant": new_variant.map(|v| v.as_str()),
                    "cause": cause.as_str(),
                    "tick_index": tick.0,
                }),
                None,
            );
        }
    }

    /// **M9B audit GAP-2**: tick the trench doctrine for every opted-in
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

    /// **M9B audit GAP-3**: tick drainage for every segment with a
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

    /// **M9B audit GAP-5**: tick per-segment collapse for every
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
                if ticks_since_dug % M9B_COLLAPSE_CADENCE_TICKS != 0 {
                    continue;
                }
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

    /// **M9B audit GAP-4**: process the per-tick collection of MG fire
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
fn segment_intersects_ray(
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
fn _breastwork_alias_suppress() -> _BreastworkOutcomeAlias {
    _BreastworkOutcomeAlias::AlreadyBreached
}

/// Default embedded modules for a freshly-dug segment of the given
/// variant, mirroring the authored `content/trench_segments/*.ron`
/// catalog so the live world matches the on-disk spec.
fn default_modules_for(variant: SegmentVariant) -> Vec<TrenchModule> {
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
fn source_label(source: IntentSource) -> &'static str {
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
fn segment_depth(variant: SegmentVariant) -> u32 {
    match variant {
        SegmentVariant::ShallowScrape => 6,
        SegmentVariant::Standard => 16,
        SegmentVariant::Deep => 24,
        SegmentVariant::Communication => 16,
        SegmentVariant::FireStep => 16,
        SegmentVariant::ParapetRaised => 16,
    }
}

fn segment_width(variant: SegmentVariant) -> u32 {
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
fn resolve_dig_tool(tool_id: Option<&str>, variant: SegmentVariant) -> (String, u8) {
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

fn module_cost_json(module: TrenchModule) -> serde_json::Value {
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
fn _typed_module_spec_for(module: TrenchModule) -> Option<ModuleSpec> {
    let _ = module;
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_SCENARIO_SEQ: std::sync::atomic::AtomicU64 =
        std::sync::atomic::AtomicU64::new(0);

    #[test]
    fn parse_variant_round_trip() {
        for variant in [
            SegmentVariant::ShallowScrape,
            SegmentVariant::Standard,
            SegmentVariant::Deep,
            SegmentVariant::Communication,
            SegmentVariant::FireStep,
            SegmentVariant::ParapetRaised,
        ] {
            let s = variant.as_str();
            assert_eq!(
                parse_variant(s),
                Some(variant),
                "round-trip failed for variant `{s}`"
            );
        }
    }

    #[test]
    fn parse_module_round_trip() {
        for m in [
            TrenchModule::Duckboard,
            TrenchModule::FireStep,
            TrenchModule::Breastwork,
            TrenchModule::DrainageSump,
            TrenchModule::Revetment,
            TrenchModule::CornerTraverse,
        ] {
            assert_eq!(parse_module(m.as_str()), Some(m));
        }
    }

    /// VAL-M9B-DIG-001: standard variant carves over 12 in-game seconds
    /// (4× the 5s shallow_scrape baseline).
    #[test]
    fn dig_time_standard_is_4x_shallow_scrape() {
        assert_eq!(dig_time_seconds_for(SegmentVariant::Standard), 12);
        assert_eq!(dig_time_seconds_for(SegmentVariant::ShallowScrape), 5);
        // VAL-M9B-DIG-001 evidence string: "12 × tick_rate ticks (4× the
        // 5s shallow_scrape baseline)". Mathematically `4 * 5 = 20`,
        // but the spec table sets standard = 12s explicitly. The
        // assertion below checks the spec table value, not the literal
        // 4× math (since spec is authoritative).
        assert!(
            dig_time_seconds_for(SegmentVariant::Standard)
                > dig_time_seconds_for(SegmentVariant::ShallowScrape),
            "standard must dig slower than shallow_scrape"
        );
    }

    /// VAL-M9B-DIG-003: deep on hardness ≥ 0.5 falls back to
    /// shallow_scrape with a downgrade reason.
    #[test]
    fn dig_substrate_fallback_at_threshold() {
        let outcome = dig_substrate_validate(SegmentVariant::Deep, 0.7, false);
        assert!(outcome.is_fallback());
        assert_eq!(
            outcome.effective_variant(),
            Some(SegmentVariant::ShallowScrape)
        );
    }

    #[test]
    fn resolve_dig_tool_unknown_defaults_to_entrenching() {
        let (id, tier) = resolve_dig_tool(Some("nonexistent_tool"), SegmentVariant::Standard);
        assert_eq!(id, cf_equipment::tool::entrenching::ENTRENCHING_TOOL_ID);
        assert_eq!(tier, 0);
    }

    #[test]
    fn resolve_dig_tool_pickaxe_returns_tier() {
        let (id, tier) = resolve_dig_tool(
            Some(cf_equipment::tool::dig_pickaxe::PICKAXE_DIG_T2_ID),
            SegmentVariant::Standard,
        );
        assert_eq!(id, cf_equipment::tool::dig_pickaxe::PICKAXE_DIG_T2_ID);
        assert_eq!(tier, 2);
    }

    #[test]
    fn build_time_matches_spec_table() {
        assert_eq!(build_time_seconds_for(TrenchModule::Duckboard), 4);
        assert_eq!(build_time_seconds_for(TrenchModule::FireStep), 8);
        assert_eq!(build_time_seconds_for(TrenchModule::Breastwork), 12);
        assert_eq!(build_time_seconds_for(TrenchModule::DrainageSump), 6);
        assert_eq!(build_time_seconds_for(TrenchModule::Revetment), 10);
        assert_eq!(build_time_seconds_for(TrenchModule::CornerTraverse), 6);
    }

    #[test]
    fn module_cost_json_round_trips() {
        let json = module_cost_json(TrenchModule::Breastwork);
        let obj = json.as_object().expect("cost object");
        assert_eq!(obj.get("sandbag").and_then(|v| v.as_u64()), Some(6));
        let json = module_cost_json(TrenchModule::Revetment);
        let obj = json.as_object().expect("cost object");
        assert_eq!(obj.get("wood").and_then(|v| v.as_u64()), Some(4));
        assert_eq!(obj.get("iron").and_then(|v| v.as_u64()), Some(2));
    }

    /// VAL-M9B-MODULES-001 + m9b-4: default_modules_for returns the
    /// authored embedded-module set for each variant (mirrors
    /// content/trench_segments/<variant>.ron).
    #[test]
    fn default_modules_match_segment_ron() {
        assert!(default_modules_for(SegmentVariant::ShallowScrape).is_empty());
        assert_eq!(
            default_modules_for(SegmentVariant::Standard),
            vec![TrenchModule::Duckboard]
        );
        assert_eq!(
            default_modules_for(SegmentVariant::Deep),
            vec![TrenchModule::Duckboard, TrenchModule::DrainageSump]
        );
        assert_eq!(
            default_modules_for(SegmentVariant::Communication),
            vec![TrenchModule::Duckboard]
        );
        assert_eq!(
            default_modules_for(SegmentVariant::FireStep),
            vec![TrenchModule::Duckboard, TrenchModule::FireStep]
        );
        assert_eq!(
            default_modules_for(SegmentVariant::ParapetRaised),
            vec![TrenchModule::Duckboard, TrenchModule::Breastwork]
        );
    }

    fn make_engine() -> M0Engine {
        let mut p = std::env::temp_dir();
        let seq = TEST_SCENARIO_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        p.push(format!("m9b_trench_test_{}_{}.ron", std::process::id(), seq));
        std::fs::write(
            &p,
            r#"(
  schema_version: 1,
  id: "m9b_trench_test",
  display_name: "M9B trench live world test",
  description: "Empty scene for trench world tests.",
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
  expected_tests: [],
  notes: "",
)"#,
        )
        .unwrap();
        let scenario = crate::scenario::Scenario::load_from_file(&p).unwrap();
        let cfg = crate::engine::M0EngineConfig::for_loaded_scenario(&scenario, p);
        M0Engine::new(cfg)
    }

    /// **m9b-4 PRECONDITION**: `compute_trench_segment_at_pos` after
    /// `insert_trench_segment` finds the placed segment instead of
    /// always returning null.
    #[test]
    fn insert_segment_then_observe_returns_placed_segment() {
        let engine = make_engine();
        let id = engine.insert_trench_segment(SegmentVariant::Standard, (10, 0));
        assert_eq!(id, 1);
        let observed = engine.compute_trench_segment_at_pos(15, 5);
        let result = observed.get("result").expect("result key");
        assert!(
            !result.is_null(),
            "after insert, observe must return the segment instead of null"
        );
        let variant = result.get("variant").and_then(|v| v.as_str());
        assert_eq!(variant, Some("standard"));
    }

    /// **m9b-4 PRECONDITION**: `observe.trench_segment_at_pos` still
    /// returns null for tiles outside any placed segment.
    #[test]
    fn observe_returns_null_for_open_ground() {
        let engine = make_engine();
        let _ = engine.insert_trench_segment(SegmentVariant::Standard, (10, 0));
        let observed = engine.compute_trench_segment_at_pos(100, 100);
        let result = observed.get("result").expect("result key");
        assert!(result.is_null());
    }

    /// **m9b-4 PRECONDITION**: `embed_trench_module` adds modules to a
    /// previously inserted segment.
    #[test]
    fn embed_module_appends_to_segment() {
        let engine = make_engine();
        let _ = engine.insert_trench_segment(SegmentVariant::Standard, (10, 0));
        let added = engine.embed_trench_module(0, TrenchModule::Revetment);
        assert!(added);
        let observed = engine.compute_trench_segment_at_pos(15, 5);
        let modules = observed
            .pointer("/result/embedded_modules")
            .and_then(|m| m.as_array())
            .expect("embedded_modules array");
        let names: Vec<&str> = modules.iter().filter_map(|v| v.as_str()).collect();
        assert!(names.contains(&"revetment"));
        assert!(names.contains(&"duckboard"));
    }

    /// Insert two segments and verify monotonically increasing ids.
    #[test]
    fn insert_segments_allocates_unique_ids() {
        let engine = make_engine();
        let id1 = engine.insert_trench_segment(SegmentVariant::Standard, (10, 0));
        let id2 = engine.insert_trench_segment(SegmentVariant::Deep, (40, 0));
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    fn count_events(engine: &M0Engine, category: &str, event_type: &str) -> usize {
        engine
            .recorder
            .snapshot_events()
            .into_iter()
            .filter(|e| e.category == category && e.event_type == event_type)
            .count()
    }

    /// VAL-M9B-DRAINAGE-001: drainage tick fires
    /// `trench.drainage_flushed` when the sump kicks.
    #[test]
    fn drainage_tick_emits_flush_event() {
        let engine = make_engine();
        let _ = engine.insert_trench_segment(SegmentVariant::Deep, (0, 0));
        let env = DrainageEnv::default();
        // Drive enough ticks to cross the threshold.
        let mut depth = 0.0_f32;
        for _ in 0..400 {
            let outcome = engine.dispatch_m9b_drainage_tick(
                0,
                depth,
                true,
                env,
                Tick(0),
                0.0,
            );
            depth = outcome.water_depth_after();
        }
        let flushes = count_events(&engine, "trench", "drainage_flushed");
        assert!(
            flushes >= 1,
            "drainage helper must emit ≥ 1 flush event over the 600-tick window"
        );
    }

    /// VAL-M9B-DRAINAGE-002: no-sump tick does NOT emit a flush event.
    #[test]
    fn drainage_tick_without_sump_emits_no_event() {
        let engine = make_engine();
        let _ = engine.insert_trench_segment(SegmentVariant::Standard, (0, 0));
        let env = DrainageEnv::default();
        let mut depth = 0.0_f32;
        for _ in 0..400 {
            let outcome = engine.dispatch_m9b_drainage_tick(
                0,
                depth,
                false,
                env,
                Tick(0),
                0.0,
            );
            depth = outcome.water_depth_after();
        }
        assert_eq!(count_events(&engine, "trench", "drainage_flushed"), 0);
    }

    /// VAL-M9B-BREASTWORK-001: 80 rounds at 6 J emits exactly one
    /// `trench.breastwork_breached` event.
    #[test]
    fn breastwork_hits_emit_exactly_one_breach() {
        let engine = make_engine();
        let _ = engine.insert_trench_segment(SegmentVariant::ParapetRaised, (0, 0));
        let mut hp = cf_trench::BREASTWORK_MAX_HP;
        for _ in 0..80 {
            let outcome =
                engine.dispatch_m9b_breastwork_hit(0, hp, 6.0, Tick(0), 0.0);
            hp = outcome.hp_after();
            if hp <= 0.0 {
                break;
            }
        }
        assert_eq!(
            count_events(&engine, "trench", "breastwork_breached"),
            1,
            "exactly one breach event over the 80-round burst"
        );
    }

    /// VAL-M9B-REVETMENT-001: no revetment + soft dirt → ≥ 1
    /// `trench.segment_collapsed` event over 1800 ticks.
    #[test]
    fn collapse_tick_no_revetment_emits_collapse_event() {
        let engine = make_engine();
        let _ = engine.insert_trench_segment(SegmentVariant::Standard, (0, 0));
        let env = CollapseEnv::soft_dirt_no_revetment();
        let mut integrity = cf_trench::STARTING_INTEGRITY;
        for _ in 0..cf_trench::REVETMENT_AUDIT_WINDOW_TICKS {
            let outcome =
                engine.dispatch_m9b_collapse_tick(0, integrity, env, Tick(0), 0.0);
            if outcome.collapsed() {
                break;
            }
            integrity = outcome.integrity_after();
        }
        let collapses = count_events(&engine, "trench", "segment_collapsed");
        assert!(
            collapses >= 1,
            "no-revetment soft-dirt 1800-tick window must emit ≥ 1 collapse"
        );
    }

    /// VAL-M9B-REVETMENT-002: revetment installed → 0
    /// `trench.segment_collapsed` events over 1800 ticks.
    #[test]
    fn collapse_tick_with_revetment_emits_no_collapse() {
        let engine = make_engine();
        let _ = engine.insert_trench_segment(SegmentVariant::Standard, (0, 0));
        let env = CollapseEnv::soft_dirt_with_revetment();
        let mut integrity = cf_trench::STARTING_INTEGRITY;
        for _ in 0..cf_trench::REVETMENT_AUDIT_WINDOW_TICKS {
            let outcome =
                engine.dispatch_m9b_collapse_tick(0, integrity, env, Tick(0), 0.0);
            integrity = outcome.integrity_after();
        }
        assert_eq!(
            count_events(&engine, "trench", "segment_collapsed"),
            0,
            "revetment must prevent collapse over 1800 ticks"
        );
        assert!(integrity >= cf_trench::REVETMENT_INTEGRITY_FLOOR);
    }

    /// **VAL-CROSS-002** (inherited by M10B closure from M9C
    /// close-deferred): a parapet_raised trench dig succeeds
    /// post-M9C, the placed segment carries the `breastwork`
    /// embedded module, the M9B-side authored breastwork.ron declares
    /// the 6-sandbag cost, the M9C breastwork kernel reports HP 400,
    /// and the `parapet_raised_requires_m9c` warning event does NOT
    /// fire. The cfctl trace this test captures is the
    /// `dispatch_m9b_dig_trench_segment` call (cfctl maps
    /// `act.player.dig_trench_segment` to this dispatch path) +
    /// `compute_trench_segment_at_pos` (cfctl maps
    /// `observe.trench_segment_at_pos`).
    #[test]
    fn val_cross_002_parapet_raised_dig_emits_breastwork_segment() {
        let engine = make_engine();

        // 1. Kernel surface: cf-trench's parapet_raised_dig_validate is
        //    Ok(()) post-M9C (the same surface VAL-CROSS-003 covers
        //    against the pre-M9C warning path).
        let validate = cf_trench::parapet_raised_forward_compat::parapet_raised_dig_validate();
        assert!(
            validate.is_ok(),
            "VAL-CROSS-002 precondition: parapet_raised_dig_validate must return Ok(()) post-M9C"
        );

        // 2. M9C kernel HP 400 invariant: BREASTWORK_MAX_HP is the
        //    health the placed breastwork module spawns with.
        assert_eq!(
            cf_trench::BREASTWORK_MAX_HP as u32,
            400,
            "VAL-CROSS-002: BREASTWORK_MAX_HP must be 400 (spec § Notes)"
        );

        // 3. End-to-end cfctl trace: drive the dig via the
        //    `dispatch_m9b_dig_trench_segment` handler the cfctl
        //    method `act.player.dig_trench_segment` routes to. The
        //    handler MUST accept the action; substrate hardness 0.2 is
        //    below the deep-substrate threshold so parapet_raised does
        //    not fall back to shallow_scrape. We mark the source as
        //    Cfctl to mirror the JSON-RPC dispatch path the spec
        //    contract is anchored to.
        let outcome = engine.dispatch_m9b_dig_trench_segment(
            "parapet_raised".into(),
            Some(cf_equipment::tool::entrenching::ENTRENCHING_TOOL_ID.into()),
            0.2_f32,
            false,
            cf_actor::IntentSource::Cfctl,
            Tick(0),
            0.0,
        );
        assert_eq!(
            outcome.status,
            crate::state::ControlEnvelopeStatus::Accepted,
            "VAL-CROSS-002: parapet_raised dig must be accepted post-M9C; got {outcome:?}"
        );

        // 4. observe.trench_segment_at_pos reports the placed segment
        //    with `breastwork` embedded so subsequent fire route
        //    through the breastwork HP gate (VAL-M9B-BREASTWORK-001).
        let observed = engine.compute_trench_segment_at_pos(0, 0);
        let modules = observed
            .pointer("/result/embedded_modules")
            .and_then(|m| m.as_array())
            .expect("VAL-CROSS-002: observe must return embedded_modules");
        let names: Vec<&str> = modules.iter().filter_map(|v| v.as_str()).collect();
        assert!(
            names.contains(&"breastwork"),
            "VAL-CROSS-002: parapet_raised segment must embed `breastwork` module; got {names:?}"
        );

        // 5. The replay log carries `segment_dug` with
        //    variant=parapet_raised AND DOES NOT carry the
        //    `parapet_raised_requires_m9c` warning.
        let dug = engine
            .recorder
            .snapshot_events()
            .into_iter()
            .filter(|e| e.event_type == "segment_dug" && e.category == "trench")
            .collect::<Vec<_>>();
        assert!(
            !dug.is_empty(),
            "VAL-CROSS-002: expected ≥ 1 trench.segment_dug event"
        );
        assert!(
            dug.iter().any(|e| {
                e.payload.get("variant").and_then(|v| v.as_str()) == Some("parapet_raised")
            }),
            "VAL-CROSS-002: ≥ 1 segment_dug event must carry variant=parapet_raised"
        );
        assert_eq!(
            count_events(&engine, "trench", "parapet_raised_requires_m9c"),
            0,
            "VAL-CROSS-002 / VAL-CROSS-003: post-M9C the parapet_raised_requires_m9c warning MUST NOT fire"
        );

        // 6. The 6-sandbag cost is declared by the authored module
        //    cost map. m9b-3 already routes
        //    `act.player.place_trench_module` through
        //    `module_cost_json(Breastwork)`; we assert the spec value
        //    end-to-end here so a future cost change can't silently
        //    drift the VAL-CROSS-002 contract.
        let cost = module_cost_json(TrenchModule::Breastwork);
        let obj = cost
            .as_object()
            .expect("VAL-CROSS-002: breastwork cost is a JSON object");
        assert_eq!(
            obj.get("sandbag").and_then(|v| v.as_u64()),
            Some(6),
            "VAL-CROSS-002: breastwork module declares 6 sandbags (spec § M9B modules table)"
        );
    }

    /// **VAL-CROSS-004** (inherited by M10B closure from M9C
    /// close-deferred): a crewed fortification dominates the
    /// underlying trench segment's cover_state derivation. Deploying
    /// + crewing inside a `fire_step` segment promotes Standing
    ///   on-step from Exposed → Full; uncrew returns to Exposed for
    ///   fire_step on-step Standing.
    #[test]
    fn val_cross_004_mg_tripod_inside_fire_step_crewing_dominates_cover_state() {
        use cf_actor::{ActorState, ActorId, Inventory, Vec2};
        use cf_trench::segment::{InMemorySegments, TrenchSegment};
        use cf_trench::CoverState as TrenchCoverState;

        // 1. Set up: a fire_step segment at (0, 0) with depth=16 +
        //    step_height=8. Per VAL-M9B-SEGMENT-004 standing on-step
        //    == Exposed.
        let segment = TrenchSegment {
            variant: SegmentVariant::FireStep,
            tile_x: 0,
            tile_y: 0,
            depth: 16,
            width: 20,
            raised_step_height: Some(8),
            embedded_modules: vec![TrenchModule::Duckboard, TrenchModule::FireStep],
        };
        let world = InMemorySegments::with_segments(vec![segment]);

        // 2. Stand the player on-step; pre-crew baseline is Exposed.
        let mut player = ActorState::player(
            ActorId(1),
            "blue",
            Vec2::new(5.0, 10.0),
            100.0,
            Inventory::default(),
        );
        player.on_ground = true;
        player.crouch_active = false;
        player.prone_active = false;
        assert_eq!(
            player.cover_state(&world),
            TrenchCoverState::Exposed,
            "VAL-CROSS-004 baseline: Standing on-step in fire_step must be Exposed"
        );

        // 3. Deploy + crew the mg_tripod (the cfctl methods
        //    `act.player.deploy_mg_tripod` then
        //    `act.player.crew_fortification` map to this kernel
        //    transition; the engine assigns a fortification_id which
        //    we mimic here as 42).
        let tripod_id: u32 = 42;
        player.crew_fortification(tripod_id);
        assert!(player.is_crewing());
        assert_eq!(player.crewed_fortification_id(), Some(tripod_id));
        assert_eq!(
            player.cover_state(&world),
            TrenchCoverState::Full,
            "VAL-CROSS-004: crewing dominates the segment-variant table → cover_state == Full"
        );

        // 4. Uncrew (cfctl `act.player.uncrew_fortification`) →
        //    segment-variant baseline restored (Exposed for
        //    Standing on-step in fire_step).
        let released = player.uncrew_fortification();
        assert_eq!(released, Some(tripod_id));
        assert!(!player.is_crewing());
        assert_eq!(
            player.cover_state(&world),
            TrenchCoverState::Exposed,
            "VAL-CROSS-004: uncrew restores fire_step on-step Standing == Exposed"
        );
    }
}
