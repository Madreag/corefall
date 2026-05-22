//! **M14A** § "cf-internal (INTEGRATE)" — heat transfer at foot contact
//! routes thermal stress to organs (humans) or thermal circuits (robots).
//!
//! Per-tick contact temperature delta is computed via the foot-contact
//! patch area + material thermal conductivity. M14A wires the producer side
//! at the planted-foot position; this module is the routing helper that
//! turns a thermal delta into per-organ stress.

use serde::{Deserialize, Serialize};

/// Approximate foot-contact patch area in m² — used to scale heat transfer
/// per tick. Spec § Constants: 0.04 m² (≈ 20 cm²).
pub const FOOT_CONTACT_AREA_M2: f32 = 0.04;

/// temperature delta from material contact.
///
/// `material_conductivity` ∈ [0, 1]; 1 = perfect conductor (metal),
/// 0.3 = baseline (dirt/concrete).
/// `material_temp_c` is the contact surface temperature.
/// `actor_temp_c` is the actor's core/skin temperature.
/// `dt_secs` is the tick time delta.
pub fn body_temp_delta_per_tick(
    material_conductivity: f32,
    material_temp_c: f32,
    actor_temp_c: f32,
    dt_secs: f32,
) -> f32 {
    let temp_diff = material_temp_c - actor_temp_c;
    let conductivity = material_conductivity.clamp(0.0, 1.0);
    // Simple Newton's law of cooling: ΔT = α · (T_mat - T_body) · area · dt
    temp_diff * conductivity * FOOT_CONTACT_AREA_M2 * dt_secs * 5.0
}

/// from the per-tick body temperature delta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThermalContactEffect {
    None,
    Cooling,
    Burning,
    Freezing,
}

impl ThermalContactEffect {
    /// Classify a per-tick delta into a thermal effect band.
    pub fn classify(delta_c_per_tick: f32) -> Self {
        if delta_c_per_tick.abs() < 0.005 {
            ThermalContactEffect::None
        } else if delta_c_per_tick > 0.05 {
            ThermalContactEffect::Burning
        } else if delta_c_per_tick > 0.0 {
            ThermalContactEffect::Cooling
        } else if delta_c_per_tick < -0.05 {
            ThermalContactEffect::Freezing
        } else {
            ThermalContactEffect::None
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ThermalContactEffect::None => "none",
            ThermalContactEffect::Cooling => "cooling",
            ThermalContactEffect::Burning => "burning",
            ThermalContactEffect::Freezing => "freezing",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lava_burns_actor() {
        let dt = body_temp_delta_per_tick(0.95, 1100.0, 36.6, 1.0 / 60.0);
        assert!(dt > 0.0);
        assert_eq!(ThermalContactEffect::classify(dt), ThermalContactEffect::Burning);
    }

    #[test]
    fn ice_cools_actor() {
        let dt = body_temp_delta_per_tick(0.7, -10.0, 36.6, 1.0 / 60.0);
        assert!(dt < 0.0);
        let cls = ThermalContactEffect::classify(dt);
        assert!(matches!(cls, ThermalContactEffect::Freezing));
    }

    #[test]
    fn baseline_concrete_does_nothing() {
        let dt = body_temp_delta_per_tick(0.3, 20.0, 36.6, 1.0 / 60.0);
        // 20 - 36.6 = -16.6 °C diff × 0.3 cond × 0.04 area × 0.0167 dt × 5
        // ≈ -0.017 → ~Cooling (below freezing threshold)
        assert!(dt < 0.0);
    }
}
