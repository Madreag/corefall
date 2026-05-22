//! Content-driven physics tuning loader.
//!
//! Reads `content/physics/tuning.json` once at engine boot and exposes
//! the values via [`PhysicsTuning`]. Code paths that previously used
//! hardcoded `pub const` values still compile; the consts now serve as
//! boot defaults for tests and the `Default` impl of this struct. New
//! code paths should prefer the struct field so modders can tune the
//! values via JSON without a rebuild.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhysicsTuning {
    pub schema_version: u32,
    #[serde(default)]
    pub ricochet: RicochetTuning,
    #[serde(default)]
    pub projectile_pair: ProjectilePairTuning,
    #[serde(default)]
    pub wounds: WoundTuning,
    #[serde(default)]
    pub rope: RopeTuning,
    #[serde(default)]
    pub perf: PerfTuning,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RicochetTuning {
    pub angle_threshold_rad: f32,
    pub hardness_factor: f32,
    pub energy_loss: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectilePairTuning {
    pub broadphase_bucket_px: f32,
    pub narrowphase_candidate_budget: usize,
    pub kinetic_deflect_energy_retained: f32,
    pub kinetic_deflect_min_angle_deg: f32,
    pub energy_cancel_min_angle_deg: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WoundTuning {
    pub m14g_tooth_threshold: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RopeTuning {
    pub default_segment_count: usize,
    pub default_solver_iterations: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerfTuning {
    pub projectile_sim_p99_budget_ms: f32,
}

impl Default for RicochetTuning {
    fn default() -> Self {
        Self {
            angle_threshold_rad: std::f32::consts::FRAC_PI_3,
            hardness_factor: 4.0,
            energy_loss: 0.4,
        }
    }
}

impl Default for ProjectilePairTuning {
    fn default() -> Self {
        Self {
            broadphase_bucket_px: 32.0,
            narrowphase_candidate_budget: 12,
            kinetic_deflect_energy_retained: 0.6,
            kinetic_deflect_min_angle_deg: 10.0,
            energy_cancel_min_angle_deg: 30.0,
        }
    }
}

impl Default for WoundTuning {
    fn default() -> Self {
        Self { m14g_tooth_threshold: 9.5 }
    }
}

impl Default for RopeTuning {
    fn default() -> Self {
        Self {
            default_segment_count: 8,
            default_solver_iterations: 4,
        }
    }
}

impl Default for PerfTuning {
    fn default() -> Self {
        Self { projectile_sim_p99_budget_ms: 1.0 }
    }
}

impl Default for PhysicsTuning {
    fn default() -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            ricochet: RicochetTuning::default(),
            projectile_pair: ProjectilePairTuning::default(),
            wounds: WoundTuning::default(),
            rope: RopeTuning::default(),
            perf: PerfTuning::default(),
        }
    }
}

impl PhysicsTuning {
    pub const SCHEMA_VERSION: u32 = 1;

    pub fn load_from_file(path: impl AsRef<std::path::Path>) -> Result<Self, TuningLoadError> {
        let path_ref = path.as_ref();
        let raw = std::fs::read_to_string(path_ref).map_err(|source| TuningLoadError::Io {
            path: path_ref.to_path_buf(),
            source,
        })?;
        let parsed: Self = serde_json::from_str(&raw).map_err(|source| TuningLoadError::Parse {
            path: path_ref.to_path_buf(),
            source,
        })?;
        if parsed.schema_version != Self::SCHEMA_VERSION {
            return Err(TuningLoadError::SchemaVersionMismatch {
                path: path_ref.to_path_buf(),
                expected: Self::SCHEMA_VERSION,
                actual: parsed.schema_version,
            });
        }
        Ok(parsed)
    }

    pub fn locate_default() -> Option<std::path::PathBuf> {
        for candidate in [
            std::path::PathBuf::from("content/physics/tuning.json"),
            std::path::PathBuf::from("../content/physics/tuning.json"),
            std::path::PathBuf::from("game/content/physics/tuning.json"),
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
                        target: "cf_physics::tuning",
                        path = %path.display(),
                        error = ?err,
                        "physics/tuning.json present but failed to load — using baseline"
                    );
                }
            }
        }
        Self::default()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TuningLoadError {
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
        let t = PhysicsTuning::default();
        assert!((t.ricochet.angle_threshold_rad - std::f32::consts::FRAC_PI_3).abs() < 1e-6);
        assert_eq!(t.ricochet.hardness_factor, 4.0);
        assert_eq!(t.ricochet.energy_loss, 0.4);
        assert_eq!(t.projectile_pair.broadphase_bucket_px, 32.0);
        assert_eq!(t.projectile_pair.narrowphase_candidate_budget, 12);
        assert_eq!(t.wounds.m14g_tooth_threshold, 9.5);
    }

    #[test]
    fn schema_round_trip() {
        let t = PhysicsTuning::default();
        let json = serde_json::to_string(&t).unwrap();
        let back: PhysicsTuning = serde_json::from_str(&json).unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn content_file_matches_baseline() {
        if let Some(path) = PhysicsTuning::locate_default() {
            let loaded = PhysicsTuning::load_from_file(&path).expect("content tuning loads");
            let baseline = PhysicsTuning::default();
            assert_eq!(loaded, baseline, "content/physics/tuning.json must match baseline constants");
        }
    }
}
