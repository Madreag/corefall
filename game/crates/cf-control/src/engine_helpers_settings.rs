//! Free helper fns + types extracted from engine.rs.
//!
//! Extracted from engine.rs.

#![allow(unused_imports, dead_code, clippy::too_many_arguments)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use chrono::{DateTime, Utc};
use serde_json::{json, Value};

use cf_actor::sim::{step as actor_step, ActorSimState, ActorTickOutcome, StepDeps, StepReport};
use cf_actor::{ActorId, ActorWorld, ControlIntent, IntentSource, ItemSlot, Vec2};
use cf_replay::{
    diagnostics, ArtifactItem, BuildInfo, BundleInputs, CapabilitiesBlock, CaptureConfig,
    ChecksumConfig, PerfSample, Recorder, RunManifest, SceneInfo, SettingsBlock, TestRecord,
    CONTROL_SCHEMA_VERSION, EVENT_ENVELOPE_VERSION, MANIFEST_SCHEMA_VERSION, SCENARIO_SCHEMA_VERSION,
};
use cf_sim_core::{
    checksum::{sim_state_v1, CHECKSUM_ALGORITHM, CHECKSUM_SCOPE},
    ids::{iso_hyphen_safe, make_run_id},
    Rng, SimClock, SimConfig, Tick, WallClock,
};

use crate::engine::*;
use crate::scenario::Scenario;
use crate::server::{async_trait, CommandResult, ControlCommand, EngineHandle, SettingsPatch};
use crate::state::{ActorView, ObserveFrame, ObserveSettings, RunStatus};
use crate::{Settings, SCHEMA_VERSION};

/// effective sim-speed percentage (0..=100) the per-tick scheduler honors
/// for the current tick. Composes:
///
/// - `settings.game_speed_assist.speed_pct()` (Off=100 / Slowdown75=75 /
///   Slowdown25=25 / FullPause=0) — single-player only; multiplayer
///   sessions force this leg to 100 per spec.
/// - `pie_menu.slowdown_factor_pct` (100 when closed, 20 in single-player
///   open, 100 in multiplayer open).
///
/// Most-restrictive wins (per the fix description: "the pie menu can stack
/// with game_speed_assist; whichever is more restrictive wins"). Pure
/// function so determinism is easy to verify in unit tests.
pub(crate) fn effective_sim_speed_pct(
    settings: &Settings,
    pie_menu: &cf_squad_ui::PieMenuState,
    multiplayer_session: bool,
) -> u8 {
    let assist_pct = if multiplayer_session {
        100
    } else {
        settings.game_speed_assist.speed_pct()
    };
    assist_pct.min(pie_menu.slowdown_factor_pct)
}

pub(crate) fn apply_settings_patch(settings: &mut Settings, patch: &SettingsPatch) -> Vec<String> {
    let mut changed = Vec::new();
    if let Some(v) = patch.ui_scale {
        let clamped = v.clamp(crate::settings::UI_SCALE_MIN, crate::settings::UI_SCALE_MAX);
        if (settings.ui_scale - clamped).abs() > f32::EPSILON {
            settings.ui_scale = clamped;
            changed.push("ui_scale".to_string());
        }
    }
    if let Some(v) = patch.high_contrast {
        if settings.high_contrast != v {
            settings.high_contrast = v;
            changed.push("high_contrast".to_string());
        }
    }
    if let Some(v) = patch.captions {
        if settings.captions != v {
            settings.captions = v;
            changed.push("captions".to_string());
        }
    }
    if let Some(v) = patch.reduced_motion {
        if settings.reduced_motion != v {
            settings.reduced_motion = v;
            changed.push("reduced_motion".to_string());
        }
    }
    if let Some(v) = patch.reduced_shake {
        if settings.reduced_shake != v {
            settings.reduced_shake = v;
            changed.push("reduced_shake".to_string());
        }
    }
    if let Some(v) = patch.reduced_flash {
        if settings.reduced_flash != v {
            settings.reduced_flash = v;
            changed.push("reduced_flash".to_string());
        }
    }
    if let Some(v) = patch.hold_to_confirm {
        if settings.hold_to_confirm != v {
            settings.hold_to_confirm = v;
            changed.push("hold_to_confirm".to_string());
        }
    }
    if let Some(v) = patch.hold_threshold_ms {
        let clamped = v.clamp(50, 2000);
        if settings.hold_threshold_ms != clamped {
            settings.hold_threshold_ms = clamped;
            changed.push("hold_threshold_ms".to_string());
        }
    }
    if let Some(v) = patch.key_remap_enabled {
        if settings.key_remap_enabled != v {
            settings.key_remap_enabled = v;
            changed.push("key_remap_enabled".to_string());
        }
    }
    if let Some(ref new_bindings) = patch.key_bindings {
        if &settings.key_bindings != new_bindings {
            settings.key_bindings = new_bindings.clone();
            changed.push("key_bindings".to_string());
        }
    }
    if let Some(v) = patch.reduce_camera_shake_pct {
        let clamped = v.clamp(0.0, 1.0);
        if (settings.reduce_camera_shake_pct - clamped).abs() > f32::EPSILON {
            settings.reduce_camera_shake_pct = clamped;
            changed.push("reduce_camera_shake_pct".to_string());
        }
    }
    if let Some(v) = patch.tick_rate_hz {
        // M1: the engine's tick_rate_hz is fixed at construction (so deterministic
        // checksums are per-rate). The setting mirrors that value so cfctl
        // observe.settings round-trips it; we accept the patch but the engine
        // does NOT live-retick to the new rate. A future M5+ command will swap
        // tick rate via scenario reload.
        let v = v.max(1);
        if settings.tick_rate_hz != v {
            settings.tick_rate_hz = v;
            changed.push("tick_rate_hz".to_string());
        }
    }
    // M1 Gap F1-F2: feel cvars (already validated by SettingsPatch::validation_error).
    if let Some(v) = patch.accel {
        if (settings.accel - v).abs() > f32::EPSILON {
            settings.accel = v;
            changed.push("accel".to_string());
        }
    }
    if let Some(v) = patch.friction {
        if (settings.friction - v).abs() > f32::EPSILON {
            settings.friction = v;
            changed.push("friction".to_string());
        }
    }
    if let Some(v) = patch.gravity {
        if (settings.gravity - v).abs() > f32::EPSILON {
            settings.gravity = v;
            changed.push("gravity".to_string());
        }
    }
    if let Some(v) = patch.jump_force {
        if (settings.jump_force - v).abs() > f32::EPSILON {
            settings.jump_force = v;
            changed.push("jump_force".to_string());
        }
    }
    if let Some(v) = patch.recoil_decay_per_tick {
        if (settings.recoil_decay_per_tick - v).abs() > f32::EPSILON {
            settings.recoil_decay_per_tick = v;
            changed.push("recoil_decay_per_tick".to_string());
        }
    }
    if let Some(v) = patch.sharp_aim_build_ticks {
        if settings.sharp_aim_build_ticks != v {
            settings.sharp_aim_build_ticks = v;
            changed.push("sharp_aim_build_ticks".to_string());
        }
    }
    if let Some(v) = patch.walk_threshold {
        if (settings.walk_threshold - v).abs() > f32::EPSILON {
            settings.walk_threshold = v;
            changed.push("walk_threshold".to_string());
        }
    }
    if let Some(ref id) = patch.ai_difficulty {
        if settings.ai_difficulty != *id {
            settings.ai_difficulty = id.clone();
            changed.push("ai_difficulty".to_string());
        }
    }
    if let Some(v) = patch.ai_debug {
        if settings.ai_debug != v {
            settings.ai_debug = v;
            changed.push("ai_debug".to_string());
        }
    }
    // === M8 accessibility / camera / debug / locale extensions ===
    if let Some(ref s) = patch.game_speed_assist {
        if let Some(parsed) = crate::settings::GameSpeedAssist::from_str(s) {
            if settings.game_speed_assist != parsed {
                settings.game_speed_assist = parsed;
                changed.push("game_speed_assist".to_string());
            }
        }
    }
    if let Some(ref s) = patch.color_cue_mode {
        if let Some(parsed) = crate::settings::ColorCueMode::from_str(s) {
            if settings.color_cue_mode != parsed {
                settings.color_cue_mode = parsed;
                changed.push("color_cue_mode".to_string());
            }
        }
    }
    if let Some(ref s) = patch.aim_assist {
        if let Some(parsed) = crate::settings::AimAssist::from_str(s) {
            if settings.aim_assist != parsed {
                settings.aim_assist = parsed;
                changed.push("aim_assist".to_string());
            }
        }
    }
    if let Some(v) = patch.damage_numbers {
        if settings.damage_numbers != v {
            settings.damage_numbers = v;
            changed.push("damage_numbers".to_string());
        }
    }
    if let Some(v) = patch.killcam_enabled {
        if settings.killcam_enabled != v {
            settings.killcam_enabled = v;
            changed.push("killcam_enabled".to_string());
        }
    }
    if let Some(v) = patch.hit_stop_enabled {
        if settings.hit_stop_enabled != v {
            settings.hit_stop_enabled = v;
            changed.push("hit_stop_enabled".to_string());
        }
    }
    if let Some(v) = patch.cinematic_kills {
        if settings.cinematic_kills != v {
            settings.cinematic_kills = v;
            changed.push("cinematic_kills".to_string());
        }
    }
    if let Some(v) = patch.mini_map_enabled {
        if settings.mini_map_enabled != v {
            settings.mini_map_enabled = v;
            changed.push("mini_map_enabled".to_string());
        }
    }
    if let Some(v) = patch.compass_enabled {
        if settings.compass_enabled != v {
            settings.compass_enabled = v;
            changed.push("compass_enabled".to_string());
        }
    }
    if let Some(v) = patch.damage_direction_enabled {
        if settings.damage_direction_enabled != v {
            settings.damage_direction_enabled = v;
            changed.push("damage_direction_enabled".to_string());
        }
    }
    if let Some(v) = patch.mini_map_zoom {
        let clamped = v.clamp(0.25, 4.0);
        if (settings.mini_map_zoom - clamped).abs() > f32::EPSILON {
            settings.mini_map_zoom = clamped;
            changed.push("mini_map_zoom".to_string());
        }
    }
    if let Some(v) = patch.scope_zoom_fov {
        let clamped = v.clamp(5.0, 90.0);
        if (settings.scope_zoom_fov - clamped).abs() > f32::EPSILON {
            settings.scope_zoom_fov = clamped;
            changed.push("scope_zoom_fov".to_string());
        }
    }
    if let Some(v) = patch.text_scale {
        let clamped = v.clamp(crate::settings::UI_SCALE_MIN, crate::settings::UI_SCALE_MAX);
        if (settings.text_scale - clamped).abs() > f32::EPSILON {
            settings.text_scale = clamped;
            changed.push("text_scale".to_string());
        }
    }
    if let Some(ref s) = patch.ui_density {
        if let Some(parsed) = crate::settings::UiDensity::from_str(s) {
            if settings.ui_density != parsed {
                settings.ui_density = parsed;
                changed.push("ui_density".to_string());
            }
        }
    }
    if let Some(ref s) = patch.language {
        if !s.is_empty() && &settings.language != s {
            settings.language = s.clone();
            changed.push("language".to_string());
        }
    }
    if let Some(v) = patch.speedrun_mode {
        if settings.speedrun_mode != v {
            settings.speedrun_mode = v;
            changed.push("speedrun_mode".to_string());
        }
    }
    if let Some(v) = patch.permadeath {
        if settings.permadeath != v {
            settings.permadeath = v;
            changed.push("permadeath".to_string());
        }
    }
    if let Some(v) = patch.no_respawn {
        if settings.no_respawn != v {
            settings.no_respawn = v;
            changed.push("no_respawn".to_string());
        }
    }
    if let Some(v) = patch.fog_of_war_on {
        if settings.fog_of_war_on != v {
            settings.fog_of_war_on = v;
            changed.push("fog_of_war_on".to_string());
        }
    }
    if let Some(v) = patch.limited_ammo {
        if settings.limited_ammo != v {
            settings.limited_ammo = v;
            changed.push("limited_ammo".to_string());
        }
    }
    if let Some(v) = patch.time_limit {
        if settings.time_limit != v {
            settings.time_limit = v;
            changed.push("time_limit".to_string());
        }
    }
    if let Some(v) = patch.no_minimap {
        if settings.no_minimap != v {
            settings.no_minimap = v;
            changed.push("no_minimap".to_string());
        }
    }
    if let Some(v) = patch.hardcore_mode {
        if settings.hardcore_mode != v {
            settings.hardcore_mode = v;
            changed.push("hardcore_mode".to_string());
        }
    }
    if let Some(v) = patch.friendly_fire_on {
        if settings.friendly_fire_on != v {
            settings.friendly_fire_on = v;
            changed.push("friendly_fire_on".to_string());
        }
    }
    if let Some(v) = patch.debug_enabled {
        if settings.debug_enabled != v {
            settings.debug_enabled = v;
            changed.push("debug_enabled".to_string());
        }
    }
    // === M11 ACC-A floor (DR-003 + DR-012 closure) ===
    if let Some(ref s) = patch.contrast_mode {
        if let Some(parsed) = crate::settings::ContrastMode::from_str(s) {
            if settings.contrast_mode != parsed {
                settings.contrast_mode = parsed;
                // Mirror to the legacy bool so M8 surfaces keep working.
                let want_high_contrast = !matches!(parsed, crate::settings::ContrastMode::Standard);
                if settings.high_contrast != want_high_contrast {
                    settings.high_contrast = want_high_contrast;
                    changed.push("high_contrast".to_string());
                }
                changed.push("contrast_mode".to_string());
            }
        }
    }
    if let Some(ref s) = patch.caption_mode {
        if let Some(parsed) = crate::settings::CaptionMode::from_str(s) {
            if settings.caption_mode != parsed {
                settings.caption_mode = parsed;
                let want_captions = !matches!(parsed, crate::settings::CaptionMode::Off);
                if settings.captions != want_captions {
                    settings.captions = want_captions;
                    changed.push("captions".to_string());
                }
                changed.push("caption_mode".to_string());
            }
        }
    }
    if let Some(v) = patch.caption_background_opacity {
        let clamped = v.clamp(0.0, 1.0);
        if (settings.caption_background_opacity - clamped).abs() > f32::EPSILON {
            settings.caption_background_opacity = clamped;
            changed.push("caption_background_opacity".to_string());
        }
    }
    if let Some(ref cats) = patch.caption_categories {
        if &settings.caption_categories != cats {
            settings.caption_categories = cats.clone();
            changed.push("caption_categories".to_string());
        }
    }
    if let Some(ref s) = patch.input_profile {
        if let Some(parsed) = crate::settings::InputProfile::from_str(s) {
            if settings.input_profile != parsed {
                settings.input_profile = parsed;
                changed.push("input_profile".to_string());
            }
        }
    }
    if let Some(ref groups) = patch.remap_groups {
        if &settings.remap_groups != groups {
            settings.remap_groups = groups.clone();
            changed.push("remap_groups".to_string());
        }
    }
    if let Some(ref s) = patch.hold_behavior {
        if let Some(parsed) = crate::settings::HoldBehavior::from_str(s) {
            if settings.hold_behavior != parsed {
                settings.hold_behavior = parsed;
                changed.push("hold_behavior".to_string());
            }
        }
    }
    if let Some(v) = patch.screen_shake_scale {
        let clamped = v.clamp(0.0, 1.0);
        if (settings.screen_shake_scale - clamped).abs() > f32::EPSILON {
            settings.screen_shake_scale = clamped;
            // Mirror to the inverse-sense legacy field so cf-app's existing
            // camera-shake path keeps working: legacy = 1.0 - scale.
            let legacy = 1.0 - clamped;
            settings.reduce_camera_shake_pct = legacy;
            changed.push("screen_shake_scale".to_string());
        }
    }
    if let Some(ref s) = patch.camera_motion {
        if let Some(parsed) = crate::settings::CameraMotion::from_str(s) {
            if settings.camera_motion != parsed {
                settings.camera_motion = parsed;
                changed.push("camera_motion".to_string());
            }
        }
    }
    if let Some(ref s) = patch.objective_help {
        if let Some(parsed) = crate::settings::ObjectiveHelp::from_str(s) {
            if settings.objective_help != parsed {
                settings.objective_help = parsed;
                changed.push("objective_help".to_string());
            }
        }
    }
    if let Some(ref s) = patch.debug_explainer_level {
        if let Some(parsed) = crate::settings::DebugExplainerLevel::from_str(s) {
            if settings.debug_explainer_level != parsed {
                settings.debug_explainer_level = parsed;
                changed.push("debug_explainer_level".to_string());
            }
        }
    }
    // === M12 cinematic story beats ===
    if let Some(ref s) = patch.comic_style_overlay {
        if let Some(parsed) = crate::settings::ComicStyleOverlay::from_str(s) {
            if settings.comic_style_overlay != parsed {
                settings.comic_style_overlay = parsed;
                changed.push("comic_style_overlay".to_string());
            }
        }
    }
    if let Some(v) = patch.comic_death_recap {
        if settings.comic_death_recap != v {
            settings.comic_death_recap = v;
            changed.push("comic_death_recap".to_string());
        }
    }
    changed
}

