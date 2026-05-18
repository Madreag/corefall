//! **M14G** § Tile thermal → typed wound producer.
//!
//! Maps a per-tick "actor zone in contact with tile of temperature T" sample
//! to a typed [`cf_wound::WoundKind`]. Pure / deterministic.
//!
//! Sustained hot-tile contact escalates the visible burn degree:
//! - `dwell_ticks ≥ FIRST_DEGREE_TICKS` → `Burn1st`
//! - `dwell_ticks ≥ SECOND_DEGREE_TICKS` → `Burn2nd`
//! - `dwell_ticks ≥ THIRD_DEGREE_TICKS` → `Burn3rd`
//!
//! Sustained cold-tile contact escalates the visible frostbite degree:
//! - `dwell_ticks ≥ FIRST_FROSTBITE_TICKS` → `Frostbite1st`
//! - `dwell_ticks ≥ SECOND_FROSTBITE_TICKS` → `Frostbite2nd`
//! - `dwell_ticks ≥ THIRD_FROSTBITE_TICKS` → `Frostbite3rd`
//!
//! Per spec Gherkin scenarios 2 + 3:
//!   - Foot on fire: tick 5 → Burn1st, tick 30 → Burn2nd, tick 60 → Burn3rd.
//!   - Hand at 250 K: tick 60 → Frostbite1st, tick 600 → Frostbite3rd.

use cf_wound::registry::ZoneId;
use cf_wound::WoundKind;

pub const HOT_TILE_THRESHOLD_K: f32 = 320.0;
pub const COLD_TILE_THRESHOLD_K: f32 = 273.15;

pub const BURN_FIRST_DEGREE_TICKS: u64 = 5;
pub const BURN_SECOND_DEGREE_TICKS: u64 = 30;
pub const BURN_THIRD_DEGREE_TICKS: u64 = 60;

pub const FROSTBITE_FIRST_DEGREE_TICKS: u64 = 60;
pub const FROSTBITE_SECOND_DEGREE_TICKS: u64 = 300;
pub const FROSTBITE_THIRD_DEGREE_TICKS: u64 = 600;

/// **M14G** tile-thermal typed wound emit. Severity is the per-band default
/// (0.2 / 0.5 / 0.85 for burns; 0.2 / 0.5 / 0.9 for frostbite).
#[derive(Debug, Clone, PartialEq)]
pub struct ThermalWoundEmit {
    pub kind: WoundKind,
    pub severity: f32,
    pub zone: ZoneId,
}

/// Classify a tile-thermal contact sample into a typed wound. Returns
/// `None` when the temperature is in the safe band or the dwell hasn't
/// crossed a burn/frostbite threshold yet.
pub fn classify_tile_thermal(zone: ZoneId, temperature_k: f32, dwell_ticks: u64) -> Option<ThermalWoundEmit> {
    if temperature_k >= HOT_TILE_THRESHOLD_K {
        if dwell_ticks >= BURN_THIRD_DEGREE_TICKS {
            return Some(ThermalWoundEmit {
                kind: WoundKind::Burn3rd,
                severity: 0.85,
                zone,
            });
        }
        if dwell_ticks >= BURN_SECOND_DEGREE_TICKS {
            return Some(ThermalWoundEmit {
                kind: WoundKind::Burn2nd,
                severity: 0.5,
                zone,
            });
        }
        if dwell_ticks >= BURN_FIRST_DEGREE_TICKS {
            return Some(ThermalWoundEmit {
                kind: WoundKind::Burn1st,
                severity: 0.2,
                zone,
            });
        }
    } else if temperature_k <= COLD_TILE_THRESHOLD_K - 13.15 {
        // ≤ 260 K — frostbite zone.
        if dwell_ticks >= FROSTBITE_THIRD_DEGREE_TICKS {
            return Some(ThermalWoundEmit {
                kind: WoundKind::Frostbite3rd,
                severity: 0.9,
                zone,
            });
        }
        if dwell_ticks >= FROSTBITE_SECOND_DEGREE_TICKS {
            return Some(ThermalWoundEmit {
                kind: WoundKind::Frostbite2nd,
                severity: 0.5,
                zone,
            });
        }
        if dwell_ticks >= FROSTBITE_FIRST_DEGREE_TICKS {
            return Some(ThermalWoundEmit {
                kind: WoundKind::Frostbite1st,
                severity: 0.2,
                zone,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// VAL-M14G-013: burn-degree escalation at 5/30/60 ticks.
    #[test]
    fn burn_degree_escalation_timeline() {
        let zone = ZoneId::from("foot_right");
        // Tick 4 → no wound (below first-degree threshold).
        assert!(classify_tile_thermal(zone.clone(), 800.0, 4).is_none());
        let first = classify_tile_thermal(zone.clone(), 800.0, 5).unwrap();
        assert_eq!(first.kind, WoundKind::Burn1st);
        let second = classify_tile_thermal(zone.clone(), 800.0, 30).unwrap();
        assert_eq!(second.kind, WoundKind::Burn2nd);
        let third = classify_tile_thermal(zone, 800.0, 60).unwrap();
        assert_eq!(third.kind, WoundKind::Burn3rd);
    }

    /// VAL-M14G-014: Frostbite1st at tick 60; Frostbite3rd at tick 600.
    #[test]
    fn frostbite_emergence_and_escalation() {
        let zone = ZoneId::from("hand_right");
        // 250 K is below 260 K so frostbite engages.
        let temp_k = 250.0;
        assert!(classify_tile_thermal(zone.clone(), temp_k, 59).is_none());
        let one = classify_tile_thermal(zone.clone(), temp_k, 60).unwrap();
        assert_eq!(one.kind, WoundKind::Frostbite1st);
        let three = classify_tile_thermal(zone, temp_k, 600).unwrap();
        assert_eq!(three.kind, WoundKind::Frostbite3rd);
    }

    /// VAL-M14G-030: hot tile escalates 1st/2nd/3rd and cold tile escalates
    /// frostbite 1st/2nd/3rd by exposure ticks.
    #[test]
    fn hot_cold_thermal_wound_escalation_ladders() {
        let zone = ZoneId::from("foot_left");
        // Hot ladder: produce 1st/2nd/3rd in order.
        assert_eq!(
            classify_tile_thermal(zone.clone(), 800.0, 5).unwrap().kind,
            WoundKind::Burn1st
        );
        assert_eq!(
            classify_tile_thermal(zone.clone(), 800.0, 30).unwrap().kind,
            WoundKind::Burn2nd
        );
        assert_eq!(
            classify_tile_thermal(zone.clone(), 800.0, 60).unwrap().kind,
            WoundKind::Burn3rd
        );
        // Cold ladder.
        assert_eq!(
            classify_tile_thermal(zone.clone(), 250.0, 60).unwrap().kind,
            WoundKind::Frostbite1st
        );
        assert_eq!(
            classify_tile_thermal(zone.clone(), 250.0, 300).unwrap().kind,
            WoundKind::Frostbite2nd
        );
        assert_eq!(
            classify_tile_thermal(zone, 250.0, 600).unwrap().kind,
            WoundKind::Frostbite3rd
        );
    }

    /// Safe-band temperature produces no wound.
    #[test]
    fn safe_band_returns_none() {
        assert!(classify_tile_thermal(ZoneId::from("foot_left"), 293.0, 1000).is_none());
    }
}
