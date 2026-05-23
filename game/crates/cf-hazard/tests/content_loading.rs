use std::path::PathBuf;

use cf_hazard::{HazardKind, HazardRegistry};

fn content_dir() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.join("../../content/hazards")
}

#[test]
fn loads_all_9_kinds_from_content_dir() {
    let reg = HazardRegistry::load_dir(&content_dir()).expect("load_dir ok");
    for k in HazardKind::all() {
        assert!(reg.specs.contains_key(k.as_str()), "missing kind {} in content", k.as_str());
    }
}

#[test]
fn fire_spec_loaded_with_expected_spread_rate() {
    let reg = HazardRegistry::load_dir(&content_dir()).expect("load_dir ok");
    let fire = reg.lookup(HazardKind::Fire);
    assert!((fire.spread_tiles_per_s - 1.0).abs() < 1e-3);
    assert!((fire.dissipation_seconds - 30.0).abs() < 1e-3);
    assert!(fire.counter_dissipation_multiplier <= 0.0,
        "fire must be DOUSED instantly by counter (water); got {}", fire.counter_dissipation_multiplier);
}

#[test]
fn radiation_spec_loaded_as_static() {
    let reg = HazardRegistry::load_dir(&content_dir()).expect("load_dir ok");
    let rad = reg.lookup(HazardKind::Radiation);
    assert!(rad.is_static);
    assert_eq!(rad.spread_tiles_per_s, 0.0);
}
