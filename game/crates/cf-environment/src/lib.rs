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

    /// **M14A** § "cf-environment (EXTEND)" — append a hazard to the active list
    /// without duplicates.
    pub fn add_hazard(&mut self, h: HazardClass) {
        if !self.active_hazards.contains(&h) {
            self.active_hazards.push(h);
        }
    }

    /// **M14A** § "per-stride hazard.actor_contact emission from cf-terrain
    /// + cf-atmos sample at planted foot position" — given a per-stride
    ///   material id + atmosphere sample, return the set of active hazards.
    pub fn from_stride_contact(material_id: u8, pressure_kpa: f32, temp_k: f32, o2_kpa: f32) -> Self {
        let mut s = Self::new();
        // Atmosphere-driven hazards.
        if o2_kpa < 16.0 {
            s.add_hazard(HazardClass::Hypoxic);
        }
        let temp_c = temp_k - 273.15;
        if temp_c > 49.0 {
            s.add_hazard(HazardClass::Hyperthermic);
        } else if temp_c < 0.0 {
            s.add_hazard(HazardClass::Hypothermic);
        }
        if pressure_kpa < 11.0 {
            s.add_hazard(HazardClass::BreachDecomp);
        }
        // Material-driven hazards.
        match material_id {
            // lava
            12 => s.add_hazard(HazardClass::Hyperthermic),
            // acid
            13 => s.add_hazard(HazardClass::ToxicAtmosphere),
            // ice
            14 => s.add_hazard(HazardClass::Hypothermic),
            // water
            18 => s.add_hazard(HazardClass::DrowningHazard),
            _ => {}
        }
        s
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
