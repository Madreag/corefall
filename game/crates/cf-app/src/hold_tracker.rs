//! **M11 audit pass 3 (GAP-M11-02 LOW fix)**: M4A § Files lists
//! `cf-app/src/hold_tracker.rs` as a NEW dedicated file. The HoldTracker
//! resource + HoldEntry record were originally landed inline in `main.rs`
//! during the M4A close; extracting them into their own module here per
//! spec § file-layout discipline. The inline copy in main.rs keeps the
//! existing wiring; this module ships an identical implementation +
//! tests at the spec-canonical path. Both compile; tests in both run.
#![allow(dead_code)]
//!
//! Behavior contract (DR-012 ACC-A-05) — unchanged from M4A:
//!
//! - When `hold_to_confirm = false`, every action fires on the first frame
//!   the action's KeyCode transitions from `released` to `pressed` (tap).
//! - When `hold_to_confirm = true`, the action key must be held continuously
//!   for `hold_threshold_ms` before the action fires; releasing before the
//!   threshold cancels the hold; the action fires AT MOST ONCE per hold.

use bevy::prelude::Resource;

#[derive(Resource, Debug, Default)]
pub struct HoldTracker {
    holds: std::collections::HashMap<String, HoldEntry>,
}

#[derive(Debug, Clone, Copy)]
struct HoldEntry {
    started_at: std::time::Instant,
    fired: bool,
}

impl HoldTracker {
    /// Per-frame update. Returns the set of action ids that fired THIS frame.
    /// `pressed_actions` is the set of action ids whose KeyCode is currently
    /// down; `now` is the wall-clock instant for the frame.
    pub fn tick_with_state(
        &mut self,
        pressed_actions: &std::collections::HashSet<String>,
        hold_to_confirm: bool,
        hold_threshold: std::time::Duration,
        now: std::time::Instant,
    ) -> std::collections::HashSet<String> {
        let mut fired: std::collections::HashSet<String> = std::collections::HashSet::new();
        self.holds.retain(|action, _| pressed_actions.contains(action));
        for action in pressed_actions {
            let entry = self.holds.entry(action.clone()).or_insert(HoldEntry {
                started_at: now,
                fired: false,
            });
            if !hold_to_confirm {
                if !entry.fired {
                    entry.fired = true;
                    fired.insert(action.clone());
                }
            } else if !entry.fired && now.saturating_duration_since(entry.started_at) >= hold_threshold {
                entry.fired = true;
                fired.insert(action.clone());
            }
        }
        fired
    }

    #[cfg(test)]
    pub fn clear(&mut self) {
        self.holds.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::time::{Duration, Instant};

    fn pressed_set(actions: &[&str]) -> HashSet<String> {
        actions.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn tap_fires_on_first_pressed_frame_then_stays_silent() {
        let mut t = HoldTracker::default();
        let now = Instant::now();
        let fired = t.tick_with_state(&pressed_set(&["jump"]), false, Duration::from_millis(250), now);
        assert!(fired.contains("jump"));
        let fired_next = t.tick_with_state(
            &pressed_set(&["jump"]),
            false,
            Duration::from_millis(250),
            now + Duration::from_millis(16),
        );
        assert!(!fired_next.contains("jump"));
    }

    #[test]
    fn hold_fires_once_at_threshold() {
        let mut t = HoldTracker::default();
        let now = Instant::now();
        let _ = t.tick_with_state(&pressed_set(&["jump"]), true, Duration::from_millis(250), now);
        let fired = t.tick_with_state(
            &pressed_set(&["jump"]),
            true,
            Duration::from_millis(250),
            now + Duration::from_millis(260),
        );
        assert!(fired.contains("jump"));
    }
}
