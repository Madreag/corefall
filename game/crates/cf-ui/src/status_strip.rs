use bevy::prelude::*;

use crate::hud_lines::{
    banner_line, breach_line, enemy_line, mission_line, mission_timer_color, module_line, objective_line,
    rifle_status_line, silhouette_line, stance_line, tool_line,
};
use crate::hud_model::{HudBanner, HudCaption, HudSettings, HudState};
use crate::palette::{
    palette_banner_bg, palette_focus_ring, palette_focus_ring_clear, palette_strip_bg, palette_text,
};
use crate::reactor_hp_bar::ReactorHpBarState;
use crate::reactor_pressure_line::ReactorPressureLineState;
use crate::timer_warnings::TimerWarningsState;

#[derive(Component, Debug)]
pub struct StatusStripRoot;

#[derive(Component, Debug)]
pub struct StatusStripText;

#[derive(Component, Debug)]
pub struct AmmoStripText;

#[derive(Component, Debug)]
pub struct ItemStripText;

#[derive(Component, Debug)]
pub struct ReticleStripText;

#[derive(Component, Debug)]
pub struct MissionStripText;

#[derive(Component, Debug)]
pub struct ObjectiveStripText;

#[derive(Component, Debug)]
pub struct EnemyStripText;

#[derive(Component, Debug)]
pub struct BreachStripText;

#[derive(Component, Debug)]
pub struct LastEventStripText;

/// Hidden when no overlay has captured controls; shown with the capturer
/// label when `controls_capture.captured=true`.
#[derive(Component, Debug)]
pub struct CapturedStripText;

#[derive(Component, Debug)]
pub struct StanceStripText;

#[derive(Component, Debug)]
pub struct StabilityStripText;

#[derive(Component, Debug)]
pub struct SilhouetteStripText;

#[derive(Component, Debug)]
pub struct ModuleStripText;

#[derive(Component, Debug)]
pub struct ToolStripText;

#[derive(Component, Debug)]
pub struct CaptionStripText;

#[derive(Component, Debug)]
pub struct CaptionStripRoot;

#[derive(Component, Debug)]
pub struct BannerStripRoot;

#[derive(Component, Debug)]
pub struct BannerStripText;

/// Stable accessibility id for a HUD node. Drives the focus ring map +
/// `cfctl ui` lookups.
#[derive(Component, Debug, Clone)]
pub struct HudAccessibilityId(pub &'static str);

/// Marker for the banner-strip focus wrapper so the focus ring can highlight it.
#[derive(Component, Debug)]
pub struct BannerFocusWrapper;

pub struct StatusStripPlugin;

impl Plugin for StatusStripPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudState>()
            .init_resource::<HudSettings>()
            .init_resource::<ReactorHpBarState>()
            .init_resource::<ReactorPressureLineState>()
            .init_resource::<TimerWarningsState>()
            .add_systems(Startup, (spawn_status_strip, spawn_banner_strip, spawn_caption_strip))
            .add_systems(
                Update,
                (
                    apply_ui_scale_from_settings,
                    update_status_strip,
                    update_palette_for_high_contrast,
                    update_banner_strip,
                    update_caption_strip,
                    update_focus_ring,
                    update_captured_strip,
                ),
            );
    }
}

fn update_captured_strip(state: Res<HudState>, mut query: Query<&mut Text, With<CapturedStripText>>) {
    let desired = match &state.controls_captured_by {
        Some(label) if !label.is_empty() => format!("CONTROLS CAPTURED: {}", label.to_uppercase()),
        Some(_) => "CONTROLS CAPTURED".to_string(),
        None => String::new(),
    };
    for mut text in &mut query {
        if text.0 != desired {
            text.0 = desired.clone();
        }
    }
}

fn spawn_status_strip(mut commands: Commands) {
    let root_node = Node {
        position_type: PositionType::Absolute,
        top: Val::Px(12.0),
        left: Val::Px(12.0),
        max_width: Val::Percent(96.0),
        flex_direction: FlexDirection::Column,
        flex_wrap: FlexWrap::NoWrap,
        align_content: AlignContent::FlexStart,
        row_gap: Val::Px(1.0),
        column_gap: Val::Px(12.0),
        padding: UiRect::all(Val::Px(8.0)),
        ..default()
    };
    let text_font = TextFont {
        font_size: 11.0,
        ..default()
    };
    let text_color = TextColor(palette_text(false));
    let line_node = || Node {
        padding: UiRect::all(Val::Px(1.0)),
        border: UiRect::all(Val::Px(2.0)),
        flex_direction: FlexDirection::Row,
        ..default()
    };
    commands
        .spawn((
            root_node,
            BackgroundColor(palette_strip_bg(false)),
            StatusStripRoot,
            Name::new("cf::ui::status_strip"),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    line_node(),
                    BorderColor::all(palette_focus_ring_clear()),
                    HudAccessibilityId("hud.status_strip"),
                ))
                .with_children(|p| {
                    p.spawn((Text::new("STATUS: --"), text_font.clone(), text_color, StatusStripText));
                });
            parent.spawn((Text::new("ITEM: --"), text_font.clone(), text_color, ItemStripText));
            parent.spawn((Text::new("HP: --"), text_font.clone(), text_color, AmmoStripText));
            parent.spawn((Text::new("NO RIFLE"), text_font.clone(), text_color, ReticleStripText));
            parent
                .spawn((
                    line_node(),
                    BorderColor::all(palette_focus_ring_clear()),
                    HudAccessibilityId("hud.stance"),
                ))
                .with_children(|p| {
                    p.spawn((Text::new("STANCE: --"), text_font.clone(), text_color, StanceStripText));
                });
            parent
                .spawn((
                    line_node(),
                    BorderColor::all(palette_focus_ring_clear()),
                    HudAccessibilityId("hud.silhouette"),
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new("BODY: --"),
                        text_font.clone(),
                        text_color,
                        SilhouetteStripText,
                    ));
                });
            parent
                .spawn((
                    line_node(),
                    BorderColor::all(palette_focus_ring_clear()),
                    HudAccessibilityId("hud.module_strip"),
                ))
                .with_children(|p| {
                    p.spawn((Text::new("MODS: --"), text_font.clone(), text_color, ModuleStripText));
                });
            parent
                .spawn((
                    line_node(),
                    BorderColor::all(palette_focus_ring_clear()),
                    HudAccessibilityId("hud.objective"),
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new("OBJECTIVE: --"),
                        text_font.clone(),
                        text_color,
                        ObjectiveStripText,
                    ));
                });
            parent
                .spawn((
                    line_node(),
                    BorderColor::all(palette_focus_ring_clear()),
                    HudAccessibilityId("hud.mission"),
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new("MISSION: --"),
                        text_font.clone(),
                        text_color,
                        MissionStripText,
                    ));
                });
            parent
                .spawn((
                    line_node(),
                    BorderColor::all(palette_focus_ring_clear()),
                    HudAccessibilityId("hud.enemy"),
                ))
                .with_children(|p| {
                    p.spawn((Text::new("ENEMY: --"), text_font.clone(), text_color, EnemyStripText));
                });
            parent
                .spawn((
                    line_node(),
                    BorderColor::all(palette_focus_ring_clear()),
                    HudAccessibilityId("hud.breach"),
                ))
                .with_children(|p| {
                    p.spawn((Text::new("BREACH: --"), text_font.clone(), text_color, BreachStripText));
                });
            parent
                .spawn((
                    line_node(),
                    BorderColor::all(palette_focus_ring_clear()),
                    HudAccessibilityId("hud.tool"),
                ))
                .with_children(|p| {
                    p.spawn((Text::new("TOOL: --"), text_font.clone(), text_color, ToolStripText));
                });
            parent
                .spawn((
                    line_node(),
                    BorderColor::all(palette_focus_ring_clear()),
                    HudAccessibilityId("hud.last_event"),
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new("EVENT: --"),
                        text_font.clone(),
                        text_color,
                        LastEventStripText,
                    ));
                });
            parent
                .spawn((
                    line_node(),
                    BorderColor::all(palette_focus_ring_clear()),
                    HudAccessibilityId("hud.captured"),
                ))
                .with_children(|p| {
                    p.spawn((Text::new(""), text_font, text_color, CapturedStripText));
                });
        });
}

fn update_focus_ring(settings: Res<HudSettings>, mut targets: Query<(&HudAccessibilityId, &mut BorderColor)>) {
    if !settings.is_changed() {
        return;
    }
    let focused = settings.focused_node.as_deref();
    let ring_color = palette_focus_ring(settings.high_contrast);
    let clear_color = palette_focus_ring_clear();
    for (id, mut border) in targets.iter_mut() {
        let next = if focused == Some(id.0) { ring_color } else { clear_color };
        *border = BorderColor::all(next);
    }
}

fn spawn_banner_strip(mut commands: Commands) {
    let root_node = Node {
        position_type: PositionType::Absolute,
        top: Val::Px(12.0),
        left: Val::Percent(54.0),
        right: Val::Px(12.0),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(2.0),
        padding: UiRect::all(Val::Px(8.0)),
        border: UiRect::all(Val::Px(2.0)),
        ..default()
    };
    let text_font = TextFont {
        font_size: 11.0,
        ..default()
    };
    let text_color = TextColor(palette_text(false));
    commands
        .spawn((
            root_node,
            BackgroundColor(palette_banner_bg(false, "info")),
            BorderColor::all(palette_focus_ring_clear()),
            BannerStripRoot,
            HudAccessibilityId("hud.banners"),
            Name::new("cf::ui::banner_strip"),
        ))
        .with_children(|parent| {
            for _ in 0..4 {
                parent.spawn((Text::new(""), text_font.clone(), text_color, BannerStripText));
            }
        });
}

fn spawn_caption_strip(mut commands: Commands) {
    let root_node = Node {
        position_type: PositionType::Absolute,
        top: Val::Px(112.0),
        left: Val::Percent(54.0),
        right: Val::Px(12.0),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(2.0),
        padding: UiRect::all(Val::Px(8.0)),
        border: UiRect::all(Val::Px(2.0)),
        ..default()
    };
    let text_font = TextFont {
        font_size: 10.0,
        ..default()
    };
    let text_color = TextColor(palette_text(false));
    commands
        .spawn((
            root_node,
            BackgroundColor(palette_strip_bg(false)),
            BorderColor::all(palette_focus_ring_clear()),
            CaptionStripRoot,
            HudAccessibilityId("hud.captions"),
            Name::new("cf::ui::caption_strip"),
        ))
        .with_children(|parent| {
            for _ in 0..3 {
                parent.spawn((Text::new(""), text_font.clone(), text_color, CaptionStripText));
            }
        });
}

fn apply_ui_scale_from_settings(settings: Res<HudSettings>, mut ui_scale: ResMut<UiScale>) {
    if !settings.is_changed() {
        return;
    }
    let clamped = settings.ui_scale.clamp(0.5, 4.0);
    if (ui_scale.0 - clamped).abs() > f32::EPSILON {
        ui_scale.0 = clamped;
    }
}

fn update_palette_for_high_contrast(
    settings: Res<HudSettings>,
    mut strip_bg: Query<
        &mut BackgroundColor,
        (
            With<StatusStripRoot>,
            Without<BannerStripRoot>,
            Without<CaptionStripRoot>,
        ),
    >,
    mut caption_bg: Query<&mut BackgroundColor, (With<CaptionStripRoot>, Without<StatusStripRoot>)>,
    mut texts: Query<
        &mut TextColor,
        Or<(
            With<StatusStripText>,
            With<ItemStripText>,
            With<AmmoStripText>,
            With<ReticleStripText>,
            With<StanceStripText>,
            With<SilhouetteStripText>,
            With<ModuleStripText>,
            With<ObjectiveStripText>,
            With<MissionStripText>,
            With<EnemyStripText>,
            With<BreachStripText>,
            With<ToolStripText>,
            With<LastEventStripText>,
            With<CaptionStripText>,
        )>,
    >,
) {
    if !settings.is_changed() {
        return;
    }
    if let Some(mut bg) = strip_bg.iter_mut().next() {
        *bg = BackgroundColor(palette_strip_bg(settings.high_contrast));
    }
    if let Some(mut bg) = caption_bg.iter_mut().next() {
        *bg = BackgroundColor(palette_strip_bg(settings.high_contrast));
    }
    let new_color = palette_text(settings.high_contrast);
    for mut tc in texts.iter_mut() {
        *tc = TextColor(new_color);
    }
}

fn update_banner_strip(
    state: Res<HudState>,
    settings: Res<HudSettings>,
    mut root: Query<(&mut BackgroundColor, &mut Node), With<BannerStripRoot>>,
    mut texts: Query<&mut Text, With<BannerStripText>>,
) {
    let mut entries: Vec<&HudBanner> = state.banners.iter().collect();
    entries.sort_by_key(|b| match b.severity.as_str() {
        "critical" => 0,
        "warning" => 1,
        _ => 2,
    });
    let top_severity = entries.first().map(|b| b.severity.as_str()).unwrap_or("info");
    if let Some((mut bg, mut node)) = root.iter_mut().next() {
        node.display = if entries.is_empty() {
            Display::None
        } else {
            Display::Flex
        };
        *bg = BackgroundColor(palette_banner_bg(settings.high_contrast, top_severity));
    }
    let mut iter = entries.into_iter();
    for mut t in texts.iter_mut() {
        match iter.next() {
            Some(b) => **t = banner_line(b),
            None => **t = String::new(),
        }
    }
}

fn update_caption_strip(
    state: Res<HudState>,
    settings: Res<HudSettings>,
    mut texts: Query<&mut Text, With<CaptionStripText>>,
    mut root: Query<&mut Node, With<CaptionStripRoot>>,
) {
    let has_captions = settings.captions && !state.captions.is_empty();
    if let Some(mut node) = root.iter_mut().next() {
        node.display = if has_captions { Display::Flex } else { Display::None };
    }
    let visible_captions: Vec<&HudCaption> = if has_captions {
        state.captions.iter().rev().take(3).collect()
    } else {
        Vec::new()
    };
    let mut iter = visible_captions.into_iter();
    for mut t in texts.iter_mut() {
        match iter.next() {
            Some(c) => **t = format!("[{}t] {}", c.raised_at_tick, sanitize_hud_text(&c.label)),
            None => **t = String::new(),
        }
    }
}

fn sanitize_hud_text(value: &str) -> String {
    value.chars().map(|c| if c.is_ascii() { c } else { ' ' }).collect()
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn update_status_strip(
    state: Res<HudState>,
    settings: Res<HudSettings>,
    mut status_query: Query<
        &mut Text,
        (
            With<StatusStripText>,
            Without<AmmoStripText>,
            Without<ItemStripText>,
            Without<ReticleStripText>,
            Without<MissionStripText>,
            Without<ObjectiveStripText>,
            Without<EnemyStripText>,
            Without<BreachStripText>,
            Without<LastEventStripText>,
        ),
    >,
    mut item_query: Query<
        &mut Text,
        (
            With<ItemStripText>,
            Without<StatusStripText>,
            Without<AmmoStripText>,
            Without<ReticleStripText>,
            Without<MissionStripText>,
            Without<ObjectiveStripText>,
            Without<EnemyStripText>,
            Without<BreachStripText>,
            Without<LastEventStripText>,
        ),
    >,
    mut ammo_query: Query<
        &mut Text,
        (
            With<AmmoStripText>,
            Without<StatusStripText>,
            Without<ItemStripText>,
            Without<ReticleStripText>,
            Without<MissionStripText>,
            Without<ObjectiveStripText>,
            Without<EnemyStripText>,
            Without<BreachStripText>,
            Without<LastEventStripText>,
        ),
    >,
    mut reticle_query: Query<
        &mut Text,
        (
            With<ReticleStripText>,
            Without<StatusStripText>,
            Without<AmmoStripText>,
            Without<ItemStripText>,
            Without<MissionStripText>,
            Without<ObjectiveStripText>,
            Without<EnemyStripText>,
            Without<BreachStripText>,
            Without<LastEventStripText>,
        ),
    >,
    mut mission_query: Query<
        (&mut Text, &mut TextColor),
        (
            With<MissionStripText>,
            Without<StatusStripText>,
            Without<AmmoStripText>,
            Without<ItemStripText>,
            Without<ReticleStripText>,
            Without<ObjectiveStripText>,
            Without<EnemyStripText>,
            Without<BreachStripText>,
            Without<LastEventStripText>,
        ),
    >,
    mut objective_query: Query<
        &mut Text,
        (
            With<ObjectiveStripText>,
            Without<StatusStripText>,
            Without<AmmoStripText>,
            Without<ItemStripText>,
            Without<ReticleStripText>,
            Without<MissionStripText>,
            Without<EnemyStripText>,
            Without<BreachStripText>,
            Without<LastEventStripText>,
        ),
    >,
    mut enemy_query: Query<
        &mut Text,
        (
            With<EnemyStripText>,
            Without<StatusStripText>,
            Without<AmmoStripText>,
            Without<ItemStripText>,
            Without<ReticleStripText>,
            Without<MissionStripText>,
            Without<ObjectiveStripText>,
            Without<BreachStripText>,
            Without<LastEventStripText>,
        ),
    >,
    mut breach_query: Query<
        &mut Text,
        (
            With<BreachStripText>,
            Without<StatusStripText>,
            Without<AmmoStripText>,
            Without<ItemStripText>,
            Without<ReticleStripText>,
            Without<MissionStripText>,
            Without<ObjectiveStripText>,
            Without<EnemyStripText>,
            Without<LastEventStripText>,
        ),
    >,
    mut last_event_query: Query<
        &mut Text,
        (
            With<LastEventStripText>,
            Without<StatusStripText>,
            Without<AmmoStripText>,
            Without<ItemStripText>,
            Without<ReticleStripText>,
            Without<MissionStripText>,
            Without<ObjectiveStripText>,
            Without<EnemyStripText>,
            Without<BreachStripText>,
            Without<StanceStripText>,
            Without<SilhouetteStripText>,
            Without<ModuleStripText>,
            Without<ToolStripText>,
        ),
    >,
    mut stance_query: Query<
        &mut Text,
        (
            With<StanceStripText>,
            Without<StatusStripText>,
            Without<AmmoStripText>,
            Without<ItemStripText>,
            Without<ReticleStripText>,
            Without<MissionStripText>,
            Without<ObjectiveStripText>,
            Without<EnemyStripText>,
            Without<BreachStripText>,
            Without<LastEventStripText>,
            Without<SilhouetteStripText>,
            Without<ModuleStripText>,
            Without<ToolStripText>,
        ),
    >,
    mut silhouette_query: Query<
        &mut Text,
        (
            With<SilhouetteStripText>,
            Without<StatusStripText>,
            Without<AmmoStripText>,
            Without<ItemStripText>,
            Without<ReticleStripText>,
            Without<MissionStripText>,
            Without<ObjectiveStripText>,
            Without<EnemyStripText>,
            Without<BreachStripText>,
            Without<LastEventStripText>,
            Without<StanceStripText>,
            Without<ModuleStripText>,
            Without<ToolStripText>,
        ),
    >,
    mut module_query: Query<
        &mut Text,
        (
            With<ModuleStripText>,
            Without<StatusStripText>,
            Without<AmmoStripText>,
            Without<ItemStripText>,
            Without<ReticleStripText>,
            Without<MissionStripText>,
            Without<ObjectiveStripText>,
            Without<EnemyStripText>,
            Without<BreachStripText>,
            Without<LastEventStripText>,
            Without<StanceStripText>,
            Without<SilhouetteStripText>,
            Without<ToolStripText>,
        ),
    >,
    mut tool_query: Query<
        &mut Text,
        (
            With<ToolStripText>,
            Without<StatusStripText>,
            Without<AmmoStripText>,
            Without<ItemStripText>,
            Without<ReticleStripText>,
            Without<MissionStripText>,
            Without<ObjectiveStripText>,
            Without<EnemyStripText>,
            Without<BreachStripText>,
            Without<LastEventStripText>,
            Without<StanceStripText>,
            Without<SilhouetteStripText>,
            Without<ModuleStripText>,
        ),
    >,
) {
    let player = state.player.as_ref();
    if let Some(mut text) = status_query.iter_mut().next() {
        **text = format!(
            "STATUS: {}",
            player
                .map(|p| p.status.to_uppercase())
                .unwrap_or_else(|| "--".to_string())
        );
    }
    if let Some(mut text) = item_query.iter_mut().next() {
        **text = format!(
            "ITEM: slot {} / {}",
            player
                .map(|p| p.selected_slot.saturating_add(1).to_string())
                .unwrap_or_else(|| "--".to_string()),
            player
                .map(|p| p.selected_item.clone())
                .unwrap_or_else(|| "--".to_string())
        );
    }
    if let Some(mut text) = ammo_query.iter_mut().next() {
        **text = match player {
            Some(p) => format!("HP: {} / {}", p.hp as i32, p.hp_max as i32),
            None => "HP: --".to_string(),
        };
    }
    if let Some(mut text) = reticle_query.iter_mut().next() {
        **text = rifle_status_line(state.rifle.as_ref());
    }
    if let Some((mut text, mut text_color)) = mission_query.iter_mut().next() {
        **text = mission_line(state.mission.as_ref(), state.tick_rate_hz);
        *text_color = TextColor(mission_timer_color(
            state.mission.as_ref(),
            state.tick_rate_hz,
            settings.high_contrast,
        ));
    }
    if let Some(mut text) = objective_query.iter_mut().next() {
        **text = objective_line(state.mission.as_ref());
    }
    if let Some(mut text) = enemy_query.iter_mut().next() {
        **text = enemy_line(state.enemy.as_ref());
    }
    if let Some(mut text) = breach_query.iter_mut().next() {
        **text = breach_line(state.breach.as_ref());
    }
    if let Some(mut text) = last_event_query.iter_mut().next() {
        **text = format!(
            "EVENT: {}",
            state.last_event.clone().unwrap_or_else(|| "--".to_string())
        );
    }
    if let Some(mut text) = stance_query.iter_mut().next() {
        **text = stance_line(&state.stance, player);
    }
    if let Some(mut text) = silhouette_query.iter_mut().next() {
        **text = silhouette_line(&state.body_silhouette);
    }
    if let Some(mut text) = module_query.iter_mut().next() {
        **text = module_line(&state.modules);
    }
    if let Some(mut text) = tool_query.iter_mut().next() {
        **text = tool_line(state.tool_validity.as_ref());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_hud_text_replaces_missing_glyph_candidates() {
        assert_eq!(sanitize_hud_text("actor 1 → unstable"), "actor 1   unstable");
    }
}
