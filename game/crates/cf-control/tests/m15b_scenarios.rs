//! **M15B** § Scenario manifest parse + smoke-load tests for the two
//! demos that ship in `content/scenarios/m15b_*.ron`.
//!
//! Per spec § "Files":
//! - `game/content/scenarios/m15b_water_cycle_demo.ron` (NEW)
//! - `game/content/scenarios/m15b_acid_rain_vulcan.ron` (NEW)
//!
//! Validates the manifests load cleanly through `Scenario::load_from_file`
//! and pass the canonical scenario validator. The runtime hook-up of the
//! precipitation cycle lives in cf-material-gpu's acceptance tests; this
//! test just confirms the manifests aren't authored against a stale
//! schema.

use cf_control::Scenario;

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
fn water_cycle_demo_manifest_parses() {
    let path = locate_scenario("m15b_water_cycle_demo");
    let s = Scenario::load_from_file(&path).expect("water cycle scenario parses");
    assert_eq!(s.id, "m15b_water_cycle_demo");
    assert_eq!(s.schema_version, 1);
    assert!(s.duration_ticks.is_some());
    assert!(s.terrain.is_some(), "water cycle demo must declare terrain");
    assert!(
        !s.atmosphere_cells.is_empty(),
        "water cycle demo must declare atmosphere cells for nucleation"
    );
}

#[test]
fn acid_rain_vulcan_manifest_parses() {
    let path = locate_scenario("m15b_acid_rain_vulcan");
    let s = Scenario::load_from_file(&path).expect("acid rain scenario parses");
    assert_eq!(s.id, "m15b_acid_rain_vulcan");
    assert_eq!(s.schema_version, 1);
    assert!(s.duration_ticks.is_some());
    assert!(s.terrain.is_some(), "acid rain demo must declare metal_nohook target");
    assert!(
        !s.atmosphere_cells.is_empty(),
        "acid rain demo must declare atmosphere cells for nucleation"
    );
}
