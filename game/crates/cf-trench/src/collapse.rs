//! M9B: revetment + soft-dirt collapse gameplay loop.
//!
//! Spec §"Acceptance criteria":
//!
//! > Scenario: Revetment prevents wall slough on hardness-0.2 dirt
//! >   Given a `standard` trench in soft dirt (hardness 0.2) without revetment
//! >   When 1800 ticks elapse (3 in-game minutes)
//! >   Then trench_segment_collapsed event fires for ≥ 1 segment (walls slough)
//! >   Given the same trench with revetment module on both walls
//! >   When 1800 ticks elapse
//! >   Then no trench_segment_collapsed event fires (revetment locks M14E
//! >        integrity ≥ 600)
//!
//! VAL-M9B-REVETMENT-001: no-revetment 1800-tick run produces ≥ 1
//! collapse event.
//! VAL-M9B-REVETMENT-002: revetment installed → 0 collapse events +
//! integrity ≥ 600 throughout the same window.
//!
//! This module owns the pure decision functions; the cfctl handler +
//! engine drive the per-tick integrity update and emit the
//! `trench.segment_collapsed` replay event when a segment reaches its
//! collapse trigger.

use serde::{Deserialize, Serialize};

use crate::segment::SegmentVariant;

/// VAL-M9B-REVETMENT-001 threshold per spec §"Notes for the
/// implementer" + the module table: revetment "M14E integrity field
/// 600 prevents wall slough". Segments with revetment retain ≥ this
/// integrity for the full 1800-tick audit window.
pub const REVETMENT_INTEGRITY_FLOOR: f32 = 600.0;

/// Spec window in ticks for the revetment audit (3 in-game minutes
/// at 600 ticks/min). Surfaced as a constant so test fixtures + the
/// closure-feature worker share one value.
pub const REVETMENT_AUDIT_WINDOW_TICKS: u32 = 1800;

/// Hardness threshold below which a no-revetment segment sloughs
/// naturally. Spec scenario sets dirt = 0.2; we treat anything < 0.3
/// as "soft" so the collapse path fires for the exemplar dirt floor
/// but does not fire on `standard` cinderblock substrates.
pub const SOFT_DIRT_THRESHOLD: f32 = 0.3;

/// Per-tick integrity decay (M14E units) when revetment is absent on
/// soft-dirt substrate. 1.0 / tick × 1800 ticks = 1800 total decay;
/// starting from `STARTING_INTEGRITY` (1200) the segment crosses the
/// 0-integrity collapse line at tick 1200 — well inside the spec's
/// 1800-tick audit window.
pub const SOFT_DIRT_DECAY_PER_TICK: f32 = 1.0;

/// Integrity at which a no-revetment segment is considered to have
/// sloughed. Once `current_integrity` reaches this floor the engine
/// fires exactly one `trench.segment_collapsed` event + retires the
/// segment from the live world index.
pub const COLLAPSE_INTEGRITY_FLOOR: f32 = 0.0;

/// Initial integrity for a freshly dug segment per the spec table:
/// "M14E integrity field locks segment boundary". Default 1200 so the
/// per-tick decay above produces the expected collapse cadence inside
/// the audit window.
pub const STARTING_INTEGRITY: f32 = 1200.0;

/// One outcome of [`collapse_tick`]: either the segment held
/// (Stable / IntegrityDecay), or it collapsed this tick.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CollapseTickOutcome {
    /// Segment has revetment OR substrate is hard enough; integrity is
    /// pinned to ≥ [`REVETMENT_INTEGRITY_FLOOR`] (when revetment is
    /// installed) or stays at the starting value (hard substrate).
    Stable { integrity: f32 },
    /// No revetment + soft substrate: integrity decreased this tick but
    /// the segment still holds (above the collapse floor).
    Decaying { integrity: f32 },
    /// Integrity crossed [`COLLAPSE_INTEGRITY_FLOOR`] this tick. The
    /// engine emits `trench.segment_collapsed` once and removes the
    /// segment from the live world.
    Collapsed {
        prev_integrity: f32,
        cause: CollapseCause,
    },
}

impl CollapseTickOutcome {
    #[must_use]
    pub fn integrity_after(&self) -> f32 {
        match self {
            CollapseTickOutcome::Stable { integrity } => *integrity,
            CollapseTickOutcome::Decaying { integrity } => *integrity,
            CollapseTickOutcome::Collapsed { .. } => 0.0,
        }
    }

    #[must_use]
    pub fn collapsed(&self) -> bool {
        matches!(self, CollapseTickOutcome::Collapsed { .. })
    }
}

/// Cause label recorded on the `trench.segment_collapsed` replay
/// event. Mirrors the schema's `cause.enum`.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollapseCause {
    /// Soft-dirt substrate + no revetment: natural wall slough.
    NoRevetmentInSoftDirt,
    /// M14E integrity field exhausted by combined damage + decay.
    IntegrityExhausted,
    /// M14F lateral wall slough (used by reactor blast adjacency cases).
    WallSlough,
}

impl CollapseCause {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            CollapseCause::NoRevetmentInSoftDirt => "no_revetment_in_soft_dirt",
            CollapseCause::IntegrityExhausted => "integrity_exhausted",
            CollapseCause::WallSlough => "wall_slough",
        }
    }
}

/// Configuration passed to [`collapse_tick`]: substrate hardness,
/// revetment flag, and the per-tick decay rate. The engine threads the
/// per-segment values through every tick.
#[derive(Debug, Clone, Copy)]
pub struct CollapseEnv {
    pub substrate_hardness: f32,
    pub has_revetment: bool,
    pub decay_per_tick: f32,
}

impl Default for CollapseEnv {
    fn default() -> Self {
        Self {
            substrate_hardness: 0.5,
            has_revetment: false,
            decay_per_tick: SOFT_DIRT_DECAY_PER_TICK,
        }
    }
}

impl CollapseEnv {
    /// Spec scenario: soft dirt (hardness 0.2), no revetment.
    #[must_use]
    pub fn soft_dirt_no_revetment() -> Self {
        Self {
            substrate_hardness: 0.2,
            has_revetment: false,
            decay_per_tick: SOFT_DIRT_DECAY_PER_TICK,
        }
    }

    /// Spec scenario: soft dirt with revetment installed.
    #[must_use]
    pub fn soft_dirt_with_revetment() -> Self {
        Self {
            substrate_hardness: 0.2,
            has_revetment: true,
            decay_per_tick: SOFT_DIRT_DECAY_PER_TICK,
        }
    }
}

/// Advance one collapse tick for a single trench segment. The
/// outcome carries the new integrity and whether the segment
/// collapsed this tick.
///
/// Pure: callers thread the result back into their integrity field
/// AND emit `trench.segment_collapsed` when the result is
/// [`CollapseTickOutcome::Collapsed`].
#[must_use]
pub fn collapse_tick(current_integrity: f32, env: CollapseEnv) -> CollapseTickOutcome {
    if env.has_revetment {
        // VAL-M9B-REVETMENT-002: revetment pins integrity at ≥ 600.
        let pinned = current_integrity.max(REVETMENT_INTEGRITY_FLOOR);
        return CollapseTickOutcome::Stable { integrity: pinned };
    }
    if env.substrate_hardness >= SOFT_DIRT_THRESHOLD {
        // Hard substrate without revetment still holds — natural
        // slough only fires on soft dirt per the spec scenario.
        return CollapseTickOutcome::Stable {
            integrity: current_integrity,
        };
    }
    let new_integrity = current_integrity - env.decay_per_tick;
    if new_integrity <= COLLAPSE_INTEGRITY_FLOOR {
        return CollapseTickOutcome::Collapsed {
            prev_integrity: current_integrity,
            cause: CollapseCause::NoRevetmentInSoftDirt,
        };
    }
    CollapseTickOutcome::Decaying {
        integrity: new_integrity,
    }
}

/// VAL-M9B-REVETMENT-001/002 audit helper: run `ticks` worth of
/// collapse updates against a single segment and return the count of
/// collapse events + the final integrity at end of window.
#[must_use]
pub fn run_revetment_audit(
    initial_integrity: f32,
    env: CollapseEnv,
    ticks: u32,
) -> (u32, f32) {
    let mut integrity = initial_integrity;
    let mut collapses = 0u32;
    let mut alive = true;
    for _ in 0..ticks {
        if !alive {
            break;
        }
        let outcome = collapse_tick(integrity, env);
        match outcome {
            CollapseTickOutcome::Collapsed { .. } => {
                collapses += 1;
                alive = false;
                integrity = 0.0;
            }
            CollapseTickOutcome::Stable { integrity: new }
            | CollapseTickOutcome::Decaying { integrity: new } => {
                integrity = new;
            }
        }
    }
    (collapses, integrity)
}

/// Variant-aware overload: routes the `parapet_raised` variant
/// through the same decay path; reserved for future spec extensions.
/// Currently unused but kept stable so the engine surface remains
/// forward-compatible.
#[must_use]
pub fn variant_supports_collapse(variant: SegmentVariant) -> bool {
    matches!(
        variant,
        SegmentVariant::ShallowScrape
            | SegmentVariant::Standard
            | SegmentVariant::Deep
            | SegmentVariant::Communication
            | SegmentVariant::FireStep
            | SegmentVariant::ParapetRaised
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// VAL-M9B-REVETMENT-001: soft dirt, no revetment, 1800 ticks →
    /// ≥ 1 collapse event.
    #[test]
    fn no_revetment_collapses_within_1800_ticks() {
        let env = CollapseEnv::soft_dirt_no_revetment();
        let (collapses, final_integrity) =
            run_revetment_audit(STARTING_INTEGRITY, env, REVETMENT_AUDIT_WINDOW_TICKS);
        assert!(
            collapses >= 1,
            "soft-dirt no-revetment must collapse at least once in {} ticks",
            REVETMENT_AUDIT_WINDOW_TICKS
        );
        assert_eq!(final_integrity, 0.0);
    }

    /// VAL-M9B-REVETMENT-002: soft dirt + revetment, 1800 ticks → 0
    /// collapse events AND integrity ≥ 600 throughout.
    #[test]
    fn revetment_prevents_collapse_over_1800_ticks() {
        let env = CollapseEnv::soft_dirt_with_revetment();
        let (collapses, final_integrity) =
            run_revetment_audit(STARTING_INTEGRITY, env, REVETMENT_AUDIT_WINDOW_TICKS);
        assert_eq!(
            collapses, 0,
            "revetment must prevent collapse over {} ticks",
            REVETMENT_AUDIT_WINDOW_TICKS
        );
        assert!(
            final_integrity >= REVETMENT_INTEGRITY_FLOOR,
            "revetment must pin integrity at ≥ 600 (got {final_integrity})"
        );
    }

    /// Hard substrate without revetment still holds — the decay path
    /// only fires on soft dirt.
    #[test]
    fn hard_substrate_no_revetment_does_not_collapse() {
        let env = CollapseEnv {
            substrate_hardness: 0.5,
            has_revetment: false,
            decay_per_tick: SOFT_DIRT_DECAY_PER_TICK,
        };
        let (collapses, final_integrity) =
            run_revetment_audit(STARTING_INTEGRITY, env, REVETMENT_AUDIT_WINDOW_TICKS);
        assert_eq!(collapses, 0);
        assert_eq!(final_integrity, STARTING_INTEGRITY);
    }

    /// Each pre-collapse tick on soft dirt without revetment is a
    /// Decaying outcome; integrity decreases monotonically until the
    /// final Collapsed tick.
    #[test]
    fn decaying_progression_until_collapse() {
        let env = CollapseEnv::soft_dirt_no_revetment();
        let mut integrity = STARTING_INTEGRITY;
        let mut last = integrity;
        let mut collapsed = false;
        for _ in 0..REVETMENT_AUDIT_WINDOW_TICKS {
            let outcome = collapse_tick(integrity, env);
            match outcome {
                CollapseTickOutcome::Decaying { integrity: new } => {
                    assert!(new < last, "integrity must decrease");
                    last = new;
                    integrity = new;
                }
                CollapseTickOutcome::Collapsed { .. } => {
                    collapsed = true;
                    break;
                }
                CollapseTickOutcome::Stable { .. } => {
                    panic!("soft dirt no revetment must decay, not stay Stable");
                }
            }
        }
        assert!(collapsed, "expected a collapse within audit window");
    }

    #[test]
    fn collapse_outcome_carries_no_revetment_cause() {
        let env = CollapseEnv::soft_dirt_no_revetment();
        // Force imminent collapse by starting just above the floor.
        let outcome = collapse_tick(0.5, env);
        match outcome {
            CollapseTickOutcome::Collapsed { cause, prev_integrity } => {
                assert_eq!(cause, CollapseCause::NoRevetmentInSoftDirt);
                assert_eq!(cause.as_str(), "no_revetment_in_soft_dirt");
                assert_eq!(prev_integrity, 0.5);
            }
            other => panic!("expected Collapsed, got {other:?}"),
        }
    }

    #[test]
    fn revetment_pins_integrity_above_floor() {
        let env = CollapseEnv::soft_dirt_with_revetment();
        // Even if we initialize below the floor, revetment lifts it.
        let outcome = collapse_tick(100.0, env);
        match outcome {
            CollapseTickOutcome::Stable { integrity } => {
                assert!(integrity >= REVETMENT_INTEGRITY_FLOOR);
            }
            other => panic!("expected Stable, got {other:?}"),
        }
    }

    #[test]
    fn collapse_cause_as_str_round_trip() {
        assert_eq!(
            CollapseCause::NoRevetmentInSoftDirt.as_str(),
            "no_revetment_in_soft_dirt"
        );
        assert_eq!(
            CollapseCause::IntegrityExhausted.as_str(),
            "integrity_exhausted"
        );
        assert_eq!(CollapseCause::WallSlough.as_str(), "wall_slough");
    }

    /// All 6 variants are eligible for the collapse path; the spec
    /// scenario uses `standard`.
    #[test]
    fn all_variants_support_collapse_path() {
        for v in SegmentVariant::ALL {
            assert!(variant_supports_collapse(v));
        }
    }
}
