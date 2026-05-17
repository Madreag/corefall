//! **M12A** § Deterministic replay parity for audio events.
//!
//! Per spec acceptance criterion:
//!
//! ```text
//! Scenario: Deterministic replay parity
//!   Given a run bundle with audio events
//!   When cf-headless replay runs
//!   Then audio events fire in deterministic order
//!   And cosmetic=true audio events skipped (per M4 cosmetic flag)
//!   Per-tick checksum unchanged
//!   Replay produces same final state
//! ```
//!
//! Audio events are flagged `cosmetic=true` by default — they do NOT
//! enter the determinism checksum. BUT they MUST fire in the same order
//! per replay tick so a recorded → replayed bundle shows the same
//! caption stream. This module owns the per-tick audio-event queue +
//! the canonical sort order.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::positional::AudioDirection;

/// **M12A** § One audio event scheduled for replay-deterministic
/// playback. Mirrors the `cf-replay::event::audio.event_played` JSON
/// schema field-for-field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AudioPlaybackEvent {
    pub tick: u64,
    pub sequence: u32,
    pub canonical_name: String,
    pub bus: String,
    pub direction: String,
    pub gain: f32,
    pub cosmetic: bool,
}

impl AudioPlaybackEvent {
    /// Construct a new event. Always marks `cosmetic = true` per the
    /// M12A spec § Replay parity contract.
    pub fn new(
        tick: u64,
        sequence: u32,
        canonical_name: impl Into<String>,
        bus: impl Into<String>,
        direction: AudioDirection,
        gain: f32,
    ) -> Self {
        Self {
            tick,
            sequence,
            canonical_name: canonical_name.into(),
            bus: bus.into(),
            direction: direction.label().to_string(),
            gain: gain.clamp(0.0, 1.0),
            cosmetic: true,
        }
    }

    /// Canonical event key used for `(tick, sequence, canonical_name)`
    /// sorting. Two events with identical keys are byte-identical replays.
    pub fn replay_key(&self) -> (u64, u32, &str) {
        (self.tick, self.sequence, self.canonical_name.as_str())
    }
}

/// **M12A** § Replay queue. cf-control's engine pushes events here when
/// the sim emits an audio cue; cf-app's bevy-audio adapter drains the
/// queue per frame and dispatches each event in order.
#[derive(Debug, Default, Clone)]
pub struct AudioReplayQueue {
    pending: VecDeque<AudioPlaybackEvent>,
    sequence_for_tick: u32,
    current_tick: u64,
}

impl AudioReplayQueue {
    /// Push a new event onto the queue. Auto-increments the per-tick
    /// sequence counter; rolls over to 0 when the tick advances.
    pub fn push(&mut self, mut event: AudioPlaybackEvent) {
        if event.tick != self.current_tick {
            self.current_tick = event.tick;
            self.sequence_for_tick = 0;
        }
        event.sequence = self.sequence_for_tick;
        self.sequence_for_tick = self.sequence_for_tick.saturating_add(1);
        self.pending.push_back(event);
    }

    /// Drain every event scheduled up to + including `up_to_tick` in
    /// canonical replay order. cf-app's adapter calls this per Bevy
    /// frame with the current sim tick.
    pub fn drain_up_to(&mut self, up_to_tick: u64) -> Vec<AudioPlaybackEvent> {
        let mut out = Vec::new();
        while let Some(front) = self.pending.front() {
            if front.tick > up_to_tick {
                break;
            }
            if let Some(ev) = self.pending.pop_front() {
                out.push(ev);
            }
        }
        // Canonical sort: tick ascending, sequence ascending, name ascending.
        // This is already the push order; sorting is a safety net for any
        // multi-producer caller that interleaves out-of-order writes.
        out.sort_by(|a, b| a.replay_key().cmp(&b.replay_key()));
        out
    }

    /// Drain every pending event regardless of tick. Used at end-of-run.
    pub fn drain_all(&mut self) -> Vec<AudioPlaybackEvent> {
        let mut out: Vec<_> = self.pending.drain(..).collect();
        out.sort_by(|a, b| a.replay_key().cmp(&b.replay_key()));
        out
    }

    /// Pending event count.
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

/// **M12A** § Spec acceptance — "Per-tick checksum unchanged" for
/// cosmetic events. This helper filters the SIM event stream so the
/// determinism checksum only hashes non-cosmetic events. cf-control's
/// existing `cosmetic` flag on the M4 envelope already does this; the
/// helper here is the audio-specific filter for tests.
#[must_use]
pub fn is_cosmetic_audio_event(category: &str) -> bool {
    matches!(category, "audio")
}

/// **M12B** § Per spec § Notes for the implementer:
///
/// > the 4 new replay events MUST register in
/// > `cf-audio::deterministic_replay::is_cosmetic_audio_event`. Treating
/// > spatial resolution as non-cosmetic would mean the replay verifier
/// > compares audio output, which we explicitly DO NOT want.
///
/// The audio-event registry of M12B-cosmetic event types is enumerated
/// here so the test suite + replay verifier can pattern-match on the
/// exact set instead of approximating with a `category == "audio"`
/// match.
pub const M12B_COSMETIC_EVENT_TYPES: &[&str] = &[
    "spatial_resolved",
    "reverb_applied",
    "occluded",
    "doppler_shifted",
];

/// **M12B** § Two-argument cosmetic-event classifier. Returns `true` for
/// the existing `audio.event_played` event AND each of the 4 new M12B
/// replay event types so they're always excluded from the determinism
/// checksum.
#[must_use]
pub fn is_cosmetic_audio_event_for(category: &str, event_type: &str) -> bool {
    if is_cosmetic_audio_event(category) {
        return true;
    }
    matches!(
        (category, event_type),
        ("audio", "spatial_resolved")
            | ("audio", "reverb_applied")
            | ("audio", "occluded")
            | ("audio", "doppler_shifted")
            | ("audio", "event_played")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_assigns_per_tick_sequence() {
        let mut q = AudioReplayQueue::default();
        q.push(AudioPlaybackEvent::new(5, 0, "sfx_a", "sfx", AudioDirection::North, 0.5));
        q.push(AudioPlaybackEvent::new(5, 0, "sfx_b", "sfx", AudioDirection::East, 0.5));
        q.push(AudioPlaybackEvent::new(6, 0, "sfx_c", "sfx", AudioDirection::West, 0.5));
        let drained = q.drain_all();
        assert_eq!(drained[0].sequence, 0);
        assert_eq!(drained[1].sequence, 1);
        assert_eq!(drained[2].sequence, 0); // tick advanced → seq resets
    }

    #[test]
    fn drain_up_to_respects_tick_window() {
        let mut q = AudioReplayQueue::default();
        q.push(AudioPlaybackEvent::new(5, 0, "sfx_a", "sfx", AudioDirection::North, 0.5));
        q.push(AudioPlaybackEvent::new(10, 0, "sfx_b", "sfx", AudioDirection::East, 0.5));
        q.push(AudioPlaybackEvent::new(15, 0, "sfx_c", "sfx", AudioDirection::West, 0.5));
        let drained = q.drain_up_to(10);
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0].canonical_name, "sfx_a");
        assert_eq!(drained[1].canonical_name, "sfx_b");
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn drain_orders_canonically_by_tick_then_sequence_then_name() {
        let mut q = AudioReplayQueue::default();
        q.push(AudioPlaybackEvent::new(5, 0, "sfx_z", "sfx", AudioDirection::North, 0.5));
        q.push(AudioPlaybackEvent::new(5, 0, "sfx_a", "sfx", AudioDirection::East, 0.5));
        let drained = q.drain_all();
        // Both at tick=5; the FIRST push gets seq=0 → wins the sort.
        assert_eq!(drained[0].canonical_name, "sfx_z");
        assert_eq!(drained[1].canonical_name, "sfx_a");
        assert!(drained[0].sequence < drained[1].sequence);
    }

    #[test]
    fn replay_key_used_for_deterministic_compare() {
        let a = AudioPlaybackEvent::new(5, 0, "sfx_a", "sfx", AudioDirection::North, 0.5);
        let b = AudioPlaybackEvent::new(5, 1, "sfx_b", "sfx", AudioDirection::North, 0.5);
        assert!(a.replay_key() < b.replay_key());
    }

    #[test]
    fn audio_events_always_cosmetic() {
        let event = AudioPlaybackEvent::new(0, 0, "sfx", "sfx", AudioDirection::Here, 1.0);
        assert!(event.cosmetic);
    }

    #[test]
    fn is_cosmetic_audio_event_only_matches_audio_category() {
        assert!(is_cosmetic_audio_event("audio"));
        assert!(!is_cosmetic_audio_event("combat"));
        assert!(!is_cosmetic_audio_event("terrain"));
    }

    #[test]
    fn m12b_cosmetic_event_types_includes_four_new_events() {
        for et in ["spatial_resolved", "reverb_applied", "occluded", "doppler_shifted"] {
            assert!(M12B_COSMETIC_EVENT_TYPES.contains(&et), "missing {et}");
        }
    }

    #[test]
    fn is_cosmetic_audio_event_for_classifies_m12b_events() {
        for et in M12B_COSMETIC_EVENT_TYPES {
            assert!(is_cosmetic_audio_event_for("audio", et), "missed audio.{et}");
        }
        assert!(is_cosmetic_audio_event_for("audio", "event_played"));
        assert!(!is_cosmetic_audio_event_for("combat", "spatial_resolved"));
        assert!(!is_cosmetic_audio_event_for("terrain", "occluded"));
    }

    #[test]
    fn gain_clamped_to_unit_range_on_construction() {
        let above = AudioPlaybackEvent::new(0, 0, "sfx", "sfx", AudioDirection::Here, 2.5);
        let below = AudioPlaybackEvent::new(0, 0, "sfx", "sfx", AudioDirection::Here, -0.5);
        assert!((above.gain - 1.0).abs() < 1e-4);
        assert!(below.gain.abs() < 1e-4);
    }

    #[test]
    fn replay_round_trips_through_serde() {
        let ev = AudioPlaybackEvent::new(42, 7, "sfx_pistol_fire", "sfx", AudioDirection::North, 0.5);
        let json = serde_json::to_string(&ev).unwrap();
        let back: AudioPlaybackEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(ev, back);
    }
}
