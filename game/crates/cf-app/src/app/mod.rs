pub(crate) mod components;
pub(crate) mod resources;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Result;
use bevy::{
    log::LogPlugin,
    prelude::*,
    window::{PresentMode, WindowResolution},
};

use cf_capture::{write_capture_manifest_from_handle, CaptureMode, CaptureStateHandle, CaptureSystems, CfCapturePlugin};
use cf_control::engine::{M0Engine, M0EngineConfig};
use cf_render_2d::{
    asset_loader::AssetIndexPlugin, ActorSpritePlugin, CfRenderPlugin, ChunkedTerrainPlugin, ColorGradingPlugin,
    JuicePlugin, ReactorVfxPlugin,
};
use cf_ui::{AnimationPlugin, ComicOverlayPlugin, SlideshowPlugin, StatusStripPlugin};

use crate::app::resources::{
    AppRuntime, CaptureRecorderCursor, EngineHolder, LocalInputEnabled, QuicksaveLoopResource, RenderEffectsCursor,
    WorkspaceAssetRoot,
};
use crate::cli::CaptureOptions;
use crate::headless::{
    finalize_engine, start_control_server, wait_for_capture_pngs_flushed, write_control_port_file,
};
use crate::input::{
    esc_or_close_to_exit, handle_window_focus_capture, ingest_focus_input, ingest_player_input,
    ingest_quicksave_input, HoldTracker,
};
use crate::m12::{
    audio::M12aAudioPlugin,
    juice::{
        m12_apply_color_grading_to_clear_color, m12_dispatch_juice_audio_cues, m12_ingest_slideshow_skip_input,
        m12_render_screen_flash_overlay, m12_sync_scene_mood_from_mission_phase, m12_sync_settings_to_juice_state,
        m12_trigger_banner_slide_in_juice,
    },
    slideshow::{
        m12_advance_slideshow_state, m12_despawn_slideshow_audio, m12_finalize_completed_slideshow,
        m12_render_slideshow_overlay, m12_spawn_slideshow_audio, m12_start_intro_slideshow_on_shell_screen_enter,
    },
};
use crate::systems_sync::{
    check_completion, drive_engine_tick, drive_engine_tick_unpaced, hydrate_asset_index_from_ledger,
    log_tick_progress, pump_recorder_events_into_capture_keyframes, pump_recorder_events_into_render_effects,
    sync_actor_state_to_render, sync_engine_tick_to_capture_clock, sync_reactor_state_to_widgets,
    sync_terrain_state_to_render,
};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) fn run_bevy(
    config: M0EngineConfig,
    control_api: bool,
    control_port: u16,
    _uds: Option<PathBuf>,
    control_port_file: Option<PathBuf>,
    local_input_enabled: bool,
    capture_opts: CaptureOptions,
    unpaced: bool,
) -> Result<()> {
    let engine = Arc::new(M0Engine::new(config.clone()));
    engine.record_run_started();
    engine.record_setting_snapshot();

    let control_rt = if control_api {
        let rt = start_control_server(engine.clone(), control_port)?;
        write_control_port_file(control_port_file.as_deref(), rt.bound_addr)?;
        Some(rt)
    } else {
        if let Some(path) = control_port_file {
            anyhow::bail!(
                "--control-port-file={} requires --control-api so there is a bound port to report",
                path.display()
            );
        }
        None
    };

    let captures_dir = engine.run_bundle_dir().join("captures");
    let capture_config = capture_opts.build_config(captures_dir.clone(), config.tick_rate_hz);
    let capture_enabled = capture_config.enabled;
    if capture_enabled && matches!(capture_config.mode, CaptureMode::OffscreenImage) {
        tracing::warn!(
            target: "cf::capture",
            "headless-capture is scope-limited (T-CAPTURE-OFFSCREEN); falling back to baseline log-only output. \
             Use windowed-hidden mode (default) for visual proof until the offscreen RenderTarget readback ships."
        );
    }
    if capture_enabled {
        if let Err(e) = cf_capture::ensure_capture_dir(&captures_dir) {
            tracing::warn!(target: "cf::capture", "failed to create captures dir {}: {e}", captures_dir.display());
        }
    }

    let mut app = App::new();
    let title = format!("Corefall — BP2 Terrain & Replay (v{APP_VERSION})");
    let workspace_root = resolve_workspace_root();
    let plugins = DefaultPlugins
        .set(WindowPlugin {
            primary_window: Some(Window {
                title,
                resolution: WindowResolution::new(1280, 720),
                present_mode: PresentMode::AutoVsync,
                resizable: true,
                ..default()
            }),
            ..default()
        })
        .set(bevy::asset::AssetPlugin {
            file_path: workspace_root.display().to_string(),
            ..default()
        })
        .disable::<LogPlugin>();
    app.insert_resource(WorkspaceAssetRoot(workspace_root.clone()));
    use bevy::winit::{UpdateMode, WinitSettings};
    app.insert_resource(WinitSettings {
        focused_mode: UpdateMode::Continuous,
        unfocused_mode: UpdateMode::Continuous,
    });

    app.add_plugins(plugins)
        .add_plugins(CfRenderPlugin::default())
        .add_plugins(ActorSpritePlugin)
        .add_plugins(ChunkedTerrainPlugin)
        .add_plugins(ReactorVfxPlugin)
        .add_plugins(M12aAudioPlugin)
        .add_plugins(AssetIndexPlugin)
        .add_plugins(StatusStripPlugin)
        .add_plugins(cf_shell::ShellPlugin)
        .add_plugins(JuicePlugin)
        .add_plugins(ColorGradingPlugin)
        .add_plugins(AnimationPlugin)
        .add_plugins(SlideshowPlugin)
        .add_plugins(ComicOverlayPlugin);
    app.add_systems(Startup, hydrate_asset_index_from_ledger);
    app.init_resource::<HoldTracker>();
    let capture_handle = CaptureStateHandle::default();
    app.add_plugins(CfCapturePlugin {
        config: capture_config.clone(),
        state_handle: capture_handle.clone(),
    });
    app.insert_resource(CaptureRecorderCursor::default());
    app.insert_resource(RenderEffectsCursor::default());
    app.insert_resource(Time::<Fixed>::from_hz(f64::from(config.tick_rate_hz)));
    app.insert_resource(EngineHolder(engine.clone()));
    app.insert_resource(LocalInputEnabled(local_input_enabled));
    app.insert_resource(AppRuntime {
        duration_ticks: config.duration_ticks,
        last_announced_tick: 0,
        unpaced,
        unpaced_max_ticks_per_frame: 1024,
    });
    if let Some(rt) = control_rt {
        app.insert_resource(rt);
    }

    app.add_systems(FixedUpdate, drive_engine_tick);
    if unpaced {
        app.add_systems(Update, drive_engine_tick_unpaced);
    }
    app.add_systems(
        Update,
        (
            esc_or_close_to_exit,
            handle_window_focus_capture,
            check_completion,
            log_tick_progress,
            ingest_player_input,
            ingest_focus_input,
            ingest_quicksave_input,
            sync_actor_state_to_render,
            sync_terrain_state_to_render,
            sync_reactor_state_to_widgets,
            sync_engine_tick_to_capture_clock,
            pump_recorder_events_into_capture_keyframes,
            pump_recorder_events_into_render_effects,
        )
            .chain(),
    );
    app.insert_resource(QuicksaveLoopResource::default());
    app.add_systems(
        Update,
        (
            m12_sync_settings_to_juice_state,
            m12_sync_scene_mood_from_mission_phase,
            m12_ingest_slideshow_skip_input,
            m12_apply_color_grading_to_clear_color,
            m12_start_intro_slideshow_on_shell_screen_enter,
            m12_advance_slideshow_state,
            m12_render_slideshow_overlay,
            m12_spawn_slideshow_audio,
            m12_despawn_slideshow_audio,
            m12_finalize_completed_slideshow,
            m12_trigger_banner_slide_in_juice,
            m12_render_screen_flash_overlay,
            m12_dispatch_juice_audio_cues,
        ),
    );
    app.configure_sets(
        Update,
        CaptureSystems
            .after(sync_engine_tick_to_capture_clock)
            .after(pump_recorder_events_into_capture_keyframes),
    );

    app.run();

    if capture_enabled {
        wait_for_capture_pngs_flushed(&capture_handle, &captures_dir, std::time::Duration::from_secs(5));
        match write_capture_manifest_from_handle(&capture_config, &capture_handle) {
            Ok(path) => tracing::info!(
                target: "cf::capture",
                "capture manifest written to {}",
                path.display()
            ),
            Err(e) => tracing::warn!(
                target: "cf::capture",
                "failed to write capture manifest: {e}"
            ),
        }
    }

    finalize_engine(engine, config.write_run_bundle)?;
    Ok(())
}

/// **M12** § Slideshow audio playback — point AssetServer at the
/// workspace root so `AssetServer::load("game/content/audio/...")` and
/// `AssetServer::load("content/assets/placeholders/...")` both resolve
/// cleanly. Without this override, Bevy defaults to a `./assets/`
/// directory next to the binary and the audio + slide PNG handles fail
/// to load.
fn resolve_workspace_root() -> PathBuf {
    std::env::current_dir()
        .ok()
        .and_then(|p| {
            let mut cur: &Path = p.as_path();
            loop {
                let manifest = cur.join("Cargo.toml");
                if manifest.exists() {
                    let content = std::fs::read_to_string(&manifest).unwrap_or_default();
                    if content.contains("[workspace]") {
                        return Some(cur.to_path_buf());
                    }
                }
                match cur.parent() {
                    Some(parent) => cur = parent,
                    None => return None,
                }
            }
        })
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
}
