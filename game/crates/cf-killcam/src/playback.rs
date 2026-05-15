//! Killcam state + tick. The state machine progresses from Idle → Recording
//! → Playing → Done; each phase carries the killer + victim ids and the
//! elapsed playback time.

use serde::{Deserialize, Serialize};

/// Spec-mandated total killcam duration in ms.
pub const KILLCAM_DURATION_MS: u32 = 3000;
/// Spec-mandated slow-mo kill cam duration in ms (boss final blow).
pub const SLOW_MO_KILL_CAM_DURATION_MS: u32 = 1500;

/// Phases of the killcam state machine.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum KillcamPhase {
    /// No killcam in flight.
    #[default]
    Idle,
    /// Recording the moments leading up to the kill.
    Recording,
    /// Playing back the recorded sequence in slow motion.
    Playing,
    /// Playback finished; cf-control will reset to Idle next tick.
    Done,
}

impl KillcamPhase {
    /// Canonical snake_case identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            KillcamPhase::Idle => "idle",
            KillcamPhase::Recording => "recording",
            KillcamPhase::Playing => "playing",
            KillcamPhase::Done => "done",
        }
    }
}

/// Killcam state. cf-control owns one instance per session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct KillcamState {
    /// Current phase.
    pub phase: KillcamPhase,
    /// Killer actor id (None when no killcam in flight).
    pub killer_actor_id: Option<u64>,
    /// Victim actor id (None when no killcam in flight).
    pub victim_actor_id: Option<u64>,
    /// Elapsed time in current phase (ms).
    pub elapsed_ms: u32,
    /// Whether this killcam is the slow-mo cinematic variant (boss final).
    pub slow_mo_kill_cam: bool,
}

impl KillcamState {
    /// Total duration for the active variant in ms.
    pub fn total_ms(&self) -> u32 {
        if self.slow_mo_kill_cam {
            SLOW_MO_KILL_CAM_DURATION_MS
        } else {
            KILLCAM_DURATION_MS
        }
    }

    /// Reset to idle. Used by cf-control to clear after Done.
    pub fn reset(&mut self) {
        self.phase = KillcamPhase::Idle;
        self.killer_actor_id = None;
        self.victim_actor_id = None;
        self.elapsed_ms = 0;
        self.slow_mo_kill_cam = false;
    }
}

/// Advance the killcam by `dt_ms`. Returns the new phase.
pub fn tick(state: &mut KillcamState, dt_ms: u32) -> KillcamPhase {
    state.elapsed_ms = state.elapsed_ms.saturating_add(dt_ms);
    let total = state.total_ms();
    state.phase = match state.phase {
        KillcamPhase::Idle => KillcamPhase::Idle,
        KillcamPhase::Recording => {
            if state.elapsed_ms < 200 {
                KillcamPhase::Recording
            } else {
                state.elapsed_ms = 0;
                KillcamPhase::Playing
            }
        }
        KillcamPhase::Playing => {
            if state.elapsed_ms < total {
                KillcamPhase::Playing
            } else {
                KillcamPhase::Done
            }
        }
        KillcamPhase::Done => KillcamPhase::Done,
    };
    state.phase
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_stays_idle() {
        let mut s = KillcamState::default();
        let p = tick(&mut s, 100);
        assert_eq!(p, KillcamPhase::Idle);
    }

    #[test]
    fn recording_advances_to_playing_after_200ms() {
        let mut s = KillcamState {
            phase: KillcamPhase::Recording,
            killer_actor_id: Some(1),
            victim_actor_id: Some(2),
            ..KillcamState::default()
        };
        assert_eq!(tick(&mut s, 100), KillcamPhase::Recording);
        assert_eq!(tick(&mut s, 150), KillcamPhase::Playing);
    }

    #[test]
    fn playing_completes_after_3s() {
        let mut s = KillcamState {
            phase: KillcamPhase::Playing,
            killer_actor_id: Some(1),
            victim_actor_id: Some(2),
            ..KillcamState::default()
        };
        let mut last = KillcamPhase::Playing;
        for _ in 0..200 {
            last = tick(&mut s, 16);
            if last == KillcamPhase::Done {
                break;
            }
        }
        assert_eq!(last, KillcamPhase::Done);
    }

    #[test]
    fn slow_mo_kill_cam_uses_1500ms() {
        let mut s = KillcamState {
            phase: KillcamPhase::Playing,
            killer_actor_id: Some(7),
            victim_actor_id: Some(8),
            slow_mo_kill_cam: true,
            ..KillcamState::default()
        };
        assert_eq!(s.total_ms(), SLOW_MO_KILL_CAM_DURATION_MS);
        assert_eq!(tick(&mut s, 1500), KillcamPhase::Done);
    }

    #[test]
    fn reset_clears_state() {
        let mut s = KillcamState {
            phase: KillcamPhase::Playing,
            killer_actor_id: Some(1),
            victim_actor_id: Some(2),
            elapsed_ms: 1000,
            slow_mo_kill_cam: true,
        };
        s.reset();
        assert_eq!(s.phase, KillcamPhase::Idle);
        assert_eq!(s.killer_actor_id, None);
        assert_eq!(s.victim_actor_id, None);
        assert_eq!(s.elapsed_ms, 0);
        assert!(!s.slow_mo_kill_cam);
    }
}
