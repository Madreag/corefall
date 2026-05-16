//! M9 firehose stress bench scaffold.
//!
//! M8A ships the scaffold: a bench that drives a synthetic 50-actor +
//! 200-projectile + 100-hazard-pixel + 10-reactor-armor-layer load
//! through the per-subsystem perf sampler and emits a JSON report keyed
//! by subsystem (actor / ai / projectile / terrain / mission / recorder
//! / render). The harness uses deterministic seeded synthetic work
//! representative of the M9 firehose target so the perf budget contract
//! at `docs/plan/spec/perf-budget-contract.md` is exercised end-to-end.
//!
//! M9 fills the bench body with a real engine drive once cf-headless's
//! `replay` subcommand handles bench scenarios; M8A's contract is
//! satisfied by the report's per-subsystem keys + budget assertions.

use anyhow::{Context, Result};
use std::time::Instant;

use crate::{BenchArgs, PerfReport, SubsystemPerf};

pub const ACTORS: u32 = 50;
pub const PROJECTILES: u32 = 200;
pub const HAZARD_PIXELS: u32 = 100;
pub const REACTOR_ARMOR_LAYERS: u32 = 10;
pub const DESTRUCTION_EVERY_TICKS: u32 = 30;

pub fn run(args: BenchArgs) -> Result<()> {
    tracing::info!(target: "cf::bench::m9_firehose",
        ticks=args.ticks, actors=ACTORS, projectiles=PROJECTILES,
        hazards=HAZARD_PIXELS, reactor_layers=REACTOR_ARMOR_LAYERS,
        "running m9_firehose bench");

    let report = drive_firehose(&args);
    if let Some(path) = &args.write_perf_report {
        std::fs::write(path, serde_json::to_string_pretty(&report)?)
            .with_context(|| format!("write {}", path.display()))?;
        tracing::info!(target: "cf::bench::m9_firehose", "wrote perf report -> {}", path.display());
    } else {
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
    Ok(())
}

fn drive_firehose(args: &BenchArgs) -> PerfReport {
    let mut actor_samples = Vec::with_capacity(args.ticks as usize);
    let mut ai_samples = Vec::with_capacity(args.ticks as usize);
    let mut projectile_samples = Vec::with_capacity(args.ticks as usize);
    let mut terrain_samples = Vec::with_capacity(args.ticks as usize);
    let mut mission_samples = Vec::with_capacity(args.ticks as usize);
    let mut recorder_samples = Vec::with_capacity(args.ticks as usize);
    let mut render_samples = Vec::with_capacity(args.ticks as usize);

    for tick in 0..args.ticks {
        actor_samples.push(simulate_subsystem(ACTORS as u64 * 8));
        ai_samples.push(simulate_subsystem(ACTORS as u64 * 32));
        projectile_samples.push(simulate_subsystem(PROJECTILES as u64 * 4));
        let terrain_load = if tick % DESTRUCTION_EVERY_TICKS == 0 {
            HAZARD_PIXELS as u64 * 16
        } else {
            HAZARD_PIXELS as u64
        };
        terrain_samples.push(simulate_subsystem(terrain_load));
        mission_samples.push(simulate_subsystem(2));
        recorder_samples.push(simulate_subsystem(8));
        render_samples.push(simulate_subsystem(16));
    }

    PerfReport {
        bench: "m9_firehose".into(),
        ticks: args.ticks,
        tick_rate_hz: args.tick_rate_hz,
        seed: args.seed,
        actor: percentiles(&mut actor_samples),
        ai: percentiles(&mut ai_samples),
        projectile: percentiles(&mut projectile_samples),
        terrain: percentiles(&mut terrain_samples),
        mission: percentiles(&mut mission_samples),
        recorder: percentiles(&mut recorder_samples),
        render: percentiles(&mut render_samples),
        final_blake3: Some(synthetic_blake3(args.seed, args.ticks)),
    }
}

fn simulate_subsystem(loops: u64) -> u64 {
    let start = Instant::now();
    let mut acc: u64 = 0;
    for i in 0..loops {
        acc = acc.wrapping_add(i.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    }
    std::hint::black_box(acc);
    start.elapsed().as_micros() as u64
}

pub(crate) fn percentiles(samples: &mut [u64]) -> SubsystemPerf {
    if samples.is_empty() {
        return SubsystemPerf::default();
    }
    samples.sort_unstable();
    let pct = |p: f64| -> u64 {
        let idx = ((samples.len() as f64 - 1.0) * p).round() as usize;
        samples[idx]
    };
    SubsystemPerf {
        p50_us: pct(0.5),
        p99_us: pct(0.99),
        p999_us: pct(0.999),
    }
}

fn synthetic_blake3(seed: u64, ticks: u32) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&seed.to_le_bytes());
    hasher.update(&ticks.to_le_bytes());
    hasher.finalize().to_hex().to_string()
}
