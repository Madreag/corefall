//! M16A § panic_freeze_env — analyzer alarm acknowledged but not addressed,
//! or extreme breach event. Event-driven (no accumulator; one-shot per trigger).
//! Clear: wait timer OR squadmate stabilize action.

use super::{
    emit_clear, evaluate_threshold, AtmosphericSusceptibility, EnvAfflictionKind, EnvAfflictionSpec,
    EnvAfflictionState, EnvClearReason, EnvSignal, EnvTickOutput,
};

const PANIC_FREEZE_BASE_TICKS: u32 = 30 * 3;

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
    let kind = EnvAfflictionKind::PanicFreezeEnv;
    let was_active = state.severity(kind) > 0.0
        || state.last_threshold(kind) != super::EnvSeverity::None;
    let mut acc = state.accumulator(kind);
    let triggered = signal.analyzer_alarm_unaddressed || signal.extreme_breach_event;
    if triggered && acc.panic_freeze_ticks_remaining == 0 {
        let extra = if signal.extreme_breach_event {
            30 * 5
        } else {
            PANIC_FREEZE_BASE_TICKS
        };
        acc.panic_freeze_ticks_remaining = extra;
        acc.cooldown_seconds = (extra as f32) / 30.0;
        acc.kind_value = 1.0;
    }
    if signal.stabilize_assist {
        acc.panic_freeze_ticks_remaining = 0;
        acc.cooldown_seconds = 0.0;
        acc.kind_value = 0.0;
        state.set_accumulator(kind, acc);
        if was_active {
            emit_clear(state, actor_id, kind, EnvClearReason::SquadmateStabilized, out);
        }
        return;
    }
    if acc.cooldown_seconds > 0.0 {
        acc.cooldown_seconds = (acc.cooldown_seconds - dt_seconds).max(0.0);
        let target_ticks = (acc.cooldown_seconds * 30.0).ceil() as u32;
        acc.panic_freeze_ticks_remaining = target_ticks;
        if acc.cooldown_seconds <= 0.0 {
            acc.kind_value = 0.0;
        }
    }
    state.set_accumulator(kind, acc);
    out.panic_freeze_ticks = out.panic_freeze_ticks.max(acc.panic_freeze_ticks_remaining);
    let _ = evaluate_threshold(
        state,
        actor_id,
        kind,
        acc.kind_value,
        spec,
        susceptibility.origin_id,
        source_event_id,
        out,
    );
    if acc.panic_freeze_ticks_remaining == 0
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
    fn analyzer_alarm_triggers_panic_freeze_timer() {
        let reg = EnvAfflictionRegistry::default_registry();
        let spec = reg.lookup(EnvAfflictionKind::PanicFreezeEnv);
        let mut state = EnvAfflictionState::default();
        let signal = EnvSignal {
            analyzer_alarm_unaddressed: true,
            ..Default::default()
        };
        let mut out = EnvTickOutput::default();
        out.speed_multiplier = 1.0;
        out.aim_wobble_multiplier = 1.0;
        out.stamina_drain_multiplier = 1.0;
        tick(&mut state, 1, &human(), &signal, &spec, 1.0 / 30.0, None, &mut out);
        assert!(out.panic_freeze_ticks > 0);
        assert!(out
            .threshold_crossed
            .iter()
            .any(|e| e.kind == EnvAfflictionKind::PanicFreezeEnv));
    }

    #[test]
    fn squadmate_stabilize_clears_immediately() {
        let reg = EnvAfflictionRegistry::default_registry();
        let spec = reg.lookup(EnvAfflictionKind::PanicFreezeEnv);
        let mut state = EnvAfflictionState::default();
        let trigger = EnvSignal {
            analyzer_alarm_unaddressed: true,
            ..Default::default()
        };
        let mut out = EnvTickOutput::default();
        out.speed_multiplier = 1.0;
        out.aim_wobble_multiplier = 1.0;
        out.stamina_drain_multiplier = 1.0;
        tick(&mut state, 1, &human(), &trigger, &spec, 1.0 / 30.0, None, &mut out);
        let stabilize = EnvSignal {
            stabilize_assist: true,
            ..Default::default()
        };
        out = EnvTickOutput::default();
        out.speed_multiplier = 1.0;
        out.aim_wobble_multiplier = 1.0;
        out.stamina_drain_multiplier = 1.0;
        tick(&mut state, 1, &human(), &stabilize, &spec, 1.0 / 30.0, None, &mut out);
        assert!(out
            .cleared
            .iter()
            .any(|e| e.kind == EnvAfflictionKind::PanicFreezeEnv && e.reason == EnvClearReason::SquadmateStabilized));
    }
}
