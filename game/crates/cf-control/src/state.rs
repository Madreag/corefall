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
    /// M1: typed projection of every actor in the world. Empty in M0 scenarios.
    #[serde(default)]
    pub actors: Vec<ActorView>,
    /// Convenience pointer to the player actor in `actors` by id, if any.
    #[serde(default)]
    pub player_actor_id: Option<u64>,
}

/// Public projection of one actor for the observe envelope. Mirrors
/// `cf_actor::ActorObservation` with extra fields (rifle ammo / cooldown / reload
/// state) the engine wires through.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ActorView {
    pub id: u64,
    pub team: String,
    pub controllable: bool,
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub aim: [f32; 2],
    pub on_ground: bool,
    pub status: String,
    pub hp: f32,
    pub hp_max: f32,
    pub selected_slot: u32,
    pub selected_item: String,
    pub rifle_ammo: Option<u32>,
    pub rifle_capacity: Option<u32>,
    pub rifle_fire_cooldown_ticks: Option<u32>,
    pub rifle_reload_remaining_ticks: Option<u32>,
    pub rifle_reload_total_ticks: Option<u32>,
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
