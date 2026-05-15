//! M8 — Branching banner HUD widget ("KILL or SNEAK" style choice).

use bevy::prelude::*;

/// One branch option label.
#[derive(Debug, Clone, PartialEq)]
pub struct BranchOption {
    /// Stable id for the branch.
    pub branch_id: String,
    /// Player-facing label (e.g. "KILL", "SNEAK").
    pub label: String,
}

/// Branching banner widget Bevy resource.
#[derive(Resource, Debug, Clone, Default)]
pub struct BranchingBannerState {
    /// Whether the banner is visible.
    pub visible: bool,
    /// Banner title (e.g. "BRANCHING OBJECTIVE").
    pub title: String,
    /// 2+ options the player can choose between.
    pub options: Vec<BranchOption>,
}

impl BranchingBannerState {
    /// Show the banner with the supplied title + options.
    pub fn show(&mut self, title: impl Into<String>, options: Vec<BranchOption>) {
        self.visible = true;
        self.title = title.into();
        self.options = options;
    }

    /// Hide the banner.
    pub fn hide(&mut self) {
        self.visible = false;
        self.options.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn show_and_hide() {
        let mut s = BranchingBannerState::default();
        s.show(
            "BRANCHING OBJECTIVE",
            vec![
                BranchOption {
                    branch_id: "kill".into(),
                    label: "KILL".into(),
                },
                BranchOption {
                    branch_id: "sneak".into(),
                    label: "SNEAK".into(),
                },
            ],
        );
        assert!(s.visible);
        assert_eq!(s.options.len(), 2);
        s.hide();
        assert!(!s.visible);
        assert!(s.options.is_empty());
    }
}
