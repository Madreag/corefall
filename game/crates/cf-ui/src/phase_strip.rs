//! M8 — Phase strip HUD widget (current mission phase 1-4 + countdown).

use bevy::prelude::*;

/// Mission phases per spec § Phase strip.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum MissionPhase {
    /// Phase 1.
    Setup = 1,
    /// Phase 2.
    Buildup = 2,
    /// Phase 3.
    Climax = 3,
    /// Phase 4.
    Debrief = 4,
}

impl MissionPhase {
    /// Player-facing label.
    pub fn label(self) -> &'static str {
        match self {
            MissionPhase::Setup => "Phase 1 — Setup",
            MissionPhase::Buildup => "Phase 2 — Buildup",
            MissionPhase::Climax => "Phase 3 — Climax",
            MissionPhase::Debrief => "Phase 4 — Debrief",
        }
    }
}

/// Phase strip widget Bevy resource.
#[derive(Resource, Debug, Clone)]
pub struct PhaseStripState {
    /// Current phase.
    pub phase: MissionPhase,
    /// Optional countdown (seconds until phase advance).
    pub countdown_seconds: Option<u32>,
}

impl Default for PhaseStripState {
    fn default() -> Self {
        Self {
            phase: MissionPhase::Setup,
            countdown_seconds: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_match_spec() {
        assert_eq!(MissionPhase::Setup.label(), "Phase 1 — Setup");
        assert_eq!(MissionPhase::Climax.label(), "Phase 3 — Climax");
    }
}
