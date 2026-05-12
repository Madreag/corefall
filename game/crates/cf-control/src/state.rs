//! Shared engine-side state types observed via the cf-control envelope.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::settings::Settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Paused,
    Stepping,
    Ended,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EngineState {
    pub run_id: String,
    pub scenario: String,
    pub tick: u64,
    pub sim_time_ms: f64,
    pub run_status: RunStatus,
    pub seed: u64,
    pub tick_rate_hz: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ObserveFrame {
    pub schema_version: u32,
    pub run_id: String,
    pub tick: u64,
    pub sim_time_ms: f64,
    pub run_status: RunStatus,
    pub scenario: String,
    pub events_since: u64,
    pub events: Vec<serde_json::Value>,
    pub settings: ObserveSettings,
    /// M1: typed projection of every actor in the world. Empty in M0 scenarios.
    #[serde(default)]
    pub actors: Vec<ActorView>,
    /// Convenience pointer to the player actor in `actors` by id, if any.
    #[serde(default)]
    pub player_actor_id: Option<u64>,
    /// M1.5: mission state machine projection. `None` for sandbox scenarios.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mission: Option<MissionView>,
    /// M1.5: breach strips in the scenario. Empty for sandbox scenarios.
    #[serde(default)]
    pub breaches: Vec<BreachView>,
    /// M1.5: reactive guards and their last-tick view. Empty for sandbox scenarios.
    #[serde(default)]
    pub enemies: Vec<EnemyView>,
    /// M2: chunked terrain summary (per-material counts + carve / refusal /
    /// dirty-chunk counters). `None` when the scenario has no chunked terrain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terrain: Option<TerrainView>,
    /// M2.5: reactor world projection. Empty for scenarios with no reactors.
    #[serde(default)]
    pub reactors: Vec<ReactorView>,
    /// M4A: HUD banner queue (status/chassis/mission events that should be
    /// surfaced as a top-priority banner). FIFO; HUD draws the most recent N.
    #[serde(default)]
    pub banners: Vec<HudBannerView>,
    /// M4A: captions queue (audio-bound events surfaced as text when
    /// `Settings.captions == true`). Drained in FIFO order; HUD draws last N.
    #[serde(default)]
    pub captions: Vec<CaptionView>,
    /// M4A: tool-validity projection for the HUD TOOL line + AI agents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_validity: Option<ToolValidityView>,
    /// M4A: resolved accessibility surface (UI scale applied, high-contrast
    /// palette active, captions visible, focusable nodes in z-order).
    #[serde(default)]
    pub accessibility: AccessibilityView,
}

/// M1.5 mission projection (re-exposed via JsonSchema-friendly types).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MissionView {
    pub result: String,
    pub loss_reason: Option<String>,
    pub elapsed_ticks: u64,
    pub time_limit_ticks: u64,
    pub ticks_remaining: Option<u64>,
    pub active_objective: Option<String>,
    pub objectives: Vec<ObjectiveView>,
    pub last_event_tick: u64,
    pub last_event_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ObjectiveView {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub optional: bool,
    pub target_actor: Option<u64>,
    pub target_breach: Option<String>,
    pub target_reactor: Option<String>,
    pub zone_min: Option<[f32; 2]>,
    pub zone_max: Option<[f32; 2]>,
}

/// M2 chunked terrain projection. Per-material pixel counts let cfctl + AI
/// hooks query "how much air do we have left?" without pulling the full
/// snapshot. Carve / refusal / dirty counters expose perf health.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TerrainView {
    pub width_px: u32,
    pub height_px: u32,
    pub anchor: [f32; 2],
    pub default_material: String,
    pub carve_count: u64,
    pub refusal_count: u64,
    pub dirty_chunk_count: u32,
    pub allocated_chunk_count: u32,
    pub material_counts: std::collections::BTreeMap<String, u64>,
}

/// M2.5 reactor projection. Drives the HUD reactor-hp bar + the cfctl
/// `inspect reactor <id>` lookup.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReactorView {
    pub id: String,
    pub position: [f32; 2],
    pub half_extents: [f32; 2],
    pub hp: f32,
    pub max_hp: f32,
    pub destroyed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BreachView {
    pub id: String,
    pub material: String,
    pub bbox_min: [f32; 2],
    pub bbox_max: [f32; 2],
    pub hp: f32,
    pub max_hp: f32,
    pub broken: bool,
    pub refusal_reason: Option<String>,
    pub dig_range: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EnemyView {
    pub actor: u64,
    pub state: String,
    pub last_tactic: String,
    pub ammo: u32,
    pub mag_capacity: u32,
    pub fire_cooldown_ticks: u32,
    pub reload_remaining_ticks: u32,
    pub aim_settle_remaining_ticks: u32,
    pub alert_dwell_remaining_ticks: u32,
    pub aim: [f32; 2],
}

/// Public projection of one actor for the observe envelope. Mirrors
/// `cf_actor::ActorObservation` with extra fields (rifle ammo / cooldown / reload
/// state) the engine wires through.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ActorView {
    pub id: u64,
    pub team: String,
    pub controllable: bool,
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub aim: [f32; 2],
    pub on_ground: bool,
    pub status: String,
    pub hp: f32,
    pub hp_max: f32,
    pub selected_slot: u32,
    pub selected_item: String,
    pub rifle_ammo: Option<u32>,
    pub rifle_capacity: Option<u32>,
    pub rifle_fire_cooldown_ticks: Option<u32>,
    pub rifle_reload_remaining_ticks: Option<u32>,
    pub rifle_reload_total_ticks: Option<u32>,
    /// M4A: derived stance label (idle/walking/running/airborne/downed/dead/...).
    /// `cfctl observe` consumers + AI agents read this without a screenshot.
    #[serde(default = "default_stance")]
    pub stance: String,
    /// M4A: per-zone body silhouette projection (head/torso/arms/legs hp%).
    /// **M5**: `placeholder=false` when sourced from a real chassis body graph;
    /// `placeholder=true` for legacy actors without a chassis.
    #[serde(default)]
    pub body_silhouette: BodySilhouetteView,
    /// M4A: chassis module strip projection. **M5**: `placeholder=false` when
    /// sourced from a real chassis; `placeholder=true` for the M4A flat-rifle
    /// derivation.
    #[serde(default)]
    pub module_strip: ModuleStripView,
    /// **M5**: full chassis projection (per-zone integrity, modules, stage,
    /// pilot state, eject window). `None` for legacy actors without a chassis.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chassis: Option<ChassisView>,
    /// **M5**: actor origin tag (`human`, `robot`, `android`).
    #[serde(default = "default_origin_id")]
    pub origin_id: String,
    /// **M5**: per-tick movement-intent flags surfaced to AI / HUD.
    #[serde(default)]
    pub crouch_active: bool,
    #[serde(default)]
    pub climb_active: bool,
    #[serde(default)]
    pub jet_active: bool,
    /// **M1**: stability scalar (0.0 = disrupted, 1.0 = solid).
    #[serde(default = "default_stability")]
    pub stability: f32,
    /// **M1**: per-tick stability recovery rate.
    #[serde(default = "default_stability_recovery_rate")]
    pub stability_recovery_rate: f32,
    /// **M1**: sharp-aim progress (0..1).
    #[serde(default)]
    pub sharp_aim_progress: f32,
    /// **M1**: alternating-recoil accumulator value (CCCP `HDFirearm.cpp:891`).
    #[serde(default)]
    pub recoil_accumulator: f32,
    /// **M1**: knockdown recovery ticks remaining (0 when not stunned).
    #[serde(default)]
    pub knockdown_ticks_remaining: u32,
    /// **M1**: DYING dwell ticks remaining (CCCP `Actor.cpp:1229`).
    #[serde(default)]
    pub dying_dwell_ticks_remaining: u32,
    /// **M1**: mission_critical flag (caps damage at DOWNED when true).
    #[serde(default)]
    pub mission_critical: bool,
    /// **M1**: most-recent reticle bloom factor (`Soldat Sprites.pas:4870`).
    #[serde(default = "default_bloom_factor")]
    pub bloom_factor: f32,
    /// **M1**: physical mass (kg). Drives mass_factor in physics + stability cost.
    #[serde(default = "default_mass_kg")]
    pub mass_kg: f32,
}

fn default_stability() -> f32 {
    1.0
}
fn default_stability_recovery_rate() -> f32 {
    0.02
}
fn default_bloom_factor() -> f32 {
    1.0
}
fn default_mass_kg() -> f32 {
    80.0
}

fn default_origin_id() -> String {
    "human".to_string()
}

/// **M5**: chassis projection for `cfctl observe` / `inspect`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ChassisView {
    pub spec_id: String,
    pub kind: String,
    pub stage: String,
    pub pilot_state: String,
    pub weapon_jammed: bool,
    pub tutorial_safety: bool,
    pub mass_kg: f32,
    pub zones: Vec<ChassisZoneView>,
    pub modules: Vec<ChassisModuleView>,
    pub integrity: f32,
    pub eject_ticks_remaining: u32,
    pub eject_ticks_total: u32,
    pub destroyed_zones: Vec<String>,
    pub salvaged_module_ids: Vec<String>,
}

impl From<&cf_actor::ChassisView> for ChassisView {
    fn from(v: &cf_actor::ChassisView) -> Self {
        Self {
            spec_id: v.spec_id.clone(),
            kind: v.kind.clone(),
            stage: v.stage.clone(),
            pilot_state: v.pilot_state.clone(),
            weapon_jammed: v.weapon_jammed,
            tutorial_safety: v.tutorial_safety,
            mass_kg: v.mass_kg,
            zones: v.zones.iter().map(ChassisZoneView::from).collect(),
            modules: v.modules.iter().map(ChassisModuleView::from).collect(),
            integrity: v.integrity,
            eject_ticks_remaining: v.eject_ticks_remaining,
            eject_ticks_total: v.eject_ticks_total,
            destroyed_zones: v.destroyed_zones.clone(),
            salvaged_module_ids: v.salvaged_module_ids.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ChassisZoneView {
    pub zone: String,
    pub external_integrity: f32,
    pub internal_integrity: f32,
    pub core_integrity: f32,
    pub wound_integrity: f32,
    pub destroyed: bool,
    pub zone_integrity: f32,
}

impl From<&cf_actor::ChassisZoneView> for ChassisZoneView {
    fn from(v: &cf_actor::ChassisZoneView) -> Self {
        Self {
            zone: v.zone.clone(),
            external_integrity: v.external_integrity,
            internal_integrity: v.internal_integrity,
            core_integrity: v.core_integrity,
            wound_integrity: v.wound_integrity,
            destroyed: v.destroyed,
            zone_integrity: v.zone_integrity,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ChassisModuleView {
    pub id: String,
    pub kind: String,
    pub state: String,
    pub bound_zone: String,
    pub integrity: f32,
    pub last_reason: String,
}

impl From<&cf_actor::ChassisModuleView> for ChassisModuleView {
    fn from(v: &cf_actor::ChassisModuleView) -> Self {
        Self {
            id: v.id.clone(),
            kind: v.kind.clone(),
            state: v.state.clone(),
            bound_zone: v.bound_zone.clone(),
            integrity: v.integrity,
            last_reason: v.last_reason.clone(),
        }
    }
}

fn default_stance() -> String {
    "idle".to_string()
}

/// M4A body silhouette projection (mirrors `cf_actor::BodySilhouette`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct BodySilhouetteView {
    pub head_hp_pct: f32,
    pub torso_hp_pct: f32,
    pub arm_left_hp_pct: f32,
    pub arm_right_hp_pct: f32,
    pub leg_left_hp_pct: f32,
    pub leg_right_hp_pct: f32,
    pub placeholder: bool,
}

impl Default for BodySilhouetteView {
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

/// M4A module strip projection (mirrors `cf_actor::ModuleStrip`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ModuleStripView {
    pub modules: Vec<ModuleStateView>,
    pub placeholder: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ModuleStateView {
    pub id: String,
    pub label: String,
    pub state: String,
    pub kind: String,
}

/// M4A HUD banner queue entry. Surfaced from chassis/status/mission events so
/// HUD + `cfctl observe` consumers see the same priority-ordered banner stack.
/// The text-only banner is the M4A floor; M4B layers comic-noir styling on
/// top without changing the queue contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct HudBannerView {
    /// Stable id (e.g., `armor_cracked_left`, `eject_now`, `hp_critical`,
    /// `ammo_out`, `mission_failed`, `mission_won`).
    pub id: String,
    /// Severity: `critical` (red-equivalent), `warning` (amber-equivalent),
    /// `info` (neutral). Severity is text + icon, never color-only — HUD MUST
    /// render the severity word + icon glyph alongside the label.
    pub severity: String,
    /// Player-facing banner text (English at M4A; Tier-A localization keys at
    /// the next milestone-enhancement pass).
    pub label: String,
    /// Tick the banner was raised at; used for FIFO drain priority + replay.
    pub raised_at_tick: u64,
    /// Tick the banner expires at (no expiration when `None`).
    pub expires_at_tick: Option<u64>,
    /// Stable accessibility id surfaced for `cfctl ui` (M4A surface; M4B+
    /// extends to comic-noir card nodes).
    pub accessibility_id: String,
}

/// M4A captions queue entry. When `Settings.captions == true`, audio-bound
/// events surface as caption rows. Until cf-audio lands at BP6+ the captions
/// queue carries event-derived placeholders so the contract is testable from
/// M4A onward (DR-012 ACC-A floor closure).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CaptionView {
    pub id: String,
    pub label: String,
    pub raised_at_tick: u64,
    pub accessibility_id: String,
}

/// M4A tool-validity projection. Mirrors the `terrain.tool_refused` reason
/// code together with the most-recent successful `terrain.terrain_carved`
/// tick. Consumed by the HUD `TOOL` line and AI agents that want a one-shot
/// "is this tile diggable right now?" answer without scanning the event log.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ToolValidityView {
    pub last_carve_tick: Option<u64>,
    pub last_refusal_tick: Option<u64>,
    pub last_refusal_reason: Option<String>,
    pub last_refusal_target: Option<String>,
    pub valid: bool,
}

/// M4A accessibility surface contract. `Settings` already lives on the wire
/// via `observe.settings`; the resolved view in the observe frame says how
/// the HUD has applied each flag at this tick (so consumers can verify a
/// 200%-scale render or high-contrast palette without a screenshot).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AccessibilityView {
    pub ui_scale_applied: f32,
    pub high_contrast_applied: bool,
    pub captions_visible: bool,
    pub reduced_motion_applied: bool,
    pub reduced_shake_applied: bool,
    pub reduced_flash_applied: bool,
    /// M4A: ACC-A-05 hold-to-press flag mirror.
    pub hold_to_confirm_applied: bool,
    /// M4A: ACC-A-05 hold threshold in milliseconds.
    pub hold_threshold_ms: u32,
    /// M4A: ACC-A-05 remap toggle.
    pub key_remap_enabled: bool,
    /// M4A: ACC-A-05 active key bindings (action → KeyCode name). When
    /// `key_remap_enabled` is true, cf-app's keyboard layer consults this
    /// table; when false, defaults apply. Surface is observable so cfctl
    /// can verify the table round-trip without reading the raw Settings.
    pub key_bindings: std::collections::BTreeMap<String, String>,
    /// Stable accessibility ids of every focusable HUD node, in z-order. Used
    /// by `cfctl ui` and `cf-e2e --verify-focus` to prove the HUD has a
    /// keyboard-focusable surface beyond color cues.
    pub focusable_nodes: Vec<String>,
    /// M4A: id of the currently-focused HUD node (`Some(<id>)` after the
    /// player presses Tab / Shift+Tab / Arrow). `None` when focus is cleared
    /// (default at scenario load + after Escape). Drives the visible focus
    /// ring in cf-ui + the cf-e2e verify path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused_node: Option<String>,
    /// M4A: monotonic counter incremented every time focus advances. Lets
    /// cfctl + cf-e2e detect "focus has cycled at least once" without a
    /// time-based assertion.
    #[serde(default)]
    pub focus_cycle: u64,
}

impl Default for AccessibilityView {
    fn default() -> Self {
        let settings = Settings::default();
        Self {
            ui_scale_applied: settings.ui_scale,
            high_contrast_applied: settings.high_contrast,
            captions_visible: settings.captions,
            reduced_motion_applied: settings.reduced_motion,
            reduced_shake_applied: settings.reduced_shake,
            reduced_flash_applied: settings.reduced_flash,
            hold_to_confirm_applied: settings.hold_to_confirm,
            hold_threshold_ms: settings.hold_threshold_ms,
            key_remap_enabled: settings.key_remap_enabled,
            key_bindings: settings.key_bindings,
            focusable_nodes: Vec::new(),
            focused_node: None,
            focus_cycle: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ObserveSettings {
    pub schema_version: u32,
    pub settings: Settings,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AccessibilityFlagsView {
    pub schema_version: u32,
    pub ui_scale: f32,
    pub high_contrast: bool,
    pub captions: bool,
    pub reduced_motion: bool,
    pub reduced_shake: bool,
    pub reduced_flash: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accessibility_view_default_matches_settings_default() {
        let view = AccessibilityView::default();
        let settings = Settings::default();
        assert!((view.ui_scale_applied - settings.ui_scale).abs() < f32::EPSILON);
        assert_eq!(view.high_contrast_applied, settings.high_contrast);
        assert_eq!(view.captions_visible, settings.captions);
        assert_eq!(view.hold_threshold_ms, settings.hold_threshold_ms);
        assert_eq!(view.key_remap_enabled, settings.key_remap_enabled);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ControlEnvelopeStatus {
    Accepted,
    Rejected,
    Queued,
}
