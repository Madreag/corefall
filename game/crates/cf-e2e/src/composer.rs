use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;

pub(crate) fn default_composer_script() -> PathBuf {
    if let Ok(repo) = std::env::var("CARGO_MANIFEST_DIR") {
        let p = PathBuf::from(repo)
            .join("..")
            .join("..")
            .join("tools")
            .join("capture_grid.py");
        if p.exists() {
            return p;
        }
    }
    let exe = std::env::current_exe().unwrap_or_default();
    let mut walk = exe.as_path();
    while let Some(parent) = walk.parent() {
        let candidate = parent.join("game").join("tools").join("capture_grid.py");
        if candidate.exists() {
            return candidate;
        }
        walk = parent;
    }
    PathBuf::from("game/tools/capture_grid.py")
}

pub(crate) fn invoke_composer(python_bin: &str, script: &Path, run_dir: &Path) -> Result<Value> {
    let captures_dir = run_dir.join("captures");
    if !captures_dir.exists() {
        anyhow::bail!(
            "captures dir {} does not exist (cf-app may not have produced any frames)",
            captures_dir.display()
        );
    }
    let output = std::process::Command::new(python_bin)
        .arg(script)
        .arg(run_dir)
        .output()
        .with_context(|| format!("spawn {python_bin} {}", script.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("composer exited with {}: {}", output.status, stderr.trim());
    }
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    serde_json::from_str(&stdout).with_context(|| format!("composer stdout was not JSON: {stdout}"))
}
