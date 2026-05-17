//! cf-shell — M11A Shell UI Foundation.
//!
//! Every screen the player sees OUTSIDE of in-mission HUD lives here:
//! splash + title + main menu + pause menu + save/load + 6-tab settings tree
//! + credits + loading screens + first-run-experience polish.
//!
//! Architecture per `specs/active/M11A.md`:
//!
//! - **Shell UI is OUTSIDE in-mission sim** — cf-shell runs without
//!   cf-sim-core active for title/menu; integrates with sim for pause/save/load.
//! - **Auto-generated UI from M38 schema** — settings tree reads schema at
//!   startup; never hardcoded. M38 not yet shipped, so a minimal scaffold
//!   schema covering ACC-A floor + 12 core settings is used at M11A.
//! - **Per-platform layout via Settings.ui_density** — Compact vs Comfortable;
//!   Steam Deck defaults Compact.
//! - **Attract mode plays a real M9 bundle** — not pre-recorded video; uses
//!   M40A spectator director on a pre-shipped run bundle. Until M40A ships,
//!   a static SVG splash (`title_splash_command_core_silhouette`) is used
//!   when `Settings.reduce_motion=true`.
//! - **All shell UI ACC-A compliant** — 12-node focus ring + per-action
//!   captions + reduce_motion respect.

#![deny(unsafe_code)]
#![allow(
    clippy::module_name_repetitions,
    clippy::type_complexity,
    clippy::needless_pass_by_value,
    clippy::too_many_arguments
)]

pub mod attract_mode;
pub mod credits;
pub mod fre_wizard;
pub mod keybinds;
pub mod loading_screen;
pub mod main_menu;
pub mod pause_menu;
pub mod save_load;
pub mod save_slot_preview;
pub mod settings_tree;
pub mod shell_api;
pub mod splash;
pub mod state;
pub mod title;

pub use shell_api::{IntroSlideshowSlot, ShellApiCommand};
pub use state::{ShellScreen, ShellState};

use bevy::prelude::*;

/// Bevy plugin that wires the entire M11A shell UI surface.
///
/// Adds the `ShellState` resource, registers the screen-state transitions,
/// and provides systems for splash → title → main_menu → pause →
/// save_load → settings_tree → credits → loading_screen flows.
///
/// **M12**: the plugin also exposes the `act.shell.OpenIntroSlideshow`
/// command surface for cfctl + cf-shell::main_menu's "Replay Intro"
/// quick action.
pub struct ShellPlugin;

impl Plugin for ShellPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShellState>()
            .init_resource::<state::SaveSlotMetadataIndex>()
            .init_resource::<state::SettingsScaffold>()
            .add_message::<state::ShellTransition>()
            .add_message::<shell_api::ShellApiCommand>()
            .add_systems(Update, (state::apply_shell_transitions, shell_api::handle_shell_api_commands));
    }
}
