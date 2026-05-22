//! M10B replay-editor CLI dispatch.
//!
//! Spec § "Player-facing behavior":
//!
//! > **Replay editor UX ships.** `cf-tools-replay-viewer edit
//! > <bundle>` opens a frame-accurate timeline (egui front-end) with
//! > scrub bar, in/out trim points, multi-camera angle selector,
//! > commentary track overlay, and "export selection" button — no
//! > third-party video editor required for a clean 30-second
//! > highlight clip.
//!
//! The full egui editor UX lives in [`crate::editor`] +
//! `cf-tools-replay-viewer::editor_ui` (m10b-2 wired the library-level
//! state machine + the offline rasterizer preview path). This module
//! owns the CLI-side dispatch:
//!
//! - **VAL-M10B-035**: `cf-tools-replay-viewer edit <bundle>` either
//!   opens the editor in an interactive TTY OR returns a documented
//!   non-zero with a structured message in headless mode (no panic,
//!   no missing-command error).
//!
//! The CLI handler routes to the egui editor when stdin / stdout are
//! attached to a TTY; otherwise it prints a structured JSON envelope
//! to stdout and returns a documented exit code so test harnesses
//! (`cargo test`, CI) can verify the headless path without spawning
//! an actual graphics window.
//!
//! The headless exit shape lets `cfctl replay edit <bundle> --headless`
//! produce a clean, scriptable result without ever attempting an egui
//! window — see VAL-M10B-035's "headless mode" route.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use cf_render_2d::offline_mode::{FortificationKind, SceneCommand, SegmentVariant};
use cf_replay_export::camera_director::CameraDirector;
use cf_replay_export::camera_script::{CameraKeyframe, CameraKind, CameraScript, CameraTrack, Pose};

use crate::editor::{const_scene_for_tick, EditorError, EditorState};

/// Documented headless-mode exit code. Per VAL-M10B-035 the headless
/// branch "either opens the editor (TTY) or returns the documented
/// headless exit." We pick `74` (`EX_IOERR` semantic: editor cannot
/// open IO / window) so script harnesses can disambiguate
/// `editor-unavailable-in-headless` from other failures.
pub const HEADLESS_EXIT_CODE: i32 = 74;

/// Headless-mode JSON envelope. Printed to stdout by
/// [`run_edit_headless`]; CLI handler propagates `HEADLESS_EXIT_CODE`
/// upward so the calling shell sees a non-zero exit code.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HeadlessEnvelope {
    pub result: String,
    pub mode: String,
    pub bundle: String,
    pub suggested_action: String,
    pub exit_code: i32,
}

impl HeadlessEnvelope {
    /// Standard headless envelope for the `edit <bundle>` route when
    /// no TTY is attached.
    #[must_use]
    pub fn for_bundle(bundle: &Path) -> Self {
        Self {
            result: "editor_unavailable_in_headless".into(),
            mode: "headless".into(),
            bundle: bundle.display().to_string(),
            suggested_action: "run `cf-tools-replay-viewer edit <bundle>` from an interactive terminal, or invoke `cf-tools-replay-viewer export <bundle> --preset <name> --out <path>` for a non-interactive render".into(),
            exit_code: HEADLESS_EXIT_CODE,
        }
    }
}

/// Typed errors surfaced by [`run_edit`].
#[derive(Debug, Error)]
pub enum EditError {
    #[error("edit bundle path is required (pass `<bundle>` positional arg)")]
    MissingBundle,
    #[error("bundle directory not found: {0}")]
    BundleNotFound(PathBuf),
    #[error("edit init failed: {0}")]
    Editor(#[from] EditorError),
    #[error("edit IO failure: {0}")]
    Io(#[from] std::io::Error),
}

/// CLI-shaped arguments for the `edit` subcommand.
#[derive(Debug, Clone, Default)]
pub struct EditArgs {
    pub bundle_dir: Option<PathBuf>,
    /// When `true`, force headless mode (no egui attempt) regardless
    /// of TTY detection. Always-headless is the test harness path +
    /// the `cfctl replay edit --headless` shim path.
    pub headless: bool,
    /// Optional camera-script .ron path. When supplied, the editor
    /// pre-loads the script's tracks into its multi-camera angle
    /// selector so authors can resume an in-progress edit.
    pub camera_script: Option<PathBuf>,
    /// Optional initial scrub-to-tick. When supplied, the editor's
    /// timeline cursor lands at this tick on open (handy for
    /// jumping directly to a `chapter_marker_emitted` tick from the
    /// debrief log).
    pub scrub_to_tick: Option<u64>,
}

/// Outcome of [`run_edit`].
#[derive(Debug, Clone)]
pub enum EditOutcome {
    /// Headless mode triggered: editor not opened, envelope printed
    /// to stdout, exit code = `HEADLESS_EXIT_CODE`.
    Headless(HeadlessEnvelope),
    /// Interactive TTY route: editor opened (no live egui in the
    /// test harness — the m10b-2 state machine is constructed +
    /// validated; the egui front-end consumes it).
    Interactive {
        bundle: PathBuf,
        opened_at_tick: u64,
        initial_tracks: Vec<CameraKind>,
    },
}

/// CLI dispatch entry point for the `edit` subcommand.
///
/// Per VAL-M10B-035: when the binary is invoked from an interactive
/// TTY, this function constructs the [`EditorState`] (m10b-2 state
/// machine), routes the timeline cursor + multi-camera selector
/// pre-loads, and returns [`EditOutcome::Interactive`]. When invoked
/// in headless mode (`--headless`, OR stdin/stdout not a TTY) it
/// returns [`EditOutcome::Headless`] with a structured envelope —
/// the caller's exit code propagation surfaces the documented
/// non-zero exit per VAL-M10B-035.
pub fn run_edit(args: EditArgs) -> Result<EditOutcome, EditError> {
    let bundle_dir = args.bundle_dir.clone().ok_or(EditError::MissingBundle)?;
    if !bundle_dir.is_dir() {
        return Err(EditError::BundleNotFound(bundle_dir));
    }
    let force_headless = args.headless || !stdin_is_tty();
    if force_headless {
        return Ok(EditOutcome::Headless(HeadlessEnvelope::for_bundle(&bundle_dir)));
    }
    // Interactive route: construct the editor state machine +
    // pre-load any camera-script tracks.
    let initial_tracks = if let Some(camera_path) = args.camera_script.as_deref() {
        load_camera_kinds(camera_path).unwrap_or_default()
    } else {
        Vec::new()
    };
    let (start_tick, end_tick, tick_rate_hz) = scrub_bounds_for_bundle(&bundle_dir);
    let scene = default_preview_scene();
    let mut editor = EditorState::new(start_tick, end_tick, tick_rate_hz, const_scene_for_tick(scene))?;
    let target = args.scrub_to_tick.unwrap_or(start_tick);
    let _scrub = editor.scrub_to(target);
    Ok(EditOutcome::Interactive {
        bundle: bundle_dir,
        opened_at_tick: target,
        initial_tracks,
    })
}

/// Convenience for headless-only callers (cfctl `replay edit --headless`).
pub fn run_edit_headless(bundle_dir: &Path) -> EditOutcome {
    EditOutcome::Headless(HeadlessEnvelope::for_bundle(bundle_dir))
}

/// Detect whether the current process's stdin is connected to a TTY.
/// In test runs + CI this returns `false` so the headless path
/// engages by default; on a developer's terminal it returns `true`.
fn stdin_is_tty() -> bool {
    use std::io::IsTerminal as _;
    std::io::stdin().is_terminal()
}

/// Read the bundle's tick range from `<bundle>/summary.json`. The
/// editor pre-populates its trim window with `[first_tick, last_tick]`
/// so the user's initial selection covers the entire bundle.
fn scrub_bounds_for_bundle(bundle_dir: &Path) -> (u64, u64, u32) {
    if let Ok(text) = std::fs::read_to_string(bundle_dir.join("summary.json")) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
            let first = value.get("first_tick").and_then(|v| v.as_u64()).unwrap_or(0);
            let last = value.get("last_tick").and_then(|v| v.as_u64()).unwrap_or(1800);
            let tick_rate = std::fs::read_to_string(bundle_dir.join("run_manifest.json"))
                .ok()
                .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
                .and_then(|v| v.get("tick_rate_hz").and_then(|t| t.as_u64()))
                .unwrap_or(60) as u32;
            return (first, last.max(first.saturating_add(1)), tick_rate);
        }
    }
    (0, 1800, 60)
}

/// Default scene the editor renders when the bundle's per-tick state
/// hasn't yet been reconstructed (m10b-2 frame_ticker is the
/// production source; here we ship a small fixture so the preview
/// pane is not blank on open).
fn default_preview_scene() -> Vec<SceneCommand> {
    vec![
        SceneCommand::TrenchSegment {
            tile_x: 1,
            tile_y: 1,
            variant: SegmentVariant::Standard,
        },
        SceneCommand::Fortification {
            tile_x: 3,
            tile_y: 2,
            kind: FortificationKind::SandbagHigh,
        },
        SceneCommand::Fortification {
            tile_x: 4,
            tile_y: 3,
            kind: FortificationKind::MgNestStatic,
        },
    ]
}

/// Load the camera script's declared camera kinds. Used by the
/// multi-camera angle selector on open (the user sees the tracks
/// they previously authored).
fn load_camera_kinds(path: &Path) -> Option<Vec<CameraKind>> {
    let text = std::fs::read_to_string(path).ok()?;
    let script = CameraScript::from_ron_str(&text).ok()?;
    let mut kinds: Vec<CameraKind> = script.tracks.iter().map(|t| t.kind).collect();
    kinds.sort_by_key(|k| *k as u8);
    kinds.dedup();
    Some(kinds)
}

/// Build a default camera director for the editor's angle selector
/// when the bundle has no `*.camera.ron` script attached. Used by
/// the m10b-4 angle_selector to seed the picker with the three
/// canonical camera kinds (free_cam / follow_player / kill_cam) so
/// the multi-camera selector is never empty.
#[must_use]
pub fn default_angle_selector() -> AngleSelector {
    AngleSelector::default()
}

/// M10B multi-camera angle selector. Used by `cf-tools-replay-viewer
/// edit <bundle>`'s timeline UI to let the author choose which
/// camera kind is active in each `[start_tick, end_tick)` window.
///
/// VAL-M10B-024 covers the live director's per-tick routing; this
/// struct is the **CLI-side** ingest surface that converts user
/// selections into a [`CameraScript`] the export pipeline consumes.
#[derive(Debug, Clone)]
pub struct AngleSelector {
    pub tracks: Vec<AngleTrack>,
}

impl Default for AngleSelector {
    fn default() -> Self {
        Self {
            tracks: vec![
                AngleTrack {
                    kind: CameraKind::FreeCam,
                    range_start_tick: 0,
                    range_end_tick: 600,
                    selected: true,
                },
                AngleTrack {
                    kind: CameraKind::FollowPlayer,
                    range_start_tick: 600,
                    range_end_tick: 1800,
                    selected: false,
                },
                AngleTrack {
                    kind: CameraKind::KillCam,
                    range_start_tick: 1800,
                    range_end_tick: 3600,
                    selected: false,
                },
            ],
        }
    }
}

impl AngleSelector {
    /// Toggle the selection state for the track at `index`. Returns
    /// `true` if the track exists.
    pub fn toggle(&mut self, index: usize) -> bool {
        if let Some(track) = self.tracks.get_mut(index) {
            track.selected = !track.selected;
            true
        } else {
            false
        }
    }

    /// Iterate the currently-selected camera kinds in track order.
    pub fn selected_kinds(&self) -> impl Iterator<Item = CameraKind> + '_ {
        self.tracks.iter().filter(|t| t.selected).map(|t| t.kind)
    }

    /// Materialise the selected tracks into a [`CameraScript`] the
    /// export pipeline can consume. Each selected track gets one
    /// [`CameraTrack`] with placeholder keyframes at the start and
    /// end tick (production callers extend these with author-edited
    /// keyframes through the editor's pose-keyframe affordance).
    #[must_use]
    pub fn to_camera_script(&self) -> CameraScript {
        let mut tracks: Vec<CameraTrack> = Vec::with_capacity(self.tracks.len());
        for sel in self.tracks.iter().filter(|t| t.selected) {
            let placeholder_pose: Pose = [0.0, 0.0, 1.0, 0.0, 0.0, 0.0];
            tracks.push(CameraTrack {
                kind: sel.kind,
                start_tick: sel.range_start_tick,
                end_tick: sel.range_end_tick,
                keyframes: vec![
                    CameraKeyframe {
                        tick: sel.range_start_tick,
                        pose: placeholder_pose,
                    },
                    CameraKeyframe {
                        tick: sel.range_end_tick.saturating_sub(1),
                        pose: placeholder_pose,
                    },
                ],
            });
        }
        CameraScript { tracks }
    }

    /// Build the director the editor uses for its preview pane.
    pub fn to_director(script: &CameraScript) -> CameraDirector<'_> {
        CameraDirector::new(script)
    }
}

/// One row in the angle selector picker UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AngleTrack {
    pub kind: CameraKind,
    pub range_start_tick: u64,
    pub range_end_tick: u64,
    pub selected: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_stub_bundle(dir: &Path) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("run_manifest.json"), "{\"run_id\":\"r\",\"tick_rate_hz\":60}").unwrap();
        fs::write(dir.join("summary.json"), "{\"first_tick\":0,\"last_tick\":1800}").unwrap();
        fs::write(dir.join("events.jsonl"), "").unwrap();
    }

    /// VAL-M10B-035 headless path: `--headless` returns a structured
    /// envelope with documented exit code.
    #[test]
    fn edit_cmd_headless_returns_structured_envelope() {
        let tmp = tempdir().unwrap();
        write_stub_bundle(tmp.path());
        let args = EditArgs {
            bundle_dir: Some(tmp.path().to_path_buf()),
            headless: true,
            camera_script: None,
            scrub_to_tick: None,
        };
        let outcome = run_edit(args).expect("headless dispatch");
        match outcome {
            EditOutcome::Headless(env) => {
                assert_eq!(env.result, "editor_unavailable_in_headless");
                assert_eq!(env.mode, "headless");
                assert_eq!(env.exit_code, HEADLESS_EXIT_CODE);
                assert!(env.bundle.contains(tmp.path().to_string_lossy().as_ref()));
                assert!(!env.suggested_action.is_empty());
            }
            EditOutcome::Interactive { .. } => panic!("expected Headless under --headless flag"),
        }
    }

    #[test]
    fn edit_cmd_missing_bundle_errors() {
        let args = EditArgs::default();
        let err = run_edit(args).expect_err("must require bundle");
        assert!(matches!(err, EditError::MissingBundle));
    }

    #[test]
    fn edit_cmd_bundle_not_found_errors() {
        let args = EditArgs {
            bundle_dir: Some(PathBuf::from("/nonexistent/m10b/edit_test")),
            ..Default::default()
        };
        let err = run_edit(args).expect_err("nonexistent bundle must error");
        assert!(matches!(err, EditError::BundleNotFound(_)));
    }

    /// Angle selector: three canonical tracks by default
    /// (free_cam / follow_player / kill_cam).
    #[test]
    fn angle_selector_has_three_canonical_tracks() {
        let sel = AngleSelector::default();
        assert_eq!(sel.tracks.len(), 3);
        let kinds: Vec<CameraKind> = sel.tracks.iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&CameraKind::FreeCam));
        assert!(kinds.contains(&CameraKind::FollowPlayer));
        assert!(kinds.contains(&CameraKind::KillCam));
    }

    /// Angle selector: toggle flips selection state.
    #[test]
    fn angle_selector_toggle_flips_selection() {
        let mut sel = AngleSelector::default();
        let initial = sel.tracks[1].selected;
        assert!(sel.toggle(1));
        assert_ne!(sel.tracks[1].selected, initial);
    }

    /// Angle selector: out-of-range toggle returns false.
    #[test]
    fn angle_selector_toggle_invalid_index() {
        let mut sel = AngleSelector::default();
        assert!(!sel.toggle(99));
    }

    /// Angle selector: `to_camera_script` produces a CameraScript
    /// with one CameraTrack per selected angle.
    #[test]
    fn angle_selector_to_camera_script_emits_one_track_per_selection() {
        let mut sel = AngleSelector::default();
        sel.tracks[1].selected = true; // follow_player
        sel.tracks[2].selected = true; // kill_cam
        let script = sel.to_camera_script();
        assert_eq!(script.tracks.len(), 3, "free_cam + follow_player + kill_cam");
    }

    /// Angle selector: empty selection yields an empty CameraScript.
    #[test]
    fn angle_selector_empty_selection_yields_empty_script() {
        let mut sel = AngleSelector::default();
        for t in &mut sel.tracks {
            t.selected = false;
        }
        let script = sel.to_camera_script();
        assert!(script.tracks.is_empty());
    }

    /// Headless envelope: round-trips through serde_json.
    #[test]
    fn headless_envelope_serializes_to_json() {
        let env = HeadlessEnvelope::for_bundle(Path::new("/tmp/bundle"));
        let json = serde_json::to_string(&env).unwrap();
        let parsed: HeadlessEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, env);
    }
}
