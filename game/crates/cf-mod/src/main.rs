#![allow(clippy::items_after_test_module)]

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
    /// **M9A**: invoke the Tier-1 SVG asset pipeline. Wraps
    /// `tools/asset_gen/build_placeholders.py` so engine-side tooling can
    /// trigger a bake without shelling out manually.
    #[command(name = "asset-gen")]
    AssetGen {
        #[command(subcommand)]
        action: Box<AssetGenAction>,
    },
    /// **M12A**: invoke the Tier-1 SFX audio pipeline. Wraps
    /// `tools/audio_gen/generate_sfx.py` so engine-side tooling can
    /// trigger an audio bake without shelling out manually. Mirrors the
    /// `asset-gen` subcommand surface (run / check / report).
    #[command(name = "audio-gen")]
    AudioGen {
        #[command(subcommand)]
        action: Box<AudioGenAction>,
    },
    /// **M4B § "cf-mod save validate"** — full schema + migration +
    /// checksum validation pass over a single `.cfsave` file.
    Save {
        #[command(subcommand)]
        action: SaveAction,
    },
}

/// **M4B** save subcommands.
#[derive(Debug, Subcommand)]
enum SaveAction {
    /// Run full validation: schema_version parsable, migration registry
    /// reaches the current version, checksum (when sidecar exists)
    /// matches the canonical-JSON BLAKE3 of the payload.
    Validate { path: PathBuf },
}

/// **M12A**: audio-gen subcommands. Per spec § Files:
/// > `cf-mod` MODIFY — add `cf-mod audio-gen run` subcommand
#[derive(Debug, Subcommand)]
enum AudioGenAction {
    /// Run the full Tier-1 SFX bake. Equivalent to
    /// `tools/asset_gen/.venv/bin/python tools/audio_gen/generate_sfx.py --all`.
    Run {
        /// Optional category filter (e.g. `weapon` / `footstep` / `impact`).
        #[arg(long)]
        category: Option<String>,
        /// Skip the bake; only invoke the pipeline's `--check` dry-run.
        #[arg(long)]
        check: bool,
        /// Print on-disk + ledger SFX counts only.
        #[arg(long)]
        report: bool,
        /// Override the path to the asset pipeline venv's python binary.
        #[arg(long = "venv-python")]
        venv_python: Option<PathBuf>,
        /// Override the path to `generate_sfx.py`.
        #[arg(long = "generate-sfx")]
        generate_sfx: Option<PathBuf>,
        /// Optional mod-pack id for modder authoring (passed to
        /// `generate_sfx.py --mod <id>`).
        #[arg(long)]
        r#mod: Option<String>,
    },
}

/// **M9A**: asset-gen subcommands. Per spec § "Source / cf-mod Cargo.toml":
/// > add `cf-mod asset-gen run` subcommand invoking the Python pipeline
#[derive(Debug, Subcommand)]
enum AssetGenAction {
    /// Run the full Tier-1 bake. Equivalent to
    /// `tools/asset_gen/.venv/bin/python tools/asset_gen/build_placeholders.py --all`.
    Run {
        /// Optional category filter (e.g. `WeaponSprite`).
        #[arg(long)]
        category: Option<String>,
        /// Parallel worker count (0 = serial, 8 = default).
        #[arg(long, default_value_t = 8u32)]
        parallel: u32,
        /// Skip the bake; only invoke the pipeline's `--check` dry-run.
        #[arg(long)]
        check: bool,
        /// Print on-disk + ledger counts only.
        #[arg(long)]
        report: bool,
        /// Override the path to the asset pipeline venv's python binary.
        #[arg(long = "venv-python")]
        venv_python: Option<PathBuf>,
        /// Override the path to `build_placeholders.py`.
        #[arg(long = "build-placeholders")]
        build_placeholders: Option<PathBuf>,
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
        /// **M4B § "Ledger chain rejects tampered bundle"** — verify the
        /// per-event BLAKE3 chain in a run bundle (rather than the asset
        /// ledger). When set, ignores `id` / `all` and walks the bundle's
        /// `events.jsonl` against the manifest's `run_id` + `seed` +
        /// `ledger_chain_anchor`.
        #[arg(long)]
        bundle: Option<PathBuf>,
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
        Cmd::AssetGen { action } => run_asset_gen(action.as_ref(), cli.json),
        Cmd::AudioGen { action } => run_audio_gen(action.as_ref(), cli.json),
        Cmd::Save { action } => run_save(action, cli.json),
    }
}

fn run_save(action: &SaveAction, json_output: bool) -> Result<()> {
    match action {
        SaveAction::Validate { path } => save_validate::run(path, json_output),
    }
}

mod bundle_chain_verify;
mod save_validate;

fn run_audio_gen(action: &AudioGenAction, json_output: bool) -> Result<()> {
    use std::process::{Command, Stdio};

    let AudioGenAction::Run {
        category,
        check,
        report,
        venv_python,
        generate_sfx,
        r#mod,
    } = action;

    let cwd = std::env::current_dir().context("get cwd")?;
    let workspace_root = if cwd.file_name().and_then(|n| n.to_str()) == Some("game") {
        cwd.parent().unwrap_or(&cwd).to_path_buf()
    } else {
        cwd.clone()
    };
    let venv_py = venv_python
        .clone()
        .unwrap_or_else(|| workspace_root.join("tools/asset_gen/.venv/bin/python"));
    let script = generate_sfx
        .clone()
        .unwrap_or_else(|| workspace_root.join("tools/audio_gen/generate_sfx.py"));
    if !venv_py.exists() {
        anyhow::bail!(
            "audio-gen: venv python not found at {}; run `python3 -m venv tools/asset_gen/.venv`",
            venv_py.display()
        );
    }
    if !script.exists() {
        anyhow::bail!(
            "audio-gen: generate_sfx.py not found at {}; M12A pipeline missing",
            script.display()
        );
    }

    let mut cmd = Command::new(&venv_py);
    cmd.arg(&script);
    if *report {
        cmd.arg("--report");
    } else if *check {
        cmd.arg("--check");
    } else {
        cmd.arg("--all");
    }
    if let Some(cat) = category {
        cmd.arg("--category").arg(cat);
    }
    if let Some(mod_id) = r#mod {
        cmd.arg("--mod").arg(mod_id);
    }
    cmd.current_dir(&workspace_root);
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());
    tracing::info!(
        target: "cf::mod::audio_gen",
        venv_python = %venv_py.display(),
        script = %script.display(),
        category = ?category,
        check,
        report,
        mod_id = ?r#mod,
        "running M12A SFX pipeline"
    );
    let status = cmd
        .status()
        .with_context(|| format!("spawn {} {}", venv_py.display(), script.display()))?;
    if json_output {
        let exit = status.code().unwrap_or(-1);
        println!(
            "{}",
            serde_json::json!({
                "subcommand": "audio-gen",
                "exit_code": exit,
                "succeeded": status.success(),
            })
        );
    }
    if !status.success() {
        anyhow::bail!("audio-gen pipeline exited with non-zero status: {:?}", status.code());
    }
    Ok(())
}

fn run_asset_gen(action: &AssetGenAction, json_output: bool) -> Result<()> {
    use std::process::{Command, Stdio};

    let AssetGenAction::Run {
        category,
        parallel,
        check,
        report,
        venv_python,
        build_placeholders,
    } = action;

    let cwd = std::env::current_dir().context("get cwd")?;
    let workspace_root = if cwd.file_name().and_then(|n| n.to_str()) == Some("game") {
        cwd.parent().unwrap_or(&cwd).to_path_buf()
    } else {
        cwd.clone()
    };
    let venv_py = venv_python
        .clone()
        .unwrap_or_else(|| workspace_root.join("tools/asset_gen/.venv/bin/python"));
    let script = build_placeholders
        .clone()
        .unwrap_or_else(|| workspace_root.join("tools/asset_gen/build_placeholders.py"));
    if !venv_py.exists() {
        anyhow::bail!(
            "asset-gen: venv python not found at {}; run `python3 -m venv tools/asset_gen/.venv`",
            venv_py.display()
        );
    }
    if !script.exists() {
        anyhow::bail!("asset-gen: build_placeholders.py not found at {}", script.display());
    }

    let mut cmd = Command::new(&venv_py);
    cmd.arg(&script);
    if *report {
        cmd.arg("--report");
    } else if *check {
        cmd.arg("--check");
    } else if let Some(cat) = category {
        cmd.arg("--category").arg(cat);
        cmd.arg("--parallel").arg(parallel.to_string());
    } else {
        cmd.arg("--all");
        cmd.arg("--parallel").arg(parallel.to_string());
    }
    cmd.current_dir(&workspace_root);
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());
    tracing::info!(
        target: "cf::mod::asset_gen",
        venv_python = %venv_py.display(),
        script = %script.display(),
        category = ?category,
        parallel,
        check,
        report,
        "running M9A asset pipeline"
    );
    let status = cmd
        .status()
        .with_context(|| format!("spawn {} {}", venv_py.display(), script.display()))?;
    if json_output {
        let exit = status.code().unwrap_or(-1);
        println!(
            "{}",
            serde_json::json!({
                "subcommand": "asset-gen",
                "exit_code": exit,
                "succeeded": status.success(),
            })
        );
    }
    if !status.success() {
        anyhow::bail!("asset-gen pipeline exited with non-zero status: {:?}", status.code());
    }
    Ok(())
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
            bundle,
        } => {
            // **M4B § "Ledger chain rejects tampered bundle"** — when
            // `--bundle <path>` is passed, switch to per-event chain
            // verification over the run bundle's `events.jsonl`.
            if let Some(bundle_dir) = bundle {
                return bundle_chain_verify::run(bundle_dir, json_output);
            }
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
            // **M13**: `content/equipment/loadouts/*.json` loadout
            // descriptors validated against `cf_equipment::LoadoutFile`.
            || (path.extension().and_then(|s| s.to_str()) == Some("json")
                && path.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str()) == Some("loadouts"))
            // **M13**: top-level `content/equipment/roles.json` mod-tooling export.
            || (path.extension().and_then(|s| s.to_str()) == Some("json")
                && path.file_name().and_then(|s| s.to_str()) == Some("roles.json")
                && path.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str()) == Some("equipment"))
            // **M5**: cf-mod validate <cf-replay/schemas/> covers all per-event
            // JSON schemas — every <family>_<type>.json under schemas/event/
            // plus the envelope schemas under schemas/v0_1/ + schemas/v1/.
            || is_event_schema_file(&path)
            || is_envelope_schema_file(&path)
        {
            validate_one(&path, report);
        }
    }
}

/// **M5**: identifies a per-event schema file living at
/// `<.../schemas/event/<family>_<type>.json>`. Used by `walk()` to pick the
/// file up and by `validate_one()` to route it to `validate_event_schema_file`.
fn is_event_schema_file(path: &Path) -> bool {
    if path.extension().and_then(|s| s.to_str()) != Some("json") {
        return false;
    }
    let Some(parent) = path.parent() else {
        return false;
    };
    if parent.file_name().and_then(|s| s.to_str()) != Some("event") {
        return false;
    }
    parent.parent().and_then(|gp| gp.file_name()).and_then(|s| s.to_str()) == Some("schemas")
}

/// **M5**: identifies an envelope schema file under
/// `<.../schemas/v0_1/*.schema.json>` or `<.../schemas/v1/*.schema.json>`.
fn is_envelope_schema_file(path: &Path) -> bool {
    if path.extension().and_then(|s| s.to_str()) != Some("json") {
        return false;
    }
    let Some(parent) = path.parent() else {
        return false;
    };
    let Some(parent_name) = parent.file_name().and_then(|s| s.to_str()) else {
        return false;
    };
    if !is_envelope_version_dir(parent_name) {
        return false;
    }
    parent.parent().and_then(|gp| gp.file_name()).and_then(|s| s.to_str()) == Some("schemas")
}

/// **M5-A1**: matches a version-suffixed envelope directory like `v0_1`,
/// `v1`, `v0_2`, `v2_5`. Strictly: `^v[0-9]+(_[0-9]+)?$`. Widens the legacy
/// `v0_1`/`v1` literal match so future M4 envelope-bump migration directories
/// (BP6+) are picked up automatically.
fn is_envelope_version_dir(name: &str) -> bool {
    let Some(rest) = name.strip_prefix('v') else {
        return false;
    };
    if rest.is_empty() {
        return false;
    }
    let mut seen_underscore = false;
    let mut current_segment_has_digit = false;
    for ch in rest.chars() {
        if ch == '_' {
            if seen_underscore || !current_segment_has_digit {
                return false;
            }
            seen_underscore = true;
            current_segment_has_digit = false;
        } else if ch.is_ascii_digit() {
            current_segment_has_digit = true;
        } else {
            return false;
        }
    }
    current_segment_has_digit
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
    // **M4A**: validate the per-pipeline regen manifest at
    // `content/asset_ledger/regen_manifest.ron` against its locked v1.0.0
    // schema (`cf-asset-ledger/schemas/v1/regen_manifest.schema.json`). Each
    // pipeline entry must declare pipeline_id / regen_command / model_version
    // / deterministic; the rest of the schema's fields are optional.
    if path.file_name().and_then(|s| s.to_str()) == Some("regen_manifest.ron") {
        validate_regen_manifest(path, report);
        return;
    }
    // **M11**: validate the M11 interim TTD floors at
    // `content/balance/ttd_floors_interim.ron` against the minimal
    // shape locked in cf-actor::ttd. The validator only confirms the
    // schema_version and that floors/compound_modifiers are non-empty
    // tagged tuples per the file's documented schema; the canonical
    // structural validator is the live cf-actor M17 loader.
    if path.file_name().and_then(|s| s.to_str()) == Some("ttd_floors_interim.ron") {
        validate_ttd_floors_interim(path, report);
        return;
    }
    // **M5**: per-event JSON schema files under cf-replay/schemas/event/.
    if is_event_schema_file(path) {
        validate_event_schema_file(path, report);
        return;
    }
    // **M5**: envelope schema files under cf-replay/schemas/v0_1/ or v1/.
    if is_envelope_schema_file(path) {
        validate_envelope_schema_file(path, report);
        return;
    }
    // **M14G** § wound_specs/*.ron validation.
    if path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        == Some("wound_specs")
        && path.extension().and_then(|s| s.to_str()) == Some("ron")
    {
        validate_wound_spec_ron(path, report);
        return;
    }
    // **M6**: validate the four equipment registries under `content/equipment/`.
    // This must come BEFORE the scenarios fallthrough so the registry RONs
    // aren't mis-routed to `validate_scenario`.
    if path.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str()) == Some("equipment") {
        match path.file_name().and_then(|s| s.to_str()) {
            Some("weapon_registry.ron") => {
                validate_weapon_registry(path, report);
                return;
            }
            Some("grenade_registry.ron") => {
                validate_grenade_registry(path, report);
                return;
            }
            Some("melee_registry.ron") => {
                validate_melee_registry(path, report);
                return;
            }
            Some("tool_registry.ron") => {
                validate_tool_registry(path, report);
                return;
            }
            Some("roles.json") => {
                // M13: roles.json mirrors cf_equipment::role_records(). The
                // file is presentation-only (mod tools + scenario editors);
                // structural validation only checks well-formed JSON +
                // top-level shape so an edit doesn't accidentally break the
                // export tool. Live role definitions remain authoritative
                // in cf_equipment::role_records().
                validate_roles_json(path, report);
                return;
            }
            _ => {}
        }
    }
    // **M13**: validate `content/equipment/loadouts/*.json` against the
    // canonical [`cf_equipment::LoadoutFile`] schema (schema_version, role-id
    // resolution, id↔filename parity). See spec § "Equipment loadouts are
    // data-driven".
    if path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        == Some("loadouts")
        && path
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            == Some("equipment")
        && path.extension().and_then(|s| s.to_str()) == Some("json")
    {
        validate_loadout_file(path, report);
        return;
    }
    // **M6B**: validate `content/equipment/items/*.ron` per spec §
    // "Validate `content/equipment/items/*.ron` against ItemSpec
    // schema". `manifest.ron` is validated against the canonical
    // `cf_equipment::item_spec` registry (mirror drift detection);
    // any other `*.ron` file in the items dir is validated as a
    // standalone `cf_equipment::ItemSpec` definition.
    //
    // **M6C** § "Crates / modules touched": `cf-mod` MODIFY — validate
    // per-category folder. The per-category folders
    // (`content/equipment/<category>/*.ron`) hold standalone
    // `cf_equipment::ItemSpec` files split per-category for modder
    // ergonomics; the validator routes them through the same
    // `validate_item_spec_ron` path used by `items/<id>.ron` so the
    // schema lock applies uniformly.
    let parent_name = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str());
    let grandparent_name = path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str());
    if grandparent_name == Some("equipment")
        && path.extension().and_then(|s| s.to_str()) == Some("ron")
    {
        if parent_name == Some("items") {
            if path.file_name().and_then(|s| s.to_str()) == Some("manifest.ron") {
                validate_item_manifest(path, report);
            } else {
                validate_item_spec_ron(path, report);
            }
            return;
        }
        if matches!(
            parent_name,
            Some("firearms")
                | Some("melee")
                | Some("grenades")
                | Some("heavy")
                | Some("medical")
                | Some("survival")
                | Some("sensors")
                | Some("ppe")
        ) {
            validate_item_spec_ron(path, report);
            return;
        }
    }
    // **M9B-2**: route trench segments + modules + templates BEFORE the
    // `scenarios` fallthrough so `content/trench_segments/*.ron`,
    // `content/trench_modules/*.ron`, and `content/trench_templates/*.trench.ron`
    // are validated through the cf-trench / cf-content loaders (which
    // reject unknown enums + negative depth with typed errors per
    // VAL-M9B-MOD-SEGMENT-001 / VAL-M9B-TEMPLATE-003).
    if path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        == Some("trench_segments")
        && path.extension().and_then(|s| s.to_str()) == Some("ron")
    {
        validate_trench_segment(path, report);
        return;
    }
    if path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        == Some("trench_modules")
        && path.extension().and_then(|s| s.to_str()) == Some("ron")
    {
        validate_trench_module(path, report);
        return;
    }
    if path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        == Some("trench_templates")
        && path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|n| n.ends_with(".trench.ron"))
            .unwrap_or(false)
    {
        validate_trench_template(path, report);
        return;
    }
    // **M9C-1 / VAL-M9C-005 / VAL-M9C-007 / VAL-M9C-MOD-MISSING-DEPENDENCY**:
    // route fortification RONs under `content/fortifications/*.ron`
    // through the cf-fortification FortificationSpec loader. Unknown
    // FortificationKind / wire_kind / mine_kind / anti_tank_kind enum
    // values fail with typed errors; references to dependencies that
    // are not yet shipped (e.g. M9B trench segment ids when M9B is
    // absent) surface as WARN entries with kind `missing_dependency`
    // rather than FAIL.
    if path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        == Some("fortifications")
        && path.extension().and_then(|s| s.to_str()) == Some("ron")
    {
        validate_fortification(path, report);
        return;
    }
    // **M14A** § "cf-mod (EXTEND): validate content/limb_paths/*.ron +
    // content/jetpacks/*.ron + content/quick_action_layouts/*.ron".
    if path.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str()) == Some("limb_paths")
        && path.extension().and_then(|s| s.to_str()) == Some("ron")
    {
        validate_m14a_limb_path(path, report);
        return;
    }
    if path.parent().and_then(|p| p.file_name()).and_then(|s| s.to_str()) == Some("jetpacks")
        && path.extension().and_then(|s| s.to_str()) == Some("ron")
    {
        validate_m14a_jetpack(path, report);
        return;
    }
    if path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        == Some("quick_action_layouts")
        && path.extension().and_then(|s| s.to_str()) == Some("ron")
    {
        validate_m14a_quick_action_layout(path, report);
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

/// **M5**: validate a per-event JSON schema file under
/// `cf-replay/schemas/event/`. Two shapes are accepted:
///
/// 1. **M5 envelope-shaped schemas** (new at M5): MUST declare
///    `properties.schema_version.const = "0.1"`, `properties.category.const`
///    matching the filename family, `properties.event_type.const` matching
///    the filename suffix, `properties.payload` as an object sub-schema,
///    and top-level `required` containing the standard envelope fields
///    `schema_version`, `category`, `event_type`, `tick`, `payload`.
///    Verified against the spec scenario "each schema declares
///    `schema_version=\"0.1\"` matching the M4 locked envelope".
///
/// 2. **Legacy payload-only schemas** (M2-M4): the file pre-dates M5 and
///    describes the payload object directly without an envelope wrapper.
///    Only well-formed-JSON + presence of a `type` or `properties` field
///    is required.
fn validate_event_schema_file(path: &Path, report: &mut ValidationReport) {
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
    let messages = validate_event_schema_value(path, &value);
    if messages.is_empty() {
        let shape = if value.pointer("/properties/schema_version/const").is_some() {
            "M5 envelope-shape"
        } else {
            "legacy payload-only"
        };
        report.add_pass(path.to_path_buf(), format!("event schema ({shape})"));
    } else {
        report.add_error(path.to_path_buf(), messages.join("; "));
    }
}

/// Pure-function half of `validate_event_schema_file` so tests can drive it
/// directly. Returns the empty Vec on success and a list of human-readable
/// error messages on failure.
fn validate_event_schema_value(path: &Path, value: &serde_json::Value) -> Vec<String> {
    let mut messages: Vec<String> = Vec::new();
    // Top-level must be an object.
    let Some(obj) = value.as_object() else {
        messages.push("schema must be a JSON object".to_string());
        return messages;
    };
    // Must be type=object.
    match obj.get("type").and_then(|v| v.as_str()) {
        Some("object") => {}
        Some(other) => messages.push(format!("schema.type must be \"object\" (got {other})")),
        None => {
            // Legacy schemas sometimes omit "type" — only enforce when M5-shaped.
            if value.pointer("/properties/schema_version/const").is_some() {
                messages.push("schema.type missing (M5 envelope shape requires type=object)".to_string());
            }
        }
    }
    // M5 envelope-shape detection: properties.schema_version.const present.
    let is_m5 = value.pointer("/properties/schema_version/const").is_some();
    if !is_m5 {
        // Legacy payload-only schema: require presence of either `properties`
        // or `type` so we know it's a real schema and not random JSON.
        if obj.get("properties").is_none() && obj.get("type").is_none() {
            messages.push("legacy schema must define either `properties` or `type`".to_string());
        }
        return messages;
    }
    // M5 envelope-shape conformance: walk the contract.
    // schema_version.const MUST equal the canonical M4 envelope literal
    // (matches cf-replay/src/lib.rs::EVENT_SCHEMA_VERSION). Producers emit
    // this exact string at envelope level; per-event schemas must declare it
    // so strict JSON Schema validators accept the events.
    let sv = value
        .pointer("/properties/schema_version/const")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if sv != "prototype-recorder-event.v0.1" {
        messages.push(format!(
            "properties.schema_version.const must be \"prototype-recorder-event.v0.1\" (got \"{sv}\")"
        ));
    }
    // Extract canonical (family, type) from the filename: <family>_<type>.json.
    let file_stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string();
    let title = value.get("title").and_then(|v| v.as_str()).unwrap_or_default();
    if title.is_empty() {
        messages.push("title must be set to \"<family>.<event_type>\"".to_string());
    } else if !title.contains('.') {
        messages.push(format!("title \"{title}\" must contain a `.` separator"));
    }
    // properties.category.const must match the family prefix of the filename
    // AND the title's family prefix.
    let cat_const = value
        .pointer("/properties/category/const")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if cat_const.is_empty() {
        messages.push("properties.category.const must be set".to_string());
    } else if !file_stem.starts_with(&format!("{cat_const}_")) && file_stem != cat_const {
        messages.push(format!(
            "filename `{file_stem}` does not start with category `{cat_const}_`"
        ));
    }
    let ty_const = value
        .pointer("/properties/event_type/const")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if ty_const.is_empty() {
        messages.push("properties.event_type.const must be set".to_string());
    } else {
        let expected_stem = format!("{cat_const}_{ty_const}");
        if file_stem != expected_stem {
            messages.push(format!(
                "filename `{file_stem}` does not match expected `{expected_stem}` (from category+event_type consts)"
            ));
        }
    }
    if !title.is_empty() && !cat_const.is_empty() && !ty_const.is_empty() {
        let expected_title = format!("{cat_const}.{ty_const}");
        if title != expected_title {
            messages.push(format!(
                "title `{title}` must equal `{expected_title}` (from category+event_type consts)"
            ));
        }
    }
    // properties.payload must be an object sub-schema with type=object.
    // M5-A1: also assert payload does NOT declare `additionalProperties: false`,
    // which would break the M4 additive-only contract (DR-002) by rejecting
    // new payload fields that M13+/M14+/M16+/M17+/M19+/M20+ producers want
    // to add additively.
    let payload_schema = value.pointer("/properties/payload");
    match payload_schema {
        Some(serde_json::Value::Object(p)) => {
            if let Some(ty) = p.get("type").and_then(|v| v.as_str()) {
                if ty != "object" {
                    messages.push(format!("properties.payload.type must be \"object\" (got \"{ty}\")"));
                }
            } else {
                messages.push("properties.payload.type must be set to \"object\"".to_string());
            }
            if let Some(serde_json::Value::Bool(false)) = p.get("additionalProperties") {
                messages.push(
                    "properties.payload.additionalProperties must NOT be `false` — M4 envelope is additive-only per DR-002; future producers must be able to add fields without an envelope bump"
                        .to_string(),
                );
            }
        }
        Some(_) => messages.push("properties.payload must be a JSON object schema".to_string()),
        None => messages.push("properties.payload must be defined".to_string()),
    }
    // top-level `required` must include the M5 envelope minimums.
    let req: Vec<&str> = value
        .get("required")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str()).collect())
        .unwrap_or_default();
    for must in ["schema_version", "category", "event_type", "tick", "payload"] {
        if !req.contains(&must) {
            messages.push(format!("top-level `required` must include `{must}` (got {req:?})"));
        }
    }
    // tick property must be declared.
    if value.pointer("/properties/tick").is_none() {
        messages.push("properties.tick must be declared".to_string());
    }
    messages
}

/// **M5**: validate an envelope schema file under
/// `cf-replay/schemas/v0_1/` or `cf-replay/schemas/v1/`. These pre-date M5 —
/// the validator just confirms well-formed JSON.
fn validate_envelope_schema_file(path: &Path, report: &mut ValidationReport) {
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
    if !value.is_object() {
        report.add_error(path.to_path_buf(), "schema must be a JSON object".to_string());
        return;
    }
    let title = value.get("title").and_then(|v| v.as_str()).unwrap_or("(no title)");
    report.add_pass(path.to_path_buf(), format!("envelope schema: {title}"));
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

/// **M1.5 (baseline) + M7 (extension)**: cf-mod validator for
/// `content/ai/difficulty.json`. Two registry shapes are accepted:
///
/// 1. **M2 baseline shape** — entries carry the three legacy ids
///    (`cakewalk`, `tough_crowd`, `veteran`) and the M2 AI tuning fields
///    (hp, aim_settle_ticks, miss_chance, sight_range, hearing_radius,
///    memory_decay_ticks, reload_ms). This shape pre-dates the M7 friend-
///    feedback expansion; the schema check stays in place so older mods
///    keep validating.
///
/// 2. **M7 extension shape** — entries declare `archetype_id` +
///    `difficulty_id` and the registry contains exactly **5 archetypes ×
///    3 difficulties = 15 entries**. The spec at `specs/active/M7.md`
///    § "AI difficulty preset registry" mandates each entry carries the
///    M2 fields plus the three new M7 fields: `cover_seek_radius`,
///    `retreat_hp_threshold` (0.15..=0.50), `squad_comm_delay_ticks`.
///    The five archetype ids are rifleman / sniper / assault / engineer /
///    medic; the three difficulty ids are the legacy trio. Every
///    (archetype_id, difficulty_id) pair MUST appear exactly once.
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
    let shape = detect_difficulty_shape(presets);
    match shape {
        DifficultyRegistryShape::M7Extension => {
            validate_difficulty_m7_shape(presets, &mut messages);
        }
        DifficultyRegistryShape::M2Baseline => {
            validate_difficulty_m2_shape(presets, &mut messages);
        }
    }
    if messages.is_empty() {
        let shape_label = match shape {
            DifficultyRegistryShape::M7Extension => "M7 extension shape",
            DifficultyRegistryShape::M2Baseline => "M2 baseline shape",
        };
        report.add_pass(
            path.to_path_buf(),
            format!("difficulty.json ({} presets, {shape_label})", presets.len()),
        );
    } else {
        report.add_error(path.to_path_buf(), messages.join("; "));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DifficultyRegistryShape {
    M2Baseline,
    M7Extension,
}

fn detect_difficulty_shape(presets: &[serde_json::Value]) -> DifficultyRegistryShape {
    if presets
        .iter()
        .any(|p| p.get("archetype_id").and_then(|v| v.as_str()).is_some())
    {
        DifficultyRegistryShape::M7Extension
    } else {
        DifficultyRegistryShape::M2Baseline
    }
}

const M2_DIFFICULTY_REQUIRED_IDS: &[&str] = &["cakewalk", "tough_crowd", "veteran"];
const M2_DIFFICULTY_REQUIRED_FIELDS: &[&str] = &[
    "hp",
    "aim_settle_ticks",
    "miss_chance",
    "sight_range",
    "hearing_radius",
    "memory_decay_ticks",
    "reload_ms",
];

fn validate_difficulty_m2_shape(presets: &[serde_json::Value], messages: &mut Vec<String>) {
    let mut found_ids: Vec<String> = Vec::new();
    for (i, preset) in presets.iter().enumerate() {
        let id = preset.get("id").and_then(|v| v.as_str()).unwrap_or("");
        if id.is_empty() {
            messages.push(format!("presets[{i}].id missing or not a string"));
            continue;
        }
        found_ids.push(id.to_string());
        for field in M2_DIFFICULTY_REQUIRED_FIELDS {
            if preset.get(*field).is_none() {
                messages.push(format!("presets[{i}={id}].{field} missing"));
            }
        }
    }
    for required in M2_DIFFICULTY_REQUIRED_IDS {
        if !found_ids.iter().any(|id| id == required) {
            messages.push(format!("required preset id `{required}` missing"));
        }
    }
}

const M7_DIFFICULTY_ARCHETYPE_IDS: &[&str] = &["rifleman", "sniper", "assault", "engineer", "medic"];
const M7_DIFFICULTY_DIFFICULTY_IDS: &[&str] = &["cakewalk", "tough_crowd", "veteran"];
const M7_DIFFICULTY_REQUIRED_FIELDS: &[&str] = &[
    "id",
    "archetype_id",
    "difficulty_id",
    "name",
    "hp",
    "hp_multiplier",
    "damage_multiplier",
    "aim_settle_ticks",
    "reaction_time_ticks",
    "miss_chance",
    "sight_range",
    "sight_fov_degrees",
    "fov_degrees",
    "hearing_radius",
    "hearing_range",
    "memory_decay_ticks",
    "reload_ms",
    "retreat_hp_threshold",
    "cover_seek_radius",
    "squad_comm_delay_ticks",
];

fn validate_difficulty_m7_shape(presets: &[serde_json::Value], messages: &mut Vec<String>) {
    let expected_total = M7_DIFFICULTY_ARCHETYPE_IDS.len() * M7_DIFFICULTY_DIFFICULTY_IDS.len();
    if presets.len() != expected_total {
        messages.push(format!(
            "M7 difficulty registry must have exactly {expected_total} entries \
             ({} archetypes x {} difficulties); got {}",
            M7_DIFFICULTY_ARCHETYPE_IDS.len(),
            M7_DIFFICULTY_DIFFICULTY_IDS.len(),
            presets.len()
        ));
    }
    let mut seen_pairs: Vec<(String, String)> = Vec::new();
    for (i, preset) in presets.iter().enumerate() {
        let arch = preset.get("archetype_id").and_then(|v| v.as_str()).unwrap_or("");
        let diff = preset.get("difficulty_id").and_then(|v| v.as_str()).unwrap_or("");
        let id = preset.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let label = if id.is_empty() {
            format!("[{i}]")
        } else {
            format!("[{i}={id}]")
        };
        if arch.is_empty() {
            messages.push(format!("presets{label}.archetype_id missing"));
        } else if !M7_DIFFICULTY_ARCHETYPE_IDS.contains(&arch) {
            messages.push(format!(
                "presets{label}.archetype_id `{arch}` not in {M7_DIFFICULTY_ARCHETYPE_IDS:?}"
            ));
        }
        if diff.is_empty() {
            messages.push(format!("presets{label}.difficulty_id missing"));
        } else if !M7_DIFFICULTY_DIFFICULTY_IDS.contains(&diff) {
            messages.push(format!(
                "presets{label}.difficulty_id `{diff}` not in {M7_DIFFICULTY_DIFFICULTY_IDS:?}"
            ));
        }
        if !arch.is_empty() && !diff.is_empty() {
            let pair = (arch.to_string(), diff.to_string());
            if seen_pairs.contains(&pair) {
                messages.push(format!(
                    "presets{label}: duplicate (archetype_id, difficulty_id) pair `({arch}, {diff})`"
                ));
            } else {
                seen_pairs.push(pair);
            }
        }
        for field in M7_DIFFICULTY_REQUIRED_FIELDS {
            if preset.get(*field).is_none() {
                messages.push(format!("presets{label}.{field} missing"));
            }
        }
        if let Some(rht) = preset.get("retreat_hp_threshold").and_then(|v| v.as_f64()) {
            if !(0.15..=0.50).contains(&rht) {
                messages.push(format!("presets{label}.retreat_hp_threshold {rht} outside 0.15..=0.50"));
            }
        }
        if let Some(csr) = preset.get("cover_seek_radius").and_then(|v| v.as_f64()) {
            if csr <= 0.0 {
                messages.push(format!("presets{label}.cover_seek_radius {csr} must be > 0"));
            }
        }
        if let Some(delay) = preset.get("squad_comm_delay_ticks").and_then(|v| v.as_i64()) {
            if delay < 0 {
                messages.push(format!("presets{label}.squad_comm_delay_ticks {delay} must be >= 0"));
            }
        }
        if let Some(mc) = preset.get("miss_chance").and_then(|v| v.as_f64()) {
            if !(0.0..=1.0).contains(&mc) {
                messages.push(format!("presets{label}.miss_chance {mc} outside 0.0..=1.0"));
            }
        }
        if let Some(hpm) = preset.get("hp_multiplier").and_then(|v| v.as_f64()) {
            if hpm <= 0.0 {
                messages.push(format!("presets{label}.hp_multiplier {hpm} must be > 0"));
            }
        }
        if let Some(dm) = preset.get("damage_multiplier").and_then(|v| v.as_f64()) {
            if dm <= 0.0 {
                messages.push(format!("presets{label}.damage_multiplier {dm} must be > 0"));
            }
        }
    }
    for arch in M7_DIFFICULTY_ARCHETYPE_IDS {
        for diff in M7_DIFFICULTY_DIFFICULTY_IDS {
            let pair = ((*arch).to_string(), (*diff).to_string());
            if !seen_pairs.contains(&pair) {
                messages.push(format!(
                    "required (archetype_id, difficulty_id) pair `({arch}, {diff})` missing"
                ));
            }
        }
    }
}

/// **M6**: shared shape for one entry in a M6 equipment registry RON.
/// Every entry MUST declare a non-empty `id` and a non-empty `kind` (or
/// `class` for weapons). Display name is optional in this minimal contract.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct M6WeaponEntry {
    id: String,
    class: String,
    #[serde(default)]
    #[allow(dead_code)]
    display_name: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct M6WeaponRegistry {
    schema_version: u32,
    weapons: Vec<M6WeaponEntry>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct M6GrenadeEntry {
    id: String,
    kind: String,
    #[serde(default)]
    #[allow(dead_code)]
    display_name: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    fuse_seconds: Option<f32>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct M6GrenadeRegistry {
    schema_version: u32,
    grenades: Vec<M6GrenadeEntry>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct M6MeleeEntry {
    id: String,
    kind: String,
    #[serde(default)]
    #[allow(dead_code)]
    display_name: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct M6MeleeRegistry {
    schema_version: u32,
    melees: Vec<M6MeleeEntry>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct M6ToolEntry {
    id: String,
    kind: String,
    #[serde(default)]
    #[allow(dead_code)]
    display_name: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct M6ToolRegistry {
    schema_version: u32,
    tools: Vec<M6ToolEntry>,
}

fn validate_m6_registry_common(
    path: &Path,
    schema_version: u32,
    entry_ids: Vec<String>,
    entry_kinds: Vec<String>,
    expected_label: &str,
) -> Vec<String> {
    let mut messages: Vec<String> = Vec::new();
    if schema_version != 1 {
        messages.push(format!(
            "{expected_label}.schema_version must be 1 (got {schema_version})"
        ));
    }
    if entry_ids.is_empty() {
        messages.push(format!("{expected_label} registry must have at least 1 entry"));
    }
    for (i, id) in entry_ids.iter().enumerate() {
        if id.trim().is_empty() {
            messages.push(format!("{expected_label}[{i}].id must be non-empty"));
        }
    }
    for (i, k) in entry_kinds.iter().enumerate() {
        if k.trim().is_empty() {
            messages.push(format!("{expected_label}[{i}].kind/class must be non-empty"));
        }
    }
    let _ = path;
    messages
}

fn validate_weapon_registry(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let parsed: M6WeaponRegistry = match ron::from_str(&raw) {
        Ok(v) => v,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("ron parse failed: {err}"));
            return;
        }
    };
    let ids: Vec<String> = parsed.weapons.iter().map(|w| w.id.clone()).collect();
    let kinds: Vec<String> = parsed.weapons.iter().map(|w| w.class.clone()).collect();
    let messages = validate_m6_registry_common(path, parsed.schema_version, ids, kinds, "weapon_registry");
    if messages.is_empty() {
        report.add_pass(
            path.to_path_buf(),
            format!("weapon_registry ({} entries)", parsed.weapons.len()),
        );
    } else {
        report.add_error(path.to_path_buf(), messages.join("; "));
    }
}

fn validate_grenade_registry(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let parsed: M6GrenadeRegistry = match ron::from_str(&raw) {
        Ok(v) => v,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("ron parse failed: {err}"));
            return;
        }
    };
    let ids: Vec<String> = parsed.grenades.iter().map(|g| g.id.clone()).collect();
    let kinds: Vec<String> = parsed.grenades.iter().map(|g| g.kind.clone()).collect();
    let messages = validate_m6_registry_common(path, parsed.schema_version, ids, kinds, "grenade_registry");
    if messages.is_empty() {
        report.add_pass(
            path.to_path_buf(),
            format!("grenade_registry ({} entries)", parsed.grenades.len()),
        );
    } else {
        report.add_error(path.to_path_buf(), messages.join("; "));
    }
}

fn validate_melee_registry(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let parsed: M6MeleeRegistry = match ron::from_str(&raw) {
        Ok(v) => v,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("ron parse failed: {err}"));
            return;
        }
    };
    let ids: Vec<String> = parsed.melees.iter().map(|m| m.id.clone()).collect();
    let kinds: Vec<String> = parsed.melees.iter().map(|m| m.kind.clone()).collect();
    let messages = validate_m6_registry_common(path, parsed.schema_version, ids, kinds, "melee_registry");
    if messages.is_empty() {
        report.add_pass(
            path.to_path_buf(),
            format!("melee_registry ({} entries)", parsed.melees.len()),
        );
    } else {
        report.add_error(path.to_path_buf(), messages.join("; "));
    }
}

fn validate_tool_registry(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let parsed: M6ToolRegistry = match ron::from_str(&raw) {
        Ok(v) => v,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("ron parse failed: {err}"));
            return;
        }
    };
    let ids: Vec<String> = parsed.tools.iter().map(|t| t.id.clone()).collect();
    let kinds: Vec<String> = parsed.tools.iter().map(|t| t.kind.clone()).collect();
    let messages = validate_m6_registry_common(path, parsed.schema_version, ids, kinds, "tool_registry");
    if messages.is_empty() {
        report.add_pass(
            path.to_path_buf(),
            format!("tool_registry ({} entries)", parsed.tools.len()),
        );
    } else {
        report.add_error(path.to_path_buf(), messages.join("; "));
    }
}

/// **M13**: validate one `content/equipment/loadouts/*.json` file.
/// Verifies the schema_version, id↔filename parity, non-empty `role_ids`,
/// and that every referenced role id resolves through `cf_equipment::role_record`.
fn validate_loadout_file(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string();
    match cf_equipment::load_loadout_from_json(&raw, Some(&stem)) {
        Ok(loadout) => report.add_pass(
            path.to_path_buf(),
            format!(
                "loadout {} ({} role{})",
                loadout.id,
                loadout.role_ids.len(),
                if loadout.role_ids.len() == 1 { "" } else { "s" }
            ),
        ),
        Err(err) => report.add_error(path.to_path_buf(), format!("{err}")),
    }
}

/// **M6B**: ItemSpec manifest entry shape (mirrors
/// `cf_equipment::ItemSpec::{id, category}`).
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct M6BItemManifestEntry {
    id: String,
    category: String,
}

/// **M6B**: full item manifest envelope. Schema_version is locked at 1
/// per spec § Files.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct M6BItemManifest {
    schema_version: u32,
    items: Vec<M6BItemManifestEntry>,
}

/// **M6B**: validate the on-disk
/// `content/equipment/items/manifest.ron` against the canonical
/// `cf_equipment::item_spec` registry. Each manifest entry MUST resolve
/// through `spec_for_id()` and the declared `category` MUST match the
/// runtime spec's `category.as_str()`. Manifest must list at least every
/// registered item id (drift-detection: a hardcoded registry entry that
/// authors forgot to list in the manifest is a bug).
fn validate_item_manifest(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let parsed: M6BItemManifest = match ron::from_str(&raw) {
        Ok(v) => v,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("ron parse failed: {err}"));
            return;
        }
    };
    let mut messages: Vec<String> = Vec::new();
    if parsed.schema_version != 1 {
        messages.push(format!(
            "item_manifest.schema_version must be 1 (got {})",
            parsed.schema_version
        ));
    }
    if parsed.items.is_empty() {
        messages.push("item_manifest must declare at least 1 item".to_string());
    }
    let mut manifest_ids: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for (i, entry) in parsed.items.iter().enumerate() {
        if entry.id.trim().is_empty() {
            messages.push(format!("items[{i}].id must be non-empty"));
            continue;
        }
        if !manifest_ids.insert(entry.id.clone()) {
            messages.push(format!("items[{i}].id `{}` duplicated", entry.id));
        }
        match cf_equipment::spec_for_id(&entry.id) {
            Some(spec) => {
                if spec.category.as_str() != entry.category {
                    messages.push(format!(
                        "items[{i}].id `{}` category mismatch (manifest={}, registry={})",
                        entry.id,
                        entry.category,
                        spec.category.as_str()
                    ));
                }
            }
            None => {
                messages.push(format!(
                    "items[{i}].id `{}` is not registered in cf_equipment::item_spec",
                    entry.id
                ));
            }
        }
    }
    // Drift-detection: every registered id must appear in the manifest.
    let registry_ids: std::collections::BTreeSet<String> = cf_equipment::item_registered_ids().into_iter().collect();
    for missing in registry_ids.difference(&manifest_ids) {
        messages.push(format!(
            "registered item `{missing}` is missing from manifest.ron (mirror drift)"
        ));
    }
    if messages.is_empty() {
        report.add_pass(
            path.to_path_buf(),
            format!("item_manifest ({} entries)", parsed.items.len()),
        );
    } else {
        report.add_error(path.to_path_buf(), messages.join("; "));
    }
}

/// **M6B**: validate a standalone `content/equipment/items/<id>.ron`
/// ItemSpec definition. The file must parse as a `cf_equipment::ItemSpec`
/// (via serde) and the canonical id MUST already be registered in the
/// runtime registry (so mods can't ship arbitrary undeclared ids while
/// the lock window stays narrow). Per spec § "Validate
/// `content/equipment/items/*.ron` against ItemSpec schema".
fn validate_item_spec_ron(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let spec: cf_equipment::ItemSpec = match ron::from_str(&raw) {
        Ok(v) => v,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("item_spec ron parse failed: {err}"));
            return;
        }
    };
    let mut messages: Vec<String> = Vec::new();
    if spec.id.trim().is_empty() {
        messages.push("item_spec.id must be non-empty".to_string());
    }
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    if !stem.is_empty() && stem != spec.id {
        messages.push(format!(
            "item_spec.id `{}` mismatches filename stem `{stem}`",
            spec.id
        ));
    }
    if spec.mass_kg < 0.0 || !spec.mass_kg.is_finite() {
        messages.push(format!("item_spec.mass_kg must be finite and >= 0 (got {})", spec.mass_kg));
    }
    if spec.dimensions.w == 0 || spec.dimensions.h == 0 {
        messages.push(format!(
            "item_spec.dimensions must be > 0 in both axes (got {}×{})",
            spec.dimensions.w, spec.dimensions.h
        ));
    }
    if spec.bulk_volume_l < 0.0 || !spec.bulk_volume_l.is_finite() {
        messages.push(format!(
            "item_spec.bulk_volume_l must be finite and >= 0 (got {})",
            spec.bulk_volume_l
        ));
    }
    if spec.stackable && spec.max_stack == 0 {
        messages.push("stackable items must declare max_stack > 0".to_string());
    }
    if let Some(cap) = &spec.container_capacity {
        if cap.max_nest_depth > cf_equipment::MAX_CONTAINER_NEST_DEPTH {
            messages.push(format!(
                "container_capacity.max_nest_depth ({}) exceeds engine cap ({})",
                cap.max_nest_depth,
                cf_equipment::MAX_CONTAINER_NEST_DEPTH
            ));
        }
    }
    if let Some(liquid_cap) = spec.liquid_capacity_l {
        if liquid_cap < 0.0 || !liquid_cap.is_finite() {
            messages.push(format!(
                "liquid_capacity_l must be finite and >= 0 (got {liquid_cap})"
            ));
        }
    }
    // Registry-pinned: the id must already be a known runtime registry
    // entry. This keeps mods from declaring unknown ids during M6B's
    // lock window; M6C+ may relax this once the SKU pipeline lands.
    if cf_equipment::spec_for_id(&spec.id).is_none() {
        messages.push(format!(
            "item_spec.id `{}` is not registered in cf_equipment::item_spec (M6B locked the registry; new ids land in M6C+)",
            spec.id
        ));
    }
    if messages.is_empty() {
        report.add_pass(
            path.to_path_buf(),
            format!("item_spec `{}` ({} kg, {}×{}, {} L)", spec.id, spec.mass_kg, spec.dimensions.w, spec.dimensions.h, spec.bulk_volume_l),
        );
    } else {
        report.add_error(path.to_path_buf(), messages.join("; "));
    }
}

/// **M13**: validate the `content/equipment/roles.json` mod-tooling export.
/// Authoritative roles live in `cf_equipment::role_records()`; the JSON file
/// is presentation-only. Validation only checks well-formed JSON with a
/// top-level `schema_version` integer + a `roles` array.
fn validate_roles_json(path: &Path, report: &mut ValidationReport) {
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
    let Some(obj) = value.as_object() else {
        report.add_error(path.to_path_buf(), "roles.json must be a JSON object".to_string());
        return;
    };
    if obj.get("schema_version").and_then(|v| v.as_u64()).is_none() {
        report.add_error(
            path.to_path_buf(),
            "roles.json must declare an integer schema_version".to_string(),
        );
        return;
    }
    let Some(roles) = obj.get("roles").and_then(|v| v.as_array()) else {
        report.add_error(path.to_path_buf(), "roles.json must declare a `roles` array".to_string());
        return;
    };
    let mut unknown: Vec<String> = Vec::new();
    for entry in roles {
        if let Some(id) = entry.get("id").and_then(|v| v.as_str()) {
            if cf_equipment::role_record(id).is_none() {
                unknown.push(id.to_string());
            }
        }
    }
    if !unknown.is_empty() {
        report.add_warn(
            path.to_path_buf(),
            format!(
                "roles.json references {} role id(s) not present in cf_equipment::role_records(): {}",
                unknown.len(),
                unknown.join(", ")
            ),
        );
    }
    report.add_pass(path.to_path_buf(), format!("roles.json ({} entries)", roles.len()));
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
            // **M9** § cf-mod validate micro_reactor_defense — extra rules per
            // spec § "When `cargo run -p cf-mod -- validate content/scenarios/`
            // runs Then the validator confirms: 1 reactor with mission_critical=true
            // + hp>0 + AABB defined, 1 player spawn, 1 guard slot, timer in
            // [1800, 10800] ticks (30-180s @60Hz), objectives[] includes
            // defend_reactor".
            if scenario.id == "micro_reactor_defense" {
                if scenario.reactors.len() != 1 {
                    messages.push(format!(
                        "M9 micro_reactor_defense must declare exactly 1 reactor (got {})",
                        scenario.reactors.len()
                    ));
                }
                if let Some(r) = scenario.reactors.first() {
                    if r.hp <= 0.0 {
                        messages.push(format!("M9 reactor.hp must be > 0 (got {})", r.hp));
                    }
                    if r.half_extents.0 <= 0.0 || r.half_extents.1 <= 0.0 {
                        messages.push("M9 reactor.half_extents must define a positive AABB".to_string());
                    }
                }
                let controllable_count = scenario.actors.iter().filter(|a| a.controllable).count();
                if controllable_count != 1 {
                    messages.push(format!(
                        "M9 micro_reactor_defense must declare exactly 1 controllable player spawn (got {controllable_count})"
                    ));
                }
                let guard_count = scenario.actors.iter().filter(|a| a.enemy.is_some()).count();
                if guard_count < 1 {
                    messages
                        .push("M9 micro_reactor_defense must declare at least 1 reactive_guard enemy slot".to_string());
                }
                if let Some(mission) = scenario.mission.as_ref() {
                    if mission.time_limit_ticks < 1800 || mission.time_limit_ticks > 10800 {
                        messages.push(format!(
                            "M9 micro_reactor_defense mission.time_limit_ticks must be in [1800, 10800] (got {})",
                            mission.time_limit_ticks
                        ));
                    }
                } else {
                    messages.push("M9 micro_reactor_defense must declare a mission timer block".to_string());
                }
                let has_defend_reactor = scenario
                    .objectives
                    .iter()
                    .any(|o| matches!(&o.kind, cf_control::ScenarioObjectiveKind::DefendReactor { .. }));
                if !has_defend_reactor {
                    messages.push(
                        "M9 micro_reactor_defense must declare at least one defend_reactor objective".to_string(),
                    );
                }
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

/// **M4A**: minimal structural mirror of `content/asset_ledger/regen_manifest.ron`,
/// kept in this binary so the validator does not pull a serde dep into
/// `cf-asset-ledger` itself. Matches the locked v1.0.0 schema at
/// `cf-asset-ledger/schemas/v1/regen_manifest.schema.json`.
#[derive(Debug, serde::Deserialize)]
struct RegenManifestV1 {
    schema_version: String,
    pipelines: Vec<RegenPipelineEntry>,
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct RegenPipelineEntry {
    pipeline_id: String,
    #[serde(default)]
    owner_milestone: String,
    regen_command: String,
    model_version: String,
    deterministic: bool,
    #[serde(default)]
    freeze_path_suffix: String,
    #[serde(default)]
    notes: String,
}

/// **M11**: minimal structural check for `content/balance/ttd_floors_interim.ron`.
/// Verifies the file is RON-parseable, declares `schema_version: "1.0.0"`,
/// and has at least one floor entry. The canonical M17 loader will replace
/// this with a strict validator once M17 ships.
fn validate_ttd_floors_interim(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    #[derive(serde::Deserialize)]
    struct FloorEntry {
        kind: String,
        origin: String,
        difficulty: String,
        seconds: f32,
    }
    #[derive(serde::Deserialize)]
    struct CompoundModifier {
        a: String,
        b: String,
        multiplier: f32,
    }
    #[derive(serde::Deserialize)]
    struct TtdFloorsInterim {
        schema_version: String,
        floors: Vec<FloorEntry>,
        #[serde(default)]
        compound_modifiers: Vec<CompoundModifier>,
    }
    let v: TtdFloorsInterim = match ron::from_str(&raw) {
        Ok(v) => v,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("ttd_floors_interim parse: {err}"));
            return;
        }
    };
    let mut messages: Vec<String> = Vec::new();
    if v.schema_version != "1.0.0" {
        messages.push(format!(
            "ttd_floors_interim.schema_version must be \"1.0.0\" (got {:?})",
            v.schema_version
        ));
    }
    if v.floors.is_empty() {
        messages.push("ttd_floors_interim.floors must contain at least one entry".to_string());
    }
    for (i, f) in v.floors.iter().enumerate() {
        if f.kind.trim().is_empty() {
            messages.push(format!("floors[{i}].kind must be non-empty"));
        }
        if f.origin.trim().is_empty() {
            messages.push(format!("floors[{i}].origin must be non-empty"));
        }
        if f.difficulty.trim().is_empty() {
            messages.push(format!("floors[{i}].difficulty must be non-empty"));
        }
        if !f.seconds.is_finite() || f.seconds < 0.0 {
            messages.push(format!(
                "floors[{i}].seconds must be a finite non-negative float (got {})",
                f.seconds
            ));
        }
    }
    for (i, cm) in v.compound_modifiers.iter().enumerate() {
        if !cm.multiplier.is_finite() || cm.multiplier < 0.0 {
            messages.push(format!(
                "compound_modifiers[{i}].multiplier must be finite and non-negative (got {})",
                cm.multiplier
            ));
        }
        if cm.a.trim().is_empty() || cm.b.trim().is_empty() {
            messages.push(format!("compound_modifiers[{i}].a and b must be non-empty"));
        }
    }
    if messages.is_empty() {
        report.add_pass(
            path.to_path_buf(),
            format!(
                "ttd_floors_interim v{} ({} floors, {} compound)",
                v.schema_version,
                v.floors.len(),
                v.compound_modifiers.len()
            ),
        );
    } else {
        report.add_error(path.to_path_buf(), messages.join("; "));
    }
}

/// **M14G § VAL-M14G-008 / VAL-CROSS-012 / VAL-CROSS-028**: validate one
/// `content/wound_specs/<name>.ron` file against the
/// [`cf_wound::WoundSpec`] schema. Rejects files that reference an unknown
/// `WoundKind`, omit any of the 11 required fields, or carry an
/// `heal_time_seconds_at_band` array of length ≠ 6.
fn validate_wound_spec_ron(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let spec: cf_wound::WoundSpec = match ron::from_str(&raw) {
        Ok(s) => s,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("wound_spec parse: {err}"));
            return;
        }
    };
    let mut messages: Vec<String> = Vec::new();
    if spec.heal_time_seconds_at_band.len() != 6 {
        messages.push(format!(
            "heal_time_seconds_at_band must have length 6, got {}",
            spec.heal_time_seconds_at_band.len()
        ));
    }
    if spec.decal_id.as_str().is_empty() {
        messages.push("decal_id must be non-empty".to_string());
    }
    if !(spec.bleed_rate_ml_per_s_per_severity.is_finite() && spec.bleed_rate_ml_per_s_per_severity >= 0.0) {
        messages.push("bleed_rate_ml_per_s_per_severity must be finite + non-negative".to_string());
    }
    if !(spec.pain_contribution_per_severity.is_finite() && spec.pain_contribution_per_severity >= 0.0) {
        messages.push("pain_contribution_per_severity must be finite + non-negative".to_string());
    }
    if !(spec.infection_base_chance_per_tick.is_finite() && spec.infection_base_chance_per_tick >= 0.0) {
        messages.push("infection_base_chance_per_tick must be finite + non-negative".to_string());
    }
    if messages.is_empty() {
        report.add_pass(
            path.to_path_buf(),
            format!("wound_spec kind={:?} decal_id={}", spec.kind, spec.decal_id.as_str()),
        );
    } else {
        report.add_error(path.to_path_buf(), messages.join("; "));
    }
}

fn validate_regen_manifest(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let manifest: RegenManifestV1 = match ron::from_str(&raw) {
        Ok(m) => m,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("regen_manifest parse: {err}"));
            return;
        }
    };
    let mut messages: Vec<String> = Vec::new();
    if manifest.schema_version != "1.0.0" {
        messages.push(format!(
            "regen_manifest.schema_version must be \"1.0.0\" (got {:?})",
            manifest.schema_version
        ));
    }
    if manifest.pipelines.is_empty() {
        messages.push("regen_manifest.pipelines must contain at least one entry".to_string());
    }
    for (i, entry) in manifest.pipelines.iter().enumerate() {
        if entry.pipeline_id.trim().is_empty() {
            messages.push(format!("pipelines[{i}].pipeline_id must be non-empty"));
        }
        if entry.regen_command.trim().is_empty() {
            messages.push(format!("pipelines[{i}].regen_command must be non-empty"));
        }
        if entry.model_version.trim().is_empty() {
            messages.push(format!("pipelines[{i}].model_version must be non-empty"));
        }
    }
    if messages.is_empty() {
        report.add_pass(
            path.to_path_buf(),
            format!(
                "regen_manifest v{} ({} pipelines)",
                manifest.schema_version,
                manifest.pipelines.len()
            ),
        );
    } else {
        report.add_error(path.to_path_buf(), messages.join("; "));
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

/// **M9B-2 / VAL-M9B-MOD-SEGMENT-001**: validate one
/// `content/trench_segments/*.ron` file. The cf-trench
/// `SegmentSpec::from_ron_str` rejects unknown enums + missing required
/// fields (e.g. negative `depth`/`width`, since both are `u32`) with a
/// typed `ron::error::SpannedError` naming the offending field. The
/// validator surfaces the error inline so cfctl + CI can pattern-match
/// on `unknown_variant` / `missing_field` / `expected unsigned integer`.
fn validate_trench_segment(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    match cf_trench::SegmentSpec::from_ron_str(&raw) {
        Ok(spec) => {
            let derived = cf_trench::segment::CoverByStance::for_variant(spec.variant);
            if derived != spec.cover_state {
                report.add_error(
                    path.to_path_buf(),
                    format!(
                        "cover_state drift for variant {:?}: authored != cf_trench::cover_state derivation",
                        spec.variant
                    ),
                );
                return;
            }
            let filename_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
            if filename_stem != spec.variant.as_str() {
                report.add_error(
                    path.to_path_buf(),
                    format!(
                        "segment variant `{}` mismatches filename stem `{filename_stem}`",
                        spec.variant.as_str()
                    ),
                );
                return;
            }
            report.add_pass(
                path.to_path_buf(),
                format!(
                    "trench_segment ({} depth={} width={})",
                    spec.variant.as_str(),
                    spec.depth,
                    spec.width
                ),
            );
        }
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("ron parse failed: {err}"));
        }
    }
}

/// **M9B-2 / VAL-M9B-MODULES-001 (mod surface)**: validate one
/// `content/trench_modules/*.ron` file via the cf-trench
/// `ModuleSpec::from_ron_str` loader. Same typed-error contract as
/// [`validate_trench_segment`].
fn validate_trench_module(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    match cf_trench::ModuleSpec::from_ron_str(&raw) {
        Ok(spec) => {
            report.add_pass(
                path.to_path_buf(),
                format!(
                    "trench_module ({} build_time_seconds={})",
                    spec.module.as_str(),
                    spec.build_time_seconds
                ),
            );
        }
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("ron parse failed: {err}"));
        }
    }
}

/// Best-effort extraction of the bad variant name from an unstructured
/// ron parse error string. Recognises `variant named \`ultra_deep\``
/// (current ron 0.12 wording) and falls back to None when the format
/// changes. Used only to tag the cf-mod error string with the spec-
/// literal `unknown_segment_variant: <bad>` label.
fn extract_bad_variant_name(msg: &str) -> Option<String> {
    if let Some(idx) = msg.find("variant named `") {
        let rest = &msg[idx + "variant named `".len()..];
        return rest.split('`').next().map(|s| s.to_string());
    }
    None
}

fn extract_bad_fortification_id(msg: &str) -> Option<String> {
    if let Some(idx) = msg.find("unknown_fortification_id:") {
        let rest = &msg[idx + "unknown_fortification_id:".len()..];
        return rest.split_whitespace().next().map(|s| s.to_string());
    }
    if let Some(idx) = msg.find("UnknownFortificationId(") {
        let rest = &msg[idx + "UnknownFortificationId(".len()..];
        return rest.split(')').next().map(|s| s.trim_matches('"').to_string());
    }
    None
}

/// **M9B-2 / VAL-M9B-TEMPLATE-003**: validate one
/// `content/trench_templates/*.trench.ron` file through the cf-content
/// loader. Unknown segment variants + unknown fortification ids fail
/// with typed errors; optional fortification placeholders that resolve
/// to KNOWN-but-not-yet-shipped M9C ids emit a WARN entry
/// (`trench_template_missing_fortification` per spec § Notes for the
/// implementer / VAL-M9B-TEMPLATE-004) rather than a FAIL.
fn validate_trench_template(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let template = match cf_content::TrenchTemplate::from_ron_str(&raw) {
        Ok(t) => t,
        Err(err) => {
            let msg = format!("{err}");
            let lower = msg.to_lowercase();
            // Tag the well-known failure shapes with the spec-literal
            // labels VAL-M9B-TEMPLATE-003 mentions, so cfctl + CI can
            // pattern-match on the exact strings.
            let tagged = if lower.contains("variant named") || lower.contains("variant `")
                || lower.contains("unknown variant")
                || lower.contains("expected one of")
                || msg.contains("UnknownSegmentVariant")
            {
                if let Some(bad) = extract_bad_variant_name(&msg) {
                    format!("unknown_segment_variant: {bad}; {msg}")
                } else {
                    format!("unknown_segment_variant: {msg}")
                }
            } else if msg.contains("unknown_fortification_id")
                || msg.contains("UnknownFortificationId")
            {
                if let Some(bad) = extract_bad_fortification_id(&msg) {
                    format!("unknown_fortification_id: {bad}; {msg}")
                } else {
                    msg
                }
            } else {
                msg
            };
            report.add_error(path.to_path_buf(), format!("ron parse failed: {tagged}"));
            return;
        }
    };
    let filename_stem = path.file_name().and_then(|s| s.to_str()).unwrap_or_default();
    let expected = format!("{}.trench.ron", template.id);
    if filename_stem != expected {
        report.add_error(
            path.to_path_buf(),
            format!(
                "template id `{}` mismatches filename `{filename_stem}` (expected `{expected}`)",
                template.id
            ),
        );
        return;
    }
    // **VAL-M9B-TEMPLATE-004** — surface missing-fortification
    // placeholders as warnings (not failures) so the template still
    // ships in degraded mode pre-M9C.
    for placeholder in &template.fortification_placeholders {
        if placeholder.optional {
            report.add_warn(
                path.to_path_buf(),
                format!(
                    "{}: optional placeholder `{}` may emit `trench_template_missing_fortification` warning event until M9C ships",
                    cf_content::placeholder_warning_label(),
                    placeholder.fortification_id
                ),
            );
        }
    }
    report.add_pass(
        path.to_path_buf(),
        format!(
            "trench_template `{}` ({} polyline pts, {} placeholders)",
            template.id,
            template.path_polyline.len(),
            template.fortification_placeholders.len()
        ),
    );
}

/// Per VAL-M9C-MOD-MISSING-DEPENDENCY: the warning event kind cf-mod
/// emits when a fortification RON's `depends_on` list references an
/// id whose owning milestone has not yet shipped. The string is
/// surfaced in the validator's `add_warn` message so cfctl + CI can
/// pattern-match on `missing_dependency:` prefixes.
pub const FORTIFICATION_MISSING_DEPENDENCY_WARNING: &str = "fortification_missing_dependency";

/// Spec § Notes for the implementer: dependencies that are NOT in
/// `specs/done/` yet must downgrade the validator's verdict to a WARN
/// (rather than FAIL) so the asset still loads in degraded mode.
/// Update this list as milestones close.
const SHIPPED_M9C_DEPENDENCIES: &[&str] = &[
    // M9B authored content kept available to M9C fortifications.
    "trench_segment:shallow_scrape",
    "trench_segment:standard",
    "trench_segment:deep",
    "trench_segment:communication",
    "trench_segment:fire_step",
    "trench_segment:parapet_raised",
    // M28F bunker template surface that bunker_firing_slit pre-embeds in.
    "bunker_template:m28f_t2",
    // M29 power-grid kernel that spotlight + electrified_fence consume.
    "m29_power_grid",
    // M30B engineering_tool tier ladder; anti_tank_ditch dig-tool.
    "engineering_tool:t2",
];

/// VAL-M9C-005 / VAL-M9C-007 / VAL-M9C-MOD-MISSING-DEPENDENCY:
/// validate one `content/fortifications/*.ron` file via the
/// cf-fortification `FortificationSpec::from_ron_str` loader.
///
/// - Unknown FortificationKind / wire_kind / mine_kind /
///   anti_tank_kind enum values fail with a typed `ron::SpannedError`.
/// - The filename stem MUST match `spec.kind.as_str()`.
/// - Each `depends_on` entry is checked against
///   [`SHIPPED_M9C_DEPENDENCIES`]: unknown ids surface as WARN
///   entries with kind `fortification_missing_dependency` so the
///   asset still loads in degraded mode.
fn validate_fortification(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    match cf_fortification::FortificationSpec::from_ron_str(&raw) {
        Ok(spec) => {
            let filename_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
            if filename_stem != spec.kind.as_str() {
                report.add_error(
                    path.to_path_buf(),
                    format!(
                        "fortification kind `{}` mismatches filename stem `{filename_stem}`",
                        spec.kind.as_str()
                    ),
                );
                return;
            }
            for dep in &spec.depends_on {
                if !SHIPPED_M9C_DEPENDENCIES.contains(&dep.as_str()) {
                    report.add_warn(
                        path.to_path_buf(),
                        format!(
                            "{FORTIFICATION_MISSING_DEPENDENCY_WARNING}: `{dep}` is not registered as a shipped dependency; \
                             fortification `{}` loads in degraded mode",
                            spec.kind.as_str()
                        ),
                    );
                }
            }
            report.add_pass(
                path.to_path_buf(),
                format!(
                    "fortification ({} hp={} footprint={}x{})",
                    spec.kind.as_str(),
                    spec.hp,
                    spec.footprint_tiles.0,
                    spec.footprint_tiles.1
                ),
            );
        }
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("ron parse failed: {err}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    static TMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn write_tmp(name: &str, contents: &str) -> PathBuf {
        // PID + atomic counter is enough for uniqueness; SystemTime::now is
        // disallowed by the workspace clippy lint.
        let seq = TMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("cf-mod-test-{pid}-{seq}"));
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

    fn m7_difficulty_entry(arch: &str, diff: &str, retreat: f64, cover: f64, squad_delay: i64) -> serde_json::Value {
        serde_json::json!({
            "id": format!("{arch}_{diff}"),
            "archetype_id": arch,
            "difficulty_id": diff,
            "name": format!("{arch} - {diff}"),
            "display_name": format!("{arch} - {diff}"),
            "hp": 80.0,
            "hp_multiplier": 1.0,
            "damage_multiplier": 1.0,
            "aim_settle_ticks": 12,
            "reaction_time_ticks": 12,
            "miss_chance": 0.1,
            "sight_range": 320.0,
            "sight_fov_degrees": 180.0,
            "fov_degrees": 180.0,
            "hearing_radius": 480.0,
            "hearing_range": 480.0,
            "memory_decay_ticks": 300,
            "reload_ms": 1800,
            "retreat_hp_pct": retreat,
            "retreat_hp_threshold": retreat,
            "cover_seek_radius": cover,
            "squad_comm_delay_ticks": squad_delay,
        })
    }

    fn m7_full_registry() -> serde_json::Value {
        let archetypes = ["rifleman", "sniper", "assault", "engineer", "medic"];
        let difficulties = ["cakewalk", "tough_crowd", "veteran"];
        let mut presets = Vec::new();
        for a in &archetypes {
            for d in &difficulties {
                presets.push(m7_difficulty_entry(a, d, 0.30, 48.0, 30));
            }
        }
        serde_json::json!({ "schema": 1, "presets": presets })
    }

    #[test]
    fn m7_difficulty_json_accepts_15_archetype_difficulty_entries() {
        let body = m7_full_registry();
        let path = write_tmp("m7_difficulty_pass.json", &body.to_string());
        let mut report = ValidationReport::default();
        validate_difficulty_json(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.pass(), 1, "expected PASS, got entries: {:?}", report.entries);
        assert_eq!(report.fail(), 0);
        assert!(report.entries[0].message.contains("M7 extension shape"));
        assert!(report.entries[0].message.contains("15 presets"));
    }

    #[test]
    fn m7_difficulty_json_rejects_short_registry() {
        let mut body = m7_full_registry();
        let presets = body["presets"].as_array_mut().unwrap();
        presets.truncate(14);
        let path = write_tmp("m7_difficulty_short.json", &body.to_string());
        let mut report = ValidationReport::default();
        validate_difficulty_json(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("exactly 15"));
    }

    #[test]
    fn m7_difficulty_json_rejects_unknown_archetype() {
        let mut body = m7_full_registry();
        body["presets"][0]["archetype_id"] = serde_json::json!("paladin");
        let path = write_tmp("m7_difficulty_bad_archetype.json", &body.to_string());
        let mut report = ValidationReport::default();
        validate_difficulty_json(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("paladin"));
    }

    #[test]
    fn m7_difficulty_json_rejects_retreat_hp_threshold_out_of_range() {
        let mut body = m7_full_registry();
        body["presets"][0]["retreat_hp_threshold"] = serde_json::json!(0.95);
        let path = write_tmp("m7_difficulty_bad_retreat.json", &body.to_string());
        let mut report = ValidationReport::default();
        validate_difficulty_json(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("retreat_hp_threshold"));
        assert!(report.entries[0].message.contains("0.15..=0.50"));
    }

    #[test]
    fn m7_difficulty_json_rejects_missing_cover_seek_radius() {
        let mut body = m7_full_registry();
        body["presets"][0].as_object_mut().unwrap().remove("cover_seek_radius");
        let path = write_tmp("m7_difficulty_missing_cover.json", &body.to_string());
        let mut report = ValidationReport::default();
        validate_difficulty_json(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("cover_seek_radius missing"));
    }

    #[test]
    fn m7_difficulty_json_rejects_duplicate_pair() {
        let mut body = m7_full_registry();
        let arch = body["presets"][1]["archetype_id"].clone();
        let diff = body["presets"][1]["difficulty_id"].clone();
        body["presets"][0]["archetype_id"] = arch;
        body["presets"][0]["difficulty_id"] = diff;
        let path = write_tmp("m7_difficulty_dup_pair.json", &body.to_string());
        let mut report = ValidationReport::default();
        validate_difficulty_json(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("duplicate"));
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

    /// **M5**: an M5 envelope-shaped schema passes validation when every const
    /// + required field is in place.
    #[test]
    fn m5_event_schema_valid_envelope_passes() {
        let body = serde_json::json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "title": "armor.layer_destroyed",
            "type": "object",
            "properties": {
                "schema_version": { "const": "prototype-recorder-event.v0.1" },
                "category": { "const": "armor" },
                "event_type": { "const": "layer_destroyed" },
                "actor_id": { "type": "integer" },
                "tick": { "type": "integer" },
                "payload": {
                    "type": "object",
                    "properties": {
                        "item_id": { "type": "integer" },
                        "zone": { "type": "string" },
                        "layer": { "type": "string" },
                        "breach_kind": { "type": "string" }
                    },
                    "required": ["item_id", "zone", "layer", "breach_kind"]
                }
            },
            "required": ["schema_version", "category", "event_type", "tick", "payload"]
        });
        let dir = std::env::temp_dir().join(format!(
            "cf-mod-m5-{}-{}",
            std::process::id(),
            TMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        fs::create_dir_all(dir.join("event")).unwrap();
        let path = dir.join("event").join("armor_layer_destroyed.json");
        fs::write(&path, body.to_string()).unwrap();
        // Reparent so the path looks like .../schemas/event/<file>.json which
        // is_event_schema_file requires.
        let path_in_schemas = dir.join("schemas").join("event").join("armor_layer_destroyed.json");
        fs::create_dir_all(path_in_schemas.parent().unwrap()).unwrap();
        fs::write(&path_in_schemas, body.to_string()).unwrap();
        let mut report = ValidationReport::default();
        validate_event_schema_file(&path_in_schemas, &mut report);
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(report.pass(), 1, "expected PASS, got {:?}", report.entries);
        assert_eq!(report.fail(), 0);
    }

    /// **M5**: schema_version drift (≠ canonical envelope literal) is rejected.
    #[test]
    fn m5_event_schema_rejects_wrong_schema_version() {
        let body = serde_json::json!({
            "title": "armor.layer_destroyed",
            "type": "object",
            "properties": {
                "schema_version": { "const": "0.2" },
                "category": { "const": "armor" },
                "event_type": { "const": "layer_destroyed" },
                "tick": { "type": "integer" },
                "payload": { "type": "object" }
            },
            "required": ["schema_version", "category", "event_type", "tick", "payload"]
        });
        let path = PathBuf::from("/tmp/schemas/event/armor_layer_destroyed.json");
        let messages = validate_event_schema_value(&path, &body);
        assert!(
            messages
                .iter()
                .any(|m| m.contains("schema_version") && m.contains("prototype-recorder-event.v0.1")),
            "messages: {messages:?}"
        );
    }

    /// **M5**: the old M5 literal (`"0.1"`) is rejected — the canonical M4
    /// envelope literal is required.
    #[test]
    fn m5_event_schema_rejects_legacy_short_literal() {
        let body = serde_json::json!({
            "title": "armor.layer_destroyed",
            "type": "object",
            "properties": {
                "schema_version": { "const": "0.1" },
                "category": { "const": "armor" },
                "event_type": { "const": "layer_destroyed" },
                "tick": { "type": "integer" },
                "payload": { "type": "object" }
            },
            "required": ["schema_version", "category", "event_type", "tick", "payload"]
        });
        let path = PathBuf::from("/tmp/schemas/event/armor_layer_destroyed.json");
        let messages = validate_event_schema_value(&path, &body);
        assert!(
            messages.iter().any(|m| m.contains("schema_version")),
            "messages: {messages:?}"
        );
    }

    /// **M5**: filename / category-const drift is rejected.
    #[test]
    fn m5_event_schema_rejects_filename_drift() {
        let body = serde_json::json!({
            "title": "armor.layer_destroyed",
            "type": "object",
            "properties": {
                "schema_version": { "const": "prototype-recorder-event.v0.1" },
                "category": { "const": "internal" },
                "event_type": { "const": "layer_destroyed" },
                "tick": { "type": "integer" },
                "payload": { "type": "object" }
            },
            "required": ["schema_version", "category", "event_type", "tick", "payload"]
        });
        let path = PathBuf::from("/tmp/schemas/event/armor_layer_destroyed.json");
        let messages = validate_event_schema_value(&path, &body);
        assert!(
            messages.iter().any(|m| m.contains("filename")),
            "messages: {messages:?}"
        );
    }

    /// **M5**: missing payload sub-schema is rejected.
    #[test]
    fn m5_event_schema_rejects_missing_payload() {
        let body = serde_json::json!({
            "title": "armor.layer_destroyed",
            "type": "object",
            "properties": {
                "schema_version": { "const": "prototype-recorder-event.v0.1" },
                "category": { "const": "armor" },
                "event_type": { "const": "layer_destroyed" },
                "tick": { "type": "integer" }
            },
            "required": ["schema_version", "category", "event_type", "tick", "payload"]
        });
        let path = PathBuf::from("/tmp/schemas/event/armor_layer_destroyed.json");
        let messages = validate_event_schema_value(&path, &body);
        assert!(
            messages.iter().any(|m| m.contains("payload") && m.contains("defined")),
            "messages: {messages:?}"
        );
    }

    /// **M5**: top-level `required` missing one of the standard envelope
    /// fields is rejected.
    #[test]
    fn m5_event_schema_rejects_missing_required_envelope_fields() {
        let body = serde_json::json!({
            "title": "armor.layer_destroyed",
            "type": "object",
            "properties": {
                "schema_version": { "const": "prototype-recorder-event.v0.1" },
                "category": { "const": "armor" },
                "event_type": { "const": "layer_destroyed" },
                "tick": { "type": "integer" },
                "payload": { "type": "object" }
            },
            "required": ["schema_version", "category", "event_type", "payload"]
        });
        let path = PathBuf::from("/tmp/schemas/event/armor_layer_destroyed.json");
        let messages = validate_event_schema_value(&path, &body);
        assert!(messages.iter().any(|m| m.contains("tick")), "messages: {messages:?}");
    }

    /// **M5**: legacy payload-only schemas (no envelope wrap) still pass as
    /// long as they have either `type` or `properties`.
    #[test]
    fn m5_legacy_payload_only_schema_passes() {
        let body = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "snapshot_actor payload",
            "type": "object",
            "required": ["actor"],
            "properties": {
                "actor": { "type": "integer" }
            }
        });
        let path = PathBuf::from("/tmp/schemas/event/snapshot_actor.json");
        let messages = validate_event_schema_value(&path, &body);
        assert!(messages.is_empty(), "expected pass, got {messages:?}");
    }

    /// **M5**: every shipped M5 schema under cf-replay/schemas/event/ passes
    /// the validator end-to-end. This is the spec scenario:
    /// "cf-mod validate game/crates/cf-replay/schemas/ exits 0".
    #[test]
    fn m5_all_shipped_schemas_validate() {
        // Try a couple of likely roots so the test works no matter what CWD
        // cargo-test picks.
        let candidates = [
            PathBuf::from("../cf-replay/schemas"),
            PathBuf::from("../../cf-replay/schemas"),
            PathBuf::from("game/crates/cf-replay/schemas"),
            PathBuf::from("crates/cf-replay/schemas"),
        ];
        let Some(schemas_root) = candidates.iter().find(|p| p.exists()).cloned() else {
            // Test environment doesn't have the schemas dir reachable — skip
            // gracefully. The integration check still runs via cargo run
            // -p cf-mod -- validate.
            eprintln!("cf-replay/schemas not found relative to test CWD; skipping");
            return;
        };
        let mut report = ValidationReport::default();
        walk(&schemas_root, &mut report);
        assert_eq!(
            report.fail(),
            0,
            "every cf-replay/schemas/* schema must validate; failures: {:?}",
            report
                .entries
                .iter()
                .filter(|e| matches!(e.result, EntryResult::Fail))
                .map(|e| format!("{}: {}", e.path.display(), e.message))
                .collect::<Vec<_>>()
        );
        // Sanity: we should have looked at at least a few schemas.
        assert!(
            report.pass() > 50,
            "expected at least 50 schemas (got {})",
            report.pass()
        );
    }

    /// **M5-A1**: `additionalProperties: false` at payload level is REJECTED
    /// because it would break M4's additive-only contract.
    #[test]
    fn m5_event_schema_rejects_payload_additional_properties_false() {
        let body = serde_json::json!({
            "title": "armor.layer_destroyed",
            "type": "object",
            "properties": {
                "schema_version": { "const": "prototype-recorder-event.v0.1" },
                "category": { "const": "armor" },
                "event_type": { "const": "layer_destroyed" },
                "tick": { "type": "integer" },
                "payload": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": { "item_id": { "type": "integer" } },
                    "required": ["item_id"]
                }
            },
            "required": ["schema_version", "category", "event_type", "tick", "payload"]
        });
        let path = PathBuf::from("/tmp/schemas/event/armor_layer_destroyed.json");
        let messages = validate_event_schema_value(&path, &body);
        assert!(
            messages
                .iter()
                .any(|m| m.contains("additionalProperties") && m.contains("DR-002")),
            "messages: {messages:?}"
        );
    }

    /// **M5-A1**: envelope-version-dir regex accepts v0_1, v1, v0_2, v2_5.
    #[test]
    fn m5_envelope_version_dir_regex_accepts_canonical_forms() {
        assert!(is_envelope_version_dir("v0_1"));
        assert!(is_envelope_version_dir("v1"));
        assert!(is_envelope_version_dir("v0_2"));
        assert!(is_envelope_version_dir("v2_5"));
        assert!(is_envelope_version_dir("v10_42"));
    }

    /// **M5-A1**: envelope-version-dir regex rejects junk names.
    #[test]
    fn m5_envelope_version_dir_regex_rejects_bad_forms() {
        assert!(!is_envelope_version_dir("v"));
        assert!(!is_envelope_version_dir("v_1"));
        assert!(!is_envelope_version_dir("v1_"));
        assert!(!is_envelope_version_dir("v1_2_3"));
        assert!(!is_envelope_version_dir("event"));
        assert!(!is_envelope_version_dir("v0.1"));
        assert!(!is_envelope_version_dir("alpha"));
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

    #[test]
    fn validate_regen_manifest_accepts_well_formed() {
        let body = r#"(
            schema_version: "1.0.0",
            pipelines: [
                (
                    pipeline_id: "M9A_svg_v1",
                    owner_milestone: "M9A",
                    regen_command: "cf-tools-svg-gen --asset-id $ASSET_ID",
                    model_version: "llm:gpt-4o-mini@2026-05",
                    deterministic: true,
                    freeze_path_suffix: ".frozen",
                    notes: "ok",
                ),
            ],
        )"#;
        let path = write_tmp("regen_manifest.ron", body);
        let mut report = ValidationReport::default();
        validate_regen_manifest(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.pass(), 1, "expected PASS, got {:?}", report.entries);
        assert_eq!(report.fail(), 0);
    }

    #[test]
    fn validate_regen_manifest_accepts_missing_optional_fields() {
        let body = r#"(
            schema_version: "1.0.0",
            pipelines: [
                (
                    pipeline_id: "minimal_v1",
                    regen_command: "cf-tools-minimal",
                    model_version: "v1",
                    deterministic: false,
                ),
            ],
        )"#;
        let path = write_tmp("regen_manifest.ron", body);
        let mut report = ValidationReport::default();
        validate_regen_manifest(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.pass(), 1, "expected PASS, got {:?}", report.entries);
    }

    #[test]
    fn validate_regen_manifest_rejects_wrong_schema_version() {
        let body = r#"(
            schema_version: "2.0.0",
            pipelines: [
                (
                    pipeline_id: "x",
                    regen_command: "y",
                    model_version: "z",
                    deterministic: true,
                ),
            ],
        )"#;
        let path = write_tmp("regen_manifest.ron", body);
        let mut report = ValidationReport::default();
        validate_regen_manifest(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("schema_version"));
    }

    #[test]
    fn validate_regen_manifest_rejects_empty_pipelines() {
        let body = r#"(
            schema_version: "1.0.0",
            pipelines: [],
        )"#;
        let path = write_tmp("regen_manifest.ron", body);
        let mut report = ValidationReport::default();
        validate_regen_manifest(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("pipelines"));
    }

    #[test]
    fn validate_regen_manifest_rejects_empty_pipeline_id() {
        let body = r#"(
            schema_version: "1.0.0",
            pipelines: [
                (
                    pipeline_id: "",
                    regen_command: "y",
                    model_version: "z",
                    deterministic: true,
                ),
            ],
        )"#;
        let path = write_tmp("regen_manifest.ron", body);
        let mut report = ValidationReport::default();
        validate_regen_manifest(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("pipeline_id"));
    }

    #[test]
    fn validate_regen_manifest_rejects_malformed_ron() {
        let body = "this is not valid ron";
        let path = write_tmp("regen_manifest.ron", body);
        let mut report = ValidationReport::default();
        validate_regen_manifest(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("regen_manifest parse"));
    }

    #[test]
    fn weapon_registry_accepts_minimal_valid_input() {
        let body = r#"(
  schema_version: 1,
  weapons: [
    (id: "rifle_m1_default", class: "rifle"),
    (id: "smg_m6_default", class: "smg"),
  ],
)"#;
        let path = write_tmp("weapon_registry.ron", body);
        let mut report = ValidationReport::default();
        validate_weapon_registry(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.pass(), 1);
        assert_eq!(report.fail(), 0);
    }

    #[test]
    fn weapon_registry_rejects_empty_id() {
        let body = r#"(
  schema_version: 1,
  weapons: [
    (id: "", class: "rifle"),
  ],
)"#;
        let path = write_tmp("weapon_registry.ron", body);
        let mut report = ValidationReport::default();
        validate_weapon_registry(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
    }

    #[test]
    fn grenade_registry_accepts_minimal_valid_input() {
        let body = r#"(
  schema_version: 1,
  grenades: [
    (id: "grenade_frag_m6", kind: "frag"),
  ],
)"#;
        let path = write_tmp("grenade_registry.ron", body);
        let mut report = ValidationReport::default();
        validate_grenade_registry(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.pass(), 1);
        assert_eq!(report.fail(), 0);
    }

    #[test]
    fn grenade_registry_rejects_empty_registry() {
        let body = r#"(
  schema_version: 1,
  grenades: [],
)"#;
        let path = write_tmp("grenade_registry.ron", body);
        let mut report = ValidationReport::default();
        validate_grenade_registry(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("at least 1 entry"));
    }

    #[test]
    fn melee_registry_accepts_minimal_valid_input() {
        let body = r#"(
  schema_version: 1,
  melees: [
    (id: "melee_knife_m6", kind: "knife"),
  ],
)"#;
        let path = write_tmp("melee_registry.ron", body);
        let mut report = ValidationReport::default();
        validate_melee_registry(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.pass(), 1);
        assert_eq!(report.fail(), 0);
    }

    #[test]
    fn melee_registry_rejects_bad_schema_version() {
        let body = r#"(
  schema_version: 2,
  melees: [
    (id: "melee_knife_m6", kind: "knife"),
  ],
)"#;
        let path = write_tmp("melee_registry.ron", body);
        let mut report = ValidationReport::default();
        validate_melee_registry(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("schema_version"));
    }

    #[test]
    fn tool_registry_accepts_minimal_valid_input() {
        let body = r#"(
  schema_version: 1,
  tools: [
    (id: "tool_repair_m6", kind: "repair"),
  ],
)"#;
        let path = write_tmp("tool_registry.ron", body);
        let mut report = ValidationReport::default();
        validate_tool_registry(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.pass(), 1);
        assert_eq!(report.fail(), 0);
    }

    #[test]
    fn tool_registry_rejects_empty_kind() {
        let body = r#"(
  schema_version: 1,
  tools: [
    (id: "tool_repair_m6", kind: ""),
  ],
)"#;
        let path = write_tmp("tool_registry.ron", body);
        let mut report = ValidationReport::default();
        validate_tool_registry(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
    }

    #[test]
    fn item_manifest_accepts_minimal_synced_manifest() {
        // **M6B**: a manifest that lists every registered id with the
        // canonical category must pass.
        let registry_ids = cf_equipment::item_registered_ids();
        let mut items = String::new();
        for id in &registry_ids {
            let spec = cf_equipment::spec_for_id(id).unwrap();
            items.push_str(&format!("(id: \"{}\", category: \"{}\"),\n", id, spec.category.as_str()));
        }
        let body = format!("(schema_version: 1, items: [{items}])");
        let path = write_tmp("manifest.ron", &body);
        let mut report = ValidationReport::default();
        validate_item_manifest(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.pass(), 1, "report: {:?}", report.entries);
        assert_eq!(report.fail(), 0);
    }

    #[test]
    fn item_manifest_rejects_unknown_id() {
        let body = r#"(
  schema_version: 1,
  items: [
    (id: "rifle_m1", category: "weapon"),
    (id: "this_id_does_not_exist", category: "weapon"),
  ],
)"#;
        let path = write_tmp("manifest.ron", body);
        let mut report = ValidationReport::default();
        validate_item_manifest(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("not registered"));
    }

    #[test]
    fn item_manifest_rejects_category_drift() {
        let body = r#"(
  schema_version: 1,
  items: [
    (id: "rifle_m1", category: "consumable"),
  ],
)"#;
        let path = write_tmp("manifest.ron", body);
        let mut report = ValidationReport::default();
        validate_item_manifest(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("category mismatch"));
    }

    #[test]
    fn item_manifest_rejects_missing_registered_id() {
        // Drift detection — when the manifest omits a registered id,
        // validation fails with "mirror drift".
        let body = r#"(
  schema_version: 1,
  items: [
    (id: "rifle_m1", category: "weapon"),
  ],
)"#;
        let path = write_tmp("manifest.ron", body);
        let mut report = ValidationReport::default();
        validate_item_manifest(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("mirror drift"));
    }

    #[test]
    fn item_manifest_rejects_bad_schema_version() {
        let body = r#"(
  schema_version: 2,
  items: [
    (id: "rifle_m1", category: "weapon"),
  ],
)"#;
        let path = write_tmp("manifest.ron", body);
        let mut report = ValidationReport::default();
        validate_item_manifest(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
    }

    #[test]
    fn item_spec_ron_accepts_registered_id() {
        // Standalone item_spec ron file mirroring a registered id.
        let body = r#"(
  id: "rifle_m1",
  display_name: "Rifle (M1)",
  mass_kg: 3.5,
  dimensions: (w: 2, h: 4),
  bulk_volume_l: 3.0,
  stackable: false,
  max_stack: 1,
  category: weapon,
  container_capacity: None,
  liquid_capacity_l: None,
  rotation_allowed: true,
  quick_slot_eligible: true,
  durability_max: Some(1000),
  repair_recipe: Some("repair.rifle_m1"),
  material_weight_breakdown: {},
  crafting_yield_count: 1,
  origin_compatibility: [],
  forbid_for_origin: [],
)"#;
        let path = write_tmp("rifle_m1.ron", body);
        let mut report = ValidationReport::default();
        validate_item_spec_ron(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.pass(), 1, "report: {:?}", report.entries);
        assert_eq!(report.fail(), 0);
    }

    #[test]
    fn item_spec_ron_rejects_unknown_id() {
        let body = r#"(
  id: "made_up_item",
  display_name: "Made Up",
  mass_kg: 1.0,
  dimensions: (w: 1, h: 1),
  bulk_volume_l: 1.0,
  stackable: false,
  max_stack: 1,
  category: weapon,
  container_capacity: None,
  liquid_capacity_l: None,
  rotation_allowed: true,
  quick_slot_eligible: false,
  durability_max: None,
  repair_recipe: None,
  material_weight_breakdown: {},
  crafting_yield_count: 1,
  origin_compatibility: [],
  forbid_for_origin: [],
)"#;
        let path = write_tmp("made_up_item.ron", body);
        let mut report = ValidationReport::default();
        validate_item_spec_ron(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("not registered"));
    }

    #[test]
    fn item_spec_ron_rejects_filename_mismatch() {
        let body = r#"(
  id: "rifle_m1",
  display_name: "Rifle (M1)",
  mass_kg: 3.5,
  dimensions: (w: 2, h: 4),
  bulk_volume_l: 3.0,
  stackable: false,
  max_stack: 1,
  category: weapon,
  container_capacity: None,
  liquid_capacity_l: None,
  rotation_allowed: true,
  quick_slot_eligible: true,
  durability_max: Some(1000),
  repair_recipe: None,
  material_weight_breakdown: {},
  crafting_yield_count: 1,
  origin_compatibility: [],
  forbid_for_origin: [],
)"#;
        let path = write_tmp("wrong_filename.ron", body);
        let mut report = ValidationReport::default();
        validate_item_spec_ron(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("mismatches filename"));
    }

    #[test]
    fn item_spec_ron_rejects_zero_dimensions() {
        let body = r#"(
  id: "rifle_m1",
  display_name: "Rifle (M1)",
  mass_kg: 3.5,
  dimensions: (w: 0, h: 4),
  bulk_volume_l: 3.0,
  stackable: false,
  max_stack: 1,
  category: weapon,
  container_capacity: None,
  liquid_capacity_l: None,
  rotation_allowed: true,
  quick_slot_eligible: true,
  durability_max: None,
  repair_recipe: None,
  material_weight_breakdown: {},
  crafting_yield_count: 1,
  origin_compatibility: [],
  forbid_for_origin: [],
)"#;
        let path = write_tmp("rifle_m1.ron", body);
        let mut report = ValidationReport::default();
        validate_item_spec_ron(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("dimensions"));
    }

    // ---- M9B-2 trench segment / module / template validators ----

    fn write_named(name: &str, contents: &str) -> PathBuf {
        write_tmp(name, contents)
    }

    /// VAL-M9B-MOD-SEGMENT-001 — a hand-crafted segment RON with a
    /// malformed enum value (`cover_state.standing: "ultra_full"`)
    /// fails validation with a typed error and no panic occurs.
    #[test]
    fn trench_segment_unknown_variant_rejected() {
        let bad = r#"(
            variant: ultra_deep,
            depth: 16,
            width: 16,
            raised_step_height: None,
            embedded_modules: [],
            cover_state: (standing: Partial, crouched: Full, prone: Full),
        )"#;
        let path = write_named("ultra_deep.ron", bad);
        let mut report = ValidationReport::default();
        validate_trench_segment(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1, "expected one FAIL entry; got {:?}", report.entries);
        let msg = &report.entries[0].message;
        assert!(
            msg.contains("ultra_deep") || msg.to_lowercase().contains("unknown") || msg.to_lowercase().contains("variant"),
            "expected unknown-variant message; got: {msg}"
        );
    }

    /// VAL-M9B-MOD-SEGMENT-001 — malformed enum value for cover_state
    /// rejects with typed error.
    #[test]
    fn trench_segment_malformed_rejected() {
        let bad = r#"(
            variant: standard,
            depth: 16,
            width: 16,
            raised_step_height: None,
            embedded_modules: [],
            cover_state: (standing: Bananas, crouched: Full, prone: Full),
        )"#;
        let path = write_named("standard.ron", bad);
        let mut report = ValidationReport::default();
        validate_trench_segment(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
    }

    /// VAL-M9B-MOD-SEGMENT-001 — depth field is `u32`; a negative
    /// literal in RON produces a typed parse error rather than a
    /// silent acceptance.
    #[test]
    fn trench_segment_negative_depth_rejected() {
        let bad = r#"(
            variant: standard,
            depth: -3,
            width: 16,
            raised_step_height: None,
            embedded_modules: [],
            cover_state: (standing: Partial, crouched: Full, prone: Full),
        )"#;
        let path = write_named("standard.ron", bad);
        let mut report = ValidationReport::default();
        validate_trench_segment(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1, "report: {:?}", report.entries);
        let msg = &report.entries[0].message;
        assert!(
            msg.to_lowercase().contains("expected") || msg.to_lowercase().contains("negative") || msg.contains("ron"),
            "expected parse-error containing field hint; got: {msg}"
        );
    }

    /// Well-formed segment RON passes.
    #[test]
    fn trench_segment_well_formed_passes() {
        let good = r#"(
            variant: standard,
            depth: 16,
            width: 16,
            raised_step_height: None,
            embedded_modules: [duckboard],
            cover_state: (standing: Partial, crouched: Full, prone: Full),
        )"#;
        let path = write_named("standard.ron", good);
        let mut report = ValidationReport::default();
        validate_trench_segment(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.pass(), 1, "report: {:?}", report.entries);
    }

    /// VAL-M9B-MOD-SEGMENT-001 (module surface) — malformed module
    /// RON rejects with typed error.
    #[test]
    fn trench_module_malformed_rejected() {
        let bad = r#"(
            module: not_a_real_module,
            material_cost: {"wood": 2},
            build_time_seconds: 4,
        )"#;
        let path = write_named("not_a_real_module.ron", bad);
        let mut report = ValidationReport::default();
        validate_trench_module(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
    }

    /// VAL-M9B-TEMPLATE-003 — unknown segment variant in a template
    /// RON fails validation with a message including the bad value.
    #[test]
    fn trench_template_unknown_variant_rejected() {
        let bad = r#"(
            id: "bad_template",
            display_name: "Bad",
            faction: None,
            doctrine_hint: None,
            recommended_garrison: None,
            footprint: (min_x: 0, min_y: 0, max_x: 10, max_y: 10),
            path_polyline: [(0,0), (5,0)],
            segment_overrides: [],
            default_variant: ultra_deep,
            fortification_placeholders: [],
            zones: [],
        )"#;
        let path = write_named("bad_template.trench.ron", bad);
        let mut report = ValidationReport::default();
        validate_trench_template(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1, "report: {:?}", report.entries);
        let msg = &report.entries[0].message;
        assert!(
            msg.contains("unknown_segment_variant")
                || msg.to_lowercase().contains("unknown variant"),
            "expected unknown_segment_variant in message: {msg}"
        );
    }

    /// VAL-M9B-TEMPLATE-003 — unknown fortification id in a template
    /// rejects with a typed error (KNOWN_FORTIFICATION_IDS gate).
    #[test]
    fn trench_template_unknown_fortification_rejected() {
        let bad = r#"(
            id: "ub",
            display_name: "Unknown bunker",
            faction: None,
            doctrine_hint: None,
            recommended_garrison: None,
            footprint: (min_x: 0, min_y: 0, max_x: 10, max_y: 10),
            path_polyline: [(0,0), (5,0)],
            segment_overrides: [],
            default_variant: standard,
            fortification_placeholders: [
                (fortification_id: "wat_is_this", offset: (0,0), optional: true),
            ],
            zones: [],
        )"#;
        let path = write_named("ub.trench.ron", bad);
        let mut report = ValidationReport::default();
        validate_trench_template(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1, "report: {:?}", report.entries);
    }

    /// VAL-M9B-TEMPLATE-004 — an optional placeholder for a not-yet-
    /// shipped M9C asset emits a WARN entry mentioning the
    /// `trench_template_missing_fortification` event label (no FAIL).
    #[test]
    fn trench_template_missing_fortification_warning() {
        let body = r#"(
            id: "outpost_demo",
            display_name: "Demo",
            faction: None,
            doctrine_hint: None,
            recommended_garrison: None,
            footprint: (min_x: 0, min_y: 0, max_x: 10, max_y: 10),
            path_polyline: [(0,0), (5,0)],
            segment_overrides: [],
            default_variant: standard,
            fortification_placeholders: [
                (fortification_id: "mg_nest_static", offset: (1,1), optional: true),
            ],
            zones: [],
        )"#;
        let path = write_named("outpost_demo.trench.ron", body);
        let mut report = ValidationReport::default();
        validate_trench_template(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 0, "report: {:?}", report.entries);
        assert!(report.warn() >= 1, "expected ≥1 WARN entry; report: {:?}", report.entries);
        assert!(report.entries.iter().any(|e| {
            e.message.contains("trench_template_missing_fortification")
                || e.message.contains("mg_nest_static")
        }));
    }

    /// Filename-stem mismatch must fail (cf-mod surface for the
    /// canonical id↔filename parity contract).
    #[test]
    fn trench_template_id_filename_mismatch_rejected() {
        let body = r#"(
            id: "abc",
            display_name: "ABC",
            faction: None,
            doctrine_hint: None,
            recommended_garrison: None,
            footprint: (min_x: 0, min_y: 0, max_x: 10, max_y: 10),
            path_polyline: [(0,0), (5,0)],
            segment_overrides: [],
            default_variant: standard,
            fortification_placeholders: [],
            zones: [],
        )"#;
        let path = write_named("xyz.trench.ron", body);
        let mut report = ValidationReport::default();
        validate_trench_template(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1);
        assert!(report.entries[0].message.contains("mismatches filename"));
    }

    // ---- M9C-1 fortification validators ----

    /// VAL-M9C-005: every authored `content/fortifications/*.ron`
    /// file loads cleanly through `validate_fortification`. The fast
    /// way to assert this is to walk the manifest dir + validate each.
    #[test]
    fn fortifications_load_all() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("content")
            .join("fortifications");
        let entries = fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()));
        let mut report = ValidationReport::default();
        let mut count = 0usize;
        for entry in entries {
            let entry = entry.expect("readdir entry");
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("ron") {
                continue;
            }
            count += 1;
            validate_fortification(&path, &mut report);
        }
        assert!(
            count >= 23,
            "expected ≥23 fortification RONs on disk; found {count}"
        );
        assert_eq!(
            report.fail(),
            0,
            "expected zero FAIL entries; report: {:?}",
            report.entries
        );
        assert!(
            report.pass() >= 23,
            "expected ≥23 PASS entries; report: {:?}",
            report.entries
        );
    }

    /// VAL-M9C-007: a fortification RON with an unknown
    /// FortificationKind enum value is rejected at parse time.
    #[test]
    fn fortifications_reject_malformed_enum() {
        let bad = r#"(
            kind: definitely_not_a_kind,
            hp: 100,
            footprint_tiles: (1, 1),
            build_time_seconds: 1,
            material_cost: {},
        )"#;
        let path = write_named("definitely_not_a_kind.ron", bad);
        let mut report = ValidationReport::default();
        validate_fortification(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1, "report: {:?}", report.entries);
    }

    /// VAL-M9C-007 (wire_kind dimension): a fortification RON with an
    /// unknown wire_kind enum value is rejected at parse time.
    #[test]
    fn fortifications_reject_malformed_wire_kind_enum() {
        let bad = r#"(
            kind: barbed_wire,
            hp: 200,
            footprint_tiles: (3, 1),
            build_time_seconds: 4,
            material_cost: {"iron": 4, "wire": 1},
            wire_kind: Some(not_a_wire_kind),
        )"#;
        let path = write_named("barbed_wire.ron", bad);
        let mut report = ValidationReport::default();
        validate_fortification(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1, "report: {:?}", report.entries);
    }

    /// VAL-M9C-007 (mine_kind dimension): malformed mine_kind enum
    /// rejected.
    #[test]
    fn fortifications_reject_malformed_mine_kind_enum() {
        let bad = r#"(
            kind: barbed_wire,
            hp: 200,
            footprint_tiles: (3, 1),
            build_time_seconds: 4,
            material_cost: {"iron": 4, "wire": 1},
            mine_kind: Some(not_a_mine_kind),
        )"#;
        let path = write_named("barbed_wire.ron", bad);
        let mut report = ValidationReport::default();
        validate_fortification(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1, "report: {:?}", report.entries);
    }

    /// VAL-M9C-MOD-MISSING-DEPENDENCY: a fortification RON with a
    /// `depends_on` entry that names an unknown dependency emits a
    /// WARN entry (NOT a FAIL) so the asset still loads in degraded
    /// mode.
    #[test]
    fn fortifications_missing_dependency_warning() {
        let body = r#"(
            kind: camo_netting,
            hp: 100,
            footprint_tiles: (4, 4),
            build_time_seconds: 12,
            material_cost: {"burlap": 4, "twine": 2},
            depends_on: ["m99_future_milestone_thing"],
        )"#;
        let path = write_named("camo_netting.ron", body);
        let mut report = ValidationReport::default();
        validate_fortification(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(
            report.fail(),
            0,
            "missing dependency must NOT fail; report: {:?}",
            report.entries
        );
        assert!(
            report.warn() >= 1,
            "expected ≥1 WARN entry; report: {:?}",
            report.entries
        );
        assert!(
            report.entries.iter().any(|e| {
                e.message.contains(FORTIFICATION_MISSING_DEPENDENCY_WARNING)
                    && e.message.contains("m99_future_milestone_thing")
            }),
            "WARN entry must mention the missing dependency: {:?}",
            report.entries
        );
    }

    /// Filename-stem mismatch (e.g. `sandbag_low.ron` declaring
    /// `kind: sandbag_high`) MUST fail.
    #[test]
    fn fortifications_reject_filename_kind_mismatch() {
        let body = r#"(
            kind: sandbag_high,
            hp: 600,
            footprint_tiles: (3, 1),
            build_time_seconds: 12,
            material_cost: {"sandbag": 12},
            cover_state: Some((standing: Full, crouched: Full, prone: Full)),
            sandbag_tier: Some(high),
        )"#;
        let path = write_named("sandbag_low.ron", body);
        let mut report = ValidationReport::default();
        validate_fortification(&path, &mut report);
        let _ = fs::remove_file(&path);
        assert_eq!(report.fail(), 1, "report: {:?}", report.entries);
        assert!(
            report.entries[0].message.contains("mismatches filename"),
            "expected mismatch message; got: {}",
            report.entries[0].message
        );
    }
}

// ============================================================================
// **M14A** § "cf-mod (EXTEND)" — validators for limb_paths / jetpacks /
// quick_action_layouts.
// ============================================================================

/// M14A: validate `content/limb_paths/*.ron`.
fn validate_m14a_limb_path(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    #[derive(serde::Deserialize)]
    #[allow(dead_code)]
    struct Spec {
        schema_version: u32,
        chassis_archetype: String,
        move_state: String,
        side: String,
        start: (f32, f32),
        segments: Vec<(f32, f32)>,
        travel_speed: Vec<f32>,
        travel_speed_multiplier: f32,
        push_force: f32,
        foot_collisions_disabled_segment: i32,
    }
    match ron::from_str::<Spec>(&raw) {
        Ok(s) => {
            if s.schema_version != 1 {
                report.add_error(
                    path.to_path_buf(),
                    format!("limb_path schema_version must be 1, got {}", s.schema_version),
                );
                return;
            }
            if s.segments.is_empty() {
                report.add_error(path.to_path_buf(), "limb_path must have ≥1 segment".to_string());
                return;
            }
            if s.push_force <= 0.0 {
                report.add_error(path.to_path_buf(), "limb_path push_force must be > 0".to_string());
                return;
            }
            if !matches!(
                s.move_state.as_str(),
                "no_move"
                    | "stand"
                    | "walk"
                    | "crouch"
                    | "crawl"
                    | "arm_crawl"
                    | "climb"
                    | "jump"
                    | "dislodge"
                    | "hover"
            ) {
                report.add_error(
                    path.to_path_buf(),
                    format!("limb_path unknown move_state: {}", s.move_state),
                );
                return;
            }
            report.add_pass(
                path.to_path_buf(),
                format!(
                    "limb_path ({} {} {} segs={})",
                    s.chassis_archetype,
                    s.move_state,
                    s.side,
                    s.segments.len()
                ),
            );
        }
        Err(err) => report.add_error(path.to_path_buf(), format!("ron parse failed: {err}")),
    }
}

/// M14A: validate `content/jetpacks/*.ron`.
fn validate_m14a_jetpack(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    #[derive(serde::Deserialize)]
    #[allow(dead_code)]
    struct Spec {
        schema_version: u32,
        id: String,
        jetpack_type: String,
        jet_time_total_ms: u32,
        jet_replenish_rate: f32,
        minimum_fuel_ratio: f32,
        jet_angle_range: f32,
        can_adjust_angle_while_firing: bool,
        adjusts_throttle_for_weight: bool,
        base_thrust_n: f32,
        burst_thrust_multiplier: f32,
        dry_mass_kg: f32,
        fuel_density_kg_per_ms: f32,
        bound_zone: String,
        emitter_offset: (f32, f32),
    }
    match ron::from_str::<Spec>(&raw) {
        Ok(s) => {
            if s.schema_version != 1 {
                report.add_error(path.to_path_buf(), "jetpack schema_version must be 1".to_string());
                return;
            }
            if !matches!(s.jetpack_type.as_str(), "standard" | "jump_pack") {
                report.add_error(
                    path.to_path_buf(),
                    format!("unknown jetpack_type: {}", s.jetpack_type),
                );
                return;
            }
            if s.minimum_fuel_ratio < 0.0 || s.minimum_fuel_ratio > 1.0 {
                report.add_error(path.to_path_buf(), "minimum_fuel_ratio out of [0,1]".to_string());
                return;
            }
            if s.base_thrust_n <= 0.0 {
                report.add_error(path.to_path_buf(), "base_thrust_n must be > 0".to_string());
                return;
            }
            report.add_pass(
                path.to_path_buf(),
                format!(
                    "jetpack ({} type={} thrust={}N)",
                    s.id, s.jetpack_type, s.base_thrust_n
                ),
            );
        }
        Err(err) => report.add_error(path.to_path_buf(), format!("ron parse failed: {err}")),
    }
}

/// M14A: validate `content/quick_action_layouts/*.ron`.
fn validate_m14a_quick_action_layout(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    #[derive(serde::Deserialize)]
    #[allow(dead_code)]
    struct SlotSpec {
        slot: u8,
        kind: String,
        item_id: String,
        ammo: u32,
        ammo_max: u32,
        cooldown_total_ticks: u32,
    }
    #[derive(serde::Deserialize)]
    #[allow(dead_code)]
    struct Spec {
        schema_version: u32,
        chassis_archetype: String,
        slots: Vec<SlotSpec>,
    }
    match ron::from_str::<Spec>(&raw) {
        Ok(s) => {
            if s.schema_version != 1 {
                report.add_error(
                    path.to_path_buf(),
                    "quick_action_layout schema_version must be 1".to_string(),
                );
                return;
            }
            if s.slots.len() != 8 {
                report.add_error(
                    path.to_path_buf(),
                    format!("quick_action_layout must define 8 slots, got {}", s.slots.len()),
                );
                return;
            }
            for slot in &s.slots {
                if !matches!(
                    slot.kind.as_str(),
                    "empty" | "weapon" | "melee" | "grenade" | "consumable" | "ability" | "tool"
                ) {
                    report.add_error(
                        path.to_path_buf(),
                        format!("quick_action_layout unknown slot kind: {}", slot.kind),
                    );
                    return;
                }
            }
            report.add_pass(
                path.to_path_buf(),
                format!("quick_action_layout ({}, 8 slots)", s.chassis_archetype),
            );
        }
        Err(err) => report.add_error(path.to_path_buf(), format!("ron parse failed: {err}")),
    }
}
