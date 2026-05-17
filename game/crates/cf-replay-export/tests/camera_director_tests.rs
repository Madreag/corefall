//! M10B camera director integration tests.
//!
//! VAL-M10B-024: routing — per-tick active camera matches script ranges.
//! VAL-M10B-025: pose boundary — `max displacement < 1 px`.
//! VAL-M10B-026: byte-identical cuts on repeated runs.

use cf_replay_export::{pose_displacement_pixels, CameraDirector, CameraKind, CameraScript, DirectorResolution, Pose};

fn three_track_script() -> CameraScript {
    let text = r#"(
        tracks: [
            (kind: "free_cam", start_tick: 0, end_tick: 600, keyframes: [
                (tick: 0, pose: Some([0.0, 0.0, 1.0, 0.0, 0.0, 0.0])),
                (tick: 599, pose: Some([100.0, 50.0, 1.0, 0.0, 0.0, 0.0])),
            ]),
            (kind: "follow_player", start_tick: 600, end_tick: 1800, keyframes: [
                (tick: 600, pose: Some([100.0, 50.0, 1.0, 0.0, 0.0, 0.0])),
                (tick: 1200, pose: Some([300.0, 150.0, 1.0, 0.0, 0.0, 0.0])),
                (tick: 1799, pose: Some([500.0, 250.0, 1.0, 0.0, 0.0, 0.0])),
            ]),
            (kind: "kill_cam", start_tick: 1800, end_tick: 3000, keyframes: [
                (tick: 1800, pose: Some([500.0, 250.0, 1.5, 0.0, 0.0, 0.0])),
                (tick: 2999, pose: Some([0.0, 0.0, 2.0, 0.0, 0.0, 0.0])),
            ]),
        ]
    )"#;
    CameraScript::from_ron_str(text).expect("script parses")
}

#[test]
fn camera_director_routing() {
    let script = three_track_script();
    let director = CameraDirector::new(&script);

    for tick in [0u64, 1, 100, 300, 599] {
        let res: DirectorResolution = director.resolve_at_tick(tick).expect("free_cam range");
        assert_eq!(res.kind, CameraKind::FreeCam, "tick {tick}");
    }
    for tick in [600u64, 601, 1200, 1700, 1799] {
        let res = director.resolve_at_tick(tick).expect("follow_player range");
        assert_eq!(res.kind, CameraKind::FollowPlayer, "tick {tick}");
    }
    for tick in [1800u64, 2400, 2999] {
        let res = director.resolve_at_tick(tick).expect("kill_cam range");
        assert_eq!(res.kind, CameraKind::KillCam, "tick {tick}");
    }
    assert!(
        director.resolve_at_tick(3000).is_none(),
        "post-end tick is out of range"
    );
}

#[test]
fn camera_pose_boundary() {
    let script = three_track_script();
    let director = CameraDirector::new(&script);

    let declared_600: Pose = [100.0, 50.0, 1.0, 0.0, 0.0, 0.0];
    let declared_1800: Pose = [500.0, 250.0, 1.5, 0.0, 0.0, 0.0];

    let r600 = director.resolve_at_tick(600).unwrap();
    let r1800 = director.resolve_at_tick(1800).unwrap();

    let disp_600 = pose_displacement_pixels(&r600.pose, &declared_600);
    let disp_1800 = pose_displacement_pixels(&r1800.pose, &declared_1800);

    assert!(disp_600 < 1.0, "boundary 600 displacement {disp_600} px (tol: 1)");
    assert!(disp_1800 < 1.0, "boundary 1800 displacement {disp_1800} px (tol: 1)");
}

#[test]
fn camera_director_byte_identical_on_repeated_runs() {
    let script = three_track_script();
    let d1 = CameraDirector::new(&script);
    let d2 = CameraDirector::new(&script);
    for tick in (0u64..3000).step_by(13) {
        assert_eq!(d1.resolve_at_tick(tick), d2.resolve_at_tick(tick));
    }
}
