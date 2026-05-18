//! **M14A** § "cf-ui::jetpack_fuel_meter" — fuel gauge widget.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct JetpackFuelMeterHud {
    pub jet_time_left_ms: u32,
    pub jet_time_total_ms: u32,
    pub fuel_ratio: f32,
    pub is_emitting: bool,
    pub throttle: f32,
    pub jet_kind: String,
}

impl JetpackFuelMeterHud {
    pub fn from_observe_payload(payload: &serde_json::Value) -> Self {
        let jet = payload.get("jetpack");
        if let Some(jet) = jet {
            let i = |k: &str| jet.get(k).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let f = |k: &str| jet.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
            let total = i("jet_time_total_ms");
            let left = i("jet_time_left_ms");
            let ratio = if total > 0 { left as f32 / total as f32 } else { 0.0 };
            Self {
                jet_time_left_ms: left,
                jet_time_total_ms: total,
                fuel_ratio: ratio,
                is_emitting: jet.get("is_emitting").and_then(|v| v.as_bool()).unwrap_or(false),
                throttle: f("throttle"),
                jet_kind: jet.get("type").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            }
        } else {
            Self::default()
        }
    }

    /// HUD label.
    pub fn format_line(&self) -> String {
        if self.jet_time_total_ms == 0 {
            return "FUEL: --".to_string();
        }
        format!(
            "FUEL: {:.0}% ({} / {} ms){}",
            self.fuel_ratio * 100.0,
            self.jet_time_left_ms,
            self.jet_time_total_ms,
            if self.is_emitting { " [BURN]" } else { "" }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_jetpack_block() {
        let h = JetpackFuelMeterHud::from_observe_payload(&json!({
            "jetpack": {
                "jet_time_left_ms": 2250u64,
                "jet_time_total_ms": 4500u64,
                "is_emitting": true,
                "throttle": 0.75,
                "type": "standard",
            }
        }));
        assert!((h.fuel_ratio - 0.5).abs() < 1e-6);
        assert!(h.is_emitting);
    }

    #[test]
    fn formats_no_jetpack_as_dashes() {
        let h = JetpackFuelMeterHud::default();
        assert!(h.format_line().contains("--"));
    }
}
