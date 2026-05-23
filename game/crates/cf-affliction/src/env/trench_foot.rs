//! M16A § trench_foot — wet duckboards (submerged feet in cold liquid).
//! Accumulator: `wet_duckboard_seconds` (1.0/tick while feet wet+cold).
//! Clear: dry boots + 24 in-game hours warm + dry.

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
    let kind = EnvAfflictionKind::TrenchFoot;
    if check_origin_immune(
        state,
        actor_id,
        kind,
        susceptibility,
        susceptibility.trench_foot_multiplier,
        "trench_foot_immune_origin",
        None,
        out,
    ) {
        return;
    }
    let was_active = state.severity(kind) > 0.0
        || state.last_threshold(kind) != super::EnvSeverity::None;
    let mut acc = state.accumulator(kind);
    if signal.wet_duckboard_contact {
        acc.kind_value += spec.accumulator_rate_per_s
            * susceptibility.trench_foot_multiplier
            * dt_seconds;
        acc.cooldown_seconds = spec.clear_cooldown_s;
    } else if signal.feet_dry_and_warm {
        if acc.cooldown_seconds > 0.0 {
            acc.cooldown_seconds = (acc.cooldown_seconds - dt_seconds).max(0.0);
            if acc.cooldown_seconds <= 0.0 {
                acc.kind_value = 0.0;
            }
        } else if acc.kind_value > 0.0 {
            acc.kind_value = (acc.kind_value - spec.decay_per_s * dt_seconds).max(0.0);
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
        out.speed_multiplier *= 1.0 - (1.0 - spec.speed_multiplier) * severity;
        state.m16b_sepsis_feed = true;
    }
    if signal.feet_dry_and_warm
        && acc.kind_value <= 0.0
        && acc.cooldown_seconds <= 0.0
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
    fn trench_foot_accumulates_on_wet_duckboard() {
        let reg = EnvAfflictionRegistry::default_registry();
        let spec = reg.lookup(EnvAfflictionKind::TrenchFoot);
        let mut state = EnvAfflictionState::default();
        let signal = EnvSignal {
            wet_duckboard_contact: true,
            ..Default::default()
        };
        let mut out = EnvTickOutput::default();
        out.speed_multiplier = 1.0;
        out.aim_wobble_multiplier = 1.0;
        out.stamina_drain_multiplier = 1.0;
        for _ in 0..7200 {
            tick(&mut state, 1, &human(), &signal, &spec, 1.0, None, &mut out);
        }
        assert!(state.accumulator(EnvAfflictionKind::TrenchFoot).kind_value >= 7200.0);
        assert!(state.m16b_sepsis_feed);
    }

    #[test]
    fn aqueous_immune_to_trench_foot() {
        let reg = EnvAfflictionRegistry::default_registry();
        let spec = reg.lookup(EnvAfflictionKind::TrenchFoot);
        let aqueous = AtmosphericSusceptibility::for_origin(OriginId::Aqueous);
        let mut state = EnvAfflictionState::default();
        let signal = EnvSignal {
            wet_duckboard_contact: true,
            ..Default::default()
        };
        let mut out = EnvTickOutput::default();
        out.speed_multiplier = 1.0;
        out.aim_wobble_multiplier = 1.0;
        out.stamina_drain_multiplier = 1.0;
        tick(&mut state, 1, &aqueous, &signal, &spec, 1.0, None, &mut out);
        assert!(out.origin_immune.iter().any(|e| e.kind == EnvAfflictionKind::TrenchFoot));
    }
}
