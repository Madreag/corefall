//! Heat-source temperatures per material id, loaded from
//! `content/materials/thermal_sources.json`. Used by the engine's
//! per-tick heat injection pass.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::MaterialId;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThermalSourceTable {
    pub schema_version: u32,
    pub sources: Vec<ThermalSource>,
    #[serde(default = "default_diffuse_mix")]
    pub diffuse_mix: f32,
    #[serde(default = "default_cool_mix")]
    pub cool_mix: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThermalSource {
    pub material_id: MaterialId,
    pub material_name: String,
    pub source_temp_k: f32,
}

fn default_diffuse_mix() -> f32 {
    0.05
}
fn default_cool_mix() -> f32 {
    0.01
}

impl Default for ThermalSourceTable {
    fn default() -> Self {
        baseline_table()
    }
}

impl ThermalSourceTable {
    pub const SCHEMA_VERSION: u32 = 1;

    pub fn lookup(&self, id: MaterialId) -> Option<f32> {
        self.sources
            .iter()
            .find(|s| s.material_id == id)
            .map(|s| s.source_temp_k)
    }

    pub fn build_map(&self) -> BTreeMap<MaterialId, f32> {
        self.sources.iter().map(|s| (s.material_id, s.source_temp_k)).collect()
    }

    pub fn load_from_file(path: impl AsRef<std::path::Path>) -> Result<Self, ThermalSourceLoadError> {
        let path_ref = path.as_ref();
        let raw = std::fs::read_to_string(path_ref).map_err(|source| ThermalSourceLoadError::Io {
            path: path_ref.to_path_buf(),
            source,
        })?;
        let parsed: Self = serde_json::from_str(&raw).map_err(|source| ThermalSourceLoadError::Parse {
            path: path_ref.to_path_buf(),
            source,
        })?;
        if parsed.schema_version != Self::SCHEMA_VERSION {
            return Err(ThermalSourceLoadError::SchemaVersionMismatch {
                path: path_ref.to_path_buf(),
                expected: Self::SCHEMA_VERSION,
                actual: parsed.schema_version,
            });
        }
        Ok(parsed)
    }

    pub fn locate_default() -> Option<std::path::PathBuf> {
        for candidate in [
            std::path::PathBuf::from("content/materials/thermal_sources.json"),
            std::path::PathBuf::from("../content/materials/thermal_sources.json"),
            std::path::PathBuf::from("game/content/materials/thermal_sources.json"),
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
                        target: "cf_material::thermal_sources",
                        path = %path.display(),
                        error = ?err,
                        "thermal_sources.json present but failed to load — using baseline"
                    );
                }
            }
        }
        baseline_table()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ThermalSourceLoadError {
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

fn baseline_table() -> ThermalSourceTable {
    ThermalSourceTable {
        schema_version: ThermalSourceTable::SCHEMA_VERSION,
        diffuse_mix: 0.05,
        cool_mix: 0.01,
        sources: vec![
            ThermalSource { material_id: 65, material_name: "fire_intense".into(), source_temp_k: 1200.0 },
            ThermalSource { material_id: 26, material_name: "lava".into(), source_temp_k: 1473.0 },
            ThermalSource { material_id: 64, material_name: "lightning".into(), source_temp_k: 30000.0 },
            ThermalSource { material_id: 63, material_name: "electric_arc".into(), source_temp_k: 6000.0 },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_lookup() {
        let t = ThermalSourceTable::default();
        assert_eq!(t.lookup(65), Some(1200.0));
        assert_eq!(t.lookup(26), Some(1473.0));
        assert_eq!(t.lookup(0), None);
    }

    #[test]
    fn schema_round_trip() {
        let t = ThermalSourceTable::default();
        let json = serde_json::to_string(&t).unwrap();
        let back: ThermalSourceTable = serde_json::from_str(&json).unwrap();
        assert_eq!(t, back);
    }
}
