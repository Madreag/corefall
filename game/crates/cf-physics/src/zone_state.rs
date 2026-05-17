//! **M14**: per-zone limb-loss state machine.
//!
//! Per the spec § "Limb-loss state machine":
//! ```rust
//! pub enum ZoneState {
//!   Intact,      // full HP; functional
//!   Damaged,     // HP < threshold; partial function
//!   Critical,    // near severance; high bleed rate
//!   Severed,     // detached; visible at ground; functional consequence active
//!   Destroyed,   // gibbed beyond recovery (M13+)
//! }
//! ```
//!
//! Each transition is deterministic given the integrity scalar. The
//! `functional_consequence_active` rule fires once the zone reaches
//! Severed or Destroyed.

use serde::{Deserialize, Serialize};

/// **M14** per-zone limb state. Single source of truth for whether a zone
/// is functional (Intact/Damaged), bleeding (Critical), or lost
/// (Severed/Destroyed).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZoneState {
    #[default]
    Intact = 0,
    Damaged = 1,
    Critical = 2,
    Severed = 3,
    Destroyed = 4,
}

impl ZoneState {
    pub fn as_str(self) -> &'static str {
        match self {
            ZoneState::Intact => "intact",
            ZoneState::Damaged => "damaged",
            ZoneState::Critical => "critical",
            ZoneState::Severed => "severed",
            ZoneState::Destroyed => "destroyed",
        }
    }

    /// Per the spec § Limb-loss state machine — functional consequence
    /// (arm-disabled / leg-limp / etc.) activates at Severed or Destroyed.
    pub fn functional_consequence_active(self) -> bool {
        matches!(self, ZoneState::Severed | ZoneState::Destroyed)
    }

    /// Per the spec § "Critical: near severance; high bleed rate" — only
    /// Critical AND Severed/Destroyed zones bleed at M14. Intact/Damaged
    /// don't bleed.
    pub fn bleeds(self) -> bool {
        matches!(
            self,
            ZoneState::Critical | ZoneState::Severed | ZoneState::Destroyed
        )
    }

    /// Bleed rate multiplier (1.0 = baseline). Spec § "Multiple limbs lost:
    /// bleed-out timer (6 HP/sec per CCCP)".
    pub fn bleed_multiplier(self) -> f32 {
        match self {
            ZoneState::Critical => 0.5,
            ZoneState::Severed | ZoneState::Destroyed => 1.0,
            _ => 0.0,
        }
    }
}

/// Derive a zone state from a normalized integrity (1.0 = full hp; 0.0 =
/// fully gone). Boundary thresholds match the M9 5-tier band machine for
/// chassis layers so consumers can index by either scalar.
#[must_use]
pub fn classify(integrity: f32, severed: bool, destroyed: bool) -> ZoneState {
    if destroyed {
        return ZoneState::Destroyed;
    }
    if severed {
        return ZoneState::Severed;
    }
    let i = integrity.clamp(0.0, 1.0);
    if i >= 0.75 {
        ZoneState::Intact
    } else if i >= 0.40 {
        ZoneState::Damaged
    } else {
        // anything below 0.40 is Critical (near severance).
        ZoneState::Critical
    }
}

/// **M14**: bleed-out per-tick damage. Per CCCP `Actor::Update` the bleed
/// rate is 6 HP/sec at full effect; M14 scales by the number of lost zones
/// (multiple limbs lost = compounding bleed).
///
/// `lost_zones` is the count of zones currently in Severed/Destroyed/Critical
/// state for this actor. Returns HP/tick to subtract.
#[must_use]
pub fn bleed_per_tick(lost_zones: u32, tick_rate_hz: u32) -> f32 {
    if lost_zones == 0 {
        return 0.0;
    }
    let tick_rate = tick_rate_hz.max(1) as f32;
    // Baseline 6 HP/sec scales linearly per lost zone, capped at 4× so a
    // fully-mutilated actor doesn't drop in one tick.
    let multiplier = (lost_zones as f32).min(4.0);
    6.0 * multiplier / tick_rate
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_full_integrity_is_intact() {
        assert_eq!(classify(1.0, false, false), ZoneState::Intact);
        assert_eq!(classify(0.8, false, false), ZoneState::Intact);
    }

    #[test]
    fn classify_mid_integrity_is_damaged() {
        assert_eq!(classify(0.5, false, false), ZoneState::Damaged);
        assert_eq!(classify(0.41, false, false), ZoneState::Damaged);
    }

    #[test]
    fn classify_low_integrity_is_critical() {
        assert_eq!(classify(0.30, false, false), ZoneState::Critical);
        assert_eq!(classify(0.05, false, false), ZoneState::Critical);
    }

    #[test]
    fn severed_overrides_integrity() {
        assert_eq!(classify(1.0, true, false), ZoneState::Severed);
    }

    #[test]
    fn destroyed_overrides_severed() {
        assert_eq!(classify(0.0, true, true), ZoneState::Destroyed);
    }

    #[test]
    fn functional_consequence_active_only_for_severed_or_destroyed() {
        assert!(!ZoneState::Intact.functional_consequence_active());
        assert!(!ZoneState::Damaged.functional_consequence_active());
        assert!(!ZoneState::Critical.functional_consequence_active());
        assert!(ZoneState::Severed.functional_consequence_active());
        assert!(ZoneState::Destroyed.functional_consequence_active());
    }

    #[test]
    fn bleed_zero_when_no_loss() {
        assert!((bleed_per_tick(0, 60)).abs() < f32::EPSILON);
    }

    #[test]
    fn bleed_scales_with_lost_zones() {
        let a = bleed_per_tick(1, 60);
        let b = bleed_per_tick(3, 60);
        assert!(b > a);
    }

    #[test]
    fn bleed_caps_at_four_zones() {
        let four = bleed_per_tick(4, 60);
        let five = bleed_per_tick(5, 60);
        assert!((four - five).abs() < 1e-3);
    }

    #[test]
    fn bleed_at_one_zone_is_six_per_sec() {
        // 6 HP/s @ 60 ticks/s = 0.1 HP/tick.
        let v = bleed_per_tick(1, 60);
        assert!((v - 0.1).abs() < 1e-3);
    }
}
