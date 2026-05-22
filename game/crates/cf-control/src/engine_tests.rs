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

    static TEST_SCENARIO_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    #[test]
    fn prototype_slice_for_milestone_uppercases_letter_suffix() {
        // milestones (m3a, m3b, m4a, m4b) must produce uppercase prototype
        // slice strings (M3A, M3B, M4A, M4B). Pre-fix, `format!("M{rest}")`
        // returned the lowercased rest from `to_lowercase()` and produced
        // `M3a` / `M3b` / etc.
        assert_eq!(prototype_slice_for_milestone("m3a"), "M3A");
        assert_eq!(prototype_slice_for_milestone("M3A"), "M3A");
        assert_eq!(prototype_slice_for_milestone("m3b"), "M3B");
        assert_eq!(prototype_slice_for_milestone("m4a"), "M4A");
        assert_eq!(prototype_slice_for_milestone("m4b"), "M4B");
    }

    #[test]
    fn prototype_slice_for_milestone_handles_numeric_and_dotted_milestones() {
        assert_eq!(prototype_slice_for_milestone("m0"), "M0");
        assert_eq!(prototype_slice_for_milestone("m1"), "M1");
        assert_eq!(prototype_slice_for_milestone("m1.5"), "M1.5");
        assert_eq!(prototype_slice_for_milestone("m2"), "M2");
        assert_eq!(prototype_slice_for_milestone("m2.5"), "M2.5");
        assert_eq!(prototype_slice_for_milestone("m5.5.5"), "M5.5.5");
    }

    #[test]
    fn prototype_slice_for_milestone_empty_input_falls_back_to_m0() {
        assert_eq!(prototype_slice_for_milestone(""), "M0");
        assert_eq!(prototype_slice_for_milestone("   "), "M0");
    }

    #[test]
    fn notes_addendum_categories_match_per_milestone_layering() {
        // claim categories that haven't shipped yet at the named milestone.
        // M0 = system / control / determinism only; M1 adds actor / combat /
        // equipment / input; M1.5 adds ai / mission / terrain; M2 adds
        // material; M3A adds snapshot. Layer is append-only.
        let m0 = notes_addendum_for_milestone("m0");
        assert!(m0.contains("`system`"));
        assert!(m0.contains("`control`"));
        assert!(m0.contains("`determinism`"));
        assert!(!m0.contains("`actor`"), "M0 must NOT advertise actor category");
        assert!(!m0.contains("`material`"), "M0 must NOT advertise material category");
        assert!(!m0.contains("`snapshot`"), "M0 must NOT advertise snapshot category");

        let m1 = notes_addendum_for_milestone("m1");
        assert!(m1.contains("`actor`"));
        assert!(m1.contains("`combat`"));
        assert!(!m1.contains("`material`"), "M1 must NOT advertise material category");
        assert!(!m1.contains("`mission`"), "M1 must NOT advertise mission category");

        let m1_5 = notes_addendum_for_milestone("m1.5");
        assert!(m1_5.contains("`ai`"));
        assert!(m1_5.contains("`mission`"));
        assert!(m1_5.contains("`terrain`"));
        assert!(!m1_5.contains("`material`"), "M1.5 must NOT advertise material (M2+)");
        assert!(!m1_5.contains("`snapshot`"), "M1.5 must NOT advertise snapshot (M3A+)");

        let m2 = notes_addendum_for_milestone("m2");
        assert!(m2.contains("`material`"));
        assert!(!m2.contains("`snapshot`"), "M2 must NOT advertise snapshot (M3A+)");

        let m3a = notes_addendum_for_milestone("m3a");
        assert!(m3a.contains("`snapshot`"));
        assert!(m3a.contains("`material`"));
        assert!(m3a.contains("`mission`"));
    }

    #[test]
    fn notes_addendum_categories_layer_correctly_for_post_m5_10_milestones() {
        // arms stopped at m5.10, so M6/M6.5/M7/M8/etc. silently fell through
        // to "categories shipped: system, control, determinism" only —
        // missing the entire append-only layer they should have inherited.
        // After the milestone_order_index refactor, M6+ correctly inherits
        // every prior category.
        for m in [
            "m6", "m6.5", "m6.6", "m7", "m7.5", "m7.7", "m8", "m8.5", "m8.6", "m9", "m9.5", "m10", "m11", "m12",
        ] {
            let body = notes_addendum_for_milestone(m);
            assert!(body.contains("`actor`"), "{m}: missing actor category");
            assert!(body.contains("`mission`"), "{m}: missing mission category");
            assert!(body.contains("`material`"), "{m}: missing material category");
            assert!(body.contains("`snapshot`"), "{m}: missing snapshot category");
        }
    }

    #[test]
    fn milestone_order_index_orders_canonical_roadmap() {
        assert!(milestone_order_index("m0") < milestone_order_index("m1"));
        assert!(milestone_order_index("m1") < milestone_order_index("m1.5"));
        assert!(milestone_order_index("m1.5") < milestone_order_index("m2"));
        assert!(milestone_order_index("m2") < milestone_order_index("m2.5"));
        assert!(milestone_order_index("m2.5") < milestone_order_index("m3a"));
        assert!(milestone_order_index("m3a") < milestone_order_index("m3b"));
        assert!(milestone_order_index("m3b") < milestone_order_index("m4a"));
        assert!(milestone_order_index("m4a") < milestone_order_index("m4b"));
        assert!(milestone_order_index("m4b") < milestone_order_index("m5"));
        assert!(milestone_order_index("m5") < milestone_order_index("m5.10"));
        assert!(milestone_order_index("m5.10") < milestone_order_index("m6"));
        assert!(milestone_order_index("m6") < milestone_order_index("m12"));
        // Unknown milestones map to MILESTONE_INDEX_UNKNOWN (after M12) so
        // future milestones default to the final-state universe rather than
        // accidentally falling back to M0's empty categories.
        assert!(milestone_order_index("future-milestone-x") > milestone_order_index("m12"));
    }

    #[test]
    fn m8_effective_sim_speed_pct_default_is_off_no_pie_menu() {
        let settings = Settings::default();
        let pie = cf_squad_ui::PieMenuState::closed();
        assert_eq!(effective_sim_speed_pct(&settings, &pie, false), 100);
    }

    #[test]
    fn m8_effective_sim_speed_pct_slowdown75_alone() {
        let mut settings = Settings::default();
        settings.game_speed_assist = crate::settings::GameSpeedAssist::Slowdown75;
        let pie = cf_squad_ui::PieMenuState::closed();
        assert_eq!(effective_sim_speed_pct(&settings, &pie, false), 75);
    }

    #[test]
    fn m8_effective_sim_speed_pct_slowdown25_alone() {
        let mut settings = Settings::default();
        settings.game_speed_assist = crate::settings::GameSpeedAssist::Slowdown25;
        let pie = cf_squad_ui::PieMenuState::closed();
        assert_eq!(effective_sim_speed_pct(&settings, &pie, false), 25);
    }

    #[test]
    fn m8_effective_sim_speed_pct_full_pause_alone() {
        let mut settings = Settings::default();
        settings.game_speed_assist = crate::settings::GameSpeedAssist::FullPause;
        let pie = cf_squad_ui::PieMenuState::closed();
        assert_eq!(effective_sim_speed_pct(&settings, &pie, false), 0);
    }

    #[test]
    fn m8_effective_sim_speed_pct_pie_menu_open_stacks_with_assist_most_restrictive_wins() {
        let mut settings = Settings::default();
        settings.game_speed_assist = crate::settings::GameSpeedAssist::Slowdown75;
        let mut pie = cf_squad_ui::PieMenuState::closed();
        pie.open(cf_squad_ui::PieMenuTarget::Void, false, 1);
        assert_eq!(pie.slowdown_factor_pct, cf_squad_ui::SINGLE_PLAYER_SLOWDOWN_PCT);
        assert_eq!(
            effective_sim_speed_pct(&settings, &pie, false),
            cf_squad_ui::SINGLE_PLAYER_SLOWDOWN_PCT,
            "pie menu's 20% slowdown is more restrictive than game_speed_assist's 75%",
        );
    }

    #[test]
    fn m8_effective_sim_speed_pct_multiplayer_ignores_assist_but_honors_pie_menu() {
        let mut settings = Settings::default();
        settings.game_speed_assist = crate::settings::GameSpeedAssist::FullPause;
        let pie = cf_squad_ui::PieMenuState::closed();
        assert_eq!(
            effective_sim_speed_pct(&settings, &pie, true),
            100,
            "multiplayer must ignore game_speed_assist (single-player only)",
        );
        let mut mp_pie = cf_squad_ui::PieMenuState::closed();
        mp_pie.open(cf_squad_ui::PieMenuTarget::Void, true, 1);
        assert_eq!(mp_pie.slowdown_factor_pct, 100);
        assert_eq!(effective_sim_speed_pct(&settings, &mp_pie, true), 100);
    }

    #[test]
    fn m8_speed_pct_75_skips_one_in_four_ticks_via_accumulator() {
        let mut acc: u16 = 0;
        let pct: u16 = 75;
        let mut advances = 0;
        let mut skips = 0;
        for _ in 0..4 {
            acc = acc.saturating_add(pct);
            if acc >= 100 {
                acc -= 100;
                advances += 1;
            } else {
                skips += 1;
            }
        }
        assert_eq!(advances, 3, "Slowdown75: 3 advances per 4 wall ticks");
        assert_eq!(skips, 1, "Slowdown75: 1 skip per 4 wall ticks");
    }

    #[test]
    fn m8_speed_pct_25_skips_three_in_four_ticks_via_accumulator() {
        let mut acc: u16 = 0;
        let pct: u16 = 25;
        let mut advances = 0;
        let mut skips = 0;
        for _ in 0..4 {
            acc = acc.saturating_add(pct);
            if acc >= 100 {
                acc -= 100;
                advances += 1;
            } else {
                skips += 1;
            }
        }
        assert_eq!(advances, 1, "Slowdown25: 1 advance per 4 wall ticks");
        assert_eq!(skips, 3, "Slowdown25: 3 skips per 4 wall ticks");
    }

    #[test]
    fn m8_speed_pct_20_pie_menu_skips_four_in_five_ticks_via_accumulator() {
        let mut acc: u16 = 0;
        let pct: u16 = u16::from(cf_squad_ui::SINGLE_PLAYER_SLOWDOWN_PCT);
        let mut advances = 0;
        for _ in 0..5 {
            acc = acc.saturating_add(pct);
            if acc >= 100 {
                acc -= 100;
                advances += 1;
            }
        }
        assert_eq!(advances, 1, "Pie menu 20%: 1 advance per 5 wall ticks");
    }

    #[test]
    fn notes_addendum_includes_dr007_for_every_m2_plus_milestone() {
        // reference documentation for the material set shape. Every M2+
        // bundle has material events in events.jsonl + benefits from the
        // addendum, regardless of whether the milestone EXTENDS or just
        // RUNS ON TOP of chunked terrain. The prior explicit allowlist
        // (M2/M2.5/M3A/M5..M5.10 only) excluded M3B/M4A/M4B + every M6+
        // milestone — including M6.6 'AI Material Competence', M7.5 'Base
        // Atmospherics', M8.5 'Material Lab', M8.6 'Mining'. Switched to
        // `idx >= MILESTONE_INDEX_M2` to match the category-layering
        // pattern.
        for m in [
            "m2", "m2.5", "m3a", "m3b", "m4a", "m4b", "m5", "m5.5", "m5.5.5", "m5.6", "m5.7", "m5.8", "m5.9", "m5.9.5",
            "m5.10", "m6", "m6.5", "m6.6", "m7", "m7.5", "m7.7", "m8", "m8.5", "m8.6", "m9", "m9.5", "m10", "m11",
            "m12",
        ] {
            assert!(
                notes_addendum_for_milestone(m).contains("DR-007 launch material set"),
                "{m} should include DR-007 addendum (idx >= M2)"
            );
        }
        // M0 and M1 are PRE-material — they don't have material events yet,
        // so the addendum is correctly omitted.
        assert!(!notes_addendum_for_milestone("m0").contains("DR-007 launch material set"));
        assert!(!notes_addendum_for_milestone("m1").contains("DR-007 launch material set"));
        assert!(!notes_addendum_for_milestone("m1.5").contains("DR-007 launch material set"));
    }

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

    #[test]
    fn run_m0_inline_writes_a_valid_bundle() {
        let root = temp_run_root();
        let scenario_path = write_test_scenario();
        let mut config = load_test_scenario_and_config(scenario_path);
        config.run_bundle_root = root.clone();
        config.write_run_bundle = true;
        config.run_mode = "test".to_string();

        let outcome = run_m0_inline(config).unwrap();
        let bundle = outcome.bundle_dir.unwrap();
        let manifest_text = std::fs::read_to_string(bundle.join("run_manifest.json")).unwrap();
        assert!(manifest_text.contains("prototype-run-manifest.v0.1"));
        assert!(manifest_text.contains("\"sim_state_v1\""));
        assert!(manifest_text.contains("\"tick_rate_hz\""));
        let summary: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(bundle.join("summary.json")).unwrap()).unwrap();
        assert!(summary.get("first_tick").is_some());
        assert!(summary.get("last_tick").is_some());
        assert!(summary["performance"]["tick_rate_hz"].is_number());
        // M2 fix: every bundle must have a non-null final checksum.
        assert!(
            summary["final_sim_checksum"].is_string(),
            "final_sim_checksum must not be null; got {}",
            summary["final_sim_checksum"]
        );
        assert!(
            summary["checksum_event_count"].as_u64().unwrap_or(0) >= 1,
            "every bundle must record at least one determinism.sim_checksum"
        );
        let notes = std::fs::read_to_string(bundle.join("notes.md")).unwrap();
        for h in [
            "## Assumptions Tested",
            "## Good",
            "## Bad",
            "## Meh",
            "## Evidence Links",
            "## Next Actions",
        ] {
            assert!(notes.contains(h));
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn run_manifest_records_active_key_bindings() {
        let root = temp_run_root();
        let scenario_path = write_test_scenario();
        let mut config = load_test_scenario_and_config(scenario_path);
        config.run_bundle_root = root.clone();
        config.write_run_bundle = true;
        config.run_mode = "test-remap-manifest".to_string();
        config.settings.key_remap_enabled = true;
        config.settings.key_bindings = std::collections::BTreeMap::from([
            ("aim_up".to_string(), "Numpad8".to_string()),
            ("fire".to_string(), "KeyF".to_string()),
        ]);

        let outcome = run_m0_inline(config).unwrap();
        let bundle = outcome.bundle_dir.unwrap();
        let manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(bundle.join("run_manifest.json")).unwrap()).unwrap();
        assert_eq!(manifest["settings"]["key_remap_enabled"], true);
        assert_eq!(manifest["settings"]["key_bindings"]["aim_up"], "Numpad8");
        assert_eq!(manifest["settings"]["key_bindings"]["fire"], "KeyF");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn for_loaded_scenario_pulls_seed_and_expected_tests_from_manifest() {
        let scenario_path = write_test_scenario();
        let scenario = crate::scenario::Scenario::load_from_file(&scenario_path).unwrap();
        let cfg = M0EngineConfig::for_loaded_scenario(&scenario, scenario_path);
        assert_eq!(cfg.seed, scenario.seed);
        assert_eq!(cfg.duration_ticks, scenario.duration_ticks.unwrap_or(0));
        assert_eq!(cfg.expected_tests, vec!["M0-SMOKE-01".to_string()]);
        assert!((cfg.region_width - 1280.0).abs() < f32::EPSILON);
        assert!((cfg.region_height - 720.0).abs() < f32::EPSILON);
    }

    #[test]
    fn m8_drive_tick_full_pause_returns_none_without_advancing_clock() {
        let scenario_path = write_test_scenario();
        let mut cfg = load_test_scenario_and_config(scenario_path);
        cfg.settings.game_speed_assist = crate::settings::GameSpeedAssist::FullPause;
        cfg.run_mode = "test-game-speed-full-pause".to_string();
        let engine = M0Engine::new(cfg);
        let start = engine.current_tick();
        for _ in 0..32 {
            assert!(
                engine.drive_tick().is_none(),
                "FullPause must always return None from drive_tick",
            );
        }
        assert_eq!(engine.current_tick(), start, "FullPause must not advance the clock",);
    }

    #[test]
    fn m8_drive_tick_slowdown75_advances_three_in_four_ticks() {
        let scenario_path = write_test_scenario();
        let mut cfg = load_test_scenario_and_config(scenario_path);
        cfg.settings.game_speed_assist = crate::settings::GameSpeedAssist::Slowdown75;
        cfg.run_mode = "test-game-speed-slowdown75".to_string();
        let engine = M0Engine::new(cfg);
        let mut advances = 0;
        let mut skips = 0;
        for _ in 0..400 {
            if engine.drive_tick().is_some() {
                advances += 1;
            } else {
                skips += 1;
            }
        }
        assert_eq!(advances, 300, "Slowdown75: 3 in 4 ticks advance (=300 of 400)");
        assert_eq!(skips, 100, "Slowdown75: 1 in 4 ticks skipped (=100 of 400)");
    }

    #[test]
    fn m8_drive_tick_slowdown25_advances_one_in_four_ticks() {
        let scenario_path = write_test_scenario();
        let mut cfg = load_test_scenario_and_config(scenario_path);
        cfg.settings.game_speed_assist = crate::settings::GameSpeedAssist::Slowdown25;
        cfg.run_mode = "test-game-speed-slowdown25".to_string();
        let engine = M0Engine::new(cfg);
        let mut advances = 0;
        let mut skips = 0;
        for _ in 0..400 {
            if engine.drive_tick().is_some() {
                advances += 1;
            } else {
                skips += 1;
            }
        }
        assert_eq!(advances, 100, "Slowdown25: 1 in 4 ticks advance (=100 of 400)");
        assert_eq!(skips, 300, "Slowdown25: 3 in 4 ticks skipped (=300 of 400)");
    }

    #[test]
    fn m8_drive_tick_off_advances_every_tick() {
        let scenario_path = write_test_scenario();
        let mut cfg = load_test_scenario_and_config(scenario_path);
        cfg.settings.game_speed_assist = crate::settings::GameSpeedAssist::Off;
        cfg.run_mode = "test-game-speed-off".to_string();
        let engine = M0Engine::new(cfg);
        for _ in 0..64 {
            assert!(
                engine.drive_tick().is_some(),
                "game_speed_assist=Off must always advance",
            );
        }
    }

    #[test]
    fn mid_run_write_run_bundle_has_final_checksum() {
        // Repro for the M0.1 follow-up gap: a `runbundle.write` request that fires
        // BEFORE the run is finalized previously produced a bundle with
        // `final_sim_checksum=null` and `checksum_event_count=0`.
        let root = temp_run_root();
        let scenario_path = write_test_scenario();
        let mut config = load_test_scenario_and_config(scenario_path);
        config.duration_ticks = 6;
        config.run_bundle_root = root.clone();
        config.run_mode = "test-mid-run".to_string();
        let engine = M0Engine::new(config);
        engine.record_run_started();
        for _ in 0..6 {
            engine.drive_tick();
        }
        // Write the bundle WITHOUT calling record_run_finished, mimicking the live
        // `runbundle.write` server path.
        let bundle = engine.write_run_bundle(WallClock.now_utc(), 0).unwrap();
        let summary: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(bundle.join("summary.json")).unwrap()).unwrap();
        assert!(
            summary["final_sim_checksum"].is_string(),
            "mid-run runbundle.write must still emit a final checksum; got {}",
            summary["final_sim_checksum"]
        );
        assert!(
            summary["checksum_event_count"].as_u64().unwrap_or(0) >= 1,
            "mid-run bundle must record at least one determinism.sim_checksum"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn panic_in_sub_thread_emits_system_panic_event_and_increments_severity() {
        // M0.2-F5: M0-008 task card requires "panic test triggers a controlled panic in a
        // sub-thread and verifies the event is emitted; counter assertion."
        //
        // The engine wires `M0Engine::new` → `diagnostics::set_panic_reporter` → a closure
        // that calls `report_panic_to_recorder(&recorder, msg)`. This test:
        //   1. Spawns a sub-thread that genuinely calls `panic!`.
        //   2. `JoinHandle::join` catches the panic (returns Err with payload).
        //   3. Routes the captured payload through `report_panic_to_recorder`, which is
        //      the SAME function the global panic hook invokes — bypassing the global
        //      `PANIC_REPORTER` slot only because cargo test parallelism would race
        //      another test's `M0Engine::new` for the slot.
        //   4. Asserts the recorder now contains a `system.panic` event AND the
        //      `by_severity.error` counter advanced.
        let scenario_path = write_test_scenario();
        let config = load_test_scenario_and_config(scenario_path);
        let engine = M0Engine::new(config);
        let recorder = engine.recorder();
        let pre_error_count = recorder.counts().by_severity.get("error").copied().unwrap_or(0);
        let pre_panic_events = recorder
            .snapshot_events()
            .iter()
            .filter(|e| e.category == "system" && e.event_type == "panic")
            .count();

        // Real panic on a sub-thread, real catch via `join`.
        let handle = std::thread::spawn(|| -> () {
            panic!("controlled M0.2-F5 panic for test");
        });
        let join_err = handle.join().expect_err("the spawned thread MUST panic");
        let panic_msg: String = if let Some(s) = join_err.downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = join_err.downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic payload".to_string()
        };

        // Same code path the global panic hook drives (see `M0Engine::new`). Use tick=0
        // because we never advanced the engine.
        report_panic_to_recorder(&recorder, 0, 0.0, &panic_msg);

        let panics: Vec<_> = recorder
            .snapshot_events()
            .into_iter()
            .filter(|e| e.category == "system" && e.event_type == "panic")
            .collect();
        assert!(
            panics.len() > pre_panic_events,
            "system.panic must land in events.jsonl after a sub-thread panic; pre={pre_panic_events} post={}",
            panics.len()
        );
        let recorded_msg = panics.last().unwrap().payload["message"].as_str().unwrap_or("");
        assert!(
            recorded_msg.contains("controlled M0.2-F5 panic for test"),
            "system.panic payload must include the panic message; got `{recorded_msg}`"
        );
        let post_error_count = recorder.counts().by_severity.get("error").copied().unwrap_or(0);
        assert!(
            post_error_count > pre_error_count,
            "system.panic must increment summary.json.event_counts.by_severity.error; pre={pre_error_count} post={post_error_count}"
        );
    }

    #[tokio::test]
    async fn scenario_load_with_mismatched_seed_is_rejected() {
        // M0.2-F3: scenario.load with a seed that differs from the active engine seed
        // must be REJECTED, not silently accepted-and-ignored. M0 cannot re-seed a live
        // engine.
        let scenario_path = write_test_scenario();
        let mut config = load_test_scenario_and_config(scenario_path);
        config.seed = 42;
        let engine = M0Engine::new(config);
        let result = engine
            .dispatch(ControlCommand::ScenarioLoad {
                scenario: "m0_blank".to_string(),
                seed: Some(7),
            })
            .await;
        assert_eq!(
            result.status,
            crate::state::ControlEnvelopeStatus::Rejected,
            "scenario.load with mismatched seed must reject; got {:?}",
            result.status
        );
        assert_eq!(result.reason.as_deref(), Some("seed_override_not_supported_in_m0"));
        // The recorder must have a `command_rejected` event with the right reason.
        let events = engine.recorder().snapshot_events();
        let rejection = events
            .iter()
            .find(|e| {
                e.category == "control" && e.event_type == "command_rejected" && e.payload["method"] == "scenario.load"
            })
            .expect("rejection event must be recorded");
        assert_eq!(rejection.payload["reason"], "seed_override_not_supported_in_m0");
        assert_eq!(rejection.payload["active_seed"], 42);
        assert_eq!(rejection.payload["requested_seed"], 7);
    }

    #[tokio::test]
    async fn scenario_load_with_matching_seed_is_accepted() {
        // F3 follow-up: scenario.load with seed == active seed is a benign no-op and
        // should be accepted (this matches the cfctl client's "reconfirm" semantics).
        let scenario_path = write_test_scenario();
        let mut config = load_test_scenario_and_config(scenario_path);
        config.seed = 42;
        let engine = M0Engine::new(config);
        let result = engine
            .dispatch(ControlCommand::ScenarioLoad {
                scenario: "m0_blank".to_string(),
                seed: Some(42),
            })
            .await;
        assert_eq!(result.status, crate::state::ControlEnvelopeStatus::Accepted);
    }

    #[tokio::test]
    async fn scenario_load_unknown_scenario_is_rejected() {
        let scenario_path = write_test_scenario();
        let config = load_test_scenario_and_config(scenario_path);
        let engine = M0Engine::new(config);
        let result = engine
            .dispatch(ControlCommand::ScenarioLoad {
                scenario: "some_other_scenario".to_string(),
                seed: None,
            })
            .await;
        assert_eq!(result.status, crate::state::ControlEnvelopeStatus::Rejected);
        assert_eq!(result.reason.as_deref(), Some("scenario_swap_not_supported_in_m0"));
    }

    #[tokio::test]
    async fn step_zero_is_rejected_without_status_drift() {
        let scenario_path = write_test_scenario();
        let config = load_test_scenario_and_config(scenario_path);
        let engine = M0Engine::new(config);
        let before = engine.snapshot(None).await;
        let result = engine.dispatch(ControlCommand::Step { ticks: 0 }).await;
        let after = engine.snapshot(None).await;
        assert_eq!(result.status, crate::state::ControlEnvelopeStatus::Rejected);
        assert_eq!(result.reason.as_deref(), Some("ticks_must_be_positive"));
        assert_eq!(after.tick, before.tick, "step(0) must not advance the sim");
        assert_eq!(
            after.run_status, before.run_status,
            "step(0) must not leave observe.once reporting a fake Stepping state"
        );
    }

    #[tokio::test]
    async fn step_completion_observation_pauses_after_requested_ticks() {
        let scenario_path = write_test_scenario();
        let config = load_test_scenario_and_config(scenario_path);
        let engine = M0Engine::new(config);
        let result = engine.dispatch(ControlCommand::Step { ticks: 2 }).await;
        assert_eq!(result.status, crate::state::ControlEnvelopeStatus::Accepted);
        assert_eq!(engine.snapshot(None).await.run_status, RunStatus::Stepping);
        engine.drive_tick();
        assert_eq!(engine.snapshot(None).await.run_status, RunStatus::Stepping);
        engine.drive_tick();
        assert_eq!(
            engine.snapshot(None).await.run_status,
            RunStatus::Paused,
            "observe.once must reflect the SimClock after the requested step count completes"
        );
    }

    #[tokio::test]
    async fn run_for_zero_ticks_is_rejected_without_status_drift() {
        let scenario_path = write_test_scenario();
        let config = load_test_scenario_and_config(scenario_path);
        let engine = M0Engine::new(config);
        let before = engine.snapshot(None).await;
        let result = engine
            .dispatch(ControlCommand::RunForTicks {
                ticks: 0,
                write_run_bundle: true,
            })
            .await;
        let after = engine.snapshot(None).await;
        assert_eq!(result.status, crate::state::ControlEnvelopeStatus::Rejected);
        assert_eq!(result.reason.as_deref(), Some("ticks_must_be_positive"));
        assert_eq!(after.tick, before.tick);
        assert_eq!(after.run_status, before.run_status);
        assert!(
            !engine.pending_runbundle(),
            "rejected run_for_ticks(0) must not queue a run bundle"
        );
    }

    #[tokio::test]
    async fn act_player_move_rejects_until_m1_actor_exists() {
        let scenario_path = write_test_scenario();
        let config = load_test_scenario_and_config(scenario_path);
        let engine = M0Engine::new(config);
        let result = engine
            .dispatch(ControlCommand::ActPlayerMove {
                x: 1.0,
                y: 0.0,
                source: IntentSource::Cfctl,
            })
            .await;
        assert_eq!(result.status, crate::state::ControlEnvelopeStatus::Rejected);
        assert_eq!(result.reason.as_deref(), Some("act_player_move_not_available_in_m0"));
        let rejection = engine
            .recorder()
            .snapshot_events()
            .into_iter()
            .find(|event| event.category == "control" && event.event_type == "command_rejected")
            .expect("rejected act.player.move must record evidence");
        assert_eq!(rejection.payload["method"], "act.player.move");
        assert_eq!(rejection.payload["reason"], "act_player_move_not_available_in_m0");
    }

    #[tokio::test]
    async fn runbundle_id_override_is_rejected_until_supported() {
        let scenario_path = write_test_scenario();
        let config = load_test_scenario_and_config(scenario_path);
        let engine = M0Engine::new(config);
        let result = engine
            .dispatch(ControlCommand::RunBundleWrite {
                id_override: Some("manual-id".to_string()),
            })
            .await;
        assert_eq!(result.status, crate::state::ControlEnvelopeStatus::Rejected);
        assert_eq!(
            result.reason.as_deref(),
            Some("runbundle_id_override_not_supported_in_m0")
        );
        assert!(
            !engine.pending_runbundle(),
            "unsupported id_override must not queue a bundle write"
        );
    }

    #[test]
    fn tick_sample_event_emitted_at_cadence() {
        // M0.2-F4: every cadence_ticks (60 by default) the engine must emit a
        // `system.tick_sample` event with avg/max/p99 in ms and the configured tick rate.
        let scenario_path = write_test_scenario();
        let mut config = load_test_scenario_and_config(scenario_path);
        config.duration_ticks = 60;
        let engine = M0Engine::new(config);
        engine.record_run_started();
        for _ in 0..60 {
            engine.drive_tick();
        }
        let events = engine.recorder().snapshot_events();
        let samples: Vec<_> = events
            .iter()
            .filter(|e| e.category == "system" && e.event_type == "tick_sample")
            .collect();
        assert!(
            !samples.is_empty(),
            "system.tick_sample should fire at least once over 60 ticks @ cadence 60"
        );
        let payload = &samples[0].payload;
        assert_eq!(payload["tick_rate_hz"].as_u64(), Some(60));
        assert!(payload["avg_tick_ms"].is_number());
        assert!(payload["max_tick_ms"].is_number());
        assert!(payload["p99_tick_ms"].is_number());
        assert!(
            payload["samples_observed"].as_u64().unwrap_or(0) >= 1,
            "tick_sample must report at least one sample"
        );
    }

    #[test]
    fn very_short_run_still_has_final_checksum() {
        let root = temp_run_root();
        let scenario_path = write_test_scenario();
        let mut config = load_test_scenario_and_config(scenario_path);
        config.duration_ticks = 1; // shorter than cadence; pre-fix this produced final_sim_checksum=null.
        config.run_bundle_root = root.clone();
        config.write_run_bundle = true;
        config.run_mode = "test-tiny".to_string();
        let outcome = run_m0_inline(config).unwrap();
        assert!(
            outcome.final_checksum_hex.is_some(),
            "1-tick run must still emit a final checksum"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn run_m0_inline_records_tick_rate_120() {
        let root = temp_run_root();
        let scenario_path = write_test_scenario();
        let mut config = load_test_scenario_and_config(scenario_path);
        config.duration_ticks = 30;
        config.tick_rate_hz = 120;
        config.run_bundle_root = root.clone();
        config.write_run_bundle = true;
        config.run_mode = "test-120hz".to_string();
        let outcome = run_m0_inline(config).unwrap();
        let bundle = outcome.bundle_dir.unwrap();
        let manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(bundle.join("run_manifest.json")).unwrap()).unwrap();
        assert_eq!(manifest["tick_rate_hz"], 120);
        assert!((manifest["duration_target_sec"].as_f64().unwrap() - 0.25).abs() < 1e-6);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn run_m0_inline_paced_takes_real_time() {
        let root = temp_run_root();
        let scenario_path = write_test_scenario();
        let mut config = load_test_scenario_and_config(scenario_path);
        config.duration_ticks = 30;
        config.tick_rate_hz = 60;
        config.run_bundle_root = root.clone();
        config.write_run_bundle = false;
        config.run_mode = "test-paced".to_string();
        config.paced = true;
        let outcome = run_m0_inline(config).unwrap();
        // 30 ticks at 60 Hz = 0.5 s. Allow a small lower bound.
        assert!(
            outcome.wall_seconds >= 0.45,
            "paced run should be near 0.5 s wall, got {}",
            outcome.wall_seconds
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn settings_set_propagates_to_observe() {
        let scenario_path = write_test_scenario();
        let mut config = load_test_scenario_and_config(scenario_path);
        config.tick_rate_hz = 60;
        let engine = M0Engine::new(config);

        let s0 = engine.settings_snapshot().await;
        assert!((s0.ui_scale - 1.0).abs() < f32::EPSILON);

        let _ = engine
            .dispatch(ControlCommand::SettingsSet {
                changes: Box::new(SettingsPatch {
                    ui_scale: Some(2.0),
                    high_contrast: Some(true),
                    captions: Some(false),
                    ..SettingsPatch::default()
                }),
            })
            .await;
        let s1 = engine.settings_snapshot().await;
        assert!((s1.ui_scale - 2.0).abs() < f32::EPSILON);
        assert!(s1.high_contrast);
        assert!(!s1.captions);

        let frame = engine.snapshot(None).await;
        assert!((frame.settings.settings.ui_scale - 2.0).abs() < f32::EPSILON);
        assert!(frame.settings.settings.high_contrast);
    }

    #[tokio::test]
    async fn settings_set_clamps_ui_scale_before_observe() {
        let scenario_path = write_test_scenario();
        let config = load_test_scenario_and_config(scenario_path);
        let engine = M0Engine::new(config);

        let _ = engine
            .dispatch(ControlCommand::SettingsSet {
                changes: Box::new(SettingsPatch {
                    ui_scale: Some(0.01),
                    ..SettingsPatch::default()
                }),
            })
            .await;
        let low_settings = engine.settings_snapshot().await;
        assert!((low_settings.ui_scale - crate::settings::UI_SCALE_MIN).abs() < f32::EPSILON);
        let low_frame = engine.snapshot(None).await;
        assert!((low_frame.accessibility.ui_scale_applied - crate::settings::UI_SCALE_MIN).abs() < f32::EPSILON);

        let _ = engine
            .dispatch(ControlCommand::SettingsSet {
                changes: Box::new(SettingsPatch {
                    ui_scale: Some(99.0),
                    ..SettingsPatch::default()
                }),
            })
            .await;
        let high_settings = engine.settings_snapshot().await;
        assert!((high_settings.ui_scale - crate::settings::UI_SCALE_MAX).abs() < f32::EPSILON);
        let high_frame = engine.snapshot(None).await;
        assert!((high_frame.accessibility.ui_scale_applied - crate::settings::UI_SCALE_MAX).abs() < f32::EPSILON);
    }

    #[test]
    fn config_hash_is_stable_for_inputs() {
        let scenario_path = PathBuf::from("/tmp/scenario.ron");
        let mut a = M0EngineConfig::for_test_scenario_only("m0_blank", scenario_path.clone());
        let mut b = M0EngineConfig::for_test_scenario_only("m0_blank", scenario_path);
        a.fill_config_hash();
        b.fill_config_hash();
        assert_eq!(a.config_hash, b.config_hash);
        assert!(!a.config_hash.is_empty());
    }

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

    #[tokio::test]
    async fn m1_act_player_move_updates_pending_intent_and_emits_input_event() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();

        let result = engine
            .dispatch(ControlCommand::ActPlayerMove {
                x: 1.0,
                y: 0.0,
                source: IntentSource::Cfctl,
            })
            .await;
        assert_eq!(result.status, crate::state::ControlEnvelopeStatus::Accepted);
        engine.drive_tick();
        let events = engine.recorder().snapshot_events();
        let intent = events
            .iter()
            .find(|e| e.category == "input" && e.event_type == "intent_received")
            .expect("input.intent_received must be recorded");
        assert!((intent.payload["move_x"].as_f64().unwrap() - 1.0).abs() < 1e-3);
    }

    #[tokio::test]
    async fn m1_act_player_fire_spawns_projectile_event() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();

        let result = engine
            .dispatch(ControlCommand::ActPlayerFire {
                pressed: true,
                ammo_kind: None,
                source: IntentSource::Cfctl,
            })
            .await;
        assert_eq!(result.status, crate::state::ControlEnvelopeStatus::Accepted);
        engine.drive_tick();
        let events = engine.recorder().snapshot_events();
        assert!(
            events
                .iter()
                .any(|e| e.category == "equipment" && e.event_type == "weapon_fired"),
            "weapon_fired must land in events: {:?}",
            events
                .iter()
                .map(|e| (e.category.clone(), e.event_type.clone()))
                .collect::<Vec<_>>()
        );
        assert!(
            events
                .iter()
                .any(|e| e.category == "combat" && e.event_type == "projectile_spawned"),
            "projectile_spawned must land in events"
        );
    }

    #[tokio::test]
    async fn m1_act_player_fire_release_preserves_queued_press() {
        // Regression proof for the cf-app keyboard bridge contract: key release sends
        // `pressed: false` so future hold-to-fire weapons can observe release edges.
        // M1's rifle is press-edge driven, so release must be accepted but must not
        // erase a still-unconsumed press before the next fixed tick drains the intent.
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();

        let press = engine
            .dispatch(ControlCommand::ActPlayerFire {
                pressed: true,
                ammo_kind: None,
                source: IntentSource::Human,
            })
            .await;
        assert_eq!(press.status, crate::state::ControlEnvelopeStatus::Accepted);

        let release = engine
            .dispatch(ControlCommand::ActPlayerFire {
                pressed: false,
                ammo_kind: None,
                source: IntentSource::Human,
            })
            .await;
        assert_eq!(
            release.status,
            crate::state::ControlEnvelopeStatus::Accepted,
            "explicit fire release must stay a valid command"
        );

        engine.drive_tick();
        let events = engine.recorder().snapshot_events();
        let intent = events
            .iter()
            .find(|e| e.category == "input" && e.event_type == "intent_received")
            .expect("input.intent_received must be recorded after press+release");
        assert_eq!(
            intent.payload.get("source").and_then(|v| v.as_str()),
            Some("human"),
            "same-tick press+release should retain the human source"
        );
        assert_eq!(
            intent.payload.get("fire").and_then(|v| v.as_bool()),
            Some(true),
            "release must not clobber the queued fire edge before drive_tick"
        );
        assert!(
            events
                .iter()
                .any(|e| e.category == "equipment" && e.event_type == "weapon_fired"),
            "queued press must still fire after same-tick release; events: {:?}",
            events
                .iter()
                .map(|e| (e.category.clone(), e.event_type.clone(), e.payload.clone()))
                .collect::<Vec<_>>()
        );
        assert!(
            events
                .iter()
                .any(|e| e.category == "combat" && e.event_type == "projectile_spawned"),
            "queued press must still spawn a projectile after same-tick release"
        );
    }

    #[tokio::test]
    async fn m1_act_player_aim_normalizes_and_records_event() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();

        let _ = engine
            .dispatch(ControlCommand::ActPlayerAim {
                x: 0.0,
                y: 1.0,
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        let frame = engine.snapshot(None).await;
        let player = frame
            .actors
            .iter()
            .find(|a| Some(a.id) == frame.player_actor_id)
            .unwrap();
        // Aim normalized to unit vector (0, 1).
        assert!((player.aim[1] - 1.0).abs() < 1e-3);
    }

    #[tokio::test]
    async fn m1_act_player_jump_rejected_in_air_recorded() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();

        // First jump from spawn (above ground) — actor is NOT on_ground until physics
        // drops it, so the first jump is refused. Tick a few times so the actor lands.
        for _ in 0..6 {
            engine.drive_tick();
        }
        // Now jump should succeed.
        let _ = engine
            .dispatch(ControlCommand::ActPlayerJump {
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        let events = engine.recorder().snapshot_events();
        let jumped = events
            .iter()
            .any(|e| e.category == "actor" && e.event_type == "actor_jumped");
        assert!(jumped, "actor_jumped should land after the actor settles on the floor");
    }

    #[tokio::test]
    async fn m1_act_player_reset_emits_actor_reset_event() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();

        let _ = engine
            .dispatch(ControlCommand::ActPlayerReset {
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        let events = engine.recorder().snapshot_events();
        assert!(events.iter().any(|e| e.event_type == "actor_reset"));
    }

    #[tokio::test]
    async fn m1_act_player_select_item_changes_slot_in_observation() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();
        let _ = engine
            .dispatch(ControlCommand::ActPlayerSelectItem {
                slot: 1,
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        let frame = engine.snapshot(None).await;
        let player = frame
            .actors
            .iter()
            .find(|a| Some(a.id) == frame.player_actor_id)
            .unwrap();
        assert_eq!(player.selected_slot, 1);
    }

    #[tokio::test]
    async fn m1_actor_render_snapshot_hides_rifle_when_non_rifle_slot_selected() {
        // M1-FIX-9 regression: actor_render_snapshot() must clear player_rifle when
        // the player's currently-selected slot is not a rifle, so the HUD shows
        // "NO RIFLE" instead of READY/COOLDOWN.
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();
        // Default selection (slot 0 = rifle) - HUD should show rifle.
        let snap_a = engine.actor_render_snapshot();
        assert!(snap_a.player_rifle.is_some(), "rifle slot selected -> HUD shows rifle");
        // Select an empty slot.
        let _ = engine
            .dispatch(ControlCommand::ActPlayerSelectItem {
                slot: 1,
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        let snap_b = engine.actor_render_snapshot();
        assert!(
            snap_b.player_rifle.is_none(),
            "non-rifle slot -> HUD hides rifle (NO RIFLE)"
        );
        // Switch back to slot 0.
        let _ = engine
            .dispatch(ControlCommand::ActPlayerSelectItem {
                slot: 0,
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        let snap_c = engine.actor_render_snapshot();
        assert!(snap_c.player_rifle.is_some(), "back to slot 0 -> HUD shows rifle again");
    }

    #[tokio::test]
    async fn m1_observe_actor_view_hides_rifle_state_when_non_rifle_slot_selected() {
        // Mirrors `m1_actor_render_snapshot_hides_rifle_when_non_rifle_slot_selected` for
        // the wire-format `ActorView` exposed via `observe.once` / `observe.subscribe`.
        // The cfctl/replay/AI consumers must see the same NO RIFLE state the player sees
        // in the HUD; otherwise external observers mis-attribute fire-press behavior.
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();
        // Default selection (slot 0 = rifle) - ActorView must show rifle fields.
        let frame_a = engine.snapshot(None).await;
        let player_a = frame_a
            .actors
            .iter()
            .find(|a| Some(a.id) == frame_a.player_actor_id)
            .unwrap();
        assert!(
            player_a.rifle_ammo.is_some(),
            "rifle slot selected -> rifle_ammo populated"
        );
        assert!(player_a.rifle_capacity.is_some());
        assert!(
            player_a.rifle_reload_total_ticks.is_some(),
            "rifle slot selected -> reload total is visible to cfctl/AI observers"
        );

        let _ = engine
            .dispatch(ControlCommand::ActPlayerFire {
                pressed: true,
                ammo_kind: None,
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        let _ = engine
            .dispatch(ControlCommand::ActPlayerReload {
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        let frame_reload = engine.snapshot(None).await;
        let player_reload = frame_reload
            .actors
            .iter()
            .find(|a| Some(a.id) == frame_reload.player_actor_id)
            .unwrap();
        assert!(
            player_reload
                .rifle_reload_remaining_ticks
                .is_some_and(|ticks| ticks > 0),
            "reload command should expose remaining reload ticks"
        );
        assert_eq!(
            player_reload.rifle_reload_total_ticks,
            Some(90),
            "M1 rifle reload is 1.5s at the 60 Hz test default"
        );

        // Select an empty slot.
        let _ = engine
            .dispatch(ControlCommand::ActPlayerSelectItem {
                slot: 1,
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        let frame_b = engine.snapshot(None).await;
        let player_b = frame_b
            .actors
            .iter()
            .find(|a| Some(a.id) == frame_b.player_actor_id)
            .unwrap();
        assert!(
            player_b.rifle_ammo.is_none(),
            "non-rifle slot -> rifle_ammo must be None on the wire"
        );
        assert!(player_b.rifle_capacity.is_none());
        assert!(player_b.rifle_fire_cooldown_ticks.is_none());
        assert!(player_b.rifle_reload_remaining_ticks.is_none());
        assert!(player_b.rifle_reload_total_ticks.is_none());
        // Re-select rifle slot 0.
        let _ = engine
            .dispatch(ControlCommand::ActPlayerSelectItem {
                slot: 0,
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        let frame_c = engine.snapshot(None).await;
        let player_c = frame_c
            .actors
            .iter()
            .find(|a| Some(a.id) == frame_c.player_actor_id)
            .unwrap();
        assert!(
            player_c.rifle_ammo.is_some(),
            "back to slot 0 -> rifle_ammo populated again"
        );
        assert_eq!(player_c.rifle_reload_total_ticks, Some(90));
    }

    #[tokio::test]
    async fn m1_actor_snapshot_event_emitted_at_cadence() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();
        for _ in 0..60 {
            engine.drive_tick();
        }
        let events = engine.recorder().snapshot_events();
        assert!(events
            .iter()
            .any(|e| e.category == "actor" && e.event_type == "actor_snapshot"));
    }

    #[tokio::test]
    async fn m1_observe_includes_actor_view_with_rifle_state() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();
        let frame = engine.snapshot(None).await;
        assert!(!frame.actors.is_empty());
        let player = frame
            .actors
            .iter()
            .find(|a| Some(a.id) == frame.player_actor_id)
            .unwrap();
        assert_eq!(player.rifle_capacity, Some(30));
        assert_eq!(player.rifle_ammo, Some(30));
    }

    #[tokio::test]
    async fn m1_dead_player_rejects_movement_input() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();
        // Force player into Dead status by directly mutating world state via reset-then-damage.
        {
            let mut state = engine.state.write().unwrap();
            if let Some(sim) = state.actor_state.as_mut() {
                let player = sim.world.player_actor_mut().unwrap();
                let _ = player.apply_damage(1000.0);
            }
        }
        let _ = engine
            .dispatch(ControlCommand::ActPlayerMove {
                x: 1.0,
                y: 0.0,
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        // Actor should not accept input. CCCP Actor.cpp:1229 — HP=0 enters
        // DYING (the death animation dwell window). Either DYING or DEAD
        // refuses input.
        let frame = engine.snapshot(None).await;
        let player = frame
            .actors
            .iter()
            .find(|a| Some(a.id) == frame.player_actor_id)
            .unwrap();
        assert!(
            player.status == "dying" || player.status == "dead",
            "expected dying or dead, got {}",
            player.status
        );
    }

    #[tokio::test]
    async fn m1_scenario_reset_rebuilds_actor_world() {
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();
        // Move + fire to mutate state.
        let _ = engine
            .dispatch(ControlCommand::ActPlayerMove {
                x: 1.0,
                y: 0.0,
                source: IntentSource::Cfctl,
            })
            .await;
        let _ = engine
            .dispatch(ControlCommand::ActPlayerFire {
                pressed: true,
                ammo_kind: None,
                source: IntentSource::Cfctl,
            })
            .await;
        engine.drive_tick();
        // Reset.
        let _ = engine.dispatch(ControlCommand::ScenarioReset).await;
        let frame = engine.snapshot(None).await;
        let player = frame
            .actors
            .iter()
            .find(|a| Some(a.id) == frame.player_actor_id)
            .unwrap();
        // After reset, the actor is at spawn (200, 32) with full ammo.
        assert!((player.position[0] - 200.0).abs() < 0.5);
        assert_eq!(player.rifle_ammo, Some(30));
    }

    #[tokio::test]
    async fn m1_scenario_reset_preserves_intent_source() {
        // Regression: ScenarioReset rebuilt pending_intent with a hardcoded
        // IntentSource::Cfctl regardless of who was previously controlling the actor.
        // Now we preserve the pre-reset source so the next idle tick's
        // input.intent_received correctly attributes (cfctl OR human) and the
        // replay event log doesn't contain spurious source flips on reset.
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();
        // Drive a Human-source aim so pending_intent.source = Human.
        let _ = engine
            .dispatch(ControlCommand::ActPlayerAim {
                x: 1.0,
                y: 0.0,
                source: IntentSource::Human,
            })
            .await;
        // Now reset — pre-fix this would clobber source back to Cfctl.
        let _ = engine.dispatch(ControlCommand::ScenarioReset).await;
        // Next tick should record input.intent_received with source = human.
        engine.drive_tick();
        let events = engine.recorder().snapshot_events();
        let intent_events: Vec<_> = events
            .iter()
            .filter(|e| e.category == "input" && e.event_type == "intent_received")
            .collect();
        let last_intent = intent_events.last().expect("at least one intent_received event");
        assert_eq!(
            last_intent.payload.get("source").and_then(|v| v.as_str()),
            Some("human"),
            "post-reset intent must preserve the Human source",
        );
    }

    #[tokio::test]
    async fn m1_act_player_aim_accepts_finite_at_engine_layer() {
        // Sanity: with finite values, engine dispatch accepts aim.
        let path = write_m1_scenario();
        let config = load_m1_test_config(path);
        let engine = M0Engine::new(config);
        engine.record_run_started();
        let result = engine
            .dispatch(ControlCommand::ActPlayerAim {
                x: 1.0,
                y: 0.0,
                source: IntentSource::Cfctl,
            })
            .await;
        assert_eq!(result.status, crate::state::ControlEnvelopeStatus::Accepted);
    }

    #[tokio::test]
    async fn m1_act_player_aim_rejects_nonfinite_at_engine_layer() {
        // Defense-in-depth: the JSON-RPC server layer rejects NaN/Inf before dispatch
        // (see live_ws_m1_act_player_aim_nan_rejected). The engine ALSO rejects at the
        // dispatch boundary so any future caller (cf-app keyboard bridge, future mouse
        // bridge, future gamepad bridge, future direct-dispatch script) cannot leak
        // NaN/Inf into pending_intent and NaN-poison the muzzle / projectile path.
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
                .dispatch(ControlCommand::ActPlayerAim {
                    x,
                    y,
                    source: IntentSource::Cfctl,
                })
                .await;
            assert_eq!(
                result.status,
                crate::state::ControlEnvelopeStatus::Rejected,
                "aim ({x}, {y}) must reject"
            );
            assert_eq!(result.reason.as_deref(), Some("non_finite"));
        }
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
