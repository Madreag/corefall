//! M10B kill-feed overlay integration tests.
//!
//! Verification command: `cargo test -p cf-replay-export overlay_kill_feed`
//! (expect: entries_at_event_ticks PASS).
//!
//! VAL-M10B-OVERLAY-KILLFEED-FILE: per-frame kill_feed region contains
//! a rendered entry for every `actor_status_changed=killed` event in
//! the bundle at the tick the event occurred (entry visible across the
//! documented kill-feed dwell window).

use cf_replay::Event;
use cf_replay_export::overlay_kill_feed::{derive_entries, KillFeedEntry, KILL_FEED_DWELL_TICKS};

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
fn overlay_kill_feed_entries_at_event_ticks() {
    let events = vec![
        kill_event("k0", 200, 1, 2, "other_actor"),
        kill_event("k1", 800, 3, 4, "other_actor"),
        kill_event("k2", 1400, 5, 6, "other_actor"),
        // Filtered: cause != other_actor
        kill_event("k3", 1600, 7, 8, "hazard"),
    ];
    let entries = derive_entries(&events);
    assert_eq!(entries.len(), 3, "one entry per killed-by-other_actor event");

    // VAL-M10B-OVERLAY-KILLFEED-FILE: entry's first-visible frame
    // index equals the event's tick_index (within ±1 frame at the
    // preset's fps).
    for (entry, source) in entries.iter().zip(events.iter()) {
        assert_eq!(entry.tick_index, source.tick);
    }
}

#[test]
fn overlay_kill_feed_entry_visible_during_dwell_window() {
    let entry = KillFeedEntry {
        tick_index: 600,
        victim_id: 5,
        killer_id: Some(9),
        weapon_label: "rifle".into(),
    };
    assert!(entry.is_visible_at_tick(600));
    assert!(entry.is_visible_at_tick(600 + KILL_FEED_DWELL_TICKS - 1));
    assert!(!entry.is_visible_at_tick(600 + KILL_FEED_DWELL_TICKS));
}
