//! M7B: regenerate `game/content/ai/archetype_bts/*.ron` from the Rust
//! builtin defs. Run via `cargo run -p cf-ai --example dump_archetype_bts`.

use cf_ai::archetype_bt::{ArchetypeBtDef, ArchetypeBtKind};

fn main() {
    let out_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content")
        .join("ai")
        .join("archetype_bts");
    std::fs::create_dir_all(&out_dir).expect("create content dir");

    let pretty = ron::ser::PrettyConfig::new()
        .depth_limit(8)
        .indentor("  ".to_string())
        .struct_names(true);

    for kind in ArchetypeBtKind::ALL {
        let def = ArchetypeBtDef::from_builtin(kind);
        let body = ron::ser::to_string_pretty(&def, pretty.clone()).expect("serialize");
        let path = out_dir.join(format!("{}.ron", kind.as_str()));
        std::fs::write(&path, format!("{body}\n")).expect("write ron");
        println!("wrote {}", path.display());
    }
}
