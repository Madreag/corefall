//! Regenerate the static JSON Schemas under `crates/cf-control/schemas/v1/`.
//!
//! Run with:
//!
//! ```bash
//! cargo run -p cf-control --example dump_schemas        # rewrite the on-disk files
//! cargo run -p cf-control --example dump_schemas -- --check  # CI: fail if any file differs
//! ```

use std::{fs, path::PathBuf};

fn main() -> anyhow::Result<()> {
    let check_mode = std::env::args().any(|a| a == "--check");
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("schemas/v1");
    fs::create_dir_all(&dir)?;
    let schemas = cf_control::schemas::dump_v1();
    let mut drift = Vec::new();
    for (name, body) in &schemas {
        let path = dir.join(name);
        let mut expected = body.clone();
        if !expected.ends_with('\n') {
            expected.push('\n');
        }
        if check_mode {
            let actual = fs::read_to_string(&path).unwrap_or_default();
            if actual != expected {
                drift.push(name.clone());
                eprintln!("schema drift: {}", path.display());
            }
        } else {
            fs::write(&path, &expected)?;
            println!("wrote {}", path.display());
        }
    }
    if check_mode {
        if drift.is_empty() {
            println!("schema check OK ({} schemas)", schemas.len());
            Ok(())
        } else {
            anyhow::bail!(
                "{} schema(s) drifted from on-disk versions: {}\nRegenerate with `cargo run -p cf-control --example dump_schemas`",
                drift.len(),
                drift.join(", ")
            );
        }
    } else {
        println!("wrote {} schema(s)", schemas.len());
        Ok(())
    }
}
