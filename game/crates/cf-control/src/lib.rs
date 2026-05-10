//! M0-006: cf-control envelope, schemas, and local control server.
//!
//! Transport pin: JSON-RPC 2.0 over WebSocket on `127.0.0.1:17890` (loopback only by default).
//! Optional Unix domain socket later. Every request/response/notification carries a
//! mandatory `schema_version: u32` (currently 1). Schema mismatches respond with
//! JSON-RPC error code `-32602` (`InvalidParams`) and a fix-hint per
//! `spec/ai-control-observability-layer.md`.

pub mod engine;
pub mod envelope;
pub mod runtime;
pub mod scenario;
pub mod schemas;
pub mod server;
pub mod settings;
pub mod state;

pub use engine::{
    run_m0_inline, ActorRenderSnapshot, BreachRenderView, EnemyHudView, ExtractionZoneView, HudCachesSnapshot,
    InitialActorWorld, InitialBreachWorld, InitialGuard, M0Engine, M0EngineConfig, M0EngineOutcome, MissionHudView,
    RifleHudView, HUD_FOCUSABLE_NODES,
};
pub use envelope::{
    error_codes, JsonRpcError, JsonRpcId, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
    METHOD_OBSERVE_FRAME,
};
pub use runtime::{build_engine_config, locate_scenario, ConfigBuildError, ConfigInputs};
pub use scenario::{Scenario, ScenarioCapabilities, ScenarioLoadError, ScenarioRegion};
pub use schemas::SCHEMA_VERSION;
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
