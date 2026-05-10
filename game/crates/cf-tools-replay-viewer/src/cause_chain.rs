//! M3B-002: cause-chain view.
//!
//! Walks `parent_event_id` chains backwards from a terminal event so the
//! agent / player can see *why* something happened. Default trigger event
//! types per the M3B-002 task card are `actor_died`, `mission_resolved`,
//! `objective_failed`, terrain breach (`terrain_carved`), reactor destroyed
//! (`reactor_damaged` with `destroyed: true` payload OR an explicit
//! `reactor_destroyed` event when one exists), and `projectile_hit`.

use std::fmt::Write;

use cf_replay::Event;

use crate::bundle::Bundle;

/// Default cap on chain length. Chains in practice are at most 5-10 long
/// (`run_started` → `command_accepted` → … → terminal), but defensive against
/// pathological loops or malformed data.
pub const DEFAULT_MAX_DEPTH: usize = 64;

/// Default terminal event types the `cause-chain` command auto-discovers
/// when the caller does not pin a specific event id.
pub const DEFAULT_TRIGGER_TYPES: &[&str] = &[
    "actor_died",
    "mission_resolved",
    "objective_failed",
    "terrain_carved",
    "reactor_damaged",
    "reactor_destroyed",
    "projectile_hit",
];

/// One link in the chain.
#[derive(Debug, Clone)]
pub struct ChainLink<'a> {
    pub depth: usize,
    pub event: &'a Event,
}

/// Result of walking a chain.
#[derive(Debug, Clone)]
pub struct CauseChain<'a> {
    pub trigger: &'a Event,
    pub links: Vec<ChainLink<'a>>,
    pub terminated_reason: ChainTermination,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainTermination {
    /// Reached an event with no `parent_event_id` (root).
    RootReached,
    /// `parent_event_id` referenced an event missing from the bundle.
    /// `Bundle::load` rejects this at load time, so this is only reachable
    /// if a caller constructs a `Bundle` outside the loader.
    ParentMissingFromBundle,
    /// Hit `max_depth` before reaching root.
    MaxDepthReached,
    /// Detected a cycle (a parent we already visited).
    CycleDetected,
}

/// Walk the parent chain of `trigger` backwards.
pub fn trace<'a>(bundle: &'a Bundle, trigger: &'a Event, max_depth: usize) -> CauseChain<'a> {
    let mut links: Vec<ChainLink<'a>> = vec![ChainLink {
        depth: 0,
        event: trigger,
    }];
    let mut visited: std::collections::HashSet<&'a str> = std::collections::HashSet::new();
    visited.insert(trigger.event_id.as_str());
    let mut current = trigger;
    let mut termination = ChainTermination::RootReached;
    while let Some(parent_id) = current.parent_event_id.as_deref() {
        if links.len() >= max_depth {
            termination = ChainTermination::MaxDepthReached;
            break;
        }
        if visited.contains(parent_id) {
            termination = ChainTermination::CycleDetected;
            break;
        }
        match bundle.event_by_id(parent_id) {
            Some(parent) => {
                visited.insert(parent.event_id.as_str());
                links.push(ChainLink {
                    depth: links.len(),
                    event: parent,
                });
                current = parent;
            }
            None => {
                termination = ChainTermination::ParentMissingFromBundle;
                break;
            }
        }
    }
    CauseChain {
        trigger,
        links,
        terminated_reason: termination,
    }
}

/// Convenience: discover the default trigger events in the bundle and trace
/// each one. Returns chains in the order their triggers appear in `events`.
pub fn trace_default_triggers<'a>(bundle: &'a Bundle, max_depth: usize) -> Vec<CauseChain<'a>> {
    let mut chains: Vec<CauseChain<'a>> = Vec::new();
    for event in bundle.events.iter() {
        if !DEFAULT_TRIGGER_TYPES.contains(&event.event_type.as_str()) {
            continue;
        }
        // For `reactor_damaged`, only treat it as a destruction trigger when
        // the payload says the reactor was destroyed; otherwise the chain
        // would balloon with every minor hit.
        if event.event_type == "reactor_damaged" {
            let destroyed = event
                .payload
                .get("destroyed")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            if !destroyed {
                continue;
            }
        }
        // Same idea for `terrain_carved`: only the *first* carve in a run
        // is the breach trigger. Subsequent carves repeat the same parent
        // pattern and bloat the report.
        if event.event_type == "terrain_carved" && chains.iter().any(|c| c.trigger.event_type == "terrain_carved") {
            continue;
        }
        chains.push(trace(bundle, event, max_depth));
    }
    chains
}

/// Render a cause chain as deterministic markdown.
pub fn render_markdown(chain: &CauseChain<'_>) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "### `{}` at tick {} — `{}`",
        chain.trigger.event_type, chain.trigger.tick, chain.trigger.event_id
    );
    let _ = writeln!(out);
    let payload_summary = compact_payload(&chain.trigger.payload);
    let _ = writeln!(out, "Trigger payload: `{payload_summary}`");
    let _ = writeln!(out);

    if chain.links.len() <= 1 {
        let _ = writeln!(out, "_no parent chain (event was emitted directly without a parent)_");
    } else {
        let _ = writeln!(out, "Cause chain (newest → oldest):");
        let _ = writeln!(out);
        for link in &chain.links {
            let arrow = if link.depth == 0 { "→" } else { "↑" };
            let _ = writeln!(
                out,
                "{indent}{arrow} tick {tick} `{cat}.{ty}` (`{eid}`) `{payload}`",
                indent = " ".repeat(link.depth * 2),
                arrow = arrow,
                tick = link.event.tick,
                cat = link.event.category,
                ty = link.event.event_type,
                eid = link.event.event_id,
                payload = compact_payload(&link.event.payload),
            );
        }
    }
    let _ = writeln!(out);
    let term_label = match chain.terminated_reason {
        ChainTermination::RootReached => "root reached",
        ChainTermination::ParentMissingFromBundle => "parent missing from bundle (corrupt or external)",
        ChainTermination::MaxDepthReached => "max depth reached",
        ChainTermination::CycleDetected => "cycle detected",
    };
    let _ = writeln!(out, "Chain depth: {} · termination: {term_label}", chain.links.len());

    out
}

/// Render a list of chains under a single heading. Used by the CLI's
/// default-trigger mode (`cause-chain` with no specific event id/type).
pub fn render_markdown_multi(bundle: &Bundle, chains: &[CauseChain<'_>]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Cause Chains — `{}`", bundle.manifest.run_id);
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "Scenario `{}` ({}); milestone `{}`; tick rate {} Hz; total events {}.",
        bundle.manifest.scene.id,
        bundle.manifest.scene.display_name,
        bundle.manifest.milestone,
        bundle.manifest.tick_rate_hz,
        bundle.summary.event_counts.total,
    );
    let _ = writeln!(out);
    if chains.is_empty() {
        let _ = writeln!(
            out,
            "_no terminal events of types {triggers:?} found in this bundle_",
            triggers = DEFAULT_TRIGGER_TYPES
        );
    } else {
        let _ = writeln!(
            out,
            "## Triggers ({} chain{})",
            chains.len(),
            if chains.len() == 1 { "" } else { "s" }
        );
        let _ = writeln!(out);
        for chain in chains {
            out.push_str(&render_markdown(chain));
            let _ = writeln!(out);
        }
    }
    out
}

fn compact_payload(value: &serde_json::Value) -> String {
    match serde_json::to_string(value) {
        Ok(s) => {
            if s.len() > 96 {
                format!("{}…", &s[..95])
            } else {
                s
            }
        }
        Err(_) => "<unserializable>".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_chain_bundle(test_name: &str) -> Bundle {
        let mut p = std::env::temp_dir();
        p.push(format!("cf_replay_viewer_chain_{}_{}", test_name, std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        let run_id = "chain_test";
        let manifest = serde_json::json!({
            "schema_version": "prototype-run-manifest.v0.1",
            "run_id": run_id,
            "prototype_slice": "M3B",
            "run_mode": "test",
            "milestone": "m3b",
            "build": {"commit_sha": "deadbeef", "rust_version": "x", "bevy_version": "y", "platform": "test"},
            "scene": {"id": "synth", "display_name": "synth", "source_path": "x"},
            "seed": 42,
            "started_at_utc": "2026-05-09T00:00:00Z",
            "duration_target_sec": 1.0,
            "material_schema_version": "n/a",
            "config_hash": "abc",
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
        // command -> weapon_fired -> projectile_spawned -> projectile_hit -> actor_died -> mission_resolved
        let event_lines = [
            (0, "system", "run_started", serde_json::json!({}), None),
            (
                10,
                "control",
                "command_accepted",
                serde_json::json!({"method": "act.player.fire"}),
                Some("0:0"),
            ),
            (
                10,
                "combat",
                "weapon_fired",
                serde_json::json!({"shooter": 1}),
                Some("10:1"),
            ),
            (
                10,
                "combat",
                "projectile_spawned",
                serde_json::json!({"id": 1000}),
                Some("10:2"),
            ),
            (
                15,
                "combat",
                "projectile_hit",
                serde_json::json!({"target": 2, "damage": 100}),
                Some("10:3"),
            ),
            (
                15,
                "actor",
                "actor_died",
                serde_json::json!({"actor": 2, "cause": "projectile"}),
                Some("15:4"),
            ),
            (
                16,
                "mission",
                "mission_resolved",
                serde_json::json!({"result": "won"}),
                Some("15:5"),
            ),
        ];
        let mut events_file = std::fs::File::create(p.join("events.jsonl")).unwrap();
        use std::io::Write;
        let mut all_events: Vec<serde_json::Value> = Vec::new();
        for (i, (tick, cat, ty, payload, parent_suffix)) in event_lines.iter().enumerate() {
            let event_id = format!("{run_id}:{tick}:{i}");
            let parent_event_id = parent_suffix.map(|s| format!("{run_id}:{s}"));
            let mut ev = serde_json::json!({
                "schema_version": "prototype-recorder-event.v0.1",
                "run_id": run_id,
                "tick": tick,
                "sim_time_ms": (*tick as f64) * 16.6,
                "event_id": event_id,
                "category": cat,
                "event_type": ty,
                "payload": payload,
            });
            if let Some(parent) = parent_event_id {
                ev["parent_event_id"] = serde_json::Value::String(parent);
            }
            writeln!(events_file, "{}", serde_json::to_string(&ev).unwrap()).unwrap();
            all_events.push(ev);
        }
        drop(events_file);
        let mut by_category: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
        let mut by_type: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
        for ev in all_events.iter() {
            if let Some(c) = ev.get("category").and_then(|v| v.as_str()) {
                *by_category.entry(c.to_string()).or_insert(0) += 1;
            }
            if let Some(t) = ev.get("event_type").and_then(|v| v.as_str()) {
                *by_type.entry(t.to_string()).or_insert(0) += 1;
            }
        }
        let summary = serde_json::json!({
            "schema_version": "prototype-run-summary.v0.1",
            "run_id": run_id,
            "manifest_run_id": run_id,
            "duration_sec": 1.0,
            "result": "pass",
            "ended_at_utc": "2026-05-09T00:00:01Z",
            "exit_code": 0,
            "event_counts": {"total": all_events.len(), "by_category": by_category, "by_type": by_type, "by_severity": {"error": 0, "warn": 0}, "dropped_total": 0},
            "volume": {"events_jsonl_bytes": 0, "event_lines": all_events.len()},
            "performance": {"avg_frame_ms": 0.0, "p99_frame_ms": 0.0, "avg_tick_ms": 0.0, "p99_tick_ms": 0.0, "ticks_run": 0, "wall_seconds": 0.0, "tick_rate_hz": 60},
            "artifacts": {"items": []},
            "blockers": [],
            "next_actions": [],
            "tests": [],
            "final_sim_checksum": null,
            "checksum_event_count": 0,
            "first_tick": 0,
            "last_tick": 16
        });
        std::fs::write(p.join("summary.json"), serde_json::to_vec_pretty(&summary).unwrap()).unwrap();
        Bundle::load(&p).expect("chain bundle loads")
    }

    #[test]
    fn trace_actor_died_walks_back_to_command() {
        let bundle = build_chain_bundle("walks_back_to_command");
        let trigger = bundle.first_event_of_type("actor_died").expect("actor_died exists");
        let chain = trace(&bundle, trigger, DEFAULT_MAX_DEPTH);
        assert_eq!(chain.terminated_reason, ChainTermination::RootReached);
        // actor_died -> projectile_hit -> projectile_spawned -> weapon_fired -> command_accepted -> run_started = 6 links
        assert_eq!(chain.links.len(), 6);
        let event_types: Vec<&str> = chain.links.iter().map(|l| l.event.event_type.as_str()).collect();
        assert_eq!(
            event_types,
            vec![
                "actor_died",
                "projectile_hit",
                "projectile_spawned",
                "weapon_fired",
                "command_accepted",
                "run_started",
            ]
        );
    }

    #[test]
    fn trace_mission_resolved_walks_back_through_actor_died() {
        let bundle = build_chain_bundle("mission_walks_back");
        let trigger = bundle
            .first_event_of_type("mission_resolved")
            .expect("mission_resolved exists");
        let chain = trace(&bundle, trigger, DEFAULT_MAX_DEPTH);
        assert_eq!(chain.terminated_reason, ChainTermination::RootReached);
        assert_eq!(chain.links.first().unwrap().event.event_type, "mission_resolved");
        assert!(chain.links.iter().any(|l| l.event.event_type == "actor_died"));
    }

    #[test]
    fn trace_default_triggers_includes_terminal_events_only() {
        let bundle = build_chain_bundle("default_triggers_only");
        let chains = trace_default_triggers(&bundle, DEFAULT_MAX_DEPTH);
        // Should produce exactly: projectile_hit, actor_died, mission_resolved (3 chains).
        // weapon_fired / projectile_spawned / command_accepted / run_started are NOT triggers.
        assert_eq!(chains.len(), 3);
        let trigger_types: Vec<&str> = chains.iter().map(|c| c.trigger.event_type.as_str()).collect();
        assert!(trigger_types.contains(&"projectile_hit"));
        assert!(trigger_types.contains(&"actor_died"));
        assert!(trigger_types.contains(&"mission_resolved"));
    }

    #[test]
    fn render_markdown_emits_event_id_and_chain_arrows() {
        let bundle = build_chain_bundle("render_emits_arrows");
        let trigger = bundle.first_event_of_type("actor_died").unwrap();
        let chain = trace(&bundle, trigger, DEFAULT_MAX_DEPTH);
        let md = render_markdown(&chain);
        assert!(md.contains("actor_died"));
        assert!(md.contains("projectile_hit"));
        assert!(md.contains("command_accepted"));
        assert!(md.contains("run_started"));
        assert!(md.contains("Cause chain (newest → oldest):"));
        assert!(md.contains("root reached"));
    }

    #[test]
    fn max_depth_caps_chain_and_reports_termination() {
        let bundle = build_chain_bundle("max_depth_caps");
        let trigger = bundle.first_event_of_type("mission_resolved").unwrap();
        let chain = trace(&bundle, trigger, 3);
        assert_eq!(chain.terminated_reason, ChainTermination::MaxDepthReached);
        assert_eq!(chain.links.len(), 3);
    }

    #[test]
    fn render_multi_with_no_triggers_says_so() {
        // Build a bundle with no terminal events.
        let mut p = std::env::temp_dir();
        p.push(format!("cf_replay_viewer_chain_empty_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        let run_id = "empty_chain";
        let manifest = serde_json::json!({
            "schema_version": "prototype-run-manifest.v0.1",
            "run_id": run_id,
            "prototype_slice": "M3B",
            "run_mode": "test",
            "milestone": "m3b",
            "build": {"commit_sha": "deadbeef", "rust_version": "x", "bevy_version": "y", "platform": "test"},
            "scene": {"id": "synth", "display_name": "synth", "source_path": "x"},
            "seed": 42,
            "started_at_utc": "2026-05-09T00:00:00Z",
            "duration_target_sec": 1.0,
            "material_schema_version": "n/a",
            "config_hash": "abc",
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
        let summary = serde_json::json!({
            "schema_version": "prototype-run-summary.v0.1",
            "run_id": run_id,
            "manifest_run_id": run_id,
            "duration_sec": 1.0,
            "result": "pass",
            "ended_at_utc": "2026-05-09T00:00:01Z",
            "exit_code": 0,
            "event_counts": {"total": 1, "by_category": {"system": 1}, "by_type": {"run_started": 1}, "by_severity": {"error": 0, "warn": 0}, "dropped_total": 0},
            "volume": {"events_jsonl_bytes": 0, "event_lines": 1},
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
        let only_event = serde_json::json!({
            "schema_version": "prototype-recorder-event.v0.1",
            "run_id": run_id,
            "tick": 0,
            "sim_time_ms": 0.0,
            "event_id": format!("{run_id}:0:0"),
            "category": "system",
            "event_type": "run_started",
            "payload": {}
        });
        std::fs::write(
            p.join("events.jsonl"),
            format!("{}\n", serde_json::to_string(&only_event).unwrap()),
        )
        .unwrap();
        let bundle = Bundle::load(&p).unwrap();
        let chains = trace_default_triggers(&bundle, DEFAULT_MAX_DEPTH);
        let md = render_markdown_multi(&bundle, &chains);
        assert!(md.contains("no terminal events"));
    }
}
