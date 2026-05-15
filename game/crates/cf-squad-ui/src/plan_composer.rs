//! Plan Composer — drag-and-drop sequences of up to 8 tactical moves
//! per spec § Smart commandable AI — Plan Composer.

use serde::{Deserialize, Serialize};

/// Spec-mandated maximum number of plan steps per actor.
pub const MAX_PLAN_STEPS: usize = 8;

/// Per-step action kind. The spec lists Door Kickers / Ready or Not
/// patterns; the enum covers the canonical 8-step set + a `Custom`
/// escape hatch for cfctl-driven scripted plans.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlanStepKind {
    /// Move to a world point.
    MoveTo {
        /// Destination x.
        x: f32,
        /// Destination y.
        y: f32,
    },
    /// Flank left around the next contact.
    FlankLeft,
    /// Flank right around the next contact.
    FlankRight,
    /// Breach the door at the cursor.
    BreachDoor,
    /// Take an overwatch posture from current position.
    Overwatch,
    /// Throw a flashbang into the current sector.
    ThrowFlash,
    /// Stack on the left side of the next door.
    StackLeft,
    /// Stack on the right side of the next door.
    StackRight,
    /// Hold here until the player issues "GO".
    WaitForGo,
    /// Hold east corner until the next order.
    HoldEastCorner,
    /// Custom scripted command (cfctl-only).
    Custom(String),
}

impl PlanStepKind {
    /// Stable cfctl-wire identifier (used in event payloads).
    pub fn label(&self) -> &str {
        match self {
            PlanStepKind::MoveTo { .. } => "move_to",
            PlanStepKind::FlankLeft => "flank_left",
            PlanStepKind::FlankRight => "flank_right",
            PlanStepKind::BreachDoor => "breach_door",
            PlanStepKind::Overwatch => "overwatch",
            PlanStepKind::ThrowFlash => "throw_flash",
            PlanStepKind::StackLeft => "stack_left",
            PlanStepKind::StackRight => "stack_right",
            PlanStepKind::WaitForGo => "wait_for_go",
            PlanStepKind::HoldEastCorner => "hold_east_corner",
            PlanStepKind::Custom(s) => s.as_str(),
        }
    }
}

/// One step in a per-bot plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanStep {
    /// Step ordinal in the plan (1-based for HUD readability).
    pub ordinal: u8,
    /// Action kind.
    pub kind: PlanStepKind,
    /// Optional ETA in seconds (player-facing label).
    pub eta_seconds: Option<f32>,
}

/// Full plan for a single bot. Up to `MAX_PLAN_STEPS` steps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    /// Bot the plan applies to.
    pub actor_id: u64,
    /// Ordered steps.
    pub steps: Vec<PlanStep>,
}

/// Failure modes for plan composition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanComposeError {
    /// Adding the step would exceed `MAX_PLAN_STEPS`.
    PlanFull,
}

impl Plan {
    /// Build an empty plan for `actor_id`.
    pub fn empty(actor_id: u64) -> Self {
        Self {
            actor_id,
            steps: Vec::new(),
        }
    }

    /// Append a step. Rejects when the cap is reached.
    pub fn add_step(&mut self, kind: PlanStepKind, eta_seconds: Option<f32>) -> Result<(), PlanComposeError> {
        if self.steps.len() >= MAX_PLAN_STEPS {
            return Err(PlanComposeError::PlanFull);
        }
        let ordinal = self.steps.len() as u8 + 1;
        self.steps.push(PlanStep {
            ordinal,
            kind,
            eta_seconds,
        });
        Ok(())
    }

    /// Replace the entire step list (used when the player drag-rearranges).
    /// Rejects when the new list exceeds `MAX_PLAN_STEPS`.
    pub fn replace_steps(&mut self, steps: Vec<PlanStep>) -> Result<(), PlanComposeError> {
        if steps.len() > MAX_PLAN_STEPS {
            return Err(PlanComposeError::PlanFull);
        }
        self.steps = steps
            .into_iter()
            .enumerate()
            .map(|(i, mut s)| {
                s.ordinal = (i + 1) as u8;
                s
            })
            .collect();
        Ok(())
    }

    /// Clear the plan without removing the bot's identity.
    pub fn clear(&mut self) {
        self.steps.clear();
    }
}

/// Construct a plan from a step kind list, performing the cap validation.
/// Convenience for cfctl `act.player.compose_plan`.
pub fn build(actor_id: u64, kinds: Vec<PlanStepKind>) -> Result<Plan, PlanComposeError> {
    if kinds.len() > MAX_PLAN_STEPS {
        return Err(PlanComposeError::PlanFull);
    }
    let mut plan = Plan::empty(actor_id);
    for k in kinds {
        plan.add_step(k, None)?;
    }
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_plan_has_no_steps() {
        let p = Plan::empty(1);
        assert_eq!(p.actor_id, 1);
        assert!(p.steps.is_empty());
    }

    #[test]
    fn add_step_appends_with_increasing_ordinal() {
        let mut p = Plan::empty(1);
        p.add_step(PlanStepKind::FlankLeft, Some(3.0)).unwrap();
        p.add_step(PlanStepKind::Overwatch, None).unwrap();
        assert_eq!(p.steps[0].ordinal, 1);
        assert_eq!(p.steps[1].ordinal, 2);
    }

    #[test]
    fn add_step_rejects_at_cap() {
        let mut p = Plan::empty(1);
        for _ in 0..MAX_PLAN_STEPS {
            p.add_step(PlanStepKind::Overwatch, None).unwrap();
        }
        let err = p.add_step(PlanStepKind::Overwatch, None).unwrap_err();
        assert_eq!(err, PlanComposeError::PlanFull);
    }

    #[test]
    fn replace_steps_re_ordinals() {
        let mut p = Plan::empty(1);
        let steps = vec![
            PlanStep {
                ordinal: 99,
                kind: PlanStepKind::FlankLeft,
                eta_seconds: None,
            },
            PlanStep {
                ordinal: 99,
                kind: PlanStepKind::FlankRight,
                eta_seconds: None,
            },
        ];
        p.replace_steps(steps).unwrap();
        assert_eq!(p.steps[0].ordinal, 1);
        assert_eq!(p.steps[1].ordinal, 2);
    }

    #[test]
    fn replace_steps_rejects_over_cap() {
        let mut p = Plan::empty(1);
        let steps: Vec<_> = (0..MAX_PLAN_STEPS + 1)
            .map(|_| PlanStep {
                ordinal: 1,
                kind: PlanStepKind::Overwatch,
                eta_seconds: None,
            })
            .collect();
        let err = p.replace_steps(steps).unwrap_err();
        assert_eq!(err, PlanComposeError::PlanFull);
    }

    #[test]
    fn build_from_kind_list_caps() {
        let kinds: Vec<PlanStepKind> = (0..MAX_PLAN_STEPS + 1).map(|_| PlanStepKind::Overwatch).collect();
        let err = build(1, kinds).unwrap_err();
        assert_eq!(err, PlanComposeError::PlanFull);
    }

    #[test]
    fn move_to_label() {
        let k = PlanStepKind::MoveTo { x: 1.0, y: 2.0 };
        assert_eq!(k.label(), "move_to");
    }

    #[test]
    fn custom_label_uses_inner_string() {
        let k = PlanStepKind::Custom("breach_window".into());
        assert_eq!(k.label(), "breach_window");
    }
}
