//! **M12C**: `Codex → Cinematics` replay surface.
//!
//! Per spec § "Codex replay surface":
//!
//! > New codex tab `Cinematics` under `Codex → Cinematics`, three
//! > subgroups: **Mission Openings** (30+) / **Between-Mission Beats**
//! > (5 per storyteller × variations) / **Campaign Endings** (5 per
//! > storyteller).
//! > Locked entries show silhouette + "watch the mission to unlock";
//! > unlocked entries are clickable for full replay.
//! > Per-cinematic metadata: `duration`, `storyteller`, `first_seen_at`,
//! > `mission_id?`.
//!
//! Per spec § Crates / modules touched:
//!
//! > `cf-ui::codex_cinematics_tab` (NEW) — Codex tab + per-cinematic
//! > replay launcher.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Subgroup classification matching spec § "three subgroups".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CinematicCodexSubgroup {
    /// Mission opening cinematics (30+).
    MissionOpenings,
    /// Between-mission storyteller monologues.
    BetweenMissionBeats,
    /// Campaign ending cinematics.
    CampaignEndings,
}

impl CinematicCodexSubgroup {
    /// Canonical snake_case identifier.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            CinematicCodexSubgroup::MissionOpenings => "mission_openings",
            CinematicCodexSubgroup::BetweenMissionBeats => "between_mission_beats",
            CinematicCodexSubgroup::CampaignEndings => "campaign_endings",
        }
    }
}

/// One row in the codex listing — metadata that the UI renders even
/// for locked entries (silhouette / "watch to unlock" hint).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CinematicCodexEntry {
    /// Cinematic id (matches save.cinematic_seen_set entry).
    pub id: String,
    /// Display label (e.g. "Reactor Defense — Drop").
    pub display_name: String,
    /// Subgroup.
    pub subgroup: CinematicCodexSubgroup,
    /// True when the player has watched (or skipped past the confirm
    /// window) — drives unlock state.
    pub unlocked: bool,
    /// Duration in ms.
    pub duration_ms: u32,
    /// Per-storyteller scope (`None` for storyteller-agnostic openings).
    pub storyteller: Option<String>,
    /// First-seen wall-clock timestamp (ISO 8601 UTC). Empty when
    /// locked.
    #[serde(default)]
    pub first_seen_at: String,
    /// Optional mission id for opening cinematics.
    #[serde(default)]
    pub mission_id: Option<String>,
}

/// Bevy `Resource` projection — cf-app rebuilds the entry list per
/// codex open from the cinematic registry + `save.cinematic_seen_set`.
#[derive(Resource, Debug, Default, Clone, PartialEq)]
pub struct CinematicCodexState {
    /// True when the player is on the Codex → Cinematics tab.
    pub tab_open: bool,
    /// Active subgroup filter (None = "show all subgroups").
    pub active_subgroup: Option<CinematicCodexSubgroup>,
    /// All known cinematics + their unlock state.
    pub entries: Vec<CinematicCodexEntry>,
    /// Currently-focused row (cursor) per accessibility focus ring.
    pub focused_index: Option<usize>,
    /// Id of the cinematic the user just clicked Replay on (cf-shell
    /// hook consumes + clears).
    pub pending_replay_id: Option<String>,
}

impl CinematicCodexState {
    /// Iterator over entries matching the active subgroup filter.
    pub fn visible_entries(&self) -> Box<dyn Iterator<Item = &CinematicCodexEntry> + '_> {
        match self.active_subgroup {
            Some(sub) => Box::new(self.entries.iter().filter(move |e| e.subgroup == sub)),
            None => Box::new(self.entries.iter()),
        }
    }

    /// Replace the entries list.
    pub fn set_entries(&mut self, entries: Vec<CinematicCodexEntry>) {
        self.entries = entries;
    }

    /// Count unlocked entries in a subgroup. Drives the "12 / 30 unlocked"
    /// HUD label.
    #[must_use]
    pub fn unlocked_count(&self, sub: CinematicCodexSubgroup) -> usize {
        self.entries.iter().filter(|e| e.subgroup == sub && e.unlocked).count()
    }

    /// Total entries in a subgroup.
    #[must_use]
    pub fn total_count(&self, sub: CinematicCodexSubgroup) -> usize {
        self.entries.iter().filter(|e| e.subgroup == sub).count()
    }

    /// Mark the entry as the user's replay target. cf-shell consumes +
    /// dispatches `act.player.replay_cinematic` then clears the field.
    pub fn request_replay(&mut self, id: &str) {
        if let Some(entry) = self.entries.iter().find(|e| e.id == id) {
            if entry.unlocked {
                self.pending_replay_id = Some(id.to_string());
            }
        }
    }
}

/// Plugin that registers `CinematicCodexState`.
#[derive(Default)]
pub struct CinematicCodexTabPlugin;

impl Plugin for CinematicCodexTabPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CinematicCodexState>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, sub: CinematicCodexSubgroup, unlocked: bool) -> CinematicCodexEntry {
        CinematicCodexEntry {
            id: id.to_string(),
            display_name: id.to_string(),
            subgroup: sub,
            unlocked,
            duration_ms: 30_000,
            storyteller: None,
            first_seen_at: if unlocked { "2026-05-17T00:00:00Z".to_string() } else { String::new() },
            mission_id: None,
        }
    }

    #[test]
    fn visible_entries_respects_subgroup_filter() {
        let mut s = CinematicCodexState::default();
        s.set_entries(vec![
            entry("cin_a", CinematicCodexSubgroup::MissionOpenings, true),
            entry("cin_b", CinematicCodexSubgroup::BetweenMissionBeats, true),
            entry("cin_c", CinematicCodexSubgroup::MissionOpenings, false),
        ]);
        s.active_subgroup = Some(CinematicCodexSubgroup::MissionOpenings);
        let ids: Vec<_> = s.visible_entries().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["cin_a", "cin_c"]);
    }

    #[test]
    fn unlocked_count_filters_by_subgroup() {
        let mut s = CinematicCodexState::default();
        s.set_entries(vec![
            entry("cin_a", CinematicCodexSubgroup::MissionOpenings, true),
            entry("cin_b", CinematicCodexSubgroup::MissionOpenings, false),
            entry("cin_c", CinematicCodexSubgroup::MissionOpenings, true),
        ]);
        assert_eq!(s.unlocked_count(CinematicCodexSubgroup::MissionOpenings), 2);
        assert_eq!(s.total_count(CinematicCodexSubgroup::MissionOpenings), 3);
    }

    #[test]
    fn request_replay_rejects_locked_entry() {
        let mut s = CinematicCodexState::default();
        s.set_entries(vec![
            entry("cin_locked", CinematicCodexSubgroup::MissionOpenings, false),
            entry("cin_unlocked", CinematicCodexSubgroup::MissionOpenings, true),
        ]);
        s.request_replay("cin_locked");
        assert!(s.pending_replay_id.is_none());
        s.request_replay("cin_unlocked");
        assert_eq!(s.pending_replay_id.as_deref(), Some("cin_unlocked"));
    }
}
