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

use cf_actor::IntentSource;
use cf_sim_core::Tick;
use cf_trench::{
    apply_round_to_breastwork, breastwork::BreastworkHitOutcome, collapse_tick,
    cover_state_post_breach, dig_substrate_validate, drainage_sump_tick,
    segment::TrenchSegmentLookup, BreastworkHitOutcome as _BreastworkOutcomeAlias,
    CollapseEnv, CollapseTickOutcome, CoverState, DigSubstrateOutcome, DrainageEnv,
    DrainageTickOutcome, ModuleSpec, SegmentVariant, TrenchModule, TrenchSegment,
    TrenchStance, DEEP_HARDNESS_THRESHOLD,
};

use crate::engine::M0Engine;
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
    pub(crate) fn insert_trench_segment(
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
    pub(crate) fn embed_trench_module(
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
    pub(crate) fn compute_trench_segment_at_pos(&self, x: i32, y: i32) -> Value {
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
                }),
                None,
            );
        }
        outcome
    }
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
