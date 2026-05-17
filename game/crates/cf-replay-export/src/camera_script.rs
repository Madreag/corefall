//! M10B camera script loader (`*.camera.ron`).
//!
//! Spec § Notes for the implementer:
//!
//! > Camera director scripts are RON files (consistent with the rest of
//! > the workspace's data format). Per-tick pose interpolation is
//! > cubic Catmull-Rom between declared keyframes; non-keyframe ticks
//! > are computed deterministically from neighboring keyframes. Avoid
//! > `f64`; pose is `[f32; 6]` `(x, y, zoom, rotation, lookahead_x,
//! > lookahead_y)`.
//!
//! VAL-M10B-010: the loader rejects 4 malformed input cases with typed
//! errors (no panic, never returns `Ok(_)`):
//!
//! 1. **Missing pose** — a keyframe without a `pose` field.
//! 2. **Non-finite pose values** — a keyframe whose pose contains
//!    `NaN` / `+Inf` / `-Inf`.
//! 3. **Overlapping tick ranges** — two camera tracks whose tick
//!    ranges overlap (the routing semantics are ambiguous and
//!    therefore rejected).
//! 4. **Unknown camera kind** — a track declaring a `kind` value
//!    outside the spec's enumerated set (`free_cam`, `follow_player`,
//!    `objective_cam`, `kill_cam`).
//!
//! VAL-M10B-024 + VAL-M10B-025 consume the parsed [`CameraScript`] and
//! drive per-tick routing + Catmull-Rom interpolation in
//! `cf-replay-export::camera_director`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Pose vector — `(x, y, zoom, rotation, lookahead_x, lookahead_y)`.
/// `f32` per spec § Notes (avoid `f64`).
pub type Pose = [f32; 6];

/// Pose component count. Used by loader validation when the input
/// supplies an array of unexpected length.
pub const POSE_COMPONENTS: usize = 6;

/// Canonical camera kinds enumerated by the spec's Player-facing
/// behavior section ("Up to 4 camera tracks (`free_cam`,
/// `follow_player`, `objective_cam`, `kill_cam`) can be cut between
/// in the editor"). The script loader rejects any other string with
/// [`CameraScriptError::UnknownCameraKind`].
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraKind {
    FreeCam,
    FollowPlayer,
    ObjectiveCam,
    KillCam,
}

impl CameraKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            CameraKind::FreeCam => "free_cam",
            CameraKind::FollowPlayer => "follow_player",
            CameraKind::ObjectiveCam => "objective_cam",
            CameraKind::KillCam => "kill_cam",
        }
    }

    #[must_use]
    pub fn from_wire(value: &str) -> Option<CameraKind> {
        Some(match value {
            "free_cam" => CameraKind::FreeCam,
            "follow_player" => CameraKind::FollowPlayer,
            "objective_cam" => CameraKind::ObjectiveCam,
            "kill_cam" => CameraKind::KillCam,
            _ => return None,
        })
    }
}

/// One keyframe — `(tick, pose)`. The pose at any non-keyframe tick
/// is computed via Catmull-Rom interpolation from the four neighboring
/// keyframes (see `cf-replay-export::camera_director`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CameraKeyframe {
    pub tick: u64,
    pub pose: Pose,
}

/// One camera track — a kind + half-open tick range + at-least-one
/// keyframe. The director routes per-tick render commands to the
/// track whose `[start_tick, end_tick)` window contains the tick.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraTrack {
    pub kind: CameraKind,
    /// Inclusive start tick.
    pub start_tick: u64,
    /// Exclusive end tick.
    pub end_tick: u64,
    pub keyframes: Vec<CameraKeyframe>,
}

impl CameraTrack {
    #[must_use]
    pub fn len_ticks(&self) -> u64 {
        self.end_tick.saturating_sub(self.start_tick)
    }

    /// `true` when `tick` lies in `[start_tick, end_tick)`.
    #[must_use]
    pub fn contains(&self, tick: u64) -> bool {
        tick >= self.start_tick && tick < self.end_tick
    }

    /// `true` if the receiver's tick range intersects `other`'s.
    #[must_use]
    pub fn overlaps(&self, other: &CameraTrack) -> bool {
        self.start_tick < other.end_tick && other.start_tick < self.end_tick
    }
}

/// Parsed camera script. The director walks `tracks` in declaration
/// order; the script loader rejects overlapping ranges so the routing
/// rule "per-tick render commands to the declared angle ranges" stays
/// well-defined.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraScript {
    pub tracks: Vec<CameraTrack>,
}

impl CameraScript {
    /// Parse + validate from a RON string. Returns the loaded script
    /// or a typed error variant for each of the 4 VAL-M10B-010 cases.
    pub fn from_ron_str(text: &str) -> Result<Self, CameraScriptError> {
        let parsed: CameraScriptRon =
            ron::from_str::<CameraScriptRon>(text).map_err(|source| CameraScriptError::Parse {
                source: Box::new(source),
            })?;
        let script = parsed.into_validated()?;
        Ok(script)
    }

    /// Load + validate from disk.
    pub fn load(path: &Path) -> Result<Self, CameraScriptError> {
        let text = fs::read_to_string(path).map_err(|source| CameraScriptError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_ron_str(&text)
    }

    /// Total track count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    /// `true` when no tracks are declared (no routing happens).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    /// Look up the active track for a given tick. Returns `None` if no
    /// track covers the tick.
    #[must_use]
    pub fn track_at_tick(&self, tick: u64) -> Option<&CameraTrack> {
        self.tracks.iter().find(|t| t.contains(tick))
    }
}

/// Raw on-disk shape. The validated [`CameraScript`] is produced by
/// running [`CameraScriptRon::into_validated`] which performs the four
/// typed-error checks per VAL-M10B-010.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CameraScriptRon {
    tracks: Vec<CameraTrackRon>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CameraTrackRon {
    kind: String,
    start_tick: u64,
    end_tick: u64,
    keyframes: Vec<CameraKeyframeRon>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CameraKeyframeRon {
    tick: u64,
    /// Optional so loader can detect the "missing pose" case and emit
    /// [`CameraScriptError::MissingPose`].
    #[serde(default)]
    pose: Option<Vec<f32>>,
}

impl CameraScriptRon {
    fn into_validated(self) -> Result<CameraScript, CameraScriptError> {
        let mut tracks: Vec<CameraTrack> = Vec::with_capacity(self.tracks.len());

        for (track_index, track) in self.tracks.into_iter().enumerate() {
            let kind = CameraKind::from_wire(&track.kind).ok_or_else(|| CameraScriptError::UnknownCameraKind {
                track_index,
                raw: track.kind.clone(),
            })?;
            if track.end_tick <= track.start_tick {
                return Err(CameraScriptError::EmptyRange {
                    track_index,
                    start_tick: track.start_tick,
                    end_tick: track.end_tick,
                });
            }

            let mut keyframes: Vec<CameraKeyframe> = Vec::with_capacity(track.keyframes.len());
            for (kf_index, kf) in track.keyframes.into_iter().enumerate() {
                let pose_vec = kf.pose.ok_or(CameraScriptError::MissingPose {
                    track_index,
                    keyframe_index: kf_index,
                })?;
                if pose_vec.len() != POSE_COMPONENTS {
                    return Err(CameraScriptError::PoseWrongArity {
                        track_index,
                        keyframe_index: kf_index,
                        got: pose_vec.len(),
                        want: POSE_COMPONENTS,
                    });
                }
                for (component_index, value) in pose_vec.iter().enumerate() {
                    if !value.is_finite() {
                        return Err(CameraScriptError::NonFinitePose {
                            track_index,
                            keyframe_index: kf_index,
                            component_index,
                            value: *value,
                        });
                    }
                }
                let mut pose: Pose = [0.0; POSE_COMPONENTS];
                pose.copy_from_slice(&pose_vec);
                keyframes.push(CameraKeyframe { tick: kf.tick, pose });
            }
            if keyframes.is_empty() {
                return Err(CameraScriptError::EmptyKeyframes { track_index });
            }
            keyframes.sort_by_key(|k| k.tick);

            let built = CameraTrack {
                kind,
                start_tick: track.start_tick,
                end_tick: track.end_tick,
                keyframes,
            };
            for (other_index, other) in tracks.iter().enumerate() {
                if built.overlaps(other) {
                    return Err(CameraScriptError::OverlappingRange {
                        track_a: other_index,
                        track_b: track_index,
                        a_start: other.start_tick,
                        a_end: other.end_tick,
                        b_start: built.start_tick,
                        b_end: built.end_tick,
                    });
                }
            }
            tracks.push(built);
        }

        Ok(CameraScript { tracks })
    }
}

/// Typed errors surfaced by the camera-script loader. VAL-M10B-010
/// requires zero generic-string errors and zero panics on malformed
/// inputs; the 4 mandatory cases are:
///
/// - [`CameraScriptError::MissingPose`]
/// - [`CameraScriptError::NonFinitePose`]
/// - [`CameraScriptError::OverlappingRange`]
/// - [`CameraScriptError::UnknownCameraKind`]
///
/// Additional variants surface other malformed-input cases (wrong
/// pose arity, empty range, empty keyframes, RON parse error, IO
/// failure) with the same typed shape so the failure mode is always
/// debuggable.
#[derive(Debug, Error)]
pub enum CameraScriptError {
    #[error("camera script read failure at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("camera script parse failure: {source}")]
    Parse {
        #[source]
        source: Box<ron::error::SpannedError>,
    },
    #[error("camera track #{track_index}: keyframe #{keyframe_index} is missing the required `pose` field")]
    MissingPose { track_index: usize, keyframe_index: usize },
    #[error("camera track #{track_index}: keyframe #{keyframe_index} pose array has {got} components (want {want})")]
    PoseWrongArity {
        track_index: usize,
        keyframe_index: usize,
        got: usize,
        want: usize,
    },
    #[error(
        "camera track #{track_index}: keyframe #{keyframe_index} pose component #{component_index} is non-finite ({value})"
    )]
    NonFinitePose {
        track_index: usize,
        keyframe_index: usize,
        component_index: usize,
        value: f32,
    },
    #[error("camera track #{track_a} [{a_start}..{a_end}) overlaps camera track #{track_b} [{b_start}..{b_end})")]
    OverlappingRange {
        track_a: usize,
        track_b: usize,
        a_start: u64,
        a_end: u64,
        b_start: u64,
        b_end: u64,
    },
    #[error("camera track #{track_index}: unknown camera kind `{raw}` (expected one of: free_cam, follow_player, objective_cam, kill_cam)")]
    UnknownCameraKind { track_index: usize, raw: String },
    #[error(
        "camera track #{track_index}: empty tick range `[{start_tick}..{end_tick})` (end must be strictly greater than start)"
    )]
    EmptyRange {
        track_index: usize,
        start_tick: u64,
        end_tick: u64,
    },
    #[error("camera track #{track_index}: declares zero keyframes (at least one required)")]
    EmptyKeyframes { track_index: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(track_count: usize) -> String {
        let tracks: Vec<CameraTrackRon> = (0..track_count)
            .map(|i| CameraTrackRon {
                kind: "free_cam".into(),
                start_tick: (i as u64) * 600,
                end_tick: ((i as u64) + 1) * 600,
                keyframes: vec![CameraKeyframeRon {
                    tick: (i as u64) * 600,
                    pose: Some(vec![0.0, 0.0, 1.0, 0.0, 0.0, 0.0]),
                }],
            })
            .collect();
        ron::ser::to_string(&CameraScriptRon { tracks }).unwrap()
    }

    /// Round-trip parse: a well-formed multi-track script loads.
    #[test]
    fn well_formed_three_track_script_loads() {
        let text = round_trip(3);
        let script = CameraScript::from_ron_str(&text).expect("should parse");
        assert_eq!(script.len(), 3);
        assert_eq!(script.tracks[0].kind, CameraKind::FreeCam);
        assert_eq!(script.tracks[1].start_tick, 600);
        assert!(!script.is_empty());
    }

    /// VAL-M10B-010 case (1): missing pose → typed error.
    #[test]
    fn camera_script_missing_pose_returns_typed_error() {
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

    /// VAL-M10B-010 case (2): non-finite pose value → typed error.
    #[test]
    fn camera_script_non_finite_pose_returns_typed_error() {
        let bad = r#"(tracks: [(kind: "free_cam", start_tick: 0, end_tick: 600, keyframes: [(tick: 0, pose: Some([NaN, 0.0, 1.0, 0.0, 0.0, 0.0]))])])"#;
        let err = CameraScript::from_ron_str(bad).expect_err("NaN pose must error");
        match err {
            CameraScriptError::NonFinitePose {
                track_index,
                keyframe_index,
                component_index,
                ..
            } => {
                assert_eq!(track_index, 0);
                assert_eq!(keyframe_index, 0);
                assert_eq!(component_index, 0);
            }
            other => panic!("expected NonFinitePose, got {other:?}"),
        }
    }

    /// VAL-M10B-010 case (3): overlapping ranges → typed error.
    #[test]
    fn camera_script_overlapping_range_returns_typed_error() {
        let bad = r#"(
            tracks: [
                (kind: "free_cam", start_tick: 0, end_tick: 800, keyframes: [(tick: 0, pose: Some([0.0, 0.0, 1.0, 0.0, 0.0, 0.0]))]),
                (kind: "follow_player", start_tick: 400, end_tick: 1200, keyframes: [(tick: 400, pose: Some([0.0, 0.0, 1.0, 0.0, 0.0, 0.0]))]),
            ]
        )"#;
        let err = CameraScript::from_ron_str(bad).expect_err("overlapping range must error");
        match err {
            CameraScriptError::OverlappingRange { track_a, track_b, .. } => {
                assert_eq!(track_a, 0);
                assert_eq!(track_b, 1);
            }
            other => panic!("expected OverlappingRange, got {other:?}"),
        }
    }

    /// VAL-M10B-010 case (4): unknown camera kind → typed error.
    #[test]
    fn camera_script_unknown_camera_kind_returns_typed_error() {
        let bad = r#"(tracks: [(kind: "drone_swarm", start_tick: 0, end_tick: 600, keyframes: [(tick: 0, pose: Some([0.0, 0.0, 1.0, 0.0, 0.0, 0.0]))])])"#;
        let err = CameraScript::from_ron_str(bad).expect_err("unknown camera kind must error");
        match err {
            CameraScriptError::UnknownCameraKind { raw, .. } => {
                assert_eq!(raw, "drone_swarm");
            }
            other => panic!("expected UnknownCameraKind, got {other:?}"),
        }
    }

    /// Pose component count contract: arity != 6 → typed error.
    #[test]
    fn camera_script_pose_wrong_arity_returns_typed_error() {
        let bad = r#"(tracks: [(kind: "free_cam", start_tick: 0, end_tick: 600, keyframes: [(tick: 0, pose: Some([0.0, 0.0, 1.0]))])])"#;
        let err = CameraScript::from_ron_str(bad).expect_err("wrong arity must error");
        assert!(
            matches!(
                err,
                CameraScriptError::PoseWrongArity {
                    got: 3,
                    want: POSE_COMPONENTS,
                    ..
                }
            ),
            "got {err:?}"
        );
    }

    /// Adjacent (non-overlapping) ranges parse cleanly.
    #[test]
    fn camera_script_adjacent_ranges_parse() {
        let text = r#"(
            tracks: [
                (kind: "free_cam", start_tick: 0, end_tick: 600, keyframes: [(tick: 0, pose: Some([0.0, 0.0, 1.0, 0.0, 0.0, 0.0]))]),
                (kind: "follow_player", start_tick: 600, end_tick: 1800, keyframes: [(tick: 600, pose: Some([0.0, 0.0, 1.0, 0.0, 0.0, 0.0]))]),
                (kind: "kill_cam", start_tick: 1800, end_tick: 3000, keyframes: [(tick: 1800, pose: Some([0.0, 0.0, 1.0, 0.0, 0.0, 0.0]))]),
            ]
        )"#;
        let script = CameraScript::from_ron_str(text).expect("adjacent ranges should parse");
        assert_eq!(script.len(), 3);
        assert_eq!(script.track_at_tick(0).map(|t| t.kind), Some(CameraKind::FreeCam));
        assert_eq!(
            script.track_at_tick(900).map(|t| t.kind),
            Some(CameraKind::FollowPlayer)
        );
        assert_eq!(script.track_at_tick(2400).map(|t| t.kind), Some(CameraKind::KillCam));
        assert_eq!(script.track_at_tick(99999), None);
    }

    #[test]
    fn camera_kind_round_trip_str() {
        for k in [
            CameraKind::FreeCam,
            CameraKind::FollowPlayer,
            CameraKind::ObjectiveCam,
            CameraKind::KillCam,
        ] {
            assert_eq!(CameraKind::from_wire(k.as_str()), Some(k));
        }
    }
}
