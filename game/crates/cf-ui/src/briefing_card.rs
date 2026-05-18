//! **M12C**: Mission-briefing 6-line fade card for opening cinematics.
//!
//! Per spec § "Mission-briefing card":
//!
//! > At T+15s, a 6-line briefing fades in over the lower third
//! > (objective + reward + risk + storyteller stinger); auto-dismisses
//! > at cinematic end.
//!
//! Per spec § Crates / modules touched:
//!
//! > `cf-ui::briefing_card` (NEW) — 6-line mission-briefing fade card;
//! > binds to opening cinematic kernel.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Default fade-in duration in ms.
pub const BRIEFING_FADE_IN_MS: u32 = 800;

/// Default fade-out duration in ms.
pub const BRIEFING_FADE_OUT_MS: u32 = 600;

/// Max lines per briefing card (spec § "6-line briefing"; we allow up
/// to 8 to give authors a small margin).
pub const BRIEFING_MAX_LINES: usize = 8;

/// Bevy `Resource` projection — cf-app mirrors per frame from the
/// active cinematic kernel.
#[derive(Resource, Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct BriefingCardState {
    /// True while the card is presenting (visible or fading).
    pub active: bool,
    /// True after the at_ms boundary, false before. Drives the fade-in.
    pub past_at_ms: bool,
    /// Lines to display (max [`BRIEFING_MAX_LINES`]; truncated otherwise).
    pub lines: Vec<String>,
    /// Current opacity (0..=1). Drives the fade animation.
    pub opacity: f32,
    /// Cinematic id (debug / accessibility id).
    pub cinematic_id: String,
}

impl BriefingCardState {
    /// Reset to inactive (called on `cinematic.ended` per spec §
    /// "auto-dismisses at cinematic end").
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Update from the latest cinematic kernel snapshot.
    pub fn update(&mut self, cinematic_id: &str, lines: &[String]) {
        self.cinematic_id = cinematic_id.to_string();
        let new_lines: Vec<String> = lines.iter().take(BRIEFING_MAX_LINES).cloned().collect();
        if new_lines.is_empty() {
            self.past_at_ms = false;
            self.lines.clear();
            self.opacity = 0.0;
            self.active = false;
            return;
        }
        self.lines = new_lines;
        self.past_at_ms = true;
        self.active = true;
    }

    /// Tick the fade-in/out animation. cf-app calls this with `dt_ms`.
    pub fn tick_fade(&mut self, dt_ms: u32) {
        if !self.active {
            self.opacity = (self.opacity - dt_ms as f32 / BRIEFING_FADE_OUT_MS as f32).max(0.0);
            return;
        }
        if self.past_at_ms {
            self.opacity = (self.opacity + dt_ms as f32 / BRIEFING_FADE_IN_MS as f32).min(1.0);
        } else {
            self.opacity = (self.opacity - dt_ms as f32 / BRIEFING_FADE_OUT_MS as f32).max(0.0);
        }
    }

    /// Whether the card should be drawn (active OR still fading out).
    #[must_use]
    pub fn should_render(&self) -> bool {
        self.opacity > 0.0
    }
}

/// Plugin that registers `BriefingCardState`. cf-app's bridge owns the
/// per-frame mirror system.
#[derive(Default)]
pub struct BriefingCardPlugin;

impl Plugin for BriefingCardPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BriefingCardState>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_hidden() {
        let s = BriefingCardState::default();
        assert!(!s.should_render());
        assert!(!s.active);
    }

    #[test]
    fn update_sets_lines_and_activates() {
        let mut s = BriefingCardState::default();
        s.update("cin_intro", &["a".to_string(), "b".to_string()]);
        assert!(s.active);
        assert_eq!(s.lines, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn empty_lines_deactivate_card() {
        let mut s = BriefingCardState::default();
        s.update("cin_intro", &["a".to_string()]);
        assert!(s.active);
        s.update("cin_intro", &[]);
        assert!(!s.active);
    }

    #[test]
    fn fade_tick_increases_opacity_when_active() {
        let mut s = BriefingCardState::default();
        s.update("cin_intro", &["a".to_string()]);
        s.tick_fade(400);
        assert!(s.opacity > 0.0);
        s.tick_fade(2_000);
        assert!((s.opacity - 1.0).abs() < 1e-3);
    }

    #[test]
    fn fade_tick_decreases_opacity_when_inactive() {
        let mut s = BriefingCardState::default();
        s.update("cin_intro", &["a".to_string()]);
        s.tick_fade(2_000); // ramp to 1.0
        s.clear();
        s.tick_fade(400);
        assert!(s.opacity < 1.0);
        s.tick_fade(2_000);
        assert!((s.opacity - 0.0).abs() < 1e-3);
    }

    #[test]
    fn lines_truncate_above_max() {
        let mut s = BriefingCardState::default();
        let many: Vec<String> = (0..20).map(|i| format!("line{i}")).collect();
        s.update("cin_intro", &many);
        assert_eq!(s.lines.len(), BRIEFING_MAX_LINES);
    }
}
