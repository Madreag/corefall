use bevy::prelude::*;

use cf_actor::ActorObservation;

/// Latest HUD model derived from the engine. The cf-app bridge writes this each
/// frame from the same `M0Engine` snapshot it feeds to `cf-render-2d::ActorRenderState`.
#[derive(Resource, Debug, Clone, Default)]
pub struct HudState {
    pub player: Option<ActorObservation>,
    pub rifle: Option<HudRifle>,
    pub tick: u64,
    pub tick_rate_hz: u32,
    pub mission: Option<HudMission>,
    pub enemy: Option<HudEnemy>,
    pub breach: Option<HudBreach>,
    pub last_event: Option<String>,
    pub stance: String,
    pub body_silhouette: HudBodySilhouette,
    pub modules: HudModuleStrip,
    pub banners: Vec<HudBanner>,
    pub captions: Vec<HudCaption>,
    pub tool_validity: Option<HudToolValidity>,
    pub stability: f32,
    pub controls_captured_by: Option<String>,
    pub resources: HudResources,
    pub concussion: HudConcussion,
}

/// M17 per-origin survival resources mirrored from the engine's origin tick.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HudResources {
    pub origin: String,
    pub blood: f32,
    pub blood_max: f32,
    pub oil: f32,
    pub oil_max: f32,
    pub power: f32,
    pub power_max: f32,
    pub caloric: f32,
    pub oxygen_seconds: f32,
    pub heat: f32,
    pub internal_shock_dose: f32,
    pub power_fire_locked: bool,
    pub overclock_tier: u8,
    pub throttled: bool,
}

/// M17 G-force / concussion HUD projection (drives the blackout vignette).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HudConcussion {
    pub dose: f32,
    pub band: String,
    pub vignette_fraction: f32,
    pub ducks_ambient: bool,
}

/// HUD accessibility/settings mirror. cf-app bridge writes this from
/// `cf-control::Settings` plus the engine HUD-cache snapshot.
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
    pub focused_node: Option<String>,
    pub ai_debug: bool,
    pub comic_style_overlay: String,
    pub comic_death_recap: bool,
    pub reduced_g_force_blackout: bool,
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
            comic_style_overlay: "subtle".to_string(),
            comic_death_recap: false,
            reduced_g_force_blackout: false,
        }
    }
}

/// Per-zone body silhouette hp percentages (clamped to `[0.0, 1.0]`).
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

/// Module strip projection (placeholder until M5 owns chassis modules).
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

/// HUD banner — surfaced from chassis/status/mission events.
#[derive(Debug, Clone, PartialEq)]
pub struct HudBanner {
    pub id: String,
    pub severity: String,
    pub label: String,
    pub raised_at_tick: u64,
}

/// HUD caption — surfaced from audio-bound events when captions are on.
#[derive(Debug, Clone, PartialEq)]
pub struct HudCaption {
    pub id: String,
    pub label: String,
    pub raised_at_tick: u64,
}

/// HUD tool-validity projection for the TOOL line.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HudToolValidity {
    pub last_carve_tick: Option<u64>,
    pub last_refusal_tick: Option<u64>,
    pub last_refusal_reason: Option<String>,
    pub last_refusal_target: Option<String>,
    pub valid: bool,
}

/// Mission HUD bundle.
#[derive(Debug, Clone, Default)]
pub struct HudMission {
    pub result: String,
    pub loss_reason: Option<String>,
    pub elapsed_ticks: u64,
    pub time_limit_ticks: u64,
    pub ticks_remaining: Option<u64>,
    pub active_objective: Option<String>,
    pub last_event_label: String,
    pub show_me_why_event_id: Option<String>,
    pub show_replay_cta: bool,
}

/// Nearest-enemy summary.
#[derive(Debug, Clone, Default)]
pub struct HudEnemy {
    pub state: String,
    pub last_tactic: String,
    pub hp: f32,
    pub hp_max: f32,
    pub status: String,
    pub intent_label: String,
    pub world_position: Option<[f32; 2]>,
}

/// Nearest-breach summary.
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
