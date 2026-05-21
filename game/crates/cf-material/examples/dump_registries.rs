//! **M15B § Content-driven extraction tool** — dump the hardcoded
//! reaction + phase registries to JSON so they can be edited as
//! content rather than recompiled. Run via:
//!
//! ```sh
//! cargo run -p cf-material --example dump_registries > /tmp/dump.json
//! ```

use cf_material::{default_phase_registry, default_reaction_registry};

fn main() {
    let arg = std::env::args().nth(1).unwrap_or_else(|| "reactions".to_string());
    match arg.as_str() {
        "reactions" => {
            let r = default_reaction_registry();
            println!("{}", serde_json::to_string_pretty(&r).expect("ser"));
        }
        "phase" => {
            let p = default_phase_registry();
            println!("{}", serde_json::to_string_pretty(&p).expect("ser"));
        }
        other => {
            eprintln!("unknown registry: {other} (try 'reactions' or 'phase')");
            std::process::exit(1);
        }
    }
}
