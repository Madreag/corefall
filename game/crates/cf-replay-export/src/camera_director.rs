//! M10B camera director — per-tick pose interpolation + routing.
//!
//! Spec § Notes for the implementer:
//!
//! > Per-tick pose interpolation is cubic Catmull-Rom between declared
//! > keyframes; non-keyframe ticks are computed deterministically from
//! > neighboring keyframes. Avoid `f64`; pose is `[f32; 6]`
//! > `(x, y, zoom, rotation, lookahead_x, lookahead_y)`.
//!
//! VAL-M10B-024: the director routes per-tick render commands to the
//! declared angle ranges. Given a script `free_cam[0..600] →
//! follow_player[600..1800] → kill_cam[1800..end]`, every tick in
//! `[0, 600)` is `free_cam`, every tick in `[600, 1800)` is
//! `follow_player`, every tick in `[1800, end)` is `kill_cam`.
//!
//! VAL-M10B-025: per-tick pose at cut boundaries (600, 1800) matches
//! the declared pose within `< 1 px` of on-screen displacement.
//! `pose_at_tick` snaps directly to a keyframe when the tick equals
//! that keyframe's `tick` field; the Catmull-Rom path is only used
//! between keyframes.
//!
//! VAL-M10B-026: same script + bundle yields byte-identical cuts on
//! repeated runs — the interpolation is pure `f32` arithmetic with no
//! RNG, no `f64`, no platform-conditional paths, so two runs on the
//! same host produce bit-identical poses.

use crate::camera_script::{CameraKeyframe, CameraKind, CameraScript, CameraTrack, Pose, POSE_COMPONENTS};

/// One per-tick pose decision: the active camera kind + interpolated
/// pose. Returned by [`CameraDirector::resolve_at_tick`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectorResolution {
    pub tick: u64,
    pub kind: CameraKind,
    pub pose: Pose,
}

/// In-process camera director. Wraps a parsed [`CameraScript`] and
/// answers per-tick pose queries.
pub struct CameraDirector<'script> {
    script: &'script CameraScript,
}

impl<'script> CameraDirector<'script> {
    #[must_use]
    pub fn new(script: &'script CameraScript) -> Self {
        Self { script }
    }

    /// Return the script the director routes against.
    #[must_use]
    pub fn script(&self) -> &CameraScript {
        self.script
    }

    /// Resolve the active camera + pose for the given tick.
    ///
    /// Returns `None` when no script track covers the tick.
    #[must_use]
    pub fn resolve_at_tick(&self, tick: u64) -> Option<DirectorResolution> {
        let track = self.script.track_at_tick(tick)?;
        Some(DirectorResolution {
            tick,
            kind: track.kind,
            pose: pose_at_tick(track, tick),
        })
    }

    /// Resolve a contiguous tick range `[start, end)`. Returns one
    /// [`DirectorResolution`] per tick whose active track is defined.
    /// Out-of-range ticks are omitted (rather than panicking) so the
    /// frame ticker can skip frames the script doesn't cover.
    pub fn resolve_range(&self, start: u64, end: u64) -> Vec<DirectorResolution> {
        let mut out = Vec::with_capacity(end.saturating_sub(start) as usize);
        for tick in start..end {
            if let Some(res) = self.resolve_at_tick(tick) {
                out.push(res);
            }
        }
        out
    }
}

/// Cubic Catmull-Rom interpolated pose for `tick` along `track`'s
/// keyframe spline.
///
/// - When `tick` exactly equals a keyframe's `tick` field, the
///   keyframe's pose is returned verbatim (VAL-M10B-025 cut-boundary
///   contract).
/// - For ticks between two keyframes, the four nearest keyframes
///   (`p0`, `p1`, `p2`, `p3` per Catmull-Rom convention) drive the
///   per-component interpolation.
/// - At the spline's edges, the missing neighbour is mirrored from the
///   inside keyframe so the curve has well-defined tangents at the
///   first / last segment.
#[must_use]
pub fn pose_at_tick(track: &CameraTrack, tick: u64) -> Pose {
    let kfs = &track.keyframes;
    if kfs.is_empty() {
        return [0.0; POSE_COMPONENTS];
    }
    if kfs.len() == 1 {
        return kfs[0].pose;
    }
    if let Some(kf) = kfs.iter().find(|k| k.tick == tick) {
        return kf.pose;
    }
    if tick <= kfs[0].tick {
        return kfs[0].pose;
    }
    if tick >= kfs[kfs.len() - 1].tick {
        return kfs[kfs.len() - 1].pose;
    }

    let upper_index = kfs.iter().position(|k| k.tick > tick).expect("non-edge");
    let i1 = upper_index - 1;
    let i2 = upper_index;
    let p1 = kfs[i1];
    let p2 = kfs[i2];
    let p0 = if i1 == 0 { mirror(p2, p1) } else { kfs[i1 - 1] };
    let p3 = if i2 == kfs.len() - 1 {
        mirror(p1, p2)
    } else {
        kfs[i2 + 1]
    };

    let span = (p2.tick - p1.tick) as f32;
    let t = if span > 0.0 {
        (tick - p1.tick) as f32 / span
    } else {
        0.0
    };

    let mut out: Pose = [0.0; POSE_COMPONENTS];
    for c in 0..POSE_COMPONENTS {
        out[c] = catmull_rom(p0.pose[c], p1.pose[c], p2.pose[c], p3.pose[c], t);
    }
    out
}

/// Mirror keyframe `inner` about `pivot` along the tick axis. Used at
/// the spline's edges so the Catmull-Rom tangent at the first / last
/// keyframe is well-defined.
fn mirror(pivot: CameraKeyframe, inner: CameraKeyframe) -> CameraKeyframe {
    let mut pose: Pose = [0.0; POSE_COMPONENTS];
    for c in 0..POSE_COMPONENTS {
        pose[c] = 2.0 * pivot.pose[c] - inner.pose[c];
    }
    CameraKeyframe {
        tick: pivot.tick.saturating_sub(inner.tick.saturating_sub(pivot.tick)),
        pose,
    }
}

/// Standard cubic Catmull-Rom for one scalar component at the
/// normalized parameter `t in [0, 1]`. Per spec § Notes pose is `f32`
/// so this never widens to `f64`.
#[must_use]
fn catmull_rom(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * (2.0 * p1
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

/// On-screen displacement (px) between two poses at the given preset
/// resolution. The director uses this in tests to assert
/// VAL-M10B-025's `< 1 px` cut-boundary contract.
///
/// `pose` layout: `(x, y, zoom, rotation, lookahead_x, lookahead_y)`.
/// Displacement is computed in scene-space units; conversion to pixels
/// scales by the preset's `(width, height) / scene_extent`. The default
/// `scene_extent` (1.0) treats pose components as already being in
/// pixel units — sufficient for the unit-test boundary contract.
#[must_use]
pub fn pose_displacement_pixels(a: &Pose, b: &Pose) -> f32 {
    let dx = (a[0] - b[0]).abs();
    let dy = (a[1] - b[1]).abs();
    dx.max(dy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera_script::CameraScript;

    fn script_three_tracks() -> CameraScript {
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

    /// VAL-M10B-024: per-tick router maps every tick in `[start, end)`
    /// to its declared camera kind.
    #[test]
    fn camera_director_routing_matches_script_ranges() {
        let script = script_three_tracks();
        let director = CameraDirector::new(&script);

        for tick in [0u64, 1, 300, 599] {
            assert_eq!(
                director.resolve_at_tick(tick).unwrap().kind,
                CameraKind::FreeCam,
                "tick {tick} should be free_cam"
            );
        }
        for tick in [600u64, 900, 1500, 1799] {
            assert_eq!(
                director.resolve_at_tick(tick).unwrap().kind,
                CameraKind::FollowPlayer,
                "tick {tick} should be follow_player"
            );
        }
        for tick in [1800u64, 2400, 2999] {
            assert_eq!(
                director.resolve_at_tick(tick).unwrap().kind,
                CameraKind::KillCam,
                "tick {tick} should be kill_cam"
            );
        }
        assert!(
            director.resolve_at_tick(3000).is_none(),
            "post-end tick is out of range"
        );
    }

    /// VAL-M10B-025: per-tick pose at cut boundaries (600, 1800)
    /// matches declared pose within < 1 px.
    #[test]
    fn camera_pose_boundary_matches_declared_within_1_px() {
        let script = script_three_tracks();
        let director = CameraDirector::new(&script);

        let r600 = director.resolve_at_tick(600).unwrap();
        assert_eq!(r600.pose[0], 100.0);
        assert_eq!(r600.pose[1], 50.0);

        let r1800 = director.resolve_at_tick(1800).unwrap();
        assert_eq!(r1800.pose[0], 500.0);
        assert_eq!(r1800.pose[1], 250.0);

        let declared_600: Pose = [100.0, 50.0, 1.0, 0.0, 0.0, 0.0];
        let declared_1800: Pose = [500.0, 250.0, 1.5, 0.0, 0.0, 0.0];

        let disp_600 = pose_displacement_pixels(&r600.pose, &declared_600);
        let disp_1800 = pose_displacement_pixels(&r1800.pose, &declared_1800);

        assert!(disp_600 < 1.0, "boundary 600 displacement {disp_600} px (tol: 1)");
        assert!(disp_1800 < 1.0, "boundary 1800 displacement {disp_1800} px (tol: 1)");
    }

    /// Catmull-Rom interpolation produces a value strictly between
    /// `p1` and `p2` for a tick mid-segment (not an exact keyframe).
    #[test]
    fn catmull_rom_interpolates_between_keyframes() {
        let track = CameraTrack {
            kind: CameraKind::FollowPlayer,
            start_tick: 600,
            end_tick: 1800,
            keyframes: vec![
                CameraKeyframe {
                    tick: 600,
                    pose: [100.0, 50.0, 1.0, 0.0, 0.0, 0.0],
                },
                CameraKeyframe {
                    tick: 1200,
                    pose: [300.0, 150.0, 1.0, 0.0, 0.0, 0.0],
                },
                CameraKeyframe {
                    tick: 1799,
                    pose: [500.0, 250.0, 1.0, 0.0, 0.0, 0.0],
                },
            ],
        };
        let pose = pose_at_tick(&track, 900);
        assert!(
            pose[0] > 100.0 && pose[0] < 300.0,
            "x={} should interpolate strictly",
            pose[0]
        );
        assert!(
            pose[1] > 50.0 && pose[1] < 150.0,
            "y={} should interpolate strictly",
            pose[1]
        );
    }

    /// Two independent director runs over the same script yield
    /// byte-identical poses per VAL-M10B-026.
    #[test]
    fn director_resolution_is_byte_identical_on_repeated_runs() {
        let script = script_three_tracks();
        let d1 = CameraDirector::new(&script);
        let d2 = CameraDirector::new(&script);
        for tick in (0..3000).step_by(37) {
            let a = d1.resolve_at_tick(tick);
            let b = d2.resolve_at_tick(tick);
            assert_eq!(a, b, "tick {tick} drifts across runs");
        }
    }

    /// Out-of-script tick returns `None` (frame ticker skips it).
    #[test]
    fn director_returns_none_outside_script_ranges() {
        let script = script_three_tracks();
        let director = CameraDirector::new(&script);
        assert!(director.resolve_at_tick(3000).is_none());
        assert!(director.resolve_at_tick(u64::MAX).is_none());
    }

    /// Catmull-Rom snaps to keyframe pose when tick equals keyframe.
    #[test]
    fn pose_at_keyframe_tick_returns_keyframe_pose_verbatim() {
        let track = CameraTrack {
            kind: CameraKind::FreeCam,
            start_tick: 0,
            end_tick: 600,
            keyframes: vec![
                CameraKeyframe {
                    tick: 0,
                    pose: [10.0, 20.0, 1.0, 0.0, 0.0, 0.0],
                },
                CameraKeyframe {
                    tick: 300,
                    pose: [50.0, 100.0, 1.0, 0.0, 0.0, 0.0],
                },
            ],
        };
        let pose = pose_at_tick(&track, 300);
        assert_eq!(pose, [50.0, 100.0, 1.0, 0.0, 0.0, 0.0]);
    }
}
