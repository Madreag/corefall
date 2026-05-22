//! Main menu (post-Continue) — career arc summary + quick actions +
//! per-faction relationship summary + news ticker + daily quest.

use serde::{Deserialize, Serialize};

use crate::shell_api::{IntroSlideshowSlot, SaveLoadMode, ShellApiCommand};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MainMenuView {
    pub career_arc_summary: Vec<CareerArcLine>,
    pub last_played_summary: Option<LastPlayedSummary>,
    pub quick_actions: Vec<QuickAction>,
    pub faction_top3: Vec<FactionRepLine>,
    pub news_ticker: Vec<String>,
    pub daily_quest_active: Option<DailyQuestLine>,
    pub server_status: ServerStatusLine,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CareerArcLine {
    pub actor_name: String,
    pub mission_count: u32,
    pub kills: u32,
    pub assists: u32,
    pub current_rank: String,
    pub portrait_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LastPlayedSummary {
    pub mission_name: String,
    pub outcome: String,
    pub timestamp_iso: String,
    pub cause_chain_summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuickAction {
    ResumeMission,
    Loadout,
    MechBay,
    Base,
    Workshop,
    ReplayIntro,
}

impl QuickAction {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ResumeMission => "Resume Mission",
            Self::Loadout => "Loadout",
            Self::MechBay => "Mech Bay",
            Self::Base => "Base",
            Self::Workshop => "Workshop",
            Self::ReplayIntro => "Replay Intro",
        }
    }

    /// `ShellApiCommand`. cf-app's main-menu input handler routes the
    /// click through this and writes the returned command on the
    /// `ShellApiCommand` event bus. Returning `None` means "no scripted
    /// command, route through gameplay layers" (used for `MechBay` and
    /// `Base` which jump into the in-mission camera).
    pub fn to_shell_command(&self) -> Option<ShellApiCommand> {
        match self {
            Self::ResumeMission => Some(ShellApiCommand::ResumeMission),
            Self::Loadout => None,
            Self::MechBay => None,
            Self::Base => None,
            Self::Workshop => Some(ShellApiCommand::OpenWorkshop),
            // M12 § Story-telling surfaces — Main Menu → Story →
            // "Replay Intro" replays the 8-slide CCCP-style slideshow.
            Self::ReplayIntro => Some(ShellApiCommand::OpenIntroSlideshow {
                slot: IntroSlideshowSlot::Replay,
            }),
        }
    }
}

/// future quick-action variants can route to Save / Load without a churn
/// of imports here.
#[doc(hidden)]
pub const _SAVE_LOAD_MODE_REEXPORT: Option<SaveLoadMode> = None;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FactionRepLine {
    pub faction_id: String,
    pub display_name: String,
    pub rep_score: i32,
    pub rep_band: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DailyQuestLine {
    pub quest_id: String,
    pub title: String,
    pub time_remaining_seconds: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerStatusLine {
    pub mode: String,
    pub connected_to: Option<String>,
    pub player_count: u32,
}

/// Build a default main-menu view with placeholder content. Production
/// wiring populates from cf-save / cf-mission / M48B community webhook.
pub fn default_main_menu_view() -> MainMenuView {
    MainMenuView {
        career_arc_summary: vec![CareerArcLine {
            actor_name: "Player".to_string(),
            mission_count: 0,
            kills: 0,
            assists: 0,
            current_rank: "Recruit".to_string(),
            portrait_id: "portrait_faction_coalition_generic_male".to_string(),
        }],
        last_played_summary: None,
        quick_actions: vec![
            QuickAction::ResumeMission,
            QuickAction::Loadout,
            QuickAction::MechBay,
            QuickAction::Base,
            QuickAction::Workshop,
            QuickAction::ReplayIntro,
        ],
        faction_top3: vec![],
        news_ticker: vec![
            "Welcome to Corefall.".to_string(),
            "Hold the line.".to_string(),
        ],
        daily_quest_active: None,
        server_status: ServerStatusLine {
            mode: "single_player".to_string(),
            connected_to: None,
            player_count: 1,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_view_has_6_quick_actions() {
        let v = default_main_menu_view();
        // M12: ReplayIntro added — 6 quick actions total.
        assert_eq!(v.quick_actions.len(), 6);
    }

    #[test]
    fn quick_action_labels_distinct() {
        let labels: Vec<_> = [
            QuickAction::ResumeMission, QuickAction::Loadout, QuickAction::MechBay,
            QuickAction::Base, QuickAction::Workshop, QuickAction::ReplayIntro,
        ].iter().map(|a| a.label()).collect();
        let unique: std::collections::HashSet<_> = labels.iter().collect();
        assert_eq!(labels.len(), unique.len());
    }

    #[test]
    fn replay_intro_is_in_default_view() {
        let v = default_main_menu_view();
        assert!(v.quick_actions.contains(&QuickAction::ReplayIntro));
    }

    #[test]
    fn replay_intro_routes_to_open_slideshow_replay() {
        let cmd = QuickAction::ReplayIntro.to_shell_command();
        match cmd {
            Some(ShellApiCommand::OpenIntroSlideshow { slot }) => {
                assert_eq!(slot, IntroSlideshowSlot::Replay);
            }
            other => panic!("expected OpenIntroSlideshow(Replay) got {other:?}"),
        }
    }

    #[test]
    fn resume_mission_routes_to_resume_command() {
        let cmd = QuickAction::ResumeMission.to_shell_command();
        assert!(matches!(cmd, Some(ShellApiCommand::ResumeMission)));
    }

    #[test]
    fn workshop_routes_to_open_workshop() {
        let cmd = QuickAction::Workshop.to_shell_command();
        assert!(matches!(cmd, Some(ShellApiCommand::OpenWorkshop)));
    }

    #[test]
    fn mech_bay_and_base_have_no_scripted_command() {
        assert!(QuickAction::MechBay.to_shell_command().is_none());
        assert!(QuickAction::Base.to_shell_command().is_none());
    }
}
