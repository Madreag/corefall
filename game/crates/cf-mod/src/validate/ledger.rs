use std::path::Path;

use crate::report::ValidationReport;

/// **M4A**: validate every JSONL line in a ledger file against the locked
/// v1 AssetEntry schema. Each line that fails surfaces as a FAIL with the
/// per-line reason; lines that recompute their AssetId mismatch
/// `id_drift` reason for CI to pattern-match.
pub(crate) fn validate_ledger_jsonl(path: &Path, report: &mut ValidationReport) {
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
