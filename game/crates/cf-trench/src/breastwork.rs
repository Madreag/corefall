//! M9B: breastwork degrade-and-breach gameplay loop.
//!
//! Spec §"Acceptance criteria":
//!
//! > Scenario: Breastwork degrades + breaches under sustained MG fire
//! >   Given a parapet_raised trench segment with full breastwork (HP 400)
//! >   When an MG nest (M9C) fires 80 rounds × 6 J across the breastwork wall
//! >   Then breastwork HP decreases per per-pixel material erosion (M14)
//! >   And when HP reaches 0, trench_breastwork_breached event fires
//! >   And cover_state for the segment downgrades from Full to Partial
//! >   And the actor inside loses head-zone cover; M14 routes future hits
//! >   through the gap
//!
//! VAL-M9B-BREASTWORK-001: HP 0 → breach event + cover downgrade; M14
//! routes future hits through the gap.
//!
//! This module owns the pure breach detector + the cover-state
//! downgrade. The cfctl handler + engine feed hits in, emit the breach
//! event when HP reaches 0, and re-derive `cover_state` from the
//! breached segment afterwards.

use serde::{Deserialize, Serialize};

use crate::cover_state::{cover_state, CoverState, TrenchStance};
use crate::segment::SegmentVariant;

/// Spec breastwork HP per the modules table: "Sandbag wall above
/// grade (M9C parts inventory); HP 400 vs small-arms". The 6 J × 80
/// rounds = 480 J budget exceeds this so the breastwork should breach
/// within the scenario's MG burst.
pub const BREASTWORK_MAX_HP: f32 = 400.0;

/// Energy per small-arms round expressed in the same units as
/// breastwork HP. 6 J per round per spec's scenario. The breach
/// detector subtracts this directly from HP per shot.
pub const ROUND_DAMAGE_J: f32 = 6.0;

/// Per-shot outcome the engine consumes. The cfctl handler feeds rounds
/// in via [`apply_round_to_breastwork`]; when HP transitions through 0
/// the `Breached` arm fires, carrying the previous HP so the replay
/// event can record `prev_hp`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BreastworkHitOutcome {
    /// HP dropped but is still > 0. The segment retains Full cover.
    Degraded { hp_after: f32 },
    /// HP reached or crossed 0 with this hit. The engine emits a
    /// `trench.breastwork_breached` event AND downgrades cover from
    /// Full → Partial for the segment going forward.
    Breached { prev_hp: f32, hp_after: f32 },
    /// HP already 0 before this hit — the engine routes the hit
    /// through the breach via the M14 damage-routing pipeline (no
    /// further breach events fire).
    AlreadyBreached,
}

impl BreastworkHitOutcome {
    #[must_use]
    pub fn breached(&self) -> bool {
        matches!(self, BreastworkHitOutcome::Breached { .. })
    }

    #[must_use]
    pub fn hp_after(&self) -> f32 {
        match self {
            BreastworkHitOutcome::Degraded { hp_after } => *hp_after,
            BreastworkHitOutcome::Breached { hp_after, .. } => *hp_after,
            BreastworkHitOutcome::AlreadyBreached => 0.0,
        }
    }
}

/// Apply one small-arms round to the breastwork wall.
///
/// `current_hp` is the breastwork's HP before the hit; `damage` is the
/// per-round energy in the same units as HP. Returns the
/// [`BreastworkHitOutcome`] the engine writes back.
#[must_use]
pub fn apply_round_to_breastwork(current_hp: f32, damage: f32) -> BreastworkHitOutcome {
    if current_hp <= 0.0 {
        return BreastworkHitOutcome::AlreadyBreached;
    }
    let new_hp = (current_hp - damage.max(0.0)).max(0.0);
    if new_hp <= 0.0 {
        return BreastworkHitOutcome::Breached {
            prev_hp: current_hp,
            hp_after: 0.0,
        };
    }
    BreastworkHitOutcome::Degraded { hp_after: new_hp }
}

/// VAL-M9B-BREASTWORK-001 sub-claim: when a `parapet_raised` segment's
/// breastwork is breached, the segment's cover state downgrades from
/// `Full` → `Partial` for any stance.
///
/// `breached=false` returns the variant's authored cover state via the
/// canonical [`cover_state`] lookup; `breached=true` clamps the result
/// to `Partial` so head/shoulders hits route around the gap.
#[must_use]
pub fn cover_state_post_breach(
    stance: TrenchStance,
    variant: SegmentVariant,
    breached: bool,
) -> CoverState {
    let base = cover_state(stance, variant);
    if !breached {
        return base;
    }
    if matches!(variant, SegmentVariant::ParapetRaised) {
        // Per spec: "cover_state for the segment downgrades from Full
        // to Partial". Even `prone` reading (which would otherwise be
        // `Full`) downgrades because the breach is at parapet height.
        return CoverState::Partial;
    }
    base
}

/// Convenience: simulate `rounds` per-shot hits against a fresh
/// breastwork at `BREASTWORK_MAX_HP`, returning the list of outcomes
/// + the final HP. Used by the unit tests + headless audit script.
#[must_use]
pub fn run_breach_sequence(rounds: u32, per_round_damage: f32) -> (Vec<BreastworkHitOutcome>, f32) {
    let mut hp = BREASTWORK_MAX_HP;
    let mut out = Vec::new();
    for _ in 0..rounds {
        let outcome = apply_round_to_breastwork(hp, per_round_damage);
        hp = outcome.hp_after();
        out.push(outcome);
        if hp <= 0.0 {
            break;
        }
    }
    (out, hp)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// VAL-M9B-BREASTWORK-001: 80 rounds × 6 J = 480 J > 400 HP. The
    /// breastwork breaches by round 67 (400 / 6 = 66.67 → ceil = 67).
    /// Assert the sequence emits exactly one `Breached` outcome.
    #[test]
    fn breastwork_breaches_within_80_rounds_of_mg_fire() {
        let (events, final_hp) = run_breach_sequence(80, ROUND_DAMAGE_J);
        let breach_count = events.iter().filter(|o| o.breached()).count();
        assert_eq!(
            breach_count, 1,
            "exactly one breach event must fire over the 80-round burst"
        );
        assert_eq!(final_hp, 0.0);
    }

    /// Each pre-breach hit returns Degraded with HP decreasing
    /// monotonically.
    #[test]
    fn breastwork_degrades_monotonically_until_breach() {
        let mut hp = BREASTWORK_MAX_HP;
        let mut previous = hp;
        for round in 1..=80 {
            let outcome = apply_round_to_breastwork(hp, ROUND_DAMAGE_J);
            hp = outcome.hp_after();
            assert!(hp <= previous, "HP must decrease (round {round})");
            previous = hp;
            if outcome.breached() {
                assert_eq!(hp, 0.0);
                return;
            }
        }
        panic!("expected breach within 80 rounds");
    }

    /// After breach, subsequent hits route through gap — apply returns
    /// `AlreadyBreached`.
    #[test]
    fn post_breach_hits_route_through_gap() {
        let (_events, hp) = run_breach_sequence(80, ROUND_DAMAGE_J);
        assert_eq!(hp, 0.0);
        let outcome = apply_round_to_breastwork(hp, ROUND_DAMAGE_J);
        assert!(matches!(outcome, BreastworkHitOutcome::AlreadyBreached));
    }

    /// VAL-M9B-BREASTWORK-001: cover state for a breached `parapet_raised`
    /// downgrades from Full → Partial regardless of stance.
    #[test]
    fn cover_state_downgrades_full_to_partial_on_breach() {
        for stance in [
            TrenchStance::Standing,
            TrenchStance::Crouched,
            TrenchStance::Prone,
        ] {
            let pre = cover_state_post_breach(stance, SegmentVariant::ParapetRaised, false);
            assert_eq!(pre, CoverState::Full, "pre-breach must be Full");
            let post = cover_state_post_breach(stance, SegmentVariant::ParapetRaised, true);
            assert_eq!(
                post,
                CoverState::Partial,
                "post-breach {stance:?} must downgrade to Partial"
            );
        }
    }

    /// Non-parapet_raised variants are unaffected by the breach flag
    /// (only the parapet has a breastwork to breach).
    #[test]
    fn non_parapet_variants_unchanged_by_breach_flag() {
        let variants = [
            SegmentVariant::ShallowScrape,
            SegmentVariant::Standard,
            SegmentVariant::Deep,
            SegmentVariant::Communication,
            SegmentVariant::FireStep,
        ];
        for variant in variants {
            for stance in [
                TrenchStance::Standing,
                TrenchStance::Crouched,
                TrenchStance::Prone,
            ] {
                let pre = cover_state_post_breach(stance, variant, false);
                let post = cover_state_post_breach(stance, variant, true);
                assert_eq!(
                    pre, post,
                    "{variant:?}/{stance:?} must be identical pre vs post (no breastwork)"
                );
            }
        }
    }

    #[test]
    fn zero_damage_does_not_breach() {
        let outcome = apply_round_to_breastwork(BREASTWORK_MAX_HP, 0.0);
        match outcome {
            BreastworkHitOutcome::Degraded { hp_after } => {
                assert_eq!(hp_after, BREASTWORK_MAX_HP);
            }
            other => panic!("expected Degraded, got {other:?}"),
        }
    }

    #[test]
    fn overkill_clamps_to_zero() {
        let outcome = apply_round_to_breastwork(10.0, 1_000_000.0);
        match outcome {
            BreastworkHitOutcome::Breached { prev_hp, hp_after } => {
                assert_eq!(prev_hp, 10.0);
                assert_eq!(hp_after, 0.0);
            }
            other => panic!("expected Breached, got {other:?}"),
        }
    }
}
