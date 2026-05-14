//! M7: Mission director v0.5 — 4-phase pacing.
//!
//! Spec § Mission director v0.5 — 4 phases (Setup / Buildup / Climax /
//! Debrief). Each phase carries a min/max dwell window; the director
//! advances when the active phase's exit condition fires (timer + objective
//! completion + reinforcement-wave trigger).

use serde::{Deserialize, Serialize};

/// **M7**: 4-phase mission pacing.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MissionPhase {
    /// Mission start; players orient + receive briefing.
    #[default]
    Setup,
    /// Early enemy encounters.
    Buildup,
    /// Main objective + reinforcement waves.
    Climax,
    /// Mission resolution.
    Debrief,
}

impl MissionPhase {
    pub const ALL: [MissionPhase; 4] = [
        MissionPhase::Setup,
        MissionPhase::Buildup,
        MissionPhase::Climax,
        MissionPhase::Debrief,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            MissionPhase::Setup => "setup",
            MissionPhase::Buildup => "buildup",
            MissionPhase::Climax => "climax",
            MissionPhase::Debrief => "debrief",
        }
    }

    pub fn from_str(value: &str) -> Option<MissionPhase> {
        Some(match value {
            "setup" => MissionPhase::Setup,
            "buildup" => MissionPhase::Buildup,
            "climax" => MissionPhase::Climax,
            "debrief" => MissionPhase::Debrief,
            _ => return None,
        })
    }

    pub fn ordinal(self) -> u8 {
        self as u8
    }

    pub fn next(self) -> Option<MissionPhase> {
        match self {
            MissionPhase::Setup => Some(MissionPhase::Buildup),
            MissionPhase::Buildup => Some(MissionPhase::Climax),
            MissionPhase::Climax => Some(MissionPhase::Debrief),
            MissionPhase::Debrief => None,
        }
    }
}

/// **M7**: phase state + dwell timers. The mission director ticks this once
/// per game tick and emits `mission.phase_changed` on transitions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhaseState {
    pub current: MissionPhase,
    pub entered_tick: u64,
    pub setup_seconds: f32,
    pub buildup_seconds: f32,
    pub climax_seconds: f32,
    /// True once `current == Debrief`; latched so the director doesn't
    /// emit duplicate transitions.
    pub debrief_entered: bool,
}

impl PhaseState {
    pub fn new(start_tick: u64) -> Self {
        Self {
            current: MissionPhase::Setup,
            entered_tick: start_tick,
            setup_seconds: 30.0,
            buildup_seconds: 60.0,
            climax_seconds: 120.0,
            debrief_entered: false,
        }
    }

    /// Compute the deadline tick for the current phase given the configured
    /// tick rate.
    pub fn deadline_tick(&self, tick_rate_hz: u32) -> Option<u64> {
        let seconds = match self.current {
            MissionPhase::Setup => Some(self.setup_seconds),
            MissionPhase::Buildup => Some(self.buildup_seconds),
            MissionPhase::Climax => Some(self.climax_seconds),
            MissionPhase::Debrief => None,
        }?;
        let ticks = (seconds * tick_rate_hz as f32).max(1.0) as u64;
        Some(self.entered_tick.saturating_add(ticks))
    }

    /// Advance to the next phase; returns the new phase or None if already
    /// in Debrief.
    pub fn advance(&mut self, tick: u64) -> Option<MissionPhase> {
        let next = self.current.next()?;
        self.current = next;
        self.entered_tick = tick;
        if matches!(next, MissionPhase::Debrief) {
            self.debrief_entered = true;
        }
        Some(next)
    }

    /// Append checksum bytes covering the active phase + transition tick.
    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(16);
        out.push(self.current as u8);
        out.extend_from_slice(&self.entered_tick.to_le_bytes());
        out.push(u8::from(self.debrief_entered));
        out
    }
}

impl Default for PhaseState {
    fn default() -> Self {
        Self::new(0)
    }
}

/// **M7**: emitted whenever the active phase changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhaseChangedEvent {
    pub from: MissionPhase,
    pub to: MissionPhase,
    pub tick: u64,
    pub cause: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordered_phase_transitions() {
        let mut s = PhaseState::new(0);
        assert_eq!(s.current, MissionPhase::Setup);
        s.advance(100);
        assert_eq!(s.current, MissionPhase::Buildup);
        s.advance(200);
        assert_eq!(s.current, MissionPhase::Climax);
        s.advance(300);
        assert_eq!(s.current, MissionPhase::Debrief);
        assert!(s.advance(400).is_none(), "debrief is terminal");
    }

    #[test]
    fn deadline_tick_uses_configured_seconds() {
        let mut s = PhaseState::new(0);
        s.setup_seconds = 30.0;
        assert_eq!(s.deadline_tick(60), Some(30 * 60));
    }
}
