//! M8 — Weapon swap overlay HUD widget.
//!
//! Per spec § UX widgets: large icon during 300ms transition.

use bevy::prelude::*;

/// Spec-mandated transition duration in ms.
pub const SWAP_TRANSITION_MS: u32 = 300;

/// Weapon swap overlay widget Bevy resource.
#[derive(Resource, Debug, Clone, Default)]
pub struct WeaponSwapOverlayState {
    /// Whether the overlay is currently rendered.
    pub active: bool,
    /// Remaining transition time in ms.
    pub remaining_ms: u32,
    /// Display label for the incoming weapon.
    pub incoming_label: Option<String>,
}

impl WeaponSwapOverlayState {
    /// Trigger a fresh transition.
    pub fn trigger(&mut self, incoming: impl Into<String>) {
        self.active = true;
        self.remaining_ms = SWAP_TRANSITION_MS;
        self.incoming_label = Some(incoming.into());
    }

    /// Decay one frame; clears the overlay when remaining_ms hits zero.
    pub fn tick(&mut self, dt_ms: u32) {
        if !self.active {
            return;
        }
        self.remaining_ms = self.remaining_ms.saturating_sub(dt_ms);
        if self.remaining_ms == 0 {
            self.active = false;
            self.incoming_label = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_starts_transition() {
        let mut s = WeaponSwapOverlayState::default();
        s.trigger("Rifle");
        assert!(s.active);
        assert_eq!(s.remaining_ms, SWAP_TRANSITION_MS);
        assert_eq!(s.incoming_label.as_deref(), Some("Rifle"));
    }

    #[test]
    fn tick_clears_when_done() {
        let mut s = WeaponSwapOverlayState::default();
        s.trigger("SMG");
        s.tick(SWAP_TRANSITION_MS);
        assert!(!s.active);
        assert!(s.incoming_label.is_none());
    }
}
