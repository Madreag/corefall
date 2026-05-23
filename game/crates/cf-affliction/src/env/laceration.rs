//! M16A § laceration — razor wire / bladed hits stack wounds.
//! Accumulator: `bleed_severity` per wound (M14G wound-list consumer).
//! Clear: bandage + 30s tend per wound.

use super::{
    emit_clear, evaluate_threshold, AtmosphericSusceptibility, EnvAfflictionKind, EnvAfflictionSpec,
    EnvAfflictionState, EnvClearReason, EnvSignal, EnvTickOutput,
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
    let kind = EnvAfflictionKind::Laceration;
    let was_active = state.severity(kind) > 0.0
        || state.last_threshold(kind) != super::EnvSeverity::None;
    let mut acc = state.accumulator(kind);
    let new_wound = signal.razor_wire_contact || signal.bladed_hit_severity > 0.0;
    if new_wound {
        let severity = if signal.bladed_hit_severity > 0.0 {
            signal.bladed_hit_severity.clamp(0.1, 1.0)
        } else {
            0.4
        };
        acc.bleed_stack = acc.bleed_stack.saturating_add(1);
        acc.kind_value += severity;
        acc.cooldown_seconds = spec.clear_cooldown_s;
        state.bleed_stack_total = state.bleed_stack_total.saturating_add(1);
    } else if acc.cooldown_seconds > 0.0 {
        acc.cooldown_seconds = (acc.cooldown_seconds - dt_seconds).max(0.0);
        if acc.cooldown_seconds <= 0.0 {
            acc.bleed_stack = acc.bleed_stack.saturating_sub(1);
            if acc.bleed_stack == 0 {
                acc.kind_value = 0.0;
            } else {
                acc.kind_value = (acc.kind_value - 1.0).max(0.0);
                acc.cooldown_seconds = spec.clear_cooldown_s;
            }
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
        out.hp_damage += spec.hp_per_second_at_threshold * (acc.bleed_stack as f32) * dt_seconds;
        state.m16b_sepsis_feed = true;
    }
    if acc.bleed_stack == 0 && acc.kind_value <= 0.0 && was_active {
        emit_clear(state, actor_id, kind, EnvClearReason::ConditionCleared, out);
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
    fn razor_wire_crossing_increments_bleed_stack() {
        let reg = EnvAfflictionRegistry::default_registry();
        let spec = reg.lookup(EnvAfflictionKind::Laceration);
        let mut state = EnvAfflictionState::default();
        let signal = EnvSignal {
            razor_wire_contact: true,
            ..Default::default()
        };
        let mut out = EnvTickOutput::default();
        out.speed_multiplier = 1.0;
        out.aim_wobble_multiplier = 1.0;
        out.stamina_drain_multiplier = 1.0;
        tick(&mut state, 1, &human(), &signal, &spec, 1.0, None, &mut out);
        assert_eq!(state.accumulator(EnvAfflictionKind::Laceration).bleed_stack, 1);
        assert!(out
            .threshold_crossed
            .iter()
            .any(|e| e.kind == EnvAfflictionKind::Laceration));
        assert!(state.bleed_stack_total >= 1);
    }
}
