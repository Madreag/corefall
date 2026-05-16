//! Priority Editor — the 22-task × 1-9 weight grid surfaced inside the
//! Tab tactical overlay. Reads from cf-priority's `PriorityTable`; writes
//! flow back through cf-control's `act.player.set_priority` cfctl method.

use std::collections::BTreeMap;

use cf_priority::{AutonomyMode, PriorityTable, RoleTemplate, TaskType};
use serde::{Deserialize, Serialize};

/// One editor view for a single bot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriorityEditorView {
    /// Actor whose table is being edited.
    pub actor_id: u64,
    /// Display role (drives the "★ role default" badges in the UI).
    pub role: RoleTemplate,
    /// Active autonomy mode.
    pub autonomy: AutonomyMode,
    /// Current per-task weights extracted from the bot's PriorityTable.
    /// `Disabled` (`-`) is encoded as 0. Keys are
    /// [`cf_priority::TaskType::as_str`] snake_case ids.
    pub table_view: BTreeMap<String, u8>,
}

/// One edit the player can apply. Sequences of edits are batched in the
/// UI and dispatched as individual `act.player.set_priority` calls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriorityEditAction {
    /// Task being edited (cf-priority `TaskType::as_str()`).
    pub task_id: String,
    /// New weight in 0..=9.
    pub weight: u8,
}

/// Failure modes for the editor's local validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PriorityEditError {
    /// Weight outside `0..=9`.
    WeightOutOfRange(u8),
    /// Task name unknown to cf-priority.
    UnknownTask(String),
}

impl PriorityEditorView {
    /// Build a view from a snapshot.
    pub fn from(actor_id: u64, role: RoleTemplate, autonomy: AutonomyMode, table: &PriorityTable) -> Self {
        let mut table_view: BTreeMap<String, u8> = BTreeMap::new();
        for task in TaskType::ALL.iter() {
            table_view.insert(task.as_str().to_string(), table.get(*task));
        }
        Self {
            actor_id,
            role,
            autonomy,
            table_view,
        }
    }

    /// Validate a player edit before dispatching it through cfctl.
    pub fn validate(action: &PriorityEditAction) -> Result<TaskType, PriorityEditError> {
        if action.weight > 9 {
            return Err(PriorityEditError::WeightOutOfRange(action.weight));
        }
        TaskType::from_str(&action.task_id).ok_or_else(|| PriorityEditError::UnknownTask(action.task_id.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_snapshot_pulls_every_task() {
        let table = RoleTemplate::Medic.priority_table();
        let view = PriorityEditorView::from(1, RoleTemplate::Medic, AutonomyMode::FullAuto, &table);
        assert_eq!(view.role, RoleTemplate::Medic);
        assert_eq!(view.table_view.len(), TaskType::ALL.len());
        assert_eq!(view.table_view.get("triage_downed_ally"), Some(&9));
    }

    #[test]
    fn validate_rejects_weight_above_9() {
        let action = PriorityEditAction {
            task_id: "triage_downed_ally".into(),
            weight: 10,
        };
        let err = PriorityEditorView::validate(&action).unwrap_err();
        assert_eq!(err, PriorityEditError::WeightOutOfRange(10));
    }

    #[test]
    fn validate_rejects_unknown_task() {
        let action = PriorityEditAction {
            task_id: "no_such_task".into(),
            weight: 5,
        };
        let err = PriorityEditorView::validate(&action).unwrap_err();
        assert_eq!(err, PriorityEditError::UnknownTask("no_such_task".into()));
    }

    #[test]
    fn validate_accepts_valid_action() {
        let action = PriorityEditAction {
            task_id: "engage_visible_enemy".into(),
            weight: 7,
        };
        let task = PriorityEditorView::validate(&action).unwrap();
        assert_eq!(task, TaskType::EngageVisibleEnemy);
    }
}
