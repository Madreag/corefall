//! **M14B** § Gravity field + wind force + gas stratification producer
//! acceptance tests.
//!
//! Each test mirrors one Gherkin scenario from
//! `specs/active/M14B.md` § Acceptance criteria.

use cf_atmos::{stratify, wind_force_at, AtmosCell, Gas, StratCell, WindSource};
use cf_control::Scenario;
use cf_mission::ScenarioGravityOverride;
use cf_physics::{apply_overrides, GravityField, GravityOverride, GravityVec};

// ----------------------------------------------------------------------------
// Scenario 1: GravityField::sample returns vector with magnitude + direction
// ----------------------------------------------------------------------------

#[test]
fn gravity_field_sample_returns_vector_with_magnitude_and_direction() {
    let g = GravityField::Layered {
        magnitude: 9.81,
        direction: [0.0, -1.0],
    };
    let v = g.sample([100.0, 50.0]);
    assert!((v.magnitude - 9.81).abs() < 1e-6);
    assert!((v.direction[0]).abs() < 1e-6);
    assert!((v.direction[1] - -1.0).abs() < 1e-6);
}

#[test]
fn gravity_field_sample_is_deterministic_across_1000_ticks() {
    let g = GravityField::Uniform(-980.0);
    let first = g.sample([100.0, 50.0]);
    for tick in 0..1024_u32 {
        let v = g.sample([100.0 + (tick as f32 * 0.0), 50.0]);
        assert_eq!(v, first);
    }
}

// ----------------------------------------------------------------------------
// Scenario 2: Gravity well override bends actor trajectory
// ----------------------------------------------------------------------------

#[test]
fn gravity_well_bends_actor_trajectory() {
    let base = GravityVec::new(9.81, [0.0, -1.0]);
    let overrides = vec![GravityOverride::UniformWell {
        id: 1,
        center: [200.0, 100.0],
        radius: 50.0,
        magnitude: 25.0,
    }];
    // Actor at tangent — 10 px to the right of the well center.
    let result = apply_overrides(base, [210.0, 100.0], Some(1), &overrides);
    // Direction must gain a leftward (negative x) component pointing
    // toward the well center.
    assert!(
        result.gravity.direction[0] < -0.1,
        "expected leftward bend: {:?}",
        result.gravity
    );
    assert!(result.active_ids.contains(&1));
}

// ----------------------------------------------------------------------------
// Scenario 3: Low-g lab cell doubles jump height
// ----------------------------------------------------------------------------

#[test]
fn low_g_lab_cell_doubles_jump_height() {
    let base = GravityVec::new(9.81, [0.0, -1.0]);
    let overrides = vec![GravityOverride::RegionLowG {
        id: 7,
        min: [0.0, 0.0],
        max: [100.0, 100.0],
        local_g: 4.9,
    }];
    let inside = apply_overrides(base, [50.0, 50.0], Some(1), &overrides);
    // Peak vertical displacement scales inversely with gravity for the
    // same jump impulse: peak_y = (v0² / 2g). low_g/normal_g = 9.81/4.9
    // ≈ 2× ± 5%.
    let ratio = base.magnitude / inside.gravity.magnitude;
    assert!(
        (ratio - 2.0).abs() / 2.0 < 0.05,
        "expected ~2× jump height in low-g: ratio={ratio}"
    );
}

// ----------------------------------------------------------------------------
// Scenario 4: Magnetic boots cancel grav override locally
// ----------------------------------------------------------------------------

#[test]
fn magnetic_boots_cancel_grav_override_locally() {
    let base = GravityVec::new(9.81, [0.0, -1.0]);
    let overrides = vec![
        GravityOverride::RegionLowG {
            id: 1,
            min: [0.0, 0.0],
            max: [100.0, 100.0],
            local_g: 4.9,
        },
        GravityOverride::MagneticBoots { id: 2, actor_id: 99 },
    ];
    // Anchored actor: local gravity returns to baseline 9.81 even
    // though the cell is low-g.
    let anchored = apply_overrides(base, [50.0, 50.0], Some(99), &overrides);
    assert!((anchored.gravity.magnitude - 9.81).abs() < 1e-6);
    // The activation event should include the magnetic_boots id so
    // cf-ui can render the "MAGNETIC ANCHOR" banner.
    assert!(anchored.active_ids.contains(&2));
    // Unanchored actor: still gets low-g.
    let unanchored = apply_overrides(base, [50.0, 50.0], Some(1), &overrides);
    assert!((unanchored.gravity.magnitude - 4.9).abs() < 1e-6);
    assert!(!unanchored.active_ids.contains(&2));
}

// ----------------------------------------------------------------------------
// Scenario 5: Wind from pressure differential applies actor impulse
// ----------------------------------------------------------------------------

#[test]
fn wind_from_pressure_differential_applies_actor_impulse() {
    let cells = vec![
        AtmosCell {
            id: 1,
            min: [0.0, 0.0],
            max: [10.0, 10.0],
            pressure_kpa: 101.0,
            temp_k: 293.15,
        },
        AtmosCell {
            id: 2,
            min: [10.0, 0.0],
            max: [20.0, 10.0],
            pressure_kpa: 96.0,
            temp_k: 293.15,
        },
    ];
    let sources = vec![WindSource {
        id: 1,
        origin: [10.0, 5.0],
        axis: [1.0, 0.0],
        aperture_area_m2: 0.5,
        cell_high_id: 1,
        cell_low_id: 2,
        jet_length: 10.0,
        jet_half_width: 1.0,
    }];
    // Actor in cell_B's flow path receives a positive-x (toward
    // cell_low) force.
    let out = wind_force_at([12.0, 5.0], &cells, &sources);
    assert!(out.force_n[0] > 0.0, "actor pushed toward low cell: {:?}", out);
    // Force proportional to (5 kPa × 0.5 m² × constant) — about 5 N
    // per our calibration.
    assert!(out.force_n[0] > 1.0);
    assert!(out.source_aperture_id == Some(1));
}

// ----------------------------------------------------------------------------
// Scenario 6: CO2 sinks to floor; H2 rises to ceiling
// ----------------------------------------------------------------------------

#[test]
fn co2_sinks_to_floor_and_h2_rises_to_ceiling() {
    let mut cells = (0..16_u32)
        .map(|i| StratCell {
            cell_id: i + 1,
            column_id: 1,
            center_y: i as f32 * 4.0,
            fractions: vec![(Gas::CO2, 0.2), (Gas::H2, 0.2), (Gas::N2, 0.6)],
        })
        .collect::<Vec<_>>();
    // 120 ticks / 4 = 30 stratification steps.
    for _ in 0..30 {
        let _ = stratify(&mut cells, 9.81);
    }
    let bottom_co2 = cells.first().unwrap().fraction_of(Gas::CO2);
    let top_h2 = cells.last().unwrap().fraction_of(Gas::H2);
    assert!(
        bottom_co2 > 0.25,
        "bottom CO2 should rise ≥5% above 20%: got {bottom_co2}"
    );
    assert!(top_h2 > 0.25, "top H2 should rise ≥5% above 20%: got {top_h2}");
}

#[test]
fn stratification_replay_is_byte_identical_across_runs() {
    let make_cells = || {
        (0..16_u32)
            .map(|i| StratCell {
                cell_id: i + 1,
                column_id: 1,
                center_y: i as f32 * 4.0,
                fractions: vec![(Gas::CO2, 0.2), (Gas::H2, 0.2), (Gas::N2, 0.6)],
            })
            .collect::<Vec<_>>()
    };
    let mut a = make_cells();
    let mut b = make_cells();
    for _ in 0..30 {
        let _ = stratify(&mut a, 9.81);
        let _ = stratify(&mut b, 9.81);
    }
    assert_eq!(a, b);
}

// ----------------------------------------------------------------------------
// Scenario 7: Pipe rupture jet stream pushes nearby actor
// ----------------------------------------------------------------------------

#[test]
fn pipe_rupture_jet_stream_pushes_nearby_actor() {
    // 70 MPa pipe behind aperture → 100 kPa room. Actor 4 tiles away
    // on jet axis.
    let cells = vec![
        AtmosCell {
            id: 1,
            min: [0.0, 0.0],
            max: [4.0, 4.0],
            pressure_kpa: 70_000.0,
            temp_k: 293.15,
        },
        AtmosCell {
            id: 2,
            min: [4.0, 0.0],
            max: [16.0, 4.0],
            pressure_kpa: 100.0,
            temp_k: 293.15,
        },
    ];
    let sources = vec![WindSource {
        id: 2,
        origin: [4.0, 2.0],
        axis: [1.0, 0.0],
        aperture_area_m2: 0.01,
        cell_high_id: 1,
        cell_low_id: 2,
        jet_length: 12.0,
        jet_half_width: 1.0,
    }];
    let out = wind_force_at([8.0, 2.0], &cells, &sources);
    assert!(
        out.staggers_light_actor(),
        "70 MPa pipe rupture must stagger light actors: {:?}",
        out
    );
}

// ----------------------------------------------------------------------------
// Scenario 8: Determinism replay across 600 ticks
// ----------------------------------------------------------------------------

#[test]
fn determinism_replay_across_600_ticks() {
    let base = GravityVec::new(9.81, [0.0, -1.0]);
    let overrides = vec![
        GravityOverride::UniformWell {
            id: 1,
            center: [200.0, 100.0],
            radius: 50.0,
            magnitude: 25.0,
        },
        GravityOverride::RegionLowG {
            id: 2,
            min: [260.0, 0.0],
            max: [380.0, 200.0],
            local_g: 4.9,
        },
    ];
    let cells = vec![
        AtmosCell {
            id: 1,
            min: [0.0, 0.0],
            max: [10.0, 10.0],
            pressure_kpa: 110.0,
            temp_k: 293.15,
        },
        AtmosCell {
            id: 2,
            min: [10.0, 0.0],
            max: [20.0, 10.0],
            pressure_kpa: 100.0,
            temp_k: 293.15,
        },
    ];
    let sources = vec![WindSource {
        id: 1,
        origin: [10.0, 5.0],
        axis: [1.0, 0.0],
        aperture_area_m2: 0.5,
        cell_high_id: 1,
        cell_low_id: 2,
        jet_length: 10.0,
        jet_half_width: 1.0,
    }];
    let mut a_positions: Vec<[f32; 2]> = Vec::new();
    let mut b_positions: Vec<[f32; 2]> = Vec::new();
    // Replay-A.
    let mut pos = [205.0, 100.0];
    for _ in 0..600 {
        let g = apply_overrides(base, pos, Some(1), &overrides);
        let w = wind_force_at(pos, &cells, &sources);
        pos[0] += w.force_n[0] * 0.001 + g.gravity.direction[0] * g.gravity.magnitude * 0.0001;
        pos[1] += w.force_n[1] * 0.001 + g.gravity.direction[1] * g.gravity.magnitude * 0.0001;
        a_positions.push(pos);
    }
    // Replay-B (same seed / inputs).
    let mut pos = [205.0, 100.0];
    for _ in 0..600 {
        let g = apply_overrides(base, pos, Some(1), &overrides);
        let w = wind_force_at(pos, &cells, &sources);
        pos[0] += w.force_n[0] * 0.001 + g.gravity.direction[0] * g.gravity.magnitude * 0.0001;
        pos[1] += w.force_n[1] * 0.001 + g.gravity.direction[1] * g.gravity.magnitude * 0.0001;
        b_positions.push(pos);
    }
    assert_eq!(a_positions, b_positions);
}

// ----------------------------------------------------------------------------
// Scenario manifest acceptance tests
// ----------------------------------------------------------------------------

fn locate_scenario(id: &str) -> std::path::PathBuf {
    let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    workspace.join("content").join("scenarios").join(format!("{id}.ron"))
}

#[test]
fn m14b_gravity_anomaly_scenario_loads() {
    let path = locate_scenario("m14b_gravity_anomaly");
    let scenario = Scenario::load_from_file(&path).expect("load gravity anomaly scenario");
    assert_eq!(scenario.id, "m14b_gravity_anomaly");
    assert_eq!(scenario.gravity_overrides.len(), 5);
    assert!(matches!(
        scenario.gravity_overrides[0],
        ScenarioGravityOverride::UniformWell { .. }
    ));
    assert!(matches!(
        scenario.gravity_overrides[2],
        ScenarioGravityOverride::MagneticBoots { actor_id: 2, .. }
    ));
}

#[test]
fn m14b_wind_tunnel_scenario_loads() {
    let path = locate_scenario("m14b_wind_tunnel");
    let scenario = Scenario::load_from_file(&path).expect("load wind tunnel scenario");
    assert_eq!(scenario.id, "m14b_wind_tunnel");
    assert_eq!(scenario.atmosphere_cells.len(), 3);
    assert_eq!(scenario.wind_sources.len(), 2);
    assert!((scenario.atmosphere_cells[2].pressure_kpa - 70_000.0).abs() < 1e-3);
}

#[test]
fn m14b_gas_layered_room_scenario_loads() {
    let path = locate_scenario("m14b_gas_layered_room");
    let scenario = Scenario::load_from_file(&path).expect("load gas layered room scenario");
    assert_eq!(scenario.id, "m14b_gas_layered_room");
    assert_eq!(scenario.atmosphere_cells.len(), 4);
    // Each cell should carry its mixed CO2 + H2 + N2 composition.
    assert!(scenario.atmosphere_cells[0].gases.iter().any(|(g, _)| g == "co2"));
    assert!(scenario.atmosphere_cells[0].gases.iter().any(|(g, _)| g == "h2"));
}

// ----------------------------------------------------------------------------
// Engine integration: drive the full engine for N ticks + verify events fire.
// ----------------------------------------------------------------------------

use cf_control::engine::{run_m0_inline, M0Engine, M0EngineConfig};
use cf_control::runtime::{build_engine_config, ConfigInputs};
use cf_control::settings::Settings;
use cf_replay::resolve_run_bundle_root;
use std::path::Path;
use std::path::PathBuf;
use tempfile::tempdir;

fn build_run_config(scenario_path: &Path, scenario_id: &str, ticks: u64, bundle_root: PathBuf) -> M0EngineConfig {
    let inputs = ConfigInputs {
        scenario_id: scenario_id.to_string(),
        scenario_path: scenario_path.to_path_buf(),
        run_mode: format!("m14b-acceptance-{scenario_id}"),
        run_bundle_root: resolve_run_bundle_root(Some(bundle_root)),
        write_run_bundle: true,
        control_api_enabled: false,
        debug_capabilities: Vec::new(),
        tick_rate_hz: 60,
        capture_grid_enabled: false,
        paced: false,
        settings: Settings::default(),
        seed_override: None,
        duration_ticks_override: Some(ticks),
        debug_inject_panic_at_tick: None,
        checksum_cadence_ticks: None,
        expected_outcome: None,
    };
    build_engine_config(inputs).expect("build_engine_config")
}

fn read_events_jsonl(bundle_dir: &Path) -> Vec<cf_replay::Event> {
    let path = bundle_dir.join("events.jsonl");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut events = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let ev: cf_replay::Event = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("parse {} line {}: {e}", path.display(), n + 1));
        events.push(ev);
    }
    events
}

fn count_events(events: &[cf_replay::Event], category: &str, event_type: &str) -> usize {
    events
        .iter()
        .filter(|e| e.category == category && e.event_type == event_type)
        .count()
}

fn drive_and_collect_events(scenario_id: &str, ticks: u64) -> Vec<cf_replay::Event> {
    let path = locate_scenario(scenario_id);
    let bundle_root = tempdir().expect("tempdir");
    let config = build_run_config(&path, scenario_id, ticks, bundle_root.path().to_path_buf());
    let engine = M0Engine::new(config);
    engine.record_run_started();
    for _ in 0..ticks {
        if engine.drive_tick().is_none() {
            break;
        }
    }
    engine.record_run_finished(0);
    let bundle_dir = engine
        .write_run_bundle(chrono::Utc::now(), 0)
        .expect("write run bundle");
    read_events_jsonl(&bundle_dir)
}

#[test]
fn engine_emits_atmos_gas_stratified_for_layered_room() {
    let events = drive_and_collect_events("m14b_gas_layered_room", 200);
    let strat_events = count_events(&events, "atmos", "gas_stratified");
    assert!(
        strat_events >= 4,
        "expected ≥4 atmos.gas_stratified events over 200 ticks (50 stratification steps); got {strat_events}"
    );
}

#[test]
fn engine_emits_atmos_wind_force_applied_in_wind_tunnel() {
    let events = drive_and_collect_events("m14b_wind_tunnel", 60);
    let wind_events = count_events(&events, "atmos", "wind_force_applied");
    assert!(
        wind_events >= 1,
        "expected ≥1 atmos.wind_force_applied event in wind tunnel; got {wind_events}"
    );
}

#[test]
fn engine_emits_gravity_override_activated_in_gravity_anomaly() {
    let events = drive_and_collect_events("m14b_gravity_anomaly", 30);
    let activated = count_events(&events, "gravity", "override_activated");
    assert!(
        activated >= 1,
        "expected ≥1 gravity.override_activated event in gravity anomaly; got {activated}"
    );
}

#[test]
fn engine_run_is_deterministic_under_m14b_producers() {
    let events_a = drive_and_collect_events("m14b_wind_tunnel", 120);
    let events_b = drive_and_collect_events("m14b_wind_tunnel", 120);
    let wind_a = count_events(&events_a, "atmos", "wind_force_applied");
    let wind_b = count_events(&events_b, "atmos", "wind_force_applied");
    assert_eq!(wind_a, wind_b, "wind event count must match across runs");
    let grav_a = count_events(&events_a, "gravity", "override_activated");
    let grav_b = count_events(&events_b, "gravity", "override_activated");
    assert_eq!(grav_a, grav_b, "gravity event count must match across runs");
}

#[test]
fn engine_observe_frame_surfaces_cells_and_gravity_vectors() {
    let id = "m14b_gas_layered_room";
    let path = locate_scenario(id);
    let bundle_root = tempdir().expect("tempdir");
    let config = build_run_config(&path, id, 10, bundle_root.path().to_path_buf());
    let engine = std::sync::Arc::new(M0Engine::new(config));
    engine.record_run_started();
    for _ in 0..10 {
        if engine.drive_tick().is_none() {
            break;
        }
    }
    // Snapshot via the EngineHandle trait surface.
    use cf_control::server::EngineHandle;
    let frame = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(engine.snapshot(None));
    assert_eq!(frame.cells.len(), 4, "observe.frame.cells must surface 4 cells");
    assert!(
        !frame.gravity_vectors.is_empty(),
        "must surface ≥1 actor gravity vector"
    );
    engine.record_run_finished(0);
    let _ = engine.write_run_bundle(chrono::Utc::now(), 0);
}

// Suppress unused warning in case run_m0_inline isn't referenced.
#[allow(dead_code)]
fn _unused_imports() {
    let _ = run_m0_inline;
}
