//! M15 cellular-automaton burst bench scaffold.
//!
//! M8A reserves the parallel-iter scaffold for M15's CA chemistry kernel
//! (sand/water/fire/gas). The bench loops over a synthetic 100K
//! active-pixel set per tick to validate that the chunk-CA scheduling
//! pattern (par_iter over active chunks; per-chunk RNG seeded by tick *
//! chunk_id) fits within the 4.0 ms p99 budget that M15 will inherit.
//!
//! M8A only ships the bench harness + report shape. M15 fills the kernel
//! when the chemistry rules land.

use anyhow::{Context, Result};

use crate::{BenchArgs, PerfReport, SubsystemPerf};

pub const ACTIVE_PIXELS: u32 = 100_000;

pub fn run(args: BenchArgs) -> Result<()> {
    tracing::info!(target: "cf::bench::m15_ca_burst",
        ticks=args.ticks, active_pixels=ACTIVE_PIXELS, "running m15_ca_burst bench");
    let report = drive_ca_burst(&args);
    if let Some(path) = &args.write_perf_report {
        std::fs::write(path, serde_json::to_string_pretty(&report)?)
            .with_context(|| format!("write {}", path.display()))?;
    } else {
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
    Ok(())
}

fn drive_ca_burst(args: &BenchArgs) -> PerfReport {
    let mut terrain_samples = Vec::with_capacity(args.ticks as usize);

    for _ in 0..args.ticks {
        terrain_samples.push(simulate_ca_step(ACTIVE_PIXELS as u64));
    }

    PerfReport {
        bench: "m15_ca_burst".into(),
        ticks: args.ticks,
        tick_rate_hz: args.tick_rate_hz,
        seed: args.seed,
        actor: SubsystemPerf::default(),
        ai: SubsystemPerf::default(),
        projectile: SubsystemPerf::default(),
        terrain: crate::m9_firehose::percentiles(&mut terrain_samples),
        mission: SubsystemPerf::default(),
        recorder: SubsystemPerf::default(),
        render: SubsystemPerf::default(),
        final_blake3: None,
    }
}

fn simulate_ca_step(pixels: u64) -> u64 {
    let start = std::time::Instant::now();
    let mut acc: u64 = 0;
    for i in 0..pixels {
        acc = acc.wrapping_add(i ^ 0xC6BC_2796_5034_7DB1);
    }
    std::hint::black_box(acc);
    start.elapsed().as_micros() as u64
}
