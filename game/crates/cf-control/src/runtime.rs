//! Shared production helpers for assembling an `M0EngineConfig` from a scenario id +
//! CLI overrides. **Both `cf-app` and `cfctl` MUST go through `build_engine_config`** so
//! every run-bundle ships the same real metadata (commit_sha + rustc + bevy version, real
//! `expected_tests` from the scenario manifest, real `config_hash`, real settings).
//!
//! Anti-pattern (forbidden in production):
//!
//! ```ignore
//! let mut cfg = M0EngineConfig::for_test_scenario_only("m0_blank", path);
//! cfg.duration_ticks = ticks;
//! ```
//!
//! …because that bypasses the scenario manifest (no real seed/expected_tests/region) AND
//! skips the build metadata. Use [`build_engine_config`] instead.

use std::path::{Path, PathBuf};

use crate::engine::M0EngineConfig;
use crate::scenario::{Scenario, ScenarioLoadError};
use crate::settings::Settings;

/// Inputs the production helper accepts. Mirrors the union of `cf-app` and `cfctl` CLI
/// surface so neither binary has to roll its own.
#[derive(Debug, Clone)]
pub struct ConfigInputs {
    pub scenario_id: String,
    pub scenario_path: PathBuf,
    pub run_mode: String,
    pub run_bundle_root: PathBuf,
    pub write_run_bundle: bool,
    pub control_api_enabled: bool,
    pub debug_capabilities: Vec<String>,
    pub tick_rate_hz: u32,
    /// M4A: cf-app sets this to `true` when `--capture-grid` is on so the run
    /// manifest's `capture_config.{screenshots,captures}` reflects what's
    /// actually emitted to disk. Default false.
    #[allow(dead_code)] // pub field; consumed by build_engine_config below.
    pub capture_grid_enabled: bool,
    pub paced: bool,
    pub settings: Settings,
    /// CLI seed override. `None` = use the scenario manifest seed.
    pub seed_override: Option<u64>,
    /// CLI duration override (in ticks). `0` or `None` = use the scenario manifest's
    /// `duration_ticks` value (which itself may be 0 = "run until shutdown").
    pub duration_ticks_override: Option<u64>,
    /// **DEBUG-ONLY**: spawn a thread that panics at the named tick to capture
    /// `system.panic` evidence in a real run bundle. Production callers leave this `None`.
    pub debug_inject_panic_at_tick: Option<u64>,
}

/// Locate `<root>/content/scenarios/<id>.ron`. Searches a few well-known prefixes so
/// the helper works whether the binary is launched from `corefall/`, `corefall/game/`,
/// or `target/release/`.
pub fn locate_scenario(scenario_id: &str) -> Result<PathBuf, std::io::Error> {
    let candidates = [
        PathBuf::from("content/scenarios").join(format!("{scenario_id}.ron")),
        PathBuf::from("../content/scenarios").join(format!("{scenario_id}.ron")),
        PathBuf::from("game/content/scenarios").join(format!("{scenario_id}.ron")),
    ];
    for c in &candidates {
        if c.exists() {
            return Ok(c.clone());
        }
    }
    let cwd = std::env::current_dir().unwrap_or_default();
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!(
            "scenario file not found for {scenario_id}; searched {:?} from cwd {}",
            candidates,
            cwd.display()
        ),
    ))
}

/// Production config-builder. Loads the scenario manifest, builds an `M0EngineConfig` from
/// it (so seed / duration_ticks / expected_tests / region come from the file), applies CLI
/// overrides, and stamps real build metadata. This MUST be the path used by every
/// production binary.
pub fn build_engine_config(inputs: ConfigInputs) -> Result<M0EngineConfig, ConfigBuildError> {
    let scenario = Scenario::load_from_file(&inputs.scenario_path).map_err(ConfigBuildError::Scenario)?;
    let mut config = M0EngineConfig::for_loaded_scenario(&scenario, inputs.scenario_path.clone());
    config.scenario_id = inputs.scenario_id;
    config.tick_rate_hz = inputs.tick_rate_hz.max(1);
    config.run_mode = inputs.run_mode;
    config.run_bundle_root = inputs.run_bundle_root;
    config.write_run_bundle = inputs.write_run_bundle;
    config.control_api_enabled = inputs.control_api_enabled;
    config.debug_capabilities = inputs.debug_capabilities;
    config.paced = inputs.paced;
    config.settings = inputs.settings;
    config.capture_grid_enabled = inputs.capture_grid_enabled;
    if let Some(seed) = inputs.seed_override {
        config.seed = seed;
    }
    if let Some(ticks) = inputs.duration_ticks_override {
        if ticks > 0 {
            config.duration_ticks = ticks;
        }
    }
    config.debug_inject_panic_at_tick = inputs.debug_inject_panic_at_tick;
    config.commit_sha = git_commit_sha();
    let worktree = git_worktree_info();
    config.worktree_dirty = worktree.dirty;
    config.worktree_fingerprint = worktree.fingerprint;
    config.worktree_dirty_files = worktree.dirty_files;
    config.rust_version = rustc_version();
    config.bevy_version = format!("bevy {}", bevy_version());
    config.fill_config_hash();
    Ok(config)
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigBuildError {
    #[error("scenario load failed: {0}")]
    Scenario(#[from] ScenarioLoadError),
}

/// Real `commit_sha` for a run bundle. Reads `git rev-parse --short=12 HEAD`; appends
/// `-dirty` if the working tree has uncommitted changes. Honors `CF_COMMIT_SHA` env var
/// for CI use.
pub fn git_commit_sha() -> String {
    if let Ok(env_sha) = std::env::var("CF_COMMIT_SHA") {
        if !env_sha.is_empty() {
            return env_sha;
        }
    }
    let head = std::process::Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output();
    let head_sha = match head {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        Ok(o) => {
            tracing::warn!(
                target: "cf::ctl",
                stderr = %String::from_utf8_lossy(&o.stderr).trim(),
                "git rev-parse failed; recording commit_sha as 'uncommitted'. \
                 Set CF_COMMIT_SHA to override (e.g. in CI)."
            );
            return "uncommitted".to_string();
        }
        Err(e) => {
            tracing::warn!(
                target: "cf::ctl",
                error = %e,
                "git binary not available or unreachable; recording commit_sha as 'uncommitted'. \
                 Set CF_COMMIT_SHA to override (e.g. in CI)."
            );
            return "uncommitted".to_string();
        }
    };
    let status = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .output();
    let dirty = match &status {
        Ok(o) if o.status.success() => !o.stdout.is_empty(),
        Ok(o) => {
            tracing::warn!(
                target: "cf::ctl",
                stderr = %String::from_utf8_lossy(&o.stderr).trim(),
                "git status --porcelain failed; recording commit_sha without -dirty marker (assumed clean)."
            );
            false
        }
        Err(e) => {
            tracing::warn!(
                target: "cf::ctl",
                error = %e,
                "git status --porcelain not runnable; recording commit_sha without -dirty marker."
            );
            false
        }
    };
    if dirty {
        format!("{head_sha}-dirty")
    } else {
        head_sha
    }
}

#[derive(Debug, Clone, Default)]
pub struct GitWorktreeInfo {
    pub dirty: bool,
    pub fingerprint: Option<String>,
    pub dirty_files: Vec<String>,
}

/// Fingerprint the exact dirty worktree state used to build run evidence.
///
/// A `commit_sha` with a `-dirty` suffix is not enough for closure gates: every
/// audit-fix iteration before commit shares the same HEAD, even though the code
/// and scenario content differ. This hash covers tracked diffs plus untracked
/// file contents so BP close-loop fallback can distinguish "same commit, old
/// dirty evidence" from "same commit, same current dirty worktree".
pub fn git_worktree_info() -> GitWorktreeInfo {
    if let Ok(env_fp) = std::env::var("CF_WORKTREE_FINGERPRINT") {
        if !env_fp.is_empty() {
            return GitWorktreeInfo {
                dirty: true,
                fingerprint: Some(env_fp),
                dirty_files: Vec::new(),
            };
        }
    }

    let status = std::process::Command::new("git")
        .args(["status", "--porcelain=v1", "-z"])
        .output();
    let status = match status {
        Ok(o) if o.status.success() => o.stdout,
        Ok(o) => {
            tracing::warn!(
                target: "cf::ctl",
                stderr = %String::from_utf8_lossy(&o.stderr).trim(),
                "git status --porcelain failed; worktree fingerprint unavailable."
            );
            return GitWorktreeInfo::default();
        }
        Err(e) => {
            tracing::warn!(
                target: "cf::ctl",
                error = %e,
                "git status --porcelain not runnable; worktree fingerprint unavailable."
            );
            return GitWorktreeInfo::default();
        }
    };
    if status.is_empty() {
        return GitWorktreeInfo::default();
    }

    let diff = std::process::Command::new("git")
        .args(["diff", "--binary", "--no-ext-diff", "HEAD", "--"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| o.stdout)
        .unwrap_or_default();
    let untracked = std::process::Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard", "-z"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| o.stdout)
        .unwrap_or_default();

    let mut hasher = blake3::Hasher::new();
    hasher.update(b"cf-worktree-fingerprint-v1\0status\0");
    hasher.update(&status);
    hasher.update(b"\0diff\0");
    hasher.update(&diff);
    hasher.update(b"\0untracked\0");

    let mut dirty_files: Vec<String> = Vec::new();
    for entry in status.split(|b| *b == 0).filter(|s| !s.is_empty()) {
        let text = String::from_utf8_lossy(entry);
        if text.len() >= 4 {
            dirty_files.push(text[3..].to_string());
        }
    }
    dirty_files.sort();
    dirty_files.dedup();

    let mut untracked_files: Vec<PathBuf> = untracked
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| PathBuf::from(String::from_utf8_lossy(s).to_string()))
        .collect();
    untracked_files.sort();
    for path in &untracked_files {
        hasher.update(path.to_string_lossy().as_bytes());
        hasher.update(b"\0");
        if let Ok(bytes) = std::fs::read(path) {
            hasher.update(blake3::hash(&bytes).as_bytes());
        }
        hasher.update(b"\0");
    }

    GitWorktreeInfo {
        dirty: true,
        fingerprint: Some(hasher.finalize().to_hex().to_string()),
        dirty_files,
    }
}

/// Real `rust_version` for a run bundle. Calls `$RUSTC --version` (falling back to `rustc`).
pub fn rustc_version() -> String {
    let output = std::process::Command::new(option_env!("RUSTC").unwrap_or("rustc"))
        .arg("--version")
        .output();
    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => "rustc unknown".to_string(),
    }
}

/// Pinned Bevy major version. Mirrored in `workspace.dependencies.bevy`.
pub fn bevy_version() -> &'static str {
    "0.18.1"
}

/// Convenience used by `cf-app`/`cfctl` when they want to pre-flight scenario discovery.
pub fn scenario_path_or_err(scenario_id: &str) -> Result<PathBuf, std::io::Error> {
    locate_scenario(scenario_id)
}

// M0.4-F7: the canonical run-bundle path resolver lives in `cf_replay::bundle_paths`
// (which is the natural owner — it's the crate that writes bundles). `cf-control` re-exports
// here so existing callers don't have to chase an import path move.
pub use cf_replay::{default_run_bundle_root, resolve_run_bundle_root};

/// Confirm at compile time that production callers cannot accidentally swap in the
/// test-only constructor. (The real safety is the rename + `#[doc(hidden)]` marker on
/// `M0EngineConfig::for_test_scenario_only`.)
#[doc(hidden)]
pub fn _assert_production_path_is_used(_path: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_run_bundle_root_is_repo_root_when_tests_run_from_game() {
        let root = default_run_bundle_root();
        assert!(
            root.ends_with("prototype_runs/native"),
            "default run bundle root should end at prototype_runs/native, got {}",
            root.display()
        );
        assert!(
            !root.ends_with("game/prototype_runs/native"),
            "default run bundle root must not nest under game/: {}",
            root.display()
        );
    }
}
