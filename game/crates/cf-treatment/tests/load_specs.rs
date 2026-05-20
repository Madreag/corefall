//! **M14H** VAL-M14H-001: per-treatment RON files load from
//! `content/treatments/` with one entry per `TreatmentKind`.

use std::path::PathBuf;

use cf_treatment::{TreatmentKind, TreatmentSpecRegistry};

fn treatments_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .join("..")
        .join("..")
        .join("content")
        .join("treatments")
        .canonicalize()
        .expect("canonicalize treatments dir")
}

#[test]
fn all_22_treatment_specs_load_from_ron() {
    let dir = treatments_dir();
    assert!(dir.exists(), "treatments dir missing: {}", dir.display());
    let registry = TreatmentSpecRegistry::load(&dir).expect("load treatment specs");
    assert_eq!(
        registry.len(),
        TreatmentKind::COUNT,
        "expected {} specs, got {}",
        TreatmentKind::COUNT,
        registry.len()
    );
    for kind in TreatmentKind::ALL.iter() {
        assert!(registry.get(*kind).is_some(), "missing spec for {kind:?}");
    }
}

#[test]
fn registered_specs_have_unique_kinds() {
    let registry = TreatmentSpecRegistry::load(&treatments_dir()).expect("load treatment specs");
    let mut seen = std::collections::HashSet::new();
    for (kind, _) in registry.iter() {
        assert!(seen.insert(*kind), "duplicate kind {:?}", kind);
    }
    assert_eq!(seen.len(), TreatmentKind::COUNT);
}

#[test]
fn baked_default_matches_ron_files() {
    let baked = TreatmentSpecRegistry::baked_default();
    let loaded = TreatmentSpecRegistry::load(&treatments_dir()).expect("load");
    assert_eq!(baked.len(), loaded.len());
    for (kind, spec) in baked.iter() {
        let other = loaded.get(*kind).expect("loaded must contain kind");
        assert_eq!(spec, other, "RON file diverges from baked default for {kind:?}");
    }
}
