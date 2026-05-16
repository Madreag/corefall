//! Hit-stop pulse — the 50-200ms frame-pause cosmetic per spec.

use crate::{CameraState, DEFAULT_HIT_STOP_MS, MAX_HIT_STOP_MS, MIN_HIT_STOP_MS};

/// Trigger a hit-stop pulse for `duration_ms`. Clamped to the spec's
/// 50-200ms band; values outside the band are clamped (a 0 input rounds
/// up to MIN_HIT_STOP_MS so the surface always produces a perceivable
/// pause). When the existing `hit_stop_remaining_ms` exceeds the supplied
/// duration, the pulse is left untouched (longer pulses dominate).
pub fn trigger_hit_stop(state: &mut CameraState, duration_ms: u32) {
    let new_pulse = if duration_ms == 0 {
        DEFAULT_HIT_STOP_MS
    } else {
        duration_ms.clamp(MIN_HIT_STOP_MS, MAX_HIT_STOP_MS)
    };
    if new_pulse > state.hit_stop_remaining_ms {
        state.hit_stop_remaining_ms = new_pulse;
    }
}

/// Decay the hit-stop pulse one frame. Returns true if the camera is
/// currently freezing the sim time-step (rendering still ticks). Always
/// false when accessibility setting `hit_stop_enabled` is off — gating is
/// the engine's responsibility, not this helper's.
pub fn tick_hit_stop(state: &mut CameraState, dt_ms: u32) -> bool {
    if state.hit_stop_remaining_ms == 0 {
        return false;
    }
    state.hit_stop_remaining_ms = state.hit_stop_remaining_ms.saturating_sub(dt_ms);
    state.hit_stop_remaining_ms > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_clamps_below_min() {
        let mut s = CameraState::default();
        trigger_hit_stop(&mut s, 10);
        assert_eq!(s.hit_stop_remaining_ms, MIN_HIT_STOP_MS);
    }

    #[test]
    fn trigger_clamps_above_max() {
        let mut s = CameraState::default();
        trigger_hit_stop(&mut s, 5000);
        assert_eq!(s.hit_stop_remaining_ms, MAX_HIT_STOP_MS);
    }

    #[test]
    fn trigger_zero_uses_default() {
        let mut s = CameraState::default();
        trigger_hit_stop(&mut s, 0);
        assert_eq!(s.hit_stop_remaining_ms, DEFAULT_HIT_STOP_MS);
    }

    #[test]
    fn longer_pulse_dominates_shorter() {
        let mut s = CameraState::default();
        trigger_hit_stop(&mut s, 200);
        trigger_hit_stop(&mut s, 60);
        assert_eq!(s.hit_stop_remaining_ms, 200);
    }

    #[test]
    fn tick_hit_stop_decays_to_zero() {
        let mut s = CameraState::default();
        trigger_hit_stop(&mut s, 100);
        assert!(tick_hit_stop(&mut s, 50));
        assert!(!tick_hit_stop(&mut s, 200));
        assert_eq!(s.hit_stop_remaining_ms, 0);
    }
}
