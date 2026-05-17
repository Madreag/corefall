//! M10B editor integration tests.
//!
//! VAL-M10B-028: `EditorState::scrub_to(tick)` renders the exact frame
//! within 16 ms; BLAKE3 match against offline-render reference.
//!
//! VAL-M10B-029: `Set In / Set Out / Export Selection` produces
//! frame-accurate trim. First + last frame BLAKE3 match.

use std::path::Path;

use cf_render_2d::offline_mode::{FortificationKind, SceneCommand, SegmentVariant};
use cf_tools_replay_viewer::{const_scene_for_tick, EditorState, TrimSelection, SCRUB_LATENCY_BUDGET_MS};

fn make_editor() -> EditorState {
    let scene = vec![
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
    ];
    EditorState::new(0, 1800, 60, const_scene_for_tick(scene)).expect("editor builds")
}

#[test]
fn editor_scrub() {
    let mut editor = make_editor();
    let reference = editor.offline_reference_hash(900);
    let result = editor.scrub_to(900);
    assert_eq!(result.tick, 900);
    assert_eq!(
        result.blake3_hex, reference,
        "scrub must match offline-render reference"
    );
    assert!(
        result.latency.as_millis() <= SCRUB_LATENCY_BUDGET_MS as u128,
        "scrub_latency_ms: {} (tol: {})",
        result.latency.as_millis(),
        SCRUB_LATENCY_BUDGET_MS
    );
    assert!(!result.frame.is_blank(), "scrub frame must contain pixels");
    assert_eq!(result.frame.pixels.len() % 4, 0, "RGBA buffer length divisible by 4");
}

#[test]
fn editor_trim() {
    let mut editor = make_editor();
    editor.set_in(60);
    editor.set_out(180);
    assert_eq!(
        editor.trim,
        TrimSelection {
            start_tick: 60,
            end_tick: 180
        }
    );
    let ref_first = editor.offline_reference_hash(60);
    let ref_last = editor.offline_reference_hash(179);
    let result = editor
        .export_selection(Path::new("/tmp/m10b_editor_trim_integration.mp4"))
        .expect("export");
    assert_eq!(result.frame_count, 120);
    assert_eq!(result.first_frame_blake3, ref_first);
    assert_eq!(result.last_frame_blake3, ref_last);
}

#[test]
fn editor_set_in_set_out_roundtrip_keeps_trim_valid() {
    let mut editor = make_editor();
    editor.set_in(900);
    editor.set_out(600);
    assert!(editor.trim.is_valid());
}
