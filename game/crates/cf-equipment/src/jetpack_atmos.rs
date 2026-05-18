//! **M14A** § "Jet thrust × atmospheric pressure efficiency".
//!
//! Wrapper helpers around `jetpack::jet_pressure_efficiency` for use in
//! engine integration / observe surfaces. Kept narrow so the cf-equipment
//! consumer surface stays focused on per-jetpack scaling.

use crate::jetpack::jet_pressure_efficiency;

/// **M14A** § "Atmospheric efficiency" — multiplier on jet thrust by ambient
/// pressure. Linear between three anchors: vacuum ×1.5, Earth ×1.0, Venus ×0.5.
pub fn pressure_modulated_thrust(base_thrust_n: f32, ambient_pressure_kpa: f32) -> f32 {
    base_thrust_n * jet_pressure_efficiency(ambient_pressure_kpa)
}

/// **M14A** § "Combustion atmosphere gating" — true when the local
/// atmosphere supports muzzle-flash ignition per Stationeers.
pub fn muzzle_flash_combusts(
    volatiles_partial_kpa: f32,
    o2_partial_kpa: f32,
    total_pressure_kpa: f32,
    temp_k: f32,
) -> bool {
    let total = total_pressure_kpa.max(1e-3);
    let vol_ratio = volatiles_partial_kpa / total;
    let o2_ratio = o2_partial_kpa / total;
    if total < cf_atmos::MIN_TOTAL_PRESSURE_FOR_IGNITION_KPA {
        return false;
    }
    if vol_ratio < cf_atmos::MIN_FUEL_RATIO_FOR_IGNITION {
        return false;
    }
    if o2_ratio < cf_atmos::MIN_OXIDIZER_RATIO_FOR_IGNITION {
        return false;
    }
    temp_k >= cf_atmos::VOLATILES_AUTOIGNITE_K
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vacuum_boosts_thrust() {
        let t = pressure_modulated_thrust(1000.0, 0.5);
        assert!((t - 1500.0).abs() < 1.0);
    }

    #[test]
    fn earth_baseline() {
        let t = pressure_modulated_thrust(1000.0, 101.0);
        assert!((t - 1000.0).abs() < 5.0);
    }

    #[test]
    fn muzzle_ignites_in_combustible_atmosphere() {
        // 10 kPa Volatiles + 21 kPa O2 + 101 kPa total + T=600 K.
        assert!(muzzle_flash_combusts(10.0, 21.0, 101.0, 600.0));
        // Cold atmosphere → no ignition.
        assert!(!muzzle_flash_combusts(10.0, 21.0, 101.0, 300.0));
        // Vacuum → no ignition.
        assert!(!muzzle_flash_combusts(0.5, 0.1, 1.0, 600.0));
    }
}
