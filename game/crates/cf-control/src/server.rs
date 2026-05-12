//! Local control server implementing the M0 method catalog over WebSocket.
//!
//! M0 methods:
//!   - `scenario.load`             (params: ScenarioLoadParams) -> CommandAck
//!   - `scenario.reset`            -> CommandAck
//!   - `sim.pause` / `sim.resume`  -> CommandAck
//!   - `sim.step`                  (params: StepParams) -> CommandAck
//!   - `sim.run_for_ticks`         (params: RunForTicksParams) -> CommandAck
//!   - `observe.once`              (params: ObserveOnceParams) -> ObserveFrame
//!   - `observe.subscribe`         (params: ObserveSubscribeParams) -> CommandAck
//!     followed by `observe.frame` notifications (server -> client).
//!   - `observe.unsubscribe`       -> CommandAck
//!   - `observe.settings`          -> ObserveSettings
//!   - `act.player.move`           (params: ActPlayerMoveParams) -> CommandAck
//!   - `act.settings.set`          (params: SettingsPatch) -> CommandAck
//!   - `runbundle.write`           (params: RunBundleWriteParams) -> CommandAck
//!   - `system.shutdown`           (params: SystemShutdownParams) -> CommandAck
//!
//! Schema mismatches (`params.schema_version != 1`) reply with -32602 + fix-hint.
//! Unknown methods reply with -32601 (`MethodNotFound`).

use std::{net::SocketAddr, sync::Arc, time::Duration};

use futures_util::{SinkExt, StreamExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{
    net::TcpListener,
    select,
    sync::{mpsc, watch, Mutex},
};

/// Sticky shutdown signal. Use a `watch::<bool>` channel — once set to `true`
/// the value is sticky (every receiver sees `true` on the next `borrow()` or
/// `changed().await` regardless of when the signal was set), unlike
/// `tokio::sync::Notify::notify_waiters()` which is non-sticky and silently
/// drops the signal if no task is currently `.notified().await`-ing on it.
///
/// The non-sticky behavior of `Notify` was the root cause of the observation-
/// loop hang reported in PR #26 review (Devin 🔴 / Bugbot Medium): when the
/// observation loop was mid-`engine.snapshot().await` or `out_tx.send().await`
/// at the moment shutdown fired, the `notify_waiters()` call landed with no
/// receiver awaiting and was lost forever; the loop returned to its `select!`,
/// created a fresh `notified()` future, and waited indefinitely for a notify
/// that would never come. `watch::<bool>` is the standard tokio primitive for
/// sticky shutdown signals.
pub type ShutdownSignal = watch::Sender<bool>;
pub type ShutdownReceiver = watch::Receiver<bool>;

/// Construct a fresh shutdown signal pair.
pub fn shutdown_signal() -> (ShutdownSignal, ShutdownReceiver) {
    watch::channel(false)
}

/// Convenience: trigger shutdown on a sender. No-op if the sender's value is
/// already `true` (idempotent).
pub fn trigger_shutdown(tx: &ShutdownSignal) {
    let _ = tx.send(true);
}

/// Await the next moment the receiver observes shutdown==true. If the value
/// was already `true` when called, returns immediately on the next poll.
async fn wait_for_shutdown(rx: &mut ShutdownReceiver) {
    if *rx.borrow() {
        return;
    }
    // `changed()` resolves on every value mutation; loop until we observe true.
    while rx.changed().await.is_ok() {
        if *rx.borrow() {
            return;
        }
    }
    // Sender dropped — treat as shutdown so we don't hang forever.
}
use tokio_tungstenite::{accept_async, tungstenite::Message};

/// Maximum number of pending outbound WebSocket messages per connection. Hit
/// when a slow client cannot keep up with the observation stream — producers
/// then `await` on `send()` and the observation loop naturally paces itself
/// to the client. Avoids the unbounded-memory growth that
/// `mpsc::unbounded_channel()` would allow under sustained backpressure.
const OUTBOUND_CHANNEL_CAPACITY: usize = 256;

use cf_actor::IntentSource;

use crate::{
    envelope::{
        error_codes, JsonRpcError, JsonRpcId, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
        METHOD_OBSERVE_FRAME,
    },
    schemas::{
        ActChassisClearJamParams, ActChassisRepairParams, ActChassisSalvageParams, ActInputCaptureControlsParams,
        ActMissionPauseParams, ActMissionResumeParams, ActPlayerAbortParams, ActPlayerAimParams, ActPlayerClimbParams,
        ActPlayerCrouchParams, ActPlayerDigParams, ActPlayerEjectParams, ActPlayerFireParams, ActPlayerJetParams,
        ActPlayerJumpParams, ActPlayerMoveParams, ActPlayerReloadParams, ActPlayerResetParams,
        ActPlayerSelectItemParams, ActPlayerSharpAimParams, InspectActorParams, InspectEquipmentParams,
        ObserveActorParams, ObserveOnceParams, ObserveSubscribeParams, RunBundleWriteParams, RunForTicksParams,
        ScenarioLoadParams, StepParams, SystemShutdownParams,
    },
    schemas::{SCHEMA_VERSION, SCHEMA_VERSION_MIN},
    state::{ControlEnvelopeStatus, ObserveFrame, ObserveSettings},
    Settings,
};

#[derive(Debug, Clone, Copy)]
pub struct ControlServerConfig {
    pub bind: SocketAddr,
    pub heartbeat: Duration,
    /// Maximum Hz the server will allow `observe.subscribe` to request.
    /// Bound to the engine's tick rate at server-construction time.
    pub max_observe_hz: u32,
}

impl Default for ControlServerConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:17890".parse().expect("static loopback bind parses"),
            heartbeat: Duration::from_secs(5),
            max_observe_hz: 240,
        }
    }
}

/// M4A focus traversal direction. Drives `act.input.focus`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusDirection {
    Next,
    Prev,
    Set(String),
    Clear,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SettingsPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_scale: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub high_contrast: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub captions: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reduced_motion: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reduced_shake: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reduced_flash: Option<bool>,
    /// M4A: ACC-A-05 hold-to-press alternative.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hold_to_confirm: Option<bool>,
    /// M4A: hold threshold in milliseconds (50..2000).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hold_threshold_ms: Option<u32>,
    /// M4A: ACC-A-05 remap toggle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_remap_enabled: Option<bool>,
    /// M4A: ACC-A-05 per-action key binding overrides. When `Some(...)` the
    /// patch REPLACES the entire table (not merged) so a remap UI clearing
    /// every binding can ship the empty map. When `None`, the existing
    /// table is preserved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_bindings: Option<std::collections::BTreeMap<String, String>>,
    /// **M1 / DR-055**: camera shake reduction. Clamped to `[0, 1]`.
    /// `1.0` = no shake (accessibility floor).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reduce_camera_shake_pct: Option<f32>,
    /// **M1**: tick-rate observable. Clamped to `>= 1`. The engine does not
    /// live-retick on patch; this mirrors the engine's configured tick rate
    /// so cfctl `observe.settings` is round-trippable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_rate_hz: Option<u32>,
    /// **M1 Gap F1**: configurable feel cvars. All values must be finite
    /// and (where applicable) positive; `apply_settings_patch` rejects
    /// invalid patches via `validation_error`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accel: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub friction: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gravity: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jump_force: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recoil_decay_per_tick: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sharp_aim_build_ticks: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub walk_threshold: Option<f32>,
    /// **M1.5 G6**: AI difficulty preset id. Valid values:
    /// `"cakewalk" | "tough_crowd" | "veteran"`. The engine looks up the
    /// preset in `cf-ai::DifficultyPreset::builtin(id)` and applies it to
    /// every `ReactiveGuard` in the world.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai_difficulty: Option<String>,
    /// **M1.5**: AI debug overlay. `true` raises the floating intent label
    /// above every reactive guard; `false` hides it. Defaults to current
    /// state when omitted (Option-based settings patch).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai_debug: Option<bool>,
}

impl SettingsPatch {
    pub fn is_empty(&self) -> bool {
        self.ui_scale.is_none()
            && self.high_contrast.is_none()
            && self.captions.is_none()
            && self.reduced_motion.is_none()
            && self.reduced_shake.is_none()
            && self.reduced_flash.is_none()
            && self.hold_to_confirm.is_none()
            && self.hold_threshold_ms.is_none()
            && self.key_remap_enabled.is_none()
            && self.key_bindings.is_none()
            && self.reduce_camera_shake_pct.is_none()
            && self.tick_rate_hz.is_none()
            && self.accel.is_none()
            && self.friction.is_none()
            && self.gravity.is_none()
            && self.jump_force.is_none()
            && self.recoil_decay_per_tick.is_none()
            && self.sharp_aim_build_ticks.is_none()
            && self.walk_threshold.is_none()
            && self.ai_difficulty.is_none()
            && self.ai_debug.is_none()
    }

    pub fn validation_error(&self) -> Option<String> {
        // M1 Gap F2: feel cvars must be finite, sane values.
        let positive = |label: &'static str, v: Option<f32>| -> Option<String> {
            v.and_then(|x| {
                if !x.is_finite() {
                    Some(format!("{label}_must_be_finite"))
                } else if x < 0.0 {
                    Some(format!("{label}_must_be_non_negative"))
                } else {
                    None
                }
            })
        };
        if let Some(reason) = positive("accel", self.accel) {
            return Some(reason);
        }
        if let Some(reason) = positive("friction", self.friction) {
            return Some(reason);
        }
        if let Some(reason) = positive("jump_force", self.jump_force) {
            return Some(reason);
        }
        if let Some(reason) = positive("recoil_decay_per_tick", self.recoil_decay_per_tick) {
            return Some(reason);
        }
        if let Some(reason) = positive("walk_threshold", self.walk_threshold) {
            return Some(reason);
        }
        if let Some(v) = self.gravity {
            // Gravity must be finite; sign is negotiable (negative = pulls
            // down; positive = anti-gravity test mode). Reject only NaN/Inf.
            if !v.is_finite() {
                return Some("gravity_must_be_finite".to_string());
            }
        }
        if let Some(v) = self.sharp_aim_build_ticks {
            if v == 0 {
                return Some("sharp_aim_build_ticks_must_be_positive".to_string());
            }
        }
        if let Some(v) = self.reduce_camera_shake_pct {
            if !v.is_finite() {
                return Some("reduce_camera_shake_pct_must_be_finite".to_string());
            }
        }
        if let Some(id) = self.ai_difficulty.as_deref() {
            if cf_ai::DifficultyPreset::builtin(id).is_none() {
                return Some(format!("ai_difficulty_unknown: {id}"));
            }
        }
        self.key_bindings
            .as_ref()
            .and_then(|bindings| crate::settings::validate_key_bindings(bindings).err())
    }
}

#[derive(Debug, Clone)]
pub enum ControlCommand {
    ScenarioLoad {
        scenario: String,
        seed: Option<u64>,
    },
    ScenarioReset,
    Pause,
    Resume,
    Step {
        ticks: u64,
    },
    RunForTicks {
        ticks: u64,
        write_run_bundle: bool,
    },
    ActPlayerMove {
        x: f32,
        y: f32,
        source: IntentSource,
    },
    ActPlayerJump {
        source: IntentSource,
    },
    ActPlayerAim {
        x: f32,
        y: f32,
        source: IntentSource,
    },
    ActPlayerFire {
        pressed: bool,
        source: IntentSource,
    },
    ActPlayerReload {
        source: IntentSource,
    },
    ActPlayerSelectItem {
        slot: u32,
        source: IntentSource,
    },
    ActPlayerReset {
        source: IntentSource,
    },
    /// M1.5: dig the soft-breach strip in front of the player. `target` is an
    /// optional explicit breach id; `None` => pick the nearest in-range strip.
    ActPlayerDig {
        target: Option<String>,
        source: IntentSource,
    },
    /// M4A: ACC-A-04 keyboard/controller focus traversal.
    /// `direction = "next" | "prev" | "set:<node_id>" | "clear"`.
    /// Drives the canonical `HUD_FOCUSABLE_NODES` cursor in the engine; the
    /// new focus state surfaces in `observe.accessibility.focused_node` +
    /// `focus_cycle`. cf-app's keyboard layer + cfctl + cf-e2e all dispatch
    /// through this same path.
    ActInputFocus {
        direction: FocusDirection,
        source: IntentSource,
    },
    /// **M5**: toggle the player actor's crouch stance.
    ActPlayerCrouch {
        active: bool,
        source: IntentSource,
    },
    /// **M5**: toggle the player actor's climb intent (placeholder cue; M5.5
    /// owns physical climb resolution).
    ActPlayerClimb {
        active: bool,
        source: IntentSource,
    },
    /// **M5**: toggle the player actor's jet thrust (requires Jet module
    /// nominal/degraded — Warning + Failed reject).
    ActPlayerJet {
        active: bool,
        source: IntentSource,
    },
    /// **M5**: trigger the chassis eject sequence.
    ActPlayerEject {
        source: IntentSource,
    },
    /// **M5**: repair a chassis zone (`zone` is `head | torso | arm_left | ...`).
    /// `reason` carries the operator label (`field_kit`, `repair_drone`, etc.).
    ActChassisRepair {
        zone: Option<String>,
        module_id: Option<String>,
        reason: String,
        source: IntentSource,
    },
    /// **M5**: salvage a wrecked chassis. Pulls surviving modules into
    /// `chassis.salvaged_modules`.
    ActChassisSalvage {
        reason: String,
        source: IntentSource,
    },
    /// **M5**: manually clear a weapon jam.
    ActChassisClearJam {
        source: IntentSource,
    },
    /// **M1**: sticky sharp-aim hold (CCCP AHuman.cpp:1779). `active=true`
    /// asks the sim to build `sharp_aim_progress`; `active=false` releases.
    ActPlayerSharpAim {
        active: bool,
        source: IntentSource,
    },
    /// **M1 / Gap S3**: stub for M1.5 mission abort. M1 rejects with
    /// `unsupported_in_m1`; M1.5 swaps in real abort logic without rewiring
    /// the cfctl surface.
    ActPlayerAbort {
        source: IntentSource,
    },
    /// **M1.5**: pause mission objective progress + timer (tutorial-modal
    /// pause path). Emits `mission.objective_paused`.
    ActMissionPause {
        source: IntentSource,
    },
    /// **M1.5**: resume after pause. Emits `mission.objective_resumed`.
    ActMissionResume {
        source: IntentSource,
    },
    /// **M1 / Gap D1**: UI tells the engine an overlay (settings panel,
    /// debrief prompt, future pause menu) has captured input. While
    /// captured, all `act.player.*` commands are rejected with
    /// `controls_captured` and the CONTROLS CAPTURED HUD zone surfaces.
    ActInputCaptureControls {
        captured: bool,
        capturer: Option<String>,
        source: IntentSource,
    },
    /// **M2**: cycle / set the material overlay mode.
    ActToggleMaterialOverlay {
        mode: Option<String>,
        source: IntentSource,
    },
    SettingsSet {
        changes: SettingsPatch,
    },
    RunBundleWrite {
        id_override: Option<String>,
    },
    Shutdown {
        write_run_bundle: bool,
    },
}

/// Trait that the engine implements so the server stays decoupled from `cf-app`.
#[async_trait::async_trait]
pub trait EngineHandle: Send + Sync + 'static {
    async fn snapshot(&self, filter: Option<&str>) -> ObserveFrame;
    async fn settings_snapshot(&self) -> Settings;
    async fn dispatch(&self, command: ControlCommand) -> CommandResult;
    /// **M1 Gap A5**: return the full `RifleSpec` for `preset_id` (firing
    /// profile + AI hints + particle/tracer metadata). Default impl returns
    /// `None` for handlers that don't have an equipment registry.
    async fn inspect_equipment(&self, _preset_id: &str) -> Option<serde_json::Value> {
        None
    }
    /// **M1 Gap B3**: return the `ActorView` for a specific actor (or the
    /// player when `actor_id` is None). Default returns `None`.
    async fn observe_actor(&self, _actor_id: Option<u64>) -> Option<serde_json::Value> {
        None
    }
    /// **M1 Gap B3**: return the actor view plus its last `n` actor-category
    /// events. Default returns `None`.
    async fn inspect_actor(&self, _target: Option<&str>, _last_n_events: usize) -> Option<serde_json::Value> {
        None
    }
    /// **M2**: return the full chunk material grid (RLE-friendly Vec) for the
    /// requested chunk coord. Default returns `None`.
    async fn inspect_terrain_chunk(&self, _cx: i32, _cy: i32) -> Option<serde_json::Value> {
        None
    }
    /// **M2**: return the full MaterialDef for the requested id. Default
    /// returns `None`.
    async fn inspect_material(&self, _id: u8) -> Option<serde_json::Value> {
        None
    }
}

#[derive(Debug, Clone)]
pub struct CommandResult {
    pub status: ControlEnvelopeStatus,
    pub effective_tick: u64,
    pub reason: Option<String>,
}

impl CommandResult {
    pub fn accepted(effective_tick: u64) -> Self {
        Self {
            status: ControlEnvelopeStatus::Accepted,
            effective_tick,
            reason: None,
        }
    }
    pub fn rejected(reason: impl Into<String>, effective_tick: u64) -> Self {
        Self {
            status: ControlEnvelopeStatus::Rejected,
            effective_tick,
            reason: Some(reason.into()),
        }
    }
}

pub struct ControlServer {
    config: ControlServerConfig,
}

impl ControlServer {
    pub fn new(config: ControlServerConfig) -> Self {
        Self { config }
    }

    /// Bind + serve forever, exiting cleanly on Ctrl-C / SIGINT.
    ///
    /// For programmatic shutdown (e.g., from the engine's `system.shutdown`
    /// command, or from tests) use [`Self::serve_with_shutdown`].
    ///
    /// Uses `tokio::select!` to race the serve future against `ctrl_c()`
    /// instead of spawning a dedicated ctrl-c handler task. Spawning a
    /// task would leak it across `serve()` lifecycles (the spawned task
    /// holds a `ShutdownSignal` clone that never fires after a programmatic
    /// shutdown returns), which is benign in a single-shot main() but adds
    /// up if `serve()` is called repeatedly within one tokio runtime
    /// (PR #26 review Devin Info).
    pub async fn serve<E: EngineHandle>(self, engine: Arc<E>) -> std::io::Result<()> {
        let (shutdown_tx, shutdown_rx) = shutdown_signal();
        let serve_fut = self.serve_with_shutdown(engine, shutdown_rx);
        tokio::pin!(serve_fut);

        select! {
            // Branch 1: serve_with_shutdown returns (programmatic shutdown
            // via system.shutdown or accept-loop error) — propagate result.
            result = &mut serve_fut => {
                drop(shutdown_tx);
                result
            }
            // Branch 2: Ctrl-C / SIGINT received — trigger sticky shutdown
            // and then poll serve_with_shutdown to completion so in-flight
            // connections drain cleanly via the per-connection observation
            // loops' shutdown handlers.
            _ = tokio::signal::ctrl_c() => {
                tracing::info!(target: "cf::ctl", "ctrl-c received; shutting down control server");
                trigger_shutdown(&shutdown_tx);
                let result = (&mut serve_fut).await;
                drop(shutdown_tx);
                result
            }
        }
    }

    /// Bind + serve until `shutdown_rx` observes `true`. The accept loop
    /// exits gracefully and in-flight connections receive the same shutdown
    /// signal so their per-connection observation loops can stop cleanly.
    ///
    /// Delegates to [`Self::serve_listener_with_shutdown`] after binding to
    /// keep the accept-loop logic in exactly one place (PR #26 round-4 review
    /// Bugbot Low: avoid duplicate `select!` accept loops drifting out of
    /// sync if a future bug fix touches one but not the other).
    pub async fn serve_with_shutdown<E: EngineHandle>(
        self,
        engine: Arc<E>,
        shutdown_rx: ShutdownReceiver,
    ) -> std::io::Result<()> {
        let listener = TcpListener::bind(self.config.bind).await?;
        tracing::info!(target: "cf::ctl", bind = %self.config.bind, "control server listening");
        let max_hz = self.config.max_observe_hz;
        Self::serve_listener_with_shutdown(listener, engine, max_hz, shutdown_rx).await
    }

    /// Bind without serving so callers can inspect the actual port (useful for
    /// tests with `127.0.0.1:0` ephemeral binding).
    pub async fn bind(self) -> std::io::Result<(TcpListener, ControlServerConfig)> {
        let listener = TcpListener::bind(self.config.bind).await?;
        let mut cfg = self.config;
        cfg.bind = listener.local_addr()?;
        Ok((listener, cfg))
    }

    /// Serve on an already-bound listener. Convenience wrapper that creates
    /// its own shutdown signal but never triggers it; callers wanting real
    /// shutdown control should use [`Self::serve_listener_with_shutdown`].
    pub async fn serve_listener<E: EngineHandle>(
        listener: TcpListener,
        engine: Arc<E>,
        max_observe_hz: u32,
    ) -> std::io::Result<()> {
        let (shutdown_tx, shutdown_rx) = shutdown_signal();
        let result = Self::serve_listener_with_shutdown(listener, engine, max_observe_hz, shutdown_rx).await;
        drop(shutdown_tx);
        result
    }

    pub async fn serve_listener_with_shutdown<E: EngineHandle>(
        listener: TcpListener,
        engine: Arc<E>,
        max_observe_hz: u32,
        mut shutdown_rx: ShutdownReceiver,
    ) -> std::io::Result<()> {
        loop {
            select! {
                accept = listener.accept() => {
                    let (stream, peer) = accept?;
                    let engine = engine.clone();
                    let max_hz = max_observe_hz;
                    let connection_shutdown = shutdown_rx.clone();
                    tokio::spawn(async move {
                        if let Err(err) = handle_connection(stream, peer, engine, max_hz, connection_shutdown).await {
                            tracing::warn!(target: "cf::ctl", %peer, error = %err, "control connection ended with error");
                        }
                    });
                }
                _ = wait_for_shutdown(&mut shutdown_rx) => {
                    tracing::info!(target: "cf::ctl", "shutdown signal received; control server stopping accept loop");
                    return Ok(());
                }
            }
        }
    }
}

async fn handle_connection<E: EngineHandle>(
    stream: tokio::net::TcpStream,
    peer: SocketAddr,
    engine: Arc<E>,
    max_observe_hz: u32,
    server_shutdown: ShutdownReceiver,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ws_stream = accept_async(stream).await?;
    tracing::info!(target: "cf::ctl", %peer, "control connection accepted");
    let (sink, mut source) = ws_stream.split();
    let sink = Arc::new(Mutex::new(sink));
    // Bounded channel applies backpressure when the client cannot keep up:
    // producers `await` on `send()` and the observation loop naturally paces
    // itself to the slow consumer. Prevents the unbounded-memory growth that
    // a naive `mpsc::unbounded_channel()` would allow under sustained load.
    let (out_tx, mut out_rx) = mpsc::channel::<Message>(OUTBOUND_CHANNEL_CAPACITY);

    // Per-connection sticky shutdown signal (watch::<bool>). Triggered when:
    //  - the source loop exits (client disconnected or error),
    //  - the server-wide shutdown signal observes `true` (Ctrl-C,
    //    system.shutdown).
    // Used by the observation loop to break out of its work cycle even if it
    // is mid-`engine.snapshot().await` or `out_tx.send().await` when shutdown
    // fires — `watch::<bool>` is sticky so the signal cannot be lost in a
    // race window the way `Notify::notify_waiters()` was (PR #26 review
    // root-cause).
    let (connection_shutdown_tx, connection_shutdown_rx) = shutdown_signal();

    let sink_for_writer = sink.clone();
    let writer_handle = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            let mut guard = sink_for_writer.lock().await;
            if guard.send(msg).await.is_err() {
                break;
            }
        }
    });

    let subscribe_hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
    let subscribe_filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let observation_handle = spawn_observation_loop(
        engine.clone(),
        out_tx.clone(),
        subscribe_hz.clone(),
        subscribe_filter.clone(),
        connection_shutdown_rx.clone(),
    );

    let mut server_shutdown = server_shutdown;
    // Inner `if .. is_err() { break }` patterns inside the match arms below
    // can't be collapsed into pattern guards because `payload` would have to
    // be moved across the guard boundary (Rust 2021; let-chains require
    // edition 2024).
    #[allow(clippy::collapsible_match, clippy::collapsible_if)]
    loop {
        select! {
            maybe_message = source.next() => {
                let Some(message) = maybe_message else { break; };
                let message = message?;
                match message {
                    Message::Text(text) => {
                        let response =
                            process_request(&text, engine.as_ref(), &subscribe_hz, &subscribe_filter, max_observe_hz).await;
                        if let Some(payload) = response {
                            // Bounded `send().await` applies backpressure if
                            // the client is slow. We race the send against the
                            // shutdown signal so a stalled client cannot wedge
                            // the connection handler past a shutdown
                            // (addresses the secondary backpressure-blocks-
                            // shutdown-detection finding in the PR #26 review).
                            select! {
                                send_result = out_tx.send(Message::Text(payload.into())) => {
                                    if send_result.is_err() { break; }
                                }
                                _ = wait_for_shutdown(&mut server_shutdown) => break,
                            }
                        }
                    }
                    Message::Ping(payload) => {
                        select! {
                            send_result = out_tx.send(Message::Pong(payload)) => {
                                if send_result.is_err() { break; }
                            }
                            _ = wait_for_shutdown(&mut server_shutdown) => break,
                        }
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
            _ = wait_for_shutdown(&mut server_shutdown) => {
                tracing::info!(target: "cf::ctl", %peer, "server shutdown signal received; closing connection");
                break;
            }
        }
    }

    // Signal observation loop to stop (sticky watch — even if the loop is
    // mid-await, the next time it polls the receiver it observes `true`),
    // then drop the producer sender so the writer task drains and exits.
    trigger_shutdown(&connection_shutdown_tx);
    drop(out_tx);
    let _ = observation_handle.await;
    let _ = writer_handle.await;
    tracing::info!(target: "cf::ctl", %peer, "control connection closed");
    Ok(())
}

fn spawn_observation_loop<E: EngineHandle>(
    engine: Arc<E>,
    out_tx: mpsc::Sender<Message>,
    subscribe_hz: Arc<Mutex<Option<u32>>>,
    subscribe_filter: Arc<Mutex<Option<String>>>,
    mut shutdown: ShutdownReceiver,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            // Sticky shutdown check: even if the signal fired mid-iteration
            // while we were awaiting a snapshot or a send below, the watch
            // sees it on the very next poll. This is the fix for the
            // PR #26 review's 🔴 finding — `Notify::notify_waiters()` would
            // silently drop the signal if no `.notified().await` was active
            // at notify time, leaving this loop spinning forever.
            if *shutdown.borrow() {
                return;
            }
            let hz_now = { *subscribe_hz.lock().await };
            let sleep_duration = match hz_now {
                None => Duration::from_millis(50),
                Some(hz) => {
                    let hz = hz.max(1);
                    let filter = subscribe_filter.lock().await.clone();
                    // Race the snapshot computation against shutdown so a
                    // long-running snapshot doesn't hold up exit.
                    let frame = select! {
                        f = engine.snapshot(filter.as_deref()) => f,
                        _ = wait_for_shutdown(&mut shutdown) => return,
                    };
                    let notification = JsonRpcNotification {
                        jsonrpc: "2.0".to_string(),
                        method: METHOD_OBSERVE_FRAME.to_string(),
                        params: serde_json::to_value(&frame).unwrap_or(serde_json::Value::Null),
                    };
                    if let Ok(payload) = serde_json::to_string(&notification) {
                        // Race the bounded send against shutdown so a slow
                        // client backpressuring the channel can't keep this
                        // loop alive past shutdown.
                        select! {
                            send_result = out_tx.send(Message::Text(payload.into())) => {
                                if send_result.is_err() { return; }
                            }
                            _ = wait_for_shutdown(&mut shutdown) => return,
                        }
                    }
                    Duration::from_millis(1000 / u64::from(hz))
                }
            };
            // Race the configured sleep against the connection shutdown
            // signal so the loop exits within milliseconds when the
            // connection closes.
            select! {
                _ = tokio::time::sleep(sleep_duration) => {}
                _ = wait_for_shutdown(&mut shutdown) => return,
            }
        }
    })
}

async fn process_request<E: EngineHandle>(
    text: &str,
    engine: &E,
    subscribe_hz: &Arc<Mutex<Option<u32>>>,
    subscribe_filter: &Arc<Mutex<Option<String>>>,
    max_observe_hz: u32,
) -> Option<String> {
    let request: JsonRpcRequest = match serde_json::from_str(text) {
        Ok(r) => r,
        Err(err) => {
            tracing::warn!(target: "cf::ctl", error = %err, "invalid jsonrpc request");
            return Some(error_response(
                JsonRpcId::Null,
                error_codes::INVALID_PARAMS,
                "InvalidRequest",
                json!({"reason": err.to_string()}),
            ));
        }
    };
    if request.jsonrpc != "2.0" {
        return Some(error_response(
            request.id,
            error_codes::INVALID_PARAMS,
            "InvalidRequest",
            json!({"reason": "jsonrpc must be \"2.0\""}),
        ));
    }

    let method = request.method.clone();
    let params = request.params.clone();
    if let Err(err) = check_schema_version(&params) {
        return Some(error_response(
            request.id,
            error_codes::INVALID_PARAMS,
            "InvalidParams",
            err,
        ));
    }

    match method.as_str() {
        "scenario.load" => {
            let p: ScenarioLoadParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ScenarioLoad {
                    scenario: p.scenario,
                    seed: p.seed,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "scenario.reset" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let result = engine.dispatch(ControlCommand::ScenarioReset).await;
            Some(ack_response(request.id, &result))
        }
        "sim.pause" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let result = engine.dispatch(ControlCommand::Pause).await;
            Some(ack_response(request.id, &result))
        }
        "sim.resume" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let result = engine.dispatch(ControlCommand::Resume).await;
            Some(ack_response(request.id, &result))
        }
        "sim.step" => {
            let p: StepParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if p.ticks == 0 {
                return Some(invalid_param_reason(request.id, "ticks_must_be_positive"));
            }
            let result = engine.dispatch(ControlCommand::Step { ticks: p.ticks }).await;
            Some(ack_response(request.id, &result))
        }
        "sim.run_for_ticks" => {
            let p: RunForTicksParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if p.ticks == 0 {
                return Some(invalid_param_reason(request.id, "ticks_must_be_positive"));
            }
            let result = engine
                .dispatch(ControlCommand::RunForTicks {
                    ticks: p.ticks,
                    write_run_bundle: p.write_run_bundle.unwrap_or(false),
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "observe.once" => {
            let p: ObserveOnceParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if p.filter.is_some() {
                return Some(invalid_param_reason(request.id, "observe_filter_not_supported_in_m0"));
            }
            let frame = engine.snapshot(p.filter.as_deref()).await;
            Some(success_response(
                request.id,
                serde_json::to_value(&frame).unwrap_or(serde_json::Value::Null),
            ))
        }
        "observe.subscribe" => {
            let p: ObserveSubscribeParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if p.filter.is_some() {
                return Some(invalid_param_reason(request.id, "observe_filter_not_supported_in_m0"));
            }
            let hz = p.hz.unwrap_or(10);
            if hz == 0 || hz > max_observe_hz {
                return Some(error_response(
                    request.id,
                    error_codes::INVALID_PARAMS,
                    "InvalidParams",
                    json!({
                        "reason": "observe_hz_out_of_range",
                        "min_hz": 1,
                        "max_hz": max_observe_hz,
                    }),
                ));
            }
            *subscribe_hz.lock().await = Some(hz);
            *subscribe_filter.lock().await = p.filter;
            Some(ack_response(request.id, &CommandResult::accepted(0)))
        }
        "observe.unsubscribe" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            *subscribe_hz.lock().await = None;
            *subscribe_filter.lock().await = None;
            Some(ack_response(request.id, &CommandResult::accepted(0)))
        }
        "observe.settings" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let settings = engine.settings_snapshot().await;
            let view = ObserveSettings {
                schema_version: SCHEMA_VERSION,
                settings,
            };
            Some(success_response(
                request.id,
                serde_json::to_value(&view).unwrap_or(serde_json::Value::Null),
            ))
        }
        "act.player.move" => {
            let p: ActPlayerMoveParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if !p.x.is_finite() || !p.y.is_finite() {
                return Some(invalid_param_reason(request.id, "axis_must_be_finite"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerMove {
                    x: p.x,
                    y: p.y,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.jump" => {
            let _p: ActPlayerJumpParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerJump {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.aim" => {
            let p: ActPlayerAimParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if !p.x.is_finite() || !p.y.is_finite() {
                return Some(invalid_param_reason(request.id, "aim_must_be_finite"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerAim {
                    x: p.x,
                    y: p.y,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.fire" => {
            let p: ActPlayerFireParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerFire {
                    pressed: p.pressed,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.reload" => {
            let _p: ActPlayerReloadParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerReload {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.select_item" => {
            let p: ActPlayerSelectItemParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerSelectItem {
                    slot: p.slot,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.reset" => {
            let _p: ActPlayerResetParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerReset {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.dig" => {
            let p: ActPlayerDigParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerDig {
                    target: p.target,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.crouch" => {
            let p: ActPlayerCrouchParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerCrouch {
                    active: p.active,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.climb" => {
            let p: ActPlayerClimbParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerClimb {
                    active: p.active,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.jet" => {
            let p: ActPlayerJetParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerJet {
                    active: p.active,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.eject" => {
            let _p: ActPlayerEjectParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerEject {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.chassis.repair" => {
            let p: ActChassisRepairParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if p.zone.is_none() && p.module_id.is_none() {
                return Some(invalid_param_reason(
                    request.id,
                    "chassis_repair_requires_zone_or_module_id",
                ));
            }
            let result = engine
                .dispatch(ControlCommand::ActChassisRepair {
                    zone: p.zone,
                    module_id: p.module_id,
                    reason: p.reason,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.chassis.salvage" => {
            let p: ActChassisSalvageParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActChassisSalvage {
                    reason: p.reason,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.chassis.clear_jam" => {
            let _p: ActChassisClearJamParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActChassisClearJam {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.sharp_aim" => {
            let p: ActPlayerSharpAimParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerSharpAim {
                    active: p.active,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.abort" => {
            let _p: ActPlayerAbortParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerAbort {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.mission.pause" => {
            let _p: ActMissionPauseParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActMissionPause {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.mission.resume" => {
            let _p: ActMissionResumeParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActMissionResume {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.input.capture_controls" => {
            let p: ActInputCaptureControlsParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActInputCaptureControls {
                    captured: p.captured,
                    capturer: p.capturer,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "inspect.equipment" => {
            let p: InspectEquipmentParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.inspect_equipment(&p.preset_id).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "unknown_preset_id")),
            }
        }
        "inspect.terrain.chunk" => {
            let p: crate::schemas::InspectTerrainChunkParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.inspect_terrain_chunk(p.x, p.y).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "terrain_unavailable")),
            }
        }
        "inspect.material" => {
            let p: crate::schemas::InspectMaterialParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.inspect_material(p.id).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "unknown_material_id")),
            }
        }
        "act.player.toggle_material_overlay" => {
            let p: crate::schemas::ActToggleMaterialOverlayParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActToggleMaterialOverlay {
                    mode: p.mode,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "observe.actor" => {
            let p: ObserveActorParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.observe_actor(p.actor_id).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_player_actor")),
            }
        }
        "inspect.actor" => {
            let p: InspectActorParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.inspect_actor(p.target.as_deref(), 30).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_player_actor")),
            }
        }
        "act.input.focus" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct FocusParams {
                schema_version: u32,
                direction: String,
                #[serde(default, skip_serializing_if = "Option::is_none")]
                node: Option<String>,
            }
            let p: FocusParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let direction = match p.direction.as_str() {
                "next" => FocusDirection::Next,
                "prev" => FocusDirection::Prev,
                "clear" => FocusDirection::Clear,
                "set" => match p.node {
                    Some(n) if !n.is_empty() => FocusDirection::Set(n),
                    _ => return Some(invalid_param_reason(request.id, "focus_set_requires_node")),
                },
                other => {
                    let reason = format!("focus_unknown_direction:{other}");
                    return Some(invalid_param_reason(request.id, &reason));
                }
            };
            let result = engine
                .dispatch(ControlCommand::ActInputFocus {
                    direction,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.settings.set" => {
            // Accept either a flat object {schema_version, ui_scale, ...} or a wrapped {schema_version, patch:{...}}.
            let patch_value = if params.get("patch").is_some() {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct WrappedPatch {
                    schema_version: u32,
                    patch: SettingsPatch,
                }
                let wrapped: WrappedPatch = match serde_json::from_value(params) {
                    Ok(v) => v,
                    Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
                };
                let _ = wrapped.schema_version;
                serde_json::to_value(wrapped.patch).unwrap_or(serde_json::Value::Null)
            } else {
                let mut p = params.clone();
                if let Some(o) = p.as_object_mut() {
                    o.remove("schema_version");
                }
                p
            };
            let patch: SettingsPatch = match serde_json::from_value(patch_value) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if patch.is_empty() {
                return Some(invalid_param_reason(request.id, "settings_patch_empty"));
            }
            if let Some(reason) = patch.validation_error() {
                return Some(invalid_param_reason(request.id, &reason));
            }
            let result = engine.dispatch(ControlCommand::SettingsSet { changes: patch }).await;
            Some(ack_response(request.id, &result))
        }
        "runbundle.write" => {
            let p: RunBundleWriteParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if let Some(ref id) = p.id_override {
                if id.contains("..") || id.contains('/') || id.contains('\\') {
                    return Some(invalid_param_reason(request.id, "path_traversal_rejected"));
                }
            }
            if p.id_override.is_some() {
                return Some(invalid_param_reason(
                    request.id,
                    "runbundle_id_override_not_supported_in_m0",
                ));
            }
            let result = engine
                .dispatch(ControlCommand::RunBundleWrite {
                    id_override: p.id_override,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "system.shutdown" => {
            let p: SystemShutdownParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::Shutdown {
                    write_run_bundle: p.write_run_bundle.unwrap_or(false),
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        _ => Some(error_response(
            request.id,
            error_codes::METHOD_NOT_FOUND,
            "MethodNotFound",
            json!({"method": method, "fix_hint": "see spec/ai-control-observability-layer.md M0 method catalog"}),
        )),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SchemaOnlyParams {
    schema_version: u32,
}

fn parse_schema_only(id: JsonRpcId, params: serde_json::Value) -> Result<(), String> {
    let parsed: SchemaOnlyParams =
        serde_json::from_value(params).map_err(|err| missing_param_error(id, &err.to_string()))?;
    let _ = parsed.schema_version;
    Ok(())
}

/// `schema_version` is MANDATORY on every M0 client→server JSON-RPC request. Missing,
/// non-numeric, or mismatched values reject with `-32602` (`InvalidParams`). There are no
/// notification methods in the M0 catalog; if a future milestone adds one, document it
/// here as an explicit exception.
fn check_schema_version(params: &serde_json::Value) -> Result<(), serde_json::Value> {
    let obj = match params.as_object() {
        Some(o) => o,
        None => {
            return Err(json!({
                "reason": "schema_version_missing",
                "server_version": SCHEMA_VERSION,
                "fix_hint": format!("All M0 control methods require an object `params` containing `schema_version: {SCHEMA_VERSION}`."),
            }));
        }
    };
    match obj.get("schema_version").and_then(|v| v.as_u64()) {
        None => Err(json!({
            "reason": "schema_version_missing",
            "server_version": SCHEMA_VERSION,
            "fix_hint": format!("All M0 control methods require `params.schema_version: {SCHEMA_VERSION}`."),
        })),
        Some(v) if (SCHEMA_VERSION_MIN..=SCHEMA_VERSION).contains(&(v as u32)) => Ok(()),
        Some(other) => Err(json!({
            "reason": "schema_version_mismatch",
            "server_version": SCHEMA_VERSION,
            "server_version_min": SCHEMA_VERSION_MIN,
            "client_version": other,
            "fix_hint": format!("Server accepts schema_version {}..{}; upgrade cfctl or pin within range", SCHEMA_VERSION_MIN, SCHEMA_VERSION),
        })),
    }
}

fn ack_response(id: JsonRpcId, result: &CommandResult) -> String {
    let (status_str, error_obj) = match result.status {
        ControlEnvelopeStatus::Accepted => ("accepted", None),
        ControlEnvelopeStatus::Queued => ("queued", None),
        ControlEnvelopeStatus::Rejected => (
            "rejected",
            Some(JsonRpcError {
                code: error_codes::COMMAND_REJECTED,
                message: "command_rejected".to_string(),
                data: Some(json!({
                    "reason": result.reason.clone().unwrap_or_else(|| "unknown".to_string()),
                    "tick": result.effective_tick
                })),
            }),
        ),
    };
    if let Some(error) = error_obj {
        let resp = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        };
        return serde_json::to_string(&resp).unwrap_or_default();
    }
    let payload = json!({
        "schema_version": SCHEMA_VERSION,
        "status": status_str,
        "effective_tick": result.effective_tick,
        "reason": result.reason
    });
    success_response(id, payload)
}

fn success_response(id: JsonRpcId, value: serde_json::Value) -> String {
    let resp = JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(value),
        error: None,
    };
    serde_json::to_string(&resp).unwrap_or_default()
}

fn error_response(id: JsonRpcId, code: i32, message: &str, data: serde_json::Value) -> String {
    let resp = JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(JsonRpcError {
            code,
            message: message.to_string(),
            data: Some(data),
        }),
    };
    serde_json::to_string(&resp).unwrap_or_default()
}

fn missing_param_error(id: JsonRpcId, reason: &str) -> String {
    error_response(
        id,
        error_codes::INVALID_PARAMS,
        "InvalidParams",
        json!({"reason": reason, "fix_hint": "see spec/prototype-roadmap CLI Reference for the M0 method catalog"}),
    )
}

fn invalid_param_reason(id: JsonRpcId, reason: &str) -> String {
    error_response(
        id,
        error_codes::INVALID_PARAMS,
        "InvalidParams",
        json!({"reason": reason, "fix_hint": "see spec/prototype-roadmap CLI Reference for the M0 method catalog"}),
    )
}

pub use async_trait::async_trait;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::RunStatus;

    #[derive(Default)]
    struct StubEngine;

    #[async_trait::async_trait]
    impl EngineHandle for StubEngine {
        async fn snapshot(&self, _filter: Option<&str>) -> ObserveFrame {
            ObserveFrame {
                schema_version: SCHEMA_VERSION,
                run_id: "stub".to_string(),
                tick: 0,
                sim_time_ms: 0.0,
                run_status: RunStatus::Paused,
                scenario: "m0_blank".to_string(),
                events_since: 0,
                events: vec![],
                settings: ObserveSettings {
                    schema_version: SCHEMA_VERSION,
                    settings: Settings::default(),
                },
                actors: vec![],
                player_actor_id: None,
                mission: None,
                breaches: vec![],
                enemies: vec![],
                terrain: None,
                reactors: vec![],
                banners: vec![],
                captions: vec![],
                tool_validity: None,
                accessibility: crate::state::AccessibilityView::default(),
                controls_capture: crate::state::ControlsCaptureView::default(),
            }
        }
        async fn settings_snapshot(&self) -> Settings {
            Settings::default()
        }
        async fn dispatch(&self, _command: ControlCommand) -> CommandResult {
            CommandResult::accepted(0)
        }
    }

    #[tokio::test]
    async fn schema_mismatch_returns_invalid_params() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.player.move",
            "params": {"schema_version": 99, "x": 1.0}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let error = parsed.error.expect("schema mismatch produces an error");
        assert_eq!(error.code, error_codes::INVALID_PARAMS);
        let data = error.data.unwrap();
        assert_eq!(data.get("reason").unwrap(), "schema_version_mismatch");
        // The error reports the current server SCHEMA_VERSION; future M5+ bumps
        // must update through the constant, not a literal — keeps the contract
        // consistent across the test surface.
        assert_eq!(data.get("server_version").unwrap(), SCHEMA_VERSION);
        assert_eq!(data.get("client_version").unwrap(), 99);
    }

    /// M0.2-F2: Every M0 method must reject a request with missing `params.schema_version`.
    /// Pre-fix, several handlers (`act.settings.set`, `runbundle.write`, `system.shutdown`,
    /// `observe.subscribe/unsubscribe`, `observe.once`) silently defaulted when the field
    /// was absent. Now `check_schema_version` requires it before any handler runs.
    #[tokio::test]
    async fn missing_schema_version_rejects_every_m0_method() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let methods_with_params: &[(&str, serde_json::Value)] = &[
            ("scenario.load", json!({"scenario": "m0_blank"})),
            ("scenario.reset", json!({})),
            ("sim.pause", json!({})),
            ("sim.resume", json!({})),
            ("sim.step", json!({"ticks": 1})),
            ("sim.run_for_ticks", json!({"ticks": 30})),
            ("observe.once", json!({})),
            ("observe.subscribe", json!({"hz": 10})),
            ("observe.unsubscribe", json!({})),
            ("observe.settings", json!({})),
            ("act.player.move", json!({"x": 1.0, "y": 0.0})),
            ("act.settings.set", json!({"ui_scale": 2.0})),
            ("runbundle.write", json!({})),
            ("system.shutdown", json!({})),
        ];
        for (method, params) in methods_with_params {
            let req = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
            let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
                .await
                .unwrap();
            let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
            let error = parsed
                .error
                .unwrap_or_else(|| panic!("{method} must reject missing schema_version"));
            assert_eq!(
                error.code,
                error_codes::INVALID_PARAMS,
                "{method} must return -32602 (InvalidParams) on missing schema_version, got code {}",
                error.code
            );
            let data = error.data.unwrap();
            assert_eq!(
                data.get("reason").unwrap(),
                "schema_version_missing",
                "{method} returned wrong reason on missing schema_version"
            );
        }
    }

    /// M0.2-F2: A request that omits `params` entirely must also reject. (clap-driven
    /// JSON-RPC clients commonly send `{"method": "...", "id": ...}` without params for
    /// no-arg methods; the server must still demand schema_version.)
    #[tokio::test]
    async fn missing_params_object_rejects() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({"jsonrpc": "2.0", "id": 1, "method": "sim.pause"});
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let error = parsed.error.expect("missing params must reject");
        assert_eq!(error.code, error_codes::INVALID_PARAMS);
        assert_eq!(error.data.unwrap().get("reason").unwrap(), "schema_version_missing");
    }

    #[tokio::test]
    async fn unknown_method_returns_method_not_found() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.player.fly",
            "params": {"schema_version": 1}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let error = parsed.error.expect("unknown method returns error");
        assert_eq!(error.code, error_codes::METHOD_NOT_FOUND);
    }

    #[tokio::test]
    async fn observe_once_returns_frame() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "observe.once",
            "params": {"schema_version": SCHEMA_VERSION}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let result = parsed.result.expect("observe.once returns success");
        assert_eq!(result.get("scenario").unwrap(), "m0_blank");
        // observe.once returns an ObserveFrame whose schema_version field equals
        // the current server SCHEMA_VERSION constant; the test reads through the
        // constant so a future M5+ bump cannot silently drift the contract.
        assert_eq!(result.get("schema_version").unwrap(), SCHEMA_VERSION);
    }

    #[tokio::test]
    async fn settings_set_dispatches_patch() {
        struct CaptureEngine {
            patch: Mutex<Option<SettingsPatch>>,
        }
        #[async_trait::async_trait]
        impl EngineHandle for CaptureEngine {
            async fn snapshot(&self, _filter: Option<&str>) -> ObserveFrame {
                StubEngine.snapshot(_filter).await
            }
            async fn settings_snapshot(&self) -> Settings {
                Settings::default()
            }
            async fn dispatch(&self, command: ControlCommand) -> CommandResult {
                if let ControlCommand::SettingsSet { changes } = command {
                    *self.patch.lock().await = Some(changes);
                }
                CommandResult::accepted(0)
            }
        }
        let engine = CaptureEngine {
            patch: Mutex::new(None),
        };
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.settings.set",
            "params": {"schema_version": 1, "ui_scale": 1.5, "high_contrast": true}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        assert!(parsed.result.is_some());
        let captured = engine.patch.lock().await.clone().unwrap();
        assert_eq!(captured.ui_scale, Some(1.5));
        assert_eq!(captured.high_contrast, Some(true));
        assert_eq!(captured.captions, None);
    }

    /// Contract-integrity regression: every M0 handler must reject unknown fields instead
    /// of accepting a request whose extra data was silently ignored.
    #[tokio::test]
    async fn unknown_params_reject_every_m0_method() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let methods_with_params: &[(&str, serde_json::Value)] = &[
            (
                "scenario.load",
                json!({"schema_version": 1, "scenario": "m0_blank", "unexpected": true}),
            ),
            ("scenario.reset", json!({"schema_version": 1, "unexpected": true})),
            ("sim.pause", json!({"schema_version": 1, "unexpected": true})),
            ("sim.resume", json!({"schema_version": 1, "unexpected": true})),
            ("sim.step", json!({"schema_version": 1, "ticks": 1, "unexpected": true})),
            (
                "sim.run_for_ticks",
                json!({"schema_version": 1, "ticks": 30, "unexpected": true}),
            ),
            ("observe.once", json!({"schema_version": 1, "unexpected": true})),
            (
                "observe.subscribe",
                json!({"schema_version": 1, "hz": 10, "unexpected": true}),
            ),
            ("observe.unsubscribe", json!({"schema_version": 1, "unexpected": true})),
            ("observe.settings", json!({"schema_version": 1, "unexpected": true})),
            (
                "act.player.move",
                json!({"schema_version": 1, "x": 1.0, "unexpected": true}),
            ),
            (
                "act.settings.set",
                json!({"schema_version": 1, "ui_scale": 2.0, "unexpected": true}),
            ),
            ("runbundle.write", json!({"schema_version": 1, "unexpected": true})),
            ("system.shutdown", json!({"schema_version": 1, "unexpected": true})),
        ];
        for (method, params) in methods_with_params {
            let req = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
            let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
                .await
                .unwrap();
            let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
            let error = parsed
                .error
                .unwrap_or_else(|| panic!("{method} must reject unknown params"));
            assert_eq!(
                error.code,
                error_codes::INVALID_PARAMS,
                "{method} must reject unknown params"
            );
            assert!(
                parsed.result.is_none(),
                "{method} must not return success with unknown params"
            );
        }
    }

    #[tokio::test]
    async fn unsupported_m0_params_reject_before_dispatch() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let cases: &[(&str, serde_json::Value, &str)] = &[
            (
                "sim.step",
                json!({"schema_version": 1, "ticks": 0}),
                "ticks_must_be_positive",
            ),
            (
                "sim.run_for_ticks",
                json!({"schema_version": 1, "ticks": 0}),
                "ticks_must_be_positive",
            ),
            (
                "observe.once",
                json!({"schema_version": 1, "filter": "system"}),
                "observe_filter_not_supported_in_m0",
            ),
            (
                "observe.subscribe",
                json!({"schema_version": 1, "hz": 0}),
                "observe_hz_out_of_range",
            ),
            (
                "observe.subscribe",
                json!({"schema_version": 1, "hz": 241}),
                "observe_hz_out_of_range",
            ),
            (
                "observe.subscribe",
                json!({"schema_version": 1, "filter": "system"}),
                "observe_filter_not_supported_in_m0",
            ),
            ("act.settings.set", json!({"schema_version": 1}), "settings_patch_empty"),
            (
                "runbundle.write",
                json!({"schema_version": 1, "id_override": "manual"}),
                "runbundle_id_override_not_supported_in_m0",
            ),
        ];
        for (method, params, reason) in cases {
            let req = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
            let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
                .await
                .unwrap();
            let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
            let error = parsed.error.unwrap_or_else(|| panic!("{method} must reject {params}"));
            assert_eq!(error.code, error_codes::INVALID_PARAMS, "{method} wrong error code");
            assert_eq!(
                error.data.unwrap().get("reason").unwrap(),
                reason,
                "{method} wrong reason"
            );
        }
    }

    /// Regression test for the PR #26 review's 🔴 finding: the original
    /// implementation used `Notify::notify_waiters()` which is non-sticky —
    /// if signaled while no `.notified().await` was active, the signal was
    /// silently lost. This test triggers shutdown BEFORE any receiver
    /// observes it, then asserts that a receiver created later still
    /// observes the signal (i.e., `watch::<bool>` is sticky and cannot lose
    /// the signal in a race window).
    #[tokio::test]
    async fn shutdown_signal_is_sticky_across_subscribe_after_trigger() {
        let (tx, rx) = shutdown_signal();
        // Trigger shutdown BEFORE anyone awaits — this is the failure mode
        // of the original Notify-based implementation.
        trigger_shutdown(&tx);
        // A new receiver created at any later time observes `true`.
        let mut late_rx = rx;
        // `wait_for_shutdown` should resolve essentially immediately because
        // the value is already `true`. A 100 ms timeout is generous; failure
        // here means the signal was lost.
        tokio::time::timeout(std::time::Duration::from_millis(100), wait_for_shutdown(&mut late_rx))
            .await
            .expect("wait_for_shutdown must resolve when value is already true (PR #26 sticky-shutdown regression)");
    }

    /// Regression test for the same root cause but in the snapshot-await
    /// race window: the observation loop checks `*shutdown.borrow()` at the
    /// top of every iteration and races every long await against the
    /// shutdown receiver inside a `select!`. This test simulates the race
    /// by triggering shutdown WHILE another task is mid-await on
    /// `wait_for_shutdown` and proves the await resolves cleanly.
    #[tokio::test]
    async fn shutdown_signal_unblocks_inflight_wait() {
        let (tx, rx) = shutdown_signal();
        let mut rx_for_task = rx.clone();
        let waiter = tokio::spawn(async move {
            wait_for_shutdown(&mut rx_for_task).await;
        });
        // Give the task a moment to enter the await.
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        trigger_shutdown(&tx);
        // The waiter should resolve well within 100 ms.
        tokio::time::timeout(std::time::Duration::from_millis(100), waiter)
            .await
            .expect("in-flight wait_for_shutdown must resolve when signal fires (PR #26 sticky-shutdown regression)")
            .expect("waiter task should not panic");
    }

    #[tokio::test]
    async fn runbundle_write_rejects_path_traversal() {
        let engine = StubEngine;
        let hz = std::sync::Arc::new(tokio::sync::Mutex::new(None::<u32>));
        let filter = std::sync::Arc::new(tokio::sync::Mutex::new(None::<String>));
        let cases = [
            json!({"schema_version": 1, "id_override": "../../../etc/passwd"}),
            json!({"schema_version": 1, "id_override": "foo/bar"}),
            json!({"schema_version": 1, "id_override": "foo\\bar"}),
        ];
        for params in cases {
            let req = json!({"jsonrpc": "2.0", "id": 1, "method": "runbundle.write", "params": params});
            let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
                .await
                .unwrap();
            let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
            let error = parsed
                .error
                .unwrap_or_else(|| panic!("runbundle.write must reject {params}"));
            assert_eq!(error.code, error_codes::INVALID_PARAMS);
            assert_eq!(error.data.unwrap().get("reason").unwrap(), "path_traversal_rejected");
        }
    }
}
