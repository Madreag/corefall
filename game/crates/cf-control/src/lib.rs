//! M0-006: cf-control envelope, schemas, and local control server.
//!
//! Transport pin: JSON-RPC 2.0 over WebSocket on `127.0.0.1:17890` (loopback only by default).
//! Optional Unix domain socket later. Every request/response/notification carries a
//! mandatory `schema_version: u32` (currently 1). Schema mismatches respond with
//! JSON-RPC error code `-32602` (`InvalidParams`) and a fix-hint per
//! `spec/ai-control-observability-layer.md`.

#![allow(
    clippy::redundant_closure,
    clippy::unnecessary_cast,
    clippy::ptr_arg
)]

pub mod components;
pub mod engine;
pub mod engine_audio;
pub mod engine_baselines;
pub mod engine_new;
#[cfg(test)]
mod engine_tests;
pub mod engine_build;
pub mod engine_chassis_emit;
pub mod engine_cinematic;
pub mod engine_dirty_flush;
pub mod engine_dispatch;
pub mod engine_drive_tick;
pub mod engine_emit_actor;
pub mod engine_emit_guard;
pub mod engine_emit_module;
pub mod engine_finalize_m14g;
pub mod engine_handle;
pub mod engine_helpers;
pub mod engine_hud_refresh;
pub mod engine_inventory_mut;
pub mod engine_m14b;
pub mod engine_m14d;
pub mod engine_m14e;
pub mod engine_m14ef_accessors;
pub mod engine_m14f;
pub mod engine_m14g;
pub mod engine_m15;
pub mod engine_m6_tick;
pub mod engine_m7;
pub mod engine_m7_baselines;
pub mod engine_m7b_emit;
pub mod engine_m8_tick;
pub mod engine_manifest;
pub mod engine_periodic_snapshots;
pub mod engine_perf;
pub mod engine_pipe_rupture;
pub mod engine_post_process_m6;
pub mod engine_save;
pub mod engine_tool_effect;
pub mod envelope;
pub mod m4b_save;
pub mod m6_actions;
pub mod m7_ai;
pub mod m7b_squad;
pub mod m8_ux;
pub mod m8b_net_admin;
pub mod m9b_trench;
pub mod m14h_treatment;
pub mod m14i_long_term;
pub mod m14j_mobility;
pub mod runtime;
pub mod scenario;
pub mod schemas;
pub mod server;
pub mod settings;
pub mod state;
pub mod world;

pub use m6_actions::{ActSquadIssueCommandParams, M6Action, M6ActionParams, SquadCommandKindOverWire};

pub use engine::{
    run_m0_inline, ActorRenderSnapshot, BreachRenderView, EnemyHudView, ExtractionZoneView, HudCachesSnapshot,
    InitialActorWorld, InitialBreachWorld, InitialGuard, M0Engine, M0EngineConfig, M0EngineOutcome, MissionHudView,
    ReactorArmorLayerView, ReactorHudView, RifleHudView, TerrainChunkUpdate, TerrainDigPreview, TerrainRenderSnapshot,
    TimerHudView, HUD_FOCUSABLE_NODES,
};
pub use envelope::{
    error_codes, JsonRpcError, JsonRpcId, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
    METHOD_OBSERVE_FRAME,
};
pub use runtime::{build_engine_config, locate_scenario, ConfigBuildError, ConfigInputs};
pub use scenario::{Scenario, ScenarioCapabilities, ScenarioLoadError, ScenarioObjectiveKind, ScenarioRegion};
pub use schemas::{SCHEMA_VERSION, SCHEMA_VERSION_MIN};
pub use server::{
    async_trait, CommandResult, ControlCommand, ControlServer, ControlServerConfig, EngineHandle, FocusDirection,
    SettingsPatch,
};
pub use settings::{
    default_key_bindings, is_supported_key_binding_action, is_supported_key_code_name, validate_key_bindings, Settings,
    SUPPORTED_KEY_BINDING_ACTIONS, SUPPORTED_KEY_CODE_NAMES,
};
pub use state::{
    AccessibilityFlagsView, ActorView, BreachView, ControlEnvelopeStatus, EnemyView, EngineState, MissionView,
    ObjectiveView, ObserveFrame, ObserveSettings, RunStatus,
};
