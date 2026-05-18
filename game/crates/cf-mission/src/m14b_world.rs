//! **M14B** § gravity field + wind force scenario manifest types.
//!
//! Scenario manifests opt into M14B by declaring `gravity_overrides: [...]`
//! and `wind_sources: [...]` arrays. The cf-control scenario loader maps
//! these into the engine's per-tick producer kernel.
//!
//! These types are intentionally `Serialize + Deserialize` and contain no
//! cf-physics / cf-atmos references so cf-mission stays the canonical
//! home for scenario manifest schemas. The cf-control wiring converts
//! them into `cf_physics::GravityOverride` / `cf_atmos::WindSource` /
//! `cf_atmos::StratCell` at engine init.

use serde::{Deserialize, Serialize};

/// **M14B** § scenario manifest entry for a gravity override.
///
/// Serde-untagged so the on-disk RON form looks like an enum without the
/// `kind: ` prefix:
///
/// ```ron
/// gravity_overrides: [
///   (id: 1, kind: "uniform_well", center: (200.0, 100.0), radius: 50.0, magnitude: 25.0),
///   (id: 2, kind: "region_low_g", min: (0.0, 0.0), max: (100.0, 100.0), local_g: 4.9),
///   (id: 3, kind: "magnetic_boots", actor_id: 99),
///   (id: 4, kind: "reverse_g", min: (200.0, 0.0), max: (260.0, 80.0)),
///   (id: 5, kind: "damaged_grav", center: (300.0, 100.0), radius: 80.0, magnitude_factor: 0.4, wave_front_radius: 40.0),
/// ],
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScenarioGravityOverride {
    UniformWell {
        id: u32,
        center: (f32, f32),
        radius: f32,
        magnitude: f32,
    },
    RegionLowG {
        id: u32,
        min: (f32, f32),
        max: (f32, f32),
        local_g: f32,
    },
    MagneticBoots {
        id: u32,
        actor_id: u64,
    },
    ReverseG {
        id: u32,
        min: (f32, f32),
        max: (f32, f32),
    },
    DamagedGrav {
        id: u32,
        center: (f32, f32),
        radius: f32,
        magnitude_factor: f32,
        /// Initial wave-front radius (defaults to 0 for true
        /// "collapse-from-edges" intro). Producer mutates this each tick
        /// per `wave_front_growth_per_s`.
        #[serde(default)]
        wave_front_radius: f32,
        /// Wave-front collapse rate in world units per second. Defaults
        /// to 0 (frozen wave-front). Scenarios that animate the collapse
        /// typically use radius / 10s (e.g. 10.0 px/s for a 100 px radius
        /// finishing in ~10s).
        #[serde(default)]
        wave_front_growth_per_s: f32,
    },
}

impl ScenarioGravityOverride {
    pub fn id(&self) -> u32 {
        match self {
            Self::UniformWell { id, .. }
            | Self::RegionLowG { id, .. }
            | Self::MagneticBoots { id, .. }
            | Self::ReverseG { id, .. }
            | Self::DamagedGrav { id, .. } => *id,
        }
    }
}

/// **M14B** § scenario manifest entry for a wind aperture coupling two
/// cells. The producer reads `(cell_high_id, cell_low_id)` pressures
/// from `atmosphere_cells` and emits an actor-facing force vector
/// along `axis` × aperture_area.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioWindSource {
    pub id: u32,
    pub origin: (f32, f32),
    pub axis: (f32, f32),
    pub aperture_area_m2: f32,
    pub cell_high_id: u32,
    pub cell_low_id: u32,
    pub jet_length: f32,
    pub jet_half_width: f32,
}

/// **M14B** § scenario manifest entry for an authored atmosphere cell.
/// Provides the pressure + temperature + gas composition the producer
/// kernel consumes. M19's full PV=nRT kernel will eventually replace
/// these authored cells with live state; the producer-facing surface
/// stays unchanged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioAtmosCell {
    pub id: u32,
    pub min: (f32, f32),
    pub max: (f32, f32),
    pub pressure_kpa: f32,
    pub temp_k: f32,
    /// Optional vertical column id; cells sharing a column form a stack
    /// for gas stratification. Defaults to the cell's own id when not
    /// declared (single-cell column).
    #[serde(default)]
    pub column_id: Option<u32>,
    /// Optional per-gas mole fractions. Empty = pure air baseline.
    #[serde(default)]
    pub gases: Vec<(String, f32)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gravity_override_serde_round_trips() {
        let ovr = ScenarioGravityOverride::UniformWell {
            id: 1,
            center: (100.0, 50.0),
            radius: 30.0,
            magnitude: 25.0,
        };
        let ron_str = ron::to_string(&ovr).unwrap();
        let back: ScenarioGravityOverride = ron::from_str(&ron_str).unwrap();
        assert_eq!(ovr, back);
    }

    #[test]
    fn magnetic_boots_serde_round_trips() {
        let ovr = ScenarioGravityOverride::MagneticBoots { id: 2, actor_id: 99 };
        let ron_str = ron::to_string(&ovr).unwrap();
        let back: ScenarioGravityOverride = ron::from_str(&ron_str).unwrap();
        assert_eq!(ovr, back);
    }

    #[test]
    fn wind_source_serde_round_trips() {
        let w = ScenarioWindSource {
            id: 1,
            origin: (10.0, 5.0),
            axis: (1.0, 0.0),
            aperture_area_m2: 0.5,
            cell_high_id: 1,
            cell_low_id: 2,
            jet_length: 12.0,
            jet_half_width: 1.0,
        };
        let ron_str = ron::to_string(&w).unwrap();
        let back: ScenarioWindSource = ron::from_str(&ron_str).unwrap();
        assert_eq!(w, back);
    }
}
