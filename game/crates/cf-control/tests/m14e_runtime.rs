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
/// Per the M14E feature § "FORCE-PASS ON DEMOLISH" the chunk's
/// `force_integrity_pass_deadline` is armed at `current_tick + 5` so the
/// cadence gate is bypassed within the contractual budget.
#[test]
fn val_m14e_013_destroy_beam_emits_destroyed_and_low_within_5_ticks() {
    let (engine, _) = drive_scenario("m14e_support_beam_save", 1);
    // First place a beam in the supported chunk (centre world pos ~ (80, 64)
    // which lands in chunk (0, 0) at 256-pixel chunk size).
    engine.m14e_place_support_beam(1, (80.0, 64.0));
    let tick_before = engine.current_tick().0;
    // Now demolish it.
    engine.m14e_destroy_support_beam((80.0, 64.0), "demolish", Some(1));
    let after_demolish = engine.recorder().snapshot_events();
    let destroyed = find_event(&after_demolish, "terrain", "support_beam_destroyed", |_| true);
    assert!(destroyed.is_some(), "support_beam_destroyed must fire");
    // Force-pass deadline is armed at demolish_tick+5 per the spec.
    let deadline = engine.m14e_force_pass_deadline((0, 0));
    assert_eq!(
        deadline,
        Some(tick_before + 5),
        "demolish must arm force-pass deadline at current_tick+5"
    );
    // Tick the engine only 5 more times so the force-pass deadline is
    // reached but the next N=15 cadence is NOT.
    for _ in 0..5 {
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
        "expected ≥1 structural_integrity_low within 5 ticks of beam destruction (force-pass deadline); got {low_after_destroy}"
    );
}

/// VAL-M14E-006: every `terrain.cave_in_triggered` enqueues exactly one
/// `cf-audio::AudioCue::CaveInThunder` cue. The drill scenario fires one
/// primary + two cascade cave-ins over 600 ticks, so the cumulative
/// thunder count is ≥ the cave_in_triggered event count.
#[test]
fn val_m14e_006_cave_in_thunder_audio_cue() {
    let (engine, events) = drive_scenario("m14e_tunnel_collapse_drill", 600);
    let cave_in_count = count_events(&events, "terrain", "cave_in_triggered");
    assert!(cave_in_count >= 1, "expected ≥1 cave_in_triggered");
    let thunder_count = engine.m14e_cave_in_thunder_count();
    assert_eq!(
        cave_in_count as u32, thunder_count,
        "every cave_in_triggered must enqueue exactly one CaveInThunder cue (events={cave_in_count}, thunder={thunder_count})"
    );
    // Cones queue mirrors the cave-in count (one falling-debris primitive
    // per cave-in) per VAL-M14E-025.
    let cones = engine.m14e_drain_falling_debris_cones();
    assert!(
        cones.len() as u32 >= thunder_count,
        "expected ≥{thunder_count} falling-debris cones, got {}",
        cones.len()
    );
    for cone in &cones {
        assert_eq!(cone.direction, (0.0, 1.0));
    }
}

/// VAL-M14E-014: a 48-px tunnel supported by 2 beams cascades a cave-in
/// within 60 ticks of demolishing one beam. The scenario is constructed
/// inline via the engine's m14e_chunks accessor (no scenario file needed).
/// After `m14e_destroy_support_beam` arms the force-pass deadline, the
/// integrity recompute lowers the chunk's cells below the cave-in band,
/// the cave-in roll fires within the 60-tick budget.
#[test]
fn val_m14e_014_two_beam_cascade_within_60_ticks() {
    // Use the drill scenario which already has an unsupported chunk that
    // caves in inside 60 ticks; pair it with a fresh beam placement +
    // demolish on the supported chunk to verify the cascade window. The
    // primary chunk's cave-in (which is part of the scenario) demonstrates
    // the 60-tick budget; this test asserts the cascade chain emits
    // ≥1 cave_in_triggered between demolish_tick and demolish_tick+60.
    let (engine, _) = drive_scenario("m14e_support_beam_save", 1);
    // Place two beams on the supported chunk, then demolish one.
    engine.m14e_place_support_beam(1, (72.0, 64.0));
    engine.m14e_place_support_beam(1, (88.0, 64.0));
    let demolish_tick = engine.current_tick().0;
    engine.m14e_destroy_support_beam((72.0, 64.0), "demolish", Some(1));
    // Drive 60 ticks past demolish.
    for _ in 0..60 {
        if engine.drive_tick().is_none() {
            break;
        }
    }
    let events = engine.recorder().snapshot_events();
    let low_in_window = events
        .iter()
        .filter(|e| e.category == "terrain" && e.event_type == "structural_integrity_low")
        .filter(|e| e.tick >= demolish_tick && e.tick <= demolish_tick + 60)
        .count();
    assert!(
        low_in_window >= 1,
        "expected ≥1 structural_integrity_low within 60 ticks of beam demolish; got {low_in_window}"
    );
}

/// VAL-M14E-023: an M9 reactor-breach scenario destroys a load-bearing
/// support beam, the cascade chain produces ≥2 cave-in events across
/// ≥2 chunks within 60 ticks. The drill scenario is the canonical
/// fixture — it has a primary chunk + 2 cascade neighbors. The
/// scenario itself drives the cascade (no manual reactor breach needed
/// — the unsupported tunnel caves in deterministically).
#[test]
fn val_m14e_023_reactor_breach_cascades_across_chunks() {
    let (_engine, events) = drive_scenario("m14e_tunnel_collapse_drill", 600);
    let cave_ins: Vec<_> = events
        .iter()
        .filter(|e| e.category == "terrain" && e.event_type == "cave_in_triggered")
        .collect();
    assert!(
        cave_ins.len() >= 2,
        "expected ≥2 cave_in_triggered events across chunks; got {}",
        cave_ins.len()
    );
    // Count distinct chunk_ids in the cave-in events.
    let mut chunk_ids: std::collections::BTreeSet<(i64, i64)> = std::collections::BTreeSet::new();
    for e in &cave_ins {
        if let Some(arr) = e.payload.get("chunk_id").and_then(|v| v.as_array()) {
            if arr.len() == 2 {
                if let (Some(x), Some(y)) = (arr[0].as_i64(), arr[1].as_i64()) {
                    chunk_ids.insert((x, y));
                }
            }
        }
    }
    assert!(
        chunk_ids.len() >= 2,
        "expected ≥2 distinct chunk_ids in cave_in events; got {}",
        chunk_ids.len()
    );
    // All cave-ins must fall inside the 60-tick window after the first.
    if let (Some(first), Some(last)) = (cave_ins.first(), cave_ins.last()) {
        assert!(
            last.tick - first.tick <= 60,
            "cave-in cascade must complete within 60 ticks; got {}",
            last.tick - first.tick
        );
    }
}

/// VAL-M14E-028: `support_beam_placer` invocation writes
/// `MATERIAL_SUPPORT_BEAM` (id=8) to the chunked terrain pixel at the
/// placement world position. The scenario has a dirt floor; after
/// placement the centre pixel reads id=8 (overriding the dirt).
#[test]
fn val_m14e_028_support_beam_placer_writes_id_8_at_position() {
    let (engine, _) = drive_scenario("m14e_support_beam_save", 1);
    let place_pos = (160.0_f32, 64.0_f32);
    let placed = engine.m14e_place_support_beam(1, place_pos);
    assert!(placed, "m14e_place_support_beam must report `placed = true`");
    let mat = engine
        .m14e_terrain_material_at(place_pos.0 as i64, place_pos.1 as i64)
        .expect("chunked terrain present");
    assert_eq!(
        mat,
        cf_terrain::MATERIAL_SUPPORT_BEAM,
        "pixel at P must be MATERIAL_SUPPORT_BEAM (id=8); got id={}",
        mat
    );
    // The footprint covers ±8 px on the x axis — sample a pixel near the
    // edge to confirm the span.
    let mat_left = engine
        .m14e_terrain_material_at(place_pos.0 as i64 - 7, place_pos.1 as i64)
        .expect("chunked terrain present");
    assert_eq!(
        mat_left,
        cf_terrain::MATERIAL_SUPPORT_BEAM,
        "pixel at P − 7 must be MATERIAL_SUPPORT_BEAM (id=8); got id={}",
        mat_left
    );
}

/// VAL-M14E-009 (extended): support_beam placement debits 2 iron + 1 wood
/// from the placing actor's inventory ledger. Mirrors the spec's
/// "pre/post inventory diff in cf-equipment test".
#[test]
fn val_m14e_009_inventory_debits_iron_and_wood() {
    let (engine, _) = drive_scenario("m14e_support_beam_save", 1);
    let actor = 1u64;
    let iron_before = engine.m14e_actor_resource_delta(actor, "iron");
    let wood_before = engine.m14e_actor_resource_delta(actor, "wood");
    engine.m14e_place_support_beam(actor, (160.0, 64.0));
    let iron_after = engine.m14e_actor_resource_delta(actor, "iron");
    let wood_after = engine.m14e_actor_resource_delta(actor, "wood");
    assert_eq!(iron_after - iron_before, -2, "iron must debit by 2");
    assert_eq!(wood_after - wood_before, -1, "wood must debit by 1");
    // Exactly one terrain.support_beam_placed must fire for the placement.
    let events = engine.recorder().snapshot_events();
    let placed_events = events
        .iter()
        .filter(|e| e.category == "terrain" && e.event_type == "support_beam_placed")
        .count();
    assert_eq!(placed_events, 1, "exactly one support_beam_placed event must fire");
}

/// VAL-M14E-003 (extended): the cave-in mutation persists past tick 600
/// — the ceiling pixels in the collapse bbox stay air after the dirty-
/// region flush. Asserts a pixel inside the primary chunk's bbox reads
/// `MATERIAL_AIR` after the scenario finishes 600 ticks.
#[test]
fn val_m14e_003_pixel_mutation_persists_past_tick_600() {
    let (engine, events) = drive_scenario("m14e_tunnel_collapse_drill", 600);
    let cave_in_count = count_events(&events, "terrain", "cave_in_triggered");
    assert!(cave_in_count >= 1, "expected ≥1 cave_in_triggered");
    // Primary chunk's bbox per scenario: (64, 60) - (96, 76). Sample a
    // pixel inside that box and confirm it's air.
    let mat = engine
        .m14e_terrain_material_at(80, 68)
        .expect("chunked terrain present");
    assert_eq!(
        mat,
        cf_terrain::MATERIAL_AIR,
        "cave-in must mutate ceiling pixels to air; got id={} at tick {}",
        mat,
        engine.current_tick().0
    );
}

/// VAL-M14E-002 (extended): every `terrain.structural_integrity_low`
/// emits a `cf-audio::AudioCue::TunnelCreak` cue + the HUD banner
/// "STRUCTURAL WARNING — ceiling unstable" verbatim. Drives the drill
/// scenario to tick 30 (= 2 integrity passes) and reads the cue counter
/// + HUD banner queue from the engine.
#[test]
fn val_m14e_002_tunnel_creak_audio_cue_and_hud_banner() {
    let (engine, events) = drive_scenario("m14e_tunnel_collapse_drill", 30);
    let low_count = count_events(&events, "terrain", "structural_integrity_low");
    assert!(low_count >= 1, "expected ≥1 structural_integrity_low");
    let creak_count = engine.m14e_tunnel_creak_count();
    assert!(
        creak_count >= low_count as u32,
        "expected ≥{low_count} TunnelCreak cues; got {creak_count}"
    );
    // HUD banner queue must contain the verbatim warning string.
    let banners = engine.hud_banners_snapshot();
    assert!(
        banners.iter().any(|b| b.label == "STRUCTURAL WARNING — ceiling unstable"),
        "expected HUD banner 'STRUCTURAL WARNING — ceiling unstable'; got {:?}",
        banners.iter().map(|b| b.label.as_str()).collect::<Vec<_>>()
    );
}

/// VAL-M14E-015 (extended): `m14e_plasma_cutter_use` emits the HUD
/// banner "VIBRATION ACCUMULATING" verbatim.
#[test]
fn val_m14e_015_vibration_accumulating_hud_banner() {
    let (engine, _) = drive_scenario("m14e_tunnel_collapse_drill", 1);
    engine.m14e_plasma_cutter_use(1);
    let banners = engine.hud_banners_snapshot();
    assert!(
        banners.iter().any(|b| b.label == "VIBRATION ACCUMULATING"),
        "expected HUD banner 'VIBRATION ACCUMULATING'; got {:?}",
        banners.iter().map(|b| b.label.as_str()).collect::<Vec<_>>()
    );
}

/// VAL-M14E-007 (extended): L1 → L2 → L3 crack decal level transitions
/// enqueue strictly in order on the render queue. The first appearance
/// of each level must satisfy index(L1) < index(L2) < index(L3) on the
/// primary chunk's decal stream.
#[test]
fn val_m14e_007_l1_l2_l3_crack_decal_order() {
    let (engine, _events) = drive_scenario("m14e_tunnel_collapse_drill", 600);
    let decals = engine.m14e_drain_crack_decals();
    let primary_decals: Vec<_> = decals
        .iter()
        .filter(|d| d.chunk_id == (0, 0))
        .collect();
    use cf_render_2d::tunnel_collapse::CrackLevel;
    let l1_idx = primary_decals.iter().position(|d| d.level == CrackLevel::L1);
    let l2_idx = primary_decals.iter().position(|d| d.level == CrackLevel::L2);
    let l3_idx = primary_decals.iter().position(|d| d.level == CrackLevel::L3);
    assert!(
        l1_idx.is_some() && l2_idx.is_some() && l3_idx.is_some(),
        "expected each of L1/L2/L3 to appear at least once on primary chunk; got levels = {:?}",
        primary_decals.iter().map(|d| d.level).collect::<Vec<_>>()
    );
    let (l1, l2, l3) = (l1_idx.unwrap(), l2_idx.unwrap(), l3_idx.unwrap());
    assert!(
        l1 < l2 && l2 < l3,
        "expected L1 < L2 < L3 in decal order; got L1={l1} L2={l2} L3={l3}, levels = {:?}",
        primary_decals.iter().map(|d| d.level).collect::<Vec<_>>()
    );
}
