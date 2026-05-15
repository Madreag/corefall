//! cf-replay-scrub — M8 mini replay-scrubber stub. Spec § Replay scrubber:
//! a 30-second window at the bottom of the HUD lets the player scrub back
//! through recent action + drop bookmarks for later review. Forward-compat
//! for M33+ replay browser.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod bookmark;
pub mod timeline;

pub use bookmark::Bookmark;
pub use timeline::{ReplayScrubState, WINDOW_SECONDS};
