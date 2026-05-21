use std::{fs, path::Path};

use crate::report::ValidationReport;

/// **M9B-2 / VAL-M9B-MOD-SEGMENT-001**: validate one
/// `content/trench_segments/*.ron` file. The cf-trench
/// `SegmentSpec::from_ron_str` rejects unknown enums + missing required
/// fields (e.g. negative `depth`/`width`, since both are `u32`) with a
/// typed `ron::error::SpannedError` naming the offending field. The
/// validator surfaces the error inline so cfctl + CI can pattern-match
/// on `unknown_variant` / `missing_field` / `expected unsigned integer`.
pub(crate) fn validate_trench_segment(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    match cf_trench::SegmentSpec::from_ron_str(&raw) {
        Ok(spec) => {
            let derived = cf_trench::segment::CoverByStance::for_variant(spec.variant);
            if derived != spec.cover_state {
                report.add_error(
                    path.to_path_buf(),
                    format!(
                        "cover_state drift for variant {:?}: authored != cf_trench::cover_state derivation",
                        spec.variant
                    ),
                );
                return;
            }
            let filename_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
            if filename_stem != spec.variant.as_str() {
                report.add_error(
                    path.to_path_buf(),
                    format!(
                        "segment variant `{}` mismatches filename stem `{filename_stem}`",
                        spec.variant.as_str()
                    ),
                );
                return;
            }
            report.add_pass(
                path.to_path_buf(),
                format!(
                    "trench_segment ({} depth={} width={})",
                    spec.variant.as_str(),
                    spec.depth,
                    spec.width
                ),
            );
        }
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("ron parse failed: {err}"));
        }
    }
}

/// **M9B-2 / VAL-M9B-MODULES-001 (mod surface)**: validate one
/// `content/trench_modules/*.ron` file via the cf-trench
/// `ModuleSpec::from_ron_str` loader. Same typed-error contract as
/// [`validate_trench_segment`].
pub(crate) fn validate_trench_module(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    match cf_trench::ModuleSpec::from_ron_str(&raw) {
        Ok(spec) => {
            report.add_pass(
                path.to_path_buf(),
                format!(
                    "trench_module ({} build_time_seconds={})",
                    spec.module.as_str(),
                    spec.build_time_seconds
                ),
            );
        }
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("ron parse failed: {err}"));
        }
    }
}

/// Best-effort extraction of the bad variant name from an unstructured
/// ron parse error string. Recognises `variant named \`ultra_deep\``
/// (current ron 0.12 wording) and falls back to None when the format
/// changes. Used only to tag the cf-mod error string with the spec-
/// literal `unknown_segment_variant: <bad>` label.
fn extract_bad_variant_name(msg: &str) -> Option<String> {
    if let Some(idx) = msg.find("variant named `") {
        let rest = &msg[idx + "variant named `".len()..];
        return rest.split('`').next().map(|s| s.to_string());
    }
    None
}

fn extract_bad_fortification_id(msg: &str) -> Option<String> {
    if let Some(idx) = msg.find("unknown_fortification_id:") {
        let rest = &msg[idx + "unknown_fortification_id:".len()..];
        return rest.split_whitespace().next().map(|s| s.to_string());
    }
    if let Some(idx) = msg.find("UnknownFortificationId(") {
        let rest = &msg[idx + "UnknownFortificationId(".len()..];
        return rest.split(')').next().map(|s| s.trim_matches('"').to_string());
    }
    None
}

/// **M9B-2 / VAL-M9B-TEMPLATE-003**: validate one
/// `content/trench_templates/*.trench.ron` file through the cf-content
/// loader. Unknown segment variants + unknown fortification ids fail
/// with typed errors; optional fortification placeholders that resolve
/// to KNOWN-but-not-yet-shipped M9C ids emit a WARN entry
/// (`trench_template_missing_fortification` per spec § Notes for the
/// implementer / VAL-M9B-TEMPLATE-004) rather than a FAIL.
pub(crate) fn validate_trench_template(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let template = match cf_content::TrenchTemplate::from_ron_str(&raw) {
        Ok(t) => t,
        Err(err) => {
            let msg = format!("{err}");
            let lower = msg.to_lowercase();
            let tagged = if lower.contains("variant named") || lower.contains("variant `")
                || lower.contains("unknown variant")
                || lower.contains("expected one of")
                || msg.contains("UnknownSegmentVariant")
            {
                if let Some(bad) = extract_bad_variant_name(&msg) {
                    format!("unknown_segment_variant: {bad}; {msg}")
                } else {
                    format!("unknown_segment_variant: {msg}")
                }
            } else if msg.contains("unknown_fortification_id")
                || msg.contains("UnknownFortificationId")
            {
                if let Some(bad) = extract_bad_fortification_id(&msg) {
                    format!("unknown_fortification_id: {bad}; {msg}")
                } else {
                    msg
                }
            } else {
                msg
            };
            report.add_error(path.to_path_buf(), format!("ron parse failed: {tagged}"));
            return;
        }
    };
    let filename_stem = path.file_name().and_then(|s| s.to_str()).unwrap_or_default();
    let expected = format!("{}.trench.ron", template.id);
    if filename_stem != expected {
        report.add_error(
            path.to_path_buf(),
            format!(
                "template id `{}` mismatches filename `{filename_stem}` (expected `{expected}`)",
                template.id
            ),
        );
        return;
    }
    for placeholder in &template.fortification_placeholders {
        if placeholder.optional {
            report.add_warn(
                path.to_path_buf(),
                format!(
                    "{}: optional placeholder `{}` may emit `trench_template_missing_fortification` warning event until M9C ships",
                    cf_content::placeholder_warning_label(),
                    placeholder.fortification_id
                ),
            );
        }
    }
    report.add_pass(
        path.to_path_buf(),
        format!(
            "trench_template `{}` ({} polyline pts, {} placeholders)",
            template.id,
            template.path_polyline.len(),
            template.fortification_placeholders.len()
        ),
    );
}
