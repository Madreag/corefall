//! M6C: Plasma Cannon (M48) — exotic; long range.
//!
//! The plasma cannon trades sustained fire for per-shot heat buildup; a
//! tank-canister coolant slot caps the chamber temperature. Firing while
//! `chamber_heat >= MAX_HEAT_BEFORE_VENT` queues a forced vent cycle
//! during which the cannon cannot fire. Engine consumers drive
//! [`PlasmaChamberState::tick_after_shot`] each fire tick + the optional
//! cooldown tick each idle tick.

use serde::{Deserialize, Serialize};

/// Maximum chamber heat (0..1). Beyond this the chamber forces a vent.
pub const MAX_HEAT_BEFORE_VENT: f32 = 1.0;

/// Per-shot heat increment (0..1 range; reaches 1.0 after ~5 sustained shots).
pub const HEAT_PER_SHOT: f32 = 0.2;

/// Per-tick passive cooldown when not firing (0..1 / second).
pub const PASSIVE_COOLDOWN_PER_S: f32 = 0.15;

/// Forced vent duration (seconds) once `MAX_HEAT_BEFORE_VENT` is reached.
pub const FORCED_VENT_SECONDS: f32 = 3.5;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PlasmaChamberState {
    /// Current chamber heat (0..1).
    pub heat: f32,
    /// Seconds remaining in a forced vent cycle. 0 = not venting.
    pub vent_seconds_remaining: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct PlasmaTickOutcome {
    /// True when the chamber crossed the vent threshold this tick.
    pub forced_vent_started: bool,
    /// True when the forced vent cycle completed this tick.
    pub forced_vent_completed: bool,
    /// True when the cannon was unable to fire (vent in progress).
    pub fire_blocked_venting: bool,
}

impl PlasmaChamberState {
    pub fn new() -> Self {
        Self::default()
    }

    /// True when the cannon can fire (not currently venting + heat below cap).
    pub fn can_fire(&self) -> bool {
        self.vent_seconds_remaining <= 0.0 && self.heat < MAX_HEAT_BEFORE_VENT
    }

    /// Record one chamber-active shot. Returns the outcome flags the
    /// engine should consume.
    pub fn tick_after_shot(&mut self) -> PlasmaTickOutcome {
        let mut out = PlasmaTickOutcome::default();
        if self.vent_seconds_remaining > 0.0 {
            out.fire_blocked_venting = true;
            return out;
        }
        self.heat = (self.heat + HEAT_PER_SHOT).min(2.0);
        if self.heat >= MAX_HEAT_BEFORE_VENT {
            self.vent_seconds_remaining = FORCED_VENT_SECONDS;
            out.forced_vent_started = true;
        }
        out
    }

    /// Idle-tick cooldown. `dt_seconds` = wall-clock seconds elapsed.
    pub fn tick_idle(&mut self, dt_seconds: f32) -> PlasmaTickOutcome {
        let mut out = PlasmaTickOutcome::default();
        let dt = dt_seconds.max(0.0);
        if self.vent_seconds_remaining > 0.0 {
            self.vent_seconds_remaining -= dt;
            if self.vent_seconds_remaining <= 0.0 {
                self.vent_seconds_remaining = 0.0;
                self.heat = 0.0;
                out.forced_vent_completed = true;
            }
            return out;
        }
        self.heat = (self.heat - PASSIVE_COOLDOWN_PER_S * dt).max(0.0);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_chamber_can_fire() {
        let c = PlasmaChamberState::new();
        assert!(c.can_fire());
    }

    #[test]
    fn five_shots_force_vent() {
        let mut c = PlasmaChamberState::new();
        let mut vent_seen = false;
        for _ in 0..5 {
            let out = c.tick_after_shot();
            if out.forced_vent_started {
                vent_seen = true;
            }
        }
        assert!(vent_seen);
        assert!(!c.can_fire());
    }

    #[test]
    fn vent_completes_after_full_duration() {
        let mut c = PlasmaChamberState::new();
        for _ in 0..5 {
            let _ = c.tick_after_shot();
        }
        assert!(!c.can_fire());
        let _ = c.tick_idle(FORCED_VENT_SECONDS + 0.1);
        assert!(c.can_fire());
        assert_eq!(c.heat, 0.0);
    }

    #[test]
    fn fire_blocked_during_vent() {
        let mut c = PlasmaChamberState::new();
        for _ in 0..5 {
            let _ = c.tick_after_shot();
        }
        let out = c.tick_after_shot();
        assert!(out.fire_blocked_venting);
    }

    #[test]
    fn passive_cooldown_reduces_heat_below_vent_threshold() {
        let mut c = PlasmaChamberState::new();
        let _ = c.tick_after_shot();
        let before = c.heat;
        let _ = c.tick_idle(1.0);
        assert!(c.heat < before);
    }
}
