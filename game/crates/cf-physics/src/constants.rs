//! M8A § Performance budgets — cf-physics's locked per-tick latency budget.
//!
//! Owned by `docs/plan/spec/perf-budget-contract.md`. Downstream
//! milestones must not re-tune.

/// **M8A**: per-tick projectile sim p99 latency budget (milliseconds).
///
/// The projectile sweep + terrain penetration runs `par_iter` over the
/// projectile pool with snapshotted terrain reads (previous-tick state).
/// Budget covers the 200-projectile load in `bench_m9_firehose`.
pub const PROJECTILE_SIM_P99_BUDGET_MS: f32 = 1.0;
