//! **M14D** — C-RAM (Counter-Rocket Artillery Mortar) cooldown latch.
//!
//! Spec § Player-facing: "C-RAM (Counter-Rocket Artillery Mortar) installations
//! defend bases by shooting down incoming projectiles per this contract."
//! Gherkin scenario 4: "And the C-RAM cooldown begins" — every successful
//! APS intercept (the `collision.projectile_pair_contact{outcome="aps_intercept"}`
//! event surfaced by `cf-physics::projectile`) latches the firing C-RAM unit
//! into a cooldown for [`Cram::cooldown_duration_ticks`] simulation ticks.
//!
//! The cf-control engine owns a `BTreeMap<u64, Cram>` keyed by the APS
//! laser's `owner_actor_id` so each base-mounted C-RAM unit (or per-actor
//! APS) tracks its own cooldown deterministically. The struct itself is
//! pure / [`Copy`] so it can be snapshot-copied across engine state-lock
//! boundaries without allocation.

use serde::{Deserialize, Serialize};

/// Default cooldown duration for the canonical C-RAM unit: 60 ticks
/// (1 second at the 60 Hz canonical tick rate). Pinned by the M14D
/// spec's Player-facing line and the runtime-evidence test
/// `val_m14d_006_runtime_aps_intercept`.
pub const DEFAULT_CRAM_COOLDOWN_TICKS: u32 = 60;

/// C-RAM cooldown latch surface. Callers (`cf-control::engine`,
/// integration tests) snapshot-read `cooldown_active` to decide whether
/// the unit may engage another pair contact this tick.
///
/// Cooldown lifecycle:
///   1. Initial state: `cooldown_active = false`, `cooldown_ticks_remaining = 0`.
///   2. APS intercept fires: [`Cram::engage_cooldown`] flips
///      `cooldown_active` false → true and sets
///      `cooldown_ticks_remaining = cooldown_duration_ticks`.
///   3. Each subsequent tick: [`Cram::tick`] decrements
///      `cooldown_ticks_remaining`; on reaching zero
///      `cooldown_active` flips true → false.
#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cram {
    pub cooldown_active: bool,
    pub cooldown_ticks_remaining: u32,
    pub cooldown_duration_ticks: u32,
}

impl Default for Cram {
    fn default() -> Self {
        Self {
            cooldown_active: false,
            cooldown_ticks_remaining: 0,
            cooldown_duration_ticks: DEFAULT_CRAM_COOLDOWN_TICKS,
        }
    }
}

impl Cram {
    /// Construct a C-RAM with the given cooldown duration (in ticks).
    /// The unit starts idle (`cooldown_active == false`).
    pub fn new(cooldown_duration_ticks: u32) -> Self {
        Self {
            cooldown_active: false,
            cooldown_ticks_remaining: 0,
            cooldown_duration_ticks,
        }
    }

    /// Latch the cooldown for one APS intercept. Sets
    /// `cooldown_active = true` and `cooldown_ticks_remaining =
    /// cooldown_duration_ticks` (re-latches if already active so back-
    /// to-back intercepts extend the cooldown rather than truncating it).
    pub fn engage_cooldown(&mut self) {
        self.cooldown_active = true;
        self.cooldown_ticks_remaining = self.cooldown_duration_ticks;
    }

    /// Advance the cooldown by one simulation tick. When
    /// `cooldown_ticks_remaining` reaches zero the latch returns to the
    /// idle state. Safe to call when the unit is already idle (no-op).
    pub fn tick(&mut self) {
        if self.cooldown_ticks_remaining > 0 {
            self.cooldown_ticks_remaining -= 1;
            if self.cooldown_ticks_remaining == 0 {
                self.cooldown_active = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_idle_with_canonical_duration() {
        let c = Cram::default();
        assert!(!c.cooldown_active);
        assert_eq!(c.cooldown_ticks_remaining, 0);
        assert_eq!(c.cooldown_duration_ticks, DEFAULT_CRAM_COOLDOWN_TICKS);
    }

    #[test]
    fn engage_flips_active_and_seeds_remaining() {
        let mut c = Cram::default();
        c.engage_cooldown();
        assert!(c.cooldown_active);
        assert_eq!(c.cooldown_ticks_remaining, DEFAULT_CRAM_COOLDOWN_TICKS);
    }

    #[test]
    fn tick_decays_and_clears_latch_after_duration() {
        let mut c = Cram::new(3);
        c.engage_cooldown();
        assert!(c.cooldown_active);
        assert_eq!(c.cooldown_ticks_remaining, 3);
        c.tick();
        assert!(c.cooldown_active);
        assert_eq!(c.cooldown_ticks_remaining, 2);
        c.tick();
        assert!(c.cooldown_active);
        assert_eq!(c.cooldown_ticks_remaining, 1);
        c.tick();
        assert!(!c.cooldown_active);
        assert_eq!(c.cooldown_ticks_remaining, 0);
    }

    #[test]
    fn tick_when_idle_is_no_op() {
        let mut c = Cram::default();
        c.tick();
        assert!(!c.cooldown_active);
        assert_eq!(c.cooldown_ticks_remaining, 0);
    }

    #[test]
    fn re_engaging_resets_remaining_to_full_duration() {
        let mut c = Cram::new(5);
        c.engage_cooldown();
        c.tick();
        c.tick();
        assert_eq!(c.cooldown_ticks_remaining, 3);
        c.engage_cooldown();
        assert!(c.cooldown_active);
        assert_eq!(c.cooldown_ticks_remaining, 5);
    }
}
