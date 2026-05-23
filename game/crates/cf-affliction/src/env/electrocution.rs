//! M16A § electrocution — electrified fence / electric hazard tile contact.
//! Accumulator: `arc_count` (per shock event) + instant stun ticks.
//! Clear: wait OR insulator removes ongoing contact.

use super::{
    emit_clear, evaluate_threshold, AtmosphericSusceptibility, EnvAfflictionKind, EnvAfflictionSpec,
    EnvAfflictionState, EnvClearReason, EnvSignal, EnvTickOutput,
};

pub const KNOCKDOWN_TICKS_PER_SHOCK: u32 = 30;

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
    let kind = EnvAfflictionKind::Electrocution;
    let resistance = susceptibility.electrocution_resistance.clamp(-1.0, 1.0);
    let was_active = state.severity(kind) > 0.0
        || state.last_threshold(kind) != super::EnvSeverity::None;
    let mut acc = state.accumulator(kind);
    if signal.electric_shock_event_j > 0.0 {
        let energy_after = signal.electric_shock_event_j * (1.0 - resistance).max(0.0);
        let arcs = (energy_after / 40.0).ceil().clamp(0.0, 4.0) as u32;
        if arcs > 0 {
            acc.arc_count = acc.arc_count.saturating_add(arcs);
            acc.knockdown_ticks_remaining = acc
                .knockdown_ticks_remaining
                .saturating_add(KNOCKDOWN_TICKS_PER_SHOCK * arcs);
            acc.kind_value = acc.arc_count as f32;
            acc.cooldown_seconds = 10.0;
            out.hp_damage += spec.hp_per_second_at_threshold * (energy_after / 80.0);
        }
    }
    if signal.electric_shock_event_j <= 0.0 && acc.cooldown_seconds > 0.0 {
        acc.cooldown_seconds = (acc.cooldown_seconds - dt_seconds).max(0.0);
        if acc.cooldown_seconds <= 0.0 {
            acc.arc_count = 0;
            acc.kind_value = 0.0;
        }
    }
    let knockdown_used = dt_seconds.max(1e-3) * 30.0;
    let to_consume = knockdown_used as u32;
    if acc.knockdown_ticks_remaining > 0 {
        acc.knockdown_ticks_remaining = acc.knockdown_ticks_remaining.saturating_sub(to_consume);
    }
    state.set_accumulator(kind, acc);
    out.knockdown_ticks = out.knockdown_ticks.max(acc.knockdown_ticks_remaining);
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
        out.aim_wobble_multiplier *= 1.0 + 0.4 * severity;
    }
    if acc.arc_count == 0
        && acc.cooldown_seconds <= 0.0
        && acc.knockdown_ticks_remaining == 0
        && was_active
    {
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
    fn single_shock_increments_arc_count_and_knockdown() {
        let reg = EnvAfflictionRegistry::default_registry();
        let spec = reg.lookup(EnvAfflictionKind::Electrocution);
        let mut state = EnvAfflictionState::default();
        let signal = EnvSignal {
            electric_shock_event_j: 80.0,
            ..Default::default()
        };
        let mut out = EnvTickOutput::default();
        out.speed_multiplier = 1.0;
        out.aim_wobble_multiplier = 1.0;
        out.stamina_drain_multiplier = 1.0;
        tick(&mut state, 1, &human(), &signal, &spec, 1.0 / 30.0, None, &mut out);
        let acc = state.accumulator(EnvAfflictionKind::Electrocution);
        assert!(acc.arc_count >= 1);
        assert!(acc.knockdown_ticks_remaining > 0);
    }

    #[test]
    fn three_shocks_escalate_to_severe() {
        let reg = EnvAfflictionRegistry::default_registry();
        let spec = reg.lookup(EnvAfflictionKind::Electrocution);
        let mut state = EnvAfflictionState::default();
        let signal = EnvSignal {
            electric_shock_event_j: 40.0,
            ..Default::default()
        };
        let mut out = EnvTickOutput::default();
        out.speed_multiplier = 1.0;
        out.aim_wobble_multiplier = 1.0;
        out.stamina_drain_multiplier = 1.0;
        for _ in 0..3 {
            tick(&mut state, 1, &human(), &signal, &spec, 1.0 / 30.0, None, &mut out);
        }
        assert!(state.accumulator(EnvAfflictionKind::Electrocution).arc_count >= 3);
        assert!(out
            .threshold_crossed
            .iter()
            .any(|e| e.kind == EnvAfflictionKind::Electrocution));
    }
}
