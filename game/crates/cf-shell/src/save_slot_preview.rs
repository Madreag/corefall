//! Save-slot preview card — what the slot card displays per slot.
//!
//! Per spec: per-slot preview card shows thumbnail screenshot + mission
//! name + tick count + wall-clock duration + last play timestamp +
//! player actor portrait + faction relationship summary + achievements
//! unlocked count + total play time + cloud sync indicator.

use crate::state::{CloudSyncState, SaveSlotMetadata};

/// Format M/D/YYYY h:MM AM/PM in America/Phoenix per personal AGENTS.md.
/// Input: ISO-8601 timestamp string (UTC). Output: human-readable Phoenix
/// local-time string. Uses simple offset arithmetic (UTC-7 fixed).
pub fn format_phoenix_local(iso_utc: &str) -> String {
    if iso_utc.is_empty() {
        return "—".to_string();
    }
    // Extract YYYY-MM-DDTHH:MM:SS portion
    let dt = iso_utc.split('Z').next().unwrap_or(iso_utc);
    let parts: Vec<&str> = dt.split('T').collect();
    if parts.len() != 2 {
        return iso_utc.to_string();
    }
    let date_parts: Vec<&str> = parts[0].split('-').collect();
    let time_parts: Vec<&str> = parts[1].split(':').collect();
    if date_parts.len() < 3 || time_parts.len() < 2 {
        return iso_utc.to_string();
    }
    let (Ok(year), Ok(month), Ok(day)) = (
        date_parts[0].parse::<i32>(),
        date_parts[1].parse::<u32>(),
        date_parts[2].parse::<u32>(),
    ) else {
        return iso_utc.to_string();
    };
    let (Ok(hour_utc), Ok(minute)) = (time_parts[0].parse::<i32>(), time_parts[1].parse::<u32>()) else {
        return iso_utc.to_string();
    };
    // Subtract 7 for Phoenix
    let mut hour_local = hour_utc - 7;
    let mut day_local = day as i32;
    let mut month_local = month as i32;
    let mut year_local = year;
    if hour_local < 0 {
        hour_local += 24;
        day_local -= 1;
        if day_local < 1 {
            month_local -= 1;
            if month_local < 1 {
                month_local = 12;
                year_local -= 1;
            }
            day_local = days_in_month(month_local as u32, year_local);
        }
    }
    let (hour_12, am_pm) = if hour_local == 0 {
        (12, "AM")
    } else if hour_local < 12 {
        (hour_local, "AM")
    } else if hour_local == 12 {
        (12, "PM")
    } else {
        (hour_local - 12, "PM")
    };
    format!("{}/{}/{} {}:{:02} {}", month_local, day_local, year_local, hour_12, minute, am_pm)
}

fn days_in_month(month: u32, year: i32) -> i32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 { 29 } else { 28 },
        _ => 30,
    }
}

/// Format wall-clock seconds → "Xh Ym Zs".
pub fn format_duration(seconds: u64) -> String {
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;
    if h > 0 {
        format!("{}h {}m {}s", h, m, s)
    } else if m > 0 {
        format!("{}m {}s", m, s)
    } else {
        format!("{}s", s)
    }
}

/// Format the cloud-sync indicator label.
pub fn cloud_sync_label(state: CloudSyncState) -> &'static str {
    match state {
        CloudSyncState::LocalOnly => "Local only",
        CloudSyncState::Synced => "Synced",
        CloudSyncState::Syncing => "Syncing",
        CloudSyncState::Conflict => "Conflict",
    }
}

/// Get the SVG icon id for the cloud sync indicator.
pub fn cloud_sync_icon_id(state: CloudSyncState) -> &'static str {
    match state {
        CloudSyncState::LocalOnly => "menu_cloud_local_only",
        CloudSyncState::Synced => "menu_cloud_synced",
        CloudSyncState::Syncing => "menu_cloud_syncing",
        CloudSyncState::Conflict => "menu_cloud_conflict",
    }
}

/// Get the SVG icon id for the slot card per its state.
pub fn slot_card_icon_id(slot: &SaveSlotMetadata) -> &'static str {
    if slot.is_corrupt {
        "menu_save_slot_corrupt"
    } else if slot.is_empty {
        "menu_save_slot_empty"
    } else if matches!(slot.slot_kind, crate::state::SlotKind::AutoSave | crate::state::SlotKind::QuickSave) {
        "menu_save_slot_autosave"
    } else {
        "menu_save_slot_filled"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_duration_seconds_only() {
        assert_eq!(format_duration(45), "45s");
    }

    #[test]
    fn format_duration_minutes_seconds() {
        assert_eq!(format_duration(125), "2m 5s");
    }

    #[test]
    fn format_duration_hours_minutes_seconds() {
        assert_eq!(format_duration(3725), "1h 2m 5s");
    }

    #[test]
    fn format_phoenix_basic() {
        let result = format_phoenix_local("2026-05-15T19:30:00Z");
        assert_eq!(result, "5/15/2026 12:30 PM");
    }

    #[test]
    fn format_phoenix_midnight_rollover() {
        let result = format_phoenix_local("2026-05-15T05:00:00Z");
        assert_eq!(result, "5/14/2026 10:00 PM");
    }

    #[test]
    fn format_phoenix_empty_returns_dash() {
        assert_eq!(format_phoenix_local(""), "—");
    }

    #[test]
    fn cloud_sync_labels_distinct() {
        let labels = [
            cloud_sync_label(CloudSyncState::LocalOnly),
            cloud_sync_label(CloudSyncState::Synced),
            cloud_sync_label(CloudSyncState::Syncing),
            cloud_sync_label(CloudSyncState::Conflict),
        ];
        let unique: std::collections::HashSet<_> = labels.iter().collect();
        assert_eq!(labels.len(), unique.len());
    }

    #[test]
    fn empty_slot_uses_empty_icon() {
        let slot = SaveSlotMetadata { is_empty: true, ..Default::default() };
        assert_eq!(slot_card_icon_id(&slot), "menu_save_slot_empty");
    }

    #[test]
    fn corrupt_slot_uses_corrupt_icon() {
        let slot = SaveSlotMetadata { is_corrupt: true, is_empty: false, ..Default::default() };
        assert_eq!(slot_card_icon_id(&slot), "menu_save_slot_corrupt");
    }
}
