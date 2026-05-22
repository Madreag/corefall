//! **M12C**: Skip / pause / replay policy + per-cinematic seen set.
//!
//! Per spec § "Skip / pause / replay UX":
//!
//! - "Skip" — `[Esc]` or `[Space]`. Fires `cinematic.skipped` + jumps
//!   directly to next gameplay tick. Skip is disabled in the **first
//!   3 seconds** of any never-before-seen cinematic.
//! - "Pause" — `[P]`. Pauses the cinematic clock; camera frozen;
//!   voice-over paused at boundary; subtitle frozen.
//! - "Replay" — any cinematic the player has watched is unlocked in
//!   `Codex → Cinematics`. Replay runs the script identically.
//!
//! Per spec § Notes for the implementer: "Skip-confirm window is
//! 3000 ms; the value is a constant in `cf-cinematic::skip_pause_replay`
//! and NOT a hardcoded `60` tick assumption — it converts via the active
//! tick rate."

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::script::CinematicId;

/// before-seen cinematic".
pub const SKIP_CONFIRM_WINDOW_MS: u32 = 3_000;

/// Reason annotation on a `cinematic.skipped` event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    /// Player pressed [Esc] / [Space].
    UserInput,
    /// Storyteller is Sandbox → suppress entirely.
    SandboxSuppressed,
    /// Cinematic ended naturally (`was_skipped: false` on
    /// `cinematic.ended` — never emitted on `cinematic.skipped` itself).
    Completed,
}

impl SkipReason {
    /// Canonical snake_case identifier.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            SkipReason::UserInput => "user_input",
            SkipReason::SandboxSuppressed => "sandbox_suppressed",
            SkipReason::Completed => "completed",
        }
    }
}

/// `save.cinematic_seen_set: HashSet<CinematicId>`; persisted via M41
/// save format." Wraps a `BTreeSet` so iteration order is deterministic.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeenSet {
    inner: BTreeSet<CinematicId>,
}

impl SeenSet {
    /// Mark `id` as watched. Returns `true` if newly inserted.
    pub fn mark_seen(&mut self, id: &str) -> bool {
        self.inner.insert(id.to_string())
    }

    /// Whether `id` has been watched (or skipped past the confirm window).
    #[must_use]
    pub fn contains(&self, id: &str) -> bool {
        self.inner.contains(id)
    }

    /// Total entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the set is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Sorted iterator over the seen-set ids. Order is deterministic
    /// (BTreeSet ascending) so codex renders are byte-stable.
    pub fn iter(&self) -> impl Iterator<Item = &CinematicId> {
        self.inner.iter()
    }
}

/// Pure policy: whether `[Esc]` / `[Space]` should accept a skip at
/// playhead `playhead_ms` for cinematic `id` given the seen set.
///
/// Per spec acceptance criterion "Skip is disabled for the first 3
/// seconds on never-before-seen cinematics":
///
/// 1. Skip is rejected if `playhead_ms < SKIP_CONFIRM_WINDOW_MS` AND
///    the id is NOT in the seen set.
/// 2. Otherwise the skip is accepted.
#[must_use]
pub fn skip_allowed_at(seen: &SeenSet, id: &str, playhead_ms: u32) -> bool {
    if seen.contains(id) {
        return true;
    }
    playhead_ms >= SKIP_CONFIRM_WINDOW_MS
}

/// Bundles every policy knob the scheduler reads each tick to decide
/// whether to fire `cinematic.skipped`, `cinematic.paused`, or
/// `cinematic.resumed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkipPauseReplayPolicy {
    /// Current seen-set (loaded from `save.cinematic_seen_set` at
    /// cinematic boot; updated on `cinematic.ended` + `cinematic.skipped`
    /// past the confirm window).
    pub seen: SeenSet,
    /// True while playback is paused.
    pub paused: bool,
}

impl Default for SkipPauseReplayPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl SkipPauseReplayPolicy {
    /// Construct a fresh policy with an empty seen-set and unpaused
    /// state. Equivalent to `Default::default()`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            seen: SeenSet::default(),
            paused: false,
        }
    }

    /// Convert the skip-confirm window to ticks for the supplied
    /// `tick_rate_hz`. Per spec § Notes: "NOT a hardcoded `60` tick
    /// assumption — it converts via the active tick rate."
    ///
    /// `tick_rate_hz == 0` is invalid input; this path returns
    /// `SKIP_CONFIRM_WINDOW_MS` directly so callers can still treat the
    /// return as a finite ms count when the engine has not yet set a
    /// tick rate (instead of dividing by zero).
    #[must_use]
    pub fn skip_confirm_window_ticks(tick_rate_hz: u32) -> u32 {
        if tick_rate_hz == 0 {
            return SKIP_CONFIRM_WINDOW_MS;
        }
        let hz = tick_rate_hz as u64;
        ((SKIP_CONFIRM_WINDOW_MS as u64 * hz).div_ceil(1_000)) as u32
    }

    /// Whether `[Esc]` / `[Space]` should accept a skip right now.
    #[must_use]
    pub fn skip_allowed(&self, id: &str, playhead_ms: u32) -> bool {
        skip_allowed_at(&self.seen, id, playhead_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_blocked_in_first_3_seconds_for_unseen() {
        let seen = SeenSet::default();
        assert!(!skip_allowed_at(&seen, "cin_intro_reactor_defense", 1_500));
        assert!(skip_allowed_at(&seen, "cin_intro_reactor_defense", 3_000));
        assert!(skip_allowed_at(&seen, "cin_intro_reactor_defense", 3_500));
    }

    #[test]
    fn skip_allowed_immediately_for_seen_cinematic() {
        let mut seen = SeenSet::default();
        seen.mark_seen("cin_intro_reactor_defense");
        assert!(skip_allowed_at(&seen, "cin_intro_reactor_defense", 0));
        assert!(skip_allowed_at(&seen, "cin_intro_reactor_defense", 1_500));
    }

    #[test]
    fn seen_set_round_trips_and_sorts() {
        let mut seen = SeenSet::default();
        seen.mark_seen("b");
        seen.mark_seen("a");
        seen.mark_seen("c");
        seen.mark_seen("a"); // idempotent
        assert_eq!(seen.len(), 3);
        let ids: Vec<_> = seen.iter().map(|s| s.as_str()).collect();
        assert_eq!(ids, vec!["a", "b", "c"]);
    }

    #[test]
    fn skip_confirm_window_ticks_converts_for_tick_rate() {
        // 60 Hz: 3_000 ms = 180 ticks.
        assert_eq!(SkipPauseReplayPolicy::skip_confirm_window_ticks(60), 180);
        // 120 Hz: 360 ticks.
        assert_eq!(SkipPauseReplayPolicy::skip_confirm_window_ticks(120), 360);
        // 30 Hz: 90 ticks.
        assert_eq!(SkipPauseReplayPolicy::skip_confirm_window_ticks(30), 90);
        // Edge: 0 hz clamps to 1 → 3000 ticks.
        assert_eq!(SkipPauseReplayPolicy::skip_confirm_window_ticks(0), 3_000);
    }
}
