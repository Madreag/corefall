//! **M14A** § "cf-ui::mass_indicator" — HUD MASS line.
//!
//! `MASS: 220kg (0.36× spd) | HELD: 4.2 | INV: 30 | FUEL: 12`

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MassIndicatorHud {
    pub total_mass_kg: f32,
    pub mass_factor_walk: f32,
    pub held_devices_mass_kg: f32,
    pub inventory_weight_kg: f32,
    pub jetpack_fuel_mass_kg: f32,
}

impl MassIndicatorHud {
    pub fn from_observe_payload(payload: &serde_json::Value) -> Self {
        let f = |k: &str| payload.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        Self {
            total_mass_kg: f("total_mass_kg"),
            mass_factor_walk: f("mass_factor_walk"),
            held_devices_mass_kg: f("held_devices_mass_kg"),
            inventory_weight_kg: f("inventory_weight_kg"),
            jetpack_fuel_mass_kg: f("jetpack_fuel_mass_kg"),
        }
    }

    /// HUD line — `MASS: 220kg (0.36× spd) | HELD: 4.2 | INV: 30 | FUEL: 12`.
    pub fn format_line(&self) -> String {
        format!(
            "MASS: {}kg ({:.2}× spd) | HELD: {:.1} | INV: {:.0} | FUEL: {:.0}",
            self.total_mass_kg.round() as i32,
            self.mass_factor_walk,
            self.held_devices_mass_kg,
            self.inventory_weight_kg,
            self.jetpack_fuel_mass_kg,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn format_matches_spec_line_shape() {
        let h = MassIndicatorHud::from_observe_payload(&json!({
            "total_mass_kg": 220.5,
            "mass_factor_walk": 0.36,
            "held_devices_mass_kg": 4.2,
            "inventory_weight_kg": 30.0,
            "jetpack_fuel_mass_kg": 12.0,
        }));
        let line = h.format_line();
        assert!(line.contains("MASS"));
        assert!(line.contains("HELD"));
        assert!(line.contains("INV"));
        assert!(line.contains("FUEL"));
        assert!(line.contains("0.36"));
    }
}
