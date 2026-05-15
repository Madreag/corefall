//! cf-ui — comic-noir HUD presentation surface.
//!
//! Layered across milestones:
//!
//! - **M1 status strip** (M1-004): four short text rows pinned to the top-left corner —
//!   STATUS / ITEM / HP / Reticle.
//! - **M1.5 mission strip** (M1.5-004): adds OBJECTIVE / MISSION / ENEMY / BREACH /
//!   EVENT lines so the reactive-guard scenario is readable from the HUD.
//! - **M4A readability + ACC-A floor** (M4A-001 / M4A-003 / M4A-004 / DR-012 closure):
//!   adds the body silhouette panel, the module strip, the stance line, the chassis
//!   banner stack, the tool-validity line, and the captions strip.  All HUD nodes
//!   carry stable accessibility ids so `cfctl observe` + AI agents see the same
//!   surface a sighted player does. Honors live `Settings.ui_scale` (200% scale +
//!   reflow), `Settings.high_contrast` (palette swap), `Settings.captions`
//!   (caption strip visibility), and `Settings.reduced_motion / reduced_shake /
//!   reduced_flash` (recorded in observe.accessibility).
//! - **M4B comic-noir polish** (BP7): layers slide/skew/cards on top of M4A's
//!   text-only banner stack without changing the HudState shape.

#![deny(unsafe_code)]
#![allow(
    clippy::module_name_repetitions,
    clippy::type_complexity,
    clippy::needless_pass_by_value,
    clippy::too_many_arguments
)]

use bevy::prelude::*;

use cf_actor::ActorObservation;

// M2 / M3 spec "## Files" wiring: the HUD widgets live in dedicated
// submodules so consumers that import per the spec paths resolve cleanly.
pub mod ai_debug_label;
pub mod enemy_hp;
pub mod last_event_ticker;
pub mod material_legend;
pub mod mission_resolved_modal;
pub mod mission_timer;
pub mod objective_banner;

// M8 spec § Files: 16 new HUD widget modules + the settings menu shell.
// Each module owns its own Bevy Resource state struct + helpers; cf-app's
// renderer mirrors them per frame from the engine snapshot.
pub mod action_prompt;
pub mod branching_banner;
pub mod compass;
pub mod cover_pip;
pub mod damage_direction;
pub mod grenade_arc;
pub mod hotbar;
pub mod lean_pip;
pub mod minimap;
pub mod phase_strip;
pub mod scope_reticle;
pub mod settings_menu;
pub mod squad_strip;
pub mod stamina_bar;
pub mod stealth_meter;
pub mod weapon_swap_overlay;

// **M9** § HUD readability + observability — reactor zone widgets.
pub mod reactor_hp_bar;
pub mod reactor_pressure_line;
pub mod timer_warnings;

pub use action_prompt::ActionPromptState;
pub use branching_banner::{BranchOption, BranchingBannerState};
pub use compass::{CompassBearing, CompassState, CARDINALS};
pub use cover_pip::{CoverLevel, CoverPipState};
pub use damage_direction::{DamageDirectionMarker, DamageDirectionState, DEFAULT_FADE_MS};
pub use grenade_arc::{ArcSample, GrenadeArcState};
pub use hotbar::{HotbarSlot, HotbarState, HOTBAR_SLOTS};
pub use lean_pip::{LeanPipState, LEAN_MAX_DEGREES, LEAN_MIN_DEGREES};
pub use material_legend::{legend_entries, MaterialLegendEntry, MaterialLegendState};
pub use minimap::{MinimapMarker, MinimapState, MINIMAP_SIZE_PX};
pub use phase_strip::{MissionPhase, PhaseStripState};
pub use scope_reticle::ScopeReticleState;
pub use settings_menu::{SettingsMenuState, SettingsTab};
pub use squad_strip::{SquadStripMember, SquadStripState, SQUAD_STRIP_MAX_MEMBERS};
pub use stamina_bar::{StaminaBarState, StaminaColor, STAMINA_CRITICAL_THRESHOLD, STAMINA_HIGH_THRESHOLD};
pub use stealth_meter::{StealthMeterState, SPOTTED_THRESHOLD};
pub use weapon_swap_overlay::{WeaponSwapOverlayState, SWAP_TRANSITION_MS};

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
    /// M4A: derived stance label (idle/walking/running/airborne/downed/dead).
    pub stance: String,
    /// M4A: per-zone body silhouette (head/torso/arms/legs hp%, 0..1).
    pub body_silhouette: HudBodySilhouette,
    /// M4A: chassis module strip placeholders (M5 fills with real chassis).
    pub modules: HudModuleStrip,
    /// M4A: priority-ordered banner stack (latest first).
    pub banners: Vec<HudBanner>,
    /// M4A: captions queue (audio-bound events; visible iff `captions=true`).
    pub captions: Vec<HudCaption>,
    /// M4A: tool-validity projection for the HUD TOOL line.
    pub tool_validity: Option<HudToolValidity>,
    /// W1.3: stability scalar (0.0=disrupted, 1.0=stable) from actor state.
    pub stability: f32,
    /// **M1 / Gap D3**: when `Some(label)` the CONTROLS CAPTURED HUD zone
    /// renders with `CONTROLS CAPTURED: <label>`; hidden when `None`.
    pub controls_captured_by: Option<String>,
}

/// M4A accessibility/settings mirror. cf-ui depends on `cf-actor` + `bevy` only;
/// the cf-app bridge writes this resource each frame from `cf-control::Settings`
/// (the live, mutable copy patched by `act.settings.set`) plus the engine's
/// HUD-cache snapshot (focus state).
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct HudSettings {
    pub ui_scale: f32,
    pub high_contrast: bool,
    pub captions: bool,
    pub reduced_motion: bool,
    pub reduced_shake: bool,
    pub reduced_flash: bool,
    pub hold_to_confirm: bool,
    pub hold_threshold_ms: u32,
    pub key_remap_enabled: bool,
    /// M4A: id of the currently-focused HUD node (drives the visible focus
    /// ring). `None` when focus is cleared (default + after F1).
    pub focused_node: Option<String>,
    /// **M1.5**: when true, the AI debug overlay renders a floating
    /// intent label above every reactive guard. When false the overlay
    /// is hidden (default). cf-app forwards the `--ai-debug` CLI flag
    /// into this field; `act.settings.set { ai_debug: ... }` mutates it
    /// at runtime.
    pub ai_debug: bool,
}

impl Default for HudSettings {
    fn default() -> Self {
        Self {
            ui_scale: 1.0,
            high_contrast: false,
            captions: true,
            reduced_motion: false,
            reduced_shake: false,
            reduced_flash: false,
            hold_to_confirm: false,
            hold_threshold_ms: 250,
            key_remap_enabled: false,
            focused_node: None,
            ai_debug: false,
        }
    }
}

/// M4A body silhouette per-zone hp percentages (clamped to `[0.0, 1.0]`).
#[derive(Debug, Clone, PartialEq)]
pub struct HudBodySilhouette {
    pub head_hp_pct: f32,
    pub torso_hp_pct: f32,
    pub arm_left_hp_pct: f32,
    pub arm_right_hp_pct: f32,
    pub leg_left_hp_pct: f32,
    pub leg_right_hp_pct: f32,
    pub placeholder: bool,
}

impl Default for HudBodySilhouette {
    fn default() -> Self {
        Self {
            head_hp_pct: 1.0,
            torso_hp_pct: 1.0,
            arm_left_hp_pct: 1.0,
            arm_right_hp_pct: 1.0,
            leg_left_hp_pct: 1.0,
            leg_right_hp_pct: 1.0,
            placeholder: true,
        }
    }
}

/// M4A module strip projection (placeholder until M5 owns chassis modules).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HudModuleStrip {
    pub modules: Vec<HudModule>,
    pub placeholder: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HudModule {
    pub id: String,
    pub label: String,
    pub state: String,
    pub kind: String,
}

/// M4A HUD banner — surfaced from chassis/status/mission events.
#[derive(Debug, Clone, PartialEq)]
pub struct HudBanner {
    pub id: String,
    pub severity: String,
    pub label: String,
    pub raised_at_tick: u64,
}

/// M4A HUD caption — surfaced from audio-bound events when captions are on.
#[derive(Debug, Clone, PartialEq)]
pub struct HudCaption {
    pub id: String,
    pub label: String,
    pub raised_at_tick: u64,
}

/// M4A HUD tool-validity projection for the TOOL line.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HudToolValidity {
    pub last_carve_tick: Option<u64>,
    pub last_refusal_tick: Option<u64>,
    pub last_refusal_reason: Option<String>,
    pub last_refusal_target: Option<String>,
    pub valid: bool,
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
    /// **M1.5**: DR-023 "Show me why" replay-handoff anchor for the
    /// mission-resolved modal. cf-ui surfaces a CTA button when
    /// `show_replay_cta` is true; the click handler hands the
    /// `show_me_why_event_id` to M3B's replay viewer (M3B owns the
    /// viewer; integration tested at BP3 close).
    pub show_me_why_event_id: Option<String>,
    pub show_replay_cta: bool,
}

/// M1.5 nearest-enemy summary.
#[derive(Debug, Clone, Default)]
pub struct HudEnemy {
    pub state: String,
    pub last_tactic: String,
    pub hp: f32,
    pub hp_max: f32,
    pub status: String,
    /// **M1.5**: floating intent label rendered above the guard's sprite
    /// when `HudSettings.ai_debug == true`. Empty string when no label is
    /// available.
    pub intent_label: String,
    /// **M1.5**: world position of the guard (in scene coords) so the
    /// floating label can be projected onto the HUD overlay. `None` when
    /// the engine did not provide a position.
    pub world_position: Option<[f32; 2]>,
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

/// **M1 / Gap D3**: text component for the CONTROLS CAPTURED HUD zone.
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

pub struct StatusStripPlugin;

impl Plugin for StatusStripPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudState>()
            .init_resource::<HudSettings>()
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

/// **M1 / Gap D3**: keep the CONTROLS CAPTURED HUD line in sync with
/// `HudState::controls_captured_by`. When `None`, the text is empty (hides
/// the line visually because the BorderColor stays transparent and the row
/// renders zero-content). When `Some(name)`, the text reads
/// `CONTROLS CAPTURED: <name>`.
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

/// M4A: stable accessibility id for a HUD node. Drives the focus ring map +
/// `cfctl ui` lookups. Mirrors the per-component canonical ids in the cf-control
/// `HUD_FOCUSABLE_NODES` constant.
#[derive(Component, Debug, Clone)]
pub struct HudAccessibilityId(pub &'static str);

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
            // M1 Gap D3: CONTROLS CAPTURED zone — invisible by default.
            // cf-app's `sync_hud_text` system updates the text to `CONTROLS
            // CAPTURED: <capturer>` whenever ControlsCaptureView.captured is
            // true and clears it otherwise.
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

/// M4A: focus ring color when no focus is set (transparent).
fn palette_focus_ring_clear() -> Color {
    Color::srgba(0.0, 0.0, 0.0, 0.0)
}

/// M4A: focus ring color when focus is set. High contrast = pure white;
/// otherwise a high-saturation amber that reads against the dark strip
/// background per WCAG 2.2 contrast guidance.
fn palette_focus_ring(high_contrast: bool) -> Color {
    if high_contrast {
        Color::srgb(1.0, 1.0, 1.0)
    } else {
        Color::srgb(1.0, 0.85, 0.0)
    }
}

/// M4A focus-ring update system: toggles the border color of each focusable
/// HUD wrapper based on `HudSettings.focused_node`. Reads from a single
/// shared source: the `HudAccessibilityId(&'static str)` component on each
/// wrapper. Each wrapper carries the canonical accessibility id from the
/// cf-control `HUD_FOCUSABLE_NODES` constant.
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

/// M4A: extend the banner + caption strip roots with accessibility ids so the
/// focus ring can highlight them.
#[derive(Component, Debug)]
pub struct BannerFocusWrapper;

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
            // 4 placeholder slots; we update text + visibility based on HudState.
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

/// M4A: high-contrast palette swap. Honors `HudSettings.high_contrast` per
/// DR-012 closure (200% scale + contrast). The accessibility floor requires
/// every state label to remain readable in high contrast, so this swaps the
/// strip background to fully opaque pure-black + text to pure white.
fn palette_text(high_contrast: bool) -> Color {
    if high_contrast {
        Color::srgb(1.0, 1.0, 1.0)
    } else {
        Color::srgb(0.96, 0.96, 0.92)
    }
}

fn palette_strip_bg(high_contrast: bool) -> Color {
    if high_contrast {
        Color::srgba(0.0, 0.0, 0.0, 1.0)
    } else {
        Color::srgba(0.0, 0.0, 0.0, 0.45)
    }
}

fn palette_banner_bg(high_contrast: bool, severity: &str) -> Color {
    if high_contrast {
        // High-contrast: solid black + text-only severity (no color cue).
        Color::srgba(0.0, 0.0, 0.0, 1.0)
    } else {
        match severity {
            "critical" => Color::srgba(0.7, 0.05, 0.05, 0.85),
            "warning" => Color::srgba(0.7, 0.5, 0.0, 0.85),
            _ => Color::srgba(0.0, 0.0, 0.0, 0.6),
        }
    }
}

/// M4A: apply UI scale from `HudSettings` to Bevy's `UiScale` resource. Bevy
/// scales every Px value, so ACC-A reflow depends on bounded percent-height
/// HUD bands plus flex wrapping rather than unbounded absolute columns.
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
    // Show critical first, then warning, then info; preserve raised-at-tick order within.
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

/// Format the banner line. Severity and an icon glyph are rendered alongside
/// the label so the HUD never communicates state with color alone (DR-012
/// ACC-A floor: color-independent state labels). The icon glyph uses ASCII
/// punctuation so it renders even when the configured TTF lacks emoji glyphs.
pub fn banner_line(banner: &HudBanner) -> String {
    let icon = match banner.severity.as_str() {
        "critical" => "[!!]",
        "warning" => "[!]",
        _ => "[*]",
    };
    format!(
        "{icon} {sev} {label}",
        icon = icon,
        sev = banner.severity.to_uppercase(),
        label = banner.label
    )
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
        // M2 audit pass 5 (2026-05-13): spec literal — TIMER turns yellow
        // at <30s, red at <10s. Default to the high-contrast-aware base
        // palette when no mission OR mission is not active.
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

/// Format the stance HUD line. The stance label IS the readable signal — color
/// is not used as the only cue. When a player observation is present and the
/// actor is in the air, the line tags `(airborne)` redundantly so screen
/// readers still describe the kinematic state when the stance string is e.g.
/// `WALKING` mid-jump.
pub fn stance_line(stance: &str, player: Option<&ActorObservation>) -> String {
    if stance.is_empty() {
        return "STANCE: --".to_string();
    }
    let air_marker = match player {
        Some(p) if !p.on_ground => " (airborne)",
        _ => "",
    };
    // M1 re-audit (2026-05-13): when knocked_down, prefer the KNOCKED_DOWN
    // descriptor over the per-stability cycle so the spec's literal six
    // descriptors (SOLID/SHAKEN/UNSTABLE/CRITICAL/DISRUPTED/KNOCKED_DOWN)
    // all surface.
    let stability_tag = match player {
        Some(p) if p.knockdown_ticks_remaining > 0 => {
            let pct = (p.stability * 100.0).round() as i32;
            format!(" | STABILITY {pct}% KNOCKED_DOWN")
        }
        Some(p) if p.stability < 0.9 => {
            let pct = (p.stability * 100.0).round() as i32;
            let label = if pct >= 60 {
                "SHAKEN"
            } else if pct >= 30 {
                "UNSTABLE"
            } else if pct > 0 {
                "CRITICAL"
            } else {
                "DISRUPTED"
            };
            format!(" | STABILITY {pct}% {label}")
        }
        _ => String::new(),
    };
    format!("STANCE: {}{}{}", stance.to_uppercase(), air_marker, stability_tag)
}

/// Format the stability HUD line. Shows the stability scalar as a percentage
/// with a readable label so the player knows WHY they feel sluggish, inaccurate,
/// or vulnerable. This is the A-FEEL-06 "damage cause explanation" surface.
///
/// **M1 re-audit (2026-05-13)**: when the actor is in a knockdown stun
/// (`knockdown_ticks_remaining > 0`), the descriptor reads "KNOCKED_DOWN"
/// instead of cycling through the stability percentage labels. This closes
/// the M1 spec drift item where the HUD STANCE stability descriptor was
/// missing the literal "KNOCKED_DOWN" state called out in the spec.
pub fn stability_line(stability: f32) -> String {
    stability_line_with_knockdown(stability, false)
}

/// Knockdown-aware variant of `stability_line`. When `knocked_down=true`,
/// returns "KNOCKED_DOWN" as the descriptor regardless of the stability
/// percentage. Used by the HUD bridge once the actor's
/// `knockdown_ticks_remaining > 0`.
pub fn stability_line_with_knockdown(stability: f32, knocked_down: bool) -> String {
    let pct = (stability * 100.0).round() as i32;
    let label = if knocked_down {
        "KNOCKED_DOWN"
    } else if pct >= 90 {
        "SOLID"
    } else if pct >= 60 {
        "SHAKEN"
    } else if pct >= 30 {
        "UNSTABLE"
    } else if pct > 0 {
        "CRITICAL"
    } else {
        "DISRUPTED"
    };
    format!("STABILITY: {pct}% {label}")
}

/// Format the silhouette HUD line. Renders six per-zone bars as ASCII so the
/// readability does not depend on color.
pub fn silhouette_line(body: &HudBodySilhouette) -> String {
    let placeholder_marker = if body.placeholder { "~" } else { "" };
    format!(
        "BODY{ph}: H{h:>3} T{t:>3} A{al:>3}/{ar:>3} L{ll:>3}/{lr:>3}",
        ph = placeholder_marker,
        h = (body.head_hp_pct * 100.0).round() as i32,
        t = (body.torso_hp_pct * 100.0).round() as i32,
        al = (body.arm_left_hp_pct * 100.0).round() as i32,
        ar = (body.arm_right_hp_pct * 100.0).round() as i32,
        ll = (body.leg_left_hp_pct * 100.0).round() as i32,
        lr = (body.leg_right_hp_pct * 100.0).round() as i32,
    )
}

/// Format the module strip HUD line. Color-independent: each module's state
/// label is text (`nominal` / `degraded` / `warning` / `failed` / `not_present`).
pub fn module_line(modules: &HudModuleStrip) -> String {
    if modules.modules.is_empty() {
        return "MODS: --".to_string();
    }
    let placeholder_marker = if modules.placeholder { "~" } else { "" };
    let mut s = format!("MODS{}:", placeholder_marker);
    for m in &modules.modules {
        s.push(' ');
        if m.state == "not_present" {
            s.push_str(&format!("{}:N/A", compact_module_name(&m.kind)));
        } else if modules.placeholder {
            s.push_str(&m.label.replace('—', "-"));
        } else {
            let state_tag = match m.state.as_str() {
                "nominal" => "OK",
                "degraded" => "DEG",
                "warning" => "WARN",
                "failed" => "FAIL",
                other => other,
            };
            s.push_str(&format!("{}:{}", compact_module_name(&m.kind), state_tag));
        }
    }
    s
}

fn compact_module_name(kind: &str) -> &'static str {
    match kind {
        "weapon_mount" => "WEAPON",
        "jet" => "JET",
        "shield" => "SHIELD",
        "sensor" => "SENSOR",
        "repair_drone" => "REPAIR",
        _ => "MOD",
    }
}

/// Format the tool-validity HUD line.
pub fn tool_line(validity: Option<&HudToolValidity>) -> String {
    let Some(v) = validity else {
        return "TOOL: --".to_string();
    };
    if v.valid {
        match v.last_carve_tick {
            Some(t) => format!("TOOL: VALID (last carve @ {t}t)"),
            None => "TOOL: VALID".to_string(),
        }
    } else {
        let reason = v.last_refusal_reason.as_deref().unwrap_or("unknown");
        match v.last_refusal_target.as_deref() {
            Some(target) => format!("TOOL: REFUSED | {reason} ({target})"),
            None => format!("TOOL: REFUSED | {reason}"),
        }
    }
}

/// **M2 audit pass 5 (2026-05-13)**: return the TIMER text color per spec
/// "TIMER turns yellow at <30s, red at <10s". Green for >30s remaining;
/// yellow for 10..=30s; red for <10s. Inactive mission OR no time limit
/// returns the default base-palette color so the strip stays readable.
pub fn mission_timer_color(mission: Option<&HudMission>, tick_rate_hz: u32, high_contrast: bool) -> Color {
    let Some(m) = mission else {
        return palette_text(high_contrast);
    };
    // Only color the timer while the mission is in progress + has a
    // finite time limit. WIN / LOST / ABORTED keep the base palette.
    let in_progress = matches!(m.result.as_str(), "in_progress" | "active");
    if !in_progress || m.time_limit_ticks == 0 {
        return palette_text(high_contrast);
    }
    let rate = tick_rate_hz.max(1) as f32;
    let remaining_s = ((m.time_limit_ticks.saturating_sub(m.elapsed_ticks)) as f32 / rate).max(0.0);
    if remaining_s < 10.0 {
        Color::srgb(1.0, 0.25, 0.25) // red
    } else if remaining_s < 30.0 {
        Color::srgb(1.0, 0.85, 0.2) // yellow
    } else {
        Color::srgb(0.4, 1.0, 0.4) // green
    }
}

/// Format the mission HUD line. Public for unit tests.
pub fn mission_line(mission: Option<&HudMission>, tick_rate_hz: u32) -> String {
    let Some(m) = mission else {
        return "MISSION: --".to_string();
    };
    let rate = tick_rate_hz.max(1) as f32;
    let elapsed_s = m.elapsed_ticks as f32 / rate;
    // M2 audit pass 7 (2026-05-13): TIMER countdown in MM:SS form per spec
    // literal "TIMER shows MM:SS countdown". Active missions: show
    // remaining time as countdown; Won/Lost: show elapsed.
    let in_progress = matches!(m.result.as_str(), "in_progress" | "active");
    let total = if m.time_limit_ticks > 0 {
        if in_progress {
            let remaining_s = (m.time_limit_ticks.saturating_sub(m.elapsed_ticks) as f32 / rate).max(0.0);
            let minutes = (remaining_s as u32) / 60;
            let seconds = (remaining_s as u32) % 60;
            format!(" / {minutes:02}:{seconds:02}")
        } else {
            format!(" / {:.0}s", m.time_limit_ticks as f32 / rate)
        }
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

/// **M1.5**: format the floating AI debug intent label rendered above the
/// guard sprite when `Settings.ai_debug == true`. Returns `None` when the
/// overlay is disabled OR no enemy is available so cf-app can despawn the
/// text node. Acceptance criterion 'AI debug labels'.
pub fn ai_debug_label(enemy: Option<&HudEnemy>, settings: &HudSettings) -> Option<String> {
    if !settings.ai_debug {
        return None;
    }
    let e = enemy?;
    if e.intent_label.is_empty() {
        return None;
    }
    Some(e.intent_label.clone())
}

/// **M1.5**: spec says the mission-resolved modal renders a "Show me why"
/// CTA button when the mission was lost. Returns the divergence event_id
/// the CTA should hand to M3B's replay viewer when clicked, or `None` if
/// the CTA should be hidden. Acceptance criterion 'Win/loss outcome modal
/// with "show me why" (DR-023 handoff)'.
pub fn show_replay_cta_event_id(mission: Option<&HudMission>) -> Option<String> {
    let m = mission?;
    if !m.show_replay_cta {
        return None;
    }
    m.show_me_why_event_id.clone()
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
        // M2 audit pass 7 (2026-05-13): active missions now show
        // remaining time as MM:SS countdown per spec literal.
        let m = HudMission {
            result: "active".to_string(),
            loss_reason: None,
            elapsed_ticks: 60,
            time_limit_ticks: 5400,
            ticks_remaining: Some(5340),
            active_objective: Some("breach".to_string()),
            last_event_label: "mission_started".to_string(),
            show_me_why_event_id: None,
            show_replay_cta: false,
        };
        // 5400 - 60 = 5340 ticks remaining at 60Hz = 89.0s = 01:29.
        assert_eq!(mission_line(Some(&m), 60), "MISSION: ACTIVE  1.0s / 01:29");
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
    fn stance_line_uppercases_and_appends_airborne_marker() {
        assert_eq!(stance_line("idle", None), "STANCE: IDLE");
        let player = ActorObservation {
            id: 1,
            team: "blue".into(),
            controllable: true,
            position: [0.0, 10.0],
            velocity: [0.0, 0.0],
            aim: [1.0, 0.0],
            on_ground: false,
            status: "stable".into(),
            hp: 100.0,
            hp_max: 100.0,
            selected_slot: 0,
            selected_item: "rifle".into(),
            inventory: vec!["rifle".into(), "empty".into(), "empty".into(), "empty".into()],
            stance: "airborne".into(),
            body_silhouette: cf_actor::BodySilhouette::default(),
            chassis: None,
            origin_id: "human".into(),
            stability: 1.0,
            stability_recovery_rate: 0.02,
            mass_kg: 80.0,
            crouch_active: false,
            climb_active: false,
            jet_active: false,
            sharp_aim_progress: 0.0,
            recoil_accumulator: 0.0,
            knockdown_ticks_remaining: 0,
            dying_dwell_ticks_remaining: 0,
            mission_critical: false,
            bloom_factor: 1.0,
            facing: "right".into(),
            stamina: 1.0,
            stamina_max: 1.0,
            sprint_active: false,
            prone_active: false,
            lean_angle_degrees: 0.0,
            lean_direction: "none".into(),
            stealth_meter: 0.0,
            spotted: false,
            cover_side: "none".into(),
            cover_effectiveness: 0.0,
            inventory_weight_kg: 0.0,
            weight_forces_walk: false,
            limb_loss: cf_actor::LimbLossFlags::default(),
            inventory_extended: Vec::new(),
            weapon_state: cf_actor::WeaponStateView::default(),
        };
        let line = stance_line("airborne", Some(&player));
        assert!(line.contains("AIRBORNE"));
        assert!(line.contains("(airborne)"));
    }

    #[test]
    fn silhouette_line_renders_per_zone_pct_with_placeholder_marker() {
        let body = HudBodySilhouette {
            head_hp_pct: 0.6,
            torso_hp_pct: 0.6,
            arm_left_hp_pct: 0.6,
            arm_right_hp_pct: 0.6,
            leg_left_hp_pct: 0.6,
            leg_right_hp_pct: 0.6,
            placeholder: true,
        };
        let line = silhouette_line(&body);
        assert!(line.starts_with("BODY~:"));
        assert!(line.contains("H 60"));
        assert!(line.contains("T 60"));
        assert!(line.contains("A 60/ 60"));
        assert!(line.contains("L 60/ 60"));
    }

    #[test]
    fn module_line_aggregates_module_labels_with_placeholder_marker() {
        let mods = HudModuleStrip {
            modules: vec![HudModule {
                id: "weapon_mount".into(),
                label: "READY 30/30".into(),
                state: "nominal".into(),
                kind: "weapon_mount".into(),
            }],
            placeholder: true,
        };
        let s = module_line(&mods);
        assert!(s.starts_with("MODS~:"));
        assert!(s.contains("READY 30/30"));
        assert!(s.is_ascii());
    }

    #[test]
    fn sanitize_hud_text_replaces_missing_glyph_candidates() {
        assert_eq!(sanitize_hud_text("actor 1 → unstable"), "actor 1   unstable");
    }

    #[test]
    fn tool_line_handles_valid_and_refused_states() {
        let valid = HudToolValidity {
            valid: true,
            last_carve_tick: Some(120),
            ..HudToolValidity::default()
        };
        assert_eq!(tool_line(Some(&valid)), "TOOL: VALID (last carve @ 120t)");
        let refused = HudToolValidity {
            valid: false,
            last_refusal_reason: Some("material_metal_nohook".into()),
            last_refusal_target: Some("anchor_post".into()),
            ..HudToolValidity::default()
        };
        let s = tool_line(Some(&refused));
        assert!(s.contains("REFUSED"));
        assert!(s.contains("material_metal_nohook"));
        assert!(s.contains("anchor_post"));
        assert_eq!(tool_line(None), "TOOL: --");
    }

    #[test]
    fn banner_line_includes_severity_word_and_icon() {
        let critical = HudBanner {
            id: "eject_now".into(),
            severity: "critical".into(),
            label: "EJECT NOW".into(),
            raised_at_tick: 90,
        };
        let s = banner_line(&critical);
        assert!(s.contains("[!!]"));
        assert!(s.contains("CRITICAL"));
        assert!(s.contains("EJECT NOW"));

        let warning = HudBanner {
            id: "ammo_out".into(),
            severity: "warning".into(),
            label: "AMMO OUT".into(),
            raised_at_tick: 200,
        };
        let s = banner_line(&warning);
        assert!(s.contains("[!]"));
        assert!(s.contains("WARNING"));
    }

    #[test]
    fn palette_helpers_swap_for_high_contrast() {
        let normal = palette_text(false);
        let hc = palette_text(true);
        assert_ne!(normal, hc);
        let normal_bg = palette_strip_bg(false);
        let hc_bg = palette_strip_bg(true);
        assert_ne!(normal_bg, hc_bg);
        // High-contrast critical banner falls back to solid black (no color cue).
        let hc_critical = palette_banner_bg(true, "critical");
        let normal_critical = palette_banner_bg(false, "critical");
        assert_ne!(hc_critical, normal_critical);
    }

    #[test]
    fn enemy_line_summarises_state() {
        let e = HudEnemy {
            state: "engaged".to_string(),
            last_tactic: "attack_target".to_string(),
            hp: 50.0,
            hp_max: 80.0,
            status: "stable".to_string(),
            intent_label: String::new(),
            world_position: None,
        };
        let s = enemy_line(Some(&e));
        assert!(s.contains("ENGAGED"));
        assert!(s.contains("attack_target"));
        assert!(s.contains("hp=50/80"));
    }

    #[test]
    fn ai_debug_label_hidden_when_flag_off() {
        let enemy = HudEnemy {
            intent_label: "ENGAGED: ATTACK".to_string(),
            ..Default::default()
        };
        let settings = HudSettings {
            ai_debug: false,
            ..Default::default()
        };
        assert_eq!(ai_debug_label(Some(&enemy), &settings), None);
    }

    #[test]
    fn ai_debug_label_renders_when_flag_on() {
        let enemy = HudEnemy {
            intent_label: "ALERT: SEARCH".to_string(),
            ..Default::default()
        };
        let settings = HudSettings {
            ai_debug: true,
            ..Default::default()
        };
        assert_eq!(
            ai_debug_label(Some(&enemy), &settings),
            Some("ALERT: SEARCH".to_string())
        );
    }

    #[test]
    fn ai_debug_label_hidden_when_no_enemy() {
        let settings = HudSettings {
            ai_debug: true,
            ..Default::default()
        };
        assert_eq!(ai_debug_label(None, &settings), None);
    }

    #[test]
    fn show_replay_cta_hidden_for_won_mission() {
        let mission = HudMission {
            result: "won".to_string(),
            show_replay_cta: false,
            show_me_why_event_id: None,
            ..Default::default()
        };
        assert_eq!(show_replay_cta_event_id(Some(&mission)), None);
    }

    #[test]
    fn show_replay_cta_returns_event_id_for_lost_mission() {
        let mission = HudMission {
            result: "lost".to_string(),
            loss_reason: Some("player_dead".to_string()),
            show_replay_cta: true,
            show_me_why_event_id: Some("event:704:3354".to_string()),
            ..Default::default()
        };
        assert_eq!(
            show_replay_cta_event_id(Some(&mission)),
            Some("event:704:3354".to_string())
        );
    }

    #[test]
    fn show_replay_cta_hidden_when_no_mission() {
        assert_eq!(show_replay_cta_event_id(None), None);
    }
}
