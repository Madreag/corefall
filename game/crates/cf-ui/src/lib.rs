//! M1 status strip: the minimum HUD per spec/native-implementation-backlog M1-004.
//!
//! Renders four short text rows pinned to the top-left corner:
//! - Status (`stable / unstable / downed / dead`).
//! - Selected slot + item label.
//! - HP (`X / 100`).
//! - Reticle / fire state (`READY`, `RELOADING NN%`, `EMPTY`, `COOLDOWN Nt`, or `NO RIFLE`).
//!
//! Comic-noir typography, mission cards, and accessibility floor land at M4. M1 only
//! needs a readable status surface so manual playtests can confirm fire/reload/status
//! behaviour without staring at the recorder.

#![deny(unsafe_code)]
#![allow(
    clippy::module_name_repetitions,
    clippy::type_complexity,
    clippy::needless_pass_by_value
)]

use bevy::prelude::*;

use cf_actor::ActorObservation;

/// Latest HUD model derived from the engine. The cf-app bridge writes this each
/// frame from the same `M0Engine` snapshot it feeds to `cf-render-2d::ActorRenderState`.
#[derive(Resource, Debug, Clone, Default)]
pub struct HudState {
    /// Player actor (if any). Owns position / aim / status / hp / inventory selection.
    pub player: Option<ActorObservation>,
    /// Rifle metadata for the player's selected rifle (if any).
    pub rifle: Option<HudRifle>,
    /// Tick the snapshot was taken at (for HUD debug).
    pub tick: u64,
    /// Tick rate in Hz; used to compute reload progress percentage.
    pub tick_rate_hz: u32,
}

/// Rifle ammo / cooldown / reload bundle for the HUD. Mirrors the rifle fields on
/// `cf-control::state::ActorView` but lives here so cf-ui doesn't depend on cf-control.
#[derive(Debug, Clone, Default)]
pub struct HudRifle {
    pub ammo: u32,
    pub capacity: u32,
    pub fire_cooldown_ticks: u32,
    pub reload_remaining_ticks: u32,
    pub reload_total_ticks: u32,
}

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

pub struct StatusStripPlugin;

impl Plugin for StatusStripPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudState>()
            .add_systems(Startup, spawn_status_strip)
            .add_systems(Update, update_status_strip);
    }
}

fn spawn_status_strip(mut commands: Commands) {
    let style = Style {
        position_type: PositionType::Absolute,
        top: Val::Px(12.0),
        left: Val::Px(12.0),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(2.0),
        padding: UiRect::all(Val::Px(8.0)),
        ..default()
    };
    let text_style = TextStyle {
        font_size: 18.0,
        color: Color::srgb(0.96, 0.96, 0.92),
        ..default()
    };
    commands
        .spawn((
            NodeBundle {
                style,
                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
                ..default()
            },
            StatusStripRoot,
            Name::new("cf::ui::status_strip"),
        ))
        .with_children(|parent| {
            parent.spawn((
                TextBundle::from_section("STATUS: --", text_style.clone()),
                StatusStripText,
            ));
            parent.spawn((TextBundle::from_section("ITEM: --", text_style.clone()), ItemStripText));
            parent.spawn((TextBundle::from_section("HP: --", text_style.clone()), AmmoStripText));
            parent.spawn((TextBundle::from_section("NO RIFLE", text_style), ReticleStripText));
        });
}

#[allow(clippy::type_complexity)]
fn update_status_strip(
    state: Res<HudState>,
    mut status_query: Query<
        &mut Text,
        (
            With<StatusStripText>,
            Without<AmmoStripText>,
            Without<ItemStripText>,
            Without<ReticleStripText>,
        ),
    >,
    mut item_query: Query<
        &mut Text,
        (
            With<ItemStripText>,
            Without<StatusStripText>,
            Without<AmmoStripText>,
            Without<ReticleStripText>,
        ),
    >,
    mut ammo_query: Query<
        &mut Text,
        (
            With<AmmoStripText>,
            Without<StatusStripText>,
            Without<ItemStripText>,
            Without<ReticleStripText>,
        ),
    >,
    mut reticle_query: Query<
        &mut Text,
        (
            With<ReticleStripText>,
            Without<StatusStripText>,
            Without<AmmoStripText>,
            Without<ItemStripText>,
        ),
    >,
) {
    let player = state.player.as_ref();
    if let Some(mut text) = status_query.iter_mut().next() {
        text.sections[0].value = format!(
            "STATUS: {}",
            player
                .map(|p| p.status.to_uppercase())
                .unwrap_or_else(|| "--".to_string())
        );
    }
    if let Some(mut text) = item_query.iter_mut().next() {
        text.sections[0].value = format!(
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
        text.sections[0].value = match player {
            Some(p) => format!("HP: {} / {}", p.hp as i32, p.hp_max as i32),
            None => "HP: --".to_string(),
        };
    }
    if let Some(mut text) = reticle_query.iter_mut().next() {
        text.sections[0].value = rifle_status_line(state.rifle.as_ref());
    }
}

/// Build the rifle status line shown in the HUD strip.
///
/// Format: `READY 30/30`, `RELOADING NN% (X/Y)`, `EMPTY (X/Y)`, `COOLDOWN Nt (X/Y)`, or
/// `NO RIFLE` when no rifle is selected.
pub fn rifle_status_line(rifle: Option<&HudRifle>) -> String {
    let Some(rifle) = rifle else {
        return "NO RIFLE".to_string();
    };
    if rifle.reload_remaining_ticks > 0 {
        let total = rifle.reload_total_ticks.max(1) as f32;
        let progress = (1.0 - (rifle.reload_remaining_ticks as f32 / total)) * 100.0;
        return format!("RELOADING {progress:>3.0}% ({}/{})", rifle.ammo, rifle.capacity);
    }
    if rifle.capacity > 0 && rifle.ammo == 0 {
        return format!("EMPTY ({}/{})", rifle.ammo, rifle.capacity);
    }
    if rifle.fire_cooldown_ticks > 0 {
        return format!(
            "COOLDOWN {}t ({}/{})",
            rifle.fire_cooldown_ticks, rifle.ammo, rifle.capacity
        );
    }
    format!("READY {}/{}", rifle.ammo, rifle.capacity)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rifle(ammo: u32, capacity: u32, cooldown: u32, remaining: u32, total: u32) -> HudRifle {
        HudRifle {
            ammo,
            capacity,
            fire_cooldown_ticks: cooldown,
            reload_remaining_ticks: remaining,
            reload_total_ticks: total,
        }
    }

    #[test]
    fn rifle_status_line_formats_ready() {
        let s = rifle_status_line(Some(&rifle(30, 30, 0, 0, 90)));
        assert_eq!(s, "READY 30/30");
    }

    #[test]
    fn rifle_status_line_formats_reload() {
        let s = rifle_status_line(Some(&rifle(0, 30, 0, 45, 90)));
        assert!(s.starts_with("RELOADING  50%"), "got `{s}`");
    }

    #[test]
    fn rifle_status_line_formats_empty() {
        let s = rifle_status_line(Some(&rifle(0, 30, 0, 0, 90)));
        assert_eq!(s, "EMPTY (0/30)");
    }

    #[test]
    fn rifle_status_line_formats_cooldown() {
        let s = rifle_status_line(Some(&rifle(15, 30, 5, 0, 90)));
        assert_eq!(s, "COOLDOWN 5t (15/30)");
    }

    #[test]
    fn rifle_status_line_formats_no_rifle() {
        let s = rifle_status_line(None);
        assert_eq!(s, "NO RIFLE");
    }
}
