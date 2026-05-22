//! M8A § Performance budgets — cf-terrain's locked per-tick latency budget.
//!
//! Owned by `docs/plan/spec/perf-budget-contract.md`. Downstream
//! milestones must not re-tune.

/// (milliseconds).
///
/// Covers per-chunk parallel mutation (`par_iter_mut` over the dirty
/// chunk set) plus the inter-chunk boundary post-pass (single-threaded,
/// `(cx, cy)` ascending). Budget covers M3-era chunked terrain at the M9
/// firehose density.
pub const TERRAIN_MUTATION_P99_BUDGET_MS: f32 = 2.5;

/// `active_region = false`. Wake-on-edit flips it back to true (and
/// wakes neighbors within 1-chunk radius). Forward-compat for the M15 CA
/// chemistry kernel.
pub const CHUNK_SLEEP_IDLE_THRESHOLD_TICKS: u64 = 300;
