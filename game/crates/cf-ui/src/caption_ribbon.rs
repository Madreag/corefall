//! **M12C**: Cinematic lower-third caption ribbon — separate from the
//! gameplay caption strip (`cf-ui::captions`).
//!
//! Per spec § Crates / modules touched:
//!
//! > `cf-ui::caption_ribbon` (NEW) — Lower-third subtitle ribbon for
//! > cinematics (separate from gameplay captions); honors
//! > `caption_visible` + reduce-motion.
//!
//! The ribbon highlights words as the cinematic playhead crosses each
//! word's `start_ms` (driven by `cf-cinematic::narration_sync`). When
//! captions are disabled the ribbon is hidden BUT the
//! `cinematic.narration_word` events still fire for replay parity
//! (acceptance criterion "cinematic.narration_word events still emit
//! (replay parity) / the caption ribbon is hidden").

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// ribbon; never silent without captions when `captions_enabled=true`."
///
/// Pure helper computing whether the cinematic caption ribbon should be
/// visible given:
///
/// - `captions_enabled` — global captions setting (M11 caption mode !=
///   Off OR M12C `cinematic_captions_enabled` toggle).
/// - `reduce_motion` — accessibility flag; the ribbon stays visible
///   (it's not a motion surface) but the renderer drops fades.
///
/// Returns `true` when the cinematic caption ribbon should be drawn.
#[must_use]
pub fn caption_ribbon_visible(captions_enabled: bool, _reduce_motion: bool) -> bool {
    // the caption ribbon is hidden". Reduce-motion does NOT hide the
    // ribbon — it only suppresses fade animations.
    captions_enabled
}

/// One word entry on the ribbon. Mirror of
/// `cf_cinematic::narration_sync::NarrationWord` decoupled from the
/// cinematic crate so cf-ui doesn't depend on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RibbonWord {
    /// Word text.
    pub word: String,
    /// Start ms from cinematic start.
    pub start_ms: u32,
    /// End ms from cinematic start.
    pub end_ms: u32,
}

/// Bevy `Resource` that cf-app mirrors per frame from the live
/// cinematic kernel. The renderer reads it to draw the ribbon.
#[derive(Resource, Debug, Default, Clone, PartialEq)]
pub struct CaptionRibbonState {
    /// True while a cinematic is active.
    pub active: bool,
    /// True when the ribbon should be visible (captions_enabled AND
    /// reduce_motion-aware contrast OK). False = ribbon hidden but
    /// events still flow.
    pub visible: bool,
    /// All words in the active cinematic's narration track.
    pub words: Vec<RibbonWord>,
    /// 0-based index of the word currently highlighted, or `None`
    /// between words / outside the track.
    pub active_word_index: Option<usize>,
    /// True when the player paused the cinematic — the renderer pins
    /// the active word indefinitely until resume.
    pub paused: bool,
    /// Cinematic id (debug / accessibility id).
    pub cinematic_id: String,
}

impl CaptionRibbonState {
    /// Clear the ribbon (called on `cinematic.ended` / `cinematic.skipped`).
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Set the active word from a narration word index. Pass `None` to
    /// clear (idle between words).
    pub fn set_active_word(&mut self, index: Option<usize>) {
        self.active_word_index = index;
    }

    /// Update the word stream from the cinematic kernel's narration
    /// track. Idempotent — replaces the word list wholesale.
    pub fn set_words(&mut self, words: Vec<RibbonWord>) {
        self.words = words;
    }

    /// is hidden". Returns `true` when the renderer should draw the
    /// ribbon.
    #[must_use]
    pub fn should_render(&self) -> bool {
        self.active && self.visible
    }

    /// Returns the highlighted word's text, or empty when no word is
    /// active or the ribbon is hidden.
    #[must_use]
    pub fn highlighted_word(&self) -> Option<&str> {
        if !self.active {
            return None;
        }
        self.active_word_index
            .and_then(|i| self.words.get(i))
            .map(|w| w.word.as_str())
    }

    /// Concatenate every word with the active word marked. Useful for
    /// accessibility-friendly screen-reader output.
    #[must_use]
    pub fn full_line(&self) -> String {
        let mut s = String::new();
        for (i, w) in self.words.iter().enumerate() {
            if i > 0 {
                s.push(' ');
            }
            if Some(i) == self.active_word_index {
                s.push_str(&format!("[{}]", w.word));
            } else {
                s.push_str(&w.word);
            }
        }
        s
    }

    /// subtitle ribbon". Convenience setter that wires the visible flag
    /// to the predicate's output. cf-app's bridge calls this each frame
    /// from the live `Settings.captions` + `Settings.reduced_motion`.
    pub fn update_visibility(&mut self, captions_enabled: bool, reduce_motion: bool) {
        self.visible = caption_ribbon_visible(captions_enabled, reduce_motion);
    }
}

/// Plugin that registers `CaptionRibbonState`. cf-app's bridge owns the
/// per-frame mirror system.
#[derive(Default)]
pub struct CaptionRibbonPlugin;

impl Plugin for CaptionRibbonPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CaptionRibbonState>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words() -> Vec<RibbonWord> {
        vec![
            RibbonWord {
                word: "the".to_string(),
                start_ms: 0,
                end_ms: 200,
            },
            RibbonWord {
                word: "dropship".to_string(),
                start_ms: 2_100,
                end_ms: 2_700,
            },
            RibbonWord {
                word: "hovers".to_string(),
                start_ms: 2_700,
                end_ms: 3_500,
            },
        ]
    }

    #[test]
    fn default_is_inactive_and_hidden() {
        let s = CaptionRibbonState::default();
        assert!(!s.should_render());
        assert!(s.highlighted_word().is_none());
    }

    #[test]
    fn should_render_requires_active_and_visible() {
        let mut s = CaptionRibbonState::default();
        s.active = true;
        assert!(!s.should_render());
        s.visible = true;
        assert!(s.should_render());
    }

    #[test]
    fn highlighted_word_reads_from_index() {
        let mut s = CaptionRibbonState::default();
        s.active = true;
        s.visible = true;
        s.set_words(words());
        s.set_active_word(Some(1));
        assert_eq!(s.highlighted_word(), Some("dropship"));
    }

    #[test]
    fn full_line_marks_active_word_with_brackets() {
        let mut s = CaptionRibbonState::default();
        s.active = true;
        s.visible = true;
        s.set_words(words());
        s.set_active_word(Some(1));
        assert_eq!(s.full_line(), "the [dropship] hovers");
    }

    #[test]
    fn clear_resets_state() {
        let mut s = CaptionRibbonState::default();
        s.active = true;
        s.visible = true;
        s.set_words(words());
        s.set_active_word(Some(1));
        s.clear();
        assert!(!s.active);
        assert!(s.words.is_empty());
    }

    #[test]
    fn caption_ribbon_visible_honors_captions_enabled() {
        assert!(caption_ribbon_visible(true, false));
        assert!(!caption_ribbon_visible(false, false));
        // Reduce-motion does NOT hide the ribbon.
        assert!(caption_ribbon_visible(true, true));
    }

    #[test]
    fn update_visibility_wires_captions_enabled_into_state() {
        let mut s = CaptionRibbonState::default();
        s.active = true;
        s.update_visibility(true, false);
        assert!(s.should_render());
        s.update_visibility(false, false);
        assert!(!s.should_render(), "captions_enabled=false hides ribbon");
    }
}
