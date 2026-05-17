//! M10B camera script integration tests.
//!
//! VAL-M10B-010 contract: the loader rejects 4 malformed input cases
//! with **typed errors** (no panic, never `Ok(_)`):
//!
//! 1. Missing pose field
//! 2. Non-finite pose values
//! 3. Overlapping tick ranges
//! 4. Unknown camera kind
//!
//! Each test below names the corresponding [`CameraScriptError`] variant
//! and asserts the loader returns it. The verification step
//! `cargo test -p cf-replay-export camera_script` picks up every test
//! in this file plus the unit tests inside `camera_script.rs`.

use cf_replay_export::{CameraScript, CameraScriptError, POSE_COMPONENTS};

#[test]
fn camera_script_rejects_missing_pose_with_typed_error() {
    let bad = r#"(tracks: [(kind: "free_cam", start_tick: 0, end_tick: 600, keyframes: [(tick: 0)])])"#;
    let err = CameraScript::from_ron_str(bad).expect_err("missing pose must error");
    assert!(
        matches!(
            err,
            CameraScriptError::MissingPose {
                track_index: 0,
                keyframe_index: 0
            }
        ),
        "got {err:?}"
    );
}

#[test]
fn camera_script_rejects_non_finite_pose_with_typed_error() {
    let bad = r#"(tracks: [(kind: "follow_player", start_tick: 0, end_tick: 600, keyframes: [(tick: 0, pose: Some([inf, 0.0, 1.0, 0.0, 0.0, 0.0]))])])"#;
    let err = CameraScript::from_ron_str(bad).expect_err("non-finite pose must error");
    match err {
        CameraScriptError::NonFinitePose { component_index, .. } => {
            assert_eq!(component_index, 0);
        }
        other => panic!("expected NonFinitePose, got {other:?}"),
    }
}

#[test]
fn camera_script_rejects_overlapping_range_with_typed_error() {
    let bad = r#"(
        tracks: [
            (kind: "free_cam", start_tick: 0, end_tick: 1000, keyframes: [(tick: 0, pose: Some([0.0, 0.0, 1.0, 0.0, 0.0, 0.0]))]),
            (kind: "kill_cam", start_tick: 500, end_tick: 1500, keyframes: [(tick: 500, pose: Some([0.0, 0.0, 1.0, 0.0, 0.0, 0.0]))]),
        ]
    )"#;
    let err = CameraScript::from_ron_str(bad).expect_err("overlap must error");
    match err {
        CameraScriptError::OverlappingRange {
            a_start,
            a_end,
            b_start,
            b_end,
            ..
        } => {
            assert_eq!(a_start, 0);
            assert_eq!(a_end, 1000);
            assert_eq!(b_start, 500);
            assert_eq!(b_end, 1500);
        }
        other => panic!("expected OverlappingRange, got {other:?}"),
    }
}

#[test]
fn camera_script_rejects_unknown_camera_kind_with_typed_error() {
    let bad = r#"(tracks: [(kind: "satellite_orbit", start_tick: 0, end_tick: 600, keyframes: [(tick: 0, pose: Some([0.0, 0.0, 1.0, 0.0, 0.0, 0.0]))])])"#;
    let err = CameraScript::from_ron_str(bad).expect_err("unknown kind must error");
    match err {
        CameraScriptError::UnknownCameraKind { raw, .. } => {
            assert_eq!(raw, "satellite_orbit");
        }
        other => panic!("expected UnknownCameraKind, got {other:?}"),
    }
}

#[test]
fn camera_script_accepts_well_formed_three_track_script() {
    let text = r#"(
        tracks: [
            (kind: "free_cam", start_tick: 0, end_tick: 600, keyframes: [(tick: 0, pose: Some([0.0, 0.0, 1.0, 0.0, 0.0, 0.0]))]),
            (kind: "follow_player", start_tick: 600, end_tick: 1800, keyframes: [(tick: 600, pose: Some([100.0, 0.0, 1.0, 0.0, 0.0, 0.0]))]),
            (kind: "kill_cam", start_tick: 1800, end_tick: 3000, keyframes: [(tick: 1800, pose: Some([500.0, 250.0, 1.5, 0.0, 0.0, 0.0]))]),
        ]
    )"#;
    let script = CameraScript::from_ron_str(text).expect("well-formed script must parse");
    assert_eq!(script.tracks.len(), 3);
    assert_eq!(POSE_COMPONENTS, 6);
}
