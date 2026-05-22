//! M7-B: chatter scaffold for the smart commandable AI surface.
//!
//! Spec § Smart commandable AI — Chatter scaffold (TTS placeholder;
//! production voice at M37). Every meaningful AI action emits an audio +
//! caption event via the chatter scaffold. M7-B ships:
//!
//! - `ChatterCategory` — taxonomy of chatter kinds the engine emits.
//! - `tts_stub(text) -> Vec<Phoneme>` — deterministic placeholder TTS that
//!   maps each ASCII character to a `Phoneme`. M37 swaps in real audio
//!   synthesis; the wire form (phoneme list) keeps the engine path stable.
//! - `ChatterCooldownTable` — per-actor per-category cooldown gate that
//!   prevents chatter spam (default 4 s between same-category emissions
//!   from the same bot, per spec § cooldown table).
//! - `ChatterEmittedEvent` — payload shape for `ai.chatter_emitted`.
//! - `ChatterCaption` — ACC-A caption marker the engine mirrors into
//!   `HudState.captions` for deaf-or-headphones-off players (DR-019).
//!
//! The scaffold is presentation-free: no audio backend is invoked. The
//! `NullAudioPlugin` in this crate continues to satisfy `AudioPlugin` for
//! headless replays + cargo tests. The actual voice swap at M37 plugs into
//! the existing trait without touching the engine.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// in future milestones). The cooldown table indexes per `(actor_id,
/// category)` so an actor can emit different categories in rapid
/// succession (e.g. "Contact!" followed by "Engaging!") but not duplicate
/// emissions within the cooldown window.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum ChatterCategory {
    /// "Contact, <direction>, <distance>m" — enemy spotted.
    Contact,
    /// "Engaging!" — bot starts firing on a target.
    Engaging,
    /// "Reloading — cover me!" — bot starts a reload mid-fight.
    Reloading,
    /// "<Name> is down! Moving to triage." — Medic spots a downed ally.
    AllyDown,
    /// "Treating <name>, hold this area" — Medic begins treatment.
    Triage,
    /// "Repairing <name>'s <module>" — Engineer begins repair.
    Repair,
    /// "Can't get through — finding another way" — path blocked.
    PathBlocked,
    /// "Falling back to cover, we're outnumbered" — Layer 5 doctrine shift.
    Doctrine,
    /// "Got it, on my way" — MMB tag acknowledged.
    TagAck,
    /// "Roger" / "Affirmative" / "Copy" — explicit order acknowledged.
    OrderAck,
    /// "Negative — friendly in line of fire" — order refused.
    OrderRefused,
    /// "Fire on the floor, watch your step" — hazard spotted.
    HazardSpotted,
}

impl ChatterCategory {
    /// Every variant in declaration order.
    pub const ALL: [ChatterCategory; 12] = [
        ChatterCategory::Contact,
        ChatterCategory::Engaging,
        ChatterCategory::Reloading,
        ChatterCategory::AllyDown,
        ChatterCategory::Triage,
        ChatterCategory::Repair,
        ChatterCategory::PathBlocked,
        ChatterCategory::Doctrine,
        ChatterCategory::TagAck,
        ChatterCategory::OrderAck,
        ChatterCategory::OrderRefused,
        ChatterCategory::HazardSpotted,
    ];

    /// Canonical snake_case identifier for replay bundles + JSON payloads.
    pub fn as_str(self) -> &'static str {
        match self {
            ChatterCategory::Contact => "contact",
            ChatterCategory::Engaging => "engaging",
            ChatterCategory::Reloading => "reloading",
            ChatterCategory::AllyDown => "ally_down",
            ChatterCategory::Triage => "triage",
            ChatterCategory::Repair => "repair",
            ChatterCategory::PathBlocked => "path_blocked",
            ChatterCategory::Doctrine => "doctrine",
            ChatterCategory::TagAck => "tag_ack",
            ChatterCategory::OrderAck => "order_ack",
            ChatterCategory::OrderRefused => "order_refused",
            ChatterCategory::HazardSpotted => "hazard_spotted",
        }
    }

    /// Parse from the wire form.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<ChatterCategory> {
        Some(match value {
            "contact" => ChatterCategory::Contact,
            "engaging" => ChatterCategory::Engaging,
            "reloading" => ChatterCategory::Reloading,
            "ally_down" => ChatterCategory::AllyDown,
            "triage" => ChatterCategory::Triage,
            "repair" => ChatterCategory::Repair,
            "path_blocked" => ChatterCategory::PathBlocked,
            "doctrine" => ChatterCategory::Doctrine,
            "tag_ack" => ChatterCategory::TagAck,
            "order_ack" => ChatterCategory::OrderAck,
            "order_refused" => ChatterCategory::OrderRefused,
            "hazard_spotted" => ChatterCategory::HazardSpotted,
            _ => return None,
        })
    }

    /// Spec § Combat-critical chatter preempts conversational chatter.
    /// Returns true for the categories that must NEVER be cooldown-gated
    /// out entirely (the engine may still rate-limit, but combat-critical
    /// chatter must always be reachable).
    pub fn is_combat_critical(self) -> bool {
        matches!(
            self,
            ChatterCategory::Contact
                | ChatterCategory::Reloading
                | ChatterCategory::AllyDown
                | ChatterCategory::HazardSpotted
        )
    }
}

/// ASCII character of a chatter text to one of these; M37's real TTS swap
/// replaces this with phonetic IPA codes + duration metadata. The wire
/// form (phoneme list) stays stable so the recorder envelope doesn't
/// change when M37 lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Phoneme {
    /// Symbol the M37 TTS backend will pronounce. M7's placeholder uses
    /// the ASCII character (lowercase) as the symbol.
    pub symbol: char,
    /// Duration in milliseconds at default playback rate. M7's placeholder
    /// uses a constant 80 ms per phoneme to keep the wire form
    /// deterministic.
    pub duration_ms: u16,
}

/// Constant duration (ms) per placeholder phoneme. M37 overrides per-symbol.
pub const PLACEHOLDER_PHONEME_MS: u16 = 80;

/// list. Each ASCII character becomes one `Phoneme` (lowercase symbol +
/// constant duration). Non-ASCII characters lower to `'?'` so the
/// recorder envelope stays byte-stable across locales. M37 swaps in real
/// TTS without changing the surface.
pub fn tts_stub(text: &str) -> Vec<Phoneme> {
    text.chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| {
            let symbol = if c.is_ascii() { c.to_ascii_lowercase() } else { '?' };
            Phoneme {
                symbol,
                duration_ms: PLACEHOLDER_PHONEME_MS,
            }
        })
        .collect()
}

/// `Some(EmissionInfo)` when the chatter slot is open AND records the
/// emission tick; `None` when the cooldown is still active.
///
/// Cooldown windows are tick-relative (not wall-clock) so replays remain
/// byte-identical across hosts at the same tick rate. The default cooldown
/// is 4 seconds (spec § cooldown table); callers convert via `seconds *
/// tick_rate_hz` themselves.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ChatterCooldownTable {
    /// Per-(actor_id, category) last emission tick. BTreeMap iterates in
    /// deterministic key order so snapshot/restore preserves layout.
    pub last_emit_tick: BTreeMap<u64, BTreeMap<ChatterCategory, u64>>,
}

/// Information about a successful chatter emission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmissionInfo {
    /// Tick the chatter was emitted on.
    pub current_tick: u64,
    /// Cooldown window (ticks) remaining before the same category can
    /// emit again. Always `cooldown_ticks` for a successful emission.
    pub cooldown_ticks: u64,
    /// Tick the previous emission of this same category fired (`None` if
    /// this is the first emission).
    pub previous_tick: Option<u64>,
}

impl ChatterCooldownTable {
    /// Build an empty cooldown table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attempt to emit a chatter event. Returns `Some(EmissionInfo)` if the
    /// cooldown window is open (and records the new emission tick), `None`
    /// otherwise.
    ///
    /// `cooldown_ticks` is the window per `(actor_id, category)` —
    /// typically `(4.0 * tick_rate_hz)` from `cf-ai::CHATTER_COOLDOWN_SECONDS`.
    pub fn try_emit(
        &mut self,
        actor_id: u64,
        category: ChatterCategory,
        current_tick: u64,
        cooldown_ticks: u64,
    ) -> Option<EmissionInfo> {
        let per_actor = self.last_emit_tick.entry(actor_id).or_default();
        let previous = per_actor.get(&category).copied();
        if let Some(prev) = previous {
            if current_tick.saturating_sub(prev) < cooldown_ticks {
                return None;
            }
        }
        per_actor.insert(category, current_tick);
        Some(EmissionInfo {
            current_tick,
            cooldown_ticks,
            previous_tick: previous,
        })
    }

    /// Compute the remaining cooldown for `(actor_id, category)` at the
    /// current tick. Returns `0` if the slot is open.
    pub fn cooldown_remaining(
        &self,
        actor_id: u64,
        category: ChatterCategory,
        current_tick: u64,
        cooldown_ticks: u64,
    ) -> u64 {
        let prev = self
            .last_emit_tick
            .get(&actor_id)
            .and_then(|inner| inner.get(&category))
            .copied();
        match prev {
            None => 0,
            Some(p) => cooldown_ticks.saturating_sub(current_tick.saturating_sub(p)),
        }
    }
}

/// chatter info the engine surfaces via cf-replay AND the M11 HUD caption
/// ticker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatterEmittedEvent {
    /// Bot speaking.
    pub actor_id: u64,
    /// Category of chatter.
    pub category: ChatterCategory,
    /// Spoken text — placeholder strings ship at M7; M37 swaps in real
    /// voice clips. Mirrored into HudState.captions for ACC-A.
    pub text: String,
    /// Voice id. M7's placeholder uses an archetype-derived id (e.g.
    /// "voice.rifleman.default"); M37 maps to real voice clips.
    pub voice_id: String,
    /// Remaining cooldown (seconds) before this `(actor, category)` slot
    /// can emit again. Surfaced for UI / observability.
    pub cooldown_remaining_seconds: f32,
}

impl ChatterEmittedEvent {
    /// JSON serialise via serde_json (caller passes the result into the
    /// event recorder). Kept off the `serde_json` dep here — the
    /// `cf-replay::Recorder` does the serialisation. This helper is here
    /// for tests + downstream consumers that already have serde_json.
    pub fn caption(&self) -> ChatterCaption {
        ChatterCaption {
            actor_id: self.actor_id,
            text: self.text.clone(),
            category: self.category,
        }
    }
}

/// deaf-or-headphones-off players see the same chatter as a text ticker.
/// Spec § ACC-A captions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatterCaption {
    /// Bot speaking.
    pub actor_id: u64,
    /// Caption text (mirrors `ChatterEmittedEvent.text`).
    pub text: String,
    /// Category — UI can tint by category (combat-critical vs conversational).
    pub category: ChatterCategory,
}

/// stable `voice_id`. M37 replaces this with a per-voice-clip lookup
/// table; M7's placeholder keeps a deterministic id per archetype so the
/// recorder envelope is byte-stable.
pub fn voice_id_for_archetype(archetype: &str) -> String {
    format!("voice.{}.default", archetype)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chatter_category_round_trip_str() {
        for c in ChatterCategory::ALL.iter() {
            assert_eq!(ChatterCategory::from_str(c.as_str()), Some(*c));
        }
    }

    #[test]
    fn tts_stub_is_deterministic() {
        let a = tts_stub("Treating Jenkins");
        let b = tts_stub("Treating Jenkins");
        assert_eq!(a, b);
        assert!(!a.is_empty());
        assert_eq!(a[0].symbol, 't');
        assert_eq!(a[0].duration_ms, PLACEHOLDER_PHONEME_MS);
    }

    #[test]
    fn tts_stub_skips_whitespace_and_lowercases() {
        let p = tts_stub("Hi Bob");
        assert_eq!(p.len(), 5);
        assert_eq!(p.iter().map(|x| x.symbol).collect::<String>(), "hibob");
    }

    #[test]
    fn tts_stub_non_ascii_becomes_question_mark() {
        let p = tts_stub("café");
        assert!(p.iter().any(|c| c.symbol == '?'));
    }

    #[test]
    fn cooldown_gates_repeat_emissions() {
        let mut t = ChatterCooldownTable::new();
        let cooldown = 240; // 4s @ 60 Hz
        let first = t.try_emit(7, ChatterCategory::Triage, 100, cooldown);
        assert!(first.is_some());
        // 100 ticks later — still in cooldown.
        let second = t.try_emit(7, ChatterCategory::Triage, 200, cooldown);
        assert!(second.is_none());
        // 240 ticks later (current 100 + 240 = 340 -> exactly at boundary).
        let third = t.try_emit(7, ChatterCategory::Triage, 340, cooldown);
        assert!(third.is_some());
    }

    #[test]
    fn cooldown_independent_per_actor_and_category() {
        let mut t = ChatterCooldownTable::new();
        let cooldown = 240;
        // Same actor, different categories — both allowed.
        assert!(t.try_emit(7, ChatterCategory::Triage, 100, cooldown).is_some());
        assert!(t.try_emit(7, ChatterCategory::Repair, 100, cooldown).is_some());
        // Different actors, same category — both allowed.
        assert!(t.try_emit(8, ChatterCategory::Triage, 100, cooldown).is_some());
    }

    #[test]
    fn cooldown_remaining_computes_correctly() {
        let mut t = ChatterCooldownTable::new();
        let cooldown = 240;
        let _ = t.try_emit(7, ChatterCategory::Triage, 100, cooldown);
        assert_eq!(t.cooldown_remaining(7, ChatterCategory::Triage, 100, cooldown), 240);
        assert_eq!(t.cooldown_remaining(7, ChatterCategory::Triage, 200, cooldown), 140);
        assert_eq!(t.cooldown_remaining(7, ChatterCategory::Triage, 400, cooldown), 0);
        // Unseen (actor, category) returns 0.
        assert_eq!(t.cooldown_remaining(8, ChatterCategory::Triage, 100, cooldown), 0);
    }

    #[test]
    fn combat_critical_categories_flagged() {
        assert!(ChatterCategory::Contact.is_combat_critical());
        assert!(ChatterCategory::Reloading.is_combat_critical());
        assert!(!ChatterCategory::OrderAck.is_combat_critical());
        assert!(!ChatterCategory::Doctrine.is_combat_critical());
    }

    #[test]
    fn chatter_emitted_event_round_trips_through_serde() {
        let e = ChatterEmittedEvent {
            actor_id: 42,
            category: ChatterCategory::Triage,
            text: "Treating Jenkins, hold this area".to_string(),
            voice_id: "voice.medic.default".to_string(),
            cooldown_remaining_seconds: 4.0,
        };
        let json = serde_json::to_string(&e).expect("serialize");
        let back: ChatterEmittedEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(e, back);
    }

    #[test]
    fn caption_mirror_carries_text_and_category() {
        let e = ChatterEmittedEvent {
            actor_id: 42,
            category: ChatterCategory::Triage,
            text: "Treating Jenkins, hold this area".to_string(),
            voice_id: "voice.medic.default".to_string(),
            cooldown_remaining_seconds: 4.0,
        };
        let cap = e.caption();
        assert_eq!(cap.actor_id, 42);
        assert_eq!(cap.text, "Treating Jenkins, hold this area");
        assert_eq!(cap.category, ChatterCategory::Triage);
    }

    #[test]
    fn voice_id_for_archetype_is_deterministic() {
        assert_eq!(voice_id_for_archetype("medic"), "voice.medic.default");
        assert_eq!(voice_id_for_archetype("medic"), "voice.medic.default");
        assert_eq!(voice_id_for_archetype("engineer"), "voice.engineer.default");
    }
}
