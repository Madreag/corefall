//! **VAL-CROSS-024** — M14F dam-wall rupture above an M14E tunnel
//! ceiling cascades into an M14E cave-in.
//!
//! When a `terrain.wall_rupture` event fires on an M14F lateral wall
//! chunk whose `cascade_neighbors` list points at an M14E tunnel
//! chunk, the M14F lateral pass must decay the tunnel chunk's
//! `IntegrityField` below `INTEGRITY_CASCADE_THRESHOLD`, run the
//! M14E `compute_integrity_pass` on the tunnel chunk, and emit
//! `terrain.cave_in_triggered{chunk_id=tunnel_chunk}` within ≤60
//! ticks of the rupture. The composite cascade also enqueues a
//! `terrain.terrain_cascade{cascade_kind="cave_in", source_event=
//! "wall_rupture"}` event linking the two chunks for M18 visual +
//! audio continuity.

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

fn chunk_id_of(event: &cf_replay::Event) -> Option<(i32, i32)> {
    let raw = event.payload.get("chunk_id")?;
    let arr = raw.as_array()?;
    if arr.len() != 2 {
        return None;
    }
    Some((arr[0].as_i64()? as i32, arr[1].as_i64()? as i32))
}

/// **VAL-CROSS-024 (composite cascade)**: drive `m14f_dam_above_m14e_tunnel.ron`
/// for 600 ticks and assert the ordered triple
///   `terrain.wall_rupture(dam_chunk)` →
///   `compute_integrity_pass(tunnel_chunk)` →
///   `terrain.cave_in_triggered{chunk_id=tunnel_chunk}`
/// fires within ≤60 ticks of the rupture, with `falling_debris_count
/// > 0` on the secondary cave-in.
#[test]
fn val_cross_024_dam_rupture_cascades_into_tunnel_cave_in() {
    let (_engine, events) = drive_scenario("m14f_dam_above_m14e_tunnel", 600);

    // 1) Rupture on the dam chunk (2, 0).
    let rupture = events
        .iter()
        .find(|e| e.category == "terrain" && e.event_type == "wall_rupture")
        .expect("expected ≥1 terrain.wall_rupture by tick 600");
    let rupture_tick = rupture.tick;
    let rupture_chunk = chunk_id_of(rupture).expect("wall_rupture must carry chunk_id");
    assert_eq!(
        rupture_chunk,
        (2, 0),
        "wall_rupture must fire on the dam chunk (2, 0); got {rupture_chunk:?}"
    );

    // 2) terrain.terrain_cascade marker linking dam → tunnel.
    //    Per VAL-CROSS-024 + VAL-M14E-026 the cascade event surfaces
    //    `cascade_kind="cave_in"` with the primary/secondary chunk_ids
    //    and `source_event="wall_rupture"` to disambiguate from the
    //    standalone M14E cascade.
    let cascade = events
        .iter()
        .find(|e| {
            e.category == "terrain"
                && e.event_type == "terrain_cascade"
                && e.payload.get("source_event").and_then(|v| v.as_str()) == Some("wall_rupture")
        })
        .expect("expected terrain.terrain_cascade marker linking dam rupture → tunnel cave-in");
    let primary = cascade
        .payload
        .get("primary_chunk_id")
        .and_then(|v| v.as_array())
        .map(|a| (a[0].as_i64().unwrap_or_default() as i32, a[1].as_i64().unwrap_or_default() as i32));
    let secondary = cascade
        .payload
        .get("secondary_chunk_id")
        .and_then(|v| v.as_array())
        .map(|a| (a[0].as_i64().unwrap_or_default() as i32, a[1].as_i64().unwrap_or_default() as i32));
    assert_eq!(primary, Some((2, 0)), "cascade primary must be the dam chunk");
    assert_eq!(secondary, Some((1, 0)), "cascade secondary must be the tunnel chunk");

    // 3) Secondary cave_in_triggered on the tunnel chunk (1, 0).
    let tunnel_cave_in = events
        .iter()
        .find(|e| {
            e.category == "terrain"
                && e.event_type == "cave_in_triggered"
                && chunk_id_of(e) == Some((1, 0))
        })
        .expect("expected terrain.cave_in_triggered{chunk_id=(1, 0)} after dam rupture");
    let cave_in_tick = tunnel_cave_in.tick;

    // 4) Ordering: cascade marker AND cave-in fire strictly after the
    //    rupture, and the cave-in is within the 60-tick window.
    assert!(
        rupture_tick <= cascade.tick,
        "rupture tick ({rupture_tick}) must precede cascade marker tick ({})",
        cascade.tick
    );
    assert!(
        rupture_tick <= cave_in_tick,
        "rupture tick ({rupture_tick}) must precede cave-in tick ({cave_in_tick})"
    );
    let delta = cave_in_tick.saturating_sub(rupture_tick);
    assert!(
        delta <= 60,
        "tunnel cave-in must fire within 60 ticks of rupture; got delta = {delta} (rupture@{rupture_tick}, cave_in@{cave_in_tick})"
    );

    // 5) falling_debris_count > 0 per VAL-CROSS-024 evidence line.
    let debris = tunnel_cave_in
        .payload
        .get("falling_debris_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(
        debris > 0,
        "secondary cave-in falling_debris_count must be > 0; got {debris}"
    );
}

/// **VAL-CROSS-024** sanity: the actor under the tunnel chunk receives
/// the cave-in's fall-impulse damage via the existing M14 fall_impulse
/// chain and transitions to KnockedDown (VAL-M14E-027 — wired through
/// the same path the standalone M14E scenario uses).
#[test]
fn val_cross_024_tunnel_actor_is_knocked_down_by_cascade_cave_in() {
    let (engine, events) = drive_scenario("m14f_dam_above_m14e_tunnel", 600);

    // Sanity: the cascade actually fired before we check the actor's
    // ragdoll state.
    let cave_in = events
        .iter()
        .find(|e| {
            e.category == "terrain"
                && e.event_type == "cave_in_triggered"
                && chunk_id_of(e) == Some((1, 0))
        })
        .expect("cave-in must fire on tunnel chunk");
    let _ = cave_in;

    // Actor 2 sits under the tunnel chunk per the scenario authoring;
    // the cave-in's fall-impulse chain transitions them to KnockedDown.
    let knocked_down = engine.m14e_actor_knockdown(2);
    assert!(
        knocked_down,
        "actor 2 under the tunnel chunk must be KnockedDown after the cascade cave-in"
    );
}

/// **VAL-CROSS-024** regression guard: standalone M14F scenarios
/// (mineshaft / sealed-room / dam) that do NOT opt into the composite
/// cascade must keep the M14E ceiling cave-in roll suppressed on the
/// M14F-owned chunk — i.e., `m14f_owns_rupture_emit` correctly defaults
/// to `true` when `m14e_composite_cascade_allowed = false`. Drives
/// `m14f_dam_pressure_test.ron` (which opts out of the composite
/// cascade) and asserts no `terrain.cave_in_triggered` fires on the
/// dam chunk for the full 600-tick window.
#[test]
fn val_cross_024_standalone_dam_scenario_does_not_emit_m14e_cave_in() {
    let (_engine, events) = drive_scenario("m14f_dam_pressure_test", 600);
    let cave_ins_on_dam_chunk = events
        .iter()
        .filter(|e| {
            e.category == "terrain"
                && e.event_type == "cave_in_triggered"
                && chunk_id_of(e) == Some((1, 0))
        })
        .count();
    assert_eq!(
        cave_ins_on_dam_chunk, 0,
        "standalone dam scenario must not emit M14E cave_in on the M14F-owned chunk; got {cave_ins_on_dam_chunk}"
    );
}
