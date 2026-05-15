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

use crate::cf_mind::{Doctrine, LlmMind, MindContext, NullLlmMind};
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
///
/// **M7-B**: optional `LlmMind` producer can be injected via `set_mind`.
/// When attached AND enabled, the layer calls `mind.query(...)` at the
/// configured cadence and caches the returned `Doctrine`. When the mind
/// returns `None`, the layer falls back to the previously cached prior
/// (still deterministic). The mind is `#[serde(skip)]` because it's a
/// runtime-only producer; snapshot/restore preserves only the cached
/// prior. M23's cf-mind crate ships the real producer.
#[derive(Serialize, Deserialize)]
pub struct LlmPriorLayer {
    pub enabled: bool,
    pub last_prior: DoctrinePrior,
    /// Cadence (ticks) between producer writes. Default = 5 sec at 60 Hz.
    pub producer_cadence_ticks: u32,
    /// **M7-B**: optional async producer. M7 default = `NullLlmMind` (always
    /// returns `None`). M23's cf-mind crate ships the real impl.
    #[serde(skip, default = "default_mind")]
    pub mind: Box<dyn LlmMind>,
}

fn default_mind() -> Box<dyn LlmMind> {
    Box::new(NullLlmMind)
}

impl std::fmt::Debug for LlmPriorLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmPriorLayer")
            .field("enabled", &self.enabled)
            .field("last_prior", &self.last_prior)
            .field("producer_cadence_ticks", &self.producer_cadence_ticks)
            .field("mind", &"<dyn LlmMind>")
            .finish()
    }
}

impl Clone for LlmPriorLayer {
    fn clone(&self) -> Self {
        Self {
            enabled: self.enabled,
            last_prior: self.last_prior.clone(),
            producer_cadence_ticks: self.producer_cadence_ticks,
            mind: default_mind(),
        }
    }
}

impl PartialEq for LlmPriorLayer {
    fn eq(&self, other: &Self) -> bool {
        self.enabled == other.enabled
            && self.last_prior == other.last_prior
            && self.producer_cadence_ticks == other.producer_cadence_ticks
    }
}

impl LlmPriorLayer {
    pub fn new() -> Self {
        Self {
            enabled: false,
            last_prior: DoctrinePrior::defensive(),
            producer_cadence_ticks: 5 * 60,
            mind: default_mind(),
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

    /// **M7-B**: install an `LlmMind` producer. Default is `NullLlmMind`
    /// (always returns `None`). M23's cf-mind crate calls this with its
    /// real producer.
    pub fn set_mind(&mut self, mind: Box<dyn LlmMind>) {
        self.mind = mind;
    }

    /// **M7-B**: query the attached `LlmMind` and, on `Some(Doctrine)`,
    /// cache the new label. Returning `None` is a legitimate fallback —
    /// the cached prior stays in place so Layer 5 still emits a stable
    /// doctrine string and Layers 1-4 continue normally (BT/HTN-only).
    /// Returns the doctrine the producer emitted (or `None`).
    pub fn query_mind(&mut self, prompt: &str, ctx: &MindContext, deadline_ticks: u32) -> Option<Doctrine> {
        if !self.enabled {
            return None;
        }
        let result = self.mind.query(prompt, ctx, deadline_ticks);
        if let Some(d) = result.as_ref() {
            self.write_prior(d.label.clone(), ctx.tick);
        }
        result
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
    use crate::cf_mind::{Doctrine, LlmMind, MindContext, NullLlmMind};
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

    /// **M7-B**: null mind keeps the BT/HTN-only fallback path intact.
    /// When the mind returns `None`, the cached prior stays at its
    /// previous (or default) value so Layer 5 still emits a stable
    /// doctrine string. Layers 1-4 continue ticking normally.
    #[test]
    fn null_mind_returns_none_and_keeps_cached_prior() {
        let mut layer = LlmPriorLayer::new();
        layer.enabled = true;
        layer.set_mind(Box::new(NullLlmMind));
        let mind_ctx = MindContext::stub();
        let result = layer.query_mind("anything", &mind_ctx, 30);
        assert!(result.is_none());
        // Cached prior unchanged.
        assert_eq!(layer.last_prior.label, "defensive");
    }

    /// **M7-B**: a custom mind that returns `Some(Doctrine)` updates the
    /// cached prior, which Layer 5 surfaces on the next `tick_layer`.
    #[test]
    fn custom_mind_writes_doctrine_through_query_mind() {
        struct StubMind;
        impl LlmMind for StubMind {
            fn query(&self, _prompt: &str, _ctx: &MindContext, _deadline: u32) -> Option<Doctrine> {
                Some(Doctrine::from_label("press_attack_north_wall"))
            }
        }
        let mut layer = LlmPriorLayer::new();
        layer.enabled = true;
        layer.set_mind(Box::new(StubMind));
        let mind_ctx = MindContext::stub();
        let result = layer.query_mind("attack", &mind_ctx, 30);
        assert!(result.is_some());
        assert_eq!(layer.last_prior.label, "press_attack_north_wall");
        // tick_layer now surfaces the new prior.
        let mut ctx = ThinkingContext::stub();
        let out = layer.tick_layer(&mut ctx);
        assert_eq!(ctx.doctrine, "press_attack_north_wall");
        assert_eq!(out.reason, "llm_prior_cached");
    }

    /// **M7-B**: a disabled layer does not query the mind even if one is
    /// attached — preserves the M7 "off by default" contract.
    #[test]
    fn disabled_layer_does_not_query_mind() {
        struct PanicMind;
        impl LlmMind for PanicMind {
            fn query(&self, _prompt: &str, _ctx: &MindContext, _deadline: u32) -> Option<Doctrine> {
                panic!("mind queried while disabled");
            }
        }
        let mut layer = LlmPriorLayer::new();
        layer.set_mind(Box::new(PanicMind));
        let mind_ctx = MindContext::stub();
        let result = layer.query_mind("anything", &mind_ctx, 30);
        assert!(result.is_none());
    }
}
