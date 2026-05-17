//! M10B kill-feed overlay renderer.
//!
//! Spec § "Player-facing behavior":
//!
//! > kill-feed ... independently per-export.
//!
//! VAL-M10B-OVERLAY-KILLFEED-FILE: "The file
//! `game/crates/cf-replay-export/src/overlay_kill_feed.rs` exists;
//! running export with `--overlay kill_feed` produces an MP4 whose
//! per-frame kill_feed region contains a rendered entry for every
//! `actor_status_changed=killed` event in the bundle at the tick the
//! event occurred (entry visible across the documented kill-feed
//! dwell window). PASS = file present AND per-tick kill-feed entries
//! match the event stream."
//!
//! Per the feature's expected behavior: "one entry per
//! `actor_status_changed=killed` with `cause=other_actor`".
//!
//! Entries are rendered to the top-right AOI of the frame; each entry
//! is visible for [`KILL_FEED_DWELL_TICKS`] ticks starting at the
//! event's `tick_index`.

use cf_replay::Event;

use crate::overlay_graph::{KILL_FEED_OVERLAY_NAME, KILL_FEED_Z_ORDER};

/// Kill-feed entry dwell — how many sim ticks each entry remains
/// visible. The live HUD uses ~5 seconds; at 60 Hz that's 300 ticks.
/// The offline rasterizer mirrors the live dwell so an exported clip
/// shows the entry for the same duration the live spectator saw it.
pub const KILL_FEED_DWELL_TICKS: u64 = 300;

/// Default kill-feed AOI at 1920×1080. Top-right; entries stack
/// vertically. Other resolutions scale proportionally.
pub const KILL_FEED_AOI_X: u32 = 1920 - 16 - 360;
pub const KILL_FEED_AOI_Y: u32 = 16;
pub const KILL_FEED_AOI_WIDTH: u32 = 360;
pub const KILL_FEED_AOI_HEIGHT: u32 = 240;

/// Cause filter — only kills caused by another actor produce a
/// kill-feed entry. Death from terrain / hazard / suicide is OUT of
/// scope of the kill-feed by spec convention (those route through the
/// debrief modal, not the kill-feed strip).
pub const KILL_FEED_CAUSE_FILTER: &str = "other_actor";

/// One kill-feed entry. The rasterizer renders this as a single line
/// of text in the AOI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KillFeedEntry {
    pub tick_index: u64,
    pub victim_id: u64,
    pub killer_id: Option<u64>,
    pub weapon_label: String,
}

impl KillFeedEntry {
    /// `true` when this entry should be visible at `tick`. The dwell
    /// window is `[tick_index, tick_index + KILL_FEED_DWELL_TICKS)`.
    #[must_use]
    pub fn is_visible_at_tick(&self, tick: u64) -> bool {
        tick >= self.tick_index && tick < self.tick_index.saturating_add(KILL_FEED_DWELL_TICKS)
    }
}

/// Walk the bundle's events to produce one kill-feed entry per
/// `actor_status_changed=killed` event whose `cause` payload field is
/// `"other_actor"`.
#[must_use]
pub fn derive_entries(events: &[Event]) -> Vec<KillFeedEntry> {
    let mut entries: Vec<KillFeedEntry> = Vec::new();
    for event in events {
        if event.event_type != "actor_status_changed" {
            continue;
        }
        let status = event.payload.get("status").and_then(|v| v.as_str()).unwrap_or("");
        if status != "killed" {
            continue;
        }
        let cause = event.payload.get("cause").and_then(|v| v.as_str()).unwrap_or("");
        if cause != KILL_FEED_CAUSE_FILTER {
            continue;
        }
        let victim_id = event
            .actor_id
            .or_else(|| event.payload.get("victim_id").and_then(|v| v.as_u64()))
            .unwrap_or(0);
        let killer_id = event
            .source_id
            .or_else(|| event.payload.get("killer_id").and_then(|v| v.as_u64()));
        let weapon_label = event
            .payload
            .get("weapon_label")
            .and_then(|v| v.as_str())
            .unwrap_or("weapon")
            .to_owned();
        let tick_index = event
            .payload
            .get("tick_index")
            .and_then(|v| v.as_u64())
            .unwrap_or(event.tick);
        entries.push(KillFeedEntry {
            tick_index,
            victim_id,
            killer_id,
            weapon_label,
        });
    }
    entries
}

/// Kill-feed overlay descriptor — the static AOI + z_order; per-frame
/// rendering uses [`derive_entries`] to walk the bundle's events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KillFeedOverlay {
    pub aoi_x: u32,
    pub aoi_y: u32,
    pub aoi_width: u32,
    pub aoi_height: u32,
    pub z_order: u32,
}

impl Default for KillFeedOverlay {
    fn default() -> Self {
        Self {
            aoi_x: KILL_FEED_AOI_X,
            aoi_y: KILL_FEED_AOI_Y,
            aoi_width: KILL_FEED_AOI_WIDTH,
            aoi_height: KILL_FEED_AOI_HEIGHT,
            z_order: KILL_FEED_Z_ORDER,
        }
    }
}

impl KillFeedOverlay {
    #[must_use]
    pub const fn name() -> &'static str {
        KILL_FEED_OVERLAY_NAME
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kill_event(event_id: &str, tick: u64, victim: u64, killer: u64, cause: &str) -> Event {
        Event {
            schema_version: cf_replay::EVENT_SCHEMA_VERSION.to_string(),
            run_id: "kill_feed_test".into(),
            tick,
            sim_time_ms: tick as f64 * 16.6,
            event_id: event_id.into(),
            category: "actor".into(),
            event_type: "actor_status_changed".into(),
            payload: serde_json::json!({
                "status": "killed",
                "cause": cause,
                "weapon_label": "rifle",
            }),
            parent_event_id: None,
            actor_id: Some(victim),
            source_id: Some(killer),
            team: None,
            pos: None,
            bbox: None,
            dropped_count: None,
            cosmetic: None,
            asset_ref: None,
            prev_event_hash: None,
            chained_hash_hex: None,
        }
    }

    #[test]
    fn derive_entries_emits_one_per_killed_event() {
        let events = vec![
            kill_event("a", 100, 7, 3, "other_actor"),
            kill_event("b", 250, 9, 4, "other_actor"),
            kill_event("c", 400, 11, 5, "hazard"), // filtered out
        ];
        let entries = derive_entries(&events);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].tick_index, 100);
        assert_eq!(entries[1].tick_index, 250);
        assert_eq!(entries[1].victim_id, 9);
        assert_eq!(entries[1].killer_id, Some(4));
    }

    #[test]
    fn entry_is_visible_during_dwell_window() {
        let entry = KillFeedEntry {
            tick_index: 1000,
            victim_id: 1,
            killer_id: Some(2),
            weapon_label: "rifle".into(),
        };
        assert!(entry.is_visible_at_tick(1000));
        assert!(entry.is_visible_at_tick(1000 + KILL_FEED_DWELL_TICKS - 1));
        assert!(!entry.is_visible_at_tick(1000 + KILL_FEED_DWELL_TICKS));
        assert!(!entry.is_visible_at_tick(999));
    }

    #[test]
    fn entries_first_visible_frame_matches_event_tick() {
        let events = vec![kill_event("a", 1234, 7, 3, "other_actor")];
        let entries = derive_entries(&events);
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].tick_index, events[0].tick,
            "entry first-visible frame must match event tick"
        );
    }

    #[test]
    fn non_killed_status_events_are_filtered() {
        let mut wounded = kill_event("w", 50, 1, 2, "other_actor");
        wounded.payload["status"] = "wounded".into();
        let entries = derive_entries(&[wounded]);
        assert!(entries.is_empty());
    }
}
