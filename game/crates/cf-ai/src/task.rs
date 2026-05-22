//! M7-A: 22-task `TaskType` enum.
//!
//! The 22-task list is locked at M8A in M8A.md § AI foundation. M7-A declares
//! the enum here so the 5-layer thinking stack + per-archetype Priority Table
//! role templates can reference it during M7. M8A's lock guarantees the
//! ordinal positions stay stable for replay determinism + persistence.
//!
//! M7 workers MUST NOT add / remove / re-order variants. M7-B and downstream
//! milestones EXTEND via additive helpers, never modify the enum itself.

use serde::{Deserialize, Serialize};

/// One-to-one with the Priority Table grid columns. Stable ordinal ↔ wire
/// representation (snake_case in JSON) — replay bundles store the names.
///
/// Locked at M8A. M7-A pre-declares to unblock the 5-layer stack.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    EngageVisibleEnemy = 0,
    SuppressFire = 1,
    HoldCover = 2,
    FlankTarget = 3,
    ThrowGrenade = 4,
    Demolish = 5,
    SharpshootTarget = 6,
    MarkThreats = 7,
    ScoutAhead = 8,
    RepairChassisModule = 9,
    RepairTerrainBreach = 10,
    SetTrap = 11,
    DigCover = 12,
    TriageDownedAlly = 13,
    HealSelf = 14,
    CoverAlly = 15,
    DefendBrainActor = 16,
    InvestigateSound = 17,
    FollowOrder = 18,
    RetreatToCover = 19,
    Patrol = 20,
    Idle = 21,
}

impl TaskType {
    /// remain exactly 22 across the project's lifetime.
    pub const ALL: [TaskType; 22] = [
        TaskType::EngageVisibleEnemy,
        TaskType::SuppressFire,
        TaskType::HoldCover,
        TaskType::FlankTarget,
        TaskType::ThrowGrenade,
        TaskType::Demolish,
        TaskType::SharpshootTarget,
        TaskType::MarkThreats,
        TaskType::ScoutAhead,
        TaskType::RepairChassisModule,
        TaskType::RepairTerrainBreach,
        TaskType::SetTrap,
        TaskType::DigCover,
        TaskType::TriageDownedAlly,
        TaskType::HealSelf,
        TaskType::CoverAlly,
        TaskType::DefendBrainActor,
        TaskType::InvestigateSound,
        TaskType::FollowOrder,
        TaskType::RetreatToCover,
        TaskType::Patrol,
        TaskType::Idle,
    ];

    pub const COUNT: usize = 22;

    pub fn as_str(self) -> &'static str {
        match self {
            TaskType::EngageVisibleEnemy => "engage_visible_enemy",
            TaskType::SuppressFire => "suppress_fire",
            TaskType::HoldCover => "hold_cover",
            TaskType::FlankTarget => "flank_target",
            TaskType::ThrowGrenade => "throw_grenade",
            TaskType::Demolish => "demolish",
            TaskType::SharpshootTarget => "sharpshoot_target",
            TaskType::MarkThreats => "mark_threats",
            TaskType::ScoutAhead => "scout_ahead",
            TaskType::RepairChassisModule => "repair_chassis_module",
            TaskType::RepairTerrainBreach => "repair_terrain_breach",
            TaskType::SetTrap => "set_trap",
            TaskType::DigCover => "dig_cover",
            TaskType::TriageDownedAlly => "triage_downed_ally",
            TaskType::HealSelf => "heal_self",
            TaskType::CoverAlly => "cover_ally",
            TaskType::DefendBrainActor => "defend_brain_actor",
            TaskType::InvestigateSound => "investigate_sound",
            TaskType::FollowOrder => "follow_order",
            TaskType::RetreatToCover => "retreat_to_cover",
            TaskType::Patrol => "patrol",
            TaskType::Idle => "idle",
        }
    }

    pub fn from_str(value: &str) -> Option<TaskType> {
        Some(match value {
            "engage_visible_enemy" => TaskType::EngageVisibleEnemy,
            "suppress_fire" => TaskType::SuppressFire,
            "hold_cover" => TaskType::HoldCover,
            "flank_target" => TaskType::FlankTarget,
            "throw_grenade" => TaskType::ThrowGrenade,
            "demolish" => TaskType::Demolish,
            "sharpshoot_target" => TaskType::SharpshootTarget,
            "mark_threats" => TaskType::MarkThreats,
            "scout_ahead" => TaskType::ScoutAhead,
            "repair_chassis_module" => TaskType::RepairChassisModule,
            "repair_terrain_breach" => TaskType::RepairTerrainBreach,
            "set_trap" => TaskType::SetTrap,
            "dig_cover" => TaskType::DigCover,
            "triage_downed_ally" => TaskType::TriageDownedAlly,
            "heal_self" => TaskType::HealSelf,
            "cover_ally" => TaskType::CoverAlly,
            "defend_brain_actor" => TaskType::DefendBrainActor,
            "investigate_sound" => TaskType::InvestigateSound,
            "follow_order" => TaskType::FollowOrder,
            "retreat_to_cover" => TaskType::RetreatToCover,
            "patrol" => TaskType::Patrol,
            "idle" => TaskType::Idle,
            _ => return None,
        })
    }

    pub fn ordinal(self) -> usize {
        self as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_count_is_locked_at_22() {
        assert_eq!(TaskType::COUNT, 22);
        assert_eq!(TaskType::ALL.len(), 22);
    }

    #[test]
    fn all_ordinals_unique_and_monotonic() {
        for (i, task) in TaskType::ALL.iter().enumerate() {
            assert_eq!(task.ordinal(), i);
        }
    }

    #[test]
    fn round_trip_str() {
        for task in TaskType::ALL.iter() {
            let s = task.as_str();
            assert_eq!(TaskType::from_str(s), Some(*task));
        }
    }
}
