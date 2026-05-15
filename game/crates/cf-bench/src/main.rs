//! M8A § Performance budgets + regression gate.
//!
//! `cf-bench` ships the M8A stress-test benches that drive the
//! `m8a_perf_gate.sh` CI gate. Subcommands:
//!
//! - `m9_firehose` — 50 actors + 200 projectiles + 100 hazard pixels +
//!   10 reactor armor layers + destruction every 30 ticks
//! - `m15_ca_burst` — 100 K active CA pixels (placeholder; M15 fills)
//! - `m22_pathfinder_load` — pathfinder load placeholder (M22 fills)
//! - `mp_8player_lan` — 8-client deterministic lockstep
//! - `baseline_perf_snapshot` — capture per-subsystem p50/p99/p999 at HEAD
//! - `perf_assert` — read a perf JSON report and assert every key is
//!   within budget; exits non-zero on regression
//!
//! Each bench writes a JSON perf report consumed by `perf_assert` and
//! `m8a_perf_gate.sh`. The benches are intentionally lightweight at M8A:
//! they exercise the per-subsystem sampling pipeline without driving the
//! engine in a heavy-load scenario (which would require either headless
//! replay infrastructure or a full Bevy plugin chain). M9 stress-fills
//! the bench bodies with real engine drives; M8A ships the contract.

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

mod m9_firehose;
mod m15_ca_burst;
mod m22_pathfinder_load;
mod mp_8player_lan;
mod perf_assert;

#[derive(Debug, Parser)]
#[command(name = "cf-bench", about = "M8A perf bench harness.")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// M9 firehose: 50 actors + 200 projectiles + 100 hazard pixels.
    M9Firehose(BenchArgs),
    /// M15 CA burst: 100K synthetic active pixels (placeholder for M15).
    M15CaBurst(BenchArgs),
    /// M22 pathfinder load: placeholder scaffold for M22's A* harness.
    M22PathfinderLoad(BenchArgs),
    /// 8-client deterministic lockstep replay.
    #[command(name = "mp-8player-lan")]
    Mp8PlayerLan(BenchArgs),
    /// Capture per-subsystem p50/p99/p999 at HEAD into a JSON report.
    BaselinePerfSnapshot {
        #[arg(long, default_value_t = 1000)]
        ticks: u32,
        #[arg(long, default_value = "/tmp/cf_bench_baseline.json")]
        output: PathBuf,
    },
    /// Read a bench perf JSON report and assert every required key is
    /// within budget. Exits non-zero on regression.
    PerfAssert {
        #[arg(long)]
        input: PathBuf,
    },
}

#[derive(Debug, Parser)]
pub struct BenchArgs {
    #[arg(long, default_value_t = 1000)]
    pub ticks: u32,
    #[arg(long, default_value_t = 60)]
    pub tick_rate_hz: u32,
    #[arg(long, default_value_t = 42)]
    pub seed: u64,
    /// Run cross-OS determinism comparison. At M8A this flag is honored
    /// by capturing the final blake3 of the bundle for later cross-OS
    /// diffing in the cross-OS gate (single-OS stub mode).
    #[arg(long, default_value_t = false)]
    pub determinism_cross_os: bool,
    /// Path to write the per-subsystem perf JSON report.
    #[arg(long)]
    pub write_perf_report: Option<PathBuf>,
}

/// Per-subsystem perf sample: p50/p99/p999 in microseconds.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubsystemPerf {
    pub p50_us: u64,
    pub p99_us: u64,
    pub p999_us: u64,
}

/// Per-bench perf report consumed by `perf_assert` and the M8A perf gate.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerfReport {
    pub bench: String,
    pub ticks: u32,
    pub tick_rate_hz: u32,
    pub seed: u64,
    pub actor: SubsystemPerf,
    pub ai: SubsystemPerf,
    pub projectile: SubsystemPerf,
    pub terrain: SubsystemPerf,
    pub mission: SubsystemPerf,
    pub recorder: SubsystemPerf,
    pub render: SubsystemPerf,
    pub final_blake3: Option<String>,
}

fn init_diagnostics() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,cf_=debug")))
        .with_target(true)
        .init();
}

fn main() -> Result<()> {
    init_diagnostics();
    let cli = Cli::parse();
    match cli.command {
        Cmd::M9Firehose(args) => m9_firehose::run(args),
        Cmd::M15CaBurst(args) => m15_ca_burst::run(args),
        Cmd::M22PathfinderLoad(args) => m22_pathfinder_load::run(args),
        Cmd::Mp8PlayerLan(args) => mp_8player_lan::run(args),
        Cmd::BaselinePerfSnapshot { ticks, output } => {
            let report = perf_assert::baseline_snapshot(ticks);
            std::fs::write(&output, serde_json::to_string_pretty(&report)?)
                .with_context(|| format!("write {}", output.display()))?;
            tracing::info!(target: "cf::bench", "baseline perf snapshot -> {}", output.display());
            Ok(())
        }
        Cmd::PerfAssert { input } => perf_assert::assert_within_budget(&input)
            .map_err(|err| anyhow!("perf budget violation: {err}")),
    }
}
