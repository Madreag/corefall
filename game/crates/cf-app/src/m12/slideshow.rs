use std::path::{Path, PathBuf};

use bevy::prelude::*;

use cf_render_2d::asset_loader::AssetIndex;
use cf_ui::{SlideshowPhase, SlideshowSlot, SlideshowState};

use crate::app::components::{
    M12SlideshowImage, M12SlideshowMusic, M12SlideshowRoot, M12SlideshowSkipPrompt, M12SlideshowSubtitle,
    M12SlideshowVoice,
};
use crate::app::resources::WorkspaceAssetRoot;

/// **M12**: when cf-shell transitions into `ShellScreen::IntroSlideshow`,
/// seed `SlideshowState` with the 8 canonical intro slides + the
/// `music_intro_campaign` track id.
pub(crate) fn m12_start_intro_slideshow_on_shell_screen_enter(
    shell_state: Res<cf_shell::ShellState>,
    mut slideshow: ResMut<SlideshowState>,
) {
    if shell_state.current != cf_shell::ShellScreen::IntroSlideshow {
        return;
    }
    if slideshow.is_playing() {
        return;
    }
    let slot = match shell_state.intro_slideshow_slot {
        Some(cf_shell::IntroSlideshowSlot::FirstLaunch) => SlideshowSlot::IntroCampaign,
        Some(cf_shell::IntroSlideshowSlot::Replay) | None => SlideshowSlot::ReplayIntro,
    };
    slideshow.start(
        slot,
        cf_ui::slideshow::intro_slides(),
        Some("music_intro_campaign".to_string()),
        Some("voice_intro_narration_corefall_universe_arc".to_string()),
    );
    tracing::info!(
        target = "cf-app",
        slot = slot.as_str(),
        slides = slideshow.slides.len(),
        "M12 slideshow started"
    );
}

/// **M12**: advance the slideshow cursor every frame. Uses Bevy's `Time`
/// resource for the delta so the slide timeline respects pause + reduced
/// virtual speed.
pub(crate) fn m12_advance_slideshow_state(time: Res<Time>, mut slideshow: ResMut<SlideshowState>) {
    if !slideshow.is_playing() {
        return;
    }
    let dt_ms = (time.delta_secs() * 1000.0).clamp(0.0, 1000.0) as u32;
    if dt_ms == 0 {
        return;
    }
    slideshow.tick(dt_ms);
}

/// **M12**: render the slideshow as a fullscreen Bevy UI overlay.
#[allow(clippy::too_many_arguments)]
pub(crate) fn m12_render_slideshow_overlay(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    asset_index: Res<AssetIndex>,
    slideshow: Res<SlideshowState>,
    roots: Query<Entity, With<M12SlideshowRoot>>,
    mut images: Query<&mut ImageNode, With<M12SlideshowImage>>,
    mut subtitles: Query<(&mut Text, &mut TextColor), (With<M12SlideshowSubtitle>, Without<M12SlideshowSkipPrompt>)>,
    mut skip_prompts: Query<&mut Visibility, With<M12SlideshowSkipPrompt>>,
) {
    let playing = slideshow.is_playing();
    let root_exists = roots.iter().next().is_some();

    if !playing {
        if root_exists {
            for entity in roots.iter() {
                commands.entity(entity).despawn();
            }
        }
        return;
    }

    let Some(slide) = slideshow.current_slide() else {
        return;
    };

    let png_handle = asset_index
        .get(&slide.asset_id)
        .and_then(|e| e.png_path().map(|p| asset_server.load(p.to_path_buf())));

    if !root_exists {
        let root = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::End,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(24.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 1.0)),
                GlobalZIndex(1000),
                M12SlideshowRoot,
                Name::new("cf::m12::slideshow_root"),
            ))
            .id();

        let image_entity = commands
            .spawn((
                if let Some(handle) = png_handle.clone() {
                    ImageNode::new(handle)
                } else {
                    ImageNode::default()
                },
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                M12SlideshowImage,
            ))
            .id();
        commands.entity(root).add_children(&[image_entity]);

        let subtitle_alpha = slideshow.current_subtitle_alpha();
        let subtitle_entity = commands
            .spawn((
                Text::new(slide.subtitle.clone()),
                TextColor(Color::srgba(1.0, 1.0, 1.0, subtitle_alpha)),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                Node {
                    margin: UiRect {
                        bottom: Val::Px(64.0),
                        ..default()
                    },
                    ..default()
                },
                M12SlideshowSubtitle,
            ))
            .id();
        commands.entity(root).add_children(&[subtitle_entity]);

        let skip_entity = commands
            .spawn((
                Text::new("Press Space / Esc / Enter to skip"),
                TextColor(Color::srgba(0.7, 0.7, 0.7, 0.6)),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(8.0),
                    right: Val::Px(16.0),
                    ..default()
                },
                Visibility::Visible,
                M12SlideshowSkipPrompt,
            ))
            .id();
        commands.entity(root).add_children(&[skip_entity]);
        return;
    }

    if let Some(handle) = png_handle {
        for mut image in images.iter_mut() {
            if image.image != handle {
                image.image = handle.clone();
            }
        }
    }
    let subtitle_alpha = slideshow.current_subtitle_alpha();
    for (mut text, mut color) in subtitles.iter_mut() {
        if text.0 != slide.subtitle {
            text.0 = slide.subtitle.clone();
        }
        let srgba = color.0.to_srgba();
        if (srgba.alpha - subtitle_alpha).abs() > 0.01 {
            color.0 = Color::srgba(srgba.red, srgba.green, srgba.blue, subtitle_alpha);
        }
    }
    for mut vis in skip_prompts.iter_mut() {
        if *vis != Visibility::Visible {
            *vis = Visibility::Visible;
        }
    }
}

/// **M12**: when the slideshow reaches `Completed` or `Skipped`, emit a
/// `ShellApiCommand::QuitToMenu` (or similar) to transition cf-shell back
/// to the Main Menu and clear the slideshow state.
pub(crate) fn m12_finalize_completed_slideshow(
    mut slideshow: ResMut<SlideshowState>,
    shell_state: Res<cf_shell::ShellState>,
    mut commands: MessageWriter<cf_shell::ShellApiCommand>,
) {
    if shell_state.current != cf_shell::ShellScreen::IntroSlideshow {
        return;
    }
    match slideshow.phase {
        SlideshowPhase::Completed | SlideshowPhase::Skipped => {
            tracing::info!(
                target = "cf-app",
                phase = slideshow.phase.as_str(),
                "M12 slideshow finished — returning to main menu"
            );
            slideshow.reset();
            commands.write(cf_shell::ShellApiCommand::OpenMainMenu);
        }
        _ => {}
    }
}

/// **M12**: resolve a ledger output_path (absolute, on-disk) to a path
/// RELATIVE to the workspace root.
pub(crate) fn m12_asset_path_relative_to(root: &Path, abs: &Path) -> Option<PathBuf> {
    abs.strip_prefix(root).ok().map(|p| p.to_path_buf())
}

/// **M12**: spawn the music + voice-over `AudioPlayer` entities when the
/// slideshow transitions from idle → playing.
pub(crate) fn m12_spawn_slideshow_audio(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    asset_index: Res<AssetIndex>,
    asset_root: Res<WorkspaceAssetRoot>,
    slideshow: Res<SlideshowState>,
    music_query: Query<Entity, With<M12SlideshowMusic>>,
    voice_query: Query<Entity, With<M12SlideshowVoice>>,
) {
    if !slideshow.is_playing() {
        return;
    }
    if music_query.iter().next().is_none() {
        if let Some(music_id) = slideshow.music_track_id.as_deref() {
            if let Some(entry) = asset_index.get(music_id) {
                if let Some(rel) = m12_asset_path_relative_to(&asset_root.0, entry.svg_path()) {
                    let handle: Handle<bevy::audio::AudioSource> = asset_server.load(rel.clone());
                    commands.spawn((
                        bevy::audio::AudioPlayer::new(handle),
                        bevy::audio::PlaybackSettings::LOOP,
                        M12SlideshowMusic,
                        Name::new("cf::m12::slideshow_music"),
                    ));
                    tracing::info!(target = "cf-app", track = music_id, path = %rel.display(), "M12 slideshow music spawned");
                } else {
                    tracing::warn!(target = "cf-app", track = music_id, "M12 slideshow music path outside workspace root");
                }
            } else {
                tracing::warn!(target = "cf-app", track = music_id, "M12 slideshow music id missing in ledger");
            }
        }
    }
    if voice_query.iter().next().is_none() {
        if let Some(voice_id) = slideshow.voice_track_id.as_deref() {
            if let Some(entry) = asset_index.get(voice_id) {
                if let Some(rel) = m12_asset_path_relative_to(&asset_root.0, entry.svg_path()) {
                    let handle: Handle<bevy::audio::AudioSource> = asset_server.load(rel.clone());
                    commands.spawn((
                        bevy::audio::AudioPlayer::new(handle),
                        bevy::audio::PlaybackSettings::ONCE,
                        M12SlideshowVoice,
                        Name::new("cf::m12::slideshow_voice"),
                    ));
                    tracing::info!(target = "cf-app", track = voice_id, path = %rel.display(), "M12 slideshow voice spawned");
                } else {
                    tracing::warn!(target = "cf-app", track = voice_id, "M12 slideshow voice path outside workspace root");
                }
            } else {
                tracing::warn!(target = "cf-app", track = voice_id, "M12 slideshow voice id missing in ledger");
            }
        }
    }
}

/// **M12**: despawn the slideshow audio entities when the slideshow is
/// not playing. Bevy stops playback when the entity is removed.
pub(crate) fn m12_despawn_slideshow_audio(
    mut commands: Commands,
    slideshow: Res<SlideshowState>,
    music_query: Query<Entity, With<M12SlideshowMusic>>,
    voice_query: Query<Entity, With<M12SlideshowVoice>>,
) {
    if slideshow.is_playing() {
        return;
    }
    for e in music_query.iter() {
        commands.entity(e).despawn();
    }
    for e in voice_query.iter() {
        commands.entity(e).despawn();
    }
}
