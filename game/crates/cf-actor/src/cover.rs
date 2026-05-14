//! M6: cover state (whether an actor is currently in cover and how much).
//!
//! "Cover" in M6 is the side-view counterpart of CCCP's per-tile occluder query.
//! The engine samples terrain solidness on each side of the actor and produces
//! a [`CoverState`]; cf-ai consumes it for retreat/peek doctrine, and the HUD
//! displays the cover indicator near the reticle.

use serde::{Deserialize, Serialize};

/// Which side of the actor a hard cover surface is on.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverSide {
    None = 0,
    Left = 1,
    Right = 2,
    Both = 3,
}

impl Default for CoverSide {
    fn default() -> Self {
        CoverSide::None
    }
}

impl CoverSide {
    pub fn as_str(self) -> &'static str {
        match self {
            CoverSide::None => "none",
            CoverSide::Left => "left",
            CoverSide::Right => "right",
            CoverSide::Both => "both",
        }
    }

    /// Fold two cover sides (e.g. left + right) into the union side.
    pub fn fold(self, other: CoverSide) -> CoverSide {
        match (self, other) {
            (CoverSide::None, x) | (x, CoverSide::None) => x,
            (a, b) if a == b => a,
            _ => CoverSide::Both,
        }
    }
}

/// Per-actor cover state. Threaded into [`crate::ActorState::cover_state`] from
/// M6 onward.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CoverState {
    pub side: CoverSide,
    /// 0..1 cover-effectiveness. 0 = no cover; 1 = full hard cover.
    pub effectiveness: f32,
    /// True when the actor is peeking out of cover (lean active).
    pub peeking: bool,
}

impl Default for CoverState {
    fn default() -> Self {
        Self {
            side: CoverSide::None,
            effectiveness: 0.0,
            peeking: false,
        }
    }
}

impl CoverState {
    pub fn open() -> Self {
        Self::default()
    }

    pub fn in_cover(side: CoverSide, effectiveness: f32) -> Self {
        Self {
            side,
            effectiveness: effectiveness.clamp(0.0, 1.0),
            peeking: false,
        }
    }

    pub fn is_in_cover(self) -> bool {
        self.side != CoverSide::None && self.effectiveness > 0.1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_left_right_gives_both() {
        assert_eq!(CoverSide::Left.fold(CoverSide::Right), CoverSide::Both);
        assert_eq!(CoverSide::Right.fold(CoverSide::Left), CoverSide::Both);
    }

    #[test]
    fn fold_none_keeps_other() {
        assert_eq!(CoverSide::None.fold(CoverSide::Left), CoverSide::Left);
        assert_eq!(CoverSide::Right.fold(CoverSide::None), CoverSide::Right);
    }

    #[test]
    fn cover_threshold() {
        let weak = CoverState::in_cover(CoverSide::Left, 0.05);
        assert!(!weak.is_in_cover());
        let strong = CoverState::in_cover(CoverSide::Left, 0.5);
        assert!(strong.is_in_cover());
    }

    #[test]
    fn effectiveness_clamped() {
        let over = CoverState::in_cover(CoverSide::Left, 5.0);
        assert!(over.effectiveness <= 1.0);
    }
}
