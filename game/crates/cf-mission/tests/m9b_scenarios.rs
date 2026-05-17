//! **VAL-M9B-SCENARIOS-001**: integration tests that every registered
//! M9B launch scenario exists under `game/content/scenarios/` AND each
//! file's RON `id` field matches the registry. This is the canonical
//! "8 scenarios register" gate the closure feature audits.
//!
//! The tests deserialize a minimal `{ id }` shape so cf-mission can
//! validate registration without depending on cf-control's full
//! `Scenario` struct (which would create a workspace dep cycle).

use std::path::PathBuf;

use serde::Deserialize;

use cf_mission::m9b_scenarios::{registry, scenario_path, tick_budget_for, SCENARIO_IDS};

/// Minimal shape that the cf-mission integration test reads to confirm
/// the on-disk RON `id` field matches the registry entry.
#[derive(Debug, Deserialize)]
struct ScenarioIdProbe {
    id: String,
    duration_ticks: Option<u64>,
    seed: u64,
}

fn game_root() -> PathBuf {
    // CARGO_MANIFEST_DIR -> crates/cf-mission -> crates -> game.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .expect("game root walking up from CARGO_MANIFEST_DIR")
        .to_path_buf()
}

fn load_probe(id: &str) -> ScenarioIdProbe {
    let rel = scenario_path(id).unwrap_or_else(|| panic!("registry missing path for `{id}`"));
    let path = game_root().join(&rel);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    ron::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

#[test]
fn all_eight_scenarios_register() {
    assert_eq!(SCENARIO_IDS.len(), 8, "VAL-M9B-SCENARIOS-001: registry has 8 ids");
    let pairs = registry();
    assert_eq!(pairs.len(), 8, "registry() returns 8 (id, tick_budget) pairs");
    for &id in SCENARIO_IDS {
        let probe = load_probe(id);
        assert_eq!(probe.id, id, "scenario file id field matches registry entry");
        let budget = tick_budget_for(id).expect("registry has tick budget");
        if let Some(declared) = probe.duration_ticks {
            assert!(
                declared >= budget,
                "scenario `{id}` declares duration_ticks={declared} but registry says \
                 tick_budget={budget}; scenario must run at least to its registry budget"
            );
        }
    }
}

#[test]
fn each_scenario_file_exists() {
    for &id in SCENARIO_IDS {
        let rel = scenario_path(id).unwrap();
        let path = game_root().join(&rel);
        assert!(path.exists(), "M9B scenario file missing: {}", path.display());
        let bytes = std::fs::metadata(&path).expect("stat").len();
        assert!(bytes > 0, "M9B scenario file is empty: {}", path.display());
    }
}

#[test]
fn determinism_scenario_has_fixed_seed_42() {
    // VAL-M9B-DETERMINISM-001 anchor: m9b_reactor_defense_zigzag MUST
    // ship with seed=42 because the cross-engine determinism contract is
    // stated literally as "two engines with world_seed=42 over 3600
    // ticks produce identical event sequence".
    let probe = load_probe("m9b_reactor_defense_zigzag");
    assert_eq!(
        probe.seed, 42,
        "VAL-M9B-DETERMINISM-001: m9b_reactor_defense_zigzag seed must be 42"
    );
    assert_eq!(
        probe.duration_ticks,
        Some(3600),
        "VAL-M9B-DETERMINISM-001: tick budget must be 3600"
    );
}

#[test]
fn each_scenario_declares_distinct_id() {
    let mut ids = SCENARIO_IDS.iter().copied().collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), SCENARIO_IDS.len(), "registry ids are distinct");
}
