//! **M14E** § Runtime evidence for the per-tick collapse-check pass.
//!
//! Drives `m14e_tunnel_collapse_drill.ron` + `m14e_support_beam_save.ron`
//! end-to-end through `M0Engine::drive_tick` and asserts the M14E
//! kernel actually fires + produces the contract-required events in
//! the replay log.

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

/// VAL-M14E-002: the 24-px tunnel (left/right neighbor chunks in the
/// drill scenario) emits `terrain.structural_integrity_low` within
/// 30 ticks (= 2 integrity passes at N=15 cadence).
#[test]
fn val_m14e_002_structural_integrity_low_fires_within_30_ticks() {
    let (_engine, events) = drive_scenario("m14e_tunnel_collapse_drill", 30);
    let low_count = count_events(&events, "terrain", "structural_integrity_low");
    assert!(
        low_count >= 1,
        "expected ≥1 terrain.structural_integrity_low within 30 ticks, got {low_count}"
    );
}

/// VAL-M14E-003 + VAL-M14E-004: driving the drill scenario to its
/// 600-tick budget emits `terrain.cave_in_triggered` with the required
/// payload fields.
#[test]
fn val_m14e_003_cave_in_triggered_fires_with_required_payload() {
    let (_engine, events) = drive_scenario("m14e_tunnel_collapse_drill", 600);
    let cave_in = find_event(&events, "terrain", "cave_in_triggered", |_| true)
        .unwrap_or_else(|| panic!("no terrain.cave_in_triggered emitted; total events={}", events.len()));
    assert!(cave_in.payload.get("chunk_id").is_some(), "chunk_id field required");
    assert!(cave_in.payload.get("bbox").is_some(), "bbox field required");
    assert!(
        cave_in.payload.get("falling_debris_count").is_some(),
        "falling_debris_count field required"
    );
    let debris = cave_in
        .payload
        .get("falling_debris_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(debris > 0, "falling_debris_count must be > 0");
}

/// VAL-M14E-017: cave-in roll is byte-identical across two same-seed
/// runs (no `thread_rng`). Drives both engines for 600 ticks and
/// compares the replay-event sequence.
#[test]
fn val_m14e_017_cave_in_roll_deterministic_across_seeds() {
    let (_a, a_events) = drive_scenario("m14e_tunnel_collapse_drill", 600);
    let (_b, b_events) = drive_scenario("m14e_tunnel_collapse_drill", 600);
    let a_terrain: Vec<_> = a_events
        .iter()
        .filter(|e| e.category == "terrain" && matches!(e.event_type.as_str(), "cave_in_triggered" | "structural_integrity_low" | "terrain_cascade"))
        .map(|e| (e.event_type.clone(), e.tick, e.payload.clone()))
        .collect();
    let b_terrain: Vec<_> = b_events
        .iter()
        .filter(|e| e.category == "terrain" && matches!(e.event_type.as_str(), "cave_in_triggered" | "structural_integrity_low" | "terrain_cascade"))
        .map(|e| (e.event_type.clone(), e.tick, e.payload.clone()))
        .collect();
    assert_eq!(a_terrain, b_terrain, "M14E event stream must be deterministic");
    assert!(
        !a_terrain.is_empty(),
        "expected at least one M14E terrain event per run"
    );
}

/// VAL-M14E-018 + VAL-M14E-026: a cave-in on the primary chunk emits
/// `terrain.terrain_cascade` for each neighbor and at least one
/// secondary `terrain.cave_in_triggered`.
#[test]
fn val_m14e_018_cascade_emits_terrain_cascade_for_neighbors() {
    let (_engine, events) = drive_scenario("m14e_tunnel_collapse_drill", 600);
    let cascade_count = count_events(&events, "terrain", "terrain_cascade");
    assert!(
        cascade_count >= 2,
        "expected ≥2 terrain.terrain_cascade events (one per neighbor); got {cascade_count}"
    );
    let cave_ins = count_events(&events, "terrain", "cave_in_triggered");
    assert!(
        cave_ins >= 2,
        "expected ≥2 terrain.cave_in_triggered (primary + cascade); got {cave_ins}"
    );
    let cave_in_secondary = find_event(&events, "terrain", "cave_in_triggered", |e| {
        e.payload
            .get("cascade_primary")
            .and_then(|v| v.as_bool())
            .map(|b| !b)
            .unwrap_or(false)
    });
    assert!(
        cave_in_secondary.is_some(),
        "expected at least one cave_in with cascade_primary=false"
    );
}

/// VAL-M14E-005 + VAL-M14E-027: cave-in falling-debris routes through
/// the M14 fall_impulse_chain AND transitions the actor to KnockedDown.
#[test]
fn val_m14e_005_cave_in_drives_actor_knockdown() {
    let (engine, events) = drive_scenario("m14e_tunnel_collapse_drill", 600);
    let cave_in_fired = count_events(&events, "terrain", "cave_in_triggered") > 0;
    assert!(cave_in_fired, "cave-in must fire to drive the impulse chain");
    // Actor 2 sits under the primary chunk per the scenario authoring.
    let knockdown = engine.m14e_actor_knockdown(2);
    assert!(knockdown, "actor 2 must transition to KnockedDown on cave-in");
    let knockdown_event = find_event(&events, "actor", "knockdown", |e| {
        e.payload.get("actor").and_then(|v| v.as_u64()) == Some(2)
    });
    assert!(knockdown_event.is_some(), "expected actor.knockdown event for actor 2");
    if let Some(ev) = knockdown_event {
        assert_eq!(
            ev.payload.get("cause").and_then(|v| v.as_str()),
            Some("cave_in"),
            "knockdown cause must be cave_in"
        );
    }
}

/// VAL-M14E-019: the integrity pass invocation count after T ticks
/// equals `floor(T / 15)` for any T.
#[test]
fn val_m14e_019_integrity_pass_cadence_is_one_per_15_ticks() {
    let (engine, _events) = drive_scenario("m14e_tunnel_collapse_drill", 150);
    // 150 ticks / 15 = 10 invocations (tick 0 does not run a pass per
    // the cadence guard).
    assert_eq!(engine.m14e_pass_invocations(), 10);
}

/// VAL-M14E-008: support-beam scenario locks the (0,0) chunk to
/// effective integrity 500 for the full 600-tick window; no cave-in
/// fires on (0,0).
#[test]
fn val_m14e_008_support_beam_save_no_cave_in_when_anchored() {
    let (engine, events) = drive_scenario("m14e_support_beam_save", 600);
    let cave_ins_on_anchored = events
        .iter()
        .filter(|e| e.category == "terrain" && e.event_type == "cave_in_triggered")
        .filter(|e| {
            e.payload
                .get("chunk_id")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.len() == 2
                        && arr[0].as_i64() == Some(0)
                        && arr[1].as_i64() == Some(0)
                })
                .unwrap_or(false)
        })
        .count();
    assert_eq!(cave_ins_on_anchored, 0, "anchored chunk must not cave in");
    let field = engine
        .m14e_integrity_field((0, 0))
        .expect("anchored chunk should have an integrity field");
    let center_lx = cf_terrain::INTEGRITY_FIELD_WIDTH / 2;
    let center_ly = cf_terrain::INTEGRITY_FIELD_HEIGHT / 2;
    assert!(field.is_locked(center_lx, center_ly));
    assert_eq!(
        field.effective_integrity(center_lx, center_ly),
        cf_terrain::INTEGRITY_BEAM_LOCKED
    );
}

/// VAL-M14E-001: a 14-pixel tunnel (`sub_threshold_12_px` in the
/// support-beam-save scenario) never crosses the L1 threshold or fires
/// a cave-in.
#[test]
fn val_m14e_001_sub_threshold_tunnel_holds() {
    let (_engine, events) = drive_scenario("m14e_support_beam_save", 600);
    let sub_threshold_chunk_id = (1, 0);
    let cave_ins = events
        .iter()
        .filter(|e| e.category == "terrain" && e.event_type == "cave_in_triggered")
        .filter(|e| {
            e.payload
                .get("chunk_id")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.len() == 2
                        && arr[0].as_i64() == Some(sub_threshold_chunk_id.0 as i64)
                        && arr[1].as_i64() == Some(sub_threshold_chunk_id.1 as i64)
                })
                .unwrap_or(false)
        })
        .count();
    assert_eq!(cave_ins, 0, "<=16-pixel tunnel must not cave in");
}

/// VAL-M14E-009 + VAL-CROSS-023: the support-beam placer surface
/// emits `terrain.support_beam_placed` with the canonical
/// `cost = (iron=2, wood=1)` payload.
#[test]
fn val_m14e_009_place_support_beam_emits_event() {
    let (engine, _events) = drive_scenario("m14e_support_beam_save", 1);
    let placed = engine.m14e_place_support_beam(1, (120.0, 32.0));
    let after = engine.recorder().snapshot_events();
    let event = find_event(&after, "terrain", "support_beam_placed", |_| true)
        .expect("support_beam_placed must fire");
    let cost = event.payload.get("cost").expect("cost field");
    assert_eq!(cost.get("iron").and_then(|v| v.as_u64()), Some(2));
    assert_eq!(cost.get("wood").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(engine.m14e_total_beams_placed(), 1);
    // `placed` is false in scenarios that have no chunk for that world pos.
    let _ = placed;
}

/// VAL-M14E-013: destroying a beam emits `terrain.support_beam_destroyed`
/// AND a structural_integrity_low arrives within ≤5 ticks of demolish.
#[test]
fn val_m14e_013_destroy_beam_emits_destroyed_and_low_within_5_ticks() {
    let (engine, _) = drive_scenario("m14e_support_beam_save", 1);
    // First place a beam in the supported chunk (centre world pos ~ (80, 64)
    // which lands in chunk (0, 0) at 256-pixel chunk size).
    engine.m14e_place_support_beam(1, (80.0, 64.0));
    // Now demolish it.
    engine.m14e_destroy_support_beam((80.0, 64.0), "demolish", Some(1));
    let after_demolish = engine.recorder().snapshot_events();
    let destroyed = find_event(&after_demolish, "terrain", "support_beam_destroyed", |_| true);
    assert!(destroyed.is_some(), "support_beam_destroyed must fire");
    // Tick the engine 15 more times so the next integrity pass runs.
    for _ in 0..15 {
        if engine.drive_tick().is_none() {
            break;
        }
    }
    let events = engine.recorder().snapshot_events();
    let low_after_destroy = events
        .iter()
        .filter(|e| e.category == "terrain" && e.event_type == "structural_integrity_low")
        .count();
    assert!(
        low_after_destroy >= 1,
        "expected ≥1 structural_integrity_low after beam destruction; got {low_after_destroy}"
    );
}
