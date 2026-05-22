//! M8B § Resimulation driver.
//!
//! Per M8B spec § Notes: the 6-frame rollback resimulation MUST reuse
//! the deterministic sim core verbatim — no parallel rollback codepath;
//! the rollback driver wraps the same `World::tick(...)` entry used by
//! the live sim.
//!
//! This module ships the orchestration + budget arithmetic + commit
//! ordering. The actual `World::tick` callable is wired by cf-control
//! at M9+.

use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::rollback::ring_buffer::{InputFrame, RollbackRingBuffer};

/// total resimulation cost ≤ 8 ms p99 on the reference platform.
/// `rollback_to_tick_budget_us` covers the "rolls back to tick T within
/// 1 ms" sub-bound; the remaining headroom is the per-frame resim cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResimulateBudget {
    /// Total p99 budget (microseconds) per spec § Acceptance.
    pub total_us: u32,
    /// Sub-budget for the initial state-rollback (per spec "within 1 ms").
    pub rollback_to_tick_budget_us: u32,
    /// Sub-budget for the per-frame resimulation steps (≤ 7 ms p99 for
    /// 6 frames on the reference platform per the spec).
    pub per_frame_resim_budget_us: u32,
}

impl Default for ResimulateBudget {
    fn default() -> Self {
        Self {
            total_us: 8000,
            rollback_to_tick_budget_us: 1000,
            per_frame_resim_budget_us: 7000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResimulateRequest {
    pub from_tick: u64,
    pub to_tick: u64,
    pub authoritative_inputs: Vec<InputFrame>,
    pub cause: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResimulateOutcome {
    pub from_tick: u64,
    pub to_tick: u64,
    pub elapsed_us: u32,
    pub rollback_to_tick_elapsed_us: u32,
    pub per_frame_resim_elapsed_us: u32,
    pub within_budget: bool,
    pub cause: String,
}

/// `step_one_tick` callable (the deterministic sim core's `World::tick`
/// entry point) and applies each authoritative input in commit order.
///
/// The callable is parameterized so cf-net's unit tests can drive a
/// synthetic sim that respects the budget arithmetic without requiring
/// the full cf-control engine.
pub fn run_resimulate<F: FnMut(&InputFrame)>(
    request: &ResimulateRequest,
    rb: &mut RollbackRingBuffer,
    budget: ResimulateBudget,
    mut step_one_tick: F,
) -> ResimulateOutcome {
    let t_total = Instant::now();
    let t_rollback = Instant::now();
    // 1. Roll the buffer back: drop any entry at-or-after `from_tick`.
    let kept: Vec<InputFrame> = rb
        .iter()
        .filter(|f| f.tick < request.from_tick)
        .cloned()
        .collect();
    *rb = RollbackRingBuffer::new();
    for k in kept {
        rb.push(k);
    }
    let rollback_to_tick_elapsed_us = t_rollback.elapsed().as_micros() as u32;
    let t_resim = Instant::now();
    // 2. Apply each authoritative input in commit order (tick ascending).
    let mut commit_ordered = request.authoritative_inputs.clone();
    commit_ordered.sort_by_key(|f| f.tick);
    for frame in &commit_ordered {
        step_one_tick(frame);
        rb.push(frame.clone());
    }
    let per_frame_resim_elapsed_us = t_resim.elapsed().as_micros() as u32;
    let elapsed_us = t_total.elapsed().as_micros() as u32;
    let within_budget = elapsed_us <= budget.total_us;
    ResimulateOutcome {
        from_tick: request.from_tick,
        to_tick: request.to_tick,
        elapsed_us,
        rollback_to_tick_elapsed_us,
        per_frame_resim_elapsed_us,
        within_budget,
        cause: request.cause.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(tick: u64) -> InputFrame {
        let mut h = [0u8; 32];
        h[0] = tick as u8;
        InputFrame::new(tick, vec![tick as u8], h)
    }

    #[test]
    fn resimulate_replays_window_in_commit_order() {
        let mut rb = RollbackRingBuffer::new();
        for tick in 614..=620u64 {
            rb.push(frame(tick));
        }
        let request = ResimulateRequest {
            from_tick: 614,
            to_tick: 620,
            authoritative_inputs: (614..=620u64).map(frame).collect(),
            cause: "input_mismatch".into(),
        };
        let mut seen: Vec<u64> = Vec::new();
        let outcome = run_resimulate(
            &request,
            &mut rb,
            ResimulateBudget::default(),
            |frame| seen.push(frame.tick),
        );
        assert_eq!(seen, vec![614, 615, 616, 617, 618, 619, 620]);
        assert_eq!(outcome.from_tick, 614);
        assert_eq!(outcome.to_tick, 620);
    }

    #[test]
    fn resimulate_within_default_budget_for_small_synthetic_workload() {
        let mut rb = RollbackRingBuffer::new();
        for tick in 600..=606u64 {
            rb.push(frame(tick));
        }
        let request = ResimulateRequest {
            from_tick: 600,
            to_tick: 606,
            authoritative_inputs: (600..=606u64).map(frame).collect(),
            cause: "test".into(),
        };
        let outcome = run_resimulate(&request, &mut rb, ResimulateBudget::default(), |_| {});
        assert!(outcome.within_budget, "synthetic workload p99 must fit budget");
    }
}
