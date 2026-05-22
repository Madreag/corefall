//! `MissionState` — the per-mission live state machine. Split out of `lib.rs`
//! for the 2k-LOC ceiling. Public API is re-exported at the crate root.

use serde::{Deserialize, Serialize};

use crate::loss::{LossConditions, LossReason};
use crate::objective_types::{Objective, ObjectiveStatus};
use crate::result::{MissionLifecycle, MissionResult};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MissionState {
    /// Empty string for legacy callers that didn't populate it; the engine
    /// sets this from the loaded scenario.
    #[serde(default)]
    pub id: String,
    /// (`Init → Loaded → InProgress → Resolved`). Distinct from `result`
    /// (which is the outcome SHAPE when `lifecycle == Resolved`).
    #[serde(default)]
    pub lifecycle: MissionLifecycle,
    pub objectives: Vec<Objective>,
    pub started_at_tick: u64,
    pub time_limit_ticks: u64,
    pub loss: LossConditions,
    pub result: MissionResult,
    pub last_event_tick: u64,
    pub last_event_label: String,
    /// Tick of the most recent objective or result state transition.
    #[serde(default)]
    pub last_transition_tick: u64,
    /// Typed loss reason vocabulary for stable replay/analytics. Populated from
    /// `LossReason::as_str()` when the mission resolves as Lost.
    #[serde(default)]
    pub loss_reason_label: Option<String>,
    /// the as_str() label). Populated alongside `loss_reason_label` so
    /// consumers can access the structured payload (e.g.
    /// `LossReason::ObjectiveFailed { id, reason }`).
    #[serde(default)]
    pub loss_reason: Option<LossReason>,
    /// no-op AND elapsed-tick accounting skips the paused duration so the
    /// mission timer does NOT advance. Toggled via `MissionState::pause()`
    /// / `resume()` so the engine can wire `act.mission.{pause,resume}`
    /// cfctl methods + emit `mission.objective_paused` / `objective_resumed`
    /// events.
    #[serde(default)]
    pub paused: bool,
    /// not paused; populated by `pause()`. `resume()` uses this to
    /// accumulate `total_paused_ticks` and then clears the field.
    #[serde(default)]
    pub pause_started_at_tick: Option<u64>,
    /// `ticks_remaining` subtract this so the timer truly freezes while
    /// the modal is up.
    #[serde(default)]
    pub total_paused_ticks: u64,
    /// the engine when the mission resolves as Lost; points at the player's
    /// last `input.intent_received` event so M3B's replay viewer can rewind
    /// to the divergence tick. Stays `None` for Won / Aborted / Active.
    #[serde(default)]
    pub show_me_why_event_id: Option<String>,
    /// mission-resolved modal when `true`. Latched from the
    /// mission_resolved event payload's `show_replay_cta` flag.
    #[serde(default)]
    pub show_replay_cta: bool,
}

impl MissionState {
    pub fn new(objectives: Vec<Objective>, started_at_tick: u64, loss: LossConditions) -> Self {
        // BP2 fix: leave all objectives Pending. The first call to `step()`
        // activates the first pending objective AND emits `mission.objective_started`
        // through the same code path that activates subsequent objectives. Without
        // this, the FIRST objective transitioned Pending → Active inside `new()`
        // (with no MissionTickReport in scope), so `mission.objective_started`
        // never fired for it — the engine only saw the second + later objectives'
        // started events. The bp2 test-coverage analyzer caught this gap by
        // cross-referencing the manifest's `required_events_emitted` list against
        // the M2.5 win bundle's events.jsonl.
        Self {
            id: String::new(),
            lifecycle: MissionLifecycle::Loaded,
            objectives,
            started_at_tick,
            time_limit_ticks: loss.time_limit_ticks,
            loss,
            result: MissionResult::InProgress,
            last_event_tick: started_at_tick,
            last_event_label: "mission_started".to_string(),
            last_transition_tick: started_at_tick,
            loss_reason_label: None,
            loss_reason: None,
            paused: false,
            pause_started_at_tick: None,
            total_paused_ticks: 0,
            show_me_why_event_id: None,
            show_replay_cta: false,
        }
    }

    /// objective (if any), per the spec's `current_objective_id` field. Walks
    /// objectives in order, returning the first `Active`.
    pub fn current_objective_id(&self) -> Option<&str> {
        self.objectives
            .iter()
            .find(|o| o.status == ObjectiveStatus::Active)
            .map(|o| o.id.as_str())
    }

    /// declaration order, per the spec's `completed_objectives[]` field.
    pub fn completed_objective_ids(&self) -> Vec<String> {
        self.objectives
            .iter()
            .filter(|o| o.status == ObjectiveStatus::Completed)
            .map(|o| o.id.clone())
            .collect()
    }

    /// declaration order, per the spec's `failed_objectives[]` field.
    pub fn failed_objective_ids(&self) -> Vec<String> {
        self.objectives
            .iter()
            .filter(|o| o.status == ObjectiveStatus::Failed)
            .map(|o| o.id.clone())
            .collect()
    }

    /// Returns the id of the currently active objective (if any) so the
    /// caller can emit `mission.objective_paused { objective: <id> }`.
    /// No-op (returns None) if the mission is terminal or already paused.
    pub fn pause(&mut self, current_tick: u64) -> Option<String> {
        if self.result.is_terminal() || self.paused {
            return None;
        }
        self.paused = true;
        self.pause_started_at_tick = Some(current_tick);
        self.last_event_tick = current_tick;
        self.last_event_label = "objective_paused".to_string();
        self.last_transition_tick = current_tick;
        self.active_objective_id()
    }

    /// `total_paused_ticks` so timer reads correctly. Returns the id of
    /// the active objective so the engine can emit
    /// `mission.objective_resumed { objective: <id> }`. No-op (returns
    /// None) if the mission is not paused.
    pub fn resume(&mut self, current_tick: u64) -> Option<String> {
        if !self.paused {
            return None;
        }
        if let Some(started) = self.pause_started_at_tick.take() {
            self.total_paused_ticks = self
                .total_paused_ticks
                .saturating_add(current_tick.saturating_sub(started));
        }
        self.paused = false;
        self.last_event_tick = current_tick;
        self.last_event_label = "objective_resumed".to_string();
        self.last_transition_tick = current_tick;
        self.active_objective_id()
    }

    fn active_objective_id(&self) -> Option<String> {
        self.active_objective_index().map(|i| self.objectives[i].id.clone())
    }

    /// Reset the mission to its starting state. Used by `scenario.reset` so the
    /// engine can rewind objectives + result + timer without rebuilding from the
    /// scenario manifest.
    pub fn reset(&mut self, started_at_tick: u64) {
        for o in &mut self.objectives {
            o.status = ObjectiveStatus::Pending;
            o.progress_milestone_index = 0;
        }
        // Same BP2 fix as `new()`: do NOT activate the first objective here.
        // step() handles the activation on its next call so the `objective_started`
        // event for the first objective fires through the same path as later ones.
        self.started_at_tick = started_at_tick;
        self.result = MissionResult::InProgress;
        self.last_event_tick = started_at_tick;
        self.last_event_label = "mission_started".to_string();
        self.last_transition_tick = started_at_tick;
        self.loss_reason_label = None;
        self.paused = false;
        self.pause_started_at_tick = None;
        self.total_paused_ticks = 0;
        self.show_me_why_event_id = None;
        self.show_replay_cta = false;
    }

    /// Number of required objectives still in `Pending` or `Active` status.
    pub fn outstanding_required(&self) -> usize {
        self.objectives
            .iter()
            .filter(|o| !o.optional && !o.status.is_terminal())
            .count()
    }

    /// Number of required objectives in `Completed` status.
    pub fn completed_required(&self) -> usize {
        self.objectives
            .iter()
            .filter(|o| !o.optional && o.status == ObjectiveStatus::Completed)
            .count()
    }

    /// Number of required objectives in `Failed` status.
    pub fn failed_required(&self) -> usize {
        self.objectives
            .iter()
            .filter(|o| !o.optional && o.status == ObjectiveStatus::Failed)
            .count()
    }

    /// Index of the currently-active required objective (i.e. the next `Active`
    /// row), if any.
    pub fn active_objective_index(&self) -> Option<usize> {
        self.objectives.iter().position(|o| o.status == ObjectiveStatus::Active)
    }

    /// Ticks elapsed since `started_at_tick`. Saturates at 0.
    /// in-flight (if `paused`) so the timer freezes while a tutorial
    /// modal is up.
    pub fn elapsed_ticks(&self, current_tick: u64) -> u64 {
        let raw = current_tick.saturating_sub(self.started_at_tick);
        let mut pause_credit = self.total_paused_ticks;
        if self.paused {
            if let Some(started) = self.pause_started_at_tick {
                pause_credit = pause_credit.saturating_add(current_tick.saturating_sub(started));
            }
        }
        raw.saturating_sub(pause_credit)
    }

    /// Ticks remaining before the timer expires. `None` when no timer is set.
    pub fn ticks_remaining(&self, current_tick: u64) -> Option<u64> {
        if self.time_limit_ticks == 0 {
            None
        } else {
            Some(self.time_limit_ticks.saturating_sub(self.elapsed_ticks(current_tick)))
        }
    }
}
