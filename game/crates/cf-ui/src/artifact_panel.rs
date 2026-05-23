//! M16 § Artifact inventory panel + carried-bonus surface.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactPanelEntry {
    pub instance_id: u64,
    pub spec_id: String,
    pub display_name: String,
    pub rarity: String,
    /// One-line bonus summary.
    pub summary_line: String,
}

#[derive(Debug, Clone, Default, Resource, Serialize, Deserialize)]
pub struct ArtifactPanelState {
    pub entries: Vec<ArtifactPanelEntry>,
    /// Aggregate +max_hp / +aim_accuracy / etc. — string-encoded for
    /// HUD readout (e.g. "+30 HP, +10% aim, anomaly reveal").
    pub aggregate_summary: String,
}

impl ArtifactPanelState {
    pub fn refresh(&mut self, entries: Vec<ArtifactPanelEntry>, aggregate_summary: String) {
        self.entries = entries;
        self.aggregate_summary = aggregate_summary;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_default_empty() {
        let state = ArtifactPanelState::default();
        assert!(state.entries.is_empty());
        assert!(state.aggregate_summary.is_empty());
    }

    #[test]
    fn refresh_replaces_entries() {
        let mut state = ArtifactPanelState::default();
        let entries = vec![ArtifactPanelEntry {
            instance_id: 1,
            spec_id: "stone_blood".to_string(),
            display_name: "Stone Blood".to_string(),
            rarity: "rare".to_string(),
            summary_line: "+20 HP".to_string(),
        }];
        state.refresh(entries.clone(), "+20 HP".to_string());
        assert_eq!(state.entries.len(), 1);
        assert_eq!(state.aggregate_summary, "+20 HP");
    }
}
