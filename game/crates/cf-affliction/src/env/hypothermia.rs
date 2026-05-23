//! M16A § hypothermia — room temperature below origin comfort_min_k.
//! Accumulator: `cold_load_celsius_seconds` (ΔT × dt).
//! Clear: T_room back in comfort + 5 min cooldown OR warm enclosure.

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
    let kind = EnvAfflictionKind::Hypothermia;
    if signal.room_temp_k <= 0.0 {
        return;
    }
    let comfort_min = susceptibility.cold_comfort_min_k;
    let was_active = state.severity(kind) > 0.0
        || state.last_threshold(kind) != super::EnvSeverity::None;
    let mut acc = state.accumulator(kind);
    if signal.room_temp_k < comfort_min {
        let delta_c = comfort_min - signal.room_temp_k;
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
    if signal.room_temp_k >= comfort_min
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
    fn hypothermia_accumulates_below_comfort_min() {
        let reg = EnvAfflictionRegistry::default_registry();
        let spec = reg.lookup(EnvAfflictionKind::Hypothermia);
        let mut state = EnvAfflictionState::default();
        let signal = EnvSignal {
            room_temp_k: 263.0,
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
        let acc = state.accumulator(EnvAfflictionKind::Hypothermia);
        assert!(acc.kind_value >= 300.0);
        assert!(out
            .threshold_crossed
            .iter()
            .any(|e| e.kind == EnvAfflictionKind::Hypothermia));
    }

    #[test]
    fn warm_enclosure_clears_after_cooldown() {
        let reg = EnvAfflictionRegistry::default_registry();
        let spec = reg.lookup(EnvAfflictionKind::Hypothermia);
        let mut state = EnvAfflictionState::default();
        let cold = EnvSignal {
            room_temp_k: 230.0,
            ..Default::default()
        };
        let mut out = EnvTickOutput::default();
        out.speed_multiplier = 1.0;
        out.aim_wobble_multiplier = 1.0;
        out.stamina_drain_multiplier = 1.0;
        for _ in 0..40 {
            tick(&mut state, 1, &human(), &cold, &spec, 1.0, None, &mut out);
        }
        let warm = EnvSignal {
            room_temp_k: 300.0,
            ..Default::default()
        };
        out = EnvTickOutput::default();
        out.speed_multiplier = 1.0;
        out.aim_wobble_multiplier = 1.0;
        out.stamina_drain_multiplier = 1.0;
        for _ in 0..400 {
            tick(&mut state, 1, &human(), &warm, &spec, 1.0, None, &mut out);
            if state.severity(EnvAfflictionKind::Hypothermia) <= 0.0
                && state.last_threshold(EnvAfflictionKind::Hypothermia)
                    == crate::env::EnvSeverity::None
            {
                break;
            }
        }
        assert!(out.cleared.iter().any(|e| e.kind == EnvAfflictionKind::Hypothermia));
    }
}
