//! **M14A** § "Simulation overlay glue" — per-tick sim-overlay sampling.
//!
//! Reads cf-atmos + cf-terrain at the planted-foot position and applies
//! modifiers to walk_speed, friction, attitude, mass, and resources.

use serde::{Deserialize, Serialize};

use crate::AtmosphereSample;

/// **M14A** § "Atmospheric overlay walk-speed modifiers".
pub const WALK_SPEED_HYPOXIA_MULT: f32 = 0.85;
pub const WALK_SPEED_HYPERTHERMIC_MULT: f32 = 0.9;
pub const WALK_SPEED_HYPOTHERMIC_MULT: f32 = 0.75;
pub const WALK_SPEED_TOXIC_STAMINA_MULT: f32 = 2.0;

/// Outcome of one sim-overlay tick.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct OverlayOutcome {
    /// Walk-speed multiplier from atmosphere (hypoxia / heat / cold).
    pub atmosphere_speed_mult: f32,
    /// Stamina drain multiplier from atmosphere (toxic).
    pub atmosphere_stamina_mult: f32,
    /// Hypoxia severity (0..3).
    pub hypoxia_severity: u8,
    /// `true` when the actor is below a low-g threshold (DR-038).
    pub low_g: bool,
    /// Local pressure (kPa) — surface for jet efficiency.
    pub pressure_kpa: f32,
    /// Local wind vector — surface for chassis lateral force.
    pub wind: [f32; 2],
    /// `true` when the local atmosphere supports muzzle-flash ignition.
    pub combustion_ready: bool,
}

/// Sample + compute the overlay outcome for a position. Today this is a
/// pure deterministic projection of the atmosphere sample; once cf-atmos
/// is wired into the engine the caller passes the real sample.
pub fn compute_overlay(atm: AtmosphereSample) -> OverlayOutcome {
    let hypoxia_severity = atm.hypoxia_severity();
    let mut speed = 1.0;
    if hypoxia_severity >= 2 {
        speed *= WALK_SPEED_HYPOXIA_MULT;
    }
    let temp_c = atm.temp_k - 273.15;
    if temp_c > 49.0 {
        speed *= WALK_SPEED_HYPERTHERMIC_MULT;
    } else if temp_c < 0.0 {
        speed *= WALK_SPEED_HYPOTHERMIC_MULT;
    }
    let stamina_mult = if atm.pollutant_partial_kpa > 0.1 {
        WALK_SPEED_TOXIC_STAMINA_MULT
    } else {
        1.0
    };
    OverlayOutcome {
        atmosphere_speed_mult: speed,
        atmosphere_stamina_mult: stamina_mult,
        hypoxia_severity,
        low_g: atm.local_gravity_m_s2 < 4.9,
        pressure_kpa: atm.pressure_kpa,
        wind: atm.wind,
        combustion_ready: atm.supports_combustion(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earth_ambient_yields_no_modifier() {
        let out = compute_overlay(AtmosphereSample::default());
        assert!((out.atmosphere_speed_mult - 1.0).abs() < 1e-6);
        assert_eq!(out.hypoxia_severity, 0);
        assert!(!out.low_g);
    }

    #[test]
    fn hypoxic_atmosphere_triggers_speed_drop_and_warning() {
        let mut atm = AtmosphereSample::default();
        atm.o2_partial_kpa = 10.0; // below 12 kPa critical
        let out = compute_overlay(atm);
        assert_eq!(out.hypoxia_severity, 2);
        assert!((out.atmosphere_speed_mult - WALK_SPEED_HYPOXIA_MULT).abs() < 1e-6);
    }

    #[test]
    fn hot_atmosphere_drops_walk_speed() {
        let mut atm = AtmosphereSample::default();
        atm.temp_k = 350.0; // 76 °C
        let out = compute_overlay(atm);
        assert!((out.atmosphere_speed_mult - WALK_SPEED_HYPERTHERMIC_MULT).abs() < 1e-6);
    }

    #[test]
    fn low_g_cell_flagged() {
        let mut atm = AtmosphereSample::default();
        atm.local_gravity_m_s2 = 3.0;
        let out = compute_overlay(atm);
        assert!(out.low_g);
    }

    #[test]
    fn combustion_ready_when_volatiles_present() {
        let atm = AtmosphereSample {
            pressure_kpa: 101.0,
            temp_k: 600.0, // > 573 K ignition threshold
            o2_partial_kpa: 21.0,
            volatiles_partial_kpa: 10.0, // > 5% of 101 kPa
            pollutant_partial_kpa: 0.0,
            smoke_pct: 0.0,
            wind: [0.0, 0.0],
            local_gravity_m_s2: 9.8,
        };
        let out = compute_overlay(atm);
        assert!(out.combustion_ready);
    }
}
