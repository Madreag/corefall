//! M10B multi-camera angle selector.
//!
//! Spec § "Files":
//!
//! > `game/crates/cf-tools-replay-viewer/src/angle_selector.rs` (NEW:
//! > multi-camera angle picker)
//!
//! Spec § "Player-facing behavior":
//!
//! > **Multi-camera angles per replay.** Up to 4 camera tracks
//! > (`free_cam`, `follow_player`, `objective_cam`, `kill_cam`) can
//! > be cut between in the editor; each camera's per-tick pose is
//! > captured into a `*.camera.ron` script that survives editing
//! > sessions.
//!
//! This module re-exports the CLI-side angle selector
//! ([`AngleSelector`], [`AngleTrack`], [`default_angle_selector`])
//! that the `cf-tools-replay-viewer edit` editor UX consumes. The
//! struct converts user selections into a
//! [`cf_replay_export::camera_script::CameraScript`] the export
//! pipeline consumes, then hands it to
//! [`cf_replay_export::camera_director::CameraDirector`] for per-tick
//! pose interpolation.
//!
//! VAL-M10B-024 covers the live director's per-tick routing; this
//! module is the CLI-side ingest surface that materialises user
//! selections into the on-disk `*.camera.ron` script grammar.

pub use crate::edit_cmd::{default_angle_selector, AngleSelector, AngleTrack};

#[cfg(test)]
mod tests {
    use super::*;
    use cf_replay_export::camera_script::CameraKind;

    /// Spec § Player-facing behavior names exactly 4 canonical camera
    /// kinds (free_cam / follow_player / objective_cam / kill_cam).
    /// The default selector pre-populates three of them as a useful
    /// starting layout; objective_cam is available via the
    /// `CameraKind` enum for explicit author selection.
    #[test]
    fn angle_selector_default_pre_populates_three_canonical_tracks() {
        let sel = default_angle_selector();
        assert_eq!(sel.tracks.len(), 3);
        let kinds: Vec<CameraKind> = sel.tracks.iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&CameraKind::FreeCam));
        assert!(kinds.contains(&CameraKind::FollowPlayer));
        assert!(kinds.contains(&CameraKind::KillCam));
    }

    /// VAL-M10B-024: the selector materialises into a CameraScript
    /// the director can consume. One selected track == one
    /// CameraTrack on the resulting script.
    #[test]
    fn angle_selector_materialises_selected_tracks_into_camera_script() {
        let mut sel = default_angle_selector();
        for t in &mut sel.tracks {
            t.selected = true;
        }
        let script = sel.to_camera_script();
        assert_eq!(script.tracks.len(), sel.tracks.len());
    }

    /// VAL-M10B-024: omitting all selections produces an empty
    /// CameraScript. The director treats this as "no per-tick pose
    /// override active" — the live spectator director takes over.
    #[test]
    fn angle_selector_empty_selection_yields_empty_camera_script() {
        let mut sel = default_angle_selector();
        for t in &mut sel.tracks {
            t.selected = false;
        }
        let script = sel.to_camera_script();
        assert!(script.tracks.is_empty());
    }
}
