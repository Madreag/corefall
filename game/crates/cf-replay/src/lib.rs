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
pub mod perf;
pub mod record_id;
pub mod schemas;
pub mod shard;

pub use bundle_paths::{default_run_bundle_root, resolve_run_bundle_root};
pub use record_id::{EntityKind, RecordId, RecordIdRegistry};

/// One DR-002 v1 event. M4 envelope is locked at v0.1; the optional fields
/// (`parent_event_id`, `actor_id`, `source_id`, `team`, `pos`, `bbox`,
/// `dropped_count`, `cosmetic`, `asset_ref`) are envelope-level so consumers
/// (cause-chain walker, replay viewer, M4A asset ledger) can index by them
/// without reaching into `payload`. Additive envelope extensions require a
/// schema bump (locked at v0.1 at M4).
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
    /// **M4 envelope v0.1**: optional envelope-level actor reference. When
    /// the event is about / caused by a specific actor, set this so
    /// downstream consumers can filter without parsing the payload.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub actor_id: Option<u64>,
    /// **M4 envelope v0.1**: optional source actor (the one taking action).
    /// Distinct from `actor_id` (the affected actor) — e.g. shooter vs
    /// victim, or carrier vs item.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source_id: Option<u64>,
    /// **M4 envelope v0.1**: optional team string ("player" / "enemy" /
    /// "neutral" / faction name) for fast filtering.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub team: Option<String>,
    /// **M4 envelope v0.1**: optional 2D world position [x, y] where the
    /// event happened. Surface-level convenience for spatial filtering.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub pos: Option<[f32; 2]>,
    /// **M4 envelope v0.1**: optional bounding box [min_x, min_y, max_x,
    /// max_y] for events that span an area (terrain carve, blast, hazard
    /// cell). Surface-level convenience for spatial filtering.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub bbox: Option<[f32; 4]>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub dropped_count: Option<u64>,
    /// M4 § DR-052 cosmetic vs gameplay split. When `Some(true)`, this event
    /// is a cosmetic surface (particle, debris spawn, UI banner, etc.) and
    /// MUST be excluded from `determinism.sim_checksum` hashing AND
    /// preferentially dropped first under recorder backpressure. The
    /// underlying STATE change (terrain integrity, hazard intensity,
    /// affliction severity) is hashed through the actor/world state — the
    /// cosmetic event only DESCRIBES the change. When `None` or `Some(false)`
    /// the event is a gameplay surface.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cosmetic: Option<bool>,
    /// **M4 ↔ M4A integration**: optional reference to a `cf-asset-ledger`
    /// entry. Set on events that reference an AI-generated asset (capture
    /// grid screenshot, audio playback, mod-supplied content). M4A's
    /// `cf-mod ledger verify` cross-references this against the ledger's
    /// `AssetId` registry. The asset_ref value is a string-encoded
    /// `AssetId` (blake3 hex prefix).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub asset_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildInfo {
    pub commit_sha: String,
    /// True when the run was produced from an uncommitted worktree. This is
    /// distinct from `commit_sha` because many dirty runs can share the same
    /// HEAD while carrying materially different code/content.
    #[serde(default)]
    pub worktree_dirty: bool,
    /// Fingerprint of the dirty diff + untracked relevant file content. Present
    /// only when `worktree_dirty` is true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_fingerprint: Option<String>,
    /// Short audit list of files contributing to the dirty fingerprint.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub worktree_dirty_files: Vec<String>,
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
    /// M4A: ACC-A-05 hold-to-press alternative.
    #[serde(default)]
    pub hold_to_confirm: bool,
    /// M4A: ACC-A-05 hold threshold (ms).
    #[serde(default = "default_hold_threshold_ms")]
    pub hold_threshold_ms: u32,
    /// M4A: ACC-A-05 future remap UI surface flag.
    #[serde(default)]
    pub key_remap_enabled: bool,
    /// M4A: ACC-A-05 active key binding overrides (action -> KeyCode name).
    /// Stored in the run manifest so bundles can reconstruct the actual input
    /// contract that produced the capture, even though key bindings do not
    /// affect the deterministic sim checksum directly.
    #[serde(default)]
    pub key_bindings: BTreeMap<String, String>,
    /// **M2 audit pass 5 (2026-05-13)**: AI difficulty preset id ("cakewalk",
    /// "tough_crowd", "veteran") — the live preset applied by
    /// `act.settings.set { ai_difficulty: ... }`. Persisted into the run
    /// manifest so consumers can reproduce the run without consulting
    /// per-event observe.settings probes. Default empty so legacy bundles
    /// deserialize cleanly.
    #[serde(default)]
    pub ai_difficulty: String,
    /// **M1 audit pass 7 (2026-05-13)**: full "feel cvars" suite persisted
    /// into run_manifest.json.settings so deterministic replay tools can
    /// reconstruct the run's tunings without consulting `observe.settings`
    /// probes. All fields default to 0.0/false/0 so legacy bundles
    /// deserialize cleanly.
    #[serde(default)]
    pub accel: f32,
    #[serde(default)]
    pub friction: f32,
    #[serde(default)]
    pub gravity: f32,
    #[serde(default)]
    pub jump_force: f32,
    #[serde(default)]
    pub recoil_decay_per_tick: f32,
    #[serde(default)]
    pub sharp_aim_build_ticks: u32,
    #[serde(default)]
    pub walk_threshold: f32,
    #[serde(default)]
    pub reduce_camera_shake_pct: f32,
    #[serde(default)]
    pub tick_rate_hz: u32,
}

fn default_hold_threshold_ms() -> u32 {
    250
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
            hold_to_confirm: false,
            hold_threshold_ms: default_hold_threshold_ms(),
            key_remap_enabled: false,
            key_bindings: BTreeMap::new(),
            ai_difficulty: String::new(),
            accel: 0.0,
            friction: 0.0,
            gravity: 0.0,
            jump_force: 0.0,
            recoil_decay_per_tick: 0.0,
            sharp_aim_build_ticks: 0,
            walk_threshold: 0.0,
            reduce_camera_shake_pct: 0.0,
            tick_rate_hz: 0,
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
    /// **M3 re-open (2026-05-13)**: terrain coalesce-cost roll-up. Populated
    /// from the engine's per-tick `terrain.terrain_dirty_region_batch`
    /// samples. `coalesce_cost_avg` is the mean rects_in over the sampled
    /// window (last 1024 ticks). `total_rects_in` / `total_rects_out` are
    /// cumulative across the run. See `specs/active/M3.md` § Re-opened gaps.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terrain: Option<TerrainPerfBlock>,
    /// **M8A § per-subsystem latency budgets** — additive-only extension
    /// of the M4 v0.1 envelope. Existing M1-M8 bundle readers ignore the
    /// optional field; M8A producers populate per-subsystem
    /// p50/p99/p999 (microseconds). Consumed by `cf-mod validate-bundle`
    /// and `m8a_perf_gate.sh`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subsystems: Option<crate::perf::M8aPerfSummary>,
}

/// **M3 re-open (2026-05-13)**: terrain coalesce-cost roll-up for
/// `summary.json.perf.terrain`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TerrainPerfBlock {
    pub coalesce_cost_avg: f64,
    pub coalesce_cost_max: u32,
    pub total_rects_in: u64,
    pub total_rects_out: u64,
    pub batches_emitted: u64,
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
    /// **M3 re-open**: optional terrain perf block.
    pub terrain: Option<TerrainPerfBlock>,
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
            terrain: s.terrain,
            subsystems: None,
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
    /// **M4 § Recorder backpressure does not drop silently**: per-run
    /// recorder telemetry. `peak_buffer_depth` records the maximum queue
    /// depth reached at any point during the run; `dropped_cosmetic` /
    /// `dropped_gameplay` partition the dropped count by event class.
    /// Surfaced via `summary.json.recorder.*` per spec literal. Defaults
    /// to all-zero so legacy bundles without the field continue to parse.
    #[serde(default)]
    pub recorder: RecorderBlock,
}

/// **M4 § Recorder backpressure**: per-run recorder telemetry block.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecorderBlock {
    pub peak_buffer_depth: usize,
    pub dropped_cosmetic: u64,
    pub dropped_gameplay: u64,
    pub dropped_total: u64,
}

/// Append-friendly recorder. Events go through here so the writer can apply backpressure
/// and surface dropped counts in `summary.json.event_counts.dropped_total`.
///
/// **M4 § Recorder backpressure**: when `capacity` is reached, the recorder
/// drops the oldest COSMETIC event first (priority-aware drop) so gameplay
/// events are never starved by particle/visual cosmetics. If no cosmetic
/// event is in the buffer, the new gameplay event itself is dropped (counted
/// in `dropped_total` and the next emitted event picks up the dropped count).
pub struct Recorder {
    run_id: String,
    seq: AtomicU64,
    inner: Mutex<RecorderInner>,
    /// Maximum events before backpressure drops. 0 = unlimited.
    capacity: usize,
}

struct RecorderInner {
    events: Vec<Event>,
    by_category: BTreeMap<String, u64>,
    by_type: BTreeMap<String, u64>,
    by_severity: BTreeMap<String, u64>,
    dropped: u64,
    dropped_cosmetic: u64,
    dropped_gameplay: u64,
    /// Outstanding drops not yet attached to a subsequent emitted event's
    /// `dropped_count` payload field (per M4 § "the per-event payload that
    /// triggered the overflow includes dropped_count=N in the next emitted
    /// event").
    pending_drop_tag: u64,
    peak_buffer_depth: usize,
    first_tick: Option<u64>,
    last_tick: Option<u64>,
    final_checksum: Option<String>,
    checksum_event_count: u64,
}

impl Recorder {
    pub fn new(run_id: String) -> Self {
        Self::with_capacity(run_id, 0)
    }

    /// Create a recorder with a maximum event capacity. 0 = unlimited.
    /// When capacity is exceeded, new events are dropped and the dropped
    /// counter is incremented (surfaced in summary.json.event_counts.dropped_total).
    pub fn with_capacity(run_id: String, capacity: usize) -> Self {
        Self {
            run_id,
            seq: AtomicU64::new(0),
            inner: Mutex::new(RecorderInner {
                events: Vec::new(),
                by_category: BTreeMap::new(),
                by_type: BTreeMap::new(),
                by_severity: BTreeMap::new(),
                dropped: 0,
                dropped_cosmetic: 0,
                dropped_gameplay: 0,
                pending_drop_tag: 0,
                peak_buffer_depth: 0,
                first_tick: None,
                last_tick: None,
                final_checksum: None,
                checksum_event_count: 0,
            }),
            capacity,
        }
    }

    pub fn peak_buffer_depth(&self) -> usize {
        self.inner.lock().expect("recorder mutex poisoned").peak_buffer_depth
    }

    pub fn dropped_cosmetic_count(&self) -> u64 {
        self.inner.lock().expect("recorder mutex poisoned").dropped_cosmetic
    }

    pub fn dropped_gameplay_count(&self) -> u64 {
        self.inner.lock().expect("recorder mutex poisoned").dropped_gameplay
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn dropped_count(&self) -> u64 {
        self.inner.lock().expect("recorder mutex poisoned").dropped
    }

    pub fn event_count(&self) -> usize {
        self.inner.lock().expect("recorder mutex poisoned").events.len()
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
        self.record_with_cosmetic(tick, sim_time_ms, category, event_type, payload, parent_event_id, false)
    }

    /// M4 § Recorder backpressure / cosmetic flag. Record an event flagged as
    /// cosmetic (`cosmetic=true`) so the determinism island excludes it from
    /// `sim_state_v1` hashing and the recorder drops it FIRST under
    /// backpressure (priority-aware drop).
    pub fn record_cosmetic(
        &self,
        tick: Tick,
        sim_time_ms: f64,
        category: &str,
        event_type: &str,
        payload: serde_json::Value,
        parent_event_id: Option<String>,
    ) -> String {
        self.record_with_cosmetic(tick, sim_time_ms, category, event_type, payload, parent_event_id, true)
    }

    /// **M4 ↔ M4A integration**: record an event referencing an asset-ledger
    /// entry (`asset_ref`). Used by capture-grid screenshots, audio playback,
    /// mod-supplied content, etc. The asset_ref value is a string-encoded
    /// `cf-asset-ledger::AssetId` (blake3 hex). Cosmetic when `cosmetic` is
    /// true — capture surfaces don't participate in the deterministic sim
    /// checksum.
    pub fn record_with_asset_ref(&self, params: AssetRefRecordParams<'_>) -> String {
        let AssetRefRecordParams {
            tick,
            sim_time_ms,
            category,
            event_type,
            payload,
            parent_event_id,
            asset_ref,
            cosmetic,
        } = params;
        let event_id = self.record_with_cosmetic(
            tick,
            sim_time_ms,
            category,
            event_type,
            payload,
            parent_event_id,
            cosmetic,
        );
        let mut inner = self.inner.lock().expect("recorder mutex poisoned; aborting run");
        if let Some(last) = inner.events.last_mut() {
            if last.event_id == event_id {
                last.asset_ref = Some(asset_ref);
            }
        }
        event_id
    }

    /// M4 § "unknown_cause" marker. Record an event with `parent_event_id = None`
    /// and inject `cause_origin: "unknown_cause"` + a `reason` field into
    /// the payload so the M10 cause-chain walker reports a clean terminal
    /// instead of a "missing parent" bug.
    ///
    /// Use this for events that have no causal predecessor in the event
    /// log (external interrupts, scenario-start defaults, sim-tick fallthroughs
    /// where no upstream cause exists).
    pub fn record_with_unknown_cause(
        &self,
        tick: Tick,
        sim_time_ms: f64,
        category: &str,
        event_type: &str,
        mut payload: serde_json::Value,
        reason: &str,
    ) -> String {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("cause_origin".to_string(), serde_json::json!("unknown_cause"));
            obj.insert("cause_origin_reason".to_string(), serde_json::json!(reason));
        }
        self.record(tick, sim_time_ms, category, event_type, payload, None)
    }

    fn record_with_cosmetic(
        &self,
        tick: Tick,
        sim_time_ms: f64,
        category: &str,
        event_type: &str,
        payload: serde_json::Value,
        parent_event_id: Option<String>,
        cosmetic: bool,
    ) -> String {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let event_id = make_event_id(&self.run_id, tick.0, seq);
        let mut inner = self.inner.lock().expect("recorder mutex poisoned; aborting run");
        let effective_tick = if let Some(last) = inner.last_tick {
            tick.0.max(last)
        } else {
            tick.0
        };
        // Drain any outstanding drop tag onto THIS event before it's stored.
        let drop_tag = if inner.pending_drop_tag > 0 && !cosmetic {
            let n = inner.pending_drop_tag;
            inner.pending_drop_tag = 0;
            Some(n)
        } else {
            None
        };
        let event = Event {
            schema_version: EVENT_SCHEMA_VERSION.to_string(),
            run_id: self.run_id.clone(),
            tick: effective_tick,
            sim_time_ms,
            event_id: event_id.clone(),
            category: category.to_string(),
            event_type: event_type.to_string(),
            payload,
            parent_event_id,
            actor_id: None,
            source_id: None,
            team: None,
            pos: None,
            bbox: None,
            dropped_count: drop_tag,
            cosmetic: if cosmetic { Some(true) } else { None },
            asset_ref: None,
        };
        *inner.by_category.entry(event.category.clone()).or_insert(0) += 1;
        *inner.by_type.entry(event.event_type.clone()).or_insert(0) += 1;
        inner.first_tick.get_or_insert(effective_tick);
        inner.last_tick = Some(effective_tick);
        if event.category == "determinism" && event.event_type == "sim_checksum" {
            inner.checksum_event_count += 1;
            if let Some(hex) = event.payload.get("checksum_hex").and_then(|v| v.as_str()) {
                inner.final_checksum = Some(hex.to_string());
            }
        }
        if self.capacity > 0 && inner.events.len() >= self.capacity {
            // M4 § "Cosmetic events drop first under pressure". If the new
            // event is cosmetic, drop the new event immediately. Otherwise,
            // try to evict the oldest cosmetic event from the buffer to make
            // room. If no cosmetic event is available, drop the gameplay
            // event itself and tag the dropped_count on the next emitted
            // event.
            if cosmetic {
                inner.dropped += 1;
                inner.dropped_cosmetic += 1;
                inner.pending_drop_tag += 1;
                return event_id;
            }
            // Search for the oldest cosmetic event to evict.
            let mut evict_idx: Option<usize> = None;
            for (idx, ev) in inner.events.iter().enumerate() {
                if ev.cosmetic == Some(true) {
                    evict_idx = Some(idx);
                    break;
                }
            }
            if let Some(idx) = evict_idx {
                let evicted = inner.events.remove(idx);
                if let Some(cat_count) = inner.by_category.get_mut(&evicted.category) {
                    *cat_count = cat_count.saturating_sub(1);
                }
                if let Some(ty_count) = inner.by_type.get_mut(&evicted.event_type) {
                    *ty_count = ty_count.saturating_sub(1);
                }
                inner.dropped += 1;
                inner.dropped_cosmetic += 1;
                inner.pending_drop_tag += 1;
                inner.events.push(event);
                let depth = inner.events.len();
                if depth > inner.peak_buffer_depth {
                    inner.peak_buffer_depth = depth;
                }
                return event_id;
            }
            // No cosmetic event to evict; drop the gameplay event itself.
            inner.dropped += 1;
            inner.dropped_gameplay += 1;
            inner.pending_drop_tag += 1;
            return event_id;
        }
        inner.events.push(event);
        let depth = inner.events.len();
        if depth > inner.peak_buffer_depth {
            inner.peak_buffer_depth = depth;
        }
        event_id
    }

    pub fn record_severity(&self, severity: &str) {
        let mut inner = self.inner.lock().expect("recorder mutex poisoned; aborting run");
        *inner.by_severity.entry(severity.to_string()).or_insert(0) += 1;
    }

    pub fn dropped(&self, count: u64) {
        let mut inner = self.inner.lock().expect("recorder mutex poisoned; aborting run");
        inner.dropped += count;
        inner.pending_drop_tag = inner.pending_drop_tag.saturating_add(count);
    }

    /// **M4**: tag the next emitted event with the current outstanding
    /// drop count so it surfaces in the bundle (per M4 § "the per-event
    /// payload that triggered the overflow includes dropped_count in the
    /// next emitted event"). Returns the count and clears the outstanding
    /// counter so it's not reported twice.
    pub fn take_outstanding_drop_count(&self) -> u64 {
        let inner = self.inner.lock().expect("recorder mutex poisoned; aborting run");
        let n = inner.dropped;
        // Note: we don't zero `inner.dropped` here — `dropped_total` is
        // cumulative for `summary.json`. Callers that want a per-emit delta
        // should diff against the prior return.
        n
    }

    /// Snapshot the entire event log. Panics on mutex poisoning per the
    /// consistent recorder error-handling strategy (issue #22): poisoning
    /// indicates a critical bug, not a transient failure to silently degrade.
    pub fn snapshot_events(&self) -> Vec<Event> {
        let inner = self.inner.lock().expect("recorder mutex poisoned; aborting run");
        inner.events.clone()
    }

    /// Return events recorded after `after_idx` (i.e., the tail since the
    /// caller last polled). Panics on mutex poisoning per the consistent
    /// recorder error-handling strategy (issue #22).
    pub fn events_since(&self, after_idx: usize) -> Vec<Event> {
        let inner = self.inner.lock().expect("recorder mutex poisoned; aborting run");
        if after_idx >= inner.events.len() {
            Vec::new()
        } else {
            inner.events[after_idx..].to_vec()
        }
    }

    pub fn counts(&self) -> EventCounts {
        let inner = self.inner.lock().expect("recorder mutex poisoned; aborting run");
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
        let inner = self.inner.lock().expect("recorder mutex poisoned; aborting run");
        (inner.first_tick, inner.last_tick)
    }

    pub fn final_checksum_hex(&self) -> Option<String> {
        let inner = self.inner.lock().expect("recorder mutex poisoned; aborting run");
        inner.final_checksum.clone()
    }

    pub fn checksum_event_count(&self) -> u64 {
        let inner = self.inner.lock().expect("recorder mutex poisoned; aborting run");
        inner.checksum_event_count
    }
}

/// **M4A ↔ M4**: parameter bundle for [`Recorder::record_with_asset_ref`].
/// Bundles the envelope identity (`tick`, `sim_time_ms`, `category`, etc.)
/// plus the `asset_ref` (string-encoded `cf-asset-ledger::AssetId`) and the
/// cosmetic flag so the recorder writes a single event with the ledger
/// pointer on the M4 envelope.
pub struct AssetRefRecordParams<'a> {
    pub tick: Tick,
    pub sim_time_ms: f64,
    pub category: &'a str,
    pub event_type: &'a str,
    pub payload: serde_json::Value,
    pub parent_event_id: Option<String>,
    pub asset_ref: String,
    pub cosmetic: bool,
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
            terrain: None,
            subsystems: None,
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
        recorder: RecorderBlock {
            peak_buffer_depth: inputs.recorder.peak_buffer_depth(),
            dropped_cosmetic: inputs.recorder.dropped_cosmetic_count(),
            dropped_gameplay: inputs.recorder.dropped_gameplay_count(),
            dropped_total: inputs.recorder.dropped_count(),
        },
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

    /// **M4 ↔ M4A integration**: `record_with_asset_ref` populates the
    /// envelope-level `asset_ref` field with a ledger AssetId string. Run
    /// bundles that capture grids / audio playback / mod assets MUST link
    /// to the canonical ledger entry via this field.
    #[test]
    fn record_with_asset_ref_populates_envelope_field() {
        let recorder = Recorder::new("m4a_test_run".to_string());
        let asset_id = "a".repeat(64);
        let id = recorder.record_with_asset_ref(AssetRefRecordParams {
            tick: Tick(1),
            sim_time_ms: 16.6,
            category: "capture",
            event_type: "capture_grid_screenshot",
            payload: serde_json::json!({"path": "captures/grid_001.png"}),
            parent_event_id: None,
            asset_ref: asset_id.clone(),
            cosmetic: true,
        });
        let events = recorder.snapshot_events();
        assert_eq!(events.len(), 1);
        let serialized = serde_json::to_string(&events[0]).unwrap();
        assert!(
            serialized.contains(&format!("\"asset_ref\":\"{asset_id}\"")),
            "asset_ref must round-trip through JSON envelope: {serialized}"
        );
        let parsed: Event = serde_json::from_str(&serialized).unwrap();
        assert_eq!(parsed.event_id, id);
        assert_eq!(parsed.asset_ref.as_deref(), Some(asset_id.as_str()));
        assert_eq!(parsed.cosmetic, Some(true));
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

    #[test]
    fn expected_outcome_default_is_clean() {
        let outcome: ExpectedOutcome = Default::default();
        assert!(matches!(outcome, ExpectedOutcome::Clean));
        assert_eq!(outcome.as_str(), "clean");
    }

    #[test]
    fn expected_outcome_as_str_covers_all_variants() {
        assert_eq!(ExpectedOutcome::Clean.as_str(), "clean");
        assert_eq!(ExpectedOutcome::Panic.as_str(), "panic");
        assert_eq!(ExpectedOutcome::Abort.as_str(), "abort");
    }

    #[test]
    fn expected_outcome_serializes_as_snake_case() {
        for (variant, expected) in [
            (ExpectedOutcome::Clean, "\"clean\""),
            (ExpectedOutcome::Panic, "\"panic\""),
            (ExpectedOutcome::Abort, "\"abort\""),
        ] {
            let s = serde_json::to_string(&variant).expect("serialize");
            assert_eq!(s, expected);
        }
    }

    #[test]
    fn expected_outcome_deserializes_from_snake_case() {
        let clean: ExpectedOutcome = serde_json::from_str("\"clean\"").expect("deserialize clean");
        let panic: ExpectedOutcome = serde_json::from_str("\"panic\"").expect("deserialize panic");
        let abort: ExpectedOutcome = serde_json::from_str("\"abort\"").expect("deserialize abort");
        assert!(matches!(clean, ExpectedOutcome::Clean));
        assert!(matches!(panic, ExpectedOutcome::Panic));
        assert!(matches!(abort, ExpectedOutcome::Abort));
    }

    #[test]
    fn expected_outcome_rejects_unknown_string() {
        let res: Result<ExpectedOutcome, _> = serde_json::from_str("\"weird\"");
        assert!(res.is_err(), "unknown expected_outcome value must reject");
    }

    #[test]
    fn expected_outcome_default_in_run_manifest_when_absent() {
        // Round-trip a manifest JSON without the expected_outcome field
        // and confirm the deserializer falls back to Clean (the M3A-005 default).
        let manifest_no_outcome = serde_json::json!({
            "schema_version": MANIFEST_SCHEMA_VERSION,
            "run_id": "m0_test_no_outcome",
            "prototype_slice": "M0",
            "run_mode": "test",
            "milestone": "m0",
            "build": {"commit_sha": "deadbeef", "rust_version": "rustc 1.95", "bevy_version": "bevy 0.18", "platform": "test"},
            "scene": {"id": "m0_blank", "display_name": "test", "source_path": "x"},
            "seed": 42,
            "started_at_utc": "2026-05-05T00:00:00Z",
            "duration_target_sec": 5.0,
            "material_schema_version": "n/a-m0",
            "config_hash": "deadbeef",
            "assumptions_tested": [],
            "linked_specs": [],
            "expected_tests": [],
            "capture_config": {"events": true, "screenshots": false, "captures": false},
            "schemas": {"control": 1, "scenario": 1, "events": 1},
            "capabilities": {"debug": false, "control_api": true, "save_load": false, "debug_capabilities": []},
            "settings": {"ui_scale": 1.0, "high_contrast": false, "captions": true, "reduced_motion": false, "reduced_shake": false, "reduced_flash": false},
            "checksum": {"algorithm": "blake3", "scope": "sim_state_v1", "cadence_ticks": 60},
            "tick_rate_hz": 60
        });
        let parsed: RunManifest =
            serde_json::from_value(manifest_no_outcome).expect("manifest without expected_outcome must parse");
        assert!(
            matches!(parsed.expected_outcome, ExpectedOutcome::Clean),
            "missing expected_outcome must default to Clean"
        );
        assert!(
            parsed.settings.key_bindings.is_empty(),
            "legacy manifests without key_bindings must deserialize with an empty remap table"
        );
    }

    #[test]
    fn recorder_record_panics_on_poisoned_mutex_instead_of_returning_phantom_event_id() {
        // Issue #18 regression: previously the recorder would silently log
        // and return an event_id for an event that was never recorded when
        // the inner mutex was poisoned. Downstream code (events_since,
        // run-bundle writer, summary aggregator) would then reference an
        // event id that doesn't exist in the events log. The new behavior
        // is to panic loudly via `expect()` — mutex poisoning means a
        // thread panicked while holding the recorder lock, which means the
        // run is already in an inconsistent state and should abort.
        use std::sync::Arc;
        let recorder = Arc::new(Recorder::new("m0_test_poison_e2e".to_string()));
        // Poison the mutex by panicking inside a thread that holds the lock.
        let recorder_for_thread = recorder.clone();
        let _ = std::thread::spawn(move || {
            let _guard = recorder_for_thread
                .inner
                .lock()
                .expect("first lock acquisition should succeed in setup");
            panic!("intentional poison for regression test");
        })
        .join();
        // Now any record() call should panic with the documented message.
        let recorder_for_call = recorder.clone();
        let panic_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            recorder_for_call.record(Tick(0), 0.0, "system", "run_started", serde_json::json!({}), None);
        }));
        assert!(
            panic_result.is_err(),
            "record() must panic when mutex is poisoned (issue #18); previously it silently returned a phantom event_id"
        );
    }

    #[test]
    fn m4_recorder_cosmetic_events_drop_first_under_pressure() {
        // M4 § Acceptance: "Cosmetic events drop first under pressure".
        let recorder = Recorder::with_capacity("m4_test_drop".to_string(), 3);
        // Fill with two cosmetic events.
        recorder.record_cosmetic(Tick(0), 0.0, "ux", "banner_raised", serde_json::json!({}), None);
        recorder.record_cosmetic(Tick(0), 0.0, "ux", "banner_raised", serde_json::json!({}), None);
        // One gameplay event.
        recorder.record(Tick(0), 0.0, "combat", "weapon_fired", serde_json::json!({}), None);
        assert_eq!(recorder.event_count(), 3);
        assert_eq!(recorder.dropped_count(), 0);

        // Now over capacity. A new GAMEPLAY event MUST evict a cosmetic
        // event so the gameplay event lands in the buffer.
        recorder.record(Tick(0), 0.0, "combat", "wound_added", serde_json::json!({}), None);
        assert_eq!(recorder.event_count(), 3, "buffer still capped at capacity");
        assert_eq!(recorder.dropped_count(), 1);
        assert_eq!(recorder.dropped_cosmetic_count(), 1);
        assert_eq!(recorder.dropped_gameplay_count(), 0);
        // Verify the gameplay event is present.
        let events = recorder.snapshot_events();
        let types: Vec<&str> = events.iter().map(|e| e.event_type.as_str()).collect();
        assert!(types.contains(&"wound_added"));
        assert!(types.contains(&"weapon_fired"));
    }

    #[test]
    fn m4_recorder_cosmetic_dropped_when_no_eviction_target_available() {
        // When the buffer is filled with gameplay events, a new cosmetic
        // event is dropped immediately (no gameplay eviction).
        let recorder = Recorder::with_capacity("m4_test_cos".to_string(), 2);
        recorder.record(Tick(0), 0.0, "combat", "weapon_fired", serde_json::json!({}), None);
        recorder.record(Tick(0), 0.0, "combat", "wound_added", serde_json::json!({}), None);
        recorder.record_cosmetic(Tick(0), 0.0, "ux", "banner_raised", serde_json::json!({}), None);
        assert_eq!(recorder.event_count(), 2);
        assert_eq!(recorder.dropped_cosmetic_count(), 1);
        assert_eq!(recorder.dropped_gameplay_count(), 0);
    }

    #[test]
    fn m4_recorder_gameplay_dropped_when_buffer_full_of_gameplay() {
        // When no cosmetic event is available to evict, a new gameplay
        // event is dropped.
        let recorder = Recorder::with_capacity("m4_test_gameplay".to_string(), 2);
        recorder.record(Tick(0), 0.0, "combat", "weapon_fired", serde_json::json!({}), None);
        recorder.record(Tick(0), 0.0, "combat", "wound_added", serde_json::json!({}), None);
        recorder.record(Tick(0), 0.0, "combat", "kill", serde_json::json!({}), None);
        assert_eq!(recorder.event_count(), 2);
        assert_eq!(recorder.dropped_count(), 1);
        assert_eq!(recorder.dropped_gameplay_count(), 1);
    }

    #[test]
    fn m4_recorder_cosmetic_field_serializes_only_when_set() {
        let recorder = Recorder::new("m4_test_field".to_string());
        recorder.record(Tick(0), 0.0, "combat", "weapon_fired", serde_json::json!({}), None);
        recorder.record_cosmetic(Tick(0), 0.0, "ux", "banner_raised", serde_json::json!({}), None);
        let events = recorder.snapshot_events();
        // Gameplay event must not serialize cosmetic field.
        let s = serde_json::to_string(&events[0]).unwrap();
        assert!(
            !s.contains("\"cosmetic\""),
            "gameplay event must not serialize cosmetic field: {s}"
        );
        // Cosmetic event must serialize cosmetic: true.
        let s = serde_json::to_string(&events[1]).unwrap();
        assert!(
            s.contains("\"cosmetic\":true"),
            "cosmetic event must serialize cosmetic: true: {s}"
        );
    }

    #[test]
    fn m4_recorder_peak_buffer_depth_tracks_high_water_mark() {
        let recorder = Recorder::new("m4_test_peak".to_string());
        recorder.record(Tick(0), 0.0, "combat", "weapon_fired", serde_json::json!({}), None);
        recorder.record(Tick(0), 0.0, "combat", "wound_added", serde_json::json!({}), None);
        recorder.record(Tick(0), 0.0, "combat", "kill", serde_json::json!({}), None);
        assert_eq!(recorder.peak_buffer_depth(), 3);
    }

    #[test]
    fn m4_recorder_pending_drop_tag_propagates_to_next_event() {
        let recorder = Recorder::with_capacity("m4_test_tag".to_string(), 1);
        recorder.record(Tick(0), 0.0, "combat", "weapon_fired", serde_json::json!({}), None);
        // Force two gameplay drops (no cosmetic available).
        recorder.record(Tick(0), 0.0, "combat", "wound_added", serde_json::json!({}), None);
        recorder.record(Tick(0), 0.0, "combat", "kill", serde_json::json!({}), None);
        // Manually trigger another drop via the public dropped() API.
        recorder.dropped(1);
        // Bigger buffer for the next event.
        let recorder = Recorder::new("m4_test_tag_2".to_string());
        recorder.dropped(7);
        let id = recorder.record(Tick(0), 0.0, "system", "run_started", serde_json::json!({}), None);
        let events = recorder.snapshot_events();
        let ev = events.iter().find(|e| e.event_id == id).expect("recorded event");
        assert_eq!(
            ev.dropped_count,
            Some(7),
            "next emitted event must carry the outstanding drop count"
        );
    }
}
