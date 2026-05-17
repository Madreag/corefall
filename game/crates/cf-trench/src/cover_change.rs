//! M9B: cover-state change-event emission.
//!
//! Spec §"Acceptance criteria":
//!
//! > Given a player standing inside a deep trench segment
//! > When observe.actor.cover_state is read
//! > Then it returns "Full"
//! > When the player switches to fire_step variant by moving to that segment
//! > And remains standing on the step
//! > Then cover_state returns "Exposed"
//! > When the player crouches off the step
//! > Then cover_state returns "Full"
//! > And trench_cover_state_changed event fires on each transition
//!
//! VAL-M9B-COVER-002: `trench.cover_state_changed` event fires on
//! segment boundary crossing OR stance change with fields `actor_id`,
//! `prev_state`, `new_state`, `cause` (one of `segment_boundary` |
//! `stance_change`).
//!
//! This module owns the pure derivation: given (previous, new,
//! previous_segment, new_segment), what (if any) event should fire?
//! The engine drives the per-tick comparison and writes the resulting
//! event through `cf-replay::Recorder`.

use crate::cover_state::CoverState;
use crate::segment::SegmentVariant;

/// Reason the event fired — used to populate the `cause` field on
/// `trench.cover_state_changed`.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum CoverStateChangeCause {
    /// Actor crossed a segment boundary (entered, exited, OR moved
    /// between two segments with different variants).
    SegmentBoundary,
    /// Actor's stance changed inside the same segment.
    StanceChange,
    /// Both boundary AND stance changed within the same tick — the
    /// engine records this as a single event with `cause="segment_boundary"`
    /// (segment_boundary is the dominant cause when both fired) so the
    /// replay stream stays linear.
    Combined,
}

impl CoverStateChangeCause {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            CoverStateChangeCause::SegmentBoundary => "segment_boundary",
            CoverStateChangeCause::StanceChange => "stance_change",
            CoverStateChangeCause::Combined => "segment_boundary",
        }
    }
}

/// Pure-data record returned by [`cover_state_change`]. When the cover
/// state has not changed the function returns `None`.
#[derive(Debug, Clone, PartialEq)]
pub struct CoverStateChangeEvent {
    pub prev_state: CoverState,
    pub new_state: CoverState,
    pub prev_segment_variant: Option<SegmentVariant>,
    pub new_segment_variant: Option<SegmentVariant>,
    pub cause: CoverStateChangeCause,
}

impl CoverStateChangeEvent {
    /// Stable wire-format string for the cause field on the event JSON.
    #[must_use]
    pub fn cause_str(&self) -> &'static str {
        self.cause.as_str()
    }
}

/// Compute the cover-state-change event payload from the previous and
/// new (cover_state, segment_variant) pairs.
///
/// Returns `Some(event)` when the cover state changed, `None` when it
/// did not. The caller (engine) decides whether to emit the event into
/// `cf-replay` based on the segment-vs-stance change axis.
///
/// `prev_segment` and `new_segment` are `None` when the actor is on
/// open ground (no segment under foot).
#[must_use]
pub fn cover_state_change(
    prev_state: CoverState,
    new_state: CoverState,
    prev_segment: Option<SegmentVariant>,
    new_segment: Option<SegmentVariant>,
    stance_changed: bool,
) -> Option<CoverStateChangeEvent> {
    if prev_state == new_state {
        return None;
    }
    let segment_changed = prev_segment != new_segment;
    let cause = match (segment_changed, stance_changed) {
        (true, true) => CoverStateChangeCause::Combined,
        (true, false) => CoverStateChangeCause::SegmentBoundary,
        (false, true) => CoverStateChangeCause::StanceChange,
        // Both unchanged but state changed — defensive default: treat
        // as stance_change so the replay stream still records the
        // transition. In practice cover_state cannot change without
        // either axis moving; the matching `if prev_state == new_state`
        // early-return rules this branch out at runtime.
        (false, false) => CoverStateChangeCause::StanceChange,
    };
    Some(CoverStateChangeEvent {
        prev_state,
        new_state,
        prev_segment_variant: prev_segment,
        new_segment_variant: new_segment,
        cause,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_change_returns_none() {
        assert!(cover_state_change(
            CoverState::Partial,
            CoverState::Partial,
            Some(SegmentVariant::Standard),
            Some(SegmentVariant::Standard),
            false
        )
        .is_none());
    }

    /// VAL-M9B-COVER-002: segment boundary cross fires the event with
    /// `cause="segment_boundary"`.
    #[test]
    fn cover_state_change_event_on_boundary_cross() {
        let ev = cover_state_change(
            CoverState::Exposed,
            CoverState::Full,
            None,
            Some(SegmentVariant::Deep),
            false,
        )
        .expect("boundary cross must fire event");
        assert_eq!(ev.prev_state, CoverState::Exposed);
        assert_eq!(ev.new_state, CoverState::Full);
        assert_eq!(ev.cause_str(), "segment_boundary");
        assert_eq!(ev.prev_segment_variant, None);
        assert_eq!(ev.new_segment_variant, Some(SegmentVariant::Deep));
    }

    /// VAL-M9B-COVER-002: stance change inside the same segment fires
    /// the event with `cause="stance_change"`.
    #[test]
    fn cover_state_change_event_on_stance_change() {
        let ev = cover_state_change(
            CoverState::Partial,
            CoverState::Full,
            Some(SegmentVariant::Standard),
            Some(SegmentVariant::Standard),
            true,
        )
        .expect("stance change must fire event");
        assert_eq!(ev.cause_str(), "stance_change");
        assert_eq!(ev.prev_segment_variant, ev.new_segment_variant);
    }

    /// Boundary + stance combined collapses to `segment_boundary` so
    /// downstream consumers see a single linear cause label.
    #[test]
    fn cover_state_change_combined_collapses_to_segment_boundary() {
        let ev = cover_state_change(
            CoverState::Exposed,
            CoverState::Full,
            None,
            Some(SegmentVariant::Deep),
            true,
        )
        .unwrap();
        assert_eq!(ev.cause_str(), "segment_boundary");
        assert!(matches!(ev.cause, CoverStateChangeCause::Combined));
    }

    #[test]
    fn cover_state_change_segment_to_open_ground() {
        let ev = cover_state_change(
            CoverState::Full,
            CoverState::Exposed,
            Some(SegmentVariant::Deep),
            None,
            false,
        )
        .unwrap();
        assert_eq!(ev.cause_str(), "segment_boundary");
        assert_eq!(ev.new_segment_variant, None);
    }
}
