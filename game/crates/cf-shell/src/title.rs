//! Title screen — attract mode background + 9 menu options.
//!
//! Per spec: 9 menu options (Continue / New Game / Load Game / Multiplayer
//! / Workshop / Tutorials / Settings / Credits / Quit). Continue only
//! visible if a save exists.

use crate::state::ShellState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TitleMenuOption {
    Continue,
    NewGame,
    LoadGame,
    Multiplayer,
    Workshop,
    Tutorials,
    Settings,
    Credits,
    Quit,
}

impl TitleMenuOption {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Continue => "Continue",
            Self::NewGame => "New Game",
            Self::LoadGame => "Load Game",
            Self::Multiplayer => "Multiplayer",
            Self::Workshop => "Workshop",
            Self::Tutorials => "Tutorials",
            Self::Settings => "Settings",
            Self::Credits => "Credits",
            Self::Quit => "Quit",
        }
    }

    pub fn keyboard_shortcut(&self) -> &'static str {
        match self {
            Self::Continue => "C",
            Self::NewGame => "N",
            Self::LoadGame => "L",
            Self::Multiplayer => "M",
            Self::Workshop => "W",
            Self::Tutorials => "T",
            Self::Settings => "S",
            Self::Credits => "R",
            Self::Quit => "Q",
        }
    }
}

/// Compute visible menu options based on current shell state. Continue
/// hidden if no save exists.
pub fn visible_menu_options(state: &ShellState) -> Vec<TitleMenuOption> {
    let mut opts = Vec::with_capacity(9);
    if state.has_save {
        opts.push(TitleMenuOption::Continue);
    }
    opts.push(TitleMenuOption::NewGame);
    opts.push(TitleMenuOption::LoadGame);
    opts.push(TitleMenuOption::Multiplayer);
    opts.push(TitleMenuOption::Workshop);
    opts.push(TitleMenuOption::Tutorials);
    opts.push(TitleMenuOption::Settings);
    opts.push(TitleMenuOption::Credits);
    opts.push(TitleMenuOption::Quit);
    opts
}

/// Get the appropriate attract-mode asset id based on reduce_motion.
/// When reduce_motion=true, return the static splash; otherwise the
/// live attract-mode bundle id.
pub fn attract_mode_asset(reduce_motion: bool) -> &'static str {
    if reduce_motion {
        "title_splash_command_core_silhouette"
    } else {
        "attract_mode_m9_reactor_defense"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_save_hides_continue() {
        let state = ShellState::default();
        let opts = visible_menu_options(&state);
        assert!(!opts.contains(&TitleMenuOption::Continue));
        assert_eq!(opts.len(), 8);
    }

    #[test]
    fn with_save_shows_continue() {
        let mut state = ShellState::default();
        state.has_save = true;
        let opts = visible_menu_options(&state);
        assert!(opts.contains(&TitleMenuOption::Continue));
        assert_eq!(opts.len(), 9);
    }

    #[test]
    fn reduce_motion_returns_static_splash() {
        assert_eq!(attract_mode_asset(true), "title_splash_command_core_silhouette");
    }

    #[test]
    fn reduce_motion_off_returns_live_attract() {
        assert_eq!(attract_mode_asset(false), "attract_mode_m9_reactor_defense");
    }

    #[test]
    fn keyboard_shortcuts_unique() {
        let opts = vec![
            TitleMenuOption::Continue, TitleMenuOption::NewGame, TitleMenuOption::LoadGame,
            TitleMenuOption::Multiplayer, TitleMenuOption::Workshop, TitleMenuOption::Tutorials,
            TitleMenuOption::Settings, TitleMenuOption::Credits, TitleMenuOption::Quit,
        ];
        let shortcuts: Vec<_> = opts.iter().map(|o| o.keyboard_shortcut()).collect();
        let unique: std::collections::HashSet<_> = shortcuts.iter().collect();
        assert_eq!(shortcuts.len(), unique.len(), "keyboard shortcuts must be unique");
    }
}
