//! Save / load slot UI orchestration.

use crate::state::{SaveSlotMetadata, SaveSlotMetadataIndex, SlotKind};
use serde::{Deserialize, Serialize};

pub const NAMED_SLOT_COUNT: usize = 10;
pub const AUTO_SAVE_SLOT_COUNT: usize = 3;

/// slot"** — UI verdict for a single save slot's schema status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SlotSchemaVerdict {
    /// Save matches the current build's schema; no migration needed.
    Current { version_pretty: String },
    /// Save is older than the current build; a "Migrate now" CTA renders.
    OutOfDate {
        version_pretty: String,
        current_pretty: String,
    },
    /// Save is from a newer build than this one supports; loading is
    /// refused with a clear message.
    UnsupportedFuture {
        version_pretty: String,
        current_pretty: String,
    },
    /// Slot is empty (no save written yet).
    Empty,
}

impl SlotSchemaVerdict {
    pub fn from_slot_version(slot_version_string: Option<&str>) -> Self {
        let Some(s) = slot_version_string else {
            return SlotSchemaVerdict::Empty;
        };
        // Accept either `vX.Y.Z` (pretty form) or `[X, Y, Z]` (array form).
        let parts: Vec<&str> = s.trim_start_matches('v').split('.').collect();
        let parsed: Option<cf_save::SaveSchemaVersion> = if parts.len() == 3 {
            let a = parts[0].parse::<u16>().ok();
            let b = parts[1].parse::<u16>().ok();
            let c = parts[2].parse::<u16>().ok();
            match (a, b, c) {
                (Some(major), Some(minor), Some(patch)) => Some(cf_save::SaveSchemaVersion::new(major, minor, patch)),
                _ => None,
            }
        } else {
            None
        };
        let current_pretty = cf_save::CURRENT_SAVE_SCHEMA_VERSION.as_string();
        match parsed {
            Some(v) if v == cf_save::CURRENT_SAVE_SCHEMA_VERSION => SlotSchemaVerdict::Current {
                version_pretty: v.as_string(),
            },
            Some(v) if v.newer_than(cf_save::CURRENT_SAVE_SCHEMA_VERSION) => SlotSchemaVerdict::UnsupportedFuture {
                version_pretty: v.as_string(),
                current_pretty,
            },
            Some(v) => SlotSchemaVerdict::OutOfDate {
                version_pretty: v.as_string(),
                current_pretty,
            },
            None => SlotSchemaVerdict::Empty,
        }
    }

    pub fn migrate_cta_label(&self) -> Option<&'static str> {
        match self {
            SlotSchemaVerdict::OutOfDate { .. } => Some("Migrate now"),
            _ => None,
        }
    }

    pub fn refusal_label(&self) -> Option<String> {
        match self {
            SlotSchemaVerdict::UnsupportedFuture {
                version_pretty,
                current_pretty,
            } => Some(format!(
                "Created in newer version {version_pretty}. Update to load (current {current_pretty})."
            )),
            _ => None,
        }
    }

    /// Display string the slot row renders next to the slot name.
    pub fn slot_row_label(&self) -> String {
        match self {
            SlotSchemaVerdict::Current { version_pretty } => format!("Schema {version_pretty}"),
            SlotSchemaVerdict::OutOfDate { version_pretty, current_pretty } => {
                format!("Schema {version_pretty} (current {current_pretty})")
            }
            SlotSchemaVerdict::UnsupportedFuture { version_pretty, .. } => {
                format!("Schema {version_pretty} (FUTURE)")
            }
            SlotSchemaVerdict::Empty => String::from("(empty)"),
        }
    }
}

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

    #[test]
    fn schema_verdict_current_when_matches_build() {
        let pretty = cf_save::CURRENT_SAVE_SCHEMA_VERSION.as_string();
        let v = SlotSchemaVerdict::from_slot_version(Some(&pretty));
        assert!(matches!(v, SlotSchemaVerdict::Current { .. }));
        assert!(v.migrate_cta_label().is_none());
        assert!(v.refusal_label().is_none());
    }

    #[test]
    fn schema_verdict_out_of_date_offers_migrate_cta() {
        let v = SlotSchemaVerdict::from_slot_version(Some("v1.0.0"));
        assert!(matches!(v, SlotSchemaVerdict::OutOfDate { .. }));
        assert_eq!(v.migrate_cta_label(), Some("Migrate now"));
    }

    #[test]
    fn schema_verdict_future_rejects_with_clear_message() {
        let v = SlotSchemaVerdict::from_slot_version(Some("v99.0.0"));
        let refusal = v.refusal_label().unwrap();
        assert!(refusal.contains("v99.0.0"));
        assert!(refusal.contains("Update to load"));
    }

    #[test]
    fn schema_verdict_empty_when_no_version_present() {
        let v = SlotSchemaVerdict::from_slot_version(None);
        assert!(matches!(v, SlotSchemaVerdict::Empty));
    }

    #[test]
    fn slot_row_label_renders_each_verdict_distinctly() {
        let current = SlotSchemaVerdict::Current {
            version_pretty: "v2.0.0".to_string(),
        };
        let out_of_date = SlotSchemaVerdict::OutOfDate {
            version_pretty: "v1.0.0".to_string(),
            current_pretty: "v2.0.0".to_string(),
        };
        let future = SlotSchemaVerdict::UnsupportedFuture {
            version_pretty: "v99.0.0".to_string(),
            current_pretty: "v2.0.0".to_string(),
        };
        assert_eq!(current.slot_row_label(), "Schema v2.0.0");
        assert!(out_of_date.slot_row_label().contains("current v2.0.0"));
        assert!(future.slot_row_label().contains("FUTURE"));
        assert_eq!(SlotSchemaVerdict::Empty.slot_row_label(), "(empty)");
    }
}
