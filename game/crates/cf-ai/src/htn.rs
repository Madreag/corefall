//! M7-A Layer 4: HTN planner.
//!
//! Spec § 5-layer thinking stack — Layer 4 decomposes long-running goals
//! into a stack of BT-runnable sub-goals. The HTN re-plans when goals are
//! achieved or fail (via Layer 5's doctrine prior + recent-events ring).
//!
//! M7-A ships a minimal HTN that decomposes 5 root goals (Protect Squad,
//! Press Attack, Hold Sector, Reach Objective, Patrol). M8/M9 extends with
//! mission-director driven goals.

use serde::{Deserialize, Serialize};

use crate::task::TaskType;
use crate::thinking_stack::{Layer, LayerKind, LayerOutput, ThinkingContext};

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum HtnRootGoal {
    #[default]
    Idle,
    ProtectSquad,
    PressAttack,
    HoldSector,
    ReachObjective,
    Patrol,
    BreachThroughTerrain,
}

impl HtnRootGoal {
    pub fn as_str(self) -> &'static str {
        match self {
            HtnRootGoal::Idle => "idle",
            HtnRootGoal::ProtectSquad => "protect_squad",
            HtnRootGoal::PressAttack => "press_attack",
            HtnRootGoal::HoldSector => "hold_sector",
            HtnRootGoal::ReachObjective => "reach_objective",
            HtnRootGoal::Patrol => "patrol",
            HtnRootGoal::BreachThroughTerrain => "breach_through_terrain",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HtnGoal {
    pub label: String,
    pub favored_task: Option<TaskType>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HtnLayer {
    pub root: HtnRootGoal,
    pub stack: Vec<HtnGoal>,
    /// Cached "/"-joined stack labels for the reason label.
    pub last_stack_label: String,
    /// Tick on which the planner last re-planned (for debug).
    pub last_replan_tick: u64,
}

impl HtnLayer {
    pub fn new() -> Self {
        Self {
            root: HtnRootGoal::Idle,
            stack: Vec::new(),
            last_stack_label: "idle".to_string(),
            last_replan_tick: 0,
        }
    }

    /// Decompose a root goal into a 2-3 deep stack. Each entry favours one
    /// task family the Utility scorer should weight high.
    pub fn decompose(root: HtnRootGoal) -> Vec<HtnGoal> {
        match root {
            HtnRootGoal::Idle => vec![HtnGoal {
                label: "idle".into(),
                favored_task: Some(TaskType::Idle),
            }],
            HtnRootGoal::ProtectSquad => vec![
                HtnGoal {
                    label: "protect_squad".into(),
                    favored_task: Some(TaskType::CoverAlly),
                },
                HtnGoal {
                    label: "triage_medic_route".into(),
                    favored_task: Some(TaskType::TriageDownedAlly),
                },
            ],
            HtnRootGoal::PressAttack => vec![
                HtnGoal {
                    label: "press_attack".into(),
                    favored_task: Some(TaskType::EngageVisibleEnemy),
                },
                HtnGoal {
                    label: "advance_under_smoke".into(),
                    favored_task: Some(TaskType::FlankTarget),
                },
            ],
            HtnRootGoal::HoldSector => vec![
                HtnGoal {
                    label: "hold_sector".into(),
                    favored_task: Some(TaskType::HoldCover),
                },
                HtnGoal {
                    label: "cover_east_corner".into(),
                    favored_task: Some(TaskType::SuppressFire),
                },
            ],
            HtnRootGoal::ReachObjective => vec![
                HtnGoal {
                    label: "reach_objective".into(),
                    favored_task: Some(TaskType::FollowOrder),
                },
                HtnGoal {
                    label: "advance_route".into(),
                    favored_task: Some(TaskType::ScoutAhead),
                },
            ],
            HtnRootGoal::Patrol => vec![HtnGoal {
                label: "patrol".into(),
                favored_task: Some(TaskType::Patrol),
            }],
            HtnRootGoal::BreachThroughTerrain => vec![
                HtnGoal {
                    label: "breach_through_terrain".into(),
                    favored_task: Some(TaskType::Demolish),
                },
                HtnGoal {
                    label: "carve_chokepoint".into(),
                    favored_task: Some(TaskType::DigCover),
                },
            ],
        }
    }

    /// Decide on a fresh root goal from context. Pure / deterministic.
    pub fn pick_root(ctx: &ThinkingContext<'_>) -> HtnRootGoal {
        if ctx.downed_ally_within_reach || ctx.ally_chassis_critical {
            HtnRootGoal::ProtectSquad
        } else if ctx.under_fire {
            HtnRootGoal::HoldSector
        } else if ctx.enemy_visible && ctx.role.as_ref() == "assault" {
            HtnRootGoal::PressAttack
        } else if ctx.has_objective_target {
            HtnRootGoal::ReachObjective
        } else if ctx.terrain_breach_within_range && ctx.role.as_ref() == "engineer" {
            HtnRootGoal::BreachThroughTerrain
        } else if !ctx.enemy_visible && !ctx.recent_sound_within_range {
            HtnRootGoal::Patrol
        } else {
            HtnRootGoal::HoldSector
        }
    }

    /// Re-plan if the root goal changed OR the stack is empty.
    fn replan(&mut self, root: HtnRootGoal, tick: u64) {
        self.root = root;
        self.stack = Self::decompose(root);
        self.last_stack_label = self.stack.iter().map(|g| g.label.clone()).collect::<Vec<_>>().join("/");
        if self.last_stack_label.is_empty() {
            self.last_stack_label = "idle".to_string();
        }
        self.last_replan_tick = tick;
    }
}

impl Default for HtnLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Layer for HtnLayer {
    fn kind(&self) -> LayerKind {
        LayerKind::Htn
    }

    fn tick_layer(&mut self, ctx: &mut ThinkingContext<'_>) -> LayerOutput {
        let root = Self::pick_root(ctx);
        if root != self.root || self.stack.is_empty() {
            self.replan(root, ctx.tick);
        }
        ctx.htn_goal_stack = self.last_stack_label.clone();
        let top_favored = self.stack.last().and_then(|g| g.favored_task);
        LayerOutput {
            override_task: top_favored,
            reason: self.root.as_str(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thinking_stack::ThinkingContext;

    #[test]
    fn protect_squad_triggers_on_downed_ally() {
        let mut ctx = ThinkingContext::stub();
        ctx.downed_ally_within_reach = true;
        assert_eq!(HtnLayer::pick_root(&ctx), HtnRootGoal::ProtectSquad);
    }

    #[test]
    fn empty_stack_replans() {
        let mut layer = HtnLayer::new();
        let mut ctx = ThinkingContext::stub();
        ctx.has_objective_target = true;
        let _ = layer.tick_layer(&mut ctx);
        assert!(!layer.stack.is_empty());
        assert!(layer.last_stack_label.contains("reach_objective"));
    }

    #[test]
    fn root_change_resets_stack() {
        let mut layer = HtnLayer::new();
        let mut ctx = ThinkingContext::stub();
        ctx.has_objective_target = true;
        let _ = layer.tick_layer(&mut ctx);
        ctx.has_objective_target = false;
        ctx.under_fire = true;
        let _ = layer.tick_layer(&mut ctx);
        assert_eq!(layer.root, HtnRootGoal::HoldSector);
    }
}
