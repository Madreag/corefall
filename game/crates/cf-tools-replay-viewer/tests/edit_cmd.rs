//! M10B edit-CLI dispatch integration tests.
//!
//! VAL-M10B-035: `cf-tools-replay-viewer edit <bundle>` opens the
//! editor in interactive TTY OR returns a documented headless exit
//! (no panic, no missing-command error).

use std::fs;
use std::path::Path;

use tempfile::tempdir;

use cf_tools_replay_viewer::edit_cmd::{
    run_edit, AngleSelector, EditArgs, EditError, EditOutcome, HEADLESS_EXIT_CODE,
};
use cf_replay_export::camera_script::CameraKind;

fn write_stub_bundle(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join("run_manifest.json"), "{\"run_id\":\"r\",\"tick_rate_hz\":60}").unwrap();
    fs::write(dir.join("summary.json"), "{\"first_tick\":0,\"last_tick\":1800}").unwrap();
    fs::write(dir.join("events.jsonl"), "").unwrap();
}

/// VAL-M10B-035 headless path: `--headless` returns an envelope with
/// the documented exit code.
#[test]
fn edit_cmd_headless_returns_structured_envelope() {
    let tmp = tempdir().unwrap();
    write_stub_bundle(tmp.path());
    let args = EditArgs {
        bundle_dir: Some(tmp.path().to_path_buf()),
        headless: true,
        ..Default::default()
    };
    let outcome = run_edit(args).expect("headless dispatch");
    match outcome {
        EditOutcome::Headless(env) => {
            assert_eq!(env.result, "editor_unavailable_in_headless");
            assert_eq!(env.exit_code, HEADLESS_EXIT_CODE);
            assert_eq!(env.exit_code, 74);
        }
        EditOutcome::Interactive { .. } => panic!("expected Headless under --headless flag"),
    }
}

/// VAL-M10B-035: missing bundle returns typed error.
#[test]
fn edit_cmd_missing_bundle_errors() {
    let args = EditArgs::default();
    let err = run_edit(args).expect_err("must require bundle");
    assert!(matches!(err, EditError::MissingBundle));
}

/// VAL-M10B-035: nonexistent bundle path returns typed error (not a
/// panic + not a generic message).
#[test]
fn edit_cmd_bundle_not_found_errors() {
    let args = EditArgs {
        bundle_dir: Some(std::path::PathBuf::from("/nonexistent/edit_cmd/test")),
        ..Default::default()
    };
    let err = run_edit(args).expect_err("must error on missing dir");
    assert!(matches!(err, EditError::BundleNotFound(_)));
}

/// AngleSelector: three canonical tracks by default (free_cam /
/// follow_player / kill_cam) for the multi-camera angle selector.
#[test]
fn angle_selector_default_has_three_canonical_tracks() {
    let sel = AngleSelector::default();
    let kinds: Vec<CameraKind> = sel.tracks.iter().map(|t| t.kind).collect();
    assert_eq!(kinds.len(), 3);
    assert!(kinds.contains(&CameraKind::FreeCam));
    assert!(kinds.contains(&CameraKind::FollowPlayer));
    assert!(kinds.contains(&CameraKind::KillCam));
}

/// AngleSelector: `toggle` flips the selection state.
#[test]
fn angle_selector_toggle_changes_selection() {
    let mut sel = AngleSelector::default();
    let initial = sel.tracks[1].selected;
    assert!(sel.toggle(1));
    assert_ne!(sel.tracks[1].selected, initial);
}

/// AngleSelector: `to_camera_script` produces one CameraTrack per
/// selected angle.
#[test]
fn angle_selector_to_camera_script_round_trip() {
    let mut sel = AngleSelector::default();
    // Keep the default `free_cam` selected; add follow_player.
    sel.tracks[1].selected = true;
    let script = sel.to_camera_script();
    assert_eq!(script.tracks.len(), 2);
    assert!(script.tracks.iter().any(|t| t.kind == CameraKind::FreeCam));
    assert!(script.tracks.iter().any(|t| t.kind == CameraKind::FollowPlayer));
}
