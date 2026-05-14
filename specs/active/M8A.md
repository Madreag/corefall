# M8A — Parallel Determinism, GPU Offload, Server Architecture

## Status

`active`

## Intent

Refactor the sim core so the game can simulate massive, deterministic, networked physics on modern many-core hardware without compromise — replacing the M1-M8 single-threaded `RwLock<EngineMutable>` foundation with a parallel-deterministic ECS architecture, GPU-offloaded cosmetic systems, and a locked server / networking contract that the M9 firehose (chemistry, atmospherics, bleeding, fragmenting, ricochets, multi-actor scenes) and M15 active material kernel can land on without a re-architect.

M8A also locks Corefall's **server-authoritative + semantic-terrain-event + LAN-lockstep + internet-rollback** multiplayer architecture — the hybrid model recommended by the vault (`cortext_command_vault/engine/network-terrain-replication-lifecycle.md` and `systems/replay-determinism-and-run-evidence.md`) after auditing CCCP / OpenSoldat / OpenLieroX / Powder Toy / Photon Quantum / Gaffer On Games as comparables. The same `cf-headless serve` binary runs on workstation x86_64 (9950X3D + RTX 5090), Apple Silicon (M4 Pro / M5+), and headless Linux VPS; every topology hits the same cross-OS determinism CI gate.

This milestone is intentionally massive. The implementing agent has explicit authority to refactor any subsystem in `cf-*/` crates to meet the contracts in this spec. No subsystem is sacred. Breaking changes to internal APIs are expected and welcome; the only invariants that must hold are: (1) the cfctl JSON-RPC wire contract from M0-M8 keeps working unchanged, (2) the run-bundle envelope locked at M4 stays additively-compatible, (3) per-tick `determinism.sim_checksum` reproduces byte-for-byte across hosts and re-runs of the same inputs.

### Target hardware — no-compromise reference platforms

**Client (single-player + LAN host + competitive client):**
**AMD Ryzen 9 9950X3D** (16 cores / 32 threads, 3D V-cache), **NVIDIA RTX 5090** (32 GB VRAM), 48 GB DDR5. Steam Deck and integrated-GPU machines are explicitly OUT of scope; M8A unlocks the engine's ceiling, not its floor. Lower-tier hardware will run M1-M8 era content fine; M9+ content density may exceed Steam Deck's thermal budget and that is acceptable.

**Dedicated server (authoritative-host topology):** at least one of:

| Tier | Hardware | Use case | Reason |
|---|---|---|---|
| Server-Workstation-x86 | Ryzen 9 9950X3D OR Threadripper 7000-series, NVIDIA RTX 5090 or 4090, 64+ GB DDR5, NVMe, 1+ Gbps NIC | Dedicated public servers, large LAN tournaments, server-side replay rendering | Same sim cost ceiling as client + GPU available for replay rendering + server-side cosmetic stress tests |
| Server-Apple-Silicon | **Mac Studio / Mac mini M4 Pro** (10P+4E cores, 273 GB/s memory bandwidth) OR **M5 Pro / M5 Max** when shipped | Indie-host shards, persistent-world nodes, datacenter Apple racks | High single-thread perf (matches 9950X3D at 50-70% of perf-per-thread), unified memory, low TDP, no driver-divergence concerns since GPU is excluded from determinism island |
| Server-Linux-VPS | 16+ vCPU Linux x86_64 box (Hetzner AX102 / OVH Game / AWS c7i.4xlarge), no GPU required, 32+ GB RAM, 10 Gbps NIC | Cloud-hosted public servers, MMO shard backbone | GPU offload is presentation-only — server doesn't need one. Same `cf-headless serve` binary as workstation; uses `cf-render-2d/stub` feature for zero-GPU compile |
| Server-Apple-Mini-Lab | Mac mini M4 Pro 64 GB / 24 GB cluster (3-5 nodes, 10 GbE switch) | Persistent-MMO dev cluster, internal QA shard, friends-and-family hosting | DR-035 prerequisite — shard-aware mesh requires cheap, low-power, deterministic-quiet hardware to validate world-merge protocols before cloud deployment |

The server tier matters because Corefall is **server-authoritative** (per DR-005, DR-034) — the server runs the canonical sim, all clients reconcile against it. Server perf-per-thread directly bounds tick rate × scene density × player count. Apple Silicon M4 Pro / M5+ is a real first-class server target, not a fallback; it must hit byte-identical determinism with x86 in the cross-OS CI gate (`m8a_cross_os_determinism.sh`).

## Player-facing behavior

Player doesn't see new content from M8A — but every scene the player ever experiences after M8A runs on a different engine. Specifically:

- **Frame rate doesn't drop** in scenes that previously stuttered. A reactor breach in `micro_reactor_defense` with 50+ falling debris pixels + 8 hazard zones + 4 actors holds 60 FPS on the reference platform (was: dropped to ~45 FPS on the single-thread foundation). 120 Hz mode hits 120 FPS sustained, not just average.
- **Massive scenes are playable.** Spawning 200 NPCs, 500 in-flight projectiles, 1000 active hazard pixels (mid-explosion), and a 2km×2km chunked terrain world all at once does not regress per-tick budget below 16.6 ms.
- **Multiplayer is real.** Two-to-eight players in `mp_lan_room` see the same world state byte-for-byte. Player A blowing a hole appears on Player B's screen within one network round-trip. Player B's local prediction reconciles with the authoritative server without snap-back stuttering thanks to rollback netcode primitives. Internet multiplayer (≤ 200 ms one-way) is playable at 60 Hz simulation rate with input prediction.
- **Replay viewer reads from a richer perf surface.** `summary.json.performance` carries per-subsystem p50/p99/p999 latencies in microseconds (was: rolled-up totals only). M10 replay viewer renders flame-graph-style "hot tick" overlays so review sessions can pinpoint perf regressions tick-by-tick.
- **The same scene runs the same way on every player's machine and every server topology.** Mac M3 client, Windows on 9950X3D client, Linux on a Threadripper client, Mac mini M4 Pro server, Hetzner Linux VPS server — same scenario.ron + same seed + same cfctl inputs → byte-identical run bundles across every node. This is the determinism island contract from DR-052, now enforced by CI on x86_64-linux, x86_64-windows, aarch64-darwin matrix.
- **Powerful servers are first-class.** A Mac mini M4 Pro 64 GB or a Hetzner Linux VPS without a GPU can run an authoritative `cf-headless serve` instance at 60 Hz for 8 clients. The server doesn't need a renderer; it just needs the deterministic sim core + cf-net transport. Apple Silicon (M4 Pro and later) and x86_64 Linux datacenter machines both pass the same cross-OS determinism CI gate as Windows clients.
- **Cosmetic events scale without budgeting risk.** Particle effects, debris rendering, damage numbers, blood splatter, shell ejection — all GPU-side, all `cosmetic: true`, all droppable under backpressure with zero impact on sim. Player sees 10,000 sparks during a reactor breach and the sim doesn't even know.

## Crates / modules touched

| Crate | Status | What changes |
|---|---|---|
| `cf-sim-core` | MAJOR REFACTOR | Replace `RwLock<EngineMutable>` with Bevy ECS components + systems. Introduce `SimStage` enum (PreSim, Input, Actors, AI, Projectiles, Terrain, Mission, Atmos, Recorder) with explicit dependency graph. Each stage is a Bevy `SystemSet` that the parallel scheduler runs concurrently when system signatures permit. Determinism preserved via snapshot-read / compute-parallel / commit-serial pattern. |
| `cf-control` | MAJOR REFACTOR | Move engine state out of one mega-struct into ECS components. `EngineMutable` is decomposed into: `ActorWorldComponent`, `ChunkedTerrainComponent`, `MissionComponent`, `RecorderComponent`, `ProjectilePoolComponent`, `ReactorWorldComponent`, etc. Each behind its own lock OR migrated to ECS query pattern. cfctl JSON-RPC handlers continue to work — they read/write ECS state via World queries. |
| `cf-actor` | MODIFY | `ActorState` becomes an ECS component bundle (`ActorBundle { Pos, Vel, Aim, Stance, Status, HP, Stability, ... }`) so the parallel scheduler can run actor systems concurrently with disjoint per-entity writes. Per-tick step is split into `apply_intent` / `step_kinematics` / `derive_status` / `latch_outcomes` sub-systems that run with explicit ordering. |
| `cf-ai` | MAJOR REFACTOR | ReactiveGuard becomes an ECS component (`GuardComponent { state, params, perception, memory_grid, ... }`). Per-guard tick runs in `par_iter` over guard entities. RNG calls pre-rolled into a `Vec<u64>` of length N before the parallel loop; each guard indexes into it by stable id mod. Determinism preserved across N=1 to N=200 guards. |
| `cf-physics` | MODIFY | Projectile sim becomes ECS-iterable. Add Verlet integrator option (forward-compat for fluid sim at M19). Penetration formula already pure; promote to `par_iter` over projectiles with snapshotted terrain reads. |
| `cf-terrain` | MODIFY | `ChunkedTerrain` becomes a Bevy resource (not boxed in `RwLock`). Chunk update becomes per-chunk parallel via `par_iter_mut` over the dirty-chunk set. Add `active_region: bool` enforcement on every Chunk write (M15 forward-compat — only "awake" chunks tick the CA when M15 lands). Sub-chunk dirty-rect tracking (already partly in place from M3) becomes the canonical mutation contract. |
| `cf-replay` | MODIFY | `Recorder` gains a thread-local buffer per Bevy worker thread. End-of-tick merge into the canonical event stream is single-threaded but deterministic (sorted by event_id which already encodes tick+seq). Adds `RecorderShard` per-stage buffer to keep the merge bounded. **Per-subsystem perf samples** rolled into `summary.json.performance` with p50/p99/p999 microseconds. |
| `cf-render-2d` | MAJOR REFACTOR | Move cosmetic particle system to GPU compute shader. Move material overlay tinting to a fragment shader (currently CPU-tinted). Add GPU-side sprite batch buffer pool. Chunked terrain texture upload uses one-time `Texture2DArray` allocation + per-tick sub-rect writes (was: one Texture per chunk; massive descriptor-set churn). |
| `cf-net` | NEW | New crate. `cf-net::server` (authoritative headless server entry point). `cf-net::client` (input prediction + rollback netcode primitives). `cf-net::snapshot` (delta-encoded world snapshots). `cf-net::transport` (QUIC over UDP via `quinn`; WebSocket fallback for LAN web clients). Locked wire protocol versioned via `prototype-net-frame.v0.1`. |
| `cf-headless` | MODIFY | Gains `cf-headless serve` subcommand — runs the authoritative server with no rendering. Reuses the deterministic sim core; emits the same run bundles a single-player session emits (server-side bundles are the "source of truth" for the lobby). `cf-headless replay` continues to work unchanged. |
| `cf-app` | MODIFY | Network client wiring: `--connect <addr:port>` flag joins a server; local single-player remains the default. Bevy plugin order rewritten so render systems are non-blocking on sim stage completion (currently sim and render run on the same task pool — wasteful on 16-thread machines). |
| `cf-bench` | MAJOR EXPANSION | Add stress-test scaffolds: `m9_firehose` (50 actors + 200 projectiles + 100 hazard pixels + 10 reactor armor layers + destruction events every 30 ticks), `m15_ca_burst` (synthetic 100K active pixels), `m22_pathfinder_load` (placeholder), `mp_8player_lan` (8-client deterministic lockstep). Each emits per-subsystem p50/p99/p999 ms into a CI-parseable JSON. |
| `cf-mod` | MODIFY | New validator path `validate-bundle <dir>` enforces the M8A perf contract — every bundle's `summary.json.performance` must include all required subsystem entries; bundles missing perf data are rejected. |
| Profiling | NEW | Optional `tracy` integration behind a `profiling/tracy` feature flag. Optional `puffin` viewer behind `profiling/puffin`. Off by default (zero cost in release). Both integrate with Bevy's existing diagnostic plugin. |

## Files

Source — refactor + new:

- `game/crates/cf-sim-core/src/lib.rs` (MAJOR REFACTOR: SimStage enum + ECS scheduler hooks)
- `game/crates/cf-sim-core/src/scheduler.rs` (NEW: deterministic parallel work-graph)
- `game/crates/cf-sim-core/src/snapshot.rs` (NEW: snapshot-read pattern utilities)
- `game/crates/cf-control/src/engine.rs` (MAJOR REFACTOR: ECS migration of `EngineMutable`)
- `game/crates/cf-control/src/components.rs` (NEW: ECS component types)
- `game/crates/cf-control/src/world.rs` (NEW: Bevy World wrapper with cfctl JSON-RPC bridge)
- `game/crates/cf-actor/src/components.rs` (NEW: actor ECS bundle)
- `game/crates/cf-actor/src/systems.rs` (NEW: per-tick sub-systems split for parallel execution)
- `game/crates/cf-ai/src/components.rs` (NEW: guard ECS bundle)
- `game/crates/cf-ai/src/systems.rs` (NEW: per-tick guard system with par_iter + pre-rolled RNG)
- `game/crates/cf-physics/src/parallel.rs` (NEW: parallel projectile sweep + terrain penetration)
- `game/crates/cf-terrain/src/parallel.rs` (NEW: per-chunk parallel mutation + deterministic merge)
- `game/crates/cf-terrain/src/active_region.rs` (NEW: active-region wake/sleep state machine)
- `game/crates/cf-replay/src/shard.rs` (NEW: per-thread event buffer + end-of-tick merge)
- `game/crates/cf-replay/src/perf.rs` (NEW: per-subsystem p50/p99/p999 sampler)
- `game/crates/cf-render-2d/src/gpu_particles.rs` (NEW: compute-shader particle pool)
- `game/crates/cf-render-2d/src/gpu_overlay.rs` (NEW: fragment-shader material overlay)
- `game/crates/cf-render-2d/src/terrain_texture_array.rs` (NEW: chunked terrain via Texture2DArray)
- `game/crates/cf-render-2d/shaders/particles.wgsl` (NEW: GPU compute shader for cosmetic particles)
- `game/crates/cf-render-2d/shaders/material_overlay.wgsl` (NEW: fragment shader for 5-mode overlay)
- `game/crates/cf-net/Cargo.toml` (NEW: crate manifest, depends on quinn + serde)
- `game/crates/cf-net/src/lib.rs` (NEW)
- `game/crates/cf-net/src/server.rs` (NEW: authoritative server loop)
- `game/crates/cf-net/src/client.rs` (NEW: client prediction + rollback)
- `game/crates/cf-net/src/snapshot.rs` (NEW: delta-encoded world snapshots)
- `game/crates/cf-net/src/transport.rs` (NEW: QUIC + WebSocket transport)
- `game/crates/cf-net/src/protocol.rs` (NEW: locked wire protocol v0.1)
- `game/crates/cf-net/src/rollback.rs` (NEW: rollback netcode primitives)
- `game/crates/cf-net/schemas/v0_1/net_frame.schema.json` (NEW: wire envelope)
- `game/crates/cf-headless/src/main.rs` (MODIFY: add `serve` subcommand)
- `game/crates/cf-headless/src/server_mode.rs` (NEW)
- `game/crates/cf-app/src/main.rs` (MODIFY: --connect flag + Bevy plugin reorder)
- `game/crates/cf-app/src/network_client.rs` (NEW: cf-net::client integration)
- `game/crates/cf-bench/src/m9_firehose.rs` (NEW)
- `game/crates/cf-bench/src/m15_ca_burst.rs` (NEW)
- `game/crates/cf-bench/src/m22_pathfinder_load.rs` (NEW)
- `game/crates/cf-bench/src/mp_8player_lan.rs` (NEW)
- `game/crates/cf-bench/src/perf_assert.rs` (NEW: CI-parseable threshold checks)
- `game/crates/cf-mod/src/validate_bundle.rs` (NEW: bundle perf contract validator)

Content + scripts:

- `game/scripts/cfctl/bench_m9_firehose.cfctl.json` (NEW: drives the M9 stress test)
- `game/scripts/cfctl/bench_8player_lan.cfctl.json` (NEW: 8-client lockstep replay)
- `game/scripts/ci/m8a_perf_gate.sh` (NEW: runs all benches, asserts thresholds, fails CI on regression)
- `game/scripts/ci/m8a_cross_os_determinism.sh` (NEW: runs same script on Linux+macOS+Windows, diffs final checksums, fails on mismatch)
- `game/content/scenarios/bench_m9_firehose.ron` (NEW)
- `game/content/scenarios/bench_mp_8player.ron` (NEW)

Documentation (terse — code is the source of truth):

- `docs/plan/spec/determinism-island-contract.md` (NEW: explicit list of deterministic vs cosmetic subsystems, RNG seeding rules, float associativity rules, hash seeding rules)
- `docs/plan/spec/parallel-scheduler-contract.md` (NEW: SimStage dependency graph, snapshot-read/compute-parallel/commit-serial pattern, recorder merge contract)
- `docs/plan/spec/network-protocol-v0_1.md` (NEW: cf-net wire protocol, frame types, version negotiation, rollback semantics, lobby flow)
- `docs/plan/spec/perf-budget-contract.md` (NEW: per-subsystem ms budgets, p99 thresholds, regression-gate rules)

Schemas:

- `game/crates/cf-replay/schemas/event/perf_sample.json` (NEW: per-tick subsystem latency event, cosmetic=true so dropped under backpressure)
- `game/crates/cf-net/schemas/v0_1/net_frame.schema.json` (NEW)
- `game/crates/cf-replay/src/lib.rs` (MODIFY: extend `PerformanceBlock` with required subsystem entries)

## Acceptance criteria

### Parallel determinism contract

```gherkin
Scenario: ECS migration replaces RwLock<EngineMutable>
  Given the current engine state behind RwLock<EngineMutable>
  When M8A refactor is applied
  Then engine state lives in Bevy ECS components (ActorWorldComponent, ChunkedTerrainComponent, MissionComponent, etc.)
  And no subsystem holds the entire engine state behind one mutex
  And cfctl JSON-RPC handlers continue to work unchanged (no wire-protocol break from M0-M8 contracts)
  And cargo test --workspace passes 100% of the M1-M8 test suite

Scenario: Sim systems run in parallel where dependencies allow
  Given 50 actors + 200 projectiles + 100 hazard pixels in flight
  When drive_tick runs
  Then actor sim systems and projectile sim systems execute concurrently on separate cores
  And per-tick wall-clock latency on the reference platform (9950X3D) is < 8 ms at p99
  And cargo run --release -p cf-bench m9_firehose holds 60 Hz for 1000 ticks

Scenario: Snapshot-read / compute-parallel / commit-serial preserves determinism
  Given the same cfctl input script run 10 times
  Then every run produces the same per-tick determinism.sim_checksum
  And the final run_id's blake3 final-state checksum is byte-identical across all 10 runs
  And no parallel system mutates shared state mid-tick (verified by Bevy's system-conflict detector + a runtime assertion that catches mutable cross-system access)

Scenario: AI tick runs in parallel across guards
  Given a scenario with 50 reactive guards
  When the AI tick fires
  Then guard systems run via par_iter over the entity set
  And RNG calls inside the parallel section read from a pre-rolled Vec<u64> indexed by stable guard id
  And running the same scenario at 60 Hz vs 120 Hz with the same seed produces stable cross-rate behavior (mission outcome identical; cross-rate exact-checksum match is M4's CI matrix scope but per-rate stability is M8A's)

Scenario: Per-chunk parallel terrain mutation
  Given an explosion that touches 8 chunks in one tick
  When ChunkedTerrain processes the chunk-mutation work
  Then chunk updates run in par_iter over the dirty-chunk set
  And inter-chunk boundary effects are handled in a single-threaded post-pass over the dirty-chunk coord-sorted list
  And the resulting per-tick blake3 checksum is identical to the equivalent single-threaded reference run
```

### GPU compute offload

```gherkin
Scenario: Cosmetic particles run on GPU
  Given a reactor breach spawning 5000 debris pixels
  When the particle system steps each frame
  Then particle position + velocity integration happens in a wgsl compute shader
  And the CPU emits ONE batched terrain.debris_spawned event with debris_count=5000 (not 5000 individual events)
  And cf-replay's recorder is NOT touched per particle (the events are cosmetic only)
  And frame budget on the reference platform (RTX 5090) is < 1 ms for 50,000 active particles

Scenario: Material overlay uses fragment shader
  Given the 5-mode overlay is active (integrity / pathability / mobility / hazard / build_repair)
  When the player presses M to cycle modes
  Then the overlay tint is applied per-fragment in material_overlay.wgsl (not per-pixel on CPU)
  And mode switch latency is < 1 frame
  And the legend HUD continues to read MaterialAffordance::overlay_rgba (CPU side; for HUD display)

Scenario: Chunked terrain uses Texture2DArray (not per-chunk Texture)
  Given a world with 200+ allocated chunks
  When the renderer uploads dirty-rect updates
  Then ALL chunks share one Texture2DArray with the chunk count as the array layer dimension
  And per-tick GPU descriptor-set bindings stay constant (was: O(chunk_count) bind churn)
  And M3's per-chunk dirty_rect contract continues to drive the sub-rect upload (no architecture regression)

Scenario: GPU work does NOT enter the determinism checksum
  Given GPU particles are flagged cosmetic: true
  When cf-headless replay verifies a bundle
  Then GPU particle state is ignored entirely (no checksum bytes consumed from the GPU)
  And the determinism island contract holds: GPU is presentation, never sim authority
```

### Server / networking foundation

```gherkin
Scenario: cf-net authoritative server runs the sim headlessly
  Given cf-headless serve --port 30000 --scenario mp_lan_room
  When 4 clients connect
  Then the server runs one canonical sim instance
  And each tick the server sends a delta-encoded snapshot to each client (over QUIC)
  And the server writes a server-side run bundle to prototype_runs/server/<run-id>/ (the lobby's source of truth)
  And per-client bandwidth at p99 < 50 kB/s during heavy combat

Scenario: Lockstep client mode for LAN
  Given 4 clients on the same LAN with cf-app --connect <ip>:30000 --net-mode lockstep
  When a player blows a hole in the wall
  Then all 4 clients see the carve on the same simulation tick (no client-side prediction needed at LAN latencies)
  And per-tick blake3 checksum matches across all 5 nodes (server + 4 clients)
  And no client snaps state back (zero observable correction)

Scenario: Rollback netcode for internet play
  Given 2 clients on the internet (one-way latency 50-200 ms) with cf-app --connect <addr> --net-mode rollback
  When client A locally predicts forward N frames
  And client A receives a server-confirmed input that diverges from prediction at frame F
  Then client A rolls back to frame F, re-applies confirmed inputs, and re-simulates forward N frames in < 8 ms total resimulation cost
  And the player observes at most one frame of corrective state-snap (or none, if prediction matched)
  And rollback resimulation re-uses the same deterministic sim core (no separate code path)

Scenario: Input delay + RTT compensation
  Given a client with 100 ms RTT to the server
  When the player presses fire
  Then local prediction fires immediately (zero perceived input lag)
  And the server's authoritative fire event matches the client's predicted event within N=6 simulation frames
  And mispredictions trigger a rollback correction (visible only when prediction was wrong)

Scenario: Wire protocol version-locked
  Given a v0.1 client connecting to a v0.1 server
  Then handshake succeeds with negotiated protocol="prototype-net-frame.v0.1"
  And a v0.2 client connecting to a v0.1 server is rejected with "protocol_version_mismatch" before any sim frame
  And protocol v0.1 envelope shape is documented at docs/plan/spec/network-protocol-v0_1.md and never breaks (additive-only extensions)

Scenario: Server bundle is the lobby's source of truth
  Given a 4-player session with one player disconnecting mid-mission
  When the session ends and the host writes the bundle
  Then prototype_runs/server/<run-id>/ contains the full canonical event stream (no missing-client gaps)
  And per-client bundles diff against the server bundle and report any divergence as a CI-flaggable test fail
  And replay of the server bundle reproduces the same final state on any host (deterministic across player machines)
```

### Cross-platform determinism

```gherkin
Scenario: Same bundle replays byte-identically on Linux + macOS + Windows
  Given an m9_firehose bundle generated on Linux/x86_64
  When the same bundle is replayed on macOS/aarch64 (Apple Silicon)
  And on Windows/x86_64 (9950X3D reference)
  Then all three final blake3 checksums match byte-identically
  And all three per-tick checksums match at every cadence point
  And the cross-OS CI gate (game/scripts/ci/m8a_cross_os_determinism.sh) passes

Scenario: f32-only in sim islands (DR-052)
  Given any sim crate (cf-sim-core, cf-physics, cf-material, cf-ai, cf-terrain, cf-atmos)
  When the codebase is grep'd for `f64`
  Then no `f64` appears in any field, function signature, or computation within the determinism island
  And the cf-mod static analyzer (or a clippy lint via cargo-cranky) rejects PRs that introduce f64 in sim crates
  And FP intrinsics that diverge across hardware (FMA, AVX-512 wide reductions) are gated behind a stable code path

Scenario: HashMap iteration order locked
  Given any state that crosses the determinism boundary (recorder events, mission state, AI memory grid, chunk coords)
  Then the data structure is BTreeMap or BTreeSet (deterministic iteration) OR FxHashMap with a fixed seed (fast + deterministic)
  And no std::collections::HashMap with default RandomState hasher exists in sim crates
  And a CI test enumerates every public field on every sim-relevant struct and asserts the type is deterministic-iterable
```

### Performance budgets + regression gate

```gherkin
Scenario: Per-subsystem p99 latency budgets enforced
  Given the per-tick budget contract at docs/plan/spec/perf-budget-contract.md
  When cf-bench m9_firehose runs on the reference platform
  Then p99 latencies stay within budget:
  - actor sim ≤ 1.5 ms
  - AI sim ≤ 2.0 ms
  - projectile sim ≤ 1.0 ms
  - terrain mutation + dirty batch ≤ 2.5 ms
  - mission director ≤ 0.2 ms
  - recorder + checksum + merge ≤ 0.5 ms
  - render dispatch ≤ 4.0 ms
  - headroom ≥ 4.0 ms
  And total p99 ≤ 15.5 ms (under the 16.6 ms 60 Hz budget)
  And the perf gate script (game/scripts/ci/m8a_perf_gate.sh) fails CI on any subsystem regressing by ≥ 25% vs the locked baseline

Scenario: summary.json.performance carries per-subsystem p50/p99/p999 microseconds
  Given any run bundle generated post-M8A
  Then summary.json.performance has keys for every required subsystem
  And each key has p50_us, p99_us, p999_us fields (microsecond precision)
  And cf-mod validate-bundle rejects bundles missing required perf keys
  And M10 replay viewer renders a per-tick perf timeline overlay

Scenario: Stress-test scaffolds available as cargo benches
  Given the user runs `cargo run --release -p cf-bench m9_firehose --ticks 6000`
  Then the bench runs the M9 worst-case scenario (50 actors + 200 projectiles + 100 hazard tiles + reactor armor + destruction every 30 ticks)
  And emits per-subsystem p50/p99/p999 to stdout AND a CI-parseable JSON at /tmp/bench_m9_firehose.json
  And the same bench accepts --tick-rate-hz 120 for the high-rate variant
  And `cargo run --release -p cf-bench m15_ca_burst --ticks 1000` is a forward-compat placeholder that allocates 100k active pixels and times the chunk-CA stepping (CA itself lands at M15; the scaffold ships at M8A so M15 has a baseline to beat)

Scenario: Memory allocation pressure in steady state is ≤ 0 allocs/tick
  Given a 5-minute m1_5min_endurance run on the reference platform
  When the bundle's memory-profile output is inspected
  Then the steady-state region (tick 300 onward) shows zero heap allocations on the sim thread per tick
  And the recorder per-thread buffers are pre-allocated to a configurable capacity
  And actor / guard / projectile pools are pre-allocated; no per-tick Vec::new() / HashMap::new() / Box::new() in any hot path

Scenario: Profiling hooks ship behind a feature flag
  Given the `profiling/tracy` feature is enabled at compile time
  When the engine runs
  Then per-system span markers emit to a Tracy server on localhost:8086
  And every cf-* crate has `#[profiling::function]` on its tick-hot entry points
  And with the feature OFF (release builds by default) the profiling hooks compile to zero-cost no-ops
```

### Cellular automaton readiness (M15 forward-compat)

```gherkin
Scenario: active_region flag is enforced on every chunk write
  Given a pixel write at world position (px, py)
  When the write commits via ChunkedTerrain::set_pixel_internal
  Then the affected chunk's active_region flag is set to true
  And neighboring chunks within a 1-chunk radius have active_region set to true (M15's wake-on-edit pattern; chunks that haven't seen a write in N=300 ticks transition back to active_region=false)
  And the wake/sleep transitions emit terrain.chunk_active_region_changed events (cosmetic=false; deterministic state)

Scenario: Chunk CA stepping is parallel-ready
  Given a scenario with 50 active_region=true chunks (M15-equivalent stress)
  When the (placeholder M8A) CA stub runs
  Then chunks are processed in par_iter over the active-chunk set
  And inter-chunk boundary writes go through a single-threaded post-pass that processes chunks in (cx, cy) ascending order
  And the resulting per-tick blake3 checksum matches the single-threaded reference

Scenario: M15 baseline measurement
  Given the m15_ca_burst bench at M8A (CA placeholder; just iterates over the active-pixel set)
  When the bench runs at 100k active pixels for 1000 ticks
  Then it completes within < 4.0 ms per tick at p99 (the budget reserved for M15's real CA)
  And the measurement is captured as the M15 baseline-to-beat
```

### Reference-platform performance targets

```gherkin
Scenario: 60 Hz hold during reactor breach
  Given the micro_reactor_defense_loss scenario on the reference platform
  When the reactor explodes and 8 hazard zones spawn
  Then per-tick wall-clock latency stays under 16.6 ms at p99 for the full 90-second scenario
  And frame rate as observed by the player stays at 60 FPS for the full scenario (no perceptible stutter)

Scenario: 120 Hz hold during ordinary scene
  Given the m1_actor_range scenario at --tick-rate-hz 120 on the reference platform
  When the player moves, fires, reloads, swaps items, sharp-aims
  Then per-tick wall-clock latency stays under 8.3 ms at p99 for the full 5-minute endurance run
  And frame rate at 120 FPS sustained

Scenario: 200-actor massive scene
  Given the bench_m9_firehose stress test with 200 NPCs + 500 projectiles + 1000 hazard pixels active simultaneously
  When the bench runs for 6000 ticks
  Then per-tick wall-clock latency stays under 16.6 ms at p99
  And memory usage stays under 8 GB resident (under 17% of the 48 GB reference)
  And GPU usage stays under 50% on the RTX 5090 (room for renderer ladders at M10+M14)

Scenario: 8-client LAN multiplayer
  Given bench_8player_lan with 8 clients on the same LAN
  When the bench runs for 1000 ticks
  Then per-tick latency at p99 stays under 16.6 ms on every client
  And per-client bandwidth stays under 100 kB/s
  And all 8 clients + 1 server agree on the final blake3 checksum byte-for-byte
```

### Vault-sourced acceptance tests (DET-A-01..07 — replay/determinism harness)

These test ids come from `cortext_command_vault/systems/replay-determinism-and-run-evidence.md` § "Acceptance Tests". M8A ships the test fixtures for every one of them; each becomes a CI gate.

```gherkin
Scenario: DET-A-01 — Input replay probe
  Given a 30-second fixed-seed actor run captured at M1-M8
  When the captured input intent stream is replayed against the M8A engine
  Then the same control commands fire through the same tick sequence
  And the replayed run's per-tick determinism.sim_checksum matches the original byte-for-byte

Scenario: DET-A-02 — Checksum surface cadence
  Given any run bundle generated at M8A or later
  When the bundle is validated by cf-mod validate-bundle
  Then sim_checksum events appear at a fixed cadence (every checksum_cadence_ticks)
  And summary.json reports checksum_mismatch_count = 0 (or names the mismatch with first_divergent_tick)

Scenario: DET-A-03 — First divergence report
  Given a run with an injected checksum mismatch at tick T
  When cf-headless replay-validate is invoked on the bundle
  Then the validator reports first_divergent_tick = T
  And the nearest parent event (within 5 ticks of T) is identified by event_id
  And the divergence category is classified (rng_drift | float_drift | hash_iteration | terrain_diff | actor_state | projectile_state)

Scenario: DET-A-04 — Snapshot restore smoke test
  Given an actor + inventory snapshot at tick T captured during a 10-minute mission
  When the snapshot is restored on a fresh engine instance (cold-start)
  Then the actor's position, velocity, status, HP, stamina, inventory items + slot, selected_weapon round-trip exactly
  And the post-restore tick produces the same observable behavior as the original tick T+1
  And the death-recap replay viewer (M10) can use the restored snapshot as a scrub anchor

Scenario: DET-A-05 — Terrain chunk evidence per dig/explosion
  Given one dig or explosion in any test scenario
  When the action commits via ChunkedTerrain
  Then a terrain.chunk_mutated event emits with chunk_coords, bbox, delta_materials, post_state_checksum
  And the bundle's summary.json.terrain_chunks_dirtied reports the chunk count and total bytes of delta_materials
  And the cross-OS CI gate re-runs the same input and produces the same post_state_checksum on Linux/macOS/Windows

Scenario: DET-A-06 — Equipment causality through replay
  Given a loadout run where actor A selects a weapon, fires at actor B, hits a body part, deals damage, B drops a weapon
  When the bundle's events.jsonl is inspected
  Then the event chain is: equipment.role_selected → projectile.fire → projectile.hit → damage.applied → inventory.item_dropped
  And every link in the chain carries the same parent_event_id reference (replay-event-architecture canonical lineage)
  And the replay viewer can render the full chain in M10's causality graph

Scenario: DET-A-07 — Run-bundle hygiene under M8A
  Given any M8A run bundle (single-player OR server-side)
  When prototype_run_check.py and cf-mod validate-bundle both run
  Then both pass with zero findings
  And every acceptance criterion in M8A cites a real event_id schema (cf-replay/schemas/event/*.json)
  And the bundle round-trips through compressed (zstd) + decompressed forms without checksum change
```

### Server hardware acceptance tests

```gherkin
Scenario: Same scenario byte-identical on x86-Linux + Apple-Silicon + Windows
  Given the bench_m9_firehose scenario with --seed 12345 and --ticks 6000
  When the scenario runs on three reference platforms:
    - Linux x86_64 (Ryzen 9 9950X3D)
    - macOS aarch64 (Apple M4 Pro Mac mini, 64 GB)
    - Windows x86_64 (Ryzen 9 9950X3D)
  Then all three final blake3 checksums match byte-identically
  And every cadence-point sim_checksum matches across all three
  And cross-OS gate (game/scripts/ci/m8a_cross_os_determinism.sh) passes
  And the per-tick p99 latency on each platform is documented in summary.json.performance.platform

Scenario: Apple Silicon server runs cf-headless serve without GPU
  Given a Mac mini M4 Pro server with no external GPU
  When cf-headless serve --port 30000 --scenario mp_lan_room --no-render starts
  Then the server starts without attempting to allocate a wgpu surface
  And the server accepts 8 client connections + processes inputs + emits snapshots
  And the per-tick wall-clock latency at p99 stays under 16.6 ms (60 Hz hold)
  And per-client bandwidth stays under 100 kB/s
  And the server emits a server-side run bundle with the same envelope shape as the workstation server

Scenario: VPS-class Linux server hits 60 Hz hold under 4-client mp_lan_room
  Given a 16-vCPU Linux x86_64 VPS (Hetzner AX102 or equivalent), no GPU, 32 GB RAM
  When cf-headless serve --port 30000 --scenario mp_lan_room runs the m9_firehose stress
  Then per-tick p99 stays under 16.6 ms for the full 1000-tick run
  And the cross-OS checksum gate passes against a workstation-class run of the same inputs

Scenario: Apple M4 Pro / M5+ server cross-OS checksum gate
  Given any cf-net mission run on a Mac mini M4 Pro server
  When the same input trace replays on an x86_64 Linux server
  Then the final blake3 checksum matches byte-for-byte
  And cf-headless replay-validate passes on both bundles independently
```

### Refactor authorization

```gherkin
Scenario: Implementing agent has explicit authority to refactor any cf-* crate
  Given an existing M1-M8 subsystem
  When the M8A agent identifies it as blocking parallel execution / GPU offload / network determinism
  Then the agent may refactor the subsystem internals freely
  And the only constraints are: (1) cfctl JSON-RPC wire shape stays additive-only-compatible, (2) the run-bundle envelope stays additive-only-compatible, (3) per-tick determinism.sim_checksum byte-reproduces across re-runs of the same inputs
  And breaking changes to internal Rust APIs between crates are expected and welcomed
```

## Out of scope

- **The actual M15 active material kernel (cellular automaton).** M8A ships the scaffolding (`active_region` enforcement, chunk-parallel CA stub, baseline measurement) so M15 can land on a parallel foundation. The CA rules themselves (sand falls, water pools, fire spreads, gas rises, chemistry reactions) land at M15.
- **The actual M22 pathfinder.** M8A ships the `dirty_region` + `path_invalidated` event surfaces (already in place at M3) and reserves a parallel-iter scaffold for pathfinder requests. The actual pathfinder lands at M22.
- **Cross-machine cheat detection / anti-cheat.** Server is "trusted host" model at M8A. Cheating in competitive play is a topic for M30+ when ranked matchmaking ladders into scope.
- **VR / spectator mode.** Out of scope. Spectators connect as zero-input clients at M8A; full broadcast-quality spectator UI lands at M11+ HUD workbench.
- **Mod sandbox / scripted mod hot-reload.** cf-mod validates static content; runtime mod scripting (Lua / WASM) lands at M30+.
- **Compute-shader CA (M15 GPU path).** The spec literal commits to CPU-side parallel CA at M15 for determinism. A GPU compute path for CA is a future BP4+ optimization gated on cross-vendor reproducibility research.
- **Replay sharing prototype.** Out of scope (BP6+ public-cloud upload + curated bundle ladder).
- **Voice chat / proximity audio over the network.** cf-audio is local-only at M8A; networked audio is M40+ optional.
- **Steam Deck / Linux ARM / iOS / Android.** No-compromise platform target is the reference desktop. Lower tiers may run M1-M8 era content fine but M9+ density is not guaranteed playable. Mobile/handheld ports are not on the M-series roadmap.
- **Save game format migration.** Save/load lands at M13+. M8A doesn't change save format; ECS-component refactor must round-trip the M1-M8 save format unchanged or break it deliberately with a version bump at M13.

## Dependencies

- **M1-M8 closed.** M8A refactors the foundation; the content the foundation supports must exist before refactoring.
- **Bevy 0.18.1 pinned.** ECS migration uses Bevy's parallel scheduler. Upgrades to Bevy 0.19+ are scheduled work; M8A locks the 0.18 surface.
- **Rust toolchain 1.95+ (current `rust-toolchain.toml`).** `let_chains` + `is_some_and` + `if let chains` used throughout the refactor.
- **`quinn` crate for QUIC.** Add as a `cf-net` dependency; pin version in workspace `Cargo.toml`.
- **`rayon` 1.10+ pinned.** Used for `par_iter` over entity sets with deterministic work-stealing seed.
- **`wgpu` (already pulled by Bevy 0.18.1).** Compute shader support requires a wgpu backend with compute (every Vulkan/Metal/DX12 target on the reference hardware tier).
- **`hwloc` (optional).** NUMA-aware thread pinning for the 9950X3D's CCX layout. Behind a feature flag; defaults off for portability.

## Notes for the implementer

### Architecture rules

- **No subsystem holds the entire engine state behind one mutex.** The M1-M8 `RwLock<EngineMutable>` was fine for early single-thread development; it is the primary blocker for parallel sim. Migrate to Bevy ECS components + queries. Bevy's scheduler runs disjoint-write systems in parallel automatically. We pay zero runtime cost for parallelism; we just stop blocking ourselves.
- **Snapshot-read / compute-parallel / commit-serial is the canonical pattern** for every sim subsystem that scales with entity count. Read previous-tick state (frozen). Compute into per-worker buffers (parallel, lock-free). Single-threaded merge at end of tick (deterministic sort key — typically entity id ascending). This is the same pattern Factorio uses.
- **Determinism trumps parallelism.** Every parallel addition must be paired with a CI test that proves the resulting per-tick checksum is identical to the single-threaded reference. If a refactor breaks determinism, it gets reverted regardless of perf gains.
- **GPU is presentation, never sim.** Anything on the GPU is `cosmetic: true` and excluded from the determinism island. If a feature MUST be deterministic, it MUST stay on CPU. Particle systems, post-FX, debris rendering, blood splatter, shell ejection — all GPU + cosmetic. Reactor explosions, damage calculations, projectile trajectories, AI decisions — all CPU + deterministic.
- **Cosmetic events drop first under backpressure.** Recorder priority threshold: cosmetic events are evicted when the ring buffer pressure exceeds 80%. Determinism-critical events are NEVER dropped (recorder grows or stalls before evicting them). The `dropped_count` field on the envelope tracks cosmetic loss.
- **Network is authoritative-server-or-lockstep, never client-authoritative.** Server is the source of truth. LAN clients can run lockstep (cheap, low-latency, no prediction needed). Internet clients run input prediction + rollback against the server's authoritative timeline.

### Comparable simulation games — what they teach us

The vault at `/Users/erol/projects/cortex-command-repos-all` carries first-hand source-level evidence from six destructible-2D / heavy-sim comparables, plus published research from the four big-engine references (Unreal replays, Photon Quantum, Gaffer On Games, YellowAfterlife). The lessons below shape M8A's parallel-determinism + networking design directly. **The implementing agent should treat this section as decisive prior art**, not optional reading.

#### CCCP / C4 — bitmap-delta terrain replication is the anti-pattern

CCCP and the C4 continuation engine actually ship live multiplayer (RakNet-era) with NAT punch-through and a dedicated server mode. **The lesson is mostly what NOT to do**:

| What CCCP did | Why M8A rejects it |
|---|---|
| Terrain changes serialized as **raw bitmap deltas** (`MsgTerrainChange` with X/Y/W/H + FG/BG bytes + LZ4 compression, fragmented to ~1280 pixels per chunk). | Bitmap deltas are huge (kilobytes per explosion), can't be replayed independently of the bitmap snapshot, and cannot be server-authoritatively re-derived from inputs. M8A uses **semantic terrain events** (see below). |
| Frame/scene transmission **mixed with** terrain/audio/effects in one message stream. | Couples sim-critical updates with cosmetic ones — no way to drop a cosmetic without dropping sim. M8A keeps `cosmetic: true` separation in the wire protocol. |
| Multiplayer activities **commented out in the active Meson build** because of unresolved sync issues. | Half-shipped multiplayer poisons the project's reputation. M8A ships LAN lockstep first (where it's actually achievable), then internet rollback. |
| Terrain mutations bypass `AddUpdatedMaterialArea` in some code paths, leading to pathfinding desync. | M8A's `ChunkedTerrain` enforces dirty-region tracking at the write barrier — every terrain change emits a deterministic `terrain.chunk_mutated` event with the post-state checksum. |

Source: `cortext_command_vault/engine/network-terrain-replication-lifecycle.md` + CCCP's `Source/Managers/NetworkServer.cpp` + `Source/System/NetworkMessages.h`.

#### The Powder Toy — snapshot field contract for deterministic restore

The Powder Toy's `Snapshot` (`comparables_repos/the-powder-toy/src/simulation/Snapshot.h`) is the gold-standard reference for a cellular-automaton snapshot: every field you need to byte-identically restore the simulation. It's also the cleanest existing reference for `SnapshotDelta` (forward + restore + bidirectional diffs). M8A adopts its field shape verbatim for the snapshot envelope:

```
Snapshot {
  // Air / atmosphere fields
  AirPressure: Vec<f32>          // pressure grid (per-cell)
  AirVelocityX: Vec<f32>          // horizontal flow grid
  AirVelocityY: Vec<f32>          // vertical flow grid
  AmbientHeat: Vec<f32>           // temperature grid

  // Particles (M9-M15 firehose: actors + projectiles + debris + hazard pixels)
  Particles: Vec<Particle>

  // Gravity fields (forward-compat for DR-038 universal gravity / DR-039 celestial bodies)
  GravForceX: Vec<f32>
  GravForceY: Vec<f32>
  GravMass: Vec<f32>
  GravMask: Vec<u32>

  // Terrain (CCCP-equivalent foreground + background terrain + electrical/fan grids)
  BlockMap: Vec<u8>
  ElecMap: Vec<u8>               // forward-compat for circuit/cable systems
  BlockAir: Vec<u8>
  BlockAirH: Vec<u8>
  FanVelocityX: Vec<f32>
  FanVelocityY: Vec<f32>

  // Special objects (DR-027+ doors, signs, stickmen are the Corefall actor/sign/door equivalent)
  PortalParticles: Vec<Particle>  // forward-compat for M21+ teleporter scope
  WirelessData: Vec<i32>
  stickmen: Vec<PlayerState>
  signs: Vec<Sign>

  // Determinism anchors
  FrameCount: u64                // tick number — MUST round-trip to deterministic.tick_id
  RngState: RngState             // the canonical RNG state — restored before any sim step
  Authors: Bson                  // provenance — content hashes, mod ids, engine build
}
```

**Hard rules adopted from Powder Toy verbatim**:

1. **`FrameCount` is the snapshot's primary deterministic key**. Two snapshots with the same FrameCount on different machines must be byte-identical or report a checksum diff with first-divergent-tick context.
2. **`RngState` is restored before any sim step on rollback or replay**. Failing to do this guarantees divergence.
3. **`SnapshotDelta` carries both forward and restore paths** (`SnapshotDelta::Forward(old) → new`, `SnapshotDelta::Restore(new) → old`). Rollback netcode uses Restore; replay scrubbing uses both.
4. **Snapshots are taken at deterministic cadence** (every N ticks where N is a power of 2; 64 is a reasonable default). Cosmetic events DO NOT trigger snapshots.

Source: `comparables_repos/the-powder-toy/src/simulation/Snapshot.h` + `SnapshotDelta.h` + `Snapshot.cpp` + `SnapshotDelta.cpp`.

#### OpenSoldat — modernization patterns for legacy 2D multiplayer

OpenSoldat is the best reference for modernizing legacy 2D multiplayer (Pascal codebase + RakNet → GameNetworkingSockets, dedicated client/server split, content purity, lobby/launcher separation). The pattern is mature and battle-tested:

| OpenSoldat pattern | Corefall adoption (M8A) |
|---|---|
| GameNetworkingSockets for transport | `quinn` for QUIC + WebSocket fallback (same role, more modern, Rust-native). Reliable streams for canonical events, unreliable datagrams for inputs + snapshot deltas. |
| Dedicated client/server split | `cf-headless serve` (server-only binary, no GPU) + `cf-app` (client, with renderer). Same deterministic sim core; renderer is the only thing the server omits. |
| Shared network message definitions | `cf-net/src/protocol.rs` — wire frames are versioned (`prototype-net-frame.v0.1`) and locked at M8A; additive-only extensions |
| `sv_pure` SHA1 content hashing for server purity | Content manifest hash on every join — server rejects clients whose manifest checksum diverges from the server's. Per DR-013 / `backend-service-hub-slice-a`. |
| `soldat://` deep links for joins | `corefall://join?server=<addr>&token=<one-time>&content_hash=<sha>` — version-locked, content-aware, no plain passwords in URI |
| Cvars + config files (IdTech / Source style) | Server runtime config via cvars surfaced through cfctl JSON-RPC (e.g., `cfctl srv.set_cvar net.snapshot.cadence_ticks 30`). Locked at M8A; configurable from any control surface that speaks cfctl. |
| Static JSON lobby prototype (OpenSoldat-lobby) | `cf-net/lobby` minimum slice — server discovery returns server list with version + protocol + content_hash + region + map + ruleset + humans + bots + trust_tier. Per `cortext_command_vault/systems/networking-backend-frontend.md` § "Backend Service Slice A". |

Sources: `cortext_command_vault/systems/networking-backend-frontend.md` § "OpenSoldat Networking Lessons" + `comparables_repos/opensoldat-base/` + `comparables_repos/opensoldat-launcher/` + `comparables_repos/opensoldat-lobby/`.

#### OpenLieroX — destructible-2D + rope physics + NewNet's lessons

OpenLieroX added save/restore/checksum rollback to a destructible-arena LAN/online game. It's the closest comparable to Corefall's combat shape (destructible terrain + tethered mobility + projectile-heavy fights). Two lessons:

| OpenLieroX evidence | Corefall consequence |
|---|---|
| Protocol enums include explicit `C2S_CARVE`, worm updates, damage reports, NewNet keys + checksums, Gusanos messages, shoot events, weapon selection, file transfer | **Every Corefall destruction action is its own protocol enum**. `terrain.dig`, `terrain.blast`, `terrain.fill`, `projectile.fire`, `weapon.select`, `actor.damage`, `mission.objective_state` — no monolithic state-blob frames |
| Worm packets serialize position + controls + weapon + rope + velocity + timing **as one bundle** | Per-actor sync bundle includes tether/jet/grapple state as first-class fields, not hidden inside transform |
| Server-side packet read accepts client movement fields and can carve terrain when server sees the worm in dirt | **M8A rejects this**: server is the only authority, client sends inputs only, server simulates the carve. LAN clients in lockstep mode run the sim independently with identical inputs (no client → server state push). |
| NewNet's save/restore is **half-built and outdated** — the restore path is not finished, but planned checksum/save/restore architecture exists | Treat as a warning: **rollback that compiles is not rollback that works**. M8A requires a full M9-stress-test rollback test (`mp_8player_lan` bench) before declaring rollback shipped. |

Sources: `cortext_command_vault/systems/networking-backend-frontend.md` § "OpenLieroX Networking Lessons" + `comparables_repos/openlierox/src/common/NewNetEngine.cpp` + `comparables_repos/openlierox/src/client/CClient_Game.cpp`.

#### Photon Quantum — content-hash + RngState + deterministic replay

Photon Quantum is the closest modern commercial reference for deterministic-lockstep replay. The replay envelope they ship is:

- Input history (one trace per player)
- Deterministic config (sim parameters that affect math)
- Runtime config (engine build version)
- Asset database hash (`content_hashes` in Corefall's run-bundle envelope)
- Initial frame / tick data (snapshot at tick 0)
- Optional per-cadence checksums (Corefall's `determinism.sim_checksum` at fixed cadence)

M8A adopts this envelope shape for the network handshake and for the replay-as-server-bundle path. The `run_manifest.json` already locks the same fields (`engine_build`, `content_hashes`, `sim_tick_rate`, `rng_policy`, `determinism_mode`) at M4.

Source: `cortext_command_vault/systems/replay-determinism-and-run-evidence.md` § "Research Comparison: Photon Quantum".

#### Gaffer On Games / Glenn Fiedler — lockstep + snapshot interp + rollback

The canonical modern netcode references. Three core patterns M8A uses verbatim:

1. **Deterministic lockstep** (Gaffer, "Deterministic Lockstep" article) — send per-frame inputs, not state; require bit-identical results across all peers; buffer inputs to hide jitter. LAN mode at M8A uses this directly. Corefall's f32-only / no-FMA / no-default-hasher rules are the standard hardness profile for cross-machine bit-identity.
2. **Snapshot interpolation** (Gaffer, "Snapshot Interpolation" article) — server sends authoritative snapshots, clients render interpolated state between received snapshots. Internet mode at M8A uses this for cosmetic-state rendering; sim-critical state goes through rollback.
3. **Rollback netcode** (GGPO / Quantum / fighting-game lineage) — client locally predicts forward N frames; server-confirmed inputs trigger rollback to first-divergent frame + resim forward. M8A's `cf-net::rollback` implements this primitive. The 6-frame rollback budget at p99 ≤ 8 ms is the hard constraint that drives the per-tick budget to ≤ 2.5 ms in single-step.

Source: `cortext_command_vault/systems/replay-determinism-and-run-evidence.md` § "Research Comparison: Gaffer deterministic lockstep" + "Gaffer snapshot interpolation".

#### Factorio / Noita / other heavy-sim references

These are not in our local vault but inform the M8A design pattern catalog:

- **Factorio** ships deterministic lockstep multiplayer on **single-threaded sim** + heavily parallelized non-sim systems (rendering, sound, GUI, save serialization). Their approach: ECS-like flat data layout, snapshot-read for all reads, commit-serial for all writes. M8A's "snapshot-read / compute-parallel / commit-serial" pattern is their pattern, generalized.
- **Noita** ships chunked CA with per-chunk wake/sleep + parallel chunk update. Their snapshot is per-chunk; their RNG is deterministic per-chunk. M8A's `active_region` flag + chunk-parallel mutation + deterministic per-chunk merge is the Noita pattern, ported.
- **Dyson Sphere Program** / **Stationeers** / **RimWorld** all ship single-thread sim + parallel non-sim. The takeaway: **sim parallelism is rare and hard**. M8A's snapshot-read/commit-serial pattern is the safest way to parallelize sim without breaking determinism. We're not the first; we're following the herd.

The Factorio + Noita patterns confirm the path: a Bevy ECS scheduler with `par_iter` over disjoint entity sets + a single-threaded commit phase + per-tick checksum is the standard modern shape. It's been validated by published shipping titles. M8A operationalizes it for Corefall.

### Server authority model — server-authoritative-entity + semantic-terrain-events

Per the vault's multiplayer architecture options matrix (`cortext_command_vault/engine/network-terrain-replication-lifecycle.md` § "Multiplayer Architecture Options"), Corefall has four viable options:

| Option | Fit for Corefall | Decision |
|---|---|---|
| Remote-render / server streaming | High bandwidth, latency feel, poor responsiveness — what CCCP shipped. | **REJECTED**. Frame streaming forces server to render and stream pixels per client; defeats the purpose of a powerful client GPU. |
| Deterministic lockstep | Attractive for LAN at small player counts (≤ 8). | **ADOPTED for LAN mode** with the M8A determinism contract enforcing f32 + RNG seeding + sorted iteration. |
| Server-authoritative entity + semantic terrain events | Best long-term online option per the vault. | **ADOPTED as primary**. Server runs canonical sim; clients send inputs and receive entity snapshots + semantic terrain events; clients predict + reconcile against server. |
| Co-op only with limited player count | Realistic first step. | **ADOPTED as M8A acceptance criterion**: 8-client LAN multiplayer is the M8A target; 16-client internet rollback is post-M8A. |

Combined model: **server-authoritative-entity + semantic-terrain-events + deterministic-lockstep-on-LAN + rollback-with-prediction-on-internet**. Same `cf-net::server` binary supports both modes; client selects mode at connect time (`--net-mode {lockstep,rollback}`).

This combination matters because Corefall's terrain mutates **continuously** (per DR-007 / `terrain-mutation-and-pathfinding-lifecycle.md`). A bitmap-delta protocol (CCCP) is too bandwidth-heavy; a pure deterministic-lockstep protocol (Factorio-on-internet) is too fragile under floating-point drift across architectures. The hybrid model splits the cost: LAN gets cheap lockstep, internet gets prediction-friendly snapshots + rollback for desync recovery.

### Semantic terrain event protocol — replace CCCP's bitmap deltas

Every terrain mutation emits a **semantic** event (not a bitmap delta). The semantic event carries the cause (what mutation), the geometric scope (what changed), and the post-state checksum (what to validate). Bitmap data ships as a **fallback** for chunks where the semantic-event chain has been lost or compressed.

Wire shape:

```
TerrainEvent {
  cause: TerrainCause,           // dig | blast | fill | settle | stamp | door_toggle | hazard_burn
  chunk_coords: (i32, i32),      // M3 chunk indexing
  bbox: (u16, u16, u16, u16),    // dirty-rect within chunk (x, y, w, h)
  delta_materials: Vec<MaterialChange>,  // per-pixel material changes — small, semantic, replayable
  post_state_checksum: [u8; 32], // blake3 of the chunk after the mutation
  cause_event_id: Option<EventId>, // forward link: the explosion event that caused this carve
  tick: u64,
}

MaterialChange {
  px: u16, py: u16,              // local-to-chunk coords
  old_material: u8,              // material id BEFORE
  new_material: u8,              // material id AFTER
}
```

**Critical compression**: a single 20-pixel dig is ~80 bytes (event header + 20 × 4-byte material changes + 32-byte checksum). The same dig as a bitmap delta is ~400 bytes (raw RGBA + LZ4 framing overhead). Across a typical M9 firehose scene, semantic events are ~5x smaller. Bandwidth budget falls within the 100 kB/s per-client cap.

**Fallback path**: if a client falls more than 256 ticks behind, the server sends a full chunk snapshot (compressed) instead of replaying every semantic event. This is the snapshot-interpolation pattern from Gaffer, applied per-chunk.

**Reconciliation invariant**: at every cadence point (every 64 ticks), the server emits a `chunk.checksum_probe` with the full post-state blake3 of each dirty chunk. Clients verify their predicted state against the probe. Any mismatch triggers a chunk-level snapshot resync.

### Snapshot field contract (Powder Toy-derived, Corefall-localized)

The canonical M8A snapshot envelope is the Powder Toy field shape, localized to Corefall's data layout:

```rust
struct Snapshot {
    // Determinism anchors (REQUIRED on every snapshot)
    tick: u64,                         // == FrameCount
    rng_state: cf_sim_core::RngState,  // restored before any sim step on rollback/replay
    engine_build: String,              // semver of the engine binary that wrote the snapshot
    content_hashes: BTreeMap<String, String>, // mod / scenario / weapon / actor hashes

    // World fields (REQUIRED for full restore)
    chunks: BTreeMap<(i32, i32), ChunkSnapshot>,
    actors: BTreeMap<ActorId, ActorSnapshot>,
    projectiles: BTreeMap<ProjectileId, ProjectileSnapshot>,
    mission: MissionSnapshot,
    inventory_world: InventoryWorldSnapshot,
    reactor_world: Option<ReactorWorldSnapshot>,

    // Per-chunk fields (Powder-Toy-equivalent)
    air: Option<AirSnapshot>,          // forward-compat for DR-037 stationeers-grade atmospherics
    ambient_heat: Option<HeatGridSnapshot>, // forward-compat for DR-040 environmental hazards
    grav_field: Option<GravFieldSnapshot>, // forward-compat for DR-038 universal gravity
    fan_velocity: Option<FanFieldSnapshot>, // forward-compat for hazard-zone airflow

    // Provenance (Powder Toy "Authors" field)
    authored_by: Vec<AuthorTag>,       // engine version + map author + mod authors + scenario author

    // Cosmetic state (NOT determinism-critical; restored best-effort)
    cosmetic: CosmeticSnapshot,        // particle counts, damage-number positions — never used for sim
}

struct ChunkSnapshot {
    coords: (i32, i32),
    material_grid: Vec<u8>,            // pixel-aligned, RLE-compressed at transport time
    dirty_rect_history: Vec<DirtyRect>, // last N ticks of mutations for delta-base support
    active_region: bool,               // M3 forward-compat — sleeping chunks don't tick the CA
    post_state_checksum: [u8; 32],     // chunk-level blake3
}
```

**Invariants**:

1. `tick + rng_state + chunks + actors + projectiles + mission` is the **minimum byte set required** to byte-identically restore the sim. Everything else is forward-compat for later milestones.
2. **Snapshot delta encoding**: every snapshot can be expressed as a delta against a base snapshot. `SnapshotDelta::Forward(old) → new` for applying server updates; `SnapshotDelta::Restore(new) → old` for rollback.
3. **Snapshot cadence**: every 64 ticks at M8A (configurable via cfctl `srv.set_cvar net.snapshot.cadence_ticks <n>`). Mismatch with replay tooling is allowed (replay tooling reads the actual cadence from the bundle).
4. **No `cosmetic` fields cross the determinism boundary**. The snapshot's `cosmetic` block is informational only; verifying snapshots ignores it.



The implementing agent MUST NOT violate these. CI gates enforce each:

1. **No `f64` in sim crates.** Use `f32` only. f32 has stable cross-platform bit-exact behavior under IEEE 754; f64 is fine for display/math-paper purposes but its hardware-acceleration paths diverge subtly across vendors.
2. **No `thread_rng()`.** Every RNG call in a sim crate uses `cf_sim_core::Rng` seeded from the engine's seed. Pre-roll RNG into a `Vec<u64>` of length N BEFORE entering a `par_iter` block; workers index by stable entity id.
3. **No `HashMap` with default `RandomState` hasher in sim state.** Use `BTreeMap` (deterministic iteration) OR `FxHashMap` (fast, but iteration is non-deterministic — only use when iteration order doesn't cross the checksum boundary). Audit every cf-* crate and replace.
4. **No FMA / wide AVX-512 intrinsics that vary by hardware.** Use the stable f32 code path. Disable LLVM `-ffast-math` (already off by default in our workspace; double-check in `.cargo/config.toml`).
5. **No `Instant::now()` / `SystemTime::now()` in sim code.** Real time is non-deterministic. Use the engine's `Clock::tick().0` for any time-like value.
6. **No `format!()` / `String::from()` allocations in tick-hot paths.** Use `tracing::Span` with structured fields. Strings format only at log-render time (off by default in release).
7. **No `Vec::new()` / `HashMap::new()` per tick in hot paths.** Pre-allocate in the relevant ECS resource at engine init. Clear-and-reuse, never re-allocate.
8. **Recorder events crossing the determinism boundary MUST have stable `event_id`s** (already enforced by the recorder; just don't bypass it).
9. **Cross-thread state mutation MUST be either (a) per-worker buffer with deterministic merge, or (b) explicit single-threaded post-pass.** Atomic-CAS-loops on shared state are FORBIDDEN in sim (they introduce thread-scheduling-dependent ordering).
10. **No `std::sync::Mutex` in sim hot paths.** Use Bevy's ECS query system which provides lock-free disjoint access by default.

### Parallel scheduler (canonical SimStage dependency graph)

```
PreSim (clock advance, RNG initialization, dirty buffer reset)
  ↓
Input (cfctl + keyboard intent → ControlIntent component on player entity)
  ↓
ActorPrePass (status derived from previous-tick HP, mass-scaled accel/jump precompute)
  ↓
[parallel: ActorTick, AITick, ProjectileTick]    ← snapshot-read previous-tick state
  ↓
TerrainMutation (dig/blast/fill — single-writer to ChunkedTerrainComponent)
  ↓
[parallel: HazardContact, AnchorContact]
  ↓
ActorPostPass (apply hits, derive new status, latch dying / dwell)
  ↓
MissionTick (objectives, timer, lifecycle)
  ↓
RecorderMerge (per-thread shards → canonical event stream, sorted by event_id)
  ↓
ChecksumEmit (per-cadence determinism.sim_checksum)
  ↓
[parallel: PerfSampleEmit (cosmetic), GpuParticleStep (cosmetic)]
```

Bevy's scheduler runs the `[parallel: ...]` rows concurrently when system signatures permit. Workers in those rows must NOT mutate state read by sibling workers in the same row — enforced by Bevy's query-conflict detector at runtime.

### Recorder shard merge contract

Per-thread `RecorderShard`s emit events with `event_id = (tick, monotonic_seq_in_shard)`. At `RecorderMerge` stage:

1. Each shard sorts its events by `monotonic_seq_in_shard` (already in insertion order).
2. Cross-shard merge orders events by `(tick, shard_id, monotonic_seq_in_shard)` — `shard_id` is a stable per-system-stage assignment, NOT the runtime thread id.
3. After merge, every event is re-stamped with a canonical `event_id = (tick, canonical_seq)` where `canonical_seq` is the post-merge position in the stream.
4. `parent_event_id` references are re-mapped from shard-local ids to canonical ids during merge.

This preserves determinism across thread-scheduling variance: same input → same per-shard event sequence → same merge order → same canonical event stream → same blake3 checksum.

### Network protocol v0.1

Wire frame (locked at M8A; additive-only extensions):

```
NetFrame {
    version: u16  // protocol_version, locked to 1 at v0.1
    seq: u32      // monotonic per-sender frame counter
    timestamp_ms: u64  // sender's monotonic clock at send time (telemetry only; not authoritative)
    payload: NetPayload  // tagged union
}

NetPayload =
  | Handshake { client_version, capabilities[], session_token }
  | InputCommand { tick: u64, intent_event_id: String, control_command: <reuse cf-control::ControlCommand serde shape> }
  | SnapshotDelta { from_tick: u64, to_tick: u64, deltas: Vec<EntityDelta> }
  | EventBatch { tick: u64, events: Vec<RecorderEvent> }  // events.jsonl rows over the wire
  | ChecksumProbe { tick: u64, checksum_hex: String }  // for debug + drift detection
  | Ping { send_ms: u64 } | Pong { send_ms: u64, recv_ms: u64 }
  | Disconnect { reason: String }
```

- **QUIC transport (via `quinn`).** Reliable streams for the canonical event stream; unreliable datagrams for low-latency input + snapshot deltas (loss is corrected by the next snapshot).
- **WebSocket fallback** for in-browser future spectator clients (M11+ scope).
- **Per-frame max size**: 1450 bytes (Ethernet MTU minus IP+UDP+QUIC headers; avoids fragmentation).
- **Snapshot delta encoding**: bit-packed per-component delta; full-snapshot keyframe every 60 ticks.
- **Lockstep mode**: clients send only `InputCommand` frames; server broadcasts back the merged input set for the target tick; all clients then sim-step that tick locally with identical inputs. Per the spec's hard determinism rules, output is byte-identical.
- **Rollback mode**: client extrapolates with locally-predicted inputs; receives server-confirmed inputs and rolls back any tick where the prediction was wrong; resimulates forward. Resimulation re-uses the same deterministic sim core (no separate code path for rollback).

### Reference-platform measurement methodology

All M8A perf claims must be measured on the reference platform OR explicitly noted as estimates. Methodology:

- Hardware: 9950X3D (16 cores / 32 threads), RTX 5090 (32 GB VRAM), 48 GB DDR5-6000, NVMe SSD.
- OS: Windows 11 24H2 (primary) + Linux 6.x (secondary for cross-platform determinism CI).
- Bench harness: `cargo bench` with the `--release` profile + `RUSTFLAGS="-C target-cpu=znver4"`.
- Per-tick latency measurement: `Instant::now()` deltas at SimStage entry/exit, captured in `cf-replay::PerfSample` events, percentile rolled over a 1000-tick window.
- Frame rate measurement: `bevy_diagnostic::FrameTimeDiagnosticsPlugin`'s rolling average over 60 frames.
- Bandwidth measurement: `cf-net::transport` per-second byte counter, p99 over 60-second window.

### Recommended order of operations for the implementing agent

1. **Lock the determinism contract first.** Ship `docs/plan/spec/determinism-island-contract.md` AND the CI gate that enforces it (no f64, no thread_rng, no default HashMap hasher in sim crates) BEFORE refactoring any code. This prevents the refactor from accidentally drifting.
2. **Audit current state.** Inventory every `RwLock<EngineMutable>` write site, every `HashMap<...>` declaration in sim crates, every `thread_rng()` call, every `f64` in sim. Generate a single audit report at `docs/plan/audit-m8a-baseline.md`.
3. **Ship the perf-budget contract + stress-test scaffolds NEXT.** With baselines locked, every subsequent refactor is measurable.
4. **Migrate one subsystem at a time to ECS.** Order: `cf-actor` → `cf-ai` → `cf-physics` → `cf-terrain` → `cf-replay` → `cf-control`. Each migration must pass the full M1-M8 test suite before the next begins.
5. **Add GPU offload AFTER ECS migration is complete.** GPU compute shaders depend on stable component types; build the foundation first.
6. **Ship `cf-net` LAST.** Network code is the most cross-cutting; ship it after the sim core has settled. Server mode first (`cf-headless serve`), then LAN lockstep, then internet rollback.
7. **Each step lands its own commits + tests + perf measurements.** Don't batch a 20k-line PR. Land 5-15 PRs over the M8A window so review + bisect stay tractable.

### Anti-patterns to avoid (the M6-M11 team's rubric, re-stated for M8A)

- `O(actors²)` interactions. Use spatial hashing or a quad-tree.
- Per-tick `Vec::new()` / `HashMap::new()` allocations in hot paths. Pre-allocate, clear-and-reuse.
- `format!()` in hot paths. Use `tracing` structured fields.
- `HashMap` with default hasher in sim state.
- RNG calls from inside `par_iter` closures. Pre-roll.
- Cosmetic events stored as authoritative (must be `cosmetic: true`).
- Atomic-CAS loops on shared sim state.
- Per-frame GPU descriptor-set churn (use Texture2DArray + persistent bind groups).
- "Just throw a Mutex around it" — every Mutex in sim hot paths is a potential serialization point; prefer ECS query-system disjoint access.

### Decision-record alignment

The DR references below cite the canonical copies at `corefall/docs/plan/decisions/`. Each DR's directional stance is satisfied or operationalized by a specific M8A scenario above.

- **DR-002 (Replay / Event taxonomy — closed at M10).** M8A extends `PerformanceBlock` with required subsystem entries; the recorder envelope shape stays additive-compatible. Per-thread shards are an implementation detail; the canonical event stream still satisfies the M4 envelope contract byte-for-byte.
- **DR-005 (Multiplayer Posture — closed at M9 by direction).** Server-authoritative solo / LAN / online / PvP / MMO-ready architecture via `cf-server`. M8A ships the `cf-net::server` + `cf-net::client` + LAN lockstep + internet rollback per the "Server Authority Model" section above. Acceptance: 8-client LAN multiplayer scenario + 2-client internet rollback scenario both pass.
- **DR-007 (Terrain / Material Model — closed at M3 by direction).** M8A's semantic-terrain-event protocol enforces dirty-region tracking at the write barrier; chunk-parallel mutation preserves the M3 dirty-rect contract. Acceptance: every terrain change emits `terrain.chunk_mutated` with `post_state_checksum`; CCCP-style raw bitmap deltas are forbidden.
- **DR-008 (AI Architecture — open).** M8A is the milestone where the layered DR-008 model becomes parallel-friendly. ReactiveGuard ECS migration is the prep work for M13+ squad AI ladders. Acceptance: AI tick runs in `par_iter` across guards with pre-rolled RNG; per-tick checksum byte-matches single-thread reference.
- **DR-013 (Backend Service Scope — closed-direction).** M8A ships the local-first slice: server discovery returns the canonical server-row shape (`version` + `protocol` + `content_hash` + `region` + `map` + `ruleset` + `humans` + `bots` + `trust_tier`); deep links (`corefall://join?...`) are version + content-aware; no plain passwords in URIs. Per `cortext_command_vault/systems/networking-backend-frontend.md` § "Backend Service Slice A".
- **DR-024 (Native Engine Stack — closed).** Bevy 0.18.1 + custom cf-* crates. M8A leans heavier into Bevy's ECS scheduler. No upgrade to Bevy 0.19+ as part of M8A (scheduled work).
- **DR-025 (Target Platforms — closed-direction).** M8A's reference-platform-only target is Windows 11 + Linux x86_64 + macOS aarch64; Steam Deck and integrated-GPU machines are explicitly anti-scoped. Apple M4 Pro / M5+ is a first-class server target.
- **DR-034 (Dedicated Server Application — closed-direction).** `cf-headless serve` is the dedicated-server binary. Same deterministic sim core as the client; no renderer; same run-bundle envelope. Server bundle is the lobby's source of truth for replay + dispute resolution + cross-client divergence detection. Acceptance: 8-client LAN session writes a single server-side run bundle to `prototype_runs/server/<run-id>/` with the full canonical event stream.
- **DR-035 (Persistent MMO Architecture — open-research-direction).** M8A's server tier matrix (workstation-x86 / Apple-Silicon / Linux-VPS / Apple-mini-lab) is the prerequisite for DR-035's shard-aware mesh. The Apple Mini Lab tier is specifically designed for persistent-MMO dev cluster validation. M8A doesn't ship the mesh; it ships the per-shard server that the mesh will be built from.
- **DR-052 (Cross-Platform Determinism + CLI-Testable Rollback — closed-direction).** M8A is the milestone that operationalizes DR-052. Documented at `docs/plan/spec/determinism-island-contract.md`. CI enforces f32-only, RNG seeding rules, hash seeding rules, no FMA in sim islands. The cross-OS CI gate (`game/scripts/ci/m8a_cross_os_determinism.sh`) runs the same scenario on Linux + macOS + Windows and diffs final checksums.
- **DR-054 (Performance Optimization + Profiling — closed-direction).** M8A's perf-budget contract (per-subsystem p50/p99/p999 in `summary.json.performance`) + Tracy / Puffin profiling hooks + perf-regression-gate CI (`game/scripts/ci/m8a_perf_gate.sh`) are the M8A operationalization of DR-054.
- **DR-055 (Game Feel — closed).** Particle / post-FX GPU offload at M8A unlocks the visual juice ladder for M9 (chemistry + bleeding + atmospherics + ricochet sparks) without sim cost.
- **DR-057 (Optional Gacha / Battle Pass / Private Prototype License — open).** N/A at M8A. Mentioned only to confirm M8A does not foreclose monetization scope; the server-authoritative model + bundle replay surface are the prerequisites for any future audit / fairness scope, but M8A doesn't decide it.

### Pitfalls that have bitten us before / will bite again

- **Bevy's parallel scheduler is conservative.** It will refuse to parallelize systems whose query signatures overlap, even when the actual data access is disjoint. Use `ParamSet` or explicit `Disjoint` markers when you know better.
- **Determinism breaks silently.** A `RandomState`-hashed HashMap in sim state will pass all unit tests on one machine, then break on another player's machine when the iteration order differs. Lint for it.
- **GPU compute is non-deterministic across drivers.** Same wgsl shader, same input, different output on NVIDIA vs AMD vs Apple Silicon. **Never** depend on GPU output for sim authority. Always treat as cosmetic.
- **Rollback resimulation amplifies sim cost.** A 6-frame rollback at 60 Hz = 6× the per-tick budget for one frame. The sim must hit p99 ≤ 2.5 ms (not 16.6 ms) in single-step mode to support 6-frame rollback within budget.
- **Network desync compounds.** A 1-bit divergence at tick 1000 explodes into a chaotic state divergence by tick 1100. The CI cross-OS determinism gate is the only way to catch this early.
- **Steam Deck users will complain.** They will. M8A explicitly anti-scopes them. The reference platform is the no-compromise target. Communicate clearly in release notes; offer a "low-density" sim toggle as a future BP option if community pressure warrants.
- **The 16-core 9950X3D has two CCDs (8+8 cores) with NUMA-like behavior.** Threads pinned to the wrong CCD pay 30-100 ns cache-coherence penalty. Optional `hwloc` integration helps; defaults off.
- **The recorder's per-thread shard merge is the single most important determinism fix in M8A.** Get this wrong and every multi-threaded scenario diverges. Get this right and the rest of the parallelism cascades.

### Existing work to credit during the M8A audit

The following M1-M8 surfaces are already aligned with the M8A direction and should NOT be re-architected — credit them and build on them:

- **ChunkedTerrain's sparse storage + dirty-region tracker** (M3). Already the right shape for chunk-parallel mutation.
- **`active_region: bool` flag on Chunk** (M3, forward-compat for M15). M8A just enforces wake-on-edit + sleep-after-N-ticks.
- **`MaterialAffordance::overlay_rgba`** table (M3). Already cache-friendly; GPU shader can read it via uniform buffer.
- **Recorder envelope v0.1 + `cosmetic` flag** (M4). Already the right shape for the determinism-island contract.
- **Per-actor `ActorTickOutcome`** (M1). Already snapshot-read + per-actor isolated; ECS migration is straightforward.
- **AI tick throttle** (M2). Already distributes work by `actor_id % stride`; promote to `par_iter` over the stride window.
- **MissionState lifecycle** (M2). Already isolated from sim hot paths; ECS migration is small.
- **`prototype_run_check.py`** + cross-OS replay matrix (M4). The CI determinism gate already exists in spirit; M8A operationalizes it.
- **The cfctl JSON-RPC wire contract** (M0). Already battle-tested across M1-M8. M8A's ECS refactor does not break it; cfctl handlers continue to read/write ECS state via World queries.

### What "done" looks like for M8A

When the implementing agent reports the M8A verdict table, every scenario above is `PASS (already in)` or `IMPLEMENTED`. The reference-platform bench numbers are captured in `summary.json.performance` for `m9_firehose` AND `mp_8player_lan`. CI gates (perf-budget + cross-OS determinism) are green on the M8A merge PR. The M6-M11 content team can begin M9 on the new foundation with no further refactor.

If any scenario is `STILL FAILING`, the agent reports it honestly. No silent overclaims. No verdict drift.

The spec then moves to `specs/done/M8A.md` and M9 is unblocked.
