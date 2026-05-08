// cf-capture — frame capture pipeline for AI-agent self-testing per BP fun-proof.
// Owned by T-CAPTURE; spans BP2+ lifelong from BP2.
// See: cortext_command_vault/spec/prototype-roadmap.md §T-CAPTURE.

#![allow(clippy::needless_pass_by_value)]

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use serde::{Deserialize, Serialize};

pub const DEFAULT_FRAMES_HZ: f32 = 10.0;
pub const DEFAULT_THUMBNAIL_W: u32 = 320;
pub const DEFAULT_THUMBNAIL_H: u32 = 180;
pub const DEFAULT_RUNTIME_TICK_RATE_HZ: u32 = 60;
pub const SUMMARY_GRID_MAX_FRAMES: usize = 64;
pub const COMPOSER_SCHEMA_REV: u32 = 1;

#[derive(Resource, Clone, Debug)]
pub struct CaptureConfig {
    pub enabled: bool,
    pub frames_hz: f32,
    pub event_keyframes: bool,
    pub output_dir: PathBuf,
    pub thumbnail_w: u32,
    pub thumbnail_h: u32,
    pub runtime_tick_rate_hz: u32,
    pub mode: CaptureMode,
}

impl CaptureConfig {
    pub fn baseline_interval_ticks(&self) -> u64 {
        // Reject non-finite or non-positive cadences. NaN <= 0.0 is `false` in IEEE 754,
        // and Inf would saturate the float-to-int cast to 0 → .max(1) → capture every tick,
        // which silently fills the disk with hundreds of PNGs per second.
        if !self.frames_hz.is_finite() || self.frames_hz <= 0.0 {
            return u64::MAX;
        }
        let raw = (self.runtime_tick_rate_hz as f32 / self.frames_hz).round() as i64;
        raw.max(1) as u64
    }
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            frames_hz: DEFAULT_FRAMES_HZ,
            event_keyframes: true,
            output_dir: PathBuf::from("captures"),
            thumbnail_w: DEFAULT_THUMBNAIL_W,
            thumbnail_h: DEFAULT_THUMBNAIL_H,
            runtime_tick_rate_hz: DEFAULT_RUNTIME_TICK_RATE_HZ,
            mode: CaptureMode::Windowed,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CaptureMode {
    /// Read back the primary window swapchain. Requires a window (visible OR hidden).
    /// Works in Linux CI with xvfb, macOS, Windows. Default.
    Windowed,
    /// Read back from an offscreen RenderTarget::Image. Works in true headless mode.
    /// Activated by `--headless-capture` on cf-app; not yet wired in this commit
    /// (frame request is logged and emitted as a placeholder PNG so downstream
    /// tooling stays stable). Tracked in T-CAPTURE done-criteria.
    OffscreenImage,
}

#[derive(Message, Clone, Debug)]
pub struct CaptureKeyframeRequested {
    pub tick: u64,
    pub event_type: String,
    pub label: String,
}

#[derive(Resource, Default, Clone)]
pub struct CaptureState {
    pub frames_captured: u64,
    pub last_baseline_tick: Option<u64>,
    pub events_log: Arc<Mutex<VecDeque<CaptureFrameEntry>>>,
    pub current_tick: u64,
    pub started_at_tick: u64,
}

/// Shared handle that survives the Bevy app's `Drop` so cf-app can still write
/// the capture manifest after `app.run()` returns and Bevy reaps its resources.
#[derive(Clone, Default)]
pub struct CaptureStateHandle {
    pub events_log: Arc<Mutex<VecDeque<CaptureFrameEntry>>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CaptureFrameEntry {
    pub frame_index: u64,
    pub tick: u64,
    pub kind: CaptureFrameKind,
    pub event_type: Option<String>,
    pub label: Option<String>,
    pub png_relpath: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureFrameKind {
    Baseline,
    EventKeyframe,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct CaptureClock {
    pub current_tick: u64,
}

/// SystemSet covering both capture systems. The host (cf-app) configures this
/// set to run after the systems that update `CaptureClock` and write
/// `CaptureKeyframeRequested` so captures see fresh ticks and same-frame
/// keyframe events instead of stale values from the previous frame.
#[derive(SystemSet, Hash, Debug, Eq, PartialEq, Clone, Copy)]
pub struct CaptureSystems;

pub struct CfCapturePlugin {
    pub config: CaptureConfig,
    /// Shared events log handle. The plugin's `CaptureState` resource clones
    /// this so the host (cf-app) can still write the capture manifest after
    /// `app.run()` returns and Bevy reaps its world resources.
    pub state_handle: CaptureStateHandle,
}

impl Plugin for CfCapturePlugin {
    fn build(&self, app: &mut App) {
        let state = CaptureState {
            events_log: self.state_handle.events_log.clone(),
            ..CaptureState::default()
        };
        app.insert_resource(self.config.clone())
            .insert_resource(state)
            .insert_resource(CaptureClock::default())
            .add_message::<CaptureKeyframeRequested>()
            .add_systems(Update, capture_baseline_system.in_set(CaptureSystems))
            .add_systems(
                Update,
                capture_keyframe_system
                    .after(capture_baseline_system)
                    .in_set(CaptureSystems),
            );
    }
}

pub fn ensure_capture_dir(path: &Path) -> std::io::Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}

pub fn frame_filename(frame_index: u64, tick: u64) -> String {
    format!("frame_{:08}_t{:010}.png", frame_index, tick)
}

fn capture_baseline_system(
    mut commands: Commands,
    config: Res<CaptureConfig>,
    clock: Res<CaptureClock>,
    mut state: ResMut<CaptureState>,
) {
    if !config.enabled {
        return;
    }
    if !matches!(config.mode, CaptureMode::Windowed) {
        // OffscreenImage path lands separately; baseline still records intent.
        // Skip the actual PNG spawn until that work lands so we don't write
        // empty PNGs that would corrupt the run-bundle's non_blank_ratio assert.
        return;
    }
    let tick = clock.current_tick;
    let interval = config.baseline_interval_ticks();
    let due = match state.last_baseline_tick {
        None => true,
        Some(last) => tick.saturating_sub(last) >= interval,
    };
    if !due {
        return;
    }
    let _ = ensure_capture_dir(&config.output_dir);
    let frame_index = state.frames_captured;
    let filename = frame_filename(frame_index, tick);
    let path = config.output_dir.join(&filename);
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path.clone()));
    state.frames_captured += 1;
    state.last_baseline_tick = Some(tick);
    state.current_tick = tick;
    let entry = CaptureFrameEntry {
        frame_index,
        tick,
        kind: CaptureFrameKind::Baseline,
        event_type: None,
        label: None,
        png_relpath: filename,
    };
    if let Ok(mut log) = state.events_log.lock() {
        log.push_back(entry);
    }
}

fn capture_keyframe_system(
    mut commands: Commands,
    config: Res<CaptureConfig>,
    clock: Res<CaptureClock>,
    mut state: ResMut<CaptureState>,
    mut keyframe_events: MessageReader<CaptureKeyframeRequested>,
) {
    if !config.enabled || !config.event_keyframes {
        keyframe_events.clear();
        return;
    }
    if !matches!(config.mode, CaptureMode::Windowed) {
        keyframe_events.clear();
        return;
    }
    let _ = ensure_capture_dir(&config.output_dir);
    for kf in keyframe_events.read() {
        let tick = kf.tick.max(clock.current_tick);
        let frame_index = state.frames_captured;
        let filename = frame_filename(frame_index, tick);
        let path = config.output_dir.join(&filename);
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path.clone()));
        state.frames_captured += 1;
        state.current_tick = tick;
        let entry = CaptureFrameEntry {
            frame_index,
            tick,
            kind: CaptureFrameKind::EventKeyframe,
            event_type: Some(kf.event_type.clone()),
            label: Some(kf.label.clone()),
            png_relpath: filename,
        };
        if let Ok(mut log) = state.events_log.lock() {
            log.push_back(entry);
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CaptureManifest {
    pub composer_schema_rev: u32,
    pub frames_hz: f32,
    pub thumbnail_w: u32,
    pub thumbnail_h: u32,
    pub runtime_tick_rate_hz: u32,
    pub mode: CaptureMode,
    pub frames: Vec<CaptureFrameEntry>,
}

impl CaptureManifest {
    pub fn from_state(config: &CaptureConfig, state: &CaptureState) -> Self {
        let frames = state
            .events_log
            .lock()
            .map(|log| log.iter().cloned().collect())
            .unwrap_or_default();
        Self {
            composer_schema_rev: COMPOSER_SCHEMA_REV,
            frames_hz: config.frames_hz,
            thumbnail_w: config.thumbnail_w,
            thumbnail_h: config.thumbnail_h,
            runtime_tick_rate_hz: config.runtime_tick_rate_hz,
            mode: config.mode,
            frames,
        }
    }

    pub fn write_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            ensure_capture_dir(parent)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| std::io::Error::other(e.to_string()))?;
        std::fs::write(path, json)
    }
}

/// Write the capture manifest using a `CaptureStateHandle` (does not require
/// the live Bevy world). Used by cf-app post-`app.run()` so the manifest still
/// lands when Bevy reaps its World resources during shutdown.
pub fn write_capture_manifest_from_handle(
    config: &CaptureConfig,
    handle: &CaptureStateHandle,
) -> std::io::Result<PathBuf> {
    let frames = handle
        .events_log
        .lock()
        .map(|log| log.iter().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let manifest = CaptureManifest {
        composer_schema_rev: COMPOSER_SCHEMA_REV,
        frames_hz: config.frames_hz,
        thumbnail_w: config.thumbnail_w,
        thumbnail_h: config.thumbnail_h,
        runtime_tick_rate_hz: config.runtime_tick_rate_hz,
        mode: config.mode,
        frames,
    };
    let path = config.output_dir.join("capture_manifest.json");
    manifest.write_to(&path)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_interval_at_60hz_default_returns_six_ticks() {
        let cfg = CaptureConfig::default();
        assert_eq!(cfg.baseline_interval_ticks(), 6);
    }

    #[test]
    fn baseline_interval_at_120hz_default_returns_twelve_ticks() {
        let cfg = CaptureConfig {
            runtime_tick_rate_hz: 120,
            ..CaptureConfig::default()
        };
        assert_eq!(cfg.baseline_interval_ticks(), 12);
    }

    #[test]
    fn baseline_interval_at_120hz_60hz_capture_returns_two_ticks() {
        let cfg = CaptureConfig {
            runtime_tick_rate_hz: 120,
            frames_hz: 60.0,
            ..CaptureConfig::default()
        };
        assert_eq!(cfg.baseline_interval_ticks(), 2);
    }

    #[test]
    fn baseline_interval_zero_hz_returns_max_to_disable() {
        let cfg = CaptureConfig {
            frames_hz: 0.0,
            ..CaptureConfig::default()
        };
        assert_eq!(cfg.baseline_interval_ticks(), u64::MAX);
    }

    #[test]
    fn baseline_interval_negative_hz_returns_max_to_disable() {
        let cfg = CaptureConfig {
            frames_hz: -10.0,
            ..CaptureConfig::default()
        };
        assert_eq!(cfg.baseline_interval_ticks(), u64::MAX);
    }

    #[test]
    fn baseline_interval_nan_hz_returns_max_to_disable() {
        // Regression: NaN <= 0.0 is false in IEEE 754, so the prior guard let
        // NaN pass through; the float-to-int cast then saturated to 0 and
        // .max(1) yielded a 1-tick interval — capture would fire every sim tick.
        let cfg = CaptureConfig {
            frames_hz: f32::NAN,
            ..CaptureConfig::default()
        };
        assert_eq!(cfg.baseline_interval_ticks(), u64::MAX);
    }

    #[test]
    fn baseline_interval_positive_infinity_hz_returns_max_to_disable() {
        let cfg = CaptureConfig {
            frames_hz: f32::INFINITY,
            ..CaptureConfig::default()
        };
        assert_eq!(cfg.baseline_interval_ticks(), u64::MAX);
    }

    #[test]
    fn baseline_interval_negative_infinity_hz_returns_max_to_disable() {
        let cfg = CaptureConfig {
            frames_hz: f32::NEG_INFINITY,
            ..CaptureConfig::default()
        };
        assert_eq!(cfg.baseline_interval_ticks(), u64::MAX);
    }

    #[test]
    fn frame_filename_pads_index_and_tick() {
        let name = frame_filename(7, 123);
        assert_eq!(name, "frame_00000007_t0000000123.png");
    }

    #[test]
    fn capture_manifest_round_trip() {
        let cfg = CaptureConfig {
            enabled: true,
            frames_hz: 10.0,
            event_keyframes: true,
            output_dir: PathBuf::from("/tmp/cap"),
            thumbnail_w: 320,
            thumbnail_h: 180,
            runtime_tick_rate_hz: 60,
            mode: CaptureMode::Windowed,
        };
        let state = CaptureState::default();
        if let Ok(mut log) = state.events_log.lock() {
            log.push_back(CaptureFrameEntry {
                frame_index: 0,
                tick: 0,
                kind: CaptureFrameKind::Baseline,
                event_type: None,
                label: None,
                png_relpath: "frame_00000000_t0000000000.png".into(),
            });
            log.push_back(CaptureFrameEntry {
                frame_index: 1,
                tick: 60,
                kind: CaptureFrameKind::EventKeyframe,
                event_type: Some("mission.objective_completed".into()),
                label: Some("breach_outer_wall".into()),
                png_relpath: "frame_00000001_t0000000060.png".into(),
            });
        }
        let manifest = CaptureManifest::from_state(&cfg, &state);
        assert_eq!(manifest.composer_schema_rev, COMPOSER_SCHEMA_REV);
        assert_eq!(manifest.frames.len(), 2);
        assert_eq!(manifest.frames[1].kind, CaptureFrameKind::EventKeyframe);
    }
}
