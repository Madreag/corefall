//! **M9C**: scenario registry for the 10 launch fortification
//! scenarios.
//!
//! Per `specs/active/M9C.md` § "Files" the milestone ships 10
//! scenarios under `game/content/scenarios/m9c_*.ron` that exercise
//! the MG nest suite, sandbag erosion, watchtower + spotter, minefield
//! sweep + IED chain, wire breach + electrified fence depower,
//! anti-tank layered defense, camo netting concealment, and the full
//! strongpoint determinism gate.
//!
//! The module is the canonical Rust-side registry so cf-control,
//! cf-headless, cf-mod, and the M9C closure-feature worker can
//! enumerate the launch roster without scanning the filesystem.
//!
//! The closure feature `m9c-6-cfctl-events-scenarios-audit-close`
//! drives every scenario via the integration-test pattern (see
//! `cf-control/tests/m9b_scenario_acceptance.rs` for the equivalent
//! M9B harness) — the project's `cf-headless` binary does NOT expose
//! a top-level `--scenario` flag (see mission AGENTS.md "Known
//! Pre-Existing Issues").
//!
//! VAL-M9C-008 lands here.

use std::path::PathBuf;

/// The 10 launch scenario ids registered by M9C. ORDER MATCHES the
/// spec's "Files" section so a grep against the spec verifies
/// registration without per-id assertions.
pub const SCENARIO_IDS: &[&str] = &[
    "m9c_mg_nest_crewed_defense",
    "m9c_sandbag_erosion",
    "m9c_watchtower_spotter_chain",
    "m9c_minefield_clearance_drill",
    "m9c_ied_chain_killzone",
    "m9c_wire_breach_assault",
    "m9c_electrified_fence_depower",
    "m9c_anti_tank_layered_defense",
    "m9c_camo_netting_concealment",
    "m9c_full_strongpoint",
];

/// Per-scenario tick budget declared in `specs/active/M9C.md`
/// Acceptance Criteria. Used by closure-feature verification +
/// integration tests that drive each scenario via cf-control's
/// `run_m0_inline` (the project's pattern for headless scenario
/// runs).
///
/// `m9c_full_strongpoint` ships a 3600-tick budget to honour
/// VAL-M9C-050 (cross-engine determinism over the 60s window) and
/// VAL-CROSS-006 (cross-spec determinism over the same window).
#[must_use]
#[allow(clippy::match_same_arms)]
pub fn tick_budget_for(id: &str) -> Option<u64> {
    match id {
        "m9c_mg_nest_crewed_defense" => Some(600),
        "m9c_sandbag_erosion" => Some(1200),
        "m9c_watchtower_spotter_chain" => Some(600),
        "m9c_minefield_clearance_drill" => Some(600),
        "m9c_ied_chain_killzone" => Some(600),
        "m9c_wire_breach_assault" => Some(600),
        "m9c_electrified_fence_depower" => Some(600),
        "m9c_anti_tank_layered_defense" => Some(1200),
        "m9c_camo_netting_concealment" => Some(600),
        "m9c_full_strongpoint" => Some(3600),
        _ => None,
    }
}

/// Resolve the on-disk RON file path for a scenario id, relative to
/// the workspace root (`game/`). Returns `None` for unknown ids.
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
/// `run_m0_inline` for each scenario without re-deriving the budget
/// separately.
#[must_use]
pub fn registry() -> Vec<(&'static str, u64)> {
    SCENARIO_IDS
        .iter()
        .map(|&id| (id, tick_budget_for(id).unwrap_or(600)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **VAL-M9C-008**: the registry contains exactly 10 ids,
    /// matching the spec's launch roster.
    #[test]
    fn m9c_scenarios_register() {
        assert_eq!(
            SCENARIO_IDS.len(),
            10,
            "M9C registry must hold exactly 10 launch scenarios"
        );
        assert_eq!(registry().len(), 10);
        let expected = [
            "m9c_mg_nest_crewed_defense",
            "m9c_sandbag_erosion",
            "m9c_watchtower_spotter_chain",
            "m9c_minefield_clearance_drill",
            "m9c_ied_chain_killzone",
            "m9c_wire_breach_assault",
            "m9c_electrified_fence_depower",
            "m9c_anti_tank_layered_defense",
            "m9c_camo_netting_concealment",
            "m9c_full_strongpoint",
        ];
        for id in expected {
            assert!(
                SCENARIO_IDS.contains(&id),
                "M9C registry must declare scenario id `{id}`"
            );
        }
    }

    /// **VAL-M9C-050 / VAL-CROSS-006**: `m9c_full_strongpoint` ships
    /// with a 3600-tick budget so the cross-engine determinism gate
    /// covers the spec-declared 60s window.
    #[test]
    fn full_strongpoint_runs_for_3600_ticks() {
        assert_eq!(
            tick_budget_for("m9c_full_strongpoint"),
            Some(3600),
            "VAL-M9C-050 contract: full_strongpoint tick budget is 3600"
        );
    }

    #[test]
    fn tick_budget_for_unknown_id_is_none() {
        assert_eq!(tick_budget_for("not_a_scenario"), None);
    }

    #[test]
    fn scenario_path_known_id_is_under_content_scenarios() {
        let path = scenario_path("m9c_full_strongpoint").expect("known id resolves");
        assert_eq!(
            path,
            PathBuf::from("content/scenarios/m9c_full_strongpoint.ron")
        );
    }

    #[test]
    fn scenario_path_unknown_id_is_none() {
        assert!(scenario_path("not_a_scenario").is_none());
    }
}
