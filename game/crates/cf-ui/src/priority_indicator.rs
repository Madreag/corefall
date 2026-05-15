//! **M11 / c4b4ea0**: priority indicator per spec § Smart-AI HUD widgets.
//! Renders a compact icon per squad member showing the top-priority task
//! from PriorityTable.
//!
//! 8-icon vocabulary per spec. Project rule: "No emoji anywhere in HUD or
//! captions" — so M11 ships with placeholder asset glyphs (M9A baked)
//! plus an ASCII fallback for MonochromeTest mode.

use bevy::prelude::*;

/// 8-icon priority task vocabulary per spec.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum PriorityIcon {
    Engaging,
    Suppressing,
    Triaging,
    Repairing,
    HoldingCover,
    MarkingThreats,
    Retreating,
    Patrolling,
}

impl PriorityIcon {
    /// snake_case identifier for the cfctl wire form + asset lookup.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            PriorityIcon::Engaging => "engaging",
            PriorityIcon::Suppressing => "suppressing",
            PriorityIcon::Triaging => "triaging",
            PriorityIcon::Repairing => "repairing",
            PriorityIcon::HoldingCover => "holding_cover",
            PriorityIcon::MarkingThreats => "marking_threats",
            PriorityIcon::Retreating => "retreating",
            PriorityIcon::Patrolling => "patrolling",
        }
    }

    /// ASCII fallback glyph for MonochromeTest mode.
    /// Two-letter sigil per task — distinguishable in monochrome.
    #[must_use]
    pub fn ascii_glyph(self) -> &'static str {
        match self {
            PriorityIcon::Engaging => "EN",
            PriorityIcon::Suppressing => "SU",
            PriorityIcon::Triaging => "TR",
            PriorityIcon::Repairing => "RP",
            PriorityIcon::HoldingCover => "HC",
            PriorityIcon::MarkingThreats => "MK",
            PriorityIcon::Retreating => "RT",
            PriorityIcon::Patrolling => "PA",
        }
    }

    /// Parse from snake_case wire form. Also accepts a small set of M7
    /// priority-task names (engage/suppress/heal/etc.) as aliases.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "engaging" | "engage" => PriorityIcon::Engaging,
            "suppressing" | "suppress" => PriorityIcon::Suppressing,
            "triaging" | "triage" | "heal" => PriorityIcon::Triaging,
            "repairing" | "repair" => PriorityIcon::Repairing,
            "holding_cover" | "hold_cover" | "cover" => PriorityIcon::HoldingCover,
            "marking_threats" | "mark_threats" | "spot" => PriorityIcon::MarkingThreats,
            "retreating" | "retreat" => PriorityIcon::Retreating,
            "patrolling" | "patrol" => PriorityIcon::Patrolling,
            _ => return None,
        })
    }
}

/// Per-squadmate priority indicator state. cf-app's bridge writes this
/// from the engine's PriorityTable per actor.
#[derive(Debug, Clone, PartialEq)]
pub struct PriorityIndicatorEntry {
    /// Actor id.
    pub actor_id: u64,
    /// Top-priority task icon.
    pub top_icon: PriorityIcon,
    /// Top-3-weighted tasks (largest first) for the squad-strip tooltip.
    pub top3: Vec<PriorityIcon>,
}

/// Resource projection of the per-squad priority indicator surface.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct PriorityIndicatorState {
    pub entries: Vec<PriorityIndicatorEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_icons_have_unique_ascii_glyphs() {
        let glyphs = [
            PriorityIcon::Engaging,
            PriorityIcon::Suppressing,
            PriorityIcon::Triaging,
            PriorityIcon::Repairing,
            PriorityIcon::HoldingCover,
            PriorityIcon::MarkingThreats,
            PriorityIcon::Retreating,
            PriorityIcon::Patrolling,
        ];
        let mut set = std::collections::HashSet::new();
        for g in glyphs {
            assert!(set.insert(g.ascii_glyph()));
        }
    }

    #[test]
    fn parses_canonical_and_alias_names() {
        assert_eq!(PriorityIcon::from_str("engaging"), Some(PriorityIcon::Engaging));
        assert_eq!(PriorityIcon::from_str("engage"), Some(PriorityIcon::Engaging));
        assert_eq!(PriorityIcon::from_str("heal"), Some(PriorityIcon::Triaging));
        assert!(PriorityIcon::from_str("unknown_task").is_none());
    }
}
