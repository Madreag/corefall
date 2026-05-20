//! **M14H** § Cardiac-arrest + CPR/defib loop.
//!
//! When an actor enters cardiac arrest (triggered by `affliction.shocked`
//! + heart organ at <30% OR M16C anxiety-acute-arrest), the M11 Triage
//! Window switches to "CARDIAC ARREST — 30s window" (spec § Defibrillator
//! + CPR loop).
//!
//! Tunable defaults (locked by spec):
//! - CPR round duration: 20s
//! - Cardiac arrest grace window: 100s (5 CPR rounds)
//! - Defibrillator base success: 50%
//! - Defibrillator boost per consecutive CPR round: +10%
//! - Defibrillator charges per pack: 4

use rand_core::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256StarStar;
use serde::{Deserialize, Serialize};

pub const CPR_ROUND_DURATION_SECONDS: f32 = 20.0;
pub const CARDIAC_ARREST_GRACE_SECONDS: f32 = 100.0;
pub const DEFIB_BASE_SUCCESS: f32 = 0.50;
pub const DEFIB_CPR_BOOST_PER_ROUND: f32 = 0.10;
pub const DEFIB_CHARGES_DEFAULT: u32 = 4;
pub const CPR_BRUISE_THRESHOLD_ROUNDS: u32 = 3;
pub const CPR_ROUNDS_MAX: u32 = 5;
/// **M14H** § spec table § "5s per shock; 8s recharge". The defib
/// dispatcher rejects shots fired within `DEFIB_RECHARGE_SECONDS` of the
/// last shock with `out_of_charges` (until 8s elapse).
pub const DEFIB_RECHARGE_SECONDS: f32 = 8.0;

/// **M14H** § cardiac arrest trigger surface.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardiacTrigger {
    /// `affliction.shocked` + heart organ < 30%.
    ShockedHeartCrit,
    /// M16C anxiety-acute-arrest.
    AnxietyAcuteArrest,
    /// Manual / scripted trigger for tests + mission events.
    Manual,
}

impl CardiacTrigger {
    pub fn as_str(self) -> &'static str {
        match self {
            CardiacTrigger::ShockedHeartCrit => "shocked_heart_crit",
            CardiacTrigger::AnxietyAcuteArrest => "anxiety_acute_arrest",
            CardiacTrigger::Manual => "manual",
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardiacOutcome {
    Active,
    Restored,
    Expired,
}

impl CardiacOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            CardiacOutcome::Active => "active",
            CardiacOutcome::Restored => "restored",
            CardiacOutcome::Expired => "expired",
        }
    }
}

/// **M14H** § cardiac event stream entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "event")]
pub enum CardiacEvent {
    Arrested {
        actor_id: u64,
        tick: u64,
        trigger: CardiacTrigger,
        grace_seconds: f32,
    },
    CprRound {
        actor_id: u64,
        tick: u64,
        round_index: u32,
        consecutive_cpr_rounds: u32,
    },
    DefibAttempted {
        actor_id: u64,
        tick: u64,
        success_probability_x1000: u32,
        roll_x1000: u32,
        passed: bool,
        charges_remaining: u32,
    },
    Restored {
        actor_id: u64,
        tick: u64,
    },
    Expired {
        actor_id: u64,
        tick: u64,
    },
}

/// **M14H** § cardiac arrest state machine.
#[derive(Debug, Clone, PartialEq)]
pub struct CardiacState {
    pub actor_id: u64,
    pub outcome: CardiacOutcome,
    pub trigger: CardiacTrigger,
    pub onset_tick: u64,
    pub seconds_elapsed: f32,
    pub grace_seconds: f32,
    /// Number of CPR rounds completed (any time).
    pub cpr_rounds_total: u32,
    /// Consecutive CPR rounds since the last defib attempt — drives the
    /// per-shock success-roll boost.
    pub consecutive_cpr_rounds: u32,
    /// Number of defib shocks delivered.
    pub defib_shocks: u32,
    /// Remaining defib charges in the active pack.
    pub charges_remaining: u32,
    /// Per-shock chest burn count (BurnAtChestPerShock → emits one Burn1st
    /// per shock).
    pub chest_burns: u32,
    /// True once a CPR round caused a Bruise on the chest (after 3+ rounds).
    pub chest_bruised: bool,
    rng: Xoshiro256StarStar,
}

impl CardiacState {
    pub fn new(actor_id: u64, onset_tick: u64, trigger: CardiacTrigger, seed: u64) -> Self {
        Self {
            actor_id,
            outcome: CardiacOutcome::Active,
            trigger,
            onset_tick,
            seconds_elapsed: 0.0,
            grace_seconds: CARDIAC_ARREST_GRACE_SECONDS,
            cpr_rounds_total: 0,
            consecutive_cpr_rounds: 0,
            defib_shocks: 0,
            charges_remaining: DEFIB_CHARGES_DEFAULT,
            chest_burns: 0,
            chest_bruised: false,
            rng: Xoshiro256StarStar::seed_from_u64(seed),
        }
    }

    pub fn arrest_event(&self, tick: u64) -> CardiacEvent {
        CardiacEvent::Arrested {
            actor_id: self.actor_id,
            tick,
            trigger: self.trigger,
            grace_seconds: self.grace_seconds,
        }
    }

    /// Advance simulated time. If the grace window expires while the
    /// outcome is still `Active`, transitions to `Expired` and emits
    /// `cardiac.expired`.
    pub fn tick(&mut self, dt_seconds: f32, sim_tick: u64) -> Vec<CardiacEvent> {
        let mut events = Vec::new();
        if !matches!(self.outcome, CardiacOutcome::Active) {
            return events;
        }
        self.seconds_elapsed += dt_seconds;
        if self.seconds_elapsed >= self.grace_seconds {
            self.outcome = CardiacOutcome::Expired;
            events.push(CardiacEvent::Expired {
                actor_id: self.actor_id,
                tick: sim_tick,
            });
        }
        events
    }

    /// Apply one CPR round (20s of compressions). Returns the cpr_round
    /// event the engine should record. After 3 CPR rounds the chest takes
    /// a BruiseLight wound (`chest_bruised` flips to true on first cross).
    pub fn cpr_round(&mut self, sim_tick: u64) -> CardiacEvent {
        // The CPR round itself buys time; the spec says "buys 100s" total
        // across 5 rounds (5 × 20s). The grace window is already 100s, so
        // CPR doesn't extend it — it preserves perfusion so the actor
        // doesn't transition to Expired even after the grace window
        // elapses, AND it boosts the next defib roll. We model this as:
        // each CPR round adds 20s to the grace window (capped at 5 rounds).
        if self.cpr_rounds_total < CPR_ROUNDS_MAX {
            self.grace_seconds += CPR_ROUND_DURATION_SECONDS;
        }
        self.cpr_rounds_total += 1;
        self.consecutive_cpr_rounds += 1;
        if self.cpr_rounds_total >= CPR_BRUISE_THRESHOLD_ROUNDS {
            self.chest_bruised = true;
        }
        CardiacEvent::CprRound {
            actor_id: self.actor_id,
            tick: sim_tick,
            round_index: self.cpr_rounds_total,
            consecutive_cpr_rounds: self.consecutive_cpr_rounds,
        }
    }

    /// Attempt a defibrillator shock. Consumes one charge if any are left.
    /// Success roll = 50% baseline + 10% per consecutive CPR round
    /// preceding the shock. Returns the `defib_attempted` event and (if
    /// success) follows up with a `restored` event in a second call to
    /// [`CardiacState::tick_resolve_defib`]. The defib roll itself is
    /// computed in-line and returned via the event.
    pub fn defib_attempt(&mut self, sim_tick: u64) -> Vec<CardiacEvent> {
        let mut events = Vec::new();
        if !matches!(self.outcome, CardiacOutcome::Active) {
            return events;
        }
        if self.charges_remaining == 0 {
            // Out of charges; event recorder uses passed=false and
            // success_probability=0 to surface the failure.
            events.push(CardiacEvent::DefibAttempted {
                actor_id: self.actor_id,
                tick: sim_tick,
                success_probability_x1000: 0,
                roll_x1000: 0,
                passed: false,
                charges_remaining: 0,
            });
            return events;
        }
        self.charges_remaining -= 1;
        self.defib_shocks += 1;
        self.chest_burns += 1;
        let p = (DEFIB_BASE_SUCCESS
            + self.consecutive_cpr_rounds as f32 * DEFIB_CPR_BOOST_PER_ROUND)
            .clamp(0.0, 1.0);
        let p_x1000 = (p * 1000.0).round() as u32;
        let roll = (self.rng.next_u64() % 1000) as u32;
        let passed = roll < p_x1000;
        events.push(CardiacEvent::DefibAttempted {
            actor_id: self.actor_id,
            tick: sim_tick,
            success_probability_x1000: p_x1000,
            roll_x1000: roll,
            passed,
            charges_remaining: self.charges_remaining,
        });
        if passed {
            self.outcome = CardiacOutcome::Restored;
            events.push(CardiacEvent::Restored {
                actor_id: self.actor_id,
                tick: sim_tick,
            });
        } else {
            // Reset consecutive CPR; next defib needs fresh CPR support.
            self.consecutive_cpr_rounds = 0;
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **M14H** Gherkin scenario 2: Defibrillator restores rhythm after CPR.
    /// Given actor in cardiac arrest, when 2 cardiac.cpr_round fire (40s)
    /// + act.player.defib fires → cardiac.defib_attempted fires AND the
    /// success roll is 50% + 20% (2 CPR rounds) = 70%.
    #[test]
    fn defib_after_two_cpr_rounds_at_70pct() {
        let mut c = CardiacState::new(7, 0, CardiacTrigger::Manual, 0);
        let _ev1 = c.cpr_round(100);
        let _ev2 = c.cpr_round(120);
        assert_eq!(c.consecutive_cpr_rounds, 2);
        let evs = c.defib_attempt(130);
        let attempted = evs
            .iter()
            .find_map(|e| match e {
                CardiacEvent::DefibAttempted {
                    success_probability_x1000,
                    ..
                } => Some(*success_probability_x1000),
                _ => None,
            })
            .expect("defib_attempted event");
        // 0.50 + 2 × 0.10 = 0.70.
        assert_eq!(attempted, 700);
    }

    /// Determinism — same seed reproduces defib outcome.
    #[test]
    fn defib_determinism_same_seed() {
        let mut a = CardiacState::new(1, 0, CardiacTrigger::Manual, 42);
        let mut b = CardiacState::new(1, 0, CardiacTrigger::Manual, 42);
        a.cpr_round(10);
        b.cpr_round(10);
        let ea = a.defib_attempt(20);
        let eb = b.defib_attempt(20);
        assert_eq!(ea, eb);
    }

    /// Out of charges — defib reports passed=false and charges_remaining=0.
    #[test]
    fn defib_out_of_charges_rejected() {
        let mut c = CardiacState::new(1, 0, CardiacTrigger::Manual, 0);
        c.charges_remaining = 0;
        let evs = c.defib_attempt(10);
        let attempted = evs
            .iter()
            .find_map(|e| match e {
                CardiacEvent::DefibAttempted {
                    passed,
                    charges_remaining,
                    ..
                } => Some((*passed, *charges_remaining)),
                _ => None,
            })
            .unwrap();
        assert!(!attempted.0);
        assert_eq!(attempted.1, 0);
    }

    /// Grace window expires → Expired event.
    #[test]
    fn grace_window_expires_to_expired() {
        let mut c = CardiacState::new(1, 0, CardiacTrigger::Manual, 0);
        let mut last = None;
        for t in 0..=200u64 {
            for e in c.tick(1.0, t) {
                last = Some(e);
            }
            if !matches!(c.outcome, CardiacOutcome::Active) {
                break;
            }
        }
        assert_eq!(c.outcome, CardiacOutcome::Expired);
        assert!(matches!(last, Some(CardiacEvent::Expired { .. })));
    }

    /// CPR extends grace window — 5 rounds buys 100s extra perfusion.
    #[test]
    fn cpr_round_extends_grace_window() {
        let mut c = CardiacState::new(1, 0, CardiacTrigger::Manual, 0);
        let g0 = c.grace_seconds;
        for _ in 0..5 {
            c.cpr_round(0);
        }
        assert!(
            (c.grace_seconds - (g0 + 5.0 * CPR_ROUND_DURATION_SECONDS)).abs() < 1e-3,
            "expected grace+100s, got {} (g0={g0})",
            c.grace_seconds
        );
        // 6th round caps — grace doesn't increase further.
        c.cpr_round(0);
        assert!(
            (c.grace_seconds - (g0 + 5.0 * CPR_ROUND_DURATION_SECONDS)).abs() < 1e-3,
            "CPR beyond max should not extend grace further"
        );
    }

    /// 3 CPR rounds → chest_bruised flag.
    #[test]
    fn cpr_three_rounds_bruises_chest() {
        let mut c = CardiacState::new(1, 0, CardiacTrigger::Manual, 0);
        assert!(!c.chest_bruised);
        c.cpr_round(0);
        c.cpr_round(0);
        assert!(!c.chest_bruised);
        c.cpr_round(0);
        assert!(c.chest_bruised);
    }
}
