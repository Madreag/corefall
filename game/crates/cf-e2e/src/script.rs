use std::path::PathBuf;

use anyhow::Result;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub(crate) struct ControlScript {
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    pub(crate) scenario: Option<String>,
    pub(crate) steps: Vec<ScriptStep>,
}

impl ControlScript {
    #[allow(dead_code)]
    pub(crate) fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct ScriptStep {
    pub(crate) method: String,
    #[serde(default)]
    pub(crate) params: Value,
}

pub(crate) fn ticks_to_wait_for(method: &str, params: &Value) -> Option<u64> {
    match method {
        "sim.step" | "sim.run_for_ticks" => params.get("ticks").and_then(|t| t.as_u64()),
        _ => None,
    }
}

pub(crate) fn locate_script(name: &str) -> Result<PathBuf> {
    let candidates = [
        PathBuf::from("scripts/cfctl").join(format!("{name}.cfctl.json")),
        PathBuf::from("../scripts/cfctl").join(format!("{name}.cfctl.json")),
        PathBuf::from("game/scripts/cfctl").join(format!("{name}.cfctl.json")),
    ];
    for c in &candidates {
        if c.exists() {
            return Ok(c.clone());
        }
    }
    anyhow::bail!("script {name} not found at scripts/cfctl/{name}.cfctl.json");
}
