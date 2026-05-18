//! Accessibility/settings surface.
//!
//! M0 locked the original 6 flags (`ui_scale`, `high_contrast`, `captions`,
//! `reduced_motion`, `reduced_shake`, `reduced_flash`) per DR-012; M4A added
//! `hold_to_confirm` + `hold_threshold_ms` + `key_remap_enabled` so the
//! ACC-A-05 "remap and holds" surface contract is testable end-to-end.
//! Flags are observable via `cfctl observe --settings --once` and recorded
//! in `run_manifest.json.settings`. `act.settings.set` round-trips them
//! live; cf-app + cf-ui mirror them every frame.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// **M8**: slow-motion accessibility (`game_speed_assist`). Spec §
/// Camera + game feel — Off / Slowdown75 / Slowdown25 / FullPause.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum GameSpeedAssist {
    /// Sim runs at native rate.
    #[default]
    Off,
    /// Sim runs at 75% speed (cosmetic; replay-deterministic).
    Slowdown75,
    /// Sim runs at 25% speed.
    Slowdown25,
    /// Sim runs at 0% (menu only).
    FullPause,
}

impl GameSpeedAssist {
    /// Canonical snake_case identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            GameSpeedAssist::Off => "off",
            GameSpeedAssist::Slowdown75 => "slowdown_75",
            GameSpeedAssist::Slowdown25 => "slowdown_25",
            GameSpeedAssist::FullPause => "full_pause",
        }
    }

    /// Sim-speed percentage (0..=100) the per-tick scheduler honors. Off=100
    /// (no slowdown), Slowdown75=75 (3 of every 4 ticks advance), Slowdown25=25
    /// (1 of every 4 ticks advance), FullPause=0 (sim halts; settings UI +
    /// cfctl still respond). Composed via [`u8::min`] with the pie menu's
    /// `slowdown_factor_pct` so whichever surface is more restrictive wins.
    pub fn speed_pct(self) -> u8 {
        match self {
            GameSpeedAssist::Off => 100,
            GameSpeedAssist::Slowdown75 => 75,
            GameSpeedAssist::Slowdown25 => 25,
            GameSpeedAssist::FullPause => 0,
        }
    }

    /// Parse from the cfctl wire form.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<GameSpeedAssist> {
        Some(match value {
            "off" => GameSpeedAssist::Off,
            "slowdown_75" => GameSpeedAssist::Slowdown75,
            "slowdown_25" => GameSpeedAssist::Slowdown25,
            "full_pause" => GameSpeedAssist::FullPause,
            _ => return None,
        })
    }
}

/// **M8**: color blind / contrast palette mode (`color_cue_mode`). Spec §
/// Accessibility extras — Default / ColorblindSafe / Protanopia /
/// Deuteranopia / Tritanopia / MonochromeTest.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum ColorCueMode {
    /// Default game palette.
    #[default]
    Default,
    /// Colorblind-safe palette (yellow + blue replacing red + green).
    ColorblindSafe,
    /// Protanopia (red-blind) palette.
    Protanopia,
    /// Deuteranopia (green-blind) palette.
    Deuteranopia,
    /// Tritanopia (blue-blind) palette.
    Tritanopia,
    /// Monochrome test palette (greyscale).
    MonochromeTest,
}

impl ColorCueMode {
    /// Canonical snake_case identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            ColorCueMode::Default => "default",
            ColorCueMode::ColorblindSafe => "colorblind_safe",
            ColorCueMode::Protanopia => "protanopia",
            ColorCueMode::Deuteranopia => "deuteranopia",
            ColorCueMode::Tritanopia => "tritanopia",
            ColorCueMode::MonochromeTest => "monochrome_test",
        }
    }

    /// Parse from the cfctl wire form.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<ColorCueMode> {
        Some(match value {
            "default" => ColorCueMode::Default,
            "colorblind_safe" => ColorCueMode::ColorblindSafe,
            "protanopia" => ColorCueMode::Protanopia,
            "deuteranopia" => ColorCueMode::Deuteranopia,
            "tritanopia" => ColorCueMode::Tritanopia,
            "monochrome_test" => ColorCueMode::MonochromeTest,
            _ => return None,
        })
    }
}

/// **M8**: aim assist mode (`aim_assist`). Spec § Accessibility extras —
/// off / steady_aim / auto_aim_with_damage_penalty.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum AimAssist {
    /// No aim assist.
    #[default]
    Off,
    /// Reduces reticle wobble while aiming.
    SteadyAim,
    /// Snaps reticle slightly toward target; -15% damage.
    AutoAimWithDamagePenalty,
}

impl AimAssist {
    /// Canonical snake_case identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            AimAssist::Off => "off",
            AimAssist::SteadyAim => "steady_aim",
            AimAssist::AutoAimWithDamagePenalty => "auto_aim_with_damage_penalty",
        }
    }

    /// Parse from the cfctl wire form.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<AimAssist> {
        Some(match value {
            "off" => AimAssist::Off,
            "steady_aim" => AimAssist::SteadyAim,
            "auto_aim_with_damage_penalty" => AimAssist::AutoAimWithDamagePenalty,
            _ => return None,
        })
    }
}

/// **M8 / M11**: HUD density preset (`ui_density`). Spec § Accessibility tab —
/// Compact / Comfortable / Spacious. M11 added `Comfortable` as the canonical
/// default-density name (was `Normal` under M8 — accepted as an alias).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum UiDensity {
    /// Compact — denser HUD layout.
    Compact,
    /// Comfortable — default density (M11 canonical name; alias `normal`).
    #[default]
    Comfortable,
    /// Spacious — looser HUD spacing.
    Spacious,
}

impl UiDensity {
    /// Canonical snake_case identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            UiDensity::Compact => "compact",
            UiDensity::Comfortable => "comfortable",
            UiDensity::Spacious => "spacious",
        }
    }

    /// Parse from the cfctl wire form. Accepts `normal` as an alias for
    /// `comfortable` (M8 → M11 rename compatibility).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<UiDensity> {
        Some(match value {
            "compact" => UiDensity::Compact,
            "comfortable" | "normal" => UiDensity::Comfortable,
            "spacious" => UiDensity::Spacious,
            _ => return None,
        })
    }
}

/// **M11**: contrast palette mode for the ACC-A floor. Replaces the legacy
/// `high_contrast: bool` with a tri-state enum (Standard / HighContrastDark /
/// HighContrastLight) per spec § Settings tree.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum ContrastMode {
    /// Standard palette (default).
    #[default]
    Standard,
    /// High-contrast dark — pure white text on solid black.
    HighContrastDark,
    /// High-contrast light — pure black text on solid white.
    HighContrastLight,
}

impl ContrastMode {
    pub fn as_str(self) -> &'static str {
        match self {
            ContrastMode::Standard => "standard",
            ContrastMode::HighContrastDark => "high_contrast_dark",
            ContrastMode::HighContrastLight => "high_contrast_light",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<ContrastMode> {
        Some(match value {
            "standard" => ContrastMode::Standard,
            "high_contrast_dark" => ContrastMode::HighContrastDark,
            "high_contrast_light" => ContrastMode::HighContrastLight,
            _ => return None,
        })
    }
}

/// **M11**: captions verbosity mode. Filters which event categories surface
/// as captions in the HUD strip.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum CaptionMode {
    /// Captions disabled (no caption strip rendered).
    #[default]
    Off,
    /// Only critical-severity events surface.
    CriticalOnly,
    /// Critical + warning (default when captions are on).
    Standard,
    /// Critical + warning + info (verbose).
    Expanded,
}

impl CaptionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            CaptionMode::Off => "off",
            CaptionMode::CriticalOnly => "critical_only",
            CaptionMode::Standard => "standard",
            CaptionMode::Expanded => "expanded",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<CaptionMode> {
        Some(match value {
            "off" => CaptionMode::Off,
            "critical_only" => CaptionMode::CriticalOnly,
            "standard" => CaptionMode::Standard,
            "expanded" => CaptionMode::Expanded,
            _ => return None,
        })
    }
}

/// **M11**: primary input profile (drives default HUD focus path).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum InputProfile {
    /// Keyboard + mouse (default).
    #[default]
    KeyboardMouse,
    /// Controller (XInput / SDL gamepad).
    Controller,
    /// Keyboard only — no mouse / no controller.
    KeyboardOnly,
    /// Custom — player has rebound mixed inputs.
    Custom,
}

impl InputProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            InputProfile::KeyboardMouse => "keyboard_mouse",
            InputProfile::Controller => "controller",
            InputProfile::KeyboardOnly => "keyboard_only",
            InputProfile::Custom => "custom",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<InputProfile> {
        Some(match value {
            "keyboard_mouse" => InputProfile::KeyboardMouse,
            "controller" => InputProfile::Controller,
            "keyboard_only" => InputProfile::KeyboardOnly,
            "custom" => InputProfile::Custom,
            _ => return None,
        })
    }
}

/// **M11**: hold-behavior variant for the ACC-A floor (replaces tap-only).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum HoldBehavior {
    /// Press-and-hold semantics (default; matches `hold_to_confirm=true`).
    #[default]
    Hold,
    /// Toggle on/off — press once to enable, again to disable.
    Toggle,
    /// Press-to-cycle — each press advances through a state ring.
    PressToCycle,
}

impl HoldBehavior {
    pub fn as_str(self) -> &'static str {
        match self {
            HoldBehavior::Hold => "hold",
            HoldBehavior::Toggle => "toggle",
            HoldBehavior::PressToCycle => "press_to_cycle",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<HoldBehavior> {
        Some(match value {
            "hold" => HoldBehavior::Hold,
            "toggle" => HoldBehavior::Toggle,
            "press_to_cycle" => HoldBehavior::PressToCycle,
            _ => return None,
        })
    }
}

/// **M11**: camera-motion granularity (paired with `reduced_motion` for
/// finer follow-camera control).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum CameraMotion {
    /// Reduced — suppress follow-camera + recoil-camera animation.
    Reduced,
    /// Standard — full camera motion (default).
    #[default]
    Standard,
}

impl CameraMotion {
    pub fn as_str(self) -> &'static str {
        match self {
            CameraMotion::Reduced => "reduced",
            CameraMotion::Standard => "standard",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<CameraMotion> {
        Some(match value {
            "reduced" => CameraMotion::Reduced,
            "standard" => CameraMotion::Standard,
            _ => return None,
        })
    }
}

/// **M11**: objective-help verbosity (tutorial / mission text density).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveHelp {
    Minimal,
    #[default]
    Standard,
    Verbose,
}

impl ObjectiveHelp {
    pub fn as_str(self) -> &'static str {
        match self {
            ObjectiveHelp::Minimal => "minimal",
            ObjectiveHelp::Standard => "standard",
            ObjectiveHelp::Verbose => "verbose",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<ObjectiveHelp> {
        Some(match value {
            "minimal" => ObjectiveHelp::Minimal,
            "standard" => ObjectiveHelp::Standard,
            "verbose" => ObjectiveHelp::Verbose,
            _ => return None,
        })
    }
}

/// **M11**: debug-explainer verbosity (for the death recap + AI debug overlays).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum DebugExplainerLevel {
    /// Player-friendly plain-language explanations (default).
    #[default]
    Player,
    /// Designer-level explanations with tuning numbers.
    Designer,
    /// Raw engine event payloads (verbose).
    Raw,
}

impl DebugExplainerLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            DebugExplainerLevel::Player => "player",
            DebugExplainerLevel::Designer => "designer",
            DebugExplainerLevel::Raw => "raw",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<DebugExplainerLevel> {
        Some(match value {
            "player" => DebugExplainerLevel::Player,
            "designer" => DebugExplainerLevel::Designer,
            "raw" => DebugExplainerLevel::Raw,
            _ => return None,
        })
    }
}

/// **M11**: ACC-A floor caption category vocabulary. Caption_categories may
/// be any subset of this list. Default is `[combat, ai, mission, accessibility]`.
pub const SUPPORTED_CAPTION_CATEGORIES: &[&str] = &["combat", "ai", "terrain", "mission", "system", "accessibility"];

/// **M11**: ACC-A floor key-remap groups. Restricts which UI subsystems
/// surface their rebindable actions in the remap editor. Default is
/// `[Gameplay]`.
pub const SUPPORTED_REMAP_GROUPS: &[&str] = &["gameplay", "ui", "replay", "workbench", "accessibility"];

/// **M11**: default caption-category subset (4 of 6 categories on).
pub fn default_caption_categories() -> BTreeSet<String> {
    let mut s = BTreeSet::new();
    s.insert("combat".to_string());
    s.insert("ai".to_string());
    s.insert("mission".to_string());
    s.insert("accessibility".to_string());
    s
}

/// **M11**: default remap-group subset (gameplay-only on by default).
pub fn default_remap_groups() -> BTreeSet<String> {
    let mut s = BTreeSet::new();
    s.insert("gameplay".to_string());
    s
}

/// Default M8 mini-map zoom (1.0 = base zoom).
pub fn default_mini_map_zoom() -> f32 {
    1.0
}

/// Default M8 scope FOV in degrees per spec § Camera + game feel
/// "Scope zoom (sniper ADS — 30° FOV)". Mirrors cf_camera::SCOPE_FOV_DEGREES.
pub fn default_scope_zoom_fov() -> f32 {
    30.0
}

/// Default M8 language code per spec § Localization (en baseline).
pub fn default_language() -> String {
    "en".to_string()
}

/// Default text-scale (mirrors `ui_scale`).
pub fn default_text_scale() -> f32 {
    1.0
}

/// Default debug overlay set (empty until the player toggles individual
/// overlays via `act.debug.toggle_overlay`).
pub fn default_debug_overlays() -> BTreeSet<String> {
    BTreeSet::new()
}

pub const SUPPORTED_KEY_BINDING_ACTIONS: &[&str] = &[
    "jump",
    "fire",
    "fire_alt",
    "reload",
    "dig",
    "reset",
    "select_slot_0",
    "select_slot_1",
    "select_slot_2",
    "select_slot_3",
    "move_left",
    "move_right",
    "move_up",
    "move_down",
    "aim_left",
    "aim_right",
    "aim_up",
    "aim_down",
    // M11: sharp_aim (ADS) added to the discrete action set so the remap
    // surface covers ACC-A's full 18+ action floor.
    "sharp_aim",
    // **M4B § "F5 / F9 hotkeys are reserved by cf-shell::keybinds"** —
    // register the save subsystem actions in the remap surface so cf-app
    // honors player overrides via `Settings.key_bindings`.
    "save.quicksave",
    "save.quickload",
];

pub const SUPPORTED_KEY_CODE_NAMES: &[&str] = &[
    "Space",
    "Enter",
    "Tab",
    "Escape",
    "Backspace",
    "ArrowUp",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "ShiftLeft",
    "ShiftRight",
    "ControlLeft",
    "ControlRight",
    "KeyA",
    "KeyB",
    "KeyC",
    "KeyD",
    "KeyE",
    "KeyF",
    "KeyG",
    "KeyH",
    "KeyI",
    "KeyJ",
    "KeyK",
    "KeyL",
    "KeyM",
    "KeyN",
    "KeyO",
    "KeyP",
    "KeyQ",
    "KeyR",
    "KeyS",
    "KeyT",
    "KeyU",
    "KeyV",
    "KeyW",
    "KeyX",
    "KeyY",
    "KeyZ",
    "Digit0",
    "Digit1",
    "Digit2",
    "Digit3",
    "Digit4",
    "Digit5",
    "Digit6",
    "Digit7",
    "Digit8",
    "Digit9",
    "Numpad0",
    "Numpad1",
    "Numpad2",
    "Numpad3",
    "Numpad4",
    "Numpad5",
    "Numpad6",
    "Numpad7",
    "Numpad8",
    "Numpad9",
    // **M4B § "F5 / F9 hotkeys are reserved by cf-shell::keybinds"** —
    // the reserved save subsystem function keys. The full function-key
    // row is registered so the player can remap quicksave/quickload onto
    // any F-key, and so future shell hotkeys (e.g. F1 help, F12 photo
    // mode) have their key names registered too.
    "F1",
    "F2",
    "F3",
    "F4",
    "F5",
    "F6",
    "F7",
    "F8",
    "F9",
    "F10",
    "F11",
    "F12",
];

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Settings {
    pub ui_scale: f32,
    pub high_contrast: bool,
    pub captions: bool,
    pub reduced_motion: bool,
    pub reduced_shake: bool,
    pub reduced_flash: bool,
    /// M4A: hold-to-press alternative for tap-to-press actions (ACC-A-05).
    /// When `true`, edge-triggered actions (jump / fire / reload / dig /
    /// reset / select_slot_*) require holding the key/button for
    /// `hold_threshold_ms` before firing instead of triggering on the press
    /// edge. Default: off (tap semantics). cf-app's `ingest_player_input`
    /// honors this through the `HoldTracker` resource; cfctl-driven
    /// dispatches bypass the hold gate (control-plane is already explicit
    /// press/release semantics).
    #[serde(default)]
    pub hold_to_confirm: bool,
    /// M4A: hold threshold in milliseconds. Default 250 ms; clamped to
    /// `[50..2000]` by `apply_settings_patch`.
    #[serde(default = "default_hold_threshold_ms")]
    pub hold_threshold_ms: u32,
    /// M4A: when `true`, cf-app's keyboard layer reads its action bindings
    /// from `key_bindings` instead of the built-in defaults. When `false`,
    /// the built-in defaults apply (Space=jump, Enter|J=fire, R=reload,
    /// G=dig, L=reset, 1..4=select_slot_0..3). The remap UI editor lands
    /// at M8; M4A ships the cfctl-settable surface so the contract is
    /// behavior-testable today.
    #[serde(default)]
    pub key_remap_enabled: bool,
    /// M4A: per-action key binding overrides. Action names are limited to
    /// [`SUPPORTED_KEY_BINDING_ACTIONS`]; KeyCode names are limited to
    /// [`SUPPORTED_KEY_CODE_NAMES`]. Empty by default; `act.settings.set`
    /// replaces the table only after validation so unsupported remaps reject
    /// at the control boundary instead of silently falling back in cf-app.
    #[serde(default)]
    pub key_bindings: BTreeMap<String, String>,
    /// **M1 / DR-055**: accessibility floor — camera shake magnitude is
    /// multiplied by `(1.0 - reduce_camera_shake_pct)`. 0.0 = full shake,
    /// 1.0 = no shake. Clamped to `[0, 1]` by `apply_settings_patch`.
    #[serde(default)]
    pub reduce_camera_shake_pct: f32,
    /// **M1**: tick-rate observable. Mirrors the engine's
    /// `tick_rate_hz` so cfctl `observe.settings` round-trips a single
    /// authoritative value. Defaults to 60.
    #[serde(default = "default_tick_rate_hz")]
    pub tick_rate_hz: u32,
    /// **M1 Gap F1**: ground acceleration (units / s²). Default 1500.0
    /// mirrors `cf-actor::sim::ActorTuning::default().ground_acceleration`.
    #[serde(default = "default_accel")]
    pub accel: f32,
    /// **M1 Gap F1**: ground friction (units / s²). Default 1200.0 mirrors
    /// `cf-actor::sim::ActorTuning::default().ground_friction`.
    #[serde(default = "default_friction")]
    pub friction: f32,
    /// **M1 Gap F1**: gravity acceleration applied to the actor in
    /// `cf-physics` (units / s²; negative pulls toward the floor). Default
    /// `-980.0`.
    #[serde(default = "default_gravity")]
    pub gravity: f32,
    /// **M1 Gap F1**: jump impulse (units / s). Default `420.0` mirrors
    /// `cf-actor::sim::ActorTuning::default().jump_impulse`.
    #[serde(default = "default_jump_force")]
    pub jump_force: f32,
    /// **M1 Gap F1**: per-tick decay of `recoil_accumulator` toward zero
    /// (CCCP `HDFirearm.cpp:891`). Default `0.05`.
    #[serde(default = "default_recoil_decay_per_tick")]
    pub recoil_decay_per_tick: f32,
    /// **M1 Gap F1**: ticks to fully build sharp aim from 0 -> 1.0 (CCCP
    /// `AHuman.cpp:1779`). Default `30` (~0.5s at 60Hz).
    #[serde(default = "default_sharp_aim_build_ticks")]
    pub sharp_aim_build_ticks: u32,
    /// **M1 Gap F1**: horizontal-speed threshold (units / s) below which the
    /// actor counts as "slow enough" to keep building sharp aim. Default 1.5.
    #[serde(default = "default_walk_threshold")]
    pub walk_threshold: f32,
    /// **M1.5 G6**: active AI difficulty preset id. Engine applies the
    /// matching `cf-ai::DifficultyPreset` to every `ReactiveGuard` whenever
    /// this changes.
    #[serde(default = "default_ai_difficulty")]
    pub ai_difficulty: String,
    /// **M1.5**: when true, cf-ui renders a floating intent label above
    /// every reactive guard ("ALERT: heard_shot", "ENGAGED", "RELOADING").
    /// Toggled via `--ai-debug` CLI flag on cf-app and `act.settings.set
    /// { ai_debug: true }` through cfctl.
    #[serde(default)]
    pub ai_debug: bool,

    // === M8 accessibility / camera / debug / locale extensions ===
    /// **M8**: slow-motion accessibility (Off / Slowdown75 / Slowdown25 /
    /// FullPause). Cosmetic per M4; replay-deterministic.
    #[serde(default)]
    pub game_speed_assist: GameSpeedAssist,
    /// **M8**: color blind / contrast palette mode.
    #[serde(default)]
    pub color_cue_mode: ColorCueMode,
    /// **M8**: aim assist mode (off / steady / auto).
    #[serde(default)]
    pub aim_assist: AimAssist,
    /// **M8**: damage numbers cosmetic (floating "+15" text on hit).
    #[serde(default)]
    pub damage_numbers: bool,
    /// **M8**: killcam toggle (on by default per spec § Killcam).
    #[serde(default = "default_true")]
    pub killcam_enabled: bool,
    /// **M8**: hit-stop pulse on impactful hits.
    #[serde(default = "default_true")]
    pub hit_stop_enabled: bool,
    /// **M8**: cinematic kill cam on boss final blow.
    #[serde(default = "default_true")]
    pub cinematic_kills: bool,
    /// **M8**: master mini-map enable toggle (Settings.no_minimap inverts).
    #[serde(default = "default_true")]
    pub mini_map_enabled: bool,
    /// **M8**: master compass enable toggle.
    #[serde(default = "default_true")]
    pub compass_enabled: bool,
    /// **M8**: damage direction indicator enable toggle.
    #[serde(default = "default_true")]
    pub damage_direction_enabled: bool,
    /// **M8**: mini-map zoom (clamped 0.25..=4.0).
    #[serde(default = "default_mini_map_zoom")]
    pub mini_map_zoom: f32,
    /// **M8**: scope ADS FOV in degrees (clamped 5..=90).
    #[serde(default = "default_scope_zoom_fov")]
    pub scope_zoom_fov: f32,
    /// **M8**: text scale (mirrors ui_scale; some HUD widgets honour this
    /// distinct field per spec § Settings menu Accessibility tab).
    #[serde(default = "default_text_scale")]
    pub text_scale: f32,
    /// **M8**: HUD density preset.
    #[serde(default)]
    pub ui_density: UiDensity,
    /// **M8**: active language code (en baseline; Tier-A 11 reserved for
    /// T-ACC-PLUS BP9+).
    #[serde(default = "default_language")]
    pub language: String,
    /// **M8**: speedrun mode (HUD shows persistent timer + segment
    /// markers; mission resolves immediately on objectives).
    #[serde(default)]
    pub speedrun_mode: bool,
    // === M8 difficulty modifiers (granular per scenario) ===
    /// **M8**: permadeath modifier.
    #[serde(default)]
    pub permadeath: bool,
    /// **M8**: no-respawn modifier.
    #[serde(default)]
    pub no_respawn: bool,
    /// **M8**: fog-of-war on (default true per spec § Difficulty modifiers).
    #[serde(default = "default_true")]
    pub fog_of_war_on: bool,
    /// **M8**: limited-ammo modifier.
    #[serde(default)]
    pub limited_ammo: bool,
    /// **M8**: time-limit modifier (mission has a hard timer).
    #[serde(default)]
    pub time_limit: bool,
    /// **M8**: hide the mini-map (overrides `mini_map_enabled` when true).
    #[serde(default)]
    pub no_minimap: bool,
    /// **M8**: hardcore mode (composite of multiple modifiers).
    #[serde(default)]
    pub hardcore_mode: bool,
    /// **M8**: friendly fire on.
    #[serde(default)]
    pub friendly_fire_on: bool,
    /// **M8**: master debug-overlay enable gate. In production builds the
    /// 7 cf-debug overlays only render when `debug_enabled = true`. Dev
    /// builds bypass the gate.
    #[serde(default)]
    pub debug_enabled: bool,
    /// **M8**: set of currently-enabled cf-debug overlays (snake_case ids
    /// per `cf_debug::DebugOverlay::as_str`). Mirrors the cf-debug
    /// `DebugOverlayState` so cfctl `observe.debug.overlays` round-trips.
    #[serde(default = "default_debug_overlays")]
    pub debug_overlays: BTreeSet<String>,

    // === M11 accessibility (DR-003 + DR-012 closure) extensions ===
    /// **M11**: contrast palette mode. Tri-state replacing the legacy
    /// `high_contrast: bool`. Standard / HighContrastDark / HighContrastLight.
    #[serde(default)]
    pub contrast_mode: ContrastMode,
    /// **M11**: captions verbosity mode (Off / CriticalOnly / Standard /
    /// Expanded). Filters which events surface as captions.
    #[serde(default)]
    pub caption_mode: CaptionMode,
    /// **M11**: caption background opacity (0.0..=1.0; default 0.8).
    #[serde(default = "default_caption_background_opacity")]
    pub caption_background_opacity: f32,
    /// **M11**: caption category subset (subset of
    /// [`SUPPORTED_CAPTION_CATEGORIES`]). Default 4 of 6.
    #[serde(default = "default_caption_categories")]
    pub caption_categories: BTreeSet<String>,
    /// **M11**: input profile (default keyboard+mouse).
    #[serde(default)]
    pub input_profile: InputProfile,
    /// **M11**: remap-action group subset (subset of
    /// [`SUPPORTED_REMAP_GROUPS`]). Default `[gameplay]`.
    #[serde(default = "default_remap_groups")]
    pub remap_groups: BTreeSet<String>,
    /// **M11**: hold-behavior variant (default Hold).
    #[serde(default)]
    pub hold_behavior: HoldBehavior,
    /// **M11**: screen-shake scale (0.0..=1.0; default 1.0 = full shake;
    /// 0.0 = no shake). Multiplicative on camera punch + recoil shake.
    /// Replaces the inverse-sense `reduce_camera_shake_pct`; the legacy
    /// field is preserved for back-compat (mirror updated whenever this
    /// changes via `apply_settings_patch`).
    #[serde(default = "default_screen_shake_scale")]
    pub screen_shake_scale: f32,
    /// **M11**: camera-motion granularity (Reduced / Standard).
    #[serde(default)]
    pub camera_motion: CameraMotion,
    /// **M11**: objective-help verbosity (Minimal / Standard / Verbose).
    #[serde(default)]
    pub objective_help: ObjectiveHelp,
    /// **M11**: debug-explainer level (Player / Designer / Raw).
    #[serde(default)]
    pub debug_explainer_level: DebugExplainerLevel,

    // === M12 cinematic story beats + optional comic overlay ===
    /// **M12**: comic-style overlay mode (`full | subtle | off`). Drives
    /// speech bubbles, onomatopoeia stamps, comic-panel boss intros, and
    /// the comic death-recap availability. Default is `Subtle` per spec
    /// § Comic-style framing — opt-in juice, not core identity.
    #[serde(default)]
    pub comic_style_overlay: ComicStyleOverlay,
    /// **M12**: when `true`, the death recap renders as a 4-panel comic-style
    /// cause chain. When `false` (default), the M10 replay viewer +
    /// cause-chain walker is used. Gated by `comic_style_overlay != Off`.
    #[serde(default)]
    pub comic_death_recap: bool,

    // === M12C in-engine cinematic surface ===
    /// **M12C**: active storyteller id matching `cf-cinematic::StorytellerId`
    /// (`cassandra_classic` | `phoebe_chillax` | `randy_random` |
    /// `ironman` | `sandbox`). Forward-compat with M25's director-driven
    /// storyteller selection. cf-shell + cfctl set this via
    /// `act.settings.set`; cf-cinematic reads it at every kernel engage
    /// per spec § "The cinematic player reads the active storyteller
    /// from M25 director state and applies its profile globally."
    #[serde(default = "default_storyteller")]
    pub storyteller: String,
    /// **M12C**: captions-enabled mirror for the cinematic caption
    /// ribbon. Defaults to `captions` (M11 mode) but exposed as a
    /// separate flag so cfctl tests can toggle the cinematic surface
    /// independently of the gameplay caption strip. Per spec § "the
    /// M12A `caption_visible` predicate gates the subtitle ribbon".
    #[serde(default = "default_true")]
    pub cinematic_captions_enabled: bool,
}

/// Default M12C storyteller (matches the cf-shell SettingsScaffold
/// "gameplay.storyteller" default).
pub fn default_storyteller() -> String {
    "cassandra_classic".to_string()
}

/// **M12**: comic-style overlay mode mirror — `full | subtle | off`.
/// Drives `cf-ui::comic_overlay::ComicOverlayMode` at runtime.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum ComicStyleOverlay {
    /// Speech bubbles for chatter + onomatopoeia on impacts + comic-panel
    /// boss intros + comic death recap available behind toggle.
    Full,
    /// Default — speech bubbles for storyteller events only; no
    /// onomatopoeia stamps; comic death recap available behind toggle.
    #[default]
    Subtle,
    /// Disabled — no comic framing renders anywhere; chatter is plain
    /// captions; death recap is M10 timeline only.
    Off,
}

impl ComicStyleOverlay {
    /// Canonical snake_case identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            ComicStyleOverlay::Full => "full",
            ComicStyleOverlay::Subtle => "subtle",
            ComicStyleOverlay::Off => "off",
        }
    }

    /// Parse from the snake_case wire form.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<ComicStyleOverlay> {
        Some(match value {
            "full" => ComicStyleOverlay::Full,
            "subtle" => ComicStyleOverlay::Subtle,
            "off" => ComicStyleOverlay::Off,
            _ => return None,
        })
    }
}

/// **M11**: default caption background opacity per spec.
pub fn default_caption_background_opacity() -> f32 {
    0.8
}

/// **M11**: default screen-shake scale (full shake = 1.0).
pub fn default_screen_shake_scale() -> f32 {
    1.0
}

fn default_true() -> bool {
    true
}

fn default_ai_difficulty() -> String {
    "tough_crowd".to_string()
}

fn default_tick_rate_hz() -> u32 {
    60
}

fn default_accel() -> f32 {
    1500.0
}

fn default_friction() -> f32 {
    1200.0
}

fn default_gravity() -> f32 {
    -980.0
}

fn default_jump_force() -> f32 {
    420.0
}

fn default_recoil_decay_per_tick() -> f32 {
    0.05
}

fn default_sharp_aim_build_ticks() -> u32 {
    30
}

fn default_walk_threshold() -> f32 {
    1.5
}

fn default_hold_threshold_ms() -> u32 {
    250
}

/// ACC-A UI scale floor. Values entering `Settings` through `act.settings.set`
/// are clamped to this bound so `observe.settings`, `observe.accessibility`,
/// and cf-ui render state all report the same applied scale.
pub const UI_SCALE_MIN: f32 = 0.5;
/// ACC-A UI scale ceiling. See [`UI_SCALE_MIN`].
pub const UI_SCALE_MAX: f32 = 4.0;

/// M4A: built-in default action → KeyCode bindings. Action names are stable
/// strings the cfctl + replay surface refers to; values are KeyCode variant
/// names (e.g. `Space`, `Enter`, `KeyJ`, `KeyR`). cf-app maps the names back
/// to `bevy::prelude::KeyCode` via cf-app's parser. Unknown names are rejected
/// by [`validate_key_bindings`] before they can enter live `Settings`.
///
/// Audit fix round-5 (2026-05-10): the remap surface now covers continuous
/// actions (move_left/move_right/move_up/move_down + aim_*) in addition to
/// discrete actions (jump/fire/reload/dig/reset/select_slot_N). `cf-app::
/// ingest_player_input` consults its `key_for_action` helper for movement
/// and aim every frame so left-handed users can swap WASD ↔ arrows or
/// rebind aim to numpad without code changes. Movement/aim are CONTINUOUS
/// (held-key → analog axis), so the remap honors `key.pressed(...)` per-
/// frame rather than the `just_pressed(...)` edge-trigger of discrete
/// actions.
pub fn default_key_bindings() -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    // Discrete actions (edge-triggered)
    m.insert("jump".into(), "Space".into());
    m.insert("fire".into(), "Enter".into());
    m.insert("fire_alt".into(), "KeyJ".into());
    m.insert("reload".into(), "KeyR".into());
    m.insert("dig".into(), "KeyG".into());
    m.insert("reset".into(), "KeyL".into());
    m.insert("select_slot_0".into(), "Digit1".into());
    m.insert("select_slot_1".into(), "Digit2".into());
    m.insert("select_slot_2".into(), "Digit3".into());
    m.insert("select_slot_3".into(), "Digit4".into());
    // Continuous actions (held-key → analog) — primary WASD bindings
    m.insert("move_left".into(), "KeyA".into());
    m.insert("move_right".into(), "KeyD".into());
    m.insert("move_up".into(), "KeyW".into());
    m.insert("move_down".into(), "KeyS".into());
    // Continuous actions — aim with arrow keys (left-hand-friendly default)
    m.insert("aim_left".into(), "ArrowLeft".into());
    m.insert("aim_right".into(), "ArrowRight".into());
    m.insert("aim_up".into(), "ArrowUp".into());
    m.insert("aim_down".into(), "ArrowDown".into());
    // M11: sharp_aim (ADS) — right-click typically, but we wire a keyboard
    // fallback so KeyboardOnly profile can still ADS.
    m.insert("sharp_aim".into(), "ShiftLeft".into());
    // **M4B § "F5 / F9 hotkeys are reserved by cf-shell::keybinds"** —
    // wire the spec's reserved defaults into the remap surface so the
    // settings UI shows them and the player can override them. cf-app's
    // ingest_quicksave_input reads these via key_for_action.
    m.insert("save.quicksave".into(), "F5".into());
    m.insert("save.quickload".into(), "F9".into());
    m
}

pub fn is_supported_key_binding_action(action: &str) -> bool {
    SUPPORTED_KEY_BINDING_ACTIONS.contains(&action)
}

pub fn is_supported_key_code_name(key: &str) -> bool {
    SUPPORTED_KEY_CODE_NAMES.contains(&key)
}

pub fn validate_key_bindings(bindings: &BTreeMap<String, String>) -> Result<(), String> {
    for (action, key) in bindings {
        if !is_supported_key_binding_action(action) {
            return Err(format!("key_binding_unknown_action:{action}"));
        }
        if !is_supported_key_code_name(key) {
            return Err(format!("key_binding_unknown_key:{action}={key}"));
        }
    }
    let mut effective = default_key_bindings();
    for (action, key) in bindings {
        effective.insert(action.clone(), key.clone());
    }
    let mut key_owner: BTreeMap<String, String> = BTreeMap::new();
    for (action, key) in effective {
        if let Some(first_action) = key_owner.insert(key.clone(), action.clone()) {
            return Err(format!("key_binding_duplicate_key:{key}={first_action},{action}"));
        }
    }
    Ok(())
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ui_scale: 1.0,
            high_contrast: false,
            captions: true,
            reduced_motion: false,
            reduced_shake: false,
            reduced_flash: false,
            hold_to_confirm: false,
            hold_threshold_ms: default_hold_threshold_ms(),
            key_remap_enabled: false,
            key_bindings: BTreeMap::new(),
            reduce_camera_shake_pct: 0.0,
            tick_rate_hz: default_tick_rate_hz(),
            accel: default_accel(),
            friction: default_friction(),
            gravity: default_gravity(),
            jump_force: default_jump_force(),
            recoil_decay_per_tick: default_recoil_decay_per_tick(),
            sharp_aim_build_ticks: default_sharp_aim_build_ticks(),
            walk_threshold: default_walk_threshold(),
            ai_difficulty: default_ai_difficulty(),
            ai_debug: false,
            game_speed_assist: GameSpeedAssist::Off,
            color_cue_mode: ColorCueMode::Default,
            aim_assist: AimAssist::Off,
            damage_numbers: false,
            killcam_enabled: true,
            hit_stop_enabled: true,
            cinematic_kills: true,
            mini_map_enabled: true,
            compass_enabled: true,
            damage_direction_enabled: true,
            mini_map_zoom: default_mini_map_zoom(),
            scope_zoom_fov: default_scope_zoom_fov(),
            text_scale: default_text_scale(),
            ui_density: UiDensity::Comfortable,
            language: default_language(),
            speedrun_mode: false,
            permadeath: false,
            no_respawn: false,
            fog_of_war_on: true,
            limited_ammo: false,
            time_limit: false,
            no_minimap: false,
            hardcore_mode: false,
            friendly_fire_on: false,
            debug_enabled: false,
            debug_overlays: default_debug_overlays(),
            // === M11 ACC-A floor defaults ===
            contrast_mode: ContrastMode::Standard,
            caption_mode: CaptionMode::Off,
            caption_background_opacity: default_caption_background_opacity(),
            caption_categories: default_caption_categories(),
            input_profile: InputProfile::KeyboardMouse,
            remap_groups: default_remap_groups(),
            hold_behavior: HoldBehavior::Hold,
            screen_shake_scale: default_screen_shake_scale(),
            camera_motion: CameraMotion::Standard,
            objective_help: ObjectiveHelp::Standard,
            debug_explainer_level: DebugExplainerLevel::Player,
            // === M12 cinematic story beats ===
            comic_style_overlay: ComicStyleOverlay::Subtle,
            comic_death_recap: false,
            // === M12C in-engine cinematic surface ===
            storyteller: default_storyteller(),
            cinematic_captions_enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_match_dr012_lock() {
        let s = Settings::default();
        assert!((s.ui_scale - 1.0).abs() < f32::EPSILON);
        assert!(!s.high_contrast);
        assert!(s.captions, "captions default-on per DR-012 lean");
        assert!(!s.reduced_motion);
        assert!(!s.reduced_shake);
        assert!(!s.reduced_flash);
        assert!(!s.hold_to_confirm);
        assert_eq!(s.hold_threshold_ms, 250);
        assert!(!s.key_remap_enabled);
    }

    #[test]
    fn settings_serialize_to_flat_kv() {
        let s = Settings::default();
        let v = serde_json::to_value(&s).unwrap();
        for k in [
            "ui_scale",
            "high_contrast",
            "captions",
            "reduced_motion",
            "reduced_shake",
            "reduced_flash",
            "hold_to_confirm",
            "hold_threshold_ms",
            "key_remap_enabled",
            "key_bindings",
        ] {
            assert!(v.get(k).is_some(), "missing key: {k}");
        }
    }

    #[test]
    fn default_key_bindings_cover_every_m4a_action() {
        let b = default_key_bindings();
        for action in SUPPORTED_KEY_BINDING_ACTIONS {
            assert!(b.contains_key(*action), "missing default binding for {action}");
        }
        assert_eq!(b.len(), SUPPORTED_KEY_BINDING_ACTIONS.len());
    }

    #[test]
    fn supported_key_names_cover_default_bindings_and_numpad() {
        let b = default_key_bindings();
        for (action, key) in &b {
            assert!(
                is_supported_key_code_name(key),
                "default binding {action}={key} must be accepted by live validation"
            );
        }
        assert!(is_supported_key_code_name("Numpad8"));
    }

    #[test]
    fn validate_key_bindings_rejects_unknown_action() {
        let mut b = BTreeMap::new();
        b.insert("frie".to_string(), "KeyF".to_string());
        assert_eq!(
            validate_key_bindings(&b).unwrap_err(),
            "key_binding_unknown_action:frie"
        );
    }

    #[test]
    fn validate_key_bindings_rejects_unknown_key() {
        let mut b = BTreeMap::new();
        b.insert("fire".to_string(), "BogusKey".to_string());
        assert_eq!(
            validate_key_bindings(&b).unwrap_err(),
            "key_binding_unknown_key:fire=BogusKey"
        );
    }

    #[test]
    fn validate_key_bindings_accepts_numpad_remap() {
        let mut b = BTreeMap::new();
        b.insert("aim_up".to_string(), "Numpad8".to_string());
        validate_key_bindings(&b).unwrap();
    }

    #[test]
    fn validate_key_bindings_rejects_collisions_with_default_bindings() {
        let mut b = BTreeMap::new();
        b.insert("fire".to_string(), "KeyA".to_string());
        assert_eq!(
            validate_key_bindings(&b).unwrap_err(),
            "key_binding_duplicate_key:KeyA=fire,move_left"
        );
    }

    #[test]
    fn validate_key_bindings_accepts_full_swap_without_collision() {
        let mut b = BTreeMap::new();
        b.insert("fire".to_string(), "KeyA".to_string());
        b.insert("move_left".to_string(), "Enter".to_string());
        validate_key_bindings(&b).unwrap();
    }

    #[test]
    fn game_speed_assist_speed_pct_matches_spec() {
        assert_eq!(GameSpeedAssist::Off.speed_pct(), 100);
        assert_eq!(GameSpeedAssist::Slowdown75.speed_pct(), 75);
        assert_eq!(GameSpeedAssist::Slowdown25.speed_pct(), 25);
        assert_eq!(GameSpeedAssist::FullPause.speed_pct(), 0);
    }

    #[test]
    fn comic_style_overlay_default_is_subtle() {
        let s = Settings::default();
        assert_eq!(s.comic_style_overlay, ComicStyleOverlay::Subtle);
        assert!(!s.comic_death_recap);
    }

    #[test]
    fn comic_style_overlay_round_trips_through_str() {
        for mode in [ComicStyleOverlay::Full, ComicStyleOverlay::Subtle, ComicStyleOverlay::Off] {
            assert_eq!(ComicStyleOverlay::from_str(mode.as_str()), Some(mode));
        }
        assert!(ComicStyleOverlay::from_str("bogus").is_none());
    }

    #[test]
    fn comic_style_overlay_serializes_as_snake_case() {
        let s = Settings::default();
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(
            v.get("comic_style_overlay").and_then(|x| x.as_str()),
            Some("subtle"),
        );
        assert_eq!(v.get("comic_death_recap").and_then(|x| x.as_bool()), Some(false));
    }
}
