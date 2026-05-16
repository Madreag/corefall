//! **M11**: focus ring helpers per spec § Focus traversal (ACC-A-04).
//! Color helpers + 12-node iterator that consumers can call to enumerate
//! the canonical focusable node list without depending on cf-control's
//! `HUD_FOCUSABLE_NODES` const.

use bevy::prelude::*;

/// Mirror of the canonical focusable node list. cf-control owns the
/// authoritative list; cf-ui exposes it here for tests + tools that
/// don't want a cf-control dependency.
pub const FOCUSABLE_NODES: &[&str] = &[
    "hud.status_strip",
    "hud.silhouette",
    "hud.module_strip",
    "hud.stance",
    "hud.objective",
    "hud.mission",
    "hud.enemy",
    "hud.breach",
    "hud.tool",
    "hud.captions",
    "hud.banners",
    "hud.last_event",
];

/// Focus-ring color when no node has focus (transparent).
#[must_use]
pub fn focus_ring_clear() -> Color {
    Color::srgba(0.0, 0.0, 0.0, 0.0)
}

/// Focus-ring color when a node has focus.
///
/// - Standard: high-saturation amber that reads against the dark strip bg.
/// - High-contrast: pure white so the focus ring remains visible.
#[must_use]
pub fn focus_ring_color(high_contrast: bool) -> Color {
    if high_contrast {
        Color::srgb(1.0, 1.0, 1.0)
    } else {
        Color::srgb(1.0, 0.85, 0.0)
    }
}

/// Advance focus by direction. Returns the new index (wrapping).
#[must_use]
pub fn advance_focus_index(current: Option<usize>, direction: FocusDirectionUi, len: usize) -> Option<usize> {
    if len == 0 {
        return None;
    }
    match direction {
        FocusDirectionUi::Next => Some(match current {
            Some(i) => (i + 1) % len,
            None => 0,
        }),
        FocusDirectionUi::Prev => Some(match current {
            Some(i) => (i + len - 1) % len,
            None => len - 1,
        }),
        FocusDirectionUi::Clear => None,
    }
}

/// Focus direction mirror — Next/Prev/Clear. cf-control owns `Set(name)`
/// but cf-ui's iterator doesn't need it.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum FocusDirectionUi {
    Next,
    Prev,
    Clear,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focusable_nodes_count_is_12() {
        assert_eq!(FOCUSABLE_NODES.len(), 12);
    }

    #[test]
    fn next_advances_wrapping() {
        assert_eq!(advance_focus_index(None, FocusDirectionUi::Next, 12), Some(0));
        assert_eq!(advance_focus_index(Some(11), FocusDirectionUi::Next, 12), Some(0));
    }

    #[test]
    fn prev_advances_wrapping() {
        assert_eq!(advance_focus_index(None, FocusDirectionUi::Prev, 12), Some(11));
        assert_eq!(advance_focus_index(Some(0), FocusDirectionUi::Prev, 12), Some(11));
    }

    #[test]
    fn clear_drops_focus() {
        assert_eq!(advance_focus_index(Some(5), FocusDirectionUi::Clear, 12), None);
    }

    #[test]
    fn empty_list_yields_no_focus() {
        assert_eq!(advance_focus_index(Some(0), FocusDirectionUi::Next, 0), None);
    }
}
