//! M7-A Layer 5: LLM-prior cache (INTERFACE only).
//!
//! Spec § 5-layer thinking stack — Layer 5 is the optional LLM mind that
//! ticks asynchronously at 5s cadence to produce a doctrine prior string
//! ("press_attack_north_wall", "fall_back_and_regroup", etc.) cached in
//! `BotMemory.doctrine_prior`. The full implementation ships at M23 in the
//! new `cf-mind` crate; M7-A ships the INTERFACE so the thinking stack
//! consumes the prior without a hard dependency on the async producer.
//!
//! When the LLM mind is off (default at M7), the layer returns a constant
//! `"defensive"` prior — preserving determinism. When it ships in M23, the
//! producer writes to `last_prior` on its own cadence; this layer just
//! reads.

use serde::{Deserialize, Serialize};

use crate::thinking_stack::{Layer, LayerKind, LayerOutput, ThinkingContext};

/// **M7-A**: doctrine prior produced by Layer 5. M7-A ships a small fixed
/// vocabulary; M23's cf-mind extends with LLM-generated strings (still
/// quantized via the LayerCache to stay deterministic per-run).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DoctrinePrior {
    /// Short label, lowercase snake_case (e.g. "defensive_hold").
    pub label: String,
    /// Tick on which the producer last wrote this prior.
    pub last_update_tick: u64,
}

impl DoctrinePrior {
    pub fn defensive() -> Self {
        Self {
            label: "defensive".to_string(),
            last_update_tick: 0,
        }
    }
}

/// **M7-A**: Layer 5 LLM-prior cache. INTERFACE only — M7 does not invoke
/// any LLM. When enabled (`enabled = true`), the producer writes to
/// `last_prior` on its own cadence. The layer's `tick_layer` exposes the
/// current cached prior via `ctx.doctrine`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmPriorLayer {
    pub enabled: bool,
    pub last_prior: DoctrinePrior,
    /// Cadence (ticks) between producer writes. Default = 5 sec at 60 Hz.
    pub producer_cadence_ticks: u32,
}

impl LlmPriorLayer {
    pub fn new() -> Self {
        Self {
            enabled: false,
            last_prior: DoctrinePrior::defensive(),
            producer_cadence_ticks: 5 * 60,
        }
    }

    /// **M23 hook**: the producer calls this when the LLM mind generates a
    /// new doctrine string. Re-writing with the same label is a no-op.
    pub fn write_prior(&mut self, label: impl Into<String>, tick: u64) {
        let next = label.into();
        if next != self.last_prior.label {
            self.last_prior = DoctrinePrior {
                label: next,
                last_update_tick: tick,
            };
        }
    }
}

impl Default for LlmPriorLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Layer for LlmPriorLayer {
    fn kind(&self) -> LayerKind {
        LayerKind::LlmPrior
    }

    fn tick_layer(&mut self, ctx: &mut ThinkingContext<'_>) -> LayerOutput {
        ctx.doctrine = self.last_prior.label.clone();
        LayerOutput {
            override_task: None,
            reason: if self.enabled {
                "llm_prior_cached"
            } else {
                "llm_prior_disabled"
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thinking_stack::ThinkingContext;

    #[test]
    fn disabled_returns_defensive_default() {
        let mut layer = LlmPriorLayer::new();
        let mut ctx = ThinkingContext::stub();
        let out = layer.tick_layer(&mut ctx);
        assert_eq!(ctx.doctrine, "defensive");
        assert_eq!(out.reason, "llm_prior_disabled");
    }

    #[test]
    fn producer_write_updates_prior() {
        let mut layer = LlmPriorLayer::new();
        layer.enabled = true;
        layer.write_prior("press_attack_north_wall", 360);
        let mut ctx = ThinkingContext::stub();
        let out = layer.tick_layer(&mut ctx);
        assert_eq!(ctx.doctrine, "press_attack_north_wall");
        assert_eq!(out.reason, "llm_prior_cached");
    }
}
