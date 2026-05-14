//! `cf-asset-ledger` — M4A asset ledger foundation.
//!
//! Append-only JSONL ledger + per-entry schema + integrity verifier + re-bake
//! engine. Every downstream asset-pipeline milestone (M9A SVG, M12A audio,
//! M18A animation, M24A VFX, M25A narrative, M32A ComfyUI, M37A voice/music,
//! M38A localization, M45A cosmetic, M48A polish, M48B marketing) writes an
//! entry per generated asset; the engine and modders cross-reference the
//! ledger by [`entry::AssetId`] for byte-identical regen across machines.
//!
//! The schema is locked at v1.0.0 ([`entry::ASSET_ENTRY_SCHEMA_VERSION`]).
//! New fields must be `serde(default)` so the JSONL stream remains
//! forward-compatible without a version bump. Layout-breaking changes bump
//! to v2 with a migration shim registered at M39.
//!
//! Public API contract (per M4A spec):
//! - [`add_entry`] / [`AssetEntry`] — append-only writer.
//! - [`list_entries`] — read + filter live entries.
//! - [`regenerate_entry`] / [`regenerator::regenerate_all`] — byte-identical
//!   re-bake via the freeze-then-store path.
//! - [`verify_entry`] — blake3-based drift detection (`Fresh` / `Stale` /
//!   `Drifted` / `Missing` / `Failed`).

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::module_name_repetitions,
    clippy::doc_markdown,
    clippy::struct_excessive_bools,
    clippy::missing_const_for_fn,
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::cast_sign_loss,
    clippy::needless_pass_by_value,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::manual_let_else,
    clippy::manual_find,
    clippy::manual_string_new,
    clippy::single_match_else,
    clippy::if_not_else,
    clippy::redundant_closure_for_method_calls,
    clippy::redundant_else,
    clippy::format_in_format_args,
    clippy::useless_format,
    clippy::uninlined_format_args,
    clippy::trivially_copy_pass_by_ref,
    clippy::match_same_arms,
    clippy::derivable_impls,
    clippy::assigning_clones,
    clippy::type_complexity,
    clippy::if_same_then_else,
    clippy::items_after_statements,
    clippy::io_other_error,
    clippy::return_self_not_must_use
)]

pub mod category;
pub mod cli;
pub mod entry;
pub mod integrity;
pub mod regenerator;
pub mod storage;

pub use category::{AssetCategory, License, PackageRef, ProductionTier, RegenStatus};
pub use cli::{
    cmd_add, cmd_compact, cmd_diff, cmd_list, cmd_regenerate, cmd_register_pack, cmd_show, cmd_summary, cmd_verify,
    render_regen_attempts, render_summary, render_verify_report, summary_to_observe_json, AddArgs, LedgerPaths,
    RegisterPackArgs, RegisteredAsset, VerifyReport,
};
pub use entry::{
    AdditionalOutput, AssetEntry, AssetEntryBuilder, AssetId, GeneratorRef, LoraRef, PaletteId, PipelineId,
    RegenInputRef, ASSET_ENTRY_SCHEMA_VERSION,
};
pub use integrity::{hash_path, verify_entry, IntegrityError, VerifyResult};
pub use regenerator::{
    freeze_path_for, mark_dependents_stale, regenerate_all, regenerate_entry, regenerate_entry_with_handle,
    regenerate_with_cascade, run_pipeline_command, snapshot_freeze, verify_all, RegenAttempt, RegenError, RegenOutcome,
};
pub use storage::{summarize, CompactStats, LedgerHandle, LedgerSummary, ListFilter, StorageError};

/// Convenience: append a new entry to the ledger at `handle`. Mirrors the
/// public-API contract listed in the M4A acceptance criteria.
pub fn add_entry(handle: &LedgerHandle, entry: &AssetEntry) -> Result<(), StorageError> {
    handle.append(entry)
}

/// Convenience: read every live entry (drop superseded history).
pub fn list_entries(handle: &LedgerHandle, filter: &ListFilter) -> Result<Vec<AssetEntry>, StorageError> {
    let entries = handle.read_all()?;
    Ok(entries.into_iter().filter(|e| filter.matches(e)).collect())
}

/// Validate that a JSON value matches the v1 AssetEntry schema. Used by
/// `cf-mod validate` to check mod-supplied ledger lines + by CI gates that
/// want to catch malformed entries before they're committed.
pub fn validate_entry_json(value: &serde_json::Value) -> Result<(), String> {
    let entry: AssetEntry =
        serde_json::from_value(value.clone()).map_err(|e| format!("AssetEntry schema rejection: {e}"))?;
    if entry.schema_version != ASSET_ENTRY_SCHEMA_VERSION {
        return Err(format!(
            "schema_version_mismatch: expected {ASSET_ENTRY_SCHEMA_VERSION}, got {}",
            entry.schema_version
        ));
    }
    if entry.id.as_str().len() != 64 || !entry.id.as_str().chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("id_not_blake3_hex: {}", entry.id.as_str()));
    }
    let computed = AssetId::compute(entry.category, &entry.canonical_name, entry.tier);
    if computed != entry.id {
        return Err(format!(
            "id_drift: ledger id {} != computed id {} from (category={}, canonical_name={}, tier={})",
            entry.id.as_str(),
            computed.as_str(),
            entry.category.as_str(),
            entry.canonical_name,
            entry.tier.as_str()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::AssetEntryBuilder;
    use std::path::PathBuf;

    static TMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn tmp_path() -> PathBuf {
        // PID + atomic counter is enough for uniqueness across parallel
        // test runners; we deliberately avoid SystemTime::now because the
        // workspace clippy.toml disallows it (determinism discipline).
        let seq = TMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("cf-asset-ledger-{pid}-{seq}.jsonl"))
    }

    #[test]
    fn public_api_round_trip() {
        let path = tmp_path();
        let handle = LedgerHandle::new(&path);
        let entry = AssetEntryBuilder::new(
            AssetCategory::WeaponSprite,
            "weapon-side",
            "rifle_test_v1",
            ProductionTier::Tier1Svg,
            "M9A_svg_v1",
            "prompt",
            42,
            "/tmp/rifle.svg",
        )
        .with_output_blake3("a".repeat(64))
        .with_output_size(100)
        .build();
        add_entry(&handle, &entry).unwrap();
        let listed = list_entries(&handle, &ListFilter::default()).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].canonical_name, "rifle_test_v1");
        let _ = std::fs::remove_file(&path);
    }

    /// AssetEntry schema version is locked at v1.0.0 per M4A acceptance.
    #[test]
    fn schema_version_locked_at_v1() {
        assert_eq!(ASSET_ENTRY_SCHEMA_VERSION, "1.0.0");
    }

    #[test]
    fn validate_entry_json_accepts_canonical_entry() {
        let entry = AssetEntryBuilder::new(
            AssetCategory::WeaponSprite,
            "weapon-side",
            "rifle_validate",
            ProductionTier::Tier1Svg,
            "M9A_svg_v1",
            "prompt",
            42,
            "/tmp/rifle_validate.svg",
        )
        .with_output_blake3("a".repeat(64))
        .with_output_size(1)
        .build();
        let json = serde_json::to_value(&entry).unwrap();
        assert!(validate_entry_json(&json).is_ok());
    }

    #[test]
    fn validate_entry_json_rejects_id_drift() {
        let mut entry = AssetEntryBuilder::new(
            AssetCategory::WeaponSprite,
            "weapon-side",
            "rifle_drift",
            ProductionTier::Tier1Svg,
            "M9A_svg_v1",
            "prompt",
            42,
            "/tmp/rifle_drift.svg",
        )
        .with_output_blake3("a".repeat(64))
        .build();
        // Corrupt the id manually.
        entry.id = AssetId("0".repeat(64));
        let json = serde_json::to_value(&entry).unwrap();
        let err = validate_entry_json(&json).expect_err("id_drift must fail validation");
        assert!(err.contains("id_drift"), "expected id_drift error, got {err}");
    }

    #[test]
    fn validate_entry_json_rejects_schema_drift() {
        let mut entry = AssetEntryBuilder::new(
            AssetCategory::WeaponSprite,
            "weapon-side",
            "rifle_sv",
            ProductionTier::Tier1Svg,
            "M9A_svg_v1",
            "prompt",
            42,
            "/tmp/rifle_sv.svg",
        )
        .with_output_blake3("a".repeat(64))
        .build();
        entry.schema_version = "2.0.0".to_string();
        let json = serde_json::to_value(&entry).unwrap();
        let err = validate_entry_json(&json).expect_err("schema drift must fail");
        assert!(err.contains("schema_version_mismatch"));
    }
}
