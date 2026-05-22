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

}
