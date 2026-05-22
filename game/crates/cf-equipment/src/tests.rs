use super::*;

use crate::presets::rifle_m1_default;

fn rifle() -> RifleState {
    rifle_at(60)
}

fn rifle_at(tick_rate_hz: u32) -> RifleState {
    RifleState::new(rifle_preset(RIFLE_M1_DEFAULT_ID).expect("default preset"), tick_rate_hz)
}

#[test]
fn rifle_starts_loaded_and_ready() {
    let r = rifle();
    let spec = rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap();
    assert!(r.ready_to_fire());
    assert_eq!(r.ammo_in_mag, spec.mag_capacity);
}

#[test]
fn fire_decrements_ammo_and_starts_cooldown() {
    let mut r = rifle();
    let cooldown = r.fire_interval_ticks();
    let mag = r.spec.mag_capacity;
    let outcomes = tick_rifle(
        &mut r,
        RifleTickInputs {
            fire_pressed: true,
            ..Default::default()
        },
    );
    assert!(outcomes.fired_this_tick);
    assert_eq!(r.ammo_in_mag, mag - 1);
    assert_eq!(r.fire_cooldown_ticks, cooldown);
}

#[test]
fn cannot_fire_during_cooldown() {
    let mut r = rifle();
    let mag = r.spec.mag_capacity;
    let _ = tick_rifle(
        &mut r,
        RifleTickInputs {
            fire_pressed: true,
            ..Default::default()
        },
    );
    let blocked = tick_rifle(
        &mut r,
        RifleTickInputs {
            fire_pressed: true,
            ..Default::default()
        },
    );
    assert!(!blocked.fired_this_tick);
    assert_eq!(r.ammo_in_mag, mag - 1);
}

#[test]
fn dry_fire_when_empty() {
    let mut r = rifle();
    let mag = r.spec.mag_capacity;
    let cooldown = r.fire_interval_ticks();
    for _ in 0..mag {
        for _ in 0..cooldown {
            let _ = tick_rifle(
                &mut r,
                RifleTickInputs {
                    fire_pressed: false,
                    ..Default::default()
                },
            );
        }
        let _ = tick_rifle(
            &mut r,
            RifleTickInputs {
                fire_pressed: true,
                ..Default::default()
            },
        );
    }
    assert_eq!(r.ammo_in_mag, 0);
    let outcomes = tick_rifle(
        &mut r,
        RifleTickInputs {
            fire_pressed: true,
            ..Default::default()
        },
    );
    assert!(outcomes.dry_fire);
    assert!(!outcomes.fired_this_tick);
}

#[test]
fn reload_takes_full_duration() {
    let mut r = rifle();
    let mag = r.spec.mag_capacity;
    let reload = r.reload_ticks();
    let _ = tick_rifle(
        &mut r,
        RifleTickInputs {
            fire_pressed: true,
            ..Default::default()
        },
    );
    let started = tick_rifle(
        &mut r,
        RifleTickInputs {
            reload_pressed: true,
            ..Default::default()
        },
    );
    assert!(started.reload_started);
    for _ in 0..(reload - 1) {
        let _ = tick_rifle(&mut r, RifleTickInputs::default());
        assert!(r.is_reloading());
    }
    let completion = tick_rifle(&mut r, RifleTickInputs::default());
    assert!(completion.reload_completed);
    assert_eq!(r.ammo_in_mag, mag);
    assert!(!r.is_reloading());
}

#[test]
fn auto_reload_when_empty_starts_after_dry_fire() {
    let mut r = rifle();
    r.ammo_in_mag = 0;
    let outcomes = tick_rifle(
        &mut r,
        RifleTickInputs {
            fire_pressed: false,
            reload_pressed: false,
            auto_reload_when_empty: true,
        },
    );
    assert!(outcomes.reload_started);
    assert!(r.is_reloading());
}

#[test]
fn reset_returns_full_mag() {
    let spec = rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap();
    let mut r = rifle();
    r.ammo_in_mag = 5;
    r.fire_cooldown_ticks = 3;
    r.reload_remaining_ticks = 30;
    r.reset();
    assert_eq!(r.ammo_in_mag, spec.mag_capacity);
    assert_eq!(r.fire_cooldown_ticks, 0);
    assert_eq!(r.reload_remaining_ticks, 0);
}

#[test]
fn rifle_preset_lookup() {
    assert!(rifle_preset(RIFLE_M1_DEFAULT_ID).is_some());
    assert!(rifle_preset(CARBINE_M5_POWERED_ID).is_some());
    assert!(rifle_preset(RIFLE_M5_MECH_HEAVY_ID).is_some());
    assert!(rifle_preset("nonexistent").is_none());
}

#[test]
fn role_record_registry_covers_every_rifle_preset() {
    for preset_id in [RIFLE_M1_DEFAULT_ID, CARBINE_M5_POWERED_ID, RIFLE_M5_MECH_HEAVY_ID] {
        let r = role_record(preset_id).unwrap_or_else(|| panic!("role record for {preset_id}"));
        assert_eq!(r.id, preset_id);
        assert!(r.firing.is_some(), "role {preset_id} must carry firing data");
        assert!(r.tutorial_safe, "M5 LOAD-A roles default to tutorial-safe");
    }
}

#[test]
fn role_record_origin_compatibility_default_is_universal() {
    let r = role_record(RIFLE_M1_DEFAULT_ID).unwrap();
    assert!(r.compatible_with_origin("human"));
    assert!(r.compatible_with_origin("robot"));
    assert!(r.compatible_with_origin("android"));
}

#[test]
fn loadout_registry_resolves_canonical_load_a_ids() {
    assert!(loadout("load_a_infantry").is_some());
    assert!(loadout("load_a_powered_armor").is_some());
    assert!(loadout("load_a_light_mech").is_some());
    assert!(loadout("missing").is_none());
}

#[test]
fn load_loadout_from_json_round_trips_canonical_payload() {
    let json = r#"{
        "schema_version": 1,
        "id": "load_a_infantry",
        "display_name": "Infantry Standard",
        "role_ids": ["rifle_m1_default"],
        "provenance": "spec/equipment-loadout#LOAD-A.infantry"
    }"#;
    let loaded = load_loadout_from_json(json, Some("load_a_infantry")).unwrap();
    assert_eq!(loaded.id, "load_a_infantry");
    assert_eq!(loaded.role_ids, vec!["rifle_m1_default".to_string()]);
    // Mirrors the hardcoded fixture in [`loadouts`] so a scenario that
    // swaps between JSON and Rust-seed paths stays identical.
    let canonical = loadout("load_a_infantry").unwrap();
    assert_eq!(loaded, canonical);
}

#[test]
fn load_loadout_from_json_rejects_schema_drift() {
    let json = r#"{
        "schema_version": 99,
        "id": "load_a_infantry",
        "display_name": "x",
        "role_ids": ["rifle_m1_default"],
        "provenance": "p"
    }"#;
    let err = load_loadout_from_json(json, None).unwrap_err();
    assert!(matches!(err, LoadoutLoadError::SchemaVersionMismatch { .. }));
}

#[test]
fn load_loadout_from_json_rejects_unknown_role() {
    let json = r#"{
        "schema_version": 1,
        "id": "load_a_infantry",
        "display_name": "x",
        "role_ids": ["nonexistent_role"],
        "provenance": "p"
    }"#;
    let err = load_loadout_from_json(json, None).unwrap_err();
    match err {
        LoadoutLoadError::UnknownRoleId(id) => assert_eq!(id, "nonexistent_role"),
        other => panic!("expected UnknownRoleId, got {other:?}"),
    }
}

#[test]
fn load_loadout_from_json_rejects_id_mismatch() {
    let json = r#"{
        "schema_version": 1,
        "id": "load_a_infantry",
        "display_name": "x",
        "role_ids": ["rifle_m1_default"],
        "provenance": "p"
    }"#;
    let err = load_loadout_from_json(json, Some("not_matching")).unwrap_err();
    assert!(matches!(err, LoadoutLoadError::IdMismatch { .. }));
}

#[test]
fn load_loadout_from_json_rejects_empty_role_ids() {
    let json = r#"{
        "schema_version": 1,
        "id": "load_a_infantry",
        "display_name": "x",
        "role_ids": [],
        "provenance": "p"
    }"#;
    let err = load_loadout_from_json(json, None).unwrap_err();
    assert!(matches!(err, LoadoutLoadError::EmptyRoleIds));
}

#[test]
fn load_loadouts_from_dir_loads_canonical_load_a_files() {
    // The canonical content directory lives under the workspace `game/`
    // root. The crate runs from `game/crates/cf-equipment` so the
    // relative path back up is `../../content/equipment/loadouts`.
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("content/equipment/loadouts");
    if !dir.exists() {
        // Skip cleanly if the content dir hasn't been checked out — the
        // test is informational on standalone crate checks.
        return;
    }
    let registry = load_loadouts_from_dir(&dir).expect("loadouts dir parses");
    for required in ["load_a_infantry", "load_a_powered_armor", "load_a_light_mech"] {
        assert!(
            registry.contains_key(required),
            "loadouts dir missing canonical id {required}"
        );
        // Each loaded loadout must agree with the in-Rust seed fixture so
        // both code paths produce identical runtime objects.
        let canonical = loadout(required).unwrap();
        let on_disk = registry.get(required).unwrap();
        assert_eq!(on_disk, &canonical, "{required} drift");
    }
}

#[test]
fn rifle_spec_roundtrips_through_role_record() {
    let r = role_record(CARBINE_M5_POWERED_ID).unwrap();
    let firing = r.firing.clone().unwrap();
    let spec = firing.into_rifle_spec(r.id.clone());
    assert!((spec.fire_interval_seconds - 0.083).abs() < 1e-6);
    assert_eq!(spec.mag_capacity, 25);
}

#[test]
fn jam_chance_clamped_to_unit_range() {
    let r = RoleRecord::from_rifle_spec(
        &rifle_m1_default(),
        RoleKind::Rifle,
        AiPolicyHint::Primary,
        "Test",
        "test",
        5.0,
        3.5,
    );
    assert!((r.jam_chance_per_shot - 1.0).abs() < 1e-6);
}

#[test]
fn timings_scale_with_tick_rate() {
    // 10 RPS / 1.5 s reload / 1.5 s flight at the canonical M1 preset.
    let spec = rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap();
    // 60 Hz: 6 / 90 / 90.
    assert_eq!(spec.fire_interval_ticks(60), 6);
    assert_eq!(spec.reload_ticks(60), 90);
    assert_eq!(spec.projectile_max_flight_ticks(60), 90);
    // 120 Hz: 12 / 180 / 180.
    assert_eq!(spec.fire_interval_ticks(120), 12);
    assert_eq!(spec.reload_ticks(120), 180);
    assert_eq!(spec.projectile_max_flight_ticks(120), 180);
    // RifleState resolves the same values via its configured tick_rate_hz.
    let r60 = rifle_at(60);
    let r120 = rifle_at(120);
    assert_eq!(r60.fire_interval_ticks(), 6);
    assert_eq!(r120.fire_interval_ticks(), 12);
    assert_eq!(r60.reload_ticks(), 90);
    assert_eq!(r120.reload_ticks(), 180);
}

#[test]
fn semi_mode_fires_once_per_press_even_when_held() {
    let mut r = rifle();
    assert_eq!(r.spec.fire_mode, FireMode::Semi);
    // Press + hold for many ticks: must produce exactly one shot.
    let first = tick_rifle(
        &mut r,
        RifleTickInputs {
            fire_pressed: true,
            ..Default::default()
        },
    );
    assert!(first.fired_this_tick);
    let mut shots_while_held = 0;
    for _ in 0..60 {
        let outcomes = tick_rifle(
            &mut r,
            RifleTickInputs {
                fire_pressed: true,
                ..Default::default()
            },
        );
        if outcomes.fired_this_tick {
            shots_while_held += 1;
        }
    }
    assert_eq!(
        shots_while_held, 0,
        "Semi must NOT auto-repeat while held; got {shots_while_held} extra shots"
    );
    // Releasing + re-pressing fires again.
    let _release = tick_rifle(&mut r, RifleTickInputs::default());
    let second = tick_rifle(
        &mut r,
        RifleTickInputs {
            fire_pressed: true,
            ..Default::default()
        },
    );
    assert!(second.fired_this_tick, "Semi must fire on a fresh press after release");
}

#[test]
fn full_auto_mode_fires_at_cadence_while_held() {
    let preset = rifle_preset(CARBINE_M5_POWERED_ID).unwrap();
    assert_eq!(preset.fire_mode, FireMode::FullAuto);
    let mut r = RifleState::new(preset, 60);
    let mut shots = 0;
    for _ in 0..120 {
        let outcomes = tick_rifle(
            &mut r,
            RifleTickInputs {
                fire_pressed: true,
                ..Default::default()
            },
        );
        if outcomes.fired_this_tick {
            shots += 1;
        }
    }
    // ~12 RPS × 2 s window = ~24 shots; clamp by mag (25). Should be > 1.
    assert!(shots > 1, "FullAuto must auto-repeat while held; only {shots} shot(s)");
}

#[test]
fn tracer_cadence_one_in_four_yields_three_tracers_in_twelve_shots() {
    let preset = rifle_preset(RIFLE_M1_TRACER_ID).unwrap();
    assert_eq!(preset.tracer_round_to_total_ratio, 4);
    let mut r = RifleState::new(preset, 60);
    let mut tracer_count = 0;
    let mut shots = 0;
    let cooldown = r.fire_interval_ticks();
    while shots < 12 {
        // Release for one tick to clear the Semi latch (preset is Semi).
        let _ = tick_rifle(&mut r, RifleTickInputs::default());
        for _ in 0..cooldown {
            let _ = tick_rifle(&mut r, RifleTickInputs::default());
        }
        let outcomes = tick_rifle(
            &mut r,
            RifleTickInputs {
                fire_pressed: true,
                ..Default::default()
            },
        );
        if outcomes.fired_this_tick {
            if outcomes.fired_is_tracer {
                tracer_count += 1;
            }
            shots += 1;
        }
    }
    assert_eq!(tracer_count, 3, "12 shots @ ratio 4 must yield exactly 3 tracers");
}

#[test]
fn fire_rate_real_time_equivalent_at_60hz_and_120hz() {
    // Drive both 60 Hz and 120 Hz FullAuto rifles for the same wall-clock
    // window and assert the same number of shots fired. Uses the carbine
    // preset (FullAuto, ~12 RPS) since the default rifle is Semi.
    fn shots_in_window(tick_rate_hz: u32, ticks: u32) -> u32 {
        let preset = rifle_preset(CARBINE_M5_POWERED_ID).unwrap();
        let mut r = RifleState::new(preset, tick_rate_hz);
        let mut shots = 0;
        for _ in 0..ticks {
            let outcomes = tick_rifle(
                &mut r,
                RifleTickInputs {
                    fire_pressed: true,
                    ..Default::default()
                },
            );
            if outcomes.fired_this_tick {
                shots += 1;
            }
        }
        shots
    }
    let shots_60 = shots_in_window(60, 60);
    let shots_120 = shots_in_window(120, 120);
    // FullAuto cadence: same wall-clock window = same shot count cross-rate.
    assert_eq!(shots_60, shots_120, "FullAuto cadence must hold across tick rates");
    // Carbine: 0.083s fire interval ≈ 12 RPS. 1.0 s ≈ 12 shots.
    assert!(
        (11..=13).contains(&shots_60),
        "expected ~12 RPS for carbine FullAuto, got {shots_60} at 60 Hz"
    );
}
