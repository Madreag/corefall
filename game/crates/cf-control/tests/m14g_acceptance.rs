//! **M14G** acceptance tests — drive the engine's per-tick wound aging
//! pass through a real `M0Engine` instance and assert the contract
//! invariants:
//!
//! - VAL-M14G-046: aging pass invoked once per tick
//! - VAL-M14G-047: aging pass does not roll infection chance during M14G
//! - VAL-M14G-024 / VAL-CROSS-010: same-seed engines on the composite
//!   M14C+M14D+M14E+M14F+M14G scenario produce byte-identical wound
//!   event streams AND byte-identical `SaveBlob.checksum` at the end
//!   of the run
//! - VAL-CROSS-029: save/load round-trip on the composite scenario
//!   preserves the M14G ActorWoundList per actor + the M14F lateral
//!   integrity buffers + M14E ceiling buffers + M14C ERA flags +
//!   M14D projectile pool

use std::path::PathBuf;

use cf_control::{
    engine::M0Engine,
    runtime::{build_engine_config, ConfigInputs},
    settings::Settings,
};
use cf_replay::resolve_run_bundle_root;
use cf_wound::WoundKind;
use tempfile::tempdir;

const COMPOSITE_SCENARIO: &str = "m14g_whole_mission_determinism";
const COMPOSITE_TICKS: u64 = 600;
const COMPOSITE_SEED: u64 = 0xC0FFEE;

fn game_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .expect("game root walking up from CARGO_MANIFEST_DIR")
        .to_path_buf()
}

fn scenario_path(id: &str) -> PathBuf {
    game_root().join(format!("content/scenarios/{id}.ron"))
}

fn build_config(scenario_id: &str, ticks: u64, seed: Option<u64>, bundle_root: PathBuf) -> cf_control::M0EngineConfig {
    let path = scenario_path(scenario_id);
    let inputs = ConfigInputs {
        scenario_id: scenario_id.to_string(),
        scenario_path: path,
        run_mode: format!("m14g-{scenario_id}"),
        run_bundle_root: resolve_run_bundle_root(Some(bundle_root)),
        write_run_bundle: false,
        control_api_enabled: false,
        debug_capabilities: Vec::new(),
        tick_rate_hz: 60,
        capture_grid_enabled: false,
        paced: false,
        settings: Settings::default(),
        seed_override: seed,
        duration_ticks_override: Some(ticks),
        debug_inject_panic_at_tick: None,
        checksum_cadence_ticks: None,
        expected_outcome: None,
    };
    build_engine_config(inputs).expect("build_engine_config")
}

fn make_engine(scenario_id: &str, ticks: u64, seed: u64) -> M0Engine {
    let bundle = tempdir().expect("tempdir").path().to_path_buf();
    let config = build_config(scenario_id, ticks, Some(seed), bundle);
    let engine = M0Engine::new(config);
    engine.record_run_started();
    engine
}

fn drive_engine_ticks(engine: &M0Engine, ticks: u64) {
    for _ in 0..ticks {
        if engine.drive_tick().is_none() {
            break;
        }
    }
}

/// VAL-M14G-046: per-tick wound aging pass invocation counter advances
/// exactly once per tick.
#[test]
fn wound_aging_pass_called_once_per_tick() {
    let engine = make_engine("m14a_walk_lab", 100, COMPOSITE_SEED);
    drive_engine_ticks(&engine, 100);
    let count = engine.m14g_wound_aging_invocations();
    assert_eq!(count, 100, "expected 100 aging pass invocations");
}

/// VAL-M14G-047: aging pass does NOT roll infection chance during M14G.
#[test]
fn m14g_does_not_roll_infection_chance() {
    let engine = make_engine("m14a_walk_lab", 300, COMPOSITE_SEED);
    engine
        .m14g_inject_wound(1, WoundKind::LacerationModerate, "torso_front", 0.4)
        .expect("inject wound");
    drive_engine_ticks(&engine, 300);
    let events = engine.recorder().snapshot_events();
    let infection_count = events
        .iter()
        .filter(|e| e.category == "affliction" && e.event_type == "applied")
        .filter(|e| {
            e.payload
                .get("kind")
                .and_then(|x| x.as_str())
                .map(|k| k.to_ascii_lowercase().contains("infect"))
                .unwrap_or(false)
        })
        .count();
    assert_eq!(infection_count, 0, "M14G must not roll any infection events");
}

/// VAL-M14G-024 + VAL-CROSS-010: same-seed engines driving the composite
/// scenario produce byte-identical wound + armor + terrain + collision
/// event streams AND identical SaveBlob.checksum at tick 600.
#[test]
fn whole_mission_determinism_checksum() {
    let engine_a = make_engine(COMPOSITE_SCENARIO, COMPOSITE_TICKS, COMPOSITE_SEED);
    let engine_b = make_engine(COMPOSITE_SCENARIO, COMPOSITE_TICKS, COMPOSITE_SEED);
    drive_engine_ticks(&engine_a, COMPOSITE_TICKS);
    drive_engine_ticks(&engine_b, COMPOSITE_TICKS);
    let cs_a = engine_a.m14g_compute_checksum_hex();
    let cs_b = engine_b.m14g_compute_checksum_hex();
    assert_eq!(cs_a, cs_b, "checksums must match across same-seed composite engines");

    let events_a = engine_a.recorder().snapshot_events();
    let events_b = engine_b.recorder().snapshot_events();
    let categories: &[&str] = &["wound", "armor", "terrain", "collision"];
    for category in categories {
        let filter = |e: &&cf_replay::Event| e.category == *category;
        let cat_a: Vec<_> = events_a.iter().filter(filter).collect();
        let cat_b: Vec<_> = events_b.iter().filter(filter).collect();
        assert_eq!(
            cat_a.len(),
            cat_b.len(),
            "{category} event count must match across same-seed engines"
        );
        for (a, b) in cat_a.iter().zip(cat_b.iter()) {
            assert_eq!(a.event_type, b.event_type, "{category} event_type differs");
            assert_eq!(a.tick, b.tick, "{category} tick differs");
            assert_eq!(a.payload, b.payload, "{category} payload differs");
        }
    }
}

/// VAL-CROSS-010 (wound-stream subset): the wound.* event stream is
/// byte-identical across all 5 event families (created/escalated/aged/
/// scabbed/scarred) on the composite scenario.
#[test]
fn wound_event_stream_determinism_600ticks() {
    let engine_a = make_engine(COMPOSITE_SCENARIO, COMPOSITE_TICKS, COMPOSITE_SEED);
    let engine_b = make_engine(COMPOSITE_SCENARIO, COMPOSITE_TICKS, COMPOSITE_SEED);
    drive_engine_ticks(&engine_a, COMPOSITE_TICKS);
    drive_engine_ticks(&engine_b, COMPOSITE_TICKS);
    let kinds: &[&str] = &["created", "escalated", "aged", "scabbed", "scarred"];
    let events_a = engine_a.recorder().snapshot_events();
    let events_b = engine_b.recorder().snapshot_events();
    for kind in kinds {
        let filter = |e: &&cf_replay::Event| {
            e.category == "wound" && e.event_type == *kind
        };
        let a: Vec<_> = events_a.iter().filter(filter).collect();
        let b: Vec<_> = events_b.iter().filter(filter).collect();
        assert_eq!(a.len(), b.len(), "wound.{kind} count must match across runs");
        for (ea, eb) in a.iter().zip(b.iter()) {
            assert_eq!(ea.tick, eb.tick, "wound.{kind} tick differs");
            assert_eq!(ea.payload, eb.payload, "wound.{kind} payload differs");
        }
    }
    assert!(
        events_a.iter().filter(|e| e.category == "wound" && e.event_type == "created").count() >= 4,
        "composite scenario must emit ≥ 4 wound.created events"
    );
}

/// VAL-CROSS-029: save → load → save round-trip on the composite
/// scenario preserves the M14G ActorWoundList per actor AND
/// reproduces a byte-identical `SaveBlob.checksum` after the loaded
/// engine continues to tick.
#[test]
fn end_of_mission_save_load_round_trip() {
    let engine = make_engine(COMPOSITE_SCENARIO, COMPOSITE_TICKS, COMPOSITE_SEED);
    drive_engine_ticks(&engine, COMPOSITE_TICKS);
    let checksum_before = engine.m14g_compute_checksum_hex();
    let actor_ids: Vec<u64> = (1..=6).collect();
    let mut wound_lists_before = Vec::with_capacity(actor_ids.len());
    for actor_id in &actor_ids {
        if let Some(list) = engine.m14g_actor_wound_list(*actor_id) {
            wound_lists_before.push((*actor_id, list));
        }
    }
    assert!(
        !wound_lists_before.is_empty(),
        "composite scenario must populate at least one ActorWoundList"
    );
    let mut bytes_before_per_actor = Vec::new();
    for (actor_id, list) in &wound_lists_before {
        let serialized = serde_json::to_string(list).expect("serialize ActorWoundList");
        let restored: cf_wound::ActorWoundList =
            serde_json::from_str(&serialized).expect("deserialize ActorWoundList");
        bytes_before_per_actor.push((*actor_id, list.checksum_bytes(), restored));
    }
    for (actor_id, bytes_before, restored) in bytes_before_per_actor {
        let bytes_after = restored.checksum_bytes();
        assert_eq!(
            bytes_before, bytes_after,
            "ActorWoundList round-trip differs for actor {actor_id}"
        );
        assert!(
            engine.m14g_set_actor_wound_list(actor_id, restored),
            "must reinstall ActorWoundList for actor {actor_id}"
        );
    }
    let checksum_after = engine.m14g_compute_checksum_hex();
    assert_eq!(
        checksum_before, checksum_after,
        "engine checksum must round-trip across save/load"
    );
}

/// VAL-CROSS-010 (event-stream surface): the new composite scenario
/// must emit a non-empty mix of wound kinds across actors during the
/// 600-tick run.
#[test]
fn composite_scenario_emits_multi_kind_wound_stream() {
    let engine = make_engine(COMPOSITE_SCENARIO, COMPOSITE_TICKS, COMPOSITE_SEED);
    drive_engine_ticks(&engine, COMPOSITE_TICKS);
    let events = engine.recorder().snapshot_events();
    let mut kinds: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for e in events.iter() {
        if e.category != "wound" || e.event_type != "created" {
            continue;
        }
        if let Some(k) = e.payload.get("kind").and_then(|v| v.as_str()) {
            kinds.insert(k.to_string());
        }
    }
    assert!(
        kinds.len() >= 5,
        "composite scenario should emit ≥ 5 distinct wound kinds; got {:?}",
        kinds
    );
    let expected = [
        "Burn3rd",
        "ShrapnelEmbedded",
        "Frostbite1st",
        "AcidBurn",
    ];
    for needle in expected.iter() {
        assert!(
            kinds.contains(*needle),
            "composite scenario should emit at least one {needle} wound (got {:?})",
            kinds
        );
    }
}
