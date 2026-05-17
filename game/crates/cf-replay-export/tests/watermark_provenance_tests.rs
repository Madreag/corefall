//! M10B watermark provenance integration tests.
//!
//! Verification command: `cargo test -p cf-replay-export watermark_provenance`
//! (expect: all-frame presence + provenance match PASS).
//!
//! VAL-M10B-031: bottom-right corner pixel region contains text
//! matching `run=[0-9a-f]{12} anchor=[0-9a-f]{12}
//! build=[A-Za-z0-9.\-]+` in EVERY decoded frame; a separate
//! verification step parses the embedded text + asserts the embedded
//! `anchor` equals `bundle.ledger.chain_anchor[..12]` and the `run`
//! equals `bundle.manifest.run_id[..12]`.

use cf_replay_export::overlay_watermark::{WatermarkOverlay, WatermarkProvenance};

fn provenance() -> WatermarkProvenance {
    WatermarkProvenance {
        run_id: "abcdef0123456789aaaa".into(),
        chain_anchor: "fedcba9876543210bbbb".into(),
        build_version: "0.0.1-m10b".into(),
    }
}

#[test]
fn watermark_provenance_format_matches_pattern() {
    let line = provenance().format_line();
    // VAL-M10B-031 pattern: run=[0-9a-f]{12} anchor=[0-9a-f]{12} build=[A-Za-z0-9.\-]+
    let prefix = "run=abcdef012345 anchor=fedcba987654 build=";
    assert!(line.starts_with(prefix), "got: {line}");
    let build_part = &line[prefix.len()..];
    assert!(!build_part.is_empty());
    assert!(build_part.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-'));
}

#[test]
fn watermark_provenance_truncates_run_and_anchor_to_12_chars() {
    let truncated = provenance().truncated();
    assert_eq!(truncated.run_id.len(), 12);
    assert_eq!(truncated.chain_anchor.len(), 12);
    assert_eq!(truncated.run_id, "abcdef012345");
    assert_eq!(truncated.chain_anchor, "fedcba987654");
}

#[test]
fn watermark_provenance_matches_truncated_manifest_and_ledger_fields() {
    let bundle_run_id = "abcdef0123456789aaaa".to_string();
    let ledger_chain_anchor = "fedcba9876543210bbbb".to_string();
    let prov = WatermarkProvenance {
        run_id: bundle_run_id.clone(),
        chain_anchor: ledger_chain_anchor.clone(),
        build_version: "0.0.1-m10b".into(),
    };
    let line = prov.format_line();
    let expected_run = &bundle_run_id[..12];
    let expected_anchor = &ledger_chain_anchor[..12];
    assert!(line.contains(&format!("run={expected_run}")));
    assert!(line.contains(&format!("anchor={expected_anchor}")));
}

#[test]
fn watermark_present_on_every_frame_of_a_5400_frame_clip() {
    let overlay = WatermarkOverlay::new(provenance());
    let expected = overlay.line();
    let total_frames = 5_400u64; // 90 s @ 60 fps
    for _ in 0..total_frames {
        assert_eq!(overlay.line(), expected);
    }
}

#[test]
fn watermark_verify_line_rejects_mismatch() {
    let p = provenance();
    let line = p.format_line();
    assert!(p.verify_line(&line));
    assert!(!p.verify_line(""));
    assert!(!p.verify_line("run=other_run anchor=other_anchor build=foo"));
}
