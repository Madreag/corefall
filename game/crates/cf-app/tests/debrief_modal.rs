//! VAL-M10B-DEBRIEF-CTA: cf-app post-mission debrief modal renders
//! "Export Last Replay" button.
//!
//! Test names cover both the spec-canonical pattern
//! (`export_last_replay_cta_present`) AND the verification-step
//! pattern (`debrief_modal_export_button*`) so `cargo test
//! debrief_modal_export_button` and `cargo test --test debrief_modal
//! export_last_replay_cta_present` both PASS.

use std::path::Path;

use cf_app::debrief_modal::{
    build_debrief_modal, EXPORT_LAST_REPLAY_BUTTON_ID, EXPORT_LAST_REPLAY_BUTTON_LABEL,
};

/// VAL-M10B-DEBRIEF-CTA: a button with the documented id/label is
/// in the modal's widget tree.
#[test]
fn export_last_replay_cta_present() {
    let modal = build_debrief_modal(Path::new("/tmp/bundle_under_test"), "run_cta_test");
    let button = modal.export_last_replay_button().expect("button present");
    assert_eq!(button.id, EXPORT_LAST_REPLAY_BUTTON_ID);
    assert_eq!(button.label, EXPORT_LAST_REPLAY_BUTTON_LABEL);
}

/// VAL-M10B-DEBRIEF-CTA: the button's on-click handler invokes the
/// `cf-tools-replay-viewer export` entry point with `--preset
/// clip_compact` against the active bundle.
#[test]
fn debrief_modal_export_button_dispatch_args_match() {
    let modal = build_debrief_modal(Path::new("/tmp/bundle_under_test"), "run_cta_args");
    let dispatch = modal
        .export_last_replay_button()
        .expect("button present")
        .on_click
        .as_ref()
        .expect("dispatch present");
    assert_eq!(dispatch.binary, "cf-tools-replay-viewer");
    assert_eq!(dispatch.preset, "clip_compact");
    assert_eq!(dispatch.argv[0], "export");
    let preset_idx = dispatch.argv.iter().position(|s| s == "--preset").unwrap();
    assert_eq!(dispatch.argv[preset_idx + 1], "clip_compact");
    let out_idx = dispatch.argv.iter().position(|s| s == "--out").unwrap();
    let out_arg = &dispatch.argv[out_idx + 1];
    assert!(
        out_arg.contains("Corefall") || out_arg.ends_with("run_cta_args.mp4"),
        "out path must include the Corefall subdir (default) or fallback name; got {out_arg}"
    );
}

/// Alias under the verification-step naming pattern: `cargo test -p
/// cf-app debrief_modal_export_button` matches this name + the file
/// stem.
#[test]
fn debrief_modal_export_button_present() {
    let modal = build_debrief_modal(Path::new("/tmp/bundle_under_test"), "run_cta_present");
    assert!(modal.has_export_last_replay_cta(), "Export Last Replay button must be present");
}
