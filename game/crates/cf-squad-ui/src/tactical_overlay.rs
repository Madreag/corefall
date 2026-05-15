//! Tab tactical overlay state. Holding Tab pauses the sim (single-player)
//! or runs at 25% speed (multiplayer) and renders the squad management
//! pane on top of the live HUD per spec § Layer 2 Per-actor Priority Editor.

use serde::{Deserialize, Serialize};

/// Multiplayer sim-speed percentage while the tactical overlay is open.
/// Single-player pauses outright.
pub const MULTIPLAYER_TACTICAL_SIM_SPEED_PCT: u8 = 25;

/// Tactical overlay state. cf-control owns one instance per session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TacticalOverlayState {
    /// Whether the overlay is currently open.
    pub open: bool,
    /// Sim speed (0..=100) while open. 0 in single-player; 25 in multiplayer.
    pub sim_speed_pct: u8,
    /// Currently-focused actor in the editor (if any).
    pub focused_actor_id: Option<u64>,
    /// Total times the overlay has been opened this session.
    pub open_count: u32,
}

impl TacticalOverlayState {
    /// Build the closed-state default.
    pub fn closed() -> Self {
        Self::default()
    }

    /// Toggle the overlay. `multiplayer` selects the spec-mandated sim-
    /// speed (single-player pauses). Returns the new "open" state.
    pub fn toggle(&mut self, multiplayer: bool) -> bool {
        if self.open {
            self.open = false;
            self.sim_speed_pct = 100;
            self.focused_actor_id = None;
            false
        } else {
            self.open = true;
            self.sim_speed_pct = if multiplayer {
                MULTIPLAYER_TACTICAL_SIM_SPEED_PCT
            } else {
                0
            };
            self.open_count = self.open_count.saturating_add(1);
            true
        }
    }

    /// Focus an actor in the editor (in-place). No-op if overlay closed.
    pub fn focus(&mut self, actor_id: u64) -> bool {
        if !self.open {
            return false;
        }
        self.focused_actor_id = Some(actor_id);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_opens_with_pause_in_single_player() {
        let mut s = TacticalOverlayState::closed();
        assert!(s.toggle(false));
        assert!(s.open);
        assert_eq!(s.sim_speed_pct, 0);
        assert_eq!(s.open_count, 1);
    }

    #[test]
    fn toggle_opens_with_25pct_in_multiplayer() {
        let mut s = TacticalOverlayState::closed();
        assert!(s.toggle(true));
        assert_eq!(s.sim_speed_pct, MULTIPLAYER_TACTICAL_SIM_SPEED_PCT);
    }

    #[test]
    fn toggle_closes_resets_state() {
        let mut s = TacticalOverlayState::closed();
        s.toggle(false);
        s.focus(7);
        assert!(!s.toggle(false));
        assert!(!s.open);
        assert_eq!(s.focused_actor_id, None);
        assert_eq!(s.sim_speed_pct, 100);
    }

    #[test]
    fn focus_requires_open() {
        let mut s = TacticalOverlayState::closed();
        assert!(!s.focus(1));
        s.toggle(false);
        assert!(s.focus(1));
        assert_eq!(s.focused_actor_id, Some(1));
    }
}
