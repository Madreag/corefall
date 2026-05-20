use std::path::PathBuf;

use cf_prosthetic::{ProstheticKind, ProstheticSpecRegistry};

fn workspace_content_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("content").join("prosthetics"))
        .expect("workspace content dir resolves")
}

#[test]
fn prosthetic_registry_loads_10_specs() {
    let dir = workspace_content_dir();
    if !dir.exists() {
        return;
    }
    let registry = ProstheticSpecRegistry::load(&dir).expect("registry loads");
    assert_eq!(registry.len(), ProstheticKind::COUNT);
    for k in ProstheticKind::ALL.iter() {
        assert!(
            registry.get(*k).is_some(),
            "missing prosthetic kind {:?}",
            k
        );
    }
}

#[test]
fn baked_default_matches_kind_count() {
    let registry = ProstheticSpecRegistry::baked_default();
    assert_eq!(registry.len(), ProstheticKind::COUNT);
}
