//! M4A append-only JSONL ledger.
//!
//! Contracts (per spec):
//!
//! - One entry per line; concurrent-write-safe via **fs2 OS-level advisory
//!   locks (`flock(LOCK_EX)`)** wrapped around every read-modify-write
//!   surface (`append`, `supersede_entry`, `compact`, `rewrite_with_status`).
//!   POSIX `O_APPEND` atomicity is *also* honored on the bare append path
//!   for sub-PIPE_BUF lines but the advisory lock is the authoritative
//!   serializer for the read-modify-write callsites where append-atomicity
//!   alone is insufficient.
//! - Lines are never modified post-write by the **append** path. Two
//!   sanctioned mutation surfaces exist (`supersede_entry` and
//!   `regenerator::rewrite_with_status`); both obtain an exclusive
//!   advisory lock on the ledger file for the read-modify-write window.
//! - File path defaults to `content/asset_ledger/ledger.jsonl` relative to
//!   the workspace root; tests set their own path.

use std::{
    collections::{BTreeMap, HashMap},
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
};

use fs2::FileExt;
use thiserror::Error;

use crate::{
    category::{AssetCategory, ProductionTier, RegenStatus},
    entry::{deterministic_generated_at_iso, deterministic_ledger_enabled, AssetEntry, AssetId},
};

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("io: {0}: {1}")]
    Io(PathBuf, std::io::Error),
    #[error("malformed jsonl line {line_number} in {path}: {message}")]
    Malformed {
        path: PathBuf,
        line_number: u64,
        message: String,
    },
    #[error("ledger entry not found: {0}")]
    NotFound(AssetId),
    #[error("ledger directory missing and could not be created: {0}: {1}")]
    DirCreate(PathBuf, std::io::Error),
}

/// Append-only JSONL ledger handle. The handle is cheap to clone (it just
/// stores the path); each append re-opens the file in append-mode so the
/// underlying writer is concurrent-safe with other processes.
#[derive(Debug, Clone)]
pub struct LedgerHandle {
    path: PathBuf,
}

impl LedgerHandle {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Ensure the parent directory exists and the JSONL file is created.
    /// Idempotent.
    pub fn ensure_exists(&self) -> Result<(), StorageError> {
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| StorageError::DirCreate(parent.to_path_buf(), e))?;
            }
        }
        if !self.path.exists() {
            File::create(&self.path).map_err(|e| StorageError::Io(self.path.clone(), e))?;
        }
        Ok(())
    }

    /// Append a single entry as one JSONL line. Each call re-opens the
    /// file in append-mode AND takes an exclusive advisory lock for the
    /// duration of the write so concurrent writers from the same or
    /// different processes serialize cleanly. POSIX `O_APPEND` is
    /// additionally atomic for sub-PIPE_BUF writes so the inner write
    /// remains correct under contention.
    pub fn append(&self, entry: &AssetEntry) -> Result<(), StorageError> {
        self.ensure_exists()?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| StorageError::Io(self.path.clone(), e))?;
        file.lock_exclusive()
            .map_err(|e| StorageError::Io(self.path.clone(), e))?;
        let result = self.append_locked(&file, entry);
        let _ = FileExt::unlock(&file);
        result
    }

    fn append_locked(&self, mut file: &File, entry: &AssetEntry) -> Result<(), StorageError> {
        let mut line = serde_json::to_string(entry).map_err(|e| StorageError::Malformed {
            path: self.path.clone(),
            line_number: 0,
            message: e.to_string(),
        })?;
        line.push('\n');
        file.write_all(line.as_bytes())
            .map_err(|e| StorageError::Io(self.path.clone(), e))?;
        file.flush().map_err(|e| StorageError::Io(self.path.clone(), e))?;
        Ok(())
    }

    /// Walk every line in the ledger. Yields one parsed `AssetEntry` per
    /// line; malformed lines surface as a `Malformed` error and abort the
    /// walk (M4A treats the ledger as canonical truth — partial reads
    /// silently dropping bad lines would defeat the integrity contract).
    pub fn read_all(&self) -> Result<Vec<AssetEntry>, StorageError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let file = File::open(&self.path).map_err(|e| StorageError::Io(self.path.clone(), e))?;
        let reader = BufReader::new(file);
        let mut out = Vec::new();
        for (i, line) in reader.lines().enumerate() {
            let line = line.map_err(|e| StorageError::Io(self.path.clone(), e))?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: AssetEntry = serde_json::from_str(&line).map_err(|e| StorageError::Malformed {
                path: self.path.clone(),
                line_number: (i + 1) as u64,
                message: e.to_string(),
            })?;
            out.push(entry);
        }
        Ok(out)
    }

    /// Return only the LATEST (live) entry per asset id. When an asset has
    /// been re-generated, all but the final entry have `superseded_by` set;
    /// downstream consumers (cf-mod ledger list / verify / summary) usually
    /// want the live set rather than the full history.
    pub fn live_entries(&self) -> Result<Vec<AssetEntry>, StorageError> {
        let all = self.read_all()?;
        let mut latest_by_id: HashMap<AssetId, AssetEntry> = HashMap::new();
        for entry in all {
            if let Some(existing) = latest_by_id.get(&entry.id) {
                // entries with `superseded_by` set are NOT the live version
                // unless they also happen to be the only one
                if existing.superseded_by.is_some() && entry.superseded_by.is_none() {
                    latest_by_id.insert(entry.id.clone(), entry);
                } else if entry.superseded_by.is_none() {
                    latest_by_id.insert(entry.id.clone(), entry);
                }
            } else {
                latest_by_id.insert(entry.id.clone(), entry);
            }
        }
        let mut out: Vec<AssetEntry> = latest_by_id
            .into_values()
            .filter(|e| e.superseded_by.is_none())
            .collect();
        out.sort_by(|a, b| a.canonical_name.cmp(&b.canonical_name));
        Ok(out)
    }

    /// Look up the latest live entry matching `id`. Returns `None` if no
    /// matching entry exists.
    pub fn find(&self, id: &AssetId) -> Result<Option<AssetEntry>, StorageError> {
        let entries = self.read_all()?;
        Ok(entries.into_iter().rev().find(|e| e.id == *id))
    }

    /// Mark the entries with `superseded_id` as superseded by `new_id`.
    /// Re-writes the ledger file in place under an exclusive advisory lock.
    /// One of two sanctioned post-write mutation surfaces (the other is
    /// `regenerator::rewrite_with_status` for the auto-stale-mark flow).
    /// Per the spec, re-generation appends a NEW entry then back-fills the
    /// old entry's `superseded_by` field.
    ///
    /// When `superseded_id == new_id` (the same logical asset has been
    /// re-generated and now has TWO entries with the same id), the LAST
    /// entry (newest by file position) is preserved as live; every earlier
    /// entry with the same id is marked superseded.
    pub fn supersede_entry(&self, superseded_id: &AssetId, new_id: &AssetId) -> Result<usize, StorageError> {
        self.ensure_exists()?;
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .map_err(|e| StorageError::Io(self.path.clone(), e))?;
        lock_file
            .lock_exclusive()
            .map_err(|e| StorageError::Io(self.path.clone(), e))?;
        let result = self.supersede_entry_locked(superseded_id, new_id);
        let _ = FileExt::unlock(&lock_file);
        result
    }

    /// Batched supersede: apply many (superseded_id, new_id) pairs in a
    /// SINGLE read-modify-rewrite pass. Drops the O(N²) cost of looping
    /// `supersede_entry` per pair (where each call rewrites the entire
    /// file) to O(N). Bulk pipeline tools (mod-pack publishers + the
    /// re-bake sweep that re-adds 5000 entries on a fresh checkout)
    /// should prefer this over the per-pair primitive.
    ///
    /// When `superseded_id == new_id` in any pair (re-generation flow),
    /// the LAST entry with that id stays live and every earlier entry is
    /// marked superseded.
    pub fn supersede_many(&self, pairs: &[(AssetId, AssetId)]) -> Result<usize, StorageError> {
        if pairs.is_empty() {
            return Ok(0);
        }
        self.ensure_exists()?;
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .map_err(|e| StorageError::Io(self.path.clone(), e))?;
        lock_file
            .lock_exclusive()
            .map_err(|e| StorageError::Io(self.path.clone(), e))?;
        let result = self.supersede_many_locked(pairs);
        let _ = FileExt::unlock(&lock_file);
        result
    }

    fn supersede_many_locked(&self, pairs: &[(AssetId, AssetId)]) -> Result<usize, StorageError> {
        let all = self.read_all()?;
        // Per-pair, identify the live-preserved index when superseded == new.
        let mut preserve: HashMap<AssetId, usize> = HashMap::new();
        for (sid, nid) in pairs {
            if sid == nid {
                if let Some(last) = all
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| e.id == *sid)
                    .map(|(i, _)| i)
                    .next_back()
                {
                    preserve.insert(sid.clone(), last);
                }
            }
        }
        let map: HashMap<AssetId, AssetId> = pairs.iter().cloned().collect();
        let mut updated = 0usize;
        let new_lines: Vec<String> = all
            .into_iter()
            .enumerate()
            .map(|(i, mut e)| {
                if let Some(nid) = map.get(&e.id) {
                    let preserve_idx = preserve.get(&e.id).copied();
                    let needs_supersede = e.superseded_by.is_none() && Some(i) != preserve_idx;
                    if needs_supersede {
                        e.superseded_by = Some(nid.clone());
                        e.deprecated_at = Some(supersede_timestamp(&e.id));
                        updated += 1;
                    }
                }
                serde_json::to_string(&e).expect("serialize entry")
            })
            .collect();
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.path)
            .map_err(|e| StorageError::Io(self.path.clone(), e))?;
        for line in &new_lines {
            file.write_all(line.as_bytes())
                .map_err(|e| StorageError::Io(self.path.clone(), e))?;
            file.write_all(b"\n")
                .map_err(|e| StorageError::Io(self.path.clone(), e))?;
        }
        file.flush().map_err(|e| StorageError::Io(self.path.clone(), e))?;
        Ok(updated)
    }

    fn supersede_entry_locked(&self, superseded_id: &AssetId, new_id: &AssetId) -> Result<usize, StorageError> {
        let all = self.read_all()?;
        let same_id_positions: Vec<usize> = all
            .iter()
            .enumerate()
            .filter(|(_, e)| e.id == *superseded_id)
            .map(|(i, _)| i)
            .collect();
        // When superseded_id == new_id, the live (current) version is the
        // LAST one with that id; everything earlier is superseded.
        let preserve_idx = if superseded_id == new_id {
            same_id_positions.last().copied()
        } else {
            None
        };
        let mut updated = 0usize;
        let new_lines: Vec<String> = all
            .into_iter()
            .enumerate()
            .map(|(i, mut e)| {
                let needs_supersede = e.id == *superseded_id && e.superseded_by.is_none() && Some(i) != preserve_idx;
                if needs_supersede {
                    e.superseded_by = Some(new_id.clone());
                    e.deprecated_at = Some(supersede_timestamp(&e.id));
                    updated += 1;
                }
                serde_json::to_string(&e).expect("serialize entry")
            })
            .collect();
        // truncate + rewrite
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.path)
            .map_err(|e| StorageError::Io(self.path.clone(), e))?;
        for line in &new_lines {
            file.write_all(line.as_bytes())
                .map_err(|e| StorageError::Io(self.path.clone(), e))?;
            file.write_all(b"\n")
                .map_err(|e| StorageError::Io(self.path.clone(), e))?;
        }
        file.flush().map_err(|e| StorageError::Io(self.path.clone(), e))?;
        Ok(updated)
    }

    /// Compact the ledger: keep ONLY the latest live entry per asset id
    /// (drop superseded history). Optional `keep_after` cutoff retains all
    /// entries whose `generated_at_iso` is >= the cutoff. Holds an
    /// exclusive advisory lock for the full read-modify-write window.
    pub fn compact(&self, keep_latest_only: bool, keep_after: Option<&str>) -> Result<CompactStats, StorageError> {
        self.ensure_exists()?;
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .map_err(|e| StorageError::Io(self.path.clone(), e))?;
        lock_file
            .lock_exclusive()
            .map_err(|e| StorageError::Io(self.path.clone(), e))?;
        let result = self.compact_locked(keep_latest_only, keep_after);
        let _ = FileExt::unlock(&lock_file);
        result
    }

    fn compact_locked(&self, keep_latest_only: bool, keep_after: Option<&str>) -> Result<CompactStats, StorageError> {
        let all = self.read_all()?;
        let total_before = all.len();
        let mut retained: Vec<AssetEntry> = if keep_latest_only {
            self.live_entries()?
        } else {
            all.iter().filter(|e| e.superseded_by.is_none()).cloned().collect()
        };
        if let Some(after) = keep_after {
            retained.retain(|e| e.generated_at_iso.as_str() >= after);
        }
        let total_after = retained.len();
        let backup_path = self.path.with_extension("jsonl.bak");
        std::fs::copy(&self.path, &backup_path).map_err(|e| StorageError::Io(backup_path.clone(), e))?;
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.path)
            .map_err(|e| StorageError::Io(self.path.clone(), e))?;
        let mut writer = BufWriter::new(&mut file);
        for entry in &retained {
            let line = serde_json::to_string(entry).expect("serialize");
            writer
                .write_all(line.as_bytes())
                .map_err(|e| StorageError::Io(self.path.clone(), e))?;
            writer
                .write_all(b"\n")
                .map_err(|e| StorageError::Io(self.path.clone(), e))?;
        }
        writer.flush().map_err(|e| StorageError::Io(self.path.clone(), e))?;
        Ok(CompactStats {
            total_before,
            total_after,
            backup_path,
        })
    }
}

/// Result of `compact`.
#[derive(Debug, Clone)]
pub struct CompactStats {
    pub total_before: usize,
    pub total_after: usize,
    pub backup_path: PathBuf,
}

/// Per-entry deterministic `deprecated_at` value when the env flag is on;
/// wall-clock RFC 3339 otherwise. Keeps the ledger byte-reproducible in CI.
fn supersede_timestamp(id: &AssetId) -> String {
    if deterministic_ledger_enabled() {
        format!("deprecated:{}", deterministic_generated_at_iso(id.as_str()))
    } else {
        chrono::Utc::now().to_rfc3339()
    }
}

/// Per-category + per-tier filter for `list_entries`.
#[derive(Debug, Clone, Default)]
pub struct ListFilter {
    pub category: Option<AssetCategory>,
    pub tier: Option<ProductionTier>,
    pub pipeline: Option<String>,
    pub status: Option<RegenStatus>,
    pub package_source_label: Option<String>,
    pub include_superseded: bool,
}

impl ListFilter {
    pub fn matches(&self, entry: &AssetEntry) -> bool {
        if let Some(c) = self.category {
            if entry.category != c {
                return false;
            }
        }
        if let Some(t) = self.tier {
            if entry.tier != t {
                return false;
            }
        }
        if let Some(p) = &self.pipeline {
            if &entry.pipeline != p {
                return false;
            }
        }
        if let Some(s) = self.status {
            if entry.regen_status != s {
                return false;
            }
        }
        if let Some(pkg) = &self.package_source_label {
            if &entry.package_source.as_label() != pkg {
                return false;
            }
        }
        if !self.include_superseded && entry.superseded_by.is_some() {
            return false;
        }
        true
    }
}

/// Summary used by `cf-mod ledger summary` and the cf-control
/// `observe.assets.ledger_summary` projection. Pure aggregation; no I/O.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LedgerSummary {
    pub total_entries: u64,
    pub live_entries: u64,
    pub superseded_entries: u64,
    pub by_category: BTreeMap<String, u64>,
    pub by_tier: BTreeMap<String, u64>,
    pub by_status: BTreeMap<String, u64>,
    /// **M4A spec literal** "per-pipeline-tier counts" — per-pipeline-id
    /// breakdown (e.g. `M9A_svg_v1`, `M32A_comfyui_v1`).
    #[serde(default)]
    pub by_pipeline: BTreeMap<String, u64>,
    /// IDs of every live entry in a non-Fresh state, grouped by status.
    pub non_fresh: BTreeMap<String, Vec<String>>,
}

pub fn summarize(entries: &[AssetEntry]) -> LedgerSummary {
    let mut total_entries = 0u64;
    let mut live_entries = 0u64;
    let mut superseded_entries = 0u64;
    let mut by_category: BTreeMap<String, u64> = BTreeMap::new();
    let mut by_tier: BTreeMap<String, u64> = BTreeMap::new();
    let mut by_status: BTreeMap<String, u64> = BTreeMap::new();
    let mut by_pipeline: BTreeMap<String, u64> = BTreeMap::new();
    let mut non_fresh: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for entry in entries {
        total_entries += 1;
        if entry.superseded_by.is_some() {
            superseded_entries += 1;
            continue;
        }
        live_entries += 1;
        *by_category.entry(entry.category.as_str().to_string()).or_default() += 1;
        *by_tier.entry(entry.tier.as_str().to_string()).or_default() += 1;
        *by_pipeline.entry(entry.pipeline.clone()).or_default() += 1;
        let status = entry.regen_status.as_str().to_string();
        *by_status.entry(status.clone()).or_default() += 1;
        if !matches!(entry.regen_status, RegenStatus::Fresh) {
            non_fresh.entry(status).or_default().push(entry.id.as_str().to_string());
        }
    }
    LedgerSummary {
        total_entries,
        live_entries,
        superseded_entries,
        by_category,
        by_tier,
        by_status,
        by_pipeline,
        non_fresh,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        category::{AssetCategory, ProductionTier},
        entry::AssetEntryBuilder,
    };

    static TMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn tmp_path() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let seq = TMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("cf-asset-ledger-storage-{pid}-{nanos}-{seq}.jsonl"))
    }

    fn build_entry(name: &str, tier: ProductionTier, category: AssetCategory) -> AssetEntry {
        AssetEntryBuilder::new(
            category,
            "kind",
            name,
            tier,
            "M9A_svg_v1",
            "prompt",
            1,
            format!("/tmp/{name}.svg"),
        )
        .with_output_blake3("a".repeat(64))
        .with_output_size(0)
        .with_generated_at_iso("2026-05-13T00:00:00Z")
        .with_generated_on_machine("ci")
        .build()
    }

    #[test]
    fn append_creates_jsonl_one_entry_per_line() {
        let path = tmp_path();
        let h = LedgerHandle::new(&path);
        for i in 0..100 {
            let e = build_entry(&format!("e{i}"), ProductionTier::Tier1Svg, AssetCategory::WeaponSprite);
            h.append(&e).unwrap();
        }
        let raw = std::fs::read_to_string(&path).unwrap();
        let n = raw.lines().count();
        assert_eq!(n, 100);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn read_all_round_trips() {
        let path = tmp_path();
        let h = LedgerHandle::new(&path);
        let e1 = build_entry("alpha", ProductionTier::Tier1Svg, AssetCategory::WeaponSprite);
        let e2 = build_entry("beta", ProductionTier::Tier1Svg, AssetCategory::WeaponSprite);
        h.append(&e1).unwrap();
        h.append(&e2).unwrap();
        let read = h.read_all().unwrap();
        assert_eq!(read.len(), 2);
        assert_eq!(read[0].canonical_name, "alpha");
        assert_eq!(read[1].canonical_name, "beta");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn list_filter_by_category_and_tier() {
        let path = tmp_path();
        let h = LedgerHandle::new(&path);
        h.append(&build_entry("a", ProductionTier::Tier1Svg, AssetCategory::WeaponSprite))
            .unwrap();
        h.append(&build_entry(
            "b",
            ProductionTier::Tier2ComfyUi,
            AssetCategory::WeaponSprite,
        ))
        .unwrap();
        h.append(&build_entry("c", ProductionTier::Tier1Svg, AssetCategory::UiIcon))
            .unwrap();
        let entries = h.read_all().unwrap();
        let filter = ListFilter {
            category: Some(AssetCategory::WeaponSprite),
            tier: Some(ProductionTier::Tier1Svg),
            ..ListFilter::default()
        };
        let filtered: Vec<_> = entries.iter().filter(|e| filter.matches(e)).collect();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].canonical_name, "a");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn supersede_back_fills_old_entry() {
        let path = tmp_path();
        let h = LedgerHandle::new(&path);
        let old = build_entry("foo", ProductionTier::Tier1Svg, AssetCategory::WeaponSprite);
        let new = build_entry("foo", ProductionTier::Tier1Svg, AssetCategory::WeaponSprite);
        // same canonical_name + same tier + same category ⇒ same id
        assert_eq!(old.id, new.id);
        h.append(&old).unwrap();
        h.append(&new).unwrap();
        // When superseded_id == new_id, only the EARLIER entries with that
        // id are marked superseded; the newest line stays live.
        let updated = h.supersede_entry(&old.id, &new.id).unwrap();
        assert_eq!(updated, 1, "only the older entry should be marked superseded");
        let live = h.live_entries().unwrap();
        assert_eq!(live.len(), 1, "the newest entry remains live");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn supersede_only_back_fills_unsuperseded_entries() {
        let path = tmp_path();
        let h = LedgerHandle::new(&path);
        // Two distinct entries with distinct canonical_names
        let v1 = AssetEntryBuilder::new(
            AssetCategory::WeaponSprite,
            "kind",
            "rifle_v1",
            ProductionTier::Tier1Svg,
            "M9A_svg_v1",
            "prompt",
            1,
            "/tmp/rifle_v1.svg",
        )
        .with_output_blake3("a".repeat(64))
        .with_generated_at_iso("2026-05-13T00:00:00Z")
        .build();
        let v2 = AssetEntryBuilder::new(
            AssetCategory::WeaponSprite,
            "kind",
            "rifle_v2",
            ProductionTier::Tier1Svg,
            "M9A_svg_v1",
            "prompt v2",
            2,
            "/tmp/rifle_v2.svg",
        )
        .with_output_blake3("b".repeat(64))
        .with_generated_at_iso("2026-05-13T01:00:00Z")
        .build();
        h.append(&v1).unwrap();
        h.append(&v2).unwrap();
        let v1_id = v1.id.clone();
        let v2_id = v2.id.clone();
        // supersede v1 by v2
        let count = h.supersede_entry(&v1_id, &v2_id).unwrap();
        assert_eq!(count, 1);
        let live = h.live_entries().unwrap();
        // v2 remains live, v1 is dead
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].id, v2_id);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn summarize_groups_by_category_tier_status() {
        let entries = vec![
            build_entry("a", ProductionTier::Tier1Svg, AssetCategory::WeaponSprite),
            build_entry("b", ProductionTier::Tier1Svg, AssetCategory::WeaponSprite),
            build_entry("c", ProductionTier::Tier2ComfyUi, AssetCategory::UiIcon),
        ];
        let summary = summarize(&entries);
        assert_eq!(summary.total_entries, 3);
        assert_eq!(summary.live_entries, 3);
        assert_eq!(summary.by_category.get("WeaponSprite"), Some(&2));
        assert_eq!(summary.by_category.get("UiIcon"), Some(&1));
        assert_eq!(summary.by_tier.get("Tier1_SVG"), Some(&2));
        assert_eq!(summary.by_tier.get("Tier2_ComfyUI"), Some(&1));
    }

    #[test]
    fn malformed_line_surfaces_error() {
        let path = tmp_path();
        std::fs::write(&path, b"this is not json\n").unwrap();
        let h = LedgerHandle::new(&path);
        let err = h.read_all().expect_err("malformed should fail");
        match err {
            StorageError::Malformed { line_number, .. } => assert_eq!(line_number, 1),
            other => panic!("unexpected error: {other:?}"),
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn supersede_many_supersedes_in_a_single_pass() {
        let path = tmp_path();
        let h = LedgerHandle::new(&path);
        // Three distinct logical assets, each re-added once
        let names = ["alpha", "beta", "gamma"];
        let mut pairs: Vec<(AssetId, AssetId)> = Vec::new();
        for name in &names {
            let v1 = build_entry(name, ProductionTier::Tier1Svg, AssetCategory::WeaponSprite);
            h.append(&v1).unwrap();
            let v2 = build_entry(name, ProductionTier::Tier1Svg, AssetCategory::WeaponSprite);
            h.append(&v2).unwrap();
            pairs.push((v1.id.clone(), v2.id.clone()));
        }
        // Batch supersede: each asset has 2 lines; the earlier one should
        // be marked superseded after a SINGLE rewrite.
        let updated = h.supersede_many(&pairs).unwrap();
        assert_eq!(updated, 3, "exactly one earlier line per asset should be marked");
        let live = h.live_entries().unwrap();
        assert_eq!(live.len(), 3, "three logical assets remain live");
        let all = h.read_all().unwrap();
        let superseded = all.iter().filter(|e| e.superseded_by.is_some()).count();
        assert_eq!(superseded, 3);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn supersede_many_handles_empty_pairs_as_noop() {
        let path = tmp_path();
        let h = LedgerHandle::new(&path);
        let updated = h.supersede_many(&[]).unwrap();
        assert_eq!(updated, 0);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn compact_drops_superseded_history() {
        let path = tmp_path();
        let h = LedgerHandle::new(&path);
        let v1 = AssetEntryBuilder::new(
            AssetCategory::WeaponSprite,
            "kind",
            "rifle_v1",
            ProductionTier::Tier1Svg,
            "M9A_svg_v1",
            "p",
            1,
            "/tmp/rifle.svg",
        )
        .with_output_blake3("a".repeat(64))
        .with_generated_at_iso("2026-05-13T00:00:00Z")
        .build();
        let v2 = AssetEntryBuilder::new(
            AssetCategory::WeaponSprite,
            "kind",
            "rifle_v2",
            ProductionTier::Tier1Svg,
            "M9A_svg_v1",
            "p",
            2,
            "/tmp/rifle.svg",
        )
        .with_output_blake3("b".repeat(64))
        .with_generated_at_iso("2026-05-13T01:00:00Z")
        .build();
        h.append(&v1).unwrap();
        h.append(&v2).unwrap();
        let _ = h.supersede_entry(&v1.id, &v2.id).unwrap();
        // pre-compact: 2 lines
        assert_eq!(h.read_all().unwrap().len(), 2);
        let stats = h.compact(true, None).unwrap();
        assert_eq!(stats.total_before, 2);
        assert_eq!(stats.total_after, 1);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(stats.backup_path);
    }
}
