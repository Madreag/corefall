---
type: spec
status: closed-direction
authority: "Performance optimization track for physics-heavy + emulation-heavy game. Hot paths inventory + SIMD + Bevy parallel + spatial partitioning + GPU compute (deterministic) + memory arenas + zero-allocation + cache-friendly SoA + PGO. AI-agent-driven via cfctl benchmarks."
ready_when: "All hot paths SIMD-optimized + multi-threaded; per-tier perf budget pass on Steam Deck/1080p/4K; CI bench regression test active; memory leak soak (24h+) clean; AI agent can drive perf hunts via cfctl."
feeds:
  - DR-001
  - DR-024
  - DR-028
  - DR-033
  - DR-036
  - DR-052
  - DR-054
---

← [[spec/index|spec section]] · [[decisions/dr-054-performance-optimization-and-profiling|DR-054]] · [[decisions/dr-028-visual-fidelity-targets|DR-028]]

# Performance Optimization & Profiling Track

> [!summary] What this page is
> Comprehensive performance contract for the physics-heavy + emulation-heavy game. AI-agent-driven optimization via cfctl benchmarks + flamegraphs + bottleneck identification.

## Per-Tier Perf Budgets

| Tier | Hardware | Frame budget | Sim tick budget |
|---|---|---|---|
| 4K @ 120 Hz | Strong desktop | 8.33ms render | 16.67ms sim @ 60Hz |
| 1080p @ 60 Hz | Mid-range desktop | 16.67ms render | 16.67ms sim @ 60Hz |
| 800p @ 60 Hz | Steam Deck OLED | 16.67ms render | 16.67ms sim @ 60Hz |

## Hot Paths Inventory

| Hot path | Crate | Target | Optimization |
|---|---|---|---|
| Material kernel | `cf-material` | 32+ active 64×64 chunks @ 60Hz Deck | SIMD update; dirty-rect; sleeping chunks; budget governor |
| Atmospherics | `cf-atmos` | 100+ atmospheres @ 60Hz Deck | SoA per-gas; lazy update on connectivity change; SIMD per-gas mole calc |
| Physics narrowphase | `cf-physics` | 1000+ entities @ 60Hz | Spatial hash + sleep islands + dynamic AABB tree; SIMD GJK/EPA; CCD only for fast bodies |
| Replay event recording | `cf-replay` | 100K events/run @ 60Hz | Pre-allocated event buffer; compressed encoding; async flush to disk |
| Pathfinding | `cf-ai`, `cf-pathfinding` | 10+ bots × per-tick replan @ 60Hz | A* with hierarchical mesh; per-bot cooldown; threaded |
| Renderer | `cf-render-2d` | 4K/120 + Deck/60 | Custom wgpu chunked terrain texture; sprite batching; instanced particles |
| Network serialization | `cf-net` | 60Hz snapshot @ MMO scale | Snapshot delta + bit-packed + RLE; per-actor interest set culling |
| AI utility scoring | `cf-ai` | 50+ bots × per-tick @ 60Hz | Cached scoring; per-tick budget cap; threaded |
| LLM Mind (async) | `cf-llm-mind` | Never blocks sim | Async background; budget-capped; deadline-driven |
| Animation event tags | `cf-anim` | Per-frame per-actor @ 60Hz | Frame-key-based tag firing; cached |
| Lighting | `cf-lighting` | Per-tier per-frame @ 60Hz | Light-volume culling; per-tier shadow LOD |
| Audio | `cf-audio-runtime` | 60Hz mix + 32-256 spatial channels | Channel cap + spatial LOD + budget governor |

## Optimization Stack

### SIMD (`std::simd` portable + `wide` crate)

Where: material kernel + atmospherics + physics narrowphase + math hot paths.

CI test: SIMD vs scalar bit-identical.

### Bevy parallel systems

Per-system dispatch; system-level threading. Bevy ECS native.

### Archetype-based ECS

Bevy native. Cache-friendly component queries.

### Spatial partitioning

| System | Method |
|---|---|
| `cf-physics` broadphase | Dynamic AABB tree |
| `cf-ai` perception | Spatial hash |
| `cf-render` culling | Quad-tree |

### Memory arenas

Per-frame arena for hot-path allocation.

```rust
// cf-arena (illustrative)
let arena = Bump::new();  // bumpalo crate
let intermediate = arena.alloc_slice(...);
// Used during tick; reset at frame end.
```

### Object pooling

Projectiles + particles + decals + animation states.

```rust
pub struct ProjectilePool {
    pool: Vec<Projectile>,
    free: Vec<usize>,
}

impl ProjectilePool {
    pub fn spawn(&mut self) -> &mut Projectile {
        let idx = self.free.pop().unwrap_or_else(|| {
            self.pool.push(Projectile::default());
            self.pool.len() - 1
        });
        &mut self.pool[idx]
    }

    pub fn despawn(&mut self, idx: usize) {
        self.pool[idx].reset();
        self.free.push(idx);
    }
}
```

### Zero-allocation patterns

Per-tick hot loops; reuse buffers; pre-allocated capacities.

```rust
// Lint via custom cargo-allocator-stats
#![deny(clippy::large_stack_allocations)]
#![deny(clippy::large_types_passed_by_value)]
```

### Cache-friendly SoA layout

```rust
// AoS (bad for vectorization)
struct Actor { pos: Vec2, vel: Vec2, health: f32, ... }

// SoA (good for vectorization)
struct ActorSet {
    pos: Vec<Vec2>,
    vel: Vec<Vec2>,
    health: Vec<f32>,
    ...
}
```

### Profile-guided optimization

| Step | Tool |
|---|---|
| Profile | Bevy tracing spans + tracy-client |
| Bench | criterion |
| Per-scenario report | `cf-bench --scenario X --profile Y` |
| Regression detect | CI baseline + threshold |
| AI-agent hunt | `cfctl bench analyze --scenario X --target-tier deck` |
| PGO | `cargo pgo` per release |

### GPU compute (deterministic)

Material kernel scale-up + particle effects. Deterministic verified per-vendor.

```rust
// Material kernel: CPU baseline; optional GPU dispatch
fn material_update(state: &mut MaterialState, gpu_enabled: bool) {
    if gpu_enabled && is_deterministic_gpu() {
        gpu_dispatch_material_update(state);
    } else {
        cpu_simd_material_update(state);
    }
}
```

## Determinism Preservation

| Constraint | Enforcement |
|---|---|
| SIMD = scalar bit-identical | CI test: SIMD vs scalar |
| GPU determinism | Per-vendor IEEE-754 verified; fallback CPU if not |
| Multi-threading deterministic | Bevy parallel system order respected; per-system seed |
| Object pools deterministic | Pool allocation order |
| Cache miss != different result | Logical correctness independent of cache |

## CLI Testability

| Command | Purpose |
|---|---|
| `cfctl bench --scenario X --profile material --runs 100 --check-checksum-stability` | Per-scenario perf bench |
| `cfctl bench analyze --scenario X --target-tier deck` | Auto-identify bottleneck |
| `cfctl bench memory --scenario X --duration 60s --soak` | Memory leak detection |
| `cfctl bench gpu --scenario X --gpu-time` | Per-GPU-pass timing |
| `cfctl bench network-snapshot --scenario X --snapshot-size --bandwidth` | Per-snapshot bandwidth |
| `cfctl bench cold-load --scenario X` | First-launch perf |
| `cfctl bench replay-throughput --bundle X` | Replay throughput |
| `cfctl bench audio-voice --scenario X --voice-count` | Audio voice budget |
| `cfctl bench parallel-system --scenario X --thread-count N` | Per-thread scaling |
| `cfctl bench simd --scenario X --enable-simd false` | SIMD vs scalar comparison |

## Per-Milestone Perf Gate

Per DR-056 universal enhancement. Each milestone done-criteria includes:

- [ ] Per-tier perf budget pass.
- [ ] AI agent-driven perf analysis report.
- [ ] CI bench regression test (no >5% regression).
- [ ] Memory leak soak (24h+) clean.

## Memory Budget

| Tier | RAM budget | GPU memory |
|---|---|---|
| Steam Deck (16GB shared) | ≤6GB system + ≤4GB GPU | ≤4GB |
| Mid-range (16GB+8GB GPU) | ≤8GB system | ≤6GB |
| High-end (32GB+24GB GPU) | ≤16GB system | ≤16GB |

## Hot-Path Zero-Allocation Enforcement

CI lint via `cargo-allocator-stats`:

```bash
$ cargo allocator-stats --crate cf-physics --hot-path
[INFO] Per-tick allocations: 0
[INFO] PASS: zero-allocation in hot path verified
```

## Done-Criteria

- [ ] All hot paths SIMD-optimized + multi-threaded.
- [ ] Per-tier perf budget pass.
- [ ] CI bench regression test active.
- [ ] Memory leak soak (24h+) clean.
- [ ] AI agent can drive perf hunts via cfctl.
- [ ] PGO workflow integrated.
- [ ] Per-milestone perf gate verified.

## Source Trail

- [[decisions/dr-054-performance-optimization-and-profiling]]
- Bevy profiling: https://github.com/bevyengine/bevy/blob/main/docs/profiling.md
- Bevy 0.17 release notes: https://bevy.org/news/bevy-0-17/
- Bevy fixed timestep docs: https://docs.rs/bevy/latest/bevy/time/struct.Fixed.html
- Bevy ECS Patterns: https://mcpmarket.com/tools/skills/bevy-ecs-patterns
- Steam Deck compatibility checklist: https://partner.steamgames.com/doc/steamdeck/compat
- NVIDIA floating-point determinism guidance: https://developer.nvidia.com/blog/controlling-floating-point-determinism-in-nvidia-cccl/
- `wide` SIMD crate: https://crates.io/crates/wide
- `tracy-client`: https://crates.io/crates/tracy-client
- `puffin` profiler: https://crates.io/crates/puffin
- `criterion`: https://github.com/bheisler/criterion.rs
- `bumpalo` arena: https://crates.io/crates/bumpalo
