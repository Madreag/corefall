//! M10B export audit event emission.
//!
//! Spec § "Player-facing behavior":
//!
//! > And `replay_export_completed` fires in the audit log with
//! > `output_path`, `codec`, `duration_seconds`, `chapter_count`
//!
//! VAL-M10B-036: "During an export, the audit log emits exactly one
//! `replay_export_started` event before the first frame is encoded,
//! one `chapter_marker_emitted` event per chapter marker (count
//! matches VAL-M10B-014 / VAL-M10B-022), and one
//! `replay_export_completed` event after the last frame is encoded;
//! ordering by `event_id` (monotonic) places `replay_export_started`
//! first, then all `chapter_marker_emitted` events in
//! chapter-time-ascending order, then `replay_export_completed` last."
//!
//! Feature (i): "replay_export_started → chapter_marker_emitted (× N)
//! → replay_export_completed event ordering."
//!
//! This module emits an ordered Vec<Event> in the correct ordering.
//! The actual replay event schemas (`replay_export_started.json` etc)
//! land in m10b-5 per the mission feature decomposition; m10b-3 here
//! produces the well-formed `Event` envelopes whose `event_type`
//! values match the eventual schemas.

use cf_replay::Event;

use crate::chapter_derivation::ChapterMarker;

pub const EVENT_TYPE_EXPORT_STARTED: &str = "replay_export_started";
pub const EVENT_TYPE_CHAPTER_MARKER_EMITTED: &str = "chapter_marker_emitted";
pub const EVENT_TYPE_EXPORT_COMPLETED: &str = "replay_export_completed";
pub const EVENT_CATEGORY: &str = "replay_export";

/// Job-level metadata captured at export start.
#[derive(Debug, Clone)]
pub struct ExportJobMetadata {
    pub run_id: String,
    pub bundle_path: String,
    pub output_path: String,
    pub preset_name: String,
    pub codec: String,
    pub duration_seconds: f64,
}

/// Emit the three event types in the contractually-required order.
/// Returns the full ordered vector so the export CLI (m10b-4) can
/// append directly to the bundle's events.jsonl.
#[must_use]
pub fn emit_export_audit_events(meta: &ExportJobMetadata, chapters: &[ChapterMarker]) -> Vec<Event> {
    let mut out: Vec<Event> = Vec::with_capacity(2 + chapters.len());
    let mut seq: u64 = 0;
    out.push(make_envelope(
        meta,
        &mut seq,
        EVENT_TYPE_EXPORT_STARTED,
        0,
        serde_json::json!({
            "bundle_path": meta.bundle_path,
            "output_path": meta.output_path,
            "preset": meta.preset_name,
            "codec": meta.codec,
            "duration_seconds": meta.duration_seconds,
        }),
    ));
    // Walk chapters in tick_index-ascending order (the derivation
    // pass already sorts them, but assert anyway for safety).
    let mut sorted_chapters: Vec<&ChapterMarker> = chapters.iter().collect();
    sorted_chapters.sort_by_key(|c| c.tick_index);
    for chapter in sorted_chapters {
        out.push(make_envelope(
            meta,
            &mut seq,
            EVENT_TYPE_CHAPTER_MARKER_EMITTED,
            chapter.tick_index,
            serde_json::json!({
                "tick_index": chapter.tick_index,
                "start_time_seconds": chapter.start_time_seconds,
                "title": chapter.title,
                "event_type": chapter.event_type,
                "source_event_id": chapter.event_id,
                "category": chapter.category,
            }),
        ));
    }
    out.push(make_envelope(
        meta,
        &mut seq,
        EVENT_TYPE_EXPORT_COMPLETED,
        // Place the completed event at a tick strictly later than the
        // last chapter so audit-log readers can detect the boundary.
        chapters.iter().map(|c| c.tick_index).max().unwrap_or(0).saturating_add(1),
        serde_json::json!({
            "output_path": meta.output_path,
            "codec": meta.codec,
            "duration_seconds": meta.duration_seconds,
            "chapter_count": chapters.len(),
        }),
    ));
    out
}

fn make_envelope(meta: &ExportJobMetadata, seq: &mut u64, event_type: &str, tick: u64, payload: serde_json::Value) -> Event {
    let id = *seq;
    *seq += 1;
    Event {
        schema_version: cf_replay::EVENT_SCHEMA_VERSION.to_string(),
        run_id: meta.run_id.clone(),
        tick,
        sim_time_ms: 0.0,
        event_id: format!("{}:export:{:06}", meta.run_id, id),
        category: EVENT_CATEGORY.into(),
        event_type: event_type.into(),
        payload,
        parent_event_id: None,
        actor_id: None,
        source_id: None,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn meta() -> ExportJobMetadata {
        ExportJobMetadata {
            run_id: "run_42".into(),
            bundle_path: "/tmp/bundle".into(),
            output_path: "/tmp/clip.mp4".into(),
            preset_name: "clip_compact".into(),
            codec: "h264".into(),
            duration_seconds: 90.0,
        }
    }

    fn chapter(tick: u64, title: &str) -> ChapterMarker {
        ChapterMarker {
            tick_index: tick,
            start_time_seconds: tick as f64 / 60.0,
            title: title.into(),
            event_type: "actor_status_changed".into(),
            event_id: format!("ev_{tick}"),
            category: Some("death".into()),
        }
    }

    /// VAL-M10B-036: started → markers (in chapter-time ascending) →
    /// completed ordering.
    #[test]
    fn audit_events_emit_in_required_order() {
        let chapters = vec![chapter(100, "a"), chapter(50, "b"), chapter(200, "c")];
        let events = emit_export_audit_events(&meta(), &chapters);
        assert_eq!(events.len(), 5);
        assert_eq!(events[0].event_type, EVENT_TYPE_EXPORT_STARTED);
        assert_eq!(events[1].event_type, EVENT_TYPE_CHAPTER_MARKER_EMITTED);
        assert_eq!(events[2].event_type, EVENT_TYPE_CHAPTER_MARKER_EMITTED);
        assert_eq!(events[3].event_type, EVENT_TYPE_CHAPTER_MARKER_EMITTED);
        assert_eq!(events[4].event_type, EVENT_TYPE_EXPORT_COMPLETED);
        // Chapter markers in tick-ascending order:
        let ticks: Vec<u64> = events[1..=3].iter().map(|e| e.tick).collect();
        assert_eq!(ticks, vec![50, 100, 200]);
    }

    /// event_id values are monotonically increasing (lexicographic
    /// + numeric tail).
    #[test]
    fn audit_event_ids_are_monotonic() {
        let chapters = vec![chapter(100, "a"), chapter(200, "b")];
        let events = emit_export_audit_events(&meta(), &chapters);
        let ids: Vec<&str> = events.iter().map(|e| e.event_id.as_str()).collect();
        for pair in ids.windows(2) {
            assert!(pair[0] < pair[1], "event_id ordering: {} < {}", pair[0], pair[1]);
        }
    }

    /// Completed payload carries every required field per the spec:
    /// output_path, codec, duration_seconds, chapter_count.
    #[test]
    fn completed_event_has_full_audit_shape() {
        let chapters = vec![chapter(0, "x")];
        let events = emit_export_audit_events(&meta(), &chapters);
        let completed = events.iter().find(|e| e.event_type == EVENT_TYPE_EXPORT_COMPLETED).unwrap();
        let p = &completed.payload;
        assert_eq!(p.get("output_path").and_then(|v| v.as_str()), Some("/tmp/clip.mp4"));
        assert_eq!(p.get("codec").and_then(|v| v.as_str()), Some("h264"));
        assert_eq!(p.get("duration_seconds").and_then(|v| v.as_f64()), Some(90.0));
        assert_eq!(p.get("chapter_count").and_then(|v| v.as_u64()), Some(1));
    }

    /// Empty chapter list still emits started + completed in correct
    /// order.
    #[test]
    fn empty_chapter_list_emits_started_and_completed() {
        let events = emit_export_audit_events(&meta(), &[]);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, EVENT_TYPE_EXPORT_STARTED);
        assert_eq!(events[1].event_type, EVENT_TYPE_EXPORT_COMPLETED);
    }
}
