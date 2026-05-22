//! **M14I** — Biological aging clock + per-year degradation curve.
//!
//! Canonical owner of:
//! - [`BiologicalAge`] — per-actor age + per-cycle stat-degradation
//!   accumulator.
//! - [`AgingOrigin`] — locked enum of "is this origin biological?"
//!   answers. Robots / crystallines skip aging entirely.
//! - [`AgingTickResult`] — typed result of a per-tick aging pass
//!   (year-advanced flag, retirement flag, terminal flag).
//! - [`AgingEvent`] — replay event variants the engine surfaces
//!   downstream.
//!
//! Determinism: the terminal-roll RNG flows through a single seeded
//! `rand_xoshiro::Xoshiro256StarStar`. Identical seed + identical input
//! reproduces identical outcomes per the M14I Gherkin determinism
//! scenario.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::items_after_statements,
    clippy::similar_names,
    clippy::manual_range_contains,
    clippy::redundant_closure_for_method_calls,
    clippy::wildcard_imports,
    clippy::uninlined_format_args,
    clippy::needless_pass_by_value,
    clippy::single_match_else,
    clippy::option_if_let_else,
    clippy::if_not_else,
    clippy::too_many_lines,
    clippy::match_same_arms,
    clippy::missing_const_for_fn,
    clippy::struct_field_names
)]

pub mod curve;

pub use curve::{
    age_curve_for_origin, AgingCurve, CALORIC_MAX_DECAY_PER_YEAR_DEFAULT,
    HEAL_RATE_DECAY_PER_YEAR_DEFAULT, MAX_SPEED_DECAY_PER_YEAR_DEFAULT,
};

use rand_core::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256StarStar;
use serde::{Deserialize, Serialize};

/// Default tick rate (Hz) used by the in-game-year arithmetic. The engine
/// passes its actual tick rate via [`BiologicalAge::tick`] so tests do not
/// have to share this constant.
pub const DEFAULT_TICK_RATE_HZ: u32 = 60;

/// One in-game year, expressed in sim seconds. The spec is silent on the
/// exact mapping; we lock 1 in-game year = 60 in-game minutes = 3600 sim
/// seconds so a typical campaign moves the clock at a meaningful cadence.
/// Modders override via the scenario manifest.
pub const SECONDS_PER_IN_GAME_YEAR: f32 = 3600.0;

/// One in-game month, expressed in sim seconds.
pub const SECONDS_PER_IN_GAME_MONTH: f32 = SECONDS_PER_IN_GAME_YEAR / 12.0;

/// One in-game week, expressed in sim seconds. Used for the per-week
/// terminal mortality roll (Gherkin scenario "Veteran death of old age"),
/// the phantom-limb panic-roll cadence, and the prosthetic maintenance
/// interval.
pub const SECONDS_PER_IN_GAME_WEEK: f32 = SECONDS_PER_IN_GAME_YEAR / 52.0;

/// One in-game day, expressed in sim seconds.
pub const SECONDS_PER_IN_GAME_DAY: f32 = SECONDS_PER_IN_GAME_YEAR / 365.25;

/// Default human retirement age (in-game years).
pub const HUMAN_RETIREMENT_AGE: f32 = 55.0;

/// Default human terminal age (in-game years).
pub const HUMAN_TERMINAL_AGE: f32 = 80.0;

///
/// `Biological` origins (humans, android_organic_side, powered_organic,
/// heavy_biomech) tick the age clock. `Mechanical` origins (robots,
/// crystallines) accumulate `chassis_wear_pct` instead — handled by the
/// caller (cf-actor::long_term).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgingOrigin {
    Human = 0,
    AndroidOrganicSide = 1,
    PoweredOrganic = 2,
    HeavyBiomech = 3,
    Robot = 4,
    Crystalline = 5,
}

impl AgingOrigin {
    pub fn as_str(self) -> &'static str {
        match self {
            AgingOrigin::Human => "human",
            AgingOrigin::AndroidOrganicSide => "android_organic_side",
            AgingOrigin::PoweredOrganic => "powered_organic",
            AgingOrigin::HeavyBiomech => "heavy_biomech",
            AgingOrigin::Robot => "robot",
            AgingOrigin::Crystalline => "crystalline",
        }
    }

    /// Best-effort parse from a snake_case label. Falls back to `Human`
    /// when the label is unrecognized so the engine never panics on a
    /// custom scenario id.
    pub fn from_label(label: &str) -> Self {
        match label {
            "human" => AgingOrigin::Human,
            "android_organic_side" | "android" | "android_organic" => AgingOrigin::AndroidOrganicSide,
            "powered_organic" => AgingOrigin::PoweredOrganic,
            "heavy_biomech" | "biomech_heavy" => AgingOrigin::HeavyBiomech,
            "robot" | "drone" => AgingOrigin::Robot,
            "crystalline" => AgingOrigin::Crystalline,
            _ => AgingOrigin::Human,
        }
    }

    /// True when this origin ticks the biological aging clock.
    pub fn is_biological(self) -> bool {
        matches!(
            self,
            AgingOrigin::Human
                | AgingOrigin::AndroidOrganicSide
                | AgingOrigin::PoweredOrganic
                | AgingOrigin::HeavyBiomech
        )
    }
}

///
/// Carried on `cf-actor::ActorState.long_term.biological_age`. Mutated
/// by [`BiologicalAge::tick`] once per simulation tick.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BiologicalAge {
    pub origin: AgingOrigin,
    /// Current in-game age (years, fractional).
    pub age_in_game_years: f32,
    /// Sub-year accumulator in sim seconds.
    pub seconds_into_current_year: f32,
    /// Sub-week accumulator (terminal-roll cadence).
    pub seconds_into_current_week: f32,
    /// Cumulative caloric_max degradation expressed as a `[0, 1]` ratio
    /// removed from the actor's baseline. 0.0 = no degradation.
    pub caloric_max_decay: f32,
    /// Cumulative max_speed degradation `[0, 1]`.
    pub max_speed_decay: f32,
    /// Cumulative wound-heal-rate degradation `[0, 1]`.
    pub heal_rate_decay: f32,
    /// True once the actor has crossed the retirement age threshold.
    pub retirement_offered: bool,
    /// True once the actor has crossed the terminal-age threshold (the
    /// per-week mortality roll fires while this is true).
    pub terminal_age_reached: bool,
    /// True once the terminal-roll fired with outcome=death. The owning
    /// engine flips the actor to `Status::Dead` once this is true.
    pub died_of_old_age: bool,
    /// Per-week mortality roll counter — increments every week past
    /// terminal_age so determinism tests can assert exact pass counts.
    pub terminal_rolls_fired: u32,
    /// Configurable thresholds (default to human defaults at construction
    /// time).
    pub retirement_age: f32,
    pub terminal_age: f32,
    /// Per-week mortality chance (probability per roll). Default 0.02 ×
    /// (age - terminal_age) i.e. ramps up after 80.
    pub mortality_base_per_week: f32,
    /// Chassis wear percentage — populated only for mechanical origins.
    /// `0.0` = pristine, `1.0` = scrap.
    pub chassis_wear_pct: f32,
}

impl BiologicalAge {
    pub fn new_human(initial_age_years: f32) -> Self {
        Self {
            origin: AgingOrigin::Human,
            age_in_game_years: initial_age_years.max(0.0),
            seconds_into_current_year: 0.0,
            seconds_into_current_week: 0.0,
            caloric_max_decay: 0.0,
            max_speed_decay: 0.0,
            heal_rate_decay: 0.0,
            retirement_offered: false,
            terminal_age_reached: false,
            died_of_old_age: false,
            terminal_rolls_fired: 0,
            retirement_age: HUMAN_RETIREMENT_AGE,
            terminal_age: HUMAN_TERMINAL_AGE,
            mortality_base_per_week: 0.02,
            chassis_wear_pct: 0.0,
        }
    }

    pub fn new_for_origin(origin: AgingOrigin, initial_age_years: f32) -> Self {
        let mut s = Self::new_human(initial_age_years);
        s.origin = origin;
        match origin {
            AgingOrigin::HeavyBiomech => {
                s.retirement_age = 90.0;
                s.terminal_age = 150.0;
                s.mortality_base_per_week = 0.005;
            }
            AgingOrigin::PoweredOrganic => {
                s.retirement_age = 70.0;
                s.terminal_age = 110.0;
                s.mortality_base_per_week = 0.01;
            }
            AgingOrigin::AndroidOrganicSide => {
                s.retirement_age = 75.0;
                s.terminal_age = 120.0;
                s.mortality_base_per_week = 0.008;
            }
            AgingOrigin::Robot | AgingOrigin::Crystalline => {
                s.retirement_age = f32::INFINITY;
                s.terminal_age = f32::INFINITY;
                s.mortality_base_per_week = 0.0;
            }
            AgingOrigin::Human => {}
        }
        s
    }

    /// Advance the clock by one sim tick. Returns the typed result the
    /// caller drives event emission from.
    pub fn tick(&mut self, dt_seconds: f32, current_tick: u64) -> AgingTickResult {
        let mut result = AgingTickResult::default();
        if self.died_of_old_age || !self.origin.is_biological() {
            return result;
        }
        let curve = age_curve_for_origin(self.origin);
        self.seconds_into_current_year += dt_seconds;
        self.seconds_into_current_week += dt_seconds;
        // Year rollover — may fire multiple times for very large dt.
        while self.seconds_into_current_year >= SECONDS_PER_IN_GAME_YEAR {
            self.seconds_into_current_year -= SECONDS_PER_IN_GAME_YEAR;
            self.age_in_game_years += 1.0;
            // Per-year degradation past age 30 (the spec's tunable
            // "Per-year caloric_max degradation post-30").
            if self.age_in_game_years > 30.0 {
                self.caloric_max_decay =
                    (self.caloric_max_decay + curve.caloric_max_decay_per_year).clamp(0.0, 0.95);
                self.max_speed_decay =
                    (self.max_speed_decay + curve.max_speed_decay_per_year).clamp(0.0, 0.90);
                self.heal_rate_decay =
                    (self.heal_rate_decay + curve.heal_rate_decay_per_year).clamp(0.0, 0.95);
            }
            result.year_advanced = true;
            result.new_age_years = self.age_in_game_years;
            // Retirement threshold (one-shot).
            if !self.retirement_offered && self.age_in_game_years >= self.retirement_age {
                self.retirement_offered = true;
                result.retirement_offered = true;
            }
            // Terminal threshold (latches, then per-week roll fires).
            if !self.terminal_age_reached && self.age_in_game_years >= self.terminal_age {
                self.terminal_age_reached = true;
                result.terminal_age_reached = true;
            }
        }
        // Per-week terminal mortality roll — only fires once the actor
        // has crossed `terminal_age`.
        while self.seconds_into_current_week >= SECONDS_PER_IN_GAME_WEEK {
            self.seconds_into_current_week -= SECONDS_PER_IN_GAME_WEEK;
            if self.terminal_age_reached && !self.died_of_old_age {
                self.terminal_rolls_fired = self.terminal_rolls_fired.saturating_add(1);
                let years_past = (self.age_in_game_years - self.terminal_age).max(0.0);
                let p = (self.mortality_base_per_week * (1.0 + years_past * 0.1))
                    .clamp(0.0, 1.0);
                // Per-roll seeded RNG: caller passes the engine seed via
                // the helper below.
                result.terminal_roll = Some(TerminalRoll {
                    probability_x1000: (p * 1000.0).round() as u32,
                    tick: current_tick,
                });
            }
        }
        result
    }

    /// Resolve a [`TerminalRoll`] outcome — call this after a tick that
    /// returned `Some(TerminalRoll)`. Uses a freshly seeded RNG derived
    /// from `seed ⊕ tick` for determinism.
    pub fn resolve_terminal_roll(&mut self, roll: TerminalRoll, seed: u64) -> bool {
        let mut rng = Xoshiro256StarStar::seed_from_u64(seed ^ roll.tick);
        let outcome = (rng.next_u64() % 1000) as u32;
        let passed = outcome < roll.probability_x1000;
        if passed {
            self.died_of_old_age = true;
        }
        passed
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AgingTickResult {
    pub year_advanced: bool,
    pub new_age_years: f32,
    pub retirement_offered: bool,
    pub terminal_age_reached: bool,
    pub terminal_roll: Option<TerminalRoll>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerminalRoll {
    pub probability_x1000: u32,
    pub tick: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum AgingEvent {
    YearAdvanced {
        actor_id: u64,
        tick: u64,
        new_age_years: f32,
        caloric_max_decay: f32,
        max_speed_decay: f32,
        heal_rate_decay: f32,
    },
    RetirementOffered {
        actor_id: u64,
        tick: u64,
        age_in_game_years: f32,
    },
    TerminalRoll {
        actor_id: u64,
        tick: u64,
        probability_x1000: u32,
        outcome: TerminalRollOutcome,
    },
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalRollOutcome {
    Survived = 0,
    Death = 1,
}

impl TerminalRollOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            TerminalRollOutcome::Survived => "survived",
            TerminalRollOutcome::Death => "death",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_aging_advances_year() {
        let mut age = BiologicalAge::new_human(30.0);
        let r = age.tick(SECONDS_PER_IN_GAME_YEAR, 60);
        assert!(r.year_advanced);
        assert!((age.age_in_game_years - 31.0).abs() < 1e-3);
    }

    #[test]
    fn caloric_max_drops_5pct_in_10_years() {
        // VAL-M14I scenario: caloric_max decreases to 95 after 10 years.
        let mut age = BiologicalAge::new_human(30.0);
        for y in 0..10 {
            age.tick(SECONDS_PER_IN_GAME_YEAR, 60 * (y as u64 + 1));
        }
        // 0.5% × 10 years = 5% degradation, applied after age>30 each year.
        // At year 30→31 the if(age>30) check fires (31>30), then 32, ... 40.
        // Total: 10 increments × 0.005 = 0.05.
        assert!(
            (age.caloric_max_decay - 0.05).abs() < 1e-5,
            "got {}",
            age.caloric_max_decay
        );
    }

    #[test]
    fn retirement_at_55_for_human() {
        let mut age = BiologicalAge::new_human(54.0);
        let r = age.tick(SECONDS_PER_IN_GAME_YEAR, 60);
        assert!(r.retirement_offered);
        assert!(age.retirement_offered);
    }

    #[test]
    fn biomech_origin_skips_robot_aging() {
        let age = BiologicalAge::new_for_origin(AgingOrigin::Robot, 30.0);
        assert!(!age.origin.is_biological());
        let mut age = age;
        let r = age.tick(SECONDS_PER_IN_GAME_YEAR * 50.0, 60);
        assert!(!r.year_advanced);
        assert_eq!(age.age_in_game_years, 30.0);
    }

    #[test]
    fn heavy_biomech_ages_slowly() {
        // Spec scenario "Per-origin aging differs": 0.2%/yr × 10 years.
        let mut age = BiologicalAge::new_for_origin(AgingOrigin::HeavyBiomech, 30.0);
        for y in 0..10 {
            age.tick(SECONDS_PER_IN_GAME_YEAR, 60 * (y as u64 + 1));
        }
        assert!((age.caloric_max_decay - 0.02).abs() < 1e-5, "got {}", age.caloric_max_decay);
    }

    #[test]
    fn terminal_roll_at_81_after_one_week() {
        let mut age = BiologicalAge::new_human(81.0);
        // Latch terminal age:
        assert!(age.age_in_game_years >= age.terminal_age);
        age.terminal_age_reached = true;
        let r = age.tick(SECONDS_PER_IN_GAME_WEEK, 60);
        assert!(r.terminal_roll.is_some(), "terminal_roll should fire");
    }

    #[test]
    fn determinism_same_seed_same_age_curve() {
        let mut a = BiologicalAge::new_human(30.0);
        let mut b = BiologicalAge::new_human(30.0);
        for tick in 0..100 {
            let dt = SECONDS_PER_IN_GAME_YEAR / 10.0;
            let _ = a.tick(dt, tick);
            let _ = b.tick(dt, tick);
        }
        assert_eq!(a.age_in_game_years, b.age_in_game_years);
        assert_eq!(a.caloric_max_decay, b.caloric_max_decay);
    }

    #[test]
    fn terminal_roll_resolution_deterministic() {
        let mut a = BiologicalAge::new_human(81.0);
        a.terminal_age_reached = true;
        // Force a roll
        let roll = TerminalRoll {
            probability_x1000: 1000,
            tick: 42,
        };
        let died_a = a.resolve_terminal_roll(roll, 7);
        let mut b = BiologicalAge::new_human(81.0);
        b.terminal_age_reached = true;
        let died_b = b.resolve_terminal_roll(roll, 7);
        assert_eq!(died_a, died_b);
        assert!(died_a, "probability=1.0 should always pass");
    }
}
