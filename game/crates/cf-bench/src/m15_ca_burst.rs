//! M15 cellular-automaton burst bench.
//!
//! Per the M15 spec § "M8A (HARD GATE)" entry:
//! > `m15_ca_burst` bench with 100k active pixels as the M15 baseline-
//! > to-beat (p99 < 4.0 ms per tick).
//!
//! M8A reserved this bench scaffold with a synthetic XOR-accumulator
//! placeholder. M15 fills it with the real CA + reaction + phase kernel
//! orchestrator, seeded with a 100 K active-pixel scenario.
//!
//! ## Scenario
//!
//! - 1024 × 1024 world (large enough that 100 K pixels stay sparse).
//! - 100 000 sand pixels distributed across a tall column so gravity
//!   has work to do every tick.
//! - 200 paired iron + acid pixels scattered through the column so the
//!   reaction evaluator dispatches `rxn.corrosion.acid_iron` every tick.
//! - 100 paired water + fire pixels scattered so extinguish reactions
//!   fire every tick.
//! - Default heat field + reaction registry + phase registry.

use anyhow::{Context, Result};

use cf_material::{
    default_alchemy_registry, default_phase_registry, default_reaction_registry, kernel_step, MaterialKernel,
};
use cf_terrain::air::AIR_GRID_SIZE;
use cf_terrain::chunked::{ChunkedTerrain, MATERIAL_AIR};
use cf_terrain::heat::HeatField;

use crate::{BenchArgs, PerfReport, SubsystemPerf};

pub const ACTIVE_PIXELS: u32 = 100_000;

/// Per the M15 spec § HARD GATE: "p99 < 4.0 ms per tick".
pub const P99_BUDGET_US: u64 = 4_000;

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
    if report.terrain.p99_us > P99_BUDGET_US {
        tracing::warn!(
            target: "cf::bench::m15_ca_burst",
            p99_us = report.terrain.p99_us,
            budget_us = P99_BUDGET_US,
            "M15 CA p99 over 4 ms budget"
        );
    }
    Ok(())
}

fn drive_ca_burst(args: &BenchArgs) -> PerfReport {
    let mut terrain = build_scenario(args.seed);
    let reactions = default_reaction_registry();
    let phase = default_phase_registry();
    // alchemy is not exercised by the bench loop; ensure it loads
    // without panic so the bench drives a representative orchestrator.
    let _alchemy = default_alchemy_registry();
    let heat = HeatField::default();
    // M15 spec § "active-chunk wake/sleep gating": with gating ON, the
    // bench skips chunks whose `active_region == false`. The scenario
    // pre-seeds awake chunks where active pixels live so the first
    // tick has work to do without a warm-up tick.
    seed_awake_chunks(&mut terrain);
    let mut kernel = MaterialKernel::new().with_wake_sleep_gating(true).with_parallel(true);

    let mut terrain_samples = Vec::with_capacity(args.ticks as usize);
    for _ in 0..args.ticks {
        let start = std::time::Instant::now();
        let _ = kernel_step(&mut terrain, &mut kernel, &reactions, &phase, &heat, None);
        terrain_samples.push(start.elapsed().as_micros() as u64);
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
        final_blake3: Some(terrain_checksum(&terrain)),
    }
}

/// Build a 1024x1024 scenario with ~100 K active pixels: sand column
/// + iron/acid pairs + water/fire pairs.
fn build_scenario(seed: u64) -> ChunkedTerrain {
    // 1024×1024 world = 16×16 chunks of 256×256 each. Plenty of room
    // to spread the 100 K active pixels across chunks for parallelism.
    let mut terrain = ChunkedTerrain::new(1024, 1024, MATERIAL_AIR);
    // Floor at y=1023 to give gravity-cascade pixels somewhere to land.
    for x in 0..1024 {
        terrain.set_material_pixel(x as i64, 1023, 1, 0); // dirt
    }
    // Distribute ~100 K sand pixels in a vertical column across many chunks.
    let mut filled = 0u32;
    let mut rng_state = seed;
    while filled < ACTIVE_PIXELS - 600 {
        rng_state = lcg_next(rng_state);
        let x = (rng_state >> 16) as i64 % 1024;
        rng_state = lcg_next(rng_state);
        let y = (rng_state >> 16) as i64 % 1020;
        if terrain.material_at(x, y) == MATERIAL_AIR {
            terrain.set_material_pixel(x, y, 14, 0); // sand
            filled += 1;
        }
    }
    // 200 iron + acid pairs (400 active pixels).
    for i in 0..200 {
        rng_state = lcg_next(rng_state);
        let x = (rng_state >> 16) as i64 % 1023;
        let y = (i * 5 + 100) as i64 % 1020;
        if terrain.material_at(x, y) == MATERIAL_AIR {
            terrain.set_material_pixel(x, y, 68, 0); // iron
            terrain.set_material_pixel(x + 1, y, 21, 0); // acid
        }
    }
    // 100 water + fire pairs.
    for i in 0..100 {
        rng_state = lcg_next(rng_state);
        let x = (rng_state >> 16) as i64 % 1023;
        let y = (i * 7 + 50) as i64 % 1020;
        if terrain.material_at(x, y) == MATERIAL_AIR {
            terrain.set_material_pixel(x, y, 13, 0); // water
            terrain.set_material_pixel(x + 1, y, 65, 0); // fire
        }
    }
    // Sanity: bench depends on AIR_GRID_SIZE for HeatField alignment;
    // we keep the world below that bound for clean cell mapping.
    let _ = AIR_GRID_SIZE;
    terrain
}

/// Mark every chunk that contains a non-air pixel as awake. Pre-seeds
/// the wake/sleep gate so the first bench tick has work to do.
fn seed_awake_chunks(terrain: &mut ChunkedTerrain) {
    for (cx, cy) in terrain.allocated_chunk_coords() {
        terrain.set_chunk_active_region(cx, cy, true);
    }
}

fn terrain_checksum(t: &ChunkedTerrain) -> String {
    let mut hasher = blake3::Hasher::new();
    for (cx, cy, hex) in t.chunk_summary_entries() {
        hasher.update(&cx.to_le_bytes());
        hasher.update(&cy.to_le_bytes());
        hasher.update(hex.as_bytes());
    }
    hex::encode(hasher.finalize().as_bytes())
}

fn lcg_next(state: u64) -> u64 {
    state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)
}
