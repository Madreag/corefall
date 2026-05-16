//! Pause menu — in-mission overlay invoked via [Esc].

use crate::state::ShellState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PauseMenuOption {
    Resume,
    Settings,
    SaveGame,
    LoadGame,
    ShowMeWhy,
    ReportBug,
    Tutorials,
    QuitToMenu,
    QuitToDesktop,
}

impl PauseMenuOption {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Resume => "Resume",
            Self::Settings => "Settings",
            Self::SaveGame => "Save Game",
            Self::LoadGame => "Load Game",
            Self::ShowMeWhy => "Show Me Why",
            Self::ReportBug => "Report Bug",
            Self::Tutorials => "Tutorials",
            Self::QuitToMenu => "Quit to Main Menu",
            Self::QuitToDesktop => "Quit to Desktop",
        }
    }

    pub fn destructive(&self) -> bool {
        matches!(self, Self::QuitToMenu | Self::QuitToDesktop | Self::LoadGame)
    }
}

/// Compute visible pause options based on current shell state.
/// Show-Me-Why hidden unless mission lost.
pub fn visible_pause_options(state: &ShellState) -> Vec<PauseMenuOption> {
    let mut opts = vec![
        PauseMenuOption::Resume,
        PauseMenuOption::Settings,
        PauseMenuOption::SaveGame,
        PauseMenuOption::LoadGame,
    ];
    if state.show_me_why_visible {
        opts.push(PauseMenuOption::ShowMeWhy);
    }
    opts.push(PauseMenuOption::ReportBug);
    opts.push(PauseMenuOption::Tutorials);
    opts.push(PauseMenuOption::QuitToMenu);
    opts.push(PauseMenuOption::QuitToDesktop);
    opts
}

/// Game-speed-assist behavior for pause overlay.
/// FullPause = sim halts; Slowdown75 = 75% speed; Slowdown25 = 25%.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GameSpeedAssist {
    Off,
    #[default]
    FullPause,
    Slowdown75,
    Slowdown25,
}

impl GameSpeedAssist {
    pub fn sim_speed_factor(&self) -> f32 {
        match self {
            Self::Off => 1.0,
            Self::FullPause => 0.0,
            Self::Slowdown75 => 0.75,
            Self::Slowdown25 => 0.25,
        }
    }

    pub fn is_paused(&self) -> bool {
        matches!(self, Self::FullPause)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn show_me_why_hidden_when_not_visible() {
        let state = ShellState::default();
        let opts = visible_pause_options(&state);
        assert!(!opts.contains(&PauseMenuOption::ShowMeWhy));
    }

    #[test]
    fn show_me_why_visible_after_mission_lost() {
        let mut state = ShellState::default();
        state.show_me_why_visible = true;
        let opts = visible_pause_options(&state);
        assert!(opts.contains(&PauseMenuOption::ShowMeWhy));
    }

    #[test]
    fn destructive_options_marked() {
        assert!(PauseMenuOption::QuitToMenu.destructive());
        assert!(PauseMenuOption::QuitToDesktop.destructive());
        assert!(PauseMenuOption::LoadGame.destructive());
        assert!(!PauseMenuOption::Resume.destructive());
        assert!(!PauseMenuOption::Settings.destructive());
    }

    #[test]
    fn game_speed_factors() {
        assert_eq!(GameSpeedAssist::Off.sim_speed_factor(), 1.0);
        assert_eq!(GameSpeedAssist::FullPause.sim_speed_factor(), 0.0);
        assert_eq!(GameSpeedAssist::Slowdown75.sim_speed_factor(), 0.75);
        assert_eq!(GameSpeedAssist::Slowdown25.sim_speed_factor(), 0.25);
    }

    #[test]
    fn full_pause_is_paused() {
        assert!(GameSpeedAssist::FullPause.is_paused());
        assert!(!GameSpeedAssist::Slowdown75.is_paused());
    }
}
