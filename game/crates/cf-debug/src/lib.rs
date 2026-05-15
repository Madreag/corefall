//! cf-debug — M8 debug overlay registry. 7 toggleable overlays gated by
//! the player's accessibility / dev settings (F1..F7 in dev builds; in
//! production builds, gated by `Settings.debug.enabled = true`).
//!
//! The crate keeps zero render coupling: each overlay produces a typed
//! "render data" struct that cf-app's renderer (or cf-tools-replay-viewer)
//! consumes per-frame. Tests stay headless.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod ai_overlay;
pub mod collision_overlay;
pub mod material_overlay;
pub mod pathfinding_overlay;
pub mod physics_overlay;
pub mod sound_overlay;
pub mod squad_overlay;

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// One of the 7 spec-mandated debug overlays.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DebugOverlay {
    /// F1 — per-AI sight cone + hearing radius + memory grid + state label.
    AiState,
    /// F2 — current path + alternates + cost (forward-compat M22).
    Pathfinding,
    /// F3 — AABB outlines for actors + items + projectiles.
    Collision,
    /// F4 — cursor tooltip: material + integrity + 9 affordance flags.
    Material,
    /// F5 — force vectors for actors with recent impulses.
    Physics,
    /// F6 — loudness ripples from sound sources + occlusion visualization.
    Sound,
    /// F7 — waypoint pins + doctrine labels per squad.
    Squad,
}

impl DebugOverlay {
    /// Every overlay variant in declaration order.
    pub const ALL: [DebugOverlay; 7] = [
        DebugOverlay::AiState,
        DebugOverlay::Pathfinding,
        DebugOverlay::Collision,
        DebugOverlay::Material,
        DebugOverlay::Physics,
        DebugOverlay::Sound,
        DebugOverlay::Squad,
    ];

    /// Canonical snake_case identifier (cfctl wire form + replay).
    pub fn as_str(self) -> &'static str {
        match self {
            DebugOverlay::AiState => "ai_state",
            DebugOverlay::Pathfinding => "pathfinding",
            DebugOverlay::Collision => "collision",
            DebugOverlay::Material => "material",
            DebugOverlay::Physics => "physics",
            DebugOverlay::Sound => "sound",
            DebugOverlay::Squad => "squad",
        }
    }

    /// Parse from the cfctl wire form.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<DebugOverlay> {
        Some(match value {
            "ai_state" => DebugOverlay::AiState,
            "pathfinding" => DebugOverlay::Pathfinding,
            "collision" => DebugOverlay::Collision,
            "material" => DebugOverlay::Material,
            "physics" => DebugOverlay::Physics,
            "sound" => DebugOverlay::Sound,
            "squad" => DebugOverlay::Squad,
            _ => return None,
        })
    }

    /// Default dev-build hotkey (F1..F7) per spec § Debug overlays.
    pub fn default_hotkey(self) -> &'static str {
        match self {
            DebugOverlay::AiState => "F1",
            DebugOverlay::Pathfinding => "F2",
            DebugOverlay::Collision => "F3",
            DebugOverlay::Material => "F4",
            DebugOverlay::Physics => "F5",
            DebugOverlay::Sound => "F6",
            DebugOverlay::Squad => "F7",
        }
    }
}

/// Registry of which overlays are currently enabled. Owned by the engine
/// and surfaced via cfctl `observe.debug.overlays`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DebugOverlayState {
    /// Set of currently-enabled overlays.
    pub enabled: BTreeSet<DebugOverlay>,
}

impl DebugOverlayState {
    /// Toggle the overlay on/off. Returns the new "enabled" state.
    pub fn toggle(&mut self, overlay: DebugOverlay) -> bool {
        if self.enabled.contains(&overlay) {
            self.enabled.remove(&overlay);
            false
        } else {
            self.enabled.insert(overlay);
            true
        }
    }

    /// Whether the overlay is currently rendered.
    pub fn is_enabled(&self, overlay: DebugOverlay) -> bool {
        self.enabled.contains(&overlay)
    }

    /// Snake_case identifiers of every currently-enabled overlay (for
    /// cfctl `observe.debug.overlays`).
    pub fn enabled_ids(&self) -> Vec<&'static str> {
        self.enabled.iter().map(|o| o.as_str()).collect()
    }

    /// Production-build gate: overlays only render when `dev_build=true` OR
    /// `settings_debug_enabled=true`. cf-control passes both; the helper
    /// composes the gate so consumers never short-circuit on their own.
    pub fn render_allowed(dev_build: bool, settings_debug_enabled: bool) -> bool {
        dev_build || settings_debug_enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_str() {
        for o in DebugOverlay::ALL {
            assert_eq!(DebugOverlay::from_str(o.as_str()), Some(o));
        }
    }

    #[test]
    fn hotkeys_are_f1_through_f7() {
        let hotkeys: Vec<_> = DebugOverlay::ALL.iter().map(|o| o.default_hotkey()).collect();
        assert_eq!(hotkeys, vec!["F1", "F2", "F3", "F4", "F5", "F6", "F7"]);
    }

    #[test]
    fn toggle_round_trips() {
        let mut s = DebugOverlayState::default();
        assert!(s.toggle(DebugOverlay::AiState));
        assert!(s.is_enabled(DebugOverlay::AiState));
        assert!(!s.toggle(DebugOverlay::AiState));
        assert!(!s.is_enabled(DebugOverlay::AiState));
    }

    #[test]
    fn enabled_ids_lists_in_canonical_order() {
        let mut s = DebugOverlayState::default();
        s.toggle(DebugOverlay::Squad);
        s.toggle(DebugOverlay::AiState);
        let ids = s.enabled_ids();
        assert_eq!(ids, vec!["ai_state", "squad"]);
    }

    #[test]
    fn render_allowed_dev_or_setting() {
        assert!(DebugOverlayState::render_allowed(true, false));
        assert!(DebugOverlayState::render_allowed(false, true));
        assert!(!DebugOverlayState::render_allowed(false, false));
    }
}
