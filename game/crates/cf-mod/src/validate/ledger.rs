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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::ValidationReport;
    use crate::test_helpers::write_tmp;
    use std::fs;

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
}
