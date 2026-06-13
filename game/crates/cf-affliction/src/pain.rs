//! pain.rs — the **M16C Pain affliction**. Pain is a per-actor scalar
//! affliction recomputed from the M14G wound list: it stacks per active wound
//! × severity and drives aim wobble, move speed, morale drain, and the
//! auto-triage trigger.
//!
//! Unlike the env / producer afflictions, Pain severity is *set* (recomputed
//! absolutely) each pass rather than stacked additively — so it tracks the
//! live wound state exactly. The pass runs every
//! `PAIN_RECOMPUTE_INTERVAL_TICKS` ticks (perf-bound, spec § Notes).
//!
//! Tuning (spec § Tunable defaults + acceptance scenario 1):
//!   - `LacerationModerate(0.5)` → `0.5 × 12 = 6` pain points.
//!   - 3 wounds `(0.5, 0.6, 0.4)` → stack `18`, severity `0.6`, wobble `1.6×`.

use serde::{Deserialize, Serialize};

use cf_wound::ActorWoundList;

use crate::{ActiveAffliction, ActorAfflictions, M16AfflictionKind};

/// Pain points contributed per 1.0 of wound severity (global flat rate).
/// `LacerationModerate(0.5)` → `0.5 × 12 = 6` pain points (spec § Tunable).
pub const PAIN_PER_SEVERITY: f32 = 12.0;

/// Pain stack at which severity saturates to 1.0 (full pain). Anchored by
/// acceptance scenario 1: stack 18 → severity 0.6 ⇒ full-stack 30.
pub const PAIN_FULL_STACK: f32 = 30.0;

/// Severity fraction at/above which pain is "> severe" and drains morale —
/// the floor of cf-wound's `SeverityBand::Severe` band `[0.50, 0.75)`.
pub const PAIN_MORALE_SEVERE_PCT: f32 = 0.50;

/// Severity fraction above which pain auto-triages (spec "Pain > 80%").
pub const PAIN_AUTOTRIAGE_PCT: f32 = 0.80;

/// Move-speed coefficient: `move_speed × (1 − pct × 0.3)` (spec).
pub const PAIN_MOVE_SPEED_COEFF: f32 = 0.30;

/// Morale delta per tick while pain is "> severe".
pub const PAIN_MORALE_DRAIN_PER_TICK: f32 = -1.0;

/// Pain is recomputed every N ticks from the wound list (perf-bound, spec
/// § Notes: "recomputed every N=5 ticks from M14G wound list").
pub const PAIN_RECOMPUTE_INTERVAL_TICKS: u64 = 5;

/// Severity fraction [0,1] for a raw pain `stack`.
pub fn pain_severity_pct(stack: f32) -> f32 {
    (stack / PAIN_FULL_STACK).clamp(0.0, 1.0)
}

/// Aim-wobble multiplier ∈ [1.0, 2.0]: `1 + severity_pct`. Stack 18 → 1.6×
/// (scenario 1); saturates at 2.0× once the stack reaches `PAIN_FULL_STACK`
/// (so "at 80 pain = 2.0×", spec § Tunable, holds for any stack ≥ 30).
pub fn pain_aim_wobble_multiplier(stack: f32) -> f32 {
    1.0 + pain_severity_pct(stack)
}

/// Move-speed factor ∈ [0.7, 1.0]: `1 − severity_pct × 0.3`.
pub fn pain_move_speed_factor(stack: f32) -> f32 {
    1.0 - pain_severity_pct(stack) * PAIN_MOVE_SPEED_COEFF
}

/// Per-tick morale drain (−1.0 once pain is "> severe", else 0.0).
pub fn pain_morale_drain_per_tick(stack: f32) -> f32 {
    if pain_severity_pct(stack) >= PAIN_MORALE_SEVERE_PCT {
        PAIN_MORALE_DRAIN_PER_TICK
    } else {
        0.0
    }
}

/// True when pain crosses the auto-triage trigger (> 80%).
pub fn pain_triggers_autotriage(stack: f32) -> bool {
    pain_severity_pct(stack) > PAIN_AUTOTRIAGE_PCT
}

/// True on ticks when the pain pass is due (every
/// `PAIN_RECOMPUTE_INTERVAL_TICKS`).
pub fn pain_recompute_due(tick: u64) -> bool {
    tick % PAIN_RECOMPUTE_INTERVAL_TICKS == 0
}

/// The pain-stack recompute event (`pain.stack_changed`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PainStackChangedEvent {
    pub actor_id: u64,
    pub tick: u64,
    pub old_stack: f32,
    pub new_stack: f32,
    pub severity: f32,
    pub aim_wobble_multiplier: f32,
}

/// Total raw pain stack from an actor's wound list: `Σ severity × 12` over
/// every active wound across every zone.
pub fn pain_stack_from_wounds(wounds: &ActorWoundList) -> f32 {
    wounds
        .wounds_by_zone
        .values()
        .flatten()
        .map(|w| w.severity * PAIN_PER_SEVERITY)
        .sum()
}

/// Recompute the Pain affliction from the M14G wound list and reconcile it
/// into `state`. Returns `Some(PainStackChangedEvent)` when the stack changed
/// (including dropping to zero, which clears the affliction). Pain severity is
/// SET (not stacked) to the recomputed fraction.
pub fn recompute_pain(
    state: &mut ActorAfflictions,
    actor_id: u64,
    wounds: &ActorWoundList,
    tick: u64,
) -> Option<PainStackChangedEvent> {
    let new_stack = pain_stack_from_wounds(wounds);
    let new_pct = pain_severity_pct(new_stack);
    let old_pct = state.severity_of(M16AfflictionKind::Pain);
    let old_stack = old_pct * PAIN_FULL_STACK;

    // Idempotent across recompute passes: unchanged stack → no event.
    if (new_stack - old_stack).abs() < 1e-4 {
        return None;
    }

    if new_stack <= 0.0 {
        // Pain fully resolved — clear the affliction + any pending banner.
        state.active.retain(|a| a.kind != M16AfflictionKind::Pain);
        state
            .critical_banner_pending
            .retain(|k| *k != M16AfflictionKind::Pain);
    } else {
        // Set (not stack) the severity, tracking a low→critical crossing for
        // the banner without holding the `find_mut` borrow across the push.
        let crossed_critical = if let Some(existing) = state.find_mut(M16AfflictionKind::Pain) {
            let was_below = existing.severity < 0.8;
            existing.severity = new_pct;
            was_below && new_pct >= 0.8
        } else {
            state.active.push(ActiveAffliction {
                kind: M16AfflictionKind::Pain,
                severity: new_pct,
                applied_at_tick: tick,
                expected_clear_tick: None,
                source_event_id: None,
            });
            new_pct >= 0.8
        };
        if crossed_critical && !state.critical_banner_pending.contains(&M16AfflictionKind::Pain) {
            state.critical_banner_pending.push(M16AfflictionKind::Pain);
        }
    }

    Some(PainStackChangedEvent {
        actor_id,
        tick,
        old_stack,
        new_stack,
        severity: new_pct,
        aim_wobble_multiplier: pain_aim_wobble_multiplier(new_stack),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_wound::{Wound, WoundId, WoundKind, ZoneId};

    fn wound_list(wounds: &[(WoundKind, f32)]) -> ActorWoundList {
        let mut list = ActorWoundList::new();
        let zone = ZoneId("torso".to_string());
        for (i, (kind, severity)) in wounds.iter().enumerate() {
            list.push(zone.clone(), Wound::new(WoundId(i as u64), *kind, *severity, zone.clone()));
        }
        list
    }

    #[test]
    fn scenario_1_three_wounds_stack_to_18_severity_0_6_wobble_1_6() {
        // Given 3 wounds: LacerationModerate(0.5), Burn2nd(0.6), FractureSimple(0.4).
        let wounds = wound_list(&[
            (WoundKind::LacerationModerate, 0.5),
            (WoundKind::Burn2nd, 0.6),
            (WoundKind::FractureSimple, 0.4),
        ]);
        let mut state = ActorAfflictions::default();
        let ev = recompute_pain(&mut state, 7, &wounds, 100).expect("pain.stack_changed fires");

        assert!((ev.new_stack - 18.0).abs() < 1e-4, "new_stack {} != 18", ev.new_stack);
        assert!((ev.severity - 0.6).abs() < 1e-4, "severity {} != 0.6", ev.severity);
        assert!(
            (ev.aim_wobble_multiplier - 1.6).abs() < 1e-4,
            "aim wobble {} != 1.6",
            ev.aim_wobble_multiplier
        );
        // And the M16 affliction.pain shows severity 0.6.
        assert!((state.severity_of(M16AfflictionKind::Pain) - 0.6).abs() < 1e-4);
    }

    #[test]
    fn single_moderate_laceration_is_6_points() {
        let wounds = wound_list(&[(WoundKind::LacerationModerate, 0.5)]);
        assert!((pain_stack_from_wounds(&wounds) - 6.0).abs() < 1e-4);
    }

    #[test]
    fn effect_curves_match_spec() {
        // Wobble: 1.0 at no pain, 1.6 at stack 18, saturates 2.0 at/above 30.
        assert!((pain_aim_wobble_multiplier(0.0) - 1.0).abs() < 1e-6);
        assert!((pain_aim_wobble_multiplier(18.0) - 1.6).abs() < 1e-6);
        assert!((pain_aim_wobble_multiplier(30.0) - 2.0).abs() < 1e-6);
        assert!((pain_aim_wobble_multiplier(80.0) - 2.0).abs() < 1e-6);
        // Move speed: ×(1 − pct×0.3). At full pain → 0.7.
        assert!((pain_move_speed_factor(30.0) - 0.7).abs() < 1e-6);
        assert!((pain_move_speed_factor(0.0) - 1.0).abs() < 1e-6);
        // Morale drain: −1 once ≥ severe (pct ≥ 0.5 ⇒ stack ≥ 15).
        assert_eq!(pain_morale_drain_per_tick(15.0), -1.0);
        assert_eq!(pain_morale_drain_per_tick(14.0), 0.0);
        // Auto-triage: > 80% ⇒ stack > 24.
        assert!(pain_triggers_autotriage(25.0));
        assert!(!pain_triggers_autotriage(24.0));
    }

    #[test]
    fn recompute_is_idempotent_then_clears() {
        let wounds = wound_list(&[(WoundKind::Burn2nd, 0.6)]);
        let mut state = ActorAfflictions::default();
        assert!(recompute_pain(&mut state, 7, &wounds, 0).is_some());
        // Same wounds again → no change, no event.
        assert!(recompute_pain(&mut state, 7, &wounds, 5).is_none());
        // Wounds heal → pain clears with a final stack=0 event.
        let healed = ActorWoundList::new();
        let ev = recompute_pain(&mut state, 7, &healed, 10).expect("clears with event");
        assert!((ev.new_stack - 0.0).abs() < 1e-6);
        assert!(state.find(M16AfflictionKind::Pain).is_none());
    }

    #[test]
    fn high_pain_sets_critical_banner_and_autotriage() {
        // Two severe wounds → stack 24 (0.8) then push higher to cross 0.8.
        let wounds = wound_list(&[(WoundKind::LacerationSevere, 0.9), (WoundKind::Burn3rd, 0.9)]);
        let mut state = ActorAfflictions::default();
        recompute_pain(&mut state, 7, &wounds, 0);
        // stack = (0.9+0.9)*12 = 21.6 → pct 0.72 — below autotriage.
        assert!(!pain_triggers_autotriage(pain_stack_from_wounds(&wounds)));
        // Add a third severe wound to exceed 80%.
        let wounds2 = wound_list(&[
            (WoundKind::LacerationSevere, 0.9),
            (WoundKind::Burn3rd, 0.9),
            (WoundKind::FractureCompound, 0.9),
        ]);
        recompute_pain(&mut state, 7, &wounds2, 5);
        assert!(state.severity_of(M16AfflictionKind::Pain) >= 0.8);
        assert!(state.critical_banner_pending.contains(&M16AfflictionKind::Pain));
    }

    #[test]
    fn pain_event_round_trips_through_json() {
        let ev = PainStackChangedEvent {
            actor_id: 7,
            tick: 100,
            old_stack: 0.0,
            new_stack: 18.0,
            severity: 0.6,
            aim_wobble_multiplier: 1.6,
        };
        let s = serde_json::to_string(&ev).unwrap();
        let back: PainStackChangedEvent = serde_json::from_str(&s).unwrap();
        assert_eq!(ev, back);
    }
}
