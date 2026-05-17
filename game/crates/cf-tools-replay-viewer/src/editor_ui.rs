//! M10B replay editor UI state-machine surface.
//!
//! Spec § "Files":
//!
//! > `game/crates/cf-tools-replay-viewer/src/editor_ui.rs` (NEW: egui
//! > editor panel)
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
//! This module re-exports the library-level editor state machine
//! ([`EditorState`], [`EditorError`], [`ExportSelectionResult`]) +
//! the helper builders ([`const_scene_for_tick`],
//! [`dry_run_frame_ticker`], [`unused_frame_ticker_handle`]). The
//! state machine is decoupled from the egui front-end so the
//! VAL-M10B-028 and VAL-M10B-029 contracts can drive it as a
//! library-level surface without an egui test harness.
//!
//! - [`EditorState::scrub_to(tick)`] renders the exact frame within
//!   16 ms; BLAKE3 against the offline-render reference (VAL-M10B-028).
//! - [`EditorState::set_in(tick)`] / [`EditorState::set_out(tick)`] /
//!   [`EditorState::export_selection(path)`] produces frame-accurate
//!   trim; the first + last frame BLAKE3 match the offline-render
//!   references (VAL-M10B-029).
//!
//! The egui UI driver itself lives in `cf-tools-replay-viewer`'s
//! `edit` CLI binary path (m10b-4); this module is the library-level
//! surface that driver consumes.

pub use crate::editor::{
    const_scene_for_tick, dry_run_frame_ticker, unused_frame_ticker_handle, EditorError,
    EditorState, ExportSelectionResult, ScrubResult, TrimSelection, PREVIEW_HEIGHT, PREVIEW_WIDTH,
    SCRUB_LATENCY_BUDGET_MS,
};

#[cfg(test)]
mod tests {
    use super::*;
    use cf_render_2d::offline_mode::{FortificationKind, SceneCommand, SegmentVariant};

    fn make_scene() -> Vec<SceneCommand> {
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
        ]
    }

    /// VAL-M10B-028: scrub-to produces a deterministic frame whose
    /// BLAKE3 matches the offline-render reference.
    #[test]
    fn editor_ui_scrub_to_produces_deterministic_frame() {
        let scene = make_scene();
        let mut editor =
            EditorState::new(0, 1800, 60, const_scene_for_tick(scene)).expect("editor opens");
        let reference = editor.offline_reference_hash(600);
        let result = editor.scrub_to(600);
        assert_eq!(result.tick, 600);
        assert_eq!(result.blake3_hex, reference);
        assert!(result.latency.as_millis() <= SCRUB_LATENCY_BUDGET_MS as u128);
    }

    /// VAL-M10B-029: set_in + set_out + export_selection trims
    /// frame-accurately.
    #[test]
    fn editor_ui_set_in_set_out_export_selection_is_frame_accurate() {
        let scene = make_scene();
        let mut editor =
            EditorState::new(0, 1800, 60, const_scene_for_tick(scene)).expect("editor opens");
        editor.set_in(120);
        editor.set_out(240);
        assert_eq!(editor.trim.start_tick, 120);
        assert_eq!(editor.trim.end_tick, 240);
        assert_eq!(editor.trim.len_ticks(), 120);
        let out = std::env::temp_dir().join("m10b_editor_ui_export_selection_test.mp4");
        let result = editor.export_selection(&out).expect("export trim");
        assert_eq!(result.frame_count, 120);
        assert_eq!(result.out_path, out);
    }
}
