//! Run-bundle summary types: per-run aggregate stats written to
//! `summary.json` alongside the manifest + event log.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

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
    /// from the engine's per-tick `terrain.terrain_dirty_region_batch`
    /// samples. `coalesce_cost_avg` is the mean rects_in over the sampled
    /// window (last 1024 ticks). `total_rects_in` / `total_rects_out` are
    /// cumulative across the run. See `specs/active/M3.md` § Re-opened gaps.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terrain: Option<TerrainPerfBlock>,
    /// of the M4 v0.1 envelope. Existing M1-M8 bundle readers ignore the
    /// optional field; M8A producers populate per-subsystem
    /// p50/p99/p999 (microseconds). Consumed by `cf-mod validate-bundle`
    /// and `m8a_perf_gate.sh`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subsystems: Option<crate::perf::M8aPerfSummary>,
}

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
    /// recorder telemetry. `peak_buffer_depth` records the maximum queue
    /// depth reached at any point during the run; `dropped_cosmetic` /
    /// `dropped_gameplay` partition the dropped count by event class.
    /// Surfaced via `summary.json.recorder.*` per spec literal. Defaults
    /// to all-zero so legacy bundles without the field continue to parse.
    #[serde(default)]
    pub recorder: RecorderBlock,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecorderBlock {
    pub peak_buffer_depth: usize,
    pub dropped_cosmetic: u64,
    pub dropped_gameplay: u64,
    pub dropped_total: u64,
}
