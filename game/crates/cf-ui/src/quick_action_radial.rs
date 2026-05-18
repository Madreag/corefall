//! **M14A** § "cf-ui::quick_action_radial" — 8-slice radial widget with
//! time-slow ramp + center deadzone.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct QuickActionRadialHud {
    pub is_open: bool,
    pub phase: String,
    pub sim_time_multiplier: f32,
    pub selected_slice: u8,
    pub reduce_motion: bool,
}

impl QuickActionRadialHud {
    pub fn from_observe_payload(payload: &serde_json::Value) -> Self {
        let phase = payload.get("radial_phase").and_then(|v| v.as_str()).unwrap_or("closed");
        let mult = payload
            .get("sim_time_multiplier")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;
        Self {
            is_open: payload.get("radial_open").and_then(|v| v.as_bool()).unwrap_or(false),
            phase: phase.to_string(),
            sim_time_multiplier: mult,
            selected_slice: 0,
            reduce_motion: false,
        }
    }

    /// Vignette alpha drives the time-slow screen-edge tint.
    pub fn vignette_alpha(&self) -> f32 {
        if self.reduce_motion {
            return 0.0;
        }
        let span = 1.0 - 0.25;
        ((1.0 - self.sim_time_multiplier) / span).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn vignette_alpha_zero_when_closed() {
        let h = QuickActionRadialHud::from_observe_payload(&json!({
            "radial_open": false,
            "radial_phase": "closed",
            "sim_time_multiplier": 1.0
        }));
        assert!((h.vignette_alpha() - 0.0).abs() < 1e-6);
    }

    #[test]
    fn vignette_alpha_one_when_fully_slow() {
        let h = QuickActionRadialHud::from_observe_payload(&json!({
            "radial_open": true,
            "radial_phase": "open",
            "sim_time_multiplier": 0.25
        }));
        assert!((h.vignette_alpha() - 1.0).abs() < 1e-6);
    }
}
