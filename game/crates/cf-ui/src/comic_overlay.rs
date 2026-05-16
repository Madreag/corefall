//! **M12** § Comic-style overlay (optional rendering layer).
//!
//! Three opt-in juice surfaces gated by `settings.ux.comic_style_overlay`:
//! - **Speech bubbles** — chatter, storyteller events.
//! - **Onomatopoeia stamps** — BOOM / CRACK / KCHK on impacts.
//! - **Comic-style death recap** — 4-panel cause chain (toggle behind
//!   `settings.ux.comic_death_recap`).
//!
//! Per spec § Comic-style framing — opt-in juice, not core identity:
//!
//! - `full` — speech bubbles for chatter, onomatopoeia on impacts,
//!   comic-panel boss intros, comic death recap available.
//! - `subtle` (DEFAULT) — speech bubbles for storyteller events only;
//!   no onomatopoeia stamps; comic death recap available behind toggle.
//! - `off` — never render any comic-style framing; all chatter shows
//!   as plain captions; impacts use particle juice only; death recap
//!   is timeline-only.
//!
//! cf-app mirrors the live `settings.ux.comic_style_overlay` value into
//! the [`ComicOverlayMode`] resource each frame; this module exposes
//! [`ComicOverlayMode::allows`] so renderers can query whether to draw a
//! specific surface.

use bevy::prelude::*;

/// **M12**: comic overlay enable mode — `full` / `subtle` (default) / `off`.
#[derive(Resource, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum ComicOverlayMode {
    /// Full opt-in — speech bubbles + onomatopoeia + boss panels + death
    /// recap available.
    Full,
    /// Default — speech bubbles for storyteller events only.
    #[default]
    Subtle,
    /// Disabled — no comic framing renders.
    Off,
}

impl ComicOverlayMode {
    /// Canonical snake_case identifier (matches the settings dropdown).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ComicOverlayMode::Full => "full",
            ComicOverlayMode::Subtle => "subtle",
            ComicOverlayMode::Off => "off",
        }
    }

    /// Parse from the snake_case wire form (case-insensitive).
    #[allow(clippy::should_implement_trait)]
    #[must_use]
    pub fn from_str(value: &str) -> Option<ComicOverlayMode> {
        Some(match value.to_ascii_lowercase().as_str() {
            "full" => ComicOverlayMode::Full,
            "subtle" => ComicOverlayMode::Subtle,
            "off" => ComicOverlayMode::Off,
            _ => return None,
        })
    }

    /// Query whether a specific comic surface is allowed under this mode.
    #[must_use]
    pub fn allows(self, surface: ComicSurface) -> bool {
        match (self, surface) {
            (ComicOverlayMode::Off, _) => false,
            (ComicOverlayMode::Full, _) => true,
            // Subtle: only storyteller speech bubbles + boss splash callout
            // get through. Routine NPC chatter, onomatopoeia, etc. are off.
            (ComicOverlayMode::Subtle, surface) => matches!(
                surface,
                ComicSurface::SpeechBubbleStoryteller | ComicSurface::BossSplashCallout
            ),
        }
    }
}

/// **M12**: comic surface taxonomy. Renderers query
/// [`ComicOverlayMode::allows`] before drawing each.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ComicSurface {
    /// Speech bubble for a storyteller event (Cassandra, etc.).
    SpeechBubbleStoryteller,
    /// Speech bubble for routine AI chatter / faction call-outs.
    SpeechBubbleChatter,
    /// Onomatopoeia stamp (BOOM / CRACK / KCHK) on impacts.
    OnomatopoeiaStamp,
    /// Boss-intro splash with onomatopoeia callout.
    BossSplashCallout,
    /// 4-panel comic-style death recap (also gated by `comic_death_recap`).
    DeathRecap,
    /// Comic-panel framing around the mission briefing or end screen.
    MissionFraming,
}

impl ComicSurface {
    /// Canonical snake_case identifier.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ComicSurface::SpeechBubbleStoryteller => "speech_bubble_storyteller",
            ComicSurface::SpeechBubbleChatter => "speech_bubble_chatter",
            ComicSurface::OnomatopoeiaStamp => "onomatopoeia_stamp",
            ComicSurface::BossSplashCallout => "boss_splash_callout",
            ComicSurface::DeathRecap => "death_recap",
            ComicSurface::MissionFraming => "mission_framing",
        }
    }
}

/// **M12**: canonical onomatopoeia vocabulary. Renderers pick a stamp
/// based on the event family (impact / shield / pickup / etc.).
pub const ONOMATOPOEIA_VOCABULARY: &[&str] = &[
    "BOOM", "CRACK", "KCHK", "WHAM", "THUD", "ZAP", "CLANK", "SPLAT", "FWOOSH", "DING",
];

/// Pick the canonical onomatopoeia stamp for an event family. Deterministic
/// (no rng — replays mirror).
#[must_use]
pub fn onomatopoeia_for(event_family: &str) -> &'static str {
    match event_family {
        "explosion" | "grenade" | "rocket" => "BOOM",
        "rifle" | "pistol" | "shotgun" | "sniper" => "CRACK",
        "melee_blade" | "knife" | "sword" => "KCHK",
        "melee_blunt" | "bash" => "WHAM",
        "body_hit" | "armor_dent" => "THUD",
        "energy" | "plasma" | "shock" => "ZAP",
        "shield_break" | "metal_break" => "CLANK",
        "fluid" | "gore" => "SPLAT",
        "flame" | "ignition" => "FWOOSH",
        "pickup" | "alert" => "DING",
        _ => "THUD",
    }
}

/// **M12**: live mode the renderer reads each frame. cf-app mirrors
/// `settings.ux.comic_style_overlay` into this resource.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComicOverlayState {
    pub mode: ComicOverlayMode,
    /// Mirror of `settings.ux.comic_death_recap`. When true AND mode != Off,
    /// the death-recap surface is allowed.
    pub comic_death_recap_toggle: bool,
}

impl Default for ComicOverlayState {
    fn default() -> Self {
        Self {
            mode: ComicOverlayMode::Subtle,
            comic_death_recap_toggle: false,
        }
    }
}

impl ComicOverlayState {
    /// Final allow-check for a surface, factoring in both the mode and the
    /// dedicated death-recap toggle.
    #[must_use]
    pub fn allows(&self, surface: ComicSurface) -> bool {
        if surface == ComicSurface::DeathRecap {
            return self.comic_death_recap_toggle && self.mode != ComicOverlayMode::Off;
        }
        self.mode.allows(surface)
    }
}

/// **M12**: comic overlay plugin wiring [`ComicOverlayState`].
pub struct ComicOverlayPlugin;

impl Plugin for ComicOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ComicOverlayState>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_round_trips_through_str() {
        for m in [
            ComicOverlayMode::Full,
            ComicOverlayMode::Subtle,
            ComicOverlayMode::Off,
        ] {
            assert_eq!(ComicOverlayMode::from_str(m.as_str()), Some(m));
            // Case-insensitive parse.
            assert_eq!(
                ComicOverlayMode::from_str(&m.as_str().to_uppercase()),
                Some(m)
            );
        }
        assert_eq!(ComicOverlayMode::from_str("garbage"), None);
    }

    #[test]
    fn default_mode_is_subtle() {
        assert_eq!(ComicOverlayMode::default(), ComicOverlayMode::Subtle);
    }

    #[test]
    fn off_mode_suppresses_every_surface() {
        let mode = ComicOverlayMode::Off;
        for surface in [
            ComicSurface::SpeechBubbleStoryteller,
            ComicSurface::SpeechBubbleChatter,
            ComicSurface::OnomatopoeiaStamp,
            ComicSurface::BossSplashCallout,
            ComicSurface::DeathRecap,
            ComicSurface::MissionFraming,
        ] {
            assert!(!mode.allows(surface), "off must suppress {surface:?}");
        }
    }

    #[test]
    fn full_mode_enables_every_surface() {
        let mode = ComicOverlayMode::Full;
        for surface in [
            ComicSurface::SpeechBubbleStoryteller,
            ComicSurface::SpeechBubbleChatter,
            ComicSurface::OnomatopoeiaStamp,
            ComicSurface::BossSplashCallout,
            ComicSurface::DeathRecap,
            ComicSurface::MissionFraming,
        ] {
            assert!(mode.allows(surface), "full must allow {surface:?}");
        }
    }

    #[test]
    fn subtle_mode_only_storyteller_and_boss_splash() {
        let mode = ComicOverlayMode::Subtle;
        assert!(mode.allows(ComicSurface::SpeechBubbleStoryteller));
        assert!(mode.allows(ComicSurface::BossSplashCallout));
        assert!(!mode.allows(ComicSurface::SpeechBubbleChatter));
        assert!(!mode.allows(ComicSurface::OnomatopoeiaStamp));
        assert!(!mode.allows(ComicSurface::MissionFraming));
    }

    #[test]
    fn death_recap_requires_explicit_toggle_and_non_off_mode() {
        let mut state = ComicOverlayState::default();
        state.comic_death_recap_toggle = false;
        assert!(!state.allows(ComicSurface::DeathRecap));
        state.comic_death_recap_toggle = true;
        assert!(state.allows(ComicSurface::DeathRecap));
        state.mode = ComicOverlayMode::Off;
        assert!(!state.allows(ComicSurface::DeathRecap));
    }

    #[test]
    fn onomatopoeia_lookup_is_deterministic() {
        assert_eq!(onomatopoeia_for("explosion"), "BOOM");
        assert_eq!(onomatopoeia_for("rifle"), "CRACK");
        assert_eq!(onomatopoeia_for("knife"), "KCHK");
        assert_eq!(onomatopoeia_for("plasma"), "ZAP");
        assert_eq!(onomatopoeia_for("ignition"), "FWOOSH");
        assert_eq!(onomatopoeia_for("unknown"), "THUD");
    }

    #[test]
    fn onomatopoeia_vocabulary_is_not_empty() {
        assert!(!ONOMATOPOEIA_VOCABULARY.is_empty());
        assert!(ONOMATOPOEIA_VOCABULARY.contains(&"BOOM"));
    }
}
