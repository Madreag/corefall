//! M17 — robot internal-shock dose (the synthetic-origin concussion analogue).
//!
//! Robots / drones / crystalline don't stack a concussion dose; instead each
//! impact past an impulse threshold both bumps an `internal_shock_dose`
//! accumulator (0-100, decays 2/s) AND rolls damage onto a random internal
//! module. This module owns the pure dose math + the impulse gate; the module
//! roll itself lives in `cf-control` (which owns the `cf_internal` graph).

/// Impulse (N·s) above which an impact rolls onto an internal module
/// (spec § "Each impact > impulse_threshold rolls damage onto random module").
pub const INTERNAL_SHOCK_IMPULSE_THRESHOLD_N_S: f32 = 4.0;

/// Fraction of incoming damage that converts into internal-shock dose.
pub const INTERNAL_SHOCK_DOSE_PER_DAMAGE: f32 = 0.6;

/// Does this impact arm an internal-shock module roll? Gated on impulse.
pub fn impulse_arms_internal_shock(impulse_n_s: f32) -> bool {
    impulse_n_s >= INTERNAL_SHOCK_IMPULSE_THRESHOLD_N_S
}

/// Add an impact's contribution to the internal-shock dose (clamped 0-100).
pub fn accrue_dose(prev_dose: f32, damage: f32) -> f32 {
    (prev_dose + damage * INTERNAL_SHOCK_DOSE_PER_DAMAGE).clamp(0.0, 100.0)
}

/// Decay the dose toward zero at `rate_per_s` over `dt_seconds`.
pub fn decay_dose(dose: f32, rate_per_s: f32, dt_seconds: f32) -> f32 {
    (dose - rate_per_s * dt_seconds).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impulse_gate() {
        assert!(!impulse_arms_internal_shock(3.9));
        assert!(impulse_arms_internal_shock(4.0));
        assert!(impulse_arms_internal_shock(50.0));
    }

    #[test]
    fn dose_accrues_and_clamps() {
        let d = accrue_dose(0.0, 50.0);
        assert!((d - 30.0).abs() < 1e-4);
        assert_eq!(accrue_dose(90.0, 100.0), 100.0);
    }

    #[test]
    fn dose_decays_to_zero() {
        let d = decay_dose(10.0, 2.0, 1.0);
        assert!((d - 8.0).abs() < 1e-6);
        assert_eq!(decay_dose(1.0, 2.0, 1.0), 0.0);
    }
}
