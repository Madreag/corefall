use std::path::PathBuf;

use cf_artifact::ArtifactRegistry;

fn content_dir() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.join("../../content/artifacts")
}

#[test]
fn loads_artifacts_from_content_dir() {
    let reg = ArtifactRegistry::load_dir(&content_dir()).expect("load_dir ok");
    assert!(reg.specs.len() >= 20, "expected ≥20 artifacts, got {}", reg.specs.len());
}

#[test]
fn stone_blood_loaded_with_20_hp() {
    let reg = ArtifactRegistry::load_dir(&content_dir()).expect("load_dir ok");
    let stone = reg.lookup("stone_blood").expect("stone_blood present");
    assert!((stone.bonus.max_hp_bonus - 20.0).abs() < 1e-3);
}

#[test]
fn compass_reveals_anomalies_from_content() {
    let reg = ArtifactRegistry::load_dir(&content_dir()).expect("load_dir ok");
    let c = reg.lookup("compass").expect("compass present");
    assert!(c.bonus.reveals_anomalies);
}
