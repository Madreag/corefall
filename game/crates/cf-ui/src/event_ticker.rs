//! **M11**: last-event ticker widget per spec § HUD EVENT_TICKER zone.
//! Renders one event at a time using the M10 plain-language renderer's
//! output; auto-dismisses after 4 seconds.

use bevy::prelude::*;

/// Default auto-dismiss in ticks at 60 Hz. 4 seconds × 60 ticks = 240.
pub const EVENT_TICKER_DEFAULT_DWELL_TICKS: u64 = 240;

/// One ticker entry. cf-app writes from the M10 renderer output.
#[derive(Debug, Clone, PartialEq)]
pub struct EventTickerEntry {
    /// Stable id (e.g., `objective.completed.123`).
    pub id: String,
    /// Plain-language text per M10's `cf_tools_replay_viewer::render_event`.
    pub text: String,
    /// Tick when the entry was raised.
    pub raised_at_tick: u64,
    /// Severity glyph carried over from the source event ([!!] / [!] / [*]).
    pub severity_glyph: String,
}

/// Resource projection of the last-event ticker. Single-slot per spec
/// (max 1 visible; oldest evicts on push).
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct EventTickerState {
    pub entry: Option<EventTickerEntry>,
    pub dwell_ticks: u64,
}

impl EventTickerState {
    /// Push a new entry, replacing whatever's there.
    pub fn push(&mut self, entry: EventTickerEntry) {
        self.entry = Some(entry);
    }

    /// Expire the entry if its dwell time has elapsed.
    pub fn tick(&mut self, now_tick: u64) {
        let dwell = if self.dwell_ticks == 0 {
            EVENT_TICKER_DEFAULT_DWELL_TICKS
        } else {
            self.dwell_ticks
        };
        if let Some(ref e) = self.entry {
            if now_tick.saturating_sub(e.raised_at_tick) >= dwell {
                self.entry = None;
            }
        }
    }

    /// HUD line for the current entry, or empty when no entry is active.
    #[must_use]
    pub fn line(&self) -> String {
        match &self.entry {
            Some(e) => format!("EVENT: {} {}", e.severity_glyph, e.text),
            None => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_ticker_renders_empty_line() {
        let s = EventTickerState::default();
        assert_eq!(s.line(), "");
    }

    #[test]
    fn push_replaces_entry() {
        let mut s = EventTickerState::default();
        s.push(EventTickerEntry {
            id: "1".into(),
            text: "foo".into(),
            raised_at_tick: 0,
            severity_glyph: "[*]".into(),
        });
        s.push(EventTickerEntry {
            id: "2".into(),
            text: "bar".into(),
            raised_at_tick: 5,
            severity_glyph: "[!]".into(),
        });
        assert!(s.line().contains("bar"));
    }

    #[test]
    fn dwell_expiry_clears_entry() {
        let mut s = EventTickerState::default();
        s.push(EventTickerEntry {
            id: "1".into(),
            text: "foo".into(),
            raised_at_tick: 0,
            severity_glyph: "[*]".into(),
        });
        s.tick(EVENT_TICKER_DEFAULT_DWELL_TICKS);
        assert!(s.entry.is_none());
    }

    #[test]
    fn before_dwell_keeps_entry() {
        let mut s = EventTickerState::default();
        s.push(EventTickerEntry {
            id: "1".into(),
            text: "foo".into(),
            raised_at_tick: 0,
            severity_glyph: "[*]".into(),
        });
        s.tick(EVENT_TICKER_DEFAULT_DWELL_TICKS - 1);
        assert!(s.entry.is_some());
    }
}
