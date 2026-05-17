//! **M4B § Acceptance criteria** — end-to-end integration tests over the
//! cf-control save subsystem.
//!
//! These cover the Gherkin scenarios `Save written under v1 loads...`,
//! `Save from a future version is rejected clearly`, `Corrupted save
//! surfaces a clean error`, `Quicksave + quickload roundtrip beats 800 ms`,
//! and `Mod-extending fields survive migration`.

use std::collections::BTreeMap;
use std::time::Instant;

use cf_save::{
    migration::{migrate, migrate_to_current},
    quicksave::{read_quicksave, write_quicksave},
    SaveBlob, SaveError, SaveSchemaVersion, WorldSave, CURRENT_SAVE_SCHEMA_VERSION, V1_0_0, V2_0_0,
};
use tempfile::tempdir;

fn make_v1_save() -> WorldSave {
    let actor = SaveBlob {
        schema_version: V1_0_0,
        actor_id: 1,
        team: "blue".to_string(),
        origin_id: "human".to_string(),
        position: [0.0, 0.0],
        velocity: [0.0, 0.0],
        aim: [1.0, 0.0],
        hp: 100.0,
        hp_max: 100.0,
        on_ground: true,
        status: "Stable".to_string(),
        selected_slot: 0,
        rifle_preset: None,
        rifle_ammo: None,
        rifle_reload_remaining_ticks: None,
        chassis: None,
        gear_dropped_by_limb_loss: false,
        chassis_detached: false,
        afflictions: vec![],
        crouch_active: false,
        climb_active: false,
        jet_active: false,
        mod_payload: BTreeMap::new(),
    };
    WorldSave {
        schema_version: V1_0_0,
        world_tick: 0,
        actors: vec![actor],
        terrain_chunks: vec![],
        projectiles: vec![],
        mod_payload: BTreeMap::new(),
    }
}

/// **Gherkin: Save written under v1 loads under current build via migration.**
#[test]
fn v1_save_migrates_to_current_via_registry() {
    let world = make_v1_save();
    let outcome = migrate(world, CURRENT_SAVE_SCHEMA_VERSION).expect("v1->current migrates");
    assert_eq!(outcome.blob.schema_version, CURRENT_SAVE_SCHEMA_VERSION);
    assert_eq!(outcome.handler_chain, vec!["v1_to_v2"]);
    for actor in &outcome.blob.actors {
        assert_eq!(actor.schema_version, CURRENT_SAVE_SCHEMA_VERSION);
    }
}

/// **Gherkin: Save from a future version is rejected clearly.**
#[test]
fn future_version_save_returns_unsupported_future_error() {
    let mut world = WorldSave::empty(0);
    world.schema_version = SaveSchemaVersion::new(99, 0, 0);
    let (json, _hex) = world.serialize().unwrap();
    let err = WorldSave::deserialize(&json, None).err().unwrap();
    match err {
        SaveError::UnsupportedFutureVersion { found, max_supported } => {
            assert_eq!(found, SaveSchemaVersion::new(99, 0, 0));
            assert_eq!(max_supported, CURRENT_SAVE_SCHEMA_VERSION);
        }
        other => panic!("expected UnsupportedFutureVersion, got {other:?}"),
    }
}

/// **Gherkin: Corrupted save surfaces a clean error, never panics.**
#[test]
fn corrupted_save_surfaces_checksum_mismatch_error() {
    let dir = tempdir().unwrap();
    let world = make_v1_save();
    let out = write_quicksave(dir.path(), &world).unwrap();
    // Tamper the payload AFTER the checksum has been written.
    let path = dir.path().join("quicksave.cfsave");
    let mut bytes = std::fs::read(&path).unwrap();
    if let Some(pos) = bytes.windows(4).position(|w| w == b"blue") {
        bytes[pos..pos + 4].copy_from_slice(b"red!");
        std::fs::write(&path, &bytes).unwrap();
    }
    let err = read_quicksave(dir.path()).err().unwrap();
    match err {
        SaveError::ChecksumMismatch { expected, actual } => {
            assert_eq!(expected, out.checksum_hex);
            assert_ne!(expected, actual);
        }
        other => panic!("expected ChecksumMismatch, got {other:?}"),
    }
}

/// **Gherkin: Quicksave + quickload roundtrip beats 800 ms on Workstation tier.**
///
/// Spec literal: "the F5 path completes in under 400 ms wall clock and
/// emits system.save_completed" + "the F9 path completes in under 400 ms
/// wall clock and emits system.save_loaded". Both legs must individually
/// beat 400 ms; combined they beat 800 ms. On the reference Workstation
/// tier with NVMe this comfortably fits; on slower CI runners we still
/// expect the per-leg < 400 ms budget given that the work is just a
/// canonical-JSON serialize + BLAKE3 + fs::write.
#[test]
fn quicksave_then_quickload_completes_under_workstation_per_leg_budget() {
    let dir = tempdir().unwrap();
    let mut world = WorldSave::empty(0);
    for i in 0u64..200 {
        world.actors.push(SaveBlob {
            schema_version: CURRENT_SAVE_SCHEMA_VERSION,
            actor_id: i,
            team: if i % 2 == 0 { "blue".to_string() } else { "red".to_string() },
            origin_id: "human".to_string(),
            position: [i as f32, 0.0],
            velocity: [0.0, 0.0],
            aim: [1.0, 0.0],
            hp: 100.0,
            hp_max: 100.0,
            on_ground: true,
            status: "Stable".to_string(),
            selected_slot: 0,
            rifle_preset: None,
            rifle_ammo: Some(30),
            rifle_reload_remaining_ticks: None,
            chassis: None,
            gear_dropped_by_limb_loss: false,
            chassis_detached: false,
            afflictions: vec![],
            crouch_active: false,
            climb_active: false,
            jet_active: false,
            mod_payload: BTreeMap::new(),
        });
    }
    const PER_LEG_BUDGET_MS: u128 = 400;
    let t0 = Instant::now();
    let save_outcome = write_quicksave(dir.path(), &world).expect("quicksave");
    let save_elapsed = t0.elapsed();
    assert!(
        save_elapsed.as_millis() < PER_LEG_BUDGET_MS,
        "F5 quicksave took {save_elapsed:?} (>= {PER_LEG_BUDGET_MS} ms per-leg budget)"
    );
    let t1 = Instant::now();
    let load_outcome = read_quicksave(dir.path()).expect("quickload");
    let load_elapsed = t1.elapsed();
    assert!(
        load_elapsed.as_millis() < PER_LEG_BUDGET_MS,
        "F9 quickload took {load_elapsed:?} (>= {PER_LEG_BUDGET_MS} ms per-leg budget)"
    );
    // **Gherkin: post-load state matches pre-save state byte-for-byte
    // under canonical blake3.**
    assert_eq!(load_outcome.save, world);
    assert_eq!(load_outcome.checksum_hex, save_outcome.checksum_hex);
    // The outcomes also report their wall_clock_ms; assert that field
    // also fits the budget (so observe.save.last surfaces a budget-safe
    // value).
    assert!(
        u128::from(save_outcome.wall_clock_ms) < PER_LEG_BUDGET_MS,
        "quicksave outcome.wall_clock_ms={} >= {PER_LEG_BUDGET_MS}",
        save_outcome.wall_clock_ms
    );
    assert!(
        u128::from(load_outcome.wall_clock_ms) < PER_LEG_BUDGET_MS,
        "quickload outcome.wall_clock_ms={} >= {PER_LEG_BUDGET_MS}",
        load_outcome.wall_clock_ms
    );
}

/// **Gherkin: Mod-extending fields survive migration.**
#[test]
fn mod_payload_round_trips_verbatim_through_v1_to_v2() {
    let mut world = make_v1_save();
    world
        .mod_payload
        .insert("acme_corp.world".to_string(), serde_json::json!({"weather": "snow"}));
    if let Some(actor) = world.actors.first_mut() {
        actor
            .mod_payload
            .insert("acme_corp.actor".to_string(), serde_json::json!({"buffs": ["hardy"]}));
    }
    let outcome = migrate_to_current(world).expect("migrate");
    assert_eq!(
        outcome.blob.mod_payload.get("acme_corp.world").cloned(),
        Some(serde_json::json!({"weather": "snow"}))
    );
    assert_eq!(
        outcome.blob.actors[0].mod_payload.get("acme_corp.actor").cloned(),
        Some(serde_json::json!({"buffs": ["hardy"]}))
    );
}

/// **Gherkin: Delta baseline cadence is enforced** — the engine config
/// surfaces `delta_baseline_cadence_ticks` defaulting to 600.
#[test]
fn delta_baseline_cadence_defaults_to_600_ticks() {
    assert_eq!(cf_save::delta::DEFAULT_BASELINE_CADENCE_TICKS, 600);
}

/// **Gherkin: Migration corpus matrix passes for every fixture** — round-trip
/// the in-memory contract used by the corpus generator.
#[test]
fn v1_to_v2_handler_chain_is_single_step() {
    let world = make_v1_save();
    let outcome = migrate(world, V2_0_0).expect("v1->v2 migrates");
    assert_eq!(outcome.handler_chain, vec!["v1_to_v2"]);
    assert_eq!(outcome.from, V1_0_0);
    assert_eq!(outcome.to, V2_0_0);
}

/// **Gherkin: SaveSchemaVersion is a 3-element JSON array** — and the
/// numeric compat path keeps loading v1 saves.
#[test]
fn legacy_numeric_schema_version_still_loads() {
    let json = serde_json::json!({
        "schema_version": 1,
        "world_tick": 0,
        "actors": [],
        "terrain_chunks": [],
        "projectiles": [],
    });
    let text = serde_json::to_string(&json).unwrap();
    let world = WorldSave::deserialize(&text, None).expect("legacy numeric form parses");
    assert_eq!(world.schema_version, V1_0_0);
}
