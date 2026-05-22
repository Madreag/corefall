//! M7-B: cf-mind interface pre-lock.
//!
//! Spec § 5-layer thinking stack — Layer 5 is an optional async producer
//! that publishes "doctrine priors" (e.g. `"press_attack_north_wall"`,
//! `"fall_back_and_regroup"`) the deterministic Layer 5 cache consumes
//! once per AI tick. The producer is the future `cf-mind` crate which
//! ships at M23 with a real LLM backend. M7-B locks the INTERFACE here in
//! cf-ai so cf-control + downstream milestones can wire the surface
//! without a circular dep on the not-yet-existing cf-mind crate.
//!
//! Contract:
//!
//! - `LlmMind::query(prompt, context, deadline_ticks) -> Option<Doctrine>`
//!   is the single entry point. Returning `None` is a legitimate response
//!   (mind is offline, over budget, or the query is non-actionable) — the
//!   5-layer stack falls back to BT/HTN-only without surfacing a gap.
//! - `Doctrine` is a small structured value (label + optional metadata)
//!   the layer caches and surfaces via `ctx.doctrine`.
//! - `MindContext` is a read-only snapshot the producer can inspect.
//!   It MUST stay cheap to construct so the engine can produce one per
//!   tick (or per cadence) without overhead.
//! - `NullLlmMind` is the default in M7 — always returns `None`. M23 swaps
//!   in `cf_mind::OpusMind` (or similar) that calls into a real model.

use serde::{Deserialize, Serialize};

use crate::archetype::Archetype;
use crate::autonomy::{AutonomyMode, DoctrineMode};
use crate::task::TaskType;

/// the interface — `label` carries the doctrine string the 5-layer stack
/// caches via `ctx.doctrine`. M23's cf-mind adds optional metadata
/// (confidence, alternates, justification) without breaking the contract;
/// extending this struct is forward-compatible because cf-mind owns the
/// implementation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Doctrine {
    /// Short label, lowercase snake_case (e.g. `"defensive_hold"`,
    /// `"press_attack_north_wall"`). Caller compares for equality to
    /// detect "new" doctrine and trigger chatter / event emissions.
    pub label: String,
    /// Optional confidence in `[0.0, 1.0]`. M7-B accepts any finite value;
    /// M23 / downstream consumers may quantize for replay determinism.
    pub confidence: f32,
    /// Optional bias hint: when present, the layer can boost the matching
    /// task in the utility scorer's prior. M7 ignores this hint (Layer 5
    /// only writes `ctx.doctrine`); M23 will let cf-mind hint a task.
    pub task_hint: Option<TaskType>,
}

impl Doctrine {
    /// Build a label-only Doctrine with confidence 1.0 and no task hint.
    /// Convenience for tests + mind stubs that don't yet emit confidence.
    pub fn from_label(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            confidence: 1.0,
            task_hint: None,
        }
    }
}

/// mind producer inspects this snapshot to choose a doctrine — but MUST
/// NOT mutate the world.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MindContext {
    /// Tick the engine produced this context. Producers may use it to age
    /// out stale results.
    pub tick: u64,
    /// Configured tick rate (Hz). Useful when converting deadline ticks
    /// into wall-clock budgets.
    pub tick_rate_hz: u32,
    /// Actor the mind is reasoning for.
    pub actor_id: u64,
    /// Bot's current archetype role.
    pub archetype: Archetype,
    /// Bot's current autonomy mode.
    pub autonomy: AutonomyMode,
    /// Bot's current doctrine mode (squad-level).
    pub doctrine_mode: DoctrineMode,
    /// HP fraction `[0, 1]`.
    pub self_hp_fraction: f32,
    /// Mood normalized to `[-1, 1]`.
    pub mood_normalized: f32,
    /// Whether the bot currently sees an enemy.
    pub enemy_visible: bool,
}

impl MindContext {
    /// Build a stub context for tests + null minds.
    pub fn stub() -> Self {
        Self {
            tick: 0,
            tick_rate_hz: 60,
            actor_id: 0,
            archetype: Archetype::Rifleman,
            autonomy: AutonomyMode::FullAuto,
            doctrine_mode: DoctrineMode::Defensive,
            self_hp_fraction: 1.0,
            mood_normalized: 0.0,
            enemy_visible: false,
        }
    }
}

/// impl; M7 ships only the interface signature so the 5-layer stack can
/// route through this trait without a runtime dependency.
///
/// Implementations MUST be `Send + Sync` so the engine can park the mind
/// behind a `Box<dyn LlmMind>` and tick it from any thread M8A schedules
/// the AI subsystem on.
///
/// Returning `None` is a legitimate response (mind is offline, budget
/// exhausted, query non-actionable). Callers MUST handle `None` and fall
/// back to BT/HTN-only behaviour.
pub trait LlmMind: Send + Sync {
    /// Query the mind for a doctrine. `prompt` carries the high-level
    /// situation description (cf-mind's prompt builder owns this string);
    /// `context` is the engine snapshot for the bot; `deadline_ticks` is
    /// the maximum number of ticks the mind may spend before yielding
    /// (callers can interpret as "p99 budget"). Returning `None` lets the
    /// 5-layer stack degrade gracefully to Layers 1-4 only.
    fn query(&self, prompt: &str, context: &MindContext, deadline_ticks: u32) -> Option<Doctrine>;
}

/// every bot's Layer 5 falls back to the cached "defensive" prior.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullLlmMind;

impl LlmMind for NullLlmMind {
    fn query(&self, _prompt: &str, _context: &MindContext, _deadline_ticks: u32) -> Option<Doctrine> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_mind_always_returns_none() {
        let mind = NullLlmMind;
        let ctx = MindContext::stub();
        assert!(mind.query("anything", &ctx, 30).is_none());
    }

    #[test]
    fn doctrine_round_trips_through_serde() {
        let d = Doctrine {
            label: "press_attack_north_wall".to_string(),
            confidence: 0.92,
            task_hint: Some(TaskType::FlankTarget),
        };
        let json = serde_json::to_string(&d).expect("serialize");
        let back: Doctrine = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(d, back);
    }

    #[test]
    fn doctrine_from_label_defaults_confidence_to_one() {
        let d = Doctrine::from_label("defensive");
        assert_eq!(d.label, "defensive");
        assert!((d.confidence - 1.0).abs() < f32::EPSILON);
        assert!(d.task_hint.is_none());
    }

    /// Spec contract: BT/HTN-only fallback path stays correct when the
    /// mind returns `None`. The 5-layer stack runs Layers 1-4 normally
    /// and the LlmPriorLayer caches its previous prior (or default).
    #[test]
    fn null_mind_fallback_keeps_bt_htn_path_correct() {
        let mind: Box<dyn LlmMind> = Box::new(NullLlmMind);
        let ctx = MindContext::stub();
        assert!(mind.query("p", &ctx, 30).is_none());
    }

    /// Custom mind impl that returns `Some(...)` exercises the success
    /// path of the trait surface.
    #[test]
    fn custom_mind_can_return_doctrine() {
        struct StubMind;
        impl LlmMind for StubMind {
            fn query(&self, prompt: &str, _ctx: &MindContext, _deadline: u32) -> Option<Doctrine> {
                if prompt.contains("attack") {
                    Some(Doctrine::from_label("press_attack"))
                } else {
                    None
                }
            }
        }
        let mind = StubMind;
        let ctx = MindContext::stub();
        assert_eq!(
            mind.query("attack the north wall", &ctx, 30).map(|d| d.label),
            Some("press_attack".to_string())
        );
        assert!(mind.query("idle", &ctx, 30).is_none());
    }
}
