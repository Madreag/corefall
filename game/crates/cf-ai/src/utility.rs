//! M7-A Layer 2: Utility scorer.
//!
//! Spec § 5-layer thinking stack — Layer 2 evaluates every relevant task
//! against the current world state and produces a final score per task:
//!
//! `final_score(task) = base_utility(task, world) × priority_weight[task] / 5.0 × situational_bonus`
//!
//! The scorer is **pure**: same inputs (world state + PriorityTable + mood)
//! → same outputs. The Behavior Tree (Layer 3) consumes the top candidate
//! task to expand into BT nodes.

use serde::{Deserialize, Serialize};

use crate::priority::PriorityTable;
use crate::task::TaskType;
use crate::thinking_stack::{Layer, LayerKind, LayerOutput, ThinkingContext};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoredTask {
    pub task: TaskType,
    pub score: f32,
    pub base: f32,
    pub priority_mult: f32,
    pub tag_bonus: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UtilityLayer {
    pub priority: PriorityTable,
    /// Latest (sorted descending) candidates from the last `tick_layer`.
    pub last_candidates: Vec<ScoredTask>,
    pub last_chosen: Option<TaskType>,
}

impl UtilityLayer {
    pub fn new(priority: PriorityTable) -> Self {
        Self {
            priority,
            last_candidates: Vec::new(),
            last_chosen: None,
        }
    }

    /// Score one task. Inputs flow through `ctx`. The base utility is a
    /// pure function of world-state fields the engine projects onto the
    /// context (no engine queries inside cf-ai).
    fn score_task(&self, task: TaskType, ctx: &ThinkingContext<'_>) -> ScoredTask {
        let base = base_utility(task, ctx);
        let priority_mult = self.priority.multiplier(task);
        let tag_bonus = situational_bonus(task, ctx);
        let raw = base * priority_mult * tag_bonus;
        ScoredTask {
            task,
            score: if raw.is_finite() { raw.max(0.0) } else { 0.0 },
            base,
            priority_mult,
            tag_bonus,
        }
    }

    /// Sort + truncate candidates to the top-3 used in the reason label.
    pub fn top3(&self) -> Vec<ScoredTask> {
        let mut out = self.last_candidates.clone();
        out.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| (a.task.ordinal()).cmp(&b.task.ordinal()))
        });
        out.truncate(3);
        out
    }
}

impl Layer for UtilityLayer {
    fn kind(&self) -> LayerKind {
        LayerKind::Utility
    }

    fn tick_layer(&mut self, ctx: &mut ThinkingContext<'_>) -> LayerOutput {
        let mut scored: Vec<ScoredTask> = Vec::with_capacity(TaskType::COUNT);
        for task in TaskType::ALL.iter() {
            scored.push(self.score_task(*task, ctx));
        }
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| (a.task.ordinal()).cmp(&b.task.ordinal()))
        });
        let chosen = scored.first().map(|s| s.task);
        self.last_chosen = chosen;
        self.last_candidates = scored;
        LayerOutput {
            override_task: chosen,
            reason: "utility_scored",
        }
    }
}

/// score. World state is whatever the engine packed onto `ThinkingContext`.
///
/// Tuning per spec § Smart commandable AI — Utility scorer. The numbers are
/// hand-tuned starting points; M7-B / M8 will calibrate against play data.
pub fn base_utility(task: TaskType, ctx: &ThinkingContext<'_>) -> f32 {
    match task {
        TaskType::EngageVisibleEnemy => {
            if ctx.enemy_visible {
                (1.0 - ctx.enemy_distance_normalized).clamp(0.0, 1.0) * 0.9
            } else {
                0.0
            }
        }
        TaskType::SuppressFire => {
            if ctx.enemy_visible && ctx.squadmate_flanking {
                0.85
            } else if ctx.enemy_visible {
                0.4
            } else {
                0.05
            }
        }
        TaskType::HoldCover => {
            if ctx.under_fire {
                0.75
            } else if ctx.enemy_visible {
                0.5
            } else {
                0.2
            }
        }
        TaskType::FlankTarget => {
            if ctx.enemy_visible && ctx.squadmate_suppressing {
                0.8
            } else if ctx.enemy_visible {
                0.3
            } else {
                0.0
            }
        }
        TaskType::ThrowGrenade => {
            if ctx.enemy_visible && ctx.enemy_distance_normalized < 0.4 && !ctx.friendly_in_blast_radius {
                0.7
            } else {
                0.05
            }
        }
        TaskType::Demolish => {
            if ctx.demolish_target_available {
                0.55
            } else {
                0.0
            }
        }
        TaskType::SharpshootTarget => {
            if ctx.enemy_visible && ctx.high_ground {
                0.9
            } else if ctx.enemy_visible {
                0.55
            } else {
                0.0
            }
        }
        TaskType::MarkThreats => {
            if ctx.enemy_visible {
                0.5
            } else {
                0.1
            }
        }
        TaskType::ScoutAhead => 0.35 + ctx.unknown_grid_fraction * 0.4,
        TaskType::RepairChassisModule => {
            if ctx.ally_chassis_critical {
                0.95
            } else if ctx.ally_chassis_degraded {
                0.7
            } else {
                0.0
            }
        }
        TaskType::RepairTerrainBreach => {
            if ctx.terrain_breach_within_range {
                0.65
            } else {
                0.0
            }
        }
        TaskType::SetTrap => {
            if ctx.has_traps_available && ctx.enemy_likely_route {
                0.55
            } else {
                0.05
            }
        }
        TaskType::DigCover => {
            if ctx.cover_unavailable && ctx.under_fire {
                0.6
            } else {
                0.1
            }
        }
        TaskType::TriageDownedAlly => {
            if ctx.downed_ally_within_reach {
                0.95
            } else if ctx.downed_ally_in_squad {
                0.5
            } else {
                0.0
            }
        }
        TaskType::HealSelf => {
            if ctx.self_hp_fraction < 0.4 {
                0.7
            } else {
                0.05
            }
        }
        TaskType::CoverAlly => {
            if ctx.ally_under_fire {
                0.55
            } else {
                0.1
            }
        }
        TaskType::DefendBrainActor => {
            if ctx.brain_actor_threatened {
                0.85
            } else {
                0.1
            }
        }
        TaskType::InvestigateSound => {
            if ctx.recent_sound_within_range {
                0.5
            } else {
                0.05
            }
        }
        TaskType::FollowOrder => {
            if ctx.has_explicit_order {
                0.85
            } else {
                0.0
            }
        }
        TaskType::RetreatToCover => {
            if ctx.under_fire && !ctx.in_cover {
                0.6
            } else if ctx.self_hp_fraction < 0.3 {
                0.55
            } else {
                0.1
            }
        }
        TaskType::Patrol => {
            if !ctx.enemy_visible && !ctx.recent_sound_within_range {
                0.35
            } else {
                0.05
            }
        }
        TaskType::Idle => 0.05,
    }
}

/// player MMB-tag (+0.5 multiplier on tasks targeting the tagged actor),
/// mood/stress modifier (low mood reduces aggressive task multipliers),
/// doctrine bias.
pub fn situational_bonus(task: TaskType, ctx: &ThinkingContext<'_>) -> f32 {
    let mut mult: f32 = 1.0;
    if ctx.tag_bonus_task == Some(task) {
        mult *= 1.0 + ctx.tag_bonus_strength;
    }
    // Mood/stress: 0.0 = neutral, +1.0 = euphoric, -1.0 = stressed.
    let mood = ctx.mood_normalized.clamp(-1.0, 1.0);
    let aggressive_tasks = matches!(
        task,
        TaskType::EngageVisibleEnemy
            | TaskType::FlankTarget
            | TaskType::ThrowGrenade
            | TaskType::Demolish
            | TaskType::SharpshootTarget
    );
    let defensive_tasks = matches!(
        task,
        TaskType::HoldCover | TaskType::RetreatToCover | TaskType::HealSelf
    );
    if aggressive_tasks {
        mult *= (1.0 + mood * 0.15).clamp(0.6, 1.4);
    } else if defensive_tasks {
        mult *= (1.0 - mood * 0.15).clamp(0.6, 1.4);
    }
    // Doctrine: aggressive shifts +0.1 for offensive tasks; defensive
    // shifts +0.1 for defensive tasks; scout shifts +0.1 for scouting.
    match ctx.doctrine.as_ref() {
        "aggressive" if aggressive_tasks => mult *= 1.1,
        "defensive" if defensive_tasks => mult *= 1.1,
        "scout" if matches!(task, TaskType::ScoutAhead | TaskType::MarkThreats | TaskType::Patrol) => mult *= 1.1,
        _ => {}
    }
    mult
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engage_score_zero_without_visible_enemy() {
        let mut ctx = ThinkingContext::stub();
        ctx.enemy_visible = false;
        let score = base_utility(TaskType::EngageVisibleEnemy, &ctx);
        assert!(score < 0.001);
    }

    #[test]
    fn triage_dominates_when_downed_in_reach() {
        let mut ctx = ThinkingContext::stub();
        ctx.downed_ally_within_reach = true;
        let triage = base_utility(TaskType::TriageDownedAlly, &ctx);
        let engage = base_utility(TaskType::EngageVisibleEnemy, &ctx);
        assert!(triage > engage);
    }

    #[test]
    fn tag_bonus_only_applies_to_tagged_task() {
        let mut ctx = ThinkingContext::stub();
        ctx.tag_bonus_task = Some(TaskType::SuppressFire);
        ctx.tag_bonus_strength = 0.5;
        let bonus = situational_bonus(TaskType::SuppressFire, &ctx);
        let no_bonus = situational_bonus(TaskType::HoldCover, &ctx);
        assert!(bonus > no_bonus);
    }

    #[test]
    fn medic_utility_with_template_picks_triage() {
        use crate::archetype::Archetype;
        let mut layer = UtilityLayer::new(Archetype::Medic.role_template());
        let mut ctx = ThinkingContext::stub();
        ctx.downed_ally_within_reach = true;
        ctx.enemy_visible = true;
        ctx.enemy_distance_normalized = 0.3;
        let out = layer.tick_layer(&mut ctx);
        assert_eq!(out.override_task, Some(TaskType::TriageDownedAlly));
    }
}
