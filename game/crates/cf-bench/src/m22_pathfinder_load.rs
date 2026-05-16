//! M22 pathfinder-load bench scaffold.
//!
//! M8A reserves the parallel-iter scaffold for M22's pathfinder. The
//! bench loops over a synthetic batch of path queries per tick to
//! validate that the par_iter pathfinder request pattern fits within the
//! AI budget envelope when M22 fills the actual A* / JPS+ kernel.

use anyhow::{Context, Result};

use crate::{BenchArgs, PerfReport, SubsystemPerf};

pub const PATH_QUERIES_PER_TICK: u32 = 16;

pub fn run(args: BenchArgs) -> Result<()> {
    tracing::info!(target: "cf::bench::m22_pathfinder_load",
        ticks=args.ticks, "running m22_pathfinder_load bench");
    let report = drive_pathfinder_load(&args);
    if let Some(path) = &args.write_perf_report {
        std::fs::write(path, serde_json::to_string_pretty(&report)?)
            .with_context(|| format!("write {}", path.display()))?;
    } else {
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
    Ok(())
}

fn drive_pathfinder_load(args: &BenchArgs) -> PerfReport {
    let mut ai_samples = Vec::with_capacity(args.ticks as usize);
    for _ in 0..args.ticks {
        ai_samples.push(simulate_path_query_batch(PATH_QUERIES_PER_TICK as u64 * 64));
    }
    PerfReport {
        bench: "m22_pathfinder_load".into(),
        ticks: args.ticks,
        tick_rate_hz: args.tick_rate_hz,
        seed: args.seed,
        actor: SubsystemPerf::default(),
        ai: crate::m9_firehose::percentiles(&mut ai_samples),
        projectile: SubsystemPerf::default(),
        terrain: SubsystemPerf::default(),
        mission: SubsystemPerf::default(),
        recorder: SubsystemPerf::default(),
        render: SubsystemPerf::default(),
        final_blake3: None,
    }
}

fn simulate_path_query_batch(work: u64) -> u64 {
    let start = std::time::Instant::now();
    let mut acc: u64 = 0;
    for i in 0..work {
        acc = acc.wrapping_add(i.rotate_left(13));
    }
    std::hint::black_box(acc);
    start.elapsed().as_micros() as u64
}
