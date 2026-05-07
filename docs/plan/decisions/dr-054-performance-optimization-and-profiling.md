---
type: decision
id: DR-054
status: closed-direction
priority: P0
closed_at: 2026-05-06
revisit_trigger: "Per-tier perf budget exceeded; SIMD path produces non-deterministic output; GPU compute hot path proves harder than CPU; profile-guided optimization regresses; AI agents cannot drive perf regression hunts via cfctl."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/performance-optimization-and-profiling|perf optimization spec]] · [[decisions/dr-024-native-engine-stack|DR-024]] · [[decisions/dr-028-visual-fidelity-targets|DR-028]] · [[decisions/dr-033-full-collision-physics-direction|DR-033]] · [[decisions/dr-036-systemic-material-simulation-direction|DR-036]]

# DR-054: Performance Optimization & Profiling Track

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-06)
> Comprehensive perf optimization track for physics-heavy + emulation-heavy game. **Hot paths**: material kernel (32+ active 64×64 chunks @ 60Hz), atmospherics (100+ atmospheres @ 60Hz), physics narrowphase (1000+ entities), replay event recording (100K+ events/run), pathfinding (10+ AI bots × per-tick), renderer (4K/120 ceiling), network serialization (snapshot delta encoding). **Optimization stack**: `std::simd` + `wide` SIMD + Bevy parallel systems + spatial partitioning + GPU compute (where deterministic) + memory arenas + zero-allocation hot paths + cache-friendly SoA layout + profile-guided optimization. **AI-agent-driven** perf regression hunts via cfctl benchmarks + flamegraphs + bottleneck identification.

## Decision

### Per-tier perf budgets (per DR-028)

| Tier | Hardware | Frame budget | Sim tick budget |
|---|---|---|---|
| **4K @ 120 Hz** | Strong desktop (modern dGPU) | 8.33ms render | 16.67ms sim @ 60Hz |
| **1080p @ 60 Hz** | Mid-range desktop | 16.67ms render | 16.67ms sim @ 60Hz |
| **800p @ 60 Hz** | Steam Deck OLED | 16.67ms render | 16.67ms sim @ 60Hz |

Each milestone must hit ALL three tiers in its acceptance suite.

### Hot paths inventory

| Hot path | Crate | Target | Optimization |
|---|---|---|---|
| **Material kernel** (chunked CA) | `cf-material` | 32+ active 64×64 chunks @ 60Hz on Steam Deck | SIMD update (8 pixels/SIMD lane); dirty-rect tracker; sleeping chunks; budget governor |
| **Atmospherics** (PV=nRT) | `cf-atmos` | 100+ atmospheres @ 60Hz on Steam Deck | SoA per-gas accumulator; lazy update on connectivity change; SIMD per-gas mole calc |
| **Physics narrowphase** | `cf-physics` | 1000+ entities @ 60Hz | Spatial hash + sleep islands + dynamic AABB tree; SIMD GJK/EPA; CCD only for fast bodies |
| **Replay event recording** | `cf-replay` | 100K events/run @ 60Hz | Pre-allocated event buffer; compressed encoding; async flush to disk |
| **Pathfinding** | `cf-ai`, `cf-pathfinding` | 10+ bots × per-tick replan @ 60Hz | A* with hierarchical mesh; per-bot cooldown; threaded in parallel system |
| **Renderer (terrain + sprites)** | `cf-render-2d` | 4K/120 + Deck/60 | Custom wgpu chunked terrain texture; sprite batching; instanced particles |
| **Network serialization** | `cf-net` | 60Hz snapshot @ MMO scale | Snapshot delta + bit-packed + RLE; per-actor interest set culling |
| **AI utility scoring** | `cf-ai` | 50+ bots × per-tick @ 60Hz | Cached scoring; per-tick budget cap; threaded |
| **LLM Mind (async)** | `cf-llm-mind` | Never blocks sim | Async background; budget-capped; deadline-driven |
| **Animation event tags** | `cf-anim` | Per-frame per-actor @ 60Hz | Frame-key-based tag firing; cached |
| **Lighting (per-light shadow)** | `cf-lighting` | Per-tier per-frame @ 60Hz | Light-volume culling; per-tier shadow LOD |
| **Audio (Steam Audio + Kira)** | `cf-audio-runtime` | 60Hz mix + 32-256 spatial channels | Channel cap + spatial LOD + budget governor |

### Optimization stack

| Technique | Where | When |
|---|---|---|
| **SIMD** (via `std::simd` portable + `wide` crate) | Material kernel + atmospherics + physics narrowphase + math hot paths | M5.5+; verified deterministic |
| **Bevy parallel systems** | Per-system dispatch; system-level threading | M0+ |
| **Archetype-based ECS** (Bevy native) | Component queries + cache-friendly | M0+ |
| **Spatial partitioning** | `cf-physics` broadphase, `cf-ai` perception, `cf-render` culling | M5.5+ |
| **Memory arenas** | Hot paths (per-tick allocation) | M2+ |
| **Object pooling** | Projectiles + particles + decals + Animation states | M5+ |
| **Zero-allocation patterns** | Per-tick hot loops; reuse buffers; pre-allocated capacities | M2+ |
| **Cache-friendly SoA layout** | Per-component data; avoid AoS where vectorizable | M5+ |
| **Profile-guided optimization** (PGO) | Release builds; compiler-driven | M9+ (when scenario library is rich enough) |
| **GPU compute (where deterministic)** | Material kernel scale-up (post-CPU baseline); particle effects | M9+ post-baseline |
| **Lazy initialization** | Per-mod hot-load; per-scenario asset load | M0+ |
| **Reduce allocations in hot loops** | Lint via `cargo-clippy-allocator-stats` | M0+ |

### Determinism preservation under optimization

| Constraint | Enforcement |
|---|---|
| SIMD must produce identical output to scalar fallback | CI test: SIMD path vs scalar; bit-identical assert. |
| GPU compute path must be deterministic | Per-vendor IEEE-754 conformance verified; if not, kept as cosmetic-only OR pre-baked for sim. |
| Multi-threading must be deterministic | Bevy parallel system order respected; per-system seed; no global RNG |
| Object pools must produce identical results across runs | Pool allocation order deterministic |
| Cache miss != different result | Logical correctness independent of cache state |

### Profile-guided optimization workflow

| Step | Tool |
|---|---|
| **Profiling** | Bevy built-in tracing spans; `tracy-client` for flamegraph; `puffin` for in-engine profiler. |
| **Bench** | `cargo bench` per crate; `criterion` for statistical benchmarks. |
| **Per-scenario perf report** | `cf-bench --scenario X --profile Y` outputs `bench_report.json` with frame ms / sim ms / dropped events / memory / GPU memory / cache stats. |
| **Regression detection** | CI baseline + threshold; >5% regression auto-fails. |
| **AI-agent-driven hunt** | `cfctl bench analyze --scenario X --target-tier deck` flags bottleneck systems; suggests optimization. |
| **PGO** | Per-release; collect profile from playtest; rebuild with profile data. |

### CLI testability for perf

Per [[spec/ai-control-observability-layer]] + DR-052 cfctl extension.

| Command | Purpose |
|---|---|
| `cfctl bench --scenario X --profile material --runs 100 --check-checksum-stability` | Per-scenario perf bench. |
| `cfctl bench analyze --scenario X --target-tier deck` | Auto-identify bottleneck system. |
| `cfctl bench memory --scenario X --duration 60s --soak` | Memory leak detection (24h soak). |
| `cfctl bench gpu --scenario X --gpu-time` | Per-GPU-pass timing. |
| `cfctl bench network-snapshot --scenario X --snapshot-size --bandwidth` | Per-snapshot bandwidth profile. |
| `cfctl bench cold-load --scenario X` | First-launch perf measurement. |
| `cfctl bench replay-throughput --bundle X` | Replay throughput per platform. |
| `cfctl bench audio-voice --scenario X --voice-count` | Audio voice budget check. |
| `cfctl bench parallel-system --scenario X --thread-count N` | Per-thread-count scaling. |
| `cfctl bench simd --scenario X --enable-simd false` | SIMD vs scalar comparison. |

### Per-milestone perf gate

Each milestone's done-criteria adds:

> [ ] Per-tier perf budget pass (Steam Deck 800p/60 + 1080p/60 + 4K/120 reference scenes).
> [ ] AI agent-driven perf analysis report logged in milestone vault note.
> [ ] CI bench regression test passing (no >5% regression vs baseline).
> [ ] Memory leak soak (24h+) clean.

### Memory budget

| Tier | RAM budget | GPU memory budget |
|---|---|---|
| **Steam Deck (16GB shared)** | ≤6GB system + ≤4GB GPU | ≤4GB |
| **Mid-range (16GB system + 8GB GPU)** | ≤8GB system | ≤6GB |
| **High-end (32GB+ system + 24GB+ GPU)** | ≤16GB system | ≤16GB |

### Hot-path zero-allocation enforcement

CI lint + per-crate `forbid` attribute:

```rust
#![forbid(clippy::missing_const_for_fn)]
#![deny(clippy::large_stack_allocations)]
#![deny(clippy::large_types_passed_by_value)]
// Per-system zero-allocation assertion (custom lint via cargo-allocator-stats)
```

Hot paths cannot allocate during sim tick; static-sized buffers; arena-pooled.

## What This Locks In

| Spec Area | Implication |
|---|---|
| `cf-bench` | New crate; Criterion-based per-scenario benchmark suite. |
| `cf-perf` | Per-system perf counter aggregator; integrates with run-bundle. |
| `cf-flamegraph` | tracy-client + puffin integration. |
| `cf-allocator-stats` | Custom lint for hot-path allocation detection. |
| `cf-simd-test` | SIMD vs scalar bit-identity test framework. |
| `cf-pgo` | PGO workflow integration. |
| `cfctl` | Extended with `bench *` commands. |
| CI | Per-PR bench regression test; per-platform matrix. |
| Per-milestone done-criteria | Updated to include perf gate. |

## What This Does NOT Lock

| Non-Commitment | Why |
|---|---|
| Specific GPU compute API | Open. Default wgpu compute passes; metal/vulkan equivalents per platform. |
| GPU compute determinism guarantee | Open. Per-vendor IEEE-754 may not match; default fallback to CPU for sim path. |
| Specific PGO toolchain | Open. Default `cargo pgo`; alternative `cargo-pgo`. |
| Console-specific perf paths | Open. Per DR-051 platform extensions; per-platform optimization post-launch. |
| `mimalloc` vs `jemalloc` | Open. Default system allocator; switch if measured benefit. |

## Why This Direction

| Driver | Detail |
|---|---|
| Physics + emulation heavy | Per project owner verbatim; perf is critical. |
| Steam Deck floor mandatory | Per DR-028; 800p/60 floor non-negotiable. |
| Determinism critical | Per DR-002 + DR-052; SIMD/GPU paths must produce identical output. |
| AI-agent-driven optimization | Per DR-026; AI agents drive perf regression hunts via cfctl. |
| Modder parity | Modders' content (additional materials, atmospheres, etc.) must fit in budget; profiler is shared tooling. |
| Replay determinism | Per DR-002 + DR-052; perf paths cannot break determinism. |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| No optimization (rely on hardware) | Steam Deck floor would fail. |
| Single-threaded sim | Modern multi-core hardware underutilized. |
| GPU compute everywhere | Determinism issues; cross-vendor IEEE-754 inconsistencies. |
| C++/native vs Rust | Per DR-024; Rust's safety + perf parity established. |
| Manual memory management | Per Rust; ECS handles pooling via archetype. |
| Defer optimization to post-launch | Steam Deck floor would gate launch. |

## Evidence Trail

- Project owner verbatim (2026-05-06): "we are a physics and emulation heavy game, systems need to be optimized."
- Bevy profiling docs: https://github.com/bevyengine/bevy/blob/main/docs/profiling.md
- Bevy 0.17 release notes: https://bevy.org/news/bevy-0-17/
- Bevy fixed timestep docs: https://docs.rs/bevy/latest/bevy/time/struct.Fixed.html
- Bevy ECS Patterns: https://mcpmarket.com/tools/skills/bevy-ecs-patterns
- Bevy Metrics: https://metrics.bevy.org/
- Bevy Cheat Book Performance: https://bevy-cheatbook.github.io/setup/perf.html
- Steam Deck compatibility checklist: https://partner.steamgames.com/doc/steamdeck/compat
- NVIDIA floating-point determinism guidance: https://developer.nvidia.com/blog/controlling-floating-point-determinism-in-nvidia-cccl/
- "The Essence of Entity Component System" (Tian Zhao): SAC2026 paper.
- `wide` Rust SIMD crate: https://crates.io/crates/wide
- `tracy-client` Rust: https://crates.io/crates/tracy-client
- `puffin` profiler: https://crates.io/crates/puffin
- `criterion` benchmark: https://github.com/bheisler/criterion.rs
- Captured in [[research-log/2026-05-07-comprehensive-audit-report]].

## Revisit Trigger

- Per-tier perf budget exceeded.
- SIMD path produces non-deterministic output.
- GPU compute hot path proves harder than CPU.
- Profile-guided optimization regresses.
- AI agents cannot drive perf regression hunts via cfctl.
