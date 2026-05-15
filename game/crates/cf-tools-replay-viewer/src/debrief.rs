//! M3B-003: debrief summary.
//!
//! Composes a concise post-run report covering:
//!
//! - Outcome (`mission_resolved.result` + reason).
//! - Objectives (`objective_started` / `objective_failed` / `objective_completed`).
//! - Key events (counts by category, top 3 event types overall).
//! - Damage / death recap (`actor_died` count + reactor-damage trajectory).
//! - Terrain changes (`terrain_carved` count + total carved pixels +
//!   chunk_dirtied count + dominant materials touched).
//! - Checksum status (`final_sim_checksum` + cadence + checksum_event_count
//!   + first/last tick).

use std::collections::BTreeMap;
use std::fmt::Write;

use crate::bundle::Bundle;

#[derive(Debug, Clone)]
pub struct Debrief<'a> {
    pub bundle: &'a Bundle,
    pub outcome: Outcome,
    pub objectives: Vec<Objective>,
    pub damage: DamageRecap,
    pub terrain: TerrainRecap,
    pub key_events: KeyEvents,
    pub checksum: ChecksumStatus,
}

#[derive(Debug, Clone, Default)]
pub struct Outcome {
    pub result: Option<String>,
    pub reason: Option<String>,
    pub resolved_at_tick: Option<u64>,
    pub resolved_event_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Objective {
    pub objective: String,
    pub started_at_tick: Option<u64>,
    pub ended_at_tick: Option<u64>,
    pub state: ObjectiveState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectiveState {
    Active,
    Completed,
    Failed,
}

impl ObjectiveState {
    fn label(&self) -> &'static str {
        match self {
            ObjectiveState::Active => "active",
            ObjectiveState::Completed => "completed",
            ObjectiveState::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DamageRecap {
    pub actor_deaths: u64,
    pub projectile_hits: u64,
    pub total_projectile_damage: f64,
    pub reactor_damage_events: u64,
    pub reactor_destroyed: bool,
    pub reactor_destroyed_at_tick: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct TerrainRecap {
    pub terrain_carved_events: u64,
    pub total_carved_pixels: u64,
    pub chunk_dirtied_events: u64,
    /// material -> carve event count (e.g., dirt: 7, concrete: 2)
    pub by_material: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Default)]
pub struct KeyEvents {
    pub by_category: BTreeMap<String, u64>,
    pub by_type: BTreeMap<String, u64>,
    pub error_count: u64,
    pub warn_count: u64,
    pub dropped_total: u64,
}

#[derive(Debug, Clone, Default)]
pub struct ChecksumStatus {
    pub algorithm: String,
    pub scope: String,
    pub cadence_ticks: u64,
    pub final_checksum: Option<String>,
    pub checksum_event_count: u64,
    pub first_tick: Option<u64>,
    pub last_tick: Option<u64>,
}

/// Compose a debrief from the bundle.
pub fn compose<'a>(bundle: &'a Bundle) -> Debrief<'a> {
    let outcome = compose_outcome(bundle);
    let objectives = compose_objectives(bundle);
    let damage = compose_damage(bundle);
    let terrain = compose_terrain(bundle);
    let key_events = compose_key_events(bundle);
    let checksum = compose_checksum(bundle);
    Debrief {
        bundle,
        outcome,
        objectives,
        damage,
        terrain,
        key_events,
        checksum,
    }
}

fn compose_outcome(bundle: &Bundle) -> Outcome {
    let resolved = bundle.first_event_of_type("mission_resolved");
    if let Some(event) = resolved {
        let result = event.payload.get("result").and_then(|v| v.as_str()).map(str::to_string);
        let reason = event.payload.get("reason").and_then(|v| v.as_str()).map(str::to_string);
        Outcome {
            result,
            reason,
            resolved_at_tick: Some(event.tick),
            resolved_event_id: Some(event.event_id.clone()),
        }
    } else {
        Outcome::default()
    }
}

fn compose_objectives(bundle: &Bundle) -> Vec<Objective> {
    let mut by_id: BTreeMap<String, Objective> = BTreeMap::new();
    for event in bundle.events.iter() {
        let key = event
            .payload
            .get("objective")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        match (event.event_type.as_str(), key) {
            ("objective_started", Some(name)) => {
                by_id
                    .entry(name.clone())
                    .or_insert(Objective {
                        objective: name,
                        started_at_tick: None,
                        ended_at_tick: None,
                        state: ObjectiveState::Active,
                    })
                    .started_at_tick
                    .get_or_insert(event.tick);
            }
            ("objective_failed", Some(name)) => {
                let entry = by_id.entry(name.clone()).or_insert(Objective {
                    objective: name,
                    started_at_tick: None,
                    ended_at_tick: None,
                    state: ObjectiveState::Active,
                });
                entry.ended_at_tick = Some(event.tick);
                entry.state = ObjectiveState::Failed;
            }
            ("objective_completed", Some(name)) => {
                let entry = by_id.entry(name.clone()).or_insert(Objective {
                    objective: name,
                    started_at_tick: None,
                    ended_at_tick: None,
                    state: ObjectiveState::Active,
                });
                entry.ended_at_tick = Some(event.tick);
                entry.state = ObjectiveState::Completed;
            }
            _ => {}
        }
    }
    by_id.into_values().collect()
}

fn compose_damage(bundle: &Bundle) -> DamageRecap {
    let mut recap = DamageRecap::default();
    for event in bundle.events.iter() {
        match event.event_type.as_str() {
            "actor_died" => recap.actor_deaths += 1,
            "projectile_hit" => {
                recap.projectile_hits += 1;
                if let Some(dmg) = event.payload.get("damage").and_then(|v| v.as_f64()) {
                    recap.total_projectile_damage += dmg;
                }
            }
            "reactor_damaged" => {
                recap.reactor_damage_events += 1;
                let destroyed = event
                    .payload
                    .get("destroyed")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if destroyed && !recap.reactor_destroyed {
                    recap.reactor_destroyed = true;
                    recap.reactor_destroyed_at_tick = Some(event.tick);
                }
            }
            "reactor_destroyed" => {
                recap.reactor_destroyed = true;
                recap.reactor_destroyed_at_tick.get_or_insert(event.tick);
            }
            _ => {}
        }
    }
    recap
}

fn compose_terrain(bundle: &Bundle) -> TerrainRecap {
    let mut recap = TerrainRecap::default();
    for event in bundle.events.iter() {
        match event.event_type.as_str() {
            "terrain_carved" => {
                recap.terrain_carved_events += 1;
                if let Some(count) = event.payload.get("count").and_then(|v| v.as_u64()) {
                    recap.total_carved_pixels += count;
                }
                if let Some(material) = event.payload.get("material").and_then(|v| v.as_str()) {
                    *recap.by_material.entry(material.to_string()).or_insert(0) += 1;
                }
            }
            "chunk_dirtied" => recap.chunk_dirtied_events += 1,
            _ => {}
        }
    }
    recap
}

fn compose_key_events(bundle: &Bundle) -> KeyEvents {
    let mut k = KeyEvents {
        by_category: bundle.summary.event_counts.by_category.clone(),
        by_type: bundle.summary.event_counts.by_type.clone(),
        error_count: bundle
            .summary
            .event_counts
            .by_severity
            .get("error")
            .copied()
            .unwrap_or(0),
        warn_count: bundle
            .summary
            .event_counts
            .by_severity
            .get("warn")
            .copied()
            .unwrap_or(0),
        dropped_total: bundle.summary.event_counts.dropped_total,
    };
    if k.by_category.is_empty() && k.by_type.is_empty() {
        for event in bundle.events.iter() {
            *k.by_category.entry(event.category.clone()).or_insert(0) += 1;
            *k.by_type.entry(event.event_type.clone()).or_insert(0) += 1;
        }
    }
    k
}

fn compose_checksum(bundle: &Bundle) -> ChecksumStatus {
    ChecksumStatus {
        algorithm: bundle.manifest.checksum.algorithm.clone(),
        scope: bundle.manifest.checksum.scope.clone(),
        cadence_ticks: bundle.manifest.checksum.cadence_ticks,
        final_checksum: bundle.summary.final_sim_checksum.clone(),
        checksum_event_count: bundle.summary.checksum_event_count,
        first_tick: bundle.summary.first_tick,
        last_tick: bundle.summary.last_tick,
    }
}

/// Render a debrief as deterministic markdown.
pub fn render_markdown(debrief: &Debrief<'_>) -> String {
    let mut out = String::new();
    let m = &debrief.bundle.manifest;
    let s = &debrief.bundle.summary;

    let _ = writeln!(out, "# Debrief — `{}`", m.run_id);
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "Scenario `{}` ({}); milestone `{}` ({}); seed {}; tick rate {} Hz; run mode `{}`.",
        m.scene.id, m.scene.display_name, m.milestone, m.prototype_slice, m.seed, m.tick_rate_hz, m.run_mode,
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "Wall duration {:.3} s; ticks run {}; total events {}; result `{}`; exit code {}.",
        s.duration_sec, s.performance.ticks_run, s.event_counts.total, s.result, s.exit_code,
    );
    let _ = writeln!(out);

    // ---- Outcome ----
    let _ = writeln!(out, "## Outcome");
    let _ = writeln!(out);
    match (&debrief.outcome.result, &debrief.outcome.reason) {
        (Some(result), Some(reason)) => {
            let _ = writeln!(out, "- Result: `{result}` (reason: `{reason}`)");
        }
        (Some(result), None) => {
            let _ = writeln!(out, "- Result: `{result}` (no explicit reason)");
        }
        (None, _) => {
            let _ = writeln!(out, "- Result: not resolved (`mission_resolved` event not emitted)");
        }
    }
    if let Some(tick) = debrief.outcome.resolved_at_tick {
        let _ = writeln!(out, "- Resolved at tick: {tick}");
    }
    if let Some(eid) = &debrief.outcome.resolved_event_id {
        let _ = writeln!(out, "- Resolved event id: `{eid}`");
    }
    let _ = writeln!(out);

    // ---- Objectives ----
    let _ = writeln!(out, "## Objectives");
    let _ = writeln!(out);
    if debrief.objectives.is_empty() {
        let _ = writeln!(out, "_no `objective_*` events emitted_");
    } else {
        let _ = writeln!(out, "| objective | state | started_tick | ended_tick |");
        let _ = writeln!(out, "|-----------|-------|--------------|------------|");
        for obj in debrief.objectives.iter() {
            let _ = writeln!(
                out,
                "| `{name}` | `{state}` | {started} | {ended} |",
                name = obj.objective,
                state = obj.state.label(),
                started = obj.started_at_tick.map(|t| t.to_string()).unwrap_or_else(|| "—".into()),
                ended = obj.ended_at_tick.map(|t| t.to_string()).unwrap_or_else(|| "—".into()),
            );
        }
    }
    let _ = writeln!(out);

    // ---- Damage / death recap ----
    let _ = writeln!(out, "## Damage & Death Recap");
    let _ = writeln!(out);
    let _ = writeln!(out, "- Actor deaths: {}", debrief.damage.actor_deaths);
    let _ = writeln!(out, "- Projectile hits: {}", debrief.damage.projectile_hits);
    let _ = writeln!(
        out,
        "- Total projectile damage delivered: {:.1}",
        debrief.damage.total_projectile_damage
    );
    let _ = writeln!(out, "- Reactor damage events: {}", debrief.damage.reactor_damage_events);
    if debrief.damage.reactor_destroyed {
        let tick = debrief
            .damage
            .reactor_destroyed_at_tick
            .map(|t| t.to_string())
            .unwrap_or_else(|| "?".into());
        let _ = writeln!(out, "- Reactor destroyed: yes (at tick {tick})");
    } else {
        let _ = writeln!(out, "- Reactor destroyed: no");
    }
    let _ = writeln!(out);

    // ---- Terrain changes ----
    let _ = writeln!(out, "## Terrain Changes");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "- `terrain_carved` events: {}",
        debrief.terrain.terrain_carved_events
    );
    let _ = writeln!(out, "- Total carved pixels: {}", debrief.terrain.total_carved_pixels);
    let _ = writeln!(
        out,
        "- `chunk_dirtied` events: {}",
        debrief.terrain.chunk_dirtied_events
    );
    if !debrief.terrain.by_material.is_empty() {
        let _ = writeln!(out, "- By material:");
        for (mat, count) in debrief.terrain.by_material.iter() {
            let _ = writeln!(out, "  - `{mat}`: {count}");
        }
    }
    let _ = writeln!(out);

    // ---- Key events ----
    let _ = writeln!(out, "## Key Events");
    let _ = writeln!(out);
    let _ = writeln!(out, "- Errors: {}", debrief.key_events.error_count);
    let _ = writeln!(out, "- Warnings: {}", debrief.key_events.warn_count);
    let _ = writeln!(out, "- Dropped events: {}", debrief.key_events.dropped_total);
    if !debrief.key_events.by_category.is_empty() {
        let _ = writeln!(out, "- By category:");
        for (cat, count) in debrief.key_events.by_category.iter() {
            let _ = writeln!(out, "  - `{cat}`: {count}");
        }
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "Top event types:");
    let _ = writeln!(out);
    let mut top: Vec<(&String, &u64)> = debrief.key_events.by_type.iter().collect();
    top.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (ty, count) in top.iter().take(8) {
        let _ = writeln!(out, "- `{ty}`: {count}");
    }
    let _ = writeln!(out);

    // ---- Checksum status ----
    let _ = writeln!(out, "## Checksum Status");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "- Algorithm: `{}` · Scope: `{}` · Cadence: every {} ticks",
        debrief.checksum.algorithm, debrief.checksum.scope, debrief.checksum.cadence_ticks,
    );
    match &debrief.checksum.final_checksum {
        Some(hex) => {
            let _ = writeln!(out, "- Final sim checksum: `{hex}`");
        }
        None => {
            let _ = writeln!(out, "- Final sim checksum: _none recorded_");
        }
    }
    let _ = writeln!(
        out,
        "- Checksum events emitted: {}",
        debrief.checksum.checksum_event_count
    );
    let _ = writeln!(
        out,
        "- First tick: {} · Last tick: {}",
        debrief
            .checksum
            .first_tick
            .map(|t| t.to_string())
            .unwrap_or_else(|| "—".into()),
        debrief
            .checksum
            .last_tick
            .map(|t| t.to_string())
            .unwrap_or_else(|| "—".into()),
    );
    let _ = writeln!(out);

    // ---- Cause chain (for losses only) — M10 § 18-section debrief ----
    let _ = writeln!(out, "## Cause Chain");
    let _ = writeln!(out);
    let lost = debrief.outcome.result.as_deref().map(|r| r == "lost").unwrap_or(false);
    if !lost {
        let _ = writeln!(
            out,
            "_N/A — mission did not end in a loss; no failure chain to explain._"
        );
    } else {
        let trigger = debrief.bundle.first_event_of_type("mission_resolved");
        match trigger {
            Some(trigger) => {
                let chain = crate::cause_chain::trace(debrief.bundle, trigger, crate::cause_chain::DEFAULT_MAX_DEPTH);
                let _ = writeln!(
                    out,
                    "Walk from `mission_resolved` back to the root cause (plain language):"
                );
                let _ = writeln!(out);
                for link in &chain.links {
                    let body = crate::renderer::render_event_body(link.event);
                    let _ = writeln!(out, "- tick {} — {}", link.event.tick, body);
                }
                let term_label = match chain.terminated_reason {
                    crate::cause_chain::ChainTermination::RootReached => "root reached",
                    crate::cause_chain::ChainTermination::ParentMissingFromBundle => {
                        "parent missing from bundle (partial bundle?)"
                    }
                    crate::cause_chain::ChainTermination::MaxDepthReached => "depth limit reached",
                    crate::cause_chain::ChainTermination::CycleDetected => "cycle detected (corrupt bundle)",
                };
                let _ = writeln!(out, "\nChain depth: {} · termination: {term_label}.", chain.links.len());
            }
            None => {
                let _ = writeln!(
                    out,
                    "_mission was lost per `result=lost` but no `mission_resolved` event was located_"
                );
            }
        }
    }
    let _ = writeln!(out);

    // ---- Accessibility surface — M10 § DR-012 audit trail ----
    let _ = writeln!(out, "## Accessibility Surface");
    let _ = writeln!(out);
    let s = &debrief.bundle.manifest.settings;
    let _ = writeln!(out, "- UI scale: `{}`", s.ui_scale);
    let _ = writeln!(out, "- High contrast: `{}`", s.high_contrast);
    let _ = writeln!(out, "- Captions: `{}`", s.captions);
    let _ = writeln!(out, "- Reduced motion: `{}`", s.reduced_motion);
    let _ = writeln!(out, "- Reduced shake: `{}`", s.reduced_shake);
    let _ = writeln!(out, "- Reduced flash: `{}`", s.reduced_flash);
    let _ = writeln!(out, "- Hold-to-confirm: `{}`", s.hold_to_confirm);
    let _ = writeln!(out, "- Hold threshold ms: `{}`", s.hold_threshold_ms);
    let _ = writeln!(out, "- Key remap enabled: `{}`", s.key_remap_enabled);
    let _ = writeln!(out, "- Key bindings: {}", s.key_bindings.len());
    let _ = writeln!(out);

    // ---- Recorder health — M10 § 18-section debrief ----
    let _ = writeln!(out, "## Recorder Health");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "- Total events: {}",
        debrief
            .key_events
            .by_category
            .values()
            .sum::<u64>()
            .max(debrief.bundle.summary.event_counts.total)
    );
    let _ = writeln!(out, "- Dropped events: {}", debrief.key_events.dropped_total);
    let _ = writeln!(out, "- Error severity events: {}", debrief.key_events.error_count);
    let _ = writeln!(out, "- Warning severity events: {}", debrief.key_events.warn_count);
    let categories_active = debrief.key_events.by_category.len();
    let _ = writeln!(out, "- Categories with events: {}", categories_active);
    if debrief.key_events.dropped_total > 0 {
        let _ = writeln!(
            out,
            "- ⚠ Recorder dropped events under backpressure (cosmetic drops are expected)."
        );
    } else {
        let _ = writeln!(out, "- Recorder under capacity (0 drops)");
    }
    let _ = writeln!(out);

    // ---- Thinking timeline (per-bot AI panel) — M10 § smart-AI surface ----
    let _ = writeln!(out, "## Thinking Timeline");
    let _ = writeln!(out);
    let died_actor = thinking_timeline_actor_id(debrief.bundle);
    match died_actor {
        Some(actor_id) => {
            let entries = crate::thinking_timeline::build_timeline(debrief.bundle, actor_id);
            // Spec: "render the killed actor's full thinking timeline for the
            // last 10 ticks before death". Use the death tick as the upper
            // bound and slice to last 10.
            let died_at_tick = debrief
                .bundle
                .events
                .iter()
                .find(|e| e.event_type == "actor_died" && event_actor_id(e) == Some(actor_id))
                .map(|e| e.tick);
            let entries = crate::thinking_timeline::slice_window(&entries, died_at_tick, Some(10));
            let panel = crate::thinking_timeline::render_markdown(actor_id, &entries);
            out.push_str(&panel);
        }
        None => {
            let _ = writeln!(out, "_no `actor_died` events in this bundle_");
        }
    }

    out
}

fn thinking_timeline_actor_id(bundle: &crate::bundle::Bundle) -> Option<u64> {
    bundle
        .events
        .iter()
        .filter(|e| e.event_type == "actor_died")
        .find_map(event_actor_id)
}

fn event_actor_id(event: &cf_replay::Event) -> Option<u64> {
    if let Some(id) = event.actor_id {
        return Some(id);
    }
    event.payload.get("actor_id").and_then(|v| v.as_u64())
}

/// Render a debrief as JSON for tooling. Used by `--json` flag.
pub fn render_json(debrief: &Debrief<'_>) -> serde_json::Value {
    let m = &debrief.bundle.manifest;
    let s = &debrief.bundle.summary;
    serde_json::json!({
        "run_id": m.run_id,
        "scenario": {
            "id": m.scene.id,
            "display_name": m.scene.display_name,
        },
        "milestone": m.milestone,
        "prototype_slice": m.prototype_slice,
        "tick_rate_hz": m.tick_rate_hz,
        "seed": m.seed,
        "duration_sec": s.duration_sec,
        "ticks_run": s.performance.ticks_run,
        "result": s.result,
        "exit_code": s.exit_code,
        "outcome": {
            "result": debrief.outcome.result,
            "reason": debrief.outcome.reason,
            "resolved_at_tick": debrief.outcome.resolved_at_tick,
            "resolved_event_id": debrief.outcome.resolved_event_id,
        },
        "objectives": debrief.objectives.iter().map(|o| {
            serde_json::json!({
                "objective": o.objective,
                "state": o.state.label(),
                "started_at_tick": o.started_at_tick,
                "ended_at_tick": o.ended_at_tick,
            })
        }).collect::<Vec<_>>(),
        "damage": {
            "actor_deaths": debrief.damage.actor_deaths,
            "projectile_hits": debrief.damage.projectile_hits,
            "total_projectile_damage": debrief.damage.total_projectile_damage,
            "reactor_damage_events": debrief.damage.reactor_damage_events,
            "reactor_destroyed": debrief.damage.reactor_destroyed,
            "reactor_destroyed_at_tick": debrief.damage.reactor_destroyed_at_tick,
        },
        "terrain": {
            "terrain_carved_events": debrief.terrain.terrain_carved_events,
            "total_carved_pixels": debrief.terrain.total_carved_pixels,
            "chunk_dirtied_events": debrief.terrain.chunk_dirtied_events,
            "by_material": debrief.terrain.by_material,
        },
        "key_events": {
            "error_count": debrief.key_events.error_count,
            "warn_count": debrief.key_events.warn_count,
            "dropped_total": debrief.key_events.dropped_total,
            "by_category": debrief.key_events.by_category,
            "by_type": debrief.key_events.by_type,
        },
        "checksum": {
            "algorithm": debrief.checksum.algorithm,
            "scope": debrief.checksum.scope,
            "cadence_ticks": debrief.checksum.cadence_ticks,
            "final_checksum": debrief.checksum.final_checksum,
            "checksum_event_count": debrief.checksum.checksum_event_count,
            "first_tick": debrief.checksum.first_tick,
            "last_tick": debrief.checksum.last_tick,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn build_bundle(run_id: &str, events: &[serde_json::Value], summary_overrides: serde_json::Value) -> Bundle {
        let mut p = std::env::temp_dir();
        p.push(format!("cf_replay_viewer_debrief_{}_{}", run_id, std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
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
        let mut by_category: BTreeMap<String, u64> = BTreeMap::new();
        let mut by_type: BTreeMap<String, u64> = BTreeMap::new();
        for ev in events.iter() {
            let cat = ev.get("category").and_then(|v| v.as_str()).unwrap().to_string();
            let ty = ev.get("event_type").and_then(|v| v.as_str()).unwrap().to_string();
            *by_category.entry(cat).or_insert(0) += 1;
            *by_type.entry(ty).or_insert(0) += 1;
        }
        let mut summary = serde_json::json!({
            "schema_version": "prototype-run-summary.v0.1",
            "run_id": run_id,
            "manifest_run_id": run_id,
            "duration_sec": 1.0,
            "result": "pass",
            "ended_at_utc": "2026-05-09T00:00:01Z",
            "exit_code": 0,
            "event_counts": {
                "total": events.len(),
                "by_category": by_category,
                "by_type": by_type,
                "by_severity": {"error": 0, "warn": 0},
                "dropped_total": 0
            },
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
        if let serde_json::Value::Object(ref mut map) = summary {
            if let serde_json::Value::Object(over) = summary_overrides {
                for (k, v) in over {
                    map.insert(k, v);
                }
            }
        }
        std::fs::write(p.join("summary.json"), serde_json::to_vec_pretty(&summary).unwrap()).unwrap();
        let mut events_file = std::fs::File::create(p.join("events.jsonl")).unwrap();
        for ev in events.iter() {
            writeln!(events_file, "{}", serde_json::to_string(ev).unwrap()).unwrap();
        }
        drop(events_file);
        Bundle::load(&p).expect("debrief test bundle loads")
    }

    #[test]
    fn debrief_extracts_outcome_for_won_mission() {
        let run_id = "debrief_won";
        let events = vec![
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 0, "sim_time_ms": 0.0, "event_id": format!("{run_id}:0:0"), "category": "system", "event_type": "run_started", "payload": {}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 100, "sim_time_ms": 1666.0, "event_id": format!("{run_id}:100:1"), "category": "mission", "event_type": "mission_resolved", "payload": {"result": "won"}}),
        ];
        let bundle = build_bundle(run_id, &events, serde_json::json!({}));
        let d = compose(&bundle);
        assert_eq!(d.outcome.result.as_deref(), Some("won"));
        assert_eq!(d.outcome.resolved_at_tick, Some(100));
        let md = render_markdown(&d);
        assert!(md.contains("Result: `won`"));
    }

    #[test]
    fn debrief_extracts_outcome_for_lost_mission_with_reason() {
        let run_id = "debrief_lost";
        let events = vec![
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 0, "sim_time_ms": 0.0, "event_id": format!("{run_id}:0:0"), "category": "system", "event_type": "run_started", "payload": {}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 50, "sim_time_ms": 833.0, "event_id": format!("{run_id}:50:1"), "category": "mission", "event_type": "objective_started", "payload": {"objective": "defend_reactor"}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 1095, "sim_time_ms": 18250.0, "event_id": format!("{run_id}:1095:2"), "category": "mission", "event_type": "objective_failed", "payload": {"objective": "defend_reactor"}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 1095, "sim_time_ms": 18250.0, "event_id": format!("{run_id}:1095:3"), "category": "mission", "event_type": "mission_resolved", "payload": {"result": "lost", "reason": "reactor_destroyed"}}),
        ];
        let bundle = build_bundle(run_id, &events, serde_json::json!({}));
        let d = compose(&bundle);
        assert_eq!(d.outcome.result.as_deref(), Some("lost"));
        assert_eq!(d.outcome.reason.as_deref(), Some("reactor_destroyed"));
        assert_eq!(d.objectives.len(), 1);
        assert_eq!(d.objectives[0].state, ObjectiveState::Failed);
        let md = render_markdown(&d);
        assert!(md.contains("Result: `lost`"));
        assert!(md.contains("reason: `reactor_destroyed`"));
        assert!(md.contains("`defend_reactor`"));
        assert!(md.contains("failed"));
    }

    #[test]
    fn debrief_aggregates_damage_terrain_and_checksum() {
        let run_id = "debrief_full";
        let events = vec![
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 0, "sim_time_ms": 0.0, "event_id": format!("{run_id}:0:0"), "category": "system", "event_type": "run_started", "payload": {}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 10, "sim_time_ms": 166.0, "event_id": format!("{run_id}:10:1"), "category": "terrain", "event_type": "terrain_carved", "payload": {"material": "dirt", "count": 100}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 20, "sim_time_ms": 333.0, "event_id": format!("{run_id}:20:2"), "category": "terrain", "event_type": "chunk_dirtied", "payload": {}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 30, "sim_time_ms": 500.0, "event_id": format!("{run_id}:30:3"), "category": "combat", "event_type": "projectile_hit", "payload": {"damage": 8.0, "target": 1}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 40, "sim_time_ms": 666.0, "event_id": format!("{run_id}:40:4"), "category": "combat", "event_type": "reactor_damaged", "payload": {"hp_after": 0, "destroyed": true}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 41, "sim_time_ms": 683.0, "event_id": format!("{run_id}:41:5"), "category": "actor", "event_type": "actor_died", "payload": {"actor": 1, "cause": "projectile"}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 50, "sim_time_ms": 833.0, "event_id": format!("{run_id}:50:6"), "category": "mission", "event_type": "mission_resolved", "payload": {"result": "lost", "reason": "reactor_destroyed"}}),
        ];
        let summary_overrides = serde_json::json!({
            "final_sim_checksum": "abcd1234deadbeef",
            "checksum_event_count": 5,
            "first_tick": 0,
            "last_tick": 50
        });
        let bundle = build_bundle(run_id, &events, summary_overrides);
        let d = compose(&bundle);
        assert_eq!(d.damage.actor_deaths, 1);
        assert_eq!(d.damage.projectile_hits, 1);
        assert_eq!(d.damage.total_projectile_damage as i64, 8);
        assert!(d.damage.reactor_destroyed);
        assert_eq!(d.damage.reactor_destroyed_at_tick, Some(40));
        assert_eq!(d.terrain.terrain_carved_events, 1);
        assert_eq!(d.terrain.total_carved_pixels, 100);
        assert_eq!(d.terrain.by_material.get("dirt"), Some(&1));
        assert_eq!(d.checksum.final_checksum.as_deref(), Some("abcd1234deadbeef"));
        let md = render_markdown(&d);
        for h in [
            "## Outcome",
            "## Objectives",
            "## Damage & Death Recap",
            "## Terrain Changes",
            "## Key Events",
            "## Checksum Status",
            "## Cause Chain",
            "## Accessibility Surface",
            "## Recorder Health",
            "## Thinking Timeline",
        ] {
            assert!(md.contains(h), "debrief markdown missing heading {h}");
        }
        assert!(md.contains("abcd1234deadbeef"));
        assert!(md.contains("Reactor destroyed: yes"));
    }

    #[test]
    fn debrief_renders_unresolved_mission_gracefully() {
        let run_id = "debrief_unresolved";
        let events = vec![
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 0, "sim_time_ms": 0.0, "event_id": format!("{run_id}:0:0"), "category": "system", "event_type": "run_started", "payload": {}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 1, "sim_time_ms": 16.0, "event_id": format!("{run_id}:1:1"), "category": "system", "event_type": "run_finished", "payload": {}}),
        ];
        let bundle = build_bundle(run_id, &events, serde_json::json!({}));
        let d = compose(&bundle);
        assert!(d.outcome.result.is_none());
        let md = render_markdown(&d);
        assert!(md.contains("not resolved"));
    }

    #[test]
    fn debrief_json_round_trip_has_expected_keys() {
        let run_id = "debrief_json";
        let events = vec![
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 0, "sim_time_ms": 0.0, "event_id": format!("{run_id}:0:0"), "category": "system", "event_type": "run_started", "payload": {}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 100, "sim_time_ms": 1666.0, "event_id": format!("{run_id}:100:1"), "category": "mission", "event_type": "mission_resolved", "payload": {"result": "won"}}),
        ];
        let bundle = build_bundle(run_id, &events, serde_json::json!({}));
        let d = compose(&bundle);
        let json = render_json(&d);
        for k in [
            "run_id",
            "scenario",
            "outcome",
            "objectives",
            "damage",
            "terrain",
            "key_events",
            "checksum",
        ] {
            assert!(json.get(k).is_some(), "json missing key {k}");
        }
    }
}
