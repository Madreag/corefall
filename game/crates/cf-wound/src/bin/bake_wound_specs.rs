//! Bakes one .ron file per WoundKind from the registry's compiled defaults.
//! Run via `cargo run -p cf-wound --bin bake_wound_specs -- <output_dir>`.

use std::path::PathBuf;

use cf_wound::{registry::spec_to_ron, registry::WoundSpecRegistry, WoundKind};

fn main() {
    let arg = std::env::args()
        .nth(1)
        .expect("usage: bake_wound_specs <output_dir>");
    let out_dir = PathBuf::from(arg).canonicalize().unwrap_or_else(|_| {
        let p = PathBuf::from(std::env::args().nth(1).unwrap());
        std::fs::create_dir_all(&p).expect("mkdir");
        p.canonicalize().expect("canonicalize")
    });
    std::fs::create_dir_all(&out_dir).expect("create out dir");
    let registry = WoundSpecRegistry::baked_default();
    for kind in WoundKind::ALL.iter() {
        let spec = registry.get(*kind).expect("baked spec");
        let s = spec_to_ron(spec);
        let file = out_dir.join(format!("{}.ron", to_snake_case(kind.as_str())));
        std::fs::write(&file, s).expect("write");
        println!("wrote {}", file.display());
    }
}

fn to_snake_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0
                && !out.ends_with('_')
                && (out.chars().last().map(|p| !p.is_ascii_digit()).unwrap_or(true))
            {
                out.push('_');
            }
            for lc in c.to_lowercase() {
                out.push(lc);
            }
        } else {
            out.push(c);
        }
    }
    out
}
