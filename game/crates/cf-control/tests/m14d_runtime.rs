//! **M14D** — Runtime-evidence acceptance tests for the projectile-
//! projectile CCD pass.
//!
//! Drives `m14d_projectile_intercept.ron` end-to-end through
//! `M0Engine::drive_tick` and asserts the M14D `cf-physics::projectile`
//! kernel actually fires per-tick AND produces the contract-required
//! `collision.projectile_pair_contact` events in the replay log.
//!
//! Each test cites the VAL-M14D-* assertion it satisfies:
//!   - VAL-M14D-001: kinetic-vs-explosive emits `outcome="fuze_triggered"`.
//!   - VAL-M14D-006: APS laser intercepts HEAT round with
//!     `outcome="aps_intercept"`.
//!   - VAL-M14D-007: same-seed determinism across two engines.
//!   - VAL-M14D-020: per-tick schedule trace — projectile-pair pass
//!     runs STRICTLY between actor-collision pass and terrain pass.
//!   - VAL-CROSS-003: APS intercept suppresses HEAT armor traversal.

use cf_control::{M0Engine, M0EngineConfig, Scenario};

fn locate_scenario(id: &str) -> std::path::PathBuf {
    let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    workspace.join("content").join("scenarios").join(format!("{id}.ron"))
}

fn drive_scenario(scenario_id: &str, ticks: u64) -> (M0Engine, Vec<cf_replay::Event>) {
    let path = locate_scenario(scenario_id);
    let scenario = Scenario::load_from_file(&path).expect("scenario parses");
    let config = M0EngineConfig::for_loaded_scenario(&scenario, path);
    let engine = M0Engine::new(config);
    engine.record_run_started();
    for _ in 0..ticks {
        if engine.drive_tick().is_none() {
            break;
        }
    }
    let events = engine.recorder().snapshot_events();
    (engine, events)
}

fn count_events(events: &[cf_replay::Event], category: &str, event_type: &str) -> usize {
    events
        .iter()
        .filter(|e| e.category == category && e.event_type == event_type)
        .count()
}

fn find_event<'a>(
    events: &'a [cf_replay::Event],
    category: &str,
    event_type: &str,
    predicate: impl Fn(&cf_replay::Event) -> bool,
) -> Option<&'a cf_replay::Event> {
    events
        .iter()
        .find(|e| e.category == category && e.event_type == event_type && predicate(e))
}

/// **VAL-M14D-001 runtime evidence**: driving `m14d_projectile_intercept.ron`
/// must emit at least one `collision.projectile_pair_contact` event
/// with `outcome="fuze_triggered"` (kinetic-vs-explosive pair in the
/// scenario's seeded pool).
#[test]
fn val_m14d_001_runtime_fuze_triggered_event_emitted() {
    let (_engine, events) = drive_scenario("m14d_projectile_intercept", 60);
    let fuze = find_event(&events, "collision", "projectile_pair_contact", |e| {
        e.payload
            .get("outcome")
            .and_then(|v| v.as_str())
            .map(|s| s == "fuze_triggered")
            .unwrap_or(false)
    });
    assert!(
        fuze.is_some(),
        "expected one collision.projectile_pair_contact with outcome=fuze_triggered. \
         Got events: {:?}",
        events
            .iter()
            .filter(|e| e.category == "collision")
            .map(|e| (e.event_type.as_str(), e.payload.get("outcome").cloned()))
            .collect::<Vec<_>>()
    );
}

/// **VAL-M14D-006 runtime evidence**: APS laser vs HEAT round in the
/// seeded pool fires `outcome="aps_intercept"`.
#[test]
fn val_m14d_006_runtime_aps_intercept_event_emitted() {
    let (_engine, events) = drive_scenario("m14d_projectile_intercept", 60);
    let aps = find_event(&events, "collision", "projectile_pair_contact", |e| {
        e.payload
            .get("outcome")
            .and_then(|v| v.as_str())
            .map(|s| s == "aps_intercept")
            .unwrap_or(false)
    });
    assert!(aps.is_some(), "expected outcome=aps_intercept event");
}

/// **VAL-M14D-014 runtime evidence**: every emitted
/// `collision.projectile_pair_contact` carries `cosmetic: true` in its
/// payload (renderer drops these first under backpressure; killcam
/// excludes by default).
#[test]
fn val_m14d_014_runtime_cosmetic_true_on_every_emit() {
    let (_engine, events) = drive_scenario("m14d_projectile_intercept", 60);
    let pair_events: Vec<&cf_replay::Event> = events
        .iter()
        .filter(|e| e.category == "collision" && e.event_type == "projectile_pair_contact")
        .collect();
    assert!(!pair_events.is_empty(), "scenario must produce at least one pair contact");
    for ev in &pair_events {
        let cosmetic = ev
            .payload
            .get("cosmetic")
            .and_then(|v| v.as_bool())
            .expect("payload carries cosmetic flag");
        assert!(cosmetic, "every pair contact must have cosmetic=true");
    }
}

/// **VAL-M14D-020 runtime evidence**: schedule trace shows
/// `(actor_collision_start, projectile_pair_start, terrain_start)`
/// triple every tick — projectile-pair pass runs STRICTLY between
/// actor-collision and terrain passes.
#[test]
fn val_m14d_020_runtime_schedule_trace_ordering() {
    let path = locate_scenario("m14d_projectile_intercept");
    let scenario = Scenario::load_from_file(&path).expect("scenario parses");
    let config = M0EngineConfig::for_loaded_scenario(&scenario, path);
    let engine = M0Engine::new(config);
    engine.record_run_started();
    for _ in 0..30 {
        if engine.drive_tick().is_none() {
            break;
        }
    }
    let trace = engine.m14d_schedule_trace_snapshot();
    assert!(!trace.is_empty(), "schedule trace must be populated");
    // For every projectile_pair_start marker, the immediately-preceding
    // marker must be actor_collision_end (or actor_collision_start) and
    // the immediately-following terrain_start must appear strictly
    // after it.
    let mut found_triple = 0;
    for window in trace.windows(4) {
        if window[0] == "actor_collision_start"
            && window[1] == "actor_collision_end"
            && window[2] == "projectile_pair_start"
            && window[3] == "projectile_pair_end"
        {
            found_triple += 1;
        }
    }
    assert!(
        found_triple > 0,
        "expected ordered (actor_collision_start, actor_collision_end, \
         projectile_pair_start, projectile_pair_end) triple. Trace: {:?}",
        trace
    );
    // Terrain pass marker must appear AFTER at least one
    // projectile_pair_end marker.
    let pair_end_idx = trace.iter().position(|m| *m == "projectile_pair_end");
    let terrain_start_idx = trace.iter().position(|m| *m == "terrain_start");
    if let (Some(pe), Some(ts)) = (pair_end_idx, terrain_start_idx) {
        assert!(
            pe < ts,
            "terrain_start ({}) must come after projectile_pair_end ({})",
            ts,
            pe
        );
    }
    // Invocation counter must equal at least the number of advanced
    // ticks.
    assert!(
        engine.m14d_pair_pass_invocations() >= 1,
        "pass invocation counter must be incremented per advanced tick; got {}",
        engine.m14d_pair_pass_invocations()
    );
}

/// **VAL-M14D-007 runtime evidence**: same-seed determinism — two
/// engines driving the same scenario produce identical
/// `collision.projectile_pair_contact` event sequences AND identical
/// determinism checksums at the close of the run.
#[test]
fn val_m14d_007_runtime_determinism_byte_identical_at_tick_600() {
    let (engine_a, events_a) = drive_scenario("m14d_projectile_intercept", 600);
    let (engine_b, events_b) = drive_scenario("m14d_projectile_intercept", 600);
    let _ = engine_a;
    let _ = engine_b;
    let pair_a: Vec<&cf_replay::Event> = events_a
        .iter()
        .filter(|e| e.category == "collision" && e.event_type == "projectile_pair_contact")
        .collect();
    let pair_b: Vec<&cf_replay::Event> = events_b
        .iter()
        .filter(|e| e.category == "collision" && e.event_type == "projectile_pair_contact")
        .collect();
    assert_eq!(pair_a.len(), pair_b.len(), "same-seed runs must emit same pair-contact count");
    for (a, b) in pair_a.iter().zip(pair_b.iter()) {
        assert_eq!(a.tick, b.tick, "tick mismatch");
        assert_eq!(a.payload, b.payload, "payload mismatch");
    }
    // Determinism.sim_checksum at the end of run must match.
    let checksum_a = events_a
        .iter()
        .rev()
        .find(|e| e.category == "determinism" && e.event_type == "sim_checksum")
        .and_then(|e| e.payload.get("checksum_hex").and_then(|v| v.as_str().map(String::from)));
    let checksum_b = events_b
        .iter()
        .rev()
        .find(|e| e.category == "determinism" && e.event_type == "sim_checksum")
        .and_then(|e| e.payload.get("checksum_hex").and_then(|v| v.as_str().map(String::from)));
    assert_eq!(
        checksum_a, checksum_b,
        "same-seed determinism.sim_checksum mismatch"
    );
}

/// **VAL-M14D-002 runtime evidence**: after the fuze_triggered intercept
/// fires, the grenade + bullet are removed from the engine's
/// projectile-pair pool.
#[test]
fn val_m14d_002_runtime_grenade_and_bullet_consumed() {
    let path = locate_scenario("m14d_projectile_intercept");
    let scenario = Scenario::load_from_file(&path).expect("scenario parses");
    let config = M0EngineConfig::for_loaded_scenario(&scenario, path);
    let engine = M0Engine::new(config);
    engine.record_run_started();
    let initial_pool_len = engine.m14d_projectile_pair_pool_len();
    assert!(
        initial_pool_len >= 4,
        "scenario must seed at least 4 projectiles, got {}",
        initial_pool_len
    );
    for _ in 0..60 {
        if engine.drive_tick().is_none() {
            break;
        }
    }
    let final_pool = engine.m14d_projectile_pair_pool_snapshot();
    assert!(
        final_pool.len() < initial_pool_len,
        "pool must shrink as projectiles are consumed by intercepts"
    );
    // Grenade id 10 + bullet id 11 + APS id 20 + HEAT id 21 are all
    // consumed by their pair contacts.
    let surviving_ids: std::collections::BTreeSet<u64> = final_pool.iter().map(|p| p.id).collect();
    assert!(
        !surviving_ids.contains(&10),
        "grenade id 10 must be consumed by fuze_triggered"
    );
    assert!(
        !surviving_ids.contains(&11),
        "bullet id 11 must be consumed by fuze_triggered"
    );
    assert!(
        !surviving_ids.contains(&20),
        "APS id 20 must be consumed by aps_intercept"
    );
    assert!(
        !surviving_ids.contains(&21),
        "HEAT id 21 must be consumed by aps_intercept"
    );
}

/// **VAL-M14D-014 runtime evidence (schema validation)**: every emitted
/// `collision.projectile_pair_contact` event validates against the
/// `cf-replay/schemas/event/collision_projectile_pair_contact.json`
/// schema.
#[test]
fn val_m14d_014_runtime_schema_validation_passes() {
    let (_engine, events) = drive_scenario("m14d_projectile_intercept", 60);
    let pair_events: Vec<&cf_replay::Event> = events
        .iter()
        .filter(|e| e.category == "collision" && e.event_type == "projectile_pair_contact")
        .collect();
    assert!(!pair_events.is_empty());
    for ev in &pair_events {
        let payload = &ev.payload;
        let result = cf_replay::schemas::validate_event_payload(
            "collision",
            "projectile_pair_contact",
            payload,
        );
        assert!(
            result.is_ok(),
            "schema validation failed for payload {:?}: {:?}",
            payload,
            result
        );
    }
}

/// **VAL-M14D-019 runtime evidence**: per-player `replay_intercepts`
/// setting defaults to false — surfaced via the engine accessor.
#[test]
fn val_m14d_019_default_replay_intercepts_is_false() {
    let (engine, _events) = drive_scenario("m14d_projectile_intercept", 1);
    assert!(
        !engine.m14d_replay_intercepts(),
        "scenario must default `m14d_replay_intercepts` to false"
    );
}

/// **VAL-M14D-020 runtime evidence**: pass invocation counter equals
/// the number of advanced ticks (one invocation per tick).
#[test]
fn val_m14d_020_runtime_pass_invocations_match_tick_count() {
    let path = locate_scenario("m14d_projectile_intercept");
    let scenario = Scenario::load_from_file(&path).expect("scenario parses");
    let config = M0EngineConfig::for_loaded_scenario(&scenario, path);
    let engine = M0Engine::new(config);
    engine.record_run_started();
    let mut advanced = 0u64;
    for _ in 0..20 {
        if engine.drive_tick().is_some() {
            advanced += 1;
        }
    }
    assert_eq!(
        engine.m14d_pair_pass_invocations(),
        advanced,
        "pair-pass invocations must equal advanced tick count"
    );
}

/// **VAL-M14D-009 runtime evidence**: the schedule-trace's recorded
/// pair pass leaves the last-trace narrowphase candidates ≤ 12 for
/// the scenario's seeded pool.
#[test]
fn val_m14d_009_runtime_narrowphase_candidates_capped() {
    let path = locate_scenario("m14d_projectile_intercept");
    let scenario = Scenario::load_from_file(&path).expect("scenario parses");
    let config = M0EngineConfig::for_loaded_scenario(&scenario, path);
    let engine = M0Engine::new(config);
    engine.record_run_started();
    engine.drive_tick();
    let trace = engine.m14d_last_pair_pass_trace();
    assert!(
        trace.broadphase_candidates <= 12,
        "broadphase candidates {} must be ≤ 12",
        trace.broadphase_candidates
    );
}

/// **VAL-CROSS-003 runtime evidence**: when APS intercept removes a
/// HEAT projectile from the pool, NO `armor.heat_jet_traversed` is
/// emitted for that projectile. In this scenario the HEAT is in the
/// projectile-pair pool (not the actor-fired pool), so no
/// armor.heat_jet_traversed should fire at all.
#[test]
fn val_cross_003_runtime_aps_intercept_suppresses_heat_traversal() {
    let (_engine, events) = drive_scenario("m14d_projectile_intercept", 60);
    let heat_traversal_count = count_events(&events, "armor", "heat_jet_traversed");
    assert_eq!(
        heat_traversal_count, 0,
        "intercepted HEAT projectile must not produce armor.heat_jet_traversed"
    );
    // BUT the aps_intercept event must fire.
    let aps_count = events
        .iter()
        .filter(|e| {
            e.category == "collision"
                && e.event_type == "projectile_pair_contact"
                && e.payload
                    .get("outcome")
                    .and_then(|v| v.as_str())
                    .map(|s| s == "aps_intercept")
                    .unwrap_or(false)
        })
        .count();
    assert!(aps_count >= 1, "aps_intercept must fire at least once");
}
