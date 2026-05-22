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
//!   - `observe.assets.ledger_summary` (M4A) -> AssetLedgerSummary
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
pub(crate) async fn wait_for_shutdown(rx: &mut ShutdownReceiver) {
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
        ActChassisClearJamParams, ActChassisRepairParams, ActChassisSalvageParams, ActInputCameraAnchorParams,
        ActInputCaptureControlsParams, ActMissionPauseParams, ActMissionResumeParams, ActPlayerAbortParams,
        ActPlayerActivateAbilityParams, ActPlayerAimParams, ActPlayerAnchorParams, ActPlayerAttachModifierParams,
        ActPlayerBoardParams, ActPlayerBrainHopParams, ActPlayerClimbParams, ActPlayerCrouchParams,
        ActPlayerDetachModifierParams, ActPlayerDigParams, ActPlayerDisembarkParams, ActPlayerEjectParams,
        ActPlayerFireParams, ActPlayerJetParams, ActPlayerJumpParams, ActPlayerMoveParams,
        ActPlayerPauseCinematicParams, ActPlayerQuickActionRadialParams, ActPlayerQuickActionSliceParams,
        ActPlayerQuickActionSlotParams, ActPlayerQuickActionToggleParams, ActPlayerReloadParams,
        ActPlayerReplayCinematicParams, ActPlayerResetParams, ActPlayerSelectItemParams, ActPlayerSetDroneModeParams,
        ActPlayerSharpAimParams, ActPlayerSkipCinematicParams, ActPlayerWeaponCycleParams, ActSquadAssignRoleParams,
        ActSquadIssueParams, ActSquadSetFormationParams,
        ActPlayerTreatParams, ActPlayerScanParams, ActPlayerCprRoundParams, ActPlayerDefibParams,
        ActPlayerSurgeryStartParams, ActPlayerTriageSelectParams,
        ActPlayerInstallProstheticParams, ActPlayerMaintainProstheticParams,
        ActPlayerRetireVeteranParams,
        ActPlayerDismountParams, ActPlayerFireGrappleParams, ActPlayerMountParams, ActPlayerReleaseRopeParams,
        ActPlayerRopeInputParams, ActPlayerVaultParams, ActPlayerWallJumpParams, ActPlayerZiplineBrakeParams,
        ActPlayerZiplineClipParams,
        InspectActorParams, InspectAiParams, InspectChassisParams, InspectEquipmentParams, InspectMissionParams,
        ObserveActorParams, ObserveAiParams, ObserveChassisSilhouetteParams, ObserveMissionParams,
        ObserveOnceParams, ObservePerceptionParams, ObserveSubscribeParams, RunBundleWriteParams,
        RunForTicksParams, ScenarioLoadParams, SrvDumpCinematicStateParams, SrvDumpSquadStateParams, StepParams,
        SystemShutdownParams,
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
    /// `1.0` = no shake (accessibility floor).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reduce_camera_shake_pct: Option<f32>,
    /// live-retick on patch; this mirrors the engine's configured tick rate
    /// so cfctl `observe.settings` is round-trippable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tick_rate_hz: Option<u32>,
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
    /// `"cakewalk" | "tough_crowd" | "veteran"`. The engine looks up the
    /// preset in `cf-ai::DifficultyPreset::builtin(id)` and applies it to
    /// every `ReactiveGuard` in the world.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai_difficulty: Option<String>,
    /// above every reactive guard; `false` hides it. Defaults to current
    /// state when omitted (Option-based settings patch).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai_debug: Option<bool>,

    // === M8 accessibility / camera / debug / locale extensions ===
    /// `off | slowdown_75 | slowdown_25 | full_pause`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_speed_assist: Option<String>,
    /// `default | colorblind_safe | protanopia | deuteranopia | tritanopia
    /// | monochrome_test`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_cue_mode: Option<String>,
    /// auto_aim_with_damage_penalty`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aim_assist: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub damage_numbers: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub killcam_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hit_stop_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cinematic_kills: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mini_map_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compass_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub damage_direction_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mini_map_zoom: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_zoom_fov: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_scale: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_density: Option<String>,
    /// T-ACC-PLUS BP9+).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speedrun_mode: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permadeath: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_respawn: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fog_of_war_on: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limited_ammo: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_limit: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_minimap: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hardcore_mode: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub friendly_fire_on: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debug_enabled: Option<bool>,

    // === M11 ACC-A floor extensions ===
    /// `standard | high_contrast_dark | high_contrast_light`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contrast_mode: Option<String>,
    /// `off | critical_only | standard | expanded`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_background_opacity: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_categories: Option<std::collections::BTreeSet<String>>,
    /// `keyboard_mouse | controller | keyboard_only | custom`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remap_groups: Option<std::collections::BTreeSet<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hold_behavior: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screen_shake_scale: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera_motion: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub objective_help: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debug_explainer_level: Option<String>,

    // === M12 cinematic story beats + optional comic overlay ===
    /// `subtle` in `Settings`; the patch lets the player drop to `off` or
    /// escalate to `full` from the settings UI. Per spec § Comic-style
    /// framing — opt-in juice, not core identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comic_style_overlay: Option<String>,
    /// renders as a 4-panel comic-style cause chain; when `false` (default),
    /// the M10 replay viewer + cause-chain walker is used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comic_death_recap: Option<bool>,
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
            && self.game_speed_assist.is_none()
            && self.color_cue_mode.is_none()
            && self.aim_assist.is_none()
            && self.damage_numbers.is_none()
            && self.killcam_enabled.is_none()
            && self.hit_stop_enabled.is_none()
            && self.cinematic_kills.is_none()
            && self.mini_map_enabled.is_none()
            && self.compass_enabled.is_none()
            && self.damage_direction_enabled.is_none()
            && self.mini_map_zoom.is_none()
            && self.scope_zoom_fov.is_none()
            && self.text_scale.is_none()
            && self.ui_density.is_none()
            && self.language.is_none()
            && self.speedrun_mode.is_none()
            && self.permadeath.is_none()
            && self.no_respawn.is_none()
            && self.fog_of_war_on.is_none()
            && self.limited_ammo.is_none()
            && self.time_limit.is_none()
            && self.no_minimap.is_none()
            && self.hardcore_mode.is_none()
            && self.friendly_fire_on.is_none()
            && self.debug_enabled.is_none()
            // M11 ACC-A floor
            && self.contrast_mode.is_none()
            && self.caption_mode.is_none()
            && self.caption_background_opacity.is_none()
            && self.caption_categories.is_none()
            && self.input_profile.is_none()
            && self.remap_groups.is_none()
            && self.hold_behavior.is_none()
            && self.screen_shake_scale.is_none()
            && self.camera_motion.is_none()
            && self.objective_help.is_none()
            && self.debug_explainer_level.is_none()
            // M12 cinematic story beats
            && self.comic_style_overlay.is_none()
            && self.comic_death_recap.is_none()
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
            // M1 re-audit (2026-05-13): reject NaN/Inf AND negative gravity.
            // Spec says invalid patches like `negative gravity` are rejected
            // (the engine uses Bevy convention `gravity = -980.0` baked into
            // the EngineConfig, so the SettingsPatch only ever sets the
            // magnitude). A negative input would flip the sim into
            // anti-gravity which is reserved for explicit scenario opt-in,
            // not user-toggleable via settings.
            if !v.is_finite() {
                return Some("gravity_must_be_finite".to_string());
            }
            if v < 0.0 {
                return Some("gravity_must_be_non_negative".to_string());
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
        // M8: validate the new enum strings + numeric ranges so cfctl
        // rejects unknown values at the dispatch boundary.
        if let Some(s) = self.game_speed_assist.as_deref() {
            if crate::settings::GameSpeedAssist::from_str(s).is_none() {
                return Some(format!("game_speed_assist_unknown: {s}"));
            }
        }
        if let Some(s) = self.color_cue_mode.as_deref() {
            if crate::settings::ColorCueMode::from_str(s).is_none() {
                return Some(format!("color_cue_mode_unknown: {s}"));
            }
        }
        if let Some(s) = self.aim_assist.as_deref() {
            if crate::settings::AimAssist::from_str(s).is_none() {
                return Some(format!("aim_assist_unknown: {s}"));
            }
        }
        if let Some(s) = self.ui_density.as_deref() {
            if crate::settings::UiDensity::from_str(s).is_none() {
                return Some(format!("ui_density_unknown: {s}"));
            }
        }
        if let Some(v) = self.mini_map_zoom {
            if !v.is_finite() {
                return Some("mini_map_zoom_must_be_finite".to_string());
            }
        }
        if let Some(v) = self.scope_zoom_fov {
            if !v.is_finite() {
                return Some("scope_zoom_fov_must_be_finite".to_string());
            }
        }
        if let Some(v) = self.text_scale {
            if !v.is_finite() {
                return Some("text_scale_must_be_finite".to_string());
            }
        }
        if let Some(s) = self.language.as_deref() {
            if s.is_empty() {
                return Some("language_must_not_be_empty".to_string());
            }
        }
        // M11 ACC-A enum/range validators.
        if let Some(s) = self.contrast_mode.as_deref() {
            if crate::settings::ContrastMode::from_str(s).is_none() {
                return Some(format!("contrast_mode_unknown:{s}"));
            }
        }
        if let Some(s) = self.caption_mode.as_deref() {
            if crate::settings::CaptionMode::from_str(s).is_none() {
                return Some(format!("caption_mode_unknown:{s}"));
            }
        }
        if let Some(v) = self.caption_background_opacity {
            if !v.is_finite() || !(0.0..=1.0).contains(&v) {
                return Some("caption_background_opacity_out_of_range".to_string());
            }
        }
        if let Some(cats) = self.caption_categories.as_ref() {
            for cat in cats {
                if !crate::settings::SUPPORTED_CAPTION_CATEGORIES.contains(&cat.as_str()) {
                    return Some(format!("caption_categories_unknown:{cat}"));
                }
            }
        }
        if let Some(s) = self.input_profile.as_deref() {
            if crate::settings::InputProfile::from_str(s).is_none() {
                return Some(format!("input_profile_unknown:{s}"));
            }
        }
        if let Some(groups) = self.remap_groups.as_ref() {
            for g in groups {
                if !crate::settings::SUPPORTED_REMAP_GROUPS.contains(&g.as_str()) {
                    return Some(format!("remap_groups_unknown:{g}"));
                }
            }
        }
        if let Some(s) = self.hold_behavior.as_deref() {
            if crate::settings::HoldBehavior::from_str(s).is_none() {
                return Some(format!("hold_behavior_unknown:{s}"));
            }
        }
        if let Some(v) = self.screen_shake_scale {
            if !v.is_finite() || !(0.0..=1.0).contains(&v) {
                return Some("screen_shake_scale_out_of_range".to_string());
            }
        }
        if let Some(s) = self.camera_motion.as_deref() {
            if crate::settings::CameraMotion::from_str(s).is_none() {
                return Some(format!("camera_motion_unknown:{s}"));
            }
        }
        if let Some(s) = self.objective_help.as_deref() {
            if crate::settings::ObjectiveHelp::from_str(s).is_none() {
                return Some(format!("objective_help_unknown:{s}"));
            }
        }
        if let Some(s) = self.debug_explainer_level.as_deref() {
            if crate::settings::DebugExplainerLevel::from_str(s).is_none() {
                return Some(format!("debug_explainer_level_unknown:{s}"));
            }
        }
        // M12 cinematic story-beats validators.
        if let Some(s) = self.comic_style_overlay.as_deref() {
            if crate::settings::ComicStyleOverlay::from_str(s).is_none() {
                return Some(format!("comic_style_overlay_unknown:{s}"));
            }
        }
        if let Some(v) = self.ui_scale {
            if !v.is_finite() {
                return Some("ui_scale_out_of_range".to_string());
            }
            // Existing behaviour: out-of-range ui_scale CLAMPS at the patch
            // boundary; no rejection. The structured rejection mode is
            // reserved for non-finite values only.
        }
        self.key_bindings
            .as_ref()
            .and_then(|bindings| crate::settings::validate_key_bindings(bindings).err())
    }
}



/// `content/asset_ledger/ledger.jsonl` from the current working directory
/// (or the parent, when invoked from inside a crate sub-directory). Engines
/// can override `EngineHandle::observe_assets_ledger_summary` to point at a
/// different ledger path or to skip reading entirely.
pub fn default_observe_assets_ledger_summary() -> Option<serde_json::Value> {
    use std::path::PathBuf;

    let candidates = [
        PathBuf::from("content/asset_ledger/ledger.jsonl"),
        PathBuf::from("../content/asset_ledger/ledger.jsonl"),
        PathBuf::from("game/content/asset_ledger/ledger.jsonl"),
    ];
    let mut ledger_path: Option<PathBuf> = None;
    for c in &candidates {
        if c.exists() {
            ledger_path = Some(c.clone());
            break;
        }
    }
    let ledger_path = ledger_path?;
    let handle = cf_asset_ledger::LedgerHandle::new(&ledger_path);
    let entries = match handle.read_all() {
        Ok(e) => e,
        Err(err) => {
            tracing::warn!(target: "cf::ctl", error = %err, "observe.assets.ledger_summary: failed to read ledger");
            return None;
        }
    };
    let summary = cf_asset_ledger::summarize(&entries);
    Some(cf_asset_ledger::summary_to_observe_json(&summary))
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

pub(crate) fn spawn_observation_loop<E: EngineHandle>(
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


#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SchemaOnlyParams {
    schema_version: u32,
}

pub(crate) fn parse_schema_only(id: JsonRpcId, params: serde_json::Value) -> Result<(), String> {
    let parsed: SchemaOnlyParams =
        serde_json::from_value(params).map_err(|err| missing_param_error(id, &err.to_string()))?;
    let _ = parsed.schema_version;
    Ok(())
}

/// `schema_version` is MANDATORY on every M0 client→server JSON-RPC request. Missing,
/// non-numeric, or mismatched values reject with `-32602` (`InvalidParams`). There are no
/// notification methods in the M0 catalog; if a future milestone adds one, document it
/// here as an explicit exception.
pub(crate) fn check_schema_version(params: &serde_json::Value) -> Result<(), serde_json::Value> {
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

pub(crate) fn ack_response(id: JsonRpcId, result: &CommandResult) -> String {
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

pub(crate) fn success_response(id: JsonRpcId, value: serde_json::Value) -> String {
    let resp = JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(value),
        error: None,
    };
    serde_json::to_string(&resp).unwrap_or_default()
}

pub(crate) fn error_response(id: JsonRpcId, code: i32, message: &str, data: serde_json::Value) -> String {
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

pub(crate) fn missing_param_error(id: JsonRpcId, reason: &str) -> String {
    error_response(
        id,
        error_codes::INVALID_PARAMS,
        "InvalidParams",
        json!({"reason": reason, "fix_hint": "see spec/prototype-roadmap CLI Reference for the M0 method catalog"}),
    )
}

/// helper that takes an `(x, y)` point) — rejects NaN/Inf coordinates at
/// the cfctl boundary.
pub(crate) fn x_y_finite(x: f32, y: f32) -> bool {
    x.is_finite() && y.is_finite()
}

/// Decode an M6 cfctl method's params into an [`crate::m6_actions::M6Action`].
pub(crate) fn decode_m6_action(method: &str, params: serde_json::Value) -> Result<crate::m6_actions::M6Action, String> {
    use crate::m6_actions::M6Action;
    let p = if params.is_null() {
        serde_json::json!({})
    } else {
        params
    };
    match method {
        "act.player.sprint" => {
            let active = p
                .get("active")
                .and_then(serde_json::Value::as_bool)
                .ok_or_else(|| "missing_active".to_string())?;
            Ok(M6Action::Sprint { active })
        }
        "act.player.prone" => {
            let active = p
                .get("active")
                .and_then(serde_json::Value::as_bool)
                .ok_or_else(|| "missing_active".to_string())?;
            Ok(M6Action::Prone { active })
        }
        "act.player.slide" => Ok(M6Action::Slide),
        "act.player.vault" => Ok(M6Action::Vault),
        "act.player.climb_up" => Ok(M6Action::ClimbUp),
        "act.player.climb_down" => Ok(M6Action::ClimbDown),
        "act.player.dive" => Ok(M6Action::Dive),
        "act.player.lean" => {
            let direction = p.get("direction").and_then(serde_json::Value::as_f64).unwrap_or(0.0) as f32;
            if !direction.is_finite() {
                return Err("non_finite_direction".to_string());
            }
            Ok(M6Action::Lean { direction })
        }
        "act.player.stealth_kill" => Ok(M6Action::StealthKill),
        "act.player.knife_throw" => Ok(M6Action::KnifeThrow),
        "act.player.weapon_swap" => {
            let slot = p
                .get("slot")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| "missing_slot".to_string())?;
            // 0..=7). Tank slots (9-11 / indices 8..=10) reject with the
            // spec-locked reason `tank_slot_locked_at_m2_2a` so the M17
            // unlock has a stable contract to clear.
            if slot >= 11 {
                return Err("slot_out_of_range".to_string());
            }
            if slot >= 8 {
                return Err(cf_equipment::TANK_SLOT_LOCKED_REASON.to_string());
            }
            Ok(M6Action::WeaponSwap { slot: slot as u8 })
        }
        "act.player.drop_item" => {
            let slot = p.get("slot").and_then(serde_json::Value::as_u64).map(|v| v as u8);
            Ok(M6Action::DropItem { slot })
        }
        "act.player.pickup" => Ok(M6Action::Pickup),
        "act.player.signal_friendly" => Ok(M6Action::SignalFriendly),
        "act.player.signal_enemy_spotted" => Ok(M6Action::SignalEnemySpotted),
        "act.player.mark_waypoint" => {
            let x = p.get("x").and_then(serde_json::Value::as_f64).unwrap_or(0.0) as f32;
            let y = p.get("y").and_then(serde_json::Value::as_f64).unwrap_or(0.0) as f32;
            if !x.is_finite() || !y.is_finite() {
                return Err("non_finite_waypoint".to_string());
            }
            Ok(M6Action::MarkWaypoint { x, y })
        }
        "act.player.deploy_bipod" => Ok(M6Action::DeployBipod),
        "act.player.stow_bipod" => Ok(M6Action::StowBipod),
        "act.player.cycle_fire_mode" => Ok(M6Action::CycleFireMode),
        "act.player.cook_grenade" => Ok(M6Action::CookGrenade),
        "act.player.throw_grenade" => Ok(M6Action::ThrowGrenade),
        "act.player.melee_bash" => Ok(M6Action::MeleeBash),
        "act.player.melee_kick" => Ok(M6Action::MeleeKick),
        "act.player.use_tool" => {
            let tool_kind = p
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "missing_kind".to_string())?
                .to_string();
            Ok(M6Action::UseTool { tool_kind })
        }
        "act.player.attach_suppressor" => Ok(M6Action::AttachSuppressor),
        "act.player.detach_suppressor" => Ok(M6Action::DetachSuppressor),
        "act.player.set_facing" => {
            let facing = p
                .get("facing")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "missing_facing".to_string())?
                .to_string();
            if facing != "left" && facing != "right" {
                return Err("invalid_facing".to_string());
            }
            Ok(M6Action::SetFacing { facing })
        }
        "act.player.aim_set_facing" => {
            let facing = p
                .get("facing")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "missing_facing".to_string())?
                .to_string();
            if facing != "left" && facing != "right" {
                return Err("invalid_facing".to_string());
            }
            Ok(M6Action::AimSetFacing { facing })
        }
        "act.player.nest_container" => {
            let parent_instance_id = p
                .get("parent_instance_id")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| "missing_parent_instance_id".to_string())?;
            let child_item_id = p
                .get("child_item_id")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| "missing_child_item_id".to_string())?
                .to_string();
            if child_item_id.is_empty() {
                return Err("empty_child_item_id".to_string());
            }
            Ok(M6Action::NestContainer {
                parent_instance_id,
                child_item_id,
            })
        }
        _ => Err(format!("unknown_m6_method:{method}")),
    }
}

pub(crate) fn invalid_param_reason(id: JsonRpcId, reason: &str) -> String {
    error_response(
        id,
        error_codes::INVALID_PARAMS,
        "InvalidParams",
        json!({"reason": reason, "fix_hint": "see spec/prototype-roadmap CLI Reference for the M0 method catalog"}),
    )
}

pub use async_trait::async_trait;


// Re-exports for sibling modules.
pub use crate::server_command::ControlCommand;
pub use crate::server_engine_handle::EngineHandle;
pub(crate) use crate::server_process_request::process_request;
