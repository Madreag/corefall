//! Mission lifecycle + outcome enums — `MissionLifecycle` + `MissionResult`.
//! Split out of `lib.rs` for the 2k-LOC ceiling. Public API is re-exported
//! at the crate root.

use serde::{Deserialize, Serialize};

use crate::loss::LossReason;

/// literal "Mission state machine: Init → Loaded → InProgress → Resolved".
/// Independent from `MissionResult` — `MissionResult` is the OUTCOME shape
/// when `lifecycle == Resolved`. Transitions:
/// - `Init` → `Loaded` on scenario load (MissionState constructed)
/// - `Loaded` → `InProgress` on first tick / mission_started event
/// - `InProgress` → `Resolved` on mission_resolved event (Won/Lost/Aborted)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MissionLifecycle {
    /// Pre-scenario-load (no MissionState exists in this state — kept for
    /// symmetry with the spec wording).
    Init,
    /// Scenario loaded; objectives present; tick 0 not yet fired.
    #[default]
    Loaded,
    /// `mission.mission_started` event has fired.
    InProgress,
    /// `mission.mission_resolved` event has fired. The resolution shape
    /// (Won / Lost / Aborted) lives on `MissionState.result`.
    Resolved,
}

impl MissionLifecycle {
    pub fn as_str(self) -> &'static str {
        match self {
            MissionLifecycle::Init => "init",
            MissionLifecycle::Loaded => "loaded",
            MissionLifecycle::InProgress => "in_progress",
            MissionLifecycle::Resolved => "resolved",
        }
    }
}

/// spec literal "MissionResult::{InProgress, Won, Lost, Aborted}".
/// `serde(rename_all = "snake_case")` makes the wire value `"in_progress"`
/// — the prior `"active"` was renamed in lockstep.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "result")]
pub enum MissionResult {
    InProgress,
    Won,
    Lost {
        reason: LossReason,
    },
    /// Player-initiated mission abandonment via `act.player.abort`.
    Aborted,
}

impl MissionResult {
    pub fn as_str(&self) -> &'static str {
        match self {
            MissionResult::InProgress => "in_progress",
            MissionResult::Won => "won",
            MissionResult::Lost { .. } => "lost",
            MissionResult::Aborted => "aborted",
        }
    }

    pub fn is_terminal(&self) -> bool {
        !matches!(self, MissionResult::InProgress)
    }
}
