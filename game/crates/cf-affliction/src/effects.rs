//! effects.rs — affliction → combat/movement effect aggregators.
//!
//! Closes the long-standing M5/M16 gap where the affliction table promised aim
//! and move-speed penalties (`specs/done/M16.md`: concussed "−aim, recoil
//! bloom", hyperthermic "reduced aim", thirst "aim wobble … speed × 0.85",
//! blinded "Affects aim_accuracy", plus M16C Pain "aim wobble + move speed ×
//! (1 − pct × 0.3)") but nothing consumed them.
//!
//! Two aggregators turn an actor's active afflictions into engine inputs:
//!   - [`affliction_aim_spread_bonus_radians`] — additive aim-cone bonus
//!     (radians), consumed by the weapon-fire spread exactly like the existing
//!     mount-motion spread bonus.
//!   - [`affliction_move_speed_multiplier`] — multiplicative walk-speed factor
//!     (≤ 1.0), consumed in the actor sim's `effective_max_speed`.
//!
//! Both are **identity** (0.0 / 1.0) for an unafflicted actor, so existing
//! combat/movement behaviour is byte-for-byte unchanged when no affliction is
//! present — only afflicted actors are affected.

use crate::{ActorAfflictions, M16AfflictionKind};

/// Maximum aggregate aim-spread bonus (radians ≈ 34°) — keeps even a fully
/// stacked actor able to roughly point a weapon.
pub const MAX_AIM_SPREAD_BONUS_RAD: f32 = 0.60;

/// Minimum aggregate move-speed multiplier — an actor never drops below 20%
/// of base walk speed from afflictions alone.
pub const MIN_MOVE_SPEED_MULTIPLIER: f32 = 0.20;

/// Per-kind additive aim-spread bonus (radians) at full severity, scaled
/// linearly by severity. Derived from the M16 affliction table.
pub fn aim_spread_bonus_radians_for(kind: M16AfflictionKind, severity: f32) -> f32 {
    let per_full = match kind {
        // "Affects aim_accuracy" — a blinded actor can barely aim.
        M16AfflictionKind::Blinded => 0.45,
        // "−aim, recoil bloom 3s".
        M16AfflictionKind::Concussed => 0.20,
        // M16C Pain "aim wobble" (the wobble multiplier's spread analogue).
        M16AfflictionKind::Pain => 0.12,
        // Disorientation from electric shock / near-drowning.
        M16AfflictionKind::Shocked | M16AfflictionKind::Drowning => 0.10,
        // "aim wobble + mental fog".
        M16AfflictionKind::Thirst => 0.08,
        // "reduced aim".
        M16AfflictionKind::Hyperthermic => 0.06,
        _ => 0.0,
    };
    per_full * severity.clamp(0.0, 1.0)
}

/// Per-kind move-speed multiplier (≤ 1.0) at the given severity. Derived from
/// the M16 affliction table.
pub fn move_speed_multiplier_for(kind: M16AfflictionKind, severity: f32) -> f32 {
    let s = severity.clamp(0.0, 1.0);
    let loss = match kind {
        // M16C Pain "move speed × (1 − pct × 0.3)".
        M16AfflictionKind::Pain => 0.30,
        // Frozen — heavy slow.
        M16AfflictionKind::Frozen => 0.50,
        // Near-drowning thrash.
        M16AfflictionKind::Drowning => 0.40,
        // Cold stiffens limbs.
        M16AfflictionKind::Hypothermic => 0.30,
        // "speed × 0.85" at full thirst → 0.15 loss.
        M16AfflictionKind::Thirst => 0.15,
        // Heat exhaustion.
        M16AfflictionKind::Hyperthermic => 0.20,
        // Blood loss weakness.
        M16AfflictionKind::Bleeding => 0.10,
        _ => 0.0,
    };
    1.0 - loss * s
}

/// Aggregate additive aim-spread bonus (radians) across all active afflictions
/// (sum, clamped to [`MAX_AIM_SPREAD_BONUS_RAD`]). `0.0` for an unafflicted
/// actor.
pub fn affliction_aim_spread_bonus_radians(afflictions: &ActorAfflictions) -> f32 {
    let total: f32 = afflictions
        .active
        .iter()
        .map(|a| aim_spread_bonus_radians_for(a.kind, a.severity))
        .sum();
    total.clamp(0.0, MAX_AIM_SPREAD_BONUS_RAD)
}

/// Aggregate move-speed multiplier across all active afflictions (product,
/// clamped to [`MIN_MOVE_SPEED_MULTIPLIER`]). `1.0` for an unafflicted actor.
pub fn affliction_move_speed_multiplier(afflictions: &ActorAfflictions) -> f32 {
    let product: f32 = afflictions
        .active
        .iter()
        .map(|a| move_speed_multiplier_for(a.kind, a.severity))
        .product();
    product.clamp(MIN_MOVE_SPEED_MULTIPLIER, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ActiveAffliction;

    fn afflicted(kind: M16AfflictionKind, severity: f32) -> ActorAfflictions {
        let mut a = ActorAfflictions::default();
        a.active.push(ActiveAffliction {
            kind,
            severity,
            applied_at_tick: 0,
            expected_clear_tick: None,
            source_event_id: None,
        });
        a
    }

    #[test]
    fn unafflicted_actor_is_identity() {
        let none = ActorAfflictions::default();
        assert_eq!(affliction_aim_spread_bonus_radians(&none), 0.0);
        assert_eq!(affliction_move_speed_multiplier(&none), 1.0);
    }

    #[test]
    fn pain_matches_spec_move_speed_curve() {
        // Pain "move speed × (1 − pct × 0.3)": at severity 0.6 → 0.82.
        let a = afflicted(M16AfflictionKind::Pain, 0.6);
        assert!((affliction_move_speed_multiplier(&a) - (1.0 - 0.6 * 0.3)).abs() < 1e-6);
        // Pain contributes a positive aim-spread bonus.
        assert!(affliction_aim_spread_bonus_radians(&a) > 0.0);
    }

    #[test]
    fn thirst_full_severity_is_speed_0_85() {
        let a = afflicted(M16AfflictionKind::Thirst, 1.0);
        assert!((affliction_move_speed_multiplier(&a) - 0.85).abs() < 1e-6);
    }

    #[test]
    fn blinded_has_the_largest_aim_penalty() {
        let blind = affliction_aim_spread_bonus_radians(&afflicted(M16AfflictionKind::Blinded, 1.0));
        let pain = affliction_aim_spread_bonus_radians(&afflicted(M16AfflictionKind::Pain, 1.0));
        assert!(blind > pain);
    }

    #[test]
    fn stacked_afflictions_aggregate_and_clamp() {
        let mut a = afflicted(M16AfflictionKind::Pain, 1.0);
        a.active.push(ActiveAffliction {
            kind: M16AfflictionKind::Concussed,
            severity: 1.0,
            applied_at_tick: 0,
            expected_clear_tick: None,
            source_event_id: None,
        });
        // Aim spread sums (0.12 + 0.20) and stays under the cap.
        let spread = affliction_aim_spread_bonus_radians(&a);
        assert!((spread - 0.32).abs() < 1e-6);
        assert!(spread <= MAX_AIM_SPREAD_BONUS_RAD);
        // Move speed: pain only (concussed has no move penalty) → 0.70.
        assert!((affliction_move_speed_multiplier(&a) - 0.70).abs() < 1e-6);
    }

    #[test]
    fn move_speed_never_below_floor() {
        // Stack heavy slows to confirm the floor clamp.
        let mut a = afflicted(M16AfflictionKind::Frozen, 1.0);
        for k in [M16AfflictionKind::Pain, M16AfflictionKind::Drowning, M16AfflictionKind::Hypothermic] {
            a.active.push(ActiveAffliction {
                kind: k,
                severity: 1.0,
                applied_at_tick: 0,
                expected_clear_tick: None,
                source_event_id: None,
            });
        }
        assert!(affliction_move_speed_multiplier(&a) >= MIN_MOVE_SPEED_MULTIPLIER);
    }
}
