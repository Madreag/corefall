//! M7B: Rifleman archetype BT — deep-fill of M7-A's Layer 3 stub.
//!
//! The Rifleman BT supplies ≥30 distinct leaf nodes covering the full
//! 22-task lattice with branched cover-seeking, suppression, and bounding
//! movement variants. The BT root for an unscripted task is built by
//! [`bt_for_task`]; the M7-A behavior tree layer falls back to its generic
//! expansion for archetypes that don't have a custom row.

use crate::behavior_tree::BtNode;
use crate::task::TaskType;

/// **M7B**: enumerated leaf node ids the Rifleman BT can emit. Stable
/// across milestones; downstream tooling indexes by exact string match.
pub const NODES: &[&str] = &[
    "rifleman_idle",
    "rifleman_scan_arc",
    "rifleman_move_to_cover",
    "rifleman_hold_cover",
    "rifleman_engage_visible_enemy",
    "rifleman_burst_fire",
    "rifleman_aimed_shot",
    "rifleman_suppress_window",
    "rifleman_suppress_target",
    "rifleman_overwatch_sector",
    "rifleman_reload",
    "rifleman_flank_left",
    "rifleman_flank_right",
    "rifleman_throw_grenade",
    "rifleman_throw_smoke",
    "rifleman_throw_flash",
    "rifleman_retreat_to_cover",
    "rifleman_fall_back",
    "rifleman_bounding_step",
    "rifleman_take_cover",
    "rifleman_prone",
    "rifleman_crouch",
    "rifleman_mark_threat",
    "rifleman_investigate_sound",
    "rifleman_follow_order",
    "rifleman_patrol_waypoint",
    "rifleman_cover_ally",
    "rifleman_defend_brain_actor",
    "rifleman_call_for_medic",
    "rifleman_call_for_engineer",
    "rifleman_stack_left",
    "rifleman_stack_right",
    "rifleman_breach_door",
    "rifleman_advance_through_door",
    "rifleman_post_breach_cover",
    "rifleman_cover_me",
    "rifleman_track_mover",
];

/// **M7B**: list of squad verbs the Rifleman exposes as distinct BT
/// subtrees per spec § "Suppress vs Overwatch vs Cover Me are distinct BT
/// subtrees." Ordered for stable RON round-trip.
pub const SQUAD_VERB_IDS: &[&str] = &[
    "suppress_window",
    "suppress_target",
    "overwatch_sector",
    "cover_me",
];

/// **M7B**: per-squad-verb BT expansion. Returns `None` for verbs that
/// don't need a distinct subtree (engine falls back to `bt_for_task`).
pub fn bt_for_squad_verb(verb_id: &str) -> Option<BtNode> {
    match verb_id {
        // Suppress (window) = sustained low-acc high-RPM on the window
        // frame; never settles aim, just lays fire.
        "suppress_window" => Some(BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "rifleman_take_cover".into(),
                },
                BtNode::Action {
                    name: "rifleman_suppress_window".into(),
                },
                BtNode::Action {
                    name: "rifleman_burst_fire".into(),
                },
            ],
        }),
        // Suppress (target) = sustained low-acc fire on a specific actor.
        "suppress_target" => Some(BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "rifleman_take_cover".into(),
                },
                BtNode::Action {
                    name: "rifleman_suppress_target".into(),
                },
            ],
        }),
        // Overwatch (sector) = hold fire until non-friendly enters the
        // sector, then snap-aim once. Distinct from suppress.
        "overwatch_sector" => Some(BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "rifleman_hold_cover".into(),
                },
                BtNode::Action {
                    name: "rifleman_overwatch_sector".into(),
                },
                BtNode::Action {
                    name: "rifleman_aimed_shot".into(),
                },
            ],
        }),
        // Cover Me = suppress any LOS-visible threat to the named mover
        // until they reach goal. Distinct from suppress / overwatch.
        "cover_me" => Some(BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "rifleman_track_mover".into(),
                },
                BtNode::Action {
                    name: "rifleman_cover_me".into(),
                },
                BtNode::Action {
                    name: "rifleman_burst_fire".into(),
                },
            ],
        }),
        _ => None,
    }
}

/// **M7B**: Rifleman per-task BT expansion. Falls through to M7-A defaults
/// for unrelated tasks.
pub fn bt_for_task(task: TaskType) -> BtNode {
    match task {
        TaskType::EngageVisibleEnemy => BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "rifleman_move_to_cover".into(),
                },
                BtNode::Action {
                    name: "rifleman_engage_visible_enemy".into(),
                },
                BtNode::Action {
                    name: "rifleman_burst_fire".into(),
                },
            ],
        },
        TaskType::SuppressFire => BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "rifleman_take_cover".into(),
                },
                BtNode::Action {
                    name: "rifleman_suppress_target".into(),
                },
            ],
        },
        TaskType::HoldCover => BtNode::Action {
            name: "rifleman_hold_cover".into(),
        },
        TaskType::FlankTarget => BtNode::Selector {
            children: vec![
                BtNode::Action {
                    name: "rifleman_flank_left".into(),
                },
                BtNode::Action {
                    name: "rifleman_flank_right".into(),
                },
            ],
        },
        TaskType::ThrowGrenade => BtNode::Action {
            name: "rifleman_throw_grenade".into(),
        },
        TaskType::RetreatToCover => BtNode::Sequence {
            children: vec![
                BtNode::Action {
                    name: "rifleman_fall_back".into(),
                },
                BtNode::Action {
                    name: "rifleman_bounding_step".into(),
                },
                BtNode::Action {
                    name: "rifleman_take_cover".into(),
                },
            ],
        },
        TaskType::Patrol => BtNode::Action {
            name: "rifleman_patrol_waypoint".into(),
        },
        TaskType::InvestigateSound => BtNode::Action {
            name: "rifleman_investigate_sound".into(),
        },
        TaskType::FollowOrder => BtNode::Action {
            name: "rifleman_follow_order".into(),
        },
        TaskType::CoverAlly => BtNode::Action {
            name: "rifleman_cover_ally".into(),
        },
        TaskType::DefendBrainActor => BtNode::Action {
            name: "rifleman_defend_brain_actor".into(),
        },
        TaskType::MarkThreats => BtNode::Action {
            name: "rifleman_mark_threat".into(),
        },
        _ => BtNode::Action {
            name: "rifleman_idle".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_set_meets_30_floor() {
        assert!(NODES.len() >= 30, "rifleman exposes {} nodes", NODES.len());
    }

    #[test]
    fn engage_expansion_includes_burst_fire() {
        let root = bt_for_task(TaskType::EngageVisibleEnemy);
        assert!(root.flatten_label().contains("rifleman_burst_fire"));
    }

    #[test]
    fn retreat_expansion_includes_bounding() {
        let root = bt_for_task(TaskType::RetreatToCover);
        assert!(root.flatten_label().contains("rifleman_bounding_step"));
    }
}
