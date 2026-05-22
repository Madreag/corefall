use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::bundle_chain_verify;
use crate::cli::{AssetGenAction, AudioGenAction, LedgerAction, SaveAction};
use crate::report::{EntryResult, ValidationReport};
use crate::save_validate;
use crate::validate::{validate_one, walk};

pub(crate) fn run_save(action: &SaveAction, json_output: bool) -> Result<()> {
    match action {
        SaveAction::Validate { path } => save_validate::run(path, json_output),
    }
}

pub(crate) fn run_audio_gen(action: &AudioGenAction, json_output: bool) -> Result<()> {
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

pub(crate) fn run_asset_gen(action: &AssetGenAction, json_output: bool) -> Result<()> {
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
pub(crate) fn run_ledger(action: &LedgerAction, global_strict: bool, json_output: bool) -> Result<()> {
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
            if let Some(bundle_dir) = bundle {
                return bundle_chain_verify::run(bundle_dir, json_output);
            }
            let paths = ledger_paths(ledger_path.as_ref());
            let target = if *all { None } else { id.as_deref() };
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

/// `cf_replay::schemas::validate_event_payload`. Returns non-zero exit on
/// any schema violation. Outputs JSON when `--json` is set so CI can parse
/// the report.
pub(crate) fn run_validate_bundle(bundle_dir: &std::path::Path, json_output: bool) -> Result<()> {
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

pub(crate) fn run_validate(paths: &[PathBuf], strict: bool, json_output: bool) -> Result<()> {
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
