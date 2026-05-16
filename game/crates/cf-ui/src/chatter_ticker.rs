//! **M11 / c4b4ea0**: chatter caption ticker per spec § Smart-AI HUD
//! widgets. DISTINCT from the M11 event ticker; this consumer reads
//! `ai.chatter_emitted` events with `{voice_id, text, archetype}` and
//! renders per-archetype color-coded captions.

use bevy::prelude::*;

/// Max chatter lines visible per spec § "max 2 lines visible; fades after
/// 3s".
pub const CHATTER_TICKER_MAX_LINES: usize = 2;

/// Default fade in ticks at 60 Hz. 3 seconds × 60 = 180.
pub const CHATTER_TICKER_DEFAULT_DWELL_TICKS: u64 = 180;

/// One chatter line entry.
#[derive(Debug, Clone, PartialEq)]
pub struct ChatterLine {
    /// Voice id (e.g., `rifleman_alpha`).
    pub voice_id: String,
    /// Bot archetype (drives per-row color).
    pub archetype: String,
    /// Caption text.
    pub text: String,
    /// Tick the line was raised.
    pub raised_at_tick: u64,
}

/// Resource projection of the chatter caption ticker.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct ChatterTickerState {
    pub lines: Vec<ChatterLine>,
    pub dwell_ticks: u64,
}

impl ChatterTickerState {
    /// Push a new line, evicting oldest if at cap.
    pub fn push(&mut self, line: ChatterLine) {
        self.lines.push(line);
        while self.lines.len() > CHATTER_TICKER_MAX_LINES {
            self.lines.remove(0);
        }
    }

    /// Expire lines that exceeded the dwell window.
    pub fn tick(&mut self, now_tick: u64) {
        let dwell = if self.dwell_ticks == 0 {
            CHATTER_TICKER_DEFAULT_DWELL_TICKS
        } else {
            self.dwell_ticks
        };
        self.lines.retain(|l| now_tick.saturating_sub(l.raised_at_tick) < dwell);
    }

    /// HUD lines per spec — `[VOICE] TEXT` (per-archetype color rendered
    /// by cf-app; this projection is text-only for the ACC-A floor).
    #[must_use]
    pub fn formatted_lines(&self) -> Vec<String> {
        self.lines
            .iter()
            .map(|l| format!("[{}] {}", l.voice_id.to_uppercase(), l.text))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l(voice: &str, archetype: &str, text: &str, tick: u64) -> ChatterLine {
        ChatterLine {
            voice_id: voice.into(),
            archetype: archetype.into(),
            text: text.into(),
            raised_at_tick: tick,
        }
    }

    #[test]
    fn cap_evicts_oldest() {
        let mut s = ChatterTickerState::default();
        s.push(l("r1", "rifleman", "Hold fire!", 0));
        s.push(l("r2", "rifleman", "Reload!", 5));
        s.push(l("m1", "medic", "Healing!", 10));
        assert_eq!(s.lines.len(), CHATTER_TICKER_MAX_LINES);
        // Oldest evicted.
        assert!(!s.formatted_lines()[0].contains("Hold fire"));
    }

    #[test]
    fn tick_expires_old_lines() {
        let mut s = ChatterTickerState::default();
        s.push(l("r1", "rifleman", "Hold fire!", 0));
        s.tick(CHATTER_TICKER_DEFAULT_DWELL_TICKS);
        assert!(s.lines.is_empty());
    }

    #[test]
    fn formatted_includes_voice_id() {
        let mut s = ChatterTickerState::default();
        s.push(l("rifleman_alpha", "rifleman", "Hold fire!", 0));
        let lines = s.formatted_lines();
        assert!(lines[0].contains("[RIFLEMAN_ALPHA]"));
        assert!(lines[0].contains("Hold fire!"));
    }
}
