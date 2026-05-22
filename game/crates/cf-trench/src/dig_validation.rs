//! M9B: per-variant placement validation.
//!
//! Spec §"Notes for the implementer":
//!
//! > Per-variant placement validation: `deep` requires
//! > `parent material.hardness < 0.5` (cannot dig through concrete/basalt);
//! > fall back to shallow_scrape with warning event.
//!
//! [`dig_substrate_validate`] is the pure decision function the cfctl
//! handler `act.player.dig_trench_segment` consumes. It returns one of
//! three outcomes:
//!
//! - [`DigSubstrateOutcome::Ok`] — substrate accepts the requested variant.
//! - [`DigSubstrateOutcome::Fallback`] — substrate too hard for the
//!   requested variant; engine should downgrade to the fallback variant
//!   and emit the corresponding warning event.
//! - [`DigSubstrateOutcome::Reject`] — substrate too hard AND no
//!   fallback is appropriate (currently unused; the contract permits
//!   either fallback OR hard error per VAL-M9B-DIG-003).
//!
//! All numeric thresholds live in this module so the cfctl + cf-mod
//! validation paths consume a single source of truth.

use crate::segment::SegmentVariant;

/// variant. Spec §"Notes": `deep` requires `parent_material.hardness < 0.5`.
pub const DEEP_HARDNESS_THRESHOLD: f32 = 0.5;

/// Event-kind id emitted in the replay log when a `deep` dig falls back
/// to `shallow_scrape` because the substrate is too hard. Matches the
/// `trench.segment_variant_downgraded` event family expected by
/// VAL-M9B-DIG-003's "Evidence: ... `trench.segment_variant_downgraded`
/// event in the replay log declaring `from: "deep"`, `to: "shallow_scrape"`,
/// `reason: "substrate_hardness_above_threshold"`".
pub const WARNING_EVENT_KIND: &str = "segment_variant_downgraded";

/// Reason label included on the downgrade warning event.
pub const SUBSTRATE_TOO_HARD_REASON: &str = "substrate_hardness_above_threshold";

/// Outcome of the dig-substrate validator. The cfctl handler turns this
/// into either an `ok=true` ack + `trench.segment_dug` event (Ok), an
/// `ok=true` ack with a `trench.segment_variant_downgraded` warning
/// event + a `trench.segment_dug` event using the fallback variant
/// (Fallback), or an `ok=false` rejection (Reject).
#[derive(Debug, Clone, PartialEq)]
pub enum DigSubstrateOutcome {
    /// Substrate accepts the requested variant.
    Ok { variant: SegmentVariant },
    /// Substrate too hard for the requested variant — engine should
    /// place `fallback_variant` instead and emit the named warning event.
    Fallback {
        requested: SegmentVariant,
        fallback_variant: SegmentVariant,
        reason: &'static str,
        event_kind: &'static str,
    },
    /// Substrate hard rejection — engine returns `ok=false`. Reserved
    /// for future variants where no fallback is appropriate.
    Reject {
        requested: SegmentVariant,
        reason: &'static str,
    },
}

impl DigSubstrateOutcome {
    /// Returns the effective variant the engine should place — either
    /// the originally requested variant (Ok) or the fallback variant
    /// (Fallback). Returns `None` for Reject outcomes.
    #[must_use]
    pub fn effective_variant(&self) -> Option<SegmentVariant> {
        match self {
            DigSubstrateOutcome::Ok { variant } => Some(*variant),
            DigSubstrateOutcome::Fallback {
                fallback_variant, ..
            } => Some(*fallback_variant),
            DigSubstrateOutcome::Reject { .. } => None,
        }
    }

    /// True when the request was downgraded to a fallback variant.
    #[must_use]
    pub fn is_fallback(&self) -> bool {
        matches!(self, DigSubstrateOutcome::Fallback { .. })
    }

    /// True when the request was hard-rejected.
    #[must_use]
    pub fn is_reject(&self) -> bool {
        matches!(self, DigSubstrateOutcome::Reject { .. })
    }
}

/// Validate a dig request against the substrate hardness.
///
/// `requested` is the variant the cfctl caller asked for; `hardness` is
/// the local-pixel-material hardness `[0.0, 1.0]` from cf-material; the
/// `prefer_hard_reject` flag controls whether a `deep` request on hard
/// substrate becomes a fallback (false; default per spec) or a hard
/// reject (true; the cfctl caller can opt in via a `strict=true` param).
#[must_use]
pub fn dig_substrate_validate(
    requested: SegmentVariant,
    hardness: f32,
    prefer_hard_reject: bool,
) -> DigSubstrateOutcome {
    match requested {
        SegmentVariant::Deep if hardness >= DEEP_HARDNESS_THRESHOLD => {
            if prefer_hard_reject {
                DigSubstrateOutcome::Reject {
                    requested,
                    reason: SUBSTRATE_TOO_HARD_REASON,
                }
            } else {
                DigSubstrateOutcome::Fallback {
                    requested,
                    fallback_variant: SegmentVariant::ShallowScrape,
                    reason: SUBSTRATE_TOO_HARD_REASON,
                    event_kind: WARNING_EVENT_KIND,
                }
            }
        }
        _ => DigSubstrateOutcome::Ok { variant: requested },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deep_accepts_soft_dirt() {
        let outcome = dig_substrate_validate(SegmentVariant::Deep, 0.2, false);
        assert_eq!(
            outcome,
            DigSubstrateOutcome::Ok {
                variant: SegmentVariant::Deep
            }
        );
        assert!(!outcome.is_fallback());
        assert!(!outcome.is_reject());
    }

    #[test]
    fn deep_falls_back_on_concrete() {
        let outcome = dig_substrate_validate(SegmentVariant::Deep, 0.7, false);
        assert!(outcome.is_fallback(), "concrete must downgrade");
        assert_eq!(
            outcome.effective_variant(),
            Some(SegmentVariant::ShallowScrape)
        );
        if let DigSubstrateOutcome::Fallback {
            reason, event_kind, ..
        } = outcome
        {
            assert_eq!(reason, SUBSTRATE_TOO_HARD_REASON);
            assert_eq!(event_kind, WARNING_EVENT_KIND);
        } else {
            unreachable!("matched is_fallback above");
        }
    }

    #[test]
    fn deep_rejects_on_concrete_when_strict() {
        let outcome = dig_substrate_validate(SegmentVariant::Deep, 0.7, true);
        assert!(outcome.is_reject());
        assert_eq!(outcome.effective_variant(), None);
    }

    #[test]
    fn shallow_scrape_accepts_any_hardness() {
        for hardness in [0.0, 0.5, 0.99, 1.0] {
            let outcome =
                dig_substrate_validate(SegmentVariant::ShallowScrape, hardness, false);
            assert_eq!(
                outcome.effective_variant(),
                Some(SegmentVariant::ShallowScrape),
                "shallow_scrape rejected at hardness {hardness}"
            );
        }
    }

    #[test]
    fn standard_accepts_any_hardness() {
        for hardness in [0.0, 0.49, 0.5, 0.7, 1.0] {
            let outcome =
                dig_substrate_validate(SegmentVariant::Standard, hardness, false);
            assert_eq!(
                outcome.effective_variant(),
                Some(SegmentVariant::Standard),
                "standard rejected at hardness {hardness}"
            );
        }
    }

    /// rejects — the spec explicitly says "fall back OR hard error", so
    /// both paths are valid.
    #[test]
    fn deep_threshold_boundary_at_exactly_point_five() {
        let fallback = dig_substrate_validate(SegmentVariant::Deep, 0.5, false);
        assert!(fallback.is_fallback());
        let reject = dig_substrate_validate(SegmentVariant::Deep, 0.5, true);
        assert!(reject.is_reject());
    }

    /// Just below the threshold, `deep` passes.
    #[test]
    fn deep_threshold_boundary_just_below_point_five() {
        let outcome =
            dig_substrate_validate(SegmentVariant::Deep, 0.499_999, false);
        assert_eq!(outcome.effective_variant(), Some(SegmentVariant::Deep));
        assert!(!outcome.is_fallback());
    }
}
