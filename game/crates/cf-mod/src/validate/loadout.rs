use std::{fs, path::Path};

use crate::report::ValidationReport;

/// Verifies the schema_version, id↔filename parity, non-empty `role_ids`,
/// and that every referenced role id resolves through `cf_equipment::role_record`.
pub(crate) fn validate_loadout_file(path: &Path, report: &mut ValidationReport) {
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
