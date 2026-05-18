//! VAL-M14G-003: per-variant wound_spec RON files load from
//! `content/wound_specs/` and the registry has one entry per `WoundKind`.

use std::path::PathBuf;

use cf_wound::{registry::WoundSpecRegistry, WoundKind};

fn wound_specs_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // crates/cf-wound -> game/content/wound_specs
    manifest_dir
        .join("..")
        .join("..")
        .join("content")
        .join("wound_specs")
        .canonicalize()
        .expect("canonicalize wound_specs dir")
}

#[test]
fn all_wound_specs_load() {
    let dir = wound_specs_dir();
    assert!(dir.exists(), "wound_specs dir missing: {}", dir.display());
    let registry = WoundSpecRegistry::load(&dir).expect("load wound specs");
    assert_eq!(
        registry.len(),
        WoundKind::COUNT,
        "expected {} specs, got {}",
        WoundKind::COUNT,
        registry.len()
    );
    for kind in WoundKind::ALL.iter() {
        assert!(registry.get(*kind).is_some(), "missing spec for {kind:?}");
    }
}

#[test]
fn registered_specs_have_one_to_one_decal_ids() {
    let registry = WoundSpecRegistry::load(&wound_specs_dir()).expect("load wound specs");
    let mut seen = std::collections::HashSet::new();
    for (_, spec) in registry.iter() {
        assert!(seen.insert(spec.decal_id.clone()), "duplicate {:?}", spec.decal_id);
    }
    assert_eq!(seen.len(), WoundKind::COUNT);
}
