//! **M14I** — cf-mod validate must accept the canonical
//! `content/prosthetics/*.ron` files and reject malformed prosthetic
//! specs.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

fn target_bin() -> PathBuf {
    let path = env!("CARGO_BIN_EXE_cf-mod");
    PathBuf::from(path)
}

static TMP_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn tmp_workspace() -> PathBuf {
    let seq = TMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let pid = std::process::id();
    let dir = std::env::temp_dir().join(format!("cf-mod-prosthetic-itest-{pid}-{seq}"));
    let inner = dir.join("prosthetics");
    std::fs::create_dir_all(&inner).unwrap();
    dir
}

fn write_spec(workspace: &Path, name: &str, contents: &str) -> PathBuf {
    let p = workspace.join("prosthetics").join(name);
    std::fs::write(&p, contents).unwrap();
    p
}

fn run_validate(target: &Path) -> (bool, String, String) {
    let out = Command::new(target_bin())
        .args(["validate"])
        .arg(target)
        .output()
        .expect("invoke cf-mod");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

#[test]
fn prosthetic_spec_validates_good_ron() {
    let ws = tmp_workspace();
    let path = write_spec(
        &ws,
        "prosthetic_leg_t1.ron",
        r#"(
    kind: prosthetic_leg_t1,
    display_name: "Prosthetic Leg (T1)",
    tier: t1,
    target_zones: ["leg_left", "leg_right"],
    compatible_origins: ["human", "android_organic_side", "powered_organic"],
    functional_restoration: 0.7,
    maintenance_interval_seconds: 604800.0,
    install_seconds: 60.0,
)"#,
    );
    let (ok, stdout, stderr) = run_validate(&path);
    assert!(
        ok,
        "prosthetic_spec should validate; stdout: {stdout}\nstderr: {stderr}"
    );
    assert!(stdout.contains("PASS"));
}

#[test]
fn prosthetic_spec_rejects_empty_zones() {
    let ws = tmp_workspace();
    let path = write_spec(
        &ws,
        "bad_empty_zones.ron",
        r#"(
    kind: prosthetic_leg_t1,
    display_name: "Bad",
    tier: t1,
    target_zones: [],
    compatible_origins: ["human"],
    functional_restoration: 0.7,
    maintenance_interval_seconds: 604800.0,
    install_seconds: 60.0,
)"#,
    );
    let (ok, stdout, _stderr) = run_validate(&path);
    assert!(!ok, "expected validation failure; stdout: {stdout}");
    assert!(stdout.contains("target_zones"));
}

#[test]
fn prosthetic_spec_rejects_restoration_out_of_range() {
    let ws = tmp_workspace();
    let path = write_spec(
        &ws,
        "bad_restoration.ron",
        r#"(
    kind: prosthetic_arm_t1,
    display_name: "Bad",
    tier: t1,
    target_zones: ["arm_left"],
    compatible_origins: ["human"],
    functional_restoration: 1.5,
    maintenance_interval_seconds: 604800.0,
    install_seconds: 60.0,
)"#,
    );
    let (ok, stdout, _stderr) = run_validate(&path);
    assert!(!ok, "expected validation failure; stdout: {stdout}");
    assert!(stdout.contains("functional_restoration"));
}
