//! **M11**: captions strip widget per spec § Captions surface — verbosity
//! modes + category filtering. Captions are the ACC-A floor's audio
//! fallback: every cue with `caption` enabled surfaces here.

use bevy::prelude::*;
use std::collections::BTreeSet;

use crate::HudCaption;

/// Max captions visible per spec § "max 4 visible; oldest evicts with
/// `[N more]` hint".
pub const CAPTION_QUEUE_MAX_VISIBLE: usize = 4;

/// Caption verbosity mode mirror of `cf_control::settings::CaptionMode`.
/// Duplicated here so cf-ui doesn't depend on cf-control's settings type.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum CaptionVerbosity {
    /// Disabled.
    #[default]
    Off,
    /// Only critical-severity events.
    CriticalOnly,
    /// Critical + warning.
    Standard,
    /// Critical + warning + info.
    Expanded,
}

impl CaptionVerbosity {
    /// Should this caption surface given the verbosity setting?
    #[must_use]
    pub fn allows(self, severity: &str) -> bool {
        match self {
            CaptionVerbosity::Off => false,
            CaptionVerbosity::CriticalOnly => severity == "critical",
            CaptionVerbosity::Standard => matches!(severity, "critical" | "warning"),
            CaptionVerbosity::Expanded => true,
        }
    }

    /// Parse from a snake_case wire string.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "critical_only" => CaptionVerbosity::CriticalOnly,
            "standard" => CaptionVerbosity::Standard,
            "expanded" => CaptionVerbosity::Expanded,
            _ => CaptionVerbosity::Off,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            CaptionVerbosity::Off => "off",
            CaptionVerbosity::CriticalOnly => "critical_only",
            CaptionVerbosity::Standard => "standard",
            CaptionVerbosity::Expanded => "expanded",
        }
    }
}

/// Resource projection of the caption queue. cf-app writes per frame from
/// the engine's `HudState.captions`.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct CaptionsState {
    pub captions: Vec<HudCaption>,
    /// Caption verbosity mode (drives event filtering).
    pub verbosity: CaptionVerbosity,
    /// Active caption categories per spec § Categories filter.
    pub categories: BTreeSet<String>,
    /// Caption background opacity (0..=1).
    pub background_opacity: f32,
}

impl CaptionsState {
    /// Visible captions per spec — capped at [`CAPTION_QUEUE_MAX_VISIBLE`].
    pub fn visible(&self) -> Vec<&HudCaption> {
        self.captions.iter().rev().take(CAPTION_QUEUE_MAX_VISIBLE).collect()
    }

    /// `true` when the named category is on (no categories restriction →
    /// all on).
    #[must_use]
    pub fn category_allowed(&self, cat: &str) -> bool {
        self.categories.is_empty() || self.categories.contains(cat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_blocks_everything() {
        assert!(!CaptionVerbosity::Off.allows("critical"));
        assert!(!CaptionVerbosity::Off.allows("info"));
    }

    #[test]
    fn critical_only_passes_critical() {
        assert!(CaptionVerbosity::CriticalOnly.allows("critical"));
        assert!(!CaptionVerbosity::CriticalOnly.allows("warning"));
    }

    #[test]
    fn standard_passes_critical_and_warning() {
        assert!(CaptionVerbosity::Standard.allows("critical"));
        assert!(CaptionVerbosity::Standard.allows("warning"));
        assert!(!CaptionVerbosity::Standard.allows("info"));
    }

    #[test]
    fn expanded_passes_everything() {
        assert!(CaptionVerbosity::Expanded.allows("critical"));
        assert!(CaptionVerbosity::Expanded.allows("info"));
    }

    #[test]
    fn visible_caps_to_4() {
        let mut s = CaptionsState::default();
        for i in 0..7 {
            s.captions.push(HudCaption {
                id: format!("c{i}"),
                label: "L".into(),
                raised_at_tick: i,
            });
        }
        assert_eq!(s.visible().len(), 4);
    }
}
