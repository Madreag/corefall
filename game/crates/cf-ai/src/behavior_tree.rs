//! M7-A Layer 3: Behavior Tree.
//!
//! Spec § 5-layer thinking stack — Layer 3 takes the Utility-chosen task and
//! expands it into a sequence of low-level actions (Sequence / Selector /
//! Decorator nodes). The BT is purely advisory at M7-A — the engine consumes
//! the leaf `BehaviorAction` and dispatches it. M8 extends with rich BT
//! authoring and persistence.
//!
//! **M7B**: when `ThinkingContext.archetype` maps onto one of the 6
//! `ArchetypeBtKind` rows, the layer expands via the deep `archetype_bt`
//! tree (≥30 distinct leaves per archetype) instead of the M7-A canned
//! tree. The generic [`BehaviorTreeLayer::expand`] is preserved for tests
//! that still want to assert the M7-A baseline.

use serde::{Deserialize, Serialize};

use crate::archetype::Archetype;
use crate::archetype_bt::{self, ArchetypeBtKind};
use crate::task::TaskType;
use crate::thinking_stack::{Layer, LayerKind, LayerOutput, ThinkingContext};

/// `Action(name)`. The tree is small (depth ≤ 4) so we serialize it as a
/// flat enum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "node")]
pub enum BtNode {
    Sequence { children: Vec<BtNode> },
    Selector { children: Vec<BtNode> },
    Decorator { kind: String, child: Box<BtNode> },
    Action { name: String },
}

impl BtNode {
    /// Render the node trail as a `→`-separated string for the reason
    /// label. Walks the depth-first leaf chain.
    pub fn flatten_label(&self) -> String {
        let mut out = Vec::new();
        walk(self, &mut out);
        out.join("→")
    }
}

fn walk(node: &BtNode, out: &mut Vec<String>) {
    match node {
        BtNode::Action { name } => out.push(name.clone()),
        BtNode::Decorator { kind, child } => {
            out.push(kind.clone());
            walk(child, out);
        }
        BtNode::Sequence { children } | BtNode::Selector { children } => {
            for c in children {
                walk(c, out);
            }
        }
    }
}

/// into actual movement / fire / repair commands.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorAction {
    Idle,
    MoveToCover,
    MoveToWaypoint,
    MoveToAlly,
    MoveToEnemy,
    Fire,
    SuppressFire,
    Reload,
    ThrowGrenade,
    SetTrap,
    ApplyMedkit,
    ApplyRepairTool,
    MarkThreat,
    ScopeSettle,
    Dodge,
    ChangeStance,
}

impl BehaviorAction {
    pub fn as_str(self) -> &'static str {
        match self {
            BehaviorAction::Idle => "idle",
            BehaviorAction::MoveToCover => "move_to_cover",
            BehaviorAction::MoveToWaypoint => "move_to_waypoint",
            BehaviorAction::MoveToAlly => "approach_ally",
            BehaviorAction::MoveToEnemy => "approach_enemy",
            BehaviorAction::Fire => "fire",
            BehaviorAction::SuppressFire => "suppress_fire",
            BehaviorAction::Reload => "reload",
            BehaviorAction::ThrowGrenade => "throw_grenade",
            BehaviorAction::SetTrap => "set_trap",
            BehaviorAction::ApplyMedkit => "treat_loop",
            BehaviorAction::ApplyRepairTool => "repair_loop",
            BehaviorAction::MarkThreat => "mark_threat",
            BehaviorAction::ScopeSettle => "scope_settle",
            BehaviorAction::Dodge => "dodge",
            BehaviorAction::ChangeStance => "change_stance",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BehaviorTreeLayer {
    /// Latest BT root the layer produced. Reset each tick.
    pub last_root: Option<BtNode>,
    /// Latest leaf action chosen by walking the BT root.
    pub last_action: Option<BehaviorAction>,
    /// Trail string consumed by the reason label.
    pub last_trail: String,
}

impl BehaviorTreeLayer {
    pub fn new() -> Self {
        Self {
            last_root: None,
            last_action: None,
            last_trail: String::new(),
        }
    }

    /// Expand the upstream task into a small BT root. Each task family has
    /// a canned tree per spec; M8 will extend with custom authoring.
    pub fn expand(task: TaskType) -> BtNode {
        match task {
            TaskType::EngageVisibleEnemy => BtNode::Sequence {
                children: vec![
                    BtNode::Action {
                        name: "move_to_cover".into(),
                    },
                    BtNode::Action { name: "fire".into() },
                ],
            },
            TaskType::SuppressFire => BtNode::Sequence {
                children: vec![
                    BtNode::Action {
                        name: "move_to_cover".into(),
                    },
                    BtNode::Action {
                        name: "suppress_fire".into(),
                    },
                ],
            },
            TaskType::HoldCover => BtNode::Action {
                name: "hold_position".into(),
            },
            TaskType::FlankTarget => BtNode::Sequence {
                children: vec![
                    BtNode::Action {
                        name: "move_to_flank".into(),
                    },
                    BtNode::Action { name: "fire".into() },
                ],
            },
            TaskType::ThrowGrenade => BtNode::Action {
                name: "throw_grenade".into(),
            },
            TaskType::Demolish => BtNode::Action {
                name: "demolish_target".into(),
            },
            TaskType::SharpshootTarget => BtNode::Sequence {
                children: vec![
                    BtNode::Action {
                        name: "scope_settle".into(),
                    },
                    BtNode::Action { name: "fire".into() },
                ],
            },
            TaskType::MarkThreats => BtNode::Action {
                name: "mark_threat".into(),
            },
            TaskType::ScoutAhead => BtNode::Action {
                name: "scout_ahead".into(),
            },
            TaskType::RepairChassisModule => BtNode::Sequence {
                children: vec![
                    BtNode::Action {
                        name: "approach_ally".into(),
                    },
                    BtNode::Action {
                        name: "repair_loop".into(),
                    },
                ],
            },
            TaskType::RepairTerrainBreach => BtNode::Sequence {
                children: vec![
                    BtNode::Action {
                        name: "approach_breach".into(),
                    },
                    BtNode::Action {
                        name: "repair_loop".into(),
                    },
                ],
            },
            TaskType::SetTrap => BtNode::Action {
                name: "set_trap".into(),
            },
            TaskType::DigCover => BtNode::Action {
                name: "dig_cover".into(),
            },
            TaskType::TriageDownedAlly => BtNode::Sequence {
                children: vec![
                    BtNode::Action {
                        name: "move_to_cover".into(),
                    },
                    BtNode::Action {
                        name: "approach_ally".into(),
                    },
                    BtNode::Action {
                        name: "treat_loop".into(),
                    },
                ],
            },
            TaskType::HealSelf => BtNode::Action {
                name: "treat_self".into(),
            },
            TaskType::CoverAlly => BtNode::Action {
                name: "cover_ally".into(),
            },
            TaskType::DefendBrainActor => BtNode::Action {
                name: "defend_brain_actor".into(),
            },
            TaskType::InvestigateSound => BtNode::Action {
                name: "investigate_sound".into(),
            },
            TaskType::FollowOrder => BtNode::Action {
                name: "follow_order".into(),
            },
            TaskType::RetreatToCover => BtNode::Sequence {
                children: vec![
                    BtNode::Action {
                        name: "move_to_cover".into(),
                    },
                    BtNode::Action {
                        name: "hold_position".into(),
                    },
                ],
            },
            TaskType::Patrol => BtNode::Sequence {
                children: vec![
                    BtNode::Action {
                        name: "move_to_waypoint".into(),
                    },
                    BtNode::Action {
                        name: "idle_pause".into(),
                    },
                ],
            },
            TaskType::Idle => BtNode::Action { name: "idle".into() },
        }
    }

    /// First (leftmost) leaf action in the BT root.
    pub fn leaf_action(node: &BtNode) -> BehaviorAction {
        match node {
            BtNode::Action { name } => name_to_action(name.as_str()),
            BtNode::Sequence { children } | BtNode::Selector { children } => {
                children.first().map(Self::leaf_action).unwrap_or(BehaviorAction::Idle)
            }
            BtNode::Decorator { child, .. } => Self::leaf_action(child),
        }
    }
}

/// reuses the rifleman tree (closest analog with cover-based behavior) until
/// a dedicated medic_bt ships in a future milestone.
pub fn archetype_to_bt_kind(archetype: Archetype) -> ArchetypeBtKind {
    match archetype {
        Archetype::Rifleman | Archetype::Medic => ArchetypeBtKind::Rifleman,
        Archetype::Sniper => ArchetypeBtKind::Sniper,
        Archetype::Assault => ArchetypeBtKind::Assault,
        Archetype::Engineer => ArchetypeBtKind::Engineer,
        Archetype::Spotter => ArchetypeBtKind::Spotter,
    }
}

fn strip_archetype_prefix(name: &str) -> &str {
    for prefix in [
        "rifleman_",
        "sniper_",
        "assault_",
        "engineer_",
        "spotter_",
        "heavy_",
        "medic_",
    ] {
        if let Some(rest) = name.strip_prefix(prefix) {
            return rest;
        }
    }
    name
}

/// `BehaviorAction`. Falls back to `Idle` for unrecognised names.
fn name_to_action(raw: &str) -> BehaviorAction {
    let stripped = strip_archetype_prefix(raw);
    match stripped {
        "move_to_cover" | "take_cover" | "post_breach_cover" | "retreat_to_cover" => BehaviorAction::MoveToCover,
        "move_to_waypoint" | "move_to_target" | "patrol_waypoint" | "patrol_ridge" | "patrol_overlook"
        | "patrol_perimeter" | "approach_breach" | "advance_through_door" | "advance_forward"
        | "advance_bounding" | "investigate_sound" | "follow_order" | "scout_ahead" | "push_to_target"
        | "storm_room" | "storm_building" | "re_position" | "seek_high_ground" | "climb_perch"
        | "bounding_step" | "fall_back" | "disengage" | "kick_door" | "breach_door"
        | "stack_left" | "stack_right" | "sweep_corner" => BehaviorAction::MoveToWaypoint,
        "approach_ally" | "approach_chassis" => BehaviorAction::MoveToAlly,
        "approach_enemy" | "engage_visible_enemy" | "flank_left" | "flank_right" | "close_quarters_fire"
        | "track_target" | "track_target_los" => BehaviorAction::MoveToEnemy,
        "fire" | "burst_fire" | "rapid_burst" | "aimed_shot" | "anti_material_shot" | "walk_fire"
        | "re_chamber" => BehaviorAction::Fire,
        "suppress_fire" | "suppress_window" | "suppress_target" | "lay_suppressive_fire"
        | "overwatch_sector" | "overwatch_door" => BehaviorAction::SuppressFire,
        "reload" | "belt_reload" | "long_reload" | "break_barrel" | "change_barrel" | "top_off_mags" => {
            BehaviorAction::Reload
        }
        "throw_grenade" | "throw_smoke" | "throw_flash" | "decoy_throw" | "smoke_to_break_los"
        | "frag_out" => BehaviorAction::ThrowGrenade,
        "set_trap" | "lay_mine" | "disarm_trap" | "dig_cover" | "dig_breach" | "dig_emplacement"
        | "pour_concrete" | "seal_pipe" | "extinguish_fire" | "deploy_bipod" | "stow_bipod"
        | "demolish_target" | "demolish_wall" | "demolish_window" => BehaviorAction::SetTrap,
        "treat_loop" | "treat_self" | "emergency_self_repair" | "call_for_medic" | "call_for_engineer" => {
            BehaviorAction::ApplyMedkit
        }
        "repair_loop" | "repair_chassis_module" | "repair_terrain_breach" => BehaviorAction::ApplyRepairTool,
        "mark_threat" | "mark_priority_target" | "spot_for_squad" | "call_target" | "call_target_lost"
        | "relay_target_to_sniper" | "relay_target_to_squad" | "scan_arc" | "sweep_360"
        | "announce_low_ammo" | "acquire_secondary_target" => BehaviorAction::MarkThreat,
        "scope_settle" | "breath_hold" | "scope_zoom_increase" | "scope_zoom_decrease" => {
            BehaviorAction::ScopeSettle
        }
        "dodge" | "prone" | "crouch" | "take_prone" => BehaviorAction::ChangeStance,
        "hold_position" | "hold_cover" | "cover_ally" | "defend_brain_actor" | "cover_me"
        | "scan_for_module_damage" | "sniper_cover" | "press_attack" | "heavy_forward" | "disappear_act"
        | "call_artillery" | "call_drone_strike" | "call_support" | "idle" | "idle_pause" => {
            BehaviorAction::Idle
        }
        _ => BehaviorAction::Idle,
    }
}

impl Default for BehaviorTreeLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Layer for BehaviorTreeLayer {
    fn kind(&self) -> LayerKind {
        LayerKind::BehaviorTree
    }

    fn tick_layer(&mut self, ctx: &mut ThinkingContext<'_>) -> LayerOutput {
        let chosen = ctx.chosen_task.unwrap_or(TaskType::Idle);
        let kind = archetype_to_bt_kind(ctx.archetype);
        let root = archetype_bt::bt_for(kind, chosen);
        let trail = root.flatten_label();
        let action = Self::leaf_action(&root);
        self.last_action = Some(action);
        self.last_trail = trail.clone();
        self.last_root = Some(root);
        ctx.behavior_tree_trail = trail;
        LayerOutput {
            override_task: Some(chosen),
            reason: "bt_expanded",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thinking_stack::ThinkingContext;

    #[test]
    fn triage_expansion_produces_three_step_trail() {
        let root = BehaviorTreeLayer::expand(TaskType::TriageDownedAlly);
        let trail = root.flatten_label();
        assert!(trail.contains("move_to_cover"));
        assert!(trail.contains("approach_ally"));
        assert!(trail.contains("treat_loop"));
    }

    #[test]
    fn repair_expansion_emits_repair_loop() {
        let root = BehaviorTreeLayer::expand(TaskType::RepairChassisModule);
        let trail = root.flatten_label();
        assert!(trail.contains("approach_ally"));
        assert!(trail.contains("repair_loop"));
    }

    #[test]
    fn bt_layer_records_action() {
        let mut layer = BehaviorTreeLayer::new();
        let mut ctx = ThinkingContext::stub();
        ctx.chosen_task = Some(TaskType::EngageVisibleEnemy);
        let _ = layer.tick_layer(&mut ctx);
        assert_eq!(layer.last_action, Some(BehaviorAction::MoveToCover));
        assert!(!layer.last_trail.is_empty());
    }
}
