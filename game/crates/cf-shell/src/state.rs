//! Shell state machine + transitions.
//!
//! `ShellScreen` enum captures every screen the player can be on outside
//! of in-mission HUD. `ShellState` is a Bevy `Resource` that holds the
//! current screen + any per-screen ephemeral state (which menu item is
//! highlighted, which settings tab is active, which save slot is selected,
//! etc).

use std::collections::BTreeMap;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Every shell screen the player can be on outside of in-mission HUD.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ShellScreen {
    /// Initial brand reveal + engine init bar; auto-advances after 3s
    /// or on any-key skip.
    #[default]
    Splash,
    /// Title screen with attract mode + 9 menu options.
    Title,
    /// Main menu post-Continue with career arc + quick actions.
    MainMenu,
    /// In-mission pause overlay.
    Pause,
    /// Save / load slot UI (10 named + 3 auto-save).
    SaveLoad,
    /// Settings tree with 6 tabs (Display / Audio / Controls /
    /// Accessibility / Gameplay / Language+Privacy).
    Settings(SettingsTab),
    /// Auto-generated credits scroll.
    Credits,
    /// Per-scenario loading screen with tip + screenshot.
    Loading,
    /// First-run experience wizard (6 steps).
    FreWizard(FreStep),
    /// In-mission HUD (cf-shell yields control to cf-app).
    InMission,
    /// Workshop browser (forwards to cf-mod UI).
    Workshop,
    /// **M12**: CCCP-style intro slideshow — 8 painted slides + subtitles
    /// + music + voice. Reached from Title → "New Game" (first run) or
    /// Main Menu → Story → "Replay Intro". Skippable.
    IntroSlideshow,
}

/// Settings tree tab identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SettingsTab {
    #[default]
    Display,
    Audio,
    Controls,
    Accessibility,
    Gameplay,
    LanguagePrivacy,
}

impl SettingsTab {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Display => "Display",
            Self::Audio => "Audio",
            Self::Controls => "Controls",
            Self::Accessibility => "Accessibility",
            Self::Gameplay => "Gameplay",
            Self::LanguagePrivacy => "Language+Privacy",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Display => Self::Audio,
            Self::Audio => Self::Controls,
            Self::Controls => Self::Accessibility,
            Self::Accessibility => Self::Gameplay,
            Self::Gameplay => Self::LanguagePrivacy,
            Self::LanguagePrivacy => Self::Display,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Self::Display => Self::LanguagePrivacy,
            Self::Audio => Self::Display,
            Self::Controls => Self::Audio,
            Self::Accessibility => Self::Controls,
            Self::Gameplay => Self::Accessibility,
            Self::LanguagePrivacy => Self::Gameplay,
        }
    }
}

/// First-run experience wizard step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum FreStep {
    #[default]
    Welcome,
    Profile,
    AccessibilityCalibration,
    ControllerCalibration,
    TutorialOffer,
    StarterWorldRecommendation,
}

impl FreStep {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Welcome => "Welcome",
            Self::Profile => "Profile",
            Self::AccessibilityCalibration => "Accessibility",
            Self::ControllerCalibration => "Controller",
            Self::TutorialOffer => "Tutorial",
            Self::StarterWorldRecommendation => "Starter World",
        }
    }

    pub fn next(&self) -> Option<Self> {
        match self {
            Self::Welcome => Some(Self::Profile),
            Self::Profile => Some(Self::AccessibilityCalibration),
            Self::AccessibilityCalibration => Some(Self::ControllerCalibration),
            Self::ControllerCalibration => Some(Self::TutorialOffer),
            Self::TutorialOffer => Some(Self::StarterWorldRecommendation),
            Self::StarterWorldRecommendation => None,
        }
    }

    pub fn prev(&self) -> Option<Self> {
        match self {
            Self::Welcome => None,
            Self::Profile => Some(Self::Welcome),
            Self::AccessibilityCalibration => Some(Self::Profile),
            Self::ControllerCalibration => Some(Self::AccessibilityCalibration),
            Self::TutorialOffer => Some(Self::ControllerCalibration),
            Self::StarterWorldRecommendation => Some(Self::TutorialOffer),
        }
    }

    pub fn step_index(&self) -> u32 {
        match self {
            Self::Welcome => 1,
            Self::Profile => 2,
            Self::AccessibilityCalibration => 3,
            Self::ControllerCalibration => 4,
            Self::TutorialOffer => 5,
            Self::StarterWorldRecommendation => 6,
        }
    }

    pub fn step_count() -> u32 {
        6
    }
}

/// Bevy `Resource` holding the current shell screen + per-screen ephemeral
/// state. Persisted across sessions for FRE-completed-flag + last-active-screen
/// hints; per-screen ephemerals are cleared on screen transitions.
#[derive(Debug, Resource)]
pub struct ShellState {
    pub current: ShellScreen,
    pub previous: ShellScreen,
    pub splash_elapsed_ms: u32,
    pub splash_skippable: bool,
    pub menu_focused_index: usize,
    pub settings_focused_row: usize,
    pub save_slot_focused: usize,
    pub fre_completed: bool,
    /// Tracks whether a save exists for the Continue option to be visible
    /// at the title screen.
    pub has_save: bool,
    /// Mission lifecycle flag — true when an in-mission scenario is loaded
    /// (governs whether pause/save/load are reachable).
    pub mission_active: bool,
    /// Show-Me-Why CTA visibility flag — true when the last mission lost.
    pub show_me_why_visible: bool,
    /// Last loading-tip index for stable rotation.
    pub last_tip_index: usize,
    /// **M12**: which intro-slideshow slot is currently playing (None when
    /// the slideshow is not on screen). Cleared when the slideshow ends
    /// or is skipped.
    pub intro_slideshow_slot: Option<crate::shell_api::IntroSlideshowSlot>,
}

impl Default for ShellState {
    fn default() -> Self {
        Self {
            current: ShellScreen::Splash,
            previous: ShellScreen::Splash,
            splash_elapsed_ms: 0,
            splash_skippable: true,
            menu_focused_index: 0,
            settings_focused_row: 0,
            save_slot_focused: 0,
            fre_completed: false,
            has_save: false,
            mission_active: false,
            show_me_why_visible: false,
            last_tip_index: 0,
            intro_slideshow_slot: None,
        }
    }
}

/// Shell-state-transition event. Emitted by user input handlers + by
/// `ShellApiCommand` handler. Consumed by `apply_shell_transitions`.
#[derive(Debug, Clone, Message)]
pub struct ShellTransition {
    pub to: ShellScreen,
    pub source: TransitionSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionSource {
    UserInput,
    ScriptedApi,
    SplashTimeout,
    AutoAfterMissionResolved,
}

/// Save-slot metadata index — rebuilt at startup from `content/saves/<slot>.meta.json`
/// (DR-029 save model). At M11A this is a placeholder Vec; M11A renders it,
/// M27 wires the actual save subsystem to populate it.
#[derive(Debug, Resource, Default)]
pub struct SaveSlotMetadataIndex {
    pub named: Vec<SaveSlotMetadata>,
    pub auto_saves: Vec<SaveSlotMetadata>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SaveSlotMetadata {
    pub slot_index: usize,
    pub slot_kind: SlotKind,
    pub display_name: String,
    pub mission_name: String,
    pub tick_count: u64,
    pub wall_clock_seconds: u64,
    pub last_play_iso: String,
    pub player_actor_portrait_id: String,
    pub faction_relations: Vec<FactionRepLine>,
    pub achievements_unlocked_count: u32,
    pub total_play_time_seconds: u64,
    pub cloud_sync_state: CloudSyncState,
    pub thumbnail_png_path: Option<String>,
    pub is_corrupt: bool,
    pub is_empty: bool,
    /// **M4B § "save_load module reads + displays schema version next to
    /// each slot"** — pretty schema version (`vMAJOR.MINOR.PATCH`) parsed
    /// from the on-disk `.cfsave` payload. Empty when the slot is empty.
    #[serde(default)]
    pub save_schema_version_pretty: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SlotKind {
    #[default]
    Named,
    AutoSave,
    QuickSave,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FactionRepLine {
    pub faction_id: String,
    pub display_name: String,
    pub rep_score: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CloudSyncState {
    #[default]
    LocalOnly,
    Synced,
    Syncing,
    Conflict,
}

/// Settings scaffold — temporary M11A surface until M38 ships full schema.
/// 9 ACC-A floor settings + 12 core settings = 21 initial keys. M38
/// schema replaces this without M11A code change (settings_tree reads
/// from the registry at startup).
#[derive(Debug, Resource)]
pub struct SettingsScaffold {
    pub keys: BTreeMap<String, SettingDescriptor>,
}

impl Default for SettingsScaffold {
    fn default() -> Self {
        let mut keys = BTreeMap::new();

        // Display tab (3 keys at scaffold; M38 adds 30+ more)
        keys.insert("display.resolution".to_string(), SettingDescriptor::dropdown(
            "Resolution", "Display", &["1280x720", "1920x1080", "2560x1440", "3840x2160"], "1920x1080",
        ));
        keys.insert("display.mode".to_string(), SettingDescriptor::dropdown(
            "Display mode", "Display", &["Fullscreen", "Borderless", "Windowed"], "Fullscreen",
        ));
        keys.insert("display.vsync".to_string(), SettingDescriptor::dropdown(
            "VSync", "Display", &["Off", "On", "Adaptive"], "On",
        ));

        // Audio tab (5 keys at scaffold)
        keys.insert("audio.master_volume".to_string(), SettingDescriptor::slider("Master volume", "Audio", 0.0, 1.0, 0.7));
        keys.insert("audio.sfx_volume".to_string(), SettingDescriptor::slider("SFX volume", "Audio", 0.0, 1.0, 0.7));
        keys.insert("audio.music_volume".to_string(), SettingDescriptor::slider("Music volume", "Audio", 0.0, 1.0, 0.5));
        keys.insert("audio.voice_volume".to_string(), SettingDescriptor::slider("Voice volume", "Audio", 0.0, 1.0, 0.8));
        keys.insert("audio.ambient_volume".to_string(), SettingDescriptor::slider("Ambient volume", "Audio", 0.0, 1.0, 0.6));

        // Controls tab (2 keys at scaffold; full rebind at M11A but only 2 surfaced)
        keys.insert("controls.mouse_sensitivity".to_string(), SettingDescriptor::slider("Mouse sensitivity", "Controls", 0.1, 5.0, 1.0));
        keys.insert("controls.invert_y".to_string(), SettingDescriptor::toggle("Invert Y", "Controls", false));

        // Accessibility tab (9 ACC-A floor keys)
        keys.insert("acc.text_scale".to_string(), SettingDescriptor::slider("Text scale", "Accessibility", 0.5, 4.0, 1.0));
        keys.insert("acc.high_contrast".to_string(), SettingDescriptor::toggle("High contrast", "Accessibility", false));
        keys.insert("acc.captions".to_string(), SettingDescriptor::dropdown("Captions", "Accessibility", &["Off", "Critical only", "Standard", "Expanded"], "Standard"));
        keys.insert("acc.reduce_motion".to_string(), SettingDescriptor::toggle("Reduce motion", "Accessibility", false));
        keys.insert("acc.reduce_shake".to_string(), SettingDescriptor::toggle("Reduce shake", "Accessibility", false));
        keys.insert("acc.reduce_flash".to_string(), SettingDescriptor::toggle("Reduce flash", "Accessibility", false));
        keys.insert("acc.hold_to_confirm".to_string(), SettingDescriptor::toggle("Hold to confirm", "Accessibility", true));
        keys.insert("acc.hold_threshold_ms".to_string(), SettingDescriptor::slider("Hold threshold (ms)", "Accessibility", 50.0, 2000.0, 250.0));
        keys.insert("acc.color_cue_mode".to_string(), SettingDescriptor::dropdown("Color cue mode", "Accessibility", &["Default", "Colorblind-safe", "Monochrome-test"], "Default"));
        // M12: cinematic story-beats + juice — comic-style overlays opt-in flag.
        // Subtle = default (speech bubbles for storyteller events only, no
        // onomatopoeia stamps, comic death recap behind toggle). Full = all
        // comic flavor on. Off = never render any comic framing.
        keys.insert("ux.comic_style_overlay".to_string(), SettingDescriptor::dropdown(
            "Comic-style overlays", "Accessibility", &["full", "subtle", "off"], "subtle",
        ));
        // M12: death-recap rendering mode. Default false = M10 replay viewer
        // + cause-chain walker. True = 4-panel comic-style cause chain.
        keys.insert("ux.comic_death_recap".to_string(), SettingDescriptor::toggle(
            "Comic death recap", "Accessibility", false,
        ));

        // Gameplay tab (1 key at scaffold)
        keys.insert("gameplay.storyteller".to_string(), SettingDescriptor::dropdown("Storyteller", "Gameplay", &["Cassandra Classic", "Phoebe Chillax", "Randy Random", "Ironman", "Sandbox"], "Cassandra Classic"));

        // Language+Privacy tab (1 key at scaffold)
        keys.insert("language.locale".to_string(), SettingDescriptor::dropdown("Language", "Language+Privacy", &["en", "es", "fr", "de", "it", "pt-BR", "ru", "pl", "tr", "ja", "ko", "zh-Hans", "zh-Hant", "th", "vi", "ar", "hi", "id", "cs"], "en"));

        Self { keys }
    }
}

#[derive(Debug, Clone)]
pub struct SettingDescriptor {
    pub label: String,
    pub tab: String,
    pub kind: SettingKind,
    pub default: String,
}

impl SettingDescriptor {
    pub fn slider(label: &str, tab: &str, min: f32, max: f32, default: f32) -> Self {
        Self {
            label: label.to_string(),
            tab: tab.to_string(),
            kind: SettingKind::Slider { min, max },
            default: format!("{}", default),
        }
    }

    pub fn toggle(label: &str, tab: &str, default: bool) -> Self {
        Self {
            label: label.to_string(),
            tab: tab.to_string(),
            kind: SettingKind::Toggle,
            default: if default { "true".to_string() } else { "false".to_string() },
        }
    }

    pub fn dropdown(label: &str, tab: &str, options: &[&str], default: &str) -> Self {
        Self {
            label: label.to_string(),
            tab: tab.to_string(),
            kind: SettingKind::Dropdown {
                options: options.iter().map(|s| s.to_string()).collect(),
            },
            default: default.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SettingKind {
    Slider { min: f32, max: f32 },
    Toggle,
    Dropdown { options: Vec<String> },
}

/// Apply pending shell transitions. Updates `ShellState.current` +
/// records the previous screen for back-navigation.
pub fn apply_shell_transitions(
    mut transitions: MessageReader<ShellTransition>,
    mut state: ResMut<ShellState>,
) {
    for t in transitions.read() {
        if t.to == state.current {
            continue;
        }
        tracing::info!(
            target = "cf-shell",
            from = ?state.current,
            to = ?t.to,
            source = ?t.source,
            "shell screen transition"
        );
        state.previous = state.current;
        state.current = t.to;
        state.menu_focused_index = 0;
        state.settings_focused_row = 0;
    }
}
