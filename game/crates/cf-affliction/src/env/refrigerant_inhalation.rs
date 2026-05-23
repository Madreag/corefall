//! M16A § refrigerant_inhalation — R-22 / ammonia / CO2 supercritical leak.
//! Accumulator: `refrigerant_partial_pressure_seconds` (PP × dt).
//! Clear: leave room + decontaminate suit + 10 min cooldown.

use super::{
    check_origin_immune, emit_clear, evaluate_threshold, AtmosphericSusceptibility,
    EnvAfflictionKind, EnvAfflictionSpec, EnvAfflictionState, EnvClearReason, EnvSignal, EnvTickOutput,
};

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
    let kind = EnvAfflictionKind::RefrigerantInhalation;
    let oxygen_toxicity_active = susceptibility.oxygen_toxic && signal.o2_partial_kpa > 16.0;
    let mult = susceptibility.refrigerant_inhalation_multiplier;
    if check_origin_immune(
        state,
        actor_id,
        kind,
        susceptibility,
        if oxygen_toxicity_active { mult.max(0.5) } else { mult },
        "refrigerant_immune_origin",
        None,
        out,
    ) {
        return;
    }
    let was_active = state.severity(kind) > 0.0
        || state.last_threshold(kind) != super::EnvSeverity::None;
    let mut acc = state.accumulator(kind);
    let pp = signal.refrigerant_partial_kpa.max(0.0);
    let mut rate = pp * mult * dt_seconds;
    if oxygen_toxicity_active && rate <= 0.0 {
        rate = signal.o2_partial_kpa.max(0.0) * 0.5 * mult * dt_seconds;
    }
    if rate > 0.0 {
        acc.kind_value += rate;
        acc.cooldown_seconds = spec.clear_cooldown_s;
    } else if acc.cooldown_seconds > 0.0 {
        acc.cooldown_seconds = (acc.cooldown_seconds - dt_seconds).max(0.0);
        if acc.cooldown_seconds <= 0.0 {
            acc.kind_value = (acc.kind_value - spec.decay_per_s * 10.0 * dt_seconds).max(0.0);
        }
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
    if severity > 0.0 {
        out.hp_damage += spec.hp_per_second_at_threshold * severity * dt_seconds;
        out.aim_wobble_multiplier *= 1.0 + (spec.aim_wobble_multiplier - 1.0) * severity;
        state.m16b_sepsis_feed = true;
    }
    if rate <= 0.0 && acc.kind_value <= 0.0 && acc.cooldown_seconds <= 0.0 && was_active {
        emit_clear(state, actor_id, kind, EnvClearReason::CooldownElapsed, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{EnvAfflictionRegistry, OriginId};

    fn human() -> AtmosphericSusceptibility {
        AtmosphericSusceptibility::for_origin(OriginId::Human)
    }

    #[test]
    fn refrigerant_inhalation_threshold_mild_at_50_kpa_s() {
        let reg = EnvAfflictionRegistry::default_registry();
        let spec = reg.lookup(EnvAfflictionKind::RefrigerantInhalation);
        let mut state = EnvAfflictionState::default();
        let signal = EnvSignal {
            refrigerant_partial_kpa: 10.0,
            ..Default::default()
        };
        let mut out = EnvTickOutput::default();
        out.speed_multiplier = 1.0;
        out.aim_wobble_multiplier = 1.0;
        out.stamina_drain_multiplier = 1.0;
        for _ in 0..6 {
            tick(&mut state, 1, &human(), &signal, &spec, 1.0, None, &mut out);
        }
        assert!(state.accumulator(EnvAfflictionKind::RefrigerantInhalation).kind_value >= 50.0);
        assert!(state.m16b_sepsis_feed);
    }
}
