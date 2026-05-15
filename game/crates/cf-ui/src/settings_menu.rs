//! M8 — Settings menu HUD widget — 6-tab tree per spec § Settings menu
//! tree (Graphics / Audio / Controls / Accessibility / Gameplay / Language).

use bevy::prelude::*;

/// One of the 6 spec-mandated settings tabs.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum SettingsTab {
    /// Graphics tab.
    #[default]
    Graphics,
    /// Audio tab.
    Audio,
    /// Controls tab.
    Controls,
    /// Accessibility tab.
    Accessibility,
    /// Gameplay tab.
    Gameplay,
    /// Language tab.
    Language,
}

impl SettingsTab {
    /// Every tab in display order.
    pub const ALL: [SettingsTab; 6] = [
        SettingsTab::Graphics,
        SettingsTab::Audio,
        SettingsTab::Controls,
        SettingsTab::Accessibility,
        SettingsTab::Gameplay,
        SettingsTab::Language,
    ];

    /// Player-facing tab label.
    pub fn label(self) -> &'static str {
        match self {
            SettingsTab::Graphics => "Graphics",
            SettingsTab::Audio => "Audio",
            SettingsTab::Controls => "Controls",
            SettingsTab::Accessibility => "Accessibility",
            SettingsTab::Gameplay => "Gameplay",
            SettingsTab::Language => "Language",
        }
    }
}

/// Settings menu widget Bevy resource.
#[derive(Resource, Debug, Clone, Default)]
pub struct SettingsMenuState {
    /// Whether the menu is currently open.
    pub open: bool,
    /// Active tab.
    pub active_tab: SettingsTab,
}

impl SettingsMenuState {
    /// Open the settings menu (defaults to the Graphics tab).
    pub fn open(&mut self) {
        self.open = true;
    }

    /// Close the settings menu.
    pub fn close(&mut self) {
        self.open = false;
    }

    /// Switch active tab (no-op when closed).
    pub fn switch(&mut self, tab: SettingsTab) -> bool {
        if !self.open {
            return false;
        }
        self.active_tab = tab;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_count_is_6() {
        assert_eq!(SettingsTab::ALL.len(), 6);
    }

    #[test]
    fn switch_requires_open() {
        let mut s = SettingsMenuState::default();
        assert!(!s.switch(SettingsTab::Audio));
        s.open();
        assert!(s.switch(SettingsTab::Audio));
        assert_eq!(s.active_tab, SettingsTab::Audio);
    }
}
