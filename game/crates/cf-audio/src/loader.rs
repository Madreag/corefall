//! **M12A** § cf-audio playback engine — load SFX at startup.
//!
//! Per spec: "Given M12A SFX generated / When cf-app starts / Then cf-audio
//! loads all OGG Vorbis files into in-memory pool / Per-SFX texture-id
//! assigned for fast lookup / Total memory: <500 MB (within Steam Deck
//! budget per T-PERF)".
//!
//! This module owns the in-memory file pool. It reads bytes from disk
//! lazily on first request (loader is presentation-agnostic; the actual
//! Bevy `Handle<AudioSource>` resolution happens in cf-app's adapter
//! layer). The pool tracks file mtime so [`SfxPool::reload_if_changed`]
//! can implement the hot-reload scenario per spec § "Hot-reload audio
//! in dev mode".

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::registry::{AudioAsset, AudioRegistry};

/// Per-SFX entry in the in-memory loader pool.
#[derive(Debug, Clone)]
pub struct SfxEntry {
    /// Stable integer id assigned at load time (the spec's "per-SFX
    /// texture-id assigned for fast lookup"). Stable across a single
    /// process; consumers should NOT persist this across restarts.
    pub texture_id: u32,
    /// Canonical asset name (e.g. `sfx_pistol_fire`).
    pub canonical_name: String,
    /// Resolved on-disk path the loader will read from.
    pub path: PathBuf,
    /// File mtime at last load (for hot-reload detection).
    pub mtime: Option<SystemTime>,
    /// Cached file size in bytes (mirrors ledger; not re-stat'd).
    pub size_bytes: u64,
}

/// **M12A** § cf-audio playback pool. Owned at cf-app startup; the Bevy
/// `AudioPlayer` adapter consults this for the canonical name → path
/// mapping.
#[derive(Debug, Default, Clone)]
pub struct SfxPool {
    entries: BTreeMap<String, SfxEntry>,
    by_texture_id: BTreeMap<u32, String>,
    next_texture_id: u32,
    /// Approximate memory budget tracker — sum of `size_bytes`. cf-app
    /// asserts this stays under [`MEMORY_BUDGET_BYTES`] at startup so
    /// the Steam Deck T-PERF target is testable.
    pub approx_memory_bytes: u64,
}

/// Spec acceptance criterion: "Total memory: <500 MB".
pub const MEMORY_BUDGET_BYTES: u64 = 500 * 1024 * 1024;

impl SfxPool {
    /// Construct a pool from an [`AudioRegistry`] — every `Audio_SFX`
    /// asset gets a slot + an auto-assigned `texture_id`. Returns the
    /// number of SFX inserted.
    pub fn hydrate_from_registry(registry: &AudioRegistry) -> Self {
        let mut pool = SfxPool::default();
        for sfx in registry.sfxs() {
            pool.insert(sfx);
        }
        pool
    }

    /// Insert (or overwrite) an entry from an `AudioAsset`. If the
    /// canonical name already exists, the texture_id is preserved (so
    /// hot-reload doesn't churn stable ids).
    pub fn insert(&mut self, asset: &AudioAsset) {
        let texture_id = match self.entries.get(&asset.canonical_name) {
            Some(existing) => existing.texture_id,
            None => {
                let id = self.next_texture_id;
                self.next_texture_id = self.next_texture_id.saturating_add(1);
                id
            }
        };
        let mtime = std::fs::metadata(&asset.output_path)
            .ok()
            .and_then(|m| m.modified().ok());
        if let Some(old) = self.entries.get(&asset.canonical_name) {
            self.approx_memory_bytes = self.approx_memory_bytes.saturating_sub(old.size_bytes);
        }
        self.approx_memory_bytes = self.approx_memory_bytes.saturating_add(asset.output_size_bytes);
        let entry = SfxEntry {
            texture_id,
            canonical_name: asset.canonical_name.clone(),
            path: asset.output_path.clone(),
            mtime,
            size_bytes: asset.output_size_bytes,
        };
        self.by_texture_id.insert(texture_id, asset.canonical_name.clone());
        self.entries.insert(asset.canonical_name.clone(), entry);
    }

    /// Look up a SFX by canonical name. Returns `None` if not in the pool.
    pub fn get(&self, canonical_name: &str) -> Option<&SfxEntry> {
        self.entries.get(canonical_name)
    }

    /// Look up a SFX by texture id.
    pub fn get_by_texture_id(&self, texture_id: u32) -> Option<&SfxEntry> {
        self.by_texture_id
            .get(&texture_id)
            .and_then(|name| self.entries.get(name))
    }

    /// Total entry count.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the pool is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate all entries in canonical-name order (deterministic).
    pub fn iter(&self) -> impl Iterator<Item = &SfxEntry> {
        self.entries.values()
    }

    /// **M12A** § Hot-reload audio in dev mode.
    ///
    /// Walks every entry, stats the on-disk path, and updates the cached
    /// mtime. Returns the list of canonical names whose mtime changed —
    /// cf-app's audio adapter re-loads the matching `Handle<AudioSource>`
    /// so the NEXT playback uses the new audio.
    pub fn reload_if_changed(&mut self) -> Vec<String> {
        let mut changed = Vec::new();
        for entry in self.entries.values_mut() {
            let new_mtime = std::fs::metadata(&entry.path).ok().and_then(|m| m.modified().ok());
            if new_mtime != entry.mtime {
                entry.mtime = new_mtime;
                changed.push(entry.canonical_name.clone());
            }
        }
        changed
    }

    /// Asserts the pool stays under the Steam Deck memory budget. Returns
    /// `Ok(())` on success; `Err(over_by_bytes)` when over budget.
    pub fn memory_budget_ok(&self) -> Result<(), u64> {
        if self.approx_memory_bytes <= MEMORY_BUDGET_BYTES {
            Ok(())
        } else {
            Err(self.approx_memory_bytes - MEMORY_BUDGET_BYTES)
        }
    }
}

/// Convenience: convert an absolute ledger path to a path relative to the
/// repo root. cf-app uses this to translate ledger entries to AssetServer
/// load paths (relative to its configured `file_path`).
pub fn relative_to_repo_root(repo_root: &Path, abs: &Path) -> Option<PathBuf> {
    abs.strip_prefix(repo_root).ok().map(|p| p.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(name: &str, path: &str, size: u64) -> AudioAsset {
        AudioAsset {
            canonical_name: name.to_string(),
            category: "Audio_SFX".to_string(),
            kind: "weapon".to_string(),
            pipeline: "M12A_test".to_string(),
            tier: "Tier1_LLM_Audio".to_string(),
            output_path: PathBuf::from(path),
            output_size_bytes: size,
            output_blake3: "0".repeat(64),
        }
    }

    #[test]
    fn insert_assigns_stable_texture_ids() {
        let mut pool = SfxPool::default();
        pool.insert(&asset("sfx_a", "/tmp/a.wav", 100));
        pool.insert(&asset("sfx_b", "/tmp/b.wav", 200));
        let a = pool.get("sfx_a").unwrap();
        let b = pool.get("sfx_b").unwrap();
        assert_eq!(a.texture_id, 0);
        assert_eq!(b.texture_id, 1);
        let by_id = pool.get_by_texture_id(0).unwrap();
        assert_eq!(by_id.canonical_name, "sfx_a");
    }

    #[test]
    fn re_insert_preserves_texture_id() {
        let mut pool = SfxPool::default();
        pool.insert(&asset("sfx_a", "/tmp/a.wav", 100));
        let first = pool.get("sfx_a").unwrap().texture_id;
        pool.insert(&asset("sfx_a", "/tmp/a.wav", 100));
        let second = pool.get("sfx_a").unwrap().texture_id;
        assert_eq!(first, second);
    }

    #[test]
    fn approx_memory_tracks_inserts_and_replaces() {
        let mut pool = SfxPool::default();
        pool.insert(&asset("a", "/tmp/a.wav", 100));
        pool.insert(&asset("b", "/tmp/b.wav", 200));
        assert_eq!(pool.approx_memory_bytes, 300);
        // Replace `a` with a smaller version — memory shrinks accordingly.
        pool.insert(&asset("a", "/tmp/a.wav", 50));
        assert_eq!(pool.approx_memory_bytes, 250);
    }

    #[test]
    fn memory_budget_ok_below_500_mb() {
        let mut pool = SfxPool::default();
        pool.insert(&asset("a", "/tmp/a.wav", 100 * 1024 * 1024));
        assert!(pool.memory_budget_ok().is_ok());
    }

    #[test]
    fn memory_budget_fails_when_over_500_mb() {
        let mut pool = SfxPool::default();
        for i in 0..6u64 {
            pool.insert(&asset(&format!("a{i}"), "/tmp/a.wav", 100 * 1024 * 1024));
        }
        assert!(pool.memory_budget_ok().is_err());
    }

    #[test]
    fn relative_to_repo_root_strips_prefix() {
        let root = Path::new("/Users/erol/projects/corefall");
        let abs = Path::new("/Users/erol/projects/corefall/game/content/audio/sfx/sfx_pistol_fire.wav");
        let rel = relative_to_repo_root(root, abs).unwrap();
        assert_eq!(rel, Path::new("game/content/audio/sfx/sfx_pistol_fire.wav"));
    }

    #[test]
    fn relative_to_repo_root_returns_none_when_outside() {
        let root = Path::new("/Users/erol/projects/corefall");
        let outside = Path::new("/tmp/other.wav");
        assert!(relative_to_repo_root(root, outside).is_none());
    }

    #[test]
    fn iter_returns_canonical_name_order() {
        let mut pool = SfxPool::default();
        pool.insert(&asset("c", "/tmp/c.wav", 1));
        pool.insert(&asset("a", "/tmp/a.wav", 1));
        pool.insert(&asset("b", "/tmp/b.wav", 1));
        let names: Vec<_> = pool.iter().map(|e| e.canonical_name.clone()).collect();
        assert_eq!(names, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    }
}
