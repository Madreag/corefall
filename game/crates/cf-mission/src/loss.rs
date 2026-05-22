//! Mission loss vocabulary — `LossReason` + `LossConditions`. Split out of
//! `lib.rs` for the 2k-LOC ceiling. Public API is re-exported at the crate
//! root.

use serde::{Deserialize, Serialize};

/// Mission outcome reason once `Lost`. M1.5 only needs two reasons; M7 adds more
/// (objective_failed, ally_lost, command_core_destroyed, etc.).
///
/// objective id + a reason label per the spec literal "ObjectiveFailed {
/// id, reason }". `Aborted` variant added so the abort path doesn't have
/// to route through a raw string literal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum LossReason {
    PlayerDead,
    TimerExpired,
    /// M2.5: a `defend_reactor` objective failed because the reactor was
    /// destroyed before the mission timer expired.
    ReactorDestroyed,
    /// M2: a player-tracked objective failed.
    ObjectiveFailed {
        id: String,
        reason: String,
    },
    /// M2: player-initiated mission abandonment via `act.player.abort`.
    Aborted,
    /// Brain death = mission lost regardless of which actor is currently
    /// being puppeted.
    BrainDestroyed,
}

impl LossReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            LossReason::PlayerDead => "player_dead",
            LossReason::TimerExpired => "timer_expired",
            LossReason::ReactorDestroyed => "reactor_destroyed",
            LossReason::ObjectiveFailed { .. } => "objective_failed",
            LossReason::Aborted => "aborted",
            LossReason::BrainDestroyed => "brain_destroyed",
        }
    }

    /// failing objective id; otherwise `None`. Used by replay viewers and
    /// debrief markdown.
    pub fn objective_id(&self) -> Option<&str> {
        match self {
            LossReason::ObjectiveFailed { id, .. } => Some(id),
            _ => None,
        }
    }

    /// failure reason label; otherwise `None`.
    pub fn objective_reason(&self) -> Option<&str> {
        match self {
            LossReason::ObjectiveFailed { reason, .. } => Some(reason),
            _ => None,
        }
    }
}

/// Loss conditions for the M1.5 micro breach scenario. M7 will replace this with
/// the typed mission director's failure graph.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LossConditions {
    /// True if the player dying ends the mission as Lost(`PlayerDead`).
    #[serde(default = "default_true")]
    pub player_dead: bool,
    /// Optional time limit in ticks. `0` = no limit.
    #[serde(default)]
    pub time_limit_ticks: u64,
}

pub(crate) fn default_true() -> bool {
    true
}

impl Default for LossConditions {
    fn default() -> Self {
        Self {
            player_dead: true,
            time_limit_ticks: 0,
        }
    }
}
