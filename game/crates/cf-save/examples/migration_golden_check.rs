//! **M4B § "Migration corpus matrix passes for every fixture"** — golden
//! hash check: load `v1_minimal.cfsave`, run it forward through the
//! migration registry, and assert the canonical-JSON BLAKE3 of the
//! migrated blob matches `v2_minimal.cfsave.checksum` byte-for-byte.
//!
//! This is the binding invariant that future schema bumps must preserve:
//! the migrated output is reproducible across builds + platforms (DR-052
//! float-determinism rule keeps the canonical JSON byte-stable).
//!
//! Run via `game/scripts/m4b_migration_matrix.sh` (CI gate) or directly:
//!
//! ```bash
//! cargo run -p cf-save --example migration_golden_check
//! ```

use std::{fs, path::PathBuf};

use cf_save::{migration::migrate, WorldSave, V2_0_0};

fn main() -> anyhow::Result<()> {
    let dir = corpus_dir()?;
    check_pair(&dir, "v1_minimal.cfsave", "v2_minimal.cfsave.checksum")?;
    println!("M4B migration golden hash check PASS");
    Ok(())
}

fn corpus_dir() -> anyhow::Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    let game_root = if cwd.file_name().and_then(|n| n.to_str()) == Some("game") {
        cwd
    } else {
        cwd.join("game")
    };
    Ok(game_root.join("content/save_corpus"))
}

fn check_pair(dir: &std::path::Path, v1_name: &str, v2_golden_checksum: &str) -> anyhow::Result<()> {
    let v1_path = dir.join(v1_name);
    let golden_path = dir.join(v2_golden_checksum);
    let v1_text = fs::read_to_string(&v1_path)
        .map_err(|e| anyhow::anyhow!("read {}: {e}", v1_path.display()))?;
    let v1_blob: WorldSave = serde_json::from_str(&v1_text)
        .map_err(|e| anyhow::anyhow!("parse {} as JSON: {e}", v1_path.display()))?;
    let outcome = migrate(v1_blob, V2_0_0)
        .map_err(|e| anyhow::anyhow!("migrate {}: {e}", v1_path.display()))?;
    let computed_hex = outcome.blob.checksum_hex()
        .map_err(|e| anyhow::anyhow!("checksum_hex for migrated blob: {e}"))?;
    let expected_hex = fs::read_to_string(&golden_path)
        .map_err(|e| anyhow::anyhow!("read {}: {e}", golden_path.display()))?
        .trim()
        .to_string();
    if computed_hex != expected_hex {
        anyhow::bail!(
            "GOLDEN MISMATCH: {} migrated to v2 produced blake3={} but {} expects {}",
            v1_name,
            computed_hex,
            v2_golden_checksum,
            expected_hex
        );
    }
    println!(
        "  {} -> v2 migrates to canonical-JSON blake3 {} (matches {})",
        v1_name, computed_hex, v2_golden_checksum
    );
    Ok(())
}
