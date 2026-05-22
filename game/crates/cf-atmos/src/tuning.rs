//! Content-driven atmospheric tuning loader.
//!
//! Reads `content/atmos/tuning.json` once at engine boot and exposes
//! the values via [`AtmosTuning`]. Existing `pub const` declarations in
//! lib.rs and wind.rs serve as boot defaults for tests and the
//! `Default` impl of this struct.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtmosTuning {
    pub schema_version: u32,
    #[serde(default)]
    pub wind: WindTuning,
    #[serde(default)]
    pub ignition: IgnitionTuning,
    #[serde(default)]
    pub respiration: RespirationTuning,
    #[serde(default)]
    pub suit: SuitTuning,
    #[serde(default)]
    pub pipes: PipeTuning,
    #[serde(default)]
    pub ambient_pressure_kpa: AmbientPressureTuning,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindTuning {
    pub buoyancy_force_per_k_delta: f32,
    pub wind_force_per_kpa_differential: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IgnitionTuning {
    pub volatiles_autoignite_k: f32,
    pub volatiles_autoignite_n2o_k: f32,
    pub min_fuel_ratio_for_ignition: f32,
    pub min_oxidizer_ratio_for_ignition: f32,
    pub min_total_pressure_for_ignition_kpa: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RespirationTuning {
    pub min_o2_partial_kpa: f32,
    pub critical_o2_partial_kpa: f32,
    pub damage_o2_partial_kpa: f32,
    pub inhaled_mol_per_tick_base: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuitTuning {
    pub pressure_min_kpa: f32,
    pub pressure_max_kpa: f32,
    pub temp_min_c: f32,
    pub temp_max_c: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PipeTuning {
    pub gas_rupture_kpa: f32,
    pub liquid_rupture_kpa: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AmbientPressureTuning {
    pub earth: f32,
    pub mars: f32,
    pub vacuum: f32,
    pub venus: f32,
}

impl Default for WindTuning {
    fn default() -> Self {
        Self {
            buoyancy_force_per_k_delta: 0.5,
            wind_force_per_kpa_differential: 2.0,
        }
    }
}

impl Default for IgnitionTuning {
    fn default() -> Self {
        Self {
            volatiles_autoignite_k: 573.15,
            volatiles_autoignite_n2o_k: 323.15,
            min_fuel_ratio_for_ignition: 0.05,
            min_oxidizer_ratio_for_ignition: 0.05,
            min_total_pressure_for_ignition_kpa: 10.0,
        }
    }
}

impl Default for RespirationTuning {
    fn default() -> Self {
        Self {
            min_o2_partial_kpa: 16.0,
            critical_o2_partial_kpa: 12.0,
            damage_o2_partial_kpa: 5.0,
            inhaled_mol_per_tick_base: 0.0048,
        }
    }
}

impl Default for SuitTuning {
    fn default() -> Self {
        Self {
            pressure_min_kpa: 11.0,
            pressure_max_kpa: 300.0,
            temp_min_c: -10.0,
            temp_max_c: 49.0,
        }
    }
}

impl Default for PipeTuning {
    fn default() -> Self {
        Self {
            gas_rupture_kpa: 60_795.0,
            liquid_rupture_kpa: 6_079.0,
        }
    }
}

impl Default for AmbientPressureTuning {
    fn default() -> Self {
        Self {
            earth: 101.0,
            mars: 2.5,
            vacuum: 0.01,
            venus: 239.0,
        }
    }
}

impl Default for AtmosTuning {
    fn default() -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            wind: WindTuning::default(),
            ignition: IgnitionTuning::default(),
            respiration: RespirationTuning::default(),
            suit: SuitTuning::default(),
            pipes: PipeTuning::default(),
            ambient_pressure_kpa: AmbientPressureTuning::default(),
        }
    }
}

impl AtmosTuning {
    pub const SCHEMA_VERSION: u32 = 1;

    pub fn load_from_file(path: impl AsRef<std::path::Path>) -> Result<Self, AtmosTuningLoadError> {
        let path_ref = path.as_ref();
        let raw = std::fs::read_to_string(path_ref).map_err(|source| AtmosTuningLoadError::Io {
            path: path_ref.to_path_buf(),
            source,
        })?;
        let parsed: Self = serde_json::from_str(&raw).map_err(|source| AtmosTuningLoadError::Parse {
            path: path_ref.to_path_buf(),
            source,
        })?;
        if parsed.schema_version != Self::SCHEMA_VERSION {
            return Err(AtmosTuningLoadError::SchemaVersionMismatch {
                path: path_ref.to_path_buf(),
                expected: Self::SCHEMA_VERSION,
                actual: parsed.schema_version,
            });
        }
        Ok(parsed)
    }

    pub fn locate_default() -> Option<std::path::PathBuf> {
        for candidate in [
            std::path::PathBuf::from("content/atmos/tuning.json"),
            std::path::PathBuf::from("../content/atmos/tuning.json"),
            std::path::PathBuf::from("game/content/atmos/tuning.json"),
        ] {
            if candidate.exists() {
                return Some(candidate);
            }
        }
        None
    }

    pub fn load_default_or_baseline() -> Self {
        if let Some(path) = Self::locate_default() {
            match Self::load_from_file(&path) {
                Ok(t) => return t,
                Err(err) => {
                    tracing::warn!(
                        target: "cf_atmos::tuning",
                        path = %path.display(),
                        error = ?err,
                        "atmos/tuning.json present but failed to load — using baseline"
                    );
                }
            }
        }
        Self::default()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AtmosTuningLoadError {
    #[error("read {}: {source}", path.display())]
    Io {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("parse {}: {source}", path.display())]
    Parse {
        path: std::path::PathBuf,
        source: serde_json::Error,
    },
    #[error("schema version mismatch {}: expected {expected}, got {actual}", path.display())]
    SchemaVersionMismatch {
        path: std::path::PathBuf,
        expected: u32,
        actual: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_matches_legacy_constants() {
        let t = AtmosTuning::default();
        assert_eq!(t.ignition.volatiles_autoignite_k, 573.15);
        assert_eq!(t.respiration.min_o2_partial_kpa, 16.0);
        assert_eq!(t.suit.pressure_min_kpa, 11.0);
        assert_eq!(t.pipes.gas_rupture_kpa, 60_795.0);
        assert_eq!(t.ambient_pressure_kpa.earth, 101.0);
        assert_eq!(t.wind.buoyancy_force_per_k_delta, 0.5);
    }

    #[test]
    fn schema_round_trip() {
        let t = AtmosTuning::default();
        let json = serde_json::to_string(&t).unwrap();
        let back: AtmosTuning = serde_json::from_str(&json).unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn content_file_matches_baseline() {
        if let Some(path) = AtmosTuning::locate_default() {
            let loaded = AtmosTuning::load_from_file(&path).expect("content tuning loads");
            let baseline = AtmosTuning::default();
            assert_eq!(loaded, baseline, "content/atmos/tuning.json must match baseline constants");
        }
    }
}
