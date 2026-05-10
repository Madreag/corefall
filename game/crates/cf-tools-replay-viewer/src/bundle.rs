//! Run-bundle loader + corrupt-bundle rejection.
//!
//! Reads `run_manifest.json` + `events.jsonl` + `summary.json` from a bundle
//! directory, validates cross-file invariants (matching `run_id`, monotonic
//! ticks, `events.jsonl` line count == `summary.event_counts.total`,
//! `parent_event_id` references events that exist in the bundle), and exposes
//! a typed `Bundle` for downstream views.

use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use cf_replay::{
    Event, RunManifest, RunSummary, EVENT_SCHEMA_VERSION, MANIFEST_SCHEMA_VERSION, SUMMARY_SCHEMA_VERSION,
};
use thiserror::Error;

/// Sentinel prefix that flags a `parent_event_id` as referencing an event
/// from outside this bundle (e.g., a savegame restored after a crash).
/// `prototype_run_check.py` accepts this; the viewer must too. Audit-flagged
/// HIGH on 2026-05-09.
pub const EXTERNAL_PARENT_PREFIX: &str = "external:";

#[derive(Debug, Error)]
pub enum BundleError {
    #[error("bundle directory does not exist: {0}")]
    BundleDirMissing(PathBuf),
    #[error("bundle file missing: {0}")]
    FileMissing(PathBuf),
    #[error("io error reading {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("malformed JSON in {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("events.jsonl line {line} parse error in {path}: {source}")]
    EventLineJson {
        path: PathBuf,
        line: usize,
        #[source]
        source: serde_json::Error,
    },
    #[error("run_manifest.json schema_version='{got}' must equal '{want}'")]
    ManifestSchemaVersion { got: String, want: &'static str },
    #[error("summary.json schema_version='{got}' must equal '{want}'")]
    SummarySchemaVersion { got: String, want: &'static str },
    #[error("events.jsonl line {line} schema_version='{got}' must equal '{want}'")]
    EventSchemaVersion {
        line: usize,
        got: String,
        want: &'static str,
    },
    #[error("run_id mismatch: manifest='{manifest}' summary='{summary}' (must match)")]
    RunIdMismatch { manifest: String, summary: String },
    #[error("event run_id mismatch at line {line}: manifest='{manifest}' event='{event}'")]
    EventRunIdMismatch {
        line: usize,
        manifest: String,
        event: String,
    },
    #[error("summary.event_counts.total ({summary_total}) does not match events.jsonl line count ({line_count})")]
    EventCountMismatch { summary_total: u64, line_count: u64 },
    #[error("events.jsonl line {line} duplicate event_id: '{event_id}'")]
    DuplicateEventId { line: usize, event_id: String },
    #[error("events.jsonl line {line} payload must be a JSON object (got: {kind})")]
    EventPayloadNotObject { line: usize, kind: &'static str },
    #[error("events.jsonl line {line} dropped_count must be a non-negative integer (got: {got})")]
    EventDroppedCountInvalid { line: usize, got: String },
    #[error("summary.event_counts.dropped_total ({declared}) must be >= sum of per-event dropped_count ({summed})")]
    DroppedTotalUnderflow { declared: u64, summed: u64 },
    #[error("summary.event_counts.by_category map is stale: expected {expected:?}, declared {declared:?}")]
    ByCategoryStale {
        expected: BTreeMap<String, u64>,
        declared: BTreeMap<String, u64>,
    },
    #[error("summary.event_counts.by_type map is stale: expected {expected:?}, declared {declared:?}")]
    ByTypeStale {
        expected: BTreeMap<String, u64>,
        declared: BTreeMap<String, u64>,
    },
    #[error("parent_event_id '{parent}' on event '{event_id}' does not resolve to an event in this bundle (external: prefix is allowed; bare ids must be present)")]
    BrokenParentChain { event_id: String, parent: String },
    #[error("events.jsonl is not tick-monotonic: line {line} tick {tick} < previous tick {prev_tick}")]
    TickRegressed { line: usize, tick: u64, prev_tick: u64 },
}

/// Loaded run bundle.
#[derive(Debug, Clone)]
pub struct Bundle {
    pub bundle_dir: PathBuf,
    pub manifest: RunManifest,
    pub summary: RunSummary,
    pub events: Vec<Event>,
    /// `event_id -> index in events`. Used for O(log n) parent lookup.
    pub event_index: BTreeMap<String, usize>,
}

impl Bundle {
    /// Load + validate a run bundle from `bundle_dir`. Strict by default —
    /// all the cross-file invariants checked here must pass for the viewer
    /// to render anything (M3B-001 corrupt-bundle rejection).
    pub fn load(bundle_dir: &Path) -> Result<Self, BundleError> {
        if !bundle_dir.exists() {
            return Err(BundleError::BundleDirMissing(bundle_dir.to_path_buf()));
        }

        let manifest_path = bundle_dir.join("run_manifest.json");
        if !manifest_path.exists() {
            return Err(BundleError::FileMissing(manifest_path));
        }
        let manifest_text = std::fs::read_to_string(&manifest_path).map_err(|source| BundleError::Io {
            path: manifest_path.clone(),
            source,
        })?;
        let manifest_value: serde_json::Value =
            serde_json::from_str(&manifest_text).map_err(|source| BundleError::Json {
                path: manifest_path.clone(),
                source,
            })?;
        let manifest_schema = manifest_value
            .get("schema_version")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if manifest_schema != MANIFEST_SCHEMA_VERSION {
            return Err(BundleError::ManifestSchemaVersion {
                got: manifest_schema.to_string(),
                want: MANIFEST_SCHEMA_VERSION,
            });
        }
        let manifest: RunManifest = serde_json::from_value(manifest_value).map_err(|source| BundleError::Json {
            path: manifest_path.clone(),
            source,
        })?;

        let summary_path = bundle_dir.join("summary.json");
        if !summary_path.exists() {
            return Err(BundleError::FileMissing(summary_path));
        }
        let summary_text = std::fs::read_to_string(&summary_path).map_err(|source| BundleError::Io {
            path: summary_path.clone(),
            source,
        })?;
        let summary_value: serde_json::Value =
            serde_json::from_str(&summary_text).map_err(|source| BundleError::Json {
                path: summary_path.clone(),
                source,
            })?;
        let summary_schema = summary_value
            .get("schema_version")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if summary_schema != SUMMARY_SCHEMA_VERSION {
            return Err(BundleError::SummarySchemaVersion {
                got: summary_schema.to_string(),
                want: SUMMARY_SCHEMA_VERSION,
            });
        }
        let summary: RunSummary = serde_json::from_value(summary_value).map_err(|source| BundleError::Json {
            path: summary_path.clone(),
            source,
        })?;

        if manifest.run_id != summary.run_id || manifest.run_id != summary.manifest_run_id {
            return Err(BundleError::RunIdMismatch {
                manifest: manifest.run_id.clone(),
                summary: summary.run_id.clone(),
            });
        }

        let events_path = bundle_dir.join("events.jsonl");
        if !events_path.exists() {
            return Err(BundleError::FileMissing(events_path));
        }
        let file = File::open(&events_path).map_err(|source| BundleError::Io {
            path: events_path.clone(),
            source,
        })?;
        let reader = BufReader::new(file);
        let mut events: Vec<Event> = Vec::with_capacity(summary.event_counts.total as usize);
        let mut prev_tick: Option<u64> = None;
        let mut event_ids_seen: BTreeMap<String, usize> = BTreeMap::new();
        let mut by_category_actual: BTreeMap<String, u64> = BTreeMap::new();
        let mut by_type_actual: BTreeMap<String, u64> = BTreeMap::new();
        let mut dropped_sum: u64 = 0;
        for (line_idx, line) in reader.lines().enumerate() {
            let line_no = line_idx + 1;
            let line = line.map_err(|source| BundleError::Io {
                path: events_path.clone(),
                source,
            })?;
            if line.trim().is_empty() {
                continue;
            }
            // Parse the raw value first so we can enforce schema_version,
            // payload-must-be-object, and dropped_count-non-negative-integer
            // BEFORE handing off to the typed `Event` struct (which would
            // accept a non-object payload because `payload: serde_json::Value`).
            let raw: serde_json::Value = serde_json::from_str(&line).map_err(|source| BundleError::EventLineJson {
                path: events_path.clone(),
                line: line_no,
                source,
            })?;
            let evt_schema = raw.get("schema_version").and_then(|v| v.as_str()).unwrap_or("");
            if evt_schema != EVENT_SCHEMA_VERSION {
                return Err(BundleError::EventSchemaVersion {
                    line: line_no,
                    got: evt_schema.to_string(),
                    want: EVENT_SCHEMA_VERSION,
                });
            }
            // payload must be a JSON object — same rule prototype_run_check.py
            // enforces. Unlike the typed `Event` (which accepts any
            // `serde_json::Value`), we reject non-objects here. Probe-flagged
            // BLOCKER on 2026-05-09.
            let payload_kind = raw.get("payload").map(json_kind).unwrap_or("missing");
            if payload_kind != "object" {
                return Err(BundleError::EventPayloadNotObject {
                    line: line_no,
                    kind: payload_kind,
                });
            }
            // dropped_count must be absent / null / a non-negative integer.
            // Floating-point or negative values are rejected.
            if let Some(dc) = raw.get("dropped_count") {
                if !dc.is_null() {
                    let dc_int = dc.as_u64();
                    match dc_int {
                        Some(n) => dropped_sum = dropped_sum.saturating_add(n),
                        None => {
                            return Err(BundleError::EventDroppedCountInvalid {
                                line: line_no,
                                got: dc.to_string(),
                            });
                        }
                    }
                }
            }
            let event: Event = serde_json::from_value(raw).map_err(|source| BundleError::EventLineJson {
                path: events_path.clone(),
                line: line_no,
                source,
            })?;
            if event.run_id != manifest.run_id {
                return Err(BundleError::EventRunIdMismatch {
                    line: line_no,
                    manifest: manifest.run_id.clone(),
                    event: event.run_id.clone(),
                });
            }
            if let Some(prev_line) = event_ids_seen.get(&event.event_id) {
                return Err(BundleError::DuplicateEventId {
                    line: line_no,
                    event_id: format!("{} (also at line {})", event.event_id, prev_line),
                });
            }
            event_ids_seen.insert(event.event_id.clone(), line_no);
            if let Some(prev) = prev_tick {
                if event.tick < prev {
                    return Err(BundleError::TickRegressed {
                        line: line_no,
                        tick: event.tick,
                        prev_tick: prev,
                    });
                }
            }
            prev_tick = Some(event.tick);
            *by_category_actual.entry(event.category.clone()).or_insert(0) += 1;
            *by_type_actual.entry(event.event_type.clone()).or_insert(0) += 1;
            events.push(event);
        }

        if (events.len() as u64) != summary.event_counts.total {
            return Err(BundleError::EventCountMismatch {
                summary_total: summary.event_counts.total,
                line_count: events.len() as u64,
            });
        }

        // dropped_total must be at least the sum of per-event dropped_count.
        // prototype_run_check.py enforces the same invariant.
        if summary.event_counts.dropped_total < dropped_sum {
            return Err(BundleError::DroppedTotalUnderflow {
                declared: summary.event_counts.dropped_total,
                summed: dropped_sum,
            });
        }

        // by_category / by_type maps in summary.json must agree with the
        // events. Bundles can drift when a writer updates the events but
        // forgets to regenerate the count map (or hand-edits the bundle).
        // prototype_run_check.py rejects this; the viewer must too.
        if summary.event_counts.by_category != by_category_actual {
            return Err(BundleError::ByCategoryStale {
                expected: by_category_actual,
                declared: summary.event_counts.by_category.clone(),
            });
        }
        if summary.event_counts.by_type != by_type_actual {
            return Err(BundleError::ByTypeStale {
                expected: by_type_actual,
                declared: summary.event_counts.by_type.clone(),
            });
        }

        let mut event_index: BTreeMap<String, usize> = BTreeMap::new();
        for (idx, event) in events.iter().enumerate() {
            event_index.insert(event.event_id.clone(), idx);
        }
        for event in &events {
            if let Some(parent) = &event.parent_event_id {
                // `external:` prefix flags a parent emitted by a different run
                // (e.g., a savegame restored after a crash). prototype_run_check.py
                // accepts this; the viewer must too. Audit-flagged HIGH on
                // 2026-05-09.
                if parent.starts_with(EXTERNAL_PARENT_PREFIX) {
                    continue;
                }
                if !event_index.contains_key(parent) {
                    return Err(BundleError::BrokenParentChain {
                        event_id: event.event_id.clone(),
                        parent: parent.clone(),
                    });
                }
            }
        }

        Ok(Bundle {
            bundle_dir: bundle_dir.to_path_buf(),
            manifest,
            summary,
            events,
            event_index,
        })
    }

    /// Resolve an event by its id. Returns None if not present.
    pub fn event_by_id(&self, event_id: &str) -> Option<&Event> {
        self.event_index.get(event_id).map(|idx| &self.events[*idx])
    }

    /// First event in the bundle whose `event_type` matches `event_type`.
    /// Used by `cause-chain --event-type` to seed the walk.
    pub fn first_event_of_type(&self, event_type: &str) -> Option<&Event> {
        self.events.iter().find(|e| e.event_type == event_type)
    }

    /// All events of a given type (in tick / sequence order).
    pub fn events_of_type<'a>(&'a self, event_type: &'a str) -> impl Iterator<Item = &'a Event> + 'a {
        self.events.iter().filter(move |e| e.event_type == event_type)
    }

    /// All events in a given category (in tick / sequence order).
    pub fn events_in_category<'a>(&'a self, category: &'a str) -> impl Iterator<Item = &'a Event> + 'a {
        self.events.iter().filter(move |e| e.category == category)
    }
}

fn json_kind(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_minimal_manifest(dir: &Path, run_id: &str) {
        let manifest = serde_json::json!({
            "schema_version": "prototype-run-manifest.v0.1",
            "run_id": run_id,
            "prototype_slice": "M3B",
            "run_mode": "test",
            "milestone": "m3b",
            "build": {"commit_sha": "deadbeef", "rust_version": "x", "bevy_version": "y", "platform": "test"},
            "scene": {"id": "fixture", "display_name": "fixture", "source_path": "x"},
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
            dir.join("run_manifest.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
    }

    fn write_minimal_summary(dir: &Path, run_id: &str, total: u64) {
        write_summary_with_counts(dir, run_id, total, BTreeMap::new(), BTreeMap::new(), 0);
    }

    fn write_summary_with_counts(
        dir: &Path,
        run_id: &str,
        total: u64,
        by_category: BTreeMap<String, u64>,
        by_type: BTreeMap<String, u64>,
        dropped_total: u64,
    ) {
        let summary = serde_json::json!({
            "schema_version": "prototype-run-summary.v0.1",
            "run_id": run_id,
            "manifest_run_id": run_id,
            "duration_sec": 1.0,
            "result": "pass",
            "ended_at_utc": "2026-05-09T00:00:01Z",
            "exit_code": 0,
            "event_counts": {
                "total": total,
                "by_category": by_category,
                "by_type": by_type,
                "by_severity": {"error": 0, "warn": 0},
                "dropped_total": dropped_total
            },
            "volume": {"events_jsonl_bytes": 0, "event_lines": total},
            "performance": {"avg_frame_ms": 0.0, "p99_frame_ms": 0.0, "avg_tick_ms": 0.0, "p99_tick_ms": 0.0, "ticks_run": 0, "wall_seconds": 0.0, "tick_rate_hz": 60},
            "artifacts": {"items": []},
            "blockers": [],
            "next_actions": [],
            "tests": [],
            "final_sim_checksum": null,
            "checksum_event_count": 0,
            "first_tick": null,
            "last_tick": null
        });
        std::fs::write(dir.join("summary.json"), serde_json::to_vec_pretty(&summary).unwrap()).unwrap();
    }

    fn count_by(events: &[serde_json::Value], field: &str) -> BTreeMap<String, u64> {
        let mut m: BTreeMap<String, u64> = BTreeMap::new();
        for e in events {
            if let Some(s) = e.get(field).and_then(|v| v.as_str()) {
                *m.entry(s.to_string()).or_insert(0) += 1;
            }
        }
        m
    }

    fn tempdir(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("cf_replay_viewer_test_{}_{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn append_event(file: &mut std::fs::File, event: serde_json::Value) {
        let line = serde_json::to_string(&event).unwrap();
        writeln!(file, "{line}").unwrap();
    }

    #[test]
    fn load_minimal_valid_bundle() {
        let dir = tempdir("valid");
        let run_id = "test_valid";
        write_minimal_manifest(&dir, run_id);
        let event_values = vec![
            serde_json::json!({
                "schema_version": "prototype-recorder-event.v0.1",
                "run_id": run_id,
                "tick": 0,
                "sim_time_ms": 0.0,
                "event_id": format!("{run_id}:0:0"),
                "category": "system",
                "event_type": "run_started",
                "payload": {}
            }),
            serde_json::json!({
                "schema_version": "prototype-recorder-event.v0.1",
                "run_id": run_id,
                "tick": 1,
                "sim_time_ms": 16.6,
                "event_id": format!("{run_id}:1:1"),
                "category": "system",
                "event_type": "run_finished",
                "payload": {},
                "parent_event_id": format!("{run_id}:0:0")
            }),
        ];
        write_summary_with_counts(
            &dir,
            run_id,
            event_values.len() as u64,
            count_by(&event_values, "category"),
            count_by(&event_values, "event_type"),
            0,
        );
        let mut events = std::fs::File::create(dir.join("events.jsonl")).unwrap();
        for e in &event_values {
            append_event(&mut events, e.clone());
        }
        drop(events);
        let bundle = Bundle::load(&dir).expect("valid bundle must load");
        assert_eq!(bundle.events.len(), 2);
        assert_eq!(bundle.manifest.run_id, run_id);
        assert!(bundle.event_by_id(&format!("{run_id}:1:1")).is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_missing_directory() {
        let p = std::env::temp_dir().join("cf_replay_viewer_does_not_exist_xyz_zzz");
        let _ = std::fs::remove_dir_all(&p);
        let err = Bundle::load(&p).unwrap_err();
        assert!(matches!(err, BundleError::BundleDirMissing(_)));
    }

    #[test]
    fn rejects_missing_manifest() {
        let dir = tempdir("no_manifest");
        std::fs::write(dir.join("events.jsonl"), b"").unwrap();
        write_minimal_summary(&dir, "x", 0);
        let err = Bundle::load(&dir).unwrap_err();
        assert!(matches!(err, BundleError::FileMissing(_)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_run_id_mismatch() {
        let dir = tempdir("runid_mismatch");
        write_minimal_manifest(&dir, "manifest_id");
        write_minimal_summary(&dir, "summary_id", 0);
        std::fs::write(dir.join("events.jsonl"), b"").unwrap();
        let err = Bundle::load(&dir).unwrap_err();
        assert!(matches!(err, BundleError::RunIdMismatch { .. }));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_event_count_mismatch() {
        let dir = tempdir("count_mismatch");
        let run_id = "count_x";
        write_minimal_manifest(&dir, run_id);
        write_minimal_summary(&dir, run_id, 5);
        let mut events = std::fs::File::create(dir.join("events.jsonl")).unwrap();
        append_event(
            &mut events,
            serde_json::json!({
                "schema_version": "prototype-recorder-event.v0.1",
                "run_id": run_id,
                "tick": 0,
                "sim_time_ms": 0.0,
                "event_id": format!("{run_id}:0:0"),
                "category": "system",
                "event_type": "run_started",
                "payload": {}
            }),
        );
        drop(events);
        let err = Bundle::load(&dir).unwrap_err();
        assert!(matches!(err, BundleError::EventCountMismatch { .. }));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_broken_parent_chain() {
        let dir = tempdir("bad_parent");
        let run_id = "broken_parent_x";
        write_minimal_manifest(&dir, run_id);
        let event_values = vec![serde_json::json!({
            "schema_version": "prototype-recorder-event.v0.1",
            "run_id": run_id,
            "tick": 0,
            "sim_time_ms": 0.0,
            "event_id": format!("{run_id}:0:0"),
            "category": "system",
            "event_type": "run_started",
            "payload": {},
            "parent_event_id": "ghost_event_id_no_such"
        })];
        write_summary_with_counts(
            &dir,
            run_id,
            event_values.len() as u64,
            count_by(&event_values, "category"),
            count_by(&event_values, "event_type"),
            0,
        );
        let mut events = std::fs::File::create(dir.join("events.jsonl")).unwrap();
        for e in &event_values {
            append_event(&mut events, e.clone());
        }
        drop(events);
        let err = Bundle::load(&dir).unwrap_err();
        assert!(matches!(err, BundleError::BrokenParentChain { .. }));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn accepts_external_parent_prefix() {
        // Audit-flagged HIGH on 2026-05-09: prototype_run_check.py accepts
        // `external:` prefix on parent_event_id (refers to an event from a
        // different run, e.g., savegame restored after crash). The viewer
        // must accept it too.
        let dir = tempdir("external_parent");
        let run_id = "external_parent_x";
        write_minimal_manifest(&dir, run_id);
        let event_values = vec![serde_json::json!({
            "schema_version": "prototype-recorder-event.v0.1",
            "run_id": run_id,
            "tick": 0,
            "sim_time_ms": 0.0,
            "event_id": format!("{run_id}:0:0"),
            "category": "system",
            "event_type": "run_started",
            "payload": {},
            "parent_event_id": "external:prior_run:1234:5"
        })];
        write_summary_with_counts(
            &dir,
            run_id,
            event_values.len() as u64,
            count_by(&event_values, "category"),
            count_by(&event_values, "event_type"),
            0,
        );
        let mut events = std::fs::File::create(dir.join("events.jsonl")).unwrap();
        for e in &event_values {
            append_event(&mut events, e.clone());
        }
        drop(events);
        let bundle = Bundle::load(&dir).expect("external: parent prefix must be accepted");
        assert_eq!(bundle.events.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_bad_manifest_schema_version() {
        let dir = tempdir("bad_manifest_schema");
        let run_id = "bad_manifest_schema_x";
        // Write a manifest with a bogus schema_version.
        let manifest = serde_json::json!({
            "schema_version": "prototype-run-manifest.v999.999",
            "run_id": run_id,
            "prototype_slice": "X",
            "run_mode": "test",
            "milestone": "x",
            "build": {"commit_sha": "x", "rust_version": "x", "bevy_version": "x", "platform": "x"},
            "scene": {"id": "x", "display_name": "x", "source_path": "x"},
            "seed": 0,
            "started_at_utc": "2026-05-09T00:00:00Z",
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
            dir.join("run_manifest.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        write_summary_with_counts(&dir, run_id, 0, BTreeMap::new(), BTreeMap::new(), 0);
        std::fs::write(dir.join("events.jsonl"), b"").unwrap();
        let err = Bundle::load(&dir).unwrap_err();
        assert!(matches!(err, BundleError::ManifestSchemaVersion { .. }), "got {err:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_bad_summary_schema_version() {
        let dir = tempdir("bad_summary_schema");
        let run_id = "bad_summary_schema_x";
        write_minimal_manifest(&dir, run_id);
        let summary = serde_json::json!({
            "schema_version": "prototype-run-summary.v999.999",
            "run_id": run_id,
            "manifest_run_id": run_id,
            "duration_sec": 0.0,
            "result": "pass",
            "ended_at_utc": "2026-05-09T00:00:01Z",
            "exit_code": 0,
            "event_counts": {"total": 0, "by_category": {}, "by_type": {}, "by_severity": {"error": 0, "warn": 0}, "dropped_total": 0},
            "volume": {"events_jsonl_bytes": 0, "event_lines": 0},
            "performance": {"avg_frame_ms": 0.0, "p99_frame_ms": 0.0, "avg_tick_ms": 0.0, "p99_tick_ms": 0.0, "ticks_run": 0, "wall_seconds": 0.0, "tick_rate_hz": 60},
            "artifacts": {"items": []},
            "blockers": [],
            "next_actions": [],
            "tests": [],
            "final_sim_checksum": null,
            "checksum_event_count": 0,
            "first_tick": null,
            "last_tick": null
        });
        std::fs::write(dir.join("summary.json"), serde_json::to_vec_pretty(&summary).unwrap()).unwrap();
        std::fs::write(dir.join("events.jsonl"), b"").unwrap();
        let err = Bundle::load(&dir).unwrap_err();
        assert!(matches!(err, BundleError::SummarySchemaVersion { .. }), "got {err:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_bad_event_schema_version() {
        let dir = tempdir("bad_event_schema");
        let run_id = "bad_event_schema_x";
        write_minimal_manifest(&dir, run_id);
        let event_values = vec![serde_json::json!({
            "schema_version": "prototype-recorder-event.v999.999",
            "run_id": run_id,
            "tick": 0,
            "sim_time_ms": 0.0,
            "event_id": format!("{run_id}:0:0"),
            "category": "system",
            "event_type": "run_started",
            "payload": {}
        })];
        write_summary_with_counts(
            &dir,
            run_id,
            event_values.len() as u64,
            count_by(&event_values, "category"),
            count_by(&event_values, "event_type"),
            0,
        );
        let mut events = std::fs::File::create(dir.join("events.jsonl")).unwrap();
        for e in &event_values {
            append_event(&mut events, e.clone());
        }
        drop(events);
        let err = Bundle::load(&dir).unwrap_err();
        assert!(matches!(err, BundleError::EventSchemaVersion { .. }), "got {err:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_duplicate_event_id() {
        let dir = tempdir("dup_event_id");
        let run_id = "dup_x";
        write_minimal_manifest(&dir, run_id);
        let event_values = vec![
            serde_json::json!({
                "schema_version": "prototype-recorder-event.v0.1",
                "run_id": run_id,
                "tick": 0,
                "sim_time_ms": 0.0,
                "event_id": format!("{run_id}:0:0"),
                "category": "system",
                "event_type": "run_started",
                "payload": {}
            }),
            serde_json::json!({
                "schema_version": "prototype-recorder-event.v0.1",
                "run_id": run_id,
                "tick": 1,
                "sim_time_ms": 16.6,
                "event_id": format!("{run_id}:0:0"),
                "category": "system",
                "event_type": "run_finished",
                "payload": {}
            }),
        ];
        write_summary_with_counts(
            &dir,
            run_id,
            event_values.len() as u64,
            count_by(&event_values, "category"),
            count_by(&event_values, "event_type"),
            0,
        );
        let mut events = std::fs::File::create(dir.join("events.jsonl")).unwrap();
        for e in &event_values {
            append_event(&mut events, e.clone());
        }
        drop(events);
        let err = Bundle::load(&dir).unwrap_err();
        assert!(matches!(err, BundleError::DuplicateEventId { .. }), "got {err:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_non_object_payload() {
        let dir = tempdir("payload_array");
        let run_id = "payload_array_x";
        write_minimal_manifest(&dir, run_id);
        let event_values = vec![serde_json::json!({
            "schema_version": "prototype-recorder-event.v0.1",
            "run_id": run_id,
            "tick": 0,
            "sim_time_ms": 0.0,
            "event_id": format!("{run_id}:0:0"),
            "category": "system",
            "event_type": "run_started",
            "payload": ["array", "instead", "of", "object"]
        })];
        write_summary_with_counts(
            &dir,
            run_id,
            event_values.len() as u64,
            count_by(&event_values, "category"),
            count_by(&event_values, "event_type"),
            0,
        );
        let mut events = std::fs::File::create(dir.join("events.jsonl")).unwrap();
        for e in &event_values {
            append_event(&mut events, e.clone());
        }
        drop(events);
        let err = Bundle::load(&dir).unwrap_err();
        assert!(matches!(err, BundleError::EventPayloadNotObject { .. }), "got {err:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_stale_by_category_map() {
        let dir = tempdir("stale_by_category");
        let run_id = "stale_by_category_x";
        write_minimal_manifest(&dir, run_id);
        let event_values = vec![serde_json::json!({
            "schema_version": "prototype-recorder-event.v0.1",
            "run_id": run_id,
            "tick": 0,
            "sim_time_ms": 0.0,
            "event_id": format!("{run_id}:0:0"),
            "category": "system",
            "event_type": "run_started",
            "payload": {}
        })];
        // Declare a stale by_category map with category "control" instead
        // of "system" (the actual category in the event).
        let mut stale_cat = BTreeMap::new();
        stale_cat.insert("control".to_string(), 1);
        write_summary_with_counts(
            &dir,
            run_id,
            event_values.len() as u64,
            stale_cat,
            count_by(&event_values, "event_type"),
            0,
        );
        let mut events = std::fs::File::create(dir.join("events.jsonl")).unwrap();
        for e in &event_values {
            append_event(&mut events, e.clone());
        }
        drop(events);
        let err = Bundle::load(&dir).unwrap_err();
        assert!(matches!(err, BundleError::ByCategoryStale { .. }), "got {err:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_stale_by_type_map() {
        let dir = tempdir("stale_by_type");
        let run_id = "stale_by_type_x";
        write_minimal_manifest(&dir, run_id);
        let event_values = vec![serde_json::json!({
            "schema_version": "prototype-recorder-event.v0.1",
            "run_id": run_id,
            "tick": 0,
            "sim_time_ms": 0.0,
            "event_id": format!("{run_id}:0:0"),
            "category": "system",
            "event_type": "run_started",
            "payload": {}
        })];
        let mut stale_type = BTreeMap::new();
        stale_type.insert("phantom_event".to_string(), 1);
        write_summary_with_counts(
            &dir,
            run_id,
            event_values.len() as u64,
            count_by(&event_values, "category"),
            stale_type,
            0,
        );
        let mut events = std::fs::File::create(dir.join("events.jsonl")).unwrap();
        for e in &event_values {
            append_event(&mut events, e.clone());
        }
        drop(events);
        let err = Bundle::load(&dir).unwrap_err();
        assert!(matches!(err, BundleError::ByTypeStale { .. }), "got {err:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_dropped_total_underflow() {
        let dir = tempdir("dropped_underflow");
        let run_id = "dropped_underflow_x";
        write_minimal_manifest(&dir, run_id);
        let event_values = vec![serde_json::json!({
            "schema_version": "prototype-recorder-event.v0.1",
            "run_id": run_id,
            "tick": 0,
            "sim_time_ms": 0.0,
            "event_id": format!("{run_id}:0:0"),
            "category": "system",
            "event_type": "run_started",
            "payload": {},
            "dropped_count": 5
        })];
        // Declare dropped_total=1 but the per-event sum is 5 — should fail.
        write_summary_with_counts(
            &dir,
            run_id,
            event_values.len() as u64,
            count_by(&event_values, "category"),
            count_by(&event_values, "event_type"),
            1,
        );
        let mut events = std::fs::File::create(dir.join("events.jsonl")).unwrap();
        for e in &event_values {
            append_event(&mut events, e.clone());
        }
        drop(events);
        let err = Bundle::load(&dir).unwrap_err();
        assert!(matches!(err, BundleError::DroppedTotalUnderflow { .. }), "got {err:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_tick_regression() {
        let dir = tempdir("tick_regress");
        let run_id = "regress_x";
        write_minimal_manifest(&dir, run_id);
        write_minimal_summary(&dir, run_id, 2);
        let mut events = std::fs::File::create(dir.join("events.jsonl")).unwrap();
        append_event(
            &mut events,
            serde_json::json!({
                "schema_version": "prototype-recorder-event.v0.1",
                "run_id": run_id,
                "tick": 5,
                "sim_time_ms": 0.0,
                "event_id": format!("{run_id}:5:0"),
                "category": "system",
                "event_type": "run_started",
                "payload": {}
            }),
        );
        append_event(
            &mut events,
            serde_json::json!({
                "schema_version": "prototype-recorder-event.v0.1",
                "run_id": run_id,
                "tick": 2,
                "sim_time_ms": 0.0,
                "event_id": format!("{run_id}:2:1"),
                "category": "system",
                "event_type": "run_finished",
                "payload": {}
            }),
        );
        drop(events);
        let err = Bundle::load(&dir).unwrap_err();
        assert!(matches!(err, BundleError::TickRegressed { .. }));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_event_run_id_mismatch() {
        let dir = tempdir("event_runid");
        let run_id = "manifest_runid";
        write_minimal_manifest(&dir, run_id);
        write_minimal_summary(&dir, run_id, 1);
        let mut events = std::fs::File::create(dir.join("events.jsonl")).unwrap();
        append_event(
            &mut events,
            serde_json::json!({
                "schema_version": "prototype-recorder-event.v0.1",
                "run_id": "different_runid",
                "tick": 0,
                "sim_time_ms": 0.0,
                "event_id": "different_runid:0:0",
                "category": "system",
                "event_type": "run_started",
                "payload": {}
            }),
        );
        drop(events);
        let err = Bundle::load(&dir).unwrap_err();
        assert!(matches!(err, BundleError::EventRunIdMismatch { .. }));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_malformed_event_line() {
        let dir = tempdir("bad_event_json");
        let run_id = "bad_event_json_x";
        write_minimal_manifest(&dir, run_id);
        write_minimal_summary(&dir, run_id, 1);
        std::fs::write(dir.join("events.jsonl"), b"this is not json\n").unwrap();
        let err = Bundle::load(&dir).unwrap_err();
        assert!(matches!(err, BundleError::EventLineJson { .. }));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
