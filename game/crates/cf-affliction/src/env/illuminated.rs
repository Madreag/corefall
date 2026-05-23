//! M16A § illuminated — spotlight cone reveals concealed actor.
//! Accumulator: `illuminated_seconds` (1.0/tick while in cone).
//! Clear: leave cone OR destroy spotlight.

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
    let kind = EnvAfflictionKind::Illuminated;
    let was_active = state.severity(kind) > 0.0
        || state.last_threshold(kind) != super::EnvSeverity::None;
    let mut acc = state.accumulator(kind);
    if signal.spotlight_lit {
        acc.kind_value += spec.accumulator_rate_per_s * dt_seconds;
    } else if acc.kind_value > 0.0 {
        acc.kind_value = 0.0;
    }
    state.set_accumulator(kind, acc);
    if signal.spotlight_lit {
        let severity = 1.0_f32.min(0.5 + acc.kind_value * 0.05);
        let from = state.severity(kind);
        if (severity - from).abs() > f32::EPSILON {
            state.set_severity(kind, severity);
            out.severity_changed.push(super::EnvSeverityChangedEvent {
                actor_id,
                kind,
                from_severity: from,
                to_severity: severity,
                accumulator_value: acc.kind_value,
            });
        }
        let band = super::EnvSeverity::from_severity_0_1(severity.max(0.1));
        if state.last_threshold(kind) == super::EnvSeverity::None {
            state.set_last_threshold(kind, band);
            out.threshold_crossed.push(super::EnvThresholdCrossedEvent {
                actor_id,
                kind,
                severity: band,
                severity_0_1: severity,
                accumulator_value: acc.kind_value,
                source_event_id,
                origin_id: susceptibility.origin_id,
            });
        }
        out.reveal_to_ai = true;
    } else {
        let _ = evaluate_threshold(
            state,
            actor_id,
            kind,
            0.0,
            spec,
            susceptibility.origin_id,
            source_event_id,
            out,
        );
        if was_active {
            emit_clear(state, actor_id, kind, EnvClearReason::ConditionCleared, out);
        }
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
    fn entering_cone_fires_threshold_and_reveals_to_ai() {
        let reg = EnvAfflictionRegistry::default_registry();
        let spec = reg.lookup(EnvAfflictionKind::Illuminated);
        let mut state = EnvAfflictionState::default();
        let signal = EnvSignal {
            spotlight_lit: true,
            ..Default::default()
        };
        let mut out = EnvTickOutput::default();
        out.speed_multiplier = 1.0;
        out.aim_wobble_multiplier = 1.0;
        out.stamina_drain_multiplier = 1.0;
        tick(&mut state, 1, &human(), &signal, &spec, 1.0, None, &mut out);
        assert!(out.reveal_to_ai);
        assert!(out
            .threshold_crossed
            .iter()
            .any(|e| e.kind == EnvAfflictionKind::Illuminated));
    }

    #[test]
    fn leaving_cone_clears_immediately() {
        let reg = EnvAfflictionRegistry::default_registry();
        let spec = reg.lookup(EnvAfflictionKind::Illuminated);
        let mut state = EnvAfflictionState::default();
        let lit = EnvSignal {
            spotlight_lit: true,
            ..Default::default()
        };
        let dark = EnvSignal {
            spotlight_lit: false,
            ..Default::default()
        };
        let mut out = EnvTickOutput::default();
        out.speed_multiplier = 1.0;
        out.aim_wobble_multiplier = 1.0;
        out.stamina_drain_multiplier = 1.0;
        tick(&mut state, 1, &human(), &lit, &spec, 1.0, None, &mut out);
        out = EnvTickOutput::default();
        out.speed_multiplier = 1.0;
        out.aim_wobble_multiplier = 1.0;
        out.stamina_drain_multiplier = 1.0;
        tick(&mut state, 1, &human(), &dark, &spec, 1.0, None, &mut out);
        assert!(!out.reveal_to_ai);
        assert!(out
            .cleared
            .iter()
            .any(|e| e.kind == EnvAfflictionKind::Illuminated));
    }
}
