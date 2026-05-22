//! M10B chapter-marker derivation pass.
//!
//! Spec § "Player-facing behavior":
//!
//! > **Chapter markers auto-generate.** Mission objectives
//! > (`mission.objective_started/completed/failed`), deaths
//! > (`actor_status_changed=killed`), reactor events
//! > (`reactor.armor_layer_destroyed`), breaches
//! > (`atmos.breach_detected`), commander beats
//! > (`mission.commander_*`) all become MP4 chapter markers visible in
//! > YouTube + VLC + QuickTime.
//!
//! VAL-M10B-022 fixture: 30-min reactor-defense mission with 12
//! objective transitions + 3 reactor.armor_layer_destroyed + 7
//! actor_status_changed=killed + 4 atmos.breach_detected = **26
//! chapter markers** total.
//!
//! VAL-M10B-023: chapter timecodes are frame-accurate to the event's
//! tick (within ≤ 1 frame at 60 fps).
//!
//! The derivation pass:
//!
//! 1. Walks every event in the bundle.
//! 2. For each `event_type`, looks up the matching rule in the
//!    [`ChapterRuleSet`] (load order: bundle's
//!    `content/replay_export/chapter_rules/default.ron` ←
//!    [`ChapterRuleSet::load`]).
//! 3. Renders the rule's template against the event's payload (`{}`
//!    placeholder substitution).
//! 4. Emits one [`ChapterMarker`] per matching event, ordered by
//!    `tick_index` ascending.
//!
//! Frame-accurate timecode: `start_time_seconds = tick_index /
//! tick_rate_hz`. At 60 Hz that's `tick / 60.0`. The export's fps is
//! independent — `chapter.start_time_seconds × fps_of_preset` rounded
//! to the nearest integer gives the chapter's first frame; per
//! VAL-M10B-023 the difference between source-event `tick_index` and
//! `(round(start_time_seconds × fps) / fps × tick_rate)` is ≤ 1 frame.

use std::collections::BTreeMap;

use cf_replay::Event;
use serde::{Deserialize, Serialize};

use crate::chapter_markers::ChapterRuleSet;

/// One derived chapter marker. `tick_index` is the source event's
/// tick; `start_time_seconds` is `tick / tick_rate_hz`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChapterMarker {
    pub tick_index: u64,
    pub start_time_seconds: f64,
    pub title: String,
    pub event_type: String,
    pub event_id: String,
    pub category: Option<String>,
}

impl ChapterMarker {
    /// Compute the chapter's first-frame index at the export preset's
    /// fps. Used by VAL-M10B-023's frame-accuracy assertion.
    #[must_use]
    pub fn frame_index(&self, fps: u32) -> i64 {
        (self.start_time_seconds * fps as f64).round() as i64
    }
}

/// Per-bundle chapter-derivation context.
pub struct ChapterDerivation<'a> {
    pub rules: &'a ChapterRuleSet,
    pub tick_rate_hz: u32,
}

impl<'a> ChapterDerivation<'a> {
    /// Derive chapter markers from `events`. Returns markers in
    /// `tick_index` ascending order (stable sort: equal-tick events
    /// preserve their relative order from the input slice).
    pub fn derive(&self, events: &[Event]) -> Vec<ChapterMarker> {
        let mut markers: Vec<ChapterMarker> = Vec::new();
        for event in events {
            let matching_rule = self.rules.rules.iter().find(|rule| {
                if rule.event_type != event.event_type {
                    return false;
                }
                match rule.status_filter.as_deref() {
                    None => true,
                    Some(want) => event
                        .payload
                        .get("status")
                        .and_then(|v| v.as_str())
                        .map(|s| s == want)
                        .unwrap_or(false),
                }
            });
            if let Some(rule) = matching_rule {
                let title = interpolate(&rule.template, event);
                let start_time_seconds = if self.tick_rate_hz == 0 {
                    0.0
                } else {
                    event.tick as f64 / self.tick_rate_hz as f64
                };
                markers.push(ChapterMarker {
                    tick_index: event.tick,
                    start_time_seconds,
                    title,
                    event_type: event.event_type.clone(),
                    event_id: event.event_id.clone(),
                    category: rule.category.clone(),
                });
            }
        }
        markers.sort_by_key(|m| m.tick_index);
        markers
    }
}

/// Interpolate `{placeholder}` tokens against the event's payload +
/// envelope. Strict braces (no nested expressions, no escape syntax
/// per spec § Notes "the rule engine intentionally stays declarative").
#[must_use]
pub fn interpolate(template: &str, event: &Event) -> String {
    let mut out = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '{' {
            out.push(ch);
            continue;
        }
        let mut key = String::new();
        while let Some(&inner) = chars.peek() {
            chars.next();
            if inner == '}' {
                break;
            }
            key.push(inner);
        }
        let value = lookup(&key, event).unwrap_or_else(|| format!("{{{key}}}"));
        out.push_str(&value);
    }
    out
}

fn lookup(key: &str, event: &Event) -> Option<String> {
    if let Some(value) = event.payload.get(key) {
        return Some(match value {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        });
    }
    match key {
        "actor_id" => event.actor_id.map(|v| v.to_string()),
        "source_id" => event.source_id.map(|v| v.to_string()),
        "tick" => Some(event.tick.to_string()),
        "team" => event.team.clone(),
        _ => None,
    }
}

/// Count derived markers grouped by event_type. Used by tests +
/// audit logs to cross-check the 12+3+7+4 = 26 fixture totals.
#[must_use]
pub fn counts_by_event_type(markers: &[ChapterMarker]) -> BTreeMap<String, usize> {
    let mut map: BTreeMap<String, usize> = BTreeMap::new();
    for m in markers {
        *map.entry(m.event_type.clone()).or_insert(0) += 1;
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chapter_markers::ChapterRule;

    fn rules_set() -> ChapterRuleSet {
        ChapterRuleSet {
            rules: vec![
                ChapterRule {
                    event_type: "mission.objective_started".into(),
                    template: "Objective started: {objective_name}".into(),
                    status_filter: None,
                    category: Some("objective".into()),
                },
                ChapterRule {
                    event_type: "mission.objective_completed".into(),
                    template: "Objective completed: {objective_name}".into(),
                    status_filter: None,
                    category: Some("objective".into()),
                },
                ChapterRule {
                    event_type: "mission.objective_failed".into(),
                    template: "Objective failed: {objective_name}".into(),
                    status_filter: None,
                    category: Some("objective".into()),
                },
                ChapterRule {
                    event_type: "actor_status_changed".into(),
                    template: "{actor_name} killed".into(),
                    status_filter: Some("killed".into()),
                    category: Some("death".into()),
                },
                ChapterRule {
                    event_type: "reactor.armor_layer_destroyed".into(),
                    template: "Reactor armor layer destroyed".into(),
                    status_filter: None,
                    category: Some("reactor".into()),
                },
                ChapterRule {
                    event_type: "atmos.breach_detected".into(),
                    template: "Atmospheric breach: {region}".into(),
                    status_filter: None,
                    category: Some("atmos".into()),
                },
            ],
        }
    }

    fn objective_event(tick: u64, idx: usize, kind: &str, name: &str) -> Event {
        Event {
            schema_version: cf_replay::EVENT_SCHEMA_VERSION.to_string(),
            run_id: "fixture".into(),
            tick,
            sim_time_ms: tick as f64 / 60.0 * 1000.0,
            event_id: format!("obj_{kind}_{idx}"),
            category: "mission".into(),
            event_type: format!("mission.objective_{kind}"),
            payload: serde_json::json!({"objective_name": name}),
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

    fn kill_event(tick: u64, idx: usize, victim: u64, actor_name: &str) -> Event {
        Event {
            schema_version: cf_replay::EVENT_SCHEMA_VERSION.to_string(),
            run_id: "fixture".into(),
            tick,
            sim_time_ms: tick as f64 / 60.0 * 1000.0,
            event_id: format!("kill_{idx}"),
            category: "actor".into(),
            event_type: "actor_status_changed".into(),
            payload: serde_json::json!({
                "status": "killed",
                "actor_name": actor_name,
            }),
            parent_event_id: None,
            actor_id: Some(victim),
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

    fn armor_event(tick: u64, idx: usize) -> Event {
        Event {
            schema_version: cf_replay::EVENT_SCHEMA_VERSION.to_string(),
            run_id: "fixture".into(),
            tick,
            sim_time_ms: tick as f64 / 60.0 * 1000.0,
            event_id: format!("armor_{idx}"),
            category: "reactor".into(),
            event_type: "reactor.armor_layer_destroyed".into(),
            payload: serde_json::json!({}),
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

    fn breach_event(tick: u64, idx: usize, region: &str) -> Event {
        Event {
            schema_version: cf_replay::EVENT_SCHEMA_VERSION.to_string(),
            run_id: "fixture".into(),
            tick,
            sim_time_ms: tick as f64 / 60.0 * 1000.0,
            event_id: format!("breach_{idx}"),
            category: "atmos".into(),
            event_type: "atmos.breach_detected".into(),
            payload: serde_json::json!({"region": region}),
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

    /// deaths + 4 breaches = 26 chapter markers.
    #[test]
    fn chapter_markers_26_fixture_produces_26_markers() {
        let rules = rules_set();
        let derivation = ChapterDerivation {
            rules: &rules,
            tick_rate_hz: 60,
        };
        // 30-min reactor-defense fixture. Spread events deterministically
        // across the tick budget (60 Hz × 30 min × 60 s = 108_000 ticks).
        let mut events: Vec<Event> = Vec::new();
        for i in 0..4 {
            events.push(objective_event(1_000 + i * 2_000, i as usize, "started", "secure_a"));
            events.push(objective_event(1_500 + i * 2_000, i as usize, "completed", "secure_a"));
            events.push(objective_event(
                1_700 + i * 2_000,
                i as usize,
                "failed",
                "defend_b",
            ));
        }
        for i in 0..3u64 {
            events.push(armor_event(20_000 + i * 1_000, i as usize));
        }
        for i in 0..7u64 {
            events.push(kill_event(40_000 + i * 1_500, i as usize, 100 + i, "player"));
        }
        for i in 0..4u64 {
            events.push(breach_event(80_000 + i * 1_000, i as usize, "med_bay"));
        }
        let markers = derivation.derive(&events);
        assert_eq!(markers.len(), 26, "fixture must produce 26 markers");
        let counts = counts_by_event_type(&markers);
        assert_eq!(counts.get("mission.objective_started"), Some(&4));
        assert_eq!(counts.get("mission.objective_completed"), Some(&4));
        assert_eq!(counts.get("mission.objective_failed"), Some(&4));
        assert_eq!(counts.get("reactor.armor_layer_destroyed"), Some(&3));
        assert_eq!(counts.get("actor_status_changed"), Some(&7));
        assert_eq!(counts.get("atmos.breach_detected"), Some(&4));
        assert_eq!(counts.values().sum::<usize>(), 26);
    }

    /// event's tick within ≤ 1 frame at 60 fps.
    #[test]
    fn chapter_timecodes_are_frame_accurate_to_within_one_frame() {
        let rules = rules_set();
        let derivation = ChapterDerivation {
            rules: &rules,
            tick_rate_hz: 60,
        };
        // Synthesize 100 kill events at deterministic ticks.
        let events: Vec<Event> = (0..100u64)
            .map(|i| kill_event(i * 37 + 13, i as usize, i, "player"))
            .collect();
        let markers = derivation.derive(&events);
        assert_eq!(markers.len(), 100);
        for marker in &markers {
            let preset_fps = 60u32;
            let expected_frame = marker.frame_index(preset_fps);
            // Round-trip: ticks → seconds → frames.
            let source_frame = marker.tick_index as i64;
            let diff = (expected_frame - source_frame).abs();
            assert!(
                diff <= 1,
                "marker @ tick {} produced frame {} (expected {} ±1)",
                marker.tick_index,
                expected_frame,
                source_frame
            );
        }
    }

    /// Markers are returned in tick-ascending order.
    #[test]
    fn markers_returned_in_tick_ascending_order() {
        let rules = rules_set();
        let derivation = ChapterDerivation {
            rules: &rules,
            tick_rate_hz: 60,
        };
        // Insert events out-of-order; derivation must reorder.
        let events = vec![
            kill_event(500, 0, 1, "p1"),
            kill_event(100, 1, 2, "p2"),
            kill_event(300, 2, 3, "p3"),
        ];
        let markers = derivation.derive(&events);
        let ticks: Vec<u64> = markers.iter().map(|m| m.tick_index).collect();
        assert_eq!(ticks, vec![100, 300, 500]);
    }

    /// Template `{placeholder}` substitution against the payload.
    #[test]
    fn template_interpolation_substitutes_payload_fields() {
        let rules = rules_set();
        let derivation = ChapterDerivation {
            rules: &rules,
            tick_rate_hz: 60,
        };
        let markers = derivation.derive(&[breach_event(1234, 0, "engineering")]);
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].title, "Atmospheric breach: engineering");
    }

    /// `status_filter` correctly narrows `actor_status_changed` to
    /// `killed` only.
    #[test]
    fn status_filter_narrows_actor_status_changed() {
        let rules = rules_set();
        let derivation = ChapterDerivation {
            rules: &rules,
            tick_rate_hz: 60,
        };
        // One killed + one wounded → only 1 marker.
        let mut wounded = kill_event(200, 0, 1, "player");
        wounded.payload["status"] = "wounded".into();
        let killed = kill_event(300, 1, 1, "player");
        let markers = derivation.derive(&[wounded, killed]);
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].tick_index, 300);
    }
}
