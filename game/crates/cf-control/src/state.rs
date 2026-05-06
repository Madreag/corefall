//! Shared engine-side state types observed via the cf-control envelope.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::settings::Settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Paused,
    Stepping,
    Ended,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EngineState {
    pub run_id: String,
    pub scenario: String,
    pub tick: u64,
    pub sim_time_ms: f64,
    pub run_status: RunStatus,
    pub seed: u64,
    pub tick_rate_hz: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ObserveFrame {
    pub schema_version: u32,
    pub run_id: String,
    pub tick: u64,
    pub sim_time_ms: f64,
    pub run_status: RunStatus,
    pub scenario: String,
    pub events_since: u64,
    pub events: Vec<serde_json::Value>,
    pub settings: ObserveSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ObserveSettings {
    pub schema_version: u32,
    pub settings: Settings,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AccessibilityFlagsView {
    pub schema_version: u32,
    pub ui_scale: f32,
    pub high_contrast: bool,
    pub captions: bool,
    pub reduced_motion: bool,
    pub reduced_shake: bool,
    pub reduced_flash: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ControlEnvelopeStatus {
    Accepted,
    Rejected,
    Queued,
}
