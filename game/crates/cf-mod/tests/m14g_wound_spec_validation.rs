//! **M14G § VAL-M14G-008 / VAL-CROSS-012 / VAL-CROSS-028**: cf-mod must
//! reject wound_spec RON with unknown WoundKind, missing fields, or
//! malformed `heal_time_seconds_at_band` arrays.
//!
//! Drives `cf-mod validate` against three synthetic "bad" RON files plus
//! a positive control to confirm the validator wires through to the
//! schema rejection path.

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
    let dir = std::env::temp_dir().join(format!("cf-mod-wound-itest-{pid}-{seq}"));
    let wound_dir = dir.join("wound_specs");
    std::fs::create_dir_all(&wound_dir).unwrap();
    dir
}

fn write_spec(workspace: &Path, name: &str, contents: &str) -> PathBuf {
    let p = workspace.join("wound_specs").join(name);
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

/// Positive control — a good spec validates.
#[test]
fn wound_spec_validates_good_ron() {
    let ws = tmp_workspace();
    let path = write_spec(
        &ws,
        "laceration_light.ron",
        r#"(
    kind: LacerationLight,
    bleed_rate_ml_per_s_per_severity: 1.0,
    pain_contribution_per_severity: 0.05,
    infection_base_chance_per_tick: 0.00001,
    heal_time_seconds_at_band: (90.0, 180.0, 540.0, 1800.0, 2700.0, 3600.0),
    treatment_difficulty: trivial,
    allowed_zones: ["torso_front"],
    decal_id: "decal.laceration.light",
    clears_via: [bandage],
    closes_to_scar: false,
    forbids_origin: ["robot"],
)"#,
    );
    let (ok, stdout, stderr) = run_validate(&path);
    assert!(ok, "wound_spec should validate; stdout: {stdout}\nstderr: {stderr}");
}

/// VAL-M14G-008 case 1: unknown WoundKind name.
#[test]
fn wound_spec_validation_rejects_unknown_kind() {
    let ws = tmp_workspace();
    let path = write_spec(
        &ws,
        "bad_unknown_kind.ron",
        r#"(
    kind: TotallyMadeUp,
    bleed_rate_ml_per_s_per_severity: 1.0,
    pain_contribution_per_severity: 0.05,
    infection_base_chance_per_tick: 0.00001,
    heal_time_seconds_at_band: (90.0, 180.0, 540.0, 1800.0, 2700.0, 3600.0),
    treatment_difficulty: trivial,
    allowed_zones: ["torso_front"],
    decal_id: "decal.laceration.light",
    clears_via: [bandage],
    closes_to_scar: false,
    forbids_origin: [],
)"#,
    );
    let (ok, _stdout, stderr) = run_validate(&path);
    assert!(!ok, "unknown kind should reject; stderr: {stderr}");
}

/// VAL-M14G-008 case 2: missing required field `decal_id`.
#[test]
fn wound_spec_validation_rejects_missing_decal_id() {
    let ws = tmp_workspace();
    let path = write_spec(
        &ws,
        "bad_missing_field.ron",
        r#"(
    kind: LacerationLight,
    bleed_rate_ml_per_s_per_severity: 1.0,
    pain_contribution_per_severity: 0.05,
    infection_base_chance_per_tick: 0.00001,
    heal_time_seconds_at_band: (90.0, 180.0, 540.0, 1800.0, 2700.0, 3600.0),
    treatment_difficulty: trivial,
    allowed_zones: ["torso_front"],
    clears_via: [bandage],
    closes_to_scar: false,
    forbids_origin: [],
)"#,
    );
    let (ok, _stdout, stderr) = run_validate(&path);
    assert!(!ok, "missing decal_id should reject; stderr: {stderr}");
}

/// VAL-M14G-008 case 3: heal_time_seconds_at_band array length ≠ 6.
#[test]
fn wound_spec_validation_rejects_bad_heal_time_array() {
    let ws = tmp_workspace();
    let path = write_spec(
        &ws,
        "bad_heal_time.ron",
        r#"(
    kind: LacerationLight,
    bleed_rate_ml_per_s_per_severity: 1.0,
    pain_contribution_per_severity: 0.05,
    infection_base_chance_per_tick: 0.00001,
    heal_time_seconds_at_band: (90.0, 180.0, 540.0, 1800.0, 2700.0),
    treatment_difficulty: trivial,
    allowed_zones: ["torso_front"],
    decal_id: "decal.laceration.light",
    clears_via: [bandage],
    closes_to_scar: false,
    forbids_origin: [],
)"#,
    );
    let (ok, _stdout, stderr) = run_validate(&path);
    assert!(!ok, "5-element heal-time array should reject; stderr: {stderr}");
}

/// VAL-CROSS-028: cf-replay schema's `kind` enum matches the cf-wound
/// WoundKind::ALL enumeration exactly.
#[test]
fn wound_kind_enum_couples_replay_schemas_to_cf_wound() {
    let schemas = [
        include_str!("../../cf-replay/schemas/event/wound_created.json"),
        include_str!("../../cf-replay/schemas/event/wound_escalated.json"),
        include_str!("../../cf-replay/schemas/event/wound_scabbed.json"),
        include_str!("../../cf-replay/schemas/event/wound_scarred.json"),
    ];
    let canonical: std::collections::HashSet<&str> = cf_wound::WoundKind::ALL.iter().map(|k| k.as_str()).collect();
    for raw in schemas {
        let v: serde_json::Value = serde_json::from_str(raw).expect("schema is json");
        // Look up the kind enum (either under `kind` or `new_kind`/`old_kind`).
        let mut found = false;
        for field in ["kind", "new_kind", "old_kind"] {
            let ptr = format!("/properties/payload/properties/{field}/enum");
            if let Some(enum_values) = v.pointer(&ptr).and_then(|x| x.as_array()) {
                let names: std::collections::HashSet<&str> =
                    enum_values.iter().filter_map(|x| x.as_str()).collect();
                assert_eq!(
                    names, canonical,
                    "schema {field} enum must equal cf_wound::WoundKind::ALL"
                );
                found = true;
            }
        }
        assert!(found, "schema must declare a wound-kind enum somewhere");
    }
}
