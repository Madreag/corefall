//! M10 — one-line `summary <bundle>` for sweep verdicts + AI Self-Test
//! grading rows.
//!
//! Per the M10 spec § "Summary subcommand (for sweep verdicts)":
//!
//! > Given a won bundle, summary emits a single line:
//! >   `micro_breach @ <run_id>: result=won, ticks=4521, checksum=<short_hex>, events=1247, dropped=0, captures=8`
//! > Given a lost bundle, summary includes loss_reason:
//! >   `micro_breach @ <run_id>: result=lost, loss_reason=PlayerDead, ticks=3214, ...`
//!
//! The summary is reproducible: same bundle → same summary line.

use std::path::Path;

use crate::bundle::Bundle;

/// One-line summary of a bundle. Reproducible across runs.
#[derive(Debug, Clone, PartialEq)]
pub struct SweepSummary {
    pub scenario_id: String,
    pub run_id: String,
    pub result: String,
    pub loss_reason: Option<String>,
    pub ticks: u64,
    pub checksum_short_hex: Option<String>,
    pub events: u64,
    pub dropped: u64,
    pub captures: usize,
}

impl SweepSummary {
    /// Compose a SweepSummary from a loaded Bundle.
    pub fn from_bundle(bundle: &Bundle) -> Self {
        let mission = bundle.first_event_of_type("mission_resolved");
        let (result_label, loss_reason) = match mission {
            Some(e) => {
                let r = e
                    .payload
                    .get("result")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let reason = e
                    .payload
                    .get("loss_reason")
                    .or_else(|| e.payload.get("reason"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                (r, reason)
            }
            None => (bundle.summary.result.clone(), None),
        };
        let ticks = bundle.summary.last_tick.unwrap_or(0);
        let checksum_short_hex = bundle.summary.final_sim_checksum.as_ref().map(|hex| short_hex(hex));
        let captures = count_captures(&bundle.bundle_dir);
        Self {
            scenario_id: bundle.manifest.scene.id.clone(),
            run_id: bundle.manifest.run_id.clone(),
            result: result_label,
            loss_reason,
            ticks,
            checksum_short_hex,
            events: bundle.summary.event_counts.total,
            dropped: bundle.summary.event_counts.dropped_total,
            captures,
        }
    }

    /// Render the summary as the canonical one-line text. Matches the spec
    /// shape exactly so sweep matrices are grep-able.
    pub fn render_text(&self) -> String {
        let mut out = format!("{} @ {}: result={}", self.scenario_id, self.run_id, self.result);
        if let Some(reason) = &self.loss_reason {
            out.push_str(&format!(", loss_reason={reason}"));
        }
        out.push_str(&format!(", ticks={}", self.ticks));
        out.push_str(&format!(
            ", checksum={}",
            self.checksum_short_hex.as_deref().unwrap_or("-")
        ));
        out.push_str(&format!(", events={}", self.events));
        out.push_str(&format!(", dropped={}", self.dropped));
        out.push_str(&format!(", captures={}", self.captures));
        out
    }

    /// JSON variant for tooling.
    pub fn render_json(&self) -> serde_json::Value {
        serde_json::json!({
            "scenario_id": self.scenario_id,
            "run_id": self.run_id,
            "result": self.result,
            "loss_reason": self.loss_reason,
            "ticks": self.ticks,
            "checksum_short_hex": self.checksum_short_hex,
            "events": self.events,
            "dropped": self.dropped,
            "captures": self.captures,
        })
    }
}

/// Short-hex helper: first 12 chars of a blake3 hex digest, stable.
fn short_hex(hex: &str) -> String {
    hex.chars().take(12).collect()
}

/// Count PNG / capture artifacts under `<bundle_dir>/captures/`.
fn count_captures(bundle_dir: &Path) -> usize {
    let captures_dir = bundle_dir.join("captures");
    if !captures_dir.exists() {
        return 0;
    }
    match std::fs::read_dir(&captures_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                let p = e.path();
                p.extension()
                    .and_then(|x| x.to_str())
                    .map(|x| x == "png")
                    .unwrap_or(false)
            })
            .count(),
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::io::Write;

    fn write_bundle_for_summary(
        test_name: &str,
        events: Vec<serde_json::Value>,
        summary_overrides: serde_json::Value,
    ) -> Bundle {
        let mut p = std::env::temp_dir();
        p.push(format!("cf_replay_viewer_summary_{}_{}", test_name, std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        let run_id = "m10_summary_test";
        let manifest = serde_json::json!({
            "schema_version": "prototype-run-manifest.v0.1",
            "run_id": run_id,
            "prototype_slice": "M10",
            "run_mode": "test",
            "milestone": "m10",
            "build": {"commit_sha": "d", "rust_version": "x", "bevy_version": "y", "platform": "test"},
            "scene": {"id": "micro_breach", "display_name": "Micro Breach", "source_path": "x"},
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
        let mut summary = serde_json::json!({
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
        if let serde_json::Value::Object(ref mut m) = summary {
            if let serde_json::Value::Object(o) = summary_overrides {
                for (k, v) in o {
                    m.insert(k, v);
                }
            }
        }
        std::fs::write(p.join("summary.json"), serde_json::to_vec_pretty(&summary).unwrap()).unwrap();
        let mut f = std::fs::File::create(p.join("events.jsonl")).unwrap();
        for e in &events {
            writeln!(f, "{}", serde_json::to_string(e).unwrap()).unwrap();
        }
        drop(f);
        Bundle::load(&p).expect("summary test bundle loads")
    }

    #[test]
    fn summary_won_bundle_renders_canonical_line() {
        let run_id = "m10_summary_test";
        let events = vec![
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 0, "sim_time_ms": 0.0, "event_id": format!("{run_id}:0:0"), "category": "system", "event_type": "run_started", "payload": {}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 4521, "sim_time_ms": 75350.0, "event_id": format!("{run_id}:4521:1"), "category": "mission", "event_type": "mission_resolved", "payload": {"result": "won"}}),
        ];
        let bundle = write_bundle_for_summary(
            "won",
            events,
            serde_json::json!({"final_sim_checksum": "abcdef0123456789fedcba9876543210", "last_tick": 4521}),
        );
        let s = SweepSummary::from_bundle(&bundle);
        let line = s.render_text();
        assert_eq!(s.scenario_id, "micro_breach");
        assert_eq!(s.result, "won");
        assert_eq!(s.loss_reason, None);
        assert!(line.starts_with("micro_breach @ m10_summary_test: result=won"));
        assert!(line.contains("ticks=4521"));
        assert!(line.contains("checksum=abcdef012345"));
        assert!(line.contains("events=2"));
        assert!(line.contains("dropped=0"));
    }

    #[test]
    fn summary_lost_bundle_includes_loss_reason() {
        let run_id = "m10_summary_test";
        let events = vec![
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 0, "sim_time_ms": 0.0, "event_id": format!("{run_id}:0:0"), "category": "system", "event_type": "run_started", "payload": {}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 3214, "sim_time_ms": 53566.0, "event_id": format!("{run_id}:3214:1"), "category": "mission", "event_type": "mission_resolved", "payload": {"result": "lost", "loss_reason": "PlayerDead"}}),
        ];
        let bundle = write_bundle_for_summary(
            "lost",
            events,
            serde_json::json!({"final_sim_checksum": null, "last_tick": 3214}),
        );
        let s = SweepSummary::from_bundle(&bundle);
        let line = s.render_text();
        assert_eq!(s.result, "lost");
        assert_eq!(s.loss_reason.as_deref(), Some("PlayerDead"));
        assert!(line.contains("loss_reason=PlayerDead"));
        assert!(line.contains("ticks=3214"));
        assert!(line.contains("checksum=-"));
    }

    #[test]
    fn short_hex_truncates_at_twelve_chars() {
        assert_eq!(short_hex("abcdef0123456789fedcba9876543210"), "abcdef012345");
        assert_eq!(short_hex("abc"), "abc");
    }

    #[test]
    fn summary_json_round_trips_keys() {
        let run_id = "m10_summary_test";
        let events = vec![
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 0, "sim_time_ms": 0.0, "event_id": format!("{run_id}:0:0"), "category": "system", "event_type": "run_started", "payload": {}}),
        ];
        let bundle = write_bundle_for_summary("json", events, serde_json::json!({}));
        let s = SweepSummary::from_bundle(&bundle);
        let j = s.render_json();
        for k in [
            "scenario_id",
            "run_id",
            "result",
            "ticks",
            "checksum_short_hex",
            "events",
            "dropped",
            "captures",
        ] {
            assert!(j.get(k).is_some(), "missing key {k}");
        }
    }
}
