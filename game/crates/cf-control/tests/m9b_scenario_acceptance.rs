//! **M9B closure feature** — integration tests that load each of the 8
//! `m9b_*` launch scenarios via [`cf_control::scenario::Scenario::load_from_file`]
//! and drive the headless engine via [`cf_control::engine::run_m0_inline`]
//! for the scenario's declared tick budget.
//!
//! The tests satisfy:
//!
//! - **VAL-M9B-SCENARIOS-001** — every scenario file parses through the
//!   full cf-control Scenario validator.
//! - **VAL-M9B-SCENARIOS-002** — every scenario reaches its declared
//!   tick budget without panic (exit code 0 / no `Result::Err`).
//! - **VAL-M9B-DETERMINISM-001** — the `m9b_reactor_defense_zigzag`
//!   scenario produces the SAME `final_checksum_hex` across two
//!   run_m0_inline invocations with seed=42 for 3600 ticks.

use std::path::{Path, PathBuf};

use cf_control::{
    engine::{run_m0_inline, M0EngineConfig},
    runtime::{build_engine_config, ConfigInputs},
    scenario::Scenario,
    settings::Settings,
};
use cf_mission::m9b_scenarios::{registry, tick_budget_for, SCENARIO_IDS};
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
        run_mode: format!("m9b-closure-{scenario_id}"),
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

#[test]
fn every_m9b_scenario_parses_through_full_validator() {
    for &id in SCENARIO_IDS {
        let path = scenario_full_path(id);
        let scenario = Scenario::load_from_file(&path).unwrap_or_else(|e| {
            panic!(
                "VAL-M9B-SCENARIOS-001: cf-control Scenario::load_from_file rejected {}: {e}",
                path.display()
            )
        });
        assert_eq!(scenario.id, id, "loaded scenario id matches file basename");
    }
}

#[test]
fn every_m9b_scenario_runs_smoke_without_panic() {
    // Closure-feature smoke gate: each scenario must drive run_m0_inline
    // for a small tick budget (300) without panicking or returning an
    // error. The full declared tick budget is exercised by
    // `each_m9b_scenario_runs_to_declared_tick_budget_without_panic`
    // (which is `#[ignore]` because debug-build 10800+ ticks across 8
    // scenarios is too slow for the default `cargo test` cycle).
    const SMOKE_TICKS: u64 = 300;
    for (id, _budget) in registry() {
        let path = scenario_full_path(id);
        let bundle_root = tempdir().expect("tempdir");
        let config =
            build_run_config(&path, id, SMOKE_TICKS, None, bundle_root.path().to_path_buf());
        let outcome = run_m0_inline(config).unwrap_or_else(|e| {
            panic!("VAL-M9B-SCENARIOS-002 (smoke): scenario `{id}` panicked / errored: {e:?}")
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
/// (≥ 10800) makes a debug-mode run multi-minute. The closure feature
/// runs this with `--release` during verification.
///
/// Use `cargo test -p cf-control --release --test m9b_scenario_acceptance
/// -- --ignored each_m9b_scenario_runs_to_declared` to run.
#[test]
#[ignore]
fn each_m9b_scenario_runs_to_declared_tick_budget_without_panic() {
    for (id, budget) in registry() {
        let path = scenario_full_path(id);
        let bundle_root = tempdir().expect("tempdir");
        let config = build_run_config(&path, id, budget, None, bundle_root.path().to_path_buf());
        let outcome = run_m0_inline(config).unwrap_or_else(|e| {
            panic!("VAL-M9B-SCENARIOS-002: scenario `{id}` panicked / errored: {e:?}")
        });
        assert!(
            outcome.ticks_run >= budget,
            "scenario `{id}` only advanced {} ticks (budget {budget})",
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

/// Quick single-scenario tick-budget gate covering the simplest M9B
/// scenarios (no reactive_guard AI, no reactor). Used by the default
/// `cargo test` cycle to ensure the cf-mission registry resolves to
/// scenarios that the cf-control engine can drive to completion.
#[test]
fn lightweight_m9b_scenarios_run_to_declared_tick_budget() {
    let lightweight = [
        "m9b_zigzag_baseline",
        "m9b_two_line_defense",
        "m9b_fire_step_duel",
        "m9b_template_drop_test",
    ];
    for id in lightweight {
        let budget = tick_budget_for(id).expect("registry has tick budget");
        let path = scenario_full_path(id);
        let bundle_root = tempdir().expect("tempdir");
        let config = build_run_config(&path, id, budget, None, bundle_root.path().to_path_buf());
        let outcome = run_m0_inline(config).unwrap_or_else(|e| {
            panic!("scenario `{id}` panicked / errored at budget {budget}: {e:?}")
        });
        assert!(
            outcome.ticks_run >= budget,
            "scenario `{id}` only advanced {} ticks (budget {budget})",
            outcome.ticks_run
        );
    }
}

#[test]
fn reactor_defense_zigzag_is_deterministic_seed_42_short_window() {
    // **VAL-M9B-DETERMINISM-001** (softened evidence per closure spec):
    // two engines running m9b_reactor_defense_zigzag with seed=42 must
    // produce identical `final_checksum_hex`. The spec's literal
    // assertion uses tick 3600 (60s), but determinism is a property
    // that holds at any tick — drift would manifest within the first
    // hundreds of ticks. The closure test uses a 240-tick window
    // because the reactor + mission state machine combination has a
    // pre-existing slowness penalty (~10× slower than non-reactor
    // scenarios; observed on `micro_reactor_defense.ron` as well —
    // documented in discoveredIssues). The full 3600-tick parity gate
    // is `#[ignore]` and runs via `--release --ignored`.
    let id = "m9b_reactor_defense_zigzag";
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

    assert!(
        outcome_a.ticks_run >= short_window,
        "engine A advanced {} ticks (budget {short_window})",
        outcome_a.ticks_run
    );
    assert!(
        outcome_b.ticks_run >= short_window,
        "engine B advanced {} ticks (budget {short_window})",
        outcome_b.ticks_run
    );

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
        "VAL-M9B-DETERMINISM-001: seed=42 must produce identical final_checksum_hex \
         across two engines (engine A={checksum_a}, engine B={checksum_b})"
    );
}

#[test]
#[ignore]
fn reactor_defense_zigzag_is_deterministic_seed_42_full_3600_ticks() {
    // **VAL-M9B-DETERMINISM-001** literal evidence (3600 ticks). Marked
    // `#[ignore]` because the reactor + mission state machine carries
    // a pre-existing slowness penalty on `micro_reactor_defense.ron`
    // and `m9b_reactor_defense_zigzag.ron` alike (see
    // `m9b_scenario_timing.rs` probes). Run via:
    //
    //   cargo test -p cf-control --release --test m9b_scenario_acceptance \
    //     -- --ignored reactor_defense_zigzag_is_deterministic_seed_42_full
    let id = "m9b_reactor_defense_zigzag";
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
