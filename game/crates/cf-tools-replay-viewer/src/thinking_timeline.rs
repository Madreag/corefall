//! M10 — per-bot thinking-timeline panel (M7 5-layer AI stack visualization).
//!
//! Reads `ai.reason_label_changed` + `ai.thinking_layer_invoked` events
//! from a bundle and surfaces a per-actor timeline that the replay viewer
//! / debrief / "Why did my bot do that?" UX consume.
//!
//! Per the M10 spec § "Per-bot thinking timeline panel":
//!
//! > Reads from M8A's `reason_label_recent` + utility_candidates +
//! > htn_goal_stack + behavior_tree_node fields serialized in the snapshot
//! > envelope. M10's debrief markdown (death-recap section) renders the
//! > killed actor's full thinking timeline for the last 10 ticks before
//! > death — gives the player closure on "why did my Medic die instead of
//! > triaging me?"
//!
//! The renderer is read-only: it never mutates the bundle and never panics
//! on missing fields. Bundles without AI events render an empty timeline.

use std::collections::BTreeMap;
use std::fmt::Write;

use cf_replay::Event;
use serde_json::Value;

use crate::bundle::Bundle;

/// One row of the per-bot thinking timeline.
#[derive(Debug, Clone, PartialEq)]
pub struct ThinkingTimelineEntry {
    pub tick: u64,
    /// Layer set that fired this tick (sorted, comma-joined).
    pub layers: String,
    /// Most recent reason label string (from
    /// `ai.reason_label_changed.label`) if one was emitted at or before
    /// this tick.
    pub reason_label: Option<String>,
    /// Chosen task (most recent reason_label_changed payload field).
    pub chosen_task: Option<String>,
    /// Chosen target (most recent reason_label_changed payload field).
    pub chosen_target: Option<String>,
    /// Final utility score for the chosen task (most recent reason_label
    /// payload field).
    pub score: Option<f64>,
    /// Doctrine prior (M7-A Layer 5).
    pub doctrine: Option<String>,
    /// Archetype name from reason_label.
    pub role: Option<String>,
    /// HTN goal stack (e.g. "protect_squad/triage_medic_route").
    pub htn_goal_stack: Option<String>,
    /// Behavior-tree node trail (e.g. "approach_ally→treat_loop").
    pub behavior_tree_node: Option<String>,
    /// True when the Reactive layer overrode the chosen plan this tick.
    pub reactive_override: bool,
    /// Event ids that contributed to this row (one per AI event).
    pub source_event_ids: Vec<String>,
}

/// Build a per-bot timeline for `actor_id` from a bundle. The result is
/// chronological by tick; consecutive rows with the same layer set may
/// share a reason_label if no new label was emitted between them.
pub fn build_timeline(bundle: &Bundle, actor_id: u64) -> Vec<ThinkingTimelineEntry> {
    // First pass: group AI events by tick.
    let mut by_tick: BTreeMap<u64, Vec<&Event>> = BTreeMap::new();
    for event in &bundle.events {
        if event.category != "ai" {
            continue;
        }
        if event.event_type != "reason_label_changed" && event.event_type != "thinking_layer_invoked" {
            continue;
        }
        if event_actor_id(event) != Some(actor_id) {
            continue;
        }
        by_tick.entry(event.tick).or_default().push(event);
    }

    let mut out: Vec<ThinkingTimelineEntry> = Vec::with_capacity(by_tick.len());
    let mut last_reason_label: Option<String> = None;
    let mut last_chosen_task: Option<String> = None;
    let mut last_chosen_target: Option<String> = None;
    let mut last_score: Option<f64> = None;
    let mut last_doctrine: Option<String> = None;
    let mut last_role: Option<String> = None;
    let mut last_htn: Option<String> = None;
    let mut last_bt: Option<String> = None;
    for (tick, events) in by_tick {
        let mut layers: Vec<String> = Vec::new();
        let mut reactive_override = false;
        let mut source_event_ids: Vec<String> = Vec::with_capacity(events.len());
        for e in &events {
            source_event_ids.push(e.event_id.clone());
            match e.event_type.as_str() {
                "thinking_layer_invoked" => {
                    if let Some(arr) = e.payload.get("layers").and_then(|v| v.as_array()) {
                        for layer in arr {
                            if let Some(s) = layer.as_str() {
                                layers.push(s.to_string());
                            }
                        }
                    }
                    if e.payload
                        .get("reactive_override")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                    {
                        reactive_override = true;
                    }
                }
                "reason_label_changed" => {
                    last_reason_label = field_str(&e.payload, "label");
                    last_chosen_task = field_str(&e.payload, "chosen_task");
                    last_chosen_target = field_str(&e.payload, "chosen_target");
                    last_score = field_f64(&e.payload, "score");
                    last_doctrine = field_str(&e.payload, "doctrine");
                    last_role = field_str(&e.payload, "role");
                    last_htn = field_str(&e.payload, "htn_goal_stack");
                    last_bt = field_str(&e.payload, "behavior_tree_node");
                }
                _ => {}
            }
        }
        layers.sort();
        layers.dedup();
        let layers_str = if layers.is_empty() {
            "—".to_string()
        } else {
            layers.join(", ")
        };
        out.push(ThinkingTimelineEntry {
            tick,
            layers: layers_str,
            reason_label: last_reason_label.clone(),
            chosen_task: last_chosen_task.clone(),
            chosen_target: last_chosen_target.clone(),
            score: last_score,
            doctrine: last_doctrine.clone(),
            role: last_role.clone(),
            htn_goal_stack: last_htn.clone(),
            behavior_tree_node: last_bt.clone(),
            reactive_override,
            source_event_ids,
        });
    }
    out
}

/// Render the timeline as a deterministic markdown table.
pub fn render_markdown(actor_id: u64, entries: &[ThinkingTimelineEntry]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "## Thinking Timeline — actor #{actor_id}");
    let _ = writeln!(out);
    if entries.is_empty() {
        let _ = writeln!(
            out,
            "_no `ai.reason_label_changed` or `ai.thinking_layer_invoked` events for this actor in this bundle_"
        );
        return out;
    }
    let _ = writeln!(
        out,
        "| tick | layers fired | chosen_task | target | score | doctrine | role | reactive_override |"
    );
    let _ = writeln!(
        out,
        "|------|--------------|-------------|--------|-------|----------|------|-------------------|"
    );
    for e in entries {
        let _ = writeln!(
            out,
            "| {tick} | {layers} | {task} | {target} | {score} | {doctrine} | {role} | {ro} |",
            tick = e.tick,
            layers = e.layers,
            task = e.chosen_task.clone().unwrap_or_else(|| "—".into()),
            target = e.chosen_target.clone().unwrap_or_else(|| "—".into()),
            score = e.score.map(|s| format!("{:.2}", s)).unwrap_or_else(|| "—".into()),
            doctrine = e.doctrine.clone().unwrap_or_else(|| "—".into()),
            role = e.role.clone().unwrap_or_else(|| "—".into()),
            ro = if e.reactive_override { "yes" } else { "no" },
        );
    }
    if entries
        .iter()
        .any(|e| e.htn_goal_stack.is_some() || e.behavior_tree_node.is_some())
    {
        let _ = writeln!(out);
        let _ = writeln!(out, "### Latest plan trail");
        let _ = writeln!(out);
        let last = entries.last().unwrap();
        if let Some(g) = &last.htn_goal_stack {
            let _ = writeln!(out, "- HTN goal stack: `{g}`");
        }
        if let Some(bt) = &last.behavior_tree_node {
            let _ = writeln!(out, "- Behavior tree node: `{bt}`");
        }
        if let Some(label) = &last.reason_label {
            let _ = writeln!(out, "- Reason label: `{label}`");
        }
    }
    out
}

/// JSON representation of the timeline for tooling.
pub fn render_json(actor_id: u64, entries: &[ThinkingTimelineEntry]) -> serde_json::Value {
    serde_json::json!({
        "actor_id": actor_id,
        "entries": entries.iter().map(|e| serde_json::json!({
            "tick": e.tick,
            "layers": e.layers,
            "chosen_task": e.chosen_task,
            "chosen_target": e.chosen_target,
            "score": e.score,
            "doctrine": e.doctrine,
            "role": e.role,
            "htn_goal_stack": e.htn_goal_stack,
            "behavior_tree_node": e.behavior_tree_node,
            "reactive_override": e.reactive_override,
            "source_event_ids": e.source_event_ids,
            "reason_label": e.reason_label,
        })).collect::<Vec<_>>(),
    })
}

/// Slice the timeline to the last `n` entries at or before `at_tick`.
/// `at_tick=None` means "all up to the latest"; `n=None` means "all".
pub fn slice_window(
    entries: &[ThinkingTimelineEntry],
    at_tick: Option<u64>,
    last_n: Option<usize>,
) -> Vec<ThinkingTimelineEntry> {
    let mut filtered: Vec<ThinkingTimelineEntry> = entries
        .iter()
        .filter(|e| match at_tick {
            Some(t) => e.tick <= t,
            None => true,
        })
        .cloned()
        .collect();
    if let Some(n) = last_n {
        if filtered.len() > n {
            let drop = filtered.len() - n;
            filtered.drain(..drop);
        }
    }
    filtered
}

fn field_str(p: &Value, key: &str) -> Option<String> {
    p.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn field_f64(p: &Value, key: &str) -> Option<f64> {
    p.get(key).and_then(|v| v.as_f64()).filter(|f| f.is_finite())
}

fn event_actor_id(event: &Event) -> Option<u64> {
    if let Some(id) = event.actor_id {
        return Some(id);
    }
    event.payload.get("actor_id").and_then(|v| v.as_u64())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::io::Write;

    fn write_bundle(test_name: &str, events: Vec<serde_json::Value>) -> Bundle {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "cf_replay_viewer_thinking_{}_{}",
            test_name,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        let run_id = "m10_thinking_test";
        let manifest = serde_json::json!({
            "schema_version": "prototype-run-manifest.v0.1",
            "run_id": run_id,
            "prototype_slice": "M10",
            "run_mode": "test",
            "milestone": "m10",
            "build": {"commit_sha": "d", "rust_version": "x", "bevy_version": "y", "platform": "test"},
            "scene": {"id": "s", "display_name": "s", "source_path": "x"},
            "seed": 42,
            "started_at_utc": "2026-05-15T00:00:00Z",
            "duration_target_sec": 1.0,
            "material_schema_version": "n/a",
            "config_hash": "x",
            "assumptions_tested": [],
            "linked_specs": [],
            "expected_tests": [],
            "capture_config": {"events": true, "screenshots": false, "captures": false},
            "schemas": {"control": 1, "scenario": 1, "events": 1},
            "capabilities": {"debug": false, "control_api": true, "save_load": false, "debug_capabilities": []},
            "settings": {"ui_scale": 1.0, "high_contrast": false, "captions": true, "reduced_motion": false, "reduced_shake": false, "reduced_flash": false},
            "checksum": {"algorithm": "blake3", "scope": "sim_state_v1", "cadence_ticks": 60},
            "tick_rate_hz": 60,
            "expected_outcome": "clean"
        });
        std::fs::write(
            p.join("run_manifest.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let mut by_category: BTreeMap<String, u64> = BTreeMap::new();
        let mut by_type: BTreeMap<String, u64> = BTreeMap::new();
        for e in &events {
            *by_category
                .entry(e.get("category").and_then(|v| v.as_str()).unwrap().to_string())
                .or_insert(0) += 1;
            *by_type
                .entry(e.get("event_type").and_then(|v| v.as_str()).unwrap().to_string())
                .or_insert(0) += 1;
        }
        let summary = serde_json::json!({
            "schema_version": "prototype-run-summary.v0.1",
            "run_id": run_id,
            "manifest_run_id": run_id,
            "duration_sec": 1.0,
            "result": "pass",
            "ended_at_utc": "2026-05-15T00:00:01Z",
            "exit_code": 0,
            "event_counts": {"total": events.len(), "by_category": by_category, "by_type": by_type, "by_severity": {"error": 0, "warn": 0}, "dropped_total": 0},
            "volume": {"events_jsonl_bytes": 0, "event_lines": events.len()},
            "performance": {"avg_frame_ms": 0.0, "p99_frame_ms": 0.0, "avg_tick_ms": 0.0, "p99_tick_ms": 0.0, "ticks_run": 0, "wall_seconds": 0.0, "tick_rate_hz": 60},
            "artifacts": {"items": []},
            "blockers": [],
            "next_actions": [],
            "tests": [],
            "final_sim_checksum": null,
            "checksum_event_count": 0,
            "first_tick": 0,
            "last_tick": 0
        });
        std::fs::write(p.join("summary.json"), serde_json::to_vec_pretty(&summary).unwrap()).unwrap();
        let mut events_file = std::fs::File::create(p.join("events.jsonl")).unwrap();
        for e in &events {
            writeln!(events_file, "{}", serde_json::to_string(e).unwrap()).unwrap();
        }
        drop(events_file);
        Bundle::load(&p).expect("thinking-timeline test bundle loads")
    }

    fn ai_event(run_id: &str, tick: u64, seq: u64, event_type: &str, payload: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "schema_version": "prototype-recorder-event.v0.1",
            "run_id": run_id,
            "tick": tick,
            "sim_time_ms": (tick as f64) * 16.6,
            "event_id": format!("{run_id}:{tick}:{seq}"),
            "category": "ai",
            "event_type": event_type,
            "payload": payload,
        })
    }

    #[test]
    fn build_timeline_groups_layers_and_reason_by_tick() {
        let run = "m10_thinking_test";
        let events = vec![
            ai_event(
                run,
                10,
                0,
                "thinking_layer_invoked",
                serde_json::json!({"actor_id": 7, "layers": ["Utility", "BehaviorTree"]}),
            ),
            ai_event(
                run,
                10,
                1,
                "reason_label_changed",
                serde_json::json!({
                    "actor_id": 7,
                    "label": "chosen=TriageDownedAlly(Mendez); score=0.92",
                    "chosen_task": "TriageDownedAlly",
                    "chosen_target": "Mendez",
                    "score": 0.92,
                    "doctrine": "defensive",
                    "role": "medic",
                    "htn_goal_stack": "protect_squad/triage",
                    "behavior_tree_node": "approach_ally→treat_loop"
                }),
            ),
            ai_event(
                run,
                20,
                2,
                "thinking_layer_invoked",
                serde_json::json!({"actor_id": 7, "layers": ["Reactive"]}),
            ),
        ];
        let bundle = write_bundle("groups_by_tick", events);
        let timeline = build_timeline(&bundle, 7);
        assert_eq!(timeline.len(), 2);
        assert_eq!(timeline[0].tick, 10);
        assert_eq!(timeline[0].layers, "BehaviorTree, Utility");
        assert_eq!(timeline[0].chosen_task.as_deref(), Some("TriageDownedAlly"));
        assert_eq!(timeline[0].chosen_target.as_deref(), Some("Mendez"));
        assert_eq!(timeline[1].tick, 20);
        assert_eq!(timeline[1].layers, "Reactive");
        // Later rows inherit the most recent reason label until a new one fires.
        assert_eq!(timeline[1].chosen_task.as_deref(), Some("TriageDownedAlly"));
    }

    #[test]
    fn build_timeline_scopes_to_actor() {
        let run = "m10_thinking_test";
        let events = vec![
            ai_event(
                run,
                10,
                0,
                "thinking_layer_invoked",
                serde_json::json!({"actor_id": 7, "layers": ["Utility"]}),
            ),
            ai_event(
                run,
                10,
                1,
                "thinking_layer_invoked",
                serde_json::json!({"actor_id": 8, "layers": ["BehaviorTree"]}),
            ),
        ];
        let bundle = write_bundle("scopes_to_actor", events);
        let actor_7 = build_timeline(&bundle, 7);
        assert_eq!(actor_7.len(), 1);
        assert_eq!(actor_7[0].layers, "Utility");
        let actor_8 = build_timeline(&bundle, 8);
        assert_eq!(actor_8.len(), 1);
        assert_eq!(actor_8[0].layers, "BehaviorTree");
    }

    #[test]
    fn build_timeline_for_empty_bundle() {
        let bundle = write_bundle(
            "empty",
            vec![serde_json::json!({
                "schema_version": "prototype-recorder-event.v0.1",
                "run_id": "m10_thinking_test",
                "tick": 0,
                "sim_time_ms": 0.0,
                "event_id": "m10_thinking_test:0:0",
                "category": "system",
                "event_type": "run_started",
                "payload": {}
            })],
        );
        assert!(build_timeline(&bundle, 7).is_empty());
    }

    #[test]
    fn render_markdown_emits_table_or_empty_message() {
        let actor = 7;
        let empty: Vec<ThinkingTimelineEntry> = vec![];
        let md_empty = render_markdown(actor, &empty);
        assert!(md_empty.contains("no `ai.reason_label_changed`"));
        let entries = vec![ThinkingTimelineEntry {
            tick: 100,
            layers: "Utility, BehaviorTree".into(),
            reason_label: Some("chosen=Suppress; score=0.61".into()),
            chosen_task: Some("Suppress".into()),
            chosen_target: None,
            score: Some(0.61),
            doctrine: Some("aggressive".into()),
            role: Some("rifleman".into()),
            htn_goal_stack: Some("eliminate".into()),
            behavior_tree_node: Some("seek_cover→suppress".into()),
            reactive_override: false,
            source_event_ids: vec!["e1".into()],
        }];
        let md = render_markdown(actor, &entries);
        assert!(md.contains("Thinking Timeline — actor #7"));
        assert!(md.contains("Suppress"));
        assert!(md.contains("0.61"));
        assert!(md.contains("seek_cover→suppress"));
    }

    #[test]
    fn slice_window_returns_last_n_entries_at_or_before_tick() {
        let entries = vec![
            ThinkingTimelineEntry {
                tick: 10,
                layers: "Utility".into(),
                reason_label: None,
                chosen_task: None,
                chosen_target: None,
                score: None,
                doctrine: None,
                role: None,
                htn_goal_stack: None,
                behavior_tree_node: None,
                reactive_override: false,
                source_event_ids: vec![],
            },
            ThinkingTimelineEntry {
                tick: 20,
                layers: "Reactive".into(),
                reason_label: None,
                chosen_task: None,
                chosen_target: None,
                score: None,
                doctrine: None,
                role: None,
                htn_goal_stack: None,
                behavior_tree_node: None,
                reactive_override: true,
                source_event_ids: vec![],
            },
            ThinkingTimelineEntry {
                tick: 30,
                layers: "BehaviorTree".into(),
                reason_label: None,
                chosen_task: None,
                chosen_target: None,
                score: None,
                doctrine: None,
                role: None,
                htn_goal_stack: None,
                behavior_tree_node: None,
                reactive_override: false,
                source_event_ids: vec![],
            },
        ];
        let last_two_before_25 = slice_window(&entries, Some(25), Some(2));
        assert_eq!(last_two_before_25.len(), 2);
        assert_eq!(last_two_before_25[0].tick, 10);
        assert_eq!(last_two_before_25[1].tick, 20);
        let none_filter = slice_window(&entries, None, None);
        assert_eq!(none_filter.len(), 3);
    }
}
