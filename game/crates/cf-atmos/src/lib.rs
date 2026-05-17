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

use cf_audio::{Medium, MediumFilter};

pub use room::{reverb_profile, RoomAtmosphere};

/// **M12B** § Per-tile medium probe. M19F humidity + condensation and
/// M19G room ↔ tile bridge populate the live atmospheric medium; until
/// those land cf-atmos returns [`Medium::Air`] for every position. The
/// spatial-resolve pipeline calls this at the midpoint of (source,
/// listener) per spec § HRTF resolution.
///
/// Per spec § Notes:
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

/// **M12B** § Override-friendly variant of [`medium_at`] for test
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
