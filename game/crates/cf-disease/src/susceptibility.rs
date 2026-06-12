//! Per-origin susceptibility matrix (10 races × 17 diseases).
//!
//! A `multiplier == 0.0` is immunity (short-circuits the exposure check);
//! `1.0` is the human baseline. The grid lives in
//! `content/diseases/_susceptibility_matrix.ron`; the hardcoded
//! `default_matrix()` derives the same values from per-origin class rules
//! for boot.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{DiseaseKind, OriginId, PathogenClass};

/// 10×17 susceptibility grid: origin id → disease id → multiplier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SusceptibilityMatrix {
    pub schema_version: u32,
    pub grid: BTreeMap<String, BTreeMap<String, f32>>,
}

impl Default for SusceptibilityMatrix {
    fn default() -> Self {
        Self::default_matrix()
    }
}

impl SusceptibilityMatrix {
    pub const SCHEMA_VERSION: u32 = 1;

    /// Susceptibility multiplier for an (origin, disease) pair. Falls back to
    /// the class-rule default when the grid lacks the entry.
    pub fn multiplier(&self, origin: OriginId, disease: DiseaseKind) -> f32 {
        self.grid
            .get(origin.as_str())
            .and_then(|m| m.get(disease.as_str()))
            .copied()
            .unwrap_or_else(|| default_multiplier(origin, disease))
    }

    /// True when the origin is immune (multiplier <= 0) to the disease.
    pub fn is_immune(&self, origin: OriginId, disease: DiseaseKind) -> bool {
        self.multiplier(origin, disease) <= 0.0
    }

    /// Build the full 10×17 grid from the per-origin class rules.
    pub fn default_matrix() -> Self {
        let mut grid = BTreeMap::new();
        for &origin in OriginId::all() {
            let mut row = BTreeMap::new();
            for &disease in DiseaseKind::all() {
                row.insert(disease.as_str().to_string(), default_multiplier(origin, disease));
            }
            grid.insert(origin.as_str().to_string(), row);
        }
        Self {
            schema_version: Self::SCHEMA_VERSION,
            grid,
        }
    }

    /// Load the matrix from `content/diseases/_susceptibility_matrix.ron`.
    /// Falls back to `default_matrix()` if the file is missing.
    pub fn load_file(path: &Path) -> Result<Self, SusceptibilityLoadError> {
        if !path.exists() {
            return Ok(Self::default_matrix());
        }
        let body = fs::read_to_string(path)
            .map_err(|e| SusceptibilityLoadError::Io(path.to_path_buf(), e.to_string()))?;
        match ron::from_str::<SusceptibilityMatrix>(&body) {
            Ok(m) => Ok(m),
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "susceptibility matrix parse failed");
                Err(SusceptibilityLoadError::Parse(path.to_path_buf(), e.to_string()))
            }
        }
    }
}

/// Per-origin class-rule default multiplier. The spec § "Per-origin
/// susceptibility matrix" anchors:
///   - Methane breathers immune to oxygen-respiratory diseases.
///   - Robots / drones immune to biological diseases.
///   - Photosynthetics 0.3× to bacterial, 1.5× to fungal.
pub fn default_multiplier(origin: OriginId, disease: DiseaseKind) -> f32 {
    let class = disease.pathogen_class();
    match origin {
        OriginId::Human => 1.0,
        OriginId::Robot | OriginId::Drone => {
            // Synthetics: immune to all biological diseases; radiation
            // damages circuits (handled by M17, not as a disease) and they
            // have no psychology — immune across the board here.
            0.0
        }
        OriginId::Android => {
            // Synthetic-organic hybrid: half-susceptible to biological, no
            // psychological illness.
            match class {
                PathogenClass::Psychological => 0.0,
                _ if class.is_biological() => 0.5,
                _ => 0.7,
            }
        }
        OriginId::HeavyBiomech => {
            // Part machine: reduced biological susceptibility.
            match class {
                PathogenClass::Psychological => 0.4,
                _ if class.is_biological() => 0.6,
                _ => 0.8,
            }
        }
        OriginId::MethaneBreather => {
            // Immune to oxygen-respiratory diseases (the airborne human
            // respiratory set) + their own anaerobic biology resists most.
            if is_oxygen_respiratory(disease) {
                0.0
            } else {
                match class {
                    PathogenClass::Bacterial => 0.7,
                    PathogenClass::Viral => 0.5,
                    PathogenClass::Psychological => 0.8,
                    _ => 1.0,
                }
            }
        }
        OriginId::Crystalline => {
            // Silicon-based: highly resistant to carbon-biology pathogens.
            match class {
                PathogenClass::Fungal => 0.0,
                PathogenClass::Bacterial | PathogenClass::Viral => 0.3,
                PathogenClass::Neoplastic => 0.5,
                PathogenClass::WoundInfection => 0.4,
                PathogenClass::Psychological => 0.6,
                _ => 1.0,
            }
        }
        OriginId::Aqueous => {
            // Water-based: extra-vulnerable to waterborne, normal otherwise.
            match disease {
                DiseaseKind::Cholera | DiseaseKind::Typhoid => 1.5,
                _ => match class {
                    PathogenClass::Psychological => 0.8,
                    _ => 1.0,
                },
            }
        }
        OriginId::Photosynthetic => {
            // 0.3× bacterial, 1.5× fungal per spec.
            match class {
                PathogenClass::Fungal => 1.5,
                PathogenClass::Bacterial => 0.3,
                PathogenClass::Viral => 0.6,
                PathogenClass::Psychological => 0.7,
                _ => 1.0,
            }
        }
        OriginId::Insectoid => {
            // Chitinous + hive biology: broad resistance, spore-vulnerable.
            match disease {
                DiseaseKind::Anthrax => 1.2,
                _ => match class {
                    PathogenClass::Bacterial | PathogenClass::Viral => 0.6,
                    PathogenClass::Fungal => 0.8,
                    PathogenClass::Psychological => 0.5,
                    _ => 1.0,
                },
            }
        }
    }
}

/// Oxygen-respiratory diseases methane breathers are immune to (the airborne
/// human respiratory set + slimelung).
fn is_oxygen_respiratory(disease: DiseaseKind) -> bool {
    matches!(
        disease,
        DiseaseKind::Slimelung
            | DiseaseKind::CommonCold
            | DiseaseKind::Flu
            | DiseaseKind::Pneumonia
            | DiseaseKind::Tuberculosis
            | DiseaseKind::InfluenzaPandemic
    )
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum SusceptibilityLoadError {
    #[error("io error reading {0:?}: {1}")]
    Io(PathBuf, String),
    #[error("parse error in {0:?}: {1}")]
    Parse(PathBuf, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_is_full_10x17() {
        let m = SusceptibilityMatrix::default_matrix();
        assert_eq!(m.grid.len(), 10);
        for &o in OriginId::all() {
            let row = m.grid.get(o.as_str()).unwrap();
            assert_eq!(row.len(), 17, "origin {} missing diseases", o.as_str());
        }
    }

    #[test]
    fn methane_breather_immune_to_slimelung() {
        let m = SusceptibilityMatrix::default_matrix();
        assert!(m.is_immune(OriginId::MethaneBreather, DiseaseKind::Slimelung));
        assert_eq!(m.multiplier(OriginId::MethaneBreather, DiseaseKind::Slimelung), 0.0);
    }

    #[test]
    fn robots_immune_to_all_biological() {
        let m = SusceptibilityMatrix::default_matrix();
        for &d in DiseaseKind::all() {
            assert!(m.is_immune(OriginId::Robot, d), "robot not immune to {}", d.as_str());
            assert!(m.is_immune(OriginId::Drone, d), "drone not immune to {}", d.as_str());
        }
    }

    #[test]
    fn photosynthetic_class_rules() {
        let m = SusceptibilityMatrix::default_matrix();
        // 1.5× fungal (slimelung), 0.3× bacterial (cholera).
        assert!((m.multiplier(OriginId::Photosynthetic, DiseaseKind::Slimelung) - 1.5).abs() < 1e-6);
        assert!((m.multiplier(OriginId::Photosynthetic, DiseaseKind::Cholera) - 0.3).abs() < 1e-6);
    }

    #[test]
    fn human_baseline_is_one() {
        let m = SusceptibilityMatrix::default_matrix();
        for &d in DiseaseKind::all() {
            assert!((m.multiplier(OriginId::Human, d) - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn aqueous_extra_vulnerable_to_waterborne() {
        let m = SusceptibilityMatrix::default_matrix();
        assert!(m.multiplier(OriginId::Aqueous, DiseaseKind::Cholera) > 1.0);
    }

    #[test]
    fn round_trips_through_ron() {
        let m = SusceptibilityMatrix::default_matrix();
        let s = ron::to_string(&m).unwrap();
        let back: SusceptibilityMatrix = ron::from_str(&s).unwrap();
        assert_eq!(m, back);
    }
}
