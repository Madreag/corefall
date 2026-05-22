//! Run-bundle manifest types: build/scene/capture/checksum/settings/capabilities
//! and the top-level `RunManifest`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use cf_sim_core::checksum::{CHECKSUM_ALGORITHM, CHECKSUM_SCOPE};

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
    /// "tough_crowd", "veteran") — the live preset applied by
    /// `act.settings.set { ai_difficulty: ... }`. Persisted into the run
    /// manifest so consumers can reproduce the run without consulting
    /// per-event observe.settings probes. Default empty so legacy bundles
    /// deserialize cleanly.
    #[serde(default)]
    pub ai_difficulty: String,
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
    /// SaveSchemaVersion this run was recorded under. Serialized as a
    /// 3-element JSON array `[major, minor, patch]` (canonical form so
    /// canonical-JSON BLAKE3 stays unambiguous). Defaults to the current
    /// build's version so legacy bundles without the field deserialize
    /// cleanly.
    #[serde(default = "default_save_schema_version")]
    pub save_schema_version: [u16; 3],
    /// snapshots are emitted into the run bundle. Default 600 ticks
    /// (10 s @ 60 Hz). Configurable per scenario.
    #[serde(default = "default_delta_baseline_cadence_ticks")]
    pub delta_baseline_cadence_ticks: u64,
    /// anchor (chained_hash_hex of the final event in the run). Set in
    /// tournament mode; `None` in dev mode so existing bundles continue
    /// to parse unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ledger_chain_anchor: Option<String>,
}

/// Default the save schema version to the current build's `[2, 0, 0]` so
/// legacy bundles without the field deserialize cleanly.
pub(crate) fn default_save_schema_version() -> [u16; 3] {
    [2, 0, 0]
}

/// Default delta baseline cadence (600 ticks = 10 s at 60 Hz) per
/// M4B § "Delta baseline cadence is enforced".
pub(crate) fn default_delta_baseline_cadence_ticks() -> u64 {
    600
}
