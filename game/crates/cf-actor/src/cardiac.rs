//! **M14H** § `cf-actor::cardiac` — per-actor cardiac-arrest state.
//!
//! Thin actor-side wrapper around [`cf_treatment::CardiacState`]. Stored on
//! the actor's `ActorState` (the engine pulls it back out when servicing
//! `act.player.cpr_round` + `act.player.defib`).

use serde::{Deserialize, Serialize};

pub use cf_treatment::{
    CardiacEvent, CardiacOutcome, CardiacState, CardiacTrigger,
    CARDIAC_ARREST_GRACE_SECONDS, CPR_ROUND_DURATION_SECONDS, DEFIB_BASE_SUCCESS,
    DEFIB_CPR_BOOST_PER_ROUND, DEFIB_CHARGES_DEFAULT,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorCardiacComponent {
    /// True if the actor is currently in cardiac arrest.
    pub in_arrest: bool,
    /// Onset tick (set when `in_arrest` flips to true).
    pub onset_tick: u64,
    /// Total CPR rounds applied since arrest.
    pub cpr_rounds_total: u32,
    /// Consecutive CPR rounds since the last defib attempt.
    pub consecutive_cpr_rounds: u32,
    /// Defib charges remaining in the active pack.
    pub charges_remaining: u32,
    /// Cumulative defib shocks delivered.
    pub defib_shocks: u32,
    /// True once a CPR round caused a chest bruise.
    pub chest_bruised: bool,
}

impl Default for ActorCardiacComponent {
    fn default() -> Self {
        Self {
            in_arrest: false,
            onset_tick: 0,
            cpr_rounds_total: 0,
            consecutive_cpr_rounds: 0,
            charges_remaining: DEFIB_CHARGES_DEFAULT,
            defib_shocks: 0,
            chest_bruised: false,
        }
    }
}

impl ActorCardiacComponent {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enter_arrest(&mut self, tick: u64) {
        self.in_arrest = true;
        self.onset_tick = tick;
        self.cpr_rounds_total = 0;
        self.consecutive_cpr_rounds = 0;
    }

    pub fn clear(&mut self) {
        self.in_arrest = false;
        self.cpr_rounds_total = 0;
        self.consecutive_cpr_rounds = 0;
    }

    pub fn apply_cpr_round(&mut self) {
        self.cpr_rounds_total = self.cpr_rounds_total.saturating_add(1);
        self.consecutive_cpr_rounds = self.consecutive_cpr_rounds.saturating_add(1);
        if self.cpr_rounds_total >= 3 {
            self.chest_bruised = true;
        }
    }

    pub fn consume_defib_charge(&mut self) -> bool {
        if self.charges_remaining == 0 {
            return false;
        }
        self.charges_remaining -= 1;
        self.defib_shocks = self.defib_shocks.saturating_add(1);
        // Defib resets the consecutive-CPR streak — fresh CPR required for
        // the next shock's boost.
        self.consecutive_cpr_rounds = 0;
        true
    }

    /// Compute the current defib success probability (×1000) given the
    /// consecutive CPR rounds preceding this attempt.
    pub fn defib_success_probability_x1000(&self) -> u32 {
        let p = DEFIB_BASE_SUCCESS
            + self.consecutive_cpr_rounds as f32 * DEFIB_CPR_BOOST_PER_ROUND;
        (p.clamp(0.0, 1.0) * 1000.0).round() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enter_clear_reset() {
        let mut c = ActorCardiacComponent::new();
        c.enter_arrest(100);
        assert!(c.in_arrest);
        c.clear();
        assert!(!c.in_arrest);
        assert_eq!(c.cpr_rounds_total, 0);
    }

    #[test]
    fn cpr_round_bruises_after_3() {
        let mut c = ActorCardiacComponent::new();
        c.apply_cpr_round();
        c.apply_cpr_round();
        assert!(!c.chest_bruised);
        c.apply_cpr_round();
        assert!(c.chest_bruised);
    }

    /// success probability.
    #[test]
    fn defib_success_70_after_2_cpr() {
        let mut c = ActorCardiacComponent::new();
        c.apply_cpr_round();
        c.apply_cpr_round();
        assert_eq!(c.defib_success_probability_x1000(), 700);
    }

    #[test]
    fn consume_charge_resets_consecutive() {
        let mut c = ActorCardiacComponent::new();
        c.apply_cpr_round();
        c.apply_cpr_round();
        assert_eq!(c.consecutive_cpr_rounds, 2);
        assert!(c.consume_defib_charge());
        assert_eq!(c.consecutive_cpr_rounds, 0);
        assert_eq!(c.charges_remaining, DEFIB_CHARGES_DEFAULT - 1);
    }
}
