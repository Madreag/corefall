//! Emits `content/diseases/*.ron` + `_susceptibility_matrix.ron` from the
//! canonical specs so the on-disk content is guaranteed to parse via
//! `DiseaseRegistry::load_dir`. Run: `cargo run -p cf-disease --example gen_content`.

use std::{fs, path::PathBuf};

use cf_disease::{registry::DiseaseSpec, susceptibility::SusceptibilityMatrix, DiseaseKind};

fn main() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/diseases")
        .canonicalize()
        .unwrap_or_else(|_| {
            let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content/diseases");
            fs::create_dir_all(&p).unwrap();
            p
        });
    fs::create_dir_all(&dir).unwrap();
    let cfg = ron::ser::PrettyConfig::new()
        .struct_names(false)
        .separate_tuple_members(true)
        .indentor("  ".to_string());

    for &kind in DiseaseKind::all() {
        let spec = DiseaseSpec::default_for(kind);
        let body = ron::ser::to_string_pretty(&spec, cfg.clone()).unwrap();
        let path = dir.join(format!("{}.ron", kind.as_str()));
        fs::write(&path, body + "\n").unwrap();
        println!("wrote {}", path.display());
    }

    let matrix = SusceptibilityMatrix::default_matrix();
    let body = ron::ser::to_string_pretty(&matrix, cfg).unwrap();
    let path = dir.join("_susceptibility_matrix.ron");
    fs::write(&path, body + "\n").unwrap();
    println!("wrote {}", path.display());
}
