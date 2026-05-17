//! M10B chapter-timeline overlay integration tests.
//!
//! Verification command: `cargo test -p cf-replay-export overlay_chapter_timeline`
//! (expect: strip_marks_chapter_offsets PASS).
//!
//! VAL-M10B-OVERLAY-CHAPTERTL-FILE: rendered strip's tick-mark
//! x-positions correspond to the chapter offsets emitted by the
//! chapter-marker derivation pass (within ±1 px).

use cf_replay_export::chapter_derivation::ChapterMarker;
use cf_replay_export::overlay_chapter_timeline::ChapterTimelineOverlay;

fn marker(tick: u64, title: &str) -> ChapterMarker {
    ChapterMarker {
        tick_index: tick,
        start_time_seconds: tick as f64 / 60.0,
        title: title.into(),
        event_type: "actor_status_changed".into(),
        event_id: format!("ev_{tick}"),
        category: Some("death".into()),
    }
}

#[test]
fn overlay_chapter_timeline_strip_marks_chapter_offsets() {
    let overlay = ChapterTimelineOverlay::default();
    let total_ticks = 108_000u64;
    let chapters: Vec<ChapterMarker> = vec![
        marker(0, "start"),
        marker(27_000, "q1"),
        marker(54_000, "half"),
        marker(81_000, "q3"),
        marker(107_999, "end"),
    ];
    let marks = overlay.tick_marks(&chapters, total_ticks);
    assert_eq!(marks.len(), 5);

    for (i, mark) in marks.iter().enumerate() {
        let chapter = &chapters[i];
        let ratio = chapter.tick_index as f64 / total_ticks as f64;
        let expected_x = overlay.aoi_x as f64 + ratio * overlay.aoi_width as f64;
        let actual_x = mark.x_pixels as f64;
        let diff = (expected_x - actual_x).abs();
        assert!(
            diff <= 1.0,
            "tick {} expected px ~{:.2}, got {} (Δ={:.2})",
            chapter.tick_index,
            expected_x,
            mark.x_pixels,
            diff
        );
    }
}

#[test]
fn overlay_chapter_timeline_handles_26_chapter_fixture() {
    let overlay = ChapterTimelineOverlay::default();
    let chapters: Vec<ChapterMarker> = (0..26).map(|i| marker((i as u64) * 4000, "ch")).collect();
    let marks = overlay.tick_marks(&chapters, 26 * 4000);
    assert_eq!(marks.len(), 26);
    // First mark at aoi_x.
    assert_eq!(marks[0].x_pixels, overlay.aoi_x);
    // Last mark close to right edge.
    let right_edge = overlay.aoi_x + overlay.aoi_width;
    assert!(marks[25].x_pixels <= right_edge);
}
