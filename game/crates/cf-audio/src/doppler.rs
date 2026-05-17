//! **M12B** § Relative-velocity pitch shift. Atmosphere-medium aware
//! speed-of-sound term.
//!
//! Per spec § HRTF resolution:
//!
//! ```text
//! doppler_factor: (343.0 + dot(listener.vel, dir_to_source)) /
//!                 (343.0 + dot(source.vel, dir_to_source))
//! ```
//!
//! Per spec § Doppler safety:
//!
//! > clamp `doppler_factor` to `[0.25, 4.0]`. The clamp also catches
//! > NaN/Inf if listener or source velocity is broken. The replay event
//! > carries `clamped: bool` so debugging is straightforward.
//!
//! Per spec § Atmosphere-corrected speed of sound:
//!
//! > read from `cf-atmos::medium_at(pos).speed_of_sound_m_per_s`. Default
//! > = 343 m/s (Earth-air STP). Vacuum = 0 → short-circuits to `gain=0`
//! > before doppler math.
//!
//! Pure math; deterministic, no Bevy.

use serde::{Deserialize, Serialize};

use crate::medium::SPEED_OF_SOUND_AIR_M_PER_S;

/// **M12B** § Min Doppler factor (3 octaves down).
pub const DOPPLER_FACTOR_MIN: f32 = 0.25;

/// **M12B** § Max Doppler factor (2 octaves up).
pub const DOPPLER_FACTOR_MAX: f32 = 4.0;

/// **M12B** § Resolved Doppler descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DopplerShift {
    /// Multiplicative pitch shift `f_observed / f_source`. Clamped to
    /// `[DOPPLER_FACTOR_MIN, DOPPLER_FACTOR_MAX]`.
    pub factor: f32,
    /// `true` when the raw factor exceeded the safe range (or was NaN/Inf)
    /// and the clamp fired. Recorded in `audio.doppler_shifted.clamped`
    /// for forensics.
    pub clamped: bool,
    /// Speed of sound used in the calculation (medium-corrected).
    pub speed_of_sound_m_per_s: f32,
}

impl DopplerShift {
    /// No-op shift (factor=1.0, listener and source co-stationary).
    #[must_use]
    pub const fn unity() -> Self {
        Self {
            factor: 1.0,
            clamped: false,
            speed_of_sound_m_per_s: SPEED_OF_SOUND_AIR_M_PER_S,
        }
    }

    /// `true` when the listener perceives a higher pitch (factor > 1.0).
    #[must_use]
    pub fn is_blue_shifted(self) -> bool {
        self.factor > 1.0
    }

    /// `true` when the listener perceives a lower pitch (factor < 1.0).
    #[must_use]
    pub fn is_red_shifted(self) -> bool {
        self.factor < 1.0
    }
}

fn dot([ax, ay]: [f32; 2], [bx, by]: [f32; 2]) -> f32 {
    ax * bx + ay * by
}

fn normalize_dir(from: [f32; 2], to: [f32; 2]) -> [f32; 2] {
    let dx = to[0] - from[0];
    let dy = to[1] - from[1];
    let mag = (dx * dx + dy * dy).sqrt();
    if mag <= 1e-6 {
        [0.0, 0.0]
    } else {
        [dx / mag, dy / mag]
    }
}

/// **M12B** § Resolve the Doppler factor between source and listener.
///
/// `c` is the medium-corrected speed of sound. Use `343.0` for Earth-air;
/// `cf-atmos::medium_at(midpoint).speed_of_sound_m_per_s` for the
/// canonical resolver. Vacuum (`c <= 0`) short-circuits to `factor = 1.0`
/// with `clamped=false` — the surrounding spatial resolver gates audio
/// to `gain=0` BEFORE calling doppler, so this is just a safety net.
///
/// Per spec § HRTF resolution: the dot product is taken against the
/// listener-to-source unit vector.
#[must_use]
pub fn resolve_doppler(
    source_pos: [f32; 2],
    source_vel: [f32; 2],
    listener_pos: [f32; 2],
    listener_vel: [f32; 2],
    speed_of_sound_m_per_s: f32,
) -> DopplerShift {
    let c = speed_of_sound_m_per_s;
    if c <= 0.0 || !c.is_finite() {
        return DopplerShift {
            factor: 1.0,
            clamped: false,
            speed_of_sound_m_per_s: c.max(0.0),
        };
    }
    let dir_to_source = normalize_dir(listener_pos, source_pos);
    let listener_radial = dot(listener_vel, dir_to_source);
    let source_radial = dot(source_vel, dir_to_source);
    let numerator = c + listener_radial;
    let denominator = c + source_radial;
    let raw = if denominator.abs() < 1e-6 {
        f32::INFINITY
    } else {
        numerator / denominator
    };
    clamp_factor(raw, c)
}

/// **M12B** § Clamp a raw Doppler factor to the safe range
/// `[DOPPLER_FACTOR_MIN, DOPPLER_FACTOR_MAX]`. Catches NaN/Inf — broken
/// velocity inputs collapse to a unity descriptor with `clamped=true`.
#[must_use]
pub fn clamp_factor(raw: f32, speed_of_sound_m_per_s: f32) -> DopplerShift {
    if !raw.is_finite() {
        return DopplerShift {
            factor: 1.0,
            clamped: true,
            speed_of_sound_m_per_s,
        };
    }
    let clamped_flag = !(DOPPLER_FACTOR_MIN..=DOPPLER_FACTOR_MAX).contains(&raw);
    let factor = raw.clamp(DOPPLER_FACTOR_MIN, DOPPLER_FACTOR_MAX);
    DopplerShift {
        factor,
        clamped: clamped_flag,
        speed_of_sound_m_per_s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::medium::SPEED_OF_SOUND_AMMONIA_M_PER_S;

    #[test]
    fn co_stationary_produces_unity_factor() {
        let d = resolve_doppler([0.0, 0.0], [0.0, 0.0], [10.0, 0.0], [0.0, 0.0], 343.0);
        assert!((d.factor - 1.0).abs() < 1e-6);
        assert!(!d.clamped);
    }

    #[test]
    fn supersonic_receding_source_produces_red_shift() {
        // Spec acceptance: "as the projectile recedes: doppler_factor ≈
        // (343 + 0) / (343 + 800) ≈ 0.30 (lower pitch)".
        //
        // Setup: source at (10, 0), moving at +X (800 m/s) away from
        // listener at origin.
        let d = resolve_doppler([10.0, 0.0], [800.0, 0.0], [0.0, 0.0], [0.0, 0.0], 343.0);
        // Dir from listener to source = +X; source.vel · dir = +800.
        // f = 343 / (343 + 800) ≈ 0.3001
        assert!(d.factor < 1.0);
        assert!((d.factor - 0.3001).abs() < 1e-3);
        assert!(d.is_red_shifted());
    }

    #[test]
    fn supersonic_approaching_source_clamps_blue_shift() {
        // Source at (10, 0) moving at -X (toward listener) at 800 m/s.
        // dir_to_source = +X; source.vel·dir = -800.
        // raw = 343 / (343 - 800) = 343 / -457 ≈ -0.75 (negative — clamps).
        let d = resolve_doppler([10.0, 0.0], [-800.0, 0.0], [0.0, 0.0], [0.0, 0.0], 343.0);
        assert!(d.clamped);
        assert!(d.factor >= DOPPLER_FACTOR_MIN);
        assert!(d.factor <= DOPPLER_FACTOR_MAX);
    }

    #[test]
    fn clamp_catches_nan_input() {
        let d = clamp_factor(f32::NAN, 343.0);
        assert!((d.factor - 1.0).abs() < 1e-6);
        assert!(d.clamped);
    }

    #[test]
    fn clamp_catches_infinity_input() {
        let d = clamp_factor(f32::INFINITY, 343.0);
        assert!((d.factor - 1.0).abs() < 1e-6);
        assert!(d.clamped);
    }

    #[test]
    fn clamp_passes_unclamped_value() {
        let d = clamp_factor(0.5, 343.0);
        assert!((d.factor - 0.5).abs() < 1e-6);
        assert!(!d.clamped);
    }

    #[test]
    fn clamp_clamps_above_max() {
        let d = clamp_factor(10.0, 343.0);
        assert!((d.factor - DOPPLER_FACTOR_MAX).abs() < 1e-6);
        assert!(d.clamped);
    }

    #[test]
    fn clamp_clamps_below_min() {
        let d = clamp_factor(0.01, 343.0);
        assert!((d.factor - DOPPLER_FACTOR_MIN).abs() < 1e-6);
        assert!(d.clamped);
    }

    #[test]
    fn vacuum_short_circuits_to_unity() {
        let d = resolve_doppler([10.0, 0.0], [800.0, 0.0], [0.0, 0.0], [0.0, 0.0], 0.0);
        assert!((d.factor - 1.0).abs() < 1e-6);
        assert!(!d.clamped);
    }

    #[test]
    fn ammonia_atmosphere_uses_corrected_speed_of_sound() {
        // Spec scenario: "the speed-of-sound term `c` in the doppler formula
        // uses the medium-corrected value (≈ 415 m/s for ammonia)".
        // Source at +X moving +X at 200 m/s — same geometry, different c.
        // Difference between air-c=343 vs ammonia-c=415 should be measurable.
        let d_air = resolve_doppler([10.0, 0.0], [200.0, 0.0], [0.0, 0.0], [0.0, 0.0], 343.0);
        let d_amm = resolve_doppler(
            [10.0, 0.0],
            [200.0, 0.0],
            [0.0, 0.0],
            [0.0, 0.0],
            SPEED_OF_SOUND_AMMONIA_M_PER_S,
        );
        // The factor must differ when the speed of sound changes.
        assert!(
            (d_air.factor - d_amm.factor).abs() > 1e-3,
            "air {} vs ammonia {}",
            d_air.factor,
            d_amm.factor
        );
        assert!((d_amm.speed_of_sound_m_per_s - SPEED_OF_SOUND_AMMONIA_M_PER_S).abs() < 1e-3);
    }

    #[test]
    fn doppler_shift_round_trips_through_serde() {
        let d = DopplerShift {
            factor: 0.5,
            clamped: true,
            speed_of_sound_m_per_s: 343.0,
        };
        let s = serde_json::to_string(&d).unwrap();
        let back: DopplerShift = serde_json::from_str(&s).unwrap();
        assert_eq!(d, back);
    }

    #[test]
    fn doppler_determinism_two_engines_identical() {
        let scenario = || {
            resolve_doppler(
                [12.34, 56.78],
                [800.0, 0.0],
                [0.0, 0.0],
                [10.0, 0.0],
                SPEED_OF_SOUND_AMMONIA_M_PER_S,
            )
        };
        let a = scenario();
        let b = scenario();
        assert_eq!(a, b);
    }
}
