//! M7: Mission director v0.5 — 4-phase pacing. M9 extends to a 7-phase
//! reactor-defense pacer.
//!
//! M7 spec § Mission director v0.5 — 4 phases (Setup / Buildup / Climax /
//! Debrief). Each phase carries a min/max dwell window; the director
//! advances when the active phase's exit condition fires (timer + objective
//! completion + reinforcement-wave trigger).
//!
//! M9 spec § Mission director scope at M9 — adds the 7-phase reactor pacer
//! `setup → prep → launch → build_up → sustain_peak → relax → debrief`.
//! Setup → Prep is a single-tick transition; Prep → Launch waits the
//! configured prep window (default 5s @ 60Hz = 300 ticks) so the player
//! can pre-dig before the guard activates; the remaining transitions
//! (BuildUp → SustainPeak → Relax → Debrief) are event-driven and fire
//! when the engine calls [`PhaseState::advance`] in response to reactor
//! pressure crossings + guard death + mission resolution.

use serde::{Deserialize, Serialize};

/// / `Climax` / `Debrief` variants; M9 adds the reactor-defense
/// `Prep` / `Launch` / `BuildUp` / `SustainPeak` / `Relax` variants. The
/// M7 4-phase wire strings (`setup`/`buildup`/`climax`/`debrief`) are
/// preserved; the M9 additions use snake-case names per the M9 spec
/// (`prep`/`launch`/`build_up`/`sustain_peak`/`relax`).
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MissionPhase {
    /// Mission start; players orient + receive briefing.
    #[default]
    Setup,
    /// Early enemy encounters. **M7**.
    Buildup,
    /// Main objective + reinforcement waves. **M7**.
    Climax,
    /// Mission resolution.
    Debrief,
    /// defaults to 5 seconds (300 ticks @ 60Hz).
    Prep,
    Launch,
    /// yet. Renamed from M7's `Buildup` to follow the M9 spec's snake-
    /// case wire string `build_up`.
    #[serde(rename = "build_up")]
    BuildUp,
    SustainPeak,
    /// pressure.
    Relax,
}

impl MissionPhase {
    /// All seven variants in canonical pacing order. Consumers walk this
    /// slice to render UI strips, audit timelines, etc.
    pub const ALL: [MissionPhase; 7] = [
        MissionPhase::Setup,
        MissionPhase::Prep,
        MissionPhase::Launch,
        MissionPhase::BuildUp,
        MissionPhase::SustainPeak,
        MissionPhase::Relax,
        MissionPhase::Debrief,
    ];

    /// with M7 missions that don't ship the M9 reactor-defense pacer).
    pub const M7_PACING: [MissionPhase; 4] = [
        MissionPhase::Setup,
        MissionPhase::Buildup,
        MissionPhase::Climax,
        MissionPhase::Debrief,
    ];

    pub const M9_PACING: [MissionPhase; 7] = [
        MissionPhase::Setup,
        MissionPhase::Prep,
        MissionPhase::Launch,
        MissionPhase::BuildUp,
        MissionPhase::SustainPeak,
        MissionPhase::Relax,
        MissionPhase::Debrief,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            MissionPhase::Setup => "setup",
            MissionPhase::Buildup => "buildup",
            MissionPhase::Climax => "climax",
            MissionPhase::Debrief => "debrief",
            MissionPhase::Prep => "prep",
            MissionPhase::Launch => "launch",
            MissionPhase::BuildUp => "build_up",
            MissionPhase::SustainPeak => "sustain_peak",
            MissionPhase::Relax => "relax",
        }
    }

    pub fn from_str(value: &str) -> Option<MissionPhase> {
        Some(match value {
            "setup" => MissionPhase::Setup,
            "buildup" => MissionPhase::Buildup,
            "climax" => MissionPhase::Climax,
            "debrief" => MissionPhase::Debrief,
            "prep" => MissionPhase::Prep,
            "launch" => MissionPhase::Launch,
            "build_up" => MissionPhase::BuildUp,
            "sustain_peak" => MissionPhase::SustainPeak,
            "relax" => MissionPhase::Relax,
            _ => return None,
        })
    }

    pub fn ordinal(self) -> u8 {
        self as u8
    }

    /// that don't carry a custom `phase_sequence`. Preserved so M7
    /// callers continue to compile + advance through Setup → Buildup →
    /// Climax → Debrief. **M9** extends the table so the 7-phase
    /// reactor-defense pacer can fall back through `next()` even when a
    /// scenario neglects to set `phase_sequence`. The Climax + Relax
    /// arms map to the same successor (Debrief) — keep them separate so
    /// callers can read the legacy 4-phase + reactor 7-phase transitions
    /// off one match.
    #[allow(clippy::match_same_arms)]
    pub fn next(self) -> Option<MissionPhase> {
        match self {
            MissionPhase::Setup => Some(MissionPhase::Buildup),
            MissionPhase::Buildup => Some(MissionPhase::Climax),
            MissionPhase::Climax => Some(MissionPhase::Debrief),
            MissionPhase::Debrief => None,
            MissionPhase::Prep => Some(MissionPhase::Launch),
            MissionPhase::Launch => Some(MissionPhase::BuildUp),
            MissionPhase::BuildUp => Some(MissionPhase::SustainPeak),
            MissionPhase::SustainPeak => Some(MissionPhase::Relax),
            MissionPhase::Relax => Some(MissionPhase::Debrief),
        }
    }
}

/// once per game tick and emits `mission.phase_changed` on transitions
/// (M7 + M9). M9 also emits the distinct `mission.director_phase_change`
/// event so downstream consumers (M10 viewer, M11 HUD) can subscribe to
/// the 7-phase reactor pacer without picking through the 4-phase M7
/// events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhaseState {
    pub current: MissionPhase,
    pub entered_tick: u64,
    /// matching variant.
    pub setup_seconds: f32,
    pub buildup_seconds: f32,
    pub climax_seconds: f32,
    /// True once `current == Debrief`; latched so the director doesn't
    /// emit duplicate transitions.
    pub debrief_entered: bool,
    /// M7 4-phase pacing for back-compat; M9 scenarios override with
    /// [`MissionPhase::M9_PACING`] so the director steps Setup → Prep →
    /// Launch → BuildUp → SustainPeak → Relax → Debrief.
    #[serde(default = "default_m7_phase_sequence")]
    pub phase_sequence: Vec<MissionPhase>,
    /// the `phases_completed` field surfaced by `observe.mission.director`.
    #[serde(default)]
    pub phases_completed: Vec<MissionPhase>,
    /// 60Hz so the guard does not fire before tick ~300.
    #[serde(default = "default_prep_seconds")]
    pub prep_seconds: f32,
    /// pacer flips to BuildUp on the very next tick, matching the spec
    /// table ("Guard spawns. Fire mission.objective_started").
    #[serde(default = "default_launch_seconds")]
    pub launch_seconds: f32,
    /// event-driven (engine calls `advance` when reactor pressure crosses
    /// into Critical).
    #[serde(default)]
    pub build_up_seconds: Option<f32>,
    #[serde(default)]
    pub sustain_peak_seconds: Option<f32>,
    #[serde(default)]
    pub relax_seconds: Option<f32>,
}

fn default_m7_phase_sequence() -> Vec<MissionPhase> {
    MissionPhase::M7_PACING.to_vec()
}

fn default_prep_seconds() -> f32 {
    5.0
}

fn default_launch_seconds() -> f32 {
    1.0 / 60.0
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
            phase_sequence: default_m7_phase_sequence(),
            phases_completed: Vec::new(),
            prep_seconds: default_prep_seconds(),
            launch_seconds: default_launch_seconds(),
            build_up_seconds: None,
            sustain_peak_seconds: None,
            relax_seconds: None,
        }
    }

    /// after 1 tick; Prep → Launch after `prep_seconds` (default 5s);
    /// Launch → BuildUp after `launch_seconds` (default 1 tick); the
    /// remaining transitions are event-driven (engine calls `advance`
    /// when reactor pressure becomes Critical / guard dies / mission
    /// resolves).
    pub fn new_m9_reactor_defense(start_tick: u64) -> Self {
        Self {
            current: MissionPhase::Setup,
            entered_tick: start_tick,
            setup_seconds: 1.0 / 60.0,
            buildup_seconds: 60.0,
            climax_seconds: 120.0,
            debrief_entered: false,
            phase_sequence: MissionPhase::M9_PACING.to_vec(),
            phases_completed: Vec::new(),
            prep_seconds: default_prep_seconds(),
            launch_seconds: default_launch_seconds(),
            build_up_seconds: None,
            sustain_peak_seconds: None,
            relax_seconds: None,
        }
    }

    /// True once the director has walked past the launch phase. The
    /// engine consults this to gate guard AI ticks (per M9 spec § Prep
    /// phase delays guard spawn).
    pub fn is_launch_or_later(&self) -> bool {
        self.ordinal_in_sequence(self.current)
            .zip(self.ordinal_in_sequence(MissionPhase::Launch))
            .map(|(now, launch)| now >= launch)
            .unwrap_or(false)
    }

    fn ordinal_in_sequence(&self, phase: MissionPhase) -> Option<usize> {
        self.phase_sequence.iter().position(|p| *p == phase)
    }

    /// Compute the deadline tick for the current phase given the configured
    /// tick rate. Returns `None` for terminal phases (Debrief) and for
    /// phases with no configured duration (event-driven M9 phases).
    pub fn deadline_tick(&self, tick_rate_hz: u32) -> Option<u64> {
        let seconds = match self.current {
            MissionPhase::Setup => Some(self.setup_seconds),
            MissionPhase::Buildup => Some(self.buildup_seconds),
            MissionPhase::Climax => Some(self.climax_seconds),
            MissionPhase::Debrief => None,
            MissionPhase::Prep => Some(self.prep_seconds),
            MissionPhase::Launch => Some(self.launch_seconds),
            MissionPhase::BuildUp => self.build_up_seconds,
            MissionPhase::SustainPeak => self.sustain_peak_seconds,
            MissionPhase::Relax => self.relax_seconds,
        }?;
        let ticks = (seconds * tick_rate_hz as f32).max(1.0) as u64;
        Some(self.entered_tick.saturating_add(ticks))
    }

    /// back to [`MissionPhase::next`] when the current phase is not in
    /// the configured sequence (M7 back-compat with code that constructs
    /// a `PhaseState` whose sequence does not contain a transient
    /// phase).
    pub fn next_in_sequence(&self) -> Option<MissionPhase> {
        if let Some(idx) = self.ordinal_in_sequence(self.current) {
            return self.phase_sequence.get(idx + 1).copied();
        }
        self.current.next()
    }

    /// Advance to the next phase per the configured `phase_sequence`;
    /// returns the new phase or `None` if already at the terminal phase.
    /// Mutates `phases_completed` to record the now-completed phase.
    pub fn advance(&mut self, tick: u64) -> Option<MissionPhase> {
        let prev = self.current;
        let next = self.next_in_sequence()?;
        self.phases_completed.push(prev);
        self.current = next;
        self.entered_tick = tick;
        if matches!(next, MissionPhase::Debrief) {
            self.debrief_entered = true;
        }
        Some(next)
    }

    /// the `duration_seconds` field on `mission.director_phase_change`.
    pub fn phase_elapsed_seconds(&self, tick: u64, tick_rate_hz: u32) -> f32 {
        if tick_rate_hz == 0 {
            return 0.0;
        }
        let elapsed_ticks = tick.saturating_sub(self.entered_tick) as f32;
        elapsed_ticks / tick_rate_hz as f32
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

/// preserved for M7 back-compat; **M9** emits this AND the new
/// `mission.director_phase_change` event with the additional
/// `duration_seconds` field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhaseChangedEvent {
    pub from: MissionPhase,
    pub to: MissionPhase,
    pub tick: u64,
    pub cause: String,
}

/// transition. Carries the time the previous phase was active so M10's
/// timeline + M11's HUD strip can render dwell-aware pacing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirectorPhaseChangeEvent {
    pub from: MissionPhase,
    pub to: MissionPhase,
    pub tick: u64,
    pub cause: String,
    pub duration_seconds: f32,
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
        assert_eq!(
            s.phases_completed,
            vec![MissionPhase::Setup, MissionPhase::Buildup, MissionPhase::Climax]
        );
    }

    #[test]
    fn deadline_tick_uses_configured_seconds() {
        let mut s = PhaseState::new(0);
        s.setup_seconds = 30.0;
        assert_eq!(s.deadline_tick(60), Some(30 * 60));
    }

    #[test]
    fn m9_phase_sequence_steps_through_seven_phases() {
        let mut s = PhaseState::new_m9_reactor_defense(0);
        assert_eq!(s.current, MissionPhase::Setup);
        assert!(!s.is_launch_or_later());
        s.advance(1);
        assert_eq!(s.current, MissionPhase::Prep);
        assert!(!s.is_launch_or_later());
        s.advance(300);
        assert_eq!(s.current, MissionPhase::Launch);
        assert!(s.is_launch_or_later());
        s.advance(301);
        assert_eq!(s.current, MissionPhase::BuildUp);
        s.advance(500);
        assert_eq!(s.current, MissionPhase::SustainPeak);
        s.advance(800);
        assert_eq!(s.current, MissionPhase::Relax);
        s.advance(1200);
        assert_eq!(s.current, MissionPhase::Debrief);
        assert!(s.advance(1300).is_none(), "debrief is terminal");
    }

    #[test]
    fn m9_prep_deadline_at_tick_300() {
        let mut s = PhaseState::new_m9_reactor_defense(0);
        s.advance(1);
        assert_eq!(s.current, MissionPhase::Prep);
        let deadline = s.deadline_tick(60).expect("prep has a configured deadline");
        assert_eq!(
            deadline,
            1 + 300,
            "prep window is 5s @ 60Hz = 300 ticks past Prep entry"
        );
    }

    #[test]
    fn event_driven_phases_have_no_default_deadline() {
        let mut s = PhaseState::new_m9_reactor_defense(0);
        s.advance(1);
        s.advance(300);
        s.advance(301);
        assert_eq!(s.current, MissionPhase::BuildUp);
        assert!(
            s.deadline_tick(60).is_none(),
            "BuildUp transition is event-driven; no deadline by default"
        );
    }

    #[test]
    fn from_str_round_trip_includes_m9_variants() {
        for phase in MissionPhase::ALL {
            let s = phase.as_str();
            assert_eq!(MissionPhase::from_str(s), Some(phase));
        }
    }
}
