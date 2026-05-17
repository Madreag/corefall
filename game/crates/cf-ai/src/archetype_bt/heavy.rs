//! M7B: Heavy archetype BT.

use crate::behavior_tree::BtNode;
use crate::task::TaskType;

pub const NODES: &[&str] = &[
    "heavy_idle",
    "heavy_deploy_bipod",
    "heavy_stow_bipod",
    "heavy_lay_suppressive_fire",
    "heavy_suppress_window",
    "heavy_suppress_target",
    "heavy_overwatch_sector",
    "heavy_engage_visible_enemy",
    "heavy_walk_fire",
    "heavy_belt_reload",
    "heavy_long_reload",
    "heavy_break_barrel",
    "heavy_change_barrel",
    "heavy_throw_grenade",
    "heavy_throw_smoke",
    "heavy_dig_emplacement",
    "heavy_take_cover",
    "heavy_hold_cover",
    "heavy_take_prone",
    "heavy_advance_forward",
    "heavy_press_attack",
    "heavy_breach_door",
    "heavy_kick_door",
    "heavy_post_breach_cover",
    "heavy_storm_room",
    "heavy_cover_me",
    "heavy_cover_ally",
    "heavy_defend_brain_actor",
    "heavy_follow_order",
    "heavy_fall_back",
    "heavy_retreat_to_cover",
    "heavy_mark_threat",
    "heavy_call_for_engineer",
    "heavy_emergency_self_repair",
    "heavy_track_mover",
];

/// **M7B**: squad verbs the Heavy exposes as distinct BT subtrees.
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
                    name: "heavy_deploy_bipod".into(),
                },
                BtNode::Action {
                    name: "heavy_suppress_window".into(),
                },
                BtNode::Action {
                    name: "heavy_lay_suppressive_fire".into(),
                },
            ],
        }),
        "suppress_target" => Some(BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "heavy_deploy_bipod".into(),
                },
                BtNode::Action {
                    name: "heavy_suppress_target".into(),
                },
                BtNode::Action {
                    name: "heavy_lay_suppressive_fire".into(),
                },
            ],
        }),
        "overwatch_sector" => Some(BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "heavy_dig_emplacement".into(),
                },
                BtNode::Action {
                    name: "heavy_overwatch_sector".into(),
                },
                BtNode::Action {
                    name: "heavy_engage_visible_enemy".into(),
                },
            ],
        }),
        "cover_me" => Some(BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "heavy_track_mover".into(),
                },
                BtNode::Action {
                    name: "heavy_cover_me".into(),
                },
                BtNode::Action {
                    name: "heavy_walk_fire".into(),
                },
            ],
        }),
        _ => None,
    }
}

pub fn bt_for_task(task: TaskType) -> BtNode {
    match task {
        TaskType::SuppressFire => BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "heavy_deploy_bipod".into(),
                },
                BtNode::Action {
                    name: "heavy_lay_suppressive_fire".into(),
                },
            ],
        },
        TaskType::EngageVisibleEnemy => BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "heavy_take_cover".into(),
                },
                BtNode::Action {
                    name: "heavy_engage_visible_enemy".into(),
                },
                BtNode::Action {
                    name: "heavy_walk_fire".into(),
                },
            ],
        },
        TaskType::HoldCover => BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "heavy_dig_emplacement".into(),
                },
                BtNode::Action {
                    name: "heavy_hold_cover".into(),
                },
            ],
        },
        TaskType::RetreatToCover => BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "heavy_fall_back".into(),
                },
                BtNode::Action {
                    name: "heavy_take_cover".into(),
                },
            ],
        },
        TaskType::FlankTarget => BtNode::Action {
            name: "heavy_advance_forward".into(),
        },
        TaskType::ThrowGrenade => BtNode::Action {
            name: "heavy_throw_grenade".into(),
        },
        TaskType::CoverAlly => BtNode::Action {
            name: "heavy_cover_ally".into(),
        },
        TaskType::DefendBrainActor => BtNode::Action {
            name: "heavy_defend_brain_actor".into(),
        },
        TaskType::FollowOrder => BtNode::Action {
            name: "heavy_follow_order".into(),
        },
        TaskType::MarkThreats => BtNode::Action {
            name: "heavy_mark_threat".into(),
        },
        TaskType::HealSelf => BtNode::Action {
            name: "heavy_emergency_self_repair".into(),
        },
        _ => BtNode::Action {
            name: "heavy_idle".into(),
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
    fn suppress_uses_bipod_deploy() {
        let root = bt_for_task(TaskType::SuppressFire);
        assert!(root.flatten_label().contains("heavy_deploy_bipod"));
    }
}
