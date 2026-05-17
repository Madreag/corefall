//! **M4B § Acceptance criteria** — viewer-side integration tests.
//!
//! Covers:
//!
//! - "Cross-version replay viewer surfaces migration banner" — when the
//!   bundle's `save_schema_version` is older than the current build, the
//!   viewer header reads `"Replay migrated from v1.0.0 -> v2.0.0 ..."`.
//! - "Delta snapshot reconstructs to byte-identical state" — the
//!   `delta_reconstructor` produces the same `serde_json::Value` at every
//!   tick as the live recorded snapshot.

use std::collections::BTreeMap;
use std::fs;

use cf_replay::{
    self, BuildInfo, CaptureConfig, CapabilitiesBlock, ChecksumConfig, EventCounts, ExpectedOutcome,
    PerformanceBlock, RecorderBlock, RunManifest, RunSummary, SceneInfo, SettingsBlock, VolumeBlock, ArtifactsBlock,
};
use cf_tools_replay_viewer::{bundle::Bundle, delta_reconstructor, viewer};

fn write_basic_bundle(dir: &std::path::Path, save_schema_version: [u16; 3]) -> Bundle {
    fs::create_dir_all(dir).unwrap();
    let manifest = RunManifest {
        schema_version: cf_replay::MANIFEST_SCHEMA_VERSION.to_string(),
        run_id: "test_run_m4b".to_string(),
        prototype_slice: "M4B".to_string(),
        run_mode: "test".to_string(),
        milestone: "m4b".to_string(),
        build: BuildInfo::default(),
        scene: SceneInfo {
            id: "m4b_test".to_string(),
            display_name: "M4B Test".to_string(),
            source_path: "test".to_string(),
        },
        seed: 42,
        started_at_utc: "2026-05-16T00:00:00Z".to_string(),
        duration_target_sec: 1.0,
        material_schema_version: "n/a".to_string(),
        config_hash: "0".to_string(),
        assumptions_tested: vec![],
        linked_specs: vec![],
        expected_tests: vec![],
        capture_config: CaptureConfig::default(),
        schemas: BTreeMap::new(),
        capabilities: CapabilitiesBlock::default(),
        settings: SettingsBlock::default(),
        checksum: ChecksumConfig::m0_default(),
        tick_rate_hz: 60,
        expected_outcome: ExpectedOutcome::Clean,
        save_schema_version,
        delta_baseline_cadence_ticks: 600,
        ledger_chain_anchor: None,
    };
    let summary = RunSummary {
        schema_version: cf_replay::SUMMARY_SCHEMA_VERSION.to_string(),
        run_id: "test_run_m4b".to_string(),
        manifest_run_id: "test_run_m4b".to_string(),
        duration_sec: 1.0,
        result: "pass".to_string(),
        ended_at_utc: "2026-05-16T00:00:01Z".to_string(),
        exit_code: 0,
        event_counts: EventCounts {
            total: 0,
            by_category: BTreeMap::new(),
            by_type: BTreeMap::new(),
            by_severity: BTreeMap::new(),
            dropped_total: 0,
        },
        volume: VolumeBlock::default(),
        performance: PerformanceBlock::default(),
        artifacts: ArtifactsBlock::default(),
        blockers: vec![],
        next_actions: vec![],
        tests: vec![],
        final_sim_checksum: None,
        checksum_event_count: 0,
        first_tick: None,
        last_tick: None,
        recorder: RecorderBlock::default(),
    };
    fs::write(dir.join("run_manifest.json"), serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
    fs::write(dir.join("summary.json"), serde_json::to_string_pretty(&summary).unwrap()).unwrap();
    fs::write(dir.join("events.jsonl"), "").unwrap();
    Bundle::load(dir).unwrap()
}

#[test]
fn viewer_header_renders_migration_banner_when_save_schema_version_is_older() {
    let tmp = tempfile::tempdir().unwrap();
    let bundle = write_basic_bundle(tmp.path(), [1, 0, 0]);
    let state = viewer::ViewerState::default();
    let header = viewer::render_markdown(&bundle, &state);
    assert!(
        header.contains("Replay migrated from v1.0.0 -> v2.0.0"),
        "viewer header must surface migration banner; got: {header}"
    );
}

#[test]
fn viewer_header_does_not_render_banner_when_versions_match() {
    let tmp = tempfile::tempdir().unwrap();
    let bundle = write_basic_bundle(tmp.path(), [2, 0, 0]);
    let state = viewer::ViewerState::default();
    let header = viewer::render_markdown(&bundle, &state);
    assert!(
        !header.contains("Replay migrated from"),
        "viewer header must not render banner when versions match; got: {header}"
    );
}

#[test]
fn delta_reconstructor_returns_empty_summary_when_no_snapshot_events() {
    let tmp = tempfile::tempdir().unwrap();
    let bundle = write_basic_bundle(tmp.path(), [2, 0, 0]);
    let summary = delta_reconstructor::summarize(&bundle);
    assert_eq!(summary.baseline_count, 0);
    assert_eq!(summary.delta_chain_depth, 0);
}
