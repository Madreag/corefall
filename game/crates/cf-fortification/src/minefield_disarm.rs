//! M9C minefield kernel — manual disarm hold-state + robot disarm
//! tick budgets. 6s crouched + adjacent + [E]; 4s robot mechanical
//! arm.

use crate::common::FortificationId;
use crate::minefield_types::{
    DisarmFailureCause, DisarmResult, MineDisarmedEvent,
    MANUAL_DISARM_SECONDS, MINE_DISARMED_EXPLOSIVE_RECOVERED, ROBOT_DISARM_SECONDS,
};

/// Inputs to one tick of a manual-disarm hold-state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DisarmInputs {
    pub mine_id: FortificationId,
    pub actor_id: u64,
    pub crouched: bool,
    pub adjacent: bool,
    pub holding_e: bool,
    pub took_damage_this_tick: bool,
    pub moved_this_tick: bool,
    /// Ticks the actor has held the disarm gesture so far (driven by
    /// the engine; the kernel computes the boundary).
    pub hold_ticks: u32,
    /// Tick budget required (per [`MANUAL_DISARM_SECONDS`] *
    /// tick_rate_hz).
    pub required_ticks: u32,
}

/// Result of one manual-disarm tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisarmTickResult {
    /// Hold continues; no event emitted this tick.
    Holding { hold_ticks: u32 },
    /// Disarm completed: emit `mine_disarmed { result: ok }` +
    /// recover [`MINE_DISARMED_EXPLOSIVE_RECOVERED`] components.
    Disarmed(MineDisarmedEvent),
    /// Disarm failed: emit `mine_disarmed { result: failed, cause }`.
    /// The mine remains armed.
    Failed(MineDisarmedEvent),
}

/// Compute the required hold tick budget for a manual disarm given
/// the engine's tick rate.
#[must_use]
pub fn manual_disarm_required_ticks(tick_rate_hz: u32) -> u32 {
    MANUAL_DISARM_SECONDS.saturating_mul(tick_rate_hz)
}

/// Compute the required hold tick budget for the bomb-disposal-robot
/// mechanical-arm disarm.
#[must_use]
pub fn robot_disarm_required_ticks(tick_rate_hz: u32) -> u32 {
    ROBOT_DISARM_SECONDS.saturating_mul(tick_rate_hz)
}

/// Drive one tick of a manual disarm. Engine consumes the result to
/// emit events / advance the hold timer.
#[must_use]
pub fn tick_manual_disarm(inputs: DisarmInputs, tick_index: u64) -> DisarmTickResult {
    // Interrupt rules (per spec § "Manual disarm: interrupt (move /
    // damage / release) → mine_disarm_failed").
    if inputs.took_damage_this_tick {
        return DisarmTickResult::Failed(MineDisarmedEvent {
            mine_id: inputs.mine_id,
            actor_id: Some(inputs.actor_id),
            result: DisarmResult::Failed,
            failure_cause: Some(DisarmFailureCause::ActorDamaged),
            explosive_recovered: 0,
            tick_index,
        });
    }
    if inputs.moved_this_tick {
        return DisarmTickResult::Failed(MineDisarmedEvent {
            mine_id: inputs.mine_id,
            actor_id: Some(inputs.actor_id),
            result: DisarmResult::Failed,
            failure_cause: Some(DisarmFailureCause::ActorMoved),
            explosive_recovered: 0,
            tick_index,
        });
    }
    if !inputs.holding_e {
        return DisarmTickResult::Failed(MineDisarmedEvent {
            mine_id: inputs.mine_id,
            actor_id: Some(inputs.actor_id),
            result: DisarmResult::Failed,
            failure_cause: Some(DisarmFailureCause::ActorReleasedE),
            explosive_recovered: 0,
            tick_index,
        });
    }
    if !inputs.crouched || !inputs.adjacent {
        return DisarmTickResult::Failed(MineDisarmedEvent {
            mine_id: inputs.mine_id,
            actor_id: Some(inputs.actor_id),
            result: DisarmResult::Failed,
            failure_cause: Some(DisarmFailureCause::InterruptedOther),
            explosive_recovered: 0,
            tick_index,
        });
    }
    let next = inputs.hold_ticks.saturating_add(1);
    if next >= inputs.required_ticks {
        DisarmTickResult::Disarmed(MineDisarmedEvent {
            mine_id: inputs.mine_id,
            actor_id: Some(inputs.actor_id),
            result: DisarmResult::Ok,
            failure_cause: None,
            explosive_recovered: MINE_DISARMED_EXPLOSIVE_RECOVERED,
            tick_index,
        })
    } else {
        DisarmTickResult::Holding { hold_ticks: next }
    }
}
