//! M7B: Assault archetype BT.

use crate::behavior_tree::BtNode;
use crate::task::TaskType;

pub const NODES: &[&str] = &[
    "assault_idle",
    "assault_push_to_target",
    "assault_flank_left",
    "assault_flank_right",
    "assault_engage_visible_enemy",
    "assault_rapid_burst",
    "assault_close_quarters_fire",
    "assault_throw_grenade",
    "assault_throw_smoke",
    "assault_throw_flash",
    "assault_breach_door",
    "assault_kick_door",
    "assault_storm_room",
    "assault_post_breach_cover",
    "assault_stack_left",
    "assault_stack_right",
    "assault_sweep_corner",
    "assault_demolish_wall",
    "assault_demolish_window",
    "assault_dig_cover",
    "assault_set_trap",
    "assault_take_cover",
    "assault_crouch",
    "assault_prone",
    "assault_reload",
    "assault_press_attack",
    "assault_disengage",
    "assault_mark_threat",
    "assault_cover_me",
    "assault_cover_ally",
    "assault_defend_brain_actor",
    "assault_follow_order",
    "assault_investigate_sound",
    "assault_advance_bounding",
    "assault_track_mover",
];

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
                    name: "assault_take_cover".into(),
                },
                BtNode::Action {
                    name: "assault_rapid_burst".into(),
                },
            ],
        }),
        "suppress_target" => Some(BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "assault_take_cover".into(),
                },
                BtNode::Action {
                    name: "assault_rapid_burst".into(),
                },
                BtNode::Action {
                    name: "assault_close_quarters_fire".into(),
                },
            ],
        }),
        "overwatch_sector" => Some(BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "assault_sweep_corner".into(),
                },
                BtNode::Action {
                    name: "assault_engage_visible_enemy".into(),
                },
            ],
        }),
        "cover_me" => Some(BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "assault_track_mover".into(),
                },
                BtNode::Action {
                    name: "assault_cover_me".into(),
                },
                BtNode::Action {
                    name: "assault_rapid_burst".into(),
                },
            ],
        }),
        _ => None,
    }
}

pub fn bt_for_task(task: TaskType) -> BtNode {
    match task {
        TaskType::EngageVisibleEnemy => BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "assault_push_to_target".into(),
                },
                BtNode::Action {
                    name: "assault_engage_visible_enemy".into(),
                },
                BtNode::Action {
                    name: "assault_rapid_burst".into(),
                },
            ],
        },
        TaskType::FlankTarget => BtNode::Selector {
            children: vec![
                BtNode::Action {
                    name: "assault_flank_left".into(),
                },
                BtNode::Action {
                    name: "assault_flank_right".into(),
                },
            ],
        },
        TaskType::ThrowGrenade => BtNode::Action {
            name: "assault_throw_grenade".into(),
        },
        TaskType::Demolish => BtNode::Selector {
            children: vec![
                BtNode::Action {
                    name: "assault_demolish_wall".into(),
                },
                BtNode::Action {
                    name: "assault_demolish_window".into(),
                },
            ],
        },
        TaskType::HoldCover => BtNode::Action {
            name: "assault_take_cover".into(),
        },
        TaskType::RetreatToCover => BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "assault_disengage".into(),
                },
                BtNode::Action {
                    name: "assault_take_cover".into(),
                },
            ],
        },
        TaskType::FollowOrder => BtNode::Action {
            name: "assault_follow_order".into(),
        },
        TaskType::CoverAlly => BtNode::Action {
            name: "assault_cover_ally".into(),
        },
        TaskType::DefendBrainActor => BtNode::Action {
            name: "assault_defend_brain_actor".into(),
        },
        TaskType::MarkThreats => BtNode::Action {
            name: "assault_mark_threat".into(),
        },
        TaskType::InvestigateSound => BtNode::Action {
            name: "assault_investigate_sound".into(),
        },
        _ => BtNode::Action {
            name: "assault_idle".into(),
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
    fn engage_expansion_includes_rapid_burst() {
        let root = bt_for_task(TaskType::EngageVisibleEnemy);
        assert!(root.flatten_label().contains("assault_rapid_burst"));
    }
}
