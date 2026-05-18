//! **M14A** § "Per-tick atmosphere contact" — wind lateral force,
//! pressure-scaled jet efficiency, O2 partial → suit life-support,
//! temperature → thermal effects, combustion atmosphere → muzzle ignition.

use serde::{Deserialize, Serialize};

use crate::AtmosphereSample;

/// Outcome of one atmosphere-contact tick.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct AtmosphereContact {
    /// Wind lateral force vector in N (chassis-mass-aware caller scales by mass).
    pub wind_force_n: [f32; 2],
    /// Hypoxia severity (0..3).
    pub hypoxia_severity: u8,
    /// True when local pressure is below survivable suit minimum.
    pub decompression_risk: bool,
    /// True when ignition would happen for a muzzle flash this tick.
    pub combustion_ready: bool,
    /// Local pressure in kPa (echo for HUD).
    pub pressure_kpa: f32,
    /// Local temperature in °C (echo for HUD).
    pub temp_c: f32,
}

/// Apply wind lateral force per CCCP's drag model. Returns the wind force
/// vector in N (callers convert via F=ma).
pub fn wind_force_for_actor(atm: &AtmosphereSample, actor_h_extent_x: f32) -> [f32; 2] {
    let pressure_factor = (atm.pressure_kpa / 101.0).clamp(0.0, 4.0);
    let drag_area = actor_h_extent_x.max(1.0) * 2.0; // proxy for cross-sectional area
    let force_x = atm.wind[0] * pressure_factor * drag_area * 0.02;
    let force_y = atm.wind[1] * pressure_factor * drag_area * 0.02;
    [force_x, force_y]
}

/// **M14A** § "Stationeers helmet breach math" — `inhaled_mol_per_tick =
/// 0.0048 · BreathingRate · BreathingEfficiency`.
pub fn suit_o2_drain_mol_per_tick(breathing_rate: f32, breathing_efficiency: f32) -> f32 {
    cf_atmos::INHALED_MOL_PER_TICK_BASE * breathing_rate.max(0.0) * breathing_efficiency.clamp(0.0, 2.0)
}

/// Resolve one tick's atmosphere contact for an actor.
pub fn resolve_atmosphere_contact(atm: &AtmosphereSample, actor_h_extent_x: f32) -> AtmosphereContact {
    AtmosphereContact {
        wind_force_n: wind_force_for_actor(atm, actor_h_extent_x),
        hypoxia_severity: atm.hypoxia_severity(),
        decompression_risk: atm.pressure_kpa < cf_atmos::SUIT_PRESSURE_MIN_KPA,
        combustion_ready: atm.supports_combustion(),
        pressure_kpa: atm.pressure_kpa,
        temp_c: atm.temp_k - 273.15,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wind_force_scales_with_pressure() {
        let mut atm = AtmosphereSample::default();
        atm.wind = [10.0, 0.0];
        let earth = wind_force_for_actor(&atm, 8.0);
        atm.pressure_kpa = 2.5;
        let mars = wind_force_for_actor(&atm, 8.0);
        assert!(earth[0].abs() > mars[0].abs());
    }

    #[test]
    fn vacuum_yields_decompression_risk() {
        let mut atm = AtmosphereSample::default();
        atm.pressure_kpa = 0.5;
        let c = resolve_atmosphere_contact(&atm, 8.0);
        assert!(c.decompression_risk);
    }

    #[test]
    fn suit_o2_drain_uses_stationeers_constants() {
        let mol = suit_o2_drain_mol_per_tick(2.0, 1.0);
        // 0.0048 * 2 * 1 = 0.0096 mol/tick
        assert!((mol - 0.0096).abs() < 1e-6);
    }
}
