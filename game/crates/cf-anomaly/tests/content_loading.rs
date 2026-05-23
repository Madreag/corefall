use std::path::PathBuf;

use cf_anomaly::{AnomalyKind, AnomalyRegistry};

fn content_dir() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.join("../../content/anomalies")
}

#[test]
fn loads_all_6_anomaly_kinds_from_content_dir() {
    let reg = AnomalyRegistry::load_dir(&content_dir()).expect("load_dir ok");
    for k in AnomalyKind::all() {
        assert!(reg.specs.contains_key(k.as_str()), "missing {}", k.as_str());
    }
}

#[test]
fn bloodsucker_lair_requires_detector() {
    let reg = AnomalyRegistry::load_dir(&content_dir()).expect("load_dir ok");
    let lair = reg.lookup(AnomalyKind::BloodsuckerLair);
    assert!(lair.detector_required);
    assert!((lair.detection_radius_m - 8.0).abs() < 1e-3);
}

#[test]
fn gravity_anomaly_slows_movement() {
    let reg = AnomalyRegistry::load_dir(&content_dir()).expect("load_dir ok");
    let g = reg.lookup(AnomalyKind::GravityAnomaly);
    assert!(g.affects_movement);
    assert!(g.movement_multiplier < 1.0);
}
