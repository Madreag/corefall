//! Cross-cutting tests for the `cf-replay` crate root: recorder envelope,
//! run-bundle writer, expected-outcome enum, and manifest deserialization.

use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};

use cf_sim_core::Tick;

use crate::{
    bundle::{write_run_bundle, BundleInputs},
    event::Event,
    manifest::{
        default_delta_baseline_cadence_ticks, default_save_schema_version, BuildInfo, CapabilitiesBlock, CaptureConfig,
        ChecksumConfig, ExpectedOutcome, RunManifest, SceneInfo, SettingsBlock,
    },
    recorder::{AssetRefRecordParams, Recorder},
    summary::TestRecord,
    CONTROL_SCHEMA_VERSION, EVENT_ENVELOPE_VERSION, EVENT_SCHEMA_VERSION, MANIFEST_SCHEMA_VERSION,
    SCENARIO_SCHEMA_VERSION,
};

fn manifest_for_test(run_id: &str) -> RunManifest {
    RunManifest {
        schema_version: MANIFEST_SCHEMA_VERSION.to_string(),
        run_id: run_id.to_string(),
        prototype_slice: "M0".to_string(),
        run_mode: "test".to_string(),
        milestone: "m0".to_string(),
        build: BuildInfo::default(),
        scene: SceneInfo {
            id: "m0_blank".to_string(),
            display_name: "M0 Blank Scene".to_string(),
            source_path: "content/scenarios/m0_blank.ron".to_string(),
        },
        seed: 42,
        started_at_utc: "2026-05-05T00:00:00Z".to_string(),
        duration_target_sec: 5.0,
        material_schema_version: "n/a-m0".to_string(),
        config_hash: "deadbeef".to_string(),
        assumptions_tested: vec!["sim ticks".to_string()],
        linked_specs: vec!["spec/prototype-roadmap".to_string()],
        expected_tests: vec!["M0-SMOKE-01".to_string()],
        capture_config: CaptureConfig::default(),
        schemas: {
            let mut m = BTreeMap::new();
            m.insert("control".to_string(), CONTROL_SCHEMA_VERSION);
            m.insert("scenario".to_string(), SCENARIO_SCHEMA_VERSION);
            m.insert("events".to_string(), EVENT_ENVELOPE_VERSION);
            m
        },
        capabilities: CapabilitiesBlock::default(),
        settings: SettingsBlock::default(),
        checksum: ChecksumConfig::m0_default(),
        tick_rate_hz: 60,
        expected_outcome: ExpectedOutcome::Clean,
        save_schema_version: default_save_schema_version(),
        delta_baseline_cadence_ticks: default_delta_baseline_cadence_ticks(),
        ledger_chain_anchor: None,
    }
}

#[test]
fn roundtrip_event_envelope() {
    let recorder = Recorder::new("m0_test_aabbccdd".to_string());
    let id = recorder.record(
        Tick(7),
        116.6,
        "system",
        "run_started",
        serde_json::json!({"reason": "test"}),
        None,
    );
    assert_eq!(id, "m0_test_aabbccdd:7:0");
    let events = recorder.snapshot_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].schema_version, EVENT_SCHEMA_VERSION);
    let line = serde_json::to_string(&events[0]).unwrap();
    let parsed: Event = serde_json::from_str(&line).unwrap();
    assert_eq!(parsed.event_id, id);
    assert_eq!(parsed.category, "system");
}

/// envelope-level `asset_ref` field with a ledger AssetId string. Run
/// bundles that capture grids / audio playback / mod assets MUST link
/// to the canonical ledger entry via this field.
#[test]
fn record_with_asset_ref_populates_envelope_field() {
    let recorder = Recorder::new("m4a_test_run".to_string());
    let asset_id = "a".repeat(64);
    let id = recorder.record_with_asset_ref(AssetRefRecordParams {
        tick: Tick(1),
        sim_time_ms: 16.6,
        category: "capture",
        event_type: "capture_grid_screenshot",
        payload: serde_json::json!({"path": "captures/grid_001.png"}),
        parent_event_id: None,
        asset_ref: asset_id.clone(),
        cosmetic: true,
    });
    let events = recorder.snapshot_events();
    assert_eq!(events.len(), 1);
    let serialized = serde_json::to_string(&events[0]).unwrap();
    assert!(
        serialized.contains(&format!("\"asset_ref\":\"{asset_id}\"")),
        "asset_ref must round-trip through JSON envelope: {serialized}"
    );
    let parsed: Event = serde_json::from_str(&serialized).unwrap();
    assert_eq!(parsed.event_id, id);
    assert_eq!(parsed.asset_ref.as_deref(), Some(asset_id.as_str()));
    assert_eq!(parsed.cosmetic, Some(true));
}

#[test]
fn write_bundle_and_validate_basics() {
    let tmp = tempdir_for_test();
    let recorder = Recorder::new("m0_test_aabbccdd".to_string());
    recorder.record(Tick(0), 0.0, "system", "run_started", serde_json::json!({}), None);
    recorder.record(
        Tick(60),
        1000.0,
        "determinism",
        "sim_checksum",
        serde_json::json!({"checksum_hex": "00"}),
        None,
    );
    recorder.record(Tick(60), 1000.0, "system", "run_finished", serde_json::json!({}), None);
    let manifest = manifest_for_test("m0_test_aabbccdd");
    let inputs = BundleInputs {
        recorder: &recorder,
        manifest: manifest.clone(),
        started_at: DateTime::parse_from_rfc3339("2026-05-05T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        ended_at: DateTime::parse_from_rfc3339("2026-05-05T00:00:05Z")
            .unwrap()
            .with_timezone(&Utc),
        exit_code: 0,
        result: "pass".to_string(),
        blockers: vec![],
        next_actions: vec!["Proceed to M1.".to_string()],
        tests: vec![TestRecord {
            id: "M0-SMOKE-01".to_string(),
            result: "pass".to_string(),
            evidence_event_ids: vec!["m0_test_aabbccdd:0:0".to_string(), "m0_test_aabbccdd:60:2".to_string()],
            notes: None,
        }],
        artifacts: vec![],
        assumptions_tested: manifest.assumptions_tested.clone(),
        good: vec!["sim stable".to_string()],
        bad: vec![],
        meh: vec![],
        evidence_links: vec!["events.jsonl".to_string()],
        notes_extra: String::new(),
        perf: None,
    };
    let summary = write_run_bundle(&tmp, inputs).unwrap();
    assert_eq!(summary.event_counts.total, 3);
    assert_eq!(summary.first_tick, Some(0));
    assert_eq!(summary.last_tick, Some(60));
    assert_eq!(summary.checksum_event_count, 1);
    let notes = std::fs::read_to_string(tmp.join("notes.md")).unwrap();
    for h in [
        "## Assumptions Tested",
        "## Good",
        "## Bad",
        "## Meh",
        "## Evidence Links",
        "## Next Actions",
    ] {
        assert!(notes.contains(h), "notes.md missing heading {h}");
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

fn tempdir_for_test() -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("cf_replay_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn expected_outcome_default_is_clean() {
    let outcome: ExpectedOutcome = Default::default();
    assert!(matches!(outcome, ExpectedOutcome::Clean));
    assert_eq!(outcome.as_str(), "clean");
}

#[test]
fn expected_outcome_as_str_covers_all_variants() {
    assert_eq!(ExpectedOutcome::Clean.as_str(), "clean");
    assert_eq!(ExpectedOutcome::Panic.as_str(), "panic");
    assert_eq!(ExpectedOutcome::Abort.as_str(), "abort");
}

#[test]
fn expected_outcome_serializes_as_snake_case() {
    for (variant, expected) in [
        (ExpectedOutcome::Clean, "\"clean\""),
        (ExpectedOutcome::Panic, "\"panic\""),
        (ExpectedOutcome::Abort, "\"abort\""),
    ] {
        let s = serde_json::to_string(&variant).expect("serialize");
        assert_eq!(s, expected);
    }
}

#[test]
fn expected_outcome_deserializes_from_snake_case() {
    let clean: ExpectedOutcome = serde_json::from_str("\"clean\"").expect("deserialize clean");
    let panic: ExpectedOutcome = serde_json::from_str("\"panic\"").expect("deserialize panic");
    let abort: ExpectedOutcome = serde_json::from_str("\"abort\"").expect("deserialize abort");
    assert!(matches!(clean, ExpectedOutcome::Clean));
    assert!(matches!(panic, ExpectedOutcome::Panic));
    assert!(matches!(abort, ExpectedOutcome::Abort));
}

#[test]
fn expected_outcome_rejects_unknown_string() {
    let res: Result<ExpectedOutcome, _> = serde_json::from_str("\"weird\"");
    assert!(res.is_err(), "unknown expected_outcome value must reject");
}

#[test]
fn expected_outcome_default_in_run_manifest_when_absent() {
    let manifest_no_outcome = serde_json::json!({
        "schema_version": MANIFEST_SCHEMA_VERSION,
        "run_id": "m0_test_no_outcome",
        "prototype_slice": "M0",
        "run_mode": "test",
        "milestone": "m0",
        "build": {"commit_sha": "deadbeef", "rust_version": "rustc 1.95", "bevy_version": "bevy 0.18", "platform": "test"},
        "scene": {"id": "m0_blank", "display_name": "test", "source_path": "x"},
        "seed": 42,
        "started_at_utc": "2026-05-05T00:00:00Z",
        "duration_target_sec": 5.0,
        "material_schema_version": "n/a-m0",
        "config_hash": "deadbeef",
        "assumptions_tested": [],
        "linked_specs": [],
        "expected_tests": [],
        "capture_config": {"events": true, "screenshots": false, "captures": false},
        "schemas": {"control": 1, "scenario": 1, "events": 1},
        "capabilities": {"debug": false, "control_api": true, "save_load": false, "debug_capabilities": []},
        "settings": {"ui_scale": 1.0, "high_contrast": false, "captions": true, "reduced_motion": false, "reduced_shake": false, "reduced_flash": false},
        "checksum": {"algorithm": "blake3", "scope": "sim_state_v1", "cadence_ticks": 60},
        "tick_rate_hz": 60
    });
    let parsed: RunManifest =
        serde_json::from_value(manifest_no_outcome).expect("manifest without expected_outcome must parse");
    assert!(
        matches!(parsed.expected_outcome, ExpectedOutcome::Clean),
        "missing expected_outcome must default to Clean"
    );
    assert!(
        parsed.settings.key_bindings.is_empty(),
        "legacy manifests without key_bindings must deserialize with an empty remap table"
    );
}

#[test]
fn recorder_record_panics_on_poisoned_mutex_instead_of_returning_phantom_event_id() {
    use std::sync::Arc;
    let recorder = Arc::new(Recorder::new("m0_test_poison_e2e".to_string()));
    let recorder_for_thread = recorder.clone();
    let _ = std::thread::spawn(move || {
        let _guard = recorder_for_thread
            .inner
            .lock()
            .expect("first lock acquisition should succeed in setup");
        panic!("intentional poison for regression test");
    })
    .join();
    let recorder_for_call = recorder.clone();
    let panic_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        recorder_for_call.record(Tick(0), 0.0, "system", "run_started", serde_json::json!({}), None);
    }));
    assert!(
        panic_result.is_err(),
        "record() must panic when mutex is poisoned (issue #18); previously it silently returned a phantom event_id"
    );
}

#[test]
fn m4_recorder_cosmetic_events_drop_first_under_pressure() {
    let recorder = Recorder::with_capacity("m4_test_drop".to_string(), 3);
    recorder.record_cosmetic(Tick(0), 0.0, "ux", "banner_raised", serde_json::json!({}), None);
    recorder.record_cosmetic(Tick(0), 0.0, "ux", "banner_raised", serde_json::json!({}), None);
    recorder.record(Tick(0), 0.0, "combat", "weapon_fired", serde_json::json!({}), None);
    assert_eq!(recorder.event_count(), 3);
    assert_eq!(recorder.dropped_count(), 0);

    recorder.record(Tick(0), 0.0, "combat", "wound_added", serde_json::json!({}), None);
    assert_eq!(recorder.event_count(), 3, "buffer still capped at capacity");
    assert_eq!(recorder.dropped_count(), 1);
    assert_eq!(recorder.dropped_cosmetic_count(), 1);
    assert_eq!(recorder.dropped_gameplay_count(), 0);
    let events = recorder.snapshot_events();
    let types: Vec<&str> = events.iter().map(|e| e.event_type.as_str()).collect();
    assert!(types.contains(&"wound_added"));
    assert!(types.contains(&"weapon_fired"));
}

#[test]
fn m4_recorder_cosmetic_dropped_when_no_eviction_target_available() {
    let recorder = Recorder::with_capacity("m4_test_cos".to_string(), 2);
    recorder.record(Tick(0), 0.0, "combat", "weapon_fired", serde_json::json!({}), None);
    recorder.record(Tick(0), 0.0, "combat", "wound_added", serde_json::json!({}), None);
    recorder.record_cosmetic(Tick(0), 0.0, "ux", "banner_raised", serde_json::json!({}), None);
    assert_eq!(recorder.event_count(), 2);
    assert_eq!(recorder.dropped_cosmetic_count(), 1);
    assert_eq!(recorder.dropped_gameplay_count(), 0);
}

#[test]
fn m4_recorder_gameplay_dropped_when_buffer_full_of_gameplay() {
    let recorder = Recorder::with_capacity("m4_test_gameplay".to_string(), 2);
    recorder.record(Tick(0), 0.0, "combat", "weapon_fired", serde_json::json!({}), None);
    recorder.record(Tick(0), 0.0, "combat", "wound_added", serde_json::json!({}), None);
    recorder.record(Tick(0), 0.0, "combat", "kill", serde_json::json!({}), None);
    assert_eq!(recorder.event_count(), 2);
    assert_eq!(recorder.dropped_count(), 1);
    assert_eq!(recorder.dropped_gameplay_count(), 1);
}

#[test]
fn m4_recorder_cosmetic_field_serializes_only_when_set() {
    let recorder = Recorder::new("m4_test_field".to_string());
    recorder.record(Tick(0), 0.0, "combat", "weapon_fired", serde_json::json!({}), None);
    recorder.record_cosmetic(Tick(0), 0.0, "ux", "banner_raised", serde_json::json!({}), None);
    let events = recorder.snapshot_events();
    let s = serde_json::to_string(&events[0]).unwrap();
    assert!(
        !s.contains("\"cosmetic\""),
        "gameplay event must not serialize cosmetic field: {s}"
    );
    let s = serde_json::to_string(&events[1]).unwrap();
    assert!(
        s.contains("\"cosmetic\":true"),
        "cosmetic event must serialize cosmetic: true: {s}"
    );
}

#[test]
fn m4_recorder_peak_buffer_depth_tracks_high_water_mark() {
    let recorder = Recorder::new("m4_test_peak".to_string());
    recorder.record(Tick(0), 0.0, "combat", "weapon_fired", serde_json::json!({}), None);
    recorder.record(Tick(0), 0.0, "combat", "wound_added", serde_json::json!({}), None);
    recorder.record(Tick(0), 0.0, "combat", "kill", serde_json::json!({}), None);
    assert_eq!(recorder.peak_buffer_depth(), 3);
}

#[test]
fn m4_recorder_pending_drop_tag_propagates_to_next_event() {
    let recorder = Recorder::with_capacity("m4_test_tag".to_string(), 1);
    recorder.record(Tick(0), 0.0, "combat", "weapon_fired", serde_json::json!({}), None);
    recorder.record(Tick(0), 0.0, "combat", "wound_added", serde_json::json!({}), None);
    recorder.record(Tick(0), 0.0, "combat", "kill", serde_json::json!({}), None);
    recorder.dropped(1);
    let recorder = Recorder::new("m4_test_tag_2".to_string());
    recorder.dropped(7);
    let id = recorder.record(Tick(0), 0.0, "system", "run_started", serde_json::json!({}), None);
    let events = recorder.snapshot_events();
    let ev = events.iter().find(|e| e.event_id == id).expect("recorded event");
    assert_eq!(
        ev.dropped_count,
        Some(7),
        "next emitted event must carry the outstanding drop count"
    );
}
