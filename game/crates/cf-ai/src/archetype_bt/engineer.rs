//! M7B: Engineer archetype BT.

use crate::behavior_tree::BtNode;
use crate::task::TaskType;

pub const NODES: &[&str] = &[
    "engineer_idle",
    "engineer_scan_for_module_damage",
    "engineer_approach_chassis",
    "engineer_repair_chassis_module",
    "engineer_repair_terrain_breach",
    "engineer_pour_concrete",
    "engineer_seal_pipe",
    "engineer_extinguish_fire",
    "engineer_set_trap",
    "engineer_disarm_trap",
    "engineer_lay_mine",
    "engineer_dig_cover",
    "engineer_dig_breach",
    "engineer_demolish_wall",
    "engineer_breach_door",
    "engineer_stack_left",
    "engineer_stack_right",
    "engineer_engage_visible_enemy",
    "engineer_burst_fire",
    "engineer_take_cover",
    "engineer_hold_cover",
    "engineer_reload",
    "engineer_throw_smoke",
    "engineer_throw_flash",
    "engineer_retreat_to_cover",
    "engineer_call_for_medic",
    "engineer_mark_threat",
    "engineer_cover_ally",
    "engineer_defend_brain_actor",
    "engineer_follow_order",
    "engineer_patrol_perimeter",
    "engineer_scout_ahead",
    "engineer_investigate_sound",
    "engineer_emergency_self_repair",
    "engineer_cover_me",
    "engineer_track_mover",
];

/// **M7B**: squad verbs the Engineer exposes as distinct BT subtrees.
pub const SQUAD_VERB_IDS: &[&str] = &[
    "suppress_window",
    "suppress_target",
    "overwatch_sector",
    "cover_me",
];

pub fn bt_for_squad_verb(verb_id: &str) -> Option<BtNode> {
    match verb_id {
        "suppress_window" => Some(BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "engineer_take_cover".into(),
                },
                BtNode::Action {
                    name: "engineer_burst_fire".into(),
                },
            ],
        }),
        "suppress_target" => Some(BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "engineer_take_cover".into(),
                },
                BtNode::Action {
                    name: "engineer_engage_visible_enemy".into(),
                },
            ],
        }),
        "overwatch_sector" => Some(BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "engineer_hold_cover".into(),
                },
                BtNode::Action {
                    name: "engineer_burst_fire".into(),
                },
            ],
        }),
        "cover_me" => Some(BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "engineer_track_mover".into(),
                },
                BtNode::Action {
                    name: "engineer_cover_me".into(),
                },
                BtNode::Action {
                    name: "engineer_burst_fire".into(),
                },
            ],
        }),
        _ => None,
    }
}

pub fn bt_for_task(task: TaskType) -> BtNode {
    match task {
        TaskType::RepairChassisModule => BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "engineer_scan_for_module_damage".into(),
                },
                BtNode::Action {
                    name: "engineer_approach_chassis".into(),
                },
                BtNode::Action {
                    name: "engineer_repair_chassis_module".into(),
                },
            ],
        },
        TaskType::RepairTerrainBreach => BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "engineer_pour_concrete".into(),
                },
                BtNode::Action {
                    name: "engineer_repair_terrain_breach".into(),
                },
            ],
        },
        TaskType::SetTrap => BtNode::Selector {
            children: vec![
                BtNode::Action {
                    name: "engineer_set_trap".into(),
                },
                BtNode::Action {
                    name: "engineer_lay_mine".into(),
                },
            ],
        },
        TaskType::DigCover => BtNode::Action {
            name: "engineer_dig_cover".into(),
        },
        TaskType::Demolish => BtNode::Action {
            name: "engineer_demolish_wall".into(),
        },
        TaskType::EngageVisibleEnemy => BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "engineer_take_cover".into(),
                },
                BtNode::Action {
                    name: "engineer_engage_visible_enemy".into(),
                },
            ],
        },
        TaskType::HoldCover => BtNode::Action {
            name: "engineer_hold_cover".into(),
        },
        TaskType::RetreatToCover => BtNode::Action {
            name: "engineer_retreat_to_cover".into(),
        },
        TaskType::CoverAlly => BtNode::Action {
            name: "engineer_cover_ally".into(),
        },
        TaskType::DefendBrainActor => BtNode::Action {
            name: "engineer_defend_brain_actor".into(),
        },
        TaskType::FollowOrder => BtNode::Action {
            name: "engineer_follow_order".into(),
        },
        TaskType::Patrol => BtNode::Action {
            name: "engineer_patrol_perimeter".into(),
        },
        TaskType::ScoutAhead => BtNode::Action {
            name: "engineer_scout_ahead".into(),
        },
        TaskType::InvestigateSound => BtNode::Action {
            name: "engineer_investigate_sound".into(),
        },
        TaskType::MarkThreats => BtNode::Action {
            name: "engineer_mark_threat".into(),
        },
        TaskType::HealSelf => BtNode::Action {
            name: "engineer_emergency_self_repair".into(),
        },
        _ => BtNode::Action {
            name: "engineer_idle".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_set_meets_30_floor() {
        assert!(NODES.len() >= 30);
    }

    #[test]
    fn repair_chassis_uses_full_three_step_chain() {
        let root = bt_for_task(TaskType::RepairChassisModule);
        let trail = root.flatten_label();
        assert!(trail.contains("engineer_scan_for_module_damage"));
        assert!(trail.contains("engineer_approach_chassis"));
        assert!(trail.contains("engineer_repair_chassis_module"));
    }
}
