//! **M12C**: Narration track loader (word-level timestamps) + caption-
//! ribbon highlight driver.
//!
//! Per spec § "ElevenLabs narration sync":
//!
//! - "Per-cinematic narration track — each cinematic ships one
//!   ElevenLabs `eleven_v3`-baked WAV (per-storyteller voice from M37A:
//!   cassandra_narrator_balanced_female, phoebe_chillax_warm_female,
//!   randy_random_chaotic_male, ironman_stoic_male,
//!   sandbox_neutral_narrator)."
//! - "Word-level timestamps — the bake emits
//!   `cinematic.narration_track.json` alongside the WAV with per-word
//!   `start_ms` + `end_ms`; the cinematic player uses these to drive
//!   caption ribbon highlighting + chapter markers."
//! - "Loudness contract — narration mixed at -14 LUFS; ambient music
//!   ducked to -22 LUFS during narration windows; SFX held at -16 LUFS
//!   (per M12A audio mix)."
//!
//! Per spec § Notes for the implementer: "The narration track JSON
//! schema is `{ word: string, start_ms: u32, end_ms: u32 }[]`. Empty
//! array = no caption highlighting; cinematic still plays."

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// One narration word with start/end timestamps (ms from cinematic
/// start). Per spec § Notes: `{ word: string, start_ms: u32, end_ms:
/// u32 }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NarrationWord {
    /// The literal word text (rendered in the caption ribbon).
    pub word: String,
    /// Time (ms from cinematic start) at which the word starts.
    pub start_ms: u32,
    /// Time (ms from cinematic start) at which the word ends.
    pub end_ms: u32,
}

/// Full narration track for a cinematic. Loaded from
/// `content/audio/voice/cinematic/<id>.narration_track.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NarrationTrack {
    /// Word stream in playback order. Empty = silent / no caption
    /// highlighting; cinematic still plays.
    #[serde(default)]
    pub words: Vec<NarrationWord>,
}

/// Errors raised by the narration loader.
#[derive(Debug, Error)]
pub enum NarrationLoadError {
    /// JSON parse failure.
    #[error("narration track parse failed: {0}")]
    Parse(String),
    /// Schema validation failure (word ordering, etc.).
    #[error("narration track validation failed: {0}")]
    Validation(String),
}

impl NarrationTrack {
    /// Parse a narration track from JSON bytes. Accepts both the bare
    /// `[{...}, ...]` array form (legacy) and the wrapped `{"words":
    /// [...]}` form (current).
    pub fn from_json(bytes: &[u8]) -> Result<Self, NarrationLoadError> {
        let s = std::str::from_utf8(bytes).map_err(|e| NarrationLoadError::Parse(e.to_string()))?;
        // Try wrapped form first, fall back to bare array.
        if let Ok(wrapped) = serde_json::from_str::<NarrationTrack>(s) {
            wrapped.validate()?;
            return Ok(wrapped);
        }
        let bare: Vec<NarrationWord> =
            serde_json::from_str(s).map_err(|e| NarrationLoadError::Parse(e.to_string()))?;
        let track = NarrationTrack { words: bare };
        track.validate()?;
        Ok(track)
    }

    /// Validate per-word ordering + non-zero durations.
    pub fn validate(&self) -> Result<(), NarrationLoadError> {
        let mut prev_start = 0u32;
        for (i, w) in self.words.iter().enumerate() {
            if w.start_ms < prev_start {
                return Err(NarrationLoadError::Validation(format!(
                    "word[{}] '{}' start_ms {} < previous start_ms {}",
                    i, w.word, w.start_ms, prev_start
                )));
            }
            if w.end_ms < w.start_ms {
                return Err(NarrationLoadError::Validation(format!(
                    "word[{}] '{}' end_ms {} < start_ms {}",
                    i, w.word, w.end_ms, w.start_ms
                )));
            }
            if w.word.is_empty() {
                return Err(NarrationLoadError::Validation(format!(
                    "word[{}] empty text",
                    i
                )));
            }
            prev_start = w.start_ms;
        }
        Ok(())
    }

    /// First word index whose `[start_ms, end_ms)` window contains
    /// `playhead_ms`, or `None` when the playhead is between words /
    /// outside the track.
    #[must_use]
    pub fn active_word_at(&self, playhead_ms: u32) -> Option<usize> {
        self.words
            .iter()
            .position(|w| playhead_ms >= w.start_ms && playhead_ms < w.end_ms)
    }

    /// Total narration duration (last word's `end_ms`, or 0).
    #[must_use]
    pub fn duration_ms(&self) -> u32 {
        self.words.last().map_or(0, |w| w.end_ms)
    }
}

/// One frame of caption ribbon state — drives the highlight cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordHighlightState {
    /// No active word (playhead is between words or outside track).
    Idle,
    /// Word at `index` is currently highlighted.
    Highlighted(usize),
}

/// Convenience: resolve the highlight state at `playhead_ms`.
#[must_use]
pub fn word_at_ms(track: &NarrationTrack, playhead_ms: u32) -> WordHighlightState {
    match track.active_word_at(playhead_ms) {
        Some(i) => WordHighlightState::Highlighted(i),
        None => WordHighlightState::Idle,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(words: &[(&str, u32, u32)]) -> NarrationTrack {
        NarrationTrack {
            words: words
                .iter()
                .map(|(w, s, e)| NarrationWord {
                    word: (*w).to_string(),
                    start_ms: *s,
                    end_ms: *e,
                })
                .collect(),
        }
    }

    #[test]
    fn active_word_at_resolves_inside_window() {
        let t = track(&[("the", 0, 200), ("dropship", 2_100, 2_700), ("hovers", 2_700, 3_500)]);
        assert_eq!(t.active_word_at(0), Some(0));
        assert_eq!(t.active_word_at(199), Some(0));
        // Between words.
        assert_eq!(t.active_word_at(500), None);
        assert_eq!(t.active_word_at(2_100), Some(1));
        assert_eq!(t.active_word_at(2_500), Some(1));
        assert_eq!(t.active_word_at(2_700), Some(2));
    }

    #[test]
    fn word_at_ms_returns_idle_outside_track() {
        let t = track(&[("a", 0, 100)]);
        assert_eq!(word_at_ms(&t, 0), WordHighlightState::Highlighted(0));
        assert_eq!(word_at_ms(&t, 500), WordHighlightState::Idle);
    }

    #[test]
    fn empty_track_is_valid() {
        let t = NarrationTrack { words: vec![] };
        t.validate().expect("empty is valid");
        assert_eq!(t.active_word_at(1_000), None);
        assert_eq!(t.duration_ms(), 0);
    }

    #[test]
    fn rejects_out_of_order_words() {
        let t = track(&[("b", 500, 800), ("a", 100, 400)]);
        assert!(t.validate().is_err());
    }

    #[test]
    fn rejects_end_before_start() {
        let t = track(&[("a", 800, 500)]);
        assert!(t.validate().is_err());
    }

    #[test]
    fn parses_bare_array_form() {
        let json = r#"[
            {"word":"the","start_ms":0,"end_ms":200},
            {"word":"dropship","start_ms":2100,"end_ms":2700}
        ]"#;
        let t = NarrationTrack::from_json(json.as_bytes()).expect("parse");
        assert_eq!(t.words.len(), 2);
        assert_eq!(t.words[1].word, "dropship");
    }

    #[test]
    fn parses_wrapped_form() {
        let json = r#"{
            "words": [
                {"word":"a","start_ms":0,"end_ms":100}
            ]
        }"#;
        let t = NarrationTrack::from_json(json.as_bytes()).expect("parse");
        assert_eq!(t.words.len(), 1);
    }
}
