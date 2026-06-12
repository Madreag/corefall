//! cf-ui — comic-noir HUD presentation surface.
//!
//! - Status / banner / caption strip with focus ring and high-contrast palette.
//! - Per-widget submodules (silhouette, module strip, captions, etc.) own their
//!   own Bevy resources; cf-app's bridge mirrors them per frame.
//! - All HUD nodes carry stable accessibility ids so AI agents see the same
//!   surface as a sighted player.

#![deny(unsafe_code)]
#![allow(
    clippy::module_name_repetitions,
    clippy::type_complexity,
    clippy::needless_pass_by_value,
    clippy::too_many_arguments,
    clippy::field_reassign_with_default,
    clippy::needless_doctest_main,
    clippy::derivable_impls
)]

pub mod ai_debug_label;
pub mod enemy_hp;
pub mod last_event_ticker;
pub mod material_legend;
pub mod mission_resolved_modal;
pub mod mission_timer;
pub mod objective_banner;

pub mod action_prompt;
pub mod branching_banner;
pub mod compass;
pub mod cover_indicator;
pub mod cover_pip;
pub mod damage_direction;
pub mod fortification_hud;
pub mod grenade_arc;
pub mod hotbar;
pub mod inventory_grid;
pub mod lean_pip;
pub mod minimap;
pub mod phase_strip;
pub mod scope_reticle;
pub mod settings_menu;
pub mod squad_strip;
pub mod stamina_bar;
pub mod stealth_meter;
pub mod weapon_swap_overlay;

pub mod reactor_hp_bar;
pub mod reactor_pressure_line;
pub mod timer_warnings;

pub mod banners;
pub mod captions;
pub mod chatter_ticker;
pub mod contrast;
pub mod event_ticker;
pub mod focus_ring;
pub mod module_strip;
pub mod priority_indicator;
pub mod silhouette;
pub mod surgery_panel;
pub mod triage_panel;
pub mod triage_window;

pub mod animation;
pub mod comic_overlay;
pub mod slideshow;

pub mod briefing_card;
pub mod caption_ribbon;
pub mod codex_cinematics_tab;

pub mod context_wheel;
pub mod tactical_overlay;
pub mod tile_inspect_overlay;

pub use tile_inspect_overlay::{InspectReactionRow, TileInspectOverlayState};

pub mod affliction_strip;
pub mod anomaly_indicator;
pub mod artifact_panel;
pub mod disease_dashboard;
pub mod oxygen_meter;

pub use affliction_strip::{
    AfflictionStripEntry, AfflictionStripState, AFFLICTION_STRIP_MAX_VISIBLE,
};
pub use anomaly_indicator::{AnomalyIndicatorMarker, AnomalyIndicatorState};
pub use artifact_panel::{ArtifactPanelEntry, ArtifactPanelState};
pub use disease_dashboard::{DiseaseDashboardEntry, DiseaseDashboardState};
pub use oxygen_meter::{OxygenBand, OxygenMeterState};

pub mod jetpack_fuel_meter;
pub mod mass_indicator;
pub mod quick_action_bar;
pub mod quick_action_radial;
pub mod walk_strip;

pub mod wound_strip;

pub mod veteran_dossier;

mod hud_lines;
mod hud_model;
mod palette;
mod status_strip;

pub use hud_lines::{
    ai_debug_label, banner_line, breach_line, enemy_line, mission_line, mission_timer_color, module_line,
    objective_line, rifle_status_line, show_replay_cta_event_id, silhouette_line, stability_line,
    stability_line_with_knockdown, stance_line, tool_line,
};
pub use hud_model::{
    HudBanner, HudBodySilhouette, HudBreach, HudCaption, HudEnemy, HudMission, HudModule, HudModuleStrip, HudRifle,
    HudSettings, HudState, HudToolValidity,
};
pub use status_strip::{
    AmmoStripText, BannerFocusWrapper, BannerStripRoot, BannerStripText, BreachStripText, CapturedStripText,
    CaptionStripRoot, CaptionStripText, EnemyStripText, HudAccessibilityId, ItemStripText, LastEventStripText,
    MissionStripText, ModuleStripText, ObjectiveStripText, ReticleStripText, SilhouetteStripText, StabilityStripText,
    StanceStripText, StatusStripPlugin, StatusStripRoot, StatusStripText, ToolStripText,
};

pub use mission_resolved_modal::{
    render_comic_death_recap, render_death_recap_with_mode, render_recap_text as render_death_recap_text,
    DeathRecapViewMode, RecapEvent, COMIC_DEATH_RECAP_PANELS, MAX_RECAP_LINES,
};
pub use reactor_hp_bar::{ArmorPipView, IntegrityBand, ReactorHpBarState};
pub use reactor_pressure_line::{PressureTint, ReactorPressureLineState};
pub use timer_warnings::{TimerColor, TimerSeverity, TimerWarning, TimerWarningsState, WARNING_THRESHOLDS};

pub use action_prompt::ActionPromptState;
pub use banners::{banner_text_line, BannerSeverity, BannerStackState, BANNER_STACK_MAX_VISIBLE};
pub use branching_banner::{BranchOption, BranchingBannerState};
pub use captions::{
    direction_string as caption_direction_string, spatial_caption_line, CaptionVerbosity, CaptionsState,
    CAPTION_AHEAD_BEHIND_CONE_RAD, CAPTION_HERE_RADIUS_M, CAPTION_QUEUE_MAX_VISIBLE,
};
pub use chatter_ticker::{
    ChatterLine, ChatterTickerState, CHATTER_TICKER_DEFAULT_DWELL_TICKS, CHATTER_TICKER_MAX_LINES,
};
pub use compass::{CompassBearing, CompassState, CARDINALS};
pub use contrast::{banner_bg_color, strip_bg_color, text_color, ContrastModeUi};
pub use cover_indicator::{
    chevron_for_ground_standing, chevron_sequence_for_walk, spec_walk_chevron_sequence,
    ChevronState, CoverIndicatorState, WalkGround, WalkPath, CHEVRON_PALETTE,
};
pub use fortification_hud::{
    AmmoBoxHudState, FortificationHpBarState, FortificationHudPlugin,
    MinefieldWarningBannerState, SpotlightPreviewState,
};
pub use cover_pip::{CoverLevel, CoverPipState};
pub use damage_direction::{DamageDirectionMarker, DamageDirectionState, DEFAULT_FADE_MS};
pub use event_ticker::{EventTickerEntry, EventTickerState, EVENT_TICKER_DEFAULT_DWELL_TICKS};
pub use focus_ring::{advance_focus_index, focus_ring_clear, focus_ring_color, FocusDirectionUi, FOCUSABLE_NODES};
pub use grenade_arc::{ArcSample, GrenadeArcState};
pub use hotbar::{HotbarSlot, HotbarState, HOTBAR_SLOTS};
pub use lean_pip::{LeanPipState, LEAN_MAX_DEGREES, LEAN_MIN_DEGREES};
pub use material_legend::{legend_entries, MaterialLegendEntry, MaterialLegendState};
pub use minimap::{MinimapMarker, MinimapState, MINIMAP_SIZE_PX};
pub use module_strip::{ModuleState, ModuleStripEntry, ModuleStripState};
pub use phase_strip::{MissionPhase, PhaseStripState};
pub use priority_indicator::{PriorityIcon, PriorityIndicatorEntry, PriorityIndicatorState};
pub use scope_reticle::ScopeReticleState;
pub use settings_menu::{SettingsMenuState, SettingsTab};
pub use silhouette::{BodySilhouetteState, SilhouetteBand};
pub use squad_strip::{squad_row_line, SquadAutonomyMode, SquadStripMember, SquadStripState, SQUAD_STRIP_MAX_MEMBERS};
pub use stamina_bar::{StaminaBarState, StaminaColor, STAMINA_CRITICAL_THRESHOLD, STAMINA_HIGH_THRESHOLD};
pub use stealth_meter::{StealthMeterState, SPOTTED_THRESHOLD};
pub use triage_window::{TriageAffliction, TriageVerdict, TriageWindowState};
pub use weapon_swap_overlay::{WeaponSwapOverlayState, SWAP_TRANSITION_MS};
pub use animation::{
    ease_in_out, ease_out_cubic, panel_skew_radians, panel_slide_offset, tick_animations, AnimationHook,
    AnimationPlugin, AnimationPulse, AnimationState,
};
pub use comic_overlay::{
    onomatopoeia_for, ComicOverlayMode, ComicOverlayPlugin, ComicOverlayState, ComicSurface, ONOMATOPOEIA_VOCABULARY,
};
pub use slideshow::{
    intro_slides, slideshow_duration_ms, subtitle_alpha, SlideshowPhase, SlideshowPlugin, SlideshowSlide,
    SlideshowSlot, SlideshowState, INTRO_NARRATIVE, SUBTITLE_FADE_IN_MS, SUBTITLE_FADE_OUT_MS,
};
