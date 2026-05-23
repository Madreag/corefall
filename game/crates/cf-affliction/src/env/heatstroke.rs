//! M16A § heatstroke — room temperature exceeds origin comfort_max_k.
//! Accumulator: `heat_load_celsius_seconds` (ΔT × dt).
//! Clear: T_room back in comfort + 5 min cooldown OR cold-water immersion.

use super::{
    emit_clear, evaluate_threshold, AtmosphericSusceptibility, EnvAfflictionKind,
    EnvAfflictionSpec, EnvAfflictionState, EnvClearReason, EnvSignal, EnvTickOutput,
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
    let kind = EnvAfflictionKind::Heatstroke;
    if signal.room_temp_k <= 0.0 {
        return;
    }
    let comfort_max = susceptibility.heat_comfort_max_k;
    let was_active = state.severity(kind) > 0.0
        || state.last_threshold(kind) != super::EnvSeverity::None;
    let mut acc = state.accumulator(kind);
    if signal.room_temp_k > comfort_max {
        let delta_c = signal.room_temp_k - comfort_max;
        acc.kind_value += delta_c * dt_seconds;
        acc.cooldown_seconds = spec.clear_cooldown_s;
    } else if acc.cooldown_seconds > 0.0 {
        acc.cooldown_seconds = (acc.cooldown_seconds - dt_seconds).max(0.0);
        if acc.cooldown_seconds <= 0.0 {
            acc.kind_value = 0.0;
        }
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
    if severity > 0.0 {
        out.hp_damage += spec.hp_per_second_at_threshold * severity * dt_seconds;
        out.speed_multiplier *= 1.0 - (1.0 - spec.speed_multiplier) * severity;
        out.aim_wobble_multiplier *= 1.0 + (spec.aim_wobble_multiplier - 1.0) * severity;
    }
    if signal.room_temp_k <= comfort_max
        && acc.cooldown_seconds <= 0.0
        && acc.kind_value <= 0.0
        && was_active
    {
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
    fn heatstroke_accumulates_above_comfort_max() {
        let reg = EnvAfflictionRegistry::default_registry();
        let spec = reg.lookup(EnvAfflictionKind::Heatstroke);
        let mut state = EnvAfflictionState::default();
        let signal = EnvSignal {
            room_temp_k: 332.0,
            ..Default::default()
        };
        let mut out = EnvTickOutput {
            speed_multiplier: 1.0,
            aim_wobble_multiplier: 1.0,
            stamina_drain_multiplier: 1.0,
            ..Default::default()
        };
        for _ in 0..40 {
            tick(&mut state, 1, &human(), &signal, &spec, 1.0, None, &mut out);
        }
        let acc = state.accumulator(EnvAfflictionKind::Heatstroke);
        assert!(acc.kind_value >= 300.0);
        assert!(out
            .threshold_crossed
            .iter()
            .any(|e| e.kind == EnvAfflictionKind::Heatstroke));
    }

    #[test]
    fn crystalline_tolerates_much_higher_heat() {
        let reg = EnvAfflictionRegistry::default_registry();
        let spec = reg.lookup(EnvAfflictionKind::Heatstroke);
        let crystalline = AtmosphericSusceptibility::for_origin(OriginId::Crystalline);
        let mut state = EnvAfflictionState::default();
        let signal = EnvSignal {
            room_temp_k: 350.0,
            ..Default::default()
        };
        let mut out = EnvTickOutput::default();
        out.speed_multiplier = 1.0;
        out.aim_wobble_multiplier = 1.0;
        out.stamina_drain_multiplier = 1.0;
        for _ in 0..100 {
            tick(&mut state, 1, &crystalline, &signal, &spec, 1.0, None, &mut out);
        }
        assert_eq!(state.accumulator(EnvAfflictionKind::Heatstroke).kind_value, 0.0);
    }
}
