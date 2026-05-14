//! M7-A: top-level 5-layer thinking stack.
//!
//! Runs Layer 1 (Reactive) → Layer 5 (LLM prior) → Layer 4 (HTN) → Layer 2
//! (Utility) → Layer 3 (BT) in dependency order per spec § 5-layer thinking
//! stack. Layer 1 can short-circuit the upper layers via an `override_task`;
//! upper layers refine the choice down to a leaf behavior action.
//!
//! Per-bot tick output is `AiTickOutput`. The engine consumes it to dispatch
//! intent + emit `ai.reason_label_changed` + `ai.thinking_layer_invoked`.

use serde::{Deserialize, Serialize};

use crate::archetype::Archetype;
use crate::autonomy::{AutonomyMode, DoctrineMode};
use crate::behavior_tree::{BehaviorAction, BehaviorTreeLayer};
use crate::bot_memory::BotMemory;
use crate::htn::HtnLayer;
use crate::llm_prior::LlmPriorLayer;
use crate::priority::PriorityTable;
use crate::reactive::ReactiveLayer;
use crate::reason_label::{ReasonLabel, ReasonLabelRing};
use crate::task::TaskType;
use crate::utility::{ScoredTask, UtilityLayer};

/// **M7-A**: identifier for one of the 5 layers (logging / event surface).
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerKind {
    Reactive = 1,
    Utility = 2,
    BehaviorTree = 3,
    Htn = 4,
    LlmPrior = 5,
}

impl LayerKind {
    pub fn as_str(self) -> &'static str {
        match self {
            LayerKind::Reactive => "reactive",
            LayerKind::Utility => "utility",
            LayerKind::BehaviorTree => "behavior_tree",
            LayerKind::Htn => "htn",
            LayerKind::LlmPrior => "llm_prior",
        }
    }

    pub fn layer_index(self) -> u8 {
        self as u8
    }
}

/// **M7-A**: output of any single layer's `tick_layer`.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerOutput {
    /// Override task this layer wants the stack to commit to (or `None`
    /// to defer to subsequent layers).
    pub override_task: Option<TaskType>,
    /// Reason / state label for the layer (used in
    /// `ai.thinking_layer_invoked` payload).
    pub reason: &'static str,
}

/// **M7-A**: per-tick context shared across layers. The engine fills this
/// snapshot once per AI tick; layers mutate the trail/result fields as they
/// go but NEVER mutate the world snapshot fields.
///
/// All fields are owned/copy types so the context is a value type the engine
/// can construct cheaply per tick.
#[derive(Debug, Clone, PartialEq)]
pub struct ThinkingContext<'a> {
    pub tick: u64,
    pub tick_rate_hz: u32,
    pub actor_id: u64,
    pub archetype: Archetype,
    pub autonomy: AutonomyMode,
    pub doctrine_mode: DoctrineMode,
    pub role: std::borrow::Cow<'a, str>,

    pub self_hp_fraction: f32,
    pub mood_normalized: f32,

    pub enemy_visible: bool,
    pub enemy_distance_normalized: f32,
    pub point_blank_threat: bool,
    pub recent_sound_within_range: bool,
    pub incoming_projectile_eta_ticks: u32,
    pub emergency_dodge_window_ticks: u32,
    pub friendly_in_line_of_fire: bool,
    pub friendly_in_blast_radius: bool,
    pub under_fire: bool,
    pub in_cover: bool,
    pub cover_unavailable: bool,
    pub high_ground: bool,
    pub squadmate_flanking: bool,
    pub squadmate_suppressing: bool,
    pub demolish_target_available: bool,
    pub has_traps_available: bool,
    pub enemy_likely_route: bool,
    pub downed_ally_within_reach: bool,
    pub downed_ally_in_squad: bool,
    pub ally_chassis_critical: bool,
    pub ally_chassis_degraded: bool,
    pub ally_under_fire: bool,
    pub brain_actor_threatened: bool,
    pub terrain_breach_within_range: bool,
    pub has_explicit_order: bool,
    pub has_objective_target: bool,
    pub unknown_grid_fraction: f32,

    pub tag_bonus_task: Option<TaskType>,
    pub tag_bonus_strength: f32,

    /// Filled by Layer 5 (LLM prior).
    pub doctrine: String,
    /// Filled by Layer 4 (HTN).
    pub htn_goal_stack: String,
    /// Filled by Layer 3 (BT).
    pub behavior_tree_trail: String,
    /// Set by Layer 1/2 when a task is chosen.
    pub chosen_task: Option<TaskType>,
}

impl<'a> ThinkingContext<'a> {
    pub fn stub() -> Self {
        Self {
            tick: 0,
            tick_rate_hz: 60,
            actor_id: 0,
            archetype: Archetype::Rifleman,
            autonomy: AutonomyMode::FullAuto,
            doctrine_mode: DoctrineMode::Defensive,
            role: std::borrow::Cow::Borrowed("rifleman"),
            self_hp_fraction: 1.0,
            mood_normalized: 0.0,
            enemy_visible: false,
            enemy_distance_normalized: 1.0,
            point_blank_threat: false,
            recent_sound_within_range: false,
            incoming_projectile_eta_ticks: u32::MAX,
            emergency_dodge_window_ticks: 12,
            friendly_in_line_of_fire: false,
            friendly_in_blast_radius: false,
            under_fire: false,
            in_cover: false,
            cover_unavailable: false,
            high_ground: false,
            squadmate_flanking: false,
            squadmate_suppressing: false,
            demolish_target_available: false,
            has_traps_available: false,
            enemy_likely_route: false,
            downed_ally_within_reach: false,
            downed_ally_in_squad: false,
            ally_chassis_critical: false,
            ally_chassis_degraded: false,
            ally_under_fire: false,
            brain_actor_threatened: false,
            terrain_breach_within_range: false,
            has_explicit_order: false,
            has_objective_target: false,
            unknown_grid_fraction: 0.0,
            tag_bonus_task: None,
            tag_bonus_strength: 0.0,
            doctrine: "defensive".to_string(),
            htn_goal_stack: "idle".to_string(),
            behavior_tree_trail: "idle".to_string(),
            chosen_task: None,
        }
    }
}

/// **M7-A**: layer trait. Every layer carries its own state + tick function.
pub trait Layer {
    fn kind(&self) -> LayerKind;
    fn tick_layer(&mut self, ctx: &mut ThinkingContext<'_>) -> LayerOutput;
}

/// **M7-A**: AI tick output the engine consumes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiTickOutput {
    pub actor_id: u64,
    pub chosen_action: BehaviorAction,
    pub chosen_task: TaskType,
    pub reason_label: ReasonLabel,
    pub reason_label_changed: bool,
    pub utility_candidates: Vec<ScoredTask>,
    pub htn_goal_stack: String,
    pub behavior_tree_trail: String,
    pub layers_invoked: Vec<LayerKind>,
    pub reactive_override: bool,
}

/// **M7-A**: the 5-layer stack instance per bot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThinkingStack {
    pub archetype: Archetype,
    pub autonomy: AutonomyMode,
    pub doctrine_mode: DoctrineMode,
    pub priority: PriorityTable,
    pub memory: BotMemory,
    pub reason_labels: ReasonLabelRing,

    pub reactive: ReactiveLayer,
    pub utility: UtilityLayer,
    pub behavior_tree: BehaviorTreeLayer,
    pub htn: HtnLayer,
    pub llm_prior: LlmPriorLayer,
}

impl ThinkingStack {
    pub fn new(archetype: Archetype) -> Self {
        let priority = archetype.role_template();
        Self {
            archetype,
            autonomy: AutonomyMode::FullAuto,
            doctrine_mode: DoctrineMode::Defensive,
            priority,
            memory: BotMemory::new(),
            reason_labels: ReasonLabelRing::new(),
            reactive: ReactiveLayer::new(),
            utility: UtilityLayer::new(priority),
            behavior_tree: BehaviorTreeLayer::new(),
            htn: HtnLayer::new(),
            llm_prior: LlmPriorLayer::new(),
        }
    }

    /// **M7-A**: apply a new role template (e.g. when the engine assigns
    /// a different archetype mid-mission). Keeps utility scorer in sync.
    pub fn apply_archetype(&mut self, archetype: Archetype) {
        self.archetype = archetype;
        self.priority = archetype.role_template();
        self.utility.priority = self.priority;
    }

    /// **M7-A**: drive all 5 layers in dependency order.
    ///
    /// Order per spec:
    /// 1. Layer 1 (Reactive)
    /// 2. Layer 5 (LLM prior) — populates doctrine string
    /// 3. Layer 4 (HTN) — populates goal stack + favored task
    /// 4. Layer 2 (Utility) — scores all 22 tasks
    /// 5. Layer 3 (BT) — expands chosen task → BT trail + leaf action
    pub fn tick(&mut self, mut ctx: ThinkingContext<'_>) -> AiTickOutput {
        let mut invoked = Vec::with_capacity(5);

        // Layer 1 — Reactive.
        let r_out = self.reactive.tick_layer(&mut ctx);
        invoked.push(LayerKind::Reactive);
        let reactive_override = r_out.override_task.is_some();

        // Layer 5 — LLM prior.
        let _l5 = self.llm_prior.tick_layer(&mut ctx);
        invoked.push(LayerKind::LlmPrior);

        // Layer 4 — HTN.
        let h_out = self.htn.tick_layer(&mut ctx);
        invoked.push(LayerKind::Htn);

        // Pre-bias the utility scorer by adding a tag bonus to the HTN
        // favored task so the utility chooses it more often (unless
        // reactive overrides).
        if let Some(favored) = h_out.override_task {
            if ctx.tag_bonus_task.is_none() {
                ctx.tag_bonus_task = Some(favored);
                ctx.tag_bonus_strength = 0.15;
            }
        }

        // Layer 2 — Utility.
        let u_out = self.utility.tick_layer(&mut ctx);
        invoked.push(LayerKind::Utility);

        // Reconcile final task: Reactive override beats Utility choice.
        let final_task = r_out.override_task.or(u_out.override_task).unwrap_or(TaskType::Idle);
        ctx.chosen_task = Some(final_task);

        // Layer 3 — BT.
        let bt_out = self.behavior_tree.tick_layer(&mut ctx);
        invoked.push(LayerKind::BehaviorTree);
        let leaf_action = self.behavior_tree.last_action.unwrap_or(BehaviorAction::Idle);

        let candidates = self.utility.top3();
        let chosen_candidate = candidates.iter().find(|c| c.task == final_task).cloned();
        let (score, base, prio_mult, tag_bonus) = match chosen_candidate {
            Some(c) => (c.score, c.base, c.priority_mult, c.tag_bonus),
            None => (0.0, 0.0, self.priority.multiplier(final_task), 1.0),
        };
        let candidates_for_label: Vec<(String, f32)> = candidates
            .iter()
            .map(|c| (format_task_camel(c.task), c.score))
            .collect();
        let label = ReasonLabel {
            chosen_task: format_task_camel(final_task),
            chosen_target: None,
            score,
            score_base: base,
            score_priority_multiplier: prio_mult,
            score_tag_bonus: tag_bonus,
            candidates: candidates_for_label,
            htn_goal_stack: ctx.htn_goal_stack.clone(),
            behavior_tree_node: ctx.behavior_tree_trail.clone(),
            doctrine: ctx.doctrine.clone(),
            role: self.archetype.as_str().to_string(),
        };
        let changed = self.reason_labels.push(label.clone());
        // M2-style trace for run bundles.
        tracing::trace!(target: "cf_ai::thinking_stack", actor = ctx.actor_id, layer = ?bt_out.reason, label = %label.format());

        AiTickOutput {
            actor_id: ctx.actor_id,
            chosen_action: leaf_action,
            chosen_task: final_task,
            reason_label: label,
            reason_label_changed: changed,
            utility_candidates: candidates,
            htn_goal_stack: ctx.htn_goal_stack,
            behavior_tree_trail: ctx.behavior_tree_trail,
            layers_invoked: invoked,
            reactive_override,
        }
    }

    /// Bytes for the determinism checksum (covers PriorityTable + last
    /// reason label string). Memory grid + ring are NOT hashed at M7-A
    /// to keep checksum cost bounded; M8A's snapshot/restore covers them
    /// via separate snapshot events.
    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(96);
        out.push(self.archetype as u8);
        out.push(self.autonomy as u8);
        out.push(self.doctrine_mode as u8);
        out.extend_from_slice(&self.priority.checksum_bytes());
        if let Some(latest) = self.reason_labels.latest() {
            out.extend_from_slice(latest.format().as_bytes());
        }
        out
    }
}

/// Convert a `TaskType` to UpperCamelCase for the reason label.
/// Stable identifier; preserves replay determinism.
pub fn format_task_camel(task: TaskType) -> String {
    match task {
        TaskType::EngageVisibleEnemy => "EngageVisibleEnemy",
        TaskType::SuppressFire => "SuppressFire",
        TaskType::HoldCover => "HoldCover",
        TaskType::FlankTarget => "FlankTarget",
        TaskType::ThrowGrenade => "ThrowGrenade",
        TaskType::Demolish => "Demolish",
        TaskType::SharpshootTarget => "SharpshootTarget",
        TaskType::MarkThreats => "MarkThreats",
        TaskType::ScoutAhead => "ScoutAhead",
        TaskType::RepairChassisModule => "RepairChassisModule",
        TaskType::RepairTerrainBreach => "RepairTerrainBreach",
        TaskType::SetTrap => "SetTrap",
        TaskType::DigCover => "DigCover",
        TaskType::TriageDownedAlly => "TriageDownedAlly",
        TaskType::HealSelf => "HealSelf",
        TaskType::CoverAlly => "CoverAlly",
        TaskType::DefendBrainActor => "DefendBrainActor",
        TaskType::InvestigateSound => "InvestigateSound",
        TaskType::FollowOrder => "FollowOrder",
        TaskType::RetreatToCover => "RetreatToCover",
        TaskType::Patrol => "Patrol",
        TaskType::Idle => "Idle",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn medic_picks_triage_when_ally_downed() {
        let mut stack = ThinkingStack::new(Archetype::Medic);
        let mut ctx = ThinkingContext::stub();
        ctx.actor_id = 42;
        ctx.downed_ally_within_reach = true;
        let out = stack.tick(ctx);
        assert_eq!(out.chosen_task, TaskType::TriageDownedAlly);
        assert!(out.reason_label.format().contains("chosen=TriageDownedAlly"));
        assert!(out.layers_invoked.contains(&LayerKind::Utility));
    }

    #[test]
    fn rifleman_picks_engage_when_enemy_close() {
        let mut stack = ThinkingStack::new(Archetype::Rifleman);
        let mut ctx = ThinkingContext::stub();
        ctx.enemy_visible = true;
        ctx.enemy_distance_normalized = 0.2;
        let out = stack.tick(ctx);
        assert!(matches!(
            out.chosen_task,
            TaskType::EngageVisibleEnemy | TaskType::SuppressFire
        ));
    }

    #[test]
    fn reactive_critical_hp_forces_retreat() {
        let mut stack = ThinkingStack::new(Archetype::Assault);
        let mut ctx = ThinkingContext::stub();
        ctx.self_hp_fraction = 0.05;
        ctx.enemy_visible = true;
        let out = stack.tick(ctx);
        assert!(out.reactive_override);
        assert_eq!(out.chosen_task, TaskType::RetreatToCover);
    }

    #[test]
    fn reason_label_change_flag_works() {
        let mut stack = ThinkingStack::new(Archetype::Rifleman);
        let ctx1 = ThinkingContext::stub();
        let out1 = stack.tick(ctx1.clone());
        assert!(out1.reason_label_changed);
        let out2 = stack.tick(ctx1);
        assert!(
            !out2.reason_label_changed,
            "identical inputs must produce identical labels"
        );
    }
}
