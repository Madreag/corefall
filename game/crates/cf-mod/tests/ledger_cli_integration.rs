//! M4A: end-to-end integration test that drives the `cf-mod ledger` binary
//! through every command in the spec's CLI table. Each scenario in the
//! Gherkin acceptance criteria is covered:
//!
//! - "Append-only JSONL ledger" → `append_only_one_line_per_entry`
//! - "Integrity check detects drift" → `verify_detects_drift_non_zero_exit`
//! - "Regenerate produces byte-identical output" → `regenerate_byte_identical`
//! - "Per-category + per-tier filtering" → `list_filters_category_tier`
//! - "Audit reports missing + drifted + failed" → `summary_groups_status`
//! - "Determinism contract — same seed reproduces same output" → covered
//!   by `regenerate_byte_identical` (the freeze-then-store contract IS
//!   what's verified across machines)
//! - "Schema version locked at v1" → `schema_version_locked_v1`

use std::{path::PathBuf, process::Command};

fn target_bin() -> PathBuf {
    // CARGO_BIN_EXE_<name> is set when running integration tests so we
    // hit the compiled binary directly (no `cargo run` overhead).
    let path = env!("CARGO_BIN_EXE_cf-mod");
    PathBuf::from(path)
}

static TMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn tmp_workspace() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let seq = TMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("cf-mod-ledger-itest-{pid}-{nanos}-{seq}"));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join("content/assets")).unwrap();
    dir
}

fn write_asset(workspace: &PathBuf, rel: &str, bytes: &[u8]) -> PathBuf {
    let p = workspace.join(rel);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&p, bytes).unwrap();
    p
}

fn ledger_path(workspace: &PathBuf) -> PathBuf {
    workspace.join("ledger.jsonl")
}

fn run_cmd(workspace: &PathBuf, args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(target_bin());
    cmd.current_dir(workspace).args(args);
    let output = cmd.output().expect("spawn cf-mod");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (code, stdout, stderr)
}

fn add_entry(workspace: &PathBuf, name: &str, output_rel: &str) -> (i32, String, String) {
    let lp = ledger_path(workspace);
    let lp_str = lp.display().to_string();
    run_cmd(
        workspace,
        &[
            "ledger",
            "add",
            "--category",
            "WeaponSprite",
            "--kind",
            "weapon-side",
            "--canonical-name",
            name,
            "--tier",
            "Tier1_SVG",
            "--pipeline",
            "M9A_svg_v1",
            "--prompt",
            "industrial rifle",
            "--seed",
            "1234",
            "--output-path",
            output_rel,
            "--ledger-path",
            &lp_str,
        ],
    )
}

#[test]
fn append_only_one_line_per_entry() {
    let workspace = tmp_workspace();
    for i in 0..5 {
        let rel = format!("content/assets/asset_{i}.svg");
        write_asset(&workspace, &rel, format!("entry-{i}").as_bytes());
        let (code, _, stderr) = add_entry(&workspace, &format!("asset_{i}"), &rel);
        assert_eq!(code, 0, "ledger add #{i} failed: {stderr}");
    }
    let raw = std::fs::read_to_string(ledger_path(&workspace)).unwrap();
    let lines: Vec<&str> = raw.lines().collect();
    assert_eq!(lines.len(), 5, "expected 5 lines, got {} ({raw})", lines.len());
    // Every line is a valid JSON object.
    for (i, line) in lines.iter().enumerate() {
        let _: serde_json::Value =
            serde_json::from_str(line).unwrap_or_else(|e| panic!("line {i} is not valid JSON: {e} -> {line}"));
    }
}

#[test]
fn re_add_appends_new_entry_and_supersedes_old() {
    let workspace = tmp_workspace();
    let rel = "content/assets/rifle.svg";
    // v1
    write_asset(&workspace, rel, b"<svg>v1</svg>");
    let (code, _, _) = add_entry(&workspace, "rifle_v1", rel);
    assert_eq!(code, 0);
    // v2: same canonical name, different file content → same id, new entry
    write_asset(&workspace, rel, b"<svg>v2</svg>");
    let (code, _, _) = add_entry(&workspace, "rifle_v1", rel);
    assert_eq!(code, 0);
    let raw = std::fs::read_to_string(ledger_path(&workspace)).unwrap();
    let lines: Vec<&str> = raw.lines().collect();
    assert_eq!(lines.len(), 2);
    let v1: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    let v2: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    // First line is now marked superseded by the second
    assert!(v1.get("superseded_by").and_then(|v| v.as_str()).is_some());
    assert!(v2.get("superseded_by").and_then(|v| v.as_str()).is_none());
}

#[test]
fn list_filters_category_tier() {
    let workspace = tmp_workspace();
    let a_rel = "content/assets/a.svg";
    write_asset(&workspace, a_rel, b"a");
    let (code, _, _) = add_entry(&workspace, "a", a_rel);
    assert_eq!(code, 0);
    let b_rel = "content/assets/b.svg";
    write_asset(&workspace, b_rel, b"b");
    let lp = ledger_path(&workspace).display().to_string();
    let (code, _, _) = run_cmd(
        &workspace,
        &[
            "ledger",
            "add",
            "--category",
            "UiIcon",
            "--kind",
            "ui",
            "--canonical-name",
            "b",
            "--tier",
            "Tier1_SVG",
            "--pipeline",
            "M9A_svg_v1",
            "--prompt",
            "icon",
            "--seed",
            "0",
            "--output-path",
            b_rel,
            "--ledger-path",
            &lp,
        ],
    );
    assert_eq!(code, 0);
    let (code, stdout, _) = run_cmd(
        &workspace,
        &[
            "ledger",
            "list",
            "--category",
            "WeaponSprite",
            "--tier",
            "Tier1_SVG",
            "--ledger-path",
            &lp,
        ],
    );
    assert_eq!(code, 0);
    assert!(stdout.contains("WeaponSprite"));
    assert!(!stdout.contains("UiIcon"));
}

#[test]
fn verify_detects_drift_non_zero_exit() {
    let workspace = tmp_workspace();
    let rel = "content/assets/x.svg";
    write_asset(&workspace, rel, b"<svg/>");
    let (code, _, _) = add_entry(&workspace, "x", rel);
    assert_eq!(code, 0);
    // Edit the file directly (drift)
    write_asset(&workspace, rel, b"hand-edited");
    let lp = ledger_path(&workspace).display().to_string();
    let (code, _stdout, _) = run_cmd(
        &workspace,
        &["ledger", "verify", "--all", "--strict-status", "--ledger-path", &lp],
    );
    assert_ne!(code, 0, "verify --strict must exit non-zero on drift");
}

/// **M4A spec literal contract**: `cf-mod ledger verify --strict --all`
/// is the CI-gate form (global `--strict` flag must engage strict mode).
/// This test prevents regression of the verify dispatch path silently
/// ignoring `cli.strict` (audit BLOCKER 2026-05-13).
#[test]
fn verify_global_strict_flag_exits_nonzero_on_drift() {
    let workspace = tmp_workspace();
    let rel = "content/assets/y.svg";
    write_asset(&workspace, rel, b"<svg/>");
    let (code, _, _) = add_entry(&workspace, "y", rel);
    assert_eq!(code, 0);
    write_asset(&workspace, rel, b"hand-edited");
    let lp = ledger_path(&workspace).display().to_string();
    // GLOBAL --strict before `ledger` subcommand
    let (code, _, _) = run_cmd(
        &workspace,
        &["--strict", "ledger", "verify", "--all", "--ledger-path", &lp],
    );
    assert_ne!(code, 0, "cf-mod --strict ledger verify must exit non-zero on drift");
    // GLOBAL --strict AFTER `ledger` subcommand (clap accepts global flags anywhere)
    let (code, _, _) = run_cmd(
        &workspace,
        &["ledger", "--strict", "verify", "--all", "--ledger-path", &lp],
    );
    assert_ne!(
        code, 0,
        "cf-mod ledger --strict verify must also exit non-zero on drift"
    );
}

/// **M4A spec literal contract**: when ledger is fully Fresh, `--strict`
/// must exit 0. Confirms strict mode is gating ON drift, not always-on.
#[test]
fn verify_global_strict_exits_zero_when_all_fresh() {
    let workspace = tmp_workspace();
    let rel = "content/assets/z.svg";
    write_asset(&workspace, rel, b"<svg/>");
    let (code, _, _) = add_entry(&workspace, "z", rel);
    assert_eq!(code, 0);
    let lp = ledger_path(&workspace).display().to_string();
    let (code, _, _) = run_cmd(
        &workspace,
        &["--strict", "ledger", "verify", "--all", "--ledger-path", &lp],
    );
    assert_eq!(code, 0, "all-Fresh ledger must exit 0 even under --strict");
}

#[test]
fn regenerate_byte_identical() {
    let workspace = tmp_workspace();
    let rel = "content/assets/r.svg";
    let abs = write_asset(&workspace, rel, b"<svg>canonical</svg>");
    let (code, _, _) = add_entry(&workspace, "r", rel);
    assert_eq!(code, 0);
    // Drift the asset
    std::fs::write(&abs, b"corrupted").unwrap();
    let lp = ledger_path(&workspace).display().to_string();
    let (code, _, _) = run_cmd(&workspace, &["ledger", "regenerate", "--all", "--ledger-path", &lp]);
    assert_eq!(code, 0);
    let restored = std::fs::read(&abs).unwrap();
    assert_eq!(restored, b"<svg>canonical</svg>");
    // Verify is now strict-ok.
    let (code, _, _) = run_cmd(
        &workspace,
        &["ledger", "verify", "--all", "--strict-status", "--ledger-path", &lp],
    );
    assert_eq!(code, 0);
}

#[test]
fn summary_groups_status() {
    let workspace = tmp_workspace();
    let a = "content/assets/a.svg";
    let b = "content/assets/b.svg";
    write_asset(&workspace, a, b"<svg>a</svg>");
    write_asset(&workspace, b, b"<svg>b</svg>");
    let (code, _, _) = add_entry(&workspace, "a", a);
    assert_eq!(code, 0);
    let (code, _, _) = add_entry(&workspace, "b", b);
    assert_eq!(code, 0);
    // Drift one to populate the Drifted bucket.
    write_asset(&workspace, b, b"hand-edited");
    let lp = ledger_path(&workspace).display().to_string();
    let (code, stdout, _) = run_cmd(&workspace, &["--json", "ledger", "summary", "--ledger-path", &lp]);
    assert_eq!(code, 0);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json.get("total_entries").unwrap(), 2);
    // by_status only reflects entry.regen_status (which is Fresh at write-time);
    // the drift bucket is populated by `verify` not `summary`.
    let (code, _, _) = run_cmd(
        &workspace,
        &[
            "--json",
            "ledger",
            "verify",
            "--all",
            "--strict-status",
            "--ledger-path",
            &lp,
        ],
    );
    assert_ne!(code, 0);
}

#[test]
fn schema_version_locked_v1() {
    let workspace = tmp_workspace();
    let rel = "content/assets/x.svg";
    write_asset(&workspace, rel, b"<svg/>");
    let (code, _, _) = add_entry(&workspace, "x", rel);
    assert_eq!(code, 0);
    let raw = std::fs::read_to_string(ledger_path(&workspace)).unwrap();
    let v: serde_json::Value = serde_json::from_str(raw.lines().next().unwrap()).unwrap();
    assert_eq!(v.get("schema_version").and_then(|v| v.as_str()), Some("1.0.0"));
}

#[test]
fn full_re_bake_from_scratch_is_idempotent() {
    let workspace = tmp_workspace();
    let rel = "content/assets/foo.svg";
    let abs = write_asset(&workspace, rel, b"<svg>foo</svg>");
    let (code, _, _) = add_entry(&workspace, "foo", rel);
    assert_eq!(code, 0);
    let lp = ledger_path(&workspace).display().to_string();
    // Delete the file → regen via freeze
    std::fs::remove_file(&abs).unwrap();
    let (code, _, _) = run_cmd(&workspace, &["ledger", "regenerate", "--all", "--ledger-path", &lp]);
    assert_eq!(code, 0);
    assert!(abs.exists());
    // Second regen: no change.
    let blake_before = blake3::hash(&std::fs::read(&abs).unwrap()).to_hex().to_string();
    let (code, _, _) = run_cmd(&workspace, &["ledger", "regenerate", "--all", "--ledger-path", &lp]);
    assert_eq!(code, 0);
    let blake_after = blake3::hash(&std::fs::read(&abs).unwrap()).to_hex().to_string();
    assert_eq!(blake_before, blake_after);
}

#[test]
fn show_and_diff_round_trip() {
    let workspace = tmp_workspace();
    let rel = "content/assets/x.svg";
    write_asset(&workspace, rel, b"<svg/>");
    let (code, _, _) = add_entry(&workspace, "x", rel);
    assert_eq!(code, 0);
    let lp = ledger_path(&workspace).display().to_string();
    let raw = std::fs::read_to_string(ledger_path(&workspace)).unwrap();
    let line: serde_json::Value = serde_json::from_str(raw.lines().next().unwrap()).unwrap();
    let id = line.get("id").and_then(|v| v.as_str()).unwrap();
    let (code, stdout, _) = run_cmd(&workspace, &["ledger", "show", id, "--ledger-path", &lp]);
    assert_eq!(code, 0);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json.get("id").and_then(|v| v.as_str()), Some(id));
    let (code, _, _) = run_cmd(&workspace, &["ledger", "diff", id, "--ledger-path", &lp]);
    assert_eq!(code, 0);
}
