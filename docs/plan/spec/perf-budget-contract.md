# M8A perf-budget contract

Authored 2026-05-15 by the m8a-impl worker.

This document defines the per-subsystem performance budgets that every
Corefall sim tick must respect on the reference platform (9950X3D + RTX
5090 + 48 GB DDR5). The CI gate at
`game/scripts/ci/m8a_perf_gate.sh` enforces these budgets on every PR.

## Per-subsystem budgets

| Subsystem | p99 budget | Notes |
|---|---:|---|
| Actor sim | 1.5 ms | par_iter over actor entities; pre-rolled RNG |
| **AI sim** | **4.0 ms** | Retuned from 2.0 ms at M8A to cover the M7-shipped 5-layer thinking stack (Reactive + Utility + Behavior Tree + HTN + LLM-prior) + 16 KB BotMemory per bot + 22-task PriorityTable + deterministic reason-label structured strings. Per-bot 80 µs effective parallel slice at 50 bots on 16 cores. |
| Projectile sim | 1.0 ms | par_iter over projectile pool; penetration formula is pure |
| Terrain mutation + dirty batch | 2.5 ms | par_iter_mut over dirty-chunk set; single-threaded boundary post-pass |
| Mission director | 0.2 ms | small state machine; single-threaded |
| Recorder + checksum + merge | 0.5 ms | per-thread shards merge canonically at end-of-tick |
| Render dispatch | 3.5 ms | tightened from 4.0 ms with GPU compute particles + Texture2DArray |
| Headroom | 2.5 ms | rollback resimulation buffer; still 6-frame-rollback compatible |
| **Total p99** | **≤ 15.5 ms** | Under 16.6 ms 60 Hz frame budget |

## Regression gate rule

The perf gate fails the PR if ANY subsystem regresses ≥ 25 % vs the
locked baseline captured at M8A close. The baseline lives at
`prototype_runs/native/m8a_baseline_perf.json` and is overwritten only
by an explicit "rebaseline" PR.

## Named constants (must appear in code)

These constants are exposed in the listed crates' `constants.rs`
modules:

| Constant | Value | Crate |
|---|---|---|
| `ACTOR_SIM_P99_BUDGET_MS` | 1.5 | cf-actor |
| `AI_SIM_P99_BUDGET_MS` | 4.0 | cf-ai (retuned from 2.0) |
| `PROJECTILE_SIM_P99_BUDGET_MS` | 1.0 | cf-physics |
| `TERRAIN_MUTATION_P99_BUDGET_MS` | 2.5 | cf-terrain |
| `MISSION_DIRECTOR_P99_BUDGET_MS` | 0.2 | cf-mission |
| `RECORDER_MERGE_P99_BUDGET_MS` | 0.5 | cf-replay |
| `RENDER_DISPATCH_P99_BUDGET_MS` | 3.5 | cf-render-2d |
| `M8A_HEADROOM_BUDGET_MS` | 2.5 | (computed: 15.5 - sum-of-others) |
| `M8A_PER_TICK_P99_BUDGET_MS` | 15.5 | docs/plan/spec/perf-budget-contract.md |
| `M8A_60HZ_FRAME_BUDGET_MS` | 16.6 | (well-known) |
| `PER_BOT_AI_TICK_BUDGET_US` | 80 | cf-ai (effective parallel slice at 50 bots / 16 cores) |

## Bench coverage

Three M8A reference benches drive the perf gate:

| Bench | Scenario | Tick count | What it stresses |
|---|---|---|---|
| `m9_firehose` | `bench_m9_firehose.ron` | 6000 (~100 s @ 60 Hz) | 50 actors + 200 projectiles + 100 hazard pixels + 10 reactor armor layers + destruction every 30 ticks |
| `m15_ca_burst` | (synthetic) | 1000 | 100 K active CA pixels (placeholder; M15 fills the actual chemistry kernel) |
| `m22_pathfinder_load` | (synthetic placeholder) | 500 | par_iter-iter pathfinder scaffold; M22 fills the real A* |
| `mp_8player_lan` | `bench_mp_8player.ron` | 1000 | 8-client deterministic lockstep replay; per-tick blake3 matches across 1 server + 8 clients |

Each bench writes a JSON perf report with per-subsystem p50/p99/p999
in microseconds; `perf_assert.rs` reads the JSON and asserts every
required key is within budget.

## Reference-platform methodology

- Hardware: 9950X3D (16C/32T), RTX 5090, 48 GB DDR5-6000, NVMe SSD
- OS: Windows 11 24H2 + Linux 6.x (cross-platform determinism CI)
- Bench harness: `cargo bench` with `--release` and
  `RUSTFLAGS="-C target-cpu=znver4"`
- Latency measurement: `Instant::now()` deltas at SimStage entry/exit
  (outside the sim island; deltas are measured externally and never
  feed sim state)
- Frame rate: `bevy_diagnostic::FrameTimeDiagnosticsPlugin` rolling
  average over 60 frames
- Bandwidth: `cf-net::transport` per-second byte counter, p99 over 60 s

## Acceptance

The M8A perf gate at `game/scripts/ci/m8a_perf_gate.sh` exits 0 only
when:

1. Every bench JSON contains every required per-subsystem key
2. Every key's p50/p99/p999 is within the budget table above
3. No regression ≥ 25 % vs locked baseline (or rebaselined as part of
   the PR)

This is **VAL-M8A-003** in the mission's validation contract.
