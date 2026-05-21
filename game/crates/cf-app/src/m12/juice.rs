use std::collections::HashSet;

use bevy::{input::keyboard::KeyCode, prelude::*};

use cf_render_2d::{ColorGradingState, JuiceAccessibility, JuiceKind, JuicePulse, JuiceState, SceneMood};
use cf_ui::{ComicOverlayMode, ComicOverlayState, HudState, SlideshowPhase, SlideshowState};

use crate::app::components::M12ScreenFlash;
use crate::app::resources::EngineHolder;

/// **M12**: mirror cf-control's accessibility + comic-overlay settings into
/// the M12 plugin resources every frame.
pub(crate) fn m12_sync_settings_to_juice_state(
    holder: Res<EngineHolder>,
    mut juice_acc: ResMut<JuiceAccessibility>,
    mut comic_state: ResMut<ComicOverlayState>,
) {
    let s = holder.0.current_settings();
    let next_acc = JuiceAccessibility {
        reduce_motion: s.reduced_motion,
        reduce_shake: s.reduced_shake,
        reduce_flash: s.reduced_flash,
    };
    if *juice_acc != next_acc {
        *juice_acc = next_acc;
    }
    let next_mode = match s.comic_style_overlay {
        cf_control::settings::ComicStyleOverlay::Full => ComicOverlayMode::Full,
        cf_control::settings::ComicStyleOverlay::Subtle => ComicOverlayMode::Subtle,
        cf_control::settings::ComicStyleOverlay::Off => ComicOverlayMode::Off,
    };
    if comic_state.mode != next_mode || comic_state.comic_death_recap_toggle != s.comic_death_recap {
        comic_state.mode = next_mode;
        comic_state.comic_death_recap_toggle = s.comic_death_recap;
    }
}

/// **M12**: infer the active `SceneMood` from the engine's current
/// mission-director phase + any environmental hazard signal, then
/// request a `ColorGradingState::cross_fade_to()` when it changes.
pub(crate) fn m12_sync_scene_mood_from_mission_phase(
    holder: Res<EngineHolder>,
    mut grading: ResMut<ColorGradingState>,
) {
    let state = holder.0.actor_render_snapshot();
    let mut mood = SceneMood::Daylight;
    if let Some(extraction) = state.extraction_zone.as_ref() {
        if !extraction.completed {
            mood = SceneMood::Nighttime;
        }
    }
    if state.breaches.iter().any(|b| !b.broken && b.hp < b.max_hp) {
        mood = SceneMood::Hazard;
    }
    grading.tick(1.0 / 60.0);
    if grading.current != mood && grading.transition.map(|(t, _)| t) != Some(mood) {
        grading.cross_fade_to(mood);
    }
}

/// **M12**: route Space / Esc / Enter input to `ShellApiCommand::SkipIntroSlideshow`
/// while the slideshow is playing. The current `ShellScreen` is checked so we
/// never collide with in-mission Esc-to-pause.
pub(crate) fn m12_ingest_slideshow_skip_input(
    keys: Res<ButtonInput<KeyCode>>,
    shell_state: Res<cf_shell::ShellState>,
    slideshow: Res<SlideshowState>,
    mut commands: MessageWriter<cf_shell::ShellApiCommand>,
) {
    if shell_state.current != cf_shell::ShellScreen::IntroSlideshow {
        return;
    }
    if slideshow.phase != SlideshowPhase::Playing {
        return;
    }
    let skip = keys.just_pressed(KeyCode::Space)
        || keys.just_pressed(KeyCode::Escape)
        || keys.just_pressed(KeyCode::Enter);
    if skip {
        commands.write(cf_shell::ShellApiCommand::SkipIntroSlideshow);
    }
}

/// **M12**: apply the live `ColorGrade` to Bevy's `ClearColor` so the
/// background frame reflects the per-scene tint.
pub(crate) fn m12_apply_color_grading_to_clear_color(
    grading: Res<ColorGradingState>,
    mut clear: ResMut<ClearColor>,
) {
    let g = grading.current_grade();
    let base = M12_BACKGROUND_LINEAR;
    let r = (base[0] * g.tint_rgb[0] * g.brightness).clamp(0.0, 1.0);
    let gg = (base[1] * g.tint_rgb[1] * g.brightness).clamp(0.0, 1.0);
    let b = (base[2] * g.tint_rgb[2] * g.brightness).clamp(0.0, 1.0);
    let new_color = Color::srgb(r, gg, b);
    if clear.0 != new_color {
        clear.0 = new_color;
    }
}

/// **M12**: baseline pixel-art-friendly cleared background (matches
/// `cf-render-2d::M0_CLEAR_COLOR`). The grading shader multiplies this
/// channel-wise before applying brightness.
const M12_BACKGROUND_LINEAR: [f32; 3] = [0.051, 0.071, 0.102];

/// **M12**: when a new banner appears in `HudState.banners` that wasn't
/// present in the previous frame, trigger a `BannerSlideIn` juice pulse
/// on the corresponding HUD node.
pub(crate) fn m12_trigger_banner_slide_in_juice(
    hud_state: Res<HudState>,
    mut seen: Local<HashSet<String>>,
    juice_acc: Res<JuiceAccessibility>,
    mut juice_state: ResMut<JuiceState>,
) {
    let current: HashSet<String> = hud_state.banners.iter().map(|b| b.id.clone()).collect();
    for id in &current {
        if !seen.contains(id) {
            let pulse = JuicePulse::new(JuiceKind::BannerSlideIn, *juice_acc);
            juice_state.push(format!("hud.banner.{id}"), pulse);
        }
    }
    *seen = current;
}

/// **M12** § Critical-hit punch screen flash + chromatic-aberration overlay.
pub(crate) fn m12_render_screen_flash_overlay(
    mut commands: Commands,
    juice: Res<JuiceState>,
    flash_query: Query<(Entity, &M12ScreenFlash)>,
) {
    let alpha = juice.screen_flash().clamp(0.0, 1.0);
    if alpha < 0.01 {
        for (entity, _) in flash_query.iter() {
            commands.entity(entity).despawn();
        }
        return;
    }
    let color = Color::srgba(1.0, 1.0, 1.0, alpha * 0.8);
    if let Some((entity, _)) = flash_query.iter().next() {
        commands.entity(entity).insert(BackgroundColor(color));
    } else {
        commands.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(color),
            GlobalZIndex(900),
            M12ScreenFlash,
            Name::new("cf::m12::screen_flash"),
        ));
    }
}

/// **M12** § Juice rule SFX cues — dispatch one `AudioCue::Juice` per
/// pulse fired this frame.
pub(crate) fn m12_dispatch_juice_audio_cues(juice: Res<JuiceState>, mut seen: Local<HashSet<String>>) {
    use cf_audio::{AudioCue, AudioPlugin, NullAudioPlugin};
    let plugin = NullAudioPlugin;
    let mut current: HashSet<String> = HashSet::new();
    juice.for_each_active_pulse(|node, pulse| {
        let key = format!("{}::{}", pulse.kind.as_str(), node);
        current.insert(key.clone());
        if !seen.contains(&key) {
            plugin.play(&AudioCue::Juice {
                rule: pulse.kind.as_str().to_string(),
                target_node: if node.is_empty() { None } else { Some(node.to_string()) },
                accessibility_suppressed: pulse.accessibility_suppressed,
            });
        }
    });
    *seen = current;
}
