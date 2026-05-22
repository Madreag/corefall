use serde::{Deserialize, Serialize};

/// Discrete states the guard can be in. The engine emits an `ai.state_changed`
/// event whenever this changes.
///
/// Dying → Dead`). `Retreating` fires when the guard's hp drops below
/// `retreat_hp_pct * max_hp` (default 30%); `Dying` mirrors the actor body
/// state machine's 1000ms DYING dwell so the AI surface stays synchronised
/// with the body surface even while the actor is being torn down.
///
/// The serde name for the previously-called `Alerted` variant is now `alert`
/// to match the spec text (`ai.state_changed { to: "alert", ... }`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardState {
    Idle,
    Alert,
    Engaged,
    /// reload + cover-seeking tactics over Attack.
    Retreating,
    /// Auto-transitions to Dead when the actor's body state machine
    /// completes its DYING dwell.
    Dying,
    Dead,
}

impl GuardState {
    pub fn as_str(self) -> &'static str {
        match self {
            GuardState::Idle => "idle",
            GuardState::Alert => "alert",
            GuardState::Engaged => "engaged",
            GuardState::Retreating => "retreating",
            GuardState::Dying => "dying",
            GuardState::Dead => "dead",
        }
    }
}

/// Tactic the utility scorer chose this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tactic {
    /// Standing by; no target.
    Hold,
    /// acquisition. Spec literal: "ai.tactic_chosen fires with
    /// tactic='aim_settle', reason='initial_acquisition'". The guard does
    /// NOT fire during this tactic (try_fire returns None while
    /// `aim_settle_remaining_ticks > 0`).
    AimSettle,
    /// Aim and (eventually) fire at the player.
    Attack,
    /// Reload the magazine.
    Reload,
    /// Lost sight; investigate / dwell.
    Search,
}

impl Tactic {
    pub fn as_str(self) -> &'static str {
        match self {
            Tactic::Hold => "hold",
            Tactic::AimSettle => "aim_settle",
            Tactic::Attack => "attack",
            Tactic::Reload => "reload",
            Tactic::Search => "search",
        }
    }
}

/// Recorded `ai.state_changed` payload.
#[derive(Debug, Clone, PartialEq)]
pub struct GuardStateTransition {
    pub previous: GuardState,
    pub next: GuardState,
    pub cause: String,
}
