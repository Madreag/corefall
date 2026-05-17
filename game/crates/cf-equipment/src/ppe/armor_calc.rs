//! M6C-2: body armor damage reduction calculus.
//!
//! Gherkin scenario M6C-2:
//! ```text
//! Scenario: M6C-2 Body armor slot separate from chassis armor
//!   Given infantry actor wearing armor_kevlar_light
//!   When hit by rifle round (kinetic 50)
//!   Then damage reduced by 20%
//!   And body_armor.degraded fires on durability tick
//! ```
//!
//! The reduction is applied as `effective_damage = raw * (1 - reduction)`.
//! Durability decreases by the absorbed damage; when durability crosses
//! the `DEGRADED_THRESHOLD_FRACTION` boundary the engine emits
//! `body_armor.degraded`.

use serde::{Deserialize, Serialize};

/// Durability fraction below which a `body_armor.degraded` event fires
/// (once per crossing).
pub const DEGRADED_THRESHOLD_FRACTION: f32 = 0.50;

/// Durability fraction below which the armor stops providing protection.
pub const ARMOR_FAILURE_FRACTION: f32 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct DamageReductionResult {
    /// Final damage that reaches the actor's HP pool.
    pub damage_after_reduction: f32,
    /// Damage absorbed by the armor layer.
    pub damage_absorbed_by_armor: f32,
    /// Armor durability after the hit.
    pub durability_after: f32,
    /// True when the durability crossed the degraded threshold this hit
    /// (used to emit `body_armor.degraded`).
    pub crossed_degraded_threshold: bool,
    /// True when the armor is now below the failure floor.
    pub failed: bool,
}

/// Apply one kinetic hit against a body-armor layer.
///
/// `raw_damage` is the inbound kinetic damage. `kinetic_reduction` is the
/// per-armor multiplier (0..1) — e.g. armor_kevlar_light = 0.20.
/// `durability_before` is the armor's current durability HP.
/// `durability_max` is the armor's max durability HP (drives the
/// degraded-threshold crossing).
pub fn apply_kinetic_hit(
    raw_damage: f32,
    kinetic_reduction: f32,
    durability_before: f32,
    durability_max: f32,
) -> DamageReductionResult {
    let max = durability_max.max(1e-6);
    let dur_before = durability_before.clamp(0.0, max);
    let fraction_before = dur_before / max;
    if fraction_before <= ARMOR_FAILURE_FRACTION {
        return DamageReductionResult {
            damage_after_reduction: raw_damage.max(0.0),
            damage_absorbed_by_armor: 0.0,
            durability_after: dur_before,
            crossed_degraded_threshold: false,
            failed: true,
        };
    }
    let reduction = kinetic_reduction.clamp(0.0, 1.0);
    let absorbed = raw_damage.max(0.0) * reduction;
    let leak = raw_damage.max(0.0) - absorbed;
    let dur_after = (dur_before - absorbed).max(0.0);
    let fraction_after = dur_after / max;
    let crossed = fraction_before > DEGRADED_THRESHOLD_FRACTION
        && fraction_after <= DEGRADED_THRESHOLD_FRACTION;
    let failed = fraction_after <= ARMOR_FAILURE_FRACTION;
    DamageReductionResult {
        damage_after_reduction: leak,
        damage_absorbed_by_armor: absorbed,
        durability_after: dur_after,
        crossed_degraded_threshold: crossed,
        failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kevlar_light_reduces_damage_by_twenty_percent() {
        // M6C-2 Scenario:
        //   Given infantry actor wearing armor_kevlar_light (kinetic reduction 0.20)
        //   When hit by rifle round (kinetic 50)
        //   Then damage reduced by 20%
        let r = apply_kinetic_hit(50.0, 0.20, 400.0, 400.0);
        // Effective damage to HP = 50 * (1 - 0.20) = 40.
        assert!((r.damage_after_reduction - 40.0).abs() < 1e-3);
        // Absorbed by armor: 10.
        assert!((r.damage_absorbed_by_armor - 10.0).abs() < 1e-3);
        // Durability decreased by 10.
        assert!((r.durability_after - 390.0).abs() < 1e-3);
    }

    #[test]
    fn degraded_threshold_fires_on_first_cross_below_fifty_percent() {
        // M6C-2 Scenario continued:
        //   And body_armor.degraded fires on durability tick
        // Start at 60%, take a hit that drops to 40%.
        let r = apply_kinetic_hit(100.0, 1.0, 240.0, 400.0);
        // All 100 absorbed; durability 240 → 140 (=35%).
        assert!(r.crossed_degraded_threshold);
        assert!((r.durability_after - 140.0).abs() < 1e-3);
    }

    #[test]
    fn no_degraded_crossing_when_starting_below_threshold() {
        // Already at 40%; further damage doesn't re-fire.
        let r = apply_kinetic_hit(50.0, 1.0, 160.0, 400.0);
        assert!(!r.crossed_degraded_threshold);
    }

    #[test]
    fn no_degraded_crossing_when_starting_above_and_staying_above() {
        let r = apply_kinetic_hit(20.0, 1.0, 400.0, 400.0);
        assert!(!r.crossed_degraded_threshold);
    }

    #[test]
    fn failed_armor_passes_full_damage() {
        let r = apply_kinetic_hit(50.0, 0.8, 10.0, 400.0);
        assert!(r.failed);
        assert!((r.damage_after_reduction - 50.0).abs() < 1e-3);
    }
}
