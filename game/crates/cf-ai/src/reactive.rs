//! M7-A Layer 1: Reactive layer.
//!
//! Spec § 5-layer thinking stack — Layer 1 is the emergency-response gate
//! that ALWAYS runs first. If the world state warrants an immediate override
//! (incoming projectile within 0.2s, point-blank threat, friendly fire
//! imminent), the reactive layer returns a high-priority intent that the
//! upper layers MUST honour.
//!
//! M2 ReactiveGuard is the M1 baseline; Layer 1 in M7 wraps + extends it so
//! the thinking stack can call into the same FSM via the layer trait.

use serde::{Deserialize, Serialize};

use crate::thinking_stack::{Layer, LayerKind, LayerOutput, ThinkingContext};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ReactiveDecision {
    /// No reactive override — defer to upper layers.
    #[default]
    Defer,
    /// Incoming projectile / explosive within emergency window. Dodge.
    EmergencyDodge,
    /// Point-blank threat appeared in cone. Snap-shoot.
    SnapShoot,
    /// Friendly entered line-of-fire. Hold trigger.
    FriendlyInLine,
    /// Self HP critically low. Force retreat regardless of role.
    CriticalHpRetreat,
}

impl ReactiveDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            ReactiveDecision::Defer => "defer",
            ReactiveDecision::EmergencyDodge => "emergency_dodge",
            ReactiveDecision::SnapShoot => "snap_shoot",
            ReactiveDecision::FriendlyInLine => "friendly_in_line",
            ReactiveDecision::CriticalHpRetreat => "critical_hp_retreat",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ReactiveLayer {
    /// Most recent decision the layer produced (for debug + audit).
    pub last_decision: ReactiveDecision,
    /// Tick on which `last_decision` was made.
    pub last_decision_tick: u64,
}

impl ReactiveLayer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Per-spec emergency gates. Each gate is a pure function of the
    /// snapshot inputs supplied via `ThinkingContext`. Deterministic.
    fn evaluate(&self, ctx: &ThinkingContext<'_>) -> ReactiveDecision {
        if ctx.self_hp_fraction <= 0.10 {
            return ReactiveDecision::CriticalHpRetreat;
        }
        if ctx.incoming_projectile_eta_ticks < ctx.emergency_dodge_window_ticks {
            return ReactiveDecision::EmergencyDodge;
        }
        if ctx.friendly_in_line_of_fire {
            return ReactiveDecision::FriendlyInLine;
        }
        if ctx.point_blank_threat {
            return ReactiveDecision::SnapShoot;
        }
        ReactiveDecision::Defer
    }
}

impl Layer for ReactiveLayer {
    fn kind(&self) -> LayerKind {
        LayerKind::Reactive
    }

    fn tick_layer(&mut self, ctx: &mut ThinkingContext<'_>) -> LayerOutput {
        let decision = self.evaluate(ctx);
        self.last_decision = decision;
        self.last_decision_tick = ctx.tick;
        let override_task = match decision {
            ReactiveDecision::Defer => None,
            ReactiveDecision::EmergencyDodge => Some(crate::task::TaskType::RetreatToCover),
            ReactiveDecision::SnapShoot => Some(crate::task::TaskType::EngageVisibleEnemy),
            ReactiveDecision::FriendlyInLine => Some(crate::task::TaskType::HoldCover),
            ReactiveDecision::CriticalHpRetreat => Some(crate::task::TaskType::RetreatToCover),
        };
        LayerOutput {
            override_task,
            reason: decision.as_str(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thinking_stack::ThinkingContext;

    fn ctx_default() -> ThinkingContext<'static> {
        ThinkingContext::stub()
    }

    #[test]
    fn defers_when_nothing_emergent() {
        let mut layer = ReactiveLayer::new();
        let mut ctx = ctx_default();
        let out = layer.tick_layer(&mut ctx);
        assert!(out.override_task.is_none());
        assert_eq!(layer.last_decision, ReactiveDecision::Defer);
    }

    #[test]
    fn critical_hp_overrides() {
        let mut layer = ReactiveLayer::new();
        let mut ctx = ctx_default();
        ctx.self_hp_fraction = 0.05;
        let out = layer.tick_layer(&mut ctx);
        assert_eq!(layer.last_decision, ReactiveDecision::CriticalHpRetreat);
        assert!(out.override_task.is_some());
    }

    #[test]
    fn imminent_projectile_triggers_dodge() {
        let mut layer = ReactiveLayer::new();
        let mut ctx = ctx_default();
        ctx.incoming_projectile_eta_ticks = 5;
        ctx.emergency_dodge_window_ticks = 12;
        let out = layer.tick_layer(&mut ctx);
        assert_eq!(layer.last_decision, ReactiveDecision::EmergencyDodge);
        assert!(out.override_task.is_some());
    }
}
