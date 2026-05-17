//! **M6C** § Files: generator binary that regenerates
//! `content/equipment/items/manifest.ron` with the full canonical id
//! list (M6B baseline + 81 new M6C SKUs). The manifest mirrors the
//! `cf_equipment::item_registered_ids()` registry so cf-mod's mirror
//! drift check stays green.

use std::path::PathBuf;

use cf_equipment::{item_registered_ids, spec_for_id};

fn main() -> anyhow::Result<()> {
    let argv: Vec<String> = std::env::args().collect();
    let manifest_path = if argv.len() >= 2 {
        PathBuf::from(&argv[1])
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("content/equipment/items/manifest.ron")
    };
    println!("writing {}", manifest_path.display());

    let mut ids = item_registered_ids();
    ids.sort();

    let mut lines: Vec<String> = vec![
        "// **M6B § Files**: lists every canonical ItemSpec id. Mirrors the".to_string(),
        "// hardcoded runtime registry in `cf_equipment::item_spec::registered_ids()`".to_string(),
        "// so cf-mod can validate that the on-disk manifest is in sync with the".to_string(),
        "// engine's view of the registry.".to_string(),
        "//".to_string(),
        "// Schema (locked at M6B):".to_string(),
        "//   schema_version: u32 = 1".to_string(),
        "//   items: Vec<ItemEntry>".to_string(),
        "//   ItemEntry { id: String, category: String }".to_string(),
        "//".to_string(),
        "// Categories MUST match `cf_equipment::ItemCategory::as_str()`.".to_string(),
        "//".to_string(),
        "// **M6C** added 81 new SKU ids (firearms + melee + grenades + heavy +".to_string(),
        "// medical + survival + sensors + ppe). Regenerate via".to_string(),
        "// `cargo run --bin generate_m6c_manifest` after registry edits.".to_string(),
        "(".to_string(),
        "    schema_version: 1,".to_string(),
        "    items: [".to_string(),
    ];
    for id in ids {
        let spec = spec_for_id(&id).unwrap_or_else(|| panic!("registry missing `{id}`"));
        lines.push(format!(
            "        (id: \"{}\", category: \"{}\"),",
            id,
            spec.category.as_str()
        ));
    }
    lines.push("    ],".to_string());
    lines.push(")".to_string());
    let body = lines.join("\n") + "\n";

    if let Some(parent) = manifest_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&manifest_path, body)?;
    println!("manifest.ron written");
    Ok(())
}
