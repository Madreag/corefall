//! M2 + M10 — Mission resolved modal + in-game death recap renderer.
//!
//! When `mission.mission_resolved` fires with `result=lost`, cf-ui needs
//! to surface a plain-language "Show me why" recap to the player without
//! exposing raw event JSON. This module provides:
//!
//! - the legacy public surface (`show_replay_cta_event_id`, `HudMission`)
//!   imported by cf-app — unchanged.
//! - the new [`render_recap_text`] entry point: given a slice of recent
//!   events + the divergence `event_id` (HudMission.show_me_why_event_id),
//!   render a 5-line plain-language recap suitable for a modal body.
//!
//! Templates are local to cf-ui so this module never reaches into
//! cf-tools-replay-viewer (which is a dev/CLI crate). The template
//! coverage is the M4 + M7 + M9 event surface; unknown event types render
//! as `tick <N>: event <category>.<type>` — never raw payload, per
//! M10 spec § "Death recap by ..." Gherkin scenarios.

pub use crate::{show_replay_cta_event_id, HudMission};

use serde_json::Value;

/// One event in the recap renderer's input slice. Mirrors the structure
/// of `cf_replay::Event` but stays local to cf-ui so cf-ui has no dep on
/// the replay crate. cf-app converts its in-memory recorder events to
/// this shape via the public field set.
#[derive(Debug, Clone)]
pub struct RecapEvent {
    pub event_id: String,
    pub parent_event_id: Option<String>,
    pub tick: u64,
    pub category: String,
    pub event_type: String,
    pub payload: Value,
}

/// Maximum number of lines surfaced in the modal body. Keeps the modal
/// compact + readable at 200% UI scale (M11 ACC-A floor).
pub const MAX_RECAP_LINES: usize = 6;

/// Render a death recap text for the modal. Walks the parent chain
/// backwards from `divergence_event_id`, picks the most informative
/// 6 events, and emits one plain-language sentence per line.
///
/// Returns a graceful fallback message when no events or no divergence
/// event are available — per M10 spec, the modal must NEVER show raw
/// payload JSON; this contract holds even when the chain is empty.
pub fn render_recap_text(events: &[RecapEvent], divergence_event_id: Option<&str>) -> String {
    if events.is_empty() {
        return "Cause chain not available — see replay viewer for full bundle.".to_string();
    }
    let Some(div_id) = divergence_event_id else {
        return "No divergence event recorded; open the replay viewer for chronological events.".to_string();
    };

    let Some(start_idx) = events.iter().position(|e| e.event_id == div_id) else {
        return format!("Divergence event `{div_id}` not in recent events; open the replay viewer for the full chain.");
    };

    let mut by_id: std::collections::BTreeMap<&str, &RecapEvent> = std::collections::BTreeMap::new();
    for e in events {
        by_id.insert(e.event_id.as_str(), e);
    }
    let mut chain: Vec<&RecapEvent> = vec![&events[start_idx]];
    let mut current = &events[start_idx];
    let mut visited: std::collections::HashSet<&str> = std::collections::HashSet::new();
    visited.insert(current.event_id.as_str());
    while chain.len() < MAX_RECAP_LINES {
        let Some(parent) = current.parent_event_id.as_deref() else {
            break;
        };
        if visited.contains(parent) {
            break;
        }
        match by_id.get(parent) {
            Some(p) => {
                visited.insert(p.event_id.as_str());
                chain.push(p);
                current = p;
            }
            None => break,
        }
    }
    let mut lines: Vec<String> = Vec::with_capacity(chain.len());
    for link in chain.iter() {
        lines.push(format!("Tick {}: {}", link.tick, render_body(link)));
    }
    lines.join("\n")
}

/// Per-event plain-language template. Mirrors the cf-tools-replay-viewer
/// renderer but stays local to cf-ui to avoid a crate-dep cycle. Coverage
/// is the M2 + M7 + M9 event surface; everything else falls back to a
/// short event-type label.
fn render_body(event: &RecapEvent) -> String {
    let p = &event.payload;
    let category = event.category.as_str();
    match event.event_type.as_str() {
        "actor_died" => format!(
            "You died ({}).",
            field_str(p, "cause").unwrap_or_else(|| "cause unknown".into())
        ),
        "wound_added" => format!(
            "You were wounded ({} severity).",
            field_str(p, "severity").unwrap_or_else(|| "unknown".into())
        ),
        "actor_status_changed" => format!(
            "Your status: {} → {} ({}).",
            field_str(p, "from")
                .or_else(|| field_str(p, "from_status"))
                .unwrap_or_else(|| "?".into()),
            field_str(p, "to")
                .or_else(|| field_str(p, "to_status"))
                .unwrap_or_else(|| "?".into()),
            field_str(p, "cause").unwrap_or_else(|| "—".into())
        ),
        "projectile_hit" | "projectile_hit_mo" => format!(
            "{}'s shot hit your {} for {} damage.",
            actor_label(p, "shooter_actor_id")
                .or_else(|| actor_label(p, "shooter"))
                .unwrap_or_else(|| "An enemy".into()),
            field_str(p, "body_zone")
                .or_else(|| field_str(p, "surface_kind"))
                .unwrap_or_else(|| "body".into()),
            field_f64(p, "damage").map(short_num).unwrap_or_else(|| "?".into())
        ),
        "weapon_fired" => format!(
            "{}'s {} fired.",
            actor_label(p, "shooter_actor_id")
                .or_else(|| actor_label(p, "shooter"))
                .unwrap_or_else(|| "An enemy".into()),
            field_str(p, "weapon_name")
                .or_else(|| field_str(p, "weapon"))
                .unwrap_or_else(|| "weapon".into())
        ),
        "target_acquired" => format!(
            "{} acquired you as a target ({}).",
            actor_label(p, "actor_id").unwrap_or_else(|| "An enemy".into()),
            field_str(p, "reason").unwrap_or_else(|| "visible".into())
        ),
        "missed_shot_reason" => format!(
            "{}'s shot missed ({}).",
            actor_label(p, "actor_id").unwrap_or_else(|| "An enemy".into()),
            field_str(p, "reason").unwrap_or_else(|| "off-target".into())
        ),
        "tactic_chosen" => format!(
            "{} chose tactic `{}`.",
            actor_label(p, "actor_id").unwrap_or_else(|| "An enemy".into()),
            field_str(p, "tactic").unwrap_or_else(|| "—".into())
        ),
        // M7 smart-AI reason-label surface: cf-app populates these into
        // the modal so the player sees WHY the AI did what it did.
        "reason_label_changed" => format!(
            "Bot {} chose `{}` (score {}).",
            actor_label(p, "actor_id").unwrap_or_else(|| "—".into()),
            field_str(p, "chosen_task").unwrap_or_else(|| "—".into()),
            field_f64(p, "score").map(short_num).unwrap_or_else(|| "?".into())
        ),
        // Hazard / affliction surface (M9 deep damage).
        "spawned" if category == "hazard" => format!(
            "{} hazard spawned nearby.",
            field_str(p, "kind").unwrap_or_else(|| "Hazard".into())
        ),
        "actor_contact" if category == "hazard" => format!(
            "You touched a {} hazard.",
            field_str(p, "kind").unwrap_or_else(|| "hazard".into())
        ),
        "applied" if category == "affliction" => format!(
            "You were afflicted with {} (severity {}).",
            field_str(p, "kind").unwrap_or_else(|| "an affliction".into()),
            field_f64(p, "severity").map(short_num).unwrap_or_else(|| "?".into())
        ),
        "mission_resolved" => match (
            field_str(p, "result"),
            field_str(p, "loss_reason").or_else(|| field_str(p, "reason")),
        ) {
            (Some(r), Some(reason)) => format!("Mission ended — {r} ({reason})."),
            (Some(r), None) => format!("Mission ended — {r}."),
            (None, _) => "Mission ended.".to_string(),
        },
        // Fallback: short event-type label, NEVER raw payload.
        _ => format!("event `{}.{}`", category, event.event_type),
    }
}

fn field_str(p: &Value, key: &str) -> Option<String> {
    p.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn field_f64(p: &Value, key: &str) -> Option<f64> {
    p.get(key).and_then(|v| v.as_f64()).filter(|f| f.is_finite())
}

fn actor_label(p: &Value, key: &str) -> Option<String> {
    if let Some(name) = p.get(format!("{key}_name")).and_then(|v| v.as_str()) {
        return Some(name.to_string());
    }
    let n = p.get(key).and_then(|v| v.as_u64())?;
    Some(format!("Bot #{n}"))
}

fn short_num(f: f64) -> String {
    if !f.is_finite() {
        return "?".into();
    }
    if f.fract().abs() < 1e-6 {
        format!("{:.0}", f)
    } else {
        format!("{:.2}", f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn event(
        id: &str,
        parent: Option<&str>,
        tick: u64,
        category: &str,
        event_type: &str,
        payload: Value,
    ) -> RecapEvent {
        RecapEvent {
            event_id: id.into(),
            parent_event_id: parent.map(|s| s.into()),
            tick,
            category: category.into(),
            event_type: event_type.into(),
            payload,
        }
    }

    #[test]
    fn empty_events_yields_graceful_fallback() {
        let s = render_recap_text(&[], Some("anything"));
        assert!(s.contains("not available"));
        assert!(!s.contains("{"));
    }

    #[test]
    fn missing_divergence_id_yields_graceful_fallback() {
        let events = vec![event("e1", None, 0, "system", "run_started", json!({}))];
        let s = render_recap_text(&events, None);
        assert!(s.contains("No divergence event recorded"));
    }

    #[test]
    fn missing_divergence_event_in_slice_yields_graceful_fallback() {
        let events = vec![event("e1", None, 0, "system", "run_started", json!({}))];
        let s = render_recap_text(&events, Some("e_missing"));
        assert!(s.contains("not in recent events"));
        assert!(s.contains("e_missing"));
    }

    #[test]
    fn renders_basic_actor_died_chain_as_plain_language() {
        let events = vec![
            event("e1", None, 0, "system", "run_started", json!({})),
            event(
                "e2",
                Some("e1"),
                10,
                "ai",
                "target_acquired",
                json!({"actor_id": 7, "reason": "saw you"}),
            ),
            event(
                "e3",
                Some("e2"),
                12,
                "equipment",
                "weapon_fired",
                json!({"shooter_actor_id": 7, "weapon_name": "rifle"}),
            ),
            event(
                "e4",
                Some("e3"),
                14,
                "combat",
                "projectile_hit",
                json!({"shooter_actor_id": 7, "body_zone": "torso", "damage": 15.0}),
            ),
            event(
                "e5",
                Some("e4"),
                15,
                "actor",
                "actor_died",
                json!({"actor_id": 1, "cause": "projectile"}),
            ),
        ];
        let s = render_recap_text(&events, Some("e5"));
        assert!(s.contains("Tick 15"), "missing tick for actor_died: {s}");
        assert!(s.contains("You died"), "missing player-facing died line: {s}");
        assert!(s.contains("Bot #7"), "missing shooter label: {s}");
        assert!(s.contains("rifle"));
        assert!(s.contains("hit your torso"));
        assert!(s.contains("acquired you"));
        assert!(!s.contains("{"), "raw JSON must NOT leak: {s}");
    }

    #[test]
    fn hazard_recap_renders_kind_name() {
        let events = vec![
            event("h1", None, 100, "hazard", "spawned", json!({"kind": "electric"})),
            event(
                "h2",
                Some("h1"),
                102,
                "hazard",
                "actor_contact",
                json!({"actor_id": 1, "kind": "electric"}),
            ),
            event(
                "h3",
                Some("h2"),
                120,
                "actor",
                "actor_died",
                json!({"actor_id": 1, "cause": "electric"}),
            ),
        ];
        let s = render_recap_text(&events, Some("h3"));
        assert!(s.contains("You died (electric)"));
        assert!(s.contains("You touched a electric hazard"));
        assert!(s.contains("electric hazard spawned nearby"));
    }

    #[test]
    fn unknown_event_type_falls_back_to_label_not_json() {
        let events = vec![event(
            "x1",
            None,
            42,
            "totally_unknown_category",
            "totally_unknown_type",
            json!({"raw": {"deep": "value"}}),
        )];
        let s = render_recap_text(&events, Some("x1"));
        assert!(s.contains("event `totally_unknown_category.totally_unknown_type`"));
        assert!(!s.contains("deep"));
        assert!(!s.contains("{"));
    }
}
