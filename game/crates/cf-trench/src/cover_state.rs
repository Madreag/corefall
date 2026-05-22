//! M9B: cover-state derivation for trench segments.
//!
//! Spec §"Player-facing behavior — Per-segment cover state field" and the
//! authoring table under §"Trench cross-section variants (6 authored)":
//!
//! | Variant         | Standing                       | Crouched              | Prone |
//! |---|---|---|---|
//! | `shallow_scrape`| Exposed                        | Partial               | Full  |
//! | `standard`      | Partial                        | Full                  | Full  |
//! | `deep`          | Full (head below grade)        | Full                  | Full  |
//! | `communication` | Partial                        | Full                  | Full  |
//! | `fire_step`     | Exposed on-step / Partial off  | Partial on / Full off | Full  |
//! | `parapet_raised`| Full                           | Full                  | Full  |
//!
//! The derivation is **pure** and **stateless** — the M9B Notes call out
//! "Cover state is derived, not stored: at every frame, `cover_state(actor)
//! = lookup(segment_at(actor.pos)) × actor.stance`. Do NOT cache per-tick".
//! Callers may re-read [`cover_state`] mid-tick after mutating stance and
//! will observe the new value on the next call.

use serde::{Deserialize, Serialize};

use crate::segment::SegmentVariant;

/// Three-state cover classification used by HUD, AI doctrine, and the M14
/// damage-routing pipeline. `Exposed` ⇒ no parapet interposed; the actor's
/// silhouette is fully visible. `Partial` ⇒ head/shoulders exposed but
/// torso/legs protected (or symmetric upper-body cover). `Full` ⇒ the
/// parapet/breastwork covers the head zone — incoming small-arms must
/// over-penetrate the cover before reaching the actor.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum CoverState {
    Exposed = 0,
    Partial = 1,
    Full = 2,
}

impl CoverState {
    pub const fn as_str(self) -> &'static str {
        match self {
            CoverState::Exposed => "Exposed",
            CoverState::Partial => "Partial",
            CoverState::Full => "Full",
        }
    }
}

impl Default for CoverState {
    fn default() -> Self {
        CoverState::Exposed
    }
}

/// The trench-cover stance axis. The cf-actor `Stance` enum has 23+
/// variants for the full M6 tactical surface; the trench-cover derivation
/// only cares about the three-way Standing/Crouched/Prone partition.
///
/// `from_actor_stance` collapses the full enum down to this axis using the
/// spec's posture mapping: sprint/walk/run/idle/airborne/etc. → Standing
/// (the actor's torso is at full standing height for cover purposes),
/// crouch-variants → Crouched, prone-variants → Prone.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TrenchStance {
    Standing = 0,
    Crouched = 1,
    Prone = 2,
}

impl TrenchStance {
    pub const fn as_str(self) -> &'static str {
        match self {
            TrenchStance::Standing => "Standing",
            TrenchStance::Crouched => "Crouched",
            TrenchStance::Prone => "Prone",
        }
    }
}

impl Default for TrenchStance {
    fn default() -> Self {
        TrenchStance::Standing
    }
}

/// Derive cover state from the (stance × segment variant) tuple per the
/// M9B spec table. For [`SegmentVariant::FireStep`] the value uses the
/// **on-step** semantics (the canonical firing posture) — Standing on the
/// step deliberately exposes the actor. The [`cover_state_fire_step`]
/// helper exposes the on/off-step axis for VAL-M9B-SEGMENT-004.
#[must_use]
pub fn cover_state(stance: TrenchStance, variant: SegmentVariant) -> CoverState {
    use CoverState::{Exposed, Full, Partial};
    use SegmentVariant::{Communication, Deep, FireStep, ParapetRaised, ShallowScrape, Standard};
    use TrenchStance::{Crouched, Prone, Standing};
    match (variant, stance) {
        (ShallowScrape, Standing) => Exposed,
        (ShallowScrape, Crouched) => Partial,
        (ShallowScrape, Prone) => Full,

        (Standard, Standing) => Partial,
        (Standard, Crouched) => Full,
        (Standard, Prone) => Full,

        (Deep, Standing) => Full,
        (Deep, Crouched) => Full,
        (Deep, Prone) => Full,

        (Communication, Standing) => Partial,
        (Communication, Crouched) => Full,
        (Communication, Prone) => Full,

        // fire_step default uses on-step semantics: the spec table reads
        // "Standing on step = exposed (firing posture)". The off-step
        // case is exposed via `cover_state_fire_step`.
        (FireStep, Standing) => Exposed,
        (FireStep, Crouched) => Partial,
        (FireStep, Prone) => Full,

        (ParapetRaised, Standing) => Full,
        (ParapetRaised, Crouched) => Full,
        (ParapetRaised, Prone) => Full,
    }
}

/// `fire_step` derivation with the on/off-step sub-axis made explicit.
/// `on_step=true` ⇒ standing on the raised platform (firing posture);
/// `on_step=false` ⇒ off-step along the trench floor.
#[must_use]
pub fn cover_state_fire_step(stance: TrenchStance, on_step: bool) -> CoverState {
    use CoverState::{Exposed, Full, Partial};
    use TrenchStance::{Crouched, Prone, Standing};
    match (on_step, stance) {
        (true, Standing) => Exposed,
        (true, Crouched) => Partial,
        (true, Prone) => Full,
        (false, Standing) => Partial,
        (false, Crouched) => Full,
        (false, Prone) => Full,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segment::SegmentVariant;

    #[test]
    fn derives_full_in_deep_standing() {
        assert_eq!(
            cover_state(TrenchStance::Standing, SegmentVariant::Deep),
            CoverState::Full
        );
    }

    #[test]
    fn fire_step_exposes_when_standing_on_step() {
        assert_eq!(
            cover_state_fire_step(TrenchStance::Standing, true),
            CoverState::Exposed
        );
        assert_eq!(
            cover_state(TrenchStance::Standing, SegmentVariant::FireStep),
            CoverState::Exposed,
            "default fire_step uses on-step semantics for the canonical firing posture"
        );
    }

    #[test]
    fn fire_step_full_when_crouched_off_step() {
        assert_eq!(
            cover_state_fire_step(TrenchStance::Crouched, false),
            CoverState::Full
        );
    }

    /// on each call. Mutating stance mid-tick must be visible on the
    /// next read with no caching layer interposed.
    #[test]
    fn derivation_is_not_cached_across_stance_change() {
        let variant = SegmentVariant::Standard;
        let mut stance = TrenchStance::Standing;
        let first = cover_state(stance, variant);
        assert_eq!(first, CoverState::Partial);
        stance = TrenchStance::Crouched;
        let second = cover_state(stance, variant);
        assert_eq!(second, CoverState::Full);
        // Mutate back; third reads new value again — proves no memoization.
        stance = TrenchStance::Standing;
        let third = cover_state(stance, variant);
        assert_eq!(third, CoverState::Partial);
        assert_ne!(first, second);
        assert_ne!(second, third);
    }

    /// Alias name for the project-style test discoverability per the
    /// feature spec evidence string `derivation_not_cached`.
    #[test]
    fn derivation_not_cached() {
        derivation_is_not_cached_across_stance_change();
    }

    /// the spec table. 6 base variants × 3 stances = 18 cells; the
    /// `fire_step` row uses on-step semantics per the table reading.
    #[test]
    fn cover_state_18_cell_matrix() {
        use CoverState::{Exposed, Full, Partial};
        use SegmentVariant::{Communication, Deep, FireStep, ParapetRaised, ShallowScrape, Standard};
        use TrenchStance::{Crouched, Prone, Standing};
        let expected: &[(SegmentVariant, TrenchStance, CoverState)] = &[
            (ShallowScrape, Standing, Exposed),
            (ShallowScrape, Crouched, Partial),
            (ShallowScrape, Prone, Full),
            (Standard, Standing, Partial),
            (Standard, Crouched, Full),
            (Standard, Prone, Full),
            (Deep, Standing, Full),
            (Deep, Crouched, Full),
            (Deep, Prone, Full),
            (Communication, Standing, Partial),
            (Communication, Crouched, Full),
            (Communication, Prone, Full),
            (FireStep, Standing, Exposed),
            (FireStep, Crouched, Partial),
            (FireStep, Prone, Full),
            (ParapetRaised, Standing, Full),
            (ParapetRaised, Crouched, Full),
            (ParapetRaised, Prone, Full),
        ];
        assert_eq!(expected.len(), 18);
        for (variant, stance, want) in expected {
            let got = cover_state(*stance, *variant);
            assert_eq!(
                got, *want,
                "({stance:?}, {variant:?}) expected {want:?} got {got:?}"
            );
        }
    }

    /// VAL-M9B-COVERMATRIX-001 extension: the on/off-step axis for
    /// `fire_step` matches both rows of the contract table.
    #[test]
    fn cover_state_fire_step_on_off_axis() {
        use CoverState::{Exposed, Full, Partial};
        use TrenchStance::{Crouched, Prone, Standing};
        let on_step = [
            (Standing, Exposed),
            (Crouched, Partial),
            (Prone, Full),
        ];
        let off_step = [
            (Standing, Partial),
            (Crouched, Full),
            (Prone, Full),
        ];
        for (stance, want) in on_step {
            assert_eq!(cover_state_fire_step(stance, true), want);
        }
        for (stance, want) in off_step {
            assert_eq!(cover_state_fire_step(stance, false), want);
        }
    }

    #[test]
    fn cover_state_default_is_exposed() {
        assert_eq!(CoverState::default(), CoverState::Exposed);
    }

    #[test]
    fn trench_stance_default_is_standing() {
        assert_eq!(TrenchStance::default(), TrenchStance::Standing);
    }
}
