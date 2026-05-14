//! M4A re-bake engine.
//!
//! Given a ledger entry, the regenerator either:
//! 1. Re-runs the pipeline command stored in `regen_command`, then verifies
//!    the resulting blake3 matches `output_blake3` (full deterministic
//!    regen contract).
//! 2. Validates a "freeze-then-store" pipeline by reading a stored canonical
//!    copy from `<output_path>.frozen` and writing it back to `output_path`
//!    (used when the underlying pipeline is non-deterministic — e.g., GPU
//!    diffusion that varies bit-for-bit across hardware).
//!
//! M4A ships the second mode by default: pipelines like SVG (Tier 1) ARE
//! deterministic and write their own output_blake3, but the regenerator
//! never assumes shell access to a Python/ComfyUI stack. The freeze-then-
//! store path always works as long as the canonical frozen output is in
//! the workspace.
//!
//! Cascade regeneration: `regenerate_with_cascade` walks `upstream_assets`
//! in reverse-topological order so a Tier 1 regen also bakes the Tier 2
//! sprites that consumed it as ControlNet input.

use std::{
    collections::{BTreeSet, HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    process::Command,
};

use thiserror::Error;

use crate::{
    category::RegenStatus,
    entry::{AssetEntry, AssetId},
    integrity::{hash_path, verify_entry, VerifyResult},
    storage::{LedgerHandle, StorageError},
};

#[derive(Debug, Error)]
pub enum RegenError {
    #[error("entry {0} not found")]
    EntryNotFound(AssetId),
    #[error("pipeline command failed for {id}: {message}")]
    PipelineFailed { id: AssetId, message: String },
    #[error("blake3 mismatch after regen for {id}: expected {expected}, got {observed}")]
    BlakeMismatch {
        id: AssetId,
        expected: String,
        observed: String,
    },
    #[error("frozen canonical output missing: {0}")]
    FrozenMissing(PathBuf),
    #[error("io: {0}: {1}")]
    Io(PathBuf, std::io::Error),
    #[error("storage: {0}")]
    Storage(#[from] StorageError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegenOutcome {
    pub id: AssetId,
    pub used_freeze_path: bool,
    pub bytes_written: u64,
    pub final_blake3: String,
    pub status: RegenStatus,
}

/// Drive a single asset re-bake. The default path is "freeze-then-store":
/// the canonical output bytes live next to the workspace tree at
/// `<output_path>.frozen` (or, if absent, the entry's existing output_path
/// is taken as the canonical reference IF its blake3 already matches).
///
/// `pipeline_runner` is an optional callback that, when set, lets a pipeline
/// tool re-run its own command and write the output before verification.
/// Tier 0 + Tier 1 SVG (deterministic) can plug in here; Tier 2 ComfyUI
/// (non-deterministic) uses the freeze path.
///
/// **Caller responsibility (M4A spec § "Upstream asset dependency graph")**:
/// after regenerating a Tier 1 entry, dependents must be marked `Stale`. The
/// in-place version `regenerate_entry_with_handle` does this automatically;
/// this overload is the pure-compute primitive used by callers that don't
/// have a ledger handle in scope (e.g. unit tests).
pub fn regenerate_entry(
    entry: &AssetEntry,
    base_dir: &Path,
    pipeline_runner: Option<&dyn Fn(&AssetEntry, &Path) -> Result<(), RegenError>>,
) -> Result<RegenOutcome, RegenError> {
    let abs_output = resolve_path(&entry.output_path, base_dir);
    if let Some(runner) = pipeline_runner {
        runner(entry, &abs_output)?;
    } else {
        rebake_from_freeze(entry, &abs_output)?;
    }
    let (size, hex) = hash_path(&abs_output).map_err(|e| {
        RegenError::Io(
            abs_output.clone(),
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
        )
    })?;
    if hex != entry.output_blake3 {
        return Err(RegenError::BlakeMismatch {
            id: entry.id.clone(),
            expected: entry.output_blake3.clone(),
            observed: hex,
        });
    }
    Ok(RegenOutcome {
        id: entry.id.clone(),
        used_freeze_path: pipeline_runner.is_none(),
        bytes_written: size,
        final_blake3: hex,
        status: RegenStatus::Fresh,
    })
}

/// Like `regenerate_entry`, but ALSO calls `mark_dependents_stale` on the
/// provided handle after a successful regen — the M4A spec contract
/// "When the Tier 1 entry is regenerated, then dependents (Tier 2, Tier 3)
/// are marked Stale." Callers that already follow up with an explicit
/// cascade walk should call `regenerate_entry` instead to avoid the extra
/// disk pass.
pub fn regenerate_entry_with_handle(
    handle: &LedgerHandle,
    entry: &AssetEntry,
    base_dir: &Path,
    pipeline_runner: Option<&dyn Fn(&AssetEntry, &Path) -> Result<(), RegenError>>,
) -> Result<RegenOutcome, RegenError> {
    let outcome = regenerate_entry(entry, base_dir, pipeline_runner)?;
    let _ = mark_dependents_stale(handle, &entry.id)?;
    Ok(outcome)
}

fn rebake_from_freeze(entry: &AssetEntry, abs_output: &Path) -> Result<(), RegenError> {
    let freeze_path = freeze_path_for(abs_output);
    if !freeze_path.exists() {
        // Determinism contract escape hatch: if the live file already exists
        // AND its blake3 already matches the ledger, the freeze copy is the
        // current file (idempotent regen). We materialize the freeze for
        // future regens.
        if abs_output.exists() {
            let (_, hex) = hash_path(abs_output).map_err(|e| {
                RegenError::Io(
                    abs_output.to_path_buf(),
                    std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
                )
            })?;
            if hex == entry.output_blake3 {
                std::fs::copy(abs_output, &freeze_path).map_err(|e| RegenError::Io(freeze_path.clone(), e))?;
                return Ok(());
            }
        }
        return Err(RegenError::FrozenMissing(freeze_path));
    }
    if let Some(parent) = abs_output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| RegenError::Io(parent.to_path_buf(), e))?;
        }
    }
    std::fs::copy(&freeze_path, abs_output).map_err(|e| RegenError::Io(abs_output.to_path_buf(), e))?;
    Ok(())
}

pub fn freeze_path_for(output_path: &Path) -> PathBuf {
    let mut s = output_path.as_os_str().to_os_string();
    s.push(".frozen");
    PathBuf::from(s)
}

/// Snapshot the current output_path as the canonical freeze copy. Pipelines
/// run this at write-time so future regens can reproduce byte-for-byte from
/// the snapshot.
pub fn snapshot_freeze(output_path: &Path) -> Result<PathBuf, RegenError> {
    if !output_path.exists() {
        return Err(RegenError::FrozenMissing(output_path.to_path_buf()));
    }
    let freeze = freeze_path_for(output_path);
    if let Some(parent) = freeze.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| RegenError::Io(parent.to_path_buf(), e))?;
        }
    }
    std::fs::copy(output_path, &freeze).map_err(|e| RegenError::Io(freeze.clone(), e))?;
    Ok(freeze)
}

fn resolve_path(path: &Path, base_dir: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

/// Drive a full ledger re-bake. Walks `live_entries()` and regenerates each
/// in order. Stops on first error unless `continue_on_error` is set. Per
/// M4A spec § "Upstream asset dependency graph", a single-entry regen auto-
/// marks dependents `Stale`; `regenerate_all` skips that pass because every
/// entry is regenerated this sweep so dependents would just flip back to
/// `Fresh` immediately (and the supersede churn would cost O(N²)).
pub fn regenerate_all(
    handle: &LedgerHandle,
    base_dir: &Path,
    continue_on_error: bool,
) -> Result<Vec<RegenAttempt>, RegenError> {
    let entries = handle.live_entries()?;
    let mut results = Vec::with_capacity(entries.len());
    for entry in entries {
        // sweep-mode: pure regen, no dependent-stale marking (see doc above).
        let outcome = regenerate_entry(&entry, base_dir, None);
        match &outcome {
            Ok(o) => results.push(RegenAttempt {
                id: entry.id.clone(),
                ok: true,
                outcome: Some(o.clone()),
                error: None,
            }),
            Err(e) => {
                results.push(RegenAttempt {
                    id: entry.id.clone(),
                    ok: false,
                    outcome: None,
                    error: Some(e.to_string()),
                });
                if !continue_on_error {
                    return Ok(results);
                }
            }
        }
    }
    Ok(results)
}

/// Cascade regen: regenerate `id` AND every entry transitively dependent on
/// it (i.e. entries whose `upstream_assets` contains it). Order is
/// dependency-first so a Tier 1 SVG is baked BEFORE the Tier 2 ComfyUI
/// sprite that uses it as ControlNet input.
pub fn regenerate_with_cascade(
    handle: &LedgerHandle,
    root_id: &AssetId,
    base_dir: &Path,
) -> Result<Vec<RegenAttempt>, RegenError> {
    let entries = handle.live_entries()?;
    let by_id: HashMap<AssetId, &AssetEntry> = entries.iter().map(|e| (e.id.clone(), e)).collect();
    let mut reverse: HashMap<AssetId, Vec<AssetId>> = HashMap::new();
    for entry in &entries {
        for up in &entry.upstream_assets {
            reverse.entry(up.clone()).or_default().push(entry.id.clone());
        }
    }
    let order = topological_descendant_order(root_id, &reverse, &by_id);
    let mut results = Vec::with_capacity(order.len());
    for id in order {
        let Some(entry) = by_id.get(&id) else {
            results.push(RegenAttempt {
                id: id.clone(),
                ok: false,
                outcome: None,
                error: Some("entry not found in live set".to_string()),
            });
            continue;
        };
        let outcome = regenerate_entry(entry, base_dir, None);
        match outcome {
            Ok(o) => results.push(RegenAttempt {
                id: id.clone(),
                ok: true,
                outcome: Some(o),
                error: None,
            }),
            Err(e) => results.push(RegenAttempt {
                id: id.clone(),
                ok: false,
                outcome: None,
                error: Some(e.to_string()),
            }),
        }
    }
    Ok(results)
}

/// Mark every entry whose upstream_assets transitively touches `root_id` as
/// `Stale`. Used by `cf-mod ledger regenerate --cascade` to flag dependents
/// before the cascade runs, AND by external pipeline tools that want to
/// invalidate downstreams without re-baking immediately.
pub fn mark_dependents_stale(handle: &LedgerHandle, root_id: &AssetId) -> Result<Vec<AssetId>, RegenError> {
    let entries = handle.read_all()?;
    let mut reverse: HashMap<AssetId, Vec<AssetId>> = HashMap::new();
    for entry in &entries {
        for up in &entry.upstream_assets {
            reverse.entry(up.clone()).or_default().push(entry.id.clone());
        }
    }
    let mut visited: HashSet<AssetId> = HashSet::new();
    let mut queue: VecDeque<AssetId> = VecDeque::new();
    if let Some(deps) = reverse.get(root_id) {
        for d in deps {
            queue.push_back(d.clone());
        }
    }
    while let Some(id) = queue.pop_front() {
        if !visited.insert(id.clone()) {
            continue;
        }
        if let Some(deps) = reverse.get(&id) {
            for d in deps {
                queue.push_back(d.clone());
            }
        }
    }
    let stale_ids: Vec<AssetId> = visited.into_iter().collect();
    // Persist by rewriting the file with status flipped where appropriate.
    // (Append-only contract: this is the only sanctioned mutation surface.)
    if !stale_ids.is_empty() {
        rewrite_with_status(handle, &stale_ids, RegenStatus::Stale)?;
    }
    Ok(stale_ids)
}

fn rewrite_with_status(handle: &LedgerHandle, ids: &[AssetId], status: RegenStatus) -> Result<(), RegenError> {
    use fs2::FileExt;

    // Hold an exclusive advisory lock for the read-modify-write window so
    // concurrent CI workers don't race against each other (or against
    // supersede_entry / compact).
    let lock_file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(handle.path())
        .map_err(|e| RegenError::Io(handle.path().to_path_buf(), e))?;
    lock_file
        .lock_exclusive()
        .map_err(|e| RegenError::Io(handle.path().to_path_buf(), e))?;
    let result = rewrite_with_status_locked(handle, ids, status);
    let _ = FileExt::unlock(&lock_file);
    result
}

fn rewrite_with_status_locked(handle: &LedgerHandle, ids: &[AssetId], status: RegenStatus) -> Result<(), RegenError> {
    let entries = handle.read_all()?;
    let target: BTreeSet<&AssetId> = ids.iter().collect();
    let new_lines: Vec<String> = entries
        .into_iter()
        .map(|mut e| {
            if target.contains(&e.id) && e.superseded_by.is_none() {
                e.regen_status = status;
            }
            serde_json::to_string(&e).expect("serialize")
        })
        .collect();
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(handle.path())
        .map_err(|e| RegenError::Io(handle.path().to_path_buf(), e))?;
    use std::io::Write;
    for line in new_lines {
        file.write_all(line.as_bytes())
            .map_err(|e| RegenError::Io(handle.path().to_path_buf(), e))?;
        file.write_all(b"\n")
            .map_err(|e| RegenError::Io(handle.path().to_path_buf(), e))?;
    }
    file.flush()
        .map_err(|e| RegenError::Io(handle.path().to_path_buf(), e))?;
    Ok(())
}

fn topological_descendant_order(
    root: &AssetId,
    reverse: &HashMap<AssetId, Vec<AssetId>>,
    by_id: &HashMap<AssetId, &AssetEntry>,
) -> Vec<AssetId> {
    let mut order = vec![root.clone()];
    let mut visited: HashSet<AssetId> = [root.clone()].into_iter().collect();
    let mut queue: VecDeque<AssetId> = VecDeque::from([root.clone()]);
    while let Some(id) = queue.pop_front() {
        let Some(deps) = reverse.get(&id) else {
            continue;
        };
        for d in deps {
            if visited.insert(d.clone()) {
                order.push(d.clone());
                queue.push_back(d.clone());
            }
        }
    }
    order.retain(|id| by_id.contains_key(id));
    order
}

/// Each per-entry attempt during a sweep regen. Keeps `id` even on failure
/// so the CLI can report the failed ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegenAttempt {
    pub id: AssetId,
    pub ok: bool,
    pub outcome: Option<RegenOutcome>,
    pub error: Option<String>,
}

/// Run a pipeline command via `std::process::Command`. Splits on whitespace
/// (the regen_command stored in the ledger is the literal shell-compatible
/// form). Best-effort: pipeline tools that need stdin/env should expose
/// their own runner closure to `regenerate_entry`.
pub fn run_pipeline_command(entry: &AssetEntry, _abs_output: &Path) -> Result<(), RegenError> {
    let parts: Vec<&str> = entry.regen_command.split_whitespace().collect();
    if parts.is_empty() {
        return Err(RegenError::PipelineFailed {
            id: entry.id.clone(),
            message: "regen_command is empty".to_string(),
        });
    }
    let status = Command::new(parts[0])
        .args(&parts[1..])
        .status()
        .map_err(|e| RegenError::PipelineFailed {
            id: entry.id.clone(),
            message: e.to_string(),
        })?;
    if !status.success() {
        return Err(RegenError::PipelineFailed {
            id: entry.id.clone(),
            message: format!("non-zero exit: {status}"),
        });
    }
    Ok(())
}

/// Convenience: verify every live entry in the ledger. Returns the
/// per-entry verify result.
pub fn verify_all(handle: &LedgerHandle, base_dir: &Path) -> Result<Vec<VerifyResult>, StorageError> {
    let entries = handle.live_entries()?;
    Ok(entries.iter().map(|e| verify_entry(e, base_dir)).collect())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::{
        category::{AssetCategory, ProductionTier},
        entry::{AssetEntryBuilder, RegenInputRef},
    };

    static TMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn tmp_dir() -> PathBuf {
        // PID + atomic counter is enough for uniqueness; SystemTime::now is
        // disallowed by the workspace clippy lint.
        let seq = TMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("cf-asset-ledger-regen-{pid}-{seq}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn freeze_then_store_round_trip() {
        let dir = tmp_dir();
        let output = dir.join("foo.svg");
        std::fs::write(&output, b"<svg/>").unwrap();
        let (_size, hash) = hash_path(&output).unwrap();
        let _ = snapshot_freeze(&output).unwrap();
        let entry = AssetEntryBuilder::new(
            AssetCategory::WeaponSprite,
            "weapon-side",
            "regen_test",
            ProductionTier::Tier1Svg,
            "M9A_svg_v1",
            "prompt",
            1,
            &output,
        )
        .with_output_blake3(&hash)
        .build();
        // Mutate the output file (simulate drift)
        std::fs::write(&output, b"corrupted").unwrap();
        // Regen restores from the freeze
        let outcome = regenerate_entry(&entry, std::path::Path::new(""), None).expect("regen");
        assert_eq!(outcome.final_blake3, hash);
        assert!(outcome.used_freeze_path);
        let actual = std::fs::read_to_string(&output).unwrap();
        assert_eq!(actual, "<svg/>");
    }

    #[test]
    fn freeze_path_picks_predictable_suffix() {
        let p = PathBuf::from("/tmp/a.svg");
        let frozen = freeze_path_for(&p);
        assert_eq!(frozen, PathBuf::from("/tmp/a.svg.frozen"));
    }

    #[test]
    fn regen_all_walks_every_live_entry() {
        let dir = tmp_dir();
        let ledger_path = dir.join("ledger.jsonl");
        let handle = LedgerHandle::new(&ledger_path);
        for i in 0..3 {
            let out = dir.join(format!("e{i}.svg"));
            std::fs::write(&out, format!("entry-{i}").as_bytes()).unwrap();
            let (_, hash) = hash_path(&out).unwrap();
            let _ = snapshot_freeze(&out).unwrap();
            let entry = AssetEntryBuilder::new(
                AssetCategory::WeaponSprite,
                "kind",
                format!("entry_{i}"),
                ProductionTier::Tier1Svg,
                "M9A_svg_v1",
                "prompt",
                i as u64,
                out,
            )
            .with_output_blake3(&hash)
            .build();
            handle.append(&entry).unwrap();
        }
        let results = regenerate_all(&handle, std::path::Path::new(""), false).expect("ok");
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.ok));
    }

    #[test]
    fn cascade_walks_dependents() {
        let dir = tmp_dir();
        let ledger_path = dir.join("ledger.jsonl");
        let handle = LedgerHandle::new(&ledger_path);

        // Tier 1 SVG
        let svg_path = dir.join("rifle.svg");
        std::fs::write(&svg_path, b"<svg/>").unwrap();
        let (_, svg_hash) = hash_path(&svg_path).unwrap();
        let _ = snapshot_freeze(&svg_path).unwrap();
        let tier1 = AssetEntryBuilder::new(
            AssetCategory::WeaponSprite,
            "weapon-side",
            "tier1_rifle",
            ProductionTier::Tier1Svg,
            "M9A_svg_v1",
            "prompt",
            1,
            svg_path,
        )
        .with_output_blake3(&svg_hash)
        .build();
        let tier1_id = tier1.id.clone();
        handle.append(&tier1).unwrap();

        // Tier 2 ComfyUI depending on Tier 1
        let webp_path = dir.join("rifle_comfy.webp");
        std::fs::write(&webp_path, b"comfyui-output").unwrap();
        let (_, webp_hash) = hash_path(&webp_path).unwrap();
        let _ = snapshot_freeze(&webp_path).unwrap();
        let tier2 = AssetEntryBuilder::new(
            AssetCategory::WeaponSprite,
            "weapon-side",
            "tier2_rifle",
            ProductionTier::Tier2ComfyUi,
            "M32A_comfyui_v1",
            "comfyui prompt",
            42,
            webp_path,
        )
        .with_output_blake3(&webp_hash)
        .with_upstream([tier1_id.clone()])
        .with_regen_inputs([RegenInputRef::AssetId(tier1_id.clone())])
        .build();
        let tier2_id = tier2.id.clone();
        handle.append(&tier2).unwrap();

        let results = regenerate_with_cascade(&handle, &tier1_id, std::path::Path::new("")).expect("cascade");
        // Tier 1 first, then Tier 2
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, tier1_id);
        assert_eq!(results[1].id, tier2_id);
        assert!(results.iter().all(|r| r.ok));
    }

    /// **M4A spec § "Upstream asset dependency graph"** contract:
    /// "When the Tier 1 entry is regenerated, then dependents (Tier 2,
    /// Tier 3) are marked Stale." This test pins the auto-stale-marking
    /// behaviour of `regenerate_entry_with_handle` so future refactors
    /// can't silently drop the side effect.
    #[test]
    fn regenerate_entry_with_handle_auto_marks_dependents_stale() {
        let dir = tmp_dir();
        let ledger_path = dir.join("ledger.jsonl");
        let handle = LedgerHandle::new(&ledger_path);

        let svg_path = dir.join("rifle.svg");
        std::fs::write(&svg_path, b"<svg/>").unwrap();
        let (_, svg_hash) = hash_path(&svg_path).unwrap();
        let _ = snapshot_freeze(&svg_path).unwrap();
        let tier1 = AssetEntryBuilder::new(
            AssetCategory::WeaponSprite,
            "weapon-side",
            "tier1_auto",
            ProductionTier::Tier1Svg,
            "M9A_svg_v1",
            "p",
            1,
            svg_path,
        )
        .with_output_blake3(&svg_hash)
        .build();
        let tier1_id = tier1.id.clone();
        handle.append(&tier1).unwrap();

        let comfy_path = dir.join("rifle_comfy.webp");
        std::fs::write(&comfy_path, b"comfy").unwrap();
        let (_, comfy_hash) = hash_path(&comfy_path).unwrap();
        let _ = snapshot_freeze(&comfy_path).unwrap();
        let tier2 = AssetEntryBuilder::new(
            AssetCategory::WeaponSprite,
            "weapon-side",
            "tier2_auto",
            ProductionTier::Tier2ComfyUi,
            "M32A_comfyui_v1",
            "p",
            2,
            comfy_path,
        )
        .with_output_blake3(&comfy_hash)
        .with_upstream([tier1_id.clone()])
        .with_regen_status(RegenStatus::Fresh)
        .build();
        let tier2_id = tier2.id.clone();
        handle.append(&tier2).unwrap();

        // Regenerate the Tier 1 entry via the handle-aware overload.
        let outcome = regenerate_entry_with_handle(&handle, &tier1, std::path::Path::new(""), None).expect("regen");
        assert_eq!(outcome.id, tier1_id);
        assert_eq!(outcome.status, RegenStatus::Fresh);

        // Tier 2 dependent MUST now be Stale even though we never called
        // mark_dependents_stale explicitly.
        let entries = handle.read_all().unwrap();
        let live_tier2 = entries.into_iter().find(|e| e.id == tier2_id).unwrap();
        assert_eq!(
            live_tier2.regen_status,
            RegenStatus::Stale,
            "Tier 1 regen must auto-mark Tier 2 dependents as Stale"
        );
    }

    #[test]
    fn mark_dependents_stale_flips_descendants() {
        let dir = tmp_dir();
        let ledger_path = dir.join("ledger.jsonl");
        let handle = LedgerHandle::new(&ledger_path);

        let svg_path = dir.join("a.svg");
        std::fs::write(&svg_path, b"<svg/>").unwrap();
        let (_, h1) = hash_path(&svg_path).unwrap();
        let tier1 = AssetEntryBuilder::new(
            AssetCategory::WeaponSprite,
            "weapon-side",
            "a",
            ProductionTier::Tier1Svg,
            "M9A_svg_v1",
            "p",
            1,
            svg_path,
        )
        .with_output_blake3(&h1)
        .build();
        let tier1_id = tier1.id.clone();
        handle.append(&tier1).unwrap();

        let comfy_path = dir.join("a_comfy.webp");
        std::fs::write(&comfy_path, b"comfy").unwrap();
        let (_, h2) = hash_path(&comfy_path).unwrap();
        let tier2 = AssetEntryBuilder::new(
            AssetCategory::WeaponSprite,
            "weapon-side",
            "a_comfy",
            ProductionTier::Tier2ComfyUi,
            "M32A_comfyui_v1",
            "p",
            2,
            comfy_path,
        )
        .with_output_blake3(&h2)
        .with_upstream([tier1_id.clone()])
        .with_regen_status(RegenStatus::Fresh)
        .build();
        let tier2_id = tier2.id.clone();
        handle.append(&tier2).unwrap();

        let stale = mark_dependents_stale(&handle, &tier1_id).expect("ok");
        assert_eq!(stale.len(), 1);
        let entries = handle.read_all().unwrap();
        let t2 = entries.into_iter().find(|e| e.id == tier2_id).unwrap();
        assert_eq!(t2.regen_status, RegenStatus::Stale);
    }
}
