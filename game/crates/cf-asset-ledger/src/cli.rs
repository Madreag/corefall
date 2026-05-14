//! M4A CLI module — the implementation of `cf-mod ledger ...`.
//!
//! The actual `clap` integration lives in `cf-mod/src/main.rs`; this module
//! holds the executable verb implementations so they can be unit-tested
//! directly AND wired identically by mod-pack tools or downstream pipelines.

use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use serde_json::{json, Value};

use crate::{
    category::{AssetCategory, License, PackageRef, ProductionTier, RegenStatus},
    entry::{AssetEntry, AssetEntryBuilder, AssetId, GeneratorRef, RegenInputRef},
    integrity::{verify_entry, VerifyResult},
    regenerator::{
        regenerate_all, regenerate_entry, regenerate_entry_with_handle, regenerate_with_cascade, snapshot_freeze,
        RegenAttempt, RegenError,
    },
    storage::{summarize, LedgerHandle, LedgerSummary, ListFilter},
};

/// Hosting policy for the CLI. The default writes to
/// `content/asset_ledger/ledger.jsonl` relative to the workspace root.
/// Tests inject a temp path. Pipeline tools may pass their own root for
/// per-mod sub-ledgers (later merged at build-end).
#[derive(Debug, Clone)]
pub struct LedgerPaths {
    pub ledger_path: PathBuf,
    pub base_dir: PathBuf,
}

impl LedgerPaths {
    pub fn default_for(workspace_root: &Path) -> Self {
        Self {
            ledger_path: workspace_root.join("content/asset_ledger/ledger.jsonl"),
            base_dir: workspace_root.to_path_buf(),
        }
    }
    pub fn handle(&self) -> LedgerHandle {
        LedgerHandle::new(&self.ledger_path)
    }
}

/// Common args parsed by `cf-mod ledger add`. Mirrors the spec's CLI table.
#[derive(Debug, Clone)]
pub struct AddArgs {
    pub category: String,
    pub kind: String,
    pub canonical_name: String,
    pub tier: String,
    pub pipeline: String,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub seed: u64,
    pub output_path: PathBuf,
    pub generator_tool: Option<String>,
    pub generator_model: Option<String>,
    pub generator_workflow: Option<String>,
    pub generator_model_version: Option<String>,
    pub palette: Option<String>,
    pub style_lora: Option<String>,
    pub upstream: Vec<String>,
    pub package_source: Option<String>,
    pub license: Option<String>,
    pub generated_by_human: bool,
    pub human_edit_notes: Option<String>,
    pub regen_command: Option<String>,
    pub freeze: bool,
    /// **M4A determinism**: override the entry's `generated_at_iso` field.
    /// When unset, the builder picks `chrono::Utc::now().to_rfc3339()`
    /// (or a deterministic placeholder when `CF_DETERMINISTIC_LEDGER=1`).
    /// Pipeline tools that want byte-reproducible ledger.jsonl across
    /// machines should set this to the source-content's commit time OR
    /// to a per-asset stable string.
    pub generated_at_iso: Option<String>,
    /// **M4A determinism**: override `generated_on_machine`. Defaults to
    /// `HOSTNAME` / `COMPUTERNAME` / `"unknown"` (or `"deterministic"`
    /// when the env flag is set).
    pub generated_on_machine: Option<String>,
}

pub fn cmd_add(paths: &LedgerPaths, args: &AddArgs) -> Result<AssetEntry> {
    let category =
        AssetCategory::parse(&args.category).ok_or_else(|| anyhow!("unknown category: {}", args.category))?;
    let tier = ProductionTier::parse(&args.tier).ok_or_else(|| anyhow!("unknown tier: {}", args.tier))?;
    let resolved_output = resolve_for_io(&args.output_path, &paths.base_dir);
    let mut builder = AssetEntryBuilder::new(
        category,
        args.kind.clone(),
        args.canonical_name.clone(),
        tier,
        args.pipeline.clone(),
        args.prompt.clone(),
        args.seed,
        args.output_path.clone(),
    );
    if let Some(neg) = &args.negative_prompt {
        builder = builder.with_negative_prompt(neg.clone());
    }
    if let Some(pal) = &args.palette {
        builder = builder.with_palette(pal.clone());
    }
    if let Some(lora) = &args.style_lora {
        builder = builder.with_style_lora(lora.clone());
    }
    if !args.upstream.is_empty() {
        let ids: Vec<AssetId> = args.upstream.iter().map(|s| AssetId(s.clone())).collect();
        builder = builder.with_regen_inputs(ids.iter().cloned().map(RegenInputRef::AssetId));
        builder = builder.with_upstream(ids);
    }
    let mut gen = GeneratorRef::default();
    if let Some(t) = &args.generator_tool {
        gen.tool = t.clone();
    }
    if let Some(m) = &args.generator_model {
        gen.model = m.clone();
    }
    if let Some(w) = &args.generator_workflow {
        gen.workflow = Some(w.clone());
    }
    if let Some(v) = &args.generator_model_version {
        gen.model_version = Some(v.clone());
    }
    if gen != GeneratorRef::default() {
        builder = builder.with_generator(gen);
    }
    if let Some(license_str) = &args.license {
        builder = builder.with_license(parse_license(license_str));
    }
    if let Some(pkg) = &args.package_source {
        builder = builder.with_package_source(parse_package_source(pkg));
    }
    if args.generated_by_human {
        builder = builder.with_human_edit(true, args.human_edit_notes.clone());
    }
    if let Some(cmd) = &args.regen_command {
        builder = builder.with_regen_command(cmd.clone());
    }
    if let Some(iso) = &args.generated_at_iso {
        builder = builder.with_generated_at_iso(iso.clone());
    }
    if let Some(host) = &args.generated_on_machine {
        builder = builder.with_generated_on_machine(host.clone());
    }
    // Hash file at write-time. If it exists, builder picks up its blake3
    // automatically; if not, the entry is marked Missing.
    if !resolved_output.exists() {
        bail!(
            "output file does not exist at {}; produce the asset before calling `cf-mod ledger add`",
            resolved_output.display()
        );
    }
    let (size, hash) = crate::integrity::hash_path(&resolved_output)
        .with_context(|| format!("hash output at {}", resolved_output.display()))?;
    builder = builder.with_output_blake3(hash).with_output_size(size);
    let entry = builder.with_regen_status(RegenStatus::Fresh).build();
    let handle = paths.handle();
    // Count the prior live entries with this id BEFORE appending. If any
    // exist, we'll back-fill them as superseded after the new entry lands.
    let prior_live_count = handle
        .read_all()
        .context("read ledger pre-append")?
        .iter()
        .filter(|e| e.id == entry.id && e.superseded_by.is_none())
        .count();
    handle.append(&entry).context("append ledger entry")?;
    if prior_live_count > 0 {
        handle
            .supersede_entry(&entry.id, &entry.id)
            .context("supersede prior entry")?;
    }
    if args.freeze {
        snapshot_freeze(&resolved_output).context("snapshot freeze")?;
    }
    Ok(entry)
}

fn parse_license(s: &str) -> License {
    let lower = s.to_ascii_lowercase();
    match lower.as_str() {
        "cc0" => License::Cc0,
        "cc-by" | "ccby" => License::CcBy,
        "cc-by-sa" | "ccbysa" => License::CcBySa,
        "proprietary" => License::Proprietary,
        other if other.starts_with("mod:") => License::ModSupplied(s.trim_start_matches("mod:").to_string()),
        _ => License::Custom(s.to_string()),
    }
}

fn parse_package_source(s: &str) -> PackageRef {
    if let Some(rest) = s.strip_prefix("mod:") {
        PackageRef::Mod(rest.to_string())
    } else if let Some(rest) = s.strip_prefix("faction-pack:") {
        PackageRef::FactionPack(rest.to_string())
    } else if s.eq_ignore_ascii_case("vanilla") {
        PackageRef::Vanilla
    } else {
        PackageRef::Mod(s.to_string())
    }
}

pub fn cmd_list(paths: &LedgerPaths, filter: &ListFilter) -> Result<Vec<AssetEntry>> {
    let handle = paths.handle();
    let entries = handle.read_all().context("read ledger")?;
    Ok(entries.into_iter().filter(|e| filter.matches(e)).collect())
}

pub fn cmd_show(paths: &LedgerPaths, id: &str) -> Result<AssetEntry> {
    let handle = paths.handle();
    let asset_id = match_id(&handle, id)?;
    handle
        .find(&asset_id)
        .context("read ledger")?
        .ok_or_else(|| anyhow!("entry not found: {id}"))
}

fn match_id(handle: &LedgerHandle, id_or_prefix: &str) -> Result<AssetId> {
    if id_or_prefix.len() == 64 && id_or_prefix.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(AssetId(id_or_prefix.to_string()));
    }
    let entries = handle.read_all()?;
    let matches: Vec<&AssetEntry> = entries
        .iter()
        .filter(|e| e.id.as_str().starts_with(id_or_prefix) || e.canonical_name == id_or_prefix)
        .collect();
    match matches.as_slice() {
        [] => bail!("no entry matches id-or-prefix {id_or_prefix}"),
        [only] => Ok(only.id.clone()),
        many => {
            let ids: Vec<String> = many.iter().map(|e| e.id.as_str().to_string()).collect();
            bail!(
                "ambiguous id prefix {id_or_prefix} matches {} entries: {ids:?}",
                many.len()
            )
        }
    }
}

pub fn cmd_diff(paths: &LedgerPaths, id: Option<&str>) -> Result<Vec<VerifyResult>> {
    let handle = paths.handle();
    let entries = if let Some(id) = id {
        let asset_id = match_id(&handle, id)?;
        let entry = handle
            .find(&asset_id)
            .context("read")?
            .ok_or_else(|| anyhow!("entry not found: {id}"))?;
        vec![entry]
    } else {
        handle.live_entries().context("live entries")?
    };
    Ok(entries.iter().map(|e| verify_entry(e, &paths.base_dir)).collect())
}

pub fn cmd_verify(paths: &LedgerPaths, id: Option<&str>, strict: bool) -> Result<VerifyReport> {
    let results = cmd_diff(paths, id)?;
    let mut report = VerifyReport {
        total: results.len() as u64,
        fresh: 0,
        stale: 0,
        drifted: 0,
        missing: 0,
        failed: 0,
        results: results.clone(),
        strict_failed: false,
    };
    for r in &results {
        match r.status {
            RegenStatus::Fresh => report.fresh += 1,
            RegenStatus::Stale => report.stale += 1,
            RegenStatus::Drifted => report.drifted += 1,
            RegenStatus::Missing => report.missing += 1,
            RegenStatus::Failed => report.failed += 1,
        }
    }
    if strict && (report.drifted + report.missing + report.failed) > 0 {
        report.strict_failed = true;
    }
    Ok(report)
}

#[derive(Debug, Clone, Serialize)]
pub struct VerifyReport {
    pub total: u64,
    pub fresh: u64,
    pub stale: u64,
    pub drifted: u64,
    pub missing: u64,
    pub failed: u64,
    pub results: Vec<VerifyResult>,
    pub strict_failed: bool,
}

impl VerifyReport {
    pub fn is_strict_ok(&self) -> bool {
        !self.strict_failed
    }
}

impl serde::Serialize for VerifyResult {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("VerifyResult", 5)?;
        s.serialize_field("id", self.id.as_str())?;
        s.serialize_field("status", self.status.as_str())?;
        s.serialize_field("observed_blake3", &self.observed_blake3)?;
        s.serialize_field("observed_size_bytes", &self.observed_size_bytes)?;
        s.serialize_field("note", &self.note)?;
        s.end()
    }
}

pub fn cmd_regenerate(
    paths: &LedgerPaths,
    id: Option<&str>,
    cascade: bool,
    category: Option<AssetCategory>,
    tier: Option<ProductionTier>,
    all: bool,
    continue_on_error: bool,
) -> Result<Vec<RegenAttempt>> {
    let handle = paths.handle();
    if all {
        return regenerate_all(&handle, &paths.base_dir, continue_on_error).map_err(map_regen_err);
    }
    if let Some(id) = id {
        let asset_id = match_id(&handle, id)?;
        if cascade {
            return regenerate_with_cascade(&handle, &asset_id, &paths.base_dir).map_err(map_regen_err);
        }
        let entry = handle
            .find(&asset_id)
            .context("read")?
            .ok_or_else(|| anyhow!("entry not found: {id}"))?;
        // **M4A spec contract**: single-entry regen auto-marks dependents
        // Stale. Use `_with_handle` so the caller (cf-mod CLI) doesn't have
        // to remember to follow up with mark_dependents_stale.
        let outcome = regenerate_entry_with_handle(&handle, &entry, &paths.base_dir, None).map_err(map_regen_err)?;
        return Ok(vec![RegenAttempt {
            id: entry.id,
            ok: true,
            outcome: Some(outcome),
            error: None,
        }]);
    }
    if category.is_some() || tier.is_some() {
        let filter = ListFilter {
            category,
            tier,
            ..ListFilter::default()
        };
        let entries: Vec<AssetEntry> = handle
            .live_entries()
            .context("live entries")?
            .into_iter()
            .filter(|e| filter.matches(e))
            .collect();
        let mut out = Vec::with_capacity(entries.len());
        for entry in entries {
            match regenerate_entry(&entry, &paths.base_dir, None) {
                Ok(o) => out.push(RegenAttempt {
                    id: entry.id,
                    ok: true,
                    outcome: Some(o),
                    error: None,
                }),
                Err(e) => out.push(RegenAttempt {
                    id: entry.id,
                    ok: false,
                    outcome: None,
                    error: Some(e.to_string()),
                }),
            }
        }
        return Ok(out);
    }
    bail!("cf-mod ledger regenerate requires an id, --cascade <id>, --category/--tier filters, or --all");
}

fn map_regen_err(err: RegenError) -> anyhow::Error {
    anyhow!(err.to_string())
}

pub fn cmd_summary(paths: &LedgerPaths) -> Result<LedgerSummary> {
    let handle = paths.handle();
    let entries = handle.read_all().context("read")?;
    Ok(summarize(&entries))
}

pub fn cmd_compact(
    paths: &LedgerPaths,
    keep_latest: bool,
    before: Option<&str>,
) -> Result<crate::storage::CompactStats> {
    if let Some(s) = before {
        // Validate the cutoff at parse-time so a malformed `--before
        // not-a-date` doesn't silently drop entries via lexical compare.
        chrono::DateTime::parse_from_rfc3339(s)
            .with_context(|| format!("--before {s} is not a valid RFC 3339 / ISO 8601 timestamp"))?;
    }
    let handle = paths.handle();
    let stats = handle.compact(keep_latest, before).context("compact")?;
    Ok(stats)
}

/// Per-asset record produced by `cmd_register_pack`. One entry per asset
/// the mod ships, with the resolved `AssetId` and a brief description so
/// callers can render a summary or persist a mod manifest.
#[derive(Debug, Clone, Serialize)]
pub struct RegisteredAsset {
    pub canonical_name: String,
    pub category: String,
    pub tier: String,
    pub asset_id: String,
    pub relative_path: PathBuf,
}

/// Arguments parsed by `cf-mod ledger register-pack`. Walks a mod
/// directory, auto-registers every asset under the listed roots as a
/// ledger entry with `category = Mod_Custom` and
/// `package_source = mod:<mod_id>`. Mirrors the M4A Gherkin "Mod pack
/// integration" scenario.
#[derive(Debug, Clone)]
pub struct RegisterPackArgs {
    pub pkg_dir: PathBuf,
    pub mod_id: String,
    pub tier: String,
    pub pipeline: String,
    pub asset_roots: Vec<PathBuf>,
    pub license: Option<String>,
    pub freeze: bool,
    pub manifest_out: Option<PathBuf>,
}

pub fn cmd_register_pack(paths: &LedgerPaths, args: &RegisterPackArgs) -> Result<Vec<RegisteredAsset>> {
    if !args.pkg_dir.exists() {
        bail!("mod package directory does not exist: {}", args.pkg_dir.display());
    }
    let mod_id = args.mod_id.trim();
    if mod_id.is_empty() {
        bail!("--mod-id must be non-empty");
    }
    let tier = ProductionTier::parse(&args.tier).ok_or_else(|| anyhow!("unknown tier: {}", args.tier))?;
    let roots: Vec<PathBuf> = if args.asset_roots.is_empty() {
        vec![PathBuf::from("assets")]
    } else {
        args.asset_roots.clone()
    };
    let mut registered: Vec<RegisteredAsset> = Vec::new();
    for root in &roots {
        let scan = args.pkg_dir.join(root);
        if !scan.exists() {
            continue;
        }
        walk_pack_dir(&scan, &mut |abs_path| {
            let rel = abs_path.strip_prefix(&args.pkg_dir).unwrap_or(abs_path).to_path_buf();
            let canonical_name = canonical_name_for(&rel, mod_id);
            let add_args = AddArgs {
                category: "Mod_Custom".to_string(),
                kind: rel.extension().and_then(|s| s.to_str()).unwrap_or("bin").to_string(),
                canonical_name: canonical_name.clone(),
                tier: args.tier.clone(),
                pipeline: args.pipeline.clone(),
                prompt: format!("mod-supplied asset {}", rel.display()),
                negative_prompt: None,
                seed: 0,
                output_path: rel.clone(),
                generator_tool: None,
                generator_model: None,
                generator_workflow: None,
                generator_model_version: None,
                palette: None,
                style_lora: None,
                upstream: Vec::new(),
                package_source: Some(format!("mod:{mod_id}")),
                license: args.license.clone(),
                generated_by_human: false,
                human_edit_notes: None,
                regen_command: None,
                freeze: args.freeze,
                generated_at_iso: None,
                generated_on_machine: None,
            };
            let pkg_paths = LedgerPaths {
                ledger_path: paths.ledger_path.clone(),
                base_dir: args.pkg_dir.clone(),
            };
            let entry = cmd_add(&pkg_paths, &add_args).with_context(|| format!("register asset {}", rel.display()))?;
            registered.push(RegisteredAsset {
                canonical_name,
                category: entry.category.as_str().to_string(),
                tier: entry.tier.as_str().to_string(),
                asset_id: entry.id.as_str().to_string(),
                relative_path: rel,
            });
            Ok(())
        })?;
    }
    if let Some(manifest_path) = &args.manifest_out {
        let manifest = json!({
            "schema_version": 1,
            "mod_id": mod_id,
            "tier": tier.as_str(),
            "pipeline": args.pipeline,
            "license": args.license,
            "assets": registered.iter().map(|r| json!({
                "canonical_name": r.canonical_name,
                "category": r.category,
                "tier": r.tier,
                "asset_id": r.asset_id,
                "relative_path": r.relative_path.display().to_string(),
            })).collect::<Vec<_>>(),
        });
        if let Some(parent) = manifest_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(
            manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap_or_default(),
        )
        .with_context(|| format!("write mod manifest {}", manifest_path.display()))?;
    }
    Ok(registered)
}

fn walk_pack_dir(dir: &Path, callback: &mut dyn FnMut(&Path) -> Result<()>) -> Result<()> {
    let entries = std::fs::read_dir(dir).with_context(|| format!("read_dir {}", dir.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if file_name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            walk_pack_dir(&path, callback)?;
            continue;
        }
        // Skip the freeze backups so we don't re-register them.
        if file_name.ends_with(".frozen") {
            continue;
        }
        if std::path::Path::new(file_name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("bak"))
        {
            continue;
        }
        callback(&path)?;
    }
    Ok(())
}

fn canonical_name_for(rel: &Path, mod_id: &str) -> String {
    let mut out = String::with_capacity(mod_id.len() + 1 + rel.as_os_str().len());
    out.push_str(mod_id);
    out.push('/');
    for component in rel.components() {
        if let Some(s) = component.as_os_str().to_str() {
            out.push_str(s);
            out.push('/');
        }
    }
    if out.ends_with('/') {
        out.pop();
    }
    out
}

/// Render a `LedgerSummary` to the spec's stdout format.
///
/// Per M4A spec example, ALWAYS emit `Missing:` and `Drifted:` lines (with
/// `[]` when empty) so log scrapers don't need a present-or-absent branch.
/// Ordering follows the spec example: `Missing`, `Drifted`, `Failed`,
/// `Stale`.
pub fn render_summary(summary: &LedgerSummary, mut out: impl Write) -> std::io::Result<()> {
    writeln!(out, "Total entries: {}", summary.total_entries)?;
    writeln!(out, "Live entries:  {}", summary.live_entries)?;
    writeln!(out, "Superseded:    {}", summary.superseded_entries)?;
    let cat_line = summary
        .by_category
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(", ");
    writeln!(out, "By category: {cat_line}")?;
    let tier_line = summary
        .by_tier
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(", ");
    writeln!(out, "By tier:     {tier_line}")?;
    let status_line = summary
        .by_status
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(", ");
    writeln!(out, "Status:      {status_line}")?;
    // Always emit the per-bucket lists in spec order.
    for bucket in &["Missing", "Drifted", "Failed", "Stale"] {
        let ids = summary.non_fresh.get(*bucket).cloned().unwrap_or_default();
        writeln!(out, "{bucket}: {ids:?}")?;
    }
    Ok(())
}

pub fn render_verify_report(report: &VerifyReport, mut out: impl Write) -> std::io::Result<()> {
    writeln!(
        out,
        "verify total={} fresh={} stale={} drifted={} missing={} failed={}",
        report.total, report.fresh, report.stale, report.drifted, report.missing, report.failed
    )?;
    for r in &report.results {
        if !matches!(r.status, RegenStatus::Fresh) {
            writeln!(
                out,
                "  {} status={} {}",
                r.id.as_str(),
                r.status.as_str(),
                r.note.as_deref().unwrap_or("")
            )?;
        }
    }
    Ok(())
}

pub fn render_regen_attempts(attempts: &[RegenAttempt], mut out: impl Write) -> std::io::Result<()> {
    let ok = attempts.iter().filter(|a| a.ok).count();
    let fail = attempts.len() - ok;
    writeln!(out, "regenerate total={} ok={} fail={}", attempts.len(), ok, fail)?;
    for a in attempts {
        if a.ok {
            writeln!(out, "  OK   {}", a.id.as_str())?;
        } else {
            writeln!(
                out,
                "  FAIL {}: {}",
                a.id.as_str(),
                a.error.as_deref().unwrap_or("unknown")
            )?;
        }
    }
    Ok(())
}

/// Convert `LedgerSummary` into the JSON shape expected by
/// `observe.assets.ledger_summary` (mirrors the M4A spec's spec table:
/// total, by_category, by_tier, by_status, missing[], drifted[]).
pub fn summary_to_observe_json(summary: &LedgerSummary) -> Value {
    let mut by_cat: BTreeMap<String, u64> = BTreeMap::new();
    for (k, v) in &summary.by_category {
        by_cat.insert(k.clone(), *v);
    }
    let mut by_tier: BTreeMap<String, u64> = BTreeMap::new();
    for (k, v) in &summary.by_tier {
        by_tier.insert(k.clone(), *v);
    }
    let mut by_status: BTreeMap<String, u64> = BTreeMap::new();
    for (k, v) in &summary.by_status {
        by_status.insert(k.clone(), *v);
    }
    let mut by_pipeline: BTreeMap<String, u64> = BTreeMap::new();
    for (k, v) in &summary.by_pipeline {
        by_pipeline.insert(k.clone(), *v);
    }
    json!({
        "schema_version": 1,
        "total_entries": summary.total_entries,
        "live_entries": summary.live_entries,
        "superseded_entries": summary.superseded_entries,
        "by_category": by_cat,
        "by_tier": by_tier,
        "by_status": by_status,
        "by_pipeline": by_pipeline,
        "missing": summary.non_fresh.get("Missing").cloned().unwrap_or_default(),
        "drifted": summary.non_fresh.get("Drifted").cloned().unwrap_or_default(),
        "failed": summary.non_fresh.get("Failed").cloned().unwrap_or_default(),
        "stale": summary.non_fresh.get("Stale").cloned().unwrap_or_default(),
    })
}

fn resolve_for_io(path: &Path, base_dir: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    static TMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn tmp_workspace() -> PathBuf {
        // PID + atomic counter is enough for uniqueness; SystemTime::now is
        // disallowed by the workspace clippy lint.
        let seq = TMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("cf-asset-ledger-cli-{pid}-{seq}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_asset(workspace: &Path, rel: &str, contents: &[u8]) -> PathBuf {
        let p = workspace.join(rel);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&p, contents).unwrap();
        p
    }

    fn add_args(rel: &str) -> AddArgs {
        AddArgs {
            category: "WeaponSprite".to_string(),
            kind: "weapon-side".to_string(),
            canonical_name: "iron_rifle_m1_side_v1".to_string(),
            tier: "Tier1_SVG".to_string(),
            pipeline: "M9A_svg_v1".to_string(),
            prompt: "industrial rifle, side-profile".to_string(),
            negative_prompt: None,
            seed: 1234,
            output_path: PathBuf::from(rel),
            generator_tool: None,
            generator_model: None,
            generator_workflow: None,
            generator_model_version: None,
            palette: None,
            style_lora: None,
            upstream: Vec::new(),
            package_source: None,
            license: None,
            generated_by_human: false,
            human_edit_notes: None,
            regen_command: None,
            freeze: true,
            generated_at_iso: None,
            generated_on_machine: None,
        }
    }

    #[test]
    fn cli_add_then_list_then_verify() {
        let workspace = tmp_workspace();
        write_asset(&workspace, "content/assets/iron_rifle.svg", b"<svg/>");
        let paths = LedgerPaths {
            ledger_path: workspace.join("ledger.jsonl"),
            base_dir: workspace.clone(),
        };
        let args = add_args("content/assets/iron_rifle.svg");
        let entry = cmd_add(&paths, &args).unwrap();
        assert_eq!(entry.canonical_name, "iron_rifle_m1_side_v1");
        let listed = cmd_list(&paths, &ListFilter::default()).unwrap();
        assert_eq!(listed.len(), 1);
        let report = cmd_verify(&paths, None, true).unwrap();
        assert_eq!(report.total, 1);
        assert_eq!(report.fresh, 1);
        assert!(report.is_strict_ok());
    }

    #[test]
    fn cli_verify_detects_drift() {
        let workspace = tmp_workspace();
        let asset = write_asset(&workspace, "content/assets/x.svg", b"<svg/>");
        let paths = LedgerPaths {
            ledger_path: workspace.join("ledger.jsonl"),
            base_dir: workspace.clone(),
        };
        let _ = cmd_add(&paths, &add_args("content/assets/x.svg")).unwrap();
        // Edit the asset behind the pipeline's back.
        fs::write(&asset, b"corrupted").unwrap();
        let report = cmd_verify(&paths, None, true).unwrap();
        assert_eq!(report.drifted, 1);
        assert!(!report.is_strict_ok());
    }

    #[test]
    fn cli_show_round_trips_by_prefix() {
        let workspace = tmp_workspace();
        write_asset(&workspace, "content/assets/x.svg", b"<svg/>");
        let paths = LedgerPaths {
            ledger_path: workspace.join("ledger.jsonl"),
            base_dir: workspace.clone(),
        };
        let entry = cmd_add(&paths, &add_args("content/assets/x.svg")).unwrap();
        let prefix = &entry.id.as_str()[..16];
        let found = cmd_show(&paths, prefix).unwrap();
        assert_eq!(found.id, entry.id);
    }

    #[test]
    fn cli_show_by_canonical_name() {
        let workspace = tmp_workspace();
        write_asset(&workspace, "content/assets/x.svg", b"<svg/>");
        let paths = LedgerPaths {
            ledger_path: workspace.join("ledger.jsonl"),
            base_dir: workspace.clone(),
        };
        let entry = cmd_add(&paths, &add_args("content/assets/x.svg")).unwrap();
        let found = cmd_show(&paths, &entry.canonical_name).unwrap();
        assert_eq!(found.id, entry.id);
    }

    #[test]
    fn cli_regenerate_byte_identical() {
        let workspace = tmp_workspace();
        let asset = write_asset(&workspace, "content/assets/x.svg", b"<svg/>");
        let paths = LedgerPaths {
            ledger_path: workspace.join("ledger.jsonl"),
            base_dir: workspace.clone(),
        };
        let entry = cmd_add(&paths, &add_args("content/assets/x.svg")).unwrap();
        // Corrupt the file then regen.
        fs::write(&asset, b"corrupted").unwrap();
        let attempts = cmd_regenerate(&paths, Some(entry.id.as_str()), false, None, None, false, false).unwrap();
        assert_eq!(attempts.len(), 1);
        assert!(attempts[0].ok);
        let restored = fs::read_to_string(&asset).unwrap();
        assert_eq!(restored, "<svg/>");
    }

    #[test]
    fn cli_summary_groups_by_category_and_tier() {
        let workspace = tmp_workspace();
        write_asset(&workspace, "content/a.svg", b"a");
        write_asset(&workspace, "content/b.svg", b"b");
        let paths = LedgerPaths {
            ledger_path: workspace.join("ledger.jsonl"),
            base_dir: workspace.clone(),
        };
        let mut a = add_args("content/a.svg");
        a.canonical_name = "a".to_string();
        let mut b = add_args("content/b.svg");
        b.canonical_name = "b".to_string();
        b.category = "UiIcon".to_string();
        cmd_add(&paths, &a).unwrap();
        cmd_add(&paths, &b).unwrap();
        let summary = cmd_summary(&paths).unwrap();
        assert_eq!(summary.total_entries, 2);
        assert_eq!(summary.live_entries, 2);
        assert_eq!(summary.by_category.get("WeaponSprite"), Some(&1));
        assert_eq!(summary.by_category.get("UiIcon"), Some(&1));
    }

    #[test]
    fn observe_summary_json_shape() {
        let workspace = tmp_workspace();
        write_asset(&workspace, "content/x.svg", b"x");
        let paths = LedgerPaths {
            ledger_path: workspace.join("ledger.jsonl"),
            base_dir: workspace.clone(),
        };
        let mut args = add_args("content/x.svg");
        args.canonical_name = "x".to_string();
        cmd_add(&paths, &args).unwrap();
        let summary = cmd_summary(&paths).unwrap();
        let json = summary_to_observe_json(&summary);
        assert_eq!(json.get("schema_version").and_then(|v| v.as_u64()), Some(1));
        assert_eq!(json.get("total_entries").and_then(|v| v.as_u64()), Some(1));
        assert!(json.get("by_category").is_some());
        assert!(json.get("by_tier").is_some());
        assert!(json.get("by_status").is_some());
        assert!(json.get("missing").unwrap().is_array());
        assert!(json.get("drifted").unwrap().is_array());
    }

    #[test]
    fn cli_add_appends_a_fresh_entry_on_re_add() {
        let workspace = tmp_workspace();
        write_asset(&workspace, "content/x.svg", b"version-1");
        let paths = LedgerPaths {
            ledger_path: workspace.join("ledger.jsonl"),
            base_dir: workspace.clone(),
        };
        let mut args = add_args("content/x.svg");
        let v1 = cmd_add(&paths, &args).unwrap();
        // Change contents + re-add (different blake3).
        write_asset(&workspace, "content/x.svg", b"version-2");
        args.seed += 1;
        let v2 = cmd_add(&paths, &args).unwrap();
        assert_eq!(v1.id, v2.id, "same canonical_name + tier -> same id");
        let raw = fs::read_to_string(&paths.ledger_path).unwrap();
        let lines: Vec<&str> = raw.lines().collect();
        assert_eq!(lines.len(), 2);
        let live = paths.handle().live_entries().unwrap();
        // Live set should contain only the newest (the older one is superseded).
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].seed, args.seed);
    }

    #[test]
    fn register_pack_walks_assets_and_registers_each_as_mod_custom() {
        let workspace = tmp_workspace();
        let pkg = workspace.join("my_mod");
        std::fs::create_dir_all(pkg.join("assets/icons")).unwrap();
        std::fs::create_dir_all(pkg.join("assets/sounds")).unwrap();
        std::fs::write(pkg.join("assets/icons/skull.svg"), b"<svg>skull</svg>").unwrap();
        std::fs::write(pkg.join("assets/icons/flame.svg"), b"<svg>flame</svg>").unwrap();
        std::fs::write(pkg.join("assets/sounds/scream.ogg"), b"oggdata").unwrap();
        let paths = LedgerPaths {
            ledger_path: workspace.join("ledger.jsonl"),
            base_dir: workspace.clone(),
        };
        let manifest_out = workspace.join("mod_manifest.json");
        let args = RegisterPackArgs {
            pkg_dir: pkg.clone(),
            mod_id: "necropolis_dlc".to_string(),
            tier: "Mod_Supplied".to_string(),
            pipeline: "Mod_Supplied_v1".to_string(),
            asset_roots: vec![PathBuf::from("assets")],
            license: Some("mod:cc-by".to_string()),
            freeze: true,
            manifest_out: Some(manifest_out.clone()),
        };
        let registered = cmd_register_pack(&paths, &args).expect("register-pack ok");
        assert_eq!(registered.len(), 3, "expected 3 mod assets registered");
        for r in &registered {
            assert_eq!(r.category, "Mod_Custom", "category must be Mod_Custom");
            assert_eq!(r.tier, "Mod_Supplied");
            assert!(r.canonical_name.starts_with("necropolis_dlc/"));
        }
        // Ledger file has 3 entries, all Mod_Custom + mod:necropolis_dlc
        let ledger = paths.handle().read_all().unwrap();
        assert_eq!(ledger.len(), 3);
        for entry in &ledger {
            assert_eq!(entry.category, AssetCategory::ModCustom);
            assert_eq!(entry.package_source.as_label(), "mod:necropolis_dlc");
            assert!(matches!(entry.license, License::ModSupplied(_)));
        }
        // Sidecar manifest references ledger entry ids, NOT raw paths
        let raw = std::fs::read_to_string(&manifest_out).unwrap();
        let manifest: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let assets = manifest.get("assets").and_then(|v| v.as_array()).unwrap();
        assert_eq!(assets.len(), 3);
        for asset in assets {
            let id = asset.get("asset_id").and_then(|v| v.as_str()).unwrap();
            assert_eq!(id.len(), 64);
            assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn register_pack_install_round_trip_verifies_blake3() {
        let workspace = tmp_workspace();
        let pkg = workspace.join("install_mod");
        std::fs::create_dir_all(pkg.join("assets")).unwrap();
        std::fs::write(pkg.join("assets/a.svg"), b"<svg>install</svg>").unwrap();
        let paths = LedgerPaths {
            ledger_path: workspace.join("ledger.jsonl"),
            base_dir: workspace.clone(),
        };
        let args = RegisterPackArgs {
            pkg_dir: pkg.clone(),
            mod_id: "install_mod".to_string(),
            tier: "Mod_Supplied".to_string(),
            pipeline: "Mod_Supplied_v1".to_string(),
            asset_roots: vec![],
            license: None,
            freeze: true,
            manifest_out: None,
        };
        let registered = cmd_register_pack(&paths, &args).unwrap();
        assert_eq!(registered.len(), 1);
        // The ledger entry's blake3 must match the on-disk file (verify all).
        let report = cmd_verify(
            &LedgerPaths {
                ledger_path: paths.ledger_path.clone(),
                base_dir: pkg.clone(),
            },
            None,
            true,
        )
        .unwrap();
        assert_eq!(report.fresh, 1);
        assert!(report.is_strict_ok(), "post-install verify must be all-Fresh");
    }

    #[test]
    fn register_pack_rejects_missing_pkg_dir() {
        let workspace = tmp_workspace();
        let paths = LedgerPaths {
            ledger_path: workspace.join("ledger.jsonl"),
            base_dir: workspace.clone(),
        };
        let args = RegisterPackArgs {
            pkg_dir: workspace.join("does_not_exist"),
            mod_id: "x".to_string(),
            tier: "Mod_Supplied".to_string(),
            pipeline: "Mod_Supplied_v1".to_string(),
            asset_roots: vec![],
            license: None,
            freeze: false,
            manifest_out: None,
        };
        let err = cmd_register_pack(&paths, &args).expect_err("missing pkg must fail");
        let msg = err.to_string();
        assert!(msg.contains("does not exist"), "got: {msg}");
    }

    /// **M4A § Gherkin "Per-category + per-tier filtering"** — the
    /// third clause of the scenario asks that `--status Drifted`
    /// filters the list. This test exercises the status filter
    /// end-to-end via cmd_list.
    #[test]
    fn cli_list_filter_status_drifted() {
        let workspace = tmp_workspace();
        write_asset(&workspace, "content/a.svg", b"a");
        write_asset(&workspace, "content/b.svg", b"b");
        let paths = LedgerPaths {
            ledger_path: workspace.join("ledger.jsonl"),
            base_dir: workspace.clone(),
        };
        let mut a = add_args("content/a.svg");
        a.canonical_name = "a".to_string();
        let mut b = add_args("content/b.svg");
        b.canonical_name = "b".to_string();
        cmd_add(&paths, &a).unwrap();
        cmd_add(&paths, &b).unwrap();
        // Drift b's file
        fs::write(workspace.join("content/b.svg"), b"hand-edit").unwrap();
        // verify --all marks b as Drifted (in-memory); use mark_dependents_stale
        // is the wrong primitive — we want the on-disk status to flip. Currently
        // the in-memory status is updated by verify but not persisted unless
        // the caller writes back. So the filter must run against a stored
        // status. Demonstrate by manually rewriting the ledger to mark b's
        // entry as Drifted:
        let handle = paths.handle();
        let mut entries = handle.read_all().unwrap();
        for e in &mut entries {
            if e.canonical_name == "b" {
                e.regen_status = RegenStatus::Drifted;
            }
        }
        // Truncate + rewrite via direct file ops (mimics the future
        // `cf-mod ledger verify --refresh` path).
        std::fs::write(
            &paths.ledger_path,
            entries
                .iter()
                .map(|e| serde_json::to_string(e).unwrap())
                .collect::<Vec<_>>()
                .join("\n")
                + "\n",
        )
        .unwrap();
        let filter = ListFilter {
            status: Some(RegenStatus::Drifted),
            ..ListFilter::default()
        };
        let list = cmd_list(&paths, &filter).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].canonical_name, "b");
        assert_eq!(list[0].regen_status, RegenStatus::Drifted);
    }

    /// **M4A § Gherkin "Ledger size bounded under regen churn"** — the
    /// `--before <date>` cutoff filters by lexical RFC 3339 timestamp.
    /// Validate the path end-to-end via cmd_compact.
    #[test]
    fn cli_compact_before_date_drops_old_entries() {
        let workspace = tmp_workspace();
        let paths = LedgerPaths {
            ledger_path: workspace.join("ledger.jsonl"),
            base_dir: workspace.clone(),
        };
        write_asset(&workspace, "content/x.svg", b"x");
        write_asset(&workspace, "content/y.svg", b"y");
        let handle = paths.handle();
        let old = AssetEntryBuilder::new(
            AssetCategory::WeaponSprite,
            "weapon-side",
            "old_x",
            ProductionTier::Tier1Svg,
            "M9A_svg_v1",
            "p",
            1,
            "content/x.svg",
        )
        .with_output_blake3("a".repeat(64))
        .with_generated_at_iso("2026-01-01T00:00:00Z")
        .build();
        let new = AssetEntryBuilder::new(
            AssetCategory::WeaponSprite,
            "weapon-side",
            "new_y",
            ProductionTier::Tier1Svg,
            "M9A_svg_v1",
            "p",
            2,
            "content/y.svg",
        )
        .with_output_blake3("b".repeat(64))
        .with_generated_at_iso("2026-06-01T00:00:00Z")
        .build();
        handle.append(&old).unwrap();
        handle.append(&new).unwrap();
        // Cutoff: keep only entries with generated_at_iso >= "2026-05-01..."
        let stats = cmd_compact(&paths, true, Some("2026-05-01T00:00:00Z")).unwrap();
        assert_eq!(stats.total_before, 2);
        assert_eq!(stats.total_after, 1);
        let remaining = handle.read_all().unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].canonical_name, "new_y");
    }

    /// **M4A § cmd_compact validation**: malformed `--before` strings
    /// must surface a clear error rather than silently lexically
    /// dropping/keeping entries.
    #[test]
    fn cli_compact_rejects_malformed_before_date() {
        let workspace = tmp_workspace();
        let paths = LedgerPaths {
            ledger_path: workspace.join("ledger.jsonl"),
            base_dir: workspace.clone(),
        };
        write_asset(&workspace, "content/x.svg", b"x");
        let _ = cmd_add(&paths, &add_args("content/x.svg")).unwrap();
        let err = cmd_compact(&paths, true, Some("not-a-date")).expect_err("malformed must fail");
        let msg = err.to_string();
        assert!(msg.contains("not-a-date"), "missing date in error: {msg}");
    }

    #[test]
    fn cli_list_filter_category_and_tier() {
        let workspace = tmp_workspace();
        write_asset(&workspace, "content/a.svg", b"a");
        write_asset(&workspace, "content/b.svg", b"b");
        let paths = LedgerPaths {
            ledger_path: workspace.join("ledger.jsonl"),
            base_dir: workspace.clone(),
        };
        let mut a = add_args("content/a.svg");
        a.canonical_name = "a".to_string();
        let mut b = add_args("content/b.svg");
        b.canonical_name = "b".to_string();
        b.tier = "Tier2_ComfyUI".to_string();
        cmd_add(&paths, &a).unwrap();
        cmd_add(&paths, &b).unwrap();
        let filter = ListFilter {
            category: Some(AssetCategory::WeaponSprite),
            tier: Some(ProductionTier::Tier1Svg),
            ..ListFilter::default()
        };
        let list = cmd_list(&paths, &filter).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].canonical_name, "a");
    }
}
