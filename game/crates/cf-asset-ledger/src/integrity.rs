//! M4A integrity check + blake3 drift detection.
//!
//! `verify_entry` re-hashes the file on disk and compares it against the
//! ledger entry's `output_blake3`. The same routine populates `RegenStatus`:
//!
//! - File missing → `Missing`
//! - File present but hash differs → `Drifted`
//! - File present and hash matches → `Fresh`
//! - I/O error on read → `Failed`

use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use thiserror::Error;

use crate::{
    category::RegenStatus,
    entry::{AdditionalOutput, AssetEntry, AssetId},
};

/// Block size for streaming blake3. Matches the std::io BufReader default
/// so we don't have to tune the buffer for small files.
const READ_BLOCK: usize = 64 * 1024;

#[derive(Debug, Error)]
pub enum IntegrityError {
    #[error("io: {0}: {1}")]
    Io(PathBuf, std::io::Error),
}

/// Hash a single file. Returns (size_bytes, hex_blake3). Streams the file so
/// arbitrarily large assets (music tracks, voice corpora) don't pull the
/// whole buffer into memory.
pub fn hash_path(path: &Path) -> Result<(u64, String), IntegrityError> {
    let mut file = File::open(path).map_err(|e| IntegrityError::Io(path.to_path_buf(), e))?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; READ_BLOCK];
    let mut total: u64 = 0;
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| IntegrityError::Io(path.to_path_buf(), e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        total += n as u64;
    }
    Ok((total, hex::encode(hasher.finalize().as_bytes())))
}

/// One verification result. Returned per-entry by `verify_entry`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyResult {
    pub id: AssetId,
    pub status: RegenStatus,
    /// The hex-encoded blake3 we observed on disk (empty if file missing /
    /// read failed).
    pub observed_blake3: String,
    pub observed_size_bytes: u64,
    /// Per-additional-output verification (mipmaps, normal maps).
    pub additional_results: Vec<VerifyResult>,
    /// Optional human-readable note (read error / size delta).
    pub note: Option<String>,
}

impl VerifyResult {
    pub fn is_fresh(&self) -> bool {
        matches!(self.status, RegenStatus::Fresh)
            && self
                .additional_results
                .iter()
                .all(|r| matches!(r.status, RegenStatus::Fresh))
    }
}

/// Compare an entry against the actual file at its `output_path`. Resolves
/// `output_path` relative to `base_dir` when it isn't already absolute, so
/// a ledger committed to git can live alongside a workspace-root content/
/// directory.
pub fn verify_entry(entry: &AssetEntry, base_dir: &Path) -> VerifyResult {
    let primary = verify_one(
        &entry.id,
        &resolve_path(&entry.output_path, base_dir),
        &entry.output_blake3,
        entry.output_size_bytes,
    );
    let mut additional_results = Vec::with_capacity(entry.additional_outputs.len());
    for extra in &entry.additional_outputs {
        let r = verify_additional(extra, base_dir);
        additional_results.push(r);
    }
    let combined_status = combine_status(&primary, &additional_results);
    VerifyResult {
        id: primary.id,
        status: combined_status,
        observed_blake3: primary.observed_blake3,
        observed_size_bytes: primary.observed_size_bytes,
        additional_results,
        note: primary.note,
    }
}

fn verify_additional(extra: &AdditionalOutput, base_dir: &Path) -> VerifyResult {
    // additional outputs share the parent entry's id; we tag them with a
    // synthetic AssetId derived from label+blake3 so the result can be
    // sorted alongside primaries without losing the parent linkage.
    let synthetic_id = AssetId(format!("{}:additional:{}", extra.blake3, extra.label));
    verify_one(
        &synthetic_id,
        &resolve_path(&extra.output_path, base_dir),
        &extra.blake3,
        extra.size_bytes,
    )
}

fn verify_one(id: &AssetId, path: &Path, expected_blake3: &str, expected_size: u64) -> VerifyResult {
    if !path.exists() {
        return VerifyResult {
            id: id.clone(),
            status: RegenStatus::Missing,
            observed_blake3: String::new(),
            observed_size_bytes: 0,
            additional_results: Vec::new(),
            note: Some(format!("output_path does not exist: {}", path.display())),
        };
    }
    match hash_path(path) {
        Ok((size, hex)) => {
            if hex == expected_blake3 {
                VerifyResult {
                    id: id.clone(),
                    status: RegenStatus::Fresh,
                    observed_blake3: hex,
                    observed_size_bytes: size,
                    additional_results: Vec::new(),
                    note: None,
                }
            } else {
                VerifyResult {
                    id: id.clone(),
                    status: RegenStatus::Drifted,
                    observed_blake3: hex,
                    observed_size_bytes: size,
                    additional_results: Vec::new(),
                    note: Some(format!(
                        "blake3 drift: expected {expected_blake3} ({expected_size}B), observed {size}B"
                    )),
                }
            }
        }
        Err(err) => VerifyResult {
            id: id.clone(),
            status: RegenStatus::Failed,
            observed_blake3: String::new(),
            observed_size_bytes: 0,
            additional_results: Vec::new(),
            note: Some(format!("io error: {err}")),
        },
    }
}

fn combine_status(primary: &VerifyResult, additional: &[VerifyResult]) -> RegenStatus {
    if primary.status != RegenStatus::Fresh {
        return primary.status;
    }
    for r in additional {
        if r.status != RegenStatus::Fresh {
            return r.status;
        }
    }
    RegenStatus::Fresh
}

fn resolve_path(path: &Path, base_dir: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;
    use crate::{
        category::{AssetCategory, ProductionTier},
        entry::AssetEntryBuilder,
    };

    static TMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn tmp_dir() -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let seq = TMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("cf-asset-ledger-test-{pid}-{nanos}-{seq}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn hash_path_is_deterministic() {
        let dir = tmp_dir();
        let p = dir.join("a.bin");
        std::fs::write(&p, b"hello world").unwrap();
        let (s1, h1) = hash_path(&p).unwrap();
        let (s2, h2) = hash_path(&p).unwrap();
        assert_eq!(s1, 11);
        assert_eq!(s2, 11);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn verify_entry_fresh() {
        let dir = tmp_dir();
        let p = dir.join("fresh.svg");
        std::fs::write(&p, b"<svg/>").unwrap();
        let (size, hash) = hash_path(&p).unwrap();
        let entry = AssetEntryBuilder::new(
            AssetCategory::WeaponSprite,
            "weapon-side",
            "fresh_test",
            ProductionTier::Tier1Svg,
            "M9A_svg_v1",
            "prompt",
            1,
            &p,
        )
        .with_output_blake3(&hash)
        .with_output_size(size)
        .build();
        let result = verify_entry(&entry, std::path::Path::new(""));
        assert_eq!(result.status, RegenStatus::Fresh);
    }

    #[test]
    fn verify_entry_missing() {
        let dir = tmp_dir();
        let p = dir.join("absent.svg");
        let entry = AssetEntryBuilder::new(
            AssetCategory::WeaponSprite,
            "weapon-side",
            "missing_test",
            ProductionTier::Tier1Svg,
            "M9A_svg_v1",
            "prompt",
            1,
            &p,
        )
        .with_output_blake3("0".repeat(64))
        .with_output_size(0)
        .build();
        let result = verify_entry(&entry, std::path::Path::new(""));
        assert_eq!(result.status, RegenStatus::Missing);
    }

    #[test]
    fn verify_entry_drift() {
        let dir = tmp_dir();
        let p = dir.join("drift.svg");
        std::fs::write(&p, b"<svg/>").unwrap();
        let entry = AssetEntryBuilder::new(
            AssetCategory::WeaponSprite,
            "weapon-side",
            "drift_test",
            ProductionTier::Tier1Svg,
            "M9A_svg_v1",
            "prompt",
            1,
            &p,
        )
        .with_output_blake3("f".repeat(64))
        .with_output_size(99999)
        .build();
        let result = verify_entry(&entry, std::path::Path::new(""));
        assert_eq!(result.status, RegenStatus::Drifted);
        let note = result.note.unwrap_or_default();
        assert!(note.contains("drift"), "missing drift note: {note}");
    }

    #[test]
    fn verify_entry_additional_drift_propagates() {
        let dir = tmp_dir();
        let primary = dir.join("primary.svg");
        std::fs::write(&primary, b"primary").unwrap();
        let extra = dir.join("extra.png");
        std::fs::write(&extra, b"extra").unwrap();
        let (psize, phash) = hash_path(&primary).unwrap();
        let entry = AssetEntryBuilder::new(
            AssetCategory::WeaponSprite,
            "weapon-side",
            "additional_test",
            ProductionTier::Tier1Svg,
            "M9A_svg_v1",
            "prompt",
            1,
            &primary,
        )
        .with_output_blake3(&phash)
        .with_output_size(psize)
        .with_additional_output(AdditionalOutput {
            label: "normal_map".to_string(),
            output_path: extra.clone(),
            blake3: "f".repeat(64),
            size_bytes: 999,
        })
        .build();
        let result = verify_entry(&entry, std::path::Path::new(""));
        assert_eq!(result.status, RegenStatus::Drifted);
    }

    #[test]
    fn verify_entry_via_base_dir() {
        // ledger paths can be relative to a base content directory
        let dir = tmp_dir();
        let rel = std::path::PathBuf::from("relative.svg");
        let full = dir.join(&rel);
        let mut f = std::fs::File::create(&full).unwrap();
        f.write_all(b"<svg/>").unwrap();
        let (size, hash) = hash_path(&full).unwrap();
        let entry = AssetEntryBuilder::new(
            AssetCategory::WeaponSprite,
            "weapon-side",
            "relative_test",
            ProductionTier::Tier1Svg,
            "M9A_svg_v1",
            "prompt",
            1,
            rel.clone(),
        )
        .with_output_blake3(&hash)
        .with_output_size(size)
        .build();
        let result = verify_entry(&entry, &dir);
        assert_eq!(result.status, RegenStatus::Fresh);
    }
}
