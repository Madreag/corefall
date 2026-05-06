//! Minimal RON scenario loader for M0. The full schema lives in
//! `spec/prototype-roadmap.md` Scenario Manifest Schema; M0 only needs the
//! fields used by the engine bootstrap.

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub schema_version: u32,
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub seed: u64,
    pub duration_ticks: Option<u64>,
    pub region: ScenarioRegion,
    pub gravity: f32,
    #[serde(default)]
    pub teams: Vec<serde_json::Value>,
    #[serde(default)]
    pub actors: Vec<serde_json::Value>,
    #[serde(default)]
    pub objectives: Vec<serde_json::Value>,
    #[serde(default)]
    pub director: Option<serde_json::Value>,
    pub capabilities: ScenarioCapabilities,
    #[serde(default)]
    pub save_fields: Vec<String>,
    #[serde(default)]
    pub expected_tests: Vec<String>,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioRegion {
    pub anchor: (f32, f32),
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScenarioCapabilities {
    pub debug: bool,
    pub control_api: bool,
    pub save_load: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ScenarioLoadError {
    #[error("io error reading scenario {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("ron parse error in scenario {path}: {source}")]
    Ron {
        path: String,
        #[source]
        source: ron::error::SpannedError,
    },
    #[error("scenario id mismatch: expected {expected}, found {found} in {path}")]
    IdMismatch {
        expected: String,
        found: String,
        path: String,
    },
}

impl Scenario {
    pub fn load_from_file(path: &Path) -> Result<Self, ScenarioLoadError> {
        let text = std::fs::read_to_string(path).map_err(|source| ScenarioLoadError::Io {
            path: path.display().to_string(),
            source,
        })?;
        let scenario: Scenario = ron::from_str(&text).map_err(|source| ScenarioLoadError::Ron {
            path: path.display().to_string(),
            source,
        })?;
        Ok(scenario)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"(
  schema_version: 1,
  id: "m0_blank",
  display_name: "M0 Blank Scene",
  description: "Empty scene used for engine bootstrap and run-bundle smoke.",
  seed: 42,
  duration_ticks: Some(300),
  region: (anchor: (0.0, 0.0), width: 1280.0, height: 720.0),
  gravity: -980.0,
  teams: [],
  actors: [],
  objectives: [],
  director: None,
  capabilities: (
    debug: false,
    control_api: true,
    save_load: false,
  ),
  save_fields: [],
  expected_tests: ["M0-SMOKE-01"],
  notes: "",
)"#;

    #[test]
    fn loads_minimal_scenario() {
        let parsed: Scenario = ron::from_str(SAMPLE).expect("sample must parse");
        assert_eq!(parsed.id, "m0_blank");
        assert_eq!(parsed.seed, 42);
        assert_eq!(parsed.duration_ticks, Some(300));
        assert_eq!(parsed.expected_tests, vec!["M0-SMOKE-01"]);
        assert!(parsed.capabilities.control_api);
    }
}
