//! M3B-001: viewer shell — event tail + category filter + tick scrubber +
//! pause/step state, rendered as deterministic markdown.
//!
//! The viewer is conceptually a stateless renderer over a `Bundle` and a
//! `ViewerState`: given the current state (tick anchor, filter, tail
//! window), render the events visible at that anchor as markdown. The CLI
//! binary advances the state by accepting `--at-tick` / `--step` /
//! `--filter` flags; future BPs may layer a TUI / GUI on top of the same
//! library without changing the rendering contract.

use std::collections::BTreeSet;
use std::fmt::Write;

use crate::bundle::Bundle;

/// Default tail length when the caller does not specify one.
pub const DEFAULT_TAIL_LEN: usize = 32;

/// One frame of viewer state. Rendering is a pure function of (Bundle, state).
#[derive(Debug, Clone)]
pub struct ViewerState {
    /// Inclusive tick anchor. Events with `tick <= at_tick` are visible.
    pub at_tick: u64,
    /// Category filter — `None` means all categories. Comma-separated input
    /// flattens into a sorted set so the rendering is deterministic.
    pub filter: Option<BTreeSet<String>>,
    /// How many events to show in the tail at most.
    pub tail_len: usize,
    /// Last event id the caller already saw — events strictly after this id
    /// are highlighted as "new since last frame". `None` means no anchor.
    pub since_event_id: Option<String>,
    /// Pause/step is a UX concept the renderer surfaces in the header so
    /// human / agent readers can confirm the state. The actual stepping is
    /// driven by re-invoking the CLI with a higher `at_tick`.
    pub paused: bool,
    /// **M10 § View subcommand**: filter to events whose envelope
    /// `actor_id`/`source_id`/payload-`actor_id` matches the requested
    /// integer. `None` means no actor filter.
    pub actor_id_filter: Option<u64>,
    /// **M10 § View subcommand**: filter to events whose `event_type`
    /// matches the requested string. `None` means no type filter.
    pub event_type_filter: Option<String>,
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            at_tick: u64::MAX,
            filter: None,
            tail_len: DEFAULT_TAIL_LEN,
            since_event_id: None,
            paused: true,
            actor_id_filter: None,
            event_type_filter: None,
        }
    }
}

impl ViewerState {
    /// Parse a comma-separated category list into a sorted set.
    /// `""` / whitespace-only → `None` (no filter).
    pub fn parse_filter(s: &str) -> Option<BTreeSet<String>> {
        let cleaned: BTreeSet<String> = s
            .split(',')
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty())
            .collect();
        if cleaned.is_empty() {
            None
        } else {
            Some(cleaned)
        }
    }
}

/// Render the viewer state against `bundle` as markdown. Output is
/// deterministic so golden tests can compare bundles offline.
pub fn render_markdown(bundle: &Bundle, state: &ViewerState) -> String {
    let mut out = String::new();

    let _ = writeln!(out, "# Replay Viewer — `{}`", bundle.manifest.run_id);
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "- Scenario: `{}` ({})",
        bundle.manifest.scene.id, bundle.manifest.scene.display_name
    );
    let _ = writeln!(
        out,
        "- Milestone: `{}` ({})",
        bundle.manifest.milestone, bundle.manifest.prototype_slice
    );
    let _ = writeln!(
        out,
        "- Tick rate: {} Hz · Run mode: `{}` · Seed: {}",
        bundle.manifest.tick_rate_hz, bundle.manifest.run_mode, bundle.manifest.seed
    );
    let _ = writeln!(
        out,
        "- Total events: {} · First tick: {} · Last tick: {}",
        bundle.summary.event_counts.total,
        bundle
            .summary
            .first_tick
            .map(|t| t.to_string())
            .unwrap_or_else(|| "n/a".into()),
        bundle
            .summary
            .last_tick
            .map(|t| t.to_string())
            .unwrap_or_else(|| "n/a".into()),
    );
    let _ = writeln!(out);

    let _ = writeln!(out, "## State");
    let _ = writeln!(out);
    let anchor = if state.at_tick == u64::MAX {
        format!(
            "end ({})",
            bundle
                .summary
                .last_tick
                .map(|t| t.to_string())
                .unwrap_or_else(|| "n/a".into())
        )
    } else {
        state.at_tick.to_string()
    };
    let _ = writeln!(out, "- Anchor tick: `{anchor}`");
    let _ = writeln!(out, "- Paused: `{}`", state.paused);
    let filter_label = state
        .filter
        .as_ref()
        .map(|s| s.iter().cloned().collect::<Vec<_>>().join(","))
        .unwrap_or_else(|| "(all categories)".into());
    let _ = writeln!(out, "- Filter: `{filter_label}`");
    if let Some(id) = state.actor_id_filter {
        let _ = writeln!(out, "- Actor filter: `actor #{id}`");
    }
    if let Some(ty) = state.event_type_filter.as_deref() {
        let _ = writeln!(out, "- Event-type filter: `{ty}`");
    }
    let _ = writeln!(out, "- Tail length: {}", state.tail_len);
    let _ = writeln!(out);

    let visible: Vec<&cf_replay::Event> = bundle
        .events
        .iter()
        .filter(|e| e.tick <= state.at_tick)
        .filter(|e| match state.filter.as_ref() {
            Some(set) => set.contains(&e.category),
            None => true,
        })
        .filter(|e| match state.actor_id_filter {
            Some(id) => event_matches_actor(e, id),
            None => true,
        })
        .filter(|e| match state.event_type_filter.as_deref() {
            Some(ty) => e.event_type == ty,
            None => true,
        })
        .collect();

    let total_visible = visible.len();
    let tail_start = total_visible.saturating_sub(state.tail_len);
    let tail = &visible[tail_start..];

    let _ = writeln!(
        out,
        "## Tail ({tail_len_actual} of {total_visible} matching, showing tick {first}..{last})",
        tail_len_actual = tail.len(),
        first = tail.first().map(|e| e.tick.to_string()).unwrap_or_else(|| "n/a".into()),
        last = tail.last().map(|e| e.tick.to_string()).unwrap_or_else(|| "n/a".into()),
    );
    let _ = writeln!(out);

    if tail.is_empty() {
        let _ = writeln!(out, "_no events match the current filter at this anchor_");
    } else {
        // Look up `since_event_id` once: an event is "new" when its
        // (tick, seq) pair strictly exceeds the anchor's (tick, seq).
        // Lexicographic comparison on the full string event_id is incorrect
        // because tick `10:0` sorts BEFORE `9:0` lexically. Audit-flagged
        // MEDIUM on 2026-05-09.
        let since_anchor: Option<(u64, u64)> = state.since_event_id.as_ref().and_then(|s| parse_event_id_tick_seq(s));
        let _ = writeln!(out, "| tick | category | type | event_id | payload (one line) |");
        let _ = writeln!(out, "|------|----------|------|----------|--------------------|");
        for event in tail {
            let highlight = match since_anchor {
                Some((since_tick, since_seq)) => match parse_event_id_tick_seq(&event.event_id) {
                    Some((tick, seq)) => (tick, seq) > (since_tick, since_seq),
                    None => false,
                },
                None => false,
            };
            let prefix = if highlight { "**" } else { "" };
            let suffix = if highlight { "**" } else { "" };
            let payload_line = compact_payload(&event.payload);
            let _ = writeln!(
                out,
                "| {prefix}{tick}{suffix} | {prefix}{cat}{suffix} | {prefix}{ty}{suffix} | `{eid}` | `{payload}` |",
                tick = event.tick,
                cat = event.category,
                ty = event.event_type,
                eid = event.event_id,
                payload = payload_line,
            );
        }
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "## Step Controls");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "- Step forward: re-run with `--at-tick {next}` (current anchor: {anchor})",
        next = state.at_tick.saturating_add(1)
    );
    let _ = writeln!(
        out,
        "- Step backward: re-run with `--at-tick {prev}` (clamps at 0)",
        prev = state.at_tick.saturating_sub(1)
    );
    let _ = writeln!(
        out,
        "- Resume / pause: re-run with `--paused` flag toggle (renderer surfaces the value above)."
    );
    let _ = writeln!(
        out,
        "- Filter: re-run with `--filter <category[,category...]>` (current: {filter_label})"
    );

    out
}

fn compact_payload(value: &serde_json::Value) -> String {
    crate::text::compact_json_payload(value)
}

/// Match an event against an `actor_id` filter. The envelope-level
/// `actor_id` / `source_id` are checked first, then the payload's
/// well-known actor identifiers (`actor_id`, `target_actor_id`,
/// `shooter_actor_id`, etc.). Used by `view --actor` filtering.
pub(crate) fn event_matches_actor(event: &cf_replay::Event, actor_id: u64) -> bool {
    if event.actor_id == Some(actor_id) || event.source_id == Some(actor_id) {
        return true;
    }
    const KEYS: &[&str] = &[
        "actor_id",
        "actor",
        "target_actor_id",
        "shooter_actor_id",
        "shooter",
        "target",
        "source_actor_id",
        "owner_actor_id",
    ];
    for key in KEYS {
        if event.payload.get(*key).and_then(|v| v.as_u64()) == Some(actor_id) {
            return true;
        }
    }
    false
}

/// **M10 § View subcommand — Watch mode**: tail an active `events.jsonl`
/// + print new events as plain-language sentences as they appear.
///
/// `max_iterations` caps the poll loop so tests + finite runs terminate
/// deterministically. `interval_ms` defaults to 100ms when zero.
///
/// Returns the number of plain-language event lines emitted on `writer`.
pub fn watch_tail<W: std::io::Write>(
    events_path: &std::path::Path,
    writer: &mut W,
    interval_ms: u64,
    max_iterations: Option<u64>,
) -> std::io::Result<u64> {
    use std::fs::File;
    use std::io::{BufRead, BufReader, Seek, SeekFrom};
    use std::time::Duration;
    let mut printed: u64 = 0;
    let mut file = File::open(events_path)?;
    let mut offset = file.seek(SeekFrom::End(0))?;
    let mut iter = 0u64;
    let interval = if interval_ms == 0 {
        Duration::from_millis(100)
    } else {
        Duration::from_millis(interval_ms)
    };
    loop {
        if let Some(max) = max_iterations {
            if iter >= max {
                break;
            }
        }
        iter += 1;
        let len = file.metadata()?.len();
        if len > offset {
            file.seek(SeekFrom::Start(offset))?;
            let reader = BufReader::new(&file);
            for line in reader.lines() {
                let line = line?;
                offset += (line.len() + 1) as u64; // +1 for newline
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<cf_replay::Event>(&line) {
                    Ok(event) => {
                        writeln!(writer, "{}", crate::renderer::render_event_plain(&event))?;
                        printed += 1;
                    }
                    Err(_) => {
                        writeln!(writer, "[watch] skipping malformed line: {}", line)?;
                    }
                }
            }
        }
        // If max_iterations is None, the loop blocks indefinitely. Callers
        // are expected to manage Ctrl-C themselves; the default --watch
        // flow does NOT pass a max.
        if max_iterations.is_some() {
            // tests use the bounded shape; never sleep in tests
            continue;
        }
        std::thread::sleep(interval);
    }
    Ok(printed)
}

/// Parse an `event_id` string of the form `<run_id>:<tick>:<seq>` into the
/// `(tick, seq)` pair used for ordering comparisons. Audit-flagged MEDIUM
/// on 2026-05-09: lexicographic comparison on the full event_id string
/// places `tick=10` before `tick=9`, which is wrong. Comparing the parsed
/// `(tick, seq)` tuple uses numeric ordering. Returns `None` when the id
/// shape is unrecognized — caller falls back to "no highlight".
pub(crate) fn parse_event_id_tick_seq(event_id: &str) -> Option<(u64, u64)> {
    // Last two `:` separated tokens are tick + seq; everything before is run_id.
    // run_id can itself contain `:` (e.g., timestamps), so we slice from the
    // right.
    let mut parts = event_id.rsplitn(3, ':');
    let seq_str = parts.next()?;
    let tick_str = parts.next()?;
    parts.next()?; // run_id (we don't actually need to capture it)
    let tick = tick_str.parse::<u64>().ok()?;
    let seq = seq_str.parse::<u64>().ok()?;
    Some((tick, seq))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_bundle(test_name: &str) -> Bundle {
        // Build a small bundle in memory by writing to a temp dir + loading.
        let mut p = std::env::temp_dir();
        p.push(format!(
            "cf_replay_viewer_synthetic_{}_{}",
            test_name,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        let run_id = "viewer_synthetic";
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
            "event_counts": {"total": 4, "by_category": {"system": 2, "control": 1, "combat": 1}, "by_type": {"run_started": 1, "command_accepted": 1, "weapon_fired": 1, "run_finished": 1}, "by_severity": {"error": 0, "warn": 0}, "dropped_total": 0},
            "volume": {"events_jsonl_bytes": 0, "event_lines": 4},
            "performance": {"avg_frame_ms": 0.0, "p99_frame_ms": 0.0, "avg_tick_ms": 0.0, "p99_tick_ms": 0.0, "ticks_run": 0, "wall_seconds": 0.0, "tick_rate_hz": 60},
            "artifacts": {"items": []},
            "blockers": [],
            "next_actions": [],
            "tests": [],
            "final_sim_checksum": null,
            "checksum_event_count": 0,
            "first_tick": 0,
            "last_tick": 5
        });
        std::fs::write(p.join("summary.json"), serde_json::to_vec_pretty(&summary).unwrap()).unwrap();
        let events_lines = [
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 0, "sim_time_ms": 0.0, "event_id": format!("{run_id}:0:0"), "category": "system", "event_type": "run_started", "payload": {}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 1, "sim_time_ms": 16.0, "event_id": format!("{run_id}:1:1"), "category": "control", "event_type": "command_accepted", "payload": {"method": "act.player.fire"}, "parent_event_id": format!("{run_id}:0:0")}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 2, "sim_time_ms": 32.0, "event_id": format!("{run_id}:2:2"), "category": "combat", "event_type": "weapon_fired", "payload": {"shooter": 1}, "parent_event_id": format!("{run_id}:1:1")}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 5, "sim_time_ms": 80.0, "event_id": format!("{run_id}:5:3"), "category": "system", "event_type": "run_finished", "payload": {}}),
        ];
        let mut e = std::fs::File::create(p.join("events.jsonl")).unwrap();
        use std::io::Write;
        for line in events_lines {
            writeln!(e, "{}", serde_json::to_string(&line).unwrap()).unwrap();
        }
        Bundle::load(&p).expect("synthetic bundle loads")
    }

    #[test]
    fn parse_filter_empty_is_none() {
        assert!(ViewerState::parse_filter("").is_none());
        assert!(ViewerState::parse_filter("  , ,  ").is_none());
    }

    #[test]
    fn parse_filter_normalizes_and_dedups() {
        let f = ViewerState::parse_filter("control, system , control").unwrap();
        assert_eq!(f.len(), 2);
        assert!(f.contains("control"));
        assert!(f.contains("system"));
    }

    #[test]
    fn render_full_tail_at_end() {
        let bundle = synthetic_bundle("full_tail");
        let state = ViewerState::default();
        let md = render_markdown(&bundle, &state);
        assert!(md.contains("Replay Viewer"));
        assert!(md.contains("run_started"));
        assert!(md.contains("weapon_fired"));
        assert!(md.contains("run_finished"));
        assert!(md.contains("act.player.fire"));
    }

    #[test]
    fn render_at_tick_clamps_visible_events() {
        let bundle = synthetic_bundle("at_tick_clamps");
        let state = ViewerState {
            at_tick: 1,
            ..Default::default()
        };
        let md = render_markdown(&bundle, &state);
        assert!(md.contains("run_started"));
        assert!(md.contains("command_accepted"));
        assert!(!md.contains("weapon_fired"), "weapon_fired is at tick 2, anchor is 1");
        assert!(!md.contains("run_finished"));
    }

    #[test]
    fn render_with_category_filter_excludes_others() {
        let bundle = synthetic_bundle("filter_excludes");
        let state = ViewerState {
            filter: ViewerState::parse_filter("combat"),
            ..Default::default()
        };
        let md = render_markdown(&bundle, &state);
        assert!(md.contains("weapon_fired"));
        assert!(!md.contains("run_started"));
        assert!(!md.contains("run_finished"));
    }

    #[test]
    fn render_with_tail_len_limits_rows() {
        let bundle = synthetic_bundle("tail_len_limits");
        let state = ViewerState {
            tail_len: 1,
            ..Default::default()
        };
        let md = render_markdown(&bundle, &state);
        assert!(md.contains("run_finished"), "tail_len=1 must show the last event");
        assert!(!md.contains("run_started"), "tail_len=1 must hide earlier events");
    }

    #[test]
    fn parse_event_id_tick_seq_handles_timestamped_run_ids() {
        // run_id can contain colons (e.g., from timestamps); rsplit is correct.
        let id = "m2.5_2026-05-09T04:47:07Z_e66a7ad6:152:622";
        assert_eq!(parse_event_id_tick_seq(id), Some((152, 622)));
        // Simple shape.
        assert_eq!(parse_event_id_tick_seq("foo:1:0"), Some((1, 0)));
        // Non-numeric tick → None.
        assert_eq!(parse_event_id_tick_seq("foo:abc:0"), None);
        // Missing seq → None.
        assert_eq!(parse_event_id_tick_seq("foo:1"), None);
    }

    #[test]
    fn event_matches_actor_inspects_envelope_and_payload() {
        // Envelope actor_id matches.
        let mut event = cf_replay::Event {
            schema_version: cf_replay::EVENT_SCHEMA_VERSION.to_string(),
            run_id: "test".into(),
            tick: 0,
            sim_time_ms: 0.0,
            event_id: "test:0:0".into(),
            category: "system".into(),
            event_type: "run_started".into(),
            payload: serde_json::json!({}),
            parent_event_id: None,
            actor_id: Some(7),
            source_id: None,
            team: None,
            pos: None,
            bbox: None,
            dropped_count: None,
            cosmetic: None,
            asset_ref: None,
        };
        assert!(event_matches_actor(&event, 7));
        assert!(!event_matches_actor(&event, 9));
        // Falls back to payload actor_id.
        event.actor_id = None;
        event.payload = serde_json::json!({"actor_id": 11});
        assert!(event_matches_actor(&event, 11));
        // Also matches `shooter_actor_id`.
        event.payload = serde_json::json!({"shooter_actor_id": 13});
        assert!(event_matches_actor(&event, 13));
    }

    #[test]
    fn view_actor_filter_excludes_other_actors() {
        let bundle = synthetic_bundle("actor_filter");
        let state = ViewerState {
            actor_id_filter: Some(1),
            ..Default::default()
        };
        let md = render_markdown(&bundle, &state);
        // weapon_fired in the synthetic bundle has shooter=1; it must appear.
        assert!(md.contains("weapon_fired"), "rendered: {md}");
        // run_started has no actor; it must NOT appear (filter is strict).
        assert!(!md.contains("run_started"));
    }

    #[test]
    fn view_event_type_filter_narrows_to_single_event_type() {
        let bundle = synthetic_bundle("event_type_filter");
        let state = ViewerState {
            event_type_filter: Some("command_accepted".to_string()),
            ..Default::default()
        };
        let md = render_markdown(&bundle, &state);
        assert!(md.contains("command_accepted"));
        assert!(!md.contains("weapon_fired"));
        assert!(!md.contains("run_finished"));
    }

    #[test]
    fn watch_tail_emits_new_events_with_plain_language() {
        use std::io::Write;
        let mut p = std::env::temp_dir();
        p.push(format!("cf_replay_viewer_watch_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        let path = p.join("events.jsonl");
        std::fs::write(&path, "").unwrap();
        // Append two events.
        let mut f = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
        writeln!(
            f,
            "{}",
            serde_json::to_string(&serde_json::json!({
                "schema_version": "prototype-recorder-event.v0.1",
                "run_id": "watch_test",
                "tick": 0,
                "sim_time_ms": 0.0,
                "event_id": "watch_test:0:0",
                "category": "system",
                "event_type": "run_started",
                "payload": {}
            }))
            .unwrap()
        )
        .unwrap();
        writeln!(
            f,
            "{}",
            serde_json::to_string(&serde_json::json!({
                "schema_version": "prototype-recorder-event.v0.1",
                "run_id": "watch_test",
                "tick": 1,
                "sim_time_ms": 16.0,
                "event_id": "watch_test:1:1",
                "category": "actor",
                "event_type": "actor_died",
                "payload": {"actor_id": 7, "cause": "projectile"}
            }))
            .unwrap()
        )
        .unwrap();
        drop(f);
        let mut out: Vec<u8> = Vec::new();
        let printed = watch_tail(&path, &mut out, 0, Some(1)).unwrap();
        let s = String::from_utf8(out).unwrap();
        assert_eq!(printed, 2);
        assert!(s.contains("run started"));
        assert!(s.contains("died"));
        let _ = std::fs::remove_dir_all(&p);
    }

    #[test]
    fn since_event_id_uses_numeric_tick_seq_not_lexicographic() {
        // Audit-flagged MEDIUM on 2026-05-09: lexicographic comparison on
        // event_id strings places "tick=10" before "tick=9" because
        // "10..." < "9..." as strings. Compare parsed (tick, seq) pairs.
        let a = parse_event_id_tick_seq("test:9:0").unwrap();
        let b = parse_event_id_tick_seq("test:10:0").unwrap();
        assert!(b > a, "tick 10 must compare strictly greater than tick 9");
        // Same-tick: seq orders.
        let c = parse_event_id_tick_seq("test:5:0").unwrap();
        let d = parse_event_id_tick_seq("test:5:1").unwrap();
        assert!(d > c, "same-tick higher seq must compare greater");
    }
}
