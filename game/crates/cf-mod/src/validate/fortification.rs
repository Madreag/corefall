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
