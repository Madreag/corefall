//! cf-environment — M5.10 (DR-040) `EnvironmentSignal` aggregator.
//!
//! BP4 + BP5 forward-compat scaffold. Real implementation lands at M5.10 (BP5).
//! Today this crate ships only the locked types so cf-control / cf-actor /
//! cf-ai can `use cf_environment::{EnvironmentSignal, HazardClass}` without
//! waiting on the full kernel.

use serde::{Deserialize, Serialize};

/// **DR-040** 15-class closed-enum hazard taxonomy. Stable variant IDs;
/// new variants append at the end to preserve `#[repr(u8)]` discriminant
/// order if/when that becomes a serialization shape.
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum HazardClass {
    Hypoxic = 0,
    CombustibleAtmosphere = 1,
    ToxicAtmosphere = 2,
    BreachDecomp = 3,
    Hyperthermic = 4,
    Hypothermic = 5,
    Radiation = 6,
    LowVisibility = 7,
    Glare = 8,
    EmDisruption = 9,
    WindForce = 10,
    DrowningHazard = 11,
    VacuumNoVoice = 12,
    CommsBlackout = 13,
    GravityShift = 14,
}

/// **DR-040** per-actor per-tick aggregated bundle. **Stub at M5**; M5.10
/// fills in atmosphere / gravity / thermal / radiation / weather / comms /
/// day_night slices.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EnvironmentSignal {
    pub schema_version: u32,
    pub active_hazards: Vec<HazardClass>,
}

impl EnvironmentSignal {
    pub const SCHEMA_VERSION: u32 = 1;
    pub fn new() -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hazard_class_round_trips() {
        for c in [HazardClass::Hypoxic, HazardClass::GravityShift] {
            let json = serde_json::to_string(&c).unwrap();
            let back: HazardClass = serde_json::from_str(&json).unwrap();
            assert_eq!(c, back);
        }
    }
    #[test]
    fn environment_signal_default_round_trip() {
        let s = EnvironmentSignal::new();
        let json = serde_json::to_string(&s).unwrap();
        let back: EnvironmentSignal = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }
}
