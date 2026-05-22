//! cf-atmos — atmospherics-grade kernel.
//!
//! Scaffolded by M0-001; real implementation lands in M7.5 (DR-036 / T-MAT)
//! + M19 atmospherics-grade kernel.
//!
//! M12B (2026-05-17) introduces the [`room`] submodule — the per-room
//! reverb derivation bridge that joins cf-atmos's room geometry with
//! `cf-audio::ReverbProfile` derivation, plus the [`medium_at`] medium
//! probe consumed by the spatial-resolve pipeline. Per M12B spec §
//! Crates / modules touched: "MODIFY: Expose `reverb_profile(room_id) ->
//! ReverbProfile` derived from `volume_m3` + wall_material_distribution.".
//! Per M12B spec § Notes for the implementer: "Atmosphere-corrected
//! speed of sound: read from `cf-atmos::medium_at(pos).speed_of_sound_m_per_s`.
//! Default = 343 m/s (Earth-air STP). Vacuum = 0 → short-circuits to
//! `gain=0` before doppler math.".

pub mod room;
pub mod stratification;
pub mod wind;

use cf_audio::{Medium, MediumFilter};
use serde::{Deserialize, Serialize};

pub use room::{reverb_profile, RoomAtmosphere};
pub use stratification::{stratify, stratify_if_due, Gas, StratCell, StratificationDelta, AIR_MOLAR_MASS_G_PER_MOL};
pub use wind::{
    buoyancy_lift_at, wind_force_at, wind_force_from_aperture, wind_force_with_buoyancy_at, AtmosCell,
    WindForceOutcome, WindSource, BUOYANCY_FORCE_PER_K_DELTA,
};

pub const IDEAL_GAS_CONSTANT_R: f32 = 8314.46;
pub const MIN_O2_PARTIAL_KPA: f32 = 16.0;
pub const CRITICAL_O2_PARTIAL_KPA: f32 = 12.0;
pub const DAMAGE_O2_PARTIAL_KPA: f32 = 5.0;
pub const SUIT_PRESSURE_MIN_KPA: f32 = 11.0;
pub const SUIT_PRESSURE_MAX_KPA: f32 = 300.0;
pub const SUIT_TEMP_MIN_C: f32 = -10.0;
pub const SUIT_TEMP_MAX_C: f32 = 49.0;
pub const INHALED_MOL_PER_TICK_BASE: f32 = 0.0048;
pub const EARTH_AMBIENT_KPA: f32 = 101.0;
pub const MARS_AMBIENT_KPA: f32 = 2.5;
pub const VACUUM_AMBIENT_KPA: f32 = 0.01;
pub const VENUS_AMBIENT_KPA: f32 = 239.0;
pub const VOLATILES_AUTOIGNITE_K: f32 = 573.15;
pub const VOLATILES_AUTOIGNITE_N2O_K: f32 = 323.15;
pub const MIN_FUEL_RATIO_FOR_IGNITION: f32 = 0.05;
pub const MIN_OXIDIZER_RATIO_FOR_IGNITION: f32 = 0.05;
pub const MIN_TOTAL_PRESSURE_FOR_IGNITION_KPA: f32 = 10.0;
pub const WIND_FORCE_PER_KPA_DIFFERENTIAL: f32 = 2.0;
pub const PIPE_GAS_RUPTURE_KPA: f32 = 60_795.0;
pub const PIPE_LIQUID_RUPTURE_KPA: f32 = 6_079.0;

///
/// Stationeers-grade direction (DR-037): pressure / temperature / partial
/// pressures + wind vector + local gravity. Today the [`sample_cell`] stub
/// returns Earth defaults (or per-planet preset); M5.9 lands the real
/// PV=nRT kernel without changing the M14A consumer API.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AtmosphereSample {
    pub pressure_kpa: f32,
    pub temp_k: f32,
    pub o2_partial_kpa: f32,
    pub pollutant_partial_kpa: f32,
    pub volatiles_partial_kpa: f32,
    pub smoke_pct: f32,
    pub wind: [f32; 2],
    pub local_gravity_m_s2: f32,
}

impl AtmosphereSample {
    pub fn earth_ambient() -> Self {
        Self {
            pressure_kpa: EARTH_AMBIENT_KPA,
            temp_k: 293.15,
            o2_partial_kpa: 21.0,
            pollutant_partial_kpa: 0.0,
            volatiles_partial_kpa: 0.0,
            smoke_pct: 0.0,
            wind: [0.0, 0.0],
            local_gravity_m_s2: 9.8,
        }
    }

    pub fn mars_ambient() -> Self {
        Self {
            pressure_kpa: MARS_AMBIENT_KPA,
            temp_k: 210.0,
            o2_partial_kpa: 0.0,
            pollutant_partial_kpa: 0.0,
            volatiles_partial_kpa: 0.0,
            smoke_pct: 0.0,
            wind: [0.0, 0.0],
            local_gravity_m_s2: 3.7,
        }
    }

    pub fn vacuum() -> Self {
        Self {
            pressure_kpa: VACUUM_AMBIENT_KPA,
            temp_k: 100.0,
            o2_partial_kpa: 0.0,
            pollutant_partial_kpa: 0.0,
            volatiles_partial_kpa: 0.0,
            smoke_pct: 0.0,
            wind: [0.0, 0.0],
            local_gravity_m_s2: 1.6,
        }
    }

    pub fn venus_ambient() -> Self {
        Self {
            pressure_kpa: VENUS_AMBIENT_KPA,
            temp_k: 737.0,
            o2_partial_kpa: 0.0,
            pollutant_partial_kpa: 0.0,
            volatiles_partial_kpa: 0.0,
            smoke_pct: 0.0,
            wind: [0.0, 0.0],
            local_gravity_m_s2: 8.87,
        }
    }

    /// Returns the severity level for hypoxia warnings: 0 = none, 1 = yellow
    /// (O2 < 16 kPa), 2 = red (O2 < 12 kPa), 3 = damage (O2 < 5 kPa).
    pub fn hypoxia_severity(&self) -> u8 {
        if self.o2_partial_kpa < DAMAGE_O2_PARTIAL_KPA {
            3
        } else if self.o2_partial_kpa < CRITICAL_O2_PARTIAL_KPA {
            2
        } else if self.o2_partial_kpa < MIN_O2_PARTIAL_KPA {
            1
        } else {
            0
        }
    }

    /// True when the local atmosphere supports muzzle-flash ignition per
    /// Stationeers combustion table.
    pub fn supports_combustion(&self) -> bool {
        let total = self.pressure_kpa;
        if total < MIN_TOTAL_PRESSURE_FOR_IGNITION_KPA {
            return false;
        }
        let fuel_ratio = self.volatiles_partial_kpa / total.max(1e-3);
        let oxidizer_ratio = self.o2_partial_kpa / total.max(1e-3);
        if fuel_ratio < MIN_FUEL_RATIO_FOR_IGNITION {
            return false;
        }
        if oxidizer_ratio < MIN_OXIDIZER_RATIO_FOR_IGNITION {
            return false;
        }
        self.temp_k >= VOLATILES_AUTOIGNITE_K
    }
}

impl Default for AtmosphereSample {
    fn default() -> Self {
        Self::earth_ambient()
    }
}

/// world-space position. Today returns Earth-ambient defaults; M5.9 swaps
/// in the real PV=nRT kernel without changing this signature.
///
/// Callers (cf-actor::sim_overlay) pass world-space px and receive the DTO
/// to drive jet thrust scaling, hypoxia gating, wind lateral force, etc.
#[must_use]
pub fn sample_cell(_pos: [f32; 2]) -> AtmosphereSample {
    AtmosphereSample::earth_ambient()
}

/// Override-friendly variant for test scenarios. Closure receives the
/// world position and returns the sample directly.
#[must_use]
pub fn sample_cell_with<F>(pos: [f32; 2], probe: F) -> AtmosphereSample
where
    F: FnOnce([f32; 2]) -> AtmosphereSample,
{
    probe(pos)
}

/// M19G room ↔ tile bridge populate the live atmospheric medium; until
/// those land cf-atmos returns [`Medium::Air`] for every position. The
/// spatial-resolve pipeline calls this at the midpoint of (source,
/// listener) per spec § HRTF resolution.
///
///
/// > Atmosphere-corrected speed of sound: read from
/// > `cf-atmos::medium_at(pos).speed_of_sound_m_per_s`. Default = 343
/// > m/s (Earth-air STP). Vacuum = 0 → short-circuits to `gain=0`
/// > before doppler math.
///
/// The function is deterministic — no `thread_rng`, no frame-time
/// integration. M19F's real implementation will read from the per-tile
/// humidity grid; M19G will adjust for vacuum-breach events.
#[must_use]
pub fn medium_at(_pos: [f32; 2]) -> MediumFilter {
    MediumFilter::for_medium(Medium::Air)
}

/// scenarios. The closure receives the world position and returns the
/// medium; useful for headless unit tests of underwater + vacuum
/// scenarios before M19F lands.
#[must_use]
pub fn medium_at_with<F>(pos: [f32; 2], probe: F) -> MediumFilter
where
    F: FnOnce([f32; 2]) -> Medium,
{
    MediumFilter::for_medium(probe(pos))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn medium_at_defaults_to_air() {
        let f = medium_at([1.0, 2.0]);
        assert_eq!(f.medium, Medium::Air);
        assert!((f.speed_of_sound_m_per_s - 343.0).abs() < 1e-6);
    }

    #[test]
    fn medium_at_with_overrides_for_test_scenarios() {
        let f = medium_at_with([0.0, -1.0], |_pos| Medium::Water);
        assert_eq!(f.medium, Medium::Water);
        assert!((f.cutoff_hz - 800.0).abs() < 1e-3);
    }

    #[test]
    fn medium_at_with_vacuum_returns_zero_speed_of_sound() {
        let f = medium_at_with([0.0, 0.0], |_| Medium::Vacuum);
        assert!(f.is_silent());
        assert_eq!(f.speed_of_sound_m_per_s, 0.0);
    }
}
