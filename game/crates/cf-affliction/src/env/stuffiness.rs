//! M16A § stuffiness — humidity > 70% + CO2 > 0.4% + occupants >= 4.
//! Accumulator: `stuffy_seconds` (+1.0 per second when triggered).
//! Clear: leave room OR ventilate (CO2 < 0.2%).

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
    let kind = EnvAfflictionKind::Stuffiness;
    if check_origin_immune(
        state,
        actor_id,
        kind,
        susceptibility,
        susceptibility.stuffiness_multiplier,
        "stuffiness_immune_origin",
        None,
        out,
    ) {
        return;
    }
    let triggered = signal.humidity_pct > 70.0
        && signal.co2_partial_kpa > 0.4
        && signal.occupant_count >= 4;
    let ventilated = signal.co2_partial_kpa < 0.2;
    let was_active = state.severity(kind) > 0.0 || state.last_threshold(kind) != super::EnvSeverity::None;
    let mut acc = state.accumulator(kind);
    if triggered {
        acc.kind_value += spec.accumulator_rate_per_s
            * susceptibility.stuffiness_multiplier
            * dt_seconds;
    } else if ventilated && acc.kind_value > 0.0 {
        let decay = spec.decay_per_s * dt_seconds * 4.0;
        acc.kind_value = (acc.kind_value - decay).max(0.0);
    } else if !triggered && acc.kind_value > 0.0 {
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
        out.stamina_drain_multiplier *= 1.0 + (spec.stamina_drain_multiplier - 1.0) * severity;
    }
    if ventilated && acc.kind_value <= 0.0 && was_active {
        emit_clear(state, actor_id, kind, EnvClearReason::ConditionCleared, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{EnvAfflictionRegistry, EnvSignal, OriginId};

    fn human() -> AtmosphericSusceptibility {
        AtmosphericSusceptibility::for_origin(OriginId::Human)
    }

    #[test]
    fn stuffiness_accumulates_in_crowded_room() {
        let reg = EnvAfflictionRegistry::default_registry();
        let spec = reg.lookup(EnvAfflictionKind::Stuffiness);
        let mut state = EnvAfflictionState::default();
        let signal = EnvSignal {
            humidity_pct: 80.0,
            co2_partial_kpa: 0.5,
            occupant_count: 6,
            ..Default::default()
        };
        let mut out = EnvTickOutput {
            speed_multiplier: 1.0,
            aim_wobble_multiplier: 1.0,
            stamina_drain_multiplier: 1.0,
            ..Default::default()
        };
        for _ in 0..610 {
            tick(&mut state, 1, &human(), &signal, &spec, 1.0, None, &mut out);
        }
        assert!(state.accumulator(EnvAfflictionKind::Stuffiness).kind_value >= 600.0);
        assert!(state.severity(EnvAfflictionKind::Stuffiness) > 0.0);
        assert!(out.threshold_crossed.iter().any(|e| e.kind == EnvAfflictionKind::Stuffiness));
    }

    #[test]
    fn stuffiness_clears_on_ventilation() {
        let reg = EnvAfflictionRegistry::default_registry();
        let spec = reg.lookup(EnvAfflictionKind::Stuffiness);
        let mut state = EnvAfflictionState::default();
        let crowded = EnvSignal {
            humidity_pct: 80.0,
            co2_partial_kpa: 0.5,
            occupant_count: 6,
            ..Default::default()
        };
        let mut out = EnvTickOutput::default();
        out.speed_multiplier = 1.0;
        out.aim_wobble_multiplier = 1.0;
        out.stamina_drain_multiplier = 1.0;
        for _ in 0..610 {
            tick(&mut state, 1, &human(), &crowded, &spec, 1.0, None, &mut out);
        }
        let ventilated = EnvSignal {
            humidity_pct: 50.0,
            co2_partial_kpa: 0.1,
            occupant_count: 6,
            ..Default::default()
        };
        out = EnvTickOutput::default();
        out.speed_multiplier = 1.0;
        out.aim_wobble_multiplier = 1.0;
        out.stamina_drain_multiplier = 1.0;
        for _ in 0..1000 {
            tick(&mut state, 1, &human(), &ventilated, &spec, 1.0, None, &mut out);
            if state.severity(EnvAfflictionKind::Stuffiness) <= 0.0 {
                break;
            }
        }
        assert!(state.severity(EnvAfflictionKind::Stuffiness) <= 0.0);
        assert!(out.cleared.iter().any(|e| e.kind == EnvAfflictionKind::Stuffiness));
    }

    #[test]
    fn robot_is_immune_to_stuffiness() {
        let reg = EnvAfflictionRegistry::default_registry();
        let spec = reg.lookup(EnvAfflictionKind::Stuffiness);
        let robot = AtmosphericSusceptibility::for_origin(OriginId::Robot);
        let mut state = EnvAfflictionState::default();
        let signal = EnvSignal {
            humidity_pct: 99.0,
            co2_partial_kpa: 1.0,
            occupant_count: 99,
            ..Default::default()
        };
        let mut out = EnvTickOutput::default();
        out.speed_multiplier = 1.0;
        out.aim_wobble_multiplier = 1.0;
        out.stamina_drain_multiplier = 1.0;
        tick(&mut state, 7, &robot, &signal, &spec, 1.0, None, &mut out);
        assert_eq!(state.accumulator(EnvAfflictionKind::Stuffiness).kind_value, 0.0);
        assert!(out
            .origin_immune
            .iter()
            .any(|e| e.kind == EnvAfflictionKind::Stuffiness));
    }
}
