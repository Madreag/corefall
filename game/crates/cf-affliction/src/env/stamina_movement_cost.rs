//! M16A § stamina_movement_cost — heavy weapon equipped (e.g. mg_tripod_portable +400 kg).
//! Accumulator: `extra_kg_carried × dt`.
//! Clear: drop heavy weapon OR reach base resupply.

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
    let kind = EnvAfflictionKind::StaminaMovementCost;
    let extra_kg = (signal.heavy_weapon_kg - signal.baseline_carry_kg).max(0.0);
    let was_active = state.severity(kind) > 0.0
        || state.last_threshold(kind) != super::EnvSeverity::None;
    let mut acc = state.accumulator(kind);
    if extra_kg > 0.0 {
        acc.kind_value =
            extra_kg * dt_seconds * susceptibility.stamina_cost_multiplier;
    } else if acc.kind_value > 0.0 {
        acc.kind_value = 0.0;
    }
    state.set_accumulator(kind, acc);
    let _ = evaluate_threshold(
        state,
        actor_id,
        kind,
        if extra_kg > 0.0 { extra_kg } else { 0.0 },
        spec,
        susceptibility.origin_id,
        source_event_id,
        out,
    );
    if extra_kg > 0.0 {
        let load_factor = (extra_kg / 100.0).max(0.0);
        out.stamina_drain_multiplier *= 1.0 + load_factor * (spec.stamina_drain_multiplier - 1.0);
        out.speed_multiplier *= 1.0 - (1.0 - spec.speed_multiplier).min(load_factor * 0.3);
    } else if was_active {
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
    fn heavy_weapon_fires_threshold_and_drains_stamina() {
        let reg = EnvAfflictionRegistry::default_registry();
        let spec = reg.lookup(EnvAfflictionKind::StaminaMovementCost);
        let mut state = EnvAfflictionState::default();
        let signal = EnvSignal {
            heavy_weapon_kg: 420.0,
            baseline_carry_kg: 20.0,
            ..Default::default()
        };
        let mut out = EnvTickOutput::default();
        out.speed_multiplier = 1.0;
        out.aim_wobble_multiplier = 1.0;
        out.stamina_drain_multiplier = 1.0;
        tick(&mut state, 1, &human(), &signal, &spec, 1.0, None, &mut out);
        assert!(out.stamina_drain_multiplier > 1.0);
        assert!(out
            .threshold_crossed
            .iter()
            .any(|e| e.kind == EnvAfflictionKind::StaminaMovementCost));
    }

    #[test]
    fn dropping_heavy_weapon_clears() {
        let reg = EnvAfflictionRegistry::default_registry();
        let spec = reg.lookup(EnvAfflictionKind::StaminaMovementCost);
        let mut state = EnvAfflictionState::default();
        let heavy = EnvSignal {
            heavy_weapon_kg: 420.0,
            baseline_carry_kg: 20.0,
            ..Default::default()
        };
        let dropped = EnvSignal {
            heavy_weapon_kg: 20.0,
            baseline_carry_kg: 20.0,
            ..Default::default()
        };
        let mut out = EnvTickOutput::default();
        out.speed_multiplier = 1.0;
        out.aim_wobble_multiplier = 1.0;
        out.stamina_drain_multiplier = 1.0;
        tick(&mut state, 1, &human(), &heavy, &spec, 1.0, None, &mut out);
        out = EnvTickOutput::default();
        out.speed_multiplier = 1.0;
        out.aim_wobble_multiplier = 1.0;
        out.stamina_drain_multiplier = 1.0;
        tick(&mut state, 1, &human(), &dropped, &spec, 1.0, None, &mut out);
        assert!(out
            .cleared
            .iter()
            .any(|e| e.kind == EnvAfflictionKind::StaminaMovementCost));
    }
}
