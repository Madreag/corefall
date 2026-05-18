//! **M14F § Runtime acceptance** — drives the three M14F scenarios
//! through `M0Engine::drive_tick`, asserts events emit, and exercises
//! the engine-side brace-strut placement + wall-event emit surfaces.
//!
//! Covers VAL-M14F-004 / -019 / -020 / -026 / -027 / VAL-CROSS-023.

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

/// VAL-M14F-020: all three M14F scenarios load + drive to 600 ticks
/// without panic.
#[test]
fn val_m14f_020_three_scenarios_drive_without_panic() {
    for scenario in [
        "m14f_vertical_mineshaft",
        "m14f_dam_pressure_test",
        "m14f_bunker_siege_wall_fail",
    ] {
        let (_engine, events) = drive_scenario(scenario, 600);
        // sanity: the recorder emitted at least the run_started event +
        // input.intent_received per tick (>= 600 events expected).
        assert!(
            events.len() > 100,
            "scenario {scenario} should emit > 100 events; got {}",
            events.len()
        );
    }
}

/// VAL-M14F-004 + VAL-CROSS-023: placing a brace-strut emits a
/// `terrain.brace_strut_placed` event with `tier`, `cost`, and a
/// `lock_radius_px` payload. The actor's iron + wood debits are
/// disjoint from the support-beam-placer slot.
#[test]
fn val_m14f_004_place_brace_strut_emits_event() {
    let (engine, _events) = drive_scenario("m14f_vertical_mineshaft", 1);
    let placed = engine.m14f_place_brace_strut(1, cf_equipment::BraceStrutTier::T1, (96.0, 64.0));
    assert!(placed);
    let after = engine.recorder().snapshot_events();
    let event = find_event(&after, "terrain", "brace_strut_placed", |_| true)
        .expect("brace_strut_placed must fire");
    let tier = event.payload.get("tier").and_then(|v| v.as_str());
    assert_eq!(tier, Some("t1"));
    let cost = event.payload.get("cost").expect("cost field");
    assert_eq!(cost.get("iron").and_then(|v| v.as_u64()), Some(2));
    assert_eq!(cost.get("wood").and_then(|v| v.as_u64()), Some(1));
    let lock_radius = event.payload.get("lock_radius_px").and_then(|v| v.as_u64());
    assert_eq!(lock_radius, Some(8));
    // VAL-CROSS-023: actor's iron debit is exactly -2.
    let iron_delta = engine.m14e_actor_resource_delta(1, "iron");
    let wood_delta = engine.m14e_actor_resource_delta(1, "wood");
    assert_eq!(iron_delta, -2, "iron delta must be exactly -2");
    assert_eq!(wood_delta, -1, "wood delta must be exactly -1");
}

/// VAL-M14F-031: T2/T3 brace-strut placements emit the corresponding
/// tier + lock_radius (12 / 16 px).
#[test]
fn val_m14f_031_t2_t3_brace_strut_emit_tier_and_radius() {
    let (engine, _) = drive_scenario("m14f_vertical_mineshaft", 1);
    let placed_t2 = engine.m14f_place_brace_strut(1, cf_equipment::BraceStrutTier::T2, (96.0, 64.0));
    let placed_t3 = engine.m14f_place_brace_strut(1, cf_equipment::BraceStrutTier::T3, (96.0, 64.0));
    assert!(placed_t2 && placed_t3);
    let events = engine.recorder().snapshot_events();
    let t2 = find_event(&events, "terrain", "brace_strut_placed", |e| {
        e.payload.get("tier").and_then(|v| v.as_str()) == Some("t2")
    })
    .expect("t2 placement event");
    let t3 = find_event(&events, "terrain", "brace_strut_placed", |e| {
        e.payload.get("tier").and_then(|v| v.as_str()) == Some("t3")
    })
    .expect("t3 placement event");
    assert_eq!(t2.payload.get("lock_radius_px").and_then(|v| v.as_u64()), Some(12));
    assert_eq!(t3.payload.get("lock_radius_px").and_then(|v| v.as_u64()), Some(16));
}

/// VAL-M14F-002 + VAL-M14F-003: emitting a wall_bulging event surfaces
/// the L1 crack decal + `MINESHAFT WALL UNSTABLE` HUD banner verbatim.
#[test]
fn val_m14f_002_wall_bulging_emit_drives_hud_banner_and_decal() {
    let (engine, _) = drive_scenario("m14f_vertical_mineshaft", 1);
    engine.m14f_emit_wall_bulging((0, 0), [64, 60], [88, 76], 24, 30);
    let events = engine.recorder().snapshot_events();
    let bulging = find_event(&events, "terrain", "wall_bulging", |_| true)
        .expect("wall_bulging event emitted");
    assert_eq!(bulging.payload.get("level").and_then(|v| v.as_str()), Some("l1"));
    assert_eq!(
        bulging.payload.get("unsupported_span_px").and_then(|v| v.as_u64()),
        Some(24)
    );
    let banners = engine.hud_banners_snapshot();
    assert!(
        banners.iter().any(|b| b.label == "MINESHAFT WALL UNSTABLE"),
        "expected MINESHAFT WALL UNSTABLE banner; got {banners:?}"
    );
    let decals = engine.m14e_drain_crack_decals();
    assert!(!decals.is_empty(), "expected L1 decal in render queue");
}

/// VAL-M14F-012 + VAL-M14F-025: wall_crack_advanced event emits the L2
/// payload + L2 decal.
#[test]
fn val_m14f_012_wall_crack_advanced_emits_l2_decal() {
    let (engine, _) = drive_scenario("m14f_vertical_mineshaft", 1);
    engine.m14f_emit_wall_crack_advanced((0, 0), [64, 60], [88, 76], 24, 30);
    let events = engine.recorder().snapshot_events();
    let crack = find_event(&events, "terrain", "wall_crack_advanced", |_| true)
        .expect("wall_crack_advanced event emitted");
    assert_eq!(crack.payload.get("level").and_then(|v| v.as_str()), Some("l2"));
    let decals = engine.m14e_drain_crack_decals();
    assert!(
        decals.iter().any(|d| matches!(d.level, cf_render_2d::tunnel_collapse::CrackLevel::L2)),
        "expected L2 decal in render queue"
    );
}

/// VAL-M14F-006 + VAL-M14F-027: wall_rupture event emits with the
/// three required payload fields chunk_id + bbox + falling_debris_count.
#[test]
fn val_m14f_006_wall_rupture_emits_required_fields() {
    let (engine, _) = drive_scenario("m14f_dam_pressure_test", 1);
    engine.m14f_emit_wall_rupture((1, 0), [256, 60], [288, 200], 32, 4, 50, "explosive_damage");
    let events = engine.recorder().snapshot_events();
    let rupture = find_event(&events, "terrain", "wall_rupture", |_| true)
        .expect("wall_rupture event emitted");
    assert!(rupture.payload.get("chunk_id").is_some());
    assert!(rupture.payload.get("bbox").is_some());
    let debris = rupture
        .payload
        .get("falling_debris_count")
        .and_then(|v| v.as_u64());
    assert!(debris.is_some() && debris.unwrap() > 0);
    assert_eq!(
        rupture.payload.get("trigger").and_then(|v| v.as_str()),
        Some("explosive_damage")
    );
}

/// VAL-M14F-019: emitted payloads round-trip the cf-replay schema
/// validator — assert the registry accepts every M14F event the
/// engine produces in a scenario drive.
#[test]
fn val_m14f_019_engine_payloads_validate_against_registered_schemas() {
    let (engine, _) = drive_scenario("m14f_vertical_mineshaft", 1);
    engine.m14f_place_brace_strut(1, cf_equipment::BraceStrutTier::T1, (96.0, 64.0));
    engine.m14f_emit_wall_bulging((0, 0), [64, 60], [88, 76], 24, 30);
    engine.m14f_emit_wall_crack_advanced((0, 0), [64, 60], [88, 76], 24, 30);
    engine.m14f_emit_wall_rupture((0, 0), [64, 60], [88, 76], 24, 4, 30, "integrity_decay");
    let events = engine.recorder().snapshot_events();
    let validate = |category: &str, event_type: &str| {
        let e = events
            .iter()
            .find(|ev| ev.category == category && ev.event_type == event_type)
            .unwrap_or_else(|| panic!("missing {category}.{event_type}"));
        let v = cf_replay::schemas::validate_event_payload(category, event_type, &e.payload);
        assert!(v.is_ok(), "{category}.{event_type} payload must validate: {v:?}\n{:?}", e.payload);
    };
    validate("terrain", "brace_strut_placed");
    validate("terrain", "wall_bulging");
    validate("terrain", "wall_crack_advanced");
    validate("terrain", "wall_rupture");
}

/// VAL-CROSS-023: placing a brace_strut does NOT touch the
/// support-beam-placer's `m14e_total_beams_placed` counter — distinct
/// slots, disjoint inventory deltas.
#[test]
fn val_cross_023_brace_strut_does_not_increment_support_beam_counter() {
    let (engine, _) = drive_scenario("m14f_vertical_mineshaft", 1);
    let beams_before = engine.m14e_total_beams_placed();
    engine.m14f_place_brace_strut(1, cf_equipment::BraceStrutTier::T1, (96.0, 64.0));
    let beams_after = engine.m14e_total_beams_placed();
    assert_eq!(beams_before, beams_after, "support_beam counter must not change on brace_strut placement");
}
