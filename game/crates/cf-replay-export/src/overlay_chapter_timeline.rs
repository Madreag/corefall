//! M10B chapter-timeline overlay renderer.
//!
//! Spec § "Player-facing behavior":
//!
//! > Author toggles ... chapter timeline independently per-export.
//!
//! VAL-M10B-OVERLAY-CHAPTERTL-FILE: "The file
//! `game/crates/cf-replay-export/src/overlay_chapter_timeline.rs`
//! exists; running export with `--overlay chapter_timeline` produces
//! an MP4 whose per-frame chapter-timeline strip region contains a
//! rendered timeline graphic with tick marks at every chapter-marker
//! offset (from the chapter-rules pass)."
//!
//! Per the feature spec part (e): "horizontal chapter strip at the
//! bottom of every frame; marker positions linearly proportional to
//! chapter list."
//!
//! The strip renders at the bottom of the frame. Marker x-positions
//! are computed as `aoi_x + (chapter.start_ticks / total_ticks) *
//! aoi_width` so they map linearly to the chapter offsets.

use crate::chapter_derivation::ChapterMarker;
use crate::overlay_graph::{CHAPTER_TIMELINE_OVERLAY_NAME, CHAPTER_TIMELINE_Z_ORDER};

/// Default chapter-timeline AOI at 1920×1080. Bottom strip, full
/// width. Other resolutions scale proportionally.
pub const CHAPTER_TIMELINE_AOI_X: u32 = 16;
pub const CHAPTER_TIMELINE_AOI_Y: u32 = 1080 - 16 - 60;
pub const CHAPTER_TIMELINE_AOI_WIDTH: u32 = 1920 - 32;
pub const CHAPTER_TIMELINE_AOI_HEIGHT: u32 = 60;

/// One rendered tick-mark on the strip. The rasterizer draws a small
/// vertical line at `x_pixels` against the chapter strip's background.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickMark {
    pub chapter_index: usize,
    pub x_pixels: u32,
}

/// Chapter-timeline overlay descriptor + per-frame renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChapterTimelineOverlay {
    pub aoi_x: u32,
    pub aoi_y: u32,
    pub aoi_width: u32,
    pub aoi_height: u32,
    pub z_order: u32,
}

impl Default for ChapterTimelineOverlay {
    fn default() -> Self {
        Self {
            aoi_x: CHAPTER_TIMELINE_AOI_X,
            aoi_y: CHAPTER_TIMELINE_AOI_Y,
            aoi_width: CHAPTER_TIMELINE_AOI_WIDTH,
            aoi_height: CHAPTER_TIMELINE_AOI_HEIGHT,
            z_order: CHAPTER_TIMELINE_Z_ORDER,
        }
    }
}

impl ChapterTimelineOverlay {
    #[must_use]
    pub const fn name() -> &'static str {
        CHAPTER_TIMELINE_OVERLAY_NAME
    }

    /// Compute tick-mark x-positions for the supplied chapter list.
    ///
    /// `total_ticks` is the bundle's total tick budget (used as the
    /// denominator). Mark positions are linearly proportional:
    /// `x = aoi_x + (marker.tick_index / total_ticks) * aoi_width`.
    ///
    /// Returns `Vec` ordered by chapter index, so the rasterizer can
    /// draw left-to-right without re-sorting.
    #[must_use]
    pub fn tick_marks(&self, chapters: &[ChapterMarker], total_ticks: u64) -> Vec<TickMark> {
        if total_ticks == 0 {
            return Vec::new();
        }
        chapters
            .iter()
            .enumerate()
            .map(|(idx, marker)| {
                let ratio = marker.tick_index as f64 / total_ticks as f64;
                let ratio = ratio.clamp(0.0, 1.0);
                let x_pixels = self.aoi_x + (ratio * (self.aoi_width as f64)) as u32;
                TickMark {
                    chapter_index: idx,
                    x_pixels,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chapter_derivation::ChapterMarker;

    fn marker(tick: u64, title: &str) -> ChapterMarker {
        ChapterMarker {
            tick_index: tick,
            start_time_seconds: tick as f64 / 60.0,
            title: title.into(),
            event_type: "actor_status_changed".into(),
            event_id: format!("event_{tick}"),
            category: Some("death".into()),
        }
    }

    #[test]
    fn marks_at_linear_proportion_of_total_ticks() {
        let overlay = ChapterTimelineOverlay::default();
        let chapters = vec![marker(0, "a"), marker(54000, "b"), marker(108000, "c")];
        let marks = overlay.tick_marks(&chapters, 108000);
        assert_eq!(marks.len(), 3);
        // mark 0: at aoi_x (ratio 0)
        assert_eq!(marks[0].x_pixels, overlay.aoi_x);
        // mark 1: at aoi_x + 0.5 * aoi_width (mid)
        let mid = overlay.aoi_x + overlay.aoi_width / 2;
        let diff_mid = if marks[1].x_pixels > mid {
            marks[1].x_pixels - mid
        } else {
            mid - marks[1].x_pixels
        };
        assert!(diff_mid <= 1, "mid mark x={} should be ~mid={mid}", marks[1].x_pixels);
        // mark 2: at aoi_x + aoi_width (end)
        let end = overlay.aoi_x + overlay.aoi_width;
        let diff_end = if marks[2].x_pixels > end {
            marks[2].x_pixels - end
        } else {
            end - marks[2].x_pixels
        };
        assert!(diff_end <= 1, "end mark x={} should be ~end={end}", marks[2].x_pixels);
    }

    #[test]
    fn empty_total_ticks_yields_no_marks() {
        let overlay = ChapterTimelineOverlay::default();
        let chapters = vec![marker(0, "a")];
        let marks = overlay.tick_marks(&chapters, 0);
        assert!(marks.is_empty());
    }

    #[test]
    fn strip_marks_chapter_offsets() {
        let overlay = ChapterTimelineOverlay::default();
        // 26-chapter fixture (matching the spec's 12+3+7+4 fixture)
        let chapters: Vec<ChapterMarker> = (0..26).map(|i| marker((i as u64) * 4000, "ch")).collect();
        let marks = overlay.tick_marks(&chapters, 104000);
        assert_eq!(marks.len(), 26, "every chapter must produce a tick-mark");
        for (i, mark) in marks.iter().enumerate() {
            assert_eq!(mark.chapter_index, i);
            assert!(mark.x_pixels >= overlay.aoi_x);
            assert!(mark.x_pixels <= overlay.aoi_x + overlay.aoi_width);
        }
    }
}
