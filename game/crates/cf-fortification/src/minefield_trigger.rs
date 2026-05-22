//! M9C minefield kernel — per-actor trigger evaluation (proximity,
//! pressure, tripwire, IED chain).

use crate::minefield_types::{
    Mine, MineKind, MineTriggerCause, MINE_PROXIMITY_TRIGGER_DECITILES,
};

/// One observed actor candidate for trigger evaluation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActorCandidate {
    pub actor_id: u64,
    pub pos_tiles: (i32, i32),
    /// True when the actor is in Standing or Crouched stance (per
    /// spec § "Pressure mine triggers on Standing/Crouched over tile").
    pub standing_or_crouched: bool,
    /// True when this actor's footprint crossed the tripwire line
    /// this tick.
    pub crossed_tripwire: bool,
    /// True when the actor is hostile to the mine's owner.
    pub hostile_to_owner: bool,
}

/// Outcome of a single per-actor trigger check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerOutcome {
    NotArmed,
    NoTrigger,
    Triggered(MineTriggerCause),
}

impl TriggerOutcome {
    #[must_use]
    pub const fn triggered(self) -> bool {
        matches!(self, TriggerOutcome::Triggered(_))
    }
}

/// Integer squared distance helper used by the proximity radius
/// check (kept integer so the predicate is deterministic across
/// architectures + FP rounding modes).
#[must_use]
fn squared_distance_decitiles(a: (i32, i32), b: (i32, i32)) -> i64 {
    let dx = i64::from(a.0 - b.0) * 10;
    let dy = i64::from(a.1 - b.1) * 10;
    dx * dx + dy * dy
}

/// Determine whether `point` lies on or near the segment between
/// `start` and `end` within `tolerance_tiles`. Integer-arithmetic
/// only.
#[must_use]
fn point_on_segment_tiles(
    point: (i32, i32),
    start: (i32, i32),
    end: (i32, i32),
    tolerance_tiles: u32,
) -> bool {
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let len2 = i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy);
    if len2 == 0 {
        // Degenerate: start == end → point-distance check.
        let pdx = i64::from(point.0 - start.0);
        let pdy = i64::from(point.1 - start.1);
        let tol = i64::from(tolerance_tiles);
        return pdx * pdx + pdy * pdy <= tol * tol;
    }
    let px = i64::from(point.0 - start.0);
    let py = i64::from(point.1 - start.1);
    let t_num = px * i64::from(dx) + py * i64::from(dy);
    let t_clamped = t_num.clamp(0, len2);
    let nearest_x = i64::from(start.0) * len2 + i64::from(dx) * t_clamped;
    let nearest_y = i64::from(start.1) * len2 + i64::from(dy) * t_clamped;
    let dx_to_point = i64::from(point.0) * len2 - nearest_x;
    let dy_to_point = i64::from(point.1) * len2 - nearest_y;
    let dist2 = dx_to_point * dx_to_point + dy_to_point * dy_to_point;
    let tol = i64::from(tolerance_tiles);
    let tol2 = tol * tol * len2 * len2;
    dist2 <= tol2
}

/// Evaluate one mine against a single actor and return the trigger
/// outcome. Used by the engine each tick.
#[must_use]
pub fn evaluate_trigger(mine: &Mine, candidate: ActorCandidate) -> TriggerOutcome {
    if mine.is_inactive() {
        return TriggerOutcome::NotArmed;
    }
    if !candidate.hostile_to_owner {
        return TriggerOutcome::NoTrigger;
    }
    match mine.kind {
        MineKind::MineProximity => {
            let dist2 = squared_distance_decitiles(mine.pos_tiles, candidate.pos_tiles);
            let radius2 = i64::from(MINE_PROXIMITY_TRIGGER_DECITILES)
                * i64::from(MINE_PROXIMITY_TRIGGER_DECITILES);
            if dist2 <= radius2 {
                TriggerOutcome::Triggered(MineTriggerCause::Proximity)
            } else {
                TriggerOutcome::NoTrigger
            }
        }
        MineKind::MinePressure => {
            if candidate.standing_or_crouched
                && candidate.pos_tiles == mine.pos_tiles
            {
                TriggerOutcome::Triggered(MineTriggerCause::Pressure)
            } else {
                TriggerOutcome::NoTrigger
            }
        }
        MineKind::TripwireMine => {
            if candidate.crossed_tripwire {
                return TriggerOutcome::Triggered(MineTriggerCause::Tripwire);
            }
            if let Some((start, end)) = mine.tripwire_endpoints {
                if point_on_segment_tiles(candidate.pos_tiles, start, end, 0) {
                    return TriggerOutcome::Triggered(MineTriggerCause::Tripwire);
                }
            }
            TriggerOutcome::NoTrigger
        }
        MineKind::IedChain => {
            // Spec § "IED chain": triggered by remote OR proximity OR
            // pressure. Without an explicit manual-detonator input we
            // honor proximity + pressure here; the engine routes
            // remote detonations through [`crate::minefield::begin_ied_chain_cascade`].
            if candidate.standing_or_crouched
                && candidate.pos_tiles == mine.pos_tiles
            {
                return TriggerOutcome::Triggered(MineTriggerCause::Pressure);
            }
            let dist2 = squared_distance_decitiles(mine.pos_tiles, candidate.pos_tiles);
            let radius2 = i64::from(MINE_PROXIMITY_TRIGGER_DECITILES)
                * i64::from(MINE_PROXIMITY_TRIGGER_DECITILES);
            if dist2 <= radius2 {
                TriggerOutcome::Triggered(MineTriggerCause::Proximity)
            } else {
                TriggerOutcome::NoTrigger
            }
        }
    }
}
