//! M7B: Spotter archetype BT.

use crate::behavior_tree::BtNode;
use crate::task::TaskType;

pub const NODES: &[&str] = &[
    "spotter_idle",
    "spotter_scan_arc",
    "spotter_sweep_360",
    "spotter_mark_threat",
    "spotter_mark_priority_target",
    "spotter_call_target_lost",
    "spotter_call_artillery",
    "spotter_call_drone_strike",
    "spotter_call_support",
    "spotter_relay_target_to_sniper",
    "spotter_relay_target_to_squad",
    "spotter_track_target_los",
    "spotter_seek_high_ground",
    "spotter_climb_perch",
    "spotter_hold_cover",
    "spotter_take_cover",
    "spotter_prone",
    "spotter_throw_smoke",
    "spotter_throw_flash",
    "spotter_engage_visible_enemy",
    "spotter_aimed_shot",
    "spotter_reload",
    "spotter_investigate_sound",
    "spotter_patrol_overlook",
    "spotter_scout_ahead",
    "spotter_cover_ally",
    "spotter_defend_brain_actor",
    "spotter_follow_order",
    "spotter_fall_back",
    "spotter_retreat_to_cover",
    "spotter_disengage",
    "spotter_re_position",
    "spotter_acquire_secondary_target",
    "spotter_announce_low_ammo",
    "spotter_cover_me",
    "spotter_track_mover",
    "spotter_suppress_window",
    "spotter_suppress_target",
];

/// **M7B**: squad verbs the Spotter exposes as distinct BT subtrees.
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
                    name: "spotter_take_cover".into(),
                },
                BtNode::Action {
                    name: "spotter_suppress_window".into(),
                },
            ],
        }),
        "suppress_target" => Some(BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "spotter_take_cover".into(),
                },
                BtNode::Action {
                    name: "spotter_suppress_target".into(),
                },
            ],
        }),
        "overwatch_sector" => Some(BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "spotter_seek_high_ground".into(),
                },
                BtNode::Action {
                    name: "spotter_scan_arc".into(),
                },
                BtNode::Action {
                    name: "spotter_aimed_shot".into(),
                },
            ],
        }),
        "cover_me" => Some(BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "spotter_track_mover".into(),
                },
                BtNode::Action {
                    name: "spotter_cover_me".into(),
                },
                BtNode::Action {
                    name: "spotter_aimed_shot".into(),
                },
            ],
        }),
        _ => None,
    }
}

pub fn bt_for_task(task: TaskType) -> BtNode {
    match task {
        TaskType::MarkThreats => BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "spotter_scan_arc".into(),
                },
                BtNode::Action {
                    name: "spotter_mark_threat".into(),
                },
                BtNode::Action {
                    name: "spotter_relay_target_to_sniper".into(),
                },
            ],
        },
        TaskType::ScoutAhead => BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "spotter_seek_high_ground".into(),
                },
                BtNode::Action {
                    name: "spotter_scout_ahead".into(),
                },
            ],
        },
        TaskType::InvestigateSound => BtNode::Action {
            name: "spotter_investigate_sound".into(),
        },
        TaskType::DefendBrainActor => BtNode::Action {
            name: "spotter_defend_brain_actor".into(),
        },
        TaskType::EngageVisibleEnemy => BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "spotter_take_cover".into(),
                },
                BtNode::Action {
                    name: "spotter_engage_visible_enemy".into(),
                },
            ],
        },
        TaskType::HoldCover => BtNode::Action {
            name: "spotter_hold_cover".into(),
        },
        TaskType::RetreatToCover => BtNode::Action {
            name: "spotter_retreat_to_cover".into(),
        },
        TaskType::Patrol => BtNode::Action {
            name: "spotter_patrol_overlook".into(),
        },
        TaskType::CoverAlly => BtNode::Action {
            name: "spotter_cover_ally".into(),
        },
        TaskType::FollowOrder => BtNode::Action {
            name: "spotter_follow_order".into(),
        },
        _ => BtNode::Action {
            name: "spotter_idle".into(),
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
    fn mark_includes_relay_to_sniper() {
        let root = bt_for_task(TaskType::MarkThreats);
        assert!(root.flatten_label().contains("spotter_relay_target_to_sniper"));
    }
}
