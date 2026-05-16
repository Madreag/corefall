//! Bookmark — a labelled tick the player can return to.

use serde::{Deserialize, Serialize};

/// One bookmark.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bookmark {
    /// Tick at which the bookmark was dropped.
    pub tick: u64,
    /// Player-supplied label.
    pub label: String,
}

impl Bookmark {
    /// Build a new bookmark at `tick` with `label`.
    pub fn new(tick: u64, label: impl Into<String>) -> Self {
        Self {
            tick,
            label: label.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_round_trips() {
        let b = Bookmark::new(42, "Got the kill");
        assert_eq!(b.tick, 42);
        assert_eq!(b.label, "Got the kill");
    }
}
