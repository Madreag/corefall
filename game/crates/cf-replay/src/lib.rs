//! M0-004: run-bundle writer + DR-002 v1 event envelope.
//!
//! Schema strings come from the canonical run-bundle checker
//! (`game/tools/prototype_run_check.py`, vendored from the planning vault):
//!   - manifest: `prototype-run-manifest.v0.1`
//!   - event: `prototype-recorder-event.v0.1`
//!   - summary: `prototype-run-summary.v0.1`
//!
//! Event envelope is the DR-002 v1 lock (approved 2026-05-05):
//!   {schema_version, run_id, tick, sim_time_ms, event_id, category, event_type, payload, parent_event_id?, dropped_count?}
//! M0 categories: `system`, `control`, `determinism`. Snapshot/checksum payloads ride on
//! the `determinism` category for M0; the `snapshot` category opens at M3.

use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use cf_sim_core::{
    checksum::{CHECKSUM_ALGORITHM, CHECKSUM_SCOPE},
    ids::make_event_id,
    Tick,
};

pub const MANIFEST_SCHEMA_VERSION: &str = "prototype-run-manifest.v0.1";
pub const EVENT_SCHEMA_VERSION: &str = "prototype-recorder-event.v0.1";
pub const SUMMARY_SCHEMA_VERSION: &str = "prototype-run-summary.v0.1";

pub const CONTROL_SCHEMA_VERSION: u32 = 1;
pub const SCENARIO_SCHEMA_VERSION: u32 = 1;
pub const EVENT_ENVELOPE_VERSION: u32 = 1;

pub mod bundle_paths;
pub mod diagnostics;

pub use bundle_paths::{default_run_bundle_root, resolve_run_bundle_root};

/// One DR-002 v1 event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub schema_version: String,
    pub run_id: String,
    pub tick: u64,
    pub sim_time_ms: f64,
    pub event_id: String,
    pub category: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub parent_event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub dropped_count: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildInfo {
    pub commit_sha: String,
    pub rust_version: String,
    pub bevy_version: String,
    pub platform: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneInfo {
    pub id: String,
    pub display_name: String,
    pub source_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureConfig {
    pub events: bool,
    pub screenshots: bool,
    pub captures: bool,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            events: true,
            screenshots: false,
            captures: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecksumConfig {
    pub algorithm: String,
    pub scope: String,
    pub cadence_ticks: u64,
}

impl ChecksumConfig {
    pub fn m0_default() -> Self {
        Self {
            algorithm: CHECKSUM_ALGORITHM.to_string(),
            scope: CHECKSUM_SCOPE.to_string(),
            cadence_ticks: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsBlock {
    pub ui_scale: f32,
    pub high_contrast: bool,
    pub captions: bool,
    pub reduced_motion: bool,
    pub reduced_shake: bool,
    pub reduced_flash: bool,
}

impl Default for SettingsBlock {
    fn default() -> Self {
        Self {
            ui_scale: 1.0,
            high_contrast: false,
            captions: true,
            reduced_motion: false,
            reduced_shake: false,
            reduced_flash: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CapabilitiesBlock {
    pub debug: bool,
    pub control_api: bool,
    pub save_load: bool,
    pub debug_capabilities: Vec<String>,
}

/// M3A-005: `run_manifest.json.expected_outcome` constrained enum. The canonical
/// run-bundle checker (`game/tools/prototype_run_check.py`) enforces this
/// alongside the `system.run_finished` / `system.panic` event checks:
///
/// - `Clean` — bundle MUST contain exactly one `system.run_finished` event, no
///   `system.panic` event, and `event_counts.by_severity.error` must be zero.
/// - `Panic` — bundle MUST contain at least one `system.panic` event.
/// - `Abort` — bundle MAY contain `system.run_finished` but `by_severity.error`
///   is allowed to be non-zero (e.g., a cfctl-driven shutdown that ran into a
///   soft failure).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedOutcome {
    #[default]
    Clean,
    Panic,
    Abort,
}

impl ExpectedOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            ExpectedOutcome::Clean => "clean",
            ExpectedOutcome::Panic => "panic",
            ExpectedOutcome::Abort => "abort",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunManifest {
    pub schema_version: String,
    pub run_id: String,
    pub prototype_slice: String,
    pub run_mode: String,
    pub milestone: String,
    pub build: BuildInfo,
    pub scene: SceneInfo,
    pub seed: u64,
    pub started_at_utc: String,
    pub duration_target_sec: f64,
    pub material_schema_version: String,
    pub config_hash: String,
    pub assumptions_tested: Vec<String>,
    pub linked_specs: Vec<String>,
    pub expected_tests: Vec<String>,
    pub capture_config: CaptureConfig,
    pub schemas: BTreeMap<String, u32>,
    pub capabilities: CapabilitiesBlock,
    pub settings: SettingsBlock,
    pub checksum: ChecksumConfig,
    pub tick_rate_hz: u32,
    /// M3A-005: declared lifecycle outcome (clean | panic | abort).
    /// Defaults to `clean`; the canonical run-bundle checker enforces the
    /// per-outcome event-count rules above.
    #[serde(default)]
    pub expected_outcome: ExpectedOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRecord {
    pub id: String,
    pub result: String,
    pub evidence_event_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventCounts {
    pub total: u64,
    pub by_category: BTreeMap<String, u64>,
    pub by_type: BTreeMap<String, u64>,
    pub by_severity: BTreeMap<String, u64>,
    pub dropped_total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VolumeBlock {
    pub events_jsonl_bytes: u64,
    pub event_lines: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerformanceBlock {
    pub avg_frame_ms: f64,
    pub p99_frame_ms: f64,
    pub avg_tick_ms: f64,
    pub p99_tick_ms: f64,
    pub ticks_run: u64,
    pub wall_seconds: f64,
    pub tick_rate_hz: u32,
}

/// Perf sample plumbed in from the engine when writing a bundle.
#[derive(Debug, Clone, Default)]
pub struct PerfSample {
    pub avg_frame_ms: f64,
    pub p99_frame_ms: f64,
    pub avg_tick_ms: f64,
    pub p99_tick_ms: f64,
    pub ticks_run: u64,
    pub wall_seconds: f64,
    pub tick_rate_hz: u32,
}

impl From<PerfSample> for PerformanceBlock {
    fn from(s: PerfSample) -> Self {
        Self {
            avg_frame_ms: s.avg_frame_ms,
            p99_frame_ms: s.p99_frame_ms,
            avg_tick_ms: s.avg_tick_ms,
            p99_tick_ms: s.p99_tick_ms,
            ticks_run: s.ticks_run,
            wall_seconds: s.wall_seconds,
            tick_rate_hz: s.tick_rate_hz,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactsBlock {
    pub items: Vec<ArtifactItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactItem {
    pub kind: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    pub schema_version: String,
    pub run_id: String,
    pub manifest_run_id: String,
    pub duration_sec: f64,
    pub result: String,
    pub ended_at_utc: String,
    pub exit_code: i32,
    pub event_counts: EventCounts,
    pub volume: VolumeBlock,
    pub performance: PerformanceBlock,
    pub artifacts: ArtifactsBlock,
    pub blockers: Vec<String>,
    pub next_actions: Vec<String>,
    pub tests: Vec<TestRecord>,
    pub final_sim_checksum: Option<String>,
    pub checksum_event_count: u64,
    pub first_tick: Option<u64>,
    pub last_tick: Option<u64>,
}

/// Append-friendly recorder. Events go through here so the writer can apply backpressure
/// and surface dropped counts in `summary.json.event_counts.dropped_total`.
pub struct Recorder {
    run_id: String,
    seq: AtomicU64,
    inner: Mutex<RecorderInner>,
}

struct RecorderInner {
    events: Vec<Event>,
    by_category: BTreeMap<String, u64>,
    by_type: BTreeMap<String, u64>,
    by_severity: BTreeMap<String, u64>,
    dropped: u64,
    first_tick: Option<u64>,
    last_tick: Option<u64>,
    final_checksum: Option<String>,
    checksum_event_count: u64,
}

impl Recorder {
    pub fn new(run_id: String) -> Self {
        Self {
            run_id,
            seq: AtomicU64::new(0),
            inner: Mutex::new(RecorderInner {
                events: Vec::new(),
                by_category: BTreeMap::new(),
                by_type: BTreeMap::new(),
                by_severity: BTreeMap::new(),
                dropped: 0,
                first_tick: None,
                last_tick: None,
                final_checksum: None,
                checksum_event_count: 0,
            }),
        }
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn record(
        &self,
        tick: Tick,
        sim_time_ms: f64,
        category: &str,
        event_type: &str,
        payload: serde_json::Value,
        parent_event_id: Option<String>,
    ) -> String {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let event_id = make_event_id(&self.run_id, tick.0, seq);
        let event = Event {
            schema_version: EVENT_SCHEMA_VERSION.to_string(),
            run_id: self.run_id.clone(),
            tick: tick.0,
            sim_time_ms,
            event_id: event_id.clone(),
            category: category.to_string(),
            event_type: event_type.to_string(),
            payload,
            parent_event_id,
            dropped_count: None,
        };
        if let Ok(mut inner) = self.inner.lock() {
            *inner.by_category.entry(event.category.clone()).or_insert(0) += 1;
            *inner.by_type.entry(event.event_type.clone()).or_insert(0) += 1;
            inner.first_tick.get_or_insert(tick.0);
            inner.last_tick = Some(tick.0);
            if event.category == "determinism" && event.event_type == "sim_checksum" {
                inner.checksum_event_count += 1;
                if let Some(hex) = event.payload.get("checksum_hex").and_then(|v| v.as_str()) {
                    inner.final_checksum = Some(hex.to_string());
                }
            }
            inner.events.push(event);
        } else {
            // Lock was poisoned; the panic hook will already have logged.
            // Increment a dropped counter on a freshly poisoned guard via Mutex::clear_poison
            // is unstable; report via tracing instead.
            tracing::error!(target: "cf::replay", "recorder mutex poisoned; event dropped");
        }
        event_id
    }

    pub fn record_severity(&self, severity: &str) {
        if let Ok(mut inner) = self.inner.lock() {
            *inner.by_severity.entry(severity.to_string()).or_insert(0) += 1;
        }
    }

    pub fn dropped(&self, count: u64) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.dropped += count;
        }
    }

    pub fn snapshot_events(&self) -> Vec<Event> {
        self.inner.lock().map(|inner| inner.events.clone()).unwrap_or_default()
    }

    pub fn events_since(&self, after_idx: usize) -> Vec<Event> {
        self.inner
            .lock()
            .map(|inner| {
                if after_idx >= inner.events.len() {
                    Vec::new()
                } else {
                    inner.events[after_idx..].to_vec()
                }
            })
            .unwrap_or_default()
    }

    pub fn counts(&self) -> EventCounts {
        let inner = self
            .inner
            .lock()
            .expect("recorder mutex poisoned; inspect prior tracing::error events");
        let mut by_severity = inner.by_severity.clone();
        by_severity.entry("error".to_string()).or_insert(0);
        by_severity.entry("warn".to_string()).or_insert(0);
        EventCounts {
            total: inner.events.len() as u64,
            by_category: inner.by_category.clone(),
            by_type: inner.by_type.clone(),
            by_severity,
            dropped_total: inner.dropped,
        }
    }

    pub fn first_last_tick(&self) -> (Option<u64>, Option<u64>) {
        let inner = self.inner.lock().expect("recorder mutex poisoned");
        (inner.first_tick, inner.last_tick)
    }

    pub fn final_checksum_hex(&self) -> Option<String> {
        self.inner.lock().ok().and_then(|i| i.final_checksum.clone())
    }

    pub fn checksum_event_count(&self) -> u64 {
        self.inner.lock().map(|i| i.checksum_event_count).unwrap_or(0)
    }
}

/// Final state passed into `write_run_bundle`.
pub struct BundleInputs<'a> {
    pub recorder: &'a Recorder,
    pub manifest: RunManifest,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub exit_code: i32,
    pub result: String,
    pub blockers: Vec<String>,
    pub next_actions: Vec<String>,
    pub tests: Vec<TestRecord>,
    pub artifacts: Vec<ArtifactItem>,
    pub assumptions_tested: Vec<String>,
    pub good: Vec<String>,
    pub bad: Vec<String>,
    pub meh: Vec<String>,
    pub evidence_links: Vec<String>,
    pub notes_extra: String,
    pub perf: Option<PerfSample>,
}

#[derive(Debug, thiserror::Error)]
pub enum BundleError {
    #[error("io error writing {path:?}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("serde error writing {path:?}: {source}")]
    Serde {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

pub fn run_bundle_dir_basename(run_id: &str) -> String {
    run_id.to_string()
}

/// Write `events.jsonl`, `run_manifest.json`, `summary.json`, `notes.md` into `bundle_dir`.
pub fn write_run_bundle(bundle_dir: &Path, inputs: BundleInputs<'_>) -> Result<RunSummary, BundleError> {
    fs::create_dir_all(bundle_dir).map_err(|source| BundleError::Io {
        path: bundle_dir.to_path_buf(),
        source,
    })?;

    let events_path = bundle_dir.join("events.jsonl");
    let mut events_writer = BufWriter::new(File::create(&events_path).map_err(|source| BundleError::Io {
        path: events_path.clone(),
        source,
    })?);
    let events = inputs.recorder.snapshot_events();
    let mut bytes_written: u64 = 0;
    for event in &events {
        let line = serde_json::to_string(event).map_err(|source| BundleError::Serde {
            path: events_path.clone(),
            source,
        })?;
        events_writer
            .write_all(line.as_bytes())
            .and_then(|_| events_writer.write_all(b"\n"))
            .map_err(|source| BundleError::Io {
                path: events_path.clone(),
                source,
            })?;
        bytes_written += line.len() as u64 + 1;
    }
    events_writer.flush().map_err(|source| BundleError::Io {
        path: events_path.clone(),
        source,
    })?;

    let manifest_path = bundle_dir.join("run_manifest.json");
    write_pretty_json(&manifest_path, &inputs.manifest)?;

    let counts = inputs.recorder.counts();
    let (first_tick, last_tick) = inputs.recorder.first_last_tick();
    let final_checksum = inputs.recorder.final_checksum_hex();
    let checksum_event_count = inputs.recorder.checksum_event_count();
    let duration_sec = (inputs.ended_at - inputs.started_at).num_milliseconds() as f64 / 1000.0;

    let notes_path = bundle_dir.join("notes.md");
    let notes = render_notes(
        &inputs.manifest,
        &inputs.assumptions_tested,
        &inputs.good,
        &inputs.bad,
        &inputs.meh,
        &inputs.evidence_links,
        &inputs.next_actions,
        &inputs.notes_extra,
    );
    fs::write(&notes_path, notes).map_err(|source| BundleError::Io {
        path: notes_path,
        source,
    })?;

    let performance = match inputs.perf {
        Some(perf) => PerformanceBlock::from(perf),
        None => PerformanceBlock {
            avg_frame_ms: 0.0,
            p99_frame_ms: 0.0,
            avg_tick_ms: 0.0,
            p99_tick_ms: 0.0,
            ticks_run: last_tick.unwrap_or(0),
            wall_seconds: duration_sec,
            tick_rate_hz: inputs.manifest.tick_rate_hz,
        },
    };
    let summary = RunSummary {
        schema_version: SUMMARY_SCHEMA_VERSION.to_string(),
        run_id: inputs.manifest.run_id.clone(),
        manifest_run_id: inputs.manifest.run_id.clone(),
        duration_sec,
        result: inputs.result,
        ended_at_utc: inputs.ended_at.to_rfc3339(),
        exit_code: inputs.exit_code,
        event_counts: counts,
        volume: VolumeBlock {
            events_jsonl_bytes: bytes_written,
            event_lines: events.len() as u64,
        },
        performance,
        artifacts: ArtifactsBlock {
            items: inputs.artifacts,
        },
        blockers: inputs.blockers,
        next_actions: inputs.next_actions,
        tests: inputs.tests,
        final_sim_checksum: final_checksum,
        checksum_event_count,
        first_tick,
        last_tick,
    };

    let summary_path = bundle_dir.join("summary.json");
    write_pretty_json(&summary_path, &summary)?;

    Ok(summary)
}

fn write_pretty_json<T: Serialize>(path: &Path, value: &T) -> Result<(), BundleError> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|source| BundleError::Serde {
        path: path.to_path_buf(),
        source,
    })?;
    fs::write(path, bytes).map_err(|source| BundleError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn render_notes(
    manifest: &RunManifest,
    assumptions: &[String],
    good: &[String],
    bad: &[String],
    meh: &[String],
    evidence_links: &[String],
    next_actions: &[String],
    extra: &str,
) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "# {} Run Notes — {}\n\n",
        manifest.milestone.to_uppercase(),
        manifest.run_id
    ));
    s.push_str("## Assumptions Tested\n");
    for a in assumptions {
        s.push_str(&format!("- {a}\n"));
    }
    if assumptions.is_empty() {
        s.push_str(&format!("- {}\n", "(none recorded)"));
    }
    s.push('\n');

    s.push_str("## Good\n");
    for g in good {
        s.push_str(&format!("- {g}\n"));
    }
    if good.is_empty() {
        s.push_str("- (none recorded)\n");
    }
    s.push('\n');

    s.push_str("## Bad\n");
    for b in bad {
        s.push_str(&format!("- {b}\n"));
    }
    if bad.is_empty() {
        s.push_str("- (none recorded)\n");
    }
    s.push('\n');

    s.push_str("## Meh\n");
    for m in meh {
        s.push_str(&format!("- {m}\n"));
    }
    if meh.is_empty() {
        s.push_str("- (none recorded)\n");
    }
    s.push('\n');

    s.push_str("## Evidence Links\n");
    for link in evidence_links {
        s.push_str(&format!("- {link}\n"));
    }
    if evidence_links.is_empty() {
        s.push_str("- events.jsonl\n- summary.json\n- run_manifest.json\n");
    }
    s.push('\n');

    s.push_str("## Next Actions\n");
    for n in next_actions {
        s.push_str(&format!("- {n}\n"));
    }
    if next_actions.is_empty() {
        s.push_str("- (none recorded)\n");
    }
    if !extra.is_empty() {
        s.push('\n');
        s.push_str(extra);
        if !extra.ends_with('\n') {
            s.push('\n');
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_for_test(run_id: &str) -> RunManifest {
        RunManifest {
            schema_version: MANIFEST_SCHEMA_VERSION.to_string(),
            run_id: run_id.to_string(),
            prototype_slice: "M0".to_string(),
            run_mode: "test".to_string(),
            milestone: "m0".to_string(),
            build: BuildInfo::default(),
            scene: SceneInfo {
                id: "m0_blank".to_string(),
                display_name: "M0 Blank Scene".to_string(),
                source_path: "content/scenarios/m0_blank.ron".to_string(),
            },
            seed: 42,
            started_at_utc: "2026-05-05T00:00:00Z".to_string(),
            duration_target_sec: 5.0,
            material_schema_version: "n/a-m0".to_string(),
            config_hash: "deadbeef".to_string(),
            assumptions_tested: vec!["sim ticks".to_string()],
            linked_specs: vec!["spec/prototype-roadmap".to_string()],
            expected_tests: vec!["M0-SMOKE-01".to_string()],
            capture_config: CaptureConfig::default(),
            schemas: {
                let mut m = BTreeMap::new();
                m.insert("control".to_string(), CONTROL_SCHEMA_VERSION);
                m.insert("scenario".to_string(), SCENARIO_SCHEMA_VERSION);
                m.insert("events".to_string(), EVENT_ENVELOPE_VERSION);
                m
            },
            capabilities: CapabilitiesBlock::default(),
            settings: SettingsBlock::default(),
            checksum: ChecksumConfig::m0_default(),
            tick_rate_hz: 60,
            expected_outcome: ExpectedOutcome::Clean,
        }
    }

    #[test]
    fn roundtrip_event_envelope() {
        let recorder = Recorder::new("m0_test_aabbccdd".to_string());
        let id = recorder.record(
            Tick(7),
            116.6,
            "system",
            "run_started",
            serde_json::json!({"reason": "test"}),
            None,
        );
        assert_eq!(id, "m0_test_aabbccdd:7:0");
        let events = recorder.snapshot_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].schema_version, EVENT_SCHEMA_VERSION);
        let line = serde_json::to_string(&events[0]).unwrap();
        let parsed: Event = serde_json::from_str(&line).unwrap();
        assert_eq!(parsed.event_id, id);
        assert_eq!(parsed.category, "system");
    }

    #[test]
    fn write_bundle_and_validate_basics() {
        let tmp = tempdir_for_test();
        let recorder = Recorder::new("m0_test_aabbccdd".to_string());
        recorder.record(Tick(0), 0.0, "system", "run_started", serde_json::json!({}), None);
        recorder.record(
            Tick(60),
            1000.0,
            "determinism",
            "sim_checksum",
            serde_json::json!({"checksum_hex": "00"}),
            None,
        );
        recorder.record(Tick(60), 1000.0, "system", "run_finished", serde_json::json!({}), None);
        let manifest = manifest_for_test("m0_test_aabbccdd");
        let inputs = BundleInputs {
            recorder: &recorder,
            manifest: manifest.clone(),
            started_at: DateTime::parse_from_rfc3339("2026-05-05T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            ended_at: DateTime::parse_from_rfc3339("2026-05-05T00:00:05Z")
                .unwrap()
                .with_timezone(&Utc),
            exit_code: 0,
            result: "pass".to_string(),
            blockers: vec![],
            next_actions: vec!["Proceed to M1.".to_string()],
            tests: vec![TestRecord {
                id: "M0-SMOKE-01".to_string(),
                result: "pass".to_string(),
                evidence_event_ids: vec!["m0_test_aabbccdd:0:0".to_string(), "m0_test_aabbccdd:60:2".to_string()],
                notes: None,
            }],
            artifacts: vec![],
            assumptions_tested: manifest.assumptions_tested.clone(),
            good: vec!["sim stable".to_string()],
            bad: vec![],
            meh: vec![],
            evidence_links: vec!["events.jsonl".to_string()],
            notes_extra: String::new(),
            perf: None,
        };
        let summary = write_run_bundle(&tmp, inputs).unwrap();
        assert_eq!(summary.event_counts.total, 3);
        assert_eq!(summary.first_tick, Some(0));
        assert_eq!(summary.last_tick, Some(60));
        assert_eq!(summary.checksum_event_count, 1);
        let notes = std::fs::read_to_string(tmp.join("notes.md")).unwrap();
        for h in [
            "## Assumptions Tested",
            "## Good",
            "## Bad",
            "## Meh",
            "## Evidence Links",
            "## Next Actions",
        ] {
            assert!(notes.contains(h), "notes.md missing heading {h}");
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    fn tempdir_for_test() -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("cf_replay_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }
}
