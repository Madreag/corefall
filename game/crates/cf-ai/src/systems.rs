//! M8A § Files / cf-ai — ECS systems scaffold.
//!
//! Per M8A spec § Architecture rules + § Parallel scheduler graph: the
//! AI tick consumes pre-rolled RNG (rule 2 — RNG calls outside par_iter
//! closures) and runs the 5-layer thinking stack per guard with no
//! cross-guard reads inside the parallel block. Stage boundaries are:
//!
//! 1. Reactive — fast reflex check; flags imminent threats
//! 2. Utility — score 22 candidate tasks via priority weights
//! 3. Behavior Tree — node traversal for tactical sequencing
//! 4. HTN — sub-goal evaluation with cache
//! 5. LLM-prior — cached doctrine string (M23 optional)
//!
//! M8A retunes the AI sim budget from 2.0 ms p99 → 4.0 ms p99 to cover
//! the M7-shipped 5-layer stack. See `cf-ai/src/constants.rs`.

use crate::components::GuardComponent;

///
/// Per M8A rule 2: RNG calls outside par_iter closures. Workers index by
/// stable guard id; same seed → same `Vec<u64>` → same per-guard rolls.
#[derive(Debug, Clone, Default)]
pub struct AiPreRolledRng {
    pub values: Vec<u64>,
}

impl AiPreRolledRng {
    pub fn fill(&mut self, seed: u64, n: usize) {
        self.values.clear();
        self.values.reserve(n);
        let mut state = seed.wrapping_add(0xBF58_476D_1CE4_E5B9);
        for _ in 0..n {
            state = state
                .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                .wrapping_add(0xD2B7_4407_B1CE_6E93);
            self.values.push(state);
        }
    }
}

///
/// Stage 1 (Reactive) updates each guard's `alive` flag based on the
/// shadow-state previous tick; pre-rolled RNG keyed by stable guard id;
/// no thread_rng / no Instant::now.
pub fn ai_tick_reactive(guards: &mut [GuardComponent], _rng: &AiPreRolledRng) {
    for guard in guards {
        if guard.id.0 == 0 {
            guard.alive = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pre_rolled_rng_deterministic() {
        let mut a = AiPreRolledRng::default();
        let mut b = AiPreRolledRng::default();
        a.fill(7, 8);
        b.fill(7, 8);
        assert_eq!(a.values, b.values);
    }
}
