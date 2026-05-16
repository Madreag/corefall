//! Main menu (post-Continue) — career arc summary + quick actions +
//! per-faction relationship summary + news ticker + daily quest.

use serde::{Deserialize, Serialize};

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
}

impl QuickAction {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ResumeMission => "Resume Mission",
            Self::Loadout => "Loadout",
            Self::MechBay => "Mech Bay",
            Self::Base => "Base",
            Self::Workshop => "Workshop",
        }
    }
}

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
    fn default_view_has_5_quick_actions() {
        let v = default_main_menu_view();
        assert_eq!(v.quick_actions.len(), 5);
    }

    #[test]
    fn quick_action_labels_distinct() {
        let labels: Vec<_> = [
            QuickAction::ResumeMission, QuickAction::Loadout, QuickAction::MechBay,
            QuickAction::Base, QuickAction::Workshop,
        ].iter().map(|a| a.label()).collect();
        let unique: std::collections::HashSet<_> = labels.iter().collect();
        assert_eq!(labels.len(), unique.len());
    }
}
