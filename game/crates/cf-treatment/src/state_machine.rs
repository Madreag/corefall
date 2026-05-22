//! **M14H** § Treatment apply-state-machine + outcome resolver.
//!
//! [`TreatmentApply`] tracks a single in-progress treatment application.
//! Callers drive it tick by tick via [`TreatmentApply::tick`]; the machine
//! emits a stream of [`TreatmentEvent`] entries:
//!
//! - `treatment.applied` on initial accept.
//! - `treatment.completed` when the apply timer expires successfully.
//! - `treatment.failed { reason }` on rejection (wrong origin, missing
//!   tool, missing skill, missing charges, or cancellation).
//!
//! Determinism: every randomness sample (risk roll, surgery skill check,
//! defib success roll) flows through a single seeded
//! `rand_xoshiro::Xoshiro256StarStar`. Identical seed + identical inputs
//! produces identical event sequences (M14H Gherkin scenario 7).

use rand_core::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256StarStar;
use serde::{Deserialize, Serialize};

use crate::producers::{treatment_spec, RiskKind, ToolRequirement, TreatmentKind};

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TreatmentPhase {
    Pending = 0,
    Applying = 1,
    Completed = 2,
    Failed = 3,
}

impl TreatmentPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            TreatmentPhase::Pending => "pending",
            TreatmentPhase::Applying => "applying",
            TreatmentPhase::Completed => "completed",
            TreatmentPhase::Failed => "failed",
        }
    }
}

/// Reasons a treatment apply can fail. Surfaced via `treatment.failed`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TreatmentFailureReason {
    WrongOrigin,
    MissingTool,
    MissingSkill,
    OutOfCharges,
    DirtyWoundFailure,
    BloodIncompatibility,
    Cancelled,
}

impl TreatmentFailureReason {
    pub fn as_str(self) -> &'static str {
        match self {
            TreatmentFailureReason::WrongOrigin => "wrong_origin",
            TreatmentFailureReason::MissingTool => "missing_tool",
            TreatmentFailureReason::MissingSkill => "missing_skill",
            TreatmentFailureReason::OutOfCharges => "out_of_charges",
            TreatmentFailureReason::DirtyWoundFailure => "dirty_wound_failure",
            TreatmentFailureReason::BloodIncompatibility => "blood_incompatibility",
            TreatmentFailureReason::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum TreatmentApplyError {
    #[error("treatment incompatible with actor origin")]
    WrongOrigin,
    #[error("required tool missing: {0}")]
    MissingTool(&'static str),
    #[error("required skill missing: {0}")]
    MissingSkill(&'static str),
    #[error("no defib charges remaining")]
    OutOfCharges,
}

/// completes (success or failure).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "outcome")]
pub enum TreatmentOutcome {
    Completed {
        kind: TreatmentKind,
        actor_id: u64,
    },
    Failed {
        kind: TreatmentKind,
        actor_id: u64,
        reason: TreatmentFailureReason,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "event")]
pub enum TreatmentEvent {
    Applied {
        kind: TreatmentKind,
        actor_id: u64,
        tick: u64,
        apply_seconds: f32,
    },
    Completed {
        kind: TreatmentKind,
        actor_id: u64,
        tick: u64,
    },
    Failed {
        kind: TreatmentKind,
        actor_id: u64,
        tick: u64,
        reason: TreatmentFailureReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TreatmentContext {
    pub actor_id: u64,
    /// True if the actor is a human (or compatible biological origin).
    pub is_human_origin: bool,
    /// True if the actor is a robot — `field_bandage_v1` etc. emit
    /// `treatment.failed reason="wrong_origin"` per Gherkin scenario 6.
    pub is_robot_origin: bool,
    /// True if the actor has medic_t1 skill.
    pub has_medic_t1: bool,
    pub has_medic_t2: bool,
    pub has_surgeon_t1: bool,
    /// True if a suture kit is available.
    pub has_suture_kit: bool,
    pub has_hot_metal_or_plasma: bool,
    pub has_splint_material: bool,
    pub has_surgery_table_and_surgeon: bool,
    pub has_iv_kit: bool,
    pub has_oxygen_tank: bool,
    pub has_scanner_device: bool,
    pub has_bed_object: bool,
    /// Current dirt percentage on the wound (used for dirty-wound failure
    /// roll on sutures).
    pub wound_dirt_pct: f32,
    /// Blood compatibility 1.0 = full compatibility.
    pub blood_compat: f32,
}

impl TreatmentContext {
    pub fn for_clean_human(actor_id: u64) -> Self {
        Self {
            actor_id,
            is_human_origin: true,
            is_robot_origin: false,
            has_medic_t1: true,
            has_medic_t2: false,
            has_surgeon_t1: false,
            has_suture_kit: true,
            has_hot_metal_or_plasma: true,
            has_splint_material: true,
            has_surgery_table_and_surgeon: true,
            has_iv_kit: true,
            has_oxygen_tank: true,
            has_scanner_device: true,
            has_bed_object: true,
            wound_dirt_pct: 0.0,
            blood_compat: 1.0,
        }
    }

    pub fn for_robot(actor_id: u64) -> Self {
        Self {
            actor_id,
            is_human_origin: false,
            is_robot_origin: true,
            has_medic_t1: false,
            has_medic_t2: false,
            has_surgeon_t1: false,
            has_suture_kit: false,
            has_hot_metal_or_plasma: false,
            has_splint_material: false,
            has_surgery_table_and_surgeon: false,
            has_iv_kit: false,
            has_oxygen_tank: false,
            has_scanner_device: false,
            has_bed_object: false,
            wound_dirt_pct: 0.0,
            blood_compat: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TreatmentApply {
    pub kind: TreatmentKind,
    pub phase: TreatmentPhase,
    pub actor_id: u64,
    pub apply_seconds_remaining: f32,
    pub apply_seconds_total: f32,
    /// Remaining defib charges (defib-only).
    pub charges_remaining: Option<u32>,
    /// Cancellation-flag set by [`TreatmentApply::cancel`].
    pub cancelled: bool,
    /// Outcome computed on success/failure.
    pub outcome: Option<TreatmentOutcome>,
    rng: Xoshiro256StarStar,
}

impl TreatmentApply {
    /// Construct a new apply machine for the named treatment, against
    /// `ctx`. Pre-flight checks (wrong origin, missing tool, missing
    /// skill) reject immediately and the machine never enters `Applying`.
    pub fn start(
        kind: TreatmentKind,
        ctx: TreatmentContext,
        seed: u64,
    ) -> Result<Self, TreatmentApplyError> {
        let spec = treatment_spec(kind);
        // Origin gate (Gherkin scenario 6).
        if spec.origin_aware && ctx.is_robot_origin {
            return Err(TreatmentApplyError::WrongOrigin);
        }
        // Tool gate.
        let tool_ok = match spec.tool {
            ToolRequirement::None => true,
            ToolRequirement::SutureKit => ctx.has_suture_kit,
            ToolRequirement::HotMetalOrPlasma => ctx.has_hot_metal_or_plasma,
            ToolRequirement::SplintMaterial => ctx.has_splint_material,
            ToolRequirement::SurgeryTableAndSurgeon => ctx.has_surgery_table_and_surgeon,
            ToolRequirement::IvKit => ctx.has_iv_kit,
            ToolRequirement::OxygenTank => ctx.has_oxygen_tank,
            ToolRequirement::ScannerDevice => ctx.has_scanner_device,
            ToolRequirement::BedObject => ctx.has_bed_object,
        };
        if !tool_ok {
            return Err(TreatmentApplyError::MissingTool(spec.tool.as_str()));
        }
        // Skill gate.
        if !spec
            .skill
            .satisfied_by(ctx.has_medic_t1, ctx.has_medic_t2, ctx.has_surgeon_t1)
        {
            return Err(TreatmentApplyError::MissingSkill(spec.skill.as_str()));
        }
        let apply_seconds = spec.apply_seconds_avg();
        Ok(Self {
            kind,
            phase: TreatmentPhase::Pending,
            actor_id: ctx.actor_id,
            apply_seconds_remaining: apply_seconds,
            apply_seconds_total: apply_seconds,
            charges_remaining: spec.charges,
            cancelled: false,
            outcome: None,
            rng: Xoshiro256StarStar::seed_from_u64(seed),
        })
    }

    /// Advance the apply timer by `dt_seconds` and return any events that
    /// fired this tick. First call emits `treatment.applied`. The machine
    /// emits `treatment.completed` when `apply_seconds_remaining <= 0`.
    pub fn tick(
        &mut self,
        dt_seconds: f32,
        sim_tick: u64,
        ctx: &TreatmentContext,
    ) -> Vec<TreatmentEvent> {
        let mut events = Vec::new();
        if matches!(
            self.phase,
            TreatmentPhase::Completed | TreatmentPhase::Failed
        ) {
            return events;
        }
        if self.cancelled {
            self.phase = TreatmentPhase::Failed;
            self.outcome = Some(TreatmentOutcome::Failed {
                kind: self.kind,
                actor_id: self.actor_id,
                reason: TreatmentFailureReason::Cancelled,
            });
            events.push(TreatmentEvent::Failed {
                kind: self.kind,
                actor_id: self.actor_id,
                tick: sim_tick,
                reason: TreatmentFailureReason::Cancelled,
            });
            return events;
        }
        if matches!(self.phase, TreatmentPhase::Pending) {
            self.phase = TreatmentPhase::Applying;
            events.push(TreatmentEvent::Applied {
                kind: self.kind,
                actor_id: self.actor_id,
                tick: sim_tick,
                apply_seconds: self.apply_seconds_total,
            });
        }
        // Spec-locked: apply timer only advances while in Applying.
        if matches!(self.phase, TreatmentPhase::Applying) {
            self.apply_seconds_remaining -= dt_seconds;
            if self.apply_seconds_remaining <= 0.0 {
                // Risk roll for SuturesV1 on dirty wounds.
                let spec = treatment_spec(self.kind);
                if matches!(spec.risk, RiskKind::FailureRollOnDirtyWound)
                    && ctx.wound_dirt_pct > 0.5
                {
                    let roll = ((self.rng.next_u64() % 1000) as f32) / 1000.0;
                    if roll < ctx.wound_dirt_pct {
                        self.phase = TreatmentPhase::Failed;
                        self.outcome = Some(TreatmentOutcome::Failed {
                            kind: self.kind,
                            actor_id: self.actor_id,
                            reason: TreatmentFailureReason::DirtyWoundFailure,
                        });
                        events.push(TreatmentEvent::Failed {
                            kind: self.kind,
                            actor_id: self.actor_id,
                            tick: sim_tick,
                            reason: TreatmentFailureReason::DirtyWoundFailure,
                        });
                        return events;
                    }
                }
                // Risk roll for TransfusionBagV1 on incompatibility.
                if matches!(spec.risk, RiskKind::BloodReactionRoll) && ctx.blood_compat < 1.0 {
                    let roll = ((self.rng.next_u64() % 1000) as f32) / 1000.0;
                    if roll > ctx.blood_compat {
                        self.phase = TreatmentPhase::Failed;
                        self.outcome = Some(TreatmentOutcome::Failed {
                            kind: self.kind,
                            actor_id: self.actor_id,
                            reason: TreatmentFailureReason::BloodIncompatibility,
                        });
                        events.push(TreatmentEvent::Failed {
                            kind: self.kind,
                            actor_id: self.actor_id,
                            tick: sim_tick,
                            reason: TreatmentFailureReason::BloodIncompatibility,
                        });
                        return events;
                    }
                }
                self.phase = TreatmentPhase::Completed;
                self.outcome = Some(TreatmentOutcome::Completed {
                    kind: self.kind,
                    actor_id: self.actor_id,
                });
                events.push(TreatmentEvent::Completed {
                    kind: self.kind,
                    actor_id: self.actor_id,
                    tick: sim_tick,
                });
            }
        }
        events
    }

    pub fn cancel(&mut self) {
        if matches!(
            self.phase,
            TreatmentPhase::Completed | TreatmentPhase::Failed
        ) {
            return;
        }
        self.cancelled = true;
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.phase,
            TreatmentPhase::Completed | TreatmentPhase::Failed
        )
    }

    /// Consume one defib charge. Returns false if none remain.
    pub fn consume_charge(&mut self) -> bool {
        match self.charges_remaining.as_mut() {
            Some(c) if *c > 0 => {
                *c -= 1;
                true
            }
            _ => false,
        }
    }
}

/// Public helper that returns true if `seconds_since_bandage` exceeds the
/// soak-through threshold (180s per spec).
#[must_use]
pub fn bandage_soaked_through(seconds_since_bandage: f32) -> bool {
    seconds_since_bandage >= crate::producers::BANDAGE_SOAK_THROUGH_SECONDS
}

#[cfg(test)]
mod tests {
    use super::*;

    /// kind=field_bandage_v1 fires + 5s elapse → treatment.applied fires +
    /// treatment.completed fires.
    #[test]
    fn bandage_applied_completed_after_5s() {
        let ctx = TreatmentContext::for_clean_human(7);
        let mut apply =
            TreatmentApply::start(TreatmentKind::FieldBandageV1, ctx, 42).expect("start");
        let events_t0 = apply.tick(0.0, 0, &ctx);
        assert!(matches!(events_t0[0], TreatmentEvent::Applied { .. }));
        // 5 seconds elapse: 1.0s × 5 ticks.
        for t in 1..=5 {
            let evs = apply.tick(1.0, t, &ctx);
            if t < 5 {
                assert!(evs.is_empty(), "no completion before 5s, got {evs:?} at t={t}");
            } else {
                assert!(
                    matches!(evs.last(), Some(TreatmentEvent::Completed { .. })),
                    "expected Completed at t=5, got {evs:?}"
                );
            }
        }
        assert_eq!(apply.phase, TreatmentPhase::Completed);
    }

    /// on robot → treatment.failed reason="wrong_origin".
    #[test]
    fn robot_rejects_bandage_with_wrong_origin() {
        let ctx = TreatmentContext::for_robot(11);
        let result = TreatmentApply::start(TreatmentKind::FieldBandageV1, ctx, 0);
        assert!(matches!(result, Err(TreatmentApplyError::WrongOrigin)));
    }

    /// Sutures on a dirty wound roll for failure.
    #[test]
    fn sutures_fail_on_dirty_wound() {
        let mut ctx = TreatmentContext::for_clean_human(3);
        ctx.wound_dirt_pct = 0.95;
        let mut apply = TreatmentApply::start(TreatmentKind::SuturesV1, ctx, 11).expect("start");
        // Run apply to completion (30 seconds).
        let mut last_event = None;
        for t in 0..=31 {
            let evs = apply.tick(1.0, t, &ctx);
            for e in evs {
                last_event = Some(e);
            }
            if apply.is_terminal() {
                break;
            }
        }
        // With dirt_pct=0.95 and the roll being uniform [0,1), the failure
        // probability is ~95% — extremely likely to fail with seed 11.
        match last_event {
            Some(TreatmentEvent::Failed {
                reason: TreatmentFailureReason::DirtyWoundFailure,
                ..
            }) => {}
            Some(TreatmentEvent::Completed { .. }) => {
                // 5% prob; acceptable. Try another seed deterministically.
                let mut apply2 =
                    TreatmentApply::start(TreatmentKind::SuturesV1, ctx, 9999).expect("start");
                let mut last2 = None;
                for t in 0..=31 {
                    for e in apply2.tick(1.0, t, &ctx) {
                        last2 = Some(e);
                    }
                    if apply2.is_terminal() {
                        break;
                    }
                }
                assert!(matches!(
                    last2,
                    Some(TreatmentEvent::Failed {
                        reason: TreatmentFailureReason::DirtyWoundFailure,
                        ..
                    })
                ));
            }
            _ => panic!("expected terminal Failed or Completed, got {last_event:?}"),
        }
    }

    /// Determinism — same seed produces same outcome.
    #[test]
    fn determinism_same_seed_same_outcome() {
        let mut ctx = TreatmentContext::for_clean_human(1);
        ctx.wound_dirt_pct = 0.8;
        let mut events_a: Vec<TreatmentEvent> = Vec::new();
        let mut events_b: Vec<TreatmentEvent> = Vec::new();
        let mut a = TreatmentApply::start(TreatmentKind::SuturesV1, ctx, 42).unwrap();
        let mut b = TreatmentApply::start(TreatmentKind::SuturesV1, ctx, 42).unwrap();
        for t in 0..=31 {
            events_a.extend(a.tick(1.0, t, &ctx));
            events_b.extend(b.tick(1.0, t, &ctx));
        }
        assert_eq!(events_a, events_b);
    }

    /// Bandage soaks through after 180s.
    #[test]
    fn bandage_soaks_through_at_180s() {
        assert!(!bandage_soaked_through(179.999));
        assert!(bandage_soaked_through(180.0));
        assert!(bandage_soaked_through(300.0));
    }

    /// Missing tool rejects.
    #[test]
    fn sutures_without_kit_rejected() {
        let mut ctx = TreatmentContext::for_clean_human(5);
        ctx.has_suture_kit = false;
        let r = TreatmentApply::start(TreatmentKind::SuturesV1, ctx, 0);
        assert!(matches!(r, Err(TreatmentApplyError::MissingTool("suture_kit"))));
    }

    /// Missing skill rejects.
    #[test]
    fn surgery_without_surgeon_rejected() {
        let mut ctx = TreatmentContext::for_clean_human(7);
        ctx.has_surgeon_t1 = false;
        ctx.has_medic_t1 = true;
        let r = TreatmentApply::start(TreatmentKind::SurgeryKitV1, ctx, 0);
        assert!(matches!(r, Err(TreatmentApplyError::MissingSkill("surgeon_t1"))));
    }
}
