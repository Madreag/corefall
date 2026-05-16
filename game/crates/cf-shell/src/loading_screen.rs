//! Loading screen — tip rotation + per-scenario screenshot + progress bar.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoadingTipsManifest {
    pub schema_version: u32,
    pub description: String,
    pub tips: Vec<LoadingTip>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoadingTip {
    pub id: String,
    pub category: String,
    pub text: String,
}

/// Load the loading-tips JSON manifest.
pub fn load_tips<P: AsRef<Path>>(path: P) -> Result<LoadingTipsManifest, String> {
    let txt = std::fs::read_to_string(path).map_err(|e| format!("read failed: {}", e))?;
    let manifest: LoadingTipsManifest = serde_json::from_str(&txt).map_err(|e| format!("JSON parse failed: {}", e))?;
    Ok(manifest)
}

/// Rotate to the next tip given a stable seed (e.g., loading session id).
/// Returns the tip index in the rotation.
pub fn rotate_tip(prev_index: usize, total: usize, seed: u64) -> usize {
    if total == 0 {
        return 0;
    }
    let next = (prev_index + 1 + (seed as usize % 7)) % total;
    next
}

/// Pick a scenario-specific loading background asset id.
/// Falls back to a generic command-core image if scenario_id unknown.
pub fn loading_bg_for_scenario(scenario_id: &str) -> &'static str {
    let lower = scenario_id.to_lowercase();
    if lower.contains("mars") {
        "loading_bg_mars_dust_plain"
    } else if lower.contains("europa") {
        "loading_bg_europa_ice_cavern"
    } else if lower.contains("vulcan") {
        "loading_bg_vulcan_magma"
    } else if lower.contains("venus") {
        "loading_bg_venus_acid_cloud"
    } else if lower.contains("mimas") {
        "loading_bg_mimas_methane_sea"
    } else if lower.contains("phobos") {
        "loading_bg_phobos_microgravity"
    } else if lower.contains("deimos") {
        "loading_bg_deimos_mining_colony"
    } else if lower.contains("moon") {
        "loading_bg_moon_vacuum"
    } else if lower.contains("belt") {
        "loading_bg_belt_asteroid_mining"
    } else if lower.contains("orbital") {
        "loading_bg_orbital_station_interior"
    } else if lower.contains("sol") {
        "loading_bg_sol_zone_stellar"
    } else if lower.contains("reactor") {
        "loading_bg_reactor_chamber"
    } else if lower.contains("breach") {
        "loading_bg_drop_ship_descent"
    } else if lower.contains("husk") {
        "loading_bg_husk_zone"
    } else if lower.contains("anomaly") {
        "loading_bg_anomaly_field"
    } else if lower.contains("earth") {
        "loading_bg_earth_wasteland"
    } else {
        "loading_bg_command_core"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotate_tip_returns_zero_for_empty() {
        assert_eq!(rotate_tip(0, 0, 12345), 0);
    }

    #[test]
    fn rotate_tip_advances_within_bounds() {
        for prev in 0..50 {
            let next = rotate_tip(prev, 60, 100);
            assert!(next < 60);
        }
    }

    #[test]
    fn loading_bg_per_world_distinct() {
        let mars = loading_bg_for_scenario("mars_reactor_defense");
        let europa = loading_bg_for_scenario("europa_ice_cavern");
        let vulcan = loading_bg_for_scenario("vulcan_magma_drill");
        assert_ne!(mars, europa);
        assert_ne!(europa, vulcan);
        assert_ne!(mars, vulcan);
    }

    #[test]
    fn loading_bg_unknown_scenario_falls_back() {
        let bg = loading_bg_for_scenario("totally_unknown_scenario_xyz");
        assert_eq!(bg, "loading_bg_command_core");
    }

    #[test]
    fn loading_bg_for_breach_scenario() {
        let bg = loading_bg_for_scenario("breach_assault_concrete");
        assert_eq!(bg, "loading_bg_drop_ship_descent");
    }
}
