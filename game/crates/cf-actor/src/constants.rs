//! M8A § Performance budgets — cf-actor's locked per-tick latency budget.
//!
//! Owned by `docs/plan/spec/perf-budget-contract.md`. Downstream
//! milestones must not re-tune.

/// **M8A**: per-tick actor sim p99 latency budget (milliseconds).
///
/// The actor sub-systems (`apply_intent`, `step_kinematics`, `derive_status`,
/// `latch_outcomes`) collectively must complete within 1.5 ms p99 on the
/// reference platform (9950X3D + RTX 5090 + 48 GB DDR5) when driving 50
/// actors via `par_iter` over `ActorBundle` entities with pre-rolled RNG.
pub const ACTOR_SIM_P99_BUDGET_MS: f32 = 1.5;
