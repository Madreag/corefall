//! M6: weapon swap timing (300 ms standard, 200 ms pistol).
//!
//! Per spec § "Weapon swap 300ms transition" + "Pistol faster swap (200ms vs
//! 300ms)". The cf-control engine tracks per-actor swap progress and gates
//! fire/reload while a swap is in flight.

use serde::{Deserialize, Serialize};

/// Standard swap duration (seconds).
pub const WEAPON_SWAP_SECONDS: f32 = 0.3;
/// Pistol swap duration (seconds).
pub const PISTOL_SWAP_SECONDS: f32 = 0.2;

/// Per-actor swap state.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WeaponSwap {
    /// Inflight or 0 when no swap is active.
    pub remaining_seconds: f32,
    /// Source slot (so the engine can hide the source weapon during swap).
    pub source_slot: u8,
    /// Target slot.
    pub target_slot: u8,
    /// Total swap duration (300 ms or 200 ms).
    pub total_seconds: f32,
}

impl Default for WeaponSwap {
    fn default() -> Self {
        Self {
            remaining_seconds: 0.0,
            source_slot: 0,
            target_slot: 0,
            total_seconds: WEAPON_SWAP_SECONDS,
        }
    }
}

impl WeaponSwap {
    pub fn start(source_slot: u8, target_slot: u8, duration_seconds: f32) -> Self {
        let duration = if duration_seconds.is_finite() && duration_seconds > 0.0 {
            duration_seconds
        } else {
            WEAPON_SWAP_SECONDS
        };
        Self {
            remaining_seconds: duration,
            source_slot,
            target_slot,
            total_seconds: duration,
        }
    }

    pub fn in_progress(&self) -> bool {
        self.remaining_seconds > 0.0
    }

    /// Advance the swap one tick. Returns true when the swap just completed.
    pub fn tick(&mut self, tick_rate_hz: u32) -> bool {
        if !self.in_progress() {
            return false;
        }
        let rate = tick_rate_hz.max(1) as f32;
        self.remaining_seconds -= 1.0 / rate;
        if self.remaining_seconds <= 0.0 {
            self.remaining_seconds = 0.0;
            return true;
        }
        false
    }

    /// Returns the swap progress 0..1.
    pub fn progress(self) -> f32 {
        if self.total_seconds <= 0.0 {
            return 1.0;
        }
        (1.0 - (self.remaining_seconds / self.total_seconds)).clamp(0.0, 1.0)
    }
}

/// Pick the swap duration for a target slot. Slot 3 (sidearm) is the pistol
/// per spec § slot layout (`sidearm`). Other slots use the standard duration.
pub fn swap_duration_for_target(target_slot: u8) -> f32 {
    if target_slot == 2 || target_slot == 3 {
        PISTOL_SWAP_SECONDS
    } else {
        WEAPON_SWAP_SECONDS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_completes_in_300ms_at_60hz() {
        let mut s = WeaponSwap::start(0, 1, WEAPON_SWAP_SECONDS);
        let mut completed = false;
        for _ in 0..20 {
            if s.tick(60) {
                completed = true;
                break;
            }
        }
        assert!(completed);
    }

    #[test]
    fn pistol_completes_faster() {
        let mut a = WeaponSwap::start(0, 1, WEAPON_SWAP_SECONDS);
        let mut b = WeaponSwap::start(0, 3, PISTOL_SWAP_SECONDS);
        let mut a_done = 0;
        let mut b_done = 0;
        for n in 1..30 {
            if a.tick(60) {
                a_done = n;
                break;
            }
        }
        for n in 1..30 {
            if b.tick(60) {
                b_done = n;
                break;
            }
        }
        assert!(b_done < a_done, "pistol={b_done} standard={a_done}");
    }

    #[test]
    fn progress_grows_with_ticks() {
        let mut s = WeaponSwap::start(0, 1, WEAPON_SWAP_SECONDS);
        let p0 = s.progress();
        for _ in 0..5 {
            s.tick(60);
        }
        let p1 = s.progress();
        assert!(p1 > p0);
    }

    #[test]
    fn nan_duration_falls_back_to_standard() {
        let s = WeaponSwap::start(0, 1, f32::NAN);
        assert!((s.total_seconds - WEAPON_SWAP_SECONDS).abs() < 1e-3);
    }

    #[test]
    fn sidearm_slot_gets_pistol_duration() {
        assert_eq!(swap_duration_for_target(3), PISTOL_SWAP_SECONDS);
        assert_eq!(swap_duration_for_target(0), WEAPON_SWAP_SECONDS);
    }
}
