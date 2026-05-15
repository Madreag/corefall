//! M8A § Performance budgets — cf-render-2d's locked per-tick latency
//! budget.

/// **M8A**: per-tick render dispatch p99 latency budget (milliseconds).
///
/// Tightened from 4.0 ms (M1-M7) to 3.5 ms after GPU compute particle
/// offload + Texture2DArray terrain upload. Owned by
/// `docs/plan/spec/perf-budget-contract.md`.
pub const RENDER_DISPATCH_P99_BUDGET_MS: f32 = 3.5;
