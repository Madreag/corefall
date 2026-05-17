//! M7B: Sniper archetype BT.

use crate::behavior_tree::BtNode;
use crate::task::TaskType;

pub const NODES: &[&str] = &[
    "sniper_idle",
    "sniper_seek_high_ground",
    "sniper_climb_perch",
    "sniper_scope_settle",
    "sniper_breath_hold",
    "sniper_aimed_shot",
    "sniper_anti_material_shot",
    "sniper_re_chamber",
    "sniper_track_target",
    "sniper_re_position",
    "sniper_disappear_act",
    "sniper_mark_threat",
    "sniper_call_target",
    "sniper_spot_for_squad",
    "sniper_overwatch_sector",
    "sniper_overwatch_door",
    "sniper_hold_cover",
    "sniper_take_cover",
    "sniper_prone",
    "sniper_decoy_throw",
    "sniper_fall_back",
    "sniper_retreat_to_cover",
    "sniper_smoke_to_break_los",
    "sniper_scope_zoom_increase",
    "sniper_scope_zoom_decrease",
    "sniper_investigate_sound",
    "sniper_patrol_ridge",
    "sniper_reload",
    "sniper_cover_ally",
    "sniper_defend_brain_actor",
    "sniper_follow_order",
    "sniper_scout_ahead",
    "sniper_acquire_secondary_target",
    "sniper_call_target_lost",
    "sniper_cover_me",
    "sniper_track_mover",
];

/// **M7B**: squad verbs the Sniper exposes as distinct BT subtrees.
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
                    name: "sniper_prone".into(),
                },
                BtNode::Action {
                    name: "sniper_scope_settle".into(),
                },
                BtNode::Action {
                    name: "sniper_aimed_shot".into(),
                },
            ],
        }),
        "suppress_target" => Some(BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "sniper_track_target".into(),
                },
                BtNode::Action {
                    name: "sniper_aimed_shot".into(),
                },
            ],
        }),
        "overwatch_sector" => Some(BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "sniper_overwatch_sector".into(),
                },
                BtNode::Action {
                    name: "sniper_scope_settle".into(),
                },
                BtNode::Action {
                    name: "sniper_aimed_shot".into(),
                },
            ],
        }),
        "cover_me" => Some(BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "sniper_track_mover".into(),
                },
                BtNode::Action {
                    name: "sniper_cover_me".into(),
                },
                BtNode::Action {
                    name: "sniper_aimed_shot".into(),
                },
            ],
        }),
        _ => None,
    }
}

pub fn bt_for_task(task: TaskType) -> BtNode {
    match task {
        TaskType::SharpshootTarget => BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "sniper_seek_high_ground".into(),
                },
                BtNode::Action {
                    name: "sniper_scope_settle".into(),
                },
                BtNode::Action {
                    name: "sniper_breath_hold".into(),
                },
                BtNode::Action {
                    name: "sniper_aimed_shot".into(),
                },
                BtNode::Action {
                    name: "sniper_re_chamber".into(),
                },
            ],
        },
        TaskType::MarkThreats => BtNode::Action {
            name: "sniper_mark_threat".into(),
        },
        TaskType::ScoutAhead => BtNode::Action {
            name: "sniper_scout_ahead".into(),
        },
        TaskType::HoldCover => BtNode::Action {
            name: "sniper_hold_cover".into(),
        },
        TaskType::RetreatToCover => BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "sniper_smoke_to_break_los".into(),
                },
                BtNode::Action {
                    name: "sniper_re_position".into(),
                },
                BtNode::Action {
                    name: "sniper_retreat_to_cover".into(),
                },
            ],
        },
        TaskType::Patrol => BtNode::Action {
            name: "sniper_patrol_ridge".into(),
        },
        TaskType::EngageVisibleEnemy => BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "sniper_scope_settle".into(),
                },
                BtNode::Action {
                    name: "sniper_aimed_shot".into(),
                },
            ],
        },
        TaskType::CoverAlly => BtNode::Action {
            name: "sniper_cover_ally".into(),
        },
        TaskType::DefendBrainActor => BtNode::Action {
            name: "sniper_defend_brain_actor".into(),
        },
        TaskType::InvestigateSound => BtNode::Action {
            name: "sniper_investigate_sound".into(),
        },
        TaskType::FollowOrder => BtNode::Action {
            name: "sniper_follow_order".into(),
        },
        _ => BtNode::Action {
            name: "sniper_idle".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_set_meets_30_floor() {
        assert!(NODES.len() >= 30, "sniper exposes {} nodes", NODES.len());
    }

    #[test]
    fn sharpshoot_expansion_includes_scope_settle() {
        let root = bt_for_task(TaskType::SharpshootTarget);
        assert!(root.flatten_label().contains("sniper_scope_settle"));
    }
}
