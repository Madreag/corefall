//! M17 — helmet + oxygen-tank + vacuum mechanics.
//!
//! Oxygen-supply (seconds) drains while an oxygen-requiring origin is exposed
//! to vacuum / low-O2. A penetrating round to a sealed helmet triggers a
//! `helmet_breach` that drains O2 at 3×. When the supply hits zero the actor
//! suffocates (hypoxia stacks + HP drain). Robots / vacuum-immune origins skip
//! the whole path.

use serde::{Deserialize, Serialize};

/// Oxygen-drain multiplier applied while a helmet is breached (spec § "helmet
/// breach → oxygen drains at 3× normal rate").
pub const HELMET_BREACH_DRAIN_MULTIPLIER: f32 = 3.0;

/// HP lost per second once oxygen supply is exhausted in vacuum
/// (spec § "actor.hp drains at 2/s").
pub const OXYGEN_EMPTY_HP_DRAIN_PER_S: f32 = 2.0;

/// Baseline O2 consumption (seconds of reserve burned per real second) at
/// rest. Combat / running scale this up via `consumption_modifier`.
pub const BASE_O2_CONSUMPTION_PER_S: f32 = 1.0;

/// Oxygen-tank tiers (spec § "4 oxygen tank tiers").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OxygenTankTier {
    /// T0 — manual bottle, ~5 min.
    ManualBottle,
    /// T1 — compressed tank, ~60 min.
    Compressed,
    /// T2 — cryogenic liquid O2, ~5 hours.
    Cryogenic,
    /// T3 — closed-loop gas cycler with CO2 scrubber (effectively unlimited).
    GasCycler,
}

impl OxygenTankTier {
    pub fn as_str(self) -> &'static str {
        match self {
            OxygenTankTier::ManualBottle => "manual_bottle",
            OxygenTankTier::Compressed => "compressed",
            OxygenTankTier::Cryogenic => "cryogenic",
            OxygenTankTier::GasCycler => "gas_cycler",
        }
    }

    /// Capacity in seconds of breathing reserve at moderate activity.
    pub fn capacity_seconds(self) -> f32 {
        match self {
            OxygenTankTier::ManualBottle => 5.0 * 60.0,
            OxygenTankTier::Compressed => 60.0 * 60.0,
            OxygenTankTier::Cryogenic => 5.0 * 3600.0,
            // Closed-loop: a very large reserve (refills from base atmospherics).
            OxygenTankTier::GasCycler => 100.0 * 3600.0,
        }
    }

    pub fn weight_kg(self) -> f32 {
        match self {
            OxygenTankTier::ManualBottle => 3.0,
            OxygenTankTier::Compressed => 8.0,
            OxygenTankTier::Cryogenic => 25.0,
            OxygenTankTier::GasCycler => 35.0,
        }
    }
}

/// A worn oxygen tank (M6 inventory slot). Robots reject the slot.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OxygenTank {
    pub tier: OxygenTankTier,
    pub current_seconds: f32,
    pub seal_integrity_pct: f32,
    pub filter_active: bool,
}

impl OxygenTank {
    pub fn full(tier: OxygenTankTier) -> Self {
        Self {
            tier,
            current_seconds: tier.capacity_seconds(),
            seal_integrity_pct: 100.0,
            filter_active: matches!(tier, OxygenTankTier::GasCycler),
        }
    }

    pub fn refill(&mut self) {
        self.current_seconds = self.tier.capacity_seconds();
        self.seal_integrity_pct = 100.0;
    }
}

/// Inputs to one tick of the vacuum / oxygen pass.
#[derive(Debug, Clone, Copy)]
pub struct OxygenTickInput {
    /// This origin breathes and is not vacuum-immune.
    pub oxygen_required: bool,
    /// Sealed helmet present (a breach drains 3×; absence means raw vacuum).
    pub helmet_sealed: bool,
    /// Helmet currently breached (penetrating round hit the sealed helmet).
    pub helmet_breached: bool,
    /// Ambient pressure is at / near vacuum (no breathable atmosphere).
    pub vacuum_exposed: bool,
    /// Activity multiplier on consumption (rest 1.0, combat/run > 1.0).
    pub consumption_modifier: f32,
    pub dt_seconds: f32,
}

/// Result of one oxygen tick.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct OxygenTickResult {
    pub from_seconds: f32,
    pub to_seconds: f32,
    pub drained: f32,
    /// True the tick the supply transitions to empty.
    pub just_emptied: bool,
    /// HP that should be drained this tick (oxygen-empty suffocation).
    pub hp_drain: f32,
    /// Effective per-second drain rate this tick (for the drain_rate event).
    pub drain_rate_per_s: f32,
}

/// Advance the actor's oxygen reserve one tick. Pure. Returns the supply
/// delta + any suffocation HP drain. No-op for origins that don't breathe or
/// are not vacuum-exposed (returns the unchanged supply).
pub fn tick_oxygen(current_seconds: f32, input: OxygenTickInput) -> OxygenTickResult {
    let mut out = OxygenTickResult {
        from_seconds: current_seconds,
        to_seconds: current_seconds,
        drain_rate_per_s: 0.0,
        ..OxygenTickResult::default()
    };
    if !input.oxygen_required || !input.vacuum_exposed || input.dt_seconds <= 0.0 {
        return out;
    }
    // With no sealed helmet at all, the actor breathes vacuum directly: the
    // reserve is irrelevant — suffocation begins immediately (handled by the
    // hypoxia path), but we still drain any in-lung reserve fast.
    let breach_mult = if input.helmet_breached || !input.helmet_sealed {
        HELMET_BREACH_DRAIN_MULTIPLIER
    } else {
        1.0
    };
    let rate = BASE_O2_CONSUMPTION_PER_S * input.consumption_modifier.max(0.1) * breach_mult;
    out.drain_rate_per_s = rate;
    let drained = (rate * input.dt_seconds).min(current_seconds.max(0.0));
    out.drained = drained;
    out.to_seconds = (current_seconds - drained).max(0.0);
    out.just_emptied = current_seconds > 0.0 && out.to_seconds <= 0.0;
    if out.to_seconds <= 0.0 {
        out.hp_drain = OXYGEN_EMPTY_HP_DRAIN_PER_S * input.dt_seconds;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_drain_when_origin_does_not_breathe() {
        let r = tick_oxygen(
            100.0,
            OxygenTickInput {
                oxygen_required: false,
                helmet_sealed: false,
                helmet_breached: false,
                vacuum_exposed: true,
                consumption_modifier: 1.0,
                dt_seconds: 1.0,
            },
        );
        assert_eq!(r.to_seconds, 100.0);
        assert_eq!(r.hp_drain, 0.0);
    }

    #[test]
    fn sealed_helmet_drains_at_base_rate() {
        let r = tick_oxygen(
            100.0,
            OxygenTickInput {
                oxygen_required: true,
                helmet_sealed: true,
                helmet_breached: false,
                vacuum_exposed: true,
                consumption_modifier: 1.0,
                dt_seconds: 1.0,
            },
        );
        assert!((r.drained - 1.0).abs() < 1e-6);
        assert!((r.drain_rate_per_s - 1.0).abs() < 1e-6);
    }

    #[test]
    fn breached_helmet_drains_three_times_faster() {
        let r = tick_oxygen(
            100.0,
            OxygenTickInput {
                oxygen_required: true,
                helmet_sealed: true,
                helmet_breached: true,
                vacuum_exposed: true,
                consumption_modifier: 1.0,
                dt_seconds: 1.0,
            },
        );
        assert!((r.drained - 3.0).abs() < 1e-6, "3x drain on breach");
    }

    #[test]
    fn empty_supply_drains_hp_at_2_per_second() {
        let r = tick_oxygen(
            0.5,
            OxygenTickInput {
                oxygen_required: true,
                helmet_sealed: true,
                helmet_breached: false,
                vacuum_exposed: true,
                consumption_modifier: 1.0,
                dt_seconds: 1.0,
            },
        );
        assert_eq!(r.to_seconds, 0.0);
        assert!(r.just_emptied);
        assert!((r.hp_drain - OXYGEN_EMPTY_HP_DRAIN_PER_S).abs() < 1e-6);
    }

    #[test]
    fn tank_tiers_have_increasing_capacity() {
        assert!(
            OxygenTankTier::ManualBottle.capacity_seconds()
                < OxygenTankTier::Compressed.capacity_seconds()
        );
        assert!(
            OxygenTankTier::Compressed.capacity_seconds()
                < OxygenTankTier::Cryogenic.capacity_seconds()
        );
    }
}
