//! **M9C closure feature** — integration tests that load each of the
//! 10 `m9c_*` launch scenarios via [`cf_control::scenario::Scenario::load_from_file`]
//! and drive the headless engine via [`cf_control::engine::run_m0_inline`]
//! for the scenario's declared tick budget.
//!
//! The tests satisfy:
//!
//! - **VAL-M9C-008** — every scenario file parses through the full
//!   cf-control Scenario validator + matches the cf-mission registry.
//! - **VAL-M9C-004 / VAL-M9C-050 (smoke evidence)** — every scenario
//!   reaches its declared smoke tick budget (300) without panic.
//! - **VAL-CROSS-006** — `m9c_full_strongpoint` produces the SAME
//!   `final_checksum_hex` across two run_m0_inline invocations with
//!   seed=42 over a short determinism window (240 ticks); the full
//!   3600-tick gate is `#[ignore]`d per the same pattern M9B uses
//!   for the reactor-defense scenario.

use std::path::{Path, PathBuf};

use cf_control::{
    engine::{run_m0_inline, M0EngineConfig},
    runtime::{build_engine_config, ConfigInputs},
    scenario::Scenario,
    settings::Settings,
};
use cf_mission::m9c_scenarios::{registry, tick_budget_for, SCENARIO_IDS};
use cf_replay::resolve_run_bundle_root;
use tempfile::tempdir;

fn game_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .expect("game root walking up from CARGO_MANIFEST_DIR")
        .to_path_buf()
}

fn scenario_full_path(id: &str) -> PathBuf {
    game_root().join(format!("content/scenarios/{id}.ron"))
}

fn build_run_config(
    scenario_path: &Path,
    scenario_id: &str,
    ticks: u64,
    seed_override: Option<u64>,
    bundle_root: PathBuf,
) -> M0EngineConfig {
    let inputs = ConfigInputs {
        scenario_id: scenario_id.to_string(),
        scenario_path: scenario_path.to_path_buf(),
        run_mode: format!("m9c-closure-{scenario_id}"),
        run_bundle_root: resolve_run_bundle_root(Some(bundle_root)),
        write_run_bundle: true,
        control_api_enabled: false,
        debug_capabilities: Vec::new(),
        tick_rate_hz: 60,
        capture_grid_enabled: false,
        paced: false,
        settings: Settings::default(),
        seed_override,
        duration_ticks_override: Some(ticks),
        debug_inject_panic_at_tick: None,
        checksum_cadence_ticks: None,
        expected_outcome: None,
    };
    build_engine_config(inputs).expect("build_engine_config")
}

/// **VAL-M9C-008** — every M9C scenario file parses through the
/// full cf-control Scenario validator.
#[test]
fn every_m9c_scenario_parses_through_full_validator() {
    for &id in SCENARIO_IDS {
        let path = scenario_full_path(id);
        let scenario = Scenario::load_from_file(&path).unwrap_or_else(|e| {
            panic!(
                "VAL-M9C-008: cf-control Scenario::load_from_file rejected {}: {e}",
                path.display()
            )
        });
        assert_eq!(scenario.id, id, "loaded scenario id matches file basename");
    }
}

/// **VAL-M9C-004 / VAL-M9C-050 (smoke)** — each scenario runs without
/// panic for a small tick budget. Full-budget gate is the `#[ignore]`d
/// `each_m9c_scenario_runs_to_declared_tick_budget_without_panic`
/// below; the closure feature runs that one with `--release --ignored`.
#[test]
fn every_m9c_scenario_runs_smoke_without_panic() {
    const SMOKE_TICKS: u64 = 300;
    for (id, _budget) in registry() {
        let path = scenario_full_path(id);
        let bundle_root = tempdir().expect("tempdir");
        let config =
            build_run_config(&path, id, SMOKE_TICKS, None, bundle_root.path().to_path_buf());
        let outcome = run_m0_inline(config).unwrap_or_else(|e| {
            panic!("VAL-M9C-004 (smoke): scenario `{id}` panicked / errored: {e:?}")
        });
        assert!(
            outcome.ticks_run >= SMOKE_TICKS,
            "scenario `{id}` only advanced {} ticks (smoke budget {SMOKE_TICKS})",
            outcome.ticks_run
        );
        let bundle_dir = outcome
            .bundle_dir
            .as_ref()
            .expect("bundle written")
            .clone();
        let events_path = bundle_dir.join("events.jsonl");
        let bytes = std::fs::metadata(&events_path)
            .unwrap_or_else(|e| panic!("stat {}: {e}", events_path.display()))
            .len();
        assert!(
            bytes > 0,
            "scenario `{id}` replay (events.jsonl) must be non-empty"
        );
    }
}

/// Full-budget gate. Marked `#[ignore]` so it's opt-in via
/// `cargo test -- --ignored` because the cumulative tick count
/// (≥ 10800 across 10 scenarios; 3600 alone for the full strongpoint)
/// makes a debug-mode run multi-minute. The closure feature runs this
/// with `--release` during verification.
#[test]
#[ignore]
fn each_m9c_scenario_runs_to_declared_tick_budget_without_panic() {
    for (id, budget) in registry() {
        let path = scenario_full_path(id);
        let bundle_root = tempdir().expect("tempdir");
        let config = build_run_config(&path, id, budget, None, bundle_root.path().to_path_buf());
        let outcome = run_m0_inline(config).unwrap_or_else(|e| {
            panic!("VAL-M9C-004 (full): scenario `{id}` panicked / errored: {e:?}")
        });
        assert!(
            outcome.ticks_run >= budget,
            "scenario `{id}` only advanced {} ticks (budget {budget})",
            outcome.ticks_run
        );
    }
}

/// **VAL-M9C-050 / VAL-CROSS-006**: m9c_full_strongpoint is
/// deterministic across two engines with seed=42 over a short window
/// (drift would manifest within hundreds of ticks). The full
/// 3600-tick parity gate is `#[ignore]`d (`--ignored`) to match the
/// M9B reactor-defense pattern.
#[test]
fn full_strongpoint_is_deterministic_seed_42_short_window() {
    let id = "m9c_full_strongpoint";
    let path = scenario_full_path(id);
    let short_window: u64 = 240;

    let bundle_a = tempdir().expect("tempdir A");
    let bundle_b = tempdir().expect("tempdir B");
    let config_a = build_run_config(
        &path,
        id,
        short_window,
        Some(42),
        bundle_a.path().to_path_buf(),
    );
    let config_b = build_run_config(
        &path,
        id,
        short_window,
        Some(42),
        bundle_b.path().to_path_buf(),
    );

    let outcome_a = run_m0_inline(config_a).expect("engine A run_m0_inline");
    let outcome_b = run_m0_inline(config_b).expect("engine B run_m0_inline");

    let checksum_a = outcome_a
        .final_checksum_hex
        .as_ref()
        .expect("engine A must record a final checksum");
    let checksum_b = outcome_b
        .final_checksum_hex
        .as_ref()
        .expect("engine B must record a final checksum");
    assert_eq!(
        checksum_a, checksum_b,
        "VAL-M9C-050: seed=42 must produce identical final_checksum_hex \
         across two engines (engine A={checksum_a}, engine B={checksum_b})"
    );
}

/// **VAL-M9C-050 / VAL-CROSS-006** literal evidence (3600 ticks).
/// `#[ignore]`d per the M9B reactor-defense pattern.
#[test]
#[ignore]
fn full_strongpoint_is_deterministic_seed_42_full_3600_ticks() {
    let id = "m9c_full_strongpoint";
    let budget = tick_budget_for(id).expect("registry has tick budget");
    let path = scenario_full_path(id);

    let bundle_a = tempdir().expect("tempdir A");
    let bundle_b = tempdir().expect("tempdir B");
    let config_a = build_run_config(&path, id, budget, Some(42), bundle_a.path().to_path_buf());
    let config_b = build_run_config(&path, id, budget, Some(42), bundle_b.path().to_path_buf());

    let outcome_a = run_m0_inline(config_a).expect("engine A run_m0_inline");
    let outcome_b = run_m0_inline(config_b).expect("engine B run_m0_inline");

    let checksum_a = outcome_a
        .final_checksum_hex
        .as_ref()
        .expect("engine A must record a final checksum");
    let checksum_b = outcome_b
        .final_checksum_hex
        .as_ref()
        .expect("engine B must record a final checksum");
    assert_eq!(checksum_a, checksum_b);
}
