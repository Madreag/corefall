//! effects.rs — the mental-health condition → combat/movement consumer.
//!
//! The M16C condition table (spec § Player-facing behavior) promises per-
//! condition combat symptoms that the lifecycle FSM tracks but, until this
//! module, nothing applied to the actor:
//!   - **Withdrawal** "aim-shake (2× wobble)" + "HP drain on tick",
//!   - **Anxiety Disorder** "−10% accuracy",
//!   - **Addiction** "bypass-cost reduces accuracy",
//!   - **Depression** "move speed × 0.85",
//!   - **PanicDisorder / PTSD / AcuteStressReaction** panic-attack "freeze 3–8s".
//!
//! This module mirrors [`cf-affliction::effects`]: each *symptomatic* condition
//! contributes to an additive aim-spread bonus (radians) and/or a multiplicative
//! move-speed factor, which the engine folds into the same `ActorState`
//! affliction fields the actor sim already consumes (`effective_max_speed` +
//! weapon-fire spread cone). The panic-freeze projection drives
//! `ActorState::panic_freeze_ticks_remaining` → `Stance::PanickedFreeze` +
//! input/fire lock, and the withdrawal HP drain is applied per tick (floored —
//! acute withdrawal is debilitating, never directly lethal: spec "Resolves over
//! 2 weeks").
//!
//! Every aggregator is **identity** (0.0 / 1.0 / 0) for an actor with no
//! symptomatic condition, so an unafflicted actor is byte-for-byte unchanged —
//! only symptomatic actors are affected.

use crate::conditions::{ActorMentalHealth, ConditionKind};
use crate::WITHDRAWAL_AIM_WOBBLE_MULTIPLIER;

/// Maximum aggregate *condition* aim-spread bonus (radians ≈ 23°). The engine
/// bridge re-clamps the combined affliction + condition spread.
pub const MAX_CONDITION_AIM_SPREAD_RAD: f32 = 0.40;

/// Minimum aggregate *condition* move-speed multiplier (a condition alone never
/// drops the actor below 30% of base walk speed).
pub const MIN_CONDITION_MOVE_MULTIPLIER: f32 = 0.30;

/// Radians of additive aim spread per `1.0` of "aim-wobble multiplier above 1×".
/// Withdrawal's spec "2× wobble" → `(2.0 − 1.0) × 0.15 = 0.15` rad — a severe
/// shake, sitting between Pain (0.12) and Concussed (0.20) on the affliction
/// aim scale.
pub const WOBBLE_TO_SPREAD_RAD: f32 = 0.15;

/// Anxiety Disorder "−10% accuracy" expressed as additive aim spread (mild).
pub const ANXIETY_AIM_SPREAD_RAD: f32 = 0.05;
/// Addiction "bypass-cost reduces accuracy" as additive aim spread (mild) — the
/// attention cost of suppressing the craving.
pub const ADDICTION_AIM_SPREAD_RAD: f32 = 0.04;
/// PTSD intrusive-flashback aim degradation between the discrete panic freezes
/// (the "flashback aim-freeze" symptom's continuous component).
pub const PTSD_AIM_SPREAD_RAD: f32 = 0.05;

/// Depression "move speed × 0.85".
pub const DEPRESSION_MOVE_MULTIPLIER: f32 = 0.85;

/// Per-tick HP drain while Withdrawal is symptomatic (the physical toll of acute
/// withdrawal). Small + tunable; the engine applies it only above the floor.
pub const WITHDRAWAL_HP_DRAIN_PER_TICK: f32 = 0.0010;
/// Withdrawal HP drain stops at this fraction of max HP — withdrawal is
/// debilitating but never directly lethal (spec "Resolves over 2 weeks").
pub const WITHDRAWAL_HP_DRAIN_FLOOR_FRAC: f32 = 0.25;

/// Additive aim-spread (radians) contributed by one symptomatic condition.
/// Withdrawal derives from the `2×` wobble multiplier so the spec literal and
/// the sim value stay tied; the others are direct per-condition magnitudes.
fn aim_spread_for(kind: ConditionKind) -> f32 {
    match kind {
        ConditionKind::Withdrawal => (WITHDRAWAL_AIM_WOBBLE_MULTIPLIER - 1.0) * WOBBLE_TO_SPREAD_RAD,
        ConditionKind::AnxietyDisorder => ANXIETY_AIM_SPREAD_RAD,
        ConditionKind::Addiction => ADDICTION_AIM_SPREAD_RAD,
        ConditionKind::Ptsd => PTSD_AIM_SPREAD_RAD,
        _ => 0.0,
    }
}

/// Multiplicative move-speed factor (≤ 1.0) from one symptomatic condition.
fn move_multiplier_for(kind: ConditionKind) -> f32 {
    match kind {
        ConditionKind::Depression => DEPRESSION_MOVE_MULTIPLIER,
        _ => 1.0,
    }
}

/// Aggregate additive aim-spread bonus (radians) across every symptomatic
/// condition (sum, clamped to [`MAX_CONDITION_AIM_SPREAD_RAD`]). `0.0` when no
/// condition is symptomatic.
pub fn condition_aim_spread_bonus_radians(mh: &ActorMentalHealth) -> f32 {
    let total: f32 = mh
        .active
        .iter()
        .filter(|c| c.stage.is_symptomatic())
        .map(|c| aim_spread_for(c.kind))
        .sum();
    total.clamp(0.0, MAX_CONDITION_AIM_SPREAD_RAD)
}

/// Aggregate move-speed multiplier across every symptomatic condition (product,
/// clamped to [`MIN_CONDITION_MOVE_MULTIPLIER`]). `1.0` when no condition is
/// symptomatic.
pub fn condition_move_speed_multiplier(mh: &ActorMentalHealth) -> f32 {
    let product: f32 = mh
        .active
        .iter()
        .filter(|c| c.stage.is_symptomatic())
        .map(|c| move_multiplier_for(c.kind))
        .product();
    product.clamp(MIN_CONDITION_MOVE_MULTIPLIER, 1.0)
}

/// Per-tick HP drain from a symptomatic Withdrawal (0.0 otherwise). The engine
/// applies it only while HP is above [`WITHDRAWAL_HP_DRAIN_FLOOR_FRAC`] × max HP
/// so withdrawal never directly kills.
pub fn condition_hp_drain_per_tick(mh: &ActorMentalHealth) -> f32 {
    if mh.is_symptomatic(ConditionKind::Withdrawal) {
        WITHDRAWAL_HP_DRAIN_PER_TICK
    } else {
        0.0
    }
}

/// Remaining panic-freeze ticks at `tick` — the max over active conditions of
/// `panic_frozen_until_tick − tick`. `0` when no condition currently freezes the
/// actor. Drives `ActorState::panic_freeze_ticks_remaining` →
/// `Stance::PanickedFreeze` (acceptance scenario 5).
pub fn condition_panic_freeze_ticks_remaining(mh: &ActorMentalHealth, tick: u64) -> u32 {
    let remaining = mh
        .active
        .iter()
        .map(|c| c.panic_frozen_until_tick.saturating_sub(tick))
        .max()
        .unwrap_or(0);
    u32::try_from(remaining).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conditions::TriggerReason;
    use crate::OriginId;

    fn with(kind: ConditionKind) -> ActorMentalHealth {
        let mut mh = ActorMentalHealth::with_origin(OriginId::Human);
        mh.trigger(7, kind, TriggerReason::SustainedStress, 0);
        mh
    }

    #[test]
    fn no_condition_is_identity() {
        let mh = ActorMentalHealth::with_origin(OriginId::Human);
        assert_eq!(condition_aim_spread_bonus_radians(&mh), 0.0);
        assert_eq!(condition_move_speed_multiplier(&mh), 1.0);
        assert_eq!(condition_hp_drain_per_tick(&mh), 0.0);
        assert_eq!(condition_panic_freeze_ticks_remaining(&mh, 0), 0);
    }

    #[test]
    fn withdrawal_shake_is_severe_and_ties_to_2x_wobble() {
        let mh = with(ConditionKind::Withdrawal);
        // (2.0 − 1.0) × 0.15 = 0.15 rad.
        assert!((condition_aim_spread_bonus_radians(&mh) - 0.15).abs() < 1e-6);
    }

    #[test]
    fn depression_slows_move_speed_to_0_85() {
        let mh = with(ConditionKind::Depression);
        assert!((condition_move_speed_multiplier(&mh) - 0.85).abs() < 1e-6);
    }

    #[test]
    fn anxiety_and_addiction_add_mild_spread() {
        assert!(condition_aim_spread_bonus_radians(&with(ConditionKind::AnxietyDisorder)) > 0.0);
        assert!(condition_aim_spread_bonus_radians(&with(ConditionKind::Addiction)) > 0.0);
        // Anxiety's accuracy hit is larger than addiction's bypass-cost.
        assert!(
            condition_aim_spread_bonus_radians(&with(ConditionKind::AnxietyDisorder))
                > condition_aim_spread_bonus_radians(&with(ConditionKind::Addiction))
        );
    }

    #[test]
    fn withdrawal_drains_hp_only_while_symptomatic() {
        let mut mh = with(ConditionKind::Withdrawal);
        assert!(condition_hp_drain_per_tick(&mh) > 0.0);
        // Force the withdrawal into Remission → drain stops.
        mh.find_mut(ConditionKind::Withdrawal).unwrap().stage =
            crate::conditions::ConditionStage::Remission;
        assert_eq!(condition_hp_drain_per_tick(&mh), 0.0);
    }

    #[test]
    fn panic_freeze_projects_remaining_ticks() {
        let mut mh = with(ConditionKind::PanicDisorder);
        mh.find_mut(ConditionKind::PanicDisorder).unwrap().panic_frozen_until_tick = 500;
        assert_eq!(condition_panic_freeze_ticks_remaining(&mh, 200), 300);
        // Past the freeze window → 0.
        assert_eq!(condition_panic_freeze_ticks_remaining(&mh, 600), 0);
    }

    #[test]
    fn remission_clears_all_symptoms() {
        let mut mh = with(ConditionKind::Depression);
        mh.find_mut(ConditionKind::Depression).unwrap().stage =
            crate::conditions::ConditionStage::Remission;
        // Remission is not symptomatic → identity move speed.
        assert_eq!(condition_move_speed_multiplier(&mh), 1.0);
    }

    #[test]
    fn stacked_conditions_aggregate_and_clamp() {
        let mut mh = with(ConditionKind::Withdrawal);
        mh.trigger(7, ConditionKind::AnxietyDisorder, TriggerReason::SustainedStress, 0);
        mh.trigger(7, ConditionKind::Ptsd, TriggerReason::WitnessDeaths, 0);
        // 0.15 + 0.05 + 0.05 = 0.25, under the cap.
        let spread = condition_aim_spread_bonus_radians(&mh);
        assert!((spread - 0.25).abs() < 1e-6);
        assert!(spread <= MAX_CONDITION_AIM_SPREAD_RAD);
    }
}
