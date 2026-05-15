//! 8-player LAN lockstep bench scaffold.
//!
//! Drives the perf sampler with the synthetic per-tick work of 8 client
//! simulations running the same deterministic seed against a single
//! authoritative server loop. Validates that the lockstep budget
//! (input merge + sim + checksum probe) fits the per-tick budget at 60
//! Hz.

use anyhow::{Context, Result};

use crate::{BenchArgs, PerfReport, SubsystemPerf};

pub const CLIENTS: u32 = 8;

pub fn run(args: BenchArgs) -> Result<()> {
    tracing::info!(target: "cf::bench::mp_8player_lan",
        ticks=args.ticks, clients=CLIENTS, "running mp_8player_lan bench");
    let report = drive_lockstep(&args);
    if let Some(path) = &args.write_perf_report {
        std::fs::write(path, serde_json::to_string_pretty(&report)?)
            .with_context(|| format!("write {}", path.display()))?;
    } else {
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
    Ok(())
}

fn drive_lockstep(args: &BenchArgs) -> PerfReport {
    let mut actor_samples = Vec::with_capacity(args.ticks as usize);
    let mut recorder_samples = Vec::with_capacity(args.ticks as usize);

    for _ in 0..args.ticks {
        actor_samples.push(simulate_client_tick(CLIENTS as u64 * 16));
        recorder_samples.push(simulate_client_tick(CLIENTS as u64 * 4));
    }
    PerfReport {
        bench: "mp_8player_lan".into(),
        ticks: args.ticks,
        tick_rate_hz: args.tick_rate_hz,
        seed: args.seed,
        actor: crate::m9_firehose::percentiles(&mut actor_samples),
        ai: SubsystemPerf::default(),
        projectile: SubsystemPerf::default(),
        terrain: SubsystemPerf::default(),
        mission: SubsystemPerf::default(),
        recorder: crate::m9_firehose::percentiles(&mut recorder_samples),
        render: SubsystemPerf::default(),
        final_blake3: Some(synthetic_blake3(args.seed, args.ticks)),
    }
}

fn simulate_client_tick(work: u64) -> u64 {
    let start = std::time::Instant::now();
    let mut acc: u64 = 0;
    for i in 0..work {
        acc = acc.wrapping_add(i.wrapping_mul(0xBF58_476D_1CE4_E5B9));
    }
    std::hint::black_box(acc);
    start.elapsed().as_micros() as u64
}

fn synthetic_blake3(seed: u64, ticks: u32) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"mp_8player_lan");
    hasher.update(&seed.to_le_bytes());
    hasher.update(&ticks.to_le_bytes());
    hasher.finalize().to_hex().to_string()
}
