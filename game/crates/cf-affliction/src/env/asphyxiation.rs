//! M16A § asphyxiation — halon dump / CO2 flood without breathing apparatus.
//! Accumulator: `asphyxia_seconds` (1.0/tick while in low-O2 ambient).
//! Clear: return to breathable atmosphere.

use super::{
    check_origin_immune, emit_clear, evaluate_threshold, AtmosphericSusceptibility,
    EnvAfflictionKind, EnvAfflictionSpec, EnvAfflictionState, EnvClearReason, EnvSignal, EnvTickOutput,
};

pub const BREATHABLE_O2_KPA: f32 = 16.0;

pub fn tick(
    state: &mut EnvAfflictionState,
    actor_id: u64,
    susceptibility: &AtmosphericSusceptibility,
    signal: &EnvSignal,
    spec: &EnvAfflictionSpec,
    dt_seconds: f32,
    source_event_id: Option<String>,
    out: &mut EnvTickOutput,
) {
    let kind = EnvAfflictionKind::Asphyxiation;
    if susceptibility.oxygen_toxic {
        let alt = Some(EnvAfflictionKind::RefrigerantInhalation);
        if check_origin_immune(
            state,
            actor_id,
            kind,
            susceptibility,
            0.0,
            "oxygen_toxic_origin",
            alt,
            out,
        ) {
            return;
        }
    }
    if !susceptibility.asphyxiation_ttd_s.is_finite() {
        check_origin_immune(
            state,
            actor_id,
            kind,
            susceptibility,
            0.0,
            "atmospheric_immune_origin",
            None,
            out,
        );
        return;
    }
    let low_o2 = signal.o2_partial_kpa > 0.0 && signal.o2_partial_kpa < BREATHABLE_O2_KPA;
    let was_active = state.severity(kind) > 0.0
        || state.last_threshold(kind) != super::EnvSeverity::None;
    let mut acc = state.accumulator(kind);
    if low_o2 {
        let rate = spec.accumulator_rate_per_s * (30.0 / susceptibility.asphyxiation_ttd_s).max(0.1);
        acc.kind_value += rate * dt_seconds;
    } else if acc.kind_value > 0.0 {
        acc.kind_value = (acc.kind_value - spec.decay_per_s * dt_seconds).max(0.0);
    }
    state.set_accumulator(kind, acc);
    let severity = evaluate_threshold(
        state,
        actor_id,
        kind,
        acc.kind_value,
        spec,
        susceptibility.origin_id,
        source_event_id,
        out,
    );
    if severity > 0.0 && acc.kind_value >= spec.mild_threshold {
        out.hp_damage += spec.hp_per_second_at_threshold * severity * dt_seconds;
        out.aim_wobble_multiplier *= 1.0 + (spec.aim_wobble_multiplier - 1.0) * severity;
    }
    if !low_o2 && acc.kind_value <= 0.0 && was_active {
        emit_clear(state, actor_id, kind, EnvClearReason::ConditionCleared, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{EnvAfflictionRegistry, EnvSeverity, OriginId};

    fn human() -> AtmosphericSusceptibility {
        AtmosphericSusceptibility::for_origin(OriginId::Human)
    }

    #[test]
    fn asphyxiation_reaches_mild_in_30s() {
        let reg = EnvAfflictionRegistry::default_registry();
        let spec = reg.lookup(EnvAfflictionKind::Asphyxiation);
        let mut state = EnvAfflictionState::default();
        let signal = EnvSignal {
            o2_partial_kpa: 10.0,
            ..Default::default()
        };
        let mut out = EnvTickOutput::default();
        out.speed_multiplier = 1.0;
        out.aim_wobble_multiplier = 1.0;
        out.stamina_drain_multiplier = 1.0;
        for _ in 0..30 {
            tick(&mut state, 1, &human(), &signal, &spec, 1.0, None, &mut out);
        }
        assert!(state.accumulator(EnvAfflictionKind::Asphyxiation).kind_value >= 30.0);
        assert!(out
            .threshold_crossed
            .iter()
            .any(|e| matches!(e.severity, EnvSeverity::Mild | EnvSeverity::Moderate | EnvSeverity::Severe | EnvSeverity::Lethal)));
    }

    #[test]
    fn asphyxiation_reaches_lethal_at_90s() {
        let reg = EnvAfflictionRegistry::default_registry();
        let spec = reg.lookup(EnvAfflictionKind::Asphyxiation);
        let mut state = EnvAfflictionState::default();
        let signal = EnvSignal {
            o2_partial_kpa: 10.0,
            ..Default::default()
        };
        let mut out = EnvTickOutput::default();
        out.speed_multiplier = 1.0;
        out.aim_wobble_multiplier = 1.0;
        out.stamina_drain_multiplier = 1.0;
        for _ in 0..90 {
            tick(&mut state, 1, &human(), &signal, &spec, 1.0, None, &mut out);
        }
        assert!(state.accumulator(EnvAfflictionKind::Asphyxiation).kind_value >= 90.0);
        assert!(out
            .threshold_crossed
            .iter()
            .any(|e| matches!(e.severity, EnvSeverity::Lethal)));
    }

    #[test]
    fn methane_breather_immune_with_alt_kind() {
        let reg = EnvAfflictionRegistry::default_registry();
        let spec = reg.lookup(EnvAfflictionKind::Asphyxiation);
        let methane = AtmosphericSusceptibility::for_origin(OriginId::MethaneBreather);
        let mut state = EnvAfflictionState::default();
        let signal = EnvSignal {
            o2_partial_kpa: 10.0,
            ..Default::default()
        };
        let mut out = EnvTickOutput::default();
        out.speed_multiplier = 1.0;
        out.aim_wobble_multiplier = 1.0;
        out.stamina_drain_multiplier = 1.0;
        tick(&mut state, 1, &methane, &signal, &spec, 1.0, None, &mut out);
        let ev = out
            .origin_immune
            .iter()
            .find(|e| e.kind == EnvAfflictionKind::Asphyxiation)
            .expect("methane breather emits origin_immune");
        assert_eq!(ev.reason, "oxygen_toxic_origin");
        assert_eq!(ev.alt_kind, Some(EnvAfflictionKind::RefrigerantInhalation));
    }
}
