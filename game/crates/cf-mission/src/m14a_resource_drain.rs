//! **M14A** § "cf-mission (INTEGRATE)" — per-origin per-stride resource
//! depletion + Stationeers helmet O2 math.
//!
//! This module exposes the spec-locked per-stride drain table for human /
//! robot / android origins. The cf-actor stride emission consumes the
//! per-tick drain (cf-actor::resource_drain) and routes the warning
//! threshold crossings to mission-side resource events.

use serde::{Deserialize, Serialize};

/// origin class. Spec-locked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginClass {
    Human,
    Robot,
    Android,
}

impl OriginClass {
    pub fn from_id(id: &str) -> Self {
        match id {
            "robot" | "synth" => OriginClass::Robot,
            "android" | "hybrid" => OriginClass::Android,
            _ => OriginClass::Human,
        }
    }
}

/// Stride drain values per origin (spec § "Per-origin resource overlay").
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct StrideDrain {
    pub caloric_energy_per_stride: f32,
    pub power_kwh_per_stride: f32,
    pub oil_per_stride: f32,
    pub blood_per_stride: f32,
}

pub fn stride_drain_for_origin(o: OriginClass) -> StrideDrain {
    match o {
        OriginClass::Human => StrideDrain {
            caloric_energy_per_stride: 0.05,
            power_kwh_per_stride: 0.0,
            oil_per_stride: 0.0,
            blood_per_stride: 0.001,
        },
        OriginClass::Robot => StrideDrain {
            caloric_energy_per_stride: 0.0,
            power_kwh_per_stride: 0.02,
            oil_per_stride: 0.0,
            blood_per_stride: 0.0,
        },
        OriginClass::Android => StrideDrain {
            caloric_energy_per_stride: 0.025,
            power_kwh_per_stride: 0.01,
            oil_per_stride: 0.005,
            blood_per_stride: 0.002,
        },
    }
}

/// 0.0048 · BreathingRate · BreathingEfficiency`. Returns mol/tick.
pub fn helmet_o2_inhaled_mol_per_tick(breathing_rate: f32, breathing_efficiency: f32) -> f32 {
    let base = 0.0048;
    base * breathing_rate.max(0.0) * breathing_efficiency.clamp(0.0, 2.0)
}

pub fn skips_unstable_for_origin(o: OriginClass) -> bool {
    matches!(o, OriginClass::Robot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_drain_uses_caloric() {
        let d = stride_drain_for_origin(OriginClass::Human);
        assert!(d.caloric_energy_per_stride > 0.0);
        assert!(d.power_kwh_per_stride == 0.0);
    }

    #[test]
    fn robot_skips_unstable() {
        assert!(skips_unstable_for_origin(OriginClass::Robot));
        assert!(!skips_unstable_for_origin(OriginClass::Human));
    }

    #[test]
    fn helmet_o2_math_matches_stationeers() {
        let mol = helmet_o2_inhaled_mol_per_tick(2.0, 1.0);
        assert!((mol - 0.0096).abs() < 1e-6);
    }
}
