//! **M9B**: scenario registry for the 8 launch trench scenarios.
//!
//! Per `specs/active/M9B.md` § "Files" + § "8 launch trench scenarios"
//! the launch milestone ships 8 scenarios under
//! `game/content/scenarios/m9b_*.ron`. This module is the canonical
//! Rust-side registry that lets downstream crates (cf-control,
//! cf-headless, cf-mod) enumerate them without scanning the filesystem.
//!
//! The registry is a `const &[ScenarioId]` plus a couple of typed
//! helpers:
//!
//! - [`SCENARIO_IDS`] — the 8 string ids the scenario RON files declare.
//! - [`scenario_path`] — resolve the on-disk RON file path relative to
//!   the workspace root (`game/`).
//! - [`tick_budget_for`] — the declared per-scenario tick budget that
//!   `cf-headless run --scenario <id>.ron --max-ticks <budget>` is
//!   expected to honour. Lower-bound budgets come from the spec's
//!   acceptance scenarios (e.g. VAL-M9B-DETERMINISM-001 requires
//!   m9b_reactor_defense_zigzag to be deterministic at tick 3600).
//!
//! Tests live in `cf-mission/tests/m9b_scenarios.rs` (integration
//! tests so they have read access to `game/content/scenarios/`).

use std::path::PathBuf;

/// The 8 launch scenario ids registered by M9B. ORDER MATCHES the spec's
/// "Files" section so a grep against the spec verifies registration
/// without per-id assertions.
pub const SCENARIO_IDS: &[&str] = &[
    "m9b_zigzag_baseline",
    "m9b_two_line_defense",
    "m9b_fire_step_duel",
    "m9b_drainage_flood",
    "m9b_reactor_defense_zigzag",
    "m9b_template_drop_test",
    "m9b_ai_in_trench_doctrine",
    "m9b_breastwork_breach",
];

/// Per-scenario tick budget declared in `specs/active/M9B.md` Acceptance
/// Criteria. Used by closure-feature verification + integration tests
/// that drive each scenario via `cf-headless run --scenario ...
/// --max-ticks <budget>`.
#[must_use]
pub fn tick_budget_for(id: &str) -> Option<u64> {
    match id {
        "m9b_zigzag_baseline" => Some(600),
        "m9b_two_line_defense" => Some(600),
        "m9b_fire_step_duel" => Some(600),
        "m9b_drainage_flood" => Some(1800),
        "m9b_reactor_defense_zigzag" => Some(3600),
        "m9b_template_drop_test" => Some(600),
        "m9b_ai_in_trench_doctrine" => Some(1200),
        "m9b_breastwork_breach" => Some(1800),
        _ => None,
    }
}

/// Resolve the on-disk RON file path for a scenario id, relative to the
/// workspace root (`game/`). Returns `None` for unknown ids.
#[must_use]
pub fn scenario_path(id: &str) -> Option<PathBuf> {
    if SCENARIO_IDS.contains(&id) {
        Some(PathBuf::from(format!("content/scenarios/{id}.ron")))
    } else {
        None
    }
}

/// Convenience: the registry as `(id, tick_budget)` pairs in declared
/// order. Used by closure-feature verification to drive
/// `cf-headless run --scenario <id>.ron --max-ticks <budget>` for each
/// scenario without re-deriving the budget separately.
#[must_use]
pub fn registry() -> Vec<(&'static str, u64)> {
    SCENARIO_IDS
        .iter()
        .map(|&id| (id, tick_budget_for(id).unwrap_or(600)))
        .collect()
}

/// **VAL-M9B-SCENARIOS-001**: the registry contains exactly 8 ids,
/// matching the spec's launch roster.
///
/// **VAL-M9B-DETERMINISM-001**: `m9b_reactor_defense_zigzag` ships with
/// a 3600-tick budget so the cross-engine determinism check can run
/// the spec-declared 60s window.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_eight_scenarios() {
        assert_eq!(SCENARIO_IDS.len(), 8, "M9B registry must hold exactly 8 launch scenarios");
        assert_eq!(registry().len(), 8);
    }

    #[test]
    fn registry_lists_spec_declared_ids() {
        let expected = [
            "m9b_zigzag_baseline",
            "m9b_two_line_defense",
            "m9b_fire_step_duel",
            "m9b_drainage_flood",
            "m9b_reactor_defense_zigzag",
            "m9b_template_drop_test",
            "m9b_ai_in_trench_doctrine",
            "m9b_breastwork_breach",
        ];
        for id in expected {
            assert!(
                SCENARIO_IDS.contains(&id),
                "M9B registry must declare scenario id `{id}`"
            );
        }
    }

    #[test]
    fn determinism_scenario_runs_for_3600_ticks() {
        assert_eq!(
            tick_budget_for("m9b_reactor_defense_zigzag"),
            Some(3600),
            "VAL-M9B-DETERMINISM-001 contract: reactor_defense_zigzag tick budget is 3600"
        );
    }

    #[test]
    fn tick_budget_for_unknown_id_is_none() {
        assert_eq!(tick_budget_for("not_a_scenario"), None);
    }

    #[test]
    fn scenario_path_known_id_is_under_content_scenarios() {
        let path = scenario_path("m9b_zigzag_baseline").expect("known id resolves");
        assert_eq!(path, PathBuf::from("content/scenarios/m9b_zigzag_baseline.ron"));
    }

    #[test]
    fn scenario_path_unknown_id_is_none() {
        assert!(scenario_path("not_a_scenario").is_none());
    }
}
