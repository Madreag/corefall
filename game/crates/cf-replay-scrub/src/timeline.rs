//! Replay timeline state — the player's scrub offset (relative to "live")
//! and the bookmark list.

use serde::{Deserialize, Serialize};

use crate::bookmark::Bookmark;

/// Default playback window in seconds per spec § Replay scrubber:
/// "Scrub through last 30s of action".
pub const WINDOW_SECONDS: u32 = 30;

/// Replay-scrubber state. cf-control owns one instance per session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayScrubState {
    /// Length of the scrubbable window in seconds.
    pub window_seconds: u32,
    /// How far back from "live" the player has scrubbed (in seconds).
    /// Clamped to `[0, window_seconds]`.
    pub current_offset_seconds: f32,
    /// Whether the scrubber widget is currently open.
    pub open: bool,
    /// Bookmarks the player has dropped (in chronological order).
    pub bookmarks: Vec<Bookmark>,
}

impl Default for ReplayScrubState {
    fn default() -> Self {
        Self {
            window_seconds: WINDOW_SECONDS,
            current_offset_seconds: 0.0,
            open: false,
            bookmarks: Vec::new(),
        }
    }
}

impl ReplayScrubState {
    /// Open the scrubber widget. Returns false when already open.
    pub fn open(&mut self) -> bool {
        if self.open {
            return false;
        }
        self.open = true;
        true
    }

    /// Close the scrubber widget + reset offset to live.
    pub fn close(&mut self) -> bool {
        if !self.open {
            return false;
        }
        self.open = false;
        self.current_offset_seconds = 0.0;
        true
    }

    /// Adjust the offset by `delta_seconds`. Negative scrubs back; positive
    /// scrubs forward (toward live). Returns the clamped offset.
    pub fn scrub(&mut self, delta_seconds: f32) -> f32 {
        let next = self.current_offset_seconds - delta_seconds;
        self.current_offset_seconds = next.clamp(0.0, self.window_seconds as f32);
        self.current_offset_seconds
    }

    /// Drop a bookmark at the given tick. Returns the new total bookmark
    /// count.
    pub fn add_bookmark(&mut self, tick: u64, label: impl Into<String>) -> usize {
        self.bookmarks.push(Bookmark::new(tick, label));
        self.bookmarks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_close_round_trips() {
        let mut s = ReplayScrubState::default();
        assert!(s.open());
        assert!(s.open);
        assert!(!s.open(), "second open is a no-op");
        assert!(s.close());
        assert!(!s.open);
        assert_eq!(s.current_offset_seconds, 0.0);
    }

    #[test]
    fn scrub_clamps_to_window() {
        let mut s = ReplayScrubState::default();
        s.scrub(-100.0);
        assert_eq!(s.current_offset_seconds, WINDOW_SECONDS as f32);
        s.scrub(1000.0);
        assert_eq!(s.current_offset_seconds, 0.0);
    }

    #[test]
    fn add_bookmark_appends() {
        let mut s = ReplayScrubState::default();
        s.add_bookmark(10, "one");
        let n = s.add_bookmark(20, "two");
        assert_eq!(n, 2);
        assert_eq!(s.bookmarks[0].tick, 10);
        assert_eq!(s.bookmarks[1].label, "two");
    }
}
