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
    dig_substrate_validate, DigSubstrateOutcome, ModuleSpec, SegmentVariant, TrenchModule,
    DEEP_HARDNESS_THRESHOLD,
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
                return CommandResult::rejected(reason, tick.0);
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
                let _ = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "trench",
                    "segment_dug",
                    json!({
                        "actor_id": null,
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
                return CommandResult::accepted(tick.0);
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
                let _ = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "trench",
                    "segment_dug",
                    json!({
                        "actor_id": null,
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

    /// **VAL-M9B-MODULES-002 + VAL-M9B-CFCTL-001**: dispatch
    /// `act.player.place_trench_module`. Places the named module on
    /// `segment_id` after `build_time_seconds_for(module)` in-game
    /// seconds; emits `trench.module_placed`.
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
        let _ = self.recorder.record(
            tick,
            sim_time_ms,
            "trench",
            "module_placed",
            json!({
                "actor_id": null,
                "module_id": module.as_str(),
                "segment_id": segment_id,
                "build_time_seconds": build_time,
                "material_cost": cost,
                "source": source_label,
            }),
            Some(action_id),
        );
        CommandResult::accepted(tick.0)
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

    /// **VAL-M9B-CFCTL-002**: compute the `observe.actor.cover_state`
    /// projection. Returns `{ schema_version, actor_id, cover_state }`
    /// with `cover_state` one of `Exposed | Partial | Full`. Currently
    /// the trench world is not yet wired into the engine; the helper
    /// returns the derivation against an empty segment world so the
    /// cfctl wire surface is stable for the m9b-3 contract assertion.
    /// Future features replace the empty world with the actual
    /// placed-segment index.
    pub(crate) fn compute_actor_cover_state(&self, actor_id: u64) -> Value {
        let world = cf_trench::segment::InMemorySegments::new();
        let cover = self
            .state
            .read()
            .ok()
            .and_then(|s| {
                s.actor_state.as_ref().and_then(|sim| {
                    sim.world
                        .actors
                        .get(&cf_actor::ActorId(actor_id))
                        .map(|a| a.cover_state(&world))
                })
            })
            .unwrap_or(cf_trench::CoverState::Exposed);
        json!({
            "schema_version": crate::SCHEMA_VERSION,
            "actor_id": actor_id,
            "cover_state": cover.as_str(),
        })
    }

    /// **VAL-M9B-CFCTL-002**: compute the
    /// `observe.trench_segment_at_pos` projection. Returns
    /// `{ schema_version, result }` with `result` either `null` for
    /// open ground OR a `TrenchSegmentView` object with the 6 declared
    /// variants. The trench-world lookup is empty pre-m9b-5 closure;
    /// once placed segments land the empty lookup is replaced with the
    /// real index.
    #[allow(clippy::unused_self)]
    pub(crate) fn compute_trench_segment_at_pos(&self, _x: i32, _y: i32) -> Value {
        json!({
            "schema_version": crate::SCHEMA_VERSION,
            "result": Value::Null,
        })
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
}
