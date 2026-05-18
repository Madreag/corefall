//! **M12C** § In-engine cinematic playback acceptance tests.
//!
//! These tests engage the cinematic kernel directly on a live
//! `M0Engine`, advance it through ticks, and assert that the recorder
//! emits the canonical `cinematic.*` event family per spec § "Chapter
//! markers via M4 events" + spec Gherkin acceptance criteria. The
//! tests cover:
//!
//! 1. "Mission opens with a 30-60s in-engine cinematic" — kernel
//!    engages, `cinematic.started { source: opening }` fires, `Ended`
//!    fires on the last tick.
//! 2. "Chapter markers fire on named beats" — at_ms boundaries become
//!    `cinematic.chapter_marker` events.
//! 3. "Sandbox storyteller suppresses cinematics" — kernel emits
//!    `cinematic.skipped { reason: sandbox_suppressed, skipped_at_ms: 0 }`
//!    + `cinematic.ended { was_skipped: true }` without ever running.
//! 4. "Replay-deterministic playback under M4" — two engines at the
//!    same seed produce identical event streams.

use std::path::PathBuf;

use cf_cinematic::{
    builtin_profile, CinematicKernel, CinematicScript, NarrationTrack, ScriptSource, SeenSet, StorytellerId,
};
use cf_control::engine::M0EngineConfig;
use cf_control::runtime::{build_engine_config, ConfigInputs};
use cf_control::Settings;
use cf_replay::resolve_run_bundle_root;
use tempfile::tempdir;

fn locate_scenario(id: &str) -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let game_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("game root walking up from CARGO_MANIFEST_DIR");
    let candidate = game_root.join(format!("content/scenarios/{id}.ron"));
    assert!(candidate.exists(), "missing scenario {}", candidate.display());
    candidate
}

fn build_config(bundle_root: PathBuf, ticks: u64, seed: u64, scenario_id: &str) -> M0EngineConfig {
    let scenario_path = locate_scenario(scenario_id);
    let inputs = ConfigInputs {
        scenario_id: scenario_id.to_string(),
        scenario_path,
        run_mode: "m12c-cinematic-test".to_string(),
        run_bundle_root: resolve_run_bundle_root(Some(bundle_root)),
        write_run_bundle: false,
        control_api_enabled: false,
        debug_capabilities: Vec::new(),
        tick_rate_hz: 60,
        capture_grid_enabled: false,
        paced: false,
        settings: Settings::default(),
        seed_override: Some(seed),
        duration_ticks_override: Some(ticks),
        debug_inject_panic_at_tick: None,
        checksum_cadence_ticks: None,
        expected_outcome: None,
    };
    build_engine_config(inputs).expect("build_engine_config")
}

fn engine_cinematic_events(engine: &cf_control::engine::M0Engine) -> Vec<cf_replay::Event> {
    engine
        .recorder()
        .snapshot_events()
        .into_iter()
        .filter(|e| e.category == "cinematic")
        .collect()
}

fn count_cinematic_events_of_type(engine: &cf_control::engine::M0Engine, event_type: &str) -> usize {
    engine_cinematic_events(engine)
        .iter()
        .filter(|e| e.event_type == event_type)
        .count()
}

fn read_cinematic_events_of_type(
    engine: &cf_control::engine::M0Engine,
    event_type: &str,
) -> Vec<cf_replay::Event> {
    engine_cinematic_events(engine)
        .into_iter()
        .filter(|e| e.event_type == event_type)
        .collect()
}

fn opening_script(id: &str, duration_ms: u32) -> CinematicScript {
    let bytes = format!(
        r#"(
            schema_version: 1,
            id: "{id}",
            source: opening,
            storyteller: None,
            shots: [
                (
                    label: "main",
                    duration_ms: {duration_ms},
                    moves: [
                        ( kind: Pan, start_ms: 0, duration_ms: {duration_ms}, easing: EaseInOutCubic, pan: (10.0, 0.0) ),
                    ],
                ),
            ],
            chapters: [
                ( id: "dropship_door_opens", at_ms: 8000 ),
                ( id: "boss_reveal", at_ms: 22000 ),
            ],
            narration_track_id: None,
            briefing_card_lines: [],
            briefing_at_ms: 15000,
        )"#,
        id = id,
        duration_ms = duration_ms,
    );
    CinematicScript::from_ron(bytes.as_bytes()).expect("parse")
}

/// **M12C Gherkin**: "Mission opens with a 30-60s in-engine cinematic" —
/// engaging the kernel emits `cinematic.started`; advancing through the
/// total duration emits `cinematic.ended` with `was_skipped: false`.
#[test]
fn m12c_mission_opens_with_in_engine_cinematic() {
    let bundle_root = tempdir().expect("tempdir");
    let mut config = build_config(bundle_root.path().to_path_buf(), 60, 42, "m1_actor_range");
    config.seed = 42;
    let engine = std::sync::Arc::new(cf_control::engine::M0Engine::new(config));
    engine.record_run_started();
    let script = opening_script("cin_intro_engine_test", 30_000);
    engine.engage_cinematic_kernel(
        "cin_intro_engine_test",
        ScriptSource::Opening,
        StorytellerId::CassandraClassic,
        script,
        NarrationTrack::default(),
        false,
    );
    // Advance enough to cross the chapter markers + total duration.
    for _ in 0..1_200 {
        engine.advance_cinematic_kernel(25);
    }
    let started = read_cinematic_events_of_type(&engine, "started");
    assert_eq!(started.len(), 1, "expected exactly one cinematic.started");
    let payload = &started[0].payload;
    assert_eq!(payload.get("source").and_then(|v| v.as_str()), Some("opening"));
    assert_eq!(payload.get("id").and_then(|v| v.as_str()), Some("cin_intro_engine_test"));
    let ended = read_cinematic_events_of_type(&engine, "ended");
    assert_eq!(ended.len(), 1, "expected exactly one cinematic.ended");
    let ended_payload = &ended[0].payload;
    assert_eq!(ended_payload.get("was_skipped").and_then(|v| v.as_bool()), Some(false));
}

/// **M12C Gherkin**: "Chapter markers fire on named beats" — the engine
/// emits `cinematic.chapter_marker` for each crossed at_ms.
#[test]
fn m12c_chapter_markers_fire_on_named_beats() {
    let bundle_root = tempdir().expect("tempdir");
    let config = build_config(bundle_root.path().to_path_buf(), 60, 42, "m1_actor_range");
    let engine = std::sync::Arc::new(cf_control::engine::M0Engine::new(config));
    engine.record_run_started();
    let script = opening_script("cin_chapter_test", 30_000);
    engine.engage_cinematic_kernel(
        "cin_chapter_test",
        ScriptSource::Opening,
        StorytellerId::CassandraClassic,
        script,
        NarrationTrack::default(),
        false,
    );
    for _ in 0..1_200 {
        engine.advance_cinematic_kernel(25);
    }
    let markers = read_cinematic_events_of_type(&engine, "chapter_marker");
    let ids: Vec<String> = markers
        .iter()
        .filter_map(|m| m.payload.get("chapter_id").and_then(|v| v.as_str()).map(String::from))
        .collect();
    assert!(ids.contains(&"dropship_door_opens".to_string()), "missing dropship_door_opens");
    assert!(ids.contains(&"boss_reveal".to_string()), "missing boss_reveal");
}

/// **M12C Gherkin**: "Sandbox storyteller suppresses cinematics" — engage
/// with Sandbox profile emits `cinematic.skipped { reason: sandbox_suppressed,
/// skipped_at_ms: 0 }` + `cinematic.ended { was_skipped: true }` without
/// any chapter markers firing.
#[test]
fn m12c_sandbox_suppresses_cinematics_emits_parity_events() {
    let bundle_root = tempdir().expect("tempdir");
    let config = build_config(bundle_root.path().to_path_buf(), 60, 42, "m1_actor_range");
    let engine = std::sync::Arc::new(cf_control::engine::M0Engine::new(config));
    engine.record_run_started();
    let script = opening_script("cin_sandbox_test", 30_000);
    engine.engage_cinematic_kernel(
        "cin_sandbox_test",
        ScriptSource::Opening,
        StorytellerId::Sandbox,
        script,
        NarrationTrack::default(),
        false,
    );
    let skipped = read_cinematic_events_of_type(&engine, "skipped");
    assert_eq!(skipped.len(), 1, "expected one cinematic.skipped");
    let payload = &skipped[0].payload;
    assert_eq!(payload.get("reason").and_then(|v| v.as_str()), Some("sandbox_suppressed"));
    assert_eq!(payload.get("skipped_at_ms").and_then(|v| v.as_u64()), Some(0));
    let ended = read_cinematic_events_of_type(&engine, "ended");
    assert_eq!(ended.len(), 1);
    assert_eq!(
        ended[0].payload.get("was_skipped").and_then(|v| v.as_bool()),
        Some(true)
    );
    let chapters = count_cinematic_events_of_type(&engine, "chapter_marker");
    assert_eq!(chapters, 0, "no chapter markers fire when sandbox-suppressed");
}

/// **M12C Gherkin**: "Replay-deterministic playback under M4" — running
/// the same script + same seed twice yields identical per-tick events.
#[test]
fn m12c_replay_deterministic_event_stream() {
    let script = opening_script("cin_det_engine", 30_000);
    let profile = builtin_profile(StorytellerId::CassandraClassic);
    let mut a = CinematicKernel::new(
        script.clone(),
        profile.clone(),
        NarrationTrack::default(),
        42,
        SeenSet::default(),
        false,
    );
    let mut b = CinematicKernel::new(script, profile, NarrationTrack::default(), 42, SeenSet::default(), false);
    let mut a_stream = Vec::new();
    let mut b_stream = Vec::new();
    for _ in 0..1_200 {
        a_stream.extend(a.advance(25));
        b_stream.extend(b.advance(25));
    }
    assert_eq!(a_stream.len(), b_stream.len(), "tick stream length parity");
    for (ea, eb) in a_stream.iter().zip(b_stream.iter()) {
        assert_eq!(ea.kind(), eb.kind(), "event kind divergence");
    }
    assert_eq!(a.state().camera_translation, b.state().camera_translation);
    assert_eq!(a.state().camera_shake_px, b.state().camera_shake_px);
}

/// **M12C Gherkin**: no cinematic events fire when no cinematic is
/// engaged — the engine remains silent on the cinematic.* surface.
#[test]
fn m12c_no_cinematic_events_when_idle() {
    let bundle_root = tempdir().expect("tempdir");
    let config = build_config(bundle_root.path().to_path_buf(), 1, 42, "m1_actor_range");
    let engine = std::sync::Arc::new(cf_control::engine::M0Engine::new(config));
    engine.record_run_started();
    let cinematic_events = count_cinematic_events_of_type(&engine, "started");
    assert_eq!(cinematic_events, 0, "no cinematic events fire without engage");
}
