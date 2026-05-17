//! M9B: drainage_sump gameplay behavior.
//!
//! Spec §"Acceptance criteria":
//!
//! > Scenario: Drainage sump flushes accumulated water
//! >   Given a `deep` trench segment with drainage_sump module + heavy rain (M31)
//! >   When 600 ticks elapse
//! >   Then water tiles in the trench floor stay ≤ 2 px deep (sump flushes at
//! >        ≥ 2 px threshold)
//! >   And trench_drainage_flushed event fires per flush cycle
//! >   When the player demolishes the drainage_sump module
//! >   Then water accumulates beyond 2 px within the next 600 ticks
//! >   And the player's footing converts to wet-mud (M3 per-pixel slippery flag)
//!
//! VAL-M9B-DRAINAGE-001: sump flushes at ≥ 2 px threshold under heavy rain.
//! VAL-M9B-DRAINAGE-002: demolishing the sump causes flood + wet-mud footing.
//!
//! This module owns the pure decision functions. The cfctl handler +
//! engine consume the result and write the `trench.drainage_flushed`
//! replay event from the [`DrainageTickOutcome`].

use serde::{Deserialize, Serialize};

/// Spec gameplay threshold per VAL-M9B-DRAINAGE-001: when the sump is
/// present, water depth in pixels is clamped to ≤ this value. When the
/// reading rises above the threshold the sump fires a flush cycle and
/// drains down to baseline.
pub const FLUSH_THRESHOLD_PX: f32 = 2.0;

/// Water depth (pixels) the sump drains accumulated water *down to* on
/// each flush. Spec says "stays ≤ 2 px" so the post-flush level is
/// strictly less than the threshold; we drain to 1.0 px so the next
/// rain tick has headroom before re-triggering.
pub const FLUSH_FLOOR_PX: f32 = 1.0;

/// Per-tick water accumulation under heavy rain when the sump is
/// **absent** (or demolished). VAL-M9B-DRAINAGE-002 requires water to
/// cross 2 px within 600 ticks; at 0.01 px/tick the depth crosses
/// 2 px at tick 200 (200 * 0.01 = 2.0). Conservative — fastest rain
/// rate stays under the spec's 600-tick budget by 3x.
pub const RAIN_ACCUMULATION_PER_TICK_PX: f32 = 0.01;

/// One outcome of [`drainage_sump_tick`]: either the sump flushed,
/// emitting an event, the sump did nothing (water below the threshold),
/// OR the sump is absent and water accumulated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DrainageTickOutcome {
    /// Sump fired a flush cycle. The engine emits
    /// `trench.drainage_flushed { water_depth_before, water_depth_after }`.
    Flushed {
        water_depth_before: f32,
        water_depth_after: f32,
    },
    /// Sump is present but water has not yet crossed the threshold —
    /// no event fired this tick.
    Idle { water_depth: f32 },
    /// Sump is absent. Water accumulated per the rain rate; the engine
    /// updates the segment's water_depth field, applies the M3 slippery
    /// flag once depth crosses the threshold, and does NOT emit a flush
    /// event.
    Accumulating { water_depth: f32, slippery: bool },
}

impl DrainageTickOutcome {
    /// Water depth at the end of the tick.
    #[must_use]
    pub fn water_depth_after(&self) -> f32 {
        match self {
            DrainageTickOutcome::Flushed { water_depth_after, .. } => *water_depth_after,
            DrainageTickOutcome::Idle { water_depth } => *water_depth,
            DrainageTickOutcome::Accumulating { water_depth, .. } => *water_depth,
        }
    }

    /// `true` when the sump fired a flush this tick.
    #[must_use]
    pub fn flushed(&self) -> bool {
        matches!(self, DrainageTickOutcome::Flushed { .. })
    }

    /// `true` when the actor footing should be wet-mud (M3 slippery
    /// flag set). Per VAL-M9B-DRAINAGE-002, this only happens when the
    /// sump is absent AND the accumulated depth has crossed the
    /// gameplay threshold.
    #[must_use]
    pub fn slippery_footing(&self) -> bool {
        matches!(
            self,
            DrainageTickOutcome::Accumulating {
                slippery: true,
                ..
            }
        )
    }
}

/// Configuration knobs the engine threads through each tick. The
/// rain-rate is exposed as a parameter so M31 weather can override the
/// default (heavy rain accumulates faster).
#[derive(Debug, Clone, Copy)]
#[allow(clippy::struct_field_names)]
pub struct DrainageEnv {
    pub rain_per_tick_px: f32,
    pub flush_threshold_px: f32,
    pub flush_floor_px: f32,
}

impl Default for DrainageEnv {
    fn default() -> Self {
        Self {
            rain_per_tick_px: RAIN_ACCUMULATION_PER_TICK_PX,
            flush_threshold_px: FLUSH_THRESHOLD_PX,
            flush_floor_px: FLUSH_FLOOR_PX,
        }
    }
}

/// Advance one drainage tick for a single trench-segment water column.
///
/// - `current_depth` is the segment's accumulated water depth in
///   pixels at the start of the tick.
/// - `sump_present` is `true` when the segment has a `drainage_sump`
///   module installed.
/// - `env` carries the rain accumulation rate + flush thresholds.
///
/// The function is **pure**: callers thread the returned
/// [`DrainageTickOutcome`] back into their water column AND emit the
/// flush replay event when [`DrainageTickOutcome::flushed`] is true.
#[must_use]
pub fn drainage_sump_tick(
    current_depth: f32,
    sump_present: bool,
    env: DrainageEnv,
) -> DrainageTickOutcome {
    if sump_present {
        let new_depth = (current_depth + env.rain_per_tick_px).max(0.0);
        if new_depth > env.flush_threshold_px {
            return DrainageTickOutcome::Flushed {
                water_depth_before: new_depth,
                water_depth_after: env.flush_floor_px,
            };
        }
        return DrainageTickOutcome::Idle {
            water_depth: new_depth,
        };
    }
    let new_depth = (current_depth + env.rain_per_tick_px).max(0.0);
    let slippery = new_depth > env.flush_threshold_px;
    DrainageTickOutcome::Accumulating {
        water_depth: new_depth,
        slippery,
    }
}

/// VAL-M9B-DRAINAGE-001 helper: run `ticks` worth of drainage updates
/// against a segment with the sump present, returning every flush
/// event observed plus the final water depth.
///
/// Used by the headless verification scenario `m9b_drainage_flood` so
/// the closure-feature worker can audit "water stays ≤ 2 px for 600
/// ticks" via `cargo test` without launching cf-headless.
#[must_use]
pub fn run_drainage_window(
    initial_depth: f32,
    sump_present: bool,
    ticks: u32,
    env: DrainageEnv,
) -> (Vec<DrainageTickOutcome>, f32) {
    let mut depth = initial_depth;
    let mut events = Vec::new();
    for _ in 0..ticks {
        let outcome = drainage_sump_tick(depth, sump_present, env);
        depth = outcome.water_depth_after();
        if matches!(outcome, DrainageTickOutcome::Flushed { .. }) {
            events.push(outcome);
        }
    }
    (events, depth)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// VAL-M9B-DRAINAGE-001: with the sump present, after 600 ticks of
    /// heavy rain the water depth stays ≤ 2 px AND ≥ 1 flush event has
    /// fired.
    #[test]
    fn drainage_sump_keeps_water_under_threshold_for_600_ticks() {
        let env = DrainageEnv::default();
        let (events, final_depth) = run_drainage_window(0.0, true, 600, env);
        assert!(
            final_depth <= env.flush_threshold_px,
            "water depth {final_depth} must stay <= {} px",
            env.flush_threshold_px
        );
        assert!(
            !events.is_empty(),
            "drainage sump must fire ≥ 1 flush over 600 ticks of rain"
        );
    }

    /// VAL-M9B-DRAINAGE-002: without the sump, after 600 ticks the
    /// water has crossed the threshold AND footing is slippery.
    #[test]
    fn no_sump_floods_within_600_ticks_and_sets_slippery() {
        let env = DrainageEnv::default();
        let (events, final_depth) = run_drainage_window(0.0, false, 600, env);
        assert!(
            events.is_empty(),
            "no sump must NOT fire flush events"
        );
        assert!(
            final_depth > env.flush_threshold_px,
            "no sump: depth {final_depth} must exceed {} after 600 ticks",
            env.flush_threshold_px
        );
        let outcome = drainage_sump_tick(final_depth, false, env);
        assert!(
            outcome.slippery_footing(),
            "no-sump tick at depth {final_depth} must mark footing slippery"
        );
    }

    /// Demolish sequence: with sump present water stays low; then
    /// demolish (sump_present=false) and accumulation begins.
    #[test]
    fn demolish_sump_transitions_from_idle_to_accumulating() {
        let env = DrainageEnv::default();
        let (_events, mid_depth) = run_drainage_window(0.0, true, 600, env);
        assert!(mid_depth <= env.flush_threshold_px);
        // Demolish: next 600 ticks have NO sump.
        let (events_after, final_depth) =
            run_drainage_window(mid_depth, false, 600, env);
        assert!(events_after.is_empty());
        assert!(
            final_depth > env.flush_threshold_px,
            "post-demolish depth {final_depth} must cross threshold"
        );
    }

    #[test]
    fn idle_outcome_when_water_below_threshold() {
        let env = DrainageEnv::default();
        let outcome = drainage_sump_tick(0.0, true, env);
        assert!(matches!(outcome, DrainageTickOutcome::Idle { .. }));
        assert!(!outcome.flushed());
        assert!(!outcome.slippery_footing());
    }

    #[test]
    fn flush_drains_to_floor_value() {
        let env = DrainageEnv::default();
        // Force a high depth so flush is guaranteed this tick.
        let outcome = drainage_sump_tick(3.0, true, env);
        match outcome {
            DrainageTickOutcome::Flushed {
                water_depth_before,
                water_depth_after,
            } => {
                assert!(water_depth_before > env.flush_threshold_px);
                assert_eq!(water_depth_after, env.flush_floor_px);
            }
            other => panic!("expected Flushed, got {other:?}"),
        }
    }

    #[test]
    fn accumulating_outcome_marks_slippery_after_threshold() {
        let env = DrainageEnv::default();
        let outcome = drainage_sump_tick(2.5, false, env);
        match outcome {
            DrainageTickOutcome::Accumulating { water_depth, slippery } => {
                assert!(water_depth > env.flush_threshold_px);
                assert!(slippery);
            }
            other => panic!("expected Accumulating slippery, got {other:?}"),
        }
    }

    #[test]
    fn accumulating_outcome_not_slippery_below_threshold() {
        let env = DrainageEnv::default();
        let outcome = drainage_sump_tick(0.5, false, env);
        match outcome {
            DrainageTickOutcome::Accumulating { water_depth, slippery } => {
                assert!(water_depth < env.flush_threshold_px);
                assert!(!slippery);
            }
            other => panic!("expected Accumulating non-slippery, got {other:?}"),
        }
    }
}
