# cf-bench — AGENTS.md

## Owns
- Performance benchmark harness (real implementation pending DR-054).
- Benchmark profiles: `actor_movement`, `chunked_terrain_carve`, `chassis_damage_pipeline`, `observe_frame_emit`.
- Baseline JSON for regression detection.
- CI step for >5% regression rejection.

## Public API Boundary
- (Stub. 38-line scaffold.)

## Does NOT Own
- Production perf counters → each crate's own `tracing` spans.
- Render perf → `cf-render-2d` GPU timing.

## Test Surface
- (Stub.) Real benchmarks land when profiles are authored.

## Source Trail
- DR-054 (performance optimization / profiling; OPEN).
