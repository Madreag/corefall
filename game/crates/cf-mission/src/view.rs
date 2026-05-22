//! `MissionView` + `ObjectiveView` — engine observe-envelope projection.
//! Split out of `lib.rs` for the 2k-LOC ceiling. Public API is re-exported
//! at the crate root.

use serde::{Deserialize, Serialize};

use crate::objective_types::{ObjectiveKind, ObjectiveStatus};
use crate::result::MissionResult;
use crate::state::MissionState;

/// Convenience used by the engine to build a per-tick view for the observe
/// envelope. M1.5 keeps it tiny; M4 will wire the comic-noir HUD on top.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MissionView {
    /// `status`. `result` retained as alias because the wire was stable
    /// across the M1-M3 era.
    #[serde(rename = "status")]
    pub result: String,
    pub loss_reason: Option<String>,
    pub elapsed_ticks: u64,
    /// `timer_total_ticks`. `time_limit_ticks` retained as alias.
    #[serde(rename = "timer_total_ticks")]
    pub time_limit_ticks: u64,
    /// `timer_ticks_remaining`.
    #[serde(rename = "timer_ticks_remaining")]
    pub ticks_remaining: Option<u64>,
    /// `current_objective_id`.
    #[serde(rename = "current_objective_id")]
    pub active_objective: Option<String>,
    /// — list of objective ids in completion order.
    #[serde(default)]
    pub completed_objectives: Vec<String>,
    #[serde(default)]
    pub failed_objectives: Vec<String>,
    pub objectives: Vec<ObjectiveView>,
    pub last_event_tick: u64,
    pub last_event_label: String,
    /// when the mission resolves as Lost; cf-ui renders the CTA button
    /// when this field is `Some`.
    #[serde(default)]
    pub show_me_why_event_id: Option<String>,
    /// `true`.
    #[serde(default)]
    pub show_replay_cta: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectiveView {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub optional: bool,
    pub target_actor: Option<u64>,
    pub target_breach: Option<String>,
    pub target_reactor: Option<String>,
    pub zone_min: Option<[f32; 2]>,
    pub zone_max: Option<[f32; 2]>,
}

impl MissionView {
    pub fn from_state(state: &MissionState, current_tick: u64) -> Self {
        let active_objective = state.active_objective_index().map(|i| state.objectives[i].id.clone());
        let loss_reason = match &state.result {
            MissionResult::Lost { reason } => Some(reason.as_str().to_string()),
            _ => None,
        };
        let objectives = state
            .objectives
            .iter()
            .map(|o| ObjectiveView {
                id: o.id.clone(),
                kind: o.kind.category().to_string(),
                status: o.status.as_str().to_string(),
                optional: o.optional,
                target_actor: match &o.kind {
                    ObjectiveKind::NeutralizeActor { target } => Some(*target),
                    _ => None,
                },
                target_breach: match &o.kind {
                    ObjectiveKind::BreachBarrier { target } => Some(target.clone()),
                    _ => None,
                },
                target_reactor: match &o.kind {
                    ObjectiveKind::DefendReactor { target } => Some(target.clone()),
                    _ => None,
                },
                zone_min: match &o.kind {
                    ObjectiveKind::ReachZone { min, .. } => Some(*min),
                    _ => None,
                },
                zone_max: match &o.kind {
                    ObjectiveKind::ReachZone { max, .. } => Some(*max),
                    _ => None,
                },
            })
            .collect();
        // failed_objectives[] arrays so observe.mission carries them in
        // the JSON, per spec MissionState surface.
        let completed_objectives = state
            .objectives
            .iter()
            .filter(|o| o.status == ObjectiveStatus::Completed)
            .map(|o| o.id.clone())
            .collect();
        let failed_objectives = state
            .objectives
            .iter()
            .filter(|o| o.status == ObjectiveStatus::Failed)
            .map(|o| o.id.clone())
            .collect();
        Self {
            result: state.result.as_str().to_string(),
            loss_reason,
            elapsed_ticks: state.elapsed_ticks(current_tick),
            time_limit_ticks: state.time_limit_ticks,
            ticks_remaining: state.ticks_remaining(current_tick),
            active_objective,
            completed_objectives,
            failed_objectives,
            objectives,
            last_event_tick: state.last_event_tick,
            last_event_label: state.last_event_label.clone(),
            show_me_why_event_id: state.show_me_why_event_id.clone(),
            show_replay_cta: state.show_replay_cta,
        }
    }
}
