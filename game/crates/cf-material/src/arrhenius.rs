//! M15D § Arrhenius rate evaluator.
//!
//! Per-tick rate per the spec literal:
//! `k_eff = k * exp(-Ea / (R · T))`
//!
//! Pure / deterministic / mass-conserving. Unit-tested at T ∈
//! {273, 500, 1000, 2000} K per spec § "Notes for implementer".
//!
//! `R` is the universal gas constant in kJ/(mol·K), `8.314e-3`. Both
//! `Ea` and `R` are kJ-scaled so the exponent is dimensionless.

/// Universal gas constant in kJ / (mol · K). M15D spec uses Ea in
/// kJ/mol so R must also be in kJ — otherwise the exponent blows up by
/// 1000×.
pub const GAS_CONSTANT_R_KJ_PER_MOL_K: f32 = 8.314e-3;

/// Arrhenius rate constant in events/sec.
///
/// `rate_constant_per_s` is the pre-exponential factor `A`;
/// `activation_energy_kj_per_mol` is `Ea`; `temperature_k` is `T`.
/// Returns 0 for non-positive `T` so the evaluator never panics on a
/// vacuum / pre-init scenario.
#[must_use]
pub fn arrhenius_rate(
    rate_constant_per_s: f32,
    activation_energy_kj_per_mol: f32,
    temperature_k: f32,
) -> f32 {
    if temperature_k <= 0.0 {
        return 0.0;
    }
    let exponent = -activation_energy_kj_per_mol / (GAS_CONSTANT_R_KJ_PER_MOL_K * temperature_k);
    rate_constant_per_s * exponent.exp()
}

/// Per-tick variant: returns events per tick at the given tick rate.
/// `tick_hz` defaults to 60 in M15 but M15D allows any positive tick
/// rate.
#[must_use]
pub fn arrhenius_rate_per_tick(
    rate_constant_per_s: f32,
    activation_energy_kj_per_mol: f32,
    temperature_k: f32,
    tick_hz: f32,
) -> f32 {
    if tick_hz <= 0.0 {
        return 0.0;
    }
    arrhenius_rate(rate_constant_per_s, activation_energy_kj_per_mol, temperature_k) / tick_hz
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32, rel: f32) -> bool {
        if a == b {
            return true;
        }
        let denom = a.abs().max(b.abs()).max(1e-30);
        (a - b).abs() / denom <= rel
    }

    #[test]
    fn arrhenius_zero_temperature_returns_zero() {
        assert_eq!(arrhenius_rate(1.0, 50.0, 0.0), 0.0);
        assert_eq!(arrhenius_rate(1.0, 50.0, -1.0), 0.0);
    }

    #[test]
    fn arrhenius_zero_activation_returns_pre_exponential() {
        assert!(approx(arrhenius_rate(0.5, 0.0, 500.0), 0.5, 1e-6));
    }

    #[test]
    fn arrhenius_higher_temperature_yields_higher_rate() {
        let cold = arrhenius_rate(1.0, 50.0, 500.0);
        let hot = arrhenius_rate(1.0, 50.0, 1000.0);
        assert!(hot > cold, "rate at 1000K ({hot}) must exceed rate at 500K ({cold})");
    }

    /// Spec § Gherkin: "k_eff at 1200 K is ≥ 10x k_eff at 600 K" for a
    /// methane combustion-grade Ea (60 kJ/mol).
    #[test]
    fn arrhenius_methane_combustion_temperature_ladder() {
        let ea = 60.0;
        let k = 0.9;
        let cold = arrhenius_rate(k, ea, 600.0);
        let hot = arrhenius_rate(k, ea, 1200.0);
        assert!(
            hot >= 10.0 * cold,
            "k_eff(1200K) must be >= 10x k_eff(600K): hot={hot}, cold={cold}"
        );
    }

    /// canonical sentinel temperatures (273, 500, 1000, 2000 K) is
    /// finite + monotonic for an exothermic reaction with Ea=50 kJ/mol.
    #[test]
    fn arrhenius_sentinel_temperatures_finite_monotonic() {
        let k = 1.0;
        let ea = 50.0;
        let temps = [273.0, 500.0, 1000.0, 2000.0];
        let rates: Vec<f32> = temps.iter().map(|&t| arrhenius_rate(k, ea, t)).collect();
        for r in &rates {
            assert!(r.is_finite(), "rate must be finite, got {r}");
            assert!(*r >= 0.0, "rate must be non-negative, got {r}");
        }
        for w in rates.windows(2) {
            assert!(
                w[1] >= w[0],
                "rate must be monotonic-non-decreasing across temperature: {} -> {}",
                w[0],
                w[1]
            );
        }
    }

    #[test]
    fn arrhenius_per_tick_scales_by_tick_rate() {
        let per_s = arrhenius_rate(1.0, 30.0, 800.0);
        let per_tick = arrhenius_rate_per_tick(1.0, 30.0, 800.0, 60.0);
        assert!(approx(per_tick * 60.0, per_s, 1e-5));
    }

    #[test]
    fn arrhenius_per_tick_zero_hz_returns_zero() {
        assert_eq!(arrhenius_rate_per_tick(1.0, 30.0, 800.0, 0.0), 0.0);
        assert_eq!(arrhenius_rate_per_tick(1.0, 30.0, 800.0, -1.0), 0.0);
    }
}
