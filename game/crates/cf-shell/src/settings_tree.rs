//! Settings tree — auto-generated UI from M38 schema (or scaffold at M11A).
//!
//! Per spec: 6 tabs (Display / Audio / Controls / Accessibility / Gameplay
//! / Language+Privacy). Per-tab keyboard navigation. Reset-to-defaults
//! per group. Live preview for text_scale + contrast_mode + reduce_motion.

use crate::state::{SettingDescriptor, SettingsScaffold, SettingsTab};

/// Get all settings rows for a given tab.
pub fn rows_for_tab<'a>(scaffold: &'a SettingsScaffold, tab: SettingsTab) -> Vec<(&'a str, &'a SettingDescriptor)> {
    let tab_label = tab.label();
    scaffold.keys.iter()
        .filter(|(_, desc)| desc.tab == tab_label)
        .map(|(k, v)| (k.as_str(), v))
        .collect()
}

/// Validate a settings value matches the descriptor's kind + range.
pub fn validate_setting(desc: &SettingDescriptor, value: &str) -> Result<(), String> {
    use crate::state::SettingKind;
    match &desc.kind {
        SettingKind::Slider { min, max } => {
            let n: f32 = value.parse().map_err(|_| format!("not a number: {}", value))?;
            if n < *min || n > *max {
                return Err(format!("value {} outside range [{}, {}]", n, min, max));
            }
            Ok(())
        }
        SettingKind::Toggle => {
            if value == "true" || value == "false" {
                Ok(())
            } else {
                Err(format!("toggle requires 'true' or 'false', got '{}'", value))
            }
        }
        SettingKind::Dropdown { options } => {
            if options.iter().any(|o| o == value) {
                Ok(())
            } else {
                Err(format!("'{}' not in dropdown options {:?}", value, options))
            }
        }
    }
}

/// Reset all settings in a tab to their defaults.
pub fn reset_tab_to_defaults(scaffold: &SettingsScaffold, tab: SettingsTab) -> Vec<(String, String)> {
    let tab_label = tab.label();
    scaffold.keys.iter()
        .filter(|(_, desc)| desc.tab == tab_label)
        .map(|(k, desc)| (k.clone(), desc.default.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accessibility_tab_has_acc_a_floor_keys() {
        let scaffold = SettingsScaffold::default();
        let rows = rows_for_tab(&scaffold, SettingsTab::Accessibility);
        let keys: Vec<_> = rows.iter().map(|(k, _)| *k).collect();
        assert!(keys.contains(&"acc.text_scale"));
        assert!(keys.contains(&"acc.high_contrast"));
        assert!(keys.contains(&"acc.captions"));
        assert!(keys.contains(&"acc.reduce_motion"));
        assert!(keys.contains(&"acc.reduce_shake"));
        assert!(keys.contains(&"acc.reduce_flash"));
        assert!(keys.contains(&"acc.hold_to_confirm"));
        assert!(keys.contains(&"acc.hold_threshold_ms"));
        assert!(keys.contains(&"acc.color_cue_mode"));
    }

    #[test]
    fn validate_slider_in_range() {
        let scaffold = SettingsScaffold::default();
        let desc = scaffold.keys.get("acc.text_scale").unwrap();
        assert!(validate_setting(desc, "1.5").is_ok());
        assert!(validate_setting(desc, "0.3").is_err());
        assert!(validate_setting(desc, "5.0").is_err());
        assert!(validate_setting(desc, "abc").is_err());
    }

    #[test]
    fn validate_toggle_only_true_false() {
        let scaffold = SettingsScaffold::default();
        let desc = scaffold.keys.get("acc.high_contrast").unwrap();
        assert!(validate_setting(desc, "true").is_ok());
        assert!(validate_setting(desc, "false").is_ok());
        assert!(validate_setting(desc, "yes").is_err());
    }

    #[test]
    fn validate_dropdown_options() {
        let scaffold = SettingsScaffold::default();
        let desc = scaffold.keys.get("acc.captions").unwrap();
        assert!(validate_setting(desc, "Off").is_ok());
        assert!(validate_setting(desc, "Standard").is_ok());
        assert!(validate_setting(desc, "Banana").is_err());
    }

    #[test]
    fn reset_tab_returns_all_keys() {
        let scaffold = SettingsScaffold::default();
        let resets = reset_tab_to_defaults(&scaffold, SettingsTab::Audio);
        assert_eq!(resets.len(), 5);
    }

    #[test]
    fn each_tab_has_at_least_one_key() {
        let scaffold = SettingsScaffold::default();
        for tab in [
            SettingsTab::Display, SettingsTab::Audio, SettingsTab::Controls,
            SettingsTab::Accessibility, SettingsTab::Gameplay, SettingsTab::LanguagePrivacy,
        ] {
            let rows = rows_for_tab(&scaffold, tab);
            assert!(!rows.is_empty(), "tab {:?} has no rows", tab);
        }
    }
}
