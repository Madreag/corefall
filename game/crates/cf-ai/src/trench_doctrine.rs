//! M9B: AI-TRENCH-A-01 doctrine — garrison + burst-and-duck.
//!
//! Spec §"Acceptance criteria":
//!
//! > Scenario: AI doctrine garrisons trench and uses cover state correctly
//! >   Given m9b_ai_in_trench_doctrine scenario: 3 AI defenders in a fire_step segment line
//! >   When an enemy advances within engagement range
//! >   Then AI-TRENCH-A-01 doctrine has each AI: step up (Exposed) → fire 1-3 round burst → step down (Full)
//! >   And ai.cover_decision event fires with reason_label="step_up_for_shot" or "step_down_to_reload"
//! >   And no AI remains Exposed continuously > 1.5 seconds (correct burst-and-duck behavior)
//!
//! VAL-M9B-AI-001 + VAL-M9B-AICOVER-001 + the m9b-3 feature spec spell
//! out 4 reason labels: `step_up_for_shot`, `step_down_to_reload`,
//! `hold_full_cover`, `reload_safe`.
//!
//! This module owns the pure decision function. The engine drives the
//! per-tick `tick(decision_inputs)` and writes the resulting
//! `ai_cover_decision` event through `cf-replay::Recorder` (the
//! `cosmetic=false` flag is set in `cf-replay/schemas/event/ai_cover_decision.json`
//! per VAL-M9B-AICOVER-001).
//!
//! Determinism: the doctrine threads a `cf_sim_core::Rng` to roll the
//! burst length (1..=3 rounds) so two engines with the same seed
//! produce identical cover-decision sequences (DR-052).

use serde::{Deserialize, Serialize};

use cf_actor::ActorId;
use cf_sim_core::Rng;
use cf_trench::CoverState;

/// Doctrine id used in the ai_cover_decision event payload (`doctrine`
/// field) and in scenario RON files that opt actors into this surface.
pub const DOCTRINE_ID: &str = "AI-TRENCH-A-01";

/// Maximum continuous exposure window in in-game seconds. The doctrine
/// forces a `step_down_to_reload` decision when this is exceeded so the
/// "no AI remains Exposed continuously > 1.5s" invariant holds.
pub const MAX_EXPOSURE_SECONDS: f32 = 1.5;

/// One of the 4 spec-mandated reason labels surfaced via the
/// `ai_cover_decision` event's `reason_label` field. Wire-form strings
/// match the spec verbatim.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverDecisionReason {
    /// AI is stepping up (Exposed) to fire a 1-3 round burst.
    StepUpForShot = 0,
    /// AI is stepping down (Full) to reload safely.
    StepDownToReload = 1,
    /// AI stays in Full cover this tick (no fire decision).
    HoldFullCover = 2,
    /// AI is reloading while remaining in Full cover.
    ReloadSafe = 3,
}

impl CoverDecisionReason {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            CoverDecisionReason::StepUpForShot => "step_up_for_shot",
            CoverDecisionReason::StepDownToReload => "step_down_to_reload",
            CoverDecisionReason::HoldFullCover => "hold_full_cover",
            CoverDecisionReason::ReloadSafe => "reload_safe",
        }
    }

    /// True when the decision forces the AI into the Exposed firing
    /// posture this tick. Used by the engine to flip the `on_step` flag
    /// on the fire_step segment derivation.
    #[must_use]
    pub const fn forces_exposed(self) -> bool {
        matches!(self, CoverDecisionReason::StepUpForShot)
    }

    /// True when the decision should drop the AI back to Full cover.
    #[must_use]
    pub const fn forces_full_cover(self) -> bool {
        matches!(
            self,
            CoverDecisionReason::StepDownToReload
                | CoverDecisionReason::HoldFullCover
                | CoverDecisionReason::ReloadSafe
        )
    }
}

/// Per-tick inputs the engine passes to [`TrenchDoctrine::tick`]. All
/// fields are derived from cf-actor / cf-ai state; the doctrine itself
/// is purely functional.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrenchDoctrineInputs {
    pub actor_id: ActorId,
    /// Current cover state at the actor's tile. Derived via
    /// `cf_trench::cover_state(stance, segment.variant)` upstream.
    pub current_cover_state: CoverState,
    /// True when an enemy is within engagement range with line-of-sight.
    pub enemy_in_range_with_los: bool,
    /// Current magazine ammo (rounds remaining).
    pub current_ammo: u32,
    /// Magazine capacity (used to gate `step_down_to_reload`).
    pub mag_capacity: u32,
    /// Ticks since the actor last entered the Exposed cover state. The
    /// engine resets to 0 each time the doctrine forces an
    /// `StepUpForShot` and increments per tick while Exposed.
    pub exposure_ticks: u32,
    /// Tick rate (Hz). Used to convert [`MAX_EXPOSURE_SECONDS`] to a
    /// tick count. Never hardcode `60` per project AGENTS.md.
    pub tick_rate_hz: u32,
    /// True when the actor is mid-reload (i.e., reload state was
    /// initiated and reload_remaining_ticks > 0).
    pub reload_in_progress: bool,
}

/// The doctrine's per-tick output. The engine consumes this to:
///   - record an `ai.cover_decision` event with `reason_label`,
///   - flip the actor's posture (step up / step down),
///   - dispatch the fire-burst action when `burst_rounds > 0`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverDecision {
    pub actor_id: ActorId,
    pub reason: CoverDecisionReason,
    pub prev_cover_state: CoverState,
    pub new_cover_state: CoverState,
    /// Planned burst length when `reason == StepUpForShot`. Otherwise 0.
    pub burst_rounds: u32,
}

impl CoverDecision {
    /// Stable wire-format string for the `reason_label` field on the
    /// `ai_cover_decision` event JSON.
    #[must_use]
    pub fn reason_label(&self) -> &'static str {
        self.reason.as_str()
    }
}

/// Configuration knobs for the doctrine. Defaults match the spec
/// implementer's notes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrenchDoctrineConfig {
    /// Minimum burst length on a `StepUpForShot` decision.
    pub min_burst_rounds: u32,
    /// Maximum burst length on a `StepUpForShot` decision.
    pub max_burst_rounds: u32,
    /// Maximum continuous exposure in seconds before forcing a
    /// step-down. Defaults to [`MAX_EXPOSURE_SECONDS`].
    pub max_exposure_seconds: f32,
    /// Ammo threshold below which the doctrine prefers
    /// `step_down_to_reload` over `step_up_for_shot`.
    pub low_ammo_threshold: u32,
}

impl Default for TrenchDoctrineConfig {
    fn default() -> Self {
        Self {
            min_burst_rounds: 1,
            max_burst_rounds: 3,
            max_exposure_seconds: MAX_EXPOSURE_SECONDS,
            low_ammo_threshold: 2,
        }
    }
}

/// Stateless decision tree for AI-TRENCH-A-01. `tick` returns the
/// cover-decision for the current tick; the engine writes the
/// corresponding event + flips the actor posture.
#[derive(Debug, Clone, Copy, Default)]
pub struct TrenchDoctrine {
    pub config: TrenchDoctrineConfig,
}

impl TrenchDoctrine {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_config(config: TrenchDoctrineConfig) -> Self {
        Self { config }
    }

    /// Per-tick decision step. Caller passes a seeded RNG so two
    /// engines with the same seed produce identical sequences.
    pub fn tick(&self, inputs: TrenchDoctrineInputs, rng: &mut Rng) -> CoverDecision {
        let max_exposure_ticks = self
            .max_exposure_ticks_for(inputs.tick_rate_hz)
            .max(1);

        // Hard guard: if the actor has been Exposed continuously beyond
        // the configured window, force a step-down regardless of other
        // signals. This is the invariant the spec scenario asserts:
        // "no AI remains Exposed continuously > 1.5 seconds".
        if matches!(inputs.current_cover_state, CoverState::Exposed)
            && inputs.exposure_ticks >= max_exposure_ticks
        {
            return CoverDecision {
                actor_id: inputs.actor_id,
                reason: CoverDecisionReason::StepDownToReload,
                prev_cover_state: inputs.current_cover_state,
                new_cover_state: CoverState::Full,
                burst_rounds: 0,
            };
        }

        // Low-ammo policy: when the actor is below the low-ammo
        // threshold OR already mid-reload, prefer to stay in Full
        // cover and reload — never expose.
        if inputs.current_ammo <= self.config.low_ammo_threshold
            || inputs.reload_in_progress
        {
            // If currently Exposed, drop down.
            if matches!(inputs.current_cover_state, CoverState::Exposed) {
                return CoverDecision {
                    actor_id: inputs.actor_id,
                    reason: CoverDecisionReason::StepDownToReload,
                    prev_cover_state: inputs.current_cover_state,
                    new_cover_state: CoverState::Full,
                    burst_rounds: 0,
                };
            }
            return CoverDecision {
                actor_id: inputs.actor_id,
                reason: CoverDecisionReason::ReloadSafe,
                prev_cover_state: inputs.current_cover_state,
                new_cover_state: CoverState::Full,
                burst_rounds: 0,
            };
        }

        // Enemy in range with LOS — burst-and-duck. Roll a 1..=3 round
        // burst from the seeded RNG.
        if inputs.enemy_in_range_with_los {
            let burst = self.roll_burst_length(rng);
            return CoverDecision {
                actor_id: inputs.actor_id,
                reason: CoverDecisionReason::StepUpForShot,
                prev_cover_state: inputs.current_cover_state,
                new_cover_state: CoverState::Exposed,
                burst_rounds: burst,
            };
        }

        // No enemy in range; if Exposed, drop back to Full.
        if matches!(inputs.current_cover_state, CoverState::Exposed) {
            return CoverDecision {
                actor_id: inputs.actor_id,
                reason: CoverDecisionReason::StepDownToReload,
                prev_cover_state: inputs.current_cover_state,
                new_cover_state: CoverState::Full,
                burst_rounds: 0,
            };
        }

        // Default: hold Full cover this tick.
        CoverDecision {
            actor_id: inputs.actor_id,
            reason: CoverDecisionReason::HoldFullCover,
            prev_cover_state: inputs.current_cover_state,
            new_cover_state: CoverState::Full,
            burst_rounds: 0,
        }
    }

    /// Roll a 1..=3 round burst length from the seeded RNG. Uses a
    /// fixed mapping: low 1/3 → min, mid 1/3 → mid, high 1/3 → max.
    fn roll_burst_length(&self, rng: &mut Rng) -> u32 {
        let span = self
            .config
            .max_burst_rounds
            .saturating_sub(self.config.min_burst_rounds);
        if span == 0 {
            return self.config.min_burst_rounds;
        }
        let raw = rng.next_u64();
        let offset = (raw % u64::from(span + 1)) as u32;
        self.config.min_burst_rounds + offset
    }

    /// Convert [`MAX_EXPOSURE_SECONDS`] to a tick count given the engine
    /// tick rate. `tick_rate_hz` must be `>= 1` per project AGENTS.md.
    pub fn max_exposure_ticks_for(&self, tick_rate_hz: u32) -> u32 {
        let tick_rate = tick_rate_hz.max(1) as f32;
        (self.config.max_exposure_seconds * tick_rate).ceil() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_sim_core::Rng;

    fn rng() -> Rng {
        Rng::from_seed(0xCAFE_BABE_DEAD_BEEF)
    }

    fn baseline_inputs(cover: CoverState, enemy: bool) -> TrenchDoctrineInputs {
        TrenchDoctrineInputs {
            actor_id: ActorId(42),
            current_cover_state: cover,
            enemy_in_range_with_los: enemy,
            current_ammo: 30,
            mag_capacity: 30,
            exposure_ticks: 0,
            tick_rate_hz: 60,
            reload_in_progress: false,
        }
    }

    /// VAL-M9B-AI-001: enemy in range → step_up_for_shot with 1..=3 burst.
    #[test]
    fn trench_doctrine_step_up_for_shot_on_enemy_in_range() {
        let doctrine = TrenchDoctrine::new();
        let mut r = rng();
        let inputs = baseline_inputs(CoverState::Full, true);
        let d = doctrine.tick(inputs, &mut r);
        assert_eq!(d.reason, CoverDecisionReason::StepUpForShot);
        assert_eq!(d.reason_label(), "step_up_for_shot");
        assert!(d.burst_rounds >= 1 && d.burst_rounds <= 3);
        assert_eq!(d.new_cover_state, CoverState::Exposed);
    }

    /// VAL-M9B-AI-001: no enemy in range → hold_full_cover.
    #[test]
    fn trench_doctrine_hold_full_cover_when_no_enemy() {
        let doctrine = TrenchDoctrine::new();
        let mut r = rng();
        let inputs = baseline_inputs(CoverState::Full, false);
        let d = doctrine.tick(inputs, &mut r);
        assert_eq!(d.reason, CoverDecisionReason::HoldFullCover);
    }

    /// VAL-M9B-AI-001: max-exposure window enforces step_down_to_reload
    /// even if enemy still in range.
    #[test]
    fn trench_doctrine_step_down_after_max_exposure() {
        let doctrine = TrenchDoctrine::new();
        let mut r = rng();
        let max_ticks = doctrine.max_exposure_ticks_for(60);
        let inputs = TrenchDoctrineInputs {
            current_cover_state: CoverState::Exposed,
            enemy_in_range_with_los: true,
            exposure_ticks: max_ticks,
            ..baseline_inputs(CoverState::Exposed, true)
        };
        let d = doctrine.tick(inputs, &mut r);
        assert_eq!(d.reason, CoverDecisionReason::StepDownToReload);
        assert_eq!(d.new_cover_state, CoverState::Full);
        assert_eq!(d.burst_rounds, 0);
    }

    /// VAL-M9B-AI-001: low-ammo + currently Exposed forces step_down.
    #[test]
    fn trench_doctrine_step_down_on_low_ammo_while_exposed() {
        let doctrine = TrenchDoctrine::new();
        let mut r = rng();
        let inputs = TrenchDoctrineInputs {
            current_cover_state: CoverState::Exposed,
            current_ammo: 1,
            ..baseline_inputs(CoverState::Exposed, true)
        };
        let d = doctrine.tick(inputs, &mut r);
        assert_eq!(d.reason, CoverDecisionReason::StepDownToReload);
    }

    /// VAL-M9B-AI-001: low-ammo while in Full cover → reload_safe.
    #[test]
    fn trench_doctrine_reload_safe_on_low_ammo_in_full_cover() {
        let doctrine = TrenchDoctrine::new();
        let mut r = rng();
        let inputs = TrenchDoctrineInputs {
            current_cover_state: CoverState::Full,
            current_ammo: 0,
            ..baseline_inputs(CoverState::Full, true)
        };
        let d = doctrine.tick(inputs, &mut r);
        assert_eq!(d.reason, CoverDecisionReason::ReloadSafe);
        assert_eq!(d.burst_rounds, 0);
    }

    /// VAL-M9B-AI-001: reload_in_progress + Full cover → reload_safe.
    #[test]
    fn trench_doctrine_reload_safe_on_reload_in_progress() {
        let doctrine = TrenchDoctrine::new();
        let mut r = rng();
        let inputs = TrenchDoctrineInputs {
            current_cover_state: CoverState::Full,
            reload_in_progress: true,
            current_ammo: 30,
            ..baseline_inputs(CoverState::Full, true)
        };
        let d = doctrine.tick(inputs, &mut r);
        assert_eq!(d.reason, CoverDecisionReason::ReloadSafe);
    }

    /// VAL-M9B-AI-001: no enemy in range + currently Exposed → step_down.
    #[test]
    fn trench_doctrine_step_down_when_exposed_no_enemy() {
        let doctrine = TrenchDoctrine::new();
        let mut r = rng();
        let inputs = TrenchDoctrineInputs {
            current_cover_state: CoverState::Exposed,
            enemy_in_range_with_los: false,
            ..baseline_inputs(CoverState::Exposed, false)
        };
        let d = doctrine.tick(inputs, &mut r);
        assert_eq!(d.reason, CoverDecisionReason::StepDownToReload);
    }

    /// Burst-and-duck end-to-end: starting Full, fire 3 ticks of
    /// StepUpForShot (each emits a burst), then trigger MaxExposure to
    /// force StepDownToReload. No tick lingers in Exposed > 1.5s.
    #[test]
    fn trench_doctrine_burst_and_duck() {
        let doctrine = TrenchDoctrine::new();
        let mut r = rng();
        let mut emitted = Vec::new();
        let mut exposure_ticks: u32 = 0;
        let max_ticks = doctrine.max_exposure_ticks_for(60);
        let mut cover = CoverState::Full;
        for tick in 0..max_ticks * 2 {
            let inputs = TrenchDoctrineInputs {
                current_cover_state: cover,
                enemy_in_range_with_los: true,
                exposure_ticks,
                current_ammo: 30,
                ..baseline_inputs(cover, true)
            };
            let d = doctrine.tick(inputs, &mut r);
            emitted.push(d.reason);
            cover = d.new_cover_state;
            if matches!(d.new_cover_state, CoverState::Exposed) {
                exposure_ticks += 1;
            } else {
                exposure_ticks = 0;
            }
            assert!(
                exposure_ticks <= max_ticks,
                "tick {tick}: exposure_ticks {exposure_ticks} exceeded max {max_ticks}"
            );
        }
        // Sequence must contain both step_up_for_shot AND
        // step_down_to_reload labels (VAL-M9B-AI-001 evidence).
        assert!(emitted.contains(&CoverDecisionReason::StepUpForShot));
        assert!(emitted.contains(&CoverDecisionReason::StepDownToReload));
    }

    /// Determinism: same seed → same decisions across two runs.
    #[test]
    fn trench_doctrine_decisions_deterministic_for_seed() {
        let doctrine = TrenchDoctrine::new();
        let inputs = baseline_inputs(CoverState::Full, true);
        let mut r1 = rng();
        let mut r2 = rng();
        let d1 = doctrine.tick(inputs, &mut r1);
        let d2 = doctrine.tick(inputs, &mut r2);
        assert_eq!(d1, d2, "doctrine must be deterministic per seed");
    }

    #[test]
    fn cover_decision_reason_wire_strings() {
        assert_eq!(CoverDecisionReason::StepUpForShot.as_str(), "step_up_for_shot");
        assert_eq!(CoverDecisionReason::StepDownToReload.as_str(), "step_down_to_reload");
        assert_eq!(CoverDecisionReason::HoldFullCover.as_str(), "hold_full_cover");
        assert_eq!(CoverDecisionReason::ReloadSafe.as_str(), "reload_safe");
    }

    #[test]
    fn cover_decision_reason_posture_forcing() {
        assert!(CoverDecisionReason::StepUpForShot.forces_exposed());
        assert!(!CoverDecisionReason::StepUpForShot.forces_full_cover());
        assert!(!CoverDecisionReason::StepDownToReload.forces_exposed());
        assert!(CoverDecisionReason::StepDownToReload.forces_full_cover());
        assert!(CoverDecisionReason::HoldFullCover.forces_full_cover());
        assert!(CoverDecisionReason::ReloadSafe.forces_full_cover());
    }

    #[test]
    fn max_exposure_ticks_scales_with_tick_rate() {
        let doctrine = TrenchDoctrine::new();
        assert_eq!(doctrine.max_exposure_ticks_for(60), 90);
        // 120 Hz engine doubles the tick budget.
        assert_eq!(doctrine.max_exposure_ticks_for(120), 180);
        // 30 Hz halves it.
        assert_eq!(doctrine.max_exposure_ticks_for(30), 45);
    }
}
