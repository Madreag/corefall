//! Read a bench perf JSON report and assert every required key is
//! within budget per `docs/plan/spec/perf-budget-contract.md`.
//!
//! Budgets (p99 microseconds):
//! - actor: 1500
//! - ai: 4000 (retuned from 2000 at M8A for the 5-layer thinking stack)
//! - projectile: 1000
//! - terrain: 2500
//! - mission: 200
//! - recorder: 500
//! - render: 3500

use anyhow::{anyhow, Context, Result};
use std::path::Path;

use crate::{PerfReport, SubsystemPerf};

pub const ACTOR_P99_BUDGET_US: u64 = 1_500;
pub const AI_P99_BUDGET_US: u64 = 4_000;
pub const PROJECTILE_P99_BUDGET_US: u64 = 1_000;
pub const TERRAIN_P99_BUDGET_US: u64 = 2_500;
pub const MISSION_P99_BUDGET_US: u64 = 200;
pub const RECORDER_P99_BUDGET_US: u64 = 500;
pub const RENDER_P99_BUDGET_US: u64 = 3_500;

/// Assert the report's per-subsystem p99 values fit the M8A perf budget.
/// Zero values are treated as "this subsystem not exercised by this bench"
/// and skipped. Non-zero values exceeding the budget cause failure.
pub fn assert_within_budget(input: &Path) -> Result<()> {
    let body = std::fs::read_to_string(input).with_context(|| format!("read {}", input.display()))?;
    let report: PerfReport = serde_json::from_str(&body).with_context(|| format!("parse {}", input.display()))?;

    let mut errs = Vec::new();
    check(&mut errs, "actor", &report.actor, ACTOR_P99_BUDGET_US);
    check(&mut errs, "ai", &report.ai, AI_P99_BUDGET_US);
    check(&mut errs, "projectile", &report.projectile, PROJECTILE_P99_BUDGET_US);
    check(&mut errs, "terrain", &report.terrain, TERRAIN_P99_BUDGET_US);
    check(&mut errs, "mission", &report.mission, MISSION_P99_BUDGET_US);
    check(&mut errs, "recorder", &report.recorder, RECORDER_P99_BUDGET_US);
    check(&mut errs, "render", &report.render, RENDER_P99_BUDGET_US);

    if !errs.is_empty() {
        return Err(anyhow!("budget violations: {}", errs.join("; ")));
    }
    tracing::info!(target: "cf::bench::perf_assert",
        bench=report.bench.as_str(), "all subsystem budgets within p99 envelope");
    Ok(())
}

fn check(errs: &mut Vec<String>, subsystem: &str, perf: &SubsystemPerf, budget: u64) {
    if perf.p99_us > 0 && perf.p99_us > budget {
        errs.push(format!("{}: p99 {} us > budget {} us", subsystem, perf.p99_us, budget));
    }
}

/// Produce a baseline perf snapshot. M8A captures synthetic baselines so
/// `m8a_perf_gate.sh` always has a reference point to compare against;
/// M9+ stress-fills the harness with real engine drives.
pub fn baseline_snapshot(ticks: u32) -> PerfReport {
    PerfReport {
        bench: "baseline_perf_snapshot".into(),
        ticks,
        tick_rate_hz: 60,
        seed: 0,
        actor: SubsystemPerf {
            p50_us: 800,
            p99_us: 1_200,
            p999_us: 1_400,
        },
        ai: SubsystemPerf {
            p50_us: 1_800,
            p99_us: 2_500,
            p999_us: 3_800,
        },
        projectile: SubsystemPerf {
            p50_us: 500,
            p99_us: 800,
            p999_us: 950,
        },
        terrain: SubsystemPerf {
            p50_us: 1_500,
            p99_us: 2_000,
            p999_us: 2_400,
        },
        mission: SubsystemPerf {
            p50_us: 80,
            p99_us: 150,
            p999_us: 190,
        },
        recorder: SubsystemPerf {
            p50_us: 200,
            p99_us: 400,
            p999_us: 480,
        },
        render: SubsystemPerf {
            p50_us: 2_500,
            p99_us: 3_400,
            p999_us: 3_450,
        },
        final_blake3: None,
    }
}
