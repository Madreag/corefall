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
mod schemas_consts;
mod schemas_lookup;
mod schemas_validate;
#[cfg(test)]
mod schemas_tests;
pub mod shard;
pub mod snapshot_baseline;
pub mod snapshot_delta;

mod bundle;
mod event;
mod manifest;
mod recorder;
mod summary;

#[cfg(test)]
mod lib_tests;

pub use bundle::{run_bundle_dir_basename, write_run_bundle, BundleError, BundleInputs};
pub use bundle_paths::{default_run_bundle_root, resolve_run_bundle_root};
pub use event::Event;
pub use manifest::{
    BuildInfo, CapabilitiesBlock, CaptureConfig, ChecksumConfig, ExpectedOutcome, RunManifest, SceneInfo,
    SettingsBlock,
};
pub use record_id::{EntityKind, RecordId, RecordIdRegistry};
pub use recorder::{AssetRefRecordParams, Recorder};
pub use summary::{
    ArtifactItem, ArtifactsBlock, EventCounts, PerformanceBlock, PerfSample, RecorderBlock, RunSummary, TerrainPerfBlock,
    TestRecord, VolumeBlock,
};
