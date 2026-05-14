use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use cf_control::Scenario;
use clap::{Parser, Subcommand};
use serde_json::json;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "cf-mod", about = "Mod and scenario validator/builder.")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
    #[arg(long, global = true)]
    strict: bool,
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Walk one or more content/mods directories and validate every scenario manifest found.
    Validate {
        /// Files or directories to validate. If empty, defaults to `content/` (then `../content/`).
        paths: Vec<PathBuf>,
    },
    /// **M1 Gap H2**: validate every event in a run-bundle's `events.jsonl`
    /// against the per-event JSON schemas under `cf-replay/schemas/event/`.
    /// Returns non-zero exit on any payload that fails the schema.
    ValidateBundle {
        /// Path to a run-bundle directory (the one containing `events.jsonl`).
        bundle_dir: PathBuf,
    },
    /// Stubbed in M0; package builder lands at M5/M8.
    Build { pkg_dir: PathBuf },
    /// Stubbed in M0.
    Inspect { cfpkg: PathBuf },
    /// **M4A**: asset-ledger CLI. Append / list / verify / regenerate /
    /// summarize entries in `content/asset_ledger/ledger.jsonl`.
    Ledger {
        #[command(subcommand)]
        action: Box<LedgerAction>,
    },
}

/// **M4A** asset-ledger subcommands.
#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
enum LedgerAction {
    /// Append a new entry to the ledger. The output file must already
    /// exist; blake3 is computed at write-time. Re-running with the same
    /// `--canonical-name`+`--tier`+`--category` appends a new line AND
    /// marks the previous entry as `superseded_by` the new one.
    Add {
        #[arg(long)]
        category: String,
        #[arg(long)]
        kind: String,
        #[arg(long = "canonical-name")]
        canonical_name: String,
        #[arg(long)]
        tier: String,
        #[arg(long)]
        pipeline: String,
        #[arg(long)]
        prompt: String,
        #[arg(long = "negative-prompt")]
        negative_prompt: Option<String>,
        #[arg(long)]
        seed: u64,
        #[arg(long = "output-path")]
        output_path: PathBuf,
        #[arg(long = "generator-tool")]
        generator_tool: Option<String>,
        #[arg(long = "generator-model")]
        generator_model: Option<String>,
        #[arg(long = "generator-workflow")]
        generator_workflow: Option<String>,
        #[arg(long = "generator-model-version")]
        generator_model_version: Option<String>,
        /// Palette reference id used by the producing pipeline. Spec
        /// names this `--palette-ref`; both `--palette` (short) and
        /// `--palette-ref` (spec-literal) work.
        #[arg(long, alias = "palette-ref")]
        palette: Option<String>,
        #[arg(long = "style-lora")]
        style_lora: Option<String>,
        #[arg(long)]
        upstream: Vec<String>,
        #[arg(long = "package-source")]
        package_source: Option<String>,
        #[arg(long)]
        license: Option<String>,
        #[arg(long = "generated-by-human")]
        generated_by_human: bool,
        #[arg(long = "human-edit-notes")]
        human_edit_notes: Option<String>,
        #[arg(long = "regen-command")]
        regen_command: Option<String>,
        /// **M4A determinism**: pin the entry's `generated_at_iso` field
        /// instead of using wall-clock time. Combined with the `freeze`
        /// snapshot this makes the ledger byte-reproducible across CI.
        #[arg(long = "generated-at-iso")]
        generated_at_iso: Option<String>,
        /// **M4A determinism**: pin `generated_on_machine`. Defaults to
        /// `HOSTNAME` / `COMPUTERNAME` / `"unknown"` (or `"deterministic"`
        /// when `CF_DETERMINISTIC_LEDGER=1`).
        #[arg(long = "generated-on-machine")]
        generated_on_machine: Option<String>,
        /// Snapshot the canonical output bytes as `<output_path>.frozen`
        /// so future regens can reproduce byte-for-byte (default true so
        /// the deterministic contract holds for non-deterministic pipelines).
        #[arg(long, default_value_t = true)]
        freeze: bool,
        #[arg(long)]
        ledger_path: Option<PathBuf>,
    },
    /// List entries. Use `--category`, `--tier`, `--pipeline`, `--status`
    /// for filtering; `--include-superseded` to walk the full history.
    List {
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        tier: Option<String>,
        #[arg(long)]
        pipeline: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long = "include-superseded")]
        include_superseded: bool,
        #[arg(long)]
        ledger_path: Option<PathBuf>,
    },
    /// Show a single entry by full hex id, by id prefix, or by canonical_name.
    Show {
        id: String,
        #[arg(long)]
        ledger_path: Option<PathBuf>,
    },
    /// Diff ledger metadata vs the actual disk state.
    Diff {
        /// Optional id; omit to diff every live entry.
        id: Option<String>,
        #[arg(long)]
        ledger_path: Option<PathBuf>,
        #[arg(long)]
        all: bool,
    },
    /// Verify integrity (re-hash and compare). With `--strict`, exits
    /// non-zero on any non-Fresh entry.
    Verify {
        id: Option<String>,
        #[arg(long)]
        all: bool,
        #[arg(long = "strict-status")]
        strict_status: bool,
        #[arg(long)]
        ledger_path: Option<PathBuf>,
    },
    /// Re-bake one or more entries. Uses the freeze-then-store path by
    /// default; pipelines may register their own deterministic runner.
    Regenerate {
        id: Option<String>,
        #[arg(long)]
        cascade: bool,
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        tier: Option<String>,
        #[arg(long)]
        all: bool,
        #[arg(long = "continue-on-error")]
        continue_on_error: bool,
        #[arg(long)]
        ledger_path: Option<PathBuf>,
    },
    /// Aggregate summary (counts by category / tier / status).
    Summary {
        #[arg(long)]
        ledger_path: Option<PathBuf>,
    },
    /// Compact the ledger: drop superseded history.
    Compact {
        #[arg(long, default_value_t = true)]
        keep_latest: bool,
        #[arg(long)]
        before: Option<String>,
        #[arg(long)]
        ledger_path: Option<PathBuf>,
    },
    /// **M4A § "Mod pack integration"**: walk a mod package directory
    /// and auto-register every asset as a new ledger entry with
    /// `category = Mod_Custom` and `package_source = mod:<mod_id>`.
    /// Optionally writes a sidecar mod manifest that references the
    /// generated ledger entry ids (not raw file paths) per the spec.
    RegisterPack {
        /// Mod package directory (must exist; contains `assets/...`).
        pkg_dir: PathBuf,
        /// Stable mod identifier; used for `package_source = mod:<id>`
        /// and as the canonical_name prefix.
        #[arg(long = "mod-id")]
        mod_id: String,
        /// Production tier for the mod's assets. Default
        /// `Mod_Supplied` per spec; pipelines that want stricter tiers
        /// can override (e.g. `--tier Tier1_SVG`).
        #[arg(long, default_value = "Mod_Supplied")]
        tier: String,
        /// Pipeline id recorded on every entry. Default
        /// `Mod_Supplied_v1`.
        #[arg(long, default_value = "Mod_Supplied_v1")]
        pipeline: String,
        /// Asset roots inside the mod package directory. Defaults to
        /// `assets`. Repeatable.
        #[arg(long = "asset-root")]
        asset_roots: Vec<PathBuf>,
        /// Per-asset license declaration. The author asserts; engine
        /// does NOT verify.
        #[arg(long)]
        license: Option<String>,
        /// Snapshot canonical bytes as `<path>.frozen` so freeze-then-
        /// store regens work for non-deterministic mod content.
        #[arg(long, default_value_t = true)]
        freeze: bool,
        /// Optional path to a sidecar mod manifest file (JSON). When
        /// set, the manifest is written referencing ledger entry ids
        /// per the M4A spec.
        #[arg(long = "manifest-out")]
        manifest_out: Option<PathBuf>,
        /// Override the global canonical ledger path (defaults to
        /// `<workspace>/content/asset_ledger/ledger.jsonl`).
        #[arg(long)]
        ledger_path: Option<PathBuf>,
    },
}

fn init_diagnostics() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,cf_=debug")))
        .with_target(true)
        .init();
    let prev_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(target: "cf::mod", panic = %info, "system.panic");
        prev_hook(info);
    }));
}

fn main() -> Result<()> {
    init_diagnostics();
    let cli = Cli::parse();
    match &cli.command {
        Cmd::Validate { paths } => run_validate(paths, cli.strict, cli.json),
        Cmd::ValidateBundle { bundle_dir } => run_validate_bundle(bundle_dir, cli.json),
        Cmd::Build { pkg_dir } => {
            anyhow::bail!(
                "cf-mod build is not implemented in M0; package builder lands at M5/M8 (got {})",
                pkg_dir.display()
            );
        }
        Cmd::Inspect { cfpkg } => {
            anyhow::bail!(
                "cf-mod inspect is not implemented in M0; package format lands at M8 (got {})",
                cfpkg.display()
            );
        }
        Cmd::Ledger { action } => run_ledger(action.as_ref(), cli.strict, cli.json),
    }
}

fn ledger_paths(override_path: Option<&PathBuf>) -> cf_asset_ledger::LedgerPaths {
    let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut paths = cf_asset_ledger::LedgerPaths::default_for(&workspace_root);
    if let Some(p) = override_path {
        paths.ledger_path = p.clone();
    }
    paths
}

#[allow(clippy::too_many_lines)]
fn run_ledger(action: &LedgerAction, global_strict: bool, json_output: bool) -> Result<()> {
    use cf_asset_ledger::{
        cmd_add, cmd_compact, cmd_diff, cmd_list, cmd_regenerate, cmd_show, cmd_summary, cmd_verify,
        render_regen_attempts, render_summary, render_verify_report, AddArgs, AssetCategory, ListFilter,
        ProductionTier, RegenStatus,
    };
    match action {
        LedgerAction::Add {
            category,
            kind,
            canonical_name,
            tier,
            pipeline,
            prompt,
            negative_prompt,
            seed,
            output_path,
            generator_tool,
            generator_model,
            generator_workflow,
            generator_model_version,
            palette,
            style_lora,
            upstream,
            package_source,
            license,
            generated_by_human,
            human_edit_notes,
            regen_command,
            generated_at_iso,
            generated_on_machine,
            freeze,
            ledger_path,
        } => {
            let paths = ledger_paths(ledger_path.as_ref());
            let args = AddArgs {
                category: category.clone(),
                kind: kind.clone(),
                canonical_name: canonical_name.clone(),
                tier: tier.clone(),
                pipeline: pipeline.clone(),
                prompt: prompt.clone(),
                negative_prompt: negative_prompt.clone(),
                seed: *seed,
                output_path: output_path.clone(),
                generator_tool: generator_tool.clone(),
                generator_model: generator_model.clone(),
                generator_workflow: generator_workflow.clone(),
                generator_model_version: generator_model_version.clone(),
                palette: palette.clone(),
                style_lora: style_lora.clone(),
                upstream: upstream.clone(),
                package_source: package_source.clone(),
                license: license.clone(),
                generated_by_human: *generated_by_human,
                human_edit_notes: human_edit_notes.clone(),
                regen_command: regen_command.clone(),
                freeze: *freeze,
                generated_at_iso: generated_at_iso.clone(),
                generated_on_machine: generated_on_machine.clone(),
            };
            let entry = cmd_add(&paths, &args).context("ledger add")?;
            if json_output {
                println!("{}", serde_json::to_string_pretty(&entry).unwrap_or_default());
            } else {
                println!(
                    "ledger add OK: id={} canonical_name={} category={} tier={} pipeline={}",
                    entry.id.as_str(),
                    entry.canonical_name,
                    entry.category.as_str(),
                    entry.tier.as_str(),
                    entry.pipeline
                );
            }
            Ok(())
        }
        LedgerAction::List {
            category,
            tier,
            pipeline,
            status,
            include_superseded,
            ledger_path,
        } => {
            let paths = ledger_paths(ledger_path.as_ref());
            let filter = ListFilter {
                category: category.as_deref().and_then(AssetCategory::parse),
                tier: tier.as_deref().and_then(ProductionTier::parse),
                pipeline: pipeline.clone(),
                status: status.as_deref().and_then(RegenStatus::parse),
                package_source_label: None,
                include_superseded: *include_superseded,
            };
            let entries = cmd_list(&paths, &filter).context("ledger list")?;
            if json_output {
                println!("{}", serde_json::to_string_pretty(&entries).unwrap_or_default());
            } else {
                for e in &entries {
                    println!(
                        "{}  {}  {}  {}  status={}",
                        e.id.as_str(),
                        e.category.as_str(),
                        e.tier.as_str(),
                        e.canonical_name,
                        e.regen_status.as_str()
                    );
                }
            }
            Ok(())
        }
        LedgerAction::Show { id, ledger_path } => {
            let paths = ledger_paths(ledger_path.as_ref());
            let entry = cmd_show(&paths, id).context("ledger show")?;
            // Show always pretty-prints — JSON is the only viable readable
            // representation of an AssetEntry; the `--json` global is a
            // no-op here but kept for consistency with the other verbs.
            let _ = json_output;
            println!("{}", serde_json::to_string_pretty(&entry).unwrap_or_default());
            Ok(())
        }
        LedgerAction::Diff { id, ledger_path, all } => {
            let paths = ledger_paths(ledger_path.as_ref());
            let target = if *all { None } else { id.as_deref() };
            let results = cmd_diff(&paths, target).context("ledger diff")?;
            let drifted = results
                .iter()
                .filter(|r| matches!(r.status, cf_asset_ledger::RegenStatus::Drifted))
                .count();
            let missing = results
                .iter()
                .filter(|r| matches!(r.status, cf_asset_ledger::RegenStatus::Missing))
                .count();
            if json_output {
                println!("{}", serde_json::to_string_pretty(&results).unwrap_or_default());
            } else {
                for r in &results {
                    println!(
                        "{}  status={}  observed_blake3={}  observed_size={}",
                        r.id.as_str(),
                        r.status.as_str(),
                        r.observed_blake3,
                        r.observed_size_bytes
                    );
                }
            }
            if drifted > 0 || missing > 0 {
                std::process::exit(1);
            }
            Ok(())
        }
        LedgerAction::Verify {
            id,
            all,
            strict_status,
            ledger_path,
        } => {
            let paths = ledger_paths(ledger_path.as_ref());
            let target = if *all { None } else { id.as_deref() };
            // **M4A spec literal**: `cf-mod ledger verify --strict` (global flag)
            // is the CI-gate form. `--strict-status` is the local alias. Either
            // one engages strict mode; both reduce to `strict = true`.
            let strict = *strict_status || global_strict;
            let report = cmd_verify(&paths, target, strict).context("ledger verify")?;
            if json_output {
                println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
            } else {
                render_verify_report(&report, std::io::stdout()).ok();
            }
            if strict && !report.is_strict_ok() {
                std::process::exit(1);
            }
            Ok(())
        }
        LedgerAction::Regenerate {
            id,
            cascade,
            category,
            tier,
            all,
            continue_on_error,
            ledger_path,
        } => {
            let paths = ledger_paths(ledger_path.as_ref());
            let cat = category.as_deref().and_then(AssetCategory::parse);
            let t = tier.as_deref().and_then(ProductionTier::parse);
            let attempts = cmd_regenerate(&paths, id.as_deref(), *cascade, cat, t, *all, *continue_on_error)
                .context("ledger regenerate")?;
            let failed = attempts.iter().filter(|a| !a.ok).count();
            if json_output {
                let json = serde_json::json!({
                    "total": attempts.len(),
                    "ok": attempts.iter().filter(|a| a.ok).count(),
                    "failed": failed,
                    "attempts": attempts.iter().map(|a| serde_json::json!({
                        "id": a.id.as_str(),
                        "ok": a.ok,
                        "error": a.error,
                    })).collect::<Vec<_>>()
                });
                println!("{}", serde_json::to_string_pretty(&json).unwrap_or_default());
            } else {
                render_regen_attempts(&attempts, std::io::stdout()).ok();
            }
            if failed > 0 {
                std::process::exit(1);
            }
            Ok(())
        }
        LedgerAction::Summary { ledger_path } => {
            let paths = ledger_paths(ledger_path.as_ref());
            let summary = cmd_summary(&paths).context("ledger summary")?;
            if json_output {
                let json = cf_asset_ledger::summary_to_observe_json(&summary);
                println!("{}", serde_json::to_string_pretty(&json).unwrap_or_default());
            } else {
                render_summary(&summary, std::io::stdout()).ok();
            }
            Ok(())
        }
        LedgerAction::Compact {
            keep_latest,
            before,
            ledger_path,
        } => {
            let paths = ledger_paths(ledger_path.as_ref());
            let stats = cmd_compact(&paths, *keep_latest, before.as_deref()).context("ledger compact")?;
            if json_output {
                let json = serde_json::json!({
                    "total_before": stats.total_before,
                    "total_after": stats.total_after,
                    "backup_path": stats.backup_path.display().to_string(),
                });
                println!("{}", serde_json::to_string_pretty(&json).unwrap_or_default());
            } else {
                println!(
                    "compact: before={} after={} backup={}",
                    stats.total_before,
                    stats.total_after,
                    stats.backup_path.display()
                );
            }
            Ok(())
        }
        LedgerAction::RegisterPack {
            pkg_dir,
            mod_id,
            tier,
            pipeline,
            asset_roots,
            license,
            freeze,
            manifest_out,
            ledger_path,
        } => {
            let paths = ledger_paths(ledger_path.as_ref());
            let register_args = cf_asset_ledger::RegisterPackArgs {
                pkg_dir: pkg_dir.clone(),
                mod_id: mod_id.clone(),
                tier: tier.clone(),
                pipeline: pipeline.clone(),
                asset_roots: asset_roots.clone(),
                license: license.clone(),
                freeze: *freeze,
                manifest_out: manifest_out.clone(),
            };
            let registered =
                cf_asset_ledger::cmd_register_pack(&paths, &register_args).context("ledger register-pack")?;
            if json_output {
                let json = serde_json::json!({
                    "mod_id": mod_id,
                    "total_registered": registered.len(),
                    "assets": registered,
                });
                println!("{}", serde_json::to_string_pretty(&json).unwrap_or_default());
            } else {
                println!(
                    "register-pack: mod_id={} tier={} pipeline={} registered={}",
                    mod_id,
                    tier,
                    pipeline,
                    registered.len()
                );
                for asset in &registered {
                    println!(
                        "  {} → {} (category={}, tier={})",
                        asset.relative_path.display(),
                        asset.asset_id,
                        asset.category,
                        asset.tier
                    );
                }
            }
            Ok(())
        }
    }
}

/// **M1 Gap H2**: validate every event in `<bundle_dir>/events.jsonl` against
/// `cf_replay::schemas::validate_event_payload`. Returns non-zero exit on
/// any schema violation. Outputs JSON when `--json` is set so CI can parse
/// the report.
fn run_validate_bundle(bundle_dir: &std::path::Path, json_output: bool) -> Result<()> {
    use std::io::BufRead;
    let events_path = bundle_dir.join("events.jsonl");
    let file =
        std::fs::File::open(&events_path).map_err(|e| anyhow::anyhow!("open {}: {}", events_path.display(), e))?;
    let reader = std::io::BufReader::new(file);
    let mut failures: Vec<serde_json::Value> = Vec::new();
    let mut checked: u64 = 0;
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        if line.trim().is_empty() {
            continue;
        }
        let event: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                failures.push(serde_json::json!({
                    "category": "parse",
                    "event_type": "parse",
                    "reason": e.to_string(),
                }));
                continue;
            }
        };
        let cat = event.get("category").and_then(|v| v.as_str()).unwrap_or("");
        let ty = event.get("event_type").and_then(|v| v.as_str()).unwrap_or("");
        let payload = event.get("payload").cloned().unwrap_or(serde_json::Value::Null);
        checked += 1;
        if let Err(reason) = cf_replay::schemas::validate_event_payload(cat, ty, &payload) {
            let event_id = event.get("event_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            failures.push(serde_json::json!({
                "event_id": event_id,
                "category": cat,
                "event_type": ty,
                "reason": reason,
            }));
        }
    }
    let report = serde_json::json!({
        "bundle_dir": bundle_dir.display().to_string(),
        "events_checked": checked,
        "failures": failures,
    });
    if json_output {
        println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
    } else {
        tracing::info!(
            target: "cf::mod",
            checked,
            failures = failures.len(),
            "validate-bundle complete"
        );
        for f in &failures {
            tracing::warn!(target: "cf::mod", failure = %f, "schema violation");
        }
    }
    if !failures.is_empty() {
        anyhow::bail!(
            "{} event(s) failed schema validation in {}",
            failures.len(),
            bundle_dir.display()
        );
    }
    Ok(())
}

fn run_validate(paths: &[PathBuf], strict: bool, json_output: bool) -> Result<()> {
    let resolved = if paths.is_empty() {
        let mut v = Vec::new();
        for p in ["content", "../content"] {
            let pb = PathBuf::from(p);
            if pb.exists() {
                v.push(pb);
                break;
            }
        }
        if v.is_empty() {
            anyhow::bail!("no paths supplied and could not locate `content/` from cwd");
        }
        v
    } else {
        paths.to_vec()
    };

    let mut report = ValidationReport::default();
    for root in &resolved {
        if !root.exists() {
            report.add_error(root.clone(), "path does not exist".to_string());
            continue;
        }
        if root.is_file() {
            validate_one(root, &mut report);
        } else {
            walk(root, &mut report);
        }
    }

    if json_output {
        println!("{}", serde_json::to_string_pretty(&report.to_json()).unwrap());
    } else {
        for entry in &report.entries {
            match entry.result {
                EntryResult::Pass => println!("PASS {}", entry.path.display()),
                EntryResult::Warn => println!("WARN {} ({})", entry.path.display(), entry.message),
                EntryResult::Fail => println!("FAIL {} ({})", entry.path.display(), entry.message),
            }
        }
        println!("---");
        println!(
            "scanned={} pass={} warn={} fail={}",
            report.entries.len(),
            report.pass(),
            report.warn(),
            report.fail()
        );
    }

    let any_fail = report.fail() > 0 || (strict && report.warn() > 0);
    if any_fail {
        std::process::exit(1);
    }
    Ok(())
}

fn walk(dir: &Path, report: &mut ValidationReport) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(err) => {
            report.add_error(dir.to_path_buf(), format!("read_dir failed: {err}"));
            return;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, report);
        } else if path.extension().and_then(|s| s.to_str()) == Some("ron")
            || (path.extension().and_then(|s| s.to_str()) == Some("json")
                && path.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str()) == Some("ai")
                && path.file_name().and_then(|s| s.to_str()) == Some("difficulty.json"))
            || (path.extension().and_then(|s| s.to_str()) == Some("json")
                && path.components().any(|c| c.as_os_str() == "materials"))
            || path.file_name().and_then(|s| s.to_str()) == Some("ledger.jsonl")
        {
            validate_one(&path, report);
        }
    }
}

/// BP4 + BP5 content surfaces. Paths under any of these directories must FAIL
/// validation (not WARN) because the content owners will start landing real
/// manifests as the listed milestones ship, and a silent WARN would let
/// half-broken or schema-drifted manifests sneak into run-bundle evidence.
/// (path-component, owning_milestone).
///
/// **M2 update**: `materials/` is now validated by `validate_material_json`
/// (cf-material loader) since M2 ships the v1 schema. Other BP4+ content
/// types remain strict-fail until their milestones land.
const STRICT_FAIL_CONTENT_CATEGORIES: &[(&str, &str)] = &[
    ("chassis", "M5"),
    ("atmospheres", "M5.9 / M7.5"),
    ("worlds", "M5.10"),
    ("origins", "M5"),
];

fn validate_one(path: &Path, report: &mut ValidationReport) {
    // **M4A**: validate ledger.jsonl entries against the v1 AssetEntry schema.
    if path.file_name().and_then(|s| s.to_str()) == Some("ledger.jsonl") {
        validate_ledger_jsonl(path, report);
        return;
    }
    if path.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str()) == Some("scenarios")
        || path
            .components()
            .any(|c| c.as_os_str().to_string_lossy().contains("scenarios"))
    {
        validate_scenario(path, report);
        return;
    }
    if path.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str()) == Some("ai")
        && path.file_name().and_then(|s| s.to_str()) == Some("difficulty.json")
    {
        validate_difficulty_json(path, report);
        return;
    }
    // **M2**: material registry files under `content/materials/*.json`.
    let path_components: Vec<String> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    let is_material_file =
        path_components.iter().any(|c| c == "materials") && path.extension().and_then(|s| s.to_str()) == Some("json");
    if is_material_file {
        validate_material_registry(path, report);
        return;
    }
    if let Some((category, milestone)) = STRICT_FAIL_CONTENT_CATEGORIES
        .iter()
        .find(|(cat, _)| path_components.iter().any(|c| c == cat))
    {
        report.add_error(
            path.to_path_buf(),
            format!(
                "cf-mod validator does not yet support {category}/* content — owning milestone is {milestone}. \
                 Until that lands, content/{category}/ files cannot be validated. Move them out or remove them."
            ),
        );
        return;
    }
    report.add_warn(
        path.to_path_buf(),
        "no validator wired for this content type yet (M0 only validates content/scenarios/*.ron)".to_string(),
    );
}

/// **M2**: validate a material registry JSON file (`content/materials/*.json`)
/// against the v1 schema. Aggregates `RegistryValidationError`s from the
/// cf-material loader and reports each as a FAIL entry with the structured
/// `kind` so cfctl + CI can pattern-match on `unknown_field`, `duplicate_id`,
/// `schema_version_mismatch`, `missing_required_field`, etc.
fn validate_material_registry(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let value: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("json parse failed: {err}"));
            return;
        }
    };
    let result = cf_material::validate_registry_json(&value);
    if result.errors.is_empty() {
        let summary = format!(
            "materials registry ({} materials; {} warning(s))",
            result.material_count,
            result.warnings.len()
        );
        report.add_pass(path.to_path_buf(), summary);
    } else {
        let messages: Vec<String> = result
            .errors
            .iter()
            .map(|e| format!("{} @ {}: {} [{}]", e.kind, e.path, e.message, e.hint))
            .collect();
        report.add_error(path.to_path_buf(), messages.join("; "));
    }
    for w in &result.warnings {
        let msg = format!("{} @ {}: {} [{}]", w.kind, w.path, w.message, w.hint);
        report.add_warn(path.to_path_buf(), msg);
    }
}

/// **M1.5**: cf-mod validator for `content/ai/difficulty.json`. Confirms the
/// schema version, the three required preset ids (cakewalk, tough_crowd,
/// veteran), and that every preset carries the required AI tuning fields.
/// Acceptance criterion in `specs/done/M1.5.md` under "Validation + scenario
/// manifest > cf-mod validates difficulty.json".
fn validate_difficulty_json(path: &Path, report: &mut ValidationReport) {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let value: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("json parse failed: {err}"));
            return;
        }
    };
    let mut messages: Vec<String> = Vec::new();
    match value.get("schema").and_then(|v| v.as_u64()) {
        Some(1) => {}
        Some(other) => messages.push(format!("difficulty.schema must be 1 (got {other})")),
        None => messages.push("difficulty.schema missing or not an integer".to_string()),
    }
    let presets = match value.get("presets").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => {
            messages.push("difficulty.presets missing or not an array".to_string());
            report.add_error(path.to_path_buf(), messages.join("; "));
            return;
        }
    };
    const REQUIRED_IDS: &[&str] = &["cakewalk", "tough_crowd", "veteran"];
    const REQUIRED_FIELDS: &[&str] = &[
        "hp",
        "aim_settle_ticks",
        "miss_chance",
        "sight_range",
        "hearing_radius",
        "memory_decay_ticks",
        "reload_ms",
    ];
    let mut found_ids: Vec<String> = Vec::new();
    for (i, preset) in presets.iter().enumerate() {
        let id = preset.get("id").and_then(|v| v.as_str()).unwrap_or("");
        if id.is_empty() {
            messages.push(format!("presets[{i}].id missing or not a string"));
            continue;
        }
        found_ids.push(id.to_string());
        for field in REQUIRED_FIELDS {
            if preset.get(*field).is_none() {
                messages.push(format!("presets[{i}={id}].{field} missing"));
            }
        }
    }
    for required in REQUIRED_IDS {
        if !found_ids.iter().any(|id| id == required) {
            messages.push(format!("required preset id `{required}` missing"));
        }
    }
    if messages.is_empty() {
        report.add_pass(
            path.to_path_buf(),
            format!("difficulty.json ({} presets)", presets.len()),
        );
    } else {
        report.add_error(path.to_path_buf(), messages.join("; "));
    }
}

/// **M4A**: validate every JSONL line in a ledger file against the locked
/// v1 AssetEntry schema. Each line that fails surfaces as a FAIL with the
/// per-line reason; lines that recompute their AssetId mismatch
/// `id_drift` reason for CI to pattern-match.
fn validate_ledger_jsonl(path: &Path, report: &mut ValidationReport) {
    use std::io::BufRead;
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("open failed: {err}"));
            return;
        }
    };
    let reader = std::io::BufReader::new(file);
    let mut total = 0u64;
    let mut failures: Vec<String> = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(l) => l,
            Err(err) => {
                failures.push(format!("line {} read error: {err}", i + 1));
                continue;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(err) => {
                failures.push(format!("line {} json parse: {err}", i + 1));
                continue;
            }
        };
        total += 1;
        if let Err(err) = cf_asset_ledger::validate_entry_json(&value) {
            failures.push(format!("line {} schema reject: {err}", i + 1));
        }
    }
    if failures.is_empty() {
        report.add_pass(path.to_path_buf(), format!("ledger ({total} entries)"));
    } else {
        report.add_error(path.to_path_buf(), failures.join("; "));
    }
}

fn validate_scenario(path: &Path, report: &mut ValidationReport) {
    match Scenario::load_from_file(path) {
        Ok(scenario) => {
            let mut messages: Vec<String> = Vec::new();
            if scenario.schema_version != 1 {
                messages.push(format!(
                    "scenario.schema_version must be 1 (got {})",
                    scenario.schema_version
                ));
            }
            if scenario.id.is_empty() {
                messages.push("scenario.id must be non-empty".to_string());
            }
            if scenario.display_name.trim().is_empty() {
                messages.push("scenario.display_name must be non-empty".to_string());
            }
            // The roadmap requires an expected_tests entry per scenario for M0 evidence.
            if scenario.expected_tests.is_empty() {
                messages.push("scenario.expected_tests must reference at least one acceptance test id".to_string());
            }
            if !messages.is_empty() {
                report.add_error(path.to_path_buf(), messages.join("; "));
            } else {
                report.add_pass(path.to_path_buf(), format!("scenario {}", scenario.id));
            }
        }
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("scenario load failed: {err}"));
        }
    }
}

#[derive(Default)]
struct ValidationReport {
    entries: Vec<Entry>,
}

#[derive(Debug)]
struct Entry {
    path: PathBuf,
    result: EntryResult,
    message: String,
}

#[derive(Debug)]
enum EntryResult {
    Pass,
    Warn,
    Fail,
}

impl ValidationReport {
    fn add_pass(&mut self, path: PathBuf, message: String) {
        self.entries.push(Entry {
            path,
            result: EntryResult::Pass,
            message,
        });
    }
    fn add_warn(&mut self, path: PathBuf, message: String) {
        self.entries.push(Entry {
            path,
            result: EntryResult::Warn,
            message,
        });
    }
    fn add_error(&mut self, path: PathBuf, message: String) {
        self.entries.push(Entry {
            path,
            result: EntryResult::Fail,
            message,
        });
    }
    fn pass(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| matches!(e.result, EntryResult::Pass))
            .count()
    }
    fn warn(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| matches!(e.result, EntryResult::Warn))
            .count()
    }
    fn fail(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| matches!(e.result, EntryResult::Fail))
            .count()
    }
    fn to_json(&self) -> serde_json::Value {
        json!({
            "schema_version": 1,
            "scanned": self.entries.len(),
            "pass": self.pass(),
            "warn": self.warn(),
            "fail": self.fail(),
            "entries": self
                .entries
                .iter()
                .map(|e| {
                    json!({
                        "path": e.path.display().to_string(),
                        "result": match e.result {
                            EntryResult::Pass => "pass",
                            EntryResult::Warn => "warn",
                            EntryResult::Fail => "fail",
                        },
                        "message": e.message,
                    })
                })
                .collect::<Vec<_>>()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    static TMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn write_tmp(name: &str, contents: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let seq = TMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("cf-mod-test-{pid}-{nanos}-{seq}"));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(contents.as_bytes()).unwrap();
        path
    }

    #[test]
    fn difficulty_json_accepts_three_required_presets() {
        let body = serde_json::json!({
            "schema": 1,
            "presets": [
                {"id": "cakewalk", "display_name": "Cakewalk", "hp": 60, "aim_settle_ticks": 24, "miss_chance": 0.3, "sight_range": 240, "sight_fov_degrees": 90, "hearing_radius": 320, "memory_decay_ticks": 180, "reload_ms": 2400, "retreat_hp_pct": 0.5},
                {"id": "tough_crowd", "display_name": "Tough Crowd", "hp": 80, "aim_settle_ticks": 12, "miss_chance": 0.1, "sight_range": 320, "sight_fov_degrees": 120, "hearing_radius": 480, "memory_decay_ticks": 300, "reload_ms": 1800, "retreat_hp_pct": 0.3},
                {"id": "veteran", "display_name": "Veteran", "hp": 120, "aim_settle_ticks": 6, "miss_chance": 0.05, "sight_range": 480, "sight_fov_degrees": 140, "hearing_radius": 600, "memory_decay_ticks": 600, "reload_ms": 1200, "retreat_hp_pct": 0.2}
            ]
        });
        let path = write_tmp("difficulty_pass.json", &body.to_string());
        let mut report = ValidationReport::default();
        validate_difficulty_json(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.pass(), 1, "expected one PASS entry");
        assert_eq!(report.fail(), 0, "expected zero FAIL entries");
    }

    #[test]
    fn difficulty_json_rejects_missing_preset() {
        let body = serde_json::json!({
            "schema": 1,
            "presets": [
                {"id": "cakewalk", "display_name": "Cakewalk", "hp": 60, "aim_settle_ticks": 24, "miss_chance": 0.3, "sight_range": 240, "sight_fov_degrees": 90, "hearing_radius": 320, "memory_decay_ticks": 180, "reload_ms": 2400, "retreat_hp_pct": 0.5}
            ]
        });
        let path = write_tmp("difficulty_missing.json", &body.to_string());
        let mut report = ValidationReport::default();
        validate_difficulty_json(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1, "expected one FAIL entry");
        assert!(report.entries[0].message.contains("tough_crowd"));
        assert!(report.entries[0].message.contains("veteran"));
    }

    #[test]
    fn difficulty_json_rejects_missing_field() {
        let body = serde_json::json!({
            "schema": 1,
            "presets": [
                {"id": "cakewalk", "display_name": "Cakewalk", "hp": 60, "aim_settle_ticks": 24, "miss_chance": 0.3, "sight_range": 240, "sight_fov_degrees": 90, "hearing_radius": 320, "memory_decay_ticks": 180, "retreat_hp_pct": 0.5},
                {"id": "tough_crowd", "display_name": "Tough Crowd", "hp": 80, "aim_settle_ticks": 12, "miss_chance": 0.1, "sight_range": 320, "sight_fov_degrees": 120, "hearing_radius": 480, "memory_decay_ticks": 300, "reload_ms": 1800, "retreat_hp_pct": 0.3},
                {"id": "veteran", "display_name": "Veteran", "hp": 120, "aim_settle_ticks": 6, "miss_chance": 0.05, "sight_range": 480, "sight_fov_degrees": 140, "hearing_radius": 600, "memory_decay_ticks": 600, "reload_ms": 1200, "retreat_hp_pct": 0.2}
            ]
        });
        let path = write_tmp("difficulty_field_missing.json", &body.to_string());
        let mut report = ValidationReport::default();
        validate_difficulty_json(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1, "expected one FAIL entry");
        assert!(report.entries[0].message.contains("reload_ms"));
    }

    #[test]
    fn material_registry_accepts_valid_registry() {
        let body = serde_json::json!({
            "schema_version": 1,
            "materials": [
                {"id": 0, "name": "air", "display_name": "Air", "hardness": 0.0, "diggable": false, "anchorable": false, "hazard": false, "path_cost": 1.0, "density": 0.0, "color_hex": "000000", "description": "Empty"},
                {"id": 1, "name": "dirt", "display_name": "Dirt", "hardness": 10.0, "diggable": true, "anchorable": true, "hazard": false, "path_cost": 1.0, "density": 1.5, "color_hex": "8B6914", "description": "Dirt"},
                {"id": 2, "name": "concrete", "display_name": "Concrete", "hardness": 40.0, "diggable": true, "anchorable": true, "hazard": false, "path_cost": 1.0, "density": 2.3, "color_hex": "808080", "description": "Concrete"},
                {"id": 3, "name": "metal_nohook", "display_name": "Metal", "hardness": 100.0, "diggable": false, "anchorable": false, "hazard": false, "path_cost": 999.0, "density": 7.8, "color_hex": "4A4A4A", "description": "Metal"},
                {"id": 4, "name": "hazard", "display_name": "Hazard", "hardness": 50.0, "diggable": false, "anchorable": false, "hazard": true, "path_cost": 10.0, "density": 3.0, "color_hex": "FF4444", "description": "Hazard"},
                {"id": 5, "name": "loose_fill", "display_name": "Loose Rubble", "hardness": 5.0, "diggable": true, "anchorable": false, "hazard": false, "path_cost": 2.0, "density": 1.2, "color_hex": "C8A864", "description": "Loose"},
                {"id": 6, "name": "repair_fill", "display_name": "Repair", "hardness": 15.0, "diggable": true, "anchorable": true, "hazard": false, "path_cost": 1.0, "density": 0.8, "color_hex": "44FF44", "description": "Repair"},
                {"id": 7, "name": "anchor", "display_name": "Anchor", "hardness": 60.0, "diggable": false, "anchorable": true, "hazard": false, "path_cost": 1.0, "density": 2.6, "color_hex": "6B4226", "description": "Anchor"}
            ]
        });
        let path = write_tmp("materials_pass.json", &body.to_string());
        let mut report = ValidationReport::default();
        validate_material_registry(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.pass(), 1);
        assert_eq!(report.fail(), 0);
    }

    #[test]
    fn material_registry_rejects_unknown_field() {
        let mut body = serde_json::json!({
            "schema_version": 1,
            "materials": [
                {"id": 0, "name": "air", "display_name": "Air", "hardness": 0.0, "diggable": false, "anchorable": false, "hazard": false, "path_cost": 1.0, "density": 0.0, "color_hex": "000000", "description": "Empty"},
                {"id": 1, "name": "dirt", "display_name": "Dirt", "hardness": 10.0, "diggable": true, "anchorable": true, "hazard": false, "path_cost": 1.0, "density": 1.5, "color_hex": "8B6914", "description": "Dirt"},
                {"id": 2, "name": "concrete", "display_name": "Concrete", "hardness": 40.0, "diggable": true, "anchorable": true, "hazard": false, "path_cost": 1.0, "density": 2.3, "color_hex": "808080", "description": "Concrete"},
                {"id": 3, "name": "metal_nohook", "display_name": "Metal", "hardness": 100.0, "diggable": false, "anchorable": false, "hazard": false, "path_cost": 999.0, "density": 7.8, "color_hex": "4A4A4A", "description": "Metal"},
                {"id": 4, "name": "hazard", "display_name": "Hazard", "hardness": 50.0, "diggable": false, "anchorable": false, "hazard": true, "path_cost": 10.0, "density": 3.0, "color_hex": "FF4444", "description": "Hazard"},
                {"id": 5, "name": "loose_fill", "display_name": "Loose", "hardness": 5.0, "diggable": true, "anchorable": false, "hazard": false, "path_cost": 2.0, "density": 1.2, "color_hex": "C8A864", "description": "Loose"},
                {"id": 6, "name": "repair_fill", "display_name": "Repair", "hardness": 15.0, "diggable": true, "anchorable": true, "hazard": false, "path_cost": 1.0, "density": 0.8, "color_hex": "44FF44", "description": "Repair"},
                {"id": 7, "name": "anchor", "display_name": "Anchor", "hardness": 60.0, "diggable": false, "anchorable": true, "hazard": false, "path_cost": 1.0, "density": 2.6, "color_hex": "6B4226", "description": "Anchor"}
            ]
        });
        body["materials"][1]["rainbow_color"] = serde_json::json!("red");
        let path = write_tmp("materials_unknown.json", &body.to_string());
        let mut report = ValidationReport::default();
        validate_material_registry(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("unknown_field"));
        assert!(report.entries[0].message.contains("rainbow_color"));
    }

    #[test]
    fn material_registry_rejects_schema_version_mismatch() {
        let body = serde_json::json!({
            "schema_version": 42,
            "materials": []
        });
        let path = write_tmp("materials_schema_drift.json", &body.to_string());
        let mut report = ValidationReport::default();
        validate_material_registry(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("schema_version_mismatch"));
    }

    #[test]
    fn difficulty_json_rejects_wrong_schema() {
        let body = serde_json::json!({
            "schema": 2,
            "presets": [
                {"id": "cakewalk", "display_name": "C", "hp": 60, "aim_settle_ticks": 24, "miss_chance": 0.3, "sight_range": 240, "sight_fov_degrees": 90, "hearing_radius": 320, "memory_decay_ticks": 180, "reload_ms": 2400, "retreat_hp_pct": 0.5},
                {"id": "tough_crowd", "display_name": "T", "hp": 80, "aim_settle_ticks": 12, "miss_chance": 0.1, "sight_range": 320, "sight_fov_degrees": 120, "hearing_radius": 480, "memory_decay_ticks": 300, "reload_ms": 1800, "retreat_hp_pct": 0.3},
                {"id": "veteran", "display_name": "V", "hp": 120, "aim_settle_ticks": 6, "miss_chance": 0.05, "sight_range": 480, "sight_fov_degrees": 140, "hearing_radius": 600, "memory_decay_ticks": 600, "reload_ms": 1200, "retreat_hp_pct": 0.2}
            ]
        });
        let path = write_tmp("difficulty_schema.json", &body.to_string());
        let mut report = ValidationReport::default();
        validate_difficulty_json(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1, "expected one FAIL entry");
        assert!(report.entries[0].message.contains("schema must be 1"));
    }

    /// **M4A**: `cf-mod validate content/asset_ledger/ledger.jsonl` happily
    /// accepts a well-formed v1 ledger AND surfaces id_drift / schema drift
    /// as FAIL.
    #[test]
    fn validate_ledger_jsonl_accepts_well_formed() {
        use cf_asset_ledger::{AssetCategory, AssetEntryBuilder, ProductionTier};
        let entry = AssetEntryBuilder::new(
            AssetCategory::WeaponSprite,
            "weapon-side",
            "rifle_validate_mod",
            ProductionTier::Tier1Svg,
            "M9A_svg_v1",
            "p",
            1,
            "/tmp/x.svg",
        )
        .with_output_blake3("a".repeat(64))
        .with_output_size(0)
        .build();
        let mut line = serde_json::to_string(&entry).unwrap();
        line.push('\n');
        let path = write_tmp("ledger_pass.jsonl", &line);
        let new_path = path.with_file_name("ledger.jsonl");
        let _ = std::fs::rename(&path, &new_path);
        let mut report = ValidationReport::default();
        validate_ledger_jsonl(&new_path, &mut report);
        let _ = fs::remove_file(&new_path);
        assert_eq!(report.pass(), 1, "expected one PASS entry, got {:?}", report.entries);
        assert_eq!(report.fail(), 0);
    }

    #[test]
    fn validate_ledger_jsonl_rejects_id_drift() {
        let body = serde_json::json!({
            "id": "0".repeat(64),
            "category": "WeaponSprite",
            "kind": "weapon-side",
            "canonical_name": "drifted",
            "tier": "Tier1_SVG",
            "pipeline": "M9A_svg_v1",
            "generator": {"tool": "", "model": ""},
            "prompt": "p",
            "seed": 0,
            "output_path": "x.svg",
            "output_format": "svg",
            "output_size_bytes": 0,
            "output_blake3": "a".repeat(64),
            "generated_at_iso": "2026-05-13T00:00:00Z",
            "generated_on_machine": "ci",
            "regen_command": "cf-mod ledger regenerate x",
            "schema_version": "1.0.0"
        });
        let mut line = body.to_string();
        line.push('\n');
        let path = write_tmp("ledger_drift.jsonl", &line);
        let new_path = path.with_file_name("ledger.jsonl");
        let _ = std::fs::rename(&path, &new_path);
        let mut report = ValidationReport::default();
        validate_ledger_jsonl(&new_path, &mut report);
        let _ = fs::remove_file(&new_path);
        assert_eq!(report.fail(), 1);
        let msg = &report.entries[0].message;
        assert!(msg.contains("id_drift"), "expected id_drift but got: {msg}");
    }
}
