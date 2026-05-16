//! Save / load slot UI orchestration.

use crate::state::{SaveSlotMetadata, SaveSlotMetadataIndex, SlotKind};
use serde::{Deserialize, Serialize};

pub const NAMED_SLOT_COUNT: usize = 10;
pub const AUTO_SAVE_SLOT_COUNT: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SortMode {
    #[default]
    LastPlayed,
    Created,
    MissionName,
    Difficulty,
}

impl SortMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::LastPlayed => "Last played",
            Self::Created => "Created",
            Self::MissionName => "Mission name",
            Self::Difficulty => "Difficulty",
        }
    }
}

/// Sort save-slot index per requested mode.
pub fn sort_slots(slots: &[SaveSlotMetadata], mode: SortMode) -> Vec<SaveSlotMetadata> {
    let mut sorted = slots.to_vec();
    match mode {
        SortMode::LastPlayed => sorted.sort_by(|a, b| b.last_play_iso.cmp(&a.last_play_iso)),
        SortMode::Created => sorted.sort_by(|a, b| a.slot_index.cmp(&b.slot_index)),
        SortMode::MissionName => sorted.sort_by(|a, b| a.mission_name.cmp(&b.mission_name)),
        SortMode::Difficulty => sorted.sort_by(|a, b| a.tick_count.cmp(&b.tick_count)),
    }
    sorted
}

/// Initialize a fresh save-slot metadata index with empty placeholders.
pub fn empty_slot_index() -> SaveSlotMetadataIndex {
    let named = (0..NAMED_SLOT_COUNT).map(|i| SaveSlotMetadata {
        slot_index: i,
        slot_kind: SlotKind::Named,
        is_empty: true,
        ..Default::default()
    }).collect();
    let auto_saves = (0..AUTO_SAVE_SLOT_COUNT).map(|i| SaveSlotMetadata {
        slot_index: i,
        slot_kind: SlotKind::AutoSave,
        is_empty: true,
        ..Default::default()
    }).collect();
    SaveSlotMetadataIndex { named, auto_saves }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_index_has_correct_counts() {
        let idx = empty_slot_index();
        assert_eq!(idx.named.len(), NAMED_SLOT_COUNT);
        assert_eq!(idx.auto_saves.len(), AUTO_SAVE_SLOT_COUNT);
        for slot in &idx.named {
            assert!(slot.is_empty);
            assert_eq!(slot.slot_kind, SlotKind::Named);
        }
        for slot in &idx.auto_saves {
            assert!(slot.is_empty);
            assert_eq!(slot.slot_kind, SlotKind::AutoSave);
        }
    }

    #[test]
    fn sort_by_last_played_descending() {
        let mut slots = vec![
            SaveSlotMetadata { slot_index: 0, last_play_iso: "2026-05-10T00:00:00Z".to_string(), ..Default::default() },
            SaveSlotMetadata { slot_index: 1, last_play_iso: "2026-05-15T00:00:00Z".to_string(), ..Default::default() },
            SaveSlotMetadata { slot_index: 2, last_play_iso: "2026-05-12T00:00:00Z".to_string(), ..Default::default() },
        ];
        slots = sort_slots(&slots, SortMode::LastPlayed);
        assert_eq!(slots[0].slot_index, 1);
        assert_eq!(slots[1].slot_index, 2);
        assert_eq!(slots[2].slot_index, 0);
    }

    #[test]
    fn sort_modes_distinct_labels() {
        assert_eq!(SortMode::LastPlayed.label(), "Last played");
        assert_eq!(SortMode::Created.label(), "Created");
        assert_eq!(SortMode::MissionName.label(), "Mission name");
        assert_eq!(SortMode::Difficulty.label(), "Difficulty");
    }
}
