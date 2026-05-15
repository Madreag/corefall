//! Killcam start helpers — entry points cf-control hits when the player
//! dies (regular killcam) or when the boss takes its final blow (slow-mo
//! kill cam).

use crate::playback::{KillcamPhase, KillcamState};

/// Outcome of [`start`] / [`start_slow_mo_kill_cam`].
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum KillcamTrigger {
    /// Killcam was started.
    Started,
    /// Killcam was skipped (settings disabled).
    Skipped,
    /// Killcam was already in flight; new request ignored.
    AlreadyActive,
}

/// Begin a regular 3-second killcam. `enabled` mirrors the accessibility
/// `Settings.killcam_enabled` toggle; when false the helper returns
/// `Skipped` without touching state.
pub fn start(state: &mut KillcamState, killer: u64, victim: u64, enabled: bool) -> KillcamTrigger {
    if !enabled {
        return KillcamTrigger::Skipped;
    }
    if state.phase != KillcamPhase::Idle {
        return KillcamTrigger::AlreadyActive;
    }
    state.phase = KillcamPhase::Recording;
    state.killer_actor_id = Some(killer);
    state.victim_actor_id = Some(victim);
    state.elapsed_ms = 0;
    state.slow_mo_kill_cam = false;
    KillcamTrigger::Started
}

/// Begin the 1.5-second slow-mo cinematic kill cam (boss final blow).
/// `enabled` mirrors `Settings.cinematic_kills`.
pub fn start_slow_mo_kill_cam(state: &mut KillcamState, killer: u64, victim: u64, enabled: bool) -> KillcamTrigger {
    if !enabled {
        return KillcamTrigger::Skipped;
    }
    if state.phase != KillcamPhase::Idle {
        return KillcamTrigger::AlreadyActive;
    }
    state.phase = KillcamPhase::Playing;
    state.killer_actor_id = Some(killer);
    state.victim_actor_id = Some(victim);
    state.elapsed_ms = 0;
    state.slow_mo_kill_cam = true;
    KillcamTrigger::Started
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_disabled_returns_skipped() {
        let mut s = KillcamState::default();
        assert_eq!(start(&mut s, 1, 2, false), KillcamTrigger::Skipped);
        assert_eq!(s.phase, KillcamPhase::Idle);
    }

    #[test]
    fn start_writes_state() {
        let mut s = KillcamState::default();
        assert_eq!(start(&mut s, 1, 2, true), KillcamTrigger::Started);
        assert_eq!(s.phase, KillcamPhase::Recording);
        assert_eq!(s.killer_actor_id, Some(1));
        assert_eq!(s.victim_actor_id, Some(2));
        assert!(!s.slow_mo_kill_cam);
    }

    #[test]
    fn second_start_is_already_active() {
        let mut s = KillcamState::default();
        start(&mut s, 1, 2, true);
        assert_eq!(start(&mut s, 3, 4, true), KillcamTrigger::AlreadyActive);
        assert_eq!(s.killer_actor_id, Some(1));
    }

    #[test]
    fn slow_mo_kill_cam_starts_at_playing() {
        let mut s = KillcamState::default();
        assert_eq!(start_slow_mo_kill_cam(&mut s, 5, 6, true), KillcamTrigger::Started);
        assert_eq!(s.phase, KillcamPhase::Playing);
        assert!(s.slow_mo_kill_cam);
    }

    #[test]
    fn slow_mo_kill_cam_disabled_returns_skipped() {
        let mut s = KillcamState::default();
        assert_eq!(start_slow_mo_kill_cam(&mut s, 5, 6, false), KillcamTrigger::Skipped);
        assert_eq!(s.phase, KillcamPhase::Idle);
    }
}
