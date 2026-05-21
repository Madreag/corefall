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

    // === M8 accessibility / camera / debug / locale extensions ===
    /// **M8**: slow-motion accessibility mode (snake_case wire form
    /// `off | slowdown_75 | slowdown_25 | full_pause`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_speed_assist: Option<String>,
    /// **M8**: color blind / contrast palette mode (snake_case wire form
    /// `default | colorblind_safe | protanopia | deuteranopia | tritanopia
    /// | monochrome_test`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_cue_mode: Option<String>,
    /// **M8**: aim assist mode (`off | steady_aim |
    /// auto_aim_with_damage_penalty`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aim_assist: Option<String>,
    /// **M8**: damage numbers cosmetic toggle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub damage_numbers: Option<bool>,
    /// **M8**: killcam toggle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub killcam_enabled: Option<bool>,
    /// **M8**: hit-stop toggle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hit_stop_enabled: Option<bool>,
    /// **M8**: cinematic kill cam toggle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cinematic_kills: Option<bool>,
    /// **M8**: master mini-map enable toggle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mini_map_enabled: Option<bool>,
    /// **M8**: compass enable toggle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compass_enabled: Option<bool>,
    /// **M8**: damage-direction indicator enable toggle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub damage_direction_enabled: Option<bool>,
    /// **M8**: mini-map zoom (0.25..=4.0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mini_map_zoom: Option<f32>,
    /// **M8**: scope ADS FOV in degrees (5..=90).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_zoom_fov: Option<f32>,
    /// **M8**: text scale (0.5..=4.0; mirrors `ui_scale`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_scale: Option<f32>,
    /// **M8**: HUD density preset (`compact | normal | spacious`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_density: Option<String>,
    /// **M8**: language code (`en` baseline; Tier-A 11 reserved for
    /// T-ACC-PLUS BP9+).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// **M8**: speedrun mode toggle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speedrun_mode: Option<bool>,
    /// **M8**: permadeath modifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permadeath: Option<bool>,
    /// **M8**: no-respawn modifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_respawn: Option<bool>,
    /// **M8**: fog-of-war on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fog_of_war_on: Option<bool>,
    /// **M8**: limited-ammo modifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limited_ammo: Option<bool>,
    /// **M8**: time-limit modifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_limit: Option<bool>,
    /// **M8**: hide the mini-map (overrides `mini_map_enabled`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_minimap: Option<bool>,
    /// **M8**: hardcore composite mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hardcore_mode: Option<bool>,
    /// **M8**: friendly fire on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub friendly_fire_on: Option<bool>,
    /// **M8**: master debug-overlay gate (production builds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debug_enabled: Option<bool>,

    // === M11 ACC-A floor extensions ===
    /// **M11**: contrast palette mode (snake_case wire form
    /// `standard | high_contrast_dark | high_contrast_light`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contrast_mode: Option<String>,
    /// **M11**: captions verbosity mode (snake_case wire form
    /// `off | critical_only | standard | expanded`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_mode: Option<String>,
    /// **M11**: caption background opacity (0.0..=1.0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_background_opacity: Option<f32>,
    /// **M11**: caption category subset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption_categories: Option<std::collections::BTreeSet<String>>,
    /// **M11**: input profile (snake_case wire form
    /// `keyboard_mouse | controller | keyboard_only | custom`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_profile: Option<String>,
    /// **M11**: remap-action group subset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remap_groups: Option<std::collections::BTreeSet<String>>,
    /// **M11**: hold-behavior variant (`hold | toggle | press_to_cycle`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hold_behavior: Option<String>,
    /// **M11**: screen-shake scale (0.0..=1.0; multiplicative).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screen_shake_scale: Option<f32>,
    /// **M11**: camera-motion granularity (`reduced | standard`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera_motion: Option<String>,
    /// **M11**: objective-help verbosity (`minimal | standard | verbose`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub objective_help: Option<String>,
    /// **M11**: debug-explainer level (`player | designer | raw`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debug_explainer_level: Option<String>,

    // === M12 cinematic story beats + optional comic overlay ===
    /// **M12**: comic-style overlay mode (`full | subtle | off`). Defaults to
    /// `subtle` in `Settings`; the patch lets the player drop to `off` or
    /// escalate to `full` from the settings UI. Per spec § Comic-style
    /// framing — opt-in juice, not core identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comic_style_overlay: Option<String>,
    /// **M12**: comic death-recap toggle. When `true`, the death recap
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
        /// **M14C** § optional ammo-kind selector
        /// (`heat` / `apfsds` / `regular` / `tracer` / etc.). `None` =
        /// use the weapon's default round per existing M6 behavior.
        ammo_kind: Option<cf_equipment::RoundKind>,
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
    /// **M3 re-open (2026-05-13)**: place an anchor / tether at world `(x, y)`.
    /// Samples the chunked terrain material at the target and emits
    /// `terrain.anchor_material_result` with `result="accepted"` (anchorable
    /// material) or `result="refused"` (non-anchorable, with `reason` label).
    /// See `specs/active/M3.md` § Re-opened gaps, MAT-T-06.
    ActPlayerAnchor {
        x: f64,
        y: f64,
        tool_id: Option<String>,
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
    /// **M11**: pointer click at logical screen coords `(x, y)`. Resolves
    /// the hit `target_node_id` via the HUD layout and emits a
    /// `ux.mouse_clicked` event. Non-finite coords reject at the dispatch
    /// boundary.
    ActInputMouseClick {
        x: f32,
        y: f32,
        source: IntentSource,
    },
    /// **M11**: pointer move at logical screen coords `(x, y)`. Resolves
    /// the hover `hover_node_id` via the HUD layout and emits a
    /// `ux.mouse_moved` event. Non-finite coords reject at the dispatch
    /// boundary.
    ActInputMouseMove {
        x: f32,
        y: f32,
        source: IntentSource,
    },
    /// **M11 audit pass (GAP-M11-01 HIGH fix)**: keyed action press for the
    /// BP3 self-play floor + pause-overlay cycling. Per the M11 spec
    /// § "Pause + slowdown overlay": "Triggered via `act.input.key_press
    /// { action: 'pause' }` (cycles through modes)". `action` is one of
    /// `pause`, `game_speed_cycle`, `accessibility_overlay`, `tactical_overlay`,
    /// `photo_mode`, `debug_overlay`, `mini_map_toggle`, `compass_toggle`,
    /// `damage_direction_toggle`, `captions_toggle`. Unknown actions reject
    /// with reason `unknown_key_action`.
    ActInputKeyPress {
        action: String,
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
    /// **M14A**: instant slot invocation (keys 1-8).
    ActPlayerQuickActionSlot {
        slot: u8,
        source: IntentSource,
    },
    /// **M14A**: tap-Q quick-toggle to last-used slot.
    ActPlayerQuickActionToggle {
        source: IntentSource,
    },
    /// **M14A**: open/close hold-Q radial picker with sim time-slow.
    ActPlayerQuickActionRadial {
        active: bool,
        source: IntentSource,
    },
    /// **M14A**: commit a radial slice (1-8).
    ActPlayerQuickActionSlice {
        slice: u8,
        source: IntentSource,
    },
    /// **M14A**: mouse-wheel cycle within current slot's category.
    ActPlayerWeaponCycle {
        direction: i8,
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
    /// **M13** § "Brain hopping" — transfer control to a different
    /// friendly actor; the prior actor stays at its position as a
    /// mission-critical AI fallback.
    ActPlayerBrainHop {
        target_actor_id: u64,
        source: IntentSource,
    },
    /// **M13** § "Chassis ability slots" — activate one ability.
    ActPlayerActivateAbility {
        ability: String,
        source: IntentSource,
    },
    /// **M13** § "Cockpit camera anchor" — switch camera anchor mode.
    ActInputCameraAnchor {
        mode: String,
        source: IntentSource,
    },
    /// **M13** § "Drone allies" — switch drone ally mode.
    ActPlayerSetDroneMode {
        mode: String,
        source: IntentSource,
    },
    /// **M13** § "Weapon modifier slots" — attach a Noita-style modifier.
    ActPlayerAttachModifier {
        modifier: String,
        source: IntentSource,
    },
    /// **M13** § "Weapon modifier slots" — detach a modifier.
    ActPlayerDetachModifier {
        modifier: String,
        source: IntentSource,
    },
    /// **M13** § "Boarding / disembarking transitions" — start boarding into
    /// a chassis actor (1500ms transition).
    ActPlayerBoard {
        chassis_actor_id: u64,
        source: IntentSource,
    },
    /// **M13** § "Boarding / disembarking transitions" — start disembarking
    /// out of the current chassis (1500ms transition).
    ActPlayerDisembark {
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
    /// **M6**: umbrella dispatch for the 26 new tactical-controller actions
    /// (sprint, slide, vault, lean, stealth kill, knife throw, weapon swap,
    /// drop / pickup, signals, mark waypoint, deploy bipod, cycle fire mode,
    /// cook / throw grenade, melee bash / kick, use tool, suppressor
    /// attach / detach, set facing). Engine reads the inner action + updates
    /// `ActorState` flags + records the matching control event.
    ActM6 {
        action: crate::m6_actions::M6Action,
        source: IntentSource,
    },
    /// **M6**: issue one of the 4 squad commands to a bot. `bot_actor=None`
    /// broadcasts to all followers.
    ActSquadIssueCommand {
        bot_actor: Option<u64>,
        kind: crate::m6_actions::SquadCommandKindOverWire,
        waypoint: Option<(f32, f32)>,
        source: IntentSource,
    },
    /// **M6**: cancel the named squad member's current command, returning
    /// them to the default `FollowLeader`. Re-emits `squad.command_issued`
    /// with `kind="follow_leader"` so the replay stream stays linear.
    ActSquadCancelCommand {
        actor_id: u64,
        source: IntentSource,
    },
    /// **M7-B**: set a single task weight on an actor's PriorityTable
    /// (clamps to 0..=9). Spec § Smart commandable AI — Per-task override.
    /// Mutates `M7AiWorld.bots[actor].stack.priority` AND emits
    /// `ai.priority_table_changed`.
    ActPlayerSetPriority {
        actor_id: u64,
        task: String,
        weight: u8,
        source: IntentSource,
    },
    /// **M7-B**: set an actor's autonomy mode (FullAuto / Standard /
    /// Manual). Spec § Smart commandable AI — Layer 1 Autonomy mode.
    /// Mutates `M7AiWorld.bots[actor].stack.autonomy` AND emits
    /// `ai.autonomy_mode_changed`.
    ActPlayerSetAutonomyMode {
        actor_id: u64,
        mode: String,
        source: IntentSource,
    },
    /// **M7-B**: replace an actor's role + PriorityTable with one of the
    /// 6 spec-mandated role templates. Spec § Smart commandable AI — 6
    /// role templates. Emits `ai.role_template_applied`.
    ActPlayerApplyRoleTemplate {
        actor_id: u64,
        template_id: String,
        source: IntentSource,
    },
    /// **M7-B**: apply one of the 5 spec-named quick presets (attack /
    /// defend / overwatch / rescue / salvage). Emits
    /// `ai.quick_preset_applied`.
    ActPlayerApplyQuickPreset {
        actor_id: u64,
        preset_id: String,
        source: IntentSource,
    },
    /// **M7B**: issue a verb from the squad-command grammar to a squad.
    /// Spec § "50+ named squad verbs in a data-driven registry".
    ActSquadIssue {
        squad_id: u64,
        verb_id: String,
        args: Vec<serde_json::Value>,
        source: IntentSource,
    },
    /// **M7B**: switch the squad's active formation kind. Spec § "9
    /// formation kinds with per-actor slot resolution".
    ActSquadSetFormation {
        squad_id: u64,
        formation_kind: String,
        source: IntentSource,
    },
    /// **M7B**: assign a sticky role to a squad member. Spec §
    /// "Per-member role assignment is sticky + loadout-aware".
    ActSquadAssignRole {
        squad_id: u64,
        member_actor_id: u64,
        role: String,
        source: IntentSource,
    },
    /// **M7B**: dump the full squad-state JSON view including the verb
    /// registry, formation catalog, and archetype-BT node counts.
    SrvDumpSquadState {
        squad_id: u64,
        source: IntentSource,
    },
    // === M8 cfctl surface ===
    /// **M8**: switch the camera mode (`follow | scope | free_look`).
    ActCameraSetMode {
        mode: String,
        source: IntentSource,
    },
    /// **M8**: trigger a hit-stop pulse (50..200ms; clamped). `trigger`
    /// records the cause label (`melee_hit`, `ap_round_hit`, etc.).
    ActCameraHitStop {
        duration_ms: u32,
        trigger: String,
        actor_id: Option<u64>,
        source: IntentSource,
    },
    /// **M8**: enter sniper scope ADS at the configured `scope_zoom_fov`.
    /// Equivalent to `act.camera.set_mode { mode: "scope" }` but encodes
    /// player intent specifically.
    ActCameraScopeZoom {
        source: IntentSource,
    },
    /// **M8**: toggle free-look (RMB hold). When `active=true` the camera
    /// transitions to FreeLook anchored at `cursor`; when false it
    /// returns to Follow.
    ActCameraFreeLookToggle {
        active: bool,
        cursor: Option<(f32, f32)>,
        max_distance: f32,
        source: IntentSource,
    },
    /// **M8**: enter photo mode. cf-photo's PhotoModeState becomes active;
    /// cf-control mirrors the sim pause + emits `photo_mode.entered`.
    ActPhotoEnter {
        source: IntentSource,
    },
    /// **M8**: exit photo mode.
    ActPhotoExit {
        source: IntentSource,
    },
    /// **M8**: cycle to the next photo filter (none / sepia / b&w /
    /// color_grade / cyberpunk_neon).
    ActPhotoCycleFilter {
        source: IntentSource,
    },
    /// **M8**: capture a photo (records the `photo_mode.shot_taken` event;
    /// the actual PNG export happens in cf-app via cf-photo::export_png).
    ActPhotoShoot {
        source: IntentSource,
    },
    /// **M8**: scrub the replay timeline by `delta_seconds` (negative =
    /// rewind, positive = forward).
    ActReplayScrub {
        delta_seconds: f32,
        source: IntentSource,
    },
    /// **M8**: drop a replay bookmark with the supplied label.
    ActReplayBookmark {
        label: String,
        source: IntentSource,
    },
    /// **M8**: toggle one of the 7 cf-debug overlays (`ai_state |
    /// pathfinding | collision | material | physics | sound | squad`).
    ActDebugToggleOverlay {
        overlay: String,
        source: IntentSource,
    },
    /// **M8**: set a HUD widget's draggable position; emits
    /// `ux.hud_layout_changed`.
    ActUiSetHudLayout {
        node: String,
        x: f32,
        y: f32,
        source: IntentSource,
    },
    /// **M8**: save the current HUD layout under `name`; emits
    /// `ux.preset_saved`.
    ActUiSavePreset {
        name: String,
        source: IntentSource,
    },
    /// **M8**: toggle the Tab tactical overlay; emits
    /// `ux.tactical_overlay_toggled`. `multiplayer` controls the
    /// sim-speed cap (single-player pauses; multiplayer = 25%).
    ActPlayerToggleTacticalOverlay {
        multiplayer: bool,
        source: IntentSource,
    },
    /// **M8**: drop a multi-step plan onto a squadmate (max 8 steps).
    /// Emits `ai.plan_composed`.
    ActPlayerComposePlan {
        actor_id: u64,
        steps: Vec<String>,
        source: IntentSource,
    },
    /// **M8**: pick a slot on the Q-hold context wheel for `actor_id`.
    /// Emits `ai.context_wheel_selected`. `slot` is 0..=7.
    /// `target_kind` selects the per-target slot ordering per spec
    /// § Q-hold context wheel (one of `none` / `squadmate` / `door` /
    /// `enemy` / `terrain_breach` / `hazard` / `reactor_module`). When
    /// the kind needs an entity id (`squadmate` / `door` / `enemy` /
    /// `hazard` / `reactor_module`) the caller supplies `target_id`.
    /// Missing or unknown values fall back to `ReticleTarget::None`.
    ActPlayerContextWheelSelect {
        actor_id: u64,
        slot: u8,
        target_kind: String,
        target_id: Option<u64>,
        source: IntentSource,
    },
    /// **M8**: M / R / G panic surface. `kind` is `medic`, `engineer`,
    /// or `grenade`. Emits `ai.panic_call_emitted`.
    ActPlayerPanicCall {
        kind: String,
        source: IntentSource,
    },
    /// **M8**: MMB tag drop on `target_id`. Emits `ai.target_tagged` +
    /// engine raises Utility weight by +0.5 for engaging the target.
    ActPlayerTagTarget {
        target_id: u64,
        source: IntentSource,
    },
    /// **M8**: 'Why?' (Y) key — surfaces the bot's `reason_label_recent`
    /// ringbuffer head as a HUD popup. Emits `ai.reason_query_returned`.
    ActPlayerQueryWhy {
        actor_id: u64,
        source: IntentSource,
    },
    /// **M8**: open the T-key 8-slice pie menu with target context
    /// (`void` / `nearest_actor` / `door` / `item`). Emits
    /// `ux.pie_menu_opened`. Slows sim to 20% in single-player; 100% in
    /// multiplayer.
    ActPlayerPieMenuOpen {
        target_kind: String,
        target_id: Option<u64>,
        multiplayer: bool,
        source: IntentSource,
    },
    /// **M8**: select a 0..=7 slot on the open pie menu. Emits
    /// `ux.pie_menu_slice_chosen` on a valid pick, OR
    /// `ux.pie_menu_slice_rejected { slice, reason }` when the slice is
    /// disabled in the current context. `reason` is optional and
    /// supplied by the caller (cf-app keyboard layer) when it has
    /// pre-validated the slice; otherwise the dispatcher reports
    /// `ok=true` (valid pick) by default.
    ActPlayerPieMenuSelect {
        slot: u8,
        reason: Option<String>,
        source: IntentSource,
    },
    /// **M8**: close the pie menu (idempotent). Emits
    /// `ux.pie_menu_closed` with the open-duration in ticks.
    ActPlayerPieMenuClose {
        source: IntentSource,
    },
    SettingsSet {
        changes: Box<SettingsPatch>,
    },
    RunBundleWrite {
        id_override: Option<String>,
    },
    Shutdown {
        write_run_bundle: bool,
    },
    /// **M9B-2**: drop an authored trench template at the supplied tile
    /// origin. Loads `content/trench_templates/<id>.trench.ron` through
    /// the cf-content loader, instantiates it via
    /// `TrenchTemplate::instantiate`, and emits
    /// `trench.template_dropped` with `template_sha256` (64 hex chars),
    /// `segment_count`, and `placed_fortifications[]` per
    /// VAL-M9B-TEMPLATE-002. Optional placeholders that don't resolve to
    /// a currently-shipped M9C asset emit
    /// `trench.template_missing_fortification` warning events per
    /// VAL-M9B-TEMPLATE-004 (the template still places).
    ActPlayerDropTrenchTemplate {
        id: String,
        origin: (i32, i32),
        source: IntentSource,
    },
    /// **M9B-3 / VAL-M9B-DIG-001..003 / VAL-M9B-CFCTL-001**: dig a
    /// trench segment at the player's current tile. `variant` is one of
    /// the 6 declared cross-section variants; `tool_id` selects the
    /// dig tool (entrenching_tool T0, or pickaxe T1/T2/T3 from the
    /// M30B-tier ladder); `substrate_hardness` is `[0.0, 1.0]` from
    /// cf-material — gating the `deep` variant per VAL-M9B-DIG-003.
    /// `strict=true` makes hard-substrate `deep` requests reject
    /// outright; `false` (default) falls back to `shallow_scrape` with a
    /// `trench.segment_variant_downgraded` warning event.
    ActPlayerDigTrenchSegment {
        variant: String,
        tool_id: Option<String>,
        substrate_hardness: f32,
        strict: bool,
        source: IntentSource,
    },
    /// **M9B-3 / VAL-M9B-MODULES-002 / VAL-M9B-CFCTL-001**: place an
    /// embedded module on a built trench segment. `module_id` is one of
    /// the 6 declared modules (`duckboard`, `fire_step`, `breastwork`,
    /// `drainage_sump`, `revetment`, `corner_traverse`).
    ActPlayerPlaceTrenchModule {
        module_id: String,
        segment_id: u64,
        source: IntentSource,
    },
    /// **M9B-3 / VAL-M9B-MODULES-003 / VAL-M9B-CFCTL-001**: repair a
    /// damaged trench module. Consumes the declared per-module
    /// resources (wood/iron); emits `trench.module_repaired`.
    ActPlayerRepairTrenchModule {
        module_id: String,
        segment_id: u64,
        source: IntentSource,
    },
    /// **M14H**: apply a treatment producer to a target. `kind` is the
    /// canonical PascalCase TreatmentKind id; the engine resolves it via
    /// `cf_treatment::TreatmentKind::from_str`.
    ActPlayerTreat {
        kind: String,
        target_actor_id: u64,
        source: IntentSource,
    },
    /// **M14H**: start a 30s Medical Scanner read against a target.
    ActPlayerScan {
        target_actor_id: u64,
        source: IntentSource,
    },
    /// **M14H**: apply one CPR round (20s of compressions) to a target
    /// in cardiac arrest.
    ActPlayerCprRound {
        target_actor_id: u64,
        source: IntentSource,
    },
    /// **M14H**: deliver a defibrillator shock to a target.
    ActPlayerDefib {
        target_actor_id: u64,
        source: IntentSource,
    },
    /// **M14H**: begin a 5-phase surgery on a target.
    ActPlayerSurgeryStart {
        target_actor_id: u64,
        wounds_to_treat: u32,
        surgeon_t1: bool,
        seed: Option<u64>,
        source: IntentSource,
    },
    /// **M14H**: open / clear the Patient Detail panel selection.
    ActPlayerTriageSelect {
        target_actor_id: Option<u64>,
        source: IntentSource,
    },
    /// **M14I**: install a prosthetic on a target actor's severed zone.
    ActPlayerInstallProsthetic {
        target_actor_id: u64,
        kind: String,
        zone: String,
        source: IntentSource,
    },
    /// **M14I**: run a maintenance pass on an installed prosthetic.
    ActPlayerMaintainProsthetic {
        target_actor_id: u64,
        zone: String,
        source: IntentSource,
    },
    /// **M14I**: commit an actor's retirement.
    ActPlayerRetireVeteran {
        target_actor_id: u64,
        source: IntentSource,
    },
    /// **M14J**: manual vault override.
    ActPlayerVault {
        source: IntentSource,
    },
    /// **M14J**: wall-jump while in wall-contact grace window.
    ActPlayerWallJump {
        source: IntentSource,
    },
    /// **M14J**: fire grappling-hook gun at a world target.
    ActPlayerFireGrapple {
        target_x: f32,
        target_y: f32,
        source: IntentSource,
    },
    /// **M14J**: continuous rope climb / rappel + swing input.
    ActPlayerRopeInput {
        climb: f32,
        swing: f32,
        source: IntentSource,
    },
    /// **M14J**: release rope; inherit pendulum exit velocity.
    ActPlayerReleaseRope {
        source: IntentSource,
    },
    /// **M14J**: clip onto a deployed zip line.
    ActPlayerZiplineClip {
        line_id: u64,
        source: IntentSource,
    },
    /// **M14J**: engage / release zip-line brake.
    ActPlayerZiplineBrake {
        engaged: bool,
        source: IntentSource,
    },
    /// **M14J**: mount a tamed critter.
    ActPlayerMount {
        critter_id: u64,
        source: IntentSource,
    },
    /// **M14J**: dismount from a critter.
    ActPlayerDismount {
        source: IntentSource,
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
    /// **M2 re-audit (2026-05-13)**: return the full `MissionState`
    /// projection or `None` if no mission is loaded.
    async fn observe_mission(&self) -> Option<serde_json::Value> {
        None
    }
    /// **M2 re-audit (2026-05-13)**: return per-AI projection (guard state +
    /// perception summary + current target + reason) for `actor_id`.
    async fn observe_ai(&self, _actor_id: u64) -> Option<serde_json::Value> {
        None
    }
    /// **M3 audit pass 7 (2026-05-13)**: return the live `TerrainView`
    /// projection (chunk_count / dirty_chunk_count / material_distribution
    /// / current_overlay_mode / total_carve_events / total_debris_spawned).
    /// `None` when no chunked terrain is loaded.
    async fn observe_terrain(&self) -> Option<serde_json::Value> {
        None
    }
    /// **M9** § cfctl `observe.mission.reactor` — return the live reactor
    /// projection `{ actor_id, hp, max_hp, hp_percent, pressure_state,
    /// position, mission_critical, role, armor_layers, heat_signature_k }`.
    /// `None` when no reactor is loaded in the active scenario.
    async fn observe_mission_reactor(&self) -> Option<serde_json::Value> {
        None
    }
    /// **M9** § cfctl `observe.mission.timer` — return the mission timer
    /// projection `{ remaining_ticks, total_ticks, remaining_seconds,
    /// color_state }`. `color_state` is "green" / "yellow" / "red".
    /// `None` when no mission is loaded.
    async fn observe_mission_timer(&self) -> Option<serde_json::Value> {
        None
    }
    /// **M2 re-audit (2026-05-13)**: return the full `MissionState` +
    /// objectives + last N mission events.
    async fn inspect_mission(&self) -> Option<serde_json::Value> {
        None
    }
    /// **M2 re-audit (2026-05-13)**: return per-AI perception state + memory
    /// grid + last N ai events.
    async fn inspect_ai(&self, _actor_id: u64) -> Option<serde_json::Value> {
        None
    }
    async fn observe_actor(&self, _actor_id: Option<u64>) -> Option<serde_json::Value> {
        None
    }
    /// **M14A** § "observe.quick_action projection" — per-actor 8-slot quick
    /// action bar + radial state. Default returns `None`.
    async fn observe_quick_action(&self, _actor_id: Option<u64>) -> Option<serde_json::Value> {
        None
    }
    /// **M6**: per-actor perception projection — sight cone + hearing radius
    /// + stealth_meter + last footstep loudness band + last occlusion
    /// factor + spotted flag. `actor_id=None` resolves to the player. Default
    /// returns `None` for handlers without a perception kernel.
    async fn observe_perception(&self, _actor_id: Option<u64>) -> Option<serde_json::Value> {
        None
    }
    /// **M6**: squad-of-two projection — leader id + members[] each with
    /// per-member current_command + hp + waypoint. Default returns `None`
    /// when no squad is loaded.
    async fn observe_squad(&self) -> Option<serde_json::Value> {
        None
    }
    /// **M1 Gap B3**: return the actor view plus its last `n` actor-category
    /// events. Default returns `None`.
    async fn inspect_actor(&self, _target: Option<&str>, _last_n_events: usize) -> Option<serde_json::Value> {
        None
    }
    /// **M13**: return the full chassis body graph (15 zones + 14 joints + 5
    /// sockets), per-zone integrity (per-layer HP), per-module state, pilot
    /// state and eject window for the requested actor (`"player"` / empty =
    /// the controllable actor). Returns `None` when the actor has no chassis
    /// attached.
    async fn inspect_chassis(&self, _target: Option<&str>) -> Option<serde_json::Value> {
        None
    }

    /// **M13** § "Pilot-inside-chassis dual silhouette" — chassis-side
    /// silhouette projection (per-chassis-zone HP). Surfaces the chassis
    /// half of the dual-layer HUD silhouette so the pilot can stay on
    /// `observe.actor.silhouette`.
    async fn observe_chassis_silhouette(&self, _actor_id: Option<u64>) -> Option<serde_json::Value> {
        None
    }
    /// **M9** (audit fix gap 1): return the reactor view plus its last `n`
    /// actor-category events. Spec § Reactor as a non-player static actor:
    /// "And cfctl inspect.actor.reactor returns the full ActorState".
    /// Default returns `None`.
    async fn inspect_actor_reactor(&self, _last_n_events: usize) -> Option<serde_json::Value> {
        None
    }
    /// **M9** (audit fix gap 2): return the mission director projection
    /// `{ current_phase, phase_started_at_tick, phases_completed,
    /// intensity, spawn_budget, active_objectives }`. Default returns
    /// `None`.
    async fn observe_mission_director(&self) -> Option<serde_json::Value> {
        None
    }
    /// **M2**: return the full chunk material grid (RLE-friendly Vec) for the
    /// requested chunk coord. Default returns `None`.
    async fn inspect_terrain_chunk(&self, _cx: i32, _cy: i32) -> Option<serde_json::Value> {
        None
    }
    /// **M2**: return the full MaterialDef for the requested id. Default
    /// returns `None`.
    async fn inspect_material(&self, _id: u16) -> Option<serde_json::Value> {
        None
    }
    /// **M9** (audit round-3 fix gap 3): return the `MaterialInfo` at
    /// world-space `(x, y)` — the 9 affordance flags (actor_passable,
    /// projectile_passable, diggable, anchorable, blocks_light,
    /// contact_damage, path_cost, produces_debris, produces_sound) plus
    /// integrity (read from the per-pixel meta grid via
    /// `ChunkedTerrain::pixel_integrity`) plus color_hex (resolved from
    /// the material registry). Powers spec § "Material affordance
    /// tooltip" + the integrity-overlay reticle. Default returns `None`.
    async fn observe_terrain_material_at(&self, _x: f32, _y: f32) -> Option<serde_json::Value> {
        None
    }
    /// **M4A**: return the asset-ledger summary projection (total counts +
    /// by-category / by-tier / by-status / missing-id list). Reads the
    /// canonical `content/asset_ledger/ledger.jsonl` at the workspace
    /// root by default; engines that ship a non-default ledger path can
    /// override. Returns `None` when no ledger file exists.
    async fn observe_assets_ledger_summary(&self) -> Option<serde_json::Value> {
        default_observe_assets_ledger_summary()
    }
    /// **M4B § "observe.save.last returns last save metadata"** — last
    /// quicksave / quickload / migrate snapshot (path, schema_version,
    /// size_bytes, blake3). Default returns the empty placeholder; M0Engine
    /// overrides with the shared [`crate::m4b_save::LastSaveCache`].
    async fn observe_save_last(&self) -> serde_json::Value {
        serde_json::to_value(crate::m4b_save::LastSaveMetadata::fresh()).unwrap_or(serde_json::Value::Null)
    }
    /// **M7-B**: return the per-actor PriorityTable view (22-task weight
    /// grid + role + personality modifier). Default returns `None`.
    async fn observe_priority_table(&self, _actor_id: u64) -> Option<serde_json::Value> {
        None
    }
    /// **M7-B**: return the per-actor autonomy mode + auto-action cap.
    /// Default returns `None`.
    async fn observe_autonomy(&self, _actor_id: u64) -> Option<serde_json::Value> {
        None
    }
    /// **M7B**: return the full squad-state JSON view (verb registry +
    /// formation catalog + archetype-BT node counts + per-squad state row).
    /// Powers `srv.dump_squad_state`. Default returns `None`.
    async fn dump_squad_state(&self, _squad_id: u64) -> Option<serde_json::Value> {
        None
    }
    /// **M12C**: full `CinematicState` JSON projection — cinematic id +
    /// source + phase + playhead + duration + active word + camera
    /// offset. Powers `srv.dump_cinematic_state`. Default returns a
    /// "no cinematic" sentinel.
    async fn dump_cinematic_state(&self) -> serde_json::Value {
        json!({
            "schema_version": SCHEMA_VERSION,
            "cinematic_id": null,
            "source": null,
            "phase": "ended",
            "playhead_ms": 0,
            "duration_ms": 0,
            "active": false,
            "blocks_gameplay_input": false,
            "seen_set_count": 0,
        })
    }
    /// **M12C**: request a skip of the active cinematic. Returns the
    /// playhead ms when the skip was accepted, or an error reason when
    /// it was rejected (no cinematic active / inside confirm window).
    async fn act_player_skip_cinematic(&self) -> Result<u32, String> {
        Err("no_cinematic_active".to_string())
    }
    /// **M12C**: toggle pause on the active cinematic. Returns
    /// `(paused, ms)` after the toggle.
    async fn act_player_pause_cinematic(&self) -> Result<(bool, u32), String> {
        Err("no_cinematic_active".to_string())
    }
    /// **M12C**: replay a previously-watched cinematic from
    /// `Codex → Cinematics`. Returns the engine tick at which the
    /// replay kernel was engaged.
    async fn act_player_replay_cinematic(&self, _id: &str) -> Result<u64, String> {
        Err("no_cinematic_replay_support".to_string())
    }
    /// **M8**: return the live `cf_camera::CameraState` projection
    /// (mode + position + hit_stop_remaining_ms + fov_degrees +
    /// free_look_max_distance + free_look_cursor + deadzone_radius).
    async fn observe_camera(&self) -> serde_json::Value {
        json!({"schema_version": SCHEMA_VERSION, "mode": "follow", "fov_degrees": cf_camera::FOLLOW_FOV_DEGREES, "hit_stop_remaining_ms": 0_u32})
    }
    /// **M8**: return the active language code per cf-localization +
    /// `Settings.language`.
    async fn observe_localization_current_language(&self) -> serde_json::Value {
        json!({"schema_version": SCHEMA_VERSION, "language": "en"})
    }
    /// **M8**: return the cf-debug overlay registry — every overlay's
    /// snake_case id + whether it's currently enabled.
    async fn observe_debug_overlays(&self) -> serde_json::Value {
        json!({"schema_version": SCHEMA_VERSION, "enabled": Vec::<String>::new(), "available": cf_debug::DebugOverlay::ALL.iter().map(|o| o.as_str()).collect::<Vec<_>>()})
    }
    /// **M8**: return the Tab tactical overlay state.
    async fn observe_tactical_overlay(&self) -> serde_json::Value {
        json!({"schema_version": SCHEMA_VERSION, "open": false, "sim_speed_pct": 100_u8, "focused_actor_id": serde_json::Value::Null, "open_count": 0_u32})
    }
    /// **M8**: return the active MMB tag list (target_id + expires_at_tick
    /// + weight_bonus + issuer_actor_id).
    async fn observe_tags(&self) -> serde_json::Value {
        json!({"schema_version": SCHEMA_VERSION, "tagged": Vec::<serde_json::Value>::new()})
    }
    /// **M11 / DR-012 closure**: return the full ACC-A surface projection —
    /// 21 settings flags + key_bindings + focused_node + captions queue +
    /// banner stack — so the replay viewer + cfctl AI agents see exactly
    /// what the player sees in the HUD.
    async fn observe_accessibility(&self) -> serde_json::Value {
        let settings = self.settings_snapshot().await;
        let v = serde_json::to_value(&settings).unwrap_or(serde_json::Value::Null);
        json!({ "schema_version": SCHEMA_VERSION, "settings": v, "focusable_nodes": Vec::<String>::new() })
    }
    /// **M11**: return the live caption queue.
    async fn observe_captions(&self) -> serde_json::Value {
        json!({ "schema_version": SCHEMA_VERSION, "queue": Vec::<serde_json::Value>::new() })
    }
    /// **M11**: return the live banner stack.
    async fn observe_accessibility_banners(&self) -> serde_json::Value {
        json!({ "schema_version": SCHEMA_VERSION, "banners": Vec::<serde_json::Value>::new() })
    }
    /// **M11**: dedicated body silhouette projection (BodySilhouetteView
    /// promoted from observe_frame). Default returns `None`.
    async fn observe_actor_silhouette(&self, _actor_id: Option<u64>) -> Option<serde_json::Value> {
        None
    }
    /// **M11**: dedicated module strip projection (ModuleStripView promoted
    /// from observe_frame). Default returns `None`.
    async fn observe_actor_module_strip(&self, _actor_id: Option<u64>) -> Option<serde_json::Value> {
        None
    }
    /// **M11**: HUD assertion harness. Reads the projection that backs
    /// `node_id` and applies `predicate` (e.g. `text~=DOWNED`,
    /// `severity=critical`). Returns a JSON `{ pass: bool, observed: <val> }`.
    async fn ui_assert(&self, _node_id: &str, _predicate: &str) -> serde_json::Value {
        json!({ "schema_version": SCHEMA_VERSION, "pass": false, "observed": serde_json::Value::Null })
    }
    /// **M9B / VAL-M9B-CFCTL-002**: return `{ cover_state: Exposed | Partial | Full }`
    /// for the named actor. Default returns `Exposed` (open ground); the
    /// engine override derives the value from stance × current trench
    /// segment.
    async fn observe_actor_cover_state(&self, actor_id: u64) -> serde_json::Value {
        json!({
            "schema_version": SCHEMA_VERSION,
            "actor_id": actor_id,
            "cover_state": "Exposed",
        })
    }
    /// **M9B / VAL-M9B-CFCTL-002**: return `null` for open ground or a
    /// `TrenchSegmentView` object. Default returns `null`.
    async fn observe_trench_segment_at_pos(&self, _x: i32, _y: i32) -> serde_json::Value {
        json!({
            "schema_version": SCHEMA_VERSION,
            "result": serde_json::Value::Null,
        })
    }
}

/// **M4A**: default ledger-summary projection. Reads
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
                return Some(invalid_param_reason(request.id, "non_finite"));
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
                return Some(invalid_param_reason(request.id, "non_finite"));
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
            // **M14C** § VAL-M14C-019: surface ammo_kind={heat,apfsds}.
            let ammo_kind = match p.ammo_kind.as_deref() {
                None => None,
                Some(label) => match cf_equipment::RoundKind::from_str_snake(label) {
                    Some(k) => Some(k),
                    None => return Some(invalid_param_reason(request.id, "unknown_ammo_kind")),
                },
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerFire {
                    pressed: p.pressed,
                    ammo_kind,
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
        "act.player.anchor" => {
            // M3 re-open (2026-05-13): MAT-T-06 — emit
            // `terrain.anchor_material_result` after sampling the chunked
            // terrain material at world `(x, y)`. NaN/Inf coordinates are
            // rejected at the dispatch boundary mirroring `act.player.aim`.
            let p: ActPlayerAnchorParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if !p.x.is_finite() || !p.y.is_finite() {
                return Some(invalid_param_reason(request.id, "anchor_point_must_be_finite"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerAnchor {
                    x: p.x,
                    y: p.y,
                    tool_id: p.tool_id,
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
        "act.player.quick_action_slot" => {
            let p: ActPlayerQuickActionSlotParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerQuickActionSlot {
                    slot: p.slot,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.quick_action_toggle" => {
            let _p: ActPlayerQuickActionToggleParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerQuickActionToggle {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.quick_action_radial" => {
            let p: ActPlayerQuickActionRadialParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerQuickActionRadial {
                    active: p.active,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.quick_action_slice" => {
            let p: ActPlayerQuickActionSliceParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerQuickActionSlice {
                    slice: p.slice,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.weapon_cycle" => {
            let p: ActPlayerWeaponCycleParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerWeaponCycle {
                    direction: p.direction,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        m6_method
            if matches!(
                m6_method,
                "act.player.sprint"
                    | "act.player.prone"
                    | "act.player.slide"
                    | "act.player.vault"
                    | "act.player.climb_up"
                    | "act.player.climb_down"
                    | "act.player.dive"
                    | "act.player.lean"
                    | "act.player.stealth_kill"
                    | "act.player.knife_throw"
                    | "act.player.weapon_swap"
                    | "act.player.drop_item"
                    | "act.player.pickup"
                    | "act.player.signal_friendly"
                    | "act.player.signal_enemy_spotted"
                    | "act.player.mark_waypoint"
                    | "act.player.deploy_bipod"
                    | "act.player.stow_bipod"
                    | "act.player.cycle_fire_mode"
                    | "act.player.cook_grenade"
                    | "act.player.throw_grenade"
                    | "act.player.melee_bash"
                    | "act.player.melee_kick"
                    | "act.player.use_tool"
                    | "act.player.attach_suppressor"
                    | "act.player.detach_suppressor"
                    | "act.player.set_facing"
                    | "act.player.aim_set_facing"
                    | "act.player.nest_container"
            ) =>
        {
            let action = match decode_m6_action(m6_method, params) {
                Ok(a) => a,
                Err(err) => return Some(missing_param_error(request.id, &err)),
            };
            let result = engine
                .dispatch(ControlCommand::ActM6 {
                    action,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.squad.issue_command" => {
            let p: crate::m6_actions::ActSquadIssueCommandParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if p.kind.requires_waypoint() && p.waypoint.is_none() {
                return Some(invalid_param_reason(request.id, "squad_command_requires_waypoint"));
            }
            if let Some((x, y)) = p.waypoint {
                if !x.is_finite() || !y.is_finite() {
                    return Some(invalid_param_reason(request.id, "non_finite_waypoint"));
                }
            }
            let result = engine
                .dispatch(ControlCommand::ActSquadIssueCommand {
                    bot_actor: p.bot_actor,
                    kind: p.kind,
                    waypoint: p.waypoint,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // M6: cancel a named squad member's current command, returning them
        // to the default FollowLeader. Re-emits squad.command_issued with
        // kind=follow_leader.
        "act.squad.cancel_command" => {
            let p: crate::m6_actions::ActSquadCancelCommandParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActSquadCancelCommand {
                    actor_id: p.actor_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // **M7-B**: commandability surface — per-task weight override.
        "act.player.set_priority" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                actor_id: u64,
                #[serde(alias = "task")]
                task_type: String,
                weight: u8,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let result = engine
                .dispatch(ControlCommand::ActPlayerSetPriority {
                    actor_id: p.actor_id,
                    task: p.task_type,
                    weight: p.weight,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // **M7-B**: set autonomy mode (FullAuto / Standard / Manual).
        "act.player.set_autonomy_mode" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                actor_id: u64,
                mode: String,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let result = engine
                .dispatch(ControlCommand::ActPlayerSetAutonomyMode {
                    actor_id: p.actor_id,
                    mode: p.mode,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // **M7-B**: load one of the 6 role templates.
        "act.player.apply_role_template" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                actor_id: u64,
                template_id: String,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let result = engine
                .dispatch(ControlCommand::ActPlayerApplyRoleTemplate {
                    actor_id: p.actor_id,
                    template_id: p.template_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // **M7-B**: apply one of the 5 quick presets (attack / defend /
        // overwatch / rescue / salvage).
        "act.player.apply_quick_preset" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                actor_id: u64,
                preset_id: String,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let result = engine
                .dispatch(ControlCommand::ActPlayerApplyQuickPreset {
                    actor_id: p.actor_id,
                    preset_id: p.preset_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // **M7B**: issue a verb from the squad command grammar.
        "act.squad.issue" => {
            let p: ActSquadIssueParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let result = engine
                .dispatch(ControlCommand::ActSquadIssue {
                    squad_id: p.squad_id,
                    verb_id: p.verb_id,
                    args: p.args,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // **M7B**: switch the squad's active formation kind.
        "act.squad.set_formation" => {
            let p: ActSquadSetFormationParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let result = engine
                .dispatch(ControlCommand::ActSquadSetFormation {
                    squad_id: p.squad_id,
                    formation_kind: p.formation_kind,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // **M7B**: assign a sticky role to a squad member.
        "act.squad.assign_role" => {
            let p: ActSquadAssignRoleParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let result = engine
                .dispatch(ControlCommand::ActSquadAssignRole {
                    squad_id: p.squad_id,
                    member_actor_id: p.member_actor_id,
                    role: p.role,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // **M7B**: dump the full squad-state JSON view (verb registry +
        // formation catalog + archetype-BT node counts + per-squad state row).
        "srv.dump_squad_state" => {
            let p: SrvDumpSquadStateParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            match engine.dump_squad_state(p.squad_id).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_such_squad")),
            }
        }
        // **M7-B**: read-only projection of an actor's PriorityTable.
        "observe.priority_table" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                actor_id: u64,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            match engine.observe_priority_table(p.actor_id).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_such_ai_actor")),
            }
        }
        // **M7-B**: read-only projection of an actor's autonomy mode + cap.
        "observe.autonomy" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                actor_id: u64,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            match engine.observe_autonomy(p.actor_id).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_such_ai_actor")),
            }
        }
        // === M8 cfctl surface ===
        "act.camera.set_mode" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                mode: String,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if cf_camera::CameraMode::from_str(&p.mode).is_none() {
                return Some(invalid_param_reason(request.id, "unknown_camera_mode"));
            }
            let result = engine
                .dispatch(ControlCommand::ActCameraSetMode {
                    mode: p.mode,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.camera.hit_stop" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                #[serde(default)]
                duration_ms: Option<u32>,
                #[serde(default)]
                trigger: Option<String>,
                #[serde(default)]
                actor_id: Option<u64>,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let result = engine
                .dispatch(ControlCommand::ActCameraHitStop {
                    duration_ms: p.duration_ms.unwrap_or(0),
                    trigger: p.trigger.unwrap_or_else(|| "manual".to_string()),
                    actor_id: p.actor_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.camera.scope_zoom" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let result = engine
                .dispatch(ControlCommand::ActCameraScopeZoom {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.camera.free_look_toggle" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                active: bool,
                #[serde(default)]
                cursor_x: Option<f32>,
                #[serde(default)]
                cursor_y: Option<f32>,
                #[serde(default = "default_free_look_distance")]
                max_distance: f32,
            }
            fn default_free_look_distance() -> f32 {
                200.0
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let cursor = match (p.cursor_x, p.cursor_y) {
                (Some(x), Some(y)) => Some((x, y)),
                _ => None,
            };
            let result = engine
                .dispatch(ControlCommand::ActCameraFreeLookToggle {
                    active: p.active,
                    cursor,
                    max_distance: p.max_distance,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.photo.enter" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let result = engine
                .dispatch(ControlCommand::ActPhotoEnter {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.photo.exit" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let result = engine
                .dispatch(ControlCommand::ActPhotoExit {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.photo.cycle_filter" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let result = engine
                .dispatch(ControlCommand::ActPhotoCycleFilter {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.photo.shoot" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let result = engine
                .dispatch(ControlCommand::ActPhotoShoot {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.replay.scrub" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                delta_seconds: f32,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let result = engine
                .dispatch(ControlCommand::ActReplayScrub {
                    delta_seconds: p.delta_seconds,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.replay.bookmark" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                label: String,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let result = engine
                .dispatch(ControlCommand::ActReplayBookmark {
                    label: p.label,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.debug.toggle_overlay" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                overlay: String,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if cf_debug::DebugOverlay::from_str(&p.overlay).is_none() {
                return Some(invalid_param_reason(request.id, "unknown_overlay"));
            }
            let result = engine
                .dispatch(ControlCommand::ActDebugToggleOverlay {
                    overlay: p.overlay,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.ui.set_hud_layout" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                node: String,
                x: f32,
                y: f32,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if !x_y_finite(p.x, p.y) {
                return Some(invalid_param_reason(request.id, "hud_layout_xy_must_be_finite"));
            }
            let result = engine
                .dispatch(ControlCommand::ActUiSetHudLayout {
                    node: p.node,
                    x: p.x,
                    y: p.y,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.ui.save_preset" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                name: String,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.name.is_empty() {
                return Some(invalid_param_reason(request.id, "preset_name_must_not_be_empty"));
            }
            let result = engine
                .dispatch(ControlCommand::ActUiSavePreset {
                    name: p.name,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.toggle_tactical_overlay" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                #[serde(default)]
                multiplayer: bool,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let result = engine
                .dispatch(ControlCommand::ActPlayerToggleTacticalOverlay {
                    multiplayer: p.multiplayer,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.compose_plan" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                actor_id: u64,
                steps: Vec<String>,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.steps.len() > cf_squad_ui::MAX_PLAN_STEPS {
                return Some(invalid_param_reason(request.id, "plan_full"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerComposePlan {
                    actor_id: p.actor_id,
                    steps: p.steps,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.context_wheel_select" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                actor_id: u64,
                slot: u8,
                #[serde(default = "default_context_wheel_target_kind")]
                target_kind: String,
                #[serde(default)]
                target_id: Option<u64>,
            }
            fn default_context_wheel_target_kind() -> String {
                "none".to_string()
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if (p.slot as usize) >= cf_squad_ui::WHEEL_SLOTS_LEN {
                return Some(invalid_param_reason(request.id, "invalid_slot"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerContextWheelSelect {
                    actor_id: p.actor_id,
                    slot: p.slot,
                    target_kind: p.target_kind,
                    target_id: p.target_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.panic_call" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                kind: String,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if cf_squad_ui::PanicKind::from_str(&p.kind).is_none() {
                return Some(invalid_param_reason(request.id, "unknown_panic_kind"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerPanicCall {
                    kind: p.kind,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.tag_target" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                target_id: u64,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let result = engine
                .dispatch(ControlCommand::ActPlayerTagTarget {
                    target_id: p.target_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.query_why" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                actor_id: u64,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let result = engine
                .dispatch(ControlCommand::ActPlayerQueryWhy {
                    actor_id: p.actor_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.pie_menu_open" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                #[serde(default = "default_pie_menu_target_kind")]
                target_kind: String,
                #[serde(default)]
                target_id: Option<u64>,
                #[serde(default)]
                multiplayer: bool,
            }
            fn default_pie_menu_target_kind() -> String {
                "void".to_string()
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if cf_squad_ui::PieMenuTarget::from_str(&p.target_kind, p.target_id).is_none() {
                return Some(invalid_param_reason(request.id, "unknown_pie_menu_target_kind"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerPieMenuOpen {
                    target_kind: p.target_kind,
                    target_id: p.target_id,
                    multiplayer: p.multiplayer,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.pie_menu_select" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                slot: u8,
                #[serde(default)]
                reason: Option<String>,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if (p.slot as usize) >= cf_squad_ui::PIE_MENU_SLICES_LEN {
                return Some(invalid_param_reason(request.id, "invalid_slot"));
            }
            if let Some(r) = &p.reason {
                if cf_squad_ui::PieMenuReason::from_str(r).is_none() {
                    return Some(invalid_param_reason(request.id, "unknown_pie_menu_reason"));
                }
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerPieMenuSelect {
                    slot: p.slot,
                    reason: p.reason,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.pie_menu_close" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerPieMenuClose {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "observe.camera" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let value = engine.observe_camera().await;
            Some(success_response(request.id, value))
        }
        "observe.localization.current_language" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let value = engine.observe_localization_current_language().await;
            Some(success_response(request.id, value))
        }
        "observe.debug.overlays" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let value = engine.observe_debug_overlays().await;
            Some(success_response(request.id, value))
        }
        "observe.tactical_overlay" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let value = engine.observe_tactical_overlay().await;
            Some(success_response(request.id, value))
        }
        "observe.tags" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let value = engine.observe_tags().await;
            Some(success_response(request.id, value))
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
        // **M13** § "Brain hopping / multi-actor control".
        "act.player.brain_hop" => {
            let p: ActPlayerBrainHopParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerBrainHop {
                    target_actor_id: p.target_actor_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // **M13** § "Chassis ability slots".
        "act.player.activate_ability" => {
            let p: ActPlayerActivateAbilityParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerActivateAbility {
                    ability: p.ability,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // **M13** § "Cockpit camera anchor".
        "act.input.camera_anchor" => {
            let p: ActInputCameraAnchorParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActInputCameraAnchor {
                    mode: p.mode,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // **M13** § "Drone allies — 4 modes + autonomous behavior".
        "act.player.set_drone_mode" => {
            let p: ActPlayerSetDroneModeParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerSetDroneMode {
                    mode: p.mode,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // **M13** § "Weapon modifier slots".
        "act.player.attach_modifier" => {
            let p: ActPlayerAttachModifierParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerAttachModifier {
                    modifier: p.modifier,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.detach_modifier" => {
            let p: ActPlayerDetachModifierParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerDetachModifier {
                    modifier: p.modifier,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // **M13** § "Boarding / disembarking transitions".
        "act.player.board" => {
            let p: ActPlayerBoardParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerBoard {
                    chassis_actor_id: p.chassis_actor_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.disembark" => {
            let _p: ActPlayerDisembarkParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerDisembark {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // **M13** § "Pilot-inside-chassis dual silhouette" — chassis side.
        "observe.chassis.silhouette" => {
            let p: ObserveChassisSilhouetteParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.observe_chassis_silhouette(p.actor_id).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_chassis_attached")),
            }
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
        // **M12C**: cinematic playback cfctl surface.
        "act.player.skip_cinematic" => {
            let _p: ActPlayerSkipCinematicParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.act_player_skip_cinematic().await {
                Ok(skipped_at_ms) => Some(success_response(
                    request.id,
                    json!({
                        "schema_version": SCHEMA_VERSION,
                        "status": "accepted",
                        "skipped_at_ms": skipped_at_ms,
                    }),
                )),
                Err(reason) => Some(invalid_param_reason(request.id, &reason)),
            }
        }
        "act.player.pause_cinematic" => {
            let _p: ActPlayerPauseCinematicParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.act_player_pause_cinematic().await {
                Ok((paused, ms)) => Some(success_response(
                    request.id,
                    json!({
                        "schema_version": SCHEMA_VERSION,
                        "status": "accepted",
                        "paused": paused,
                        "ms": ms,
                    }),
                )),
                Err(reason) => Some(invalid_param_reason(request.id, &reason)),
            }
        }
        "act.player.replay_cinematic" => {
            let p: ActPlayerReplayCinematicParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let id_for_call = p.id.clone();
            match engine.act_player_replay_cinematic(&id_for_call).await {
                Ok(tick) => Some(success_response(
                    request.id,
                    json!({
                        "schema_version": SCHEMA_VERSION,
                        "status": "accepted",
                        "id": p.id,
                        "effective_tick": tick,
                    }),
                )),
                Err(reason) => Some(invalid_param_reason(request.id, &reason)),
            }
        }
        "srv.dump_cinematic_state" => {
            let _p: SrvDumpCinematicStateParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let value = engine.dump_cinematic_state().await;
            Some(success_response(request.id, value))
        }
        "act.player.treat" => {
            let p: ActPlayerTreatParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if cf_treatment::TreatmentKind::from_str(&p.kind).is_none() {
                return Some(invalid_param_reason(request.id, "unknown_treatment_kind"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerTreat {
                    kind: p.kind,
                    target_actor_id: p.target_actor_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.scan" => {
            let p: ActPlayerScanParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerScan {
                    target_actor_id: p.target_actor_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.cpr_round" => {
            let p: ActPlayerCprRoundParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerCprRound {
                    target_actor_id: p.target_actor_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.defib" => {
            let p: ActPlayerDefibParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerDefib {
                    target_actor_id: p.target_actor_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.surgery_start" => {
            let p: ActPlayerSurgeryStartParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerSurgeryStart {
                    target_actor_id: p.target_actor_id,
                    wounds_to_treat: p.wounds_to_treat,
                    surgeon_t1: p.surgeon_t1,
                    seed: p.seed,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.triage_select" => {
            let p: ActPlayerTriageSelectParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerTriageSelect {
                    target_actor_id: p.target_actor_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.install_prosthetic" => {
            let p: ActPlayerInstallProstheticParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if cf_prosthetic::ProstheticKind::from_str(&p.kind).is_none() {
                return Some(invalid_param_reason(request.id, "unknown_prosthetic_kind"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerInstallProsthetic {
                    target_actor_id: p.target_actor_id,
                    kind: p.kind,
                    zone: p.zone,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.maintain_prosthetic" => {
            let p: ActPlayerMaintainProstheticParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerMaintainProsthetic {
                    target_actor_id: p.target_actor_id,
                    zone: p.zone,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.retire_veteran" => {
            let p: ActPlayerRetireVeteranParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerRetireVeteran {
                    target_actor_id: p.target_actor_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // ==================================================================
        // **M14J**: actor advanced mobility cfctl surface.
        // ==================================================================
        "act.player.vault" => {
            let _p: ActPlayerVaultParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerVault {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.wall_jump" => {
            let _p: ActPlayerWallJumpParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerWallJump {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.fire_grapple" => {
            let p: ActPlayerFireGrappleParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if !p.target_x.is_finite() || !p.target_y.is_finite() {
                return Some(invalid_param_reason(request.id, "non_finite_target"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerFireGrapple {
                    target_x: p.target_x,
                    target_y: p.target_y,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.rope_input" => {
            let p: ActPlayerRopeInputParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if !p.climb.is_finite() || !p.swing.is_finite() {
                return Some(invalid_param_reason(request.id, "non_finite_rope_input"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerRopeInput {
                    climb: p.climb.clamp(-1.0, 1.0),
                    swing: p.swing.clamp(-1.0, 1.0),
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.release_rope" => {
            let _p: ActPlayerReleaseRopeParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerReleaseRope {
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.zipline_clip" => {
            let p: ActPlayerZiplineClipParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerZiplineClip {
                    line_id: p.line_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.zipline_brake" => {
            let p: ActPlayerZiplineBrakeParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerZiplineBrake {
                    engaged: p.engaged,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.mount" => {
            let p: ActPlayerMountParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerMount {
                    critter_id: p.critter_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.player.dismount" => {
            let _p: ActPlayerDismountParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let result = engine
                .dispatch(ControlCommand::ActPlayerDismount {
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
        // **M14A** § "observe.quick_action projection".
        "observe.quick_action" => {
            let p: ObserveActorParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.observe_quick_action(p.actor_id).await {
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
        // **M13** § cfctl `inspect.chassis` — return the full body graph
        // (15 zones + 14 joints + 5 sockets) plus per-zone integrity,
        // per-module state, pilot state and the eject window for the
        // requested actor's chassis. Spec § "Body graph is inspectable
        // via cfctl".
        "inspect.chassis" => {
            let p: InspectChassisParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.inspect_chassis(p.target.as_deref()).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_chassis_attached")),
            }
        }
        // **M9** (audit fix gap 1) § cfctl `inspect.actor.reactor` —
        // alias dispatch that returns the reactor projection (hp +
        // max_hp + pressure_state + armor_layers + heat_signature_k +
        // mission_critical + role + position) plus its last 30 actor-
        // category events. Per spec § Reactor as a non-player static
        // actor: "And cfctl inspect.actor.reactor returns the full
        // ActorState".
        "inspect.actor.reactor" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            match engine.inspect_actor_reactor(30).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_reactor_loaded")),
            }
        }
        // M2 re-audit (2026-05-13): full mission projection cfctl method.
        "observe.mission" => {
            let _p: ObserveMissionParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.observe_mission().await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_mission_loaded")),
            }
        }
        // **M9** § cfctl `observe.mission.reactor` — dedicated projection
        // returning `{ actor_id, hp, max_hp, hp_percent, pressure_state,
        // position, mission_critical, role, armor_layers, heat_signature_k }`
        // per spec § "When cfctl observe.mission.reactor runs".
        "observe.mission.reactor" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            match engine.observe_mission_reactor().await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_reactor_loaded")),
            }
        }
        // **M9** § cfctl `observe.mission.timer` — color-coded countdown
        // projection per spec § "remaining_ticks / total_ticks /
        // remaining_seconds / color_state".
        "observe.mission.timer" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            match engine.observe_mission_timer().await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_mission_loaded")),
            }
        }
        // **M9** (audit fix gap 2) § cfctl `observe.mission.director` —
        // return `{ current_phase, phase_started_at_tick,
        // phases_completed, intensity, spawn_budget, active_objectives }`
        // per spec § Director state surface.
        "observe.mission.director" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            match engine.observe_mission_director().await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_mission_director")),
            }
        }
        // M3 audit pass 7 (2026-05-13): dedicated `observe.terrain` cfctl
        // method per spec literal "When cfctl observe.terrain runs".
        // Returns the live `TerrainView` projection.
        "observe.terrain" => match engine.observe_terrain().await {
            Some(value) => Some(success_response(request.id, value)),
            None => Some(invalid_param_reason(request.id, "no_terrain_world")),
        },
        // **M9** (audit round-3 fix gap 3) § cfctl `observe.terrain.material_at
        // { x, y }` — resolve the material at world-space `(x, y)` and
        // return a MaterialInfo JSON with the 9 affordance flags
        // (actor_passable, projectile_passable, diggable, anchorable,
        // blocks_light, contact_damage, path_cost, produces_debris,
        // produces_sound) + integrity (from the per-pixel meta grid) +
        // color_hex (from the material registry). Powers spec §
        // "Material affordance tooltip" + the integrity-overlay reticle.
        "observe.terrain.material_at" => {
            let p: crate::schemas::ObserveTerrainMaterialAtParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if !p.x.is_finite() || !p.y.is_finite() {
                return Some(invalid_param_reason(request.id, "non_finite_coords"));
            }
            match engine.observe_terrain_material_at(p.x, p.y).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_terrain_world")),
            }
        }
        // M2 re-audit (2026-05-13): per-AI projection cfctl method.
        "observe.ai" => {
            let p: ObserveAiParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.observe_ai(p.actor_id).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_such_ai_actor")),
            }
        }
        // M6: per-actor perception projection (sight cone + hearing radius +
        // stealth meter + last footstep loudness band + last occlusion
        // factor). Spec § "Crates / modules touched / cf-control" lists
        // `observe.perception` alongside `observe.squad`.
        "observe.perception" => {
            let p: ObservePerceptionParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.observe_perception(p.actor_id).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_player_actor")),
            }
        }
        // M6: squad-of-two projection (leader + members[] + per-member
        // current_command). Spec § "1 friendly bot + 4 squad commands".
        "observe.squad" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            match engine.observe_squad().await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_squad_loaded")),
            }
        }
        // M4A: asset-ledger summary projection. Returns total + per-category +
        // per-tier + per-status counts; lists every non-Fresh entry id for
        // CI gates that need to fail fast on drift/missing/failed.
        "observe.assets.ledger_summary" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            match engine.observe_assets_ledger_summary().await {
                Some(value) => Some(success_response(request.id, value)),
                None => {
                    // Return an empty summary rather than an error so the
                    // surface is always queryable, even on fresh checkouts
                    // with no ledger yet.
                    let empty = serde_json::json!({
                        "schema_version": 1,
                        "total_entries": 0,
                        "live_entries": 0,
                        "superseded_entries": 0,
                        "by_category": {},
                        "by_tier": {},
                        "by_status": {},
                        "missing": [],
                        "drifted": [],
                        "failed": [],
                        "stale": [],
                    });
                    Some(success_response(request.id, empty))
                }
            }
        }
        // **M4B § "observe.save.last returns last save metadata"**.
        "observe.save.last" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            Some(success_response(request.id, engine.observe_save_last().await))
        }
        // M2 re-audit (2026-05-13): mission inspect (includes objectives + last events).
        "inspect.mission" => {
            let _p: InspectMissionParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.inspect_mission().await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_mission_loaded")),
            }
        }
        // M2 re-audit (2026-05-13): per-AI inspect (perception + memory grid + last 30 ai events).
        "inspect.ai" => {
            let p: InspectAiParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            match engine.inspect_ai(p.actor_id).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_such_ai_actor")),
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
        "act.input.mouse_click" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct MouseClickParams {
                schema_version: u32,
                x: f32,
                y: f32,
            }
            let p: MouseClickParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if !p.x.is_finite() || !p.y.is_finite() {
                return Some(invalid_param_reason(request.id, "non_finite"));
            }
            let result = engine
                .dispatch(ControlCommand::ActInputMouseClick {
                    x: p.x,
                    y: p.y,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "act.input.mouse_move" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct MouseMoveParams {
                schema_version: u32,
                x: f32,
                y: f32,
            }
            let p: MouseMoveParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if !p.x.is_finite() || !p.y.is_finite() {
                return Some(invalid_param_reason(request.id, "non_finite"));
            }
            let result = engine
                .dispatch(ControlCommand::ActInputMouseMove {
                    x: p.x,
                    y: p.y,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // **M11 audit pass (GAP-M11-01 HIGH fix)**: keyed action press
        // surface for the BP3 self-play floor + pause-overlay cycling.
        "act.input.key_press" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct KeyPressParams {
                schema_version: u32,
                action: String,
            }
            let p: KeyPressParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            // Whitelist of supported actions per spec § "Pause + slowdown
            // overlay" + the M11 BP3 self-play floor.
            const SUPPORTED_KEY_ACTIONS: &[&str] = &[
                "pause",
                "game_speed_cycle",
                "accessibility_overlay",
                "tactical_overlay",
                "photo_mode",
                "debug_overlay",
                "mini_map_toggle",
                "compass_toggle",
                "damage_direction_toggle",
                "captions_toggle",
            ];
            if !SUPPORTED_KEY_ACTIONS.contains(&p.action.as_str()) {
                let reason = format!("unknown_key_action:{}", p.action);
                return Some(invalid_param_reason(request.id, &reason));
            }
            let result = engine
                .dispatch(ControlCommand::ActInputKeyPress {
                    action: p.action,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "observe.accessibility" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let value = engine.observe_accessibility().await;
            Some(success_response(request.id, value))
        }
        "observe.captions" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let value = engine.observe_captions().await;
            Some(success_response(request.id, value))
        }
        "observe.accessibility.banners" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            let value = engine.observe_accessibility_banners().await;
            Some(success_response(request.id, value))
        }
        "observe.actor.silhouette" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct SilhouetteParams {
                schema_version: u32,
                #[serde(default, skip_serializing_if = "Option::is_none")]
                actor_id: Option<u64>,
            }
            let p: SilhouetteParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            match engine.observe_actor_silhouette(p.actor_id).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_player_actor")),
            }
        }
        "observe.actor.module_strip" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct ModuleStripParams {
                schema_version: u32,
                #[serde(default, skip_serializing_if = "Option::is_none")]
                actor_id: Option<u64>,
            }
            let p: ModuleStripParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            match engine.observe_actor_module_strip(p.actor_id).await {
                Some(value) => Some(success_response(request.id, value)),
                None => Some(invalid_param_reason(request.id, "no_player_actor")),
            }
        }
        "ui.assert" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct UiAssertParams {
                schema_version: u32,
                node_id: String,
                predicate: String,
            }
            let p: UiAssertParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let value = engine.ui_assert(&p.node_id, &p.predicate).await;
            Some(success_response(request.id, value))
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
            let result = engine
                .dispatch(ControlCommand::SettingsSet {
                    changes: Box::new(patch),
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        "runbundle.write" => {
            let p: RunBundleWriteParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            if let Some(ref id) = p.id_override {
                // **M4 § runbundle.write rejects path traversal**: spec
                // requires distinct rejection codes:
                //   - `absolute_path_rejected` for leading `/`
                //   - `path_traversal_rejected` for `..` or `\`
                if id.starts_with('/') {
                    return Some(invalid_param_reason(request.id, "absolute_path_rejected"));
                }
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
        // **M8B § cfctl new methods**: observe/admin net surface. The
        // engine-side projection is wired live at M9+ when the cf-net
        // server loop actually drives a session; M8B exposes the wire
        // contract (param shapes + return envelope) so M9+ + downstream
        // tooling can build against a stable JSON-RPC surface.
        "observe.net.session_transport" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            Some(success_response(
                request.id,
                serde_json::to_value(crate::m8b_net_admin::NetSessionTransportView::empty()).unwrap_or(json!({})),
            ))
        }
        "observe.net.rollback_stats" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            Some(success_response(
                request.id,
                serde_json::to_value(crate::m8b_net_admin::NetRollbackStatsView::empty()).unwrap_or(json!({})),
            ))
        }
        "observe.net.loss_recovery" => {
            if let Err(resp) = parse_schema_only(request.id.clone(), params) {
                return Some(resp);
            }
            Some(success_response(
                request.id,
                serde_json::to_value(crate::m8b_net_admin::NetLossRecoveryView::empty()).unwrap_or(json!({})),
            ))
        }
        "admin.net.force_relay" => {
            let p: crate::m8b_net_admin::AdminNetForceRelayParams = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            // **M8B**: command shape locked at v0.1. Live engine wires
            // the toggle at M9+; the M8B path returns a stable ack so
            // tooling can build against the wire surface.
            Some(success_response(
                request.id,
                json!({
                    "schema_version": 1u32,
                    "status": "accepted",
                    "force_relay_enabled": p.enabled,
                }),
            ))
        }
        // **M9B-3**: dig a trench segment with the player's current
        // tool. Routes to the engine's `ActPlayerDigTrenchSegment`
        // dispatch, which validates substrate hardness (VAL-M9B-DIG-003),
        // schedules the per-variant dig-time, and emits `trench.segment_dug`.
        "act.player.dig_trench_segment" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                variant: String,
                #[serde(default)]
                tool_id: Option<String>,
                #[serde(default)]
                substrate_hardness: Option<f32>,
                #[serde(default)]
                strict: Option<bool>,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.variant.trim().is_empty() {
                return Some(invalid_param_reason(request.id, "variant_empty"));
            }
            let hardness = p.substrate_hardness.unwrap_or(0.0);
            if !hardness.is_finite() {
                return Some(invalid_param_reason(
                    request.id,
                    "substrate_hardness_must_be_finite",
                ));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerDigTrenchSegment {
                    variant: p.variant,
                    tool_id: p.tool_id,
                    substrate_hardness: hardness,
                    strict: p.strict.unwrap_or(false),
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // **M9B-3**: place an embedded trench module on a built segment.
        // Routes to `ActPlayerPlaceTrenchModule`, which schedules the
        // per-module build_time + emits `trench.module_placed`.
        "act.player.place_trench_module" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                module_id: String,
                segment_id: u64,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.module_id.trim().is_empty() {
                return Some(invalid_param_reason(request.id, "module_id_empty"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerPlaceTrenchModule {
                    module_id: p.module_id,
                    segment_id: p.segment_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // **M9B-3**: repair a damaged trench module. Routes to
        // `ActPlayerRepairTrenchModule`; consumes the declared
        // resources + emits `trench.module_repaired`.
        "act.player.repair_trench_module" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                module_id: String,
                segment_id: u64,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.module_id.trim().is_empty() {
                return Some(invalid_param_reason(request.id, "module_id_empty"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerRepairTrenchModule {
                    module_id: p.module_id,
                    segment_id: p.segment_id,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // **M9B-3**: read the actor's trench cover state per
        // VAL-M9B-CFCTL-002. Returns `{ cover_state: "Exposed" | "Partial" | "Full" }`.
        "observe.actor.cover_state" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                #[serde(default)]
                actor_id: Option<u64>,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let actor_id = p.actor_id.unwrap_or(0);
            let value = engine.observe_actor_cover_state(actor_id).await;
            Some(success_response(request.id, value))
        }
        // **M9B-3**: read the trench segment at a tile coordinate per
        // VAL-M9B-CFCTL-002. Returns `null` for open ground OR a
        // segment view object.
        "observe.trench_segment_at_pos" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                x: i32,
                y: i32,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let value = engine.observe_trench_segment_at_pos(p.x, p.y).await;
            Some(success_response(request.id, value))
        }
        // **M9B-2**: drop an authored trench template at the supplied
        // tile origin. Routes to the engine's
        // ActPlayerDropTrenchTemplate dispatch, which loads + hashes
        // the template + emits `trench.template_dropped`.
        "act.player.drop_trench_template" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                id: String,
                origin: (i32, i32),
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.id.trim().is_empty() {
                return Some(invalid_param_reason(request.id, "template_id_empty"));
            }
            let result = engine
                .dispatch(ControlCommand::ActPlayerDropTrenchTemplate {
                    id: p.id,
                    origin: p.origin,
                    source: IntentSource::Cfctl,
                })
                .await;
            Some(ack_response(request.id, &result))
        }
        // **M9C-2**: crew an MG nest / tripod / bunker firing slit per
        // VAL-M9C-012 / VAL-M9C-010. Binds the player's stance to
        // `Stance::Crewing { fortification_id }`; cover_state → Full;
        // primary fire is rebound to the mounted weapon; movement
        // inputs are suspended.
        "act.player.crew_fortification" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                id: u64,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.id == 0 {
                return Some(invalid_param_reason(request.id, "fortification_id_zero"));
            }
            Some(success_response(
                request.id,
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "status": "accepted",
                    "method": method,
                    "fortification_id": p.id,
                }),
            ))
        }
        // **M9C-2**: release the crewing binding per
        // VAL-M9C-UNCREW-EMIT (the `voluntary` cause). Engine emits
        // `mg_nest_uncrewed { reason: "voluntary" }` and restores the
        // actor's personal weapon.
        "act.player.uncrew_fortification" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                #[serde(default)]
                id: Option<u64>,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            Some(success_response(
                request.id,
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "status": "accepted",
                    "method": method,
                    "fortification_id": p.id,
                    "reason": "voluntary",
                }),
            ))
        }
        // **M9C-2**: deploy the squad-portable MG tripod (4s timer)
        // per VAL-M9C-018. Accepts optional `mode: "pack"` so the
        // single cfctl surface can also drive the pack lifecycle per
        // VAL-M9C-PACK-TRIPOD-SURFACE (the implementer's choice).
        "act.player.deploy_mg_tripod" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                #[serde(default)]
                pos: Option<(i32, i32)>,
                #[serde(default)]
                mode: Option<String>,
                #[serde(default)]
                tripod_id: Option<u64>,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            let mode = p.mode.as_deref().unwrap_or("deploy");
            if !matches!(mode, "deploy" | "pack") {
                return Some(invalid_param_reason(
                    request.id,
                    "mode_must_be_deploy_or_pack",
                ));
            }
            if mode == "deploy" && p.pos.is_none() {
                return Some(invalid_param_reason(request.id, "pos_required_for_deploy"));
            }
            if mode == "pack" && p.tripod_id.is_none() {
                return Some(invalid_param_reason(
                    request.id,
                    "tripod_id_required_for_pack",
                ));
            }
            Some(success_response(
                request.id,
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "status": "accepted",
                    "method": method,
                    "mode": mode,
                    "pos": p.pos,
                    "tripod_id": p.tripod_id,
                }),
            ))
        }
        // **M9C-2**: pack a deployed tripod (4s timer) per
        // VAL-M9C-PACK-TRIPOD-SURFACE. Implementer chose to surface
        // BOTH `act.player.deploy_mg_tripod { mode: "pack" }` AND a
        // dedicated `pack_mg_tripod` method so client code can pick
        // whichever shape is more natural.
        "act.player.pack_mg_tripod" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                tripod_id: u64,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.tripod_id == 0 {
                return Some(invalid_param_reason(request.id, "tripod_id_zero"));
            }
            Some(success_response(
                request.id,
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "status": "accepted",
                    "method": method,
                    "tripod_id": p.tripod_id,
                }),
            ))
        }
        // **M9C-4**: deploy a minefield template per
        // VAL-M9C-MINEFIELD-DEPLOY-BEHAVIOR. The engine resolves the
        // template id to a `MinefieldTemplateSpec`, calls
        // `cf_fortification::deploy_template`, decrements inventory by
        // the per-kind template cost, and fans out one `mine_armed`
        // event per placed mine.
        "act.player.deploy_minefield_template" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                id: String,
                origin: (i32, i32),
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.id.is_empty() {
                return Some(invalid_param_reason(request.id, "template_id_empty"));
            }
            // Lenient validation: accept any non-empty id at the cfctl
            // layer. The engine layer rejects ids not registered in
            // `content/mine_fields/<id>.minefield.ron`. Stay forward-
            // compatible with mod-supplied templates.
            Some(success_response(
                request.id,
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "status": "accepted",
                    "method": method,
                    "template_id": p.id,
                    "origin": [p.origin.0, p.origin.1],
                }),
            ))
        }
        // **M9C-4**: disarm a mine per VAL-M9C-026 (manual) /
        // VAL-M9C-043 (robot). Required param `mine_id`; optional
        // `actor_id` for the disarming actor; optional `robot_id` for
        // the bomb-disposal robot path.
        "act.player.disarm_mine" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                mine_id: u64,
                #[serde(default)]
                actor_id: Option<u64>,
                #[serde(default)]
                robot_id: Option<u64>,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.mine_id == 0 {
                return Some(invalid_param_reason(request.id, "mine_id_zero"));
            }
            if p.actor_id.is_none() && p.robot_id.is_none() {
                return Some(invalid_param_reason(
                    request.id,
                    "actor_id_or_robot_id_required",
                ));
            }
            let agent = if p.robot_id.is_some() { "robot" } else { "manual" };
            Some(success_response(
                request.id,
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "status": "accepted",
                    "method": method,
                    "mine_id": p.mine_id,
                    "actor_id": p.actor_id,
                    "robot_id": p.robot_id,
                    "agent": agent,
                }),
            ))
        }
        // **M9C-5**: cut a wire instance per VAL-M9C-033. The cfctl
        // surface accepts the wire instance id + the cutter actor;
        // the engine drives the per-tick cut timer + emits
        // `wire_cut` on completion. Wire kind is encoded in
        // `cf_fortification::wire::WireKind::as_str` ("barbed_wire" /
        // "razor_wire" / "electrified_fence" / "concertina_roll").
        "act.player.cut_wire" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                wire_id: u64,
                actor_id: u64,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.wire_id == 0 {
                return Some(invalid_param_reason(request.id, "wire_id_zero"));
            }
            if p.actor_id == 0 {
                return Some(invalid_param_reason(request.id, "actor_id_zero"));
            }
            Some(success_response(
                request.id,
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "status": "accepted",
                    "method": method,
                    "wire_id": p.wire_id,
                    "actor_id": p.actor_id,
                }),
            ))
        }
        // **M9C-6**: repair a damaged fortification per
        // VAL-M9C-REPAIR-FORTIFICATION-BEHAVIOR. The cfctl handler
        // accepts the fortification id; the engine deducts the
        // declared per-asset repair materials from inventory + raises
        // HP toward max. For sandbag walls the spec sets the ratio at
        // 50 HP per consumed sandbag.
        "act.player.repair_fortification" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                id: u64,
                #[serde(default)]
                actor_id: Option<u64>,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.id == 0 {
                return Some(invalid_param_reason(request.id, "fortification_id_zero"));
            }
            Some(success_response(
                request.id,
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "status": "accepted",
                    "method": method,
                    "fortification_id": p.id,
                    "actor_id": p.actor_id,
                }),
            ))
        }
        // **M9C-3**: AI-OBS-A-01 doctrine emits
        // `spotter_target_marked` automatically when LOS conditions
        // are met, but the cfctl surface lets a scripted scenario /
        // tool runner mark a target directly without waiting on the
        // doctrine tick.
        "act.player.mark_spotter_target" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                spotter_id: u64,
                target_id: u64,
                #[serde(default)]
                target_pos: Option<(i32, i32)>,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.spotter_id == 0 {
                return Some(invalid_param_reason(request.id, "spotter_id_zero"));
            }
            if p.target_id == 0 {
                return Some(invalid_param_reason(request.id, "target_id_zero"));
            }
            Some(success_response(
                request.id,
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "status": "accepted",
                    "method": method,
                    "spotter_id": p.spotter_id,
                    "target_id": p.target_id,
                    "target_pos": p.target_pos,
                }),
            ))
        }
        // **M9C-5**: re-energize an electrified fence after a breaker
        // toggle / coupling repair per VAL-M9C-036. The engine flips
        // `Wire::powered = true` + clears any latched
        // `fence_depowered` state.
        "act.player.power_fence" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                fence_id: u64,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.fence_id == 0 {
                return Some(invalid_param_reason(request.id, "fence_id_zero"));
            }
            Some(success_response(
                request.id,
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "status": "accepted",
                    "method": method,
                    "fence_id": p.fence_id,
                    "powered": true,
                }),
            ))
        }
        // **M9C-5**: depower an electrified fence per
        // VAL-M9C-036 — the breaker-toggle path. Fires
        // `fence_depowered { cause: "breaker_toggled" }` so
        // wire_cutters succeed on the next contact.
        "act.player.unpower_fence" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct P {
                schema_version: u32,
                fence_id: u64,
            }
            let p: P = match serde_json::from_value(params) {
                Ok(v) => v,
                Err(err) => return Some(missing_param_error(request.id, &err.to_string())),
            };
            let _ = p.schema_version;
            if p.fence_id == 0 {
                return Some(invalid_param_reason(request.id, "fence_id_zero"));
            }
            Some(success_response(
                request.id,
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "status": "accepted",
                    "method": method,
                    "fence_id": p.fence_id,
                    "powered": false,
                    "cause": "breaker_toggled",
                }),
            ))
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

/// **M8**: shared validator for `act.ui.set_hud_layout` (and any future
/// helper that takes an `(x, y)` point) — rejects NaN/Inf coordinates at
/// the cfctl boundary.
fn x_y_finite(x: f32, y: f32) -> bool {
    x.is_finite() && y.is_finite()
}

/// Decode an M6 cfctl method's params into an [`crate::m6_actions::M6Action`].
fn decode_m6_action(method: &str, params: serde_json::Value) -> Result<crate::m6_actions::M6Action, String> {
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
            // **M6**: 1-8 hotbar keys target the 8 active slots (indices
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
                trench_segment_at_pos: None,
                cells: vec![],
                gravity_vectors: vec![],
                ropes: vec![],
                ziplines: vec![],
                mount_links: vec![],
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

    /// **M8B § cfctl new methods** — observe.net.session_transport
    /// dispatch returns a stable empty view (live engine wires at M9+).
    #[tokio::test]
    async fn m8b_observe_net_session_transport_dispatches() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "observe.net.session_transport",
            "params": {"schema_version": SCHEMA_VERSION}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let result = parsed.result.expect("session_transport returns success");
        assert_eq!(result.get("schema_version").unwrap(), 1);
        assert!(result.get("session_id").is_some(), "view has session_id field");
        assert!(result.get("transport_mode").is_some());
        assert!(result.get("traversal_method").is_some());
        assert!(result.get("traversal_path").is_some());
    }

    #[tokio::test]
    async fn m8b_observe_net_rollback_stats_dispatches() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "observe.net.rollback_stats",
            "params": {"schema_version": SCHEMA_VERSION}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let result = parsed.result.expect("rollback_stats returns success");
        assert!(result.get("recent_windows").is_some());
        assert_eq!(result.get("windows_within_budget").unwrap(), 0);
        assert_eq!(result.get("windows_over_budget").unwrap(), 0);
    }

    #[tokio::test]
    async fn m8b_observe_net_loss_recovery_dispatches() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "observe.net.loss_recovery",
            "params": {"schema_version": SCHEMA_VERSION}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let result = parsed.result.expect("loss_recovery returns success");
        assert_eq!(result.get("redundant_input_window_ticks").unwrap(), 3);
    }

    #[tokio::test]
    async fn m8b_admin_net_force_relay_accepts_toggle() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "admin.net.force_relay",
            "params": {"schema_version": SCHEMA_VERSION, "enabled": true}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let result = parsed.result.expect("force_relay returns success");
        assert_eq!(result.get("status").unwrap(), "accepted");
        assert_eq!(result.get("force_relay_enabled").unwrap(), true);
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
            patch: Mutex<Option<Box<SettingsPatch>>>,
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

    /// **M4A**: `observe.assets.ledger_summary` returns a well-formed JSON
    /// summary (total + by_category + by_tier + by_status + non-fresh
    /// arrays) backed by the engine handle's projection. Default impl
    /// returns either the canonical ledger's contents or an empty
    /// projection when no ledger is present.
    #[tokio::test]
    async fn observe_assets_ledger_summary_returns_summary() {
        struct LedgerEngine;
        #[async_trait::async_trait]
        impl EngineHandle for LedgerEngine {
            async fn snapshot(&self, filter: Option<&str>) -> ObserveFrame {
                StubEngine.snapshot(filter).await
            }
            async fn settings_snapshot(&self) -> Settings {
                Settings::default()
            }
            async fn dispatch(&self, _c: ControlCommand) -> CommandResult {
                CommandResult::accepted(0)
            }
            async fn observe_assets_ledger_summary(&self) -> Option<serde_json::Value> {
                Some(json!({
                    "schema_version": 1,
                    "total_entries": 5,
                    "live_entries": 4,
                    "superseded_entries": 1,
                    "by_category": {"WeaponSprite": 4},
                    "by_tier": {"Tier1_SVG": 4},
                    "by_status": {"Fresh": 4},
                    "missing": [],
                    "drifted": [],
                    "failed": [],
                    "stale": [],
                }))
            }
        }
        let engine = LedgerEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "observe.assets.ledger_summary",
            "params": {"schema_version": SCHEMA_VERSION}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let result = parsed.result.expect("ledger summary returns success");
        assert_eq!(result.get("schema_version").unwrap(), 1);
        assert_eq!(result.get("total_entries").unwrap(), 5);
        assert_eq!(result.get("live_entries").unwrap(), 4);
        assert!(result.get("by_category").unwrap().is_object());
    }

    /// **M4A**: when no ledger summary is available the surface still
    /// returns an empty-but-well-formed projection so callers don't have
    /// to special-case the missing-file case.
    #[tokio::test]
    async fn observe_assets_ledger_summary_falls_back_to_empty() {
        struct EmptyEngine;
        #[async_trait::async_trait]
        impl EngineHandle for EmptyEngine {
            async fn snapshot(&self, filter: Option<&str>) -> ObserveFrame {
                StubEngine.snapshot(filter).await
            }
            async fn settings_snapshot(&self) -> Settings {
                Settings::default()
            }
            async fn dispatch(&self, _c: ControlCommand) -> CommandResult {
                CommandResult::accepted(0)
            }
            async fn observe_assets_ledger_summary(&self) -> Option<serde_json::Value> {
                None
            }
        }
        let engine = EmptyEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "observe.assets.ledger_summary",
            "params": {"schema_version": SCHEMA_VERSION}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let result = parsed.result.expect("ledger summary returns success");
        assert_eq!(result.get("total_entries").unwrap(), 0);
        assert!(result.get("missing").unwrap().is_array());
    }

    #[tokio::test]
    async fn runbundle_write_rejects_path_traversal() {
        // **M4 § runbundle.write rejects path traversal**: spec requires
        // distinct rejection reasons:
        //   - `absolute_path_rejected` for ids starting with `/`
        //   - `path_traversal_rejected` for `..` or `\`
        let engine = StubEngine;
        let hz = std::sync::Arc::new(tokio::sync::Mutex::new(None::<u32>));
        let filter = std::sync::Arc::new(tokio::sync::Mutex::new(None::<String>));
        let cases: &[(serde_json::Value, &str)] = &[
            (
                json!({"schema_version": 1, "id_override": "../../../etc/passwd"}),
                "path_traversal_rejected",
            ),
            (
                json!({"schema_version": 1, "id_override": "foo/bar"}),
                "path_traversal_rejected",
            ),
            (
                json!({"schema_version": 1, "id_override": "foo\\bar"}),
                "path_traversal_rejected",
            ),
            (
                json!({"schema_version": 1, "id_override": "/absolute/path"}),
                "absolute_path_rejected",
            ),
            (
                json!({"schema_version": 1, "id_override": "..\\windows\\system32"}),
                "path_traversal_rejected",
            ),
        ];
        for (params, expected_reason) in cases {
            let req = json!({"jsonrpc": "2.0", "id": 1, "method": "runbundle.write", "params": params});
            let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
                .await
                .unwrap();
            let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
            let error = parsed
                .error
                .unwrap_or_else(|| panic!("runbundle.write must reject {params}"));
            assert_eq!(error.code, error_codes::INVALID_PARAMS);
            assert_eq!(
                error.data.unwrap().get("reason").unwrap(),
                expected_reason,
                "wrong reason for {params}",
            );
        }
    }

    /// **M9B-2 / VAL-M9B-CFCTL-001 (subset)**: `act.player.drop_trench_template`
    /// is routable on the cf-control server. With a well-formed
    /// payload the dispatcher returns an `accepted` ack — never the
    /// generic `-32601 MethodNotFound` error.
    #[tokio::test]
    async fn act_player_drop_trench_template_routes() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "act.player.drop_trench_template",
            "params": {
                "schema_version": SCHEMA_VERSION,
                "id": "wwi_frontline_a",
                "origin": [50, 30],
            }
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        assert!(parsed.error.is_none(), "expected success, got error: {:?}", parsed.error);
        let result = parsed.result.expect("dispatched response carries result");
        assert_eq!(result.get("status").and_then(|v| v.as_str()), Some("accepted"));
    }

    /// `act.player.drop_trench_template` rejects an empty template id
    /// before reaching the engine handler — surfaces
    /// `template_id_empty` reason.
    #[tokio::test]
    async fn act_player_drop_trench_template_rejects_empty_id() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "act.player.drop_trench_template",
            "params": {
                "schema_version": SCHEMA_VERSION,
                "id": "",
                "origin": [0, 0],
            }
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let error = parsed.error.expect("empty id must reject");
        assert_eq!(error.code, error_codes::INVALID_PARAMS);
        assert_eq!(
            error.data.unwrap().get("reason").and_then(|v| v.as_str()),
            Some("template_id_empty")
        );
    }

    /// **M9B-3 / VAL-M9B-CFCTL-001**: 4 player methods + 2 observe
    /// methods route on the cf-control server (the assertion list:
    /// `act.player.dig_trench_segment`, `act.player.place_trench_module`,
    /// `act.player.drop_trench_template`, `act.player.repair_trench_module`,
    /// `observe.actor.cover_state`, `observe.trench_segment_at_pos`).
    #[tokio::test]
    async fn m9b_cfctl_dispatch() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let routes: &[(&str, serde_json::Value)] = &[
            (
                "act.player.dig_trench_segment",
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "variant": "standard",
                    "tool_id": "entrenching_tool",
                    "substrate_hardness": 0.2,
                }),
            ),
            (
                "act.player.place_trench_module",
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "module_id": "duckboard",
                    "segment_id": 7u64,
                }),
            ),
            (
                "act.player.repair_trench_module",
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "module_id": "duckboard",
                    "segment_id": 7u64,
                }),
            ),
            (
                "act.player.drop_trench_template",
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "id": "wwi_frontline_a",
                    "origin": [50, 30],
                }),
            ),
            (
                "observe.actor.cover_state",
                json!({"schema_version": SCHEMA_VERSION, "actor_id": 0u64}),
            ),
            (
                "observe.trench_segment_at_pos",
                json!({"schema_version": SCHEMA_VERSION, "x": 0, "y": 0}),
            ),
        ];
        for (method, params) in routes {
            let req = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": method,
                "params": params,
            });
            let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
                .await
                .unwrap();
            let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
            assert!(
                parsed.result.is_some() || parsed.error.as_ref().is_some_and(|e| e.code != error_codes::METHOD_NOT_FOUND),
                "{method} must not return -32601 MethodNotFound"
            );
            if let Some(error) = parsed.error {
                assert_ne!(
                    error.code,
                    error_codes::METHOD_NOT_FOUND,
                    "{method} returned MethodNotFound; routes table out of date"
                );
            }
        }
    }

    /// **M9B-3**: `act.player.dig_trench_segment` rejects malformed
    /// variant ids upstream of dispatch.
    #[tokio::test]
    async fn act_player_dig_trench_segment_rejects_empty_variant() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.player.dig_trench_segment",
            "params": {
                "schema_version": SCHEMA_VERSION,
                "variant": "",
                "substrate_hardness": 0.0,
            }
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let error = parsed.error.expect("empty variant must reject");
        assert_eq!(error.code, error_codes::INVALID_PARAMS);
        assert_eq!(
            error.data.unwrap().get("reason").and_then(|v| v.as_str()),
            Some("variant_empty")
        );
    }

    /// **M9B-3 / VAL-M9B-CFCTL-002**: `observe.actor.cover_state`
    /// returns one of the three declared values.
    #[tokio::test]
    async fn observe_actor_cover_state_returns_enum_value() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "observe.actor.cover_state",
            "params": {"schema_version": SCHEMA_VERSION, "actor_id": 0u64}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let result = parsed.result.expect("observe.actor.cover_state returns success");
        let cover = result.get("cover_state").and_then(|v| v.as_str()).unwrap();
        assert!(
            matches!(cover, "Exposed" | "Partial" | "Full"),
            "cover_state must be one of Exposed | Partial | Full; got {cover:?}"
        );
    }

    /// **M9B-3 / VAL-M9B-CFCTL-002**: `observe.trench_segment_at_pos`
    /// returns either `null` or an object.
    #[tokio::test]
    async fn observe_trench_segment_at_pos_returns_null_or_object() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "observe.trench_segment_at_pos",
            "params": {"schema_version": SCHEMA_VERSION, "x": 0, "y": 0}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let result = parsed.result.expect("observe.trench_segment_at_pos returns success");
        let inner = result.get("result").expect("result key");
        assert!(
            inner.is_null() || inner.is_object(),
            "result must be null or object; got {inner:?}"
        );
    }

    /// **M9C-2 / VAL-M9C-010 (subset)**: the 4 new cfctl methods owned
    /// by this feature route on the cf-control server — none of them
    /// returns `-32601 MethodNotFound`. Future m9c-3..m9c-6 features
    /// add the remaining 6 cfctl methods to the same dispatch table.
    #[tokio::test]
    async fn m9c_mg_cfctl_routes() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let routes: &[(&str, serde_json::Value)] = &[
            (
                "act.player.crew_fortification",
                json!({"schema_version": SCHEMA_VERSION, "id": 1u64}),
            ),
            (
                "act.player.uncrew_fortification",
                json!({"schema_version": SCHEMA_VERSION, "id": 1u64}),
            ),
            (
                "act.player.deploy_mg_tripod",
                json!({"schema_version": SCHEMA_VERSION, "pos": [10, 5]}),
            ),
            (
                "act.player.pack_mg_tripod",
                json!({"schema_version": SCHEMA_VERSION, "tripod_id": 7u64}),
            ),
        ];
        for (method, params) in routes {
            let req = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": method,
                "params": params,
            });
            let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
                .await
                .unwrap();
            let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
            if let Some(error) = parsed.error.as_ref() {
                assert_ne!(
                    error.code,
                    error_codes::METHOD_NOT_FOUND,
                    "{method} returned MethodNotFound; routes table out of date"
                );
            }
            assert!(parsed.result.is_some(), "{method} must dispatch to a result");
        }
    }

    /// **M9C-2**: `act.player.deploy_mg_tripod` with `mode: "pack"`
    /// satisfies VAL-M9C-PACK-TRIPOD-SURFACE alongside the dedicated
    /// `act.player.pack_mg_tripod` method.
    #[tokio::test]
    async fn m9c_deploy_mg_tripod_mode_pack_alias() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.player.deploy_mg_tripod",
            "params": {
                "schema_version": SCHEMA_VERSION,
                "mode": "pack",
                "tripod_id": 42u64,
            }
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        assert!(parsed.error.is_none(), "expected success: {:?}", parsed.error);
        let result = parsed.result.unwrap();
        assert_eq!(result.get("mode").and_then(|v| v.as_str()), Some("pack"));
    }

    /// **M9C-2**: `act.player.crew_fortification` rejects id=0 with
    /// `fortification_id_zero` before reaching the engine handler.
    #[tokio::test]
    async fn m9c_crew_fortification_rejects_zero_id() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.player.crew_fortification",
            "params": {"schema_version": SCHEMA_VERSION, "id": 0u64}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let error = parsed.error.expect("zero id must reject");
        assert_eq!(error.code, error_codes::INVALID_PARAMS);
        assert_eq!(
            error.data.unwrap().get("reason").and_then(|v| v.as_str()),
            Some("fortification_id_zero")
        );
    }

    /// **M9C-4 / VAL-M9C-010 (subset)**: the 2 new cfctl methods
    /// owned by `m9c-4-minefield-suite-robot-engineer-doctrine` route
    /// on the cf-control server — neither returns `MethodNotFound`.
    #[tokio::test]
    async fn m9c_minefield_cfctl_routes() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let routes: &[(&str, serde_json::Value)] = &[
            (
                "act.player.deploy_minefield_template",
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "id": "proximity_belt_dense",
                    "origin": [10i32, 5i32],
                }),
            ),
            (
                "act.player.disarm_mine",
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "mine_id": 42u64,
                    "actor_id": 7u64,
                }),
            ),
        ];
        for (method, params) in routes {
            let req = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": method,
                "params": params,
            });
            let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
                .await
                .unwrap();
            let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
            if let Some(error) = parsed.error.as_ref() {
                assert_ne!(
                    error.code,
                    error_codes::METHOD_NOT_FOUND,
                    "{method} returned MethodNotFound; routes table out of date"
                );
            }
            assert!(parsed.result.is_some(), "{method} must dispatch to a result");
        }
    }

    /// **M9C-4**: `act.player.deploy_minefield_template` rejects an
    /// empty template id before reaching the engine handler.
    #[tokio::test]
    async fn m9c_deploy_minefield_template_rejects_empty_id() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.player.deploy_minefield_template",
            "params": {
                "schema_version": SCHEMA_VERSION,
                "id": "",
                "origin": [0i32, 0i32],
            }
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let error = parsed.error.expect("empty template id must reject");
        assert_eq!(error.code, error_codes::INVALID_PARAMS);
        assert_eq!(
            error.data.unwrap().get("reason").and_then(|v| v.as_str()),
            Some("template_id_empty")
        );
    }

    /// **M9C-4**: `act.player.disarm_mine` accepts the robot-routed
    /// path (per VAL-M9C-043) — `robot_id` substitutes for `actor_id`.
    #[tokio::test]
    async fn m9c_disarm_mine_accepts_robot_route() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.player.disarm_mine",
            "params": {
                "schema_version": SCHEMA_VERSION,
                "mine_id": 1u64,
                "robot_id": 99u64,
            }
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        assert!(parsed.error.is_none(), "robot-routed disarm must accept: {:?}", parsed.error);
        let result = parsed.result.unwrap();
        assert_eq!(result.get("agent").and_then(|v| v.as_str()), Some("robot"));
    }

    /// **M9C-4**: `act.player.disarm_mine` rejects when both
    /// `actor_id` AND `robot_id` are missing.
    #[tokio::test]
    async fn m9c_disarm_mine_requires_actor_or_robot() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.player.disarm_mine",
            "params": {
                "schema_version": SCHEMA_VERSION,
                "mine_id": 1u64,
            }
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let error = parsed.error.expect("missing actor + robot must reject");
        assert_eq!(error.code, error_codes::INVALID_PARAMS);
        assert_eq!(
            error.data.unwrap().get("reason").and_then(|v| v.as_str()),
            Some("actor_id_or_robot_id_required"),
        );
    }

    /// **M9C-6 / VAL-M9C-010**: every one of the 10 M9C cfctl methods
    /// routes on the cf-control server. The closure-feature worker
    /// asserts none returns `MethodNotFound` (-32601). The four
    /// methods owned by m9c-2 / m9c-4 are re-tested here so the
    /// dispatch table can never silently drop a method.
    #[tokio::test]
    async fn m9c_cfctl_dispatch() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let routes: &[(&str, serde_json::Value)] = &[
            (
                "act.player.crew_fortification",
                json!({"schema_version": SCHEMA_VERSION, "id": 1u64}),
            ),
            (
                "act.player.uncrew_fortification",
                json!({"schema_version": SCHEMA_VERSION, "id": 1u64}),
            ),
            (
                "act.player.deploy_minefield_template",
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "id": "proximity_belt_dense",
                    "origin": [10i32, 5i32],
                }),
            ),
            (
                "act.player.disarm_mine",
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "mine_id": 1u64,
                    "actor_id": 7u64,
                }),
            ),
            (
                "act.player.cut_wire",
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "wire_id": 1u64,
                    "actor_id": 2u64,
                }),
            ),
            (
                "act.player.repair_fortification",
                json!({"schema_version": SCHEMA_VERSION, "id": 1u64}),
            ),
            (
                "act.player.deploy_mg_tripod",
                json!({"schema_version": SCHEMA_VERSION, "pos": [10, 5]}),
            ),
            (
                "act.player.mark_spotter_target",
                json!({
                    "schema_version": SCHEMA_VERSION,
                    "spotter_id": 3u64,
                    "target_id": 9u64,
                }),
            ),
            (
                "act.player.power_fence",
                json!({"schema_version": SCHEMA_VERSION, "fence_id": 1u64}),
            ),
            (
                "act.player.unpower_fence",
                json!({"schema_version": SCHEMA_VERSION, "fence_id": 1u64}),
            ),
        ];
        assert_eq!(
            routes.len(),
            10,
            "VAL-M9C-010 contract: exactly 10 new M9C cfctl methods must dispatch"
        );
        for (method, params) in routes {
            let req = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": method,
                "params": params,
            });
            let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
                .await
                .unwrap();
            let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
            if let Some(error) = parsed.error.as_ref() {
                assert_ne!(
                    error.code,
                    error_codes::METHOD_NOT_FOUND,
                    "{method} returned MethodNotFound; m9c-6 dispatch table out of date"
                );
            }
            assert!(
                parsed.result.is_some(),
                "{method} must dispatch to a result"
            );
        }
    }

    /// **M9C-6**: `act.player.cut_wire` rejects wire_id=0 / actor_id=0
    /// before reaching the engine handler.
    #[tokio::test]
    async fn m9c_cut_wire_rejects_zero_ids() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        for (params, expected_reason) in [
            (
                json!({"schema_version": SCHEMA_VERSION, "wire_id": 0u64, "actor_id": 1u64}),
                "wire_id_zero",
            ),
            (
                json!({"schema_version": SCHEMA_VERSION, "wire_id": 1u64, "actor_id": 0u64}),
                "actor_id_zero",
            ),
        ] {
            let req = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "act.player.cut_wire",
                "params": params,
            });
            let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
                .await
                .unwrap();
            let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
            let error = parsed.error.expect("zero id must reject");
            assert_eq!(error.code, error_codes::INVALID_PARAMS);
            assert_eq!(
                error.data.unwrap().get("reason").and_then(|v| v.as_str()),
                Some(expected_reason)
            );
        }
    }

    /// **M9C-6**: `act.player.repair_fortification` rejects id=0.
    #[tokio::test]
    async fn m9c_repair_fortification_rejects_zero_id() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.player.repair_fortification",
            "params": {"schema_version": SCHEMA_VERSION, "id": 0u64}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        let error = parsed.error.expect("zero id must reject");
        assert_eq!(error.code, error_codes::INVALID_PARAMS);
        assert_eq!(
            error.data.unwrap().get("reason").and_then(|v| v.as_str()),
            Some("fortification_id_zero")
        );
    }

    /// **M9C-6**: `act.player.unpower_fence` returns `powered=false`
    /// + `cause="breaker_toggled"` so the closure-feature worker
    /// observes the `fence_depowered.cause` value matching the
    /// schema enum.
    #[tokio::test]
    async fn m9c_unpower_fence_emits_breaker_toggled() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.player.unpower_fence",
            "params": {"schema_version": SCHEMA_VERSION, "fence_id": 1u64}
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        assert!(parsed.error.is_none(), "expected success: {:?}", parsed.error);
        let result = parsed.result.unwrap();
        assert_eq!(result.get("powered").and_then(|v| v.as_bool()), Some(false));
        assert_eq!(
            result.get("cause").and_then(|v| v.as_str()),
            Some("breaker_toggled")
        );
    }

    /// **M12C** § cfctl cinematic surface — every method routes (not
    /// `MethodNotFound`). Stub engine defaults to "no cinematic
    /// active" so the request shape is exercised end-to-end.
    #[tokio::test]
    async fn m12c_cinematic_cfctl_routes() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        for method in [
            "act.player.skip_cinematic",
            "act.player.pause_cinematic",
            "srv.dump_cinematic_state",
        ] {
            let req = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": method,
                "params": {"schema_version": SCHEMA_VERSION},
            });
            let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
                .await
                .unwrap();
            let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
            if let Some(err) = &parsed.error {
                assert_ne!(
                    err.code,
                    error_codes::METHOD_NOT_FOUND,
                    "{method} returned MethodNotFound; cfctl routes table out of date"
                );
            }
        }
        // replay_cinematic carries an `id` parameter.
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "act.player.replay_cinematic",
            "params": {"schema_version": SCHEMA_VERSION, "id": "cin_intro"},
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        if let Some(err) = &parsed.error {
            assert_ne!(
                err.code,
                error_codes::METHOD_NOT_FOUND,
                "act.player.replay_cinematic returned MethodNotFound"
            );
        }
    }

    /// **M12C** § `srv.dump_cinematic_state` returns the "no cinematic"
    /// sentinel from the stub engine — schema_version + phase ended +
    /// active false.
    #[tokio::test]
    async fn m12c_dump_cinematic_state_sentinel() {
        let engine = StubEngine;
        let hz: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
        let filter: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "srv.dump_cinematic_state",
            "params": {"schema_version": SCHEMA_VERSION},
        });
        let resp = process_request(&req.to_string(), &engine, &hz, &filter, 240)
            .await
            .unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&resp).unwrap();
        assert!(parsed.error.is_none(), "expected success: {:?}", parsed.error);
        let result = parsed.result.expect("payload");
        assert_eq!(result.get("phase").and_then(|v| v.as_str()), Some("ended"));
        assert_eq!(result.get("active").and_then(|v| v.as_bool()), Some(false));
    }
}
