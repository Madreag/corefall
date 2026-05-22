//! Tests for engine.rs (moved here so engine.rs stays under 2000 LOC).
#[allow(unused_imports)]
use crate::engine::*;
#[allow(unused_imports)]
use crate::engine_helpers::*;
#[allow(unused_imports)]
use crate::server::*;
#[allow(unused_imports)]
use crate::server_command::*;
#[allow(unused_imports)]
use crate::server_engine_handle::*;
#[allow(unused_imports)]
use crate::state::*;
#[allow(unused_imports)]
use crate::{Settings, SCHEMA_VERSION, SCHEMA_VERSION_MIN};
#[allow(unused_imports)]
use cf_actor::{ActorId, ControlIntent, IntentSource};
#[allow(unused_imports)]
use cf_sim_core::{Tick, WallClock};
#[allow(unused_imports)]
use std::path::{Path, PathBuf};
#[allow(unused_imports)]
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;
    use super::*;
    fn write_m1_scenario() -> PathBuf {
        let mut p = std::env::temp_dir();
        let seq = TEST_SCENARIO_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        p.push(format!("m1_actor_range_{}_{}.ron", std::process::id(), seq));
        std::fs::write(
            &p,
            r#"(
  schema_version: 1,
  id: "m1_actor_range",
  display_name: "M1 Actor Range",
  description: "M1 engine test fixture.",
  seed: 7,
  duration_ticks: Some(120),
  region: (anchor: (0.0, 0.0), width: 1280.0, height: 720.0),
  gravity: -980.0,
  floor_y: 16.0,
  teams: [],
  actors: [
    (id: 1, team: "blue", spawn: (200.0, 32.0), controllable: true, hp: 100.0,
      inventory: (rifle: Some("rifle_m1_default")), half_extents: Some((8.0, 16.0))),
    (id: 2, team: "red", spawn: (900.0, 32.0), controllable: false, hp: 100.0,
      inventory: (rifle: None)),
  ],
  objectives: [],
  director: None,
  capabilities: (debug: false, control_api: true, save_load: false),
  save_fields: [],
  expected_tests: ["M1-SMOKE-01"],
  notes: "",
)"#,
        )
        .unwrap();
        p
    }

    fn load_m1_test_config(path: PathBuf) -> M0EngineConfig {
        let scenario = crate::scenario::Scenario::load_from_file(&path).unwrap();
        M0EngineConfig::for_loaded_scenario(&scenario, path)
    }

    static TEST_SCENARIO_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn temp_run_root() -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("cf_engine_test_{}_{}", std::process::id(), uuid_like()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn uuid_like() -> String {
        let now = WallClock.now_utc();
        format!("{}", now.timestamp_nanos_opt().unwrap_or_default())
    }

    fn write_test_scenario() -> PathBuf {
        let mut p = std::env::temp_dir();
        let seq = TEST_SCENARIO_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        p.push(format!("m0_blank_{}_{}.ron", std::process::id(), seq));
        std::fs::write(
            &p,
            r#"(
  schema_version: 1,
  id: "m0_blank",
  display_name: "M0 Blank Scene",
  description: "Empty scene used for engine bootstrap and run-bundle smoke.",
  seed: 42,
  duration_ticks: Some(60),
  region: (anchor: (0.0, 0.0), width: 1280.0, height: 720.0),
  gravity: -980.0,
  teams: [],
  actors: [],
  objectives: [],
  director: None,
  capabilities: (
    debug: false,
    control_api: true,
    save_load: false,
  ),
  save_fields: [],
  expected_tests: ["M0-SMOKE-01"],
  notes: "",
)"#,
        )
        .unwrap();
        p
    }

    fn load_test_scenario_and_config(path: PathBuf) -> M0EngineConfig {
        let scenario = crate::scenario::Scenario::load_from_file(&path).unwrap();
        M0EngineConfig::for_loaded_scenario(&scenario, path)
    }

    #[tokio::test]
    async fn m1_act_player_move_rejects_nonfinite_at_engine_layer() {
        // Defense-in-depth mirror for act.player.move (cf-app's keyboard bridge produces
        // 0.0 / ±1.0 today, but a future mouse / gamepad / scripted bridge could send a
        // NaN/Inf move axis through engine.dispatch directly).
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();
        for (x, y) in [
            (f32::NAN, 0.0),
            (0.0, f32::NAN),
            (f32::INFINITY, 0.0),
            (0.0, f32::NEG_INFINITY),
        ] {
            let result = engine
                .dispatch(ControlCommand::ActPlayerMove {
                    x,
                    y,
                    source: IntentSource::Cfctl,
                })
                .await;
            assert_eq!(
                result.status,
                crate::state::ControlEnvelopeStatus::Rejected,
                "move ({x}, {y}) must reject"
            );
            assert_eq!(result.reason.as_deref(), Some("non_finite"));
        }
    }

    #[tokio::test]
    async fn m1_kill_chain_records_actor_status_changed_with_projectile_hit_cause() {
        // M1-D04 end-to-end evidence via the dispatch path: drive the engine through
        // act.player.aim + act.player.fire enough times to kill the dummy, then assert
        // the recorder captured an actor.actor_status_changed event with cause
        // "projectile_hit". Engine + sim test `projectile_eventually_hits_dummy_and_can_kill_it`
        // already proves the underlying physics; this test adds the dispatch + event
        // emission proof.
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();
        // Settle to ground first.
        for _ in 0..10 {
            engine.drive_tick();
        }
        let _ = engine
            .dispatch(ControlCommand::ActPlayerAim {
                x: 1.0,
                y: 0.0,
                source: IntentSource::Cfctl,
            })
            .await;
        // Fire 9 shots (dummy has 100 HP, rifle 12 dmg/hit → 9 hits = 108 dmg). Each shot
        // requires the rifle's fire interval (6 ticks) to cool down between presses.
        let fire_interval_ticks = cf_equipment::rifle_preset(cf_equipment::RIFLE_M1_DEFAULT_ID)
            .unwrap()
            .fire_interval_ticks(60) as usize;
        for _ in 0..12 {
            let _ = engine
                .dispatch(ControlCommand::ActPlayerFire {
                    pressed: true,
                    ammo_kind: None,
                    source: IntentSource::Cfctl,
                })
                .await;
            // Drive enough ticks for the fired projectile to reach the dummy at x=900
            // before the next shot (player at x=200, projectile speed 1200 unit/s ≈ 20
            // unit/tick at 60 Hz → 35 ticks to cross 700 units).
            for _ in 0..fire_interval_ticks.max(35) {
                engine.drive_tick();
            }
            // Release the trigger so the Semi rifle latch clears and the next
            // pressed:true can fire (M1 default rifle is Semi).
            let _ = engine
                .dispatch(ControlCommand::ActPlayerFire {
                    pressed: false,
                    ammo_kind: None,
                    source: IntentSource::Cfctl,
                })
                .await;
            engine.drive_tick();
        }
        let events = engine.recorder().snapshot_events();
        // CCCP Actor.cpp:1229 — HP=0 enters DYING (not DEAD); the DEAD
        // transition fires later when the dwell elapses. Accept either as
        // proof the projectile_hit cause-chain reached the terminal status.
        let kill_event = events.iter().find(|e| {
            e.category == "actor"
                && e.event_type == "actor_status_changed"
                && (e.payload["new_status"] == "dying" || e.payload["new_status"] == "dead")
                && e.payload["cause"] == "projectile_hit"
        });
        assert!(
            kill_event.is_some(),
            "expected a projectile_hit-caused dying/dead status transition; got events: {:?}",
            events
                .iter()
                .filter(|e| e.event_type == "actor_status_changed")
                .map(|e| e.payload.clone())
                .collect::<Vec<_>>()
        );
    }

    /// **Enhancement D2**: in-process cross-run determinism. Drive the engine
    /// twice with the same seed + same script and assert the final
    /// determinism checksum hex strings match byte-for-byte.
    #[tokio::test]
    async fn cross_run_determinism_same_seed_same_final_checksum() {
        async fn drive_run() -> Option<String> {
            let path = write_m1_scenario();
            let config = load_m1_test_config(path);
            let engine = M0Engine::new(config);
            engine.record_run_started();
            // Settle to ground.
            for _ in 0..6 {
                engine.drive_tick();
            }
            let _ = engine
                .dispatch(ControlCommand::ActPlayerAim {
                    x: 1.0,
                    y: 0.0,
                    source: IntentSource::Cfctl,
                })
                .await;
            // Fire/release a handful of shots to exercise the cause chain.
            for _ in 0..3 {
                let _ = engine
                    .dispatch(ControlCommand::ActPlayerFire {
                        pressed: true,
                        ammo_kind: None,
                        source: IntentSource::Cfctl,
                    })
                    .await;
                for _ in 0..12 {
                    engine.drive_tick();
                }
                let _ = engine
                    .dispatch(ControlCommand::ActPlayerFire {
                        pressed: false,
                        ammo_kind: None,
                        source: IntentSource::Cfctl,
                    })
                    .await;
                engine.drive_tick();
            }
            for _ in 0..120 {
                engine.drive_tick();
            }
            engine.recorder().final_checksum_hex()
        }
        let cs_a = drive_run().await.expect("run a produced a checksum");
        let cs_b = drive_run().await.expect("run b produced a checksum");
        assert_eq!(
            cs_a, cs_b,
            "cross-run determinism: same seed + same script must produce byte-identical final sim checksum"
        );
    }

    /// **Gap C4**: walk parent_event_id from `actor.inventory_dropped` back to
    /// the root `input.intent_received`. Every link must resolve to a real
    /// recorded event id (no `ParentMissingFromBundle`). The expected chain:
    ///   inventory_dropped -> status_changed(DYING) -> projectile_hit
    ///     -> projectile_spawned -> weapon_fired -> input.intent_received
    #[tokio::test]
    async fn cause_chain_walks_from_inventory_dropped_to_intent() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();
        for _ in 0..10 {
            engine.drive_tick();
        }
        let _ = engine
            .dispatch(ControlCommand::ActPlayerAim {
                x: 1.0,
                y: 0.0,
                source: IntentSource::Cfctl,
            })
            .await;
        let fire_interval_ticks = cf_equipment::rifle_preset(cf_equipment::RIFLE_M1_DEFAULT_ID)
            .unwrap()
            .fire_interval_ticks(60) as usize;
        // Kill the dummy (100 HP / 12 dmg => 9 hits + buffer).
        for _ in 0..12 {
            let _ = engine
                .dispatch(ControlCommand::ActPlayerFire {
                    pressed: true,
                    ammo_kind: None,
                    source: IntentSource::Cfctl,
                })
                .await;
            for _ in 0..fire_interval_ticks.max(35) {
                engine.drive_tick();
            }
            let _ = engine
                .dispatch(ControlCommand::ActPlayerFire {
                    pressed: false,
                    ammo_kind: None,
                    source: IntentSource::Cfctl,
                })
                .await;
            engine.drive_tick();
        }
        // Let the DYING dwell elapse so inventory_dropped + DEAD chain emit.
        for _ in 0..120 {
            engine.drive_tick();
        }
        let events = engine.recorder().snapshot_events();
        // Build id -> event lookup for the walk.
        let by_id: std::collections::BTreeMap<String, &cf_replay::Event> =
            events.iter().map(|e| (e.event_id.clone(), e)).collect();
        // Find the inventory_dropped for the dummy (actor_id 2).
        let drop_event = events.iter().find(|e| {
            e.category == "actor"
                && e.event_type == "inventory_dropped"
                && e.payload.get("actor").and_then(|v| v.as_u64()) == Some(2)
        });
        // The dummy carries no rifle in m1_actor_range (its inventory.rifle: None),
        // so the inventory_dropped event may not fire (label="empty"). In that
        // case the chain test still has value via status_changed(DYING).
        let chain_root = drop_event.or_else(|| {
            events.iter().find(|e| {
                e.category == "actor"
                    && e.event_type == "actor_status_changed"
                    && e.payload.get("new_status").and_then(|v| v.as_str()) == Some("dying")
                    && e.payload.get("actor").and_then(|v| v.as_u64()) == Some(2)
            })
        });
        let chain_root = chain_root.expect("must find inventory_dropped OR status_changed(DYING) for actor 2");
        // Walk the parent_event_id chain.
        let mut chain_types: Vec<String> = Vec::new();
        let mut current = chain_root;
        chain_types.push(format!("{}.{}", current.category, current.event_type));
        let mut walked = 0;
        while let Some(parent_id) = current.parent_event_id.clone() {
            walked += 1;
            assert!(walked < 50, "chain walk runaway (events={:?})", chain_types);
            let parent = by_id
                .get(&parent_id)
                .unwrap_or_else(|| panic!("ParentMissingFromBundle: parent_id={parent_id} not in run"));
            chain_types.push(format!("{}.{}", parent.category, parent.event_type));
            current = parent;
        }
        // The walk must terminate at an input.intent_received root.
        let terminal = chain_types.last().expect("chain must have at least one link").clone();
        assert!(
            terminal == "input.intent_received",
            "cause chain must terminate at input.intent_received; got chain: {:?}",
            chain_types
        );
        // The chain must include projectile_hit and weapon_fired links.
        assert!(
            chain_types.iter().any(|s| s == "combat.projectile_hit"),
            "chain missing combat.projectile_hit: {chain_types:?}",
        );
        assert!(
            chain_types.iter().any(|s| s == "equipment.weapon_fired"),
            "chain missing equipment.weapon_fired: {chain_types:?}",
        );
    }

    // --- M3 re-open (2026-05-13): coalesce-logic regression tests ---

    #[test]
    fn rects_touch_or_overlap_detects_shared_edge() {
        // Two CHUNK_SIZE × CHUNK_SIZE rects sitting edge-to-edge along x.
        // Chunk (0,0) occupies [0,0..256] and chunk (1,0) occupies [256,0..512].
        // The shared edge at x=256 means the AABBs touch.
        let a_min = [0i64, 0i64];
        let a_max = [256i64, 256i64];
        let b_min = [256i64, 0i64];
        let b_max = [512i64, 256i64];
        assert!(rects_touch_or_overlap(a_min, a_max, b_min, b_max));
    }

    #[test]
    fn rects_touch_or_overlap_detects_diagonal_neighbor() {
        // Corner-touching rects (diagonal). a.max == b.min for both axes.
        // The greedy coalescer treats this as touching so the union covers
        // both chunks in one pass.
        let a_min = [0i64, 0i64];
        let a_max = [256i64, 256i64];
        let b_min = [256i64, 256i64];
        let b_max = [512i64, 512i64];
        assert!(rects_touch_or_overlap(a_min, a_max, b_min, b_max));
    }

    #[test]
    fn rects_touch_or_overlap_rejects_disjoint() {
        // A gap of 10 pixels between rects → no overlap → coalesce keeps
        // them as separate batch entries.
        let a_min = [0i64, 0i64];
        let a_max = [256i64, 256i64];
        let b_min = [266i64, 0i64];
        let b_max = [522i64, 256i64];
        assert!(!rects_touch_or_overlap(a_min, a_max, b_min, b_max));
    }

    #[test]
    fn rects_touch_or_overlap_detects_interior_overlap() {
        // A rect fully contained inside another.
        let a_min = [0i64, 0i64];
        let a_max = [256i64, 256i64];
        let b_min = [100i64, 100i64];
        let b_max = [120i64, 120i64];
        assert!(rects_touch_or_overlap(a_min, a_max, b_min, b_max));
    }

    #[tokio::test]
    async fn m6_sprint_drains_stamina_and_auto_cancels() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();

        let result = engine
            .dispatch(ControlCommand::ActM6 {
                action: crate::m6_actions::M6Action::Sprint { active: true },
                source: IntentSource::Cfctl,
            })
            .await;
        assert_eq!(result.status, crate::state::ControlEnvelopeStatus::Accepted);

        for _ in 0..(5 * 60 + 2) {
            engine.drive_tick();
        }

        let events = engine.recorder().snapshot_events();
        assert!(
            events
                .iter()
                .any(|e| e.category == "actor" && e.event_type == "stamina_changed"),
            "actor.stamina_changed must be emitted as stamina drains"
        );
        let state = engine.state.read().expect("engine state poisoned");
        let sim = state.actor_state.as_ref().expect("actor sim present");
        let actor = sim.world.actors.get(&ActorId(1)).expect("player actor present");
        assert!(
            actor.stamina.current <= 0.01,
            "after 5s sprint stamina must drain to ~0: {}",
            actor.stamina.current
        );
        assert!(!actor.sprint_active, "sprint must auto-cancel at zero stamina");
    }

    #[tokio::test]
    async fn m6_cinematic_slide_transitions_back_to_crouch() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();

        engine
            .dispatch(ControlCommand::ActM6 {
                action: crate::m6_actions::M6Action::Sprint { active: true },
                source: IntentSource::Cfctl,
            })
            .await;
        engine
            .dispatch(ControlCommand::ActM6 {
                action: crate::m6_actions::M6Action::Slide,
                source: IntentSource::Cfctl,
            })
            .await;

        for _ in 0..40 {
            engine.drive_tick();
        }

        let events = engine.recorder().snapshot_events();
        let stance_changed = events
            .iter()
            .find(|e| {
                e.category == "actor"
                    && e.event_type == "stance_changed"
                    && e.payload
                        .get("cause")
                        .and_then(|v| v.as_str())
                        .map(|s| s == "cinematic_complete")
                        .unwrap_or(false)
            })
            .expect("actor.stance_changed must fire when slide finishes");
        let to_stance = stance_changed
            .payload
            .get("to_stance")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert_eq!(to_stance, "crouching", "slide must transition to crouch");

        let state = engine.state.read().expect("engine state poisoned");
        let sim = state.actor_state.as_ref().expect("actor sim present");
        let actor = sim.world.actors.get(&ActorId(1)).expect("player actor present");
        assert_eq!(actor.cinematic_ticks_remaining, 0);
        assert!(actor.cinematic_kind.is_none());
    }

    #[tokio::test]
    async fn m6_lean_angle_approaches_target_over_time() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();

        engine
            .dispatch(ControlCommand::ActM6 {
                action: crate::m6_actions::M6Action::Lean { direction: 1.0 },
                source: IntentSource::Cfctl,
            })
            .await;

        for _ in 0..120 {
            engine.drive_tick();
        }

        let state = engine.state.read().expect("engine state poisoned");
        let sim = state.actor_state.as_ref().expect("actor sim present");
        let actor = sim.world.actors.get(&ActorId(1)).expect("player actor present");
        assert!(
            actor.lean_state.angle_degrees >= 40.0,
            "lean angle must approach +45° (got {})",
            actor.lean_state.angle_degrees
        );
    }

    #[tokio::test]
    async fn m6_weapon_swap_completes_after_300ms() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();

        engine
            .dispatch(ControlCommand::ActM6 {
                action: crate::m6_actions::M6Action::WeaponSwap { slot: 1 },
                source: IntentSource::Cfctl,
            })
            .await;

        for _ in 0..30 {
            engine.drive_tick();
        }

        let events = engine.recorder().snapshot_events();
        assert!(
            events
                .iter()
                .any(|e| e.category == "equipment" && e.event_type == "weapon_swap_started"),
            "weapon_swap_started must fire when swap is requested"
        );
        assert!(
            events
                .iter()
                .any(|e| e.category == "equipment" && e.event_type == "weapon_swap_completed"),
            "weapon_swap_completed must fire after 300ms tick path: {:?}",
            events
                .iter()
                .map(|e| (e.category.clone(), e.event_type.clone()))
                .collect::<Vec<_>>()
        );
    }

    /// Full engine round trip: a chest at depth-1 holding a crate at
    /// depth-2; attempting to nest a third container into the crate
    /// rejects with the spec-locked `max_depth_exceeded` reason and
    /// emits `actor.action_rejected` (no `inventory.container_nested`
    /// fires for the rejection).
    #[tokio::test]
    async fn m6b_nest_container_engine_rejects_max_depth() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();

        // Seed the player actor's grid with a chest (top-level) +
        // crate (nested into chest at depth 2).
        let chest_id;
        let crate_id;
        {
            let mut state = engine.state.write().unwrap();
            let player_id = state.player_actor.unwrap();
            let actor = state
                .actor_state
                .as_mut()
                .unwrap()
                .world
                .actors
                .get_mut(&player_id)
                .unwrap();
            actor.inventory_grid_attach();
            let grid = actor.inventory_grid_mut().unwrap();
            chest_id = grid.add_top_level("chest", 1, 0.0);
            crate_id = grid.try_nest_container(chest_id, "crate").unwrap();
        }
        engine.drive_tick();

        // Step 1: nest another container (crate) into the crate. This
        // would land at depth 3 = MAX_CONTAINER_NEST_DEPTH+1; the
        // dispatch returns Rejected with the locked reason.
        let result_rejected = engine
            .dispatch(ControlCommand::ActM6 {
                action: crate::m6_actions::M6Action::NestContainer {
                    parent_instance_id: crate_id,
                    child_item_id: "crate".to_string(),
                },
                source: IntentSource::Cfctl,
            })
            .await;
        assert_eq!(result_rejected.status, crate::state::ControlEnvelopeStatus::Rejected);
        assert_eq!(
            result_rejected.reason.as_deref(),
            Some(cf_equipment::MAX_DEPTH_EXCEEDED)
        );

        // Step 2: nest a medkit into the crate. Non-container child at
        // depth 3 is allowed (depth cap only constrains containers).
        let result_accepted = engine
            .dispatch(ControlCommand::ActM6 {
                action: crate::m6_actions::M6Action::NestContainer {
                    parent_instance_id: crate_id,
                    child_item_id: "medkit".to_string(),
                },
                source: IntentSource::Cfctl,
            })
            .await;
        assert_eq!(result_accepted.status, crate::state::ControlEnvelopeStatus::Accepted);

        engine.drive_tick();

        let events = engine.recorder().snapshot_events();
        // Rejection emits actor.action_rejected with the locked reason.
        let rejected = events
            .iter()
            .find(|e| {
                e.category == "actor"
                    && e.event_type == "action_rejected"
                    && e.payload
                        .get("reason")
                        .and_then(|r| r.as_str())
                        .map(|s| s == cf_equipment::MAX_DEPTH_EXCEEDED)
                        .unwrap_or(false)
            })
            .expect(
                "expected actor.action_rejected with reason 'max_depth_exceeded'; \
                 saw events: see test output",
            );
        assert_eq!(
            rejected.payload.get("action").and_then(|v| v.as_str()),
            Some("act.player.nest_container")
        );

        // Success path emits inventory.container_nested with depth=3.
        let nested = events
            .iter()
            .find(|e| e.category == "inventory" && e.event_type == "container_nested")
            .expect("expected inventory.container_nested for successful medkit nest");
        let depth = nested.payload.get("depth").and_then(|v| v.as_u64()).unwrap_or(0);
        assert_eq!(depth, 3, "medkit nested at depth 3 (inside crate)");
        assert_eq!(
            nested.payload.get("child_item_id").and_then(|v| v.as_str()),
            Some("medkit")
        );
        assert_eq!(
            nested.payload.get("child_is_container").and_then(|v| v.as_bool()),
            Some(false)
        );
    }

    /// `inventory.encumbrance_threshold_crossed` event**.
    #[tokio::test]
    async fn m6b_encumbrance_band_transition_fires_event() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();

        // Seed the player with 15 rifles → Heavy band (52.5 / 50 ratio).
        {
            let mut state = engine.state.write().unwrap();
            let player_id = state.player_actor.unwrap();
            let actor = state
                .actor_state
                .as_mut()
                .unwrap()
                .world
                .actors
                .get_mut(&player_id)
                .unwrap();
            actor.inventory_grid_attach();
            let grid = actor.inventory_grid_mut().unwrap();
            for _ in 0..15 {
                grid.add_top_level("rifle_m1", 1, 0.0);
            }
        }
        // The tick recomputes encumbrance + detects band change.
        engine.drive_tick();
        engine.drive_tick();

        let events = engine.recorder().snapshot_events();
        let band_crossed = events
            .iter()
            .find(|e| e.category == "inventory" && e.event_type == "encumbrance_threshold_crossed")
            .expect("encumbrance_threshold_crossed must fire when band changes");
        assert_eq!(
            band_crossed.payload.get("to_band").and_then(|v| v.as_str()),
            Some("heavy")
        );
        let walk_mult = band_crossed
            .payload
            .get("walk_speed_multiplier")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        assert!((walk_mult - 0.5).abs() < 0.01, "walk_speed_multiplier must be ~0.5");
    }

    /// to the inventory grid AND emits `equipment.item_picked_up_with_mass`**.
    #[tokio::test]
    async fn m6b_pickup_emits_mass_aware_event_and_updates_grid() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();

        // Spawn a dropped rifle near the player.
        let player_pos = {
            let state = engine.state.read().unwrap();
            let player_id = state.player_actor.unwrap();
            state
                .actor_state
                .as_ref()
                .unwrap()
                .world
                .actors
                .get(&player_id)
                .unwrap()
                .position
        };
        // Drop the held rifle so it lands in the world.
        engine
            .dispatch(ControlCommand::ActM6 {
                action: crate::m6_actions::M6Action::DropItem { slot: Some(0) },
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        // Push the dropped item next to the player so pickup is in range.
        {
            let mut state = engine.state.write().unwrap();
            for item in state.m6_dropped_items.iter_mut() {
                item.position = player_pos;
            }
        }
        engine.drive_tick();
        // Now pick it up.
        engine
            .dispatch(ControlCommand::ActM6 {
                action: crate::m6_actions::M6Action::Pickup,
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        engine.drive_tick();

        let events = engine.recorder().snapshot_events();
        // Both the legacy event AND the mass-aware sibling MUST fire.
        let legacy = events
            .iter()
            .filter(|e| e.category == "equipment" && e.event_type == "item_picked_up")
            .count();
        let mass_aware = events
            .iter()
            .filter(|e| e.category == "equipment" && e.event_type == "item_picked_up_with_mass")
            .count();
        assert!(legacy >= 1, "legacy equipment.item_picked_up must still fire");
        assert!(
            mass_aware >= 1,
            "M6B equipment.item_picked_up_with_mass must fire alongside legacy event"
        );
        // The mass_aware event carries canonical mass + dimensions from
        // the ItemSpec registry (mass=3.5, dims=2×4 per rifle_m1_default
        // → falls back to legacy weight when not in registry; rifle_m1_default
        // IS in the registry so we expect 3.5).
        let mass_event = events
            .iter()
            .find(|e| e.category == "equipment" && e.event_type == "item_picked_up_with_mass")
            .unwrap();
        let mass_kg = mass_event
            .payload
            .get("mass_kg")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        assert!(
            (mass_kg - 3.5).abs() < 0.01,
            "mass_kg from registry must be 3.5 (got {mass_kg})"
        );
        let total = mass_event
            .payload
            .get("inventory_total_mass_kg")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        assert!(total > 0.0, "inventory_total_mass_kg must be > 0 after pickup");
    }
}
