//! **VAL-M9C-008**: integration tests that every registered M9C
//! launch scenario exists under `game/content/scenarios/` AND each
//! file's RON `id` field matches the registry. The closure-feature
//! worker audits this as the canonical "10 scenarios register" gate.
//!
//! The tests deserialize a minimal `{ id, duration_ticks, seed }`
//! shape so cf-mission can validate registration without depending on
//! cf-control's full `Scenario` struct (avoiding a workspace dep
//! cycle).

use std::path::PathBuf;

use serde::Deserialize;

use cf_mission::m9c_scenarios::{registry, scenario_path, tick_budget_for, SCENARIO_IDS};

#[derive(Debug, Deserialize)]
struct ScenarioIdProbe {
    id: String,
    duration_ticks: Option<u64>,
    seed: u64,
    #[serde(default)]
    notes: String,
    #[serde(default)]
    scenario_tags: Vec<String>,
}

fn game_root() -> PathBuf {
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

/// **VAL-M9C-008**: all 10 scenarios register with file presence +
/// id-field match.
#[test]
fn all_ten_scenarios_register() {
    assert_eq!(SCENARIO_IDS.len(), 10, "VAL-M9C-008: registry has 10 ids");
    let pairs = registry();
    assert_eq!(pairs.len(), 10, "registry() returns 10 (id, tick_budget) pairs");
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
        assert!(path.exists(), "M9C scenario file missing: {}", path.display());
        let bytes = std::fs::metadata(&path).expect("stat").len();
        assert!(bytes > 0, "M9C scenario file is empty: {}", path.display());
    }
}

/// **VAL-M9C-050 / VAL-CROSS-006**: m9c_full_strongpoint anchors
/// the cross-engine determinism gate at seed=42 + 3600 ticks.
#[test]
fn full_strongpoint_seed_42_for_determinism() {
    let probe = load_probe("m9c_full_strongpoint");
    assert_eq!(
        probe.seed, 42,
        "VAL-M9C-050: m9c_full_strongpoint seed must be 42"
    );
    assert_eq!(
        probe.duration_ticks,
        Some(3600),
        "VAL-M9C-050: tick budget must be 3600"
    );
}

/// **VAL-CROSS-005**: m9c_full_strongpoint references M9B trench
/// variant ids in its RON content (via notes / scenario_tags pointing
/// at the `forward_outpost_with_mgnest` template).
#[test]
fn full_strongpoint_references_m9b_trench_variants() {
    let probe = load_probe("m9c_full_strongpoint");
    let m9b_variants = [
        "shallow_scrape",
        "standard",
        "deep",
        "communication",
        "fire_step",
        "parapet_raised",
    ];
    let mention_count = m9b_variants
        .iter()
        .filter(|v| probe.notes.contains(*v))
        .count();
    assert!(
        mention_count >= 1,
        "VAL-CROSS-005: m9c_full_strongpoint must reference ≥1 M9B trench variant in notes; \
         counted {mention_count} matches"
    );
    assert!(
        probe
            .scenario_tags
            .iter()
            .any(|t| t.contains("m9b") || t.contains("trench") || t.contains("strongpoint")),
        "VAL-CROSS-005: m9c_full_strongpoint scenario_tags must mark the M9B trench bridge"
    );
}

#[test]
fn each_scenario_declares_distinct_id() {
    let mut ids = SCENARIO_IDS.to_vec();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), SCENARIO_IDS.len(), "registry ids are distinct");
}
