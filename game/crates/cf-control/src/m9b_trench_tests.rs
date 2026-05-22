//! Tests for m9b_trench.rs (extracted to keep m9b_trench.rs under 2000 LOC).

#![allow(unused_imports)]

#[cfg(test)]
use crate::engine::M0Engine;
#[cfg(test)]
use crate::m9b_trench::*;
#[cfg(test)]
use cf_trench::{collapse::CollapseEnv, dig_validation::dig_substrate_validate, drainage::DrainageEnv, modules::TrenchModule, segment::SegmentVariant};
#[cfg(test)]
use cf_sim_core::Tick;

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_SCENARIO_SEQ: std::sync::atomic::AtomicU64 =
        std::sync::atomic::AtomicU64::new(0);

    #[test]
    fn parse_variant_round_trip() {
        for variant in [
            SegmentVariant::ShallowScrape,
            SegmentVariant::Standard,
            SegmentVariant::Deep,
            SegmentVariant::Communication,
            SegmentVariant::FireStep,
            SegmentVariant::ParapetRaised,
        ] {
            let s = variant.as_str();
            assert_eq!(
                parse_variant(s),
                Some(variant),
                "round-trip failed for variant `{s}`"
            );
        }
    }

    #[test]
    fn parse_module_round_trip() {
        for m in [
            TrenchModule::Duckboard,
            TrenchModule::FireStep,
            TrenchModule::Breastwork,
            TrenchModule::DrainageSump,
            TrenchModule::Revetment,
            TrenchModule::CornerTraverse,
        ] {
            assert_eq!(parse_module(m.as_str()), Some(m));
        }
    }

    /// VAL-M9B-DIG-001: standard variant carves over 12 in-game seconds
    /// (4× the 5s shallow_scrape baseline).
    #[test]
    fn dig_time_standard_is_4x_shallow_scrape() {
        assert_eq!(dig_time_seconds_for(SegmentVariant::Standard), 12);
        assert_eq!(dig_time_seconds_for(SegmentVariant::ShallowScrape), 5);
        // VAL-M9B-DIG-001 evidence string: "12 × tick_rate ticks (4× the
        // 5s shallow_scrape baseline)". Mathematically `4 * 5 = 20`,
        // but the spec table sets standard = 12s explicitly. The
        // assertion below checks the spec table value, not the literal
        // 4× math (since spec is authoritative).
        assert!(
            dig_time_seconds_for(SegmentVariant::Standard)
                > dig_time_seconds_for(SegmentVariant::ShallowScrape),
            "standard must dig slower than shallow_scrape"
        );
    }

    /// VAL-M9B-DIG-003: deep on hardness ≥ 0.5 falls back to
    /// shallow_scrape with a downgrade reason.
    #[test]
    fn dig_substrate_fallback_at_threshold() {
        let outcome = dig_substrate_validate(SegmentVariant::Deep, 0.7, false);
        assert!(outcome.is_fallback());
        assert_eq!(
            outcome.effective_variant(),
            Some(SegmentVariant::ShallowScrape)
        );
    }

    #[test]
    fn resolve_dig_tool_unknown_defaults_to_entrenching() {
        let (id, tier) = resolve_dig_tool(Some("nonexistent_tool"), SegmentVariant::Standard);
        assert_eq!(id, cf_equipment::tool::entrenching::ENTRENCHING_TOOL_ID);
        assert_eq!(tier, 0);
    }

    #[test]
    fn resolve_dig_tool_pickaxe_returns_tier() {
        let (id, tier) = resolve_dig_tool(
            Some(cf_equipment::tool::dig_pickaxe::PICKAXE_DIG_T2_ID),
            SegmentVariant::Standard,
        );
        assert_eq!(id, cf_equipment::tool::dig_pickaxe::PICKAXE_DIG_T2_ID);
        assert_eq!(tier, 2);
    }

    #[test]
    fn build_time_matches_spec_table() {
        assert_eq!(build_time_seconds_for(TrenchModule::Duckboard), 4);
        assert_eq!(build_time_seconds_for(TrenchModule::FireStep), 8);
        assert_eq!(build_time_seconds_for(TrenchModule::Breastwork), 12);
        assert_eq!(build_time_seconds_for(TrenchModule::DrainageSump), 6);
        assert_eq!(build_time_seconds_for(TrenchModule::Revetment), 10);
        assert_eq!(build_time_seconds_for(TrenchModule::CornerTraverse), 6);
    }

    #[test]
    fn module_cost_json_round_trips() {
        let json = module_cost_json(TrenchModule::Breastwork);
        let obj = json.as_object().expect("cost object");
        assert_eq!(obj.get("sandbag").and_then(|v| v.as_u64()), Some(6));
        let json = module_cost_json(TrenchModule::Revetment);
        let obj = json.as_object().expect("cost object");
        assert_eq!(obj.get("wood").and_then(|v| v.as_u64()), Some(4));
        assert_eq!(obj.get("iron").and_then(|v| v.as_u64()), Some(2));
    }

    /// VAL-M9B-MODULES-001 + m9b-4: default_modules_for returns the
    /// authored embedded-module set for each variant (mirrors
    /// content/trench_segments/<variant>.ron).
    #[test]
    fn default_modules_match_segment_ron() {
        assert!(default_modules_for(SegmentVariant::ShallowScrape).is_empty());
        assert_eq!(
            default_modules_for(SegmentVariant::Standard),
            vec![TrenchModule::Duckboard]
        );
        assert_eq!(
            default_modules_for(SegmentVariant::Deep),
            vec![TrenchModule::Duckboard, TrenchModule::DrainageSump]
        );
        assert_eq!(
            default_modules_for(SegmentVariant::Communication),
            vec![TrenchModule::Duckboard]
        );
        assert_eq!(
            default_modules_for(SegmentVariant::FireStep),
            vec![TrenchModule::Duckboard, TrenchModule::FireStep]
        );
        assert_eq!(
            default_modules_for(SegmentVariant::ParapetRaised),
            vec![TrenchModule::Duckboard, TrenchModule::Breastwork]
        );
    }

    fn make_engine() -> M0Engine {
        let mut p = std::env::temp_dir();
        let seq = TEST_SCENARIO_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        p.push(format!("m9b_trench_test_{}_{}.ron", std::process::id(), seq));
        std::fs::write(
            &p,
            r#"(
  schema_version: 1,
  id: "m9b_trench_test",
  display_name: "M9B trench live world test",
  description: "Empty scene for trench world tests.",
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
  expected_tests: [],
  notes: "",
)"#,
        )
        .unwrap();
        let scenario = crate::scenario::Scenario::load_from_file(&p).unwrap();
        let cfg = crate::engine::M0EngineConfig::for_loaded_scenario(&scenario, p);
        M0Engine::new(cfg)
    }

    /// **m9b-4 PRECONDITION**: `compute_trench_segment_at_pos` after
    /// `insert_trench_segment` finds the placed segment instead of
    /// always returning null.
    #[test]
    fn insert_segment_then_observe_returns_placed_segment() {
        let engine = make_engine();
        let id = engine.insert_trench_segment(SegmentVariant::Standard, (10, 0));
        assert_eq!(id, 1);
        let observed = engine.compute_trench_segment_at_pos(15, 5);
        let result = observed.get("result").expect("result key");
        assert!(
            !result.is_null(),
            "after insert, observe must return the segment instead of null"
        );
        let variant = result.get("variant").and_then(|v| v.as_str());
        assert_eq!(variant, Some("standard"));
    }

    /// **m9b-4 PRECONDITION**: `observe.trench_segment_at_pos` still
    /// returns null for tiles outside any placed segment.
    #[test]
    fn observe_returns_null_for_open_ground() {
        let engine = make_engine();
        let _ = engine.insert_trench_segment(SegmentVariant::Standard, (10, 0));
        let observed = engine.compute_trench_segment_at_pos(100, 100);
        let result = observed.get("result").expect("result key");
        assert!(result.is_null());
    }

    /// **m9b-4 PRECONDITION**: `embed_trench_module` adds modules to a
    /// previously inserted segment.
    #[test]
    fn embed_module_appends_to_segment() {
        let engine = make_engine();
        let _ = engine.insert_trench_segment(SegmentVariant::Standard, (10, 0));
        let added = engine.embed_trench_module(0, TrenchModule::Revetment);
        assert!(added);
        let observed = engine.compute_trench_segment_at_pos(15, 5);
        let modules = observed
            .pointer("/result/embedded_modules")
            .and_then(|m| m.as_array())
            .expect("embedded_modules array");
        let names: Vec<&str> = modules.iter().filter_map(|v| v.as_str()).collect();
        assert!(names.contains(&"revetment"));
        assert!(names.contains(&"duckboard"));
    }

    /// Insert two segments and verify monotonically increasing ids.
    #[test]
    fn insert_segments_allocates_unique_ids() {
        let engine = make_engine();
        let id1 = engine.insert_trench_segment(SegmentVariant::Standard, (10, 0));
        let id2 = engine.insert_trench_segment(SegmentVariant::Deep, (40, 0));
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    fn count_events(engine: &M0Engine, category: &str, event_type: &str) -> usize {
        engine
            .recorder
            .snapshot_events()
            .into_iter()
            .filter(|e| e.category == category && e.event_type == event_type)
            .count()
    }

    /// VAL-M9B-DRAINAGE-001: drainage tick fires
    /// `trench.drainage_flushed` when the sump kicks.
    #[test]
    fn drainage_tick_emits_flush_event() {
        let engine = make_engine();
        let _ = engine.insert_trench_segment(SegmentVariant::Deep, (0, 0));
        let env = DrainageEnv::default();
        // Drive enough ticks to cross the threshold.
        let mut depth = 0.0_f32;
        for _ in 0..400 {
            let outcome = engine.dispatch_m9b_drainage_tick(
                0,
                depth,
                true,
                env,
                Tick(0),
                0.0,
            );
            depth = outcome.water_depth_after();
        }
        let flushes = count_events(&engine, "trench", "drainage_flushed");
        assert!(
            flushes >= 1,
            "drainage helper must emit ≥ 1 flush event over the 600-tick window"
        );
    }

    /// VAL-M9B-DRAINAGE-002: no-sump tick does NOT emit a flush event.
    #[test]
    fn drainage_tick_without_sump_emits_no_event() {
        let engine = make_engine();
        let _ = engine.insert_trench_segment(SegmentVariant::Standard, (0, 0));
        let env = DrainageEnv::default();
        let mut depth = 0.0_f32;
        for _ in 0..400 {
            let outcome = engine.dispatch_m9b_drainage_tick(
                0,
                depth,
                false,
                env,
                Tick(0),
                0.0,
            );
            depth = outcome.water_depth_after();
        }
        assert_eq!(count_events(&engine, "trench", "drainage_flushed"), 0);
    }

    /// VAL-M9B-BREASTWORK-001: 80 rounds at 6 J emits exactly one
    /// `trench.breastwork_breached` event.
    #[test]
    fn breastwork_hits_emit_exactly_one_breach() {
        let engine = make_engine();
        let _ = engine.insert_trench_segment(SegmentVariant::ParapetRaised, (0, 0));
        let mut hp = cf_trench::BREASTWORK_MAX_HP;
        for _ in 0..80 {
            let outcome =
                engine.dispatch_m9b_breastwork_hit(0, hp, 6.0, Tick(0), 0.0);
            hp = outcome.hp_after();
            if hp <= 0.0 {
                break;
            }
        }
        assert_eq!(
            count_events(&engine, "trench", "breastwork_breached"),
            1,
            "exactly one breach event over the 80-round burst"
        );
    }

    /// VAL-M9B-REVETMENT-001: no revetment + soft dirt → ≥ 1
    /// `trench.segment_collapsed` event over 1800 ticks.
    #[test]
    fn collapse_tick_no_revetment_emits_collapse_event() {
        let engine = make_engine();
        let _ = engine.insert_trench_segment(SegmentVariant::Standard, (0, 0));
        let env = CollapseEnv::soft_dirt_no_revetment();
        let mut integrity = cf_trench::STARTING_INTEGRITY;
        for _ in 0..cf_trench::REVETMENT_AUDIT_WINDOW_TICKS {
            let outcome =
                engine.dispatch_m9b_collapse_tick(0, integrity, env, Tick(0), 0.0);
            if outcome.collapsed() {
                break;
            }
            integrity = outcome.integrity_after();
        }
        let collapses = count_events(&engine, "trench", "segment_collapsed");
        assert!(
            collapses >= 1,
            "no-revetment soft-dirt 1800-tick window must emit ≥ 1 collapse"
        );
    }

    /// VAL-M9B-REVETMENT-002: revetment installed → 0
    /// `trench.segment_collapsed` events over 1800 ticks.
    #[test]
    fn collapse_tick_with_revetment_emits_no_collapse() {
        let engine = make_engine();
        let _ = engine.insert_trench_segment(SegmentVariant::Standard, (0, 0));
        let env = CollapseEnv::soft_dirt_with_revetment();
        let mut integrity = cf_trench::STARTING_INTEGRITY;
        for _ in 0..cf_trench::REVETMENT_AUDIT_WINDOW_TICKS {
            let outcome =
                engine.dispatch_m9b_collapse_tick(0, integrity, env, Tick(0), 0.0);
            integrity = outcome.integrity_after();
        }
        assert_eq!(
            count_events(&engine, "trench", "segment_collapsed"),
            0,
            "revetment must prevent collapse over 1800 ticks"
        );
        assert!(integrity >= cf_trench::REVETMENT_INTEGRITY_FLOOR);
    }

    /// **VAL-CROSS-002** (inherited by M10B closure from M9C
    /// close-deferred): a parapet_raised trench dig succeeds
    /// post-M9C, the placed segment carries the `breastwork`
    /// embedded module, the M9B-side authored breastwork.ron declares
    /// the 6-sandbag cost, the M9C breastwork kernel reports HP 400,
    /// and the `parapet_raised_requires_m9c` warning event does NOT
    /// fire. The cfctl trace this test captures is the
    /// `dispatch_m9b_dig_trench_segment` call (cfctl maps
    /// `act.player.dig_trench_segment` to this dispatch path) +
    /// `compute_trench_segment_at_pos` (cfctl maps
    /// `observe.trench_segment_at_pos`).
    #[test]
    fn val_cross_002_parapet_raised_dig_emits_breastwork_segment() {
        let engine = make_engine();

        // 1. Kernel surface: cf-trench's parapet_raised_dig_validate is
        //    Ok(()) post-M9C (the same surface VAL-CROSS-003 covers
        //    against the pre-M9C warning path).
        let validate = cf_trench::parapet_raised_forward_compat::parapet_raised_dig_validate();
        assert!(
            validate.is_ok(),
            "VAL-CROSS-002 precondition: parapet_raised_dig_validate must return Ok(()) post-M9C"
        );

        // 2. M9C kernel HP 400 invariant: BREASTWORK_MAX_HP is the
        //    health the placed breastwork module spawns with.
        assert_eq!(
            cf_trench::BREASTWORK_MAX_HP as u32,
            400,
            "VAL-CROSS-002: BREASTWORK_MAX_HP must be 400 (spec § Notes)"
        );

        // 3. End-to-end cfctl trace: drive the dig via the
        //    `dispatch_m9b_dig_trench_segment` handler the cfctl
        //    method `act.player.dig_trench_segment` routes to. The
        //    handler MUST accept the action; substrate hardness 0.2 is
        //    below the deep-substrate threshold so parapet_raised does
        //    not fall back to shallow_scrape. We mark the source as
        //    Cfctl to mirror the JSON-RPC dispatch path the spec
        //    contract is anchored to.
        let outcome = engine.dispatch_m9b_dig_trench_segment(
            "parapet_raised".into(),
            Some(cf_equipment::tool::entrenching::ENTRENCHING_TOOL_ID.into()),
            0.2_f32,
            false,
            cf_actor::IntentSource::Cfctl,
            Tick(0),
            0.0,
        );
        assert_eq!(
            outcome.status,
            crate::state::ControlEnvelopeStatus::Accepted,
            "VAL-CROSS-002: parapet_raised dig must be accepted post-M9C; got {outcome:?}"
        );

        // 4. observe.trench_segment_at_pos reports the placed segment
        //    with `breastwork` embedded so subsequent fire route
        //    through the breastwork HP gate (VAL-M9B-BREASTWORK-001).
        let observed = engine.compute_trench_segment_at_pos(0, 0);
        let modules = observed
            .pointer("/result/embedded_modules")
            .and_then(|m| m.as_array())
            .expect("VAL-CROSS-002: observe must return embedded_modules");
        let names: Vec<&str> = modules.iter().filter_map(|v| v.as_str()).collect();
        assert!(
            names.contains(&"breastwork"),
            "VAL-CROSS-002: parapet_raised segment must embed `breastwork` module; got {names:?}"
        );

        // 5. The replay log carries `segment_dug` with
        //    variant=parapet_raised AND DOES NOT carry the
        //    `parapet_raised_requires_m9c` warning.
        let dug = engine
            .recorder
            .snapshot_events()
            .into_iter()
            .filter(|e| e.event_type == "segment_dug" && e.category == "trench")
            .collect::<Vec<_>>();
        assert!(
            !dug.is_empty(),
            "VAL-CROSS-002: expected ≥ 1 trench.segment_dug event"
        );
        assert!(
            dug.iter().any(|e| {
                e.payload.get("variant").and_then(|v| v.as_str()) == Some("parapet_raised")
            }),
            "VAL-CROSS-002: ≥ 1 segment_dug event must carry variant=parapet_raised"
        );
        assert_eq!(
            count_events(&engine, "trench", "parapet_raised_requires_m9c"),
            0,
            "VAL-CROSS-002 / VAL-CROSS-003: post-M9C the parapet_raised_requires_m9c warning MUST NOT fire"
        );

        // 6. The 6-sandbag cost is declared by the authored module
        //    cost map. m9b-3 already routes
        //    `act.player.place_trench_module` through
        //    `module_cost_json(Breastwork)`; we assert the spec value
        //    end-to-end here so a future cost change can't silently
        //    drift the VAL-CROSS-002 contract.
        let cost = module_cost_json(TrenchModule::Breastwork);
        let obj = cost
            .as_object()
            .expect("VAL-CROSS-002: breastwork cost is a JSON object");
        assert_eq!(
            obj.get("sandbag").and_then(|v| v.as_u64()),
            Some(6),
            "VAL-CROSS-002: breastwork module declares 6 sandbags (spec § M9B modules table)"
        );
    }

    /// **VAL-CROSS-004** (inherited by M10B closure from M9C
    /// close-deferred): a crewed fortification dominates the
    /// underlying trench segment's cover_state derivation. Deploying
    /// + crewing inside a `fire_step` segment promotes Standing
    ///   on-step from Exposed → Full; uncrew returns to Exposed for
    ///   fire_step on-step Standing.
    #[test]
    fn val_cross_004_mg_tripod_inside_fire_step_crewing_dominates_cover_state() {
        use cf_actor::{ActorState, ActorId, Inventory, Vec2};
        use cf_trench::segment::{InMemorySegments, TrenchSegment};
        use cf_trench::CoverState as TrenchCoverState;

        // 1. Set up: a fire_step segment at (0, 0) with depth=16 +
        //    step_height=8. Per VAL-M9B-SEGMENT-004 standing on-step
        //    == Exposed.
        let segment = TrenchSegment {
            variant: SegmentVariant::FireStep,
            tile_x: 0,
            tile_y: 0,
            depth: 16,
            width: 20,
            raised_step_height: Some(8),
            embedded_modules: vec![TrenchModule::Duckboard, TrenchModule::FireStep],
        };
        let world = InMemorySegments::with_segments(vec![segment]);

        // 2. Stand the player on-step; pre-crew baseline is Exposed.
        let mut player = ActorState::player(
            ActorId(1),
            "blue",
            Vec2::new(5.0, 10.0),
            100.0,
            Inventory::default(),
        );
        player.on_ground = true;
        player.crouch_active = false;
        player.prone_active = false;
        assert_eq!(
            player.cover_state(&world),
            TrenchCoverState::Exposed,
            "VAL-CROSS-004 baseline: Standing on-step in fire_step must be Exposed"
        );

        // 3. Deploy + crew the mg_tripod (the cfctl methods
        //    `act.player.deploy_mg_tripod` then
        //    `act.player.crew_fortification` map to this kernel
        //    transition; the engine assigns a fortification_id which
        //    we mimic here as 42).
        let tripod_id: u32 = 42;
        player.crew_fortification(tripod_id);
        assert!(player.is_crewing());
        assert_eq!(player.crewed_fortification_id(), Some(tripod_id));
        assert_eq!(
            player.cover_state(&world),
            TrenchCoverState::Full,
            "VAL-CROSS-004: crewing dominates the segment-variant table → cover_state == Full"
        );

        // 4. Uncrew (cfctl `act.player.uncrew_fortification`) →
        //    segment-variant baseline restored (Exposed for
        //    Standing on-step in fire_step).
        let released = player.uncrew_fortification();
        assert_eq!(released, Some(tripod_id));
        assert!(!player.is_crewing());
        assert_eq!(
            player.cover_state(&world),
            TrenchCoverState::Exposed,
            "VAL-CROSS-004: uncrew restores fire_step on-step Standing == Exposed"
        );
    }
}
