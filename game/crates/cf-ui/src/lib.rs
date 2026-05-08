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
    /// M1.5: mission state machine bundle. `None` for sandbox scenarios.
    pub mission: Option<HudMission>,
    /// M1.5: nearest enemy summary (the M1.5 scenario has at most one).
    pub enemy: Option<HudEnemy>,
    /// M1.5: nearest breach strip the player is in range of.
    pub breach: Option<HudBreach>,
    /// M1.5: last important event label (mission/objective/state-change).
    pub last_event: Option<String>,
}

/// M1.5 mission HUD bundle.
#[derive(Debug, Clone, Default)]
pub struct HudMission {
    pub result: String,
    pub loss_reason: Option<String>,
    pub elapsed_ticks: u64,
    pub time_limit_ticks: u64,
    pub ticks_remaining: Option<u64>,
    pub active_objective: Option<String>,
    pub last_event_label: String,
}

/// M1.5 nearest-enemy summary.
#[derive(Debug, Clone, Default)]
pub struct HudEnemy {
    pub state: String,
    pub last_tactic: String,
    pub hp: f32,
    pub hp_max: f32,
    pub status: String,
}

/// M1.5 nearest-breach summary.
#[derive(Debug, Clone, Default)]
pub struct HudBreach {
    pub id: String,
    pub material: String,
    pub hp: f32,
    pub max_hp: f32,
    pub broken: bool,
    pub refusal_reason: Option<String>,
    pub in_range: bool,
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

pub struct StatusStripPlugin;

impl Plugin for StatusStripPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudState>()
            .add_systems(Startup, spawn_status_strip)
            .add_systems(Update, update_status_strip);
    }
}

fn spawn_status_strip(mut commands: Commands) {
    let root_node = Node {
        position_type: PositionType::Absolute,
        top: Val::Px(12.0),
        left: Val::Px(12.0),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(2.0),
        padding: UiRect::all(Val::Px(8.0)),
        ..default()
    };
    let text_font = TextFont {
        font_size: 18.0,
        ..default()
    };
    let text_color = TextColor(Color::srgb(0.96, 0.96, 0.92));
    commands
        .spawn((
            root_node,
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
            StatusStripRoot,
            Name::new("cf::ui::status_strip"),
        ))
        .with_children(|parent| {
            parent.spawn((Text::new("STATUS: --"), text_font.clone(), text_color, StatusStripText));
            parent.spawn((Text::new("ITEM: --"), text_font.clone(), text_color, ItemStripText));
            parent.spawn((Text::new("HP: --"), text_font.clone(), text_color, AmmoStripText));
            parent.spawn((Text::new("NO RIFLE"), text_font.clone(), text_color, ReticleStripText));
            parent.spawn((
                Text::new("OBJECTIVE: --"),
                text_font.clone(),
                text_color,
                ObjectiveStripText,
            ));
            parent.spawn((
                Text::new("MISSION: --"),
                text_font.clone(),
                text_color,
                MissionStripText,
            ));
            parent.spawn((Text::new("ENEMY: --"), text_font.clone(), text_color, EnemyStripText));
            parent.spawn((Text::new("BREACH: --"), text_font.clone(), text_color, BreachStripText));
            parent.spawn((Text::new("EVENT: --"), text_font, text_color, LastEventStripText));
        });
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn update_status_strip(
    state: Res<HudState>,
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
        &mut Text,
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
    if let Some(mut text) = mission_query.iter_mut().next() {
        **text = mission_line(state.mission.as_ref(), state.tick_rate_hz);
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
}

/// Format the mission HUD line. Public for unit tests.
pub fn mission_line(mission: Option<&HudMission>, tick_rate_hz: u32) -> String {
    let Some(m) = mission else {
        return "MISSION: --".to_string();
    };
    let rate = tick_rate_hz.max(1) as f32;
    let elapsed_s = m.elapsed_ticks as f32 / rate;
    let total = if m.time_limit_ticks > 0 {
        format!(" / {:.0}s", m.time_limit_ticks as f32 / rate)
    } else {
        String::new()
    };
    let label = match m.result.as_str() {
        "won" => "WON".to_string(),
        "lost" => format!("LOST ({})", m.loss_reason.clone().unwrap_or_else(|| "?".into())),
        _ => "ACTIVE".to_string(),
    };
    format!("MISSION: {label} {elapsed_s:>4.1}s{total}")
}

/// Format the objective line. Public for unit tests.
pub fn objective_line(mission: Option<&HudMission>) -> String {
    let Some(m) = mission else {
        return "OBJECTIVE: --".to_string();
    };
    match &m.active_objective {
        Some(id) => format!("OBJECTIVE: {id}"),
        None => "OBJECTIVE: (none active)".to_string(),
    }
}

/// Format the enemy summary line.
pub fn enemy_line(enemy: Option<&HudEnemy>) -> String {
    let Some(e) = enemy else {
        return "ENEMY: --".to_string();
    };
    format!(
        "ENEMY: {} hp={}/{}, {} ({})",
        e.status.to_uppercase(),
        e.hp as i32,
        e.hp_max as i32,
        e.state.to_uppercase(),
        e.last_tactic
    )
}

/// Format the breach summary line.
pub fn breach_line(breach: Option<&HudBreach>) -> String {
    let Some(b) = breach else {
        return "BREACH: --".to_string();
    };
    if b.broken {
        return format!("BREACH: {} BROKEN", b.id);
    }
    if let Some(reason) = &b.refusal_reason {
        return format!("BREACH: {} REFUSED ({})", b.id, reason);
    }
    let pct = if b.max_hp > 0.0 { (b.hp / b.max_hp) * 100.0 } else { 0.0 };
    let range_label = if b.in_range { "" } else { " (out of range)" };
    format!(
        "BREACH: {} {}/{} ({:>3.0}%){range_label}",
        b.id, b.hp as i32, b.max_hp as i32, pct
    )
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

    #[test]
    fn mission_line_formats_active_with_timer() {
        let m = HudMission {
            result: "active".to_string(),
            loss_reason: None,
            elapsed_ticks: 60,
            time_limit_ticks: 5400,
            ticks_remaining: Some(5340),
            active_objective: Some("breach".to_string()),
            last_event_label: "mission_started".to_string(),
        };
        assert_eq!(mission_line(Some(&m), 60), "MISSION: ACTIVE  1.0s / 90s");
    }

    #[test]
    fn mission_line_formats_won_and_lost() {
        let won = HudMission {
            result: "won".to_string(),
            ..HudMission::default()
        };
        assert!(mission_line(Some(&won), 60).starts_with("MISSION: WON"));
        let lost = HudMission {
            result: "lost".to_string(),
            loss_reason: Some("player_dead".to_string()),
            ..HudMission::default()
        };
        assert!(mission_line(Some(&lost), 60).starts_with("MISSION: LOST (player_dead)"));
    }

    #[test]
    fn breach_line_formats_progress_and_broken_states() {
        let progress = HudBreach {
            id: "outer_wall".to_string(),
            material: "concrete_soft".to_string(),
            hp: 30.0,
            max_hp: 60.0,
            broken: false,
            refusal_reason: None,
            in_range: true,
        };
        assert!(breach_line(Some(&progress)).contains("50%"));
        let broken = HudBreach {
            broken: true,
            id: "outer_wall".to_string(),
            ..HudBreach::default()
        };
        assert_eq!(breach_line(Some(&broken)), "BREACH: outer_wall BROKEN");
        let metal = HudBreach {
            id: "anchor".to_string(),
            refusal_reason: Some("metal_nohook".to_string()),
            ..HudBreach::default()
        };
        assert_eq!(breach_line(Some(&metal)), "BREACH: anchor REFUSED (metal_nohook)");
    }

    #[test]
    fn objective_line_handles_no_mission() {
        assert_eq!(objective_line(None), "OBJECTIVE: --");
        let m = HudMission {
            active_objective: Some("extract".to_string()),
            ..HudMission::default()
        };
        assert_eq!(objective_line(Some(&m)), "OBJECTIVE: extract");
    }

    #[test]
    fn enemy_line_summarises_state() {
        let e = HudEnemy {
            state: "engaged".to_string(),
            last_tactic: "attack_target".to_string(),
            hp: 50.0,
            hp_max: 80.0,
            status: "stable".to_string(),
        };
        let s = enemy_line(Some(&e));
        assert!(s.contains("ENGAGED"));
        assert!(s.contains("attack_target"));
        assert!(s.contains("hp=50/80"));
    }
}
