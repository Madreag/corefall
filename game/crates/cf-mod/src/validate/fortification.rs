use std::{fs, path::Path};

use crate::report::ValidationReport;

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
pub(crate) fn validate_fortification(path: &Path, report: &mut ValidationReport) {
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
    use crate::report::ValidationReport;
    use crate::test_helpers::write_tmp;
    use std::path::PathBuf;

    fn write_named(name: &str, contents: &str) -> PathBuf {
        write_tmp(name, contents)
    }

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
