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
    /// M1.5: mission state machine projection. `None` for sandbox scenarios.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mission: Option<MissionView>,
    /// M1.5: breach strips in the scenario. Empty for sandbox scenarios.
    #[serde(default)]
    pub breaches: Vec<BreachView>,
    /// M1.5: reactive guards and their last-tick view. Empty for sandbox scenarios.
    #[serde(default)]
    pub enemies: Vec<EnemyView>,
}

/// M1.5 mission projection (re-exposed via JsonSchema-friendly types).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MissionView {
    pub result: String,
    pub loss_reason: Option<String>,
    pub elapsed_ticks: u64,
    pub time_limit_ticks: u64,
    pub ticks_remaining: Option<u64>,
    pub active_objective: Option<String>,
    pub objectives: Vec<ObjectiveView>,
    pub last_event_tick: u64,
    pub last_event_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ObjectiveView {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub optional: bool,
    pub target_actor: Option<u64>,
    pub target_breach: Option<String>,
    pub zone_min: Option<[f32; 2]>,
    pub zone_max: Option<[f32; 2]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BreachView {
    pub id: String,
    pub material: String,
    pub bbox_min: [f32; 2],
    pub bbox_max: [f32; 2],
    pub hp: f32,
    pub max_hp: f32,
    pub broken: bool,
    pub refusal_reason: Option<String>,
    pub dig_range: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EnemyView {
    pub actor: u64,
    pub state: String,
    pub last_tactic: String,
    pub ammo: u32,
    pub mag_capacity: u32,
    pub fire_cooldown_ticks: u32,
    pub reload_remaining_ticks: u32,
    pub aim_settle_remaining_ticks: u32,
    pub alert_dwell_remaining_ticks: u32,
    pub aim: [f32; 2],
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
