//! M17: power/heat/vacuum-aware AI doctrine.
//!
//! Pure, deterministic decision functions (no RNG, no clock) that let a bot
//! react to its M17 personal-power / robot-thermal / oxygen survival state:
//! retreat to charge on low battery, retreat from heat under involuntary
//! downclock, refuse vacuum without a sealed helmet + O2, and shed utility
//! equipment to keep the weapon powered.

use serde::{Deserialize, Serialize};

/// Reason a bot's M17 doctrine fired this tick. `as_str()` is the stable
/// reason-label string consumed by replay viewers.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum M17DoctrineReason {
    LowBattery,
    ThermalRetreat,
    NoBreathableAtmosphere,
    PowerShed,
}

impl M17DoctrineReason {
    pub fn as_str(self) -> &'static str {
        match self {
            M17DoctrineReason::LowBattery => "low_battery",
            M17DoctrineReason::ThermalRetreat => "thermal_retreat",
            M17DoctrineReason::NoBreathableAtmosphere => "no_breathable_atmosphere",
            M17DoctrineReason::PowerShed => "power_shed",
        }
    }
}

/// Battery floor below which a powered bot breaks off to recharge.
pub const BATTERY_RETREAT_FRACTION: f32 = 0.20;
/// Chassis-heat fraction at/above which a robot retreats from the heat source.
pub const THERMAL_RETREAT_FRACTION: f32 = 0.70;
/// Battery floor below which a bot sheds utility equipment to keep its weapon.
pub const POWER_SHED_FRACTION: f32 = 0.40;
/// O2 floor (seconds) below which an organic bot refuses vacuum exposure.
pub const VACUUM_MIN_OXYGEN_SECONDS: f32 = 30.0;

/// Robot/android bot retreats to charge when battery is below 20%.
pub fn battery_retreat(power_fraction: f32) -> bool {
    power_fraction < BATTERY_RETREAT_FRACTION
}

/// Robot bot under involuntary thermal downclock (or passive heat at/above the
/// throttle band) retreats from the heat source.
pub fn thermal_retreat(heat_fraction: f32, throttled: bool) -> bool {
    throttled || heat_fraction >= THERMAL_RETREAT_FRACTION
}

/// Organic bot refuses to step into vacuum without a sealed helmet AND at least
/// `VACUUM_MIN_OXYGEN_SECONDS` of O2. Robots (`is_organic == false`) never
/// refuse — they are vacuum-immune.
pub fn refuses_vacuum(is_organic: bool, helmet_sealed: bool, oxygen_seconds: f32) -> Option<&'static str> {
    if is_organic && (!helmet_sealed || oxygen_seconds < VACUUM_MIN_OXYGEN_SECONDS) {
        Some("no_breathable_atmosphere")
    } else {
        None
    }
}

/// Below 40% battery a bot drops flashlight/optics utility draw but keeps the
/// weapon powered (power shedding).
pub fn shed_utility_equipment(power_fraction: f32) -> bool {
    power_fraction < POWER_SHED_FRACTION
}

/// Snapshot inputs for the combined M17 doctrine pass. Deterministic — the
/// caller supplies neutral values (full power, zero heat, sealed helmet) for
/// origins a given branch does not apply to.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct M17DoctrineInputs {
    pub power_fraction: f32,
    pub heat_fraction: f32,
    pub throttled: bool,
    pub is_organic: bool,
    pub helmet_sealed: bool,
    pub oxygen_seconds: f32,
    pub vacuum_exposed: bool,
}

/// Collect every active M17 doctrine reason, highest priority first:
/// `NoBreathableAtmosphere` > `ThermalRetreat` > `LowBattery` > `PowerShed`.
pub fn evaluate_m17_doctrine(inputs: M17DoctrineInputs) -> Vec<M17DoctrineReason> {
    let mut reasons = Vec::new();
    if inputs.vacuum_exposed
        && refuses_vacuum(inputs.is_organic, inputs.helmet_sealed, inputs.oxygen_seconds).is_some()
    {
        reasons.push(M17DoctrineReason::NoBreathableAtmosphere);
    }
    if thermal_retreat(inputs.heat_fraction, inputs.throttled) {
        reasons.push(M17DoctrineReason::ThermalRetreat);
    }
    if battery_retreat(inputs.power_fraction) {
        reasons.push(M17DoctrineReason::LowBattery);
    }
    if shed_utility_equipment(inputs.power_fraction) {
        reasons.push(M17DoctrineReason::PowerShed);
    }
    reasons
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_retreat_below_twenty_percent() {
        assert!(battery_retreat(0.05));
        assert!(battery_retreat(0.19));
        assert!(!battery_retreat(0.20));
        assert!(!battery_retreat(0.25));
        assert!(!battery_retreat(1.0));
    }

    #[test]
    fn thermal_retreat_on_throttle_or_high_heat() {
        assert!(thermal_retreat(0.0, true), "involuntary downclock forces retreat");
        assert!(thermal_retreat(0.70, false), "heat at the throttle band retreats");
        assert!(thermal_retreat(0.95, false));
        assert!(!thermal_retreat(0.69, false), "below the band, no retreat");
        assert!(!thermal_retreat(0.0, false));
    }

    #[test]
    fn refuses_vacuum_for_organic_without_helmet_but_not_robot() {
        assert_eq!(
            refuses_vacuum(true, false, 600.0),
            Some("no_breathable_atmosphere"),
            "organic with no sealed helmet refuses"
        );
        assert_eq!(
            refuses_vacuum(true, true, 10.0),
            Some("no_breathable_atmosphere"),
            "organic with sealed helmet but <30s O2 refuses"
        );
        assert_eq!(refuses_vacuum(true, true, 30.0), None, "sealed helmet + 30s O2 is fine");
        assert_eq!(refuses_vacuum(true, true, 600.0), None, "fully equipped organic proceeds");
        assert_eq!(refuses_vacuum(false, false, 0.0), None, "robot is vacuum-immune");
    }

    #[test]
    fn shed_utility_equipment_below_forty_percent() {
        assert!(shed_utility_equipment(0.10));
        assert!(shed_utility_equipment(0.39));
        assert!(!shed_utility_equipment(0.40));
        assert!(!shed_utility_equipment(0.55));
    }

    #[test]
    fn evaluate_priority_ordering_all_active() {
        let inputs = M17DoctrineInputs {
            power_fraction: 0.10,
            heat_fraction: 0.80,
            throttled: true,
            is_organic: true,
            helmet_sealed: false,
            oxygen_seconds: 5.0,
            vacuum_exposed: true,
        };
        assert_eq!(
            evaluate_m17_doctrine(inputs),
            vec![
                M17DoctrineReason::NoBreathableAtmosphere,
                M17DoctrineReason::ThermalRetreat,
                M17DoctrineReason::LowBattery,
                M17DoctrineReason::PowerShed,
            ]
        );
    }

    #[test]
    fn evaluate_vacuum_only_when_exposed() {
        let base = M17DoctrineInputs {
            power_fraction: 1.0,
            heat_fraction: 0.0,
            throttled: false,
            is_organic: true,
            helmet_sealed: false,
            oxygen_seconds: 0.0,
            vacuum_exposed: false,
        };
        assert!(evaluate_m17_doctrine(base).is_empty(), "no vacuum exposure → no refusal");
        let exposed = M17DoctrineInputs {
            vacuum_exposed: true,
            ..base
        };
        assert_eq!(
            evaluate_m17_doctrine(exposed),
            vec![M17DoctrineReason::NoBreathableAtmosphere]
        );
    }

    #[test]
    fn evaluate_robot_in_vacuum_ignores_atmosphere() {
        let inputs = M17DoctrineInputs {
            power_fraction: 0.15,
            heat_fraction: 0.75,
            throttled: false,
            is_organic: false,
            helmet_sealed: false,
            oxygen_seconds: 0.0,
            vacuum_exposed: true,
        };
        assert_eq!(
            evaluate_m17_doctrine(inputs),
            vec![
                M17DoctrineReason::ThermalRetreat,
                M17DoctrineReason::LowBattery,
                M17DoctrineReason::PowerShed,
            ],
            "robot ignores vacuum but still manages power + heat"
        );
    }

    #[test]
    fn evaluate_power_shed_without_low_battery() {
        let inputs = M17DoctrineInputs {
            power_fraction: 0.30,
            heat_fraction: 0.0,
            throttled: false,
            is_organic: true,
            helmet_sealed: true,
            oxygen_seconds: 600.0,
            vacuum_exposed: false,
        };
        assert_eq!(
            evaluate_m17_doctrine(inputs),
            vec![M17DoctrineReason::PowerShed],
            "between 20% and 40% sheds utility but does not retreat"
        );
    }

    #[test]
    fn reason_as_str_round_trip() {
        assert_eq!(M17DoctrineReason::LowBattery.as_str(), "low_battery");
        assert_eq!(M17DoctrineReason::ThermalRetreat.as_str(), "thermal_retreat");
        assert_eq!(
            M17DoctrineReason::NoBreathableAtmosphere.as_str(),
            "no_breathable_atmosphere"
        );
        assert_eq!(M17DoctrineReason::PowerShed.as_str(), "power_shed");
    }
}
