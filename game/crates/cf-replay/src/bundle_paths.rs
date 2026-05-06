//! Canonical run-bundle path resolution.
//!
//! Per the corefall AGENTS.md run-bundle contract, all run bundles MUST live under
//! `<corefall-repo-root>/prototype_runs/native/`. Standard validation, however, runs
//! commands from the `game/` workspace, so a naive relative `prototype_runs/native`
//! path would land at `game/prototype_runs/native` — which the corefall path-safety
//! contract forbids.
//!
//! Every binary that writes a bundle MUST resolve its target root via
//! [`default_run_bundle_root`] (or [`resolve_run_bundle_root`] when the user supplied
//! an explicit path). The resolver walks `cwd` upward looking for the corefall repo
//! root marker (`game/Cargo.toml`) and returns that root's `prototype_runs/native`.
//!
//! `cf-control::runtime` re-exports both helpers for backwards compatibility.

use std::path::PathBuf;

/// Default M0 run bundles live at the Corefall repo root (`prototype_runs/native`),
/// while standard validation runs commands from `game/`. Resolve that default once
/// here so all binaries agree on the same evidence location and never accidentally
/// write to `game/prototype_runs`.
///
/// Resolution order:
///   1. cwd is the corefall repo root → `<cwd>/prototype_runs/native`
///   2. cwd is `game/` (workspace root) → `<cwd>/../prototype_runs/native`
///   3. some ancestor contains `game/Cargo.toml` → `<ancestor>/prototype_runs/native`
///   4. fallback (orphaned cwd) → relative `prototype_runs/native`
pub fn default_run_bundle_root() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Launched from repo root (`corefall/`).
    if cwd.join("game/Cargo.toml").exists() {
        return cwd.join("prototype_runs/native");
    }

    // Launched from workspace root (`corefall/game/`).
    if cwd.file_name().and_then(|name| name.to_str()) == Some("game") && cwd.join("Cargo.toml").exists() {
        if let Some(parent) = cwd.parent() {
            return parent.join("prototype_runs/native");
        }
    }

    // Launched from a nested directory inside the repo.
    for ancestor in cwd.ancestors() {
        if ancestor.join("game/Cargo.toml").exists() {
            return ancestor.join("prototype_runs/native");
        }
    }

    PathBuf::from("prototype_runs/native")
}

/// Caller-facing resolver. If `explicit` is `Some`, return it unchanged; otherwise
/// fall back to [`default_run_bundle_root`].
pub fn resolve_run_bundle_root(explicit: Option<PathBuf>) -> PathBuf {
    explicit.unwrap_or_else(default_run_bundle_root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;

    /// Serialize tests that mutate the process cwd so they don't race.
    static CWD_LOCK: Mutex<()> = Mutex::new(());

    fn make_corefall_layout() -> PathBuf {
        let mut tmp = std::env::temp_dir();
        tmp.push(format!(
            "cf_replay_path_test_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(tmp.join("game/crates")).unwrap();
        fs::write(tmp.join("game/Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
        tmp
    }

    /// Strip the macOS `/private` symlink prefix so `/private/var/...` and `/var/...`
    /// compare equal. Other platforms pass through unchanged.
    fn norm(p: &std::path::Path) -> PathBuf {
        let s = p.to_string_lossy();
        if let Some(rest) = s.strip_prefix("/private/") {
            PathBuf::from(format!("/{rest}"))
        } else {
            p.to_path_buf()
        }
    }

    /// M0.4-F7: when the binary's cwd is `<repo>/game/`, the bundle writer must resolve
    /// to `<repo>/prototype_runs/native`, NOT `<repo>/game/prototype_runs/native`.
    ///
    /// This is the regression test for the M0.3-F9 root cause: a relative
    /// `prototype_runs` path combined with a `game/` cwd would write bundles to
    /// `game/prototype_runs/`, which violates the repo-root contract.
    #[test]
    fn default_root_resolves_above_game_when_cwd_is_game() {
        let _g = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let repo = make_corefall_layout();
        let prev_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(repo.join("game")).unwrap();

        let resolved = default_run_bundle_root();

        let _ = std::env::set_current_dir(prev_cwd);

        let expected = repo.join("prototype_runs/native");
        assert_eq!(
            norm(&resolved),
            norm(&expected),
            "cwd=game/ must resolve to <repo>/prototype_runs/native; got {}",
            resolved.display()
        );
        assert!(
            !norm(&resolved).starts_with(norm(&repo.join("game"))),
            "resolver MUST NOT return a path under <repo>/game/: {}",
            resolved.display()
        );

        let _ = fs::remove_dir_all(repo);
    }

    #[test]
    fn default_root_uses_cwd_when_cwd_is_repo_root() {
        let _g = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let repo = make_corefall_layout();
        let prev_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(&repo).unwrap();

        let resolved = default_run_bundle_root();

        let _ = std::env::set_current_dir(prev_cwd);

        assert_eq!(
            norm(&resolved),
            norm(&repo.join("prototype_runs/native")),
            "cwd=<repo>/ must resolve to <repo>/prototype_runs/native; got {}",
            resolved.display()
        );

        let _ = fs::remove_dir_all(repo);
    }

    #[test]
    fn default_root_walks_up_from_nested_cwd() {
        let _g = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let repo = make_corefall_layout();
        let nested = repo.join("game/crates");
        let prev_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(&nested).unwrap();

        let resolved = default_run_bundle_root();

        let _ = std::env::set_current_dir(prev_cwd);

        assert_eq!(
            norm(&resolved),
            norm(&repo.join("prototype_runs/native")),
            "cwd=<repo>/game/crates must walk up to <repo>; got {}",
            resolved.display()
        );

        let _ = fs::remove_dir_all(repo);
    }

    #[test]
    fn resolve_run_bundle_root_returns_explicit_unchanged() {
        let p = PathBuf::from("/tmp/some/explicit/path");
        assert_eq!(resolve_run_bundle_root(Some(p.clone())), p);
    }

    #[test]
    fn resolve_run_bundle_root_falls_back_to_default_when_none() {
        let _g = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let repo = make_corefall_layout();
        let prev_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(repo.join("game")).unwrap();

        let resolved = resolve_run_bundle_root(None);

        let _ = std::env::set_current_dir(prev_cwd);

        assert_eq!(norm(&resolved), norm(&repo.join("prototype_runs/native")));
        let _ = fs::remove_dir_all(repo);
    }
}
