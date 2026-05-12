use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
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
const STRICT_FAIL_CONTENT_CATEGORIES: &[(&str, &str)] = &[
    ("materials", "M5.6"),
    ("chassis", "M5"),
    ("atmospheres", "M5.9 / M7.5"),
    ("worlds", "M5.10"),
    ("origins", "M5"),
];

fn validate_one(path: &Path, report: &mut ValidationReport) {
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
    let path_components: Vec<String> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
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

    fn write_tmp(name: &str, contents: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("cf-mod-test-{name}"));
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
}
